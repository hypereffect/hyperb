//! The history of a session, for `hypb ui`: every message both ways, each
//! with the signals the client held once it was sent or applied, which
//! includes what the user had typed since the message before. Consecutive entries
//! with the same signals share one snapshot.
//!
//! Going back to an entry rebuilds the page from scratch: a fresh client
//! takes the server's messages up to it, in order, then the signals of
//! that entry. What a message does never depends on the signals at the
//! time (they are only read when attributes are), so that is the page as
//! it was.

use hyperb_core::hypereffect::Value;
use hyperb_core::hypereffect::client::Client;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// From the client to the app.
    Client,
    /// From the app to the client.
    Server,
}

pub struct Entry {
    /// Seconds since the flow began.
    pub t: f64,
    pub dir: Dir,
    pub line: String,
    /// The client's signals once the message was sent or applied.
    pub signals: Arc<Value>,
}

#[derive(Default)]
pub struct History {
    pub entries: Vec<Entry>,
}

impl History {
    pub fn push(&mut self, t: f64, dir: Dir, line: &str, signals: &Value) {
        let signals = match self.entries.last() {
            Some(e) if *e.signals == *signals => e.signals.clone(),
            _ => Arc::new(signals.clone()),
        };
        self.entries.push(Entry {
            t,
            dir,
            line: line.trim_end().to_string(),
            signals,
        });
    }

    /// Whether entry `i` has the same signals as the one before it.
    pub fn same_signals(&self, i: usize) -> bool {
        i > 0 && Arc::ptr_eq(&self.entries[i - 1].signals, &self.entries[i].signals)
    }

    /// The client as it was right after entry `k`.
    pub fn replay(&self, k: usize) -> Client {
        let mut client = Client::new();
        client.hello();
        let Some(last) = self.entries.get(k) else {
            return client;
        };
        for e in &self.entries[..=k] {
            if e.dir == Dir::Server {
                client.receive_line(&e.line);
            }
        }
        set_signals(&mut client, &last.signals);
        client.take_diagnostics();
        client.take_logs();
        client
    }
}

/// Makes the client's signals `target`, writing only what differs.
fn set_signals(c: &mut Client, target: &Value) {
    let current = c.signals().clone();
    diff(c, "", &current, target);
}

fn diff(c: &mut Client, prefix: &str, current: &Value, target: &Value) {
    let join = |k: &str| {
        if prefix.is_empty() {
            k.to_string()
        } else {
            format!("{prefix}.{k}")
        }
    };
    match (current, target) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                match a.get(k) {
                    Some(old) => diff(c, &join(k), old, v),
                    None => c.write(&join(k), v.clone()),
                }
            }
            for k in a.keys().filter(|k| !b.contains_key(*k)) {
                c.write(&join(k), Value::Null);
            }
        }
        _ if current == target || prefix.is_empty() => {}
        // A merge patch replaces anything that is not an object on both
        // sides.
        _ => c.write(prefix, target.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyperb_core::hypereffect::json;

    #[test]
    fn replays_with_what_the_user_typed() {
        let mut h = History::default();
        let mut live = Client::new();
        let hello = live.hello();
        h.push(0.0, Dir::Client, &hello, live.signals());
        for line in [
            r#"signal {"order":{"address":"","n":1},"gone":true}"#,
            r#"element {"id":"a","type":"input","bind":"order.address"}"#,
        ] {
            live.receive_line(line);
            h.push(0.1, Dir::Server, line, live.signals());
        }
        assert!(h.same_signals(2) && !h.same_signals(1));
        live.write("order.address", json!("221B Baker St"));
        live.write("gone", Value::Null);
        let line = r#"signal {"order":{"n":2}}"#;
        live.receive_line(line);
        h.push(0.2, Dir::Server, line, live.signals());

        assert_eq!(h.replay(3).signals(), live.signals());
        assert_eq!(
            h.replay(2).signals(),
            &json!({"order":{"address":"","n":1},"gone":true})
        );
        assert!(h.replay(0).tree().is_empty());
    }
}
