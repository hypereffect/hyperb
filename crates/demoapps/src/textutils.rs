//! The text utilities demo: a pocket knife of small tools on one form.
//!
//! The client picks a category and a tool; the form then shows what that
//! tool takes (a text box, fields, menus, checkboxes and a slider, each with
//! an id from the tool and its label) and, when the user presses Run, the
//! server answers with the tool's output as elements: values, tables, lists,
//! code, verdicts and pictures. The interface is a view of the server's
//! state, diffed with `vtree`. This one holds no memory between sessions; it
//! ticks only to know the time, for the tools on dates and for random seeds.

mod calc;
mod code;
mod compress;
mod crypto;
mod data;
mod dates;
mod encode;
mod markup;
mod math;
mod names;
mod output;
mod pictures;
mod random;
mod science;
mod text;
mod units;
mod web;

use hypereffect::vtree::{self, Node};
use hypereffect::{Action, App, Handler, Outbox, Value, cel, json};
pub use output::Output;
use std::collections::BTreeMap;

pub type Run = fn(&str, &mut Opts) -> Result<Output, String>;
pub type Menu = &'static [(&'static str, &'static str)];

/// A tool: what the form shows for it, and the function that runs it on the
/// text and the options.
#[derive(Clone, Copy)]
pub struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    pub about: &'static str,
    /// The label of the text box, empty for tools that take none.
    pub text: &'static str,
    /// Labels and initial values.
    pub fields: &'static [(&'static str, &'static str)],
    /// Labels and options; the first option is the initial one.
    pub selects: &'static [(&'static str, Menu)],
    pub checks: &'static [&'static str],
    /// Label, minimum, maximum and initial value.
    pub slider: Option<(&'static str, f64, f64, f64)>,
    pub run: Run,
}

/// A tool on text, with no options.
pub const fn tool(id: &'static str, name: &'static str, about: &'static str, run: Run) -> Tool {
    Tool {
        id,
        name,
        about,
        text: "Text",
        fields: &[],
        selects: &[],
        checks: &[],
        slider: None,
        run,
    }
}

impl Tool {
    /// A tool whose form is only fields and menus.
    pub const fn form(self) -> Tool {
        Tool { text: "", ..self }
    }

    pub const fn text(self, text: &'static str) -> Tool {
        Tool { text, ..self }
    }

    pub const fn fields(self, fields: &'static [(&'static str, &'static str)]) -> Tool {
        Tool { fields, ..self }
    }

    pub const fn selects(self, selects: &'static [(&'static str, Menu)]) -> Tool {
        Tool { selects, ..self }
    }

    pub const fn checks(self, checks: &'static [&'static str]) -> Tool {
        Tool { checks, ..self }
    }

    pub const fn slider(self, label: &'static str, min: f64, max: f64, value: f64) -> Tool {
        Tool {
            slider: Some((label, min, max, value)),
            ..self
        }
    }

    /// The options of the form, in order: fields, menus, checkboxes, slider.
    fn options(&self) -> Vec<Opt> {
        let mut all: Vec<Opt> = Vec::new();
        all.extend(
            self.fields
                .iter()
                .map(|&(label, value)| Opt::new(label, Kind::Field(value))),
        );
        all.extend(
            self.selects
                .iter()
                .map(|&(label, menu)| Opt::new(label, Kind::Select(menu))),
        );
        all.extend(
            self.checks
                .iter()
                .map(|&label| Opt::new(label, Kind::Check)),
        );
        all.extend(
            self.slider
                .map(|(label, min, max, value)| Opt::new(label, Kind::Slider(min, max, value))),
        );
        // A label is named without its parenthesis (“Weight (kg)” is
        // `weight`), unless two would then share a name.
        let short: Vec<String> = all
            .iter()
            .map(|o| output::slug(o.label.split(" (").next().unwrap_or(o.label)))
            .collect();
        for (i, o) in all.iter_mut().enumerate() {
            o.name = if short.iter().filter(|s| **s == short[i]).count() == 1 {
                short[i].clone()
            } else {
                output::slug(o.label)
            };
        }
        all
    }

    /// The signal under which the form keeps this tool's options: the id in
    /// snake case, after `tool_` if it starts with a digit (as `555` does).
    fn key(&self) -> String {
        let key = self.id.replace('-', "_");
        if key.starts_with(|c: char| c.is_ascii_digit()) {
            format!("tool_{key}")
        } else {
            key
        }
    }

    /// The initial values of the options.
    fn defaults(&self) -> Value {
        let values: hypereffect::Map<String, Value> = self
            .options()
            .into_iter()
            .map(|o| {
                let v = match o.kind {
                    Kind::Field(v) => json!(v),
                    Kind::Select(menu) => json!(menu.first().map_or("", |m| m.0)),
                    Kind::Check => json!(false),
                    Kind::Slider(_, _, v) => json!(v),
                };
                (o.key(), v)
            })
            .collect();
        Value::Object(values)
    }
}

