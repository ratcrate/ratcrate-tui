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
    Normal, // Navigation mode
    Command, // Command mode (after pressing ':')
            // Try,         // Try mode - confirming installation
}

#[derive(Debug, Clone, PartialEq)]
enum View {
    List,  // List + Detail view
    Stats, // Statistics view
    Help,  // Help view
}

struct App {
    // Data
    all_crates: Vec<CratePackage>,
    filtered_crates: Vec<CratePackage>,
    metadata: types::Metadata,

    // UI State
    list_state: ListState,
    mode: Mode,
    view: View,
    command_input: String,
    status_message: String,

    // Try mode
    // try_crate: Option<String>,
    // try_temp_dir: Option<String>,

    // Search state
    last_search: String,
}

impl App {
    fn new(data: CratesData) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let all_crates = data.crates.clone();
        let filtered_crates = all_crates.clone();
        let metadata = data.metadata.clone();

        Self {
            all_crates,
            filtered_crates,
            metadata: metadata.clone(),
            list_state,
            mode: Mode::Normal,
            view: View::List,
            command_input: String::new(),
            status_message: format!(
                "📦 {} crates | ⭐ {} core | 🌍 {} community | Press TAB for stats, ? for help, : for commands",
                metadata.total_crates,
                metadata.core_libraries,
                metadata.community_packages
            ),
            // try_crate: None,
            // try_temp_dir: None,
            last_search: String::new(),
        }
    }

    fn selected_crate(&self) -> Option<&CratePackage> {
        self.list_state
            .selected()
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

    //     fn execute_command(&mut self) {
    //         let cmd = self.command_input.trim();
    //
    //         if cmd.is_empty() {
    //             self.mode = Mode::Normal;
    //             return;
    //         }
    //
    //         // Parse command
    //         let parts: Vec<&str> = cmd.split_whitespace().collect();
    //         let command = parts[0];
    //
    //         match command {
    //             "q" | "quit" => {
    //                 // Will be handled in main loop
    //             }
    //             "core" => {
    //                 self.filtered_crates = self.all_crates
    //                     .iter()
    //                     .filter(|c| c.is_core_library)
    //                     .cloned()
    //                     .collect();
    //                 self.list_state.select(Some(0));
    //                 self.status_message = format!("Showing {} core libraries", self.filtered_crates.len());
    //             }
    //             "all" => {
    //                 self.filtered_crates = self.all_crates.clone();
    //                 self.list_state.select(Some(0));
    //                 self.status_message = format!("Showing all {} crates", self.filtered_crates.len());
    //             }
    //             "top" => {
    //                 let limit: usize = parts.get(1)
    //                     .and_then(|s| s.parse().ok())
    //                     .unwrap_or(10);
    //
    //                 let mut sorted = self.all_crates.clone();
    //                 sorted.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    //                 self.filtered_crates = sorted.into_iter().take(limit).collect();
    //                 self.list_state.select(Some(0));
    //                 self.status_message = format!("Showing top {} by downloads", limit);
    //             }
    //             "recent" => {
    //                 let limit: usize = parts.get(1)
    //                     .and_then(|s| s.parse().ok())
    //                     .unwrap_or(10);
    //
    //                 let mut sorted = self.all_crates.clone();
    //                 sorted.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));
    //                 self.filtered_crates = sorted.into_iter().take(limit).collect();
    //                 self.list_state.select(Some(0));
    //                 self.status_message = format!("Showing top {} by weekly downloads", limit);
    //             }
    //             "new" => {
    //                 let limit: usize = parts.get(1)
    //                     .and_then(|s| s.parse().ok())
    //                     .unwrap_or(10);
    //
    //                 let mut sorted = self.all_crates.clone();
    //                 sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    //                 self.filtered_crates = sorted.into_iter().take(limit).collect();
    //                 self.list_state.select(Some(0));
    //                 self.status_message = format!("Showing {} newest crates", limit);
    //             }
    //             "search" | "/" => {
    //                 if parts.len() > 1 {
    //                     let query = parts[1..].join(" ").to_lowercase();
    //                     self.last_search = query.clone();
    //                     self.filtered_crates = self.all_crates
    //                         .iter()
    //                         .filter(|c| {
    //                             c.name.to_lowercase().contains(&query)
    //                                 || c.description.to_lowercase().contains(&query)
    //                         })
    //                         .cloned()
    //                         .collect();
    //                     self.list_state.select(Some(0));
    //                     self.status_message = format!(
    //                         "Found {} crates matching '{}'",
    //                         self.filtered_crates.len(),
    //                         self.last_search
    //                     );
    //                 } else {
    //                     self.status_message = "Usage: :search <query> or /<query>".to_string();
    //                 }
    //             }
    //             "help" | "?" => {
    //                 self.view = if self.view == View::Help { View::List } else { View::Help };
    //                 self.status_message = if self.view == View::Help {
    //                     "Showing help - Press ? or TAB to go back".to_string()
    //                 } else {
    //                     "Help hidden".to_string()
    //                 };
    //             }
    //             "try" => {
    //                 if let Some(crate_pkg) = self.selected_crate().cloned() {
    //                     self.try_crate = Some(crate_pkg.name.clone());
    //                     self.mode = Mode::Try;
    //                     self.status_message = format!(
    //                         "Try '{}' in /tmp/ratcrate-try? Press 'y' to confirm, 'n' to cancel",
    //                         crate_pkg.name
    //                     );
    //                 } else {
    //                     self.status_message = "No crate selected".to_string();
    //                 }
    //             }
    //             _ => {
    //                 // Try as search query
    //                 let query = cmd.to_lowercase();
    //                 self.last_search = query.clone();
    //                 self.filtered_crates = self.all_crates
    //                     .iter()
    //                     .filter(|c| {
    //                         c.name.to_lowercase().contains(&query)
    //                             || c.description.to_lowercase().contains(&query)
    //                     })
    //                     .cloned()
    //                     .collect();
    //                 self.list_state.select(Some(0));
    //                 self.status_message = format!(
    //                     "Found {} crates matching '{}'",
    //                     self.filtered_crates.len(),
    //                     query
    //                 );
    //             }
    //         }
    //
    //         self.command_input.clear();
    //         self.mode = Mode::Normal;
    //     }
    //

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
                self.filtered_crates = self
                    .all_crates
                    .iter()
                    .filter(|c| c.is_core_library)
                    .cloned()
                    .collect();
                self.list_state.select(Some(0));
                self.status_message =
                    format!("Showing {} core libraries", self.filtered_crates.len());
            }
            "all" => {
                self.filtered_crates = self.all_crates.clone();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing all {} crates", self.filtered_crates.len());
            }
            "top" => {
                let limit: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);

                let mut sorted = self.all_crates.clone();
                sorted.sort_by(|a, b| b.downloads.cmp(&a.downloads));
                self.filtered_crates = sorted.into_iter().take(limit).collect();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing top {} by downloads", limit);
            }
            "recent" => {
                let limit: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);

                let mut sorted = self.all_crates.clone();
                sorted.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));
                self.filtered_crates = sorted.into_iter().take(limit).collect();
                self.list_state.select(Some(0));
                self.status_message = format!("Showing top {} by weekly downloads", limit);
            }
            "new" => {
                let limit: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);

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
                    self.filtered_crates = self
                        .all_crates
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
                self.view = if self.view == View::Help {
                    View::List
                } else {
                    View::Help
                };
                self.status_message = if self.view == View::Help {
                    "Showing help - Press ? or TAB to go back".to_string()
                } else {
                    "Help hidden".to_string()
                };
            }
            // "try" => {
            //     if let Some(crate_pkg) = self.selected_crate().cloned() {
            //         self.try_crate = Some(crate_pkg.name.clone());
            //         self.mode = Mode::Try;
            //         self.status_message = format!(
            //             "Try '{}' in /tmp/ratcrate-try? Press 'y' to confirm, 'n' to cancel",
            //             crate_pkg.name
            //         );
            //     } else {
            //         self.status_message = "No crate selected".to_string();
            //     }
            // }
            _ => {
                // Try as search query
                let query = cmd.to_lowercase();
                self.last_search = query.clone();
                self.filtered_crates = self
                    .all_crates
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

        // Clear typed command, but DO NOT forcibly exit Try mode if we just entered it.
        self.command_input.clear();
        // if self.mode != Mode::Try {
        self.mode = Mode::Normal;
        // }
    }
}

