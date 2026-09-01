use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use linkd_core::{display_path, Ecosystem, LinkEntry, LinkSyncStatus};
use linkd_ipc::{DaemonEvent, IpcClient};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
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
    pub group_mode: GroupMode,
    pub collapsed_groups: HashSet<String>,
    pub logs: Vec<LogItem>,
    pub focus_logs: bool,
    pub log_scroll: usize,
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
            group_mode: GroupMode::ByPackage,
            collapsed_groups: HashSet::new(),
            logs: Vec::new(),
            focus_logs: false,
            log_scroll: 0,
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

    pub fn selected_link<'a>(&'a self, items: &[VisualTreeItem]) -> Option<&'a LinkEntry> {
        match items.get(self.selected_tree_row)? {
            VisualTreeItem::LinkItem { link_index, .. } => self.links.get(*link_index),
            VisualTreeItem::PackageHeader { link_indices, .. } => {
                link_indices.first().and_then(|&idx| self.links.get(idx))
            }
            VisualTreeItem::ConsumerHeader { link_indices, .. } => {
                link_indices.first().and_then(|&idx| self.links.get(idx))
            }
        }
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
    let mut ticker = tokio::time::interval(Duration::from_millis(100));

    loop {
        // Drain any incoming daemon events
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
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                if event::poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                            break;
                        }

                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                break;
                            }
                            KeyCode::Tab => {
                                state.focus_logs = !state.focus_logs;
                            }
                            KeyCode::Char('g') => {
                                state.group_mode = state.group_mode.next();
                                state.set_status(format!("Grouping: {}", state.group_mode.label()));
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.focus_logs {
                                    state.log_scroll = state.log_scroll.saturating_sub(1);
                                } else if state.selected_tree_row > 0 {
                                    state.selected_tree_row -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.focus_logs {
                                    state.log_scroll = (state.log_scroll + 1).min(state.logs.len().saturating_sub(1));
                                } else if !tree_items.is_empty() && state.selected_tree_row + 1 < tree_items.len() {
                                    state.selected_tree_row += 1;
                                }
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                if let Some(item) = tree_items.get(state.selected_tree_row) {
                                    match item {
                                        VisualTreeItem::PackageHeader { package_name, .. } => {
                                            if state.collapsed_groups.contains(package_name) {
                                                state.collapsed_groups.remove(package_name);
                                            } else {
                                                state.collapsed_groups.insert(package_name.clone());
                                            }
                                        }
                                        VisualTreeItem::ConsumerHeader { consumer_path, .. } => {
                                            let key = consumer_path.display().to_string();
                                            if state.collapsed_groups.contains(&key) {
                                                state.collapsed_groups.remove(&key);
                                            } else {
                                                state.collapsed_groups.insert(key);
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
                                            state.add_log("CMD", None, format!("Reconciling all links for {package_name}"));
                                            state.set_status(format!("Reconcile triggered for {package_name} ({link_indices_len} consumers)", link_indices_len = link_indices.len()));
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
                                            state.add_log("CMD", None, format!("Reconciling all packages in {c_name}"));
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
                                                let pkg = link.package_name.clone();
                                                let id = link.id;
                                                state.add_log("CMD", Some(format!("{:?}", link.ecosystem)), format!("Manual reconcile requested for {pkg}"));
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
                            KeyCode::Char('p') => {
                                if let Some(link) = state.selected_link(&tree_items) {
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
                                state.set_status("Logs cleared");
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(())
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

fn render_ui(frame: &mut ratatui::Frame, state: &MonitorState, tree_items: &[VisualTreeItem]) {
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
        Span::styled(
            format!("Daemon: {pid_str}"),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("Uptime: {uptime}"),
            Style::default().fg(Color::White),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("Active Links: {total_links}"),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" │ "),
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
            msg,
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let p = Paragraph::new(Line::from(header_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(p, area);
}

fn render_workspace(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &MonitorState,
    tree_items: &[VisualTreeItem],
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(48), // Left panel: Nested tree + Details
            Constraint::Percentage(52), // Right panel: Live Logs
        ])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(55), // Hierarchical Links tree
            Constraint::Percentage(45), // Details / Inspector
        ])
        .split(columns[0]);

    render_links_list(frame, left_chunks[0], state, tree_items);
    render_link_inspector(frame, left_chunks[1], state, tree_items);
    render_logs_panel(frame, columns[1], state);
}

fn render_links_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &MonitorState,
    tree_items: &[VisualTreeItem],
) {
    let border_color = if !state.focus_logs {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = format!(
        " Active Links ({}) [{}] [g: switch] ",
        state.links.len(),
        state.group_mode.label()
    );
    let mut lines = Vec::new();

    if state.links.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active links. Run `linkd link` or `linkd use` to add.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, item) in tree_items.iter().enumerate() {
            let is_selected = i == state.selected_tree_row;
            let prefix = if is_selected { "> " } else { "  " };
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
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::LightCyan)
                            .add_modifier(Modifier::BOLD)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(prefix_color)),
                        Span::styled(format!("{arrow} 📦 "), Style::default().fg(Color::Yellow)),
                        Span::styled(package_name, header_style),
                        Span::raw(" "),
                        eco_badge,
                        Span::raw(" "),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
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
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::LightMagenta)
                            .add_modifier(Modifier::BOLD)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(prefix_color)),
                        Span::styled(format!("{arrow} 📂 "), Style::default().fg(Color::Magenta)),
                        Span::styled(c_name, header_style),
                        Span::raw(" "),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
                    ]));
                }
                VisualTreeItem::LinkItem {
                    link_index,
                    is_last,
                    ..
                } => {
                    if let Some(link) = state.links.get(*link_index) {
                        let branch = if *is_last { "└──" } else { "├──" };
                        let status_badge = format_status_badge(link.last_sync_status);

                        let name_style = if is_selected {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::Gray)
                        };

                        let item_display = match state.group_mode {
                            GroupMode::ByPackage => {
                                let c_name = display_path(&link.consumer_root);
                                vec![
                                    Span::styled(prefix, Style::default().fg(prefix_color)),
                                    Span::styled(
                                        format!("   {branch} 📂 "),
                                        Style::default().fg(Color::Cyan),
                                    ),
                                    Span::styled(c_name, name_style),
                                    Span::raw(" "),
                                    status_badge,
                                    Span::raw(" "),
                                    Span::styled(
                                        format!("({} files)", link.file_count),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                ]
                            }
                            GroupMode::ByConsumer => {
                                let eco_badge = format_eco_badge(link.ecosystem);
                                vec![
                                    Span::styled(prefix, Style::default().fg(prefix_color)),
                                    Span::styled(
                                        format!("   {branch} 📦 "),
                                        Style::default().fg(Color::Cyan),
                                    ),
                                    Span::styled(&link.package_name, name_style),
                                    Span::raw(" "),
                                    eco_badge,
                                    Span::raw(" "),
                                    status_badge,
                                ]
                            }
                            GroupMode::Flat => {
                                let eco_badge = format_eco_badge(link.ecosystem);
                                let c_name = display_path(&link.consumer_root);
                                vec![
                                    Span::styled(prefix, Style::default().fg(prefix_color)),
                                    Span::styled("📦 ", Style::default().fg(Color::Yellow)),
                                    Span::styled(&link.package_name, name_style),
                                    Span::styled(" → ", Style::default().fg(Color::DarkGray)),
                                    Span::styled(c_name, Style::default().fg(Color::DarkGray)),
                                    Span::raw(" "),
                                    eco_badge,
                                    Span::raw(" "),
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

fn format_status_badge(status: LinkSyncStatus) -> Span<'static> {
    match status {
        LinkSyncStatus::Synced => Span::styled("[IDLE]", Style::default().fg(Color::Green)),
        LinkSyncStatus::Syncing => Span::styled(
            "[SYNC]",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        LinkSyncStatus::Error => Span::styled(
            "[ERR]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        LinkSyncStatus::Pending => Span::styled("[PEND]", Style::default().fg(Color::Yellow)),
        LinkSyncStatus::Paused => Span::styled("[PAUSED]", Style::default().fg(Color::DarkGray)),
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
                        Span::styled("Source:           ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.source_path)),
                    ]));
                }

                lines.push(Line::from(vec![
                    Span::styled("Active Consumers: ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{count} projects linked"),
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
                            LinkSyncStatus::Synced => "Synced",
                            LinkSyncStatus::Syncing => "Syncing...",
                            LinkSyncStatus::Error => "Error",
                            LinkSyncStatus::Pending => "Pending",
                            LinkSyncStatus::Paused => "Paused",
                        };
                        lines.push(Line::from(vec![
                            Span::styled("  • ", Style::default().fg(Color::DarkGray)),
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
                    Span::styled("Total Synced:     ", Style::default().fg(Color::Cyan)),
                    Span::raw(format!("{total_files} files across {count} consumers")),
                ]));

                lines.push(Line::from(vec![
                    Span::styled("Actions:          ", Style::default().fg(Color::Cyan)),
                    Span::styled("[r]", Style::default().fg(Color::Yellow)),
                    Span::raw(" Reconcile All  "),
                    Span::styled("[Space]", Style::default().fg(Color::Green)),
                    Span::raw(" Toggle Expand  "),
                    Span::styled("[u]", Style::default().fg(Color::Red)),
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
                    Span::styled("Linked Packages:  ", Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!("{count} packages"),
                        Style::default().fg(Color::Green),
                    ),
                ]));

                for &idx in link_indices {
                    if let Some(link) = state.links.get(idx) {
                        lines.push(Line::from(vec![
                            Span::styled("  • ", Style::default().fg(Color::DarkGray)),
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
                    Span::styled("Actions:          ", Style::default().fg(Color::Cyan)),
                    Span::styled("[r]", Style::default().fg(Color::Yellow)),
                    Span::raw(" Reconcile All  "),
                    Span::styled("[Space]", Style::default().fg(Color::Green)),
                    Span::raw(" Toggle Expand"),
                ]));
            }
            VisualTreeItem::LinkItem { link_index, .. } => {
                if let Some(link) = state.links.get(*link_index) {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "Package:  ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(&link.package_name),
                        Span::raw(" "),
                        format_eco_badge(link.ecosystem),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Source:   ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.source_path)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Consumer: ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.consumer_root)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Target:   ", Style::default().fg(Color::Cyan)),
                        Span::raw(display_path(&link.sync_target)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Strategy: ", Style::default().fg(Color::Cyan)),
                        Span::raw(format!("{:?}", link.strategy)),
                        Span::raw("  · Mode: "),
                        Span::raw(format!("{:?}", link.link_mode)),
                        Span::raw("  · Files: "),
                        Span::raw(format!("{}", link.file_count)),
                    ]));
                    lines.push(Line::from(vec![
                        Span::styled("Status:   ", Style::default().fg(Color::Cyan)),
                        format_status_badge(link.last_sync_status),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:?}", link.last_sync_status),
                            Style::default().fg(Color::White),
                        ),
                    ]));
                    if let Some(at) = link.last_sync_at {
                        lines.push(Line::from(vec![
                            Span::styled("Last Sync:", Style::default().fg(Color::Cyan)),
                            Span::raw(format!(" {}", at.format("%Y-%m-%d %H:%M:%S UTC"))),
                        ]));
                    }
                }
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Select a link or package group to inspect details",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Details & Health ")
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

    let title = format!(" Live Sync & Engine Logs ({}) ", state.logs.len());
    let mut lines = Vec::new();

    let visible_height = area.height.saturating_sub(2) as usize;
    let start_idx = if state.logs.len() > visible_height {
        if state.focus_logs {
            state
                .log_scroll
                .min(state.logs.len().saturating_sub(visible_height))
        } else {
            state.logs.len() - visible_height
        }
    } else {
        0
    };

    for item in state.logs.iter().skip(start_idx).take(visible_height) {
        let level_style = match item.level.as_str() {
            "ERR" | "ERROR" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            "DONE" => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            "SYNC" => Style::default().fg(Color::Yellow),
            "HINT" => Style::default().fg(Color::Magenta),
            "CMD" => Style::default().fg(Color::Cyan),
            _ => Style::default().fg(Color::DarkGray),
        };

        let mut spans = vec![
            Span::styled(
                format!("{} ", item.time),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(format!("[{}]", item.level), level_style),
        ];

        if let Some(eco) = &item.ecosystem {
            spans.push(Span::styled(
                format!(" [{eco}]"),
                Style::default().fg(Color::Blue),
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
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );

    frame.render_widget(p, area);
}

fn render_footer(frame: &mut ratatui::Frame, area: Rect, _state: &MonitorState) {
    let keybinds = vec![
        Span::styled("[↑/↓/j/k]", Style::default().fg(Color::Cyan)),
        Span::raw(" Nav  "),
        Span::styled("[g]", Style::default().fg(Color::Magenta)),
        Span::raw(" Group View  "),
        Span::styled("[Space/↵]", Style::default().fg(Color::Green)),
        Span::raw(" Expand/Pause  "),
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw(" Reconcile  "),
        Span::styled("[u]", Style::default().fg(Color::Red)),
        Span::raw(" Unlink  "),
        Span::styled("[c]", Style::default().fg(Color::Magenta)),
        Span::raw(" Clear Logs  "),
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Focus  "),
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
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(p, area);
}
