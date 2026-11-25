//! `hypb ui`: the tree of `hypb state`, live and interactive in the terminal.
//!
//! The page is the same raw listing, one line an element with its type, id,
//! what it shows and its handlers, except that controls are widgets: inputs
//! can be typed into, buttons pressed, checkboxes toggled, selects, sliders
//! and tabs changed, as a wallet would (bound signals written, handlers
//! run, actions sent), as a renderer would. The page changes in place as
//! the app sends.
//!
//! The sidebar is the history of the session: every message both ways. Going
//! to one rebuilds the page as it was right after it, from scratch (see
//! [`crate::history`]), and stays there, read-only, while new messages
//! arrive, until the last row, `live`, is chosen again.

use crate::history::{Dir, History};
use crate::session::{Session, Update};
use crate::tree::{self, clip, compact};
use hyperb_core::hypereffect::client::{Client, Resolved, ResolvedTree};
use hyperb_core::hypereffect::message::parse_line;
use hyperb_core::hypereffect::{Component, Map, Value, json, serde_json};
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;
use std::io::stdout;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

/// How long an input waits after the last keystroke before running onChange.
const CHANGE_IDLE: Duration = Duration::from_millis(700);
/// The same, for slider steps.
const SLIDER_IDLE: Duration = Duration::from_millis(400);
/// How often the app is checked for messages while the user is idle.
const TICK: Duration = Duration::from_millis(40);
const SIDEBAR: u16 = 34;
const SLIDER_CELLS: u16 = 20;
const NOTICE_FOR: Duration = Duration::from_secs(6);

const FOCUS_BG: Color = Color::Indexed(237);
const MUTED: Color = Color::DarkGray;

pub fn run(mut s: Session, max_text: usize) -> Result<(), String> {
    s.history = Some(History::default());
    s.notes = Some(Vec::new());
    let mut terminal = ratatui::init();
    let _ = execute!(stdout(), EnableMouseCapture, EnableBracketedPaste);
    let mut app = App::new(s, max_text);
    let result = app.run(&mut terminal);
    let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
    result
}

/// A text being edited, with the cursor in chars.
struct Editor {
    id: String,
    text: String,
    cursor: usize,
}

#[derive(PartialEq)]
enum Edit {
    Changed,
    Moved,
    Ignored,
}

impl Editor {
    fn new(id: &str, text: String) -> Self {
        let cursor = text.chars().count();
        Editor {
            id: id.to_string(),
            text,
            cursor,
        }
    }

    fn byte(&self, at: usize) -> usize {
        self.text
            .char_indices()
            .nth(at)
            .map_or(self.text.len(), |(b, _)| b)
    }

    fn set(&mut self, text: String) {
        self.text = text;
        self.cursor = self.cursor.min(self.text.chars().count());
    }

    fn insert(&mut self, s: &str) {
        let b = self.byte(self.cursor);
        self.text.insert_str(b, s);
        self.cursor += s.chars().count();
    }

    fn remove(&mut self, from: usize, to: usize) -> Edit {
        if from >= to {
            return Edit::Moved;
        }
        let (a, b) = (self.byte(from), self.byte(to));
        self.text.replace_range(a..b, "");
        self.cursor = from;
        Edit::Changed
    }

    fn key(&mut self, k: &KeyEvent, multiline: bool) -> Edit {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let len = self.text.chars().count();
        match k.code {
            KeyCode::Char('a') if ctrl => self.cursor = 0,
            KeyCode::Char('e') if ctrl => self.cursor = len,
            KeyCode::Char('u') if ctrl => return self.remove(0, self.cursor),
            KeyCode::Char('k') if ctrl => return self.remove(self.cursor, len),
            KeyCode::Char('w') if ctrl => {
                let chars: Vec<char> = self.text.chars().collect();
                let mut at = self.cursor;
                while at > 0 && !chars[at - 1].is_alphanumeric() {
                    at -= 1;
                }
                while at > 0 && chars[at - 1].is_alphanumeric() {
                    at -= 1;
                }
                return self.remove(at, self.cursor);
            }
            KeyCode::Char(c) if !ctrl => {
                self.insert(c.encode_utf8(&mut [0; 4]));
                return Edit::Changed;
            }
            KeyCode::Enter if multiline => {
                self.insert("\n");
                return Edit::Changed;
            }
            KeyCode::Backspace => return self.remove(self.cursor.saturating_sub(1), self.cursor),
            KeyCode::Delete => return self.remove(self.cursor, (self.cursor + 1).min(len)),
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = len,
            _ => return Edit::Ignored,
        }
        Edit::Moved
    }
}

/// Where a click on part of a row lands.
enum Hit {
    /// An option of a select, by index.
    Option(usize),
    /// A tab of tabs, by id.
    Tab(String),
    /// The bar of a slider, from this column, [`SLIDER_CELLS`] wide.
    Slider(u16),
}

/// One line of the page.
#[derive(Default)]
struct Row {
    spans: Vec<Span<'static>>,
    /// The cells used so far.
    col: u16,
    /// The element the line is for, if it can take focus.
    id: Option<String>,
    hits: Vec<(u16, u16, Hit)>,
    /// The column of the text cursor, for the input being edited.
    cursor: Option<u16>,
}