/// An option of a tool's form.
struct Opt {
    label: &'static str,
    /// The option's name, from its label, in kebab case.
    name: String,
    kind: Kind,
}

enum Kind {
    /// A field and its initial value.
    Field(&'static str),
    Select(Menu),
    Check,
    /// Minimum, maximum and initial value.
    Slider(f64, f64, f64),
}

impl Opt {
    fn new(label: &'static str, kind: Kind) -> Self {
        Opt {
            label,
            name: String::new(),
            kind,
        }
    }

    /// The name as a signal path segment.
    fn key(&self) -> String {
        self.name.replace('-', "_")
    }
}

/// SplitMix64, seeded from the clock. Not for secrets.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number below `n`, which must not be 0.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    /// A number in `0..1`.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            v.swap(i, self.below(i + 1));
        }
    }
}

/// What a tool is given besides its text.
pub struct Opts<'a> {
    /// The first menu.
    pub mode: &'a str,
    /// The first two fields.
    pub first: &'a str,
    pub second: &'a str,
    /// The first checkbox.
    pub flag: bool,
    /// The slider, rounded.
    pub number: usize,
    pub rng: &'a mut Rng,
    /// The host's clock, in milliseconds since 1970, or 0 before it is known.
    pub now_ms: f64,
    fields: Vec<&'a str>,
    choices: Vec<&'a str>,
    checks: Vec<bool>,
    labels: &'static [(&'static str, &'static str)],
}

impl<'a> Opts<'a> {
    pub fn new(mode: &'a str, rng: &'a mut Rng) -> Self {
        Opts {
            mode,
            first: "",
            second: "",
            flag: false,
            number: 0,
            rng,
            now_ms: 0.0,
            fields: Vec::new(),
            choices: vec![mode],
            checks: Vec::new(),
            labels: &[],
        }
    }

    /// Fields, menus and checkboxes, for tests.
    #[cfg(test)]
    pub fn with(mut self, fields: &[&'a str], choices: &[&'a str], checks: &[bool]) -> Self {
        self.fields = fields.to_vec();
        self.first = fields.first().copied().unwrap_or("");
        self.second = fields.get(1).copied().unwrap_or("");
        if !choices.is_empty() {
            self.choices = choices.to_vec();
            self.mode = choices[0];
        }
        self.checks = checks.to_vec();
        self.flag = checks.first().copied().unwrap_or(false);
        self
    }

