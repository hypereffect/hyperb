//! hypb: talks to hypereffect apps from the command line.
//!
//! ```text
//! hypb <address>                    the messages the app sends
//! hypb state <address>              its signals and tree, again after every change
//! hypb action <address> <name>      fires an action
//! hypb run <address> [script.js]    runs a script against it
//! ```
//!
//! Addresses are the ones every renderer opens (`SPEC.md`): HTTP, iroh and
//! wasm apps alike, with the session parameters in the fragment.

mod script;
mod session;
mod tree;

use clap::{Args, Parser, Subcommand};
use hyperb_core::address;
use hyperb_core::hypereffect::{Component, ServerMessage, Value, serde_json};
use session::{Session, Settled, Update};
use std::collections::HashSet;
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Parser)]
#[command(
    name = "hypb",
    version,
    about = "Inspects and scripts hypereffect apps at hyperb:// addresses",
    long_about = "Inspects and scripts hypereffect apps at hyperb:// addresses.\n\n\
        Without a command, connects, sends the hello and prints every message the app \
        sends, parsed and encoded again, one per line.",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    target: Target,
    #[command(flatten)]
    idle: Idle,
    #[command(flatten)]
    filters: Filters,
}

#[derive(Subcommand)]
enum Command {
    /// Prints the signals and a short tree of the interface, again after every change
    #[command(after_help = STATE_EXAMPLE)]
    State {
        #[command(flatten)]
        target: Target,
        #[command(flatten)]
        idle: Idle,
        /// Wait this long without changes before printing the state again, in milliseconds
        #[arg(long, value_name = "MS", default_value_t = 1000)]
        debounce: u64,
        #[command(flatten)]
        look: Look,
    },
    /// Fires an action, then prints it (after `> `) and what the app sends back (after `< `)
    #[command(after_help = ACTION_EXAMPLES)]
    Action {
        #[command(flatten)]
        target: Target,
        /// The name of the action
        name: String,
        /// The payload, as JSON
        #[arg(long, value_name = "JSON")]
        payload: Option<String>,
        /// Signals to send, as a JSON object merged into the app's current signals
        #[arg(long, value_name = "JSON")]
        signals: Option<String>,
        /// Before firing, wait for the app to send nothing for this long, in milliseconds
        #[arg(long, value_name = "MS", default_value_t = 300)]
        settle: u64,
        /// Close the connection after this long without updates, in milliseconds
        #[arg(long, value_name = "MS", default_value_t = 1000)]
        idle: u64,
    },
    /// Runs a JavaScript script against the app
    #[command(after_help = RUN_EXAMPLES, after_long_help = SCRIPT_HELP)]
    Run {
        #[command(flatten)]
        target: Target,
        /// The script file; standard input if absent or `-`
        #[arg(conflicts_with = "eval")]
        script: Option<PathBuf>,
        /// The script itself
        #[arg(short, long, value_name = "CODE")]
        eval: Option<String>,
        /// Before and after the script, wait for the app to send nothing for this long, in milliseconds
        #[arg(long, value_name = "MS", default_value_t = 300)]
        settle: u64,
        /// How long wait() and settle() wait by default, in milliseconds
        #[arg(long, value_name = "MS", default_value_t = 10_000)]
        timeout: u64,
        #[command(flatten)]
        look: Look,
    },
}

