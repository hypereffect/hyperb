//! A short depiction of the resolved tree: one line per element with its
//! type, id and the one or two attributes that say what it shows, and in
//! full the handlers of the ones that can do something.
//!
//! ```text
//! tree {
//!   column main
//!     text title "Inbox" h1
//!     input email "Email" [form.email = "a@b.c"]  ⚡ onSubmit → send
//!     button del "Delete" (disabled)  ⚡ onPress → delete {"id":4}
//! }
//! ```

use hyperb_core::hypereffect::client::{Client, ResolvedTree};
use hyperb_core::hypereffect::component::AttrType;
use hyperb_core::hypereffect::{Component, Map, Value};
use std::fmt::Write;

#[derive(Clone)]
pub struct Opts {
    /// Longest text shown, in characters.
    pub max_text: usize,
    pub color: bool,
}

/// The signals as pretty JSON, then the tree in a `tree { … }` block.
pub fn snapshot(client: &Client, opts: &Opts) -> String {
    let signals = hyperb_core::hypereffect::serde_json::to_string_pretty(client.signals())
        .unwrap_or_default();
    format!("{signals}\n{}", render(&client.tree(), client, opts))
}

/// The tree in a `tree { … }` block, or `tree {}` when it is empty.
pub fn render(tree: &[ResolvedTree], client: &Client, opts: &Opts) -> String {
    if tree.is_empty() {
        return "tree {}\n".into();
    }
    let mut out = String::from("tree {\n");
    for node in tree {
        node_into(&mut out, node, 1, client, opts);
    }
    out.push_str("}\n");
    out
}

fn node_into(out: &mut String, n: &ResolvedTree, depth: usize, client: &Client, opts: &Opts) {
    let a = &n.attrs;
    let s = |k: &str| a.get(k).and_then(Value::as_str).filter(|s| !s.is_empty());
    let q = |t: &str| quote(t, opts.max_text);
    let mut parts: Vec<String> = Vec::new();
    // What the element shows.
    match n.component {
        Component::Text => {
            parts.extend(s("text").map(q));
            parts.extend(s("variant").filter(|v| *v != "body").map(str::to_string));
        }
        Component::Notice => {
            parts.extend(s("title").map(q));
            parts.extend(s("text").map(q));
            parts.extend(s("variant").map(str::to_string));
        }
        Component::Badge => parts.extend(s("text").map(q)),
        Component::Image => parts.extend(s("alt").map(q)),
        Component::Card | Component::Dialog => parts.extend(s("title").map(q)),
        Component::Tab | Component::Button | Component::Checkbox => parts.extend(s("label").map(q)),
        Component::Input => parts.extend(s("label").or(s("placeholder")).map(q)),
        Component::Select | Component::Slider | Component::Progress => {
            parts.extend(s("label").map(q))
        }
        Component::List if n.children.is_empty() => {
            parts.extend(s("empty").map(|e| format!("empty: {}", q(e))))
        }
        _ => {}
    }
    match n.component {
        Component::Progress => match a.get("value") {
            Some(v @ Value::Number(_)) => parts.push(num(v)),
            _ => parts.push("indeterminate".into()),
        },
        Component::Slider => {
            let get = |k| a.get(k).map_or("?".into(), num);
            parts.push(format!("{}..{}", get("min"), get("max")))
        }
        Component::Select => {
            let opts_max = opts.max_text;
            let opts: Vec<&str> = a
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|o| o.get("value")?.as_str())
                .collect();
            parts.push(clip(&format!("<{}>", opts.join("|")), 2 * opts_max));
        }
        _ => {}
    }
    // The signal a control or tabs edit, with its value.
    if let Some(path) = s("bind") {
        let v = client.value(path).map_or("null".into(), compact);
        let v = clip(&v, 2 * opts.max_text);
        parts.push(paint(opts, CYAN, &format!("[{path} = {v}]")));
    }
    if a.get("visible") == Some(&Value::Bool(false)) {
        parts.push(paint(opts, DIM, "(hidden)"));
    }
    if a.get("disabled") == Some(&Value::Bool(true)) {
        parts.push(paint(opts, DIM, "(disabled)"));
    }
    if n.component == Component::Dialog && a.get("open") == Some(&Value::Bool(false)) {
        parts.push(paint(opts, DIM, "(closed)"));
    }

    let indent = "  ".repeat(depth);
    let _ = write!(
        out,
        "{indent}{} {}",
        paint(opts, BOLD, n.component.name()),
        paint(opts, DIM, &n.id)
    );
    for p in &parts {
        out.push(' ');
        out.push_str(p);
    }
    for (event, h) in handlers(n.component, a) {
        let _ = write!(out, "  {}", paint(opts, YELLOW, &format!("⚡ {event} →")));
        out.push_str(&handler(h, opts));
    }
    out.push('\n');
    for c in &n.children {
        node_into(out, c, depth + 1, client, opts);
    }
}

/// The handlers an element has, as `(attribute, handler object)`.
pub fn handlers<'a>(
    component: Component,
    attrs: &'a Map<String, Value>,
) -> impl Iterator<Item = (&'static str, &'a Map<String, Value>)> + 'a {
    component
        .attrs()
        .iter()
        .filter(|a| a.ty == AttrType::Handler)
        .filter_map(|a| Some((a.name, attrs.get(a.name)?.as_object()?)))
}

/// Whether an element has a handler that sends an action.
pub fn fires_actions(component: Component, attrs: &Map<String, Value>) -> bool {
    handlers(component, attrs).any(|(_, h)| h.get("action").is_some())
}

/// A handler as ` action payload`, ` patch{keys}` and ` open url`.
fn handler(h: &Map<String, Value>, opts: &Opts) -> String {
    let mut out = String::new();
    if let Some(a) = h.get("action").and_then(Value::as_str) {
        let _ = write!(out, " {}", paint(opts, GREEN, a));
        if let Some(p) = h.get("payload") {
            let _ = write!(out, " {}", clip(&compact(p), 2 * opts.max_text));
        }
    }
    if let Some(Value::Object(p)) = h.get("patch") {
        let keys: Vec<&str> = p.keys().map(String::as_str).collect();
        let _ = write!(out, " patch{{{}}}", keys.join(","));
    }
    if let Some(o) = h.get("open") {
        let o = o.as_str().map_or_else(|| compact(o), str::to_string);
        let _ = write!(out, " open {}", clip(&o, 2 * opts.max_text));
    }
    out
}

/// A number without the `.0` of whole floats.
fn num(v: &Value) -> String {
    match v.as_f64() {
        Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
        _ => compact(v),
    }
}

pub fn compact(v: &Value) -> String {
    hyperb_core::hypereffect::serde_json::to_string(v).unwrap_or_default()
}

/// `s` as a JSON string, cut to `max` characters.
fn quote(s: &str, max: usize) -> String {
    compact(&Value::String(clip(s, max)))
}

/// `s` cut to `max` characters, with an ellipsis if it was longer.
pub fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
    t.push('…');
    t
}

const BOLD: &str = "1";
const DIM: &str = "2";
const GREEN: &str = "32";
const YELLOW: &str = "33";
const CYAN: &str = "36";

fn paint(opts: &Opts, code: &str, s: &str) -> String {
    if opts.color {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}