    /// Field `i`, trimmed.
    pub fn field(&self, i: usize) -> &'a str {
        self.fields.get(i).copied().unwrap_or("").trim()
    }

    pub fn choice(&self, i: usize) -> &'a str {
        self.choices.get(i).copied().unwrap_or("")
    }

    pub fn check(&self, i: usize) -> bool {
        self.checks.get(i).copied().unwrap_or(false)
    }

    fn label(&self, i: usize) -> String {
        self.labels
            .get(i)
            .map_or_else(|| format!("Field {}", i + 1), |l| l.0.to_owned())
    }

    /// Field `i` as a number, if it has one.
    pub fn opt_num(&self, i: usize) -> Result<Option<f64>, String> {
        let raw = self.field(i);
        if raw.is_empty() {
            return Ok(None);
        }
        parse_number(raw)
            .map(Some)
            .ok_or_else(|| format!("“{}” needs a number, not “{raw}”.", self.label(i)))
    }

    /// Field `i` as a number.
    pub fn num(&self, i: usize) -> Result<f64, String> {
        self.opt_num(i)?
            .ok_or_else(|| format!("Fill in “{}”.", self.label(i)))
    }

    /// Field `i` as a number above zero.
    pub fn positive(&self, i: usize) -> Result<f64, String> {
        let n = self.num(i)?;
        if n > 0.0 {
            Ok(n)
        } else {
            Err(format!("“{}” must be more than zero.", self.label(i)))
        }
    }

    /// Field `i` as a whole number.
    pub fn int(&self, i: usize) -> Result<i64, String> {
        let n = self.num(i)?;
        if n.fract() == 0.0 && n.abs() < 9e15 {
            Ok(n as i64)
        } else {
            Err(format!("“{}” needs a whole number.", self.label(i)))
        }
    }

    /// The host's clock as days since 1970 and milliseconds into the day.
    pub fn now(&self) -> Result<f64, String> {
        if self.now_ms > 0.0 {
            Ok(self.now_ms)
        } else {
            Err("The clock is not known yet; try again in a second.".into())
        }
    }
}

/// A number as people write it: `1,234.5`, `1 234,5`, `50%`, `$12`, `1e3`.
pub fn parse_number(s: &str) -> Option<f64> {
    let t: String = s
        .trim()
        .trim_end_matches('%')
        .chars()
        .filter(|c| !c.is_whitespace() && !"$€£¥_'".contains(*c))
        .collect();
    let t = if t.matches(',').count() == 1 && !t.contains('.') && t.rsplit(',').next()?.len() != 3 {
        t.replace(',', ".")
    } else {
        t.replace(',', "")
    };
    t.replace('−', "-")
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// A number for people: up to `digits` decimals, without trailing zeros, and
/// grouped thousands.
pub fn fmt(v: f64, digits: usize) -> String {
    if !v.is_finite() {
        return "∞".into();
    }
    if v != 0.0 && (v.abs() >= 1e15 || v.abs() < 10f64.powi(-(digits as i32))) {
        return format!("{v:.4e}");
    }
    let s = format!("{:.*}", digits, v.abs());
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        &s
    };
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    let sign = if v < 0.0 && s.chars().any(|c| c != '0' && c != '.') {
        "-"
    } else {
        ""
    };
    let grouped = super::common::group(whole);
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{frac}")
    }
}

/// Text to copy or to work on again.
pub fn snippet(text: String) -> Output {
    Output::default().code(text)
}

/// Labelled values.
pub fn table(rows: &[(&str, String)]) -> Output {
    Output::default().facts(rows)
}

/// Items, one per line.
pub fn list(items: impl IntoIterator<Item = impl Into<String>>) -> Output {
    Output::default().list(items)
}

/// Rows under a header.
pub fn grid<S: Into<String>>(head: &[&str], rows: impl IntoIterator<Item = Vec<S>>) -> Output {
    Output::default().table(head, rows)
}

/// Whether the input passed a check.
pub fn verdict(ok: bool, text: impl Into<String>) -> Output {
    Output::default().verdict(ok, text)
}

/// Prose.
pub fn prose(text: impl Into<String>) -> Output {
    Output::default().text(text)
}

