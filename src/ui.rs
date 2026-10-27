use crate::db::{Db, EntryRow};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::io;

// ============================================================
// COLOR SCHEME
// ============================================================
struct Colors {
    highlight_bg: Color,   // Background of selected item
    unread_text: Color,    // Unread article text
    read_text: Color,      // Read article text (dimmed)
    search_border: Color,  // Search bar border
    search_text: Color,    // Search bar text
    help_title: Color,     // Section titles in help
    help_key: Color,       // Key names in help
    help_border: Color,    // Help modal border
    meta_label: Color,     // "Feed:", "Title:" etc.
    meta_value: Color,     // metadata values
}

impl Colors {
    fn default() -> Self {
        Self {
            highlight_bg: Color::Rgb(160, 80, 80),
            unread_text: Color::White,
            read_text: Color::Rgb(196, 196, 196),
            search_border: Color::Rgb(255, 200, 0),    // Golden yellow
            search_text: Color::White,
            help_title: Color::Rgb(255, 200, 0),       // Golden yellow
            help_key: Color::Rgb(100, 200, 255),       // Light cyan/blue
            help_border: Color::Rgb(255, 200, 0),      // Golden yellow
            meta_label: Color::Rgb(255, 200, 0),       // Golden yellow
            meta_value: Color::White,
        }
    }
}

pub fn run(db: &Db) -> Result<(), Box<dyn std::error::Error>> {
    let entries = db.all_entries()?;
    let mut app = App::new(entries);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, db, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    res
}

struct ArticleView {
    feed: String,
    title: String,
    date: String,
    link: String,
    text: String,
}

struct App {
    entries: Vec<EntryRow>,
    search_query: String,
    is_searching: bool,
    selected_feed: Option<i64>,
    list_state: ListState,
    view: View,
    current_article: Option<ArticleView>,
    scroll_offset: u16,
    feed_list: Vec<(i64, String, i64)>,
    feed_list_state: ListState,
}

enum View {
    List,
    Article,
    FeedSelect,
    Help,
}

impl App {
    fn new(entries: Vec<EntryRow>) -> Self {
        let mut ls = ListState::default();
        if !entries.is_empty() {
            ls.select(Some(0));
        }
        Self {
            entries,
            search_query: String::new(),
            is_searching: false,
            selected_feed: None,
            list_state: ls,
            view: View::List,
            current_article: None,
            scroll_offset: 0,
            feed_list: Vec::new(),
            feed_list_state: ListState::default(),
        }
    }

    fn filtered_entries(&self) -> Vec<&EntryRow> {
        self.entries.iter().filter(|e| {
            let matches_query = if self.search_query.is_empty() {
                true
            } else {
                let q = self.search_query.to_lowercase();
                e.title.to_lowercase().contains(&q)
            };
            let matches_feed = match self.selected_feed {
                Some(feed_id) => {
                    if let Some((_, name, _)) = self.feed_list.iter().find(|(id, _, _)| *id == feed_id) {
                        &e.feed == name
                    } else {
                        true
                    }
                }
                None => true,
            };
            matches_query && matches_feed
        }).collect()
    }

    fn next(&mut self) {
        let filtered = self.filtered_entries();
        if filtered.is_empty() { return; }
        let i = self.list_state.selected().map(|i| (i + 1) % filtered.len()).unwrap_or(0);
        self.list_state.select(Some(i));
    }

    fn prev(&mut self) {
        let filtered = self.filtered_entries();
        if filtered.is_empty() { return; }
        let i = self.list_state.selected()
            .map(|i| if i == 0 { filtered.len() - 1 } else { i - 1 })
            .unwrap_or(0);
        self.list_state.select(Some(i));
    }

