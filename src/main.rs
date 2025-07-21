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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap, Gauge},
    Frame, Terminal,
    text::{Span, Line, Text}, // Added Text for Paragraph widget
};
use serde::Deserialize;
use std::{error::Error, io, path::PathBuf, process::Command};
use tokio::sync::mpsc;
use tempfile::TempDir;
use chrono::{DateTime, Utc};

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
    let res = run(&mut terminal, &mut app).await; // 'run' now handles its own exit

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
    downloads: u64,
    recent_downloads: Option<u64>,
    created_at: String,
    updated_at: String,
    homepage: Option<String>,
    repository: Option<String>,
    documentation: Option<String>,
    #[serde(default)]
    yanked: bool,
    keywords: Option<Vec<String>>,
    categories: Option<Vec<String>>,
}

#[derive(Deserialize, Clone)]
struct GitHubRepo {
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    stargazers_count: u32,
    forks_count: u32,
    language: Option<String>,
    updated_at: String,
    topics: Vec<String>,
}

#[derive(Deserialize)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepo>,
    total_count: u32,
}

#[derive(Clone, PartialEq)]
struct TempInstall {
    crate_name: String,
    // temp_dir: TempDir, // TempDir does not implement Clone, so we cannot derive Clone for TempInstall directly if it contains TempDir
    temp_dir_path: PathBuf, // Store the path instead
    cargo_home: PathBuf,
    binary_path: PathBuf,
}

#[derive(Clone, PartialEq)]
enum InstallProgress {
    None,
    Starting,
    Downloading(u8), // percentage
    Compiling(u8),   // percentage
    Finished,
    Error(String),
    TempInstallSuccess(TempInstall), // Added to send the TempInstall object back
}

// New enum to represent the outcome of a command, for graceful exit
enum CommandOutcome {
    Continue,
    Quit,
}

struct App {
    items: Vec<Crate>,
    github_items: Vec<GitHubRepo>,
    filtered_items: Vec<Crate>,
    filtered_github_items: Vec<GitHubRepo>,
    selected: usize,
    list_state: ListState,
    cmd_buffer: String,
    cmd_mode: bool,
    search_mode: bool,
    search_buffer: String,
    loading: bool,
    error: Option<String>,
    rx: Option<mpsc::UnboundedReceiver<FetchResult>>,
    github_rx: Option<mpsc::UnboundedReceiver<GitHubFetchResult>>,
    install_rx: Option<mpsc::UnboundedReceiver<InstallProgress>>, // New channel for install progress
    show_welcome: bool,
    temp_installs: Vec<TempInstall>,
    status_message: Option<String>,
    install_progress: InstallProgress,
    view_mode: ViewMode, // crates.io or github
}

#[derive(Clone, PartialEq)]
enum ViewMode {
    Crates,
    GitHub,
}

type FetchResult = Result<Vec<Crate>, String>;
type GitHubFetchResult = Result<Vec<GitHubRepo>, String>;

