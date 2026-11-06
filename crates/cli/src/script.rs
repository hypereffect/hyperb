//! `hypb run`: a JavaScript script, on the boa engine, driving one session.
//!
//! The script runs once the app has sent its first interface and gone quiet.
//! Every helper is synchronous: the ones that wait (`wait`, `sleep`,
//! `settle`) take in what the app sends while they block, so the tree and
//! signals they look at are always current.
//!
//! ```js
//! press("inc");
//! wait(m => m.kind == "signal" && signal("count") == 1);
//! const email = queryOne({ type: "input", label: "Email" });
//! input(email.id, "a@b.c");
//! action("save", { draft: false }, { form: { email: "a@b.c" } });
//! wait(m => m.id == "status" && m.element.attrs.text.startsWith("Saved"));
//! printTree();
//! ```

use crate::session::{Received, Session, Settled, Update};
use crate::tree::{self, Opts};
use boa_engine::object::ObjectInitializer;
use boa_engine::{
    Context, JsArgs, JsError, JsResult, JsValue, NativeFunction, Source, js_error, js_string,
};
use hyperb_core::hypereffect::client::Resolved;
use hyperb_core::hypereffect::{ServerMessage, Value, json, serde_json};
use std::cell::RefCell;
use std::path::Path;
use std::time::{Duration, Instant};

/// What the helpers act on.
pub struct Host {
    pub session: Session,
    pub url: String,
    /// How long `wait` waits by default.
    pub timeout: Duration,
    pub tree: Opts,
    /// How long the app must be quiet after the script, so that the last
    /// actions it fired are delivered.
    pub settle: Duration,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Host) -> R) -> R {
    HOST.with_borrow_mut(|h| f(h.as_mut().expect("the script host is set")))
}

/// Runs `src` against the session in `host`. `path` names the script in
/// error messages.
pub fn run(host: Host, src: &str, path: Option<&Path>) -> Result<(), String> {
    let url = host.url.clone();
    HOST.set(Some(host));
    let mut ctx = Context::default();
    let result = install(&mut ctx, &url).and_then(|()| {
        let source = Source::from_bytes(src);
        let source = match path {
            Some(p) => source.with_path(p),
            None => source,
        };
        ctx.eval(source)?;
        ctx.run_jobs()
    });
    with(|h| {
        h.session
            .settle(h.settle, Instant::now() + h.settle + Duration::from_secs(1))
    });
    HOST.set(None);
    result.map_err(|e| describe(e, &mut ctx))
}

fn describe(e: JsError, ctx: &mut Context) -> String {
    match e.into_opaque(ctx) {
        Ok(v) => v
            .to_string(ctx)
            .map_or_else(|_| v.display().to_string(), |s| s.to_std_string_escaped()),
        Err(e) => e.to_string(),
    }
}

type Native = fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue>;

fn install(ctx: &mut Context, url: &str) -> JsResult<()> {
    let globals: &[(&str, usize, Native)] = &[
        ("signals", 0, signals),
        ("signal", 1, signal),
        ("metadata", 0, metadata),
        ("tree", 0, tree_fn),
        ("element", 1, element),
        ("query", 1, query),
        ("queryOne", 1, query_one),
        ("wait", 2, wait),
        ("action", 3, action),
        ("trigger", 2, trigger),
        ("press", 1, press),
        ("input", 2, input),
        ("write", 2, write),
        ("sleep", 1, sleep),
        ("settle", 2, settle),
        ("print", 0, print),
        ("printTree", 0, print_tree),
    ];
    for (name, len, f) in globals {
        ctx.register_global_callable(js_string!(*name), *len, NativeFunction::from_fn_ptr(*f))?;
    }
    let console = ObjectInitializer::new(ctx)
        .function(NativeFunction::from_fn_ptr(print), js_string!("log"), 0)
        .function(NativeFunction::from_fn_ptr(print), js_string!("info"), 0)
        .function(NativeFunction::from_fn_ptr(eprint), js_string!("error"), 0)
        .function(NativeFunction::from_fn_ptr(eprint), js_string!("warn"), 0)
        .build();
    let attr = boa_engine::property::Attribute::all();
    ctx.register_global_property(js_string!("console"), console, attr)?;
    ctx.register_global_property(js_string!("url"), js_string!(url), attr)?;
    Ok(())
}

