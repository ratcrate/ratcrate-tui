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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use serde::Deserialize;
use std::{error::Error, io, path::PathBuf, process::Command};
use tokio::sync::mpsc;
use tempfile::TempDir;

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

    // Cleanup temp installations before exit
    app.cleanup_temp_installs();

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

struct TempInstall {
    crate_name: String,
    temp_dir: TempDir,
    cargo_home: PathBuf,
    binary_path: PathBuf,
}

struct App {
    items: Vec<Crate>,
    filtered_items: Vec<Crate>,
    selected: usize,
    list_state: ListState,
    cmd_buffer: String,
    cmd_mode: bool,
    search_mode: bool,
    search_buffer: String,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::UnboundedReceiver<FetchResult>>,
    show_welcome: bool,
    temp_installs: Vec<TempInstall>,
    status_message: Option<String>,
}

type FetchResult = Result<Vec<Crate>, String>;

impl App {
    fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        let mut app = Self {
            items: vec![],
            filtered_items: vec![],
            selected: 0,
            list_state,
            cmd_buffer: String::new(),
            cmd_mode: false,
            search_mode: false,
            search_buffer: String::new(),
            loading: false,
            error: None,
            rx: None,
            show_welcome: true,
            temp_installs: Vec::new(),
            status_message: None,
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

    fn current_items(&self) -> &Vec<Crate> {
        if self.search_buffer.is_empty() {
            &self.items
        } else {
            &self.filtered_items
        }
    }

    fn update_filtered_items(&mut self) {
        if self.search_buffer.is_empty() {
            self.filtered_items = self.items.clone();
        } else {
            let search_lower = self.search_buffer.to_lowercase();
            self.filtered_items = self.items.iter()
                .filter(|crate_item| {
                    crate_item.name.to_lowercase().contains(&search_lower) ||
                    crate_item.description.as_ref()
                        .map(|desc| desc.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();
        }
        
        // Reset selection to first item
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    fn next(&mut self) {
        let items = self.current_items();
        if !items.is_empty() {
            self.selected = (self.selected + 1) % items.len();
            self.list_state.select(Some(self.selected));
        }
    }

    fn prev(&mut self) {
        let items = self.current_items();
        if !items.is_empty() {
            self.selected = if self.selected == 0 {
                items.len() - 1
            } else {
                self.selected - 1
            };
            self.list_state.select(Some(self.selected));
        }
    }

    fn enter_cmd_mode(&mut self) {
        self.cmd_mode = true;
        self.search_mode = false;
        self.cmd_buffer.clear();
    }

    fn exit_cmd_mode(&mut self) {
        self.cmd_mode = false;
        self.cmd_buffer.clear();
    }

    fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.cmd_mode = false;
    }

    fn exit_search_mode(&mut self) {
        self.search_mode = false;
        self.search_buffer.clear();
        self.update_filtered_items();
    }

    fn push_char(&mut self, c: char) {
        if self.search_mode {
            self.search_buffer.push(c);
            self.update_filtered_items();
        } else {
            self.cmd_buffer.push(c);
        }
    }

    fn pop_char(&mut self) {
        if self.search_mode {
            self.search_buffer.pop();
            self.update_filtered_items();
        } else {
            self.cmd_buffer.pop();
        }
    }

    fn show_help(&mut self) {
        self.show_welcome = true;
    }

    fn try_install_crate(&mut self) {
        let items = self.current_items();
        if items.is_empty() {
            let message = "No crates available".to_string();
            self.status_message = Some(message);
            return;
        }

        let selected_crate = &items[self.selected];
        let crate_name = selected_crate.name.clone(); // Clone to avoid borrowing issues

        // Check if already installed temporarily
        if self.temp_installs.iter().any(|install| install.crate_name == crate_name) {
            let message = format!("{} is already installed temporarily", crate_name);
            self.status_message = Some(message);
            return;
        }

        let installing_message = format!("Installing {} temporarily...", crate_name);
        self.status_message = Some(installing_message);
        
        match self.install_crate_temp(&crate_name) {
            Ok(temp_install) => {
                // Create the success message before moving temp_install
                let success_message = format!(
                    "{} installed temporarily! Run with: {}",
                    crate_name, temp_install.binary_path.display()
                );
                
                // Now move temp_install into the vector
                self.temp_installs.push(temp_install);
                
                // Set the status message
                self.status_message = Some(success_message);
            }
            Err(err) => {
                let error_message = format!("Failed to install {}: {}", crate_name, err);
                self.status_message = Some(error_message);
            }
        }
    }

    fn install_crate_temp(&self, crate_name: &str) -> Result<TempInstall, String> {
        // Create temporary directory
        let temp_dir = TempDir::new()
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;
        
        let cargo_home = temp_dir.path().join("cargo_home");
        let cargo_bin = cargo_home.join("bin");
        
        // Create cargo home directory structure
        std::fs::create_dir_all(&cargo_bin)
            .map_err(|e| format!("Failed to create cargo directories: {}", e))?;

        // Install the crate with custom CARGO_HOME
        let output = Command::new("cargo")
            .args(&["install", crate_name])
            .env("CARGO_HOME", &cargo_home)
            .output()
            .map_err(|e| format!("Failed to execute cargo install: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Cargo install failed: {}", stderr));
        }

        // Find the installed binary
        let binary_path = cargo_bin.join(crate_name);
        
        // Check if binary exists, if not try common variations
        let actual_binary_path = if binary_path.exists() {
            binary_path
        } else {
            // Sometimes the binary name differs from crate name
            std::fs::read_dir(&cargo_bin)
                .map_err(|e| format!("Failed to read bin directory: {}", e))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .find(|path| path.is_file())
                .ok_or_else(|| "No binary found in installation".to_string())?
        };

        Ok(TempInstall {
            crate_name: crate_name.to_string(),
            temp_dir,
            cargo_home,
            binary_path: actual_binary_path,
        })
    }

    fn install_crate_permanent(&mut self) {
        let items = self.current_items();
        if items.is_empty() {
            let message = "No crates available".to_string();
            self.status_message = Some(message);
            return;
        }

        let selected_crate = &items[self.selected];
        let crate_name = selected_crate.name.clone(); // Clone to avoid borrowing issues

        let installing_message = format!("Installing {} permanently...", crate_name);
        self.status_message = Some(installing_message);

        let output = Command::new("cargo")
            .args(&["install", &crate_name])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let success_message = format!("{} installed permanently!", crate_name);
                    self.status_message = Some(success_message);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let error_message = format!("Failed to install {}: {}", crate_name, stderr);
                    self.status_message = Some(error_message);
                }
            }
            Err(err) => {
                let error_message = format!("Failed to execute cargo install: {}", err);
                self.status_message = Some(error_message);
            }
        }
    }