impl App {
    fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut app = Self {
            items: vec![],
            github_items: vec![],
            filtered_items: vec![],
            filtered_github_items: vec![],
            selected: 0,
            list_state,
            cmd_buffer: String::new(),
            cmd_mode: false,
            search_mode: false,
            search_buffer: String::new(),
            loading: false,
            error: None,
            rx: None,
            github_rx: None,
            install_rx: None, // Initialize new install_rx
            show_welcome: true,
            temp_installs: Vec::new(),
            status_message: None,
            install_progress: InstallProgress::None,
            view_mode: ViewMode::Crates,
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

    fn spawn_github_fetch(&mut self) {
        if self.loading {
            return;
        }
        self.loading = true;
        self.error = None;

        let (tx, rx) = mpsc::unbounded_channel();
        self.github_rx = Some(rx);

        tokio::spawn(async move {
            let result = fetch_github_ratatui_repos().await;
            let _ = tx.send(result);
        });
    }

    fn current_items(&self) -> usize {
        match self.view_mode {
            ViewMode::Crates => {
                if self.search_buffer.is_empty() {
                    self.items.len()
                } else {
                    self.filtered_items.len()
                }
            }
            ViewMode::GitHub => {
                if self.search_buffer.is_empty() {
                    self.github_items.len()
                } else {
                    self.filtered_github_items.len()
                }
            }
        }
    }

    fn update_filtered_items(&mut self) {
        match self.view_mode {
            ViewMode::Crates => {
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
            }
            ViewMode::GitHub => {
                if self.search_buffer.is_empty() {
                    self.filtered_github_items = self.github_items.clone();
                } else {
                    let search_lower = self.search_buffer.to_lowercase();
                    self.filtered_github_items = self.github_items.iter()
                        .filter(|repo| {
                            repo.name.to_lowercase().contains(&search_lower) ||
                            repo.description.as_ref()
                                .map(|desc| desc.to_lowercase().contains(&search_lower))
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                }
            }
        }

        // Reset selection to first item
        self.selected = 0;
        self.list_state.select(Some(0));
    }

    fn next(&mut self) {
        let items_len = self.current_items();
        if items_len > 0 {
            self.selected = (self.selected + 1) % items_len;
            self.list_state.select(Some(self.selected));
        }
    }

    fn prev(&mut self) {
        let items_len = self.current_items();
        if items_len > 0 {
            self.selected = if self.selected == 0 {
                items_len - 1
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
        if self.view_mode != ViewMode::Crates {
            self.status_message = Some("Can only install crates from crates.io view".to_string());
            return;
        }

        let items = if self.search_buffer.is_empty() { &self.items } else { &self.filtered_items };
        if items.is_empty() {
            let message = "No crates available".to_string();
            self.status_message = Some(message);
            return;
        }

        let selected_crate = &items[self.selected];
        let crate_name = selected_crate.name.clone();

        // Check if already installed temporarily
        if self.temp_installs.iter().any(|install| install.crate_name == crate_name) {
            let message = format!("{} is already installed temporarily", crate_name);
            self.status_message = Some(message);
            return;
        }

        self.install_progress = InstallProgress::Starting;

        // Spawn async installation with progress updates
        let (tx_progress, rx_progress) = mpsc::unbounded_channel();
        self.install_rx = Some(rx_progress); // Set the receiver for install progress

        let crate_name_clone = crate_name.clone();

        tokio::spawn(async move {
            let _ = tx_progress.send(InstallProgress::Downloading(10));
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Actual temporary installation logic
            match App::install_crate_temp_internal(&crate_name_clone, tx_progress.clone()).await {
                Ok(temp_install) => {
                    let _ = tx_progress.send(InstallProgress::TempInstallSuccess(temp_install)); // Send the TempInstall object
                }
                Err(e) => {
                    let _ = tx_progress.send(InstallProgress::Error(e.clone()));
                }
            }
        });

        self.status_message = Some(format!("Installing {} temporarily...", crate_name));
    }

    // This function is now async and accepts a progress sender
    async fn install_crate_temp_internal(crate_name: &str, tx_progress: mpsc::UnboundedSender<InstallProgress>) -> Result<TempInstall, String> {
        let _ = tx_progress.send(InstallProgress::Downloading(25));
        let temp_dir = TempDir::new()
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;

        let temp_dir_path = temp_dir.path().to_path_buf(); // Store the path

        let cargo_home = temp_dir_path.join("cargo_home");
        let cargo_bin = cargo_home.join("bin");

        std::fs::create_dir_all(&cargo_bin)
            .map_err(|e| format!("Failed to create cargo directories: {}", e))?;

        let _ = tx_progress.send(InstallProgress::Downloading(75));

        let output = Command::new("cargo")
            .args(&["install", crate_name])
            .env("CARGO_HOME", &cargo_home)
            .output()
            .map_err(|e| format!("Failed to execute cargo install: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = tx_progress.send(InstallProgress::Error(format!("Cargo install failed: {}", stderr)));
            return Err(format!("Cargo install failed: {}", stderr));
        }

        let _ = tx_progress.send(InstallProgress::Compiling(50)); // Simulated compiling progress

        let binary_path = cargo_bin.join(crate_name);

        let actual_binary_path = if binary_path.exists() {
            binary_path
        } else {
            std::fs::read_dir(&cargo_bin)
                .map_err(|e| format!("Failed to read bin directory: {}", e))?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .find(|path| path.is_file())
                .ok_or_else(|| "No binary found in installation".to_string())?
        };

        let _ = tx_progress.send(InstallProgress::Compiling(100)); // Simulated compiling progress
        Ok(TempInstall {
            crate_name: crate_name.to_string(),
            temp_dir_path, // Store the path
            cargo_home,
            binary_path: actual_binary_path,
        })
    }

    fn install_crate_permanent(&mut self) {
        if self.view_mode != ViewMode::Crates {
            self.status_message = Some("Can only install crates from crates.io view".to_string());
            return;
        }

        let items = if self.search_buffer.is_empty() { &self.items } else { &self.filtered_items };
        if items.is_empty() {
            let message = "No crates available".to_string();
            self.status_message = Some(message);
            return;
        }

        let selected_crate = &items[self.selected];
        let crate_name = selected_crate.name.clone();

        self.install_progress = InstallProgress::Starting;
        let installing_message = format!("Installing {} permanently...", crate_name);
        self.status_message = Some(installing_message);

        let (tx_progress, rx_progress) = mpsc::unbounded_channel();
        self.install_rx = Some(rx_progress); // Use the install_rx for progress updates

        tokio::spawn(async move {
            let _ = tx_progress.send(InstallProgress::Downloading(10));
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let output = Command::new("cargo")
                .args(&["install", &crate_name])
                .output();

            let _ = tx_progress.send(InstallProgress::Downloading(80)); // Simulate more progress

            match output {
                Ok(output) => {
                    if output.status.success() {
                        let _ = tx_progress.send(InstallProgress::Finished);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let _ = tx_progress.send(InstallProgress::Error(format!("Failed to install {}: {}", crate_name, stderr)));
                    }
                }
                Err(err) => {
                    let _ = tx_progress.send(InstallProgress::Error(format!("Failed to execute cargo install: {}", err)));
                }
            }
        });
    }

    fn run_temp_crate(&mut self) {
        if self.view_mode != ViewMode::Crates {
            self.status_message = Some("Can only run crates from crates.io view".to_string());
            return;
        }

        let items = if self.search_buffer.is_empty() { &self.items } else { &self.filtered_items };
        if items.is_empty() {
            let message = "No crates available".to_string();
            self.status_message = Some(message);
            return;
        }

        let selected_crate = &items[self.selected];
        let crate_name = selected_crate.name.clone();

        // Find the temp installation
        if let Some(install) = self.temp_installs.iter().find(|i| i.crate_name == crate_name) {
            let running_message = format!("Running {}...", crate_name);
            self.status_message = Some(running_message);

            let binary_path = install.binary_path.clone();

            // Exit the TUI temporarily to run the program
            let _ = disable_raw_mode();
            let _ = execute!(
                io::stdout(),
                LeaveAlternateScreen,
                cursor::Show
            );

            // Run the binary - This is still blocking for the TUI, which is generally desired for
            // running interactive CLI tools.
            let status = Command::new(&binary_path).status();

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
        // Since we are storing PathBuf now, the TempDir will be dropped when it goes out of scope,
        // which normally handles cleanup. However, if we need explicit cleanup before the program exits
        // (e.g., if a user quits prematurely), we might need to manually remove the directories.
        // For this pattern, it's common to let the `TempDir` handle its `Drop` implementation on program exit.
        // Since we changed to `temp_dir_path: PathBuf`, we are no longer holding the `TempDir` object itself.
        // If explicit cleanup is still desired, it would involve iterating `temp_installs` and trying to remove
        // the `temp_dir_path` directories. For now, we'll just clear the list.
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
        // Check crates.io results
        if let Some(rx) = &mut self.rx {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(mut crates) => {
                            crates.retain(|c| !c.yanked); // Filter out yanked crates

                            self.items = crates;
                            self.update_filtered_items();
                            if self.items.is_empty() {
                                self.error = Some("No crates found".to_string());
                            } else {
                                self.error = None;
                            }
                            self.selected = 0;
                            self.list_state.select(Some(0));
                            if self.view_mode == ViewMode::Crates {
                                self.show_welcome = false;
                            }
                        }
                        Err(err) => {
                            self.error = Some(err);
                            self.items.clear();
                            if self.view_mode == ViewMode::Crates {
                                self.show_welcome = false;
                            }
                        }
                    }
                    self.loading = false;
                    self.rx = None; // Channel processed, clear it
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel disconnected, task finished or failed
                    self.loading = false;
                    self.rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // No message yet, keep waiting
                }
            }
        }

        // Check GitHub results
        if let Some(rx) = &mut self.github_rx {
            match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(repos) => {
                            self.github_items = repos;
                            self.update_filtered_items();
                            if self.github_items.is_empty() {
                                self.error = Some("No GitHub repositories found".to_string());
                            } else {
                                self.error = None;
                            }
                            self.selected = 0;
                            self.list_state.select(Some(0));
                            if self.view_mode == ViewMode::GitHub {
                                self.show_welcome = false;
                            }
                        }
                        Err(err) => {
                            self.error = Some(err);
                            self.github_items.clear();
                            if self.view_mode == ViewMode::GitHub {
                                self.show_welcome = false;
                            }
                        }
                    }
                    self.loading = false;
                    self.github_rx = None; // Channel processed, clear it
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.loading = false;
                    self.github_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {}
            }
        }

        // Check install progress results (New)
        if let Some(rx_install) = &mut self.install_rx {
            match rx_install.try_recv() {
                Ok(progress) => {
                    self.install_progress = progress.clone();
                    match progress {
                        InstallProgress::Finished => {
                            self.status_message = Some("Installation completed!".to_string());
                            self.install_rx = None; // Clear the channel
                        }
                        InstallProgress::Error(e) => {
                            self.status_message = Some(format!("Installation error: {}", e));
                            self.install_rx = None; // Clear the channel
                        }
                        InstallProgress::TempInstallSuccess(temp_install) => {
                            self.temp_installs.push(temp_install);
                            self.status_message = Some("Temporary installation completed!".to_string());
                            self.install_rx = None; // Clear the channel
                        }
                        _ => {
                            // Update status message based on progress
                            self.status_message = Some(match &self.install_progress {
                                InstallProgress::Starting => "Installation starting...".to_string(),
                                InstallProgress::Downloading(p) => format!("Downloading... {}%", p),
                                InstallProgress::Compiling(p) => format!("Compiling... {}%", p),
                                _ => "Installation in progress...".to_string(),
                            });
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.install_rx = None; // Channel disconnected, assume task finished
                    if let InstallProgress::Starting | InstallProgress::Downloading(_) | InstallProgress::Compiling(_) = self.install_progress {
                        // If it disconnected before finishing, it's an error.
                        self.install_progress = InstallProgress::Error("Installation process unexpectedly ended.".to_string());
                        self.status_message = Some("Installation process unexpectedly ended.".to_string());
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => {} // No update yet
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

async fn fetch_github_ratatui_repos() -> GitHubFetchResult {
    let url = "https://api.github.com/search/repositories?q=ratatui+language:rust&sort=stars&order=desc&per_page=100";

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

            match serde_json::from_str::<GitHubSearchResponse>(&response_text) {
                Ok(data) => Ok(data.items),
                Err(e) => Err(format!("Failed to parse JSON: {}. Response was: {}", e, response_text.chars().take(200).collect::<String>())),
            }
        },
        Err(e) => Err(format!("Failed to fetch data: {}", e)),
    }
}

// run function modified to return early on quit
async fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        app.check_fetch_result();
        terminal.draw(|f| ui::<B>(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let outcome = if app.cmd_mode {
                handle_cmd_mode(app, key)
            } else {
                handle_normal_mode(app, key);
                CommandOutcome::Continue
            };

            if let CommandOutcome::Quit = outcome {
                return Ok(()); // Exit the loop gracefully
            }
        }
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.next();
            app.show_welcome = false;
            app.status_message = None; // Clear status on navigation
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.prev();
            app.show_welcome = false;
            app.status_message = None; // Clear status on navigation
        }
        KeyCode::Char(':') => app.enter_cmd_mode(),
        KeyCode::Char('/') => {
            app.enter_search_mode();
            app.show_welcome = false;
        }
        KeyCode::Tab => {
            // Switch between crates.io and GitHub views
            app.view_mode = match app.view_mode {
                ViewMode::Crates => ViewMode::GitHub,
                ViewMode::GitHub => ViewMode::Crates,
            };
            app.selected = 0;
            app.list_state.select(Some(0));
            app.update_filtered_items();
            app.show_welcome = false;
            // Trigger fetch for the new view if not already loading
            match app.view_mode {
                ViewMode::Crates => app.spawn_fetch(),
                ViewMode::GitHub => app.spawn_github_fetch(),
            }
        }
        KeyCode::Esc => {
            if app.search_mode {
                app.exit_search_mode();
            } else if app.cmd_mode {
                app.exit_cmd_mode();
            }
            app.status_message = None; // Clear status on escape
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

// handle_cmd_mode modified to return CommandOutcome
fn handle_cmd_mode(app: &mut App, key: KeyEvent) -> CommandOutcome {
    match key.code {
        KeyCode::Esc => {
            app.exit_cmd_mode();
            CommandOutcome::Continue
        }
        KeyCode::Enter => {
            let cmd = app.cmd_buffer.trim();
            match cmd {
                "pkg list" => {
                    if app.view_mode == ViewMode::Crates {
                        app.spawn_fetch();
                        app.status_message = Some("Refreshing crates.io package list...".to_string());
                    } else {
                        app.status_message = Some("Use 'pkg github' to refresh GitHub repositories".to_string());
                    }
                }
                "pkg github" => {
                    app.view_mode = ViewMode::GitHub;
                    app.spawn_github_fetch();
                    app.status_message = Some("Fetching GitHub repositories...".to_string());
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
                    app.exit_cmd_mode();
                    return CommandOutcome::Quit; // Signal to quit gracefully
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
            CommandOutcome::Continue
        }
        KeyCode::Backspace => {
            app.pop_char();
            CommandOutcome::Continue
        }
        KeyCode::Char(c) => {
            app.push_char(c);
            CommandOutcome::Continue
        }
        _ => CommandOutcome::Continue,
    }
}

fn ui<B: Backend>(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0), // Main content (list and details)
            Constraint::Length(2), // New bottom area: 1 for status/hint, 1 for input
        ])
        .split(f.size());

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]); // Split the main content area

    // Create list items based on current view mode
    let items: Vec<ListItem> = match app.view_mode {
        ViewMode::Crates => {
            let crate_list = if app.search_buffer.is_empty() { &app.items } else { &app.filtered_items };
            crate_list.iter()
                .enumerate()
                .map(|(i, crate_item)| {
                    let mut style = if i == app.selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };

                    if app.temp_installs.iter().any(|install| install.crate_name == crate_item.name) {
                        style = style.fg(Color::Green);
                    }

                    ListItem::new(crate_item.name.as_str()).style(style)
                })
                .collect()
        }
        ViewMode::GitHub => {
            let repo_list = if app.search_buffer.is_empty() { &app.github_items } else { &app.filtered_github_items };
            repo_list.iter()
                .enumerate()
                .map(|(i, repo)| {
                    let style = if i == app.selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };

                    ListItem::new(repo.name.as_str()).style(style)
                })
                .collect()
        }
    };

    let list_title = match app.view_mode {
        ViewMode::Crates => {
            if app.search_mode && !app.search_buffer.is_empty() {
                format!("Crates ({}) [Tab: GitHub] (Search: {})", app.current_items(), app.search_buffer)
            } else {
                format!("Crates ({}) [Tab: GitHub]", app.current_items())
            }
        }
        ViewMode::GitHub => {
            if app.search_mode && !app.search_buffer.is_empty() {
                format!("GitHub ({}) [Tab: Crates] (Search: {})", app.current_items(), app.search_buffer)
            } else {
                format!("GitHub ({}) [Tab: Crates]", app.current_items())
            }
        }
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(list_title));
    f.render_stateful_widget(list, top[0], &mut app.list_state);

    // Create details panel content
    let detail_text: Text = if app.show_welcome {
        Text::from(
            "🦀 Rust Crate Explorer - Enhanced Edition\n\n\
            Welcome to the Enhanced Rust Crate Explorer! This tool allows you to\n\
            search, browse, and install crates from crates.io and GitHub safely.\n\n\
            📋 COMMANDS:\n\
            • :pkg list    - Refresh crates.io package list\n\
            • :pkg github  - Fetch GitHub repositories with ratatui\n\
            • :try         - Install selected crate temporarily (crates.io only)\n\
            • :run         - Run temporarily installed crate\n\
            • :install     - Install selected crate permanently (crates.io only)\n\
            • :temp        - List temporary installations\n\
            • :help / :h   - Show this help\n\
            • :q           - Quit (cleans up temp installs)\n\n\
            🔤 NAVIGATION:\n\
            • j / ↓        - Move down in the list\n\
            • k / ↑        - Move up in the list\n\
            • Tab          - Switch between crates.io and GitHub views\n\
            • :            - Enter command mode\n\
            • /            - Enter search mode (real-time filter)\n\
            • ESC          - Exit command/search mode\n\n\
            📦 FEATURES:\n\
            • Try crates without permanent installation\n\
            • Progress bars during installation\n\
            • Browse GitHub repositories with ratatui\n\
            • Enhanced crate details with downloads, dates, links\n\
            • Automatic yanked crate filtering\n\
            • Real-time search through names and descriptions\n\
            • Wrapped text descriptions\n\
            • Green highlighting for temp-installed crates\n\n\
            Press Tab to switch views or any navigation key to start exploring!"
        )
    } else if let Some(status) = &app.status_message {
        Text::from(status.as_str())
    } else if let Some(err) = &app.error {
        Text::from(err.as_str())
    } else if app.current_items() == 0 {
        Text::from(
            if app.loading {
                "Loading..."
            } else if !app.search_buffer.is_empty() {
                match app.view_mode {
                    ViewMode::Crates => "No crates match your search",
                    ViewMode::GitHub => "No repositories match your search",
                }
            } else {
                match app.view_mode {
                    ViewMode::Crates => "No crates available",
                    ViewMode::GitHub => "No repositories available. Use :pkg github to fetch.",
                }
            }
        )
    } else {
        match app.view_mode {
            ViewMode::Crates => {
                let crate_list = if app.search_buffer.is_empty() { &app.items } else { &app.filtered_items };
                if let Some(selected_crate) = crate_list.get(app.selected) {
                    let created: DateTime<Utc> = selected_crate.created_at.parse().unwrap_or_else(|_| Utc::now());
                    let updated: DateTime<Utc> = selected_crate.updated_at.parse().unwrap_or_else(|_| Utc::now());

                    Text::from(format!(
                        "Name: {}\n\
                        Version: {}\n\
                        Description: {}\n\
                        Downloads: {}\n\
                        Recent Downloads: {}\n\
                        Created At: {}\n\
                        Updated At: {}\n\
                        Homepage: {}\n\
                        Repository: {}\n\
                        Documentation: {}",
                        selected_crate.name,
                        selected_crate.max_version,
                        selected_crate.description.as_deref().unwrap_or("N/A"),
                        selected_crate.downloads,
                        selected_crate.recent_downloads.map_or_else(|| "N/A".to_string(), |d| d.to_string()),
                        created.format("%Y-%m-%d %H:%M:%S UTC"),
                        updated.format("%Y-%m-%d %H:%M:%S UTC"),
                        selected_crate.homepage.as_deref().unwrap_or("N/A"),
                        selected_crate.repository.as_deref().unwrap_or("N/A"),
                        selected_crate.documentation.as_deref().unwrap_or("N/A"),
                    ))
                } else {
                    Text::from("Select a crate to see details")
                }
            }
            ViewMode::GitHub => {
                let repo_list = if app.search_buffer.is_empty() { &app.github_items } else { &app.filtered_github_items };
                if let Some(selected_repo) = repo_list.get(app.selected) {
                    let updated: DateTime<Utc> = selected_repo.updated_at.parse().unwrap_or_else(|_| Utc::now());

                    Text::from(format!(
                        "Name: {}\n\
                        Full Name: {}\n\
                        Description: {}\n\
                        URL: {}\n\
                        Stars: {}\n\
                        Forks: {}\n\
                        Language: {}\n\
                        Updated At: {}\n\
                        Topics: {}",
                        selected_repo.name,
                        selected_repo.full_name,
                        selected_repo.description.as_deref().unwrap_or("N/A"),
                        selected_repo.html_url,
                        selected_repo.stargazers_count,
                        selected_repo.forks_count,
                        selected_repo.language.as_deref().unwrap_or("N/A"),
                        updated.format("%Y-%m-%d %H:%M:%S UTC"),
                        if selected_repo.topics.is_empty() { "N/A".to_string() } else { selected_repo.topics.join(", ") },
                    ))
                } else {
                    Text::from("Select a repository to see details")
                }
            }
        }
    };

    let details_block_title = match app.view_mode {
        ViewMode::Crates => "Crate Details",
        ViewMode::GitHub => "GitHub Repository Details",
    };

    let details_panel = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(details_block_title))
        .wrap(Wrap { trim: false });
    f.render_widget(details_panel, top[1]);

    // New bottom area split for status/hint and input
    let bottom_area = chunks[1]; // This is the 2-line bottom area
    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status/Hint line
            Constraint::Length(1), // Command/Search input line
        ])
        .split(bottom_area);

    // Status/Hint message for the bottom bar
    let status_hint_text = if app.cmd_mode {
        Span::styled("COMMAND MODE (Press Enter to execute, Esc to exit)", Style::default().fg(Color::Yellow))
    } else if app.search_mode {
        Span::styled("SEARCH MODE (Press Esc to clear/exit)", Style::default().fg(Color::Yellow))
    } else if let Some(status) = &app.status_message {
        Span::styled(status.as_str(), Style::default().fg(Color::LightBlue))
    } else if let Some(err) = &app.error {
        Span::styled(err.as_str(), Style::default().fg(Color::Red))
    } else {
        Span::styled("Press ':' for command, '/' for search, '?' for help", Style::default().fg(Color::DarkGray))
    };

    let status_hint_paragraph = Paragraph::new(status_hint_text);
    f.render_widget(status_hint_paragraph, bottom_chunks[0]); // Render in the top part of bottom area

    // Command/Search input
    let input_text_content = if app.cmd_mode {
        format!(":{}", app.cmd_buffer)
    } else if app.search_mode {
        format!("/{}", app.search_buffer)
    } else {
        "".to_string() // No input text when not in mode
    };

    let input_paragraph = Paragraph::new(input_text_content)
        .style(Style::default().fg(Color::White).bg(Color::Black)); // Use black background for clarity

    f.render_widget(input_paragraph, bottom_chunks[1]); // Render in the bottom part of bottom area

    // Cursor positioning
    if app.cmd_mode || app.search_mode {
        let cursor_x = bottom_chunks[1].x + if app.cmd_mode { 1 } else { 1 } + app.cmd_buffer.len() as u16; // +1 for ':' or '/'
        let cursor_y = bottom_chunks[1].y;
        f.set_cursor(cursor_x, cursor_y);
    }

    // Render install progress gauge
    if app.install_progress != InstallProgress::None &&
       app.install_progress != InstallProgress::Finished &&
       !matches!(app.install_progress, InstallProgress::Error(_))
    {
        let (label, ratio) = match &app.install_progress {
            InstallProgress::Starting => ("Starting...".to_string(), 0.0),
            InstallProgress::Downloading(p) => (format!("Downloading... {}%", p), *p as f64 / 100.0),
            InstallProgress::Compiling(p) => (format!("Compiling... {}%", p), *p as f64 / 100.0),
            _ => ("Processing...".to_string(), 0.0),
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Installation Progress"))
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent((ratio * 100.0) as u16)
            .label(Span::styled(label, Style::default().fg(Color::White)));

        // Position the gauge slightly above the new 2-line bottom bar
        let gauge_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(3), // Height of the gauge
                Constraint::Length(2), // Total height for status and input bars (2 lines)
            ])
            .split(f.size())[1]; // Get the middle chunk for the gauge

        f.render_widget(gauge, gauge_area);
    }
}