    fn reset_selection(&mut self) {
        if !self.filtered_entries().is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    fn open_feed_select(&mut self, db: &Db) {
        if let Ok(feeds) = db.feeds_with_unread() {
            self.feed_list = feeds;
            let mut ls = ListState::default();
            if !self.feed_list.is_empty() {
                ls.select(Some(0));
            }
            self.feed_list_state = ls;
            self.view = View::FeedSelect;
        }
    }
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    db: &Db,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>>
where
    <B as Backend>::Error: 'static,
{
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Help closes on any key
            if let View::Help = app.view {
                app.view = View::List;
                continue;
            }

            match app.view {
                View::List => match key.code {
                    KeyCode::F(1) | KeyCode::Char('?') => {
                        app.view = View::Help;
                    }
                    KeyCode::Char('/') => {
                        app.is_searching = true;
                        app.search_query.clear();
                        app.reset_selection();
                    }
                    KeyCode::Char('f') if !app.is_searching => {
                        app.open_feed_select(db);
                    }
                    KeyCode::Char('a') if !app.is_searching => {
                        app.selected_feed = None;
                        app.reset_selection();
                    }
                    KeyCode::Char('q') if !app.is_searching => return Ok(()),
                    KeyCode::Esc => {
                        if app.is_searching {
                            app.is_searching = false;
                            app.search_query.clear();
                            app.reset_selection();
                        } else {
                            return Ok(());
                        }
                    }
                    KeyCode::Backspace if app.is_searching => {
                        app.search_query.pop();
                        app.reset_selection();
                    }
                    KeyCode::Char('h') if app.is_searching && key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.search_query.pop();
                        app.reset_selection();
                    }
                    KeyCode::Char(c) if app.is_searching => {
                        app.search_query.push(c);
                        app.reset_selection();
                    }
                    KeyCode::Enter | KeyCode::Right => {
                        if let Some(i) = app.list_state.selected() {
                            let filtered = app.filtered_entries();
                            let entry = &filtered[i];
                            let id = entry.id.clone();

                            let text = match db.get_content(&id) {
                                Ok(Some(t)) if !t.is_empty() => t,
                                _ => "Text not downloaded yet. Run: newsreader fetch".into(),
                            };

                            app.current_article = Some(ArticleView {
                                feed: entry.feed.clone(),
                                title: entry.title.clone(),
                                date: format_date(&entry.published),
                                link: entry.url.clone(),
                                text,
                            });
                            let _ = db.mark_read(&id);
                            if let Some(entry) = app.entries.iter_mut().find(|e| e.id == id) {
                            	entry.status = "read".to_string();
                            	}
                            app.view = View::Article;
                            app.scroll_offset = 0;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.prev(),
                    _ => {}
                },
                View::Article => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Left => {
                        app.view = View::List;
                        app.current_article = None;
                        app.scroll_offset = 0;
                        if !app.search_query.is_empty() {
                            app.is_searching = true;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::PageDown | KeyCode::Char(' ') => {
                        let area = terminal.size().unwrap_or_default();
                        let header_height = article_header_height(&app.current_article, area.width);
                        let body_height = area.height.saturating_sub(header_height);

                        let text = app.current_article.as_ref().map(|a| a.text.as_str()).unwrap_or("");
                        let formatted = format_text(text, area.width);
                        let total_lines = formatted.lines().count() as u16;
                        let max_scroll = total_lines.saturating_sub(body_height.saturating_sub(1));

                        let step = if matches!(key.code, KeyCode::PageDown | KeyCode::Char(' ')) {
                            body_height.saturating_sub(1)
                        } else {
                            5
                        };
                        app.scroll_offset = (app.scroll_offset + step).min(max_scroll);
                    }
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::PageUp => {
                        let step = if matches!(key.code, KeyCode::PageUp) {
                            let area = terminal.size().unwrap_or_default();
                            let header_height = article_header_height(&app.current_article, area.width);
                            area.height.saturating_sub(header_height).saturating_sub(1)
                        } else {
                            5
                        };
                        app.scroll_offset = app.scroll_offset.saturating_sub(step);
                    }
                    _ => {}
                },
                View::FeedSelect => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.view = View::List;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let total = app.feed_list.len() + 1;
                        let i = app.feed_list_state.selected().map(|i| (i + 1) % total).unwrap_or(0);
                        app.feed_list_state.select(Some(i));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let total = app.feed_list.len() + 1;
                        let i = app.feed_list_state.selected()
                            .map(|i| if i == 0 { total - 1 } else { i - 1 })
                            .unwrap_or(0);
                        app.feed_list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(i) = app.feed_list_state.selected() {
                            if i == 0 {
                                app.selected_feed = None;
                            } else {
                                app.selected_feed = Some(app.feed_list[i - 1].0);
                            }
                            app.view = View::List;
                            app.reset_selection();
                        }
                    }
                    _ => {}
                },
                View::Help => {} // handled above
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let colors = Colors::default();

    match app.view {
        View::List => {
            let chunks = if app.is_searching {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                    .split(f.area())
            } else {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0)].as_ref())
                    .split(f.area())
            };

            if app.is_searching {
                let search_bar = Paragraph::new(app.search_query.clone())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Search by title (Esc to cancel) ")
                            .border_style(Style::default().fg(colors.search_border)),
                    )
                    .style(Style::default().fg(colors.search_text));
                f.render_widget(search_bar, chunks[0]);
            }

            let filtered = app.filtered_entries();
            let items: Vec<ListItem> = filtered
                .iter()
                .map(|e| {
                    let date = format_date(&e.published);
                    let short_feed = shorten_feed_name(&e.feed);
                    let content = format!("{}  [{}]  {}", date, short_feed, e.title);
                    let style = if e.status == "read" {
                        Style::default().fg(colors.read_text)
                    } else {
                        Style::default().fg(colors.unread_text).add_modifier(Modifier::BOLD)
                    };
                    ListItem::new(content).style(style)
                })
                .collect();

            let total = app.entries.len();
            let shown = filtered.len();
            let mut title_parts = Vec::new();
            title_parts.push(format!("shown: {} of {}", shown, total));
            if let Some(feed_id) = app.selected_feed {
                if let Some((_, name, _)) = app.feed_list.iter().find(|(id, _, _)| *id == feed_id) {
                    title_parts.push(format!("feed: {}", name));
                }
            }
            if !app.search_query.is_empty() {
                title_parts.push(format!("search: '{}'", app.search_query));
            }
            let title = format!(" Feed ({}) — /: search, f: feed, a: all, ↑/↓: navigate, Enter/→: read, F1/?: help, Esc/q: quit ", title_parts.join(" | "));

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(
                    Style::default()
                        .bg(colors.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                )
                .highlight_symbol(">> ");

            let list_area = if app.is_searching { chunks[1] } else { chunks[0] };
            f.render_stateful_widget(list, list_area, &mut app.list_state);
        }
        View::Article => {
            let article = app.current_article.as_ref().unwrap();
            let header_height = article_header_height(&app.current_article, f.area().width);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(header_height),
                    Constraint::Min(0),
                ])
                .split(f.area());

