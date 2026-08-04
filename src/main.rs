use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use regex::Regex;
use std::io;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Focus {
    Pattern,
    Text,
}

struct App {
    pattern: String,
    text: String,
    focus: Focus,
    pattern_cursor: usize,
    text_cursor: usize,
    status: String,
}

impl App {
    fn new() -> Self {
        Self {
            pattern: String::from(r"(\w+)@(\w+\.\w+)"),
            text: String::from(
                "Contact us at alice@example.com or bob@rust-lang.org for details.\nMore info: help@company.io",
            ),
            focus: Focus::Pattern,
            pattern_cursor: 0,
            text_cursor: 0,
            status: String::from("Tab: switch pane   Esc: quit   Ctrl+U: clear pane"),
        }
    }

    fn active_buffer(&mut self) -> (&mut String, &mut usize) {
        match self.focus {
            Focus::Pattern => (&mut self.pattern, &mut self.pattern_cursor),
            Focus::Text => (&mut self.text, &mut self.text_cursor),
        }
    }

    fn insert_char(&mut self, c: char) {
        let (buf, cur) = self.active_buffer();
        let idx = byte_index_from_char_index(buf, *cur);
        buf.insert(idx, c);
        *cur += 1;
    }

    fn backspace(&mut self) {
        let (buf, cur) = self.active_buffer();
        if *cur == 0 {
            return;
        }
        let start = byte_index_from_char_index(buf, *cur - 1);
        let end = byte_index_from_char_index(buf, *cur);
        buf.replace_range(start..end, "");
        *cur -= 1;
    }

    fn delete(&mut self) {
        let (buf, cur) = self.active_buffer();
        let char_count = buf.chars().count();
        if *cur >= char_count {
            return;
        }
        let start = byte_index_from_char_index(buf, *cur);
        let end = byte_index_from_char_index(buf, *cur + 1);
        buf.replace_range(start..end, "");
    }

    fn move_left(&mut self) {
        let (_buf, cur) = self.active_buffer();
        if *cur > 0 {
            *cur -= 1;
        }
    }

    fn move_right(&mut self) {
        let (buf, cur) = self.active_buffer();
        let char_count = buf.chars().count();
        if *cur < char_count {
            *cur += 1;
        }
    }

    fn move_home(&mut self) {
        let (_buf, cur) = self.active_buffer();
        *cur = 0;
    }

    fn move_end(&mut self) {
        let (buf, cur) = self.active_buffer();
        *cur = buf.chars().count();
    }

    fn clear_active(&mut self) {
        let (buf, cur) = self.active_buffer();
        buf.clear();
        *cur = 0;
    }

    fn newline_in_text(&mut self) {
        if self.focus == Focus::Text {
            self.insert_char('\n');
        }
    }

    fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Pattern => Focus::Text,
            Focus::Text => Focus::Pattern,
        };
    }
}

fn byte_index_from_char_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

fn build_highlighted_text(pattern: &str, text: &str) -> (Text<'static>, Option<String>) {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            let plain = text.lines().map(|l| Line::from(l.to_string())).collect::<Vec<_>>();
            return (Text::from(plain), Some(e.to_string()));
        }
    };
    let mut lines: Vec<Line<'static>> = Vec::new();
    for line in text.split('\n') {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut last = 0;
        for m in re.find_iter(line) {
            if m.start() > last {
                spans.push(Span::raw(line[last..m.start()].to_string()));
            }
            spans.push(Span::styled(
                m.as_str().to_string(),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            last = m.end();
        }
        if last < line.len() {
            spans.push(Span::raw(line[last..].to_string()));
        }
        if spans.is_empty() {
            spans.push(Span::raw(String::new()));
        }
        lines.push(Line::from(spans));
    }
    (Text::from(lines), None)
}

fn build_matches_view(pattern: &str, text: &str) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            lines.push(Line::from(Span::styled(
                format!("Invalid regex: {}", e),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            return Text::from(lines);
        }
    };
    let captures: Vec<_> = re.captures_iter(text).collect();
    lines.push(Line::from(vec![
        Span::styled("Matches: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(
            captures.len().to_string(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
    ]));
    for (i, cap) in captures.iter().enumerate() {
        let whole = cap.get(0).unwrap();
        lines.push(Line::from(vec![
            Span::styled(format!("  #{}  ", i + 1), Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("'{}'", whole.as_str()),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("  [{}..{}]", whole.start(), whole.end()),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        for gi in 1..cap.len() {
            if let Some(g) = cap.get(gi) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("      group {}: ", gi),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::styled(
                        format!("'{}'", g.as_str()),
                        Style::default().fg(Color::White),
                    ),
                ]));
            }
        }
    }
    if captures.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no matches)",
            Style::default().fg(Color::DarkGray),
        )));
    }
    Text::from(lines)
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(f.area());

    let (highlighted, regex_error) = build_highlighted_text(&app.pattern, &app.text);

    let pattern_border = if app.focus == Focus::Pattern {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let text_border = if app.focus == Focus::Text {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let pattern_title = match &regex_error {
        Some(_) => "Pattern [INVALID]",
        None => "Pattern",
    };

    let pattern_widget = Paragraph::new(app.pattern.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(pattern_title)
                .border_style(pattern_border),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(pattern_widget, chunks[0]);

    let text_widget = Paragraph::new(highlighted)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Test Text")
                .border_style(text_border),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(text_widget, chunks[1]);

    let matches_widget = Paragraph::new(build_matches_view(&app.pattern, &app.text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Matches & Capture Groups")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(matches_widget, chunks[2]);

    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(Color::Black).bg(Color::Cyan));
    f.render_widget(status, chunks[3]);
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if ctrl => return Ok(()),
                KeyCode::Char('u') if ctrl => app.clear_active(),
                KeyCode::Tab => app.switch_focus(),
                KeyCode::Backspace => app.backspace(),
                KeyCode::Delete => app.delete(),
                KeyCode::Left => app.move_left(),
                KeyCode::Right => app.move_right(),
                KeyCode::Home => app.move_home(),
                KeyCode::End => app.move_end(),
                KeyCode::Enter => app.newline_in_text(),
                KeyCode::Char(c) => app.insert_char(c),
                _ => {}
            }
        }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}