/// The examples of `hypb run`, at the end of both its short and long help.
macro_rules! run_examples {
    () => {
        "\
Examples:
  hypb run hyperb://example.com/app script.js
  hypb run ~/apps/counter.wasm -e 'press(\"inc\"); sleep(500); print(signal(\"count\"))'
  echo 'printTree()' | hypb run hyperb://example.com/app
  hypb run hyperb://example.com/app -e '
    input(\"email\", \"a@b.c\");
    trigger(\"email\", \"onSubmit\");
    const m = wait(m => m.id == \"status\" && m.element.attrs.text != \"\");
    print(m.element.attrs.text)'"
    };
}

const RUN_EXAMPLES: &str = run_examples!();

const STATE_EXAMPLE: &str = "\
Example:
  $ hypb state ~/apps/counter.wasm --idle 2000
  ── +1.0s
  {
    \"count\": 2,
    \"step\": 1
  }
  tree {
    column main
      text value \"Count: 2\" h2
      row buttons
        button dec \"−\" (disabled)  ⚡ onPress → add -1
        button inc \"+\"  ⚡ onPress → add 1
      slider step \"Step\" 1..10 [step = 1]  ⚡ onChange → patch{step}
  }

Each line is an element's type and id, what it shows, the signal it is bound to
with its value, and what its handlers do: the action and payload they send, the
signals they patch or the address they open.";

const ACTION_EXAMPLES: &str = "\
Examples:
  hypb action hyperb://example.com/app refresh
  hypb action ~/apps/todo.wasm remove --payload 42
  hypb action ~/apps/todo.wasm add --payload '{\"title\":\"Milk\"}' --idle 500
  hypb action hyperb://example.com/app save --signals '{\"form\":{\"email\":\"a@b.c\"}}'";

const SCRIPT_HELP: &str = concat!(
    "\
The script runs once the app has sent its interface and gone quiet. Its helpers:

  url                                 the address
  signals()                           the current signals
  signal(path)                        one signal, such as signal(\"form.email\")
  metadata()                          what the app says about itself
  tree()                              the resolved tree
  element(id)                         one element, or null
  query(filter), queryOne(filter)     elements passing a filter: a function of the
                                      element, or an object such as
                                      {type: \"button\", action: \"add\", label: \"+\"}
  wait(fn, ms?)                       calls fn(message) for every element and signal
                                      message the app sends from now on, until it
                                      returns true; returns that message:
                                      {kind: \"element\", id, element, message} or
                                      {kind: \"signal\", patch, signals}
  action(name, payload?, signals?)    sends an action; `signals` is merged into the
                                      current ones
  press(id)                           presses a button
  input(id, value)                    edits a control: writes its signal, runs onChange
  trigger(id, event)                  runs any handler, such as trigger(id, \"onSubmit\")
  write(path, value)                  writes a signal without sending anything
  sleep(ms)                           waits, taking in what the app sends
  settle(ms = 300, max?)              waits until the app sends nothing for `ms` (at
                                      most `max`, --timeout by default); returns
                                      whether it went quiet
  print(...), console.log(...)        prints, objects as JSON
  printTree()                         prints the signals and tree as `hypb state` does

Elements are {id, type, parent, children, attrs, value, handlers}: `attrs` are
resolved, `value` is the bound signal's and `handlers` lists {event, action,
payload, patch, open}. Each message reaches wait() already applied, so the
other helpers show the state right after it. wait() throws after its timeout
(--timeout by default) or if the session ends, and an uncaught exception
fails the run.

",
    run_examples!()
);

#[derive(Args)]
struct Target {
    /// The app's address, e.g. hyperb://example.com/app or ~/apps/qr.wasm
    #[arg(required = true)]
    address: Option<String>,
    /// Print the client's diagnostics, such as rejected elements and failing expressions
    #[arg(short, long)]
    verbose: bool,
    /// Connect with this ed25519 secret key, in hex, instead of the renderers' identity.
    /// Missing trailing bytes are zeroes, so `--key 01` is 01 followed by 31 zero bytes
    #[arg(long, value_name = "HEX", value_parser = secret_key)]
    key: Option<[u8; 32]>,
}

/// A secret key of up to 32 bytes in hex, padded with zero bytes.
fn secret_key(s: &str) -> Result<[u8; 32], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return Err("expected whole bytes: an even number of hex digits".into());
    }
    if s.len() > 64 {
        return Err("a secret key is at most 32 bytes (64 hex digits)".into());
    }
    let mut key = [0u8; 32];
    for (i, pair) in s.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(pair).map_err(|_| "not hex")?;
        key[i] = u8::from_str_radix(pair, 16).map_err(|_| format!("not hex: {pair}"))?;
    }
    Ok(key)
}

#[derive(Args)]
struct Idle {
    /// Close the connection after this long without updates, in milliseconds
    #[arg(long, value_name = "MS")]
    idle: Option<u64>,
}

#[derive(Args, Default)]
struct Filters {
    /// Print only the elements with this id (repeatable)
    #[arg(long = "id", value_name = "ID")]
    ids: Vec<String>,
    /// Print only the elements of this type (repeatable)
    #[arg(long = "type", value_name = "TYPE", value_parser = component)]
    types: Vec<Component>,
    /// Print only the elements that can fire actions
    #[arg(long)]
    actions: bool,
}

#[derive(Args)]
struct Look {
    /// Cut texts longer than this, in characters
    #[arg(long, value_name = "N", default_value_t = 48)]
    max_text: usize,
    /// No colors, even on a terminal
    #[arg(long)]
    no_color: bool,
}

impl Look {
    fn opts(&self) -> tree::Opts {
        let color = !self.no_color
            && std::env::var_os("NO_COLOR").is_none()
            && std::io::stdout().is_terminal();
        tree::Opts {
            max_text: self.max_text.max(2),
            color,
        }
    }
}

fn component(s: &str) -> Result<Component, String> {
    Component::from_name(s).ok_or_else(|| {
        let all: Vec<&str> = Component::ALL.iter().map(|c| c.name()).collect();
        format!("expected one of {}", all.join(", "))
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        None => connect(&cli.target).and_then(|mut s| {
            stream(
                &mut s,
                &cli.filters,
                cli.idle.idle.map(Duration::from_millis),
                "",
            )
        }),
        Some(Command::State {
            target,
            idle,
            debounce,
            look,
        }) => connect(&target).and_then(|mut s| {
            state(
                &mut s,
                Duration::from_millis(debounce),
                idle.idle.map(Duration::from_millis),
                &look.opts(),
            )
        }),
        Some(Command::Action {
            target,
            name,
            payload,
            signals,
            settle,
            idle,
        }) => (|| {
            let payload = payload
                .as_deref()
                .map(|p| json_arg(p, "--payload"))
                .transpose()?;
            let signals = signals
                .as_deref()
                .map(|p| json_arg(p, "--signals"))
                .transpose()?;
            if signals.as_ref().is_some_and(|s| !s.is_object()) {
                return Err("--signals must be a JSON object".to_string());
            }
            let mut s = connect(&target)?;
            ready(&mut s, Duration::from_millis(settle))?;
            let line = s.action_line(&name, payload, signals.as_ref());
            out(&format!("> {line}"));
            s.send(line);
            stream(
                &mut s,
                &Filters::default(),
                Some(Duration::from_millis(idle)),
                "< ",
            )
        })(),
        Some(Command::Run {
            target,
            script,
            eval,
            settle,
            timeout,
            look,
        }) => (|| {
            let (src, path) = match (eval, script) {
                (Some(code), _) => (code, None),
                (None, Some(p)) if p.as_os_str() != "-" => {
                    let src = std::fs::read_to_string(&p)
                        .map_err(|e| format!("cannot read {}: {e}", p.display()))?;
                    (src, Some(p))
                }
                (None, _) => {
                    let mut src = String::new();
                    std::io::stdin()
                        .read_to_string(&mut src)
                        .map_err(|e| format!("cannot read the script: {e}"))?;
                    (src, None)
                }
            };
            let url = parse(&target)?;
            let mut session = Session::connect(&url, target.key, target.verbose);
            let settle = Duration::from_millis(settle);
            ready(&mut session, settle)?;
            let host = script::Host {
                session,
                url: address::hyperb(&url),
                timeout: Duration::from_millis(timeout),
                tree: look.opts(),
                settle,
            };
            script::run(host, &src, path.as_deref())
        })(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("hypb: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Writes to standard output. A closed pipe, as in `hypb … | head`, ends
/// the program quietly.
pub fn out(text: &str) {
    let mut o = std::io::stdout().lock();
    if let Err(e) = o.write_all(text.as_bytes()).and_then(|()| o.flush()) {
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            eprintln!("hypb: cannot write the output: {e}");
            std::process::exit(1);
        }
        std::process::exit(0);
    }
}

fn parse(target: &Target) -> Result<Url, String> {
    let a = target.address.as_deref().unwrap_or_default();
    address::normalize(&address::absolute_file(a)).map_err(|e| format!("invalid address: {e}"))
}

fn connect(target: &Target) -> Result<Session, String> {
    Ok(Session::connect(
        &parse(target)?,
        target.key,
        target.verbose,
    ))
}

fn json_arg(s: &str, flag: &str) -> Result<Value, String> {
    serde_json::from_str(s).map_err(|e| format!("{flag} is not JSON: {e}"))
}

/// How long the app has to send its first message.
const FIRST: Duration = Duration::from_secs(30);

/// How much longer than its quiet time a settle waits for a busy app, such
/// as one with a running clock.
const SETTLE: Duration = Duration::from_secs(3);

/// Waits until the app has sent its first messages and then nothing for
/// `quiet` (or for at most [`SETTLE`] more).
fn ready(s: &mut Session, quiet: Duration) -> Result<(), String> {
    match s.next(Some(Instant::now() + FIRST)) {
        None => return Err(format!("the app sent nothing in {}s", FIRST.as_secs())),
        Some(Update::Closed(r)) => return Err(closed_before(r)),
        Some(Update::Lines(_)) => {}
    }
    if s.settle(quiet, Instant::now() + quiet + SETTLE) == Settled::Closed {
        let Some(Update::Closed(r)) = s.next(None) else {
            unreachable!("the session has closed")
        };
        return Err(closed_before(r));
    }
    Ok(())
}

fn closed_before(reason: Option<String>) -> String {
    match reason {
        Some(r) => format!("the session closed: {r}"),
        None => "the app closed the session before it was ready".into(),
    }
}

/// The end of a session: an error if the transport failed.
fn closed(reason: Option<String>) -> Result<(), String> {
    match reason {
        Some(r) => Err(format!("the session closed: {r}")),
        None => {
            eprintln!("hypb: the app closed the session");
            Ok(())
        }
    }
}

/// Prints the messages the app sends, one per line and after `prefix`,
/// until it closes or is quiet for `idle`.
fn stream(
    s: &mut Session,
    filters: &Filters,
    idle: Option<Duration>,
    prefix: &str,
) -> Result<(), String> {
    let filtering = !filters.ids.is_empty() || !filters.types.is_empty() || filters.actions;
    // The elements printed, so their removals are printed too.
    let mut shown = HashSet::new();
    loop {
        let deadline = idle.map(|d| Instant::now() + d);
        let lines = match s.next(deadline) {
            None => return Ok(()),
            Some(Update::Closed(r)) => return closed(r),
            Some(Update::Lines(lines)) => lines,
        };
        for line in lines {
            let m = match ServerMessage::parse(&line) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("hypb: {e}: {line}");
                    continue;
                }
            };
            let print = match &m {
                _ if !filtering => true,
                ServerMessage::Element(e) => {
                    let ok = (filters.ids.is_empty() || filters.ids.contains(&e.id))
                        && (filters.types.is_empty() || filters.types.contains(&e.component))
                        && (!filters.actions || tree::fires_actions(e.component, &e.attrs));
                    if ok {
                        shown.insert(e.id.clone());
                    }
                    ok
                }
                ServerMessage::Remove { id } => shown.remove(id) || filters.ids.contains(id),
                ServerMessage::Signal(_) | ServerMessage::Metadata(_) => false,
            };
            if print {
                out(&format!("{prefix}{}", m.to_line()));
            }
        }
    }
}

/// Prints the signals and the tree once changes have paused for
/// `debounce` (or every five debounces while they keep coming), until the
/// app closes or is quiet for `idle`.
fn state(
    s: &mut Session,
    debounce: Duration,
    idle: Option<Duration>,
    opts: &tree::Opts,
) -> Result<(), String> {
    let start = Instant::now();
    let mut printed = false;
    let mut show = |s: &mut Session| {
        let gap = if printed { "\n" } else { "" };
        printed = true;
        let rule = format!("── +{:.1}s", start.elapsed().as_secs_f64());
        let rule = match opts.color {
            true => format!("\x1b[2m{rule}\x1b[0m"),
            false => rule,
        };
        out(&format!("{gap}{rule}\n{}", tree::snapshot(&s.client, opts)));
        s.report();
    };
    // The first and the last change not printed yet.
    let mut pending: Option<(Instant, Instant)> = None;
    let mut quiet_until = idle.map(|d| Instant::now() + d);
    loop {
        let due = pending.map(|(first, last)| (last + debounce).min(first + 5 * debounce));
        let deadline = due.into_iter().chain(quiet_until).min();
        let update = s.next(deadline);
        let now = Instant::now();
        match update {
            Some(Update::Lines(_)) => {
                pending = Some((pending.map_or(now, |(first, _)| first), now));
                quiet_until = idle.map(|d| now + d);
            }
            Some(Update::Closed(r)) => {
                if pending.is_some() {
                    show(s);
                }
                return closed(r);
            }
            None => {}
        }
        let due = pending.map(|(first, last)| (last + debounce).min(first + 5 * debounce));
        if due.is_some_and(|d| now >= d) {
            show(s);
            pending = None;
        }
        if quiet_until.is_some_and(|d| now >= d) {
            if pending.is_some() {
                show(s);
            }
            return Ok(());
        }
    }
}