impl Row {
    fn text(text: impl Into<String>, style: Style) -> Row {
        let mut r = Row::default();
        r.push(text, style);
        r
    }

    fn push(&mut self, text: impl Into<String>, style: Style) {
        let text = text.into();
        self.col += text.width() as u16;
        self.spans.push(Span::styled(text, style));
    }

    fn hit(&mut self, text: impl Into<String>, style: Style, hit: Hit) {
        let start = self.col;
        self.push(text, style);
        self.hits.push((start, self.col, hit));
    }
}

struct App {
    s: Session,
    opts: tree::Opts,
    /// The history entry shown and the client rebuilt up to it; `None` is
    /// live.
    pinned: Option<(usize, Client)>,
    focus: Option<String>,
    /// Scroll the focused row into view on the next frame.
    reveal: bool,
    editing: Option<Editor>,
    /// Controls whose edits onChange has not reported yet, and when to.
    pending: HashMap<String, Instant>,
    scroll: u16,
    show_attrs: bool,
    show_signals: bool,
    notice: Option<(String, Instant)>,
    /// Set once the transport has closed, with the reason if it failed.
    closed: Option<Option<String>>,
    // What the last frame drew, for the mouse.
    rows: Vec<Row>,
    page_area: Rect,
    side_area: Rect,
    side_offset: usize,
    /// Whether the keys move in the history rather than on the page.
    side: bool,
    quit: bool,
}

