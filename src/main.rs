use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    cursor,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use serde::Deserialize;
use std::{error::Error, io};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear the screen immediately
    terminal.clear()?;

    let mut app = App::new();
    let res = run(&mut terminal, &mut app).await;

    // Proper cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    terminal.show_cursor()?;
    
    // Clear the screen one more time and reset cursor
    execute!(
        io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )?;

    if let Err(err) = res {
        println!("{err}");
    }
    Ok(())
}

#[derive(Deserialize)]
struct CratesApiResponse {
    crates: Vec<Crate>,
    meta: Meta,
}

#[derive(Deserialize)]
struct Meta {
    total: u32,
}

#[derive(Deserialize, Clone)]
struct Crate {
    name: String,
    #[serde(rename = "max_version")]
    max_version: String,
    description: Option<String>,
}

struct App {
    items: Vec<Crate>,
    selected: usize,
    cmd_buffer: String,
    cmd_mode: bool,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::UnboundedReceiver<FetchResult>>,
    show_welcome: bool,
}

type FetchResult = Result<Vec<Crate>, String>;

impl App {
    fn new() -> Self {
        let mut app = Self {
            items: vec![],
            selected: 0,
            cmd_buffer: String::new(),
            cmd_mode: false,
            loading: false,
            error: None,
            rx: None,
            show_welcome: true,
        };
        app.spawn_fetch();
        app
    }

    fn spawn_fetch(&mut self) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;
        
        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = Some(rx);
        
        tokio::spawn(async move {
            let result = fetch_ratatui_crates().await;
            let _ = tx.send(result);
        });
    }

    fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    fn prev(&mut self) {
        if !self.items.is_empty() {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    fn enter_cmd_mode(&mut self) {
        self.cmd_mode = true;
        self.cmd_buffer.clear();
    }

    fn exit_cmd_mode(&mut self) {
        self.cmd_mode = false;
        self.cmd_buffer.clear();
    }

    fn push_char(&mut self, c: char) {
        self.cmd_buffer.push(c);
    }

    fn show_help(&mut self) {
        self.show_welcome = true;
    }

    fn pop_char(&mut self) {
        self.cmd_buffer.pop();
    }

    fn check_fetch_result(&mut self) {
        if let Some(rx) = &mut self.rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(crates) => {
                        self.items = crates;
                        if self.items.is_empty() {
                            self.error = Some("No crates found".to_string());
                        } else {
                            self.error = None;
                        }
                        self.selected = 0;
                        self.show_welcome = false; // Hide welcome message when data loads
                    }
                    Err(err) => {
                        self.error = Some(err);
                        self.items.clear();
                        self.show_welcome = false; // Hide welcome message on error
                    }
                }
                self.loading = false;
                self.rx = None;
            }
        }
    }
}

async fn fetch_ratatui_crates() -> FetchResult {
    let url = "https://crates.io/api/v1/crates?q=ratatui&per_page=100";
    
    let client = reqwest::Client::builder()
        .user_agent("rust-tui-app/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let response_text = resp.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
            
            if !status.is_success() {
                return Err(format!("HTTP error {}: {}", status, response_text));
            }
            
            match serde_json::from_str::<CratesApiResponse>(&response_text) {
                Ok(data) => Ok(data.crates),
                Err(e) => Err(format!("Failed to parse JSON: {}. Response was: {}", e, response_text.chars().take(200).collect::<String>())),
            }
        },
        Err(e) => Err(format!("Failed to fetch data: {}", e)),
    }
}

async fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        // Check for async fetch results
        app.check_fetch_result();

        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.cmd_mode {
                handle_cmd_mode(app, key);
            } else {
                handle_normal_mode(app, key);
            }

            if key.code == KeyCode::Char('q') && !app.cmd_mode {
                // Remove the 'q' quit functionality since we now use ':q'
            }
        }
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.next();
            app.show_welcome = false; // Hide welcome on navigation
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.prev();
            app.show_welcome = false; // Hide welcome on navigation
        }
        KeyCode::Char(':') => app.enter_cmd_mode(),
        _ => {}
    }
}

fn handle_cmd_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.exit_cmd_mode(),
        KeyCode::Enter => {
            let cmd = app.cmd_buffer.trim();
            match cmd {
                "pkg list" => app.spawn_fetch(),
                "q" => {
                    // Proper cleanup before exit
                    let _ = disable_raw_mode();
                    let _ = execute!(
                        io::stdout(),
                        LeaveAlternateScreen,
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                        crossterm::cursor::MoveTo(0, 0),
                        cursor::Show
                    );
                    std::process::exit(0);
                }
                "help" | "h" => app.show_help(),
                _ => {} // TODO: unknown command feedback
            }
            app.exit_cmd_mode();
        }
        KeyCode::Backspace => app.pop_char(),
        KeyCode::Char(c) => app.push_char(c),
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    let items: Vec<_> = app
        .items
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let style = if i == app.selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(k.name.as_str()).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Packages"));
    f.render_widget(list, top[0]);

    let detail_text = if app.show_welcome {
        "🦀 Rust Crate Explorer - Ratatui Edition\n\n\
        Welcome to the Rust Crate Explorer! This tool allows you to search\n\
        and browse crates from crates.io that are related to 'ratatui'.\n\n\
        📋 COMMANDS:\n\
        • :pkg list    - Refresh the package list\n\
        • :q           - Quit the application\n\n\
        🔤 NAVIGATION:\n\
        • j / ↓        - Move down in the list\n\
        • k / ↑        - Move up in the list\n\
        • :            - Enter command mode\n\
        • ESC          - Exit command mode\n\n\
        📦 FEATURES:\n\
        • Browse ratatui-related crates\n\
        • View crate details and descriptions\n\
        • Real-time loading with async support\n\n\
        Press any navigation key to start exploring!"
    } else if let Some(err) = &app.error {
        err.as_str()
    } else if app.items.is_empty() {
        if app.loading {
            "Loading…"
        } else {
            "No packages"
        }
    } else {
        let selected_crate = &app.items[app.selected];
        &format!("{} ({})\n{}", 
            selected_crate.name, 
            selected_crate.max_version,
            selected_crate.description.as_deref().unwrap_or("No description available")
        )
    };
    let details = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(details, top[1]);

    let prompt = if app.cmd_mode {
        format!(":{}", app.cmd_buffer)
    } else {
        "Press ':' for commands | Available: :pkg list, :help, :h, :q".to_string()
    };
    let prompt_widget = Paragraph::new(prompt);
    f.render_widget(prompt_widget, chunks[1]);
}
