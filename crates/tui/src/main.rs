//! Player preview: search a player, then see how their stats actually changed in later editions.
//! Usage: tui [history.db]. Keys: type to search, Up/Down, Enter open, Left/Right pick season, Esc back, Ctrl-C quit.
use engine::{career, edition_label, player_name, search, Connection, Hit, Snapshot, KEEPER, OUTFIELD};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, List, ListItem, ListState, Paragraph, Row, Table},
    Frame,
};

struct App {
    conn: Connection,
    query: String,
    hits: Vec<Hit>,
    list: ListState,
    // player view
    open: bool,
    name: String,
    career: Vec<Snapshot>,
    col: usize,    // selected edition column
    scroll: usize, // first visible attribute row
    error: Option<String>,
}

impl App {
    fn new(conn: Connection) -> Self {
        App { conn, query: String::new(), hits: vec![], list: ListState::default(), open: false, name: String::new(),
              career: vec![], col: 0, scroll: 0, error: None }
    }

    fn refresh_search(&mut self) {
        self.hits.clear();
        if self.query.trim().len() >= 2 {
            match search(&self.conn, &self.query, 20) {
                Ok(h) => self.hits = h,
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        self.list.select(if self.hits.is_empty() { None } else { Some(0) });
    }

    fn open_player(&mut self, id: i64) {
        match (player_name(&self.conn, id), career(&self.conn, id)) {
            (Ok(n), Ok(c)) => {
                self.name = n;
                self.career = c;
                self.col = 0;
                self.scroll = 0;
                self.open = true;
            }
            (Err(e), _) | (_, Err(e)) => self.error = Some(e.to_string()),
        }
    }

    /// Returns false to quit.
    fn key(&mut self, code: KeyCode, ctrl: bool) -> bool {
        if ctrl && code == KeyCode::Char('c') {
            return false;
        }
        if self.open {
            match code {
                KeyCode::Esc => self.open = false,
                KeyCode::Left => self.col = self.col.saturating_sub(1),
                KeyCode::Right => self.col = (self.col + 1).min(self.career.len().saturating_sub(1)),
                KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down => self.scroll += 1,
                _ => {}
            }
        } else {
            match code {
                KeyCode::Char(c) => { self.query.push(c); self.refresh_search(); }
                KeyCode::Backspace => { self.query.pop(); self.refresh_search(); }
                KeyCode::Down => self.list.select(self.list.selected().map(|i| (i + 1).min(self.hits.len() - 1))),
                KeyCode::Up => self.list.select(self.list.selected().map(|i| i.saturating_sub(1))),
                KeyCode::Enter => {
                    if let Some(h) = self.list.selected().and_then(|i| self.hits.get(i)) {
                        let id = h.player_id;
                        self.open_player(id);
                    }
                }
                KeyCode::Esc => return false,
                _ => {}
            }
        }
        true
    }
}

/// "82" or "82 +2" / "82 -1" compared with the previous edition.
fn with_delta(cur: Option<i64>, prev: Option<i64>) -> Cell<'static> {
    let Some(v) = cur else { return Cell::from("-") };
    match prev.map(|p| v - p) {
        Some(d) if d > 0 => Cell::from(format!("{v} +{d}")).style(Style::new().fg(Color::Green)),
        Some(d) if d < 0 => Cell::from(format!("{v} {d}")).style(Style::new().fg(Color::Red)),
        _ => Cell::from(v.to_string()),
    }
}

fn money(v: Option<i64>, unit: (&str, f64)) -> String {
    v.map_or("-".into(), |v| format!("€{:.1}{}", v as f64 / unit.1, unit.0))
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let [top, mid, bottom, help] = Layout::vertical([
        Constraint::Length(3), Constraint::Length(9), Constraint::Min(5), Constraint::Length(2),
    ]).areas(area);

    if !app.open {
        let [input, results] = Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).areas(area);
        f.render_widget(Paragraph::new(format!("{}_", app.query)).block(Block::bordered().title(" Search player (name) ")), input);
        let items: Vec<ListItem> = app.hits.iter().map(|h| {
            ListItem::new(format!("{:<34} {} to {}   best overall {}", h.name, edition_label(h.first), edition_label(h.last), h.best))
        }).collect();
        let mut state = app.list.clone();
        f.render_stateful_widget(
            List::new(items).block(Block::bordered().title(" Results ")).highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
            results, &mut state);
        if let Some(e) = &app.error {
            f.render_widget(Paragraph::new(e.as_str()).style(Style::new().fg(Color::Red)), help);
        }
        return;
    }