impl App {
    fn new(s: Session, max_text: usize) -> Self {
        App {
            s,
            opts: tree::Opts {
                color: false,
                max_text: max_text.max(2),
            },
            pinned: None,
            focus: None,
            reveal: false,
            editing: None,
            pending: HashMap::new(),
            scroll: 0,
            show_attrs: false,
            show_signals: true,
            notice: None,
            closed: None,
            rows: Vec::new(),
            page_area: Rect::default(),
            side_area: Rect::default(),
            side_offset: 0,
            side: false,
            quit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<(), String> {
        while !self.quit {
            self.receive();
            self.flush_due();
            self.sync_editor();
            terminal
                .draw(|f| self.draw(f))
                .map_err(|e| format!("cannot draw: {e}"))?;
            let wait = self.pending.values().min().map_or(TICK, |d| {
                d.saturating_duration_since(Instant::now()).min(TICK)
            });
            if event::poll(wait).map_err(|e| format!("terminal: {e}"))? {
                loop {
                    match event::read().map_err(|e| format!("terminal: {e}"))? {
                        Event::Key(k) if k.kind != KeyEventKind::Release => self.key(k),
                        Event::Mouse(m) => self.mouse(m),
                        Event::Paste(p) => self.paste(&p),
                        _ => {}
                    }
                    if !event::poll(Duration::ZERO).unwrap_or(false) {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn history(&self) -> &History {
        self.s.history.as_ref().expect("hypb ui keeps a history")
    }

    fn page(&self) -> &Client {
        self.pinned.as_ref().map_or(&self.s.client, |(_, c)| c)
    }

    fn live(&self) -> bool {
        self.pinned.is_none()
    }

    fn say(&mut self, message: impl Into<String>) {
        self.notice = Some((message.into(), Instant::now()));
    }

    /// Takes what the app sent, and the notes and logs it caused.
    fn receive(&mut self) {
        while self.closed.is_none() {
            match self.s.next(Some(Instant::now())) {
                None => break,
                Some(Update::Lines(_)) => {}
                Some(Update::Closed(r)) => self.closed = Some(r),
            }
        }
        let mut notes = self
            .s
            .notes
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default();
        for log in self.s.client.take_logs() {
            notes.push(match log.detail {
                Some(d) => format!("log: {} ({d})", log.summary),
                None => format!("log: {}", log.summary),
            });
        }
        if let Some(last) = notes.into_iter().last() {
            self.say(last);
        }
        if self
            .editing
            .as_ref()
            .is_some_and(|e| !self.s.client.contains(&e.id))
        {
            self.editing = None;
        }
    }

    /// Runs the onChange of edits that have been idle long enough.
    fn flush_due(&mut self) {
        let now = Instant::now();
        let due: Vec<String> = self
            .pending
            .iter()
            .filter(|(_, at)| **at <= now)
            .map(|(id, _)| id.clone())
            .collect();
        for id in due {
            self.flush(&id);
        }
    }

    fn flush(&mut self, id: &str) {
        if self.pending.remove(id).is_some() {
            self.fire(id, "onChange");
        }
    }

    /// Follows the app when it changes the signal of the input being
    /// edited.
    fn sync_editor(&mut self) {
        let Some(ed) = &self.editing else { return };
        let Some(el) = self.s.client.get(&ed.id) else {
            return;
        };
        let Some(path) = str_attr(&el.attrs, "bind") else {
            return;
        };
        let current = self.s.client.value(path).cloned().unwrap_or(Value::Null);
        let text = input_text(&current);
        if text != ed.text && typed_value(&el.attrs, &ed.text) != current {
            self.editing.as_mut().unwrap().set(text);
        }
    }

    // --- history ---------------------------------------------------------

    fn go(&mut self, to: Option<usize>) {
        let len = self.history().entries.len();
        match to {
            Some(k) if k < len => {
                if self.pinned.as_ref().is_some_and(|(p, _)| *p == k) {
                    return;
                }
                self.finish_editing();
                let page = self.history().replay(k);
                self.pinned = Some((k, page));
            }
            _ => self.pinned = None,
        }
    }

    /// One step back (`-1`) or forward (`1`) in the history.
    fn step(&mut self, d: isize) {
        let len = self.history().entries.len();
        if len == 0 {
            return;
        }
        let at = self.pinned.as_ref().map_or(len, |(k, _)| *k) as isize + d;
        self.go(Some(at.clamp(0, len as isize) as usize));
    }

    // --- input -----------------------------------------------------------

    fn key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && matches!(k.code, KeyCode::Char('c' | 'q')) {
            self.quit = true;
            return;
        }
        if self.editing.is_some() {
            self.edit_key(k);
            return;
        }
        if self.side {
            self.side_key(k);
            return;
        }
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => self.move_focus(-1),
            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => self.move_focus(1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(id) = self.focus.clone() {
                    self.activate(&id);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => self.adjust(-1),
            KeyCode::Right | KeyCode::Char('l') => self.adjust(1),
            KeyCode::Char('[') | KeyCode::Char('p') => {
                self.side = true;
                self.step(-1);
            }
            KeyCode::Char(']') | KeyCode::Char('n') => {
                self.side = true;
                self.step(1);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.side = true;
                self.go(Some(0));
            }
            KeyCode::End | KeyCode::Char('G') | KeyCode::Esc => self.go(None),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(self.page_area.height / 2),
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(self.page_area.height / 2)
            }
            KeyCode::Char('a') => self.show_attrs = !self.show_attrs,
            KeyCode::Char('s') => self.show_signals = !self.show_signals,
            _ => {}
        }
    }

    /// Keys while the history has them: arrows move between entries.
    fn side_key(&mut self, k: KeyEvent) {
        let len = self.history().entries.len();
        let page = (self.side_area.height.saturating_sub(2) as isize).max(1);
        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Up | KeyCode::Char('k' | '[' | 'p') => self.step(-1),
            KeyCode::Down | KeyCode::Char('j' | ']' | 'n') => self.step(1),
            KeyCode::PageUp => self.step(-page),
            KeyCode::PageDown => self.step(page),
            KeyCode::Home | KeyCode::Char('g') => self.go(Some(0)),
            KeyCode::End | KeyCode::Char('G') => self.go(Some(len)),
            KeyCode::Esc => {
                self.go(None);
                self.side = false;
            }
            KeyCode::Enter | KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                self.side = false
            }
            KeyCode::Char('a') => self.show_attrs = !self.show_attrs,
            KeyCode::Char('s') => self.show_signals = !self.show_signals,
            _ => {}
        }
    }

    fn edit_key(&mut self, k: KeyEvent) {
        let Some(ed) = &mut self.editing else { return };
        let id = ed.id.clone();
        let multiline = self.s.client.attr(&id, "variant") == Some(json!("multiline"));
        match k.code {
            KeyCode::Esc => self.finish_editing(),
            KeyCode::Enter if !multiline => {
                self.finish_editing();
                self.fire(&id, "onSubmit");
            }
            KeyCode::Tab | KeyCode::Down if !multiline || k.code == KeyCode::Tab => {
                self.finish_editing();
                self.move_focus(1);
            }
            KeyCode::BackTab | KeyCode::Up if !multiline || k.code == KeyCode::BackTab => {
                self.finish_editing();
                self.move_focus(-1);
            }
            _ => {
                if ed.key(&k, multiline) == Edit::Changed {
                    self.typed();
                }
            }
        }
    }

    fn paste(&mut self, text: &str) {
        if let Some(ed) = &mut self.editing {
            ed.insert(text);
            self.typed();
        }
    }

    /// The text of the input being edited changed: write it to its signal
    /// and report it with onChange once the user pauses.
    fn typed(&mut self) {
        let Some(ed) = &self.editing else { return };
        let id = ed.id.clone();
        let Some(el) = self.s.client.get(&id) else {
            return;
        };
        let value = typed_value(&el.attrs, &ed.text);
        if self.write(&el, value) {
            self.pending.insert(id, Instant::now() + CHANGE_IDLE);
        }
    }

    fn finish_editing(&mut self) {
        if let Some(ed) = self.editing.take() {
            self.flush(&ed.id);
        }
    }

    fn mouse(&mut self, m: MouseEvent) {
        let at = Position::new(m.column, m.row);
        let in_side = self.side_area.contains(at);
        match m.kind {
            MouseEventKind::ScrollUp if in_side => {
                self.side = true;
                self.step(-1);
            }
            MouseEventKind::ScrollDown if in_side => {
                self.side = true;
                self.step(1);
            }
            MouseEventKind::ScrollUp => self.scroll = self.scroll.saturating_sub(2),
            MouseEventKind::ScrollDown => self.scroll = self.scroll.saturating_add(2),
            MouseEventKind::Down(MouseButton::Left) if in_side => {
                let row = (m.row.saturating_sub(self.side_area.y + 1)) as usize + self.side_offset;
                if m.row > self.side_area.y && m.row + 1 < self.side_area.bottom() {
                    self.side = true;
                    self.go(Some(row));
                }
            }
            MouseEventKind::Down(MouseButton::Left) if self.page_area.contains(at) => {
                self.side = false;
                let i = (m.row - self.page_area.y + self.scroll) as usize;
                let col = m.column - self.page_area.x;
                let Some(row) = self.rows.get(i) else { return };
                let Some(id) = row.id.clone() else { return };
                let hit = row
                    .hits
                    .iter()
                    .find(|(a, b, _)| (*a..*b).contains(&col))
                    .map(|(_, _, h)| match h {
                        Hit::Option(i) => Hit::Option(*i),
                        Hit::Tab(t) => Hit::Tab(t.clone()),
                        Hit::Slider(x) => Hit::Slider(col - *x),
                    });
                if self.focus.as_ref() != Some(&id) {
                    self.finish_editing();
                    self.focus = Some(id.clone());
                }
                match hit {
                    Some(h) => self.click(&id, h),
                    None => self.activate(&id),
                }
            }
            _ => {}
        }
    }

    fn move_focus(&mut self, d: isize) {
        let ids: Vec<&String> = self.rows.iter().filter_map(|r| r.id.as_ref()).collect();
        if ids.is_empty() {
            return;
        }
        let at = self
            .focus
            .as_ref()
            .and_then(|f| ids.iter().position(|i| *i == f));
        let next = match at {
            Some(i) => (i as isize + d).rem_euclid(ids.len() as isize) as usize,
            None if d < 0 => ids.len() - 1,
            None => 0,
        };
        self.focus = Some(ids[next].clone());
        self.reveal = true;
    }

    /// The element, if the page is live and it can be used.
    fn usable(&mut self, id: &str) -> Option<Resolved> {
        if !self.live() {
            self.say("this is an old page, read-only: End (or Esc) goes back to the live one");
            return None;
        }
        let el = self.s.client.get(id)?;
        if !self.s.client.is_connected() {
            self.say("not connected");
            return None;
        }
        if el.attrs.get("disabled") == Some(&Value::Bool(true)) {
            self.say(format!("{id} is disabled"));
            return None;
        }
        Some(el)
    }

    /// Enter, Space or a click on the row of `id`.
    fn activate(&mut self, id: &str) {
        let Some(el) = self.usable(id) else { return };
        match el.component {
            Component::Button => self.fire(id, "onPress"),
            Component::Dialog => self.fire(id, "onDismiss"),
            Component::Input => {
                let v = str_attr(&el.attrs, "bind").and_then(|p| self.s.client.value(p));
                let text = v.map_or(String::new(), input_text);
                self.editing = Some(Editor::new(id, text));
            }
            Component::Checkbox => {
                let on = bound(&self.s.client, &el).and_then(Value::as_bool) == Some(true);
                if self.write(&el, Value::Bool(!on)) {
                    self.fire(id, "onChange");
                }
            }
            Component::Select | Component::Tabs | Component::Slider => self.adjust(1),
            _ => {}
        }
    }

    /// Left (`-1`) or right (`1`) on the focused select, slider or tabs.
    fn adjust(&mut self, d: isize) {
        let Some(id) = self.focus.clone() else { return };
        let Some(el) = self.usable(&id) else { return };
        let client = &self.s.client;
        match el.component {
            Component::Select => {
                let opts = options(&el.attrs);
                if opts.is_empty() {
                    return;
                }
                let at = selected_option(client, &el, &opts);
                let i = match at {
                    Some(i) => (i as isize + d).rem_euclid(opts.len() as isize) as usize,
                    None => 0,
                };
                self.click(&id, Hit::Option(i));
            }
            Component::Tabs => {
                let tabs = tabs(client, &el);
                if tabs.is_empty() {
                    return;
                }
                let at = selected_tab(client, &el, &tabs);
                let i = (at as isize + d).rem_euclid(tabs.len() as isize) as usize;
                self.click(&id, Hit::Tab(tabs[i].0.clone()));
            }
            Component::Slider => {
                let (min, max, step) = slider_range(&el.attrs);
                let v = slider_value(client, &el);
                self.slide(&el, (v + d as f64 * step).clamp(min, max));
            }
            _ => {}
        }
    }

    fn click(&mut self, id: &str, hit: Hit) {
        let Some(el) = self.usable(id) else { return };
        let client = &self.s.client;
        match hit {
            Hit::Option(i) => {
                let opts = options(&el.attrs);
                if selected_option(client, &el, &opts) == Some(i) {
                    return;
                }
                if let Some((v, _)) = opts.get(i)
                    && self.write(&el, json!(v))
                {
                    self.fire(id, "onChange");
                }
            }
            Hit::Tab(tab) => {
                if self.write(&el, json!(tab)) {
                    self.fire(id, "onChange");
                }
            }
            Hit::Slider(x) => {
                let (min, max, _) = slider_range(&el.attrs);
                let f = f64::from(x.min(SLIDER_CELLS - 1)) / f64::from(SLIDER_CELLS - 1);
                self.slide(&el, min + f * (max - min));
            }
        }
    }

    fn slide(&mut self, el: &Resolved, v: f64) {
        let (min, max, step) = slider_range(&el.attrs);
        let v = (min + ((v - min) / step).round() * step).clamp(min, max);
        let v = (v * 1e9).round() / 1e9;
        if (v - slider_value(&self.s.client, el)).abs() < 1e-9 {
            return;
        }
        if self.write(el, number(v)) {
            self.pending
                .insert(el.id.clone(), Instant::now() + SLIDER_IDLE);
        }
    }

    /// Writes the bound signal of a control, as the control does.
    fn write(&mut self, el: &Resolved, value: Value) -> bool {
        let Some(path) = str_attr(&el.attrs, "bind").map(str::to_string) else {
            self.say(format!("{} is not bound to a signal", el.id));
            return false;
        };
        self.s.client.write(&path, value);
        true
    }

    /// Runs a handler and says what it did.
    fn fire(&mut self, id: &str, event: &str) {
        let Some(Value::Object(h)) = self.s.client.attr(id, event) else {
            if event != "onChange" && event != "onSubmit" {
                self.say(format!("{id} has no {event}"));
            }
            return;
        };
        let sent = self.s.trigger(id, event);
        let message = if sent {
            let action = h.get("action").and_then(Value::as_str).unwrap_or("?");
            format!("{id}.{event} sent action {action}")
        } else if h.contains_key("action") {
            format!("{id}.{event} sent nothing: it or a parent is hidden or disabled")
        } else {
            format!("{id}.{event} ran; it sends no action")
        };
        self.say(message);
    }

    // --- drawing ---------------------------------------------------------

    fn draw(&mut self, f: &mut Frame) {
        let [body, status] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(f.area());
        let side_w = SIDEBAR.min(body.width / 2);
        let [side, main] =
            Layout::horizontal([Constraint::Length(side_w), Constraint::Min(10)]).areas(body);
        self.draw_history(f, side);

        let raw = self.pinned.as_ref().map(|(k, _)| self.raw_entry(*k));
        let raw_h = raw.as_ref().map_or(0, |(_, l)| {
            let w = main.width.saturating_sub(2).max(1) as usize;
            (l.width().div_ceil(w) as u16 + 2).clamp(3, main.height / 3 + 2)
        });
        let [page, raw_area] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(raw_h)]).areas(main);

        let title = match &self.pinned {
            None => Span::styled(" live ", Style::new().fg(Color::Green).bold()),
            Some((k, _)) => Span::styled(
                format!(
                    " #{} of {} · read-only ",
                    k + 1,
                    self.history().entries.len()
                ),
                Style::new().fg(Color::Yellow).bold(),
            ),
        };
        let block = Block::bordered()
            .title(title)
            .border_style(Style::new().fg(MUTED));
        let inner = block.inner(page);
        f.render_widget(block, page);
        self.page_area = inner;
        self.rows = self.build();
        if self
            .focus
            .as_ref()
            .is_some_and(|f| !self.rows.iter().any(|r| r.id.as_ref() == Some(f)))
        {
            self.focus = None;
        }
        let focus_row = self
            .rows
            .iter()
            .position(|r| r.id.is_some() && r.id == self.focus);
        if let (true, Some(i)) = (self.reveal, focus_row) {
            let i = i as u16;
            if i < self.scroll {
                self.scroll = i;
            } else if i >= self.scroll + inner.height {
                self.scroll = i + 1 - inner.height;
            }
        }
        self.reveal = false;
        self.scroll = self
            .scroll
            .min((self.rows.len() as u16).saturating_sub(inner.height));
        let lines: Vec<Line> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let line = Line::from(r.spans.clone());
                if Some(i) == focus_row {
                    line.style(Style::new().bg(FOCUS_BG))
                } else {
                    line
                }
            })
            .collect();
        f.render_widget(Paragraph::new(lines).scroll((self.scroll, 0)), inner);
        if let Some(i) = focus_row
            && let Some(c) = self.rows[i].cursor
            && (self.scroll..self.scroll + inner.height).contains(&(i as u16))
        {
            f.set_cursor_position((inner.x + c, inner.y + i as u16 - self.scroll));
        }

        if let Some((title, line)) = raw {
            let block = Block::bordered()
                .title(title)
                .border_style(Style::new().fg(MUTED));
            f.render_widget(
                Paragraph::new(line).wrap(Wrap { trim: false }).block(block),
                raw_area,
            );
        }
        self.draw_status(f, status);
    }