    fn run_temp_crate(&mut self) {
        let items = self.current_items();
        if items.is_empty() {
            let message = "No crates available".to_string();
            self.status_message = Some(message);
            return;
        }

        let selected_crate = &items[self.selected];
        let crate_name = selected_crate.name.clone(); // Clone to avoid borrowing issues

        // Find the temp installation
        if let Some(install) = self.temp_installs.iter().find(|i| i.crate_name == crate_name) {
            let running_message = format!("Running {}...", crate_name);
            self.status_message = Some(running_message);
            
            // Store the binary path to avoid borrowing issues
            let binary_path = install.binary_path.clone();
            
            // Exit the TUI temporarily to run the program
            let _ = disable_raw_mode();
            let _ = execute!(
                io::stdout(),
                LeaveAlternateScreen,
                cursor::Show
            );

            // Run the binary
            let status = Command::new(&binary_path)
                .status();

            // Restore the TUI
            let _ = enable_raw_mode();
            let _ = execute!(io::stdout(), EnterAlternateScreen);

            match status {
                Ok(exit_status) => {
                    let result_message = if exit_status.success() {
                        format!("{} executed successfully", crate_name)
                    } else {
                        format!("{} exited with error", crate_name)
                    };
                    self.status_message = Some(result_message);
                }
                Err(err) => {
                    let error_message = format!("Failed to run {}: {}", crate_name, err);
                    self.status_message = Some(error_message);
                }
            }
        } else {
            let message = format!("{} is not installed temporarily. Use :try first", crate_name);
            self.status_message = Some(message);
        }
    }

    fn cleanup_temp_installs(&mut self) {
        // TempDir automatically cleans up when dropped
        self.temp_installs.clear();
    }

    fn list_temp_installs(&self) -> String {
        if self.temp_installs.is_empty() {
            "No temporary installations".to_string()
        } else {
            let mut list = "Temporary installations:\n".to_string();
            for install in &self.temp_installs {
                list.push_str(&format!("• {} ({})\n", install.crate_name, install.binary_path.display()));
            }
            list
        }
    }

