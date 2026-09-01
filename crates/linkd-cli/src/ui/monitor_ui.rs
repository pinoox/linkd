use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use linkd_core::{display_path, Ecosystem, LinkEntry, LinkSyncStatus};
use linkd_ipc::{DaemonEvent, IpcClient};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LogItem {
    pub time: String,
    pub level: String,
    pub ecosystem: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    ByPackage,
    ByConsumer,
    Flat,
}

impl GroupMode {
    pub fn next(self) -> Self {
        match self {
            GroupMode::ByPackage => GroupMode::ByConsumer,
            GroupMode::ByConsumer => GroupMode::Flat,
            GroupMode::Flat => GroupMode::ByPackage,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GroupMode::ByPackage => "By Package",
            GroupMode::ByConsumer => "By Consumer",
            GroupMode::Flat => "Flat List",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualTreeItem {
    PackageHeader {
        package_name: String,
        ecosystem: Ecosystem,
        count: usize,
        is_expanded: bool,
        link_indices: Vec<usize>,
    },
    ConsumerHeader {
        consumer_path: PathBuf,
        count: usize,
        is_expanded: bool,
        link_indices: Vec<usize>,
    },
    LinkItem {
        link_id: Uuid,
        link_index: usize,
        is_last: bool,
    },
}

pub struct MonitorState {
    pub daemon_pid: Option<u32>,
    pub pm_hint: Option<String>,
    pub links: Vec<LinkEntry>,
    pub selected_tree_row: usize,
    pub tree_scroll_offset: usize,
    pub group_mode: GroupMode,
    pub collapsed_groups: HashSet<String>,
    pub logs: Vec<LogItem>,
    pub focus_logs: bool,
    pub log_scroll: usize,
    pub auto_scroll_logs: bool,
    pub started_at: Instant,
    pub status_message: Option<(String, Instant)>,
}

impl MonitorState {
    pub fn new(daemon_pid: Option<u32>) -> Self {
        Self {
            daemon_pid,
            pm_hint: None,
            links: Vec::new(),
            selected_tree_row: 0,
            tree_scroll_offset: 0,
            group_mode: GroupMode::ByPackage,
            collapsed_groups: HashSet::new(),
            logs: Vec::new(),
            focus_logs: false,
            log_scroll: 0,
            auto_scroll_logs: true,
            started_at: Instant::now(),
            status_message: None,
        }
    }

    pub fn build_tree_items(&self) -> Vec<VisualTreeItem> {
        let mut items = Vec::new();
        match self.group_mode {
            GroupMode::ByPackage => {
                let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                for (idx, link) in self.links.iter().enumerate() {
                    groups
                        .entry(link.package_name.clone())
                        .or_default()
                        .push(idx);
                }

                for (pkg_name, link_indices) in groups {
                    let first_link = &self.links[link_indices[0]];
                    let is_expanded = !self.collapsed_groups.contains(&pkg_name);
                    items.push(VisualTreeItem::PackageHeader {
                        package_name: pkg_name.clone(),
                        ecosystem: first_link.ecosystem,
                        count: link_indices.len(),
                        is_expanded,
                        link_indices: link_indices.clone(),
                    });

                    if is_expanded {
                        let total = link_indices.len();
                        for (i, &link_idx) in link_indices.iter().enumerate() {
                            items.push(VisualTreeItem::LinkItem {
                                link_id: self.links[link_idx].id,
                                link_index: link_idx,
                                is_last: i + 1 == total,
                            });
                        }
                    }
                }
            }
            GroupMode::ByConsumer => {
                let mut groups: BTreeMap<PathBuf, Vec<usize>> = BTreeMap::new();
                for (idx, link) in self.links.iter().enumerate() {
                    groups
                        .entry(link.consumer_root.clone())
                        .or_default()
                        .push(idx);
                }

                for (consumer_path, link_indices) in groups {
                    let group_key = consumer_path.display().to_string();
                    let is_expanded = !self.collapsed_groups.contains(&group_key);
                    items.push(VisualTreeItem::ConsumerHeader {
                        consumer_path: consumer_path.clone(),
                        count: link_indices.len(),
                        is_expanded,
                        link_indices: link_indices.clone(),
                    });

                    if is_expanded {
                        let total = link_indices.len();
                        for (i, &link_idx) in link_indices.iter().enumerate() {
                            items.push(VisualTreeItem::LinkItem {
                                link_id: self.links[link_idx].id,
                                link_index: link_idx,
                                is_last: i + 1 == total,
                            });
                        }
                    }
                }
            }
            GroupMode::Flat => {
                for (idx, link) in self.links.iter().enumerate() {
                    items.push(VisualTreeItem::LinkItem {
                        link_id: link.id,
                        link_index: idx,
                        is_last: true,
                    });
                }
            }
        }
        items
    }

    pub fn selected_tree_item<'a>(
        &self,
        items: &'a [VisualTreeItem],
    ) -> Option<&'a VisualTreeItem> {
        items.get(self.selected_tree_row)
    }

    pub fn add_log(
        &mut self,
        level: impl Into<String>,
        eco: Option<String>,
        msg: impl Into<String>,
    ) {
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(LogItem {
            time: now,
            level: level.into(),
            ecosystem: eco,
            message: msg.into(),
        });
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
        if self.auto_scroll_logs {
            self.log_scroll = self.logs.len();
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn uptime_str(&self) -> String {
        let s = self.started_at.elapsed().as_secs();
        let h = s / 3600;
        let m = (s % 3600) / 60;
        let sec = s % 60;
        format!("{h:02}:{m:02}:{sec:02}")
    }

    pub fn ensure_tree_row_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected_tree_row < self.tree_scroll_offset {
            self.tree_scroll_offset = self.selected_tree_row;
        } else if self.selected_tree_row >= self.tree_scroll_offset + visible_height {
            self.tree_scroll_offset = self.selected_tree_row + 1 - visible_height;
        }
    }
}

pub async fn run_monitor_ui(daemon_pid: Option<u32>) -> io::Result<()> {
    let client = IpcClient::new().map_err(|e| io::Error::other(e.to_string()))?;
    let mut event_rx = match client.subscribe_events().await {
        Ok(rx) => rx,
        Err(e) => return Err(io::Error::other(e.to_string())),
    };

    let mut terminal = ratatui::init();
    let mut state = MonitorState::new(daemon_pid);
    state.add_log("INFO", None, "Attached to linkd background daemon");

    // Fetch initial link list snapshot
    if let Ok(links) = client.list_links().await {
        state.links = links;
    }

    let result = run_loop(&mut terminal, &mut state, &mut event_rx, &client).await;
    ratatui::restore();
    result
}

async fn run_loop(
    terminal: &mut DefaultTerminal,
    state: &mut MonitorState,
    event_rx: &mut mpsc::Receiver<DaemonEvent>,
    client: &IpcClient,
) -> io::Result<()> {
    let mut ticker = tokio::time::interval(Duration::from_millis(80));

    loop {
        // Drain any incoming daemon events non-blockingly
        while let Ok(event) = event_rx.try_recv() {
            handle_daemon_event(state, event);
        }

        // Clean status message after 3 seconds
        if let Some((_, set_at)) = &state.status_message {
            if set_at.elapsed() > Duration::from_secs(3) {
                state.status_message = None;
            }
        }

        let tree_items = state.build_tree_items();
        if state.selected_tree_row >= tree_items.len() && !tree_items.is_empty() {
            state.selected_tree_row = tree_items.len() - 1;
        }

        terminal.draw(|frame| {
            render_ui(frame, state, &tree_items);
        })?;

        tokio::select! {
            _ = ticker.tick() => {}
            _ = tokio::time::sleep(Duration::from_millis(25)) => {
                // Poll input with zero delay
                while event::poll(Duration::from_millis(0))? {
                    let ev = event::read()?;
                    if let Event::Key(key) = ev {
                        // CRITICAL FIX: Only react to KeyPress events!
                        // On Windows crossterm emits KeyEventKind::Release and Repeat which causes cursor jumping!
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }

                        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                            return Ok(());
                        }

                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(());
                            }
                            KeyCode::Tab => {
                                state.focus_logs = !state.focus_logs;
                                state.set_status(if state.focus_logs {
                                    "Focus: Logs Panel (↑/↓ to scroll)"
                                } else {
                                    "Focus: Links Tree (↑/↓ to navigate)"
                                });
                            }
                            KeyCode::Char('g') => {
                                state.group_mode = state.group_mode.next();
                                state.selected_tree_row = 0;
                                state.tree_scroll_offset = 0;
                                state.set_status(format!("View Mode: {}", state.group_mode.label()));
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.focus_logs {
                                    state.auto_scroll_logs = false;
                                    state.log_scroll = state.log_scroll.saturating_sub(1);
                                } else if state.selected_tree_row > 0 {
                                    state.selected_tree_row -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.focus_logs {
                                    state.log_scroll = (state.log_scroll + 1).min(state.logs.len());
                                    if state.log_scroll >= state.logs.len() {
                                        state.auto_scroll_logs = true;
                                    }
                                } else if !tree_items.is_empty() && state.selected_tree_row + 1 < tree_items.len() {
                                    state.selected_tree_row += 1;
                                }
                            }
                            KeyCode::Home => {
                                if state.focus_logs {
                                    state.auto_scroll_logs = false;
                                    state.log_scroll = 0;
                                } else {
                                    state.selected_tree_row = 0;
                                    state.tree_scroll_offset = 0;
                                }
                            }
                            KeyCode::End => {
                                if state.focus_logs {
                                    state.auto_scroll_logs = true;
                                    state.log_scroll = state.logs.len();
                                } else if !tree_items.is_empty() {
                                    state.selected_tree_row = tree_items.len() - 1;
                                }
                            }
                            KeyCode::PageUp => {
                                if state.focus_logs {
                                    state.auto_scroll_logs = false;
                                    state.log_scroll = state.log_scroll.saturating_sub(10);
                                } else {
                                    state.selected_tree_row = state.selected_tree_row.saturating_sub(10);
                                }
                            }
                            KeyCode::PageDown => {
                                if state.focus_logs {
                                    state.log_scroll = (state.log_scroll + 10).min(state.logs.len());
                                } else if !tree_items.is_empty() {
                                    state.selected_tree_row = (state.selected_tree_row + 10).min(tree_items.len() - 1);
                                }
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                if let Some(item) = tree_items.get(state.selected_tree_row) {
                                    match item {
                                        VisualTreeItem::PackageHeader { package_name, .. } => {
                                            if state.collapsed_groups.contains(package_name) {
                                                state.collapsed_groups.remove(package_name);
                                                state.set_status(format!("Expanded {package_name}"));
                                            } else {
                                                state.collapsed_groups.insert(package_name.clone());
                                                state.set_status(format!("Collapsed {package_name}"));
                                            }
                                        }
                                        VisualTreeItem::ConsumerHeader { consumer_path, .. } => {
                                            let key = consumer_path.display().to_string();
                                            if state.collapsed_groups.contains(&key) {
                                                state.collapsed_groups.remove(&key);
                                                state.set_status("Expanded consumer");
                                            } else {
                                                state.collapsed_groups.insert(key);
                                                state.set_status("Collapsed consumer");
                                            }
                                        }
                                        VisualTreeItem::LinkItem { link_index, .. } => {
                                            if let Some(link) = state.links.get(*link_index) {
                                                let pkg = link.package_name.clone();
                                                let client_clone = IpcClient::new().ok();
                                                if let Some(c) = client_clone {
                                                    let pkg_c = pkg.clone();
                                                    tokio::spawn(async move {
                                                        let _ = c.toggle_pause_link(&pkg_c).await;
                                                    });
                                                }
                                                state.set_status(format!("Toggled pause for {pkg}"));
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('r') => {
                                if let Some(item) = tree_items.get(state.selected_tree_row) {
                                    match item {
                                        VisualTreeItem::PackageHeader { package_name, link_indices, .. } => {
                                            state.add_log("CMD", None, format!("Reconciling {package_name}"));
                                            state.set_status(format!("Reconcile triggered for {package_name}"));
                                            for &idx in link_indices {
                                                if let Some(link) = state.links.get(idx) {
                                                    let id = link.id;
                                                    let client_clone = IpcClient::new().ok();
                                                    if let Some(c) = client_clone {
                                                        tokio::spawn(async move {
                                                            let _ = c.trigger_reconcile(Some(id)).await;
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        VisualTreeItem::ConsumerHeader { consumer_path, link_indices, .. } => {
                                            let c_name = display_path(consumer_path);
                                            state.add_log("CMD", None, format!("Reconciling {c_name}"));
                                            state.set_status(format!("Reconcile triggered for {c_name}"));
                                            for &idx in link_indices {
                                                if let Some(link) = state.links.get(idx) {
                                                    let id = link.id;
                                                    let client_clone = IpcClient::new().ok();
                                                    if let Some(c) = client_clone {
                                                        tokio::spawn(async move {
                                                            let _ = c.trigger_reconcile(Some(id)).await;
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        VisualTreeItem::LinkItem { link_index, .. } => {
                                            if let Some(link) = state.links.get(*link_index) {
                                                let id = link.id;
                                                let pkg = link.package_name.clone();
                                                state.add_log("CMD", Some(format!("{:?}", link.ecosystem)), format!("Reconciling {pkg}"));
                                                state.set_status(format!("Reconcile triggered for {pkg}"));
                                                let client_clone = IpcClient::new().ok();
                                                if let Some(c) = client_clone {
                                                    tokio::spawn(async move {
                                                        let _ = c.trigger_reconcile(Some(id)).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('u') => {
                                if let Some(item) = tree_items.get(state.selected_tree_row) {
                                    match item {
                                        VisualTreeItem::PackageHeader { package_name, .. } => {
                                            let pkg = package_name.clone();
                                            state.add_log("CMD", None, format!("Unlinking all instances of {pkg}"));
                                            state.set_status(format!("Unlinked {pkg}"));
                                            let client_clone = IpcClient::new().ok();
                                            if let Some(c) = client_clone {
                                                let pkg_c = pkg.clone();
                                                tokio::spawn(async move {
                                                    let _ = c.remove_link(&pkg_c).await;
                                                });
                                            }
                                        }
                                        VisualTreeItem::ConsumerHeader { .. } => {}
                                        VisualTreeItem::LinkItem { link_index, .. } => {
                                            if let Some(link) = state.links.get(*link_index) {
                                                let pkg = link.package_name.clone();
                                                state.add_log("CMD", Some(format!("{:?}", link.ecosystem)), format!("Unlinking {pkg}"));
                                                state.set_status(format!("Unlinked {pkg}"));
                                                let client_clone = IpcClient::new().ok();
                                                if let Some(c) = client_clone {
                                                    let pkg_c = pkg.clone();
                                                    tokio::spawn(async move {
                                                        let _ = c.remove_link(&pkg_c).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    if let Ok(updated_links) = client.list_links().await {
                                        state.links = updated_links;
                                    }
                                }
                            }
                            KeyCode::Char('c') => {
                                state.logs.clear();
                                state.log_scroll = 0;
                                state.auto_scroll_logs = true;
                                state.set_status("Logs cleared");
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn handle_daemon_event(state: &mut MonitorState, event: DaemonEvent) {
    match event {
        DaemonEvent::Snapshot { snapshot } => {
            state.links = snapshot.links;
            state.pm_hint = snapshot.pm_install_hint;
        }
        DaemonEvent::LinkStatusChanged {
            package_name,
            status,
            last_synced_at,
            ..
        } => {
            if let Some(link) = state
                .links
                .iter_mut()
                .find(|l| l.package_name == package_name)
            {
                link.last_sync_status = status;
                if let Some(at) = last_synced_at {
                    link.last_sync_at = Some(at);
                }
            }
        }
        DaemonEvent::SyncStarted {
            package_name,
            files_count,
            ..
        } => {
            if let Some(link) = state
                .links
                .iter_mut()
                .find(|l| l.package_name == package_name)
            {
                link.last_sync_status = LinkSyncStatus::Syncing;
                link.file_count = files_count as u32;
            }
            state.add_log(
                "SYNC",
                None,
                format!("Sync started for {package_name} ({files_count} files)"),
            );
        }
        DaemonEvent::SyncCompleted {
            package_name,
            duration_ms,
            files_synced,
            ..
        } => {
            if let Some(link) = state
                .links
                .iter_mut()
                .find(|l| l.package_name == package_name)
            {
                link.last_sync_status = LinkSyncStatus::Synced;
                link.file_count = files_synced as u32;
                link.last_sync_at = Some(chrono::Utc::now());
            }
            state.add_log(
                "DONE",
                None,
                format!("Synced {files_synced} files for {package_name} in {duration_ms}ms"),
            );
        }
        DaemonEvent::SyncFailed {
            package_name,
            error,
            ..
        } => {
            if let Some(link) = state
                .links
                .iter_mut()
                .find(|l| l.package_name == package_name)
            {
                link.last_sync_status = LinkSyncStatus::Error;
            }
            state.add_log(
                "ERR",
                None,
                format!("Sync failed for {package_name}: {error}"),
            );
        }
        DaemonEvent::LogMessage {
            level,
            ecosystem,
            message,
            ..
        } => {
            state.add_log(level, ecosystem, message);
        }
    }
}

fn render_ui(frame: &mut ratatui::Frame, state: &mut MonitorState, tree_items: &[VisualTreeItem]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header / Status bar
            Constraint::Min(8),    // Main workspace (Panels)
            Constraint::Length(3), // Footer / Keybindings
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_workspace(frame, chunks[1], state, tree_items);
    render_footer(frame, chunks[2], state);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, state: &MonitorState) {
    let pid_str = state
        .daemon_pid
        .map(|p| format!("PID {p}"))
        .unwrap_or_else(|| "RUNNING".into());

    let uptime = state.uptime_str();
    let total_links = state.links.len();

    let mut header_spans = vec![
        Span::styled(
            " ⚡ linkd monitor ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ "),
        Span::styled("● ", Style::default().fg(Color::Green)),
        Span::styled(
            format!("Daemon: {pid_str}"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ "),
        Span::styled("⏱ ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("Uptime: {uptime}"),
            Style::default().fg(Color::White),
        ),
        Span::raw(" │ "),
        Span::styled("🔗 ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("Active Links: {total_links}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" │ "),
        Span::styled("👁 ", Style::default().fg(Color::Magenta)),
        Span::styled(
            format!("View: [{}]", state.group_mode.label()),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    if let Some((msg, _)) = &state.status_message {
        header_spans.push(Span::raw(" │ "));
        header_spans.push(Span::styled(
            format!("🔔 {msg}"),
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let p = Paragraph::new(Line::from(header_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(p, area);
}

fn render_workspace(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &mut MonitorState,
    tree_items: &[VisualTreeItem],
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left panel: Nested tree + Details
            Constraint::Percentage(50), // Right panel: Live Logs
        ])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(54), // Hierarchical Links tree
            Constraint::Percentage(46), // Details / Inspector
        ])
        .split(columns[0]);

    render_links_list(frame, left_chunks[0], state, tree_items);
    render_link_inspector(frame, left_chunks[1], state, tree_items);
    render_logs_panel(frame, columns[1], state);
}

fn render_links_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &mut MonitorState,
    tree_items: &[VisualTreeItem],
) {
    let is_focused = !state.focus_logs;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    state.ensure_tree_row_visible(visible_height);

    let scroll_info = if tree_items.len() > visible_height {
        format!(" [{}/{}]", state.selected_tree_row + 1, tree_items.len())
    } else {
        String::new()
    };

    let title = format!(
        " 🌲 Active Links ({}) — {}{scroll_info} ",
        state.links.len(),
        state.group_mode.label()
    );
    let mut lines = Vec::new();

    if state.links.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active links registered. Run `linkd init` to link packages.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let start_idx = state.tree_scroll_offset;

        for (i, item) in tree_items
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(visible_height)
        {
            let is_selected = i == state.selected_tree_row;
            let row_bg = if is_selected && is_focused {
                Color::Rgb(22, 44, 75)
            } else if is_selected {
                Color::Rgb(30, 35, 45)
            } else {
                Color::Reset
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let prefix_color = if is_selected {
                Color::Cyan
            } else {
                Color::DarkGray
            };

            match item {
                VisualTreeItem::PackageHeader {
                    package_name,
                    ecosystem,
                    count,
                    is_expanded,
                    ..
                } => {
                    let arrow = if *is_expanded { "▼" } else { "▶" };
                    let eco_badge = format_eco_badge(*ecosystem);
                    let count_str = if *count == 1 {
                        "(1 consumer)".to_string()
                    } else {
                        format!("({count} consumers)")
                    };

                    let header_style = if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::LightCyan)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(prefix_color).bg(row_bg)),
                        Span::styled(
                            format!("{arrow} 📦 "),
                            Style::default().fg(Color::Yellow).bg(row_bg),
                        ),
                        Span::styled(package_name, header_style),
                        Span::styled(" ", Style::default().bg(row_bg)),
                        eco_badge,
                        Span::styled(" ", Style::default().bg(row_bg)),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray).bg(row_bg)),
                    ]));
                }
                VisualTreeItem::ConsumerHeader {
                    consumer_path,
                    count,
                    is_expanded,
                    ..
                } => {
                    let arrow = if *is_expanded { "▼" } else { "▶" };
                    let count_str = if *count == 1 {
                        "(1 package)".to_string()
                    } else {
                        format!("({count} packages)")
                    };
                    let c_name = display_path(consumer_path);

                    let header_style = if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::LightMagenta)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(prefix_color).bg(row_bg)),
                        Span::styled(
                            format!("{arrow} 📂 "),
                            Style::default().fg(Color::Magenta).bg(row_bg),
                        ),
                        Span::styled(c_name, header_style),
                        Span::styled(" ", Style::default().bg(row_bg)),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray).bg(row_bg)),
                    ]));
                }
                VisualTreeItem::LinkItem {
                    link_index,
                    is_last,
                    ..
                } => {
                    if let Some(link) = state.links.get(*link_index) {
                        let branch = if *is_last { "└──" } else { "├──" };
                        let status_badge = format_status_badge(link.last_sync_status, row_bg);

                        let name_style = if is_selected {
                            Style::default()
                                .fg(Color::White)
                                .bg(row_bg)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray).bg(row_bg)
                        };

                        let item_display = match state.group_mode {
                            GroupMode::ByPackage => {
                                let c_name = display_path(&link.consumer_root);
                                vec![
                                    Span::styled(
                                        prefix,
                                        Style::default().fg(prefix_color).bg(row_bg),
                                    ),
                                    Span::styled(
                                        format!("   {branch} 📂 "),
                                        Style::default().fg(Color::Cyan).bg(row_bg),
                                    ),
                                    Span::styled(c_name, name_style),
                                    Span::styled(" ", Style::default().bg(row_bg)),
                                    status_badge,
                                    Span::styled(" ", Style::default().bg(row_bg)),
                                    Span::styled(
                                        format!("({} files)", link.file_count),
                                        Style::default().fg(Color::DarkGray).bg(row_bg),
                                    ),
                                ]
                            }
                            GroupMode::ByConsumer => {
                                let eco_badge = format_eco_badge(link.ecosystem);
                                vec![
                                    Span::styled(
                                        prefix,
                                        Style::default().fg(prefix_color).bg(row_bg),
                                    ),
                                    Span::styled(
                                        format!("   {branch} 📦 "),
                                        Style::default().fg(Color::Cyan).bg(row_bg),
                                    ),
                                    Span::styled(&link.package_name, name_style),
                                    Span::styled(" ", Style::default().bg(row_bg)),
                                    eco_badge,
                                    Span::styled(" ", Style::default().bg(row_bg)),
                                    status_badge,
                                ]
                            }
                            GroupMode::Flat => {
                                let eco_badge = format_eco_badge(link.ecosystem);
                                let c_name = display_path(&link.consumer_root);
                                vec![
                                    Span::styled(
                                        prefix,
                                        Style::default().fg(prefix_color).bg(row_bg),
                                    ),
                                    Span::styled(
                                        "📦 ",
                                        Style::default().fg(Color::Yellow).bg(row_bg),
                                    ),
                                    Span::styled(&link.package_name, name_style),
                                    Span::styled(
                                        " → ",
                                        Style::default().fg(Color::DarkGray).bg(row_bg),
                                    ),
                                    Span::styled(
                                        c_name,
                                        Style::default().fg(Color::DarkGray).bg(row_bg),
                                    ),
                                    Span::styled(" ", Style::default().bg(row_bg)),
                                    eco_badge,
                                    Span::styled(" ", Style::default().bg(row_bg)),
                                    status_badge,
                                ]
                            }
                        };

                        lines.push(Line::from(item_display));
                    }
                }
            }
        }
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_widget(p, area);
}

fn format_eco_badge(ecosystem: Ecosystem) -> Span<'static> {
    match ecosystem {
        Ecosystem::Npm => Span::styled("[npm]", Style::default().fg(Color::Red)),
        Ecosystem::Composer => Span::styled("[composer]", Style::default().fg(Color::Magenta)),
        Ecosystem::Python => Span::styled("[python]", Style::default().fg(Color::Yellow)),
        Ecosystem::Go => Span::styled("[go]", Style::default().fg(Color::Cyan)),
        Ecosystem::Cargo => Span::styled("[cargo]", Style::default().fg(Color::LightRed)),
        Ecosystem::Jvm => Span::styled("[jvm]", Style::default().fg(Color::LightGreen)),
        Ecosystem::Dart => Span::styled("[dart/flutter]", Style::default().fg(Color::LightCyan)),
        Ecosystem::Dotnet => Span::styled("[dotnet]", Style::default().fg(Color::Magenta)),
        Ecosystem::Ruby => Span::styled("[ruby]", Style::default().fg(Color::Red)),
        Ecosystem::Swift => Span::styled("[swift]", Style::default().fg(Color::LightYellow)),
        Ecosystem::Elixir => Span::styled("[elixir]", Style::default().fg(Color::LightMagenta)),
        Ecosystem::Custom => Span::styled("[custom]", Style::default().fg(Color::Blue)),
    }
}

fn format_status_badge(status: LinkSyncStatus, bg: Color) -> Span<'static> {
    match status {
        LinkSyncStatus::Synced => Span::styled(
            "[✓ IDLE]",
            Style::default()
                .fg(Color::Green)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        LinkSyncStatus::Syncing => Span::styled(
            "[⚡ SYNC]",
            Style::default()
                .fg(Color::Yellow)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        LinkSyncStatus::Error => Span::styled(
            "[✕ ERROR]",
            Style::default()
                .fg(Color::Red)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        LinkSyncStatus::Pending => {
            Span::styled("[… PENDING]", Style::default().fg(Color::Yellow).bg(bg))
        }
        LinkSyncStatus::Paused => {
            Span::styled("[⏸ PAUSED]", Style::default().fg(Color::DarkGray).bg(bg))
        }
    }
}

fn render_link_inspector(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &MonitorState,
    tree_items: &[VisualTreeItem],
) {
    let mut lines = Vec::new();

    if let Some(item) = state.selected_tree_item(tree_items) {
        match item {
            VisualTreeItem::PackageHeader {
                package_name,
                ecosystem,
                count,
                link_indices,
                ..
            } => {
                let first_link = link_indices.first().and_then(|&idx| state.links.get(idx));

                lines.push(Line::from(vec![
                    Span::styled(
                        "📦 Package Group: ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        package_name,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    format_eco_badge(*ecosystem),
                ]));

                if let Some(link) = first_link {
                    lines.push(Line::from(vec![
                        Span::styled("   Source Path:   ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.source_path)),
                    ]));
                }

                lines.push(Line::from(vec![
                    Span::styled("   Consumers:     ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{count} linked project(s)"),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                let mut total_files = 0;
                for &idx in link_indices {
                    if let Some(link) = state.links.get(idx) {
                        total_files += link.file_count;
                        let status_str = match link.last_sync_status {
                            LinkSyncStatus::Synced => "✓ Synced",
                            LinkSyncStatus::Syncing => "⚡ Syncing...",
                            LinkSyncStatus::Error => "✕ Error",
                            LinkSyncStatus::Pending => "… Pending",
                            LinkSyncStatus::Paused => "⏸ Paused",
                        };
                        lines.push(Line::from(vec![
                            Span::styled("     • ", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                display_path(&link.consumer_root),
                                Style::default().fg(Color::LightCyan),
                            ),
                            Span::styled(
                                format!(" ({status_str}, {} files)", link.file_count),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                }

                lines.push(Line::from(vec![
                    Span::styled("   Total Files:   ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!(
                        "{total_files} files synchronized across {count} consumers"
                    )),
                ]));

                lines.push(Line::from(vec![
                    Span::styled("   Quick Actions: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "[r]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Reconcile All  "),
                    Span::styled(
                        "[Space]",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Expand/Collapse  "),
                    Span::styled(
                        "[u]",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Unlink All"),
                ]));
            }
            VisualTreeItem::ConsumerHeader {
                consumer_path,
                count,
                link_indices,
                ..
            } => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "📂 Consumer App:  ",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        display_path(consumer_path),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("   Packages:      ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{count} linked package(s)"),
                        Style::default().fg(Color::Green),
                    ),
                ]));

                for &idx in link_indices {
                    if let Some(link) = state.links.get(idx) {
                        lines.push(Line::from(vec![
                            Span::styled("     • ", Style::default().fg(Color::DarkGray)),
                            Span::styled(&link.package_name, Style::default().fg(Color::LightCyan)),
                            Span::raw(" "),
                            format_eco_badge(link.ecosystem),
                            Span::styled(
                                format!(" ({} files)", link.file_count),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                }

                lines.push(Line::from(vec![
                    Span::styled("   Quick Actions: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "[r]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Reconcile All  "),
                    Span::styled(
                        "[Space]",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" Expand/Collapse"),
                ]));
            }
            VisualTreeItem::LinkItem { link_index, .. } => {
                if let Some(link) = state.links.get(*link_index) {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "📦 Package:       ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            &link.package_name,
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        format_eco_badge(link.ecosystem),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("   Source Path:   ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.source_path)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("   Consumer Root: ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.consumer_root)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("   Sync Target:   ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.sync_target)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("   Strategy/Mode: ", Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("{:?}", link.strategy),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(" │ Mode: "),
                        Span::styled(
                            format!("{:?}", link.link_mode),
                            Style::default().fg(Color::White),
                        ),
                        Span::raw(" │ Files: "),
                        Span::styled(
                            format!("{}", link.file_count),
                            Style::default().fg(Color::Green),
                        ),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("   Sync Status:   ", Style::default().fg(Color::Cyan)),
                        format_status_badge(link.last_sync_status, Color::Reset),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:?}", link.last_sync_status),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                    if let Some(at) = link.last_sync_at {
                        lines.push(Line::from(vec![
                            Span::styled("   Last Synced:   ", Style::default().fg(Color::Cyan)),
                            Span::raw(format!("{}", at.format("%Y-%m-%d %H:%M:%S UTC"))),
                        ]));
                    }
                }
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  Select a link or package group from the list above to inspect details.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" 🔍 Link Details & Health ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(p, area);
}

fn render_logs_panel(frame: &mut ratatui::Frame, area: Rect, state: &MonitorState) {
    let border_color = if state.focus_logs {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let visible_height = area.height.saturating_sub(2) as usize;
    let total_logs = state.logs.len();

    let scroll_pos = if state.auto_scroll_logs || state.log_scroll >= total_logs {
        total_logs.saturating_sub(visible_height)
    } else {
        state
            .log_scroll
            .min(total_logs.saturating_sub(visible_height))
    };

    let title = format!(" 📜 Live Engine Logs ({total_logs}) ");
    let mut lines = Vec::new();

    for item in state.logs.iter().skip(scroll_pos).take(visible_height) {
        let level_style = match item.level.as_str() {
            "ERR" | "ERROR" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            "DONE" => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            "SYNC" => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            "HINT" => Style::default().fg(Color::Magenta),
            "CMD" => Style::default().fg(Color::Cyan),
            "WATCH" => Style::default().fg(Color::Blue),
            _ => Style::default().fg(Color::DarkGray),
        };

        let mut spans = vec![
            Span::styled(
                format!("{} ", item.time),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(format!("[{:<5}]", item.level), level_style),
        ];

        if let Some(eco) = &item.ecosystem {
            spans.push(Span::styled(
                format!(" [{eco}]"),
                Style::default().fg(Color::LightMagenta),
            ));
        }

        spans.push(Span::raw(format!(" {}", item.message)));
        lines.push(Line::from(spans));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "Waiting for engine events...",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_widget(p, area);
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, _state: &MonitorState) {
    let keybinds = vec![
        Span::styled(
            "[↑/↓/j/k]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Nav  "),
        Span::styled(
            "[g]",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Group View  "),
        Span::styled(
            "[Space/↵]",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Expand/Pause  "),
        Span::styled(
            "[r]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Reconcile  "),
        Span::styled(
            "[u]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Unlink  "),
        Span::styled(
            "[c]",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Clear Logs  "),
        Span::styled(
            "[Tab]",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Switch Focus  "),
        Span::styled(
            "[q/Esc]",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Detach"),
    ];

    let p = Paragraph::new(Line::from(keybinds)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(p, area);
}
