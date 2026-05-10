use std::collections::BTreeMap;
use std::io::{self, Stdout};
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Terminal;
use tokio::runtime::Runtime;
use uuid::Uuid;

use loom_client::DaemonClient;
use loom_types::api::{CreateSessionRequest, DispatchEventRequest, RunBindingRequest};
use loom_types::artifacts::{IntentPacket, TaskType};
use loom_types::ops::SessionEvent;
use loom_types::session::{SessionPhase, SessionSnapshot};

#[derive(Debug, Parser)]
#[command(name = "x07-studio-forge")]
#[command(version)]
#[command(about = "Terminal shell for x07 Studio.")]
struct Cli {
    #[arg(long, default_value = "http://127.0.0.1:7719")]
    daemon_url: String,
}

struct App {
    rt: Runtime,
    client: DaemonClient,
    sessions: Vec<SessionSnapshot>,
    selected: Option<Uuid>,
    status: String,
    should_quit: bool,
}

impl App {
    fn new(daemon_url: String) -> anyhow::Result<Self> {
        let rt = Runtime::new()?;
        let client = DaemonClient::new(daemon_url);
        let mut app = Self {
            rt,
            client,
            sessions: Vec::new(),
            selected: None,
            status: "Press R to refresh".to_string(),
            should_quit: false,
        };
        app.refresh();
        Ok(app)
    }

    fn refresh(&mut self) {
        match self.rt.block_on(self.client.list_sessions()) {
            Ok(sessions) => {
                self.sessions = sessions;
                if self.selected.is_none() {
                    self.selected = self.sessions.first().map(|session| session.session_id);
                }
                self.status = format!("Loaded {} sessions", self.sessions.len());
            }
            Err(error) => self.status = format!("refresh failed: {error}"),
        }
    }

    fn selected_session(&self) -> Option<&SessionSnapshot> {
        let id = self.selected?;
        self.sessions
            .iter()
            .find(|session| session.session_id == id)
    }

    fn replace_session(&mut self, snapshot: SessionSnapshot) {
        if let Some(slot) = self
            .sessions
            .iter_mut()
            .find(|session| session.session_id == snapshot.session_id)
        {
            *slot = snapshot;
        } else {
            self.sessions.push(snapshot);
        }
    }

    fn dispatch_selected(&mut self, event: SessionEvent) {
        let Some(session_id) = self.selected else {
            self.status = "no session selected".to_string();
            return;
        };
        match self.rt.block_on(
            self.client
                .dispatch_event(session_id, &DispatchEventRequest { event }),
        ) {
            Ok(snapshot) => {
                self.replace_session(snapshot);
                self.status = "event applied".to_string();
            }
            Err(error) => self.status = format!("event failed: {error}"),
        }
    }

