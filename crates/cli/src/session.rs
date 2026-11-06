//! One session with an app: the connection, the [`Client`] that keeps its
//! tree and signals, and a blocking way to wait for what it sends.
//!
//! The transport's events are forwarded to a channel by a thread, as in the
//! terminal renderer, so every mode can wait on them with a deadline. The
//! `hello` goes out as soon as the transport opens.

use hyperb_core::hypereffect::client::Client;
use hyperb_core::hypereffect::signals::{merge_patch, strip_local_map};
use hyperb_core::hypereffect::{Action, LineBuffer, RendererMessage, Value};
use hyperb_core::identity::Identity;
use hyperb_core::net::{Connection, Event};
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use url::Url;

pub struct Session {
    pub client: Client,
    conn: Connection,
    rx: Receiver<Event>,
    reader: LineBuffer,
    /// Lines received but not applied to the client yet.
    pending: VecDeque<String>,
    /// Set once the transport has closed, with the reason if it failed.
    closed: Option<Option<String>>,
    /// Whether diagnostics from the client go to stderr.
    verbose: bool,
}

/// How [`Session::settle`] ended.
#[derive(PartialEq)]
pub enum Settled {
    /// The app went quiet.
    Quiet,
    /// The app kept sending until the deadline.
    Busy,
    Closed,
}

pub enum Received {
    /// A line from the app, already applied to the client.
    Line(String),
    /// The session ended, with the reason if it was an error.
    Closed(Option<String>),
}

pub enum Update {
    /// Lines from the app, already applied to the client.
    Lines(Vec<String>),
    /// The session ended, with the reason if it was an error.
    Closed(Option<String>),
}

impl Session {
    /// Connects to `url` as the identity with secret key `key`, or without
    /// one as the identity the renderers share.
    pub fn connect(url: &Url, key: Option<[u8; 32]>, verbose: bool) -> Session {
        let identity = match key {
            Some(k) => Identity::from_secret(k),
            None => Identity::load_default().unwrap_or_else(|e| {
                eprintln!("hypb: cannot load identity ({e}); using a temporary one");
                Identity::generate()
            }),
        };
        if verbose {
            eprintln!("hypb: connecting as {}", identity.public());
        }
        let conn = Connection::open(url, &identity);
        let events = conn.events();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(ev) = events.recv_blocking() {
                if tx.send(ev).is_err() {
                    break;
                }
            }
        });
        Session {
            client: Client::new(),
            conn,
            rx,
            reader: LineBuffer::new(),
            pending: VecDeque::new(),
            closed: None,
            verbose,
        }
    }

    /// Waits until the app sends a line or the session ends, or until
    /// `deadline` (forever without one), which returns `None`. The line is
    /// applied to the client first, so the client is as it was right after
    /// that line. Once the session has ended this returns `Closed` at once.
    pub fn next_line(&mut self, deadline: Option<Instant>) -> Option<Received> {
        loop {
            if let Some(line) = self.pending.pop_front() {
                self.client.receive_line(&line);
                self.report();
                return Some(Received::Line(line));
            }
            if let Some(reason) = &self.closed {
                return Some(Received::Closed(reason.clone()));
            }
            let ev = match deadline {
                None => self.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(d) => self
                    .rx
                    .recv_timeout(d.saturating_duration_since(Instant::now())),
            };
            let ev = match ev {
                Ok(ev) => ev,
                Err(RecvTimeoutError::Timeout) => return None,
                Err(RecvTimeoutError::Disconnected) => Event::Closed(None),
            };
            match ev {
                Event::Open => {
                    let hello = self.client.hello_with(self.conn.params());
                    self.reader = LineBuffer::new();
                    self.conn.send(hello);
                }
                Event::Data(data) => self.pending.extend(self.reader.push(data.as_bytes())),
                Event::Closed(reason) => {
                    self.client.close();
                    self.closed = Some(reason);
                }
            }
        }
    }

    /// Like [`next_line`](Self::next_line), but takes every line that has
    /// arrived along with the first.
    pub fn next(&mut self, deadline: Option<Instant>) -> Option<Update> {
        let first = match self.next_line(deadline)? {
            Received::Line(l) => l,
            Received::Closed(reason) => return Some(Update::Closed(reason)),
        };
        let mut lines = vec![first];
        while let Some(line) = self.pending.pop_front() {
            self.client.receive_line(&line);
            lines.push(line);
        }
        self.report();
        Some(Update::Lines(lines))
    }

    /// Waits until the app has said nothing for `quiet`, the session ends or
    /// `deadline` passes.
    pub fn settle(&mut self, quiet: Duration, deadline: Instant) -> Settled {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Settled::Busy;
            }
            match self.next(Some((now + quiet).min(deadline))) {
                None if Instant::now() < deadline => return Settled::Quiet,
                None | Some(Update::Lines(_)) => {}
                Some(Update::Closed(_)) => return Settled::Closed,
            }
        }
    }

    pub fn send(&self, line: String) {
        self.conn.send(line);
    }

    /// The action line for `name`, carrying the current signals (without
    /// local ones) with `signals` merged into them.
    pub fn action_line(
        &self,
        name: &str,
        payload: Option<Value>,
        signals: Option<&Value>,
    ) -> String {
        let current = self
            .client
            .signals()
            .as_object()
            .map(strip_local_map)
            .unwrap_or_default();
        let mut merged = Value::Object(current);
        if let Some(s) = signals {
            merge_patch(&mut merged, s);
        }
        let Value::Object(signals) = merged else {
            unreachable!("merging into an object keeps an object")
        };
        RendererMessage::Action(Action {
            name: name.to_string(),
            payload,
            signals,
        })
        .to_line()
    }

    /// Runs the handler in attribute `event` of element `id`, as a click or
    /// an edit in a renderer would, and sends its action, if it has one.
    /// Returns whether an action was sent.
    pub fn trigger(&mut self, id: &str, event: &str) -> bool {
        let line = self.client.trigger(id, event);
        for url in self.client.take_opens() {
            eprintln!("hypb: {id}.{event} asks to open {url}");
        }
        self.report();
        match line {
            Some(l) => {
                self.send(l);
                true
            }
            None => false,
        }
    }

    /// Prints the client's diagnostics, if asked to.
    pub fn report(&mut self) {
        let diags = self.client.take_diagnostics();
        if self.verbose {
            for d in diags {
                eprintln!("hypb: {d}");
            }
        }
    }
}