    /// The title and line of history entry `k`.
    fn raw_entry(&self, k: usize) -> (String, String) {
        let h = self.history();
        let e = &h.entries[k];
        let arrow = if e.dir == Dir::Client { '>' } else { '<' };
        let signals = if h.same_signals(k) {
            "signals as before"
        } else {
            "signals changed"
        };
        (
            format!(" #{} {arrow} +{:.2}s · {signals} ", k + 1, e.t),
            e.line.clone(),
        )
    }

    fn draw_history(&mut self, f: &mut Frame, area: Rect) {
        let h = self.history();
        let mut items: Vec<ListItem> = h
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let (arrow, color) = match e.dir {
                    Dir::Client => ('>', Color::Green),
                    Dir::Server => ('<', Color::Reset),
                };
                let (tag, detail) = summary(&e.line);
                let mut spans = vec![
                    Span::styled(format!("{:>3} ", i + 1), Style::new().fg(MUTED)),
                    Span::styled(format!("{arrow} {tag}"), Style::new().fg(color)),
                ];
                if !detail.is_empty() {
                    spans.push(Span::styled(format!(" {detail}"), Style::new().fg(MUTED)));
                }
                if !h.same_signals(i) && i > 0 {
                    spans.push(Span::styled(" •", Style::new().fg(Color::Cyan)));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        let live = match &self.closed {
            None => "● live",
            Some(_) => "● live (closed)",
        };
        items.push(ListItem::new(Span::styled(
            format!("    {live}"),
            Style::new().fg(Color::Green).bold(),
        )));
        let selected = self.pinned.as_ref().map_or(h.entries.len(), |(k, _)| *k);
        let (title, border) = if self.side {
            (" history · ↑↓ ", Style::new().fg(Color::Cyan))
        } else {
            (" history ", Style::new().fg(MUTED))
        };
        let list = List::new(items)
            .block(Block::bordered().title(title).border_style(border))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        let mut state = ListState::default()
            .with_offset(self.side_offset)
            .with_selected(Some(selected));
        f.render_stateful_widget(list, area, &mut state);
        self.side_offset = state.offset();
        self.side_area = area;
    }

    fn draw_status(&mut self, f: &mut Frame, area: Rect) {
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() > NOTICE_FOR)
        {
            self.notice = None;
        }
        let (text, style) = match (&self.notice, &self.editing) {
            (Some((n, _)), _) => (n.clone(), Style::new().fg(Color::Yellow)),
            (None, None) if self.side => (
                "history · ↑↓ step · Home first · End live · Enter/Tab/→ back to the page · Esc live · q quit"
                    .into(),
                Style::new().fg(MUTED),
            ),
            (None, Some(_)) => (
                "editing · Enter submit · Esc done · Tab next".into(),
                Style::new().fg(MUTED),
            ),
            (None, None) => {
                let state = match (&self.closed, self.s.client.is_connected()) {
                    (Some(Some(r)), _) => format!("connection lost: {r}"),
                    (Some(None), _) => "closed".into(),
                    (None, true) => "connected".into(),
                    (None, false) => "connecting…".into(),
                };
                (
                    format!(
                        "{state} · ↑↓ move · Enter press/edit · ←→ change · [ ] history (then ↑↓) · End live · a attrs · s signals · q quit"
                    ),
                    Style::new().fg(MUTED),
                )
            }
        };
        f.render_widget(Paragraph::new(Span::styled(text, style)), area);
    }