// --- conversions -------------------------------------------------------------

fn to_js(v: &Value, ctx: &mut Context) -> JsResult<JsValue> {
    JsValue::from_json(v, ctx)
}

/// A JS value as JSON: `undefined` is `None`, and whole floats are integers,
/// so that `2.0` reaches the app as `2`.
fn from_js(v: &JsValue, ctx: &mut Context) -> JsResult<Option<Value>> {
    Ok(v.to_json(ctx)?.map(integral))
}

fn integral(v: Value) -> Value {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 2f64.powi(53) => {
                (f as i64).into()
            }
            _ => Value::Number(n),
        },
        Value::Array(a) => Value::Array(a.into_iter().map(integral).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, v)| (k, integral(v))).collect()),
        v => v,
    }
}

fn string_arg(args: &[JsValue], i: usize, what: &str, ctx: &mut Context) -> JsResult<String> {
    let v = args.get_or_undefined(i);
    if v.is_undefined() {
        return Err(js_error!(TypeError: "missing {}", what));
    }
    Ok(v.to_string(ctx)?.to_std_string_escaped())
}

/// The timeout argument at `i`, in milliseconds, or the default.
fn timeout_arg(args: &[JsValue], i: usize, ctx: &mut Context) -> JsResult<Duration> {
    let v = args.get_or_undefined(i);
    if v.is_null_or_undefined() {
        return Ok(with(|h| h.timeout));
    }
    Ok(Duration::from_millis(v.to_number(ctx)?.max(0.0) as u64))
}

/// Deep equality, with numbers compared by value (`1` equals `1.0`).
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| json_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        (a, b) => a == b,
    }
}

// --- elements ----------------------------------------------------------------

/// An element as scripts see it: its resolved attributes, its children's
/// ids, the value of the signal it is bound to, and what its handlers do.
fn element_json(r: &Resolved, host: &Host) -> Value {
    let client = &host.session.client;
    let value = r
        .attrs
        .get("bind")
        .and_then(Value::as_str)
        .map(|p| client.value(p).cloned().unwrap_or(Value::Null));
    let handlers: Vec<Value> = tree::handlers(r.component, &r.attrs)
        .map(|(event, h)| {
            let mut h = h.clone();
            h.insert("event".into(), event.into());
            Value::Object(h)
        })
        .collect();
    json!({
        "id": r.id,
        "type": r.component.name(),
        "parent": r.parent,
        "children": r.children,
        "attrs": r.attrs,
        "value": value,
        "handlers": handlers,
    })
}