// ============================================================================
// UI Rendering
// ============================================================================

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Command/status bar
        ])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // List
            Constraint::Percentage(65), // Detail
        ])
        .split(chunks[0]);

    // Render list
    render_list(f, app, main_chunks[0]);

    // Render detail/help/stats based on view
    match app.view {
        View::List => render_detail(f, app, main_chunks[1]),
        View::Help => render_help(f, main_chunks[1]),
        View::Stats => render_stats(f, app, main_chunks[1]),
    }

    // Render command/status bar
    render_command_bar(f, app, chunks[1]);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_crates
        .iter()
        .enumerate()
        .map(|(_idx, crate_pkg)| {
            let icon = if crate_pkg.is_core_library {
                "⭐"
            } else {
                "📦"
            };

            // Create a colorful list item
            let content = vec![
                Line::from(vec![
                    Span::styled(
                        format!("{} ", icon),
                        if crate_pkg.is_core_library {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default().fg(Color::Cyan)
                        },
                    ),
                    Span::styled(
                        &crate_pkg.name,
                        if crate_pkg.is_core_library {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        },
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled("↓ ", Style::default().fg(Color::Green)),
                    Span::styled(
                        format_number(crate_pkg.downloads),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(" 📈 ", Style::default().fg(Color::Blue)),
                    Span::styled(
                        format_number(crate_pkg.recent_downloads),
                        Style::default().fg(Color::Blue),
                    ),
                ]),
            ];

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(vec![
                    Span::styled(
                        " 📦 Crates ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("({}/{}) ", app.filtered_crates.len(), app.all_crates.len()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
                .style(Style::default()),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(60, 60, 80))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let detail = if let Some(crate_pkg) = app.selected_crate() {
        let mut lines = vec![];

        // Title with colorful icon
        let icon = if crate_pkg.is_core_library {
            "⭐"
        } else {
            "📦"
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} {} ", icon, crate_pkg.name),
                Style::default()
                    .fg(if crate_pkg.is_core_library {
                        Color::Yellow
                    } else {
                        Color::Cyan
                    })
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ),
            Span::styled(
                format!("v{}", crate_pkg.version),
                Style::default().fg(Color::Magenta),
            ),
        ]));

        if crate_pkg.is_core_library {
            lines.push(Line::from(Span::styled(
                "⭐ CORE LIBRARY ⭐",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        lines.push(Line::from(""));

        // Description with nice formatting
        lines.push(Line::from(Span::styled(
            "📝 Description:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));

        // Simple word wrapping for description
        let words: Vec<&str> = crate_pkg.description.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 > 60 {
                lines.push(Line::from(Span::styled(
                    format!("  {}", current_line),
                    Style::default().fg(Color::White),
                )));
                current_line = word.to_string();
            } else {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  {}", current_line),
                Style::default().fg(Color::White),
            )));
        }
        lines.push(Line::from(""));

        // Statistics with icons and colors
        lines.push(Line::from(Span::styled(
            "📊 Statistics:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("↓ Downloads:       ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_number(crate_pkg.downloads),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("📈 Weekly:          ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_number(crate_pkg.recent_downloads),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));

        // Install command with colorful box
        lines.push(Line::from(Span::styled(
            "📦 Install:",
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

        // Try mode hint
        lines.push(Line::from(vec![
            Span::styled(
                "💡 Tip: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("Use ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                ":try",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to test this crate in a temporary project!",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(""));

        // Links with icons
        if crate_pkg.repository.is_some()
            || crate_pkg.documentation.is_some()
            || crate_pkg.homepage.is_some()
        {
            lines.push(Line::from(Span::styled(
                "🔗 Links:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )));

            if let Some(repo) = &crate_pkg.repository {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("📁 Repo:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(repo, Style::default().fg(Color::Blue)),
                ]));
            }
            if let Some(docs) = &crate_pkg.documentation {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("📖 Docs:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(docs, Style::default().fg(Color::Blue)),
                ]));
            }
            if let Some(home) = &crate_pkg.homepage {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("🏠 Home:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(home, Style::default().fg(Color::Blue)),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Categories with colorful tags
        if let Some(categories) = &crate_pkg.categories {
            if !categories.is_empty() {
                lines.push(Line::from(Span::styled(
                    "🏷️  Categories:",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));

                let cat_spans: Vec<Span> = categories
                    .iter()
                    .flat_map(|cat| {
                        vec![
                            Span::styled("  [", Style::default().fg(Color::DarkGray)),
                            Span::styled(
                                cat,
                                Style::default()
                                    .fg(Color::Magenta)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled("]", Style::default().fg(Color::DarkGray)),
                            Span::raw(" "),
                        ]
                    })
                    .collect();

                lines.push(Line::from(cat_spans));
            }
        }

        Text::from(lines)
    } else {
        Text::from(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No crate selected",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Use j/k or ↑/↓ to navigate",
                Style::default().fg(Color::DarkGray),
            )),
        ])
    };

    let paragraph = Paragraph::new(detail)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled(
                    " 📋 Detail ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default()),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "╔═══════════════════════════════════════════════════════╗",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "║                                                       ║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                "║   ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "🐀 RATCRATE TUI",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  - Ratatui Ecosystem Explorer   ║",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "║                                                       ║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "╚═══════════════════════════════════════════════════════╝",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "🎹 Navigation:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  j / ↓      ", Style::default().fg(Color::Cyan)),
            Span::raw("- Move down"),
        ]),
        Line::from(vec![
            Span::styled("  k / ↑      ", Style::default().fg(Color::Cyan)),
            Span::raw("- Move up"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d     ", Style::default().fg(Color::Cyan)),
            Span::raw("- Page down"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u     ", Style::default().fg(Color::Cyan)),
            Span::raw("- Page up"),
        ]),
        Line::from(vec![
            Span::styled("  g          ", Style::default().fg(Color::Cyan)),
            Span::raw("- Go to top"),
        ]),
        Line::from(vec![
            Span::styled("  G          ", Style::default().fg(Color::Cyan)),
            Span::raw("- Go to bottom"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "📑 Views:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  TAB        ", Style::default().fg(Color::Yellow)),
            Span::raw("- Toggle Stats view"),
        ]),
        Line::from(vec![
            Span::styled("  ?          ", Style::default().fg(Color::Yellow)),
            Span::raw("- Toggle this help"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "⚡ Commands (press ':'):",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  :q, :quit         ", Style::default().fg(Color::Magenta)),
            Span::raw("- Quit"),
        ]),
        Line::from(vec![
            Span::styled("  :all              ", Style::default().fg(Color::Magenta)),
            Span::raw("- Show all crates"),
        ]),
        Line::from(vec![
            Span::styled("  :core             ", Style::default().fg(Color::Magenta)),
            Span::raw("- Show core libraries only"),
        ]),
        Line::from(vec![
            Span::styled("  :top [N]          ", Style::default().fg(Color::Magenta)),
            Span::raw("- Top N by downloads (default: 10)"),
        ]),
        Line::from(vec![
            Span::styled("  :recent [N]       ", Style::default().fg(Color::Magenta)),
            Span::raw("- Top N by weekly downloads"),
        ]),
        Line::from(vec![
            Span::styled("  :new [N]          ", Style::default().fg(Color::Magenta)),
            Span::raw("- N newest crates"),
        ]),
        Line::from(vec![
            Span::styled("  :search <query>   ", Style::default().fg(Color::Magenta)),
            Span::raw("- Search crates"),
        ]),
        Line::from(vec![
            Span::styled("  /<query>          ", Style::default().fg(Color::Magenta)),
            Span::raw("- Quick search"),
        ]),
        // Line::from(vec![
        //     Span::styled("  :try              ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        //     Span::raw("- Try selected crate in temp directory"),
        // ]),
        // Line::from(""),
        // Line::from(Span::styled(
        //     "🧪 Try Mode:",
        //     Style::default()
        //         .fg(Color::Yellow)
        //         .add_modifier(Modifier::BOLD),
        // )),
        // Line::from("  Creates a temporary Cargo project with the selected crate."),
        // Line::from("  Perfect for quick experiments! Auto-cleaned after exit."),
        // Line::from(""),
        // Line::from(Span::styled(
        //     "💡 Examples:",
        //     Style::default()
        //         .fg(Color::Green)
        //         .add_modifier(Modifier::BOLD),
        // )),
        Line::from(vec![
            Span::styled("  :top 5         ", Style::default().fg(Color::Cyan)),
            Span::raw("- Top 5 most downloaded"),
        ]),
        Line::from(vec![
            Span::styled("  :search bevy   ", Style::default().fg(Color::Cyan)),
            Span::raw("- Search for 'bevy'"),
        ]),
        Line::from(vec![
            Span::styled("  /terminal      ", Style::default().fg(Color::Cyan)),
            Span::raw("- Quick search 'terminal'"),
        ]),
        // Line::from(vec![
        //     Span::styled("  :try           ", Style::default().fg(Color::Cyan)),
        //     Span::raw("- Try selected crate"),
        // ]),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(vec![
                    Span::styled(" ❓ ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "Help",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " (Press ? or TAB to close) ",
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
                .style(Style::default()),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_stats(f: &mut Frame, app: &App, area: Rect) {
    // Calculate statistics
    let total = app.all_crates.len();
    let core = app.metadata.core_libraries;
    let community = app.metadata.community_packages;

    let total_downloads: u64 = app.all_crates.iter().map(|c| c.downloads).sum();
    let avg_downloads = if total > 0 {
        total_downloads / total as u64
    } else {
        0
    };

    let total_weekly: u64 = app.all_crates.iter().map(|c| c.recent_downloads).sum();

    // Top 5 by downloads
    let mut sorted_by_downloads = app.all_crates.clone();
    sorted_by_downloads.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    let top_5 = sorted_by_downloads.iter().take(5);

    let mut lines = vec![];

    // Banner
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "╔═══════════════════════════════════════════════════════╗",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled(
            "║   ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "📊 RATATUI ECOSYSTEM STATISTICS",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "            ║",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(Span::styled(
        "╚═══════════════════════════════════════════════════════╝",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Overview
    lines.push(Line::from(Span::styled(
        "📦 Overview:",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::raw("  Total Packages:     "),
        Span::styled(
            format!("{}", total),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  ⭐ Core Libraries:  "),
        Span::styled(
            format!("{}", core),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  🌍 Community:       "),
        Span::styled(
            format!("{}", community),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // Download stats
    lines.push(Line::from(Span::styled(
        "📈 Download Statistics:",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::raw("  Total Downloads:    "),
        Span::styled(
            format_number(total_downloads),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Average/Crate:      "),
        Span::styled(
            format_number(avg_downloads),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Weekly Downloads:   "),
        Span::styled(
            format_number(total_weekly),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // Simple bar chart
    lines.push(Line::from(Span::styled(
        "📊 Distribution:",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));

    let core_pct = (core as f64 / total as f64 * 100.0) as usize;
    let community_pct = 100 - core_pct;

    let core_bar = "█".repeat(core_pct / 2);
    let community_bar = "█".repeat(community_pct / 2);

    lines.push(Line::from(vec![
        Span::raw("  Core:      ["),
        Span::styled(core_bar, Style::default().fg(Color::Yellow)),
        Span::raw(format!("] {}%", core_pct)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Community: ["),
        Span::styled(community_bar, Style::default().fg(Color::Green)),
        Span::raw(format!("] {}%", community_pct)),
    ]));
    lines.push(Line::from(""));

    // Top 5
    lines.push(Line::from(Span::styled(
        "🏆 Top 5 Most Downloaded:",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));

    for (i, crate_pkg) in top_5.enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };

        lines.push(Line::from(vec![
            Span::raw(format!("  {} ", medal)),
            Span::styled(
                format!("{:20}", crate_pkg.name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:>10}", format_number(crate_pkg.downloads)),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "💡 Tip: Press TAB to go back to list view",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(vec![
                    Span::styled(" 📊 ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        "Statistics",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
                .style(Style::default()),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_command_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = match app.mode {
        Mode::Normal => Text::from(Line::from(vec![
            Span::styled(
                " NORMAL ",
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(&app.status_message, Style::default().fg(Color::White)),
        ])),
        Mode::Command => Text::from(Line::from(vec![
            Span::styled(
                " COMMAND ",
                Style::default()
                    .bg(Color::Green)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " :",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&app.command_input, Style::default().fg(Color::Yellow)),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
        ])), // Mode::Try => {
             //     Text::from(Line::from(vec![
             //         Span::styled(
             //             " TRY ",
             //             Style::default()
             //                 .bg(Color::Magenta)
             //                 .fg(Color::Black)
             //                 .add_modifier(Modifier::BOLD),
             //         ),
             //         Span::raw(" "),
             //         Span::styled(&app.status_message, Style::default().fg(Color::Magenta)),
             //     ]))
             // }
    };

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

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
                    KeyCode::Char('G') => app
                        .list_state
                        .select(Some(app.filtered_crates.len().saturating_sub(1))),

                    // Views
                    KeyCode::Tab => {
                        app.view = match app.view {
                            View::List => View::Stats,
                            View::Stats => View::List,
                            View::Help => View::List,
                        };
                    }
                    KeyCode::Char('?') => {
                        app.view = if app.view == View::Help {
                            View::List
                        } else {
                            View::Help
                        };
                    }

                    // Commands
                    KeyCode::Char(':') | KeyCode::Char('/') => {
                        app.mode = Mode::Command;
                        app.command_input.clear();
                        if key.code == KeyCode::Char('/') {
                            app.command_input.push_str("search ");
                        }
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
                // Mode::Try => match key.code {
                //     KeyCode::Char('y') | KeyCode::Char('Y') => {
                //         if let Some(crate_name) = app.try_crate.clone() {
                //             // Update status to show we're working
                //             app.status_message = format!("🔄 Setting up try environment for {}... (this may take a moment)", crate_name);
                //             app.mode = Mode::Normal; // Exit try mode immediately
                //
                //             // Force redraw to show the status
                //             // terminal.draw(|f| ui(f, app))?;
                //
                //             // Now do the work
                //             match setup_try_environment(&crate_name) {
                //                 Ok(temp_dir) => {
                //                     app.try_temp_dir = Some(temp_dir.clone());
                //                     app.status_message = format!(
                //                         "✅ Ready! Run:  cd {}  &&  cargo run  |  Cleanup:  rm -rf /tmp/ratcrate-try/{}",
                //                         temp_dir, crate_name
                //                     );
                //                 }
                //                 Err(e) => {
                //                     app.status_message = format!("❌ Error: {}", e);
                //                 }
                //             }
                //
                //             // Redraw with final status
                //             // terminal.draw(|f| ui(f, app))?;
                //         } else {
                //             app.status_message = "No crate selected for try mode".to_string();
                //             app.mode = Mode::Normal;
                //         }
                //         app.try_crate = None;
                //     }
                //     KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                //         app.mode = Mode::Normal;
                //         app.try_crate = None;
                //         app.status_message = "Try cancelled".to_string();
                //     }
                //     _ => {}
                // },
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

// ============================================================================
// Note: cache.rs and types.rs are EXACTLY the same as ratcrate-cli
// Just copy them from the CLI project!
// ============================================================================