    /// The lines of the page.
    fn build(&self) -> Vec<Row> {
        let client = self.page();
        let max = self.opts.max_text;
        let dim = Style::new().fg(MUTED);
        let mut rows = Vec::new();
        let m = client.metadata();
        let meta: Vec<String> = m
            .name
            .iter()
            .map(|n| compact(&json!(clip(n, max))))
            .chain(
                m.description
                    .iter()
                    .map(|d| format!("— {}", compact(&json!(clip(d, max))))),
            )
            .collect();
        if !meta.is_empty() {
            let mut r = Row::text("metadata ", dim);
            r.push(meta.join(" "), Style::new().bold());
            rows.push(r);
        }
        let tree = client.tree();
        if tree.is_empty() {
            rows.push(Row::text("tree {}", dim));
        } else {
            rows.push(Row::text("tree {", dim));
            for n in &tree {
                self.element(&mut rows, n, 1, client);
            }
            rows.push(Row::text("}", dim));
        }
        if let Some(c) = client.closed() {
            let text = match &c.reason {
                Some(r) => format!("closed: {r}"),
                None => "closed".into(),
            };
            rows.push(Row::text(text, Style::new().fg(Color::Red)));
        }
        if self.show_signals {
            rows.push(Row::text("signals", dim));
            let pretty = serde_json::to_string_pretty(client.signals()).unwrap_or_default();
            for line in pretty.lines() {
                rows.push(Row::text(format!("  {line}"), Style::new().fg(Color::Cyan)));
            }
        }
        rows
    }

