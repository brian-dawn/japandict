use clap::Parser;
use dictionary_data::{WORD_COUNT, KANJI_STRINGS_COUNT, KANA_STRINGS_COUNT, ENGLISH_STRINGS_COUNT};
use japandict_core::{search_dictionary, WordEntry};
use rustyline::{Editor, Result};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, ClearType},
    tty::IsTty,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io::{self, stdout, Write};

#[derive(Parser)]
#[command(name = "dict_cli")]
#[command(about = "Japanese dictionary CLI using JMDict")]
struct Args {
    /// Search term(s)
    query: Vec<String>,
    
    /// Maximum number of results to display
    #[arg(short, long, default_value = "10")]
    limit: usize,
    
    /// Interactive mode (ignore query if provided)
    #[arg(short, long)]
    interactive: bool,
    
    /// Live search mode with real-time results as you type
    #[arg(long)]
    live: bool,
    
    /// TUI mode with ratatui interface
    #[arg(long)]
    tui: bool,
}

fn search_and_display(query: &str, limit: usize) {
    if query.trim().is_empty() {
        return;
    }
    
    let start = std::time::Instant::now();
    let results = search_dictionary(query);
    let duration = start.elapsed();
    
    println!("🔍 Search Results for \"{}\"", query);
    println!("Found {} results in {:?}", results.len(), duration);
    println!("{}", "─".repeat(60));
    
    for (i, entry) in results.iter().enumerate() {
        if i >= limit {
            println!("... and {} more results", results.len() - limit);
            break;
        }
        
        print!("{:2}. ", i + 1);
        
        if !entry.kanji.is_empty() {
            print!("{}", entry.kanji.join(", "));
            if !entry.kana.is_empty() {
                print!(" ({})", entry.kana.join(", "));
            }
        } else if !entry.kana.is_empty() {
            print!("{}", entry.kana.join(", "));
        }
        
        if !entry.english.is_empty() {
            print!(" → {}", entry.english[..entry.english.len().min(3)].join("; "));
        }
        
        if !entry.pos.is_empty() {
            print!(" [{}]", entry.pos.join(", "));
        }
        
        if entry.is_common {
            print!(" ⭐");
        }
        
        println!();
    }
    println!();
}

struct App {
    query: String,
    cursor_pos: usize,
    results: Vec<WordEntry>,
    search_time: Option<std::time::Duration>,
    scroll: usize,
    should_quit: bool,
}

impl App {
    fn new() -> App {
        App {
            query: String::new(),
            cursor_pos: 0,
            results: Vec::new(),
            search_time: None,
            scroll: 0,
            should_quit: false,
        }
    }

    fn search(&mut self) {
        if self.query.trim().is_empty() {
            self.results.clear();
            self.search_time = None;
            return;
        }

        let start = std::time::Instant::now();
        self.results = search_dictionary(&self.query);
        self.search_time = Some(start.elapsed());
        self.scroll = 0;
    }

    fn handle_input(&mut self, key: KeyEvent) {
        use crossterm::event::KeyModifiers;
        
        match (key.code, key.modifiers) {
            // Quit commands
            (KeyCode::Char('q'), KeyModifiers::NONE) => self.should_quit = true,
            (KeyCode::Esc, _) => self.should_quit = true,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.should_quit = true,
            
            // Readline-style cursor movement
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.cursor_pos = 0;
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.cursor_pos = self.query.len();
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) | (KeyCode::Right, _) => {
                if self.cursor_pos < self.query.len() {
                    self.cursor_pos += 1;
                }
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) | (KeyCode::Left, _) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            
            // Readline-style result navigation
            (KeyCode::Char('n'), KeyModifiers::CONTROL) | (KeyCode::Down, _) => {
                if self.scroll < self.results.len().saturating_sub(1) {
                    self.scroll += 1;
                }
            }
            (KeyCode::Char('p'), KeyModifiers::CONTROL) | (KeyCode::Up, _) => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                }
            }
            
            // Readline-style editing
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.query.truncate(self.cursor_pos);
                self.search();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.query.drain(0..self.cursor_pos);
                self.cursor_pos = 0;
                self.search();
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                if self.cursor_pos < self.query.len() {
                    self.query.remove(self.cursor_pos);
                    self.search();
                }
            }
            (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.query.remove(self.cursor_pos);
                    self.search();
                }
            }
            
            // Regular character input
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.query.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                self.search();
            }
            
            // Page navigation
            (KeyCode::PageDown, _) => {
                self.scroll = (self.scroll + 10).min(self.results.len().saturating_sub(1));
            }
            (KeyCode::PageUp, _) => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            
            _ => {}
        }
    }
}

