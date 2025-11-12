// ============================================================================
// src/main.rs
// ============================================================================

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

mod cache;
mod types;

use cache::get_data;
use types::{CratePackage, CratesData};

// ============================================================================
// App State
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,      // Navigation mode
    Command,     // Command mode (after pressing ':')
}

struct App {
    // Data
    all_crates: Vec<CratePackage>,
    filtered_crates: Vec<CratePackage>,
    metadata: types::Metadata,
    
    // UI State
    list_state: ListState,
    mode: Mode,
    command_input: String,
    status_message: String,
    show_help: bool,
    
    // Search state
    last_search: String,
}

impl App {
    fn new(data: CratesData) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        let all_crates = data.crates.clone();
        let filtered_crates = all_crates.clone();
        let metadata = data.metadata.clone();  // Clone here so we can use it twice
        
        Self {
            all_crates,
            filtered_crates,
            metadata: metadata.clone(),  // First use
            list_state,
            mode: Mode::Normal,
            command_input: String::new(),
            status_message: format!(
                "Total: {} | Core: {} | Community: {} | Press ':' for commands or '?' for help",
                metadata.total_crates,
                metadata.core_libraries,
                metadata.community_packages  // Second use
            ),
            show_help: false,
            last_search: String::new(),
        }
    }
    
    fn selected_crate(&self) -> Option<&CratePackage> {
        self.list_state.selected()
            .and_then(|i| self.filtered_crates.get(i))
    }
    
    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_crates.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_crates.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn next_page(&mut self) {
        let jump = 10;
        let i = match self.list_state.selected() {
            Some(i) => {
                if i + jump >= self.filtered_crates.len() {
                    self.filtered_crates.len() - 1
                } else {
                    i + jump
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn previous_page(&mut self) {
        let jump = 10;
        let i = match self.list_state.selected() {
            Some(i) => {
                if i < jump {
                    0
                } else {
                    i - jump
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    
    fn execute_command(&mut self) {
        let cmd = self.command_input.trim();
        
        if cmd.is_empty() {
            self.mode = Mode::Normal;
            return;
        }
        
        // Parse command
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let command = parts[0];
        
        match command {
            "q" | "quit" => {
                // Will be handled in main loop
            }
            "core" => {
                self.filtered_crates = self.all_crates
                    .iter()
                    .filter(|c| c.is_core_library)
                    .cloned()
                    .collect();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing {} core libraries", self.filtered_crates.len());
            }
            "all" => {
                self.filtered_crates = self.all_crates.clone();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing all {} crates", self.filtered_crates.len());
            }
            "top" => {
                let limit: usize = parts.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                
                let mut sorted = self.all_crates.clone();
                sorted.sort_by(|a, b| b.downloads.cmp(&a.downloads));
                self.filtered_crates = sorted.into_iter().take(limit).collect();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing top {} by downloads", limit);
            }
            "recent" => {
                let limit: usize = parts.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                
                let mut sorted = self.all_crates.clone();
                sorted.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));
                self.filtered_crates = sorted.into_iter().take(limit).collect();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing top {} by weekly downloads", limit);
            }
            "new" => {
                let limit: usize = parts.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10);
                
                let mut sorted = self.all_crates.clone();
                sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                self.filtered_crates = sorted.into_iter().take(limit).collect();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing {} newest crates", limit);
            }
            "search" | "/" => {
                if parts.len() > 1 {
                    let query = parts[1..].join(" ").to_lowercase();
                    self.last_search = query.clone();
                    self.filtered_crates = self.all_crates
                        .iter()
                        .filter(|c| {
                            c.name.to_lowercase().contains(&query)
                                || c.description.to_lowercase().contains(&query)
                        })
                        .cloned()
                        .collect();
                    self.list_state.select(Some(0));
                    self.status_message = format!(
                        "Found {} crates matching '{}'",
                        self.filtered_crates.len(),
                        self.last_search
                    );
                } else {
                    self.status_message = "Usage: :search <query> or /<query>".to_string();
                }
            }
            "help" | "?" => {
                self.show_help = !self.show_help;
                self.status_message = if self.show_help {
                    "Showing help".to_string()
                } else {
                    "Help hidden".to_string()
                };
            }
            _ => {
                // Try as search query
                let query = cmd.to_lowercase();
                self.last_search = query.clone();
                self.filtered_crates = self.all_crates
                    .iter()
                    .filter(|c| {
                        c.name.to_lowercase().contains(&query)
                            || c.description.to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect();
                self.list_state.select(Some(0));
                self.status_message = format!(
                    "Found {} crates matching '{}'",
                    self.filtered_crates.len(),
                    query
                );
            }
        }
        
        self.command_input.clear();
        self.mode = Mode::Normal;
    }
}

// ============================================================================
// UI Rendering
// ============================================================================

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // Main content
            Constraint::Length(3),   // Command/status bar
        ])
        .split(f.size());
    
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),  // List
            Constraint::Percentage(65),  // Detail
        ])
        .split(chunks[0]);
    
    // Render list
    render_list(f, app, main_chunks[0]);
    
    // Render detail
    if app.show_help {
        render_help(f, main_chunks[1]);
    } else {
        render_detail(f, app, main_chunks[1]);
    }
    
    // Render command/status bar
    render_command_bar(f, app, chunks[1]);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_crates
        .iter()
        .map(|crate_pkg| {
            let icon = if crate_pkg.is_core_library { "⭐" } else { "📦" };
            let name = format!("{} {}", icon, crate_pkg.name);
            
            ListItem::new(Line::from(vec![
                Span::styled(
                    name,
                    if crate_pkg.is_core_library {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Cyan)
                    },
                ),
            ]))
        })
        .collect();
    
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Crates ({}/{}) ",
                    app.filtered_crates.len(),
                    app.all_crates.len()
                ))
                .style(Style::default().fg(Color::White)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail = if let Some(crate_pkg) = app.selected_crate() {
        let mut lines = vec![];
        
        // Title
        let icon = if crate_pkg.is_core_library { "⭐" } else { "📦" };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} {} ", icon, crate_pkg.name),
                Style::default()
                    .fg(if crate_pkg.is_core_library { Color::Yellow } else { Color::Cyan })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("v{}", crate_pkg.version),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        
        if crate_pkg.is_core_library {
            lines.push(Line::from(Span::styled(
                "CORE LIBRARY",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        
        lines.push(Line::from(""));
        
        // Description
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(crate_pkg.description.clone()));
        lines.push(Line::from(""));
        
        // Statistics
        lines.push(Line::from(Span::styled(
            "Statistics:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("  Downloads:        "),
            Span::styled(
                format_number(crate_pkg.downloads),
                Style::default().fg(Color::Green),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  Weekly Downloads: "),
            Span::styled(
                format_number(crate_pkg.recent_downloads),
                Style::default().fg(Color::Blue),
            ),
        ]));
        lines.push(Line::from(""));
        
        // Install
        lines.push(Line::from(Span::styled(
            "Install:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("cargo add {}", crate_pkg.name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        
        // Links
        if crate_pkg.repository.is_some()
            || crate_pkg.documentation.is_some()
            || crate_pkg.homepage.is_some()
        {
            lines.push(Line::from(Span::styled(
                "Links:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));
            
            if let Some(repo) = &crate_pkg.repository {
                lines.push(Line::from(vec![
                    Span::raw("  Repository:    "),
                    Span::styled(repo, Style::default().fg(Color::Blue)),
                ]));
            }
            if let Some(docs) = &crate_pkg.documentation {
                lines.push(Line::from(vec![
                    Span::raw("  Documentation: "),
                    Span::styled(docs, Style::default().fg(Color::Blue)),
                ]));
            }
            if let Some(home) = &crate_pkg.homepage {
                lines.push(Line::from(vec![
                    Span::raw("  Homepage:      "),
                    Span::styled(home, Style::default().fg(Color::Blue)),
                ]));
            }
            lines.push(Line::from(""));
        }
        
        // Categories
        if let Some(categories) = &crate_pkg.categories {
            if !categories.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Categories:",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        categories.join(", "),
                        Style::default().fg(Color::Magenta),
                    ),
                ]));
            }
        }
        
        Text::from(lines)
    } else {
        Text::from("No crate selected")
    };
    
    let paragraph = Paragraph::new(detail)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail ")
                .style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled(
            "RATCRATE TUI - Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓      - Move down"),
        Line::from("  k / ↑      - Move up"),
        Line::from("  h          - (reserved)"),
        Line::from("  l          - (reserved)"),
        Line::from("  Ctrl+d     - Page down"),
        Line::from("  Ctrl+u     - Page up"),
        Line::from("  g          - Go to top"),
        Line::from("  G          - Go to bottom"),
        Line::from(""),
        Line::from(Span::styled(
            "Commands (press ':'):",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  :q, :quit         - Quit"),
        Line::from("  :all              - Show all crates"),
        Line::from("  :core             - Show core libraries only"),
        Line::from("  :top [N]          - Show top N by downloads (default: 10)"),
        Line::from("  :recent [N]       - Show top N by weekly downloads"),
        Line::from("  :new [N]          - Show N newest crates"),
        Line::from("  :search <query>   - Search crates"),
        Line::from("  /<query>          - Quick search"),
        Line::from("  :help, ?          - Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "Examples:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  :top 5            - Top 5 most downloaded"),
        Line::from("  :search bevy      - Search for 'bevy'"),
        Line::from("  /terminal         - Quick search 'terminal'"),
        Line::from("  :recent 10        - Top 10 by weekly downloads"),
        Line::from(""),
        Line::from(Span::styled(
            "Tips:",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  • Press '?' to toggle help"),
        Line::from("  • Press 'q' to quit quickly"),
        Line::from("  • Use ':' for commands, Esc to cancel"),
        Line::from("  • Vim-like navigation (hjkl)"),
    ];
    
    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help (Press '?' to close) ")
                .style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });
    
    f.render_widget(paragraph, area);
}

fn render_command_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = match app.mode {
        Mode::Normal => {
            Text::from(Line::from(vec![
                Span::styled(
                    " NORMAL ",
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::raw(&app.status_message),
            ]))
        }
        Mode::Command => {
            Text::from(Line::from(vec![
                Span::styled(
                    " COMMAND ",
                    Style::default()
                        .bg(Color::Green)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" :"),
                Span::styled(
                    &app.command_input,
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
            ]))
        }
    };
    
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(paragraph, area);
}

// ============================================================================
// Event Handling
// ============================================================================

fn handle_events(app: &mut App) -> Result<bool> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Normal => match key.code {
                    // Quit
                    KeyCode::Char('q') => return Ok(true),
                    
                    // Navigation
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.next_page()
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.previous_page()
                    }
                    KeyCode::Char('g') => app.list_state.select(Some(0)),
                    KeyCode::Char('G') => {
                        app.list_state.select(Some(app.filtered_crates.len().saturating_sub(1)))
                    }
                    
                    // Commands
                    KeyCode::Char(':') | KeyCode::Char('/') => {
                        app.mode = Mode::Command;
                        app.command_input.clear();
                        if key.code == KeyCode::Char('/') {
                            app.command_input.push_str("search ");
                        }
                    }
                    
                    // Help
                    KeyCode::Char('?') => {
                        app.show_help = !app.show_help;
                    }
                    
                    _ => {}
                },
                Mode::Command => match key.code {
                    KeyCode::Enter => {
                        if app.command_input == "q" || app.command_input == "quit" {
                            return Ok(true);
                        }
                        app.execute_command();
                    }
                    KeyCode::Char(c) => {
                        app.command_input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.command_input.pop();
                    }
                    KeyCode::Esc => {
                        app.mode = Mode::Normal;
                        app.command_input.clear();
                    }
                    _ => {}
                },
            }
        }
    }
    Ok(false)
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    // Load data
    println!("Loading Ratcrate data...");
    let data = get_data(false)?;
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app
    let mut app = App::new(data);
    
    // Run app
    let result = run_app(&mut terminal, &mut app);
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        
        if handle_events(app)? {
            break;
        }
    }
    Ok(())
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
