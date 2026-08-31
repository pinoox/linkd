use std::io;
use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use linkd_adapters::{resolve_link, validate_link_paths};
use linkd_core::Ecosystem;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::DefaultTerminal;

#[derive(Debug, Clone)]
pub struct WizardResult {
    pub source: PathBuf,
    pub consumer: PathBuf,
    pub target: Option<PathBuf>,
    pub ecosystem: Ecosystem,
    pub start_daemon: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    LinkType,
    Source,
    Consumer,
    Target,
    Confirm,
}

struct WizardState {
    step: Step,
    link_type: usize,
    source: String,
    consumer: String,
    target: String,
    start_daemon: bool,
    error: Option<String>,
}

impl WizardState {
    fn new() -> Self {
        Self {
            step: Step::LinkType,
            link_type: 0,
            source: "./packages/my-lib".into(),
            consumer: "../my-app".into(),
            target: "./lib/shared".into(),
            start_daemon: true,
            error: None,
        }
    }

    fn step_index(&self) -> usize {
        match self.step {
            Step::LinkType => 1,
            Step::Source => 2,
            Step::Consumer => 3,
            Step::Target => 4,
            Step::Confirm => 5,
        }
    }

    fn active_field(&self) -> &str {
        match self.step {
            Step::Source => &self.source,
            Step::Consumer => &self.consumer,
            Step::Target => &self.target,
            _ => "",
        }
    }

    fn active_field_mut(&mut self) -> &mut String {
        match self.step {
            Step::Source => &mut self.source,
            Step::Consumer => &mut self.consumer,
            Step::Target => &mut self.target,
            _ => &mut self.source,
        }
    }

    fn next(&mut self) {
        self.error = None;
        self.step = match self.step {
            Step::LinkType => Step::Source,
            Step::Source => Step::Consumer,
            Step::Consumer if self.link_type == 2 => Step::Target,
            Step::Consumer => Step::Confirm,
            Step::Target => Step::Confirm,
            Step::Confirm => Step::Confirm,
        };
    }

    fn back(&mut self) {
        self.error = None;
        self.step = match self.step {
            Step::LinkType => Step::LinkType,
            Step::Source => Step::LinkType,
            Step::Consumer => Step::Source,
            Step::Target => Step::Consumer,
            Step::Confirm if self.link_type == 2 => Step::Target,
            Step::Confirm => Step::Consumer,
        };
    }

    fn ecosystem(&self) -> Ecosystem {
        match self.link_type {
            1 => Ecosystem::Composer,
            2 => Ecosystem::Custom,
            _ => Ecosystem::Npm,
        }
    }

    fn try_finish(&self) -> Result<WizardResult, String> {
        let source = PathBuf::from(&self.source);
        let consumer = PathBuf::from(&self.consumer);
        let target = if self.link_type == 2 {
            Some(PathBuf::from(&self.target))
        } else {
            None
        };

        let eco = self.ecosystem();

        resolve_link(&source, &consumer, Some(eco), target.as_deref())
            .map_err(|e| e.to_string())?;

        if let Some(ref t) = target {
            validate_link_paths(&source, t).map_err(|e| e.to_string())?;
        }

        Ok(WizardResult {
            source,
            consumer,
            target,
            ecosystem: eco,
            start_daemon: self.start_daemon,
        })
    }
}

pub fn run_wizard_ui() -> io::Result<Option<WizardResult>> {
    let mut terminal = ratatui::init();
    let mut state = WizardState::new();
    let result = run_loop(&mut terminal, &mut state);
    ratatui::restore();
    result
}

fn run_loop(
    terminal: &mut DefaultTerminal,
    state: &mut WizardState,
) -> io::Result<Option<WizardResult>> {
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(8), Constraint::Length(2)])
                .split(frame.area());

            let title = format!(" linkd wizard — step {}/5 ", state.step_index());
            let mut lines = vec![Line::from(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            ))];

            match state.step {
                Step::LinkType => {
                    for (i, label) in ["npm package", "composer package", "custom path"]
                        .iter()
                        .enumerate()
                    {
                        let style = if i == state.link_type {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default()
                        };
                        let prefix = if i == state.link_type { "> " } else { "  " };
                        lines.push(Line::from(Span::styled(format!("{prefix}{label}"), style)));
                    }
                    lines.push(Line::from("↑/↓ select · Enter next · Esc cancel"));
                }
                Step::Source | Step::Consumer | Step::Target => {
                    let label = match state.step {
                        Step::Source => "Source",
                        Step::Consumer => "Consumer",
                        Step::Target => "Target",
                        _ => "",
                    };
                    lines.push(Line::from(format!("{label}: {}", state.active_field())));
                    lines.push(Line::from(
                        "Type to edit · Enter next · Backspace delete · Alt+Left back · Esc cancel",
                    ));
                }
                Step::Confirm => {
                    lines.push(Line::from(format!("Source:   {}", state.source)));
                    lines.push(Line::from(format!("Consumer: {}", state.consumer)));
                    if state.link_type == 2 {
                        lines.push(Line::from(format!("Target:   {}", state.target)));
                    }
                    lines.push(Line::from(format!(
                        "Start daemon: {}",
                        if state.start_daemon { "yes" } else { "no" }
                    )));
                    lines.push(Line::from("Enter confirm · d toggle daemon · Esc cancel"));
                }
            }

            if let Some(err) = &state.error {
                lines.push(Line::from(Span::styled(
                    format!("Error: {err}"),
                    Style::default().fg(Color::Red),
                )));
            }

            let block = Block::default()
                .borders(Borders::ALL)
                .title(" linkd wizard ");
            frame.render_widget(Paragraph::new(lines).block(block), chunks[0]);
            frame.render_widget(Paragraph::new(" Ctrl+C cancel · Left back "), chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(None);
                }

                match state.step {
                    Step::LinkType => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.link_type = state.link_type.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.link_type = (state.link_type + 1).min(2);
                        }
                        KeyCode::Enter => state.next(),
                        KeyCode::Esc => return Ok(None),
                        _ => {}
                    },
                    Step::Confirm => match key.code {
                        KeyCode::Enter => match state.try_finish() {
                            Ok(r) => return Ok(Some(r)),
                            Err(e) => state.error = Some(e),
                        },
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Left | KeyCode::Char('h') => state.back(),
                        KeyCode::Char('d') => state.start_daemon = !state.start_daemon,
                        _ => {}
                    },
                    Step::Source | Step::Consumer | Step::Target => match key.code {
                        KeyCode::Enter => state.next(),
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Backspace => {
                            state.active_field_mut().pop();
                        }
                        KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => state.back(),
                        KeyCode::Char(c) => {
                            state.active_field_mut().push(c);
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}