    fn run_binding(&mut self, binding_id: &str) {
        let Some(session_id) = self.selected else {
            self.status = "no session selected".to_string();
            return;
        };
        match self.rt.block_on(self.client.run_binding(
            session_id,
            &RunBindingRequest {
                binding_id: binding_id.to_string(),
                vars: BTreeMap::new(),
            },
        )) {
            Ok(snapshot) => {
                self.replace_session(snapshot);
                self.status = format!("ran `{binding_id}`");
            }
            Err(error) => self.status = format!("binding failed: {error}"),
        }
    }

    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('R') => self.refresh(),
            KeyCode::Down => {
                if let Some(current) = self.selected {
                    if let Some(index) = self
                        .sessions
                        .iter()
                        .position(|session| session.session_id == current)
                    {
                        if index + 1 < self.sessions.len() {
                            self.selected = Some(self.sessions[index + 1].session_id);
                        }
                    }
                }
            }
            KeyCode::Up => {
                if let Some(current) = self.selected {
                    if let Some(index) = self
                        .sessions
                        .iter()
                        .position(|session| session.session_id == current)
                    {
                        if index > 0 {
                            self.selected = Some(self.sessions[index - 1].session_id);
                        }
                    }
                }
            }
            KeyCode::Char('n') => {
                match self
                    .rt
                    .block_on(self.client.create_session(&CreateSessionRequest {
                        title: "New session".to_string(),
                        task_type: TaskType::NewBehavior,
                    })) {
                    Ok(snapshot) => {
                        self.selected = Some(snapshot.session_id);
                        self.sessions.push(snapshot);
                        self.status = "new session created".to_string();
                    }
                    Err(error) => self.status = format!("create failed: {error}"),
                }
            }
            KeyCode::Char('i') => {
                if let Some(session_id) = self.selected {
                    let root = self
                        .selected_session()
                        .map(|session| session.root.clone())
                        .unwrap_or_else(|| ".".to_string());
                    self.dispatch_selected(SessionEvent::FormalizeIntent(Box::new(
                        IntentPacket::demo(session_id, root),
                    )));
                }
            }
            KeyCode::Char('s') => {
                let phase = self.selected_session().map(|session| session.phase.clone());
                match phase {
                    Some(SessionPhase::IntentReady) => {
                        self.dispatch_selected(SessionEvent::DraftSpec)
                    }
                    Some(SessionPhase::SpecDraft) | Some(SessionPhase::SpecReview) => {
                        self.dispatch_selected(SessionEvent::ApproveSpec)
                    }
                    _ => {}
                }
            }
            KeyCode::Char('r') => {
                let phase = self.selected_session().map(|session| session.phase.clone());
                match phase {
                    Some(SessionPhase::SpecApproved) => {
                        self.dispatch_selected(SessionEvent::ProposeRealization)
                    }
                    Some(SessionPhase::RealizationProposed) => {
                        self.dispatch_selected(SessionEvent::AcceptRealization)
                    }
                    Some(SessionPhase::RepairEligible) => {
                        self.dispatch_selected(SessionEvent::RepairSpecPreserving)
                    }
                    _ => {}
                }
            }
            KeyCode::Char('v') => self.run_binding("xtal.verify"),
            KeyCode::Char('f') => self.dispatch_selected(SessionEvent::VerificationFailed),
            KeyCode::Char('p') => self.dispatch_selected(SessionEvent::VerificationPassed),
            KeyCode::Char('t') => self.dispatch_selected(SessionEvent::ApproveTrust),
            KeyCode::Char('c') => self.run_binding("xtal.certify"),
            KeyCode::Char('x') => self.run_binding("xtal.repair"),
            KeyCode::Char('o') => self.dispatch_selected(SessionEvent::IngestIncident),
            _ => {}
        }
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let app = App::new(cli.daemon_url)?;
    let result = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>, mut app: App) -> anyhow::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key.code);
            }
        }
    }

    Ok(())
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new("x07 Studio Forge")
            .block(Block::default().borders(Borders::ALL).title("Header")),
        outer[0],
    );

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(42),
            Constraint::Percentage(30),
        ])
        .split(outer[1]);

    let sessions = app
        .sessions
        .iter()
        .map(|session| {
            let prefix = if app.selected == Some(session.session_id) {
                "▶ "
            } else {
                "  "
            };
            ListItem::new(format!("{prefix}{} · {:?}", session.title, session.phase))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(sessions).block(Block::default().borders(Borders::ALL).title("Sessions")),
        main[0],
    );

    let center_text = if let Some(session) = app.selected_session() {
        format!(
            "Phase: {:?}\nRoom: {:?}\nAllowed verbs:\n- {}\n\nIntent: {}\n\nRecent op: {}",
            session.phase,
            session.room,
            session
                .allowed_verbs
                .iter()
                .map(|verb| format!("{verb:?}"))
                .collect::<Vec<_>>()
                .join("\n- "),
            session
                .intent
                .as_ref()
                .and_then(|intent| serde_json::to_string_pretty(intent).ok())
                .unwrap_or_else(|| "none".to_string()),
            session
                .op_log
                .last()
                .map(|op| format!("{} · {:?}", op.op, op.status))
                .unwrap_or_else(|| "none".to_string())
        )
    } else {
        "No session selected".to_string()
    };

    frame.render_widget(
        Paragraph::new(center_text)
            .block(Block::default().borders(Borders::ALL).title("Session"))
            .wrap(Wrap { trim: false }),
        main[1],
    );

    let ops = app
        .selected_session()
        .map(|session| {
            session
                .op_log
                .iter()
                .rev()
                .map(|record| ListItem::new(format!("{} · {:?}", record.op, record.status)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    frame.render_widget(
        List::new(ops).block(Block::default().borders(Borders::ALL).title("Op log")),
        main[2],
    );

    frame.render_widget(
        Paragraph::new("q quit • R refresh • n new • i intent • s spec • r realize/repair • v verify • p pass • f fail • t trust • c certify • x repair • o ingest")
            .block(Block::default().borders(Borders::ALL).title(format!("Status: {}", app.status)))
            .wrap(Wrap { trim: true }),
        outer[2],
    );
}