fn run_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                app.handle_input(key);
                if app.should_quit {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.size());

    // Results area
    if !app.results.is_empty() {
        let items: Vec<ListItem> = app.results
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let mut spans = vec![
                    Span::styled(format!("{:2}. ", i + 1), Style::default().fg(Color::DarkGray)),
                ];

                // Kanji in bold magenta
                if !entry.kanji.is_empty() {
                    spans.push(Span::styled(
                        entry.kanji.join(", "),
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ));
                    
                    // Kana in cyan
                    if !entry.kana.is_empty() {
                        spans.push(Span::styled(
                            format!(" ({})", entry.kana.join(", ")),
                            Style::default().fg(Color::Cyan),
                        ));
                    }
                } else if !entry.kana.is_empty() {
                    spans.push(Span::styled(
                        entry.kana.join(", "),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ));
                }

                // English in green
                if !entry.english.is_empty() {
                    spans.push(Span::styled(" → ", Style::default().fg(Color::DarkGray)));
                    spans.push(Span::styled(
                        entry.english[..entry.english.len().min(3)].join("; "),
                        Style::default().fg(Color::Green),
                    ));
                }

                // Part of speech in dim style
                if !entry.pos.is_empty() {
                    spans.push(Span::styled(
                        format!(" [{}]", entry.pos.join(", ")),
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                    ));
                }

                // Common word indicator
                if entry.is_common {
                    spans.push(Span::styled(
                        " ⭐",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let results_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Results: {} found", app.results.len()))
                    .border_style(Style::default().fg(Color::White)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(results_list, chunks[0], &mut ratatui::widgets::ListState::default().with_selected(Some(app.scroll)));
    } else {
        let no_results = Paragraph::new("Type to search Japanese dictionary...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Results")
                    .border_style(Style::default().fg(Color::White)),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(no_results, chunks[0]);
    }

    // Search input at bottom with cursor
    let search_text = if app.query.is_empty() {
        "Search: █".to_string()
    } else {
        let (before_cursor, after_cursor) = app.query.split_at(app.cursor_pos);
        if app.cursor_pos >= app.query.len() {
            format!("Search: {}█", app.query)
        } else {
            format!("Search: {}█{}", before_cursor, after_cursor)
        }
    };
    
    let help_text = "C-a:start C-e:end C-k:kill C-u:clear C-n/p:nav q/C-c:quit";
    
    let search_input = Paragraph::new(vec![
        Line::from(search_text),
        Line::from(Span::styled(help_text, Style::default().fg(Color::DarkGray))),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .style(Style::default().fg(Color::White));
    f.render_widget(search_input, chunks[1]);
}

fn format_entry(entry: &WordEntry) -> String {
    let mut output = String::new();
    
    if !entry.kanji.is_empty() {
        output.push_str(&entry.kanji.join(", "));
        if !entry.kana.is_empty() {
            output.push_str(&format!(" ({})", entry.kana.join(", ")));
        }
    } else if !entry.kana.is_empty() {
        output.push_str(&entry.kana.join(", "));
    }
    
    if !entry.english.is_empty() {
        output.push_str(" — ");
        output.push_str(&entry.english[..entry.english.len().min(3)].join("; "));
    }
    
    if !entry.pos.is_empty() {
        output.push_str(&format!(" [{}]", entry.pos.join(", ")));
    }
    
    if entry.is_common {
        output.push_str(" ⭐");
    }
    
    output
}

fn live_search() -> Result<()> {
    let mut stdout = io::stdout();
    
    // Check if we're in an interactive terminal
    if !stdout.is_tty() {
        eprintln!("Error: Live search mode requires an interactive terminal");
        return Ok(());
    }
    
    terminal::enable_raw_mode().map_err(|e| {
        rustyline::error::ReadlineError::Io(std::io::Error::new(
            std::io::ErrorKind::Other, 
            format!("Failed to enable raw mode: {}", e)
        ))
    })?;
    
    execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
    
    println!("JMDict Live Search - {} words loaded", WORD_COUNT);
    println!("Type to search, Ctrl+C to exit\n");
    
    let mut query = String::new();
    let mut last_query = String::new();
    let mut results = Vec::new();
    
    loop {
        // Only search if query changed
        if query != last_query {
            if query.trim().is_empty() {
                results.clear();
            } else {
                let start = std::time::Instant::now();
                results = search_dictionary(&query);
                let duration = start.elapsed();
                
                // Clear previous results
                execute!(stdout, cursor::MoveTo(0, 3), terminal::Clear(ClearType::FromCursorDown))?;
                
                println!("Search: {} ({} results in {:?})\n", query, results.len(), duration);
                
                // Show top 10 results
                for entry in results.iter().take(10) {
                    println!("{}", format_entry(entry));
                }
                
                if results.len() > 10 {
                    println!("\n... and {} more results", results.len() - 10);
                }
            }
            last_query = query.clone();
        }
        
        // Position cursor at search prompt
        execute!(stdout, cursor::MoveTo(8 + query.len() as u16, 1))?;
        stdout.flush()?;
        
        // Read input
        match event::read()? {
            Event::Key(KeyEvent { code, modifiers, .. }) => {
                match (code, modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        break;
                    }
                    (KeyCode::Char(c), _) => {
                        query.push(c);
                        execute!(stdout, cursor::MoveTo(0, 1))?;
                        print!("Search: {}", query);
                        stdout.flush()?;
                    }
                    (KeyCode::Backspace, _) => {
                        if !query.is_empty() {
                            query.pop();
                            execute!(stdout, cursor::MoveTo(0, 1), terminal::Clear(ClearType::UntilNewLine))?;
                            print!("Search: {}", query);
                            stdout.flush()?;
                        }
                    }
                    (KeyCode::Enter, _) => {
                        // Enter doesn't do anything special in live search, just continue
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    terminal::disable_raw_mode()?;
    println!("\nGoodbye!");
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Build search indices on startup for fast searches
    print!("Building search indices... ");
    std::io::stdout().flush().unwrap();
    let start = std::time::Instant::now();
    japandict_core::search::build_search_indices();
    println!("done in {:?}", start.elapsed());
    
    // TUI mode with ratatui
    if args.tui {
        return run_tui();
    }
    
    // Live search mode  
    if args.live {
        return live_search();
    }
    
    println!("JMDict CLI - {} words loaded", WORD_COUNT);
    println!("Dictionary contains {} kanji, {} kana, {} english terms", 
        KANJI_STRINGS_COUNT, KANA_STRINGS_COUNT, ENGLISH_STRINGS_COUNT);
    println!();
    
    // If query provided and not interactive mode, search and exit
    if !args.query.is_empty() && !args.interactive {
        let query = args.query.join(" ");
        search_and_display(&query, args.limit);
        return Ok(());
    }
    
    // Interactive mode with readline
    let mut rl: Editor<(), _> = Editor::new()?;
    
    println!("Interactive mode - type Japanese or English to search (Ctrl+C to exit)");
    
    loop {
        let readline = rl.readline("dict> ");
        match readline {
            Ok(line) => {
                if line.trim() == "quit" || line.trim() == "exit" {
                    break;
                }
                rl.add_history_entry(line.as_str())?;
                search_and_display(&line, args.limit);
            }
            Err(_) => {
                println!("Goodbye!");
                break;
            }
        }
    }
    
    Ok(())
}