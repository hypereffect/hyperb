//! The text utilities demo: a pocket knife of small tools on one form.
//!
//! The client picks a category and a tool; the form then shows what that
//! tool takes (a text box, up to eight fields, three menus, two checkboxes
//! and a slider, each labelled by the tool) and the server answers with text,
//! and for some tools a picture. Tools run again whenever an input changes.
//! This one holds no memory between sessions; it ticks only to know the time,
//! for the tools on dates and for random seeds.

mod calc;
mod code;
mod data;
mod dates;
mod encode;
mod markup;
mod math;
mod names;
mod pictures;
mod random;
mod science;
mod text;
mod units;
mod web;

use super::common::feedback;
use super::img;
use hypereffect::{Action, App, Handler, Outbox, Value, cel, json, ui};
use std::collections::BTreeMap;

/// The slots of the form: fields, menus and checkboxes.
const FIELDS: usize = 8;
const SELECTS: usize = 3;
const CHECKS: usize = 2;

pub type Run = fn(&str, &mut Opts) -> Result<String, String>;
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

    /// The JSON of the form, every slot present so that no expression reads
    /// a missing one.
    fn form_json(&self) -> Value {
        let fields: Vec<Value> = (0..FIELDS)
            .map(|i| json!({"label": self.fields.get(i).map_or("", |f| f.0)}))
            .collect();
        let selects: Vec<Value> = (0..SELECTS)
            .map(|i| match self.selects.get(i) {
                Some((label, options)) => json!({"label": label, "options": super::common::menu(options)}),
                None => json!({"label": "", "options": []}),
            })
            .collect();
        let checks: Vec<Value> = (0..CHECKS)
            .map(|i| json!(self.checks.get(i).copied().unwrap_or("")))
            .collect();
        let (label, min, max, _) = self.slider.unwrap_or(("", 0.0, 1.0, 0.0));
        json!({
            "about": self.about,
            "text": self.text,
            "fields": fields,
            "selects": selects,
            "checks": checks,
            "slider": {"label": label, "min": min, "max": max},
        })
    }

    /// The initial values of the form's slots.
    fn defaults(&self) -> Value {
        let mut form = hypereffect::Map::new();
        for i in 0..FIELDS {
            form.insert(format!("f{i}"), json!(self.fields.get(i).map_or("", |f| f.1)));
        }
        for i in 0..SELECTS {
            let first = self.selects.get(i).and_then(|s| s.1.first()).map_or("", |o| o.0);
            form.insert(format!("s{i}"), json!(first));
        }
        for i in 0..CHECKS {
            form.insert(format!("c{i}"), json!(false));
        }
        form.insert("slider".into(), json!(self.slider.map_or(0.0, |s| s.3)));
        Value::Object(form)
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
    /// A line to show under the result.
    pub info: Option<String>,
    /// A PNG to show with the result.
    pub image: Option<Vec<u8>>,
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
            info: None,
            image: None,
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
        self.labels.get(i).map_or_else(|| format!("Field {}", i + 1), |l| l.0.to_owned())
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
        self.opt_num(i)?.ok_or_else(|| format!("Fill in “{}”.", self.label(i)))
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
    t.replace('−', "-").parse::<f64>().ok().filter(|v| v.is_finite())
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
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { &s };
    let (whole, frac) = s.split_once('.').unwrap_or((s, ""));
    let sign = if v < 0.0 && s.chars().any(|c| c != '0' && c != '.') { "-" } else { "" };
    let grouped = super::common::group(whole);
    if frac.is_empty() { format!("{sign}{grouped}") } else { format!("{sign}{grouped}.{frac}") }
}