/// The categories, in menu order.
const CATEGORIES: &[(&str, &str, &[Tool])] = &[
    ("text", "Text", text::TEXT),
    ("writing", "Writing", text::WRITING),
    ("extract", "Extract", text::EXTRACT),
    ("format", "Code formatters", code::FORMAT),
    ("convert", "Data converters", data::CONVERT),
    ("sort", "Sorting", data::SORT),
    ("encode", "Encoders and ciphers", encode::ENCODE),
    ("hash", "Hashes", encode::HASH),
    ("compression", "Compression", compress::COMPRESSION),
    (
        "security",
        "Encryption, signatures and sharing",
        crypto::SECURITY,
    ),
    ("developer", "Developer", web::DEVELOPER),
    ("validate", "Validators", web::VALIDATE),
    ("web", "Web and SEO", web::WEB),
    ("network", "Network and Unix", web::NETWORK),
    ("calc", "Calculators", math::CALC),
    ("math", "Math", math::MATH),
    ("numbers", "Numbers", math::NUMBERS),
    ("stats", "Statistics", math::STATS),
    ("units", "Units", units::UNITS),
    ("dates", "Calendar and time", dates::DATES),
    ("finance", "Finance", calc::FINANCE),
    ("business", "Business and real estate", calc::BUSINESS),
    ("health", "Health and fitness", calc::HEALTH),
    ("home", "Home, DIY and garden", calc::HOME),
    ("cooking", "Cooking", calc::COOKING),
    ("auto", "Automotive and travel", calc::AUTO),
    ("science", "Physics and chemistry", science::SCIENCE),
    ("space", "Astronomy", science::SPACE),
    (
        "electronics",
        "Electronics and engineering",
        science::ELECTRONICS,
    ),
    ("printing", "3D printing", science::PRINTING),
    ("photo", "Photography", science::PHOTO),
    ("music", "Music", science::MUSIC),
    ("blockchain", "Blockchain", encode::BLOCKCHAIN),
    ("color", "Color and design", pictures::COLOR),
    ("images", "Codes, fractals and images", pictures::IMAGES),
    ("random", "Random", random::RANDOM),
    ("fun", "Fun", random::FUN),
    ("reference", "Reference", web::REFERENCE),
];

fn find(id: &str) -> Option<(usize, &'static Tool)> {
    CATEGORIES
        .iter()
        .enumerate()
        .find_map(|(c, cat)| cat.2.iter().find(|t| t.id == id).map(|t| (c, t)))
}

fn menu<T>(items: &[T], f: impl Fn(&T) -> (&'static str, &'static str)) -> Value {
    Value::Array(
        items
            .iter()
            .map(f)
            .map(|(value, label)| json!({"value": value, "label": label}))
            .collect(),
    )
}

pub struct TextUtils {
    rng: Option<Rng>,
    now_ms: f64,
    /// The chosen category, an index into `CATEGORIES`.
    category: usize,
    tool: &'static Tool,
    /// What the last run of `tool` gave, if it ran since it was chosen.
    result: Option<Result<Output, String>>,
    /// The interface as last sent.
    shown: Vec<Node>,
}

impl Default for TextUtils {
    fn default() -> Self {
        TextUtils {
            rng: None,
            now_ms: 0.0,
            category: 0,
            tool: &CATEGORIES[0].2[0],
            result: None,
            shown: Vec::new(),
        }
    }
}

impl TextUtils {
    fn view(&self) -> Node {
        let tool = self.tool;
        let (_, _, tools) = CATEGORIES[self.category];
        vtree::column("main")
            .gap("large")
            .children([
                vtree::text("title").variant("h1").text("Text Utilities"),
                vtree::card("picker").children([
                    vtree::row("menus").gap("medium").wrap(true).children([
                        vtree::select("category")
                            .label("Category")
                            .bind("nav.category")
                            .options(menu(CATEGORIES, |c| (c.0, c.1)))
                            .on_change(Handler::action("category")),
                        vtree::select("tool")
                            .label("Tool")
                            .bind("nav.tool")
                            .options(menu(tools, |t| (t.id, t.name)))
                            .on_change(Handler::action("tool")),
                    ]),
                    vtree::text("about").variant("caption").text(tool.about),
                ]),
            ])
            .children(self.options())
            .children((!tool.text.is_empty()).then(|| {
                vtree::card("source").title(tool.text).child(
                    vtree::input("source-text")
                        .bind("text")
                        .variant("multiline")
                        .placeholder("Paste text here…"),
                )
            }))
            .child(self.actions())
            .child(self.result())
    }