            // --- Header (metadata) ---
            let link_line = if article.link.is_empty() {
                Line::from(vec![
                    Span::styled("Link:  ", Style::default().fg(colors.meta_label)),
                    Span::raw("(none)"),
                ])
            } else {
                Line::from(vec![
                    Span::styled("Link:  ", Style::default().fg(colors.meta_label)),
                    Span::styled(
                        &article.link,
                        Style::default()
                            .fg(colors.meta_value)
                            .add_modifier(Modifier::UNDERLINED),
                    ),
                ])
            };

            let header = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("Feed:  ", Style::default().fg(colors.meta_label)),
                    Span::styled(&article.feed, Style::default().fg(colors.meta_value)),
                ]),
                Line::from(vec![
                    Span::styled("Title: ", Style::default().fg(colors.meta_label)),
                    Span::styled(
                        &article.title,
                        Style::default()
                            .fg(colors.meta_value)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Date:  ", Style::default().fg(colors.meta_label)),
                    Span::styled(&article.date, Style::default().fg(colors.meta_value)),
                ]),
                link_line,
                Line::from(Span::styled(
                    "─".repeat(f.area().width.saturating_sub(2) as usize),
                    Style::default().fg(colors.help_border),
                )),
            ])
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Article ")
                    .border_style(Style::default().fg(colors.help_border)),
            );
            f.render_widget(header, chunks[0]);

            // --- Body (scrollable text) ---
            let formatted = format_text(&article.text, f.area().width);
            let body = Paragraph::new(formatted)
                .wrap(Wrap { trim: false })
                .scroll((app.scroll_offset, 0));
            f.render_widget(body, chunks[1]);
        }
        View::FeedSelect => {
            let mut items: Vec<ListItem> = vec![ListItem::new("📚 All feeds")];
            for (id, name, unread) in &app.feed_list {
                items.push(ListItem::new(format!("  {} — unread: {}", name, unread)));
                let _ = id;
            }
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" Select feed (↑/↓: navigate, Enter: select, Esc: cancel) "))
                .highlight_style(
                    Style::default()
                        .bg(colors.highlight_bg)
                        .add_modifier(Modifier::BOLD)
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, f.area(), &mut app.feed_list_state);
        }
        View::Help => {
            render_help(f);
        }
    }
}