/// Aligned `label: value` lines.
pub fn table(rows: &[(&str, String)]) -> String {
    let width = rows.iter().map(|(l, _)| l.chars().count()).max().unwrap_or(0);
    rows.iter()
        .map(|(l, v)| {
            if l.is_empty() {
                v.clone()
            } else {
                format!("{l}:{} {v}", " ".repeat(width - l.chars().count()))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
    ("electronics", "Electronics and engineering", science::ELECTRONICS),
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

fn category(id: &str) -> &'static (&'static str, &'static str, &'static [Tool]) {
    CATEGORIES.iter().find(|c| c.0 == id).unwrap_or(&CATEGORIES[0])
}

fn find(id: &str) -> Option<&'static Tool> {
    CATEGORIES.iter().flat_map(|c| c.2.iter()).find(|t| t.id == id)
}

fn tool_menu(tools: &[Tool]) -> Vec<Value> {
    tools.iter().map(|t| json!({"value": t.id, "label": t.name})).collect()
}

#[derive(Default)]
pub struct TextUtils {
    rng: Option<Rng>,
    now_ms: f64,
}

impl TextUtils {
    /// Shows `tool`'s form with its initial values, and runs it.
    fn pick(&mut self, out: &mut Outbox, action: &Action, tool: &'static Tool) {
        out.signal(json!({"tool": tool.form_json(), "form": tool.defaults()}));
        // The action carries the old values: run on the new ones.
        let mut signals = action.signals.clone();
        if let Some(Value::Object(form)) = signals.get_mut("form") {
            if let Value::Object(d) = tool.defaults() {
                form.extend(d);
            }
            form.insert("tool".into(), json!(tool.id));
        }
        let action = Action { name: action.name.clone(), payload: None, signals };
        self.run(out, &action, true);
    }

    fn run(&mut self, out: &mut Outbox, action: &Action, quiet: bool) {
        let s = |path: &str| action.signal(path).and_then(Value::as_str).unwrap_or_default();
        let Some(tool) = find(s("form.tool")) else {
            return;
        };
        let text = s("form.text");
        if !tool.text.is_empty() && text.trim().is_empty() {
            out.signal(json!({"result": {"text": "", "image": "", "info": ""}}));
            out.signal(feedback(if quiet { "info" } else { "failure" }, if quiet { "" } else { "Paste some text first." }));
            return;
        }
        let fields: Vec<&str> = (0..FIELDS).map(|i| s(&format!("form.f{i}"))).collect();
        // A menu's value must be one of its options.
        let choices: Vec<&str> = (0..SELECTS)
            .map(|i| {
                let options = tool.selects.get(i).map_or(&[][..], |m| m.1);
                let v = s(&format!("form.s{i}"));
                options.iter().find(|o| o.0 == v).or(options.first()).map_or("", |o| o.0)
            })
            .collect();
        let checks: Vec<bool> = (0..CHECKS)
            .map(|i| action.signal(&format!("form.c{i}")).and_then(Value::as_bool).unwrap_or(false))
            .collect();
        let slider = match tool.slider {
            Some((_, min, max, value)) => {
                action.signal("form.slider").and_then(Value::as_f64).unwrap_or(value).clamp(min, max)
            }
            None => 0.0,
        };
        let now_ms = self.now_ms;
        let rng = self.rng.get_or_insert_with(|| Rng(now_ms.to_bits()));
        let mut o = Opts {
            mode: choices[0],
            first: fields[0],
            second: fields[1],
            flag: checks[0],
            number: slider.round() as usize,
            rng,
            now_ms,
            info: None,
            image: None,
            fields,
            choices,
            checks,
            labels: tool.fields,
        };
        match (tool.run)(text, &mut o) {
            Ok(result) => {
                let info = o.info.take().unwrap_or_else(|| {
                    if tool.text.is_empty() {
                        String::new()
                    } else if result.is_empty() {
                        "The result is empty.".into()
                    } else {
                        let lines = result.lines().count();
                        format!(
                            "{lines} line{} · {} characters",
                            if lines == 1 { "" } else { "s" },
                            result.chars().count()
                        )
                    }
                });
                let image = o.image.as_deref().map_or(String::new(), img::data_uri);
                out.signal(json!({"result": {"text": result, "image": image, "info": info}}));
                out.signal(feedback("info", ""));
            }
            Err(e) => {
                out.signal(json!({"result": {"text": "", "image": "", "info": ""}}));
                out.signal(feedback("failure", e));
            }
        }
    }
}

impl App for TextUtils {
    fn hello(&mut self, out: &mut Outbox, _: &BTreeMap<String, String>) {
        let (cat, _, tools) = CATEGORIES[0];
        let first = &tools[0];
        let mut form = first.defaults();
        form["category"] = json!(cat);
        form["tool"] = json!(first.id);
        form["text"] = json!("");
        let categories: Vec<Value> = CATEGORIES.iter().map(|(id, label, _)| json!({"value": id, "label": label})).collect();
        out.signal(json!({
            "form": form,
            "menu": {"categories": categories, "tools": tool_menu(tools)},
            "tool": first.form_json(),
            "result": {"text": "", "image": "", "info": ""},
            "feedback": {"kind": "info", "text": ""},
        }));
        out.element(ui::column("main").gap("large"));
        out.element(ui::text("title").parent("main").variant("h1").text("Text Utilities"));

        out.element(ui::card("picker").parent("main").gap("medium"));
        out.element(ui::row("menus").parent("picker").gap("medium").wrap(true));
        out.element(
            ui::select("category")
                .parent("menus")
                .label("Category")
                .bind("form.category")
                .options(cel("s.menu.categories"))
                .on_change(Handler::action("category")),
        );
        out.element(
            ui::select("tool")
                .parent("menus")
                .label("Tool")
                .bind("form.tool")
                .options(cel("s.menu.tools"))
                .on_change(Handler::action("tool")),
        );
        out.element(ui::text("about").parent("picker").variant("caption").text(cel("s.tool.about")));

        let rerun = || Handler::action("change");
        out.element(
            ui::card("options")
                .parent("main")
                .title("Options")
                .visible(cel(
                    "s.tool.fields.exists(f, f.label != '') || s.tool.selects.exists(m, m.label != '') \
                     || s.tool.checks.exists(c, c != '') || s.tool.slider.label != ''",
                )),
        );
        out.element(ui::row("option-menus").parent("options").gap("medium").wrap(true));
        for i in 0..SELECTS {
            out.element(
                ui::select(format!("s{i}"))
                    .parent("option-menus")
                    .label(cel(format!("s.tool.selects[{i}].label")))
                    .bind(format!("form.s{i}"))
                    .options(cel(format!("s.tool.selects[{i}].options")))
                    .visible(cel(format!("s.tool.selects[{i}].label != ''")))
                    .on_change(rerun()),
            );
        }
        out.element(ui::row("option-fields").parent("options").gap("medium").wrap(true));
        for i in 0..FIELDS {
            out.element(
                ui::input(format!("f{i}"))
                    .parent("option-fields")
                    .label(cel(format!("s.tool.fields[{i}].label")))
                    .bind(format!("form.f{i}"))
                    .visible(cel(format!("s.tool.fields[{i}].label != ''")))
                    .on_change(rerun())
                    .on_submit(rerun()),
            );
        }
        for i in 0..CHECKS {
            out.element(
                ui::checkbox(format!("c{i}"))
                    .parent("options")
                    .label(cel(format!("s.tool.checks[{i}]")))
                    .bind(format!("form.c{i}"))
                    .visible(cel(format!("s.tool.checks[{i}] != ''")))
                    .on_change(rerun()),
            );
        }
        out.element(
            ui::slider("slider")
                .parent("options")
                .label(cel("s.tool.slider.label + ': ' + string(int(s.form.slider))"))
                .bind("form.slider")
                .min(cel("s.tool.slider.min"))
                .max(cel("s.tool.slider.max"))
                .visible(cel("s.tool.slider.label != ''"))
                .on_change(rerun()),
        );

        out.element(
            ui::card("input")
                .parent("main")
                .title(cel("s.tool.text"))
                .visible(cel("s.tool.text != ''")),
        );
        out.element(
            ui::input("source")
                .parent("input")
                .bind("form.text")
                .variant("multiline")
                .placeholder("Paste text here…")
                .on_change(rerun()),
        );
        out.element(ui::row("actions").parent("main").gap("small").wrap(true));
        out.element(
            ui::button("run")
                .parent("actions")
                .label(cel("s.tool.text != '' ? 'Run' : 'Run again'"))
                .variant("primary")
                .on_press(Handler::action("run")),
        );
        out.element(
            ui::button("reuse")
                .parent("actions")
                .label("Use the result as input")
                .visible(cel("s.tool.text != '' && s.result.text != ''"))
                .on_press(Handler::action("change").with_patch(json!({"form": {"text": cel("s.result.text")}}))),
        );
        out.element(
            ui::button("clear")
                .parent("actions")
                .label("Clear")
                .variant("plain")
                .visible(cel("s.tool.text != '' && s.form.text != ''"))
                .on_press(Handler::action("change").with_patch(json!({"form": {"text": ""}}))),
        );

        out.element(ui::card("result").parent("main").title("Result").gap("medium"));
        out.element(
            ui::text("empty")
                .parent("result")
                .variant("caption")
                .visible(cel("s.result.text == '' && s.result.image == ''"))
                .text(cel("s.tool.text != '' ? 'Paste some text above.' : 'Fill in the options above.'")),
        );
        out.element(
            ui::image("picture")
                .parent("result")
                .variant("full")
                .visible(cel("s.result.image != ''"))
                .src(cel("s.result.image"))
                .alt(cel("s.result.text != '' ? s.result.text : s.tool.about")),
        );
        out.element(
            ui::text("output")
                .parent("result")
                .variant("code")
                .visible(cel("s.result.text != ''"))
                .text(cel("s.result.text")),
        );
        out.element(
            ui::badge("info")
                .parent("result")
                .variant("info")
                .visible(cel("s.result.info != ''"))
                .text(cel("s.result.info")),
        );
        out.element(
            ui::notice("feedback")
                .parent("main")
                .variant(cel("s.feedback.kind"))
                .visible(cel("s.feedback.text != ''"))
                .text(cel("s.feedback.text")),
        );
    }

    fn action(&mut self, out: &mut Outbox, action: &Action) {
        let s = |path: &str| action.signal(path).and_then(Value::as_str).unwrap_or_default();
        match action.name.as_str() {
            "category" => {
                let (id, _, tools) = category(s("form.category"));
                out.signal(json!({"form": {"category": id}, "menu": {"tools": tool_menu(tools)}}));
                self.pick(out, action, &tools[0]);
            }
            "tool" => {
                // A tool from another category is still a valid choice.
                let tool = find(s("form.tool")).unwrap_or(&category(s("form.category")).2[0]);
                self.pick(out, action, tool);
            }
            "change" => self.run(out, action, true),
            "run" => self.run(out, action, false),
            _ => {}
        }
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

    #[test]
    fn ids_are_unique_and_forms_fit() {
        let mut seen = HashSet::new();
        for (_, _, tools) in CATEGORIES {
            assert!(!tools.is_empty());
            for t in *tools {
                assert!(seen.insert(t.id), "{} twice", t.id);
                assert!(t.fields.len() <= FIELDS, "{}", t.id);
                assert!(t.selects.len() <= SELECTS, "{}", t.id);
                assert!(t.checks.len() <= CHECKS, "{}", t.id);
            }
        }
    }

    /// Every tool runs on its initial values (and a sample text) without
    /// panicking; form-only tools also succeed.
    #[test]
    fn every_tool_runs() {
        let mut l = Loopback::<TextUtils>::new();
        l.tick(1_700_000_000_000.0);
        l.write("form.text", json!("Hello World, 42 and 3.5 at 2024-01-02. <b>bold</b> {\"a\": 1}"));
        for (cat, _, tools) in CATEGORIES {
            l.write("form.category", json!(cat));
            assert!(l.trigger("category", "onChange"));
            for t in *tools {
                l.write("form.tool", json!(t.id));
                assert!(l.trigger("tool", "onChange"));
                if t.text.is_empty() {
                    assert_eq!(l.client.attr("feedback", "visible"), Some(json!(false)), "{}: {:?}", t.id, l.client.value("feedback.text"));
                }
            }
        }
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn counts_words() {
        let mut l = Loopback::<TextUtils>::new();
        l.write("form.tool", json!("counter"));
        assert!(l.trigger("tool", "onChange"));
        l.write("form.text", json!("rose rose tulip fern"));
        assert!(l.trigger("source", "onChange"));
        let out = l.client.attr("output", "text").unwrap();
        assert!(out.as_str().unwrap().contains("Words:                     4"), "{out}");
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn options_show_per_tool() {
        let mut l = Loopback::<TextUtils>::new();
        l.write("form.tool", json!("find-replace"));
        assert!(l.trigger("tool", "onChange"));
        assert_eq!(l.client.attr("f0", "visible"), Some(json!(true)));
        assert_eq!(l.client.attr("f0", "label"), Some(json!("Find")));
        assert_eq!(l.client.attr("f2", "visible"), Some(json!(false)));
        assert_eq!(l.client.attr("slider", "visible"), Some(json!(false)));
        l.write("form.text", json!("a cat, a Cat"));
        l.write("form.f0", json!("cat"));
        l.write("form.f1", json!("dog"));
        l.write("form.c0", json!(true));
        assert!(l.trigger("c0", "onChange"));
        assert_eq!(l.client.attr("output", "text"), Some(json!("a dog, a dog")));
        assert_eq!(l.client.attr("info", "text"), Some(json!("2 replaced")));
        // The result can become the input.
        assert!(l.trigger("reuse", "onPress"));
        assert_eq!(l.client.value("form.text"), Some(&json!("a dog, a dog")));
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn empty_text_is_rejected_on_run() {
        let mut l = Loopback::<TextUtils>::new();
        assert!(l.trigger("run", "onPress"));
        assert_eq!(l.client.attr("feedback", "text"), Some(json!("Paste some text first.")));
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn form_tools_run_at_once() {
        let mut l = Loopback::<TextUtils>::new();
        l.write("form.category", json!("health"));
        assert!(l.trigger("category", "onChange"));
        l.write("form.tool", json!("bmi"));
        assert!(l.trigger("tool", "onChange"));
        assert_eq!(l.client.attr("input", "visible"), Some(json!(false)));
        assert!(l.client.attr("output", "text").unwrap().as_str().unwrap().contains("BMI"));
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