    /// The card of the tool's options, with ids such as `loan-interest-rate`.
    fn options(&self) -> Option<Node> {
        let tool = self.tool;
        let opts = tool.options();
        if opts.is_empty() {
            return None;
        }
        let key = tool.key();
        let id = |o: &Opt| format!("{}-{}", tool.id, o.name);
        let bind = |o: &Opt| format!("args.{key}.{}", o.key());
        let menus = opts.iter().filter_map(|o| match o.kind {
            Kind::Select(m) => Some(
                vtree::select(id(o))
                    .label(o.label)
                    .bind(bind(o))
                    .options(super::common::menu(m)),
            ),
            _ => None,
        });
        let fields = opts.iter().filter_map(|o| match o.kind {
            Kind::Field(_) => Some(vtree::input(id(o)).label(o.label).bind(bind(o))),
            _ => None,
        });
        let rest = opts.iter().filter_map(|o| match o.kind {
            Kind::Check => Some(vtree::checkbox(id(o)).label(o.label).bind(bind(o))),
            Kind::Slider(min, max, _) => Some(
                vtree::slider(id(o))
                    .label(cel(format!(
                        "'{}: ' + string(int(s.args.{key}.{}))",
                        o.label.replace('\'', "\\'"),
                        o.key()
                    )))
                    .bind(bind(o))
                    .min(min)
                    .max(max),
            ),
            _ => None,
        });
        Some(
            vtree::card(format!("{}-options", tool.id))
                .title("Options")
                .child(
                    vtree::row(format!("{}-menus", tool.id))
                        .gap("medium")
                        .wrap(true)
                        .children(menus),
                )
                .child(
                    vtree::row(format!("{}-fields", tool.id))
                        .gap("medium")
                        .wrap(true)
                        .children(fields),
                )
                .children(rest),
        )
    }

    fn actions(&self) -> Node {
        let takes_text = !self.tool.text.is_empty();
        let reusable = matches!(&self.result, Some(Ok(o)) if o.reusable().is_some());
        vtree::row("actions")
            .wrap(true)
            .child(
                vtree::button("run")
                    .label(cel("s._running ? 'Running…' : 'Run'"))
                    .variant("primary")
                    .disabled(cel("s._running"))
                    .on_press(Handler::action("run").indicator("_running")),
            )
            .children((takes_text && reusable).then(|| {
                vtree::button("reuse")
                    .label("Use the result as input")
                    .on_press(Handler::patch(json!({"text": cel("s.result.text")})))
            }))
            .children(takes_text.then(|| {
                vtree::button("clear")
                    .label("Clear")
                    .variant("plain")
                    .visible(cel("s.text != ''"))
                    .on_press(Handler::patch(json!({"text": ""})))
            }))
    }

    fn result(&self) -> Node {
        let card = vtree::card("result").title("Result");
        match &self.result {
            None => card.child(vtree::text("result-pending").variant("caption").text(
                if self.tool.text.is_empty() {
                    "Fill in the options and press Run."
                } else {
                    "Paste some text and press Run."
                },
            )),
            Some(Err(e)) => card.child(
                vtree::notice("result-error")
                    .variant("failure")
                    .text(e.as_str()),
            ),
            Some(Ok(o)) => card.children(o.view("result", !self.tool.text.is_empty())),
        }
    }

    /// Sends what changed since the last call, and remembers what was sent.
    fn update(&mut self, out: &mut Outbox) {
        let next = vec![self.view()];
        out.extend(vtree::diff_into(None, &self.shown, &next));
        self.shown = next;
    }