fn render_help(f: &mut ratatui::Frame) {
    let colors = Colors::default();
    let area = f.area();

    // Modal window size
    let width = 60u16;
    let height = 20u16;
    let x = area.width.saturating_sub(width) / 2;
    let y = area.height.saturating_sub(height) / 2;
    let popup_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    // Dim background
    f.render_widget(Clear, popup_area);

    let help_lines = vec![
        Line::from(Span::styled("Navigation", Style::default().fg(colors.help_title).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  ↑/↓  or  j/k", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — navigate list, scroll text"),
        ]),
        Line::from(vec![
            Span::styled("  Enter  or  →", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — open article"),
        ]),
        Line::from(vec![
            Span::styled("  Esc or ←", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — return to list"),
        ]),
        Line::from(Span::styled("Search & filters", Style::default().fg(colors.help_title).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  /", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — search by title"),
        ]),
        Line::from(vec![
            Span::styled("  f", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — select feed (filter by source)"),
        ]),
        Line::from(vec![
            Span::styled("  a", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — show all feeds (reset filter)"),
        ]),
        Line::from(Span::styled("Text copying", Style::default().fg(colors.help_title).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  Shift+mouse selection", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — select text"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Shift+C", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — copy selected text"),
        ]),
        Line::from(Span::styled("Other", Style::default().fg(colors.help_title).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  q  or  Esc", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD)),
            Span::raw("  — exit program"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Author", Style::default().fg(colors.help_key).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::raw("  hektr <hektr992@gmail.com>"),
        ]),
        Line::from(vec![
            Span::raw("  https://github.com/hektr"),
        ]),
    ];

    let paragraph = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help (any key to close) ")
                .border_style(Style::default().fg(colors.help_border)),
        );
    f.render_widget(paragraph, popup_area);
}

/// Calculates the height of the article header (metadata block).
/// Rough estimate: 4 lines of metadata + 1 separator + possible wraps for long title/link.
fn article_header_height(article: &Option<ArticleView>, terminal_width: u16) -> u16 {
    let a = match article {
        Some(a) => a,
        None => return 0,
    };

    let usable_width = terminal_width.saturating_sub(4) as usize; // borders + padding
    let label_width = 7; // "Feed:  " etc.
    let content_width = usable_width.saturating_sub(label_width);

    let wraps_for = |s: &str| -> u16 {
        if content_width == 0 || s.is_empty() {
            return 1;
        }
        ((s.len().saturating_sub(1)) / content_width) as u16 + 1
    };

    let feed_lines = wraps_for(&a.feed);
    let title_lines = wraps_for(&a.title);
    let date_lines = 1;
    let link_lines = wraps_for(&a.link);
    let separator = 1;

    feed_lines + title_lines + date_lines + link_lines + separator + 1
}

/// Format date from this one (2026-10-24) to this one (Oct 24)
fn format_date(date_str: &str) -> String {
    let date_part = if date_str.len() >= 10 {
        &date_str[..10]
    } else {
        return "?".to_string();
    };

    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return date_part.to_string();
    }

    let year = parts[0];
    let month = parts[1];
    let day = parts[2];

    let month_name = match month {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => return date_part.to_string(),
    };

    let day_clean = day.trim_start_matches('0');
    format!("{} {} {}", day_clean, month_name, year)
}

fn format_text(text: &str, terminal_width: u16) -> String {
    let target = 75usize;
    let width = if terminal_width < target as u16 {
        (terminal_width.saturating_sub(4)) as usize
    } else {
        target
    };

    let paragraphs = normalize_paragraphs(text);
    let indent = "  ";

    let formatted: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            let cleaned = p.split_whitespace().collect::<Vec<_>>().join("  ");
            textwrap::wrap(&cleaned, width)
                .into_iter()
                .map(|l| format!("{}{}", indent, l))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();

    formatted.join("\n\n")
}

fn normalize_paragraphs(text: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current = String::new();

    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.trim().to_string());
                current = String::new();
            }
            continue;
        }

        let is_sentence_end = current.ends_with('.')
            || current.ends_with('!')
            || current.ends_with('?')
            || current.ends_with('"');

        let starts_with_capital = trimmed
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);

        if !current.is_empty() && is_sentence_end && starts_with_capital {
            paragraphs.push(current.trim().to_string());
            current = trimmed.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(trimmed);
        }
    }

    if !current.is_empty() {
        paragraphs.push(current.trim().to_string());
    }

    paragraphs
}

fn shorten_feed_name(feed: &str) -> String {
    if feed.starts_with("http") {
        feed.trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .split('/')
            .next()
            .unwrap_or(feed)
            .to_string()
    } else {
        feed.to_string()
    }
}