    fn element(&self, rows: &mut Vec<Row>, n: &ResolvedTree, depth: usize, client: &Client) {
        let opts = &self.opts;
        let dim = Style::new().fg(MUTED);
        let focusable = focusable(n);
        let focused = focusable && self.focus.as_deref() == Some(n.id.as_str());
        let hidden = n.attrs.get("visible") == Some(&Value::Bool(false));
        let mut r = Row::default();
        r.push(
            if focused { "▶ " } else { "  " },
            Style::new().fg(Color::Cyan),
        );
        r.push("  ".repeat(depth.saturating_sub(1)), Style::new());
        let ty = Style::new().bold();
        r.push(n.component.name(), if hidden { dim } else { ty });
        r.push(" ", Style::new());
        r.push(n.id.clone(), dim);
        for p in tree::describe(n, client, opts, true) {
            r.push(" ", Style::new());
            r.push(p, Style::new());
        }
        self.widget(&mut r, n, client, focused);
        if let Some(path) = str_attr(&n.attrs, "bind") {
            r.push(format!(" → {path}"), dim);
        }
        for (event, h) in tree::handlers(n.component, &n.attrs) {
            r.push(format!("  ⚡ {event} →"), Style::new().fg(Color::Yellow));
            r.push(tree::handler(h, opts), Style::new().fg(Color::Green));
        }
        if focusable {
            r.id = Some(n.id.clone());
        }
        rows.push(r);
        if self.show_attrs {
            let attrs: Map<String, Value> = n
                .attrs
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let indent = "  ".repeat(depth + 1);
            rows.push(Row::text(
                format!("{indent}{}", compact(&Value::Object(attrs))),
                dim,
            ));
        }
        for c in &n.children {
            self.element(rows, c, depth + 1, client);
        }
    }