    /// Chooses `tool`, with its options as the user left them or else at
    /// their initial values. Its form starts without a result.
    fn pick(&mut self, out: &mut Outbox, action: &Action, category: usize, tool: &'static Tool) {
        self.category = category;
        self.tool = tool;
        self.result = None;
        let mut patch = json!({"nav": {"category": CATEGORIES[category].0, "tool": tool.id}});
        if !matches!(
            action.signal(&format!("args.{}", tool.key())),
            Some(Value::Object(_))
        ) {
            patch["args"] = json!({tool.key(): tool.defaults()});
        }
        out.signal(patch);
    }

    fn run(&mut self, action: &Action) -> Result<Output, String> {
        let tool = self.tool;
        let text = action
            .signal("text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !tool.text.is_empty() && text.trim().is_empty() {
            return Err("Paste some text first.".into());
        }
        let key = tool.key();
        let opts = tool.options();
        let arg = |o: &Opt| action.signal(&format!("args.{key}.{}", o.key()));
        let fields: Vec<&str> = opts
            .iter()
            .filter(|o| matches!(o.kind, Kind::Field(_)))
            .map(|o| arg(o).and_then(Value::as_str).unwrap_or_default())
            .collect();
        // A menu's value must be one of its options.
        let mut choices: Vec<&str> = opts
            .iter()
            .filter_map(|o| match o.kind {
                Kind::Select(menu) => {
                    let v = arg(o).and_then(Value::as_str).unwrap_or_default();
                    Some(
                        menu.iter()
                            .find(|m| m.0 == v)
                            .or(menu.first())
                            .map_or("", |m| m.0),
                    )
                }
                _ => None,
            })
            .collect();
        if choices.is_empty() {
            choices.push("");
        }
        let checks: Vec<bool> = opts
            .iter()
            .filter(|o| matches!(o.kind, Kind::Check))
            .map(|o| arg(o).and_then(Value::as_bool).unwrap_or(false))
            .collect();
        let slider = opts.iter().find_map(|o| match o.kind {
            Kind::Slider(min, max, value) => Some(
                arg(o)
                    .and_then(Value::as_f64)
                    .unwrap_or(value)
                    .clamp(min, max),
            ),
            _ => None,
        });
        let now_ms = self.now_ms;
        let rng = self.rng.get_or_insert_with(|| Rng(now_ms.to_bits()));
        let mut o = Opts {
            mode: choices[0],
            first: fields.first().copied().unwrap_or(""),
            second: fields.get(1).copied().unwrap_or(""),
            flag: checks.first().copied().unwrap_or(false),
            number: slider.unwrap_or(0.0).round() as usize,
            rng,
            now_ms,
            fields,
            choices,
            checks,
            labels: tool.fields,
        };
        (tool.run)(text, &mut o)
    }
}

impl App for TextUtils {
    fn hello(&mut self, out: &mut Outbox, _: &BTreeMap<String, String>) {
        let tool = self.tool;
        out.signal(json!({
            "nav": {"category": CATEGORIES[self.category].0, "tool": tool.id},
            "args": {tool.key(): tool.defaults()},
            "text": "",
            "result": {"text": ""},
            "_running": false,
        }));
        self.update(out);
    }

    fn action(&mut self, out: &mut Outbox, action: &Action) {
        let nav = |name: &str| {
            action
                .signal(&format!("nav.{name}"))
                .and_then(Value::as_str)
                .unwrap_or_default()
        };
        match action.name.as_str() {
            "category" => {
                let c = CATEGORIES
                    .iter()
                    .position(|c| c.0 == nav("category"))
                    .unwrap_or(self.category);
                self.pick(out, action, c, &CATEGORIES[c].2[0]);
            }
            "tool" => match find(nav("tool")) {
                Some((c, tool)) => self.pick(out, action, c, tool),
                None => out.signal(json!({"nav": {"tool": self.tool.id}})),
            },
            "run" => {
                let result = self.run(action);
                let reuse = result
                    .as_ref()
                    .ok()
                    .and_then(Output::reusable)
                    .unwrap_or_default();
                out.signal(json!({"result": {"text": reuse}, "_running": false}));
                self.result = Some(result);
            }
            _ => return,
        }
        self.update(out);
    }