/// Every element, depth first in display order.
fn all_elements(host: &Host) -> Vec<Value> {
    fn walk(host: &Host, ids: &[String], out: &mut Vec<Value>) {
        for id in ids {
            if let Some(r) = host.session.client.get(id) {
                out.push(element_json(&r, host));
                walk(host, &r.children, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(host, host.session.client.children(None), &mut out);
    out
}

/// Whether an element (as [`element_json`]) has everything in `filter`:
/// `id`, `type`, `parent` and `value` are compared with the element's,
/// `action` with the actions of its handlers, and any other key with the
/// attribute of that name.
fn matches(el: &Value, filter: &serde_json::Map<String, Value>) -> bool {
    filter.iter().all(|(k, want)| match k.as_str() {
        "id" | "type" | "parent" | "value" => json_eq(&el[k], want),
        "action" => el["handlers"]
            .as_array()
            .is_some_and(|hs| hs.iter().any(|h| json_eq(&h["action"], want))),
        _ => json_eq(&el["attrs"][k], want),
    })
}

/// Whether `el` passes `filter`: a function called with it, or an object
/// as in [`matches`].
fn passes(el: &Value, filter: &JsValue, ctx: &mut Context) -> JsResult<bool> {
    if let Some(f) = filter.as_callable() {
        let arg = to_js(el, ctx)?;
        return Ok(f.call(&JsValue::undefined(), &[arg], ctx)?.to_boolean());
    }
    match from_js(filter, ctx)? {
        Some(Value::Object(f)) => Ok(matches(el, &f)),
        None => Ok(true),
        Some(_) => Err(js_error!(TypeError: "a filter is a function or an object")),
    }
}

fn select(filter: &JsValue, ctx: &mut Context) -> JsResult<Vec<Value>> {
    let all = with(|h| all_elements(h));
    let mut out = Vec::new();
    for el in all {
        if passes(&el, filter, ctx)? {
            out.push(el);
        }
    }
    Ok(out)
}

// --- waiting -----------------------------------------------------------------

/// Takes in what the app sends until `deadline`. Returns false if the
/// session has ended.
fn pump(deadline: Instant) -> bool {
    with(|h| !matches!(h.session.next(Some(deadline)), Some(Update::Closed(_))))
}

/// A message as `wait` hands it over: `{kind: "element", id, element,
/// message}` with the element as `element(id)` gives it (`null` if it was
/// rejected), or `{kind: "signal", patch, signals}`. Other messages are
/// `None`.
fn message_json(line: &str, host: &Host) -> Option<Value> {
    let client = &host.session.client;
    match ServerMessage::parse(line).ok()? {
        ServerMessage::Element(e) => {
            let element = client.get(&e.id).map(|r| element_json(&r, host));
            Some(json!({
                "kind": "element",
                "id": e.id,
                "element": element,
                "message": serde_json::to_value(&e).unwrap_or_default(),
            }))
        }
        ServerMessage::Signal(patch) => Some(json!({
            "kind": "signal",
            "patch": patch,
            "signals": client.signals(),
        })),
        _ => None,
    }
}

// --- helpers -----------------------------------------------------------------

/// `signals()`: the current signals.
fn signals(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = with(|h| h.session.client.signals().clone());
    to_js(&v, ctx)
}

/// `signal(path)`: the value at a signal path such as `form.email`, or
/// `undefined`.
fn signal(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let path = string_arg(args, 0, "signal path", ctx)?;
    match with(|h| h.session.client.value(&path).cloned()) {
        Some(v) => to_js(&v, ctx),
        None => Ok(JsValue::undefined()),
    }
}

/// `metadata()`: what the app said about itself.
fn metadata(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = with(|h| serde_json::to_value(h.session.client.metadata()).unwrap_or_default());
    to_js(&v, ctx)
}

/// `tree()`: the whole resolved tree, as nested elements.
fn tree_fn(_: &JsValue, _: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = with(|h| serde_json::to_value(h.session.client.tree()).unwrap_or_default());
    to_js(&v, ctx)
}

/// `element(id)`: the element, or `null`.
fn element(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = string_arg(args, 0, "element id", ctx)?;
    match with(|h| h.session.client.get(&id).map(|r| element_json(&r, h))) {
        Some(v) => to_js(&v, ctx),
        None => Ok(JsValue::null()),
    }
}

/// `query(filter)`: the elements that pass a filter (a function, or an
/// object such as `{type: "button", action: "add"}`).
fn query(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let found = select(args.get_or_undefined(0), ctx)?;
    to_js(&Value::Array(found), ctx)
}

/// `queryOne(filter)`: the first element that passes a filter, or `null`.
fn query_one(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    match select(args.get_or_undefined(0), ctx)?.into_iter().next() {
        Some(v) => to_js(&v, ctx),
        None => Ok(JsValue::null()),
    }
}

/// `wait(fn, timeoutMs?)`: calls `fn` with every element and signal
/// message the app sends from now on, as [`message_json`] gives it and with
/// the message already applied, until `fn` returns true. Returns that
/// message; throws on timeout or if the session ends first.
fn wait(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let Some(f) = args.get_or_undefined(0).as_callable() else {
        return Err(js_error!(TypeError: "wait takes a function"));
    };
    let timeout = timeout_arg(args, 1, ctx)?;
    let deadline = Instant::now() + timeout;
    loop {
        let line = match with(|h| h.session.next_line(Some(deadline))) {
            Some(Received::Line(l)) => l,
            Some(Received::Closed(_)) => {
                return Err(js_error!(Error: "the session closed while waiting"));
            }
            None => {
                return Err(js_error!(Error: "timed out after {}ms waiting", timeout.as_millis()));
            }
        };
        let Some(m) = with(|h| message_json(&line, h)) else {
            continue;
        };
        let m = to_js(&m, ctx)?;
        if f.call(&JsValue::undefined(), std::slice::from_ref(&m), ctx)?
            .to_boolean()
        {
            return Ok(m);
        }
    }
}

/// `action(name, payload?, signals?)`: sends an action. It carries the
/// current signals, with `signals` merged into them.
fn action(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let name = string_arg(args, 0, "action name", ctx)?;
    let payload = from_js(args.get_or_undefined(1), ctx)?;
    let patch = match from_js(args.get_or_undefined(2), ctx)? {
        None | Some(Value::Null) => None,
        Some(p @ Value::Object(_)) => Some(p),
        Some(_) => return Err(js_error!(TypeError: "signals must be an object")),
    };
    with(|h| {
        let line = h.session.action_line(&name, payload, patch.as_ref());
        h.session.send(line);
    });
    Ok(JsValue::undefined())
}

/// Runs the handler of element `id` in attribute `event`, as a renderer
/// does. Returns whether it sent an action.
fn fire(id: &str, event: &str) -> JsResult<JsValue> {
    let sent = with(|h| {
        if !h.session.client.contains(id) {
            return Err(js_error!(Error: "no element {}", id));
        }
        Ok(h.session.trigger(id, event))
    })?;
    Ok(sent.into())
}

/// `trigger(id, event)`: runs one of the element's handlers, such as
/// `onSubmit`. Returns whether it sent an action.
fn trigger(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = string_arg(args, 0, "element id", ctx)?;
    let event = string_arg(args, 1, "event", ctx)?;
    fire(&id, &event)
}

/// `press(id)`: presses a button.
fn press(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = string_arg(args, 0, "element id", ctx)?;
    fire(&id, "onPress")
}

/// `input(id, value)`: edits a control: writes its bound signal and runs
/// its `onChange`. Returns whether that sent an action.
fn input(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let id = string_arg(args, 0, "element id", ctx)?;
    let value = from_js(args.get_or_undefined(1), ctx)?.unwrap_or(Value::Null);
    with(|h| {
        let client = &mut h.session.client;
        let Some(Value::String(path)) = client.attr(&id, "bind") else {
            return Err(js_error!(Error: "element {} is not bound to a signal", id));
        };
        client.write(&path, value);
        Ok(())
    })?;
    fire(&id, "onChange")
}

/// `write(path, value)`: writes a signal, as a bound control does. Nothing
/// is sent: the app sees it with the next action.
fn write(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let path = string_arg(args, 0, "signal path", ctx)?;
    let value = from_js(args.get_or_undefined(1), ctx)?.unwrap_or(Value::Null);
    with(|h| h.session.client.write(&path, value));
    Ok(JsValue::undefined())
}

/// `sleep(ms)`: waits, taking in what the app sends meanwhile.
fn sleep(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let ms = args.get_or_undefined(0).to_number(ctx)?.max(0.0) as u64;
    let deadline = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < deadline && pump(deadline) {}
    Ok(JsValue::undefined())
}

/// `settle(ms = 300, maxMs?)`: waits until the app has sent nothing for
/// `ms`, for at most `maxMs`. Returns whether it went quiet.
fn settle(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let v = args.get_or_undefined(0);
    let ms = if v.is_undefined() {
        300.0
    } else {
        v.to_number(ctx)?.max(0.0)
    };
    let quiet = Duration::from_millis(ms as u64);
    let max = timeout_arg(args, 1, ctx)?;
    let settled = with(|h| h.session.settle(quiet, Instant::now() + max));
    Ok((settled == Settled::Quiet).into())
}

/// The arguments joined by spaces: strings as they are, anything else as
/// JSON.
fn line(args: &[JsValue], ctx: &mut Context) -> JsResult<String> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        if let Some(s) = a.as_string() {
            out.push(s.to_std_string_escaped());
        } else if a.is_callable() || a.is_undefined() {
            out.push(a.display().to_string());
        } else {
            out.push(from_js(a, ctx)?.map_or_else(String::new, |v| tree::compact(&v)));
        }
    }
    Ok(out.join(" "))
}

/// `print(...)`, `console.log(...)`.
fn print(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    crate::out(&format!("{}\n", line(args, ctx)?));
    Ok(JsValue::undefined())
}

/// `console.error(...)`.
fn eprint(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    eprintln!("{}", line(args, ctx)?);
    Ok(JsValue::undefined())
}

/// `printTree()`: prints the signals and the tree as `hypb state` does.
fn print_tree(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let text = with(|h| tree::snapshot(&h.session.client, &h.tree));
    crate::out(&text);
    Ok(JsValue::undefined())
}
