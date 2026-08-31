use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use linkd_registry::RegistryStore;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::DefaultTerminal;

pub fn run_tui() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal);
    ratatui::restore();
    result
}

fn run_loop(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let started = Instant::now();

    loop {
        let store = RegistryStore::default();
        let links = store.load().map(|r| r.links).unwrap_or_default();

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)])
                .split(frame.area());

            let mut lines = vec![Line::from(Span::styled(
                " linkd ",
                Style::default().add_modifier(Modifier::BOLD),
            ))];

            if links.is_empty() {
                lines.push(Line::from(" No links yet. Use: linkd link <source> [consumer]"));
            } else {
                for link in &links {
                    let status = format!("{:?}", link.last_sync_status);
                    let style = if status.contains("Synced") {
                        Style::default().fg(Color::Green)
                    } else if status.contains("Syncing") {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(vec![
                        Span::raw(" 🔗 "),
                        Span::styled(format!("{}", link.package_name), style),
                        Span::raw(format!(
                            " → {} ({:?}) ",
                            link.consumer_root.display(),
                            link.strategy
                        )),
                        Span::styled(status, style),
                    ]));
                    if let Some(at) = link.last_sync_at {
                        lines.push(Line::from(format!(
                            "    last sync: {} · {} files",
                            at.format("%H:%M:%S"),
                            link.file_count
                        )));
                    }
                }
            }

            let block = Block::default().borders(Borders::ALL).title(" linkd ");
            let paragraph = Paragraph::new(lines).block(block);
            frame.render_widget(paragraph, chunks[0]);

            let footer = Paragraph::new(format!(
                " running {}s · Ctrl+C to exit ",
                started.elapsed().as_secs()
            ));
            frame.render_widget(footer, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }
            }
        }
    }

    Ok(())
}