    fn tick(&mut self, _: &mut Outbox, now_ms: f64) {
        self.now_ms = now_ms;
    }

    /// One tick soon to learn the time, then one a second to keep it.
    fn tick_interval(&self) -> Option<u32> {
        Some(if self.now_ms == 0.0 { 1 } else { 1000 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypereffect::server::Loopback;
    use std::collections::HashSet;

    fn pick(l: &mut Loopback<TextUtils>, category: &str, tool: &str) {
        l.write("nav.category", json!(category));
        assert!(l.trigger("category", "onChange"));
        l.write("nav.tool", json!(tool));
        assert!(l.trigger("tool", "onChange"));
    }

    #[test]
    fn tool_and_option_ids() {
        let mut seen = HashSet::new();
        let mut clashes = Vec::new();
        for (_, _, tools) in CATEGORIES {
            assert!(!tools.is_empty());
            for t in *tools {
                assert!(seen.insert(t.id), "{} twice", t.id);
                assert!(
                    hypereffect::signals::parse_path(&t.key()).is_some(),
                    "{}",
                    t.id
                );
                let names: Vec<String> = t.options().into_iter().map(|o| o.name).collect();
                let unique: HashSet<&String> = names.iter().collect();
                if unique.len() != names.len() {
                    clashes.push(format!("{}: {names:?}", t.id));
                }
            }
        }
        assert!(
            clashes.is_empty(),
            "options with the same name:\n{}",
            clashes.join("\n")
        );
    }

    /// Every tool shows its form and runs on its initial values (and a
    /// sample text) without panicking; form-only tools also succeed.
    #[test]
    fn every_tool_runs() {
        let mut l = Loopback::<TextUtils>::new();
        l.tick(1_700_000_000_000.0);
        l.write(
            "text",
            json!("Hello World, 42 and 3.5 at 2024-01-02. <b>bold</b> {\"a\": 1}"),
        );
        for (cat, _, tools) in CATEGORIES {
            for t in *tools {
                pick(&mut l, cat, t.id);
                assert_eq!(l.client.value("nav.tool"), Some(&json!(t.id)));
                assert!(l.trigger("run", "onPress"));
                if t.text.is_empty() {
                    assert_eq!(l.client.attr("result-error", "text"), None, "{}", t.id);
                }
            }
        }
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn runs_only_on_request() {
        let mut l = Loopback::<TextUtils>::new();
        pick(&mut l, "text", "counter");
        l.write("text", json!("rose rose tulip fern"));
        assert!(l.client.attr("result-pending", "text").is_some());
        assert!(!l.trigger("source-text", "onChange"));
        assert!(l.trigger("run", "onPress"));
        assert_eq!(
            l.client.attr("result-facts-words-value", "text"),
            Some(json!("4"))
        );
        assert_eq!(l.client.value("_running"), Some(&json!(false)));
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn options_have_named_ids() {
        let mut l = Loopback::<TextUtils>::new();
        pick(&mut l, "text", "find-replace");
        assert_eq!(
            l.client.attr("find-replace-find", "label"),
            Some(json!("Find"))
        );
        assert_eq!(
            l.client.attr("find-replace-find", "bind"),
            Some(json!("args.find_replace.find"))
        );
        l.write("text", json!("a cat, a Cat"));
        l.write("args.find_replace.find", json!("cat"));
        l.write("args.find_replace.replace_with", json!("dog"));
        l.write("args.find_replace.ignore_case", json!(true));
        assert!(l.trigger("run", "onPress"));
        assert_eq!(
            l.client.attr("result-code", "text"),
            Some(json!("a dog, a dog"))
        );
        assert_eq!(
            l.client.attr("result-note", "text"),
            Some(json!("2 replaced"))
        );
        // The result can become the input, without asking the server.
        assert!(!l.trigger("reuse", "onPress"));
        assert_eq!(l.client.value("text"), Some(&json!("a dog, a dog")));
        // Another tool and back: the options are as the user left them.
        pick(&mut l, "text", "case");
        assert_eq!(l.client.attr("find-replace-find", "label"), None);
        pick(&mut l, "text", "find-replace");
        assert_eq!(
            l.client.value("args.find_replace.find"),
            Some(&json!("cat"))
        );
        assert!(l.client.attr("result-pending", "text").is_some());
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn empty_text_is_rejected_on_run() {
        let mut l = Loopback::<TextUtils>::new();
        assert!(l.trigger("run", "onPress"));
        assert_eq!(
            l.client.attr("result-error", "text"),
            Some(json!("Paste some text first."))
        );
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn form_tools_show_values() {
        let mut l = Loopback::<TextUtils>::new();
        pick(&mut l, "health", "bmi");
        assert_eq!(l.client.attr("source", "type"), None);
        assert!(l.trigger("run", "onPress"));
        assert!(l.client.attr("result-facts-bmi-value", "text").is_some());
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn numbers_parse_and_print() {
        assert_eq!(parse_number("1,234.5"), Some(1234.5));
        assert_eq!(parse_number("12,5"), Some(12.5));
        assert_eq!(parse_number(" $3 "), Some(3.0));
        assert_eq!(parse_number("50%"), Some(50.0));
        assert_eq!(fmt(1234.5678, 2), "1,234.57");
        assert_eq!(fmt(-0.5, 2), "-0.5");
        assert_eq!(fmt(3.0, 2), "3");
    }
}

#[cfg(test)]
mod dump {
    use super::*;

    #[test]
    #[ignore]
    fn dump() {
        let mut rng = Rng(7);
        let sample = "Hello World, 42 and 3.5 at 2024-01-02. <b>bold</b> {\"a\": 1}";
        for (_, _, tools) in CATEGORIES {
            for t in *tools {
                let opts = t.options();
                let fields: Vec<&str> = opts
                    .iter()
                    .filter_map(|o| match o.kind {
                        Kind::Field(v) => Some(v),
                        _ => None,
                    })
                    .collect();
                let choices: Vec<&str> = opts
                    .iter()
                    .filter_map(|o| match o.kind {
                        Kind::Select(m) => m.first().map(|x| x.0),
                        _ => None,
                    })
                    .collect();
                let mut o = Opts::new("", &mut rng).with(&fields, &choices, &[]);
                o.labels = t.fields;
                o.number = t.slider.map_or(0, |s| s.3 as usize);
                o.now_ms = 1_700_000_000_000.0;
                let kinds = match (t.run)(if t.text.is_empty() { "" } else { sample }, &mut o) {
                    Ok(out) => out
                        .blocks
                        .iter()
                        .map(|b| match b {
                            output::Block::Facts(r) => format!("facts{}", r.len()),
                            output::Block::Table { rows, .. } => format!("table{}", rows.len()),
                            output::Block::Code(_) => "code".into(),
                            output::Block::Text(_) => "text".into(),
                            output::Block::List(i) => format!("list{}", i.len()),
                            output::Block::Verdict { ok, .. } => format!("verdict-{ok}"),
                            output::Block::Image { .. } => "image".into(),
                            output::Block::Note(_) => "note".into(),
                        })
                        .collect::<Vec<_>>()
                        .join(","),
                    Err(e) => format!("ERR {e}"),
                };
                let names: Vec<String> = opts.iter().map(|o| o.name.clone()).collect();
                println!("{} | {} | {}", t.id, kinds, names.join(" "));
            }
        }
    }
}