    fn check_fetch_result(&mut self) {
        if let Some(rx) = &mut self.rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(crates) => {
                        self.items = crates;
                        self.update_filtered_items();
                        if self.items.is_empty() {
                            self.error = Some("No crates found".to_string());
                        } else {
                            self.error = None;
                        }
                        self.selected = 0;
                        self.list_state.select(Some(0));
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
        KeyCode::Char('/') => {
            app.enter_search_mode();
            app.show_welcome = false;
        }
        KeyCode::Esc => {
            if app.search_mode {
                app.exit_search_mode();
            }
        }
        KeyCode::Backspace => {
            if app.search_mode {
                app.pop_char();
            }
        }
        KeyCode::Char(c) => {
            if app.search_mode {
                app.push_char(c);
            }
        }
        _ => {}
    }
}

fn handle_cmd_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.exit_cmd_mode(),
        KeyCode::Enter => {
            let cmd = app.cmd_buffer.trim();
            match cmd {
                "pkg list" => {
                    app.spawn_fetch();
                    app.status_message = Some("Refreshing package list...".to_string());
                }
                "try" => {
                    app.try_install_crate();
                }
                "run" => {
                    app.run_temp_crate();
                }
                "install" => {
                    app.install_crate_permanent();
                }
                "temp" => {
                    app.status_message = Some(app.list_temp_installs());
                }
                "q" => {
                    // Cleanup before exit
                    app.cleanup_temp_installs();
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
                "help" | "h" => {
                    app.show_help();
                    app.status_message = None;
                }
                _ => {
                    app.status_message = Some(format!("Unknown command: {}", cmd));
                }
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
        .current_items()
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut style = if i == app.selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            // Highlight temporarily installed crates
            if app.temp_installs.iter().any(|install| install.crate_name == k.name) {
                style = style.fg(Color::Green);
            }

            ListItem::new(k.name.as_str()).style(style)
        })
        .collect();

    let list_title = if app.search_mode && !app.search_buffer.is_empty() {
        format!("Packages ({})", app.current_items().len())
    } else {
        "Packages".to_string()
    };
    
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(list_title));
    
    // Use stateful rendering for proper scrolling
    f.render_stateful_widget(list, top[0], &mut app.list_state.clone());

    let detail_text = if app.show_welcome {
        "🦀 Rust Crate Explorer - Safe Edition\n\n\
        Welcome to the Safe Rust Crate Explorer! This tool allows you to\n\
        search, browse, and install crates from crates.io safely.\n\n\
        📋 COMMANDS:\n\
        • :pkg list    - Refresh the package list\n\
        • :try         - Install selected crate temporarily\n\
        • :run         - Run temporarily installed crate\n\
        • :install     - Install selected crate permanently\n\
        • :temp        - List temporary installations\n\
        • :help / :h   - Show this help\n\
        • :q           - Quit (cleans up temp installs)\n\n\
        🔤 NAVIGATION:\n\
        • j / ↓        - Move down in the list\n\
        • k / ↑        - Move up in the list\n\
        • :            - Enter command mode\n\
        • /            - Enter search mode (real-time filter)\n\
        • ESC          - Exit command/search mode\n\n\
        📦 FEATURES:\n\
        • Try crates without permanent installation\n\
        • Run temporary installations directly\n\
        • Real-time search through crate names and descriptions\n\
        • No unsafe code - fully safe Rust\n\
        • Automatic cleanup on exit\n\
        • Green highlighting for temp-installed crates\n\n\
        Press any navigation key to start exploring!"
    } else if let Some(status) = &app.status_message {
        status.as_str()
    } else if let Some(err) = &app.error {
        err.as_str()
    } else if app.current_items().is_empty() {
        if app.loading {
            "Loading…"
        } else if !app.search_buffer.is_empty() {
            "No packages match your search"
        } else {
            "No packages"
        }
    } else {
        let selected_crate = &app.current_items()[app.selected];
        let is_temp_installed = app.temp_installs.iter().any(|install| install.crate_name == selected_crate.name);
        let temp_status = if is_temp_installed { " [TEMP INSTALLED]" } else { "" };
        
        &format!("{} ({}){}\n\n{}", 
            selected_crate.name, 
            selected_crate.max_version,
            temp_status,
            selected_crate.description.as_deref().unwrap_or("No description available")
        )
    };
    let details = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title("Details"));
    f.render_widget(details, top[1]);

    let prompt = if app.cmd_mode {
        format!(":{}", app.cmd_buffer)
    } else if app.search_mode {
        format!("Search: {}", app.search_buffer)
    } else {
        "Commands: :try, :run, :install, :temp, :pkg list, :help, :q | Navigate: j/k or ↓/↑ | Search: /".to_string()
    };
    let prompt_widget = Paragraph::new(prompt);
    f.render_widget(prompt_widget, chunks[1]);
} 