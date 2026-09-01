use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use linkd_core::{LinkEntry, LinkSyncStatus};
use linkd_ipc::{DaemonEvent, IpcClient};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct LogItem {
    pub time: String,
    pub level: String,
    pub ecosystem: Option<String>,
    pub message: String,
}

pub struct MonitorState {
    pub daemon_pid: Option<u32>,
    pub pm_hint: Option<String>,
    pub links: Vec<LinkEntry>,
    pub selected_index: usize,
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
            selected_index: 0,
            logs: Vec::new(),
            focus_logs: false,
            log_scroll: 0,
            started_at: Instant::now(),
            status_message: None,
        }
    }

    pub fn selected_link(&self) -> Option<&LinkEntry> {
        self.links.get(self.selected_index)
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

        terminal.draw(|frame| {
            render_ui(frame, state);
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
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.focus_logs {
                                    state.log_scroll = state.log_scroll.saturating_sub(1);
                                } else if state.selected_index > 0 {
                                    state.selected_index -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.focus_logs {
                                    state.log_scroll = (state.log_scroll + 1).min(state.logs.len().saturating_sub(1));
                                } else if !state.links.is_empty() && state.selected_index + 1 < state.links.len() {
                                    state.selected_index += 1;
                                }
                            }
                            KeyCode::Char('r') => {
                                if let Some(link) = state.selected_link() {
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
                            KeyCode::Char(' ') => {
                                if let Some(link) = state.selected_link() {
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
                                if let Some(link) = state.selected_link() {
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
                                    if let Ok(updated_links) = client.list_links().await {
                                        state.links = updated_links;
                                        if state.selected_index >= state.links.len() && !state.links.is_empty() {
                                            state.selected_index = state.links.len() - 1;
                                        }
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
            if state.selected_index >= state.links.len() && !state.links.is_empty() {
                state.selected_index = state.links.len() - 1;
            }
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
            }
            state.add_log(
                "SYNC",
                None,
                format!("{package_name} sync started ({files_count} files)"),
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
                link.last_sync_at = Some(chrono::Utc::now());
                link.file_count = files_synced as u32;
            }
            state.add_log(
                "DONE",
                None,
                format!("{package_name} synced {files_synced} files in {duration_ms}ms"),
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
            state.add_log("ERR", None, format!("{package_name} sync error: {error}"));
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

fn render_ui(frame: &mut ratatui::Frame, state: &MonitorState) {
    let size = frame.area();
    let root_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Main body
            Constraint::Length(3), // Footer
        ])
        .split(size);

    render_header(frame, root_layout[0], state);
    render_body(frame, root_layout[1], state);
    render_footer(frame, root_layout[2], state);
}

fn render_header(frame: &mut ratatui::Frame, area: Rect, state: &MonitorState) {
    let pid_text = state
        .daemon_pid
        .map(|p| format!("PID {p}"))
        .unwrap_or_else(|| "Running".into());
    let uptime = state.uptime_str();

    let mut header_spans = vec![
        Span::styled(
            " linkd live monitor ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("─ [Daemon: "),
        Span::styled(
            "Active",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" · {pid_text} · Up {uptime}] ")),
    ];

    if let Some(hint) = &state.pm_hint {
        header_spans.push(Span::styled(
            format!(" [PM: {hint}] "),
            Style::default().fg(Color::Yellow),
        ));
    }

    if let Some((status, _)) = &state.status_message {
        header_spans.push(Span::styled(
            format!(" [{status}] "),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let header_p = Paragraph::new(Line::from(header_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Status ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(header_p, area);
}

fn render_body(frame: &mut ratatui::Frame, area: Rect, state: &MonitorState) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45), // Left: links + inspector
            Constraint::Percentage(55), // Right: live logs
        ])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(55), // Links list
            Constraint::Percentage(45), // Details / Inspector
        ])
        .split(columns[0]);

    render_links_list(frame, left_chunks[0], state);
    render_link_inspector(frame, left_chunks[1], state);
    render_logs_panel(frame, columns[1], state);
}

fn render_links_list(frame: &mut ratatui::Frame, area: Rect, state: &MonitorState) {
    let border_color = if !state.focus_logs {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let title = format!(" Active Links ({}) ", state.links.len());
    let mut lines = Vec::new();

    if state.links.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active links. Run `linkd link` to add a package.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, link) in state.links.iter().enumerate() {
            let is_selected = i == state.selected_index;
            let prefix = if is_selected { "> " } else { "  " };

            let eco_badge = match link.ecosystem {
                linkd_core::Ecosystem::Npm => {
                    Span::styled("[npm]", Style::default().fg(Color::Red))
                }
                linkd_core::Ecosystem::Composer => {
                    Span::styled("[composer]", Style::default().fg(Color::Magenta))
                }
                linkd_core::Ecosystem::Python => {
                    Span::styled("[python]", Style::default().fg(Color::Yellow))
                }
                linkd_core::Ecosystem::Go => Span::styled("[go]", Style::default().fg(Color::Cyan)),
                linkd_core::Ecosystem::Cargo => {
                    Span::styled("[cargo]", Style::default().fg(Color::LightRed))
                }
                linkd_core::Ecosystem::Jvm => {
                    Span::styled("[jvm]", Style::default().fg(Color::LightGreen))
                }
                linkd_core::Ecosystem::Dart => {
                    Span::styled("[dart/flutter]", Style::default().fg(Color::LightCyan))
                }
                linkd_core::Ecosystem::Custom => {
                    Span::styled("[custom]", Style::default().fg(Color::Blue))
                }
            };

            let status_badge = match link.last_sync_status {
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
                LinkSyncStatus::Pending => {
                    Span::styled("[PEND]", Style::default().fg(Color::Yellow))
                }
                LinkSyncStatus::Paused => {
                    Span::styled("[PAUSED]", Style::default().fg(Color::DarkGray))
                }
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            lines.push(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default().fg(if is_selected {
                        Color::Cyan
                    } else {
                        Color::DarkGray
                    }),
                ),
                Span::styled(&link.package_name, name_style),
                Span::raw(" "),
                eco_badge,
                Span::raw(" "),
                status_badge,
            ]));
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

fn render_link_inspector(frame: &mut ratatui::Frame, area: Rect, state: &MonitorState) {
    let mut lines = Vec::new();

    if let Some(link) = state.selected_link() {
        lines.push(Line::from(vec![
            Span::styled(
                "Package:  ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&link.package_name),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Source:   ", Style::default().fg(Color::Cyan)),
            Span::raw(link.source_path.display().to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Target:   ", Style::default().fg(Color::Cyan)),
            Span::raw(link.sync_target.display().to_string()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Strategy: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:?}", link.strategy)),
            Span::raw("  · Mode: "),
            Span::raw(format!("{:?}", link.link_mode)),
            Span::raw("  · Isolation: "),
            Span::raw(format!("{:?}", link.isolation_mode)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Files:    ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", link.file_count)),
            Span::raw("  · Status: "),
            Span::raw(format!("{:?}", link.last_sync_status)),
        ]));
        if let Some(at) = link.last_sync_at {
            lines.push(Line::from(vec![
                Span::styled("Last Sync:", Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {}", at.format("%Y-%m-%d %H:%M:%S UTC"))),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Select a link to inspect details",
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
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw(" Reconcile  "),
        Span::styled("[Space]", Style::default().fg(Color::Green)),
        Span::raw(" Pause/Resume  "),
        Span::styled("[u]", Style::default().fg(Color::Red)),
        Span::raw(" Unlink  "),
        Span::styled("[c]", Style::default().fg(Color::Magenta)),
        Span::raw(" Clear  "),
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
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
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(p, area);
}