    let c = &app.career;
    let last = c.last();
    f.render_widget(
        Paragraph::new(format!("{}   {}   {} to {}", app.name, last.map_or("", |s| s.positions.as_str()),
            edition_label(c[0].version), edition_label(c[c.len() - 1].version)))
            .block(Block::bordered().title(" Player ")),
        top);

    let mut widths = vec![Constraint::Length(16)];
    widths.extend(c.iter().map(|_| Constraint::Length(9)));
    let header = |title: &str| {
        let mut cells = vec![Cell::from(title.to_string())];
        cells.extend(c.iter().enumerate().map(|(i, s)| {
            let st = if i == app.col { Style::new().add_modifier(Modifier::REVERSED) } else { Style::new().add_modifier(Modifier::BOLD) };
            Cell::from(edition_label(s.version)).style(st)
        }));
        Row::new(cells)
    };
    let prev = |i: usize| i.checked_sub(1).map(|p| &c[p]);
    let line = |label: &str, cell: &dyn Fn(usize, &Snapshot) -> Cell<'static>| {
        let mut cells = vec![Cell::from(label.to_string())];
        cells.extend(c.iter().enumerate().map(|(i, s)| cell(i, s)));
        Row::new(cells)
    };
    let summary = vec![
        line("Age", &|_, s| Cell::from(s.age.map_or("-".into(), |a| a.to_string()))),
        line("Overall", &|i, s| with_delta(s.overall, prev(i).and_then(|p| p.overall))),
        line("Potential", &|i, s| with_delta(s.potential, prev(i).and_then(|p| p.potential))),
        line("Club", &|_, s| Cell::from(s.club.clone().unwrap_or("-".into()).chars().take(8).collect::<String>())),
        line("Value", &|_, s| Cell::from(money(s.value, ("M", 1e6)))),
        line("Wage", &|_, s| Cell::from(money(s.wage, ("k", 1e3)))),
    ];
    f.render_widget(Table::new(summary, widths.clone()).header(header("")).block(Block::bordered().title(" Career ")), mid);

    // Goalkeepers show keeper stats, everyone else the outfield ones.
    let is_gk = last.is_some_and(|s| s.positions.split(',').any(|p| p.trim() == "GK"));
    let list = if is_gk { KEEPER } else { OUTFIELD };
    let rows: Vec<Row> = list.iter().skip(app.scroll.min(list.len() - 1)).map(|&a| {
        line(a.label(), &|i, s| with_delta(s.attrs[a].map(i64::from), prev(i).and_then(|p| p.attrs[a].map(i64::from))))
    }).collect();
    f.render_widget(Table::new(rows, widths).header(header("Attributes")).block(Block::bordered().title(" Attributes (change vs previous edition) ")), bottom);

    let sel = &c[app.col];
    let msg = match c.get(app.col + 1) {
        Some(n) => format!("From {}: overall {} -> {} next edition, potential {} -> {}",
            edition_label(sel.version), sel.overall.unwrap_or(0), n.overall.unwrap_or(0), sel.potential.unwrap_or(0), n.potential.unwrap_or(0)),
        None => format!("{} is the last edition we have data for", edition_label(sel.version)),
    };
    f.render_widget(Paragraph::new(format!("{msg}\nLeft/Right: season   Up/Down: scroll   Esc: back   Ctrl-C: quit")), help);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or("history.db".into());
    if !std::path::Path::new(&path).exists() {
        return Err(format!("{path} not found. Build it first: cargo run -p import --release").into());
    }
    let mut app = App::new(Connection::open(&path)?);
    let mut terminal = ratatui::init();
    let result = (|| -> std::io::Result<()> {
        loop {
            terminal.draw(|f| draw(f, &app))?;
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press && !app.key(k.code, k.modifiers.contains(KeyModifiers::CONTROL)) {
                    return Ok(());
                }
            }
        }
    })();
    ratatui::restore();
    Ok(result?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render(app: &App) -> String {
        let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
        t.draw(|f| draw(f, app)).unwrap();
        t.backend().buffer().content().chunks(120).map(|r| r.iter().map(|c| c.symbol()).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn search_then_open_shows_career_with_changes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../history.db");
        if !std::path::Path::new(path).exists() { return; } // needs `cargo run -p import` first
        let mut app = App::new(Connection::open(path).unwrap());
        for ch in "rooney".chars() { app.key(KeyCode::Char(ch), false); }
        assert!(render(&app).contains("Wayne"));
        app.key(KeyCode::Enter, false);
        let screen = render(&app);
        assert!(screen.contains("FIFA 15") && screen.contains("FIFA 21"), "{screen}");
        assert!(screen.contains("86"), "{screen}");
        assert!(screen.contains("Derby"), "{screen}");
        println!("{screen}");
    }
}