    /// The control of an element, if it is one.
    fn widget(&self, r: &mut Row, n: &ResolvedTree, client: &Client, focused: bool) {
        let a = &n.attrs;
        let on = Style::new().fg(Color::Cyan).bold();
        let field = Style::new().bg(Color::Indexed(236));
        let off = Style::new().fg(MUTED);
        let value = || str_attr(a, "bind").and_then(|p| client.value(p));
        match n.component {
            Component::Input => {
                r.push(" ", Style::new());
                let mask = str_attr(a, "variant") == Some("password");
                let shown = |t: &str| -> String {
                    let t: String = if mask {
                        "•".repeat(t.chars().count())
                    } else {
                        t.replace('\n', "⏎")
                    };
                    t
                };
                match self.editing.as_ref().filter(|e| e.id == n.id && focused) {
                    Some(ed) => {
                        let before: String = ed.text.chars().take(ed.cursor).collect();
                        let start = r.col + 1;
                        r.push(format!("[{}", shown(&ed.text)), field.fg(Color::White));
                        r.push("]", field);
                        r.cursor = Some(start + shown(&before).width() as u16);
                    }
                    None => {
                        let text = value().map(input_text).unwrap_or_default();
                        if text.is_empty() {
                            let ph = str_attr(a, "placeholder").unwrap_or("");
                            r.push(
                                format!("[{}]", clip(ph, self.opts.max_text)),
                                field.fg(MUTED),
                            );
                        } else {
                            r.push(
                                format!("[{}]", clip(&shown(&text), 2 * self.opts.max_text)),
                                field,
                            );
                        }
                    }
                }
                if let Some(e) = str_attr(a, "error").filter(|e| !e.is_empty()) {
                    r.push(format!(" ✗ {e}"), Style::new().fg(Color::Red));
                }
            }
            Component::Checkbox => {
                let checked = value().and_then(Value::as_bool) == Some(true);
                let text = match (str_attr(a, "variant") == Some("switch"), checked) {
                    (true, true) => " (on)",
                    (true, false) => " (off)",
                    (false, true) => " [x]",
                    (false, false) => " [ ]",
                };
                r.push(text, if checked { on } else { Style::new() });
            }
            Component::Select => {
                let opts = options(a);
                let el = resolved(n);
                let at = selected_option(client, &el, &opts);
                r.push(" ", Style::new());
                for (i, (_, label)) in opts.iter().enumerate() {
                    if i > 0 {
                        r.push(" | ", off);
                    }
                    let label = clip(label, self.opts.max_text);
                    if at == Some(i) {
                        r.hit(format!("[{label}]"), on, Hit::Option(i));
                    } else {
                        r.hit(label, Style::new(), Hit::Option(i));
                    }
                }
                if opts.is_empty() {
                    r.push("(no options)", off);
                }
            }
            Component::Slider => {
                let el = resolved(n);
                let (min, max, step) = slider_range(a);
                let v = slider_value(client, &el);
                let filled = if max > min {
                    (((v - min) / (max - min)) * f64::from(SLIDER_CELLS)).round() as usize
                } else {
                    0
                };
                let filled = filled.min(SLIDER_CELLS as usize);
                r.push(" [", off);
                let start = r.col;
                r.hit(
                    format!(
                        "{}{}",
                        "■".repeat(filled),
                        "·".repeat(SLIDER_CELLS as usize - filled)
                    ),
                    Style::new().fg(Color::Cyan),
                    Hit::Slider(start),
                );
                r.push(format!("] {}", format_number(v, step)), Style::new());
            }
            Component::Tabs => {
                let el = resolved(n);
                let tabs = tabs(client, &el);
                let at = selected_tab(client, &el, &tabs);
                r.push(" ", Style::new());
                for (i, (id, label)) in tabs.iter().enumerate() {
                    if i > 0 {
                        r.push(" | ", off);
                    }
                    let label = clip(label, self.opts.max_text);
                    if i == at {
                        r.hit(format!("[{label}]"), on, Hit::Tab(id.clone()));
                    } else {
                        r.hit(label, Style::new(), Hit::Tab(id.clone()));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Whether an element can take focus: controls, tabs that edit a signal,
/// dialogs that can be dismissed.
fn focusable(n: &ResolvedTree) -> bool {
    match n.component {
        Component::Button
        | Component::Input
        | Component::Checkbox
        | Component::Select
        | Component::Slider => true,
        Component::Tabs => str_attr(&n.attrs, "bind").is_some(),
        Component::Dialog => n.attrs.get("onDismiss").is_some_and(Value::is_object),
        _ => false,
    }
}

/// A tree node as a [`Resolved`], for the helpers that take one.
fn resolved(n: &ResolvedTree) -> Resolved {
    Resolved {
        id: n.id.clone(),
        component: n.component,
        parent: None,
        children: n.children.iter().map(|c| c.id.clone()).collect(),
        attrs: n.attrs.clone(),
    }
}

/// The tag of a history line and a word or two about it.
fn summary(line: &str) -> (String, String) {
    let Ok((tag, body)) = parse_line(line) else {
        return ("?".into(), clip(line, 20));
    };
    let s = |k: &str| {
        body.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let detail = match tag {
        "element" | "remove" => s("id"),
        "action" => s("name"),
        "metadata" => s("name"),
        "signal" => body.keys().cloned().collect::<Vec<_>>().join(","),
        "close" => s("reason"),
        "log" => s("summary"),
        _ => String::new(),
    };
    (tag.to_string(), clip(&detail, 18))
}

fn str_attr<'a>(a: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str).filter(|s| !s.is_empty())
}

fn bound<'a>(client: &'a Client, el: &Resolved) -> Option<&'a Value> {
    client.value(str_attr(&el.attrs, "bind")?)
}

/// What an input shows for a signal value.
fn input_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        v => compact(v),
    }
}

/// The value an input writes for its text: a number (or null) for number
/// inputs, the text otherwise.
fn typed_value(a: &Map<String, Value>, text: &str) -> Value {
    if str_attr(a, "variant") != Some("number") {
        return Value::String(text.to_string());
    }
    let t = text.trim();
    if let Ok(i) = t.parse::<i64>() {
        return json!(i);
    }
    match t.parse::<f64>() {
        Ok(f) if f.is_finite() => json!(f),
        _ => Value::Null,
    }
}

/// A select's options as (value, label).
fn options(a: &Map<String, Value>) -> Vec<(String, String)> {
    a.get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|o| {
            let value = o.get("value")?.as_str()?.to_string();
            let label = o
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(&value)
                .to_string();
            Some((value, label))
        })
        .collect()
}

fn selected_option(client: &Client, el: &Resolved, opts: &[(String, String)]) -> Option<usize> {
    let v = bound(client, el)?.as_str()?;
    opts.iter().position(|o| o.0 == v)
}

/// The tabs of a tabs element, as (id, label).
fn tabs(client: &Client, el: &Resolved) -> Vec<(String, String)> {
    el.children
        .iter()
        .filter_map(|id| {
            let t = client.get(id)?;
            (t.component == Component::Tab).then(|| {
                let label = str_attr(&t.attrs, "label").unwrap_or(id).to_string();
                (id.clone(), label)
            })
        })
        .collect()
}

/// The selected tab: the one the signal names, or the first.
fn selected_tab(client: &Client, el: &Resolved, tabs: &[(String, String)]) -> usize {
    bound(client, el)
        .and_then(Value::as_str)
        .and_then(|v| tabs.iter().position(|t| t.0 == v))
        .unwrap_or(0)
}

fn slider_range(a: &Map<String, Value>) -> (f64, f64, f64) {
    let min = a.get("min").and_then(Value::as_f64).unwrap_or(0.0);
    let max = a
        .get("max")
        .and_then(Value::as_f64)
        .unwrap_or(100.0)
        .max(min);
    let step = a
        .get("step")
        .and_then(Value::as_f64)
        .filter(|s| *s > 0.0)
        .unwrap_or(1.0);
    (min, max, step)
}

fn slider_value(client: &Client, el: &Resolved) -> f64 {
    let (min, max, _) = slider_range(&el.attrs);
    bound(client, el)
        .and_then(Value::as_f64)
        .unwrap_or(min)
        .clamp(min, max)
}

fn number(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

fn format_number(v: f64, step: f64) -> String {
    let decimals = format!("{step}")
        .split_once('.')
        .map_or(0, |(_, f)| f.len());
    format!("{v:.decimals$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_text() {
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        let mut e = Editor::new("a", "héllo".into());
        e.key(&key(KeyCode::Left), false);
        assert!(e.key(&key(KeyCode::Backspace), false) == Edit::Changed);
        assert_eq!(e.text, "hélo");
        e.key(&key(KeyCode::Home), false);
        e.key(&key(KeyCode::Char('>')), false);
        assert_eq!(e.text, ">hélo");
        assert!(e.key(&key(KeyCode::Enter), false) == Edit::Ignored);
    }

    #[test]
    fn values_of_inputs() {
        let number = json!({"variant": "number"});
        let number = number.as_object().unwrap();
        assert_eq!(typed_value(number, "12"), json!(12));
        assert_eq!(typed_value(number, "1.5"), json!(1.5));
        assert_eq!(typed_value(number, "x"), Value::Null);
        assert_eq!(typed_value(&Map::new(), "12"), json!("12"));
        assert_eq!(input_text(&json!(3)), "3");
        assert_eq!(format_number(2.5, 0.5), "2.5");
    }
}
