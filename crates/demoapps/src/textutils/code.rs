//! Tools on code: CSS, JavaScript, JSON and SQL formatting, JavaScript
//! objects to JSON, and JSON to CSV.

use super::Opts;
use super::Output;
use hypereffect::serde_json::{self, Map, Value};
use serde::Serialize;

/// JSON indented by `indent` spaces, or on one line when it is 0.
pub fn json_text(v: &Value, indent: usize) -> String {
    if indent == 0 {
        return v.to_string();
    }
    let pad = vec![b' '; indent];
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(
        &mut buf,
        serde_json::ser::PrettyFormatter::with_indent(&pad),
    );
    v.serialize(&mut ser).expect("JSON serializes");
    String::from_utf8(buf).expect("JSON is UTF-8")
}

fn parse_json(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("Not JSON: {e}."))
}

pub fn json(text: &str, o: &mut Opts) -> Result<Output, String> {
    let v = parse_json(text)?;
    Ok(json_text(&v, if o.mode == "minify" { 0 } else { o.number }).into())
}

/// A piece of code for the formatters: a string or comment kept verbatim,
/// or anything else.
#[derive(Debug, PartialEq)]
enum Piece<'a> {
    Quoted(&'a str),
    Comment(&'a str),
    Code(&'a str),
}

/// Splits code into strings, comments and the rest. `line` is the line
/// comment marker, if the language has one.
fn pieces<'a>(src: &'a str, quotes: &[char], line: Option<&str>) -> Vec<Piece<'a>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = src.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let rest = &src[i..];
        let end = if quotes.contains(&c) {
            let mut end = src.len();
            let mut escaped = false;
            for (j, d) in rest.char_indices().skip(1) {
                if escaped {
                    escaped = false;
                } else if d == '\\' {
                    escaped = true;
                } else if d == c {
                    end = i + j + 1;
                    break;
                }
            }
            Some((end, false))
        } else if rest.starts_with("/*") {
            Some((rest[2..].find("*/").map_or(src.len(), |j| i + j + 4), true))
        } else if line.is_some_and(|l| rest.starts_with(l)) {
            Some((rest.find('\n').map_or(src.len(), |j| i + j), true))
        } else {
            None
        };
        if let Some((end, comment)) = end {
            if start < i {
                out.push(Piece::Code(&src[start..i]));
            }
            out.push(if comment {
                Piece::Comment(&src[i..end])
            } else {
                Piece::Quoted(&src[i..end])
            });
            start = end;
            while chars.peek().is_some_and(|(j, _)| *j < end) {
                chars.next();
            }
        }
    }
    if start < src.len() {
        out.push(Piece::Code(&src[start..]));
    }
    out
}

/// Collapses whitespace, and drops it around the characters in `tight`.
fn squeeze(code: &str, tight: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for c in code.chars() {
        if c.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() && !tight.contains(c) && !out.ends_with(|p| tight.contains(p)) {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    if space && !out.is_empty() {
        out.push(' ');
    }
    out
}

/// Breaks code into lines at braces and semicolons, indenting blocks.
/// `parens` keeps semicolons inside parentheses on the line (as in a `for`).
fn blocks(
    src: &str,
    indent: usize,
    quotes: &[char],
    line_comment: Option<&str>,
    after_comma: bool,
) -> String {
    let pad = |depth: usize| " ".repeat(depth * indent);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut parens = 0usize;
    let push = |lines: &mut Vec<String>, current: &mut String, depth: usize| {
        let t = current.trim();
        if !t.is_empty() {
            lines.push(format!("{}{t}", pad(depth)));
        }
        current.clear();
    };
    for piece in pieces(src, quotes, line_comment) {
        match piece {
            Piece::Quoted(q) => current.push_str(q),
            Piece::Comment(c) => {
                if !current.trim().is_empty() && c.starts_with("//") {
                    current.push(' ');
                    current.push_str(c);
                    push(&mut lines, &mut current, depth);
                } else {
                    push(&mut lines, &mut current, depth);
                    current.push_str(c);
                    push(&mut lines, &mut current, depth);
                }
            }
            Piece::Code(code) => {
                let code = squeeze(code, "");
                for c in code.chars() {
                    match c {
                        '{' => {
                            let t = current.trim_end().to_owned();
                            current = t;
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push('{');
                            push(&mut lines, &mut current, depth);
                            depth += 1;
                        }
                        '}' => {
                            push(&mut lines, &mut current, depth);
                            depth = depth.saturating_sub(1);
                            current.push('}');
                        }
                        '(' | '[' => {
                            parens += 1;
                            current.push(c);
                        }
                        ')' | ']' => {
                            parens = parens.saturating_sub(1);
                            current.push(c);
                        }
                        ';' if parens == 0 => {
                            current.push(';');
                            push(&mut lines, &mut current, depth);
                        }
                        ',' if after_comma && parens == 0 && depth == 0 => {
                            current.push(',');
                            push(&mut lines, &mut current, depth);
                        }
                        ' ' if current.trim().is_empty() => {}
                        c => {
                            // A `}` followed by more on its line, as in `} else {`.
                            if current.trim() == "}" && !matches!(c, ';' | ',' | ')') {
                                if starts_word(c) {
                                    current.push(' ');
                                } else {
                                    push(&mut lines, &mut current, depth);
                                }
                            }
                            current.push(c);
                        }
                    }
                }
            }
        }
    }
    push(&mut lines, &mut current, depth);
    // Merge a lone `}` with what continues the statement.
    let mut merged: Vec<String> = Vec::new();
    for l in lines {
        let t = l.trim_start();
        if let Some(prev) = merged.last_mut() {
            if prev.trim() == "}"
                && (t.starts_with(';') || t.starts_with(',') || t.starts_with(')'))
            {
                prev.push_str(t);
                continue;
            }
        }
        merged.push(l);
    }
    merged.join("\n")
}

fn starts_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

fn minify(src: &str, quotes: &[char], line_comment: Option<&str>, tight: &str) -> String {
    // Whether `next` starts something that needs a space after `prev` when
    // two code pieces meet: `return` + `"x"`, or `let` + `a`.
    let separates = |prev: char, next: char| {
        let operand_end = prev.is_alphanumeric()
            || matches!(prev, '_' | '$' | '"' | '\'' | '`' | ')' | ']' | '}');
        let operand_start = next.is_alphanumeric()
            || matches!(next, '_' | '$' | '"' | '\'' | '`' | '(' | '[' | '{');
        operand_end && operand_start && !matches!(prev, ';' | ',' | '{' | '}')
    };
    let mut out = String::new();
    for piece in pieces(src, quotes, line_comment) {
        match piece {
            Piece::Quoted(q) => {
                if let (Some(prev), Some(next)) = (out.chars().last(), q.chars().next()) {
                    if separates(prev, next) {
                        out.push(' ');
                    }
                }
                out.push_str(q);
            }
            Piece::Comment(c) if c.starts_with("/*!") => out.push_str(c),
            Piece::Comment(_) => {}
            Piece::Code(code) => {
                let s = squeeze(code, tight);
                let s = s.trim();
                if s.is_empty() {
                    continue;
                }
                if let (Some(prev), Some(next)) = (out.chars().last(), s.chars().next()) {
                    if separates(prev, next) {
                        out.push(' ');
                    }
                }
                out.push_str(s);
            }
        }
    }
    out.trim().to_owned()
}

pub fn css(text: &str, o: &mut Opts) -> Result<Output, String> {
    if o.mode == "minify" {
        return Ok(minify(text, &['"', '\''], None, "{}:;,>+~()")
            .replace(";}", "}")
            .into());
    }
    let formatted = blocks(text, o.number, &['"', '\''], None, false);
    // One space after a property's colon, and an empty line between rules.
    let mut out = Vec::new();
    for line in formatted.lines() {
        let t = line.trim_start();
        let pad = &line[..line.len() - t.len()];
        // A rule that follows another is on its own line, with a blank line
        // between top-level rules.
        if let Some(rest) = t.strip_prefix("} ") {
            out.push(format!("{pad}}}"));
            if pad.is_empty() {
                out.push(String::new());
            }
            out.push(format!("{pad}{}", rest.replace(" ,", ",")));
            continue;
        }
        let t = if !t.ends_with('{') && !t.starts_with('@') {
            match t.split_once(':') {
                Some((k, v)) if !k.contains(['(', '"', '\'']) => {
                    format!("{}: {}", k.trim(), v.trim())
                }
                _ => t.replace(" ,", ","),
            }
        } else {
            t.replace(" ,", ",")
        };
        out.push(format!("{pad}{t}"));
        if t == "}" && pad.is_empty() {
            out.push(String::new());
        }
    }
    Ok(out.join("\n").trim_end().to_owned().into())
}

pub fn javascript(text: &str, o: &mut Opts) -> Result<Output, String> {
    if o.mode == "minify" {
        return Ok(minify(text, &['"', '\'', '`'], Some("//"), "{}()[];,:=+-*/<>!&|?").into());
    }
    let formatted = blocks(text, o.number, &['"', '\'', '`'], Some("//"), false);
    // Spaces around operators and after commas, outside strings.
    Ok(formatted
        .lines()
        .map(|line| {
            let t = line.trim_start();
            let pad = &line[..line.len() - t.len()];
            let mut out = String::from(pad);
            for p in pieces(t, &['"', '\'', '`'], Some("//")) {
                match p {
                    Piece::Code(c) => out.push_str(&space_operators(c)),
                    Piece::Quoted(q) | Piece::Comment(q) => out.push_str(q),
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn space_operators(code: &str) -> String {
    const OPS: [&str; 22] = [
        "===", "!==", "=>", "==", "!=", "<=", ">=", "&&", "||", "+=", "-=", "*=", "/=", "??", "=",
        "<", ">", "+", "-", "*", "/", "?",
    ];
    let mut out = String::new();
    let mut rest = code;
    while let Some(c) = rest.chars().next() {
        if c == ',' {
            out.push_str(", ");
            rest = rest[1..].trim_start();
            continue;
        }
        if let Some(op) = OPS.iter().find(|op| rest.starts_with(**op)) {
            // `++`, `--`, unary minus and `a.b` stay as they are.
            let prev = out.trim_end().chars().last();
            let unary = matches!(*op, "-" | "+")
                && (prev.is_none() || prev.is_some_and(|p| "=(,:?[!&|{;".contains(p)));
            let doubled = (*op == "+" || *op == "-") && rest[1..].starts_with(*op);
            if unary || doubled {
                let take = if doubled { 2 } else { 1 };
                out.push_str(&rest[..take]);
                rest = &rest[take..];
                continue;
            }
            let t = out.trim_end().len();
            out.truncate(t);
            out.push_str(&format!(" {op} "));
            rest = rest[op.len()..].trim_start();
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

const SQL_BREAK: [&str; 20] = [
    "SELECT",
    "FROM",
    "WHERE",
    "GROUP BY",
    "ORDER BY",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "UNION ALL",
    "UNION",
    "INSERT INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE FROM",
    "LEFT JOIN",
    "RIGHT JOIN",
    "INNER JOIN",
    "JOIN",
    "RETURNING",
];
const SQL_WORDS: [&str; 34] = [
    "SELECT",
    "FROM",
    "WHERE",
    "GROUP",
    "BY",
    "ORDER",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "UNION",
    "ALL",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "LEFT",
    "RIGHT",
    "INNER",
    "OUTER",
    "JOIN",
    "ON",
    "AND",
    "OR",
    "NOT",
    "AS",
    "IN",
    "IS",
    "NULL",
    "DISTINCT",
    "ASC",
    "DESC",
    "RETURNING",
];

pub fn sql(text: &str, o: &mut Opts) -> Result<Output, String> {
    // Words, with keywords in capitals when asked.
    let mut flat = String::new();
    for p in pieces(text, &['\'', '"', '`'], Some("--")) {
        match p {
            Piece::Quoted(q) => flat.push_str(q),
            Piece::Comment(_) => flat.push(' '),
            Piece::Code(c) => {
                let mut word = String::new();
                for ch in c.chars().chain(std::iter::once(' ')) {
                    if ch.is_alphanumeric() || ch == '_' {
                        word.push(ch);
                        continue;
                    }
                    if !word.is_empty() {
                        let upper = word.to_uppercase();
                        flat.push_str(if o.flag && SQL_WORDS.contains(&upper.as_str()) {
                            &upper
                        } else {
                            &word
                        });
                        word.clear();
                    }
                    flat.push(ch);
                }
                flat.pop();
            }
        }
    }
    let flat = squeeze(&flat, "");
    let flat = flat
        .replace(" ,", ",")
        .replace("( ", "(")
        .replace(" )", ")");
    if o.mode == "minify" {
        return Ok(flat.trim().to_owned().into());
    }
    let pad = " ".repeat(o.number);
    let mut out = String::new();
    let mut depth = 0usize;
    let mut clause = "";
    let mut rest = flat.trim();
    let mut quoted = false;
    while let Some(c) = rest.chars().next() {
        if c == '\'' {
            quoted = !quoted;
        }
        let at_word = !quoted && !out.ends_with(|p: char| p.is_alphanumeric() || p == '_');
        let keyword = SQL_BREAK.iter().find(|k| {
            at_word
                && rest.len() >= k.len()
                && rest[..k.len()].eq_ignore_ascii_case(k)
                && !rest[k.len()..].starts_with(|n: char| n.is_alphanumeric() || n == '_')
        });
        if let Some(k) = keyword {
            let t = out.trim_end().len();
            out.truncate(t);
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&pad.repeat(depth));
            out.push_str(&rest[..k.len()]);
            clause = k;
            rest = rest[k.len()..].trim_start();
            if !rest.is_empty() {
                out.push('\n');
                out.push_str(&pad.repeat(depth + 1));
            }
            continue;
        }
        let is = |w: &str| {
            at_word
                && rest.len() > w.len()
                && rest[..w.len()].eq_ignore_ascii_case(w)
                && rest[w.len()..].starts_with(' ')
        };
        if (is("AND") || is("OR")) && clause != "SELECT" {
            let t = out.trim_end().len();
            out.truncate(t);
            out.push('\n');
            out.push_str(&pad.repeat(depth + 1));
        }
        match c {
            ',' if !quoted && clause == "SELECT" || c == ',' && !quoted && clause == "SET" => {
                out.push(',');
                out.push('\n');
                out.push_str(&pad.repeat(depth + 1));
                rest = rest[1..].trim_start();
                continue;
            }
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.saturating_sub(1),
            ';' if !quoted => {
                out.push_str(";\n");
                rest = rest[1..].trim_start();
                clause = "";
                continue;
            }
            _ => {}
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    Ok(out.trim_end().to_owned().into())
}

/// A JavaScript object literal, read leniently: unquoted and single-quoted
/// keys, single-quoted strings, comments, trailing commas, `undefined`.
struct Js<'a> {
    s: &'a [u8],
    src: &'a str,
    i: usize,
}

impl Js<'_> {
    fn skip(&mut self) {
        loop {
            while self.s.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
                self.i += 1;
            }
            if self.src[self.i..].starts_with("//") {
                self.i = self.src[self.i..]
                    .find('\n')
                    .map_or(self.s.len(), |j| self.i + j);
            } else if self.src[self.i..].starts_with("/*") {
                self.i = self.src[self.i + 2..]
                    .find("*/")
                    .map_or(self.s.len(), |j| self.i + j + 4);
            } else {
                return;
            }
        }
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        let line = self.src[..self.i.min(self.src.len())].matches('\n').count() + 1;
        Err(format!("Expected {what} on line {line}."))
    }

    fn eat(&mut self, c: u8) -> bool {
        self.skip();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> Result<String, String> {
        let quote = self.s[self.i];
        self.i += 1;
        let mut out = String::new();
        let rest = &self.src[self.i..];
        let mut chars = rest.char_indices();
        while let Some((j, c)) = chars.next() {
            if c as u32 == quote as u32 {
                self.i += j + 1;
                return Ok(out);
            }
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next().map(|(_, e)| e) {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('0') => out.push('\0'),
                Some('u') => {
                    let hex: String = (0..4)
                        .filter_map(|_| chars.next().map(|(_, h)| h))
                        .collect();
                    let c = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
                    out.push(c.unwrap_or('\u{fffd}'));
                }
                Some('\n') => {}
                Some(e) => out.push(e),
                None => break,
            }
        }
        self.err("a closing quote")
    }

    fn ident(&mut self) -> &str {
        let start = self.i;
        while self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'$' || *c >= 0x80)
        {
            self.i += 1;
        }
        &self.src[start..self.i]
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip();
        match self.s.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                let mut m = Map::new();
                while !self.eat(b'}') {
                    self.skip();
                    let key = match self.s.get(self.i) {
                        Some(b'"' | b'\'' | b'`') => self.string()?,
                        _ => {
                            let k = self.ident().to_owned();
                            if k.is_empty() {
                                return self.err("a key");
                            }
                            k
                        }
                    };
                    if !self.eat(b':') {
                        return self.err("“:” after a key");
                    }
                    let v = self.value()?;
                    m.insert(key, v);
                    if !self.eat(b',') && !self.eat(b'}') {
                        return self.err("“,” or “}”");
                    } else if self.s.get(self.i - 1) == Some(&b'}') {
                        break;
                    }
                }
                Ok(Value::Object(m))
            }
            Some(b'[') => {
                self.i += 1;
                let mut v = Vec::new();
                while !self.eat(b']') {
                    v.push(self.value()?);
                    if !self.eat(b',') && !self.eat(b']') {
                        return self.err("“,” or “]”");
                    } else if self.s.get(self.i - 1) == Some(&b']') {
                        break;
                    }
                }
                Ok(Value::Array(v))
            }
            Some(b'"' | b'\'' | b'`') => self.string().map(Value::String),
            Some(_) => {
                let start = self.i;
                let word = self.ident().to_owned();
                match word.as_str() {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    "null" | "undefined" | "NaN" | "Infinity" => Ok(Value::Null),
                    _ => {
                        self.i = start;
                        while self
                            .s
                            .get(self.i)
                            .is_some_and(|c| c.is_ascii_alphanumeric() || b"+-.".contains(c))
                        {
                            self.i += 1;
                        }
                        let n = &self.src[start..self.i];
                        let parsed = if let Some(h) = n.strip_prefix("0x") {
                            i64::from_str_radix(h, 16).ok().map(Value::from)
                        } else {
                            n.parse::<i64>().ok().map(Value::from).or_else(|| {
                                n.parse::<f64>().ok().and_then(|f| {
                                    serde_json::Number::from_f64(f).map(Value::Number)
                                })
                            })
                        };
                        match parsed {
                            Some(v) => Ok(v),
                            None => {
                                self.i = start;
                                self.err("a value")
                            }
                        }
                    }
                }
            }
            None => self.err("a value"),
        }
    }
}

/// A JavaScript value, or JSON with the usual mistakes, as JSON.
pub fn lenient_json(text: &str) -> Result<Value, String> {
    // Allow `const x = {...};` and `module.exports = ...`.
    let t = text.trim().trim_end_matches(';');
    let t = match t.find(['{', '[']) {
        Some(i) if t[..i].trim_end().ends_with('=') => &t[i..],
        _ => t,
    };
    let mut p = Js {
        s: t.as_bytes(),
        src: t,
        i: 0,
    };
    let v = p.value()?;
    p.skip();
    if p.i < t.len() {
        return p.err("the end of the object");
    }
    Ok(v)
}

pub fn js_object(text: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(json_text(&lenient_json(text)?, o.number).into())
}

/// Flattens nested objects to `a.b` columns; arrays of scalars are joined.
fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        Value::Array(a) if a.iter().all(|v| !v.is_object() && !v.is_array()) => {
            let joined: Vec<String> = a.iter().map(scalar).collect();
            out.push((prefix.to_owned(), joined.join(", ")));
        }
        Value::Array(_) => out.push((prefix.to_owned(), v.to_string())),
        v => out.push((prefix.to_owned(), scalar(v))),
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}

pub fn json_to_csv(text: &str, o: &mut Opts) -> Result<Output, String> {
    let v = parse_json(text)?;
    let rows: Vec<Value> = match v {
        Value::Array(a) => a,
        Value::Object(ref m) if m.values().any(Value::is_array) && m.len() == 1 => {
            m.values().next().unwrap().as_array().unwrap().clone()
        }
        v => vec![v],
    };
    let sep = match o.mode {
        "semicolon" => ";",
        "tab" => "\t",
        _ => ",",
    };
    let flat: Vec<Vec<(String, String)>> = rows
        .iter()
        .map(|r| {
            let mut cells = Vec::new();
            flatten(if r.is_object() { "" } else { "value" }, r, &mut cells);
            cells
        })
        .collect();
    let mut columns: Vec<String> = Vec::new();
    for row in &flat {
        for (k, _) in row {
            if !columns.contains(k) {
                columns.push(k.clone());
            }
        }
    }
    let cell = |s: &str| {
        if s.contains([',', ';', '\t', '"', '\n', '\r']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_owned()
        }
    };
    let mut out = vec![
        columns
            .iter()
            .map(|c| cell(c))
            .collect::<Vec<_>>()
            .join(sep),
    ];
    for row in &flat {
        out.push(
            columns
                .iter()
                .map(|c| {
                    row.iter()
                        .find(|(k, _)| k == c)
                        .map_or(String::new(), |(_, v)| cell(v))
                })
                .collect::<Vec<_>>()
                .join(sep),
        );
    }
    Ok(out.join("\n").into())
}

use super::{Tool, tool};

const LAYOUT: super::Menu = &[("beautify", "Beautify"), ("minify", "Minify")];

pub const FORMAT: &[Tool] = &[
    tool("css", "CSS formatter", "Indents CSS, or minifies it.", css)
        .text("CSS")
        .selects(&[("Layout", LAYOUT)])
        .slider("Indent", 1.0, 8.0, 2.0),
    tool(
        "html",
        "HTML formatter",
        "Indents HTML, or minifies it.",
        super::markup::html,
    )
    .text("HTML")
    .selects(&[("Layout", LAYOUT)])
    .slider("Indent", 1.0, 8.0, 2.0),
    tool(
        "javascript",
        "JavaScript formatter",
        "Indents JavaScript, or minifies it. Also unminifies code.",
        javascript,
    )
    .text("JavaScript")
    .selects(&[("Layout", LAYOUT)])
    .slider("Indent", 1.0, 8.0, 2.0),
    tool(
        "json",
        "JSON formatter",
        "Pretty-prints JSON with any indent, or puts it on one line. Also validates it.",
        json,
    )
    .text("JSON")
    .selects(&[("Layout", LAYOUT)])
    .slider("Indent", 1.0, 8.0, 2.0),
    tool(
        "sql",
        "SQL formatter",
        "Lays out SQL queries a clause per line.",
        sql,
    )
    .text("SQL")
    .selects(&[("Layout", LAYOUT)])
    .checks(&["Keywords in capitals"])
    .slider("Indent", 1.0, 8.0, 2.0),
    tool(
        "xml",
        "XML formatter",
        "Indents XML, or minifies it. Also checks that it is well formed.",
        super::markup::xml,
    )
    .text("XML")
    .selects(&[("Layout", LAYOUT)])
    .slider("Indent", 1.0, 8.0, 2.0),
];

#[cfg(test)]
mod tests {
    use super::super::{Opts, Rng};
    use super::*;

    fn run(f: fn(&str, &mut Opts) -> Result<Output, String>, text: &str, mode: &str) -> String {
        let mut rng = Rng(1);
        let mut o = Opts::new(mode, &mut rng);
        o.number = 2;
        o.flag = true;
        f(text, &mut o).unwrap().plain()
    }

    #[test]
    fn formats_json() {
        assert_eq!(
            run(json, r#"{"a":[1,2]}"#, "beautify"),
            "{\n  \"a\": [\n    1,\n    2\n  ]\n}"
        );
        assert_eq!(run(json, "{ \"a\" : 1 }", "minify"), r#"{"a":1}"#);
    }

    #[test]
    fn formats_css() {
        assert_eq!(
            run(css, "a{color:red;margin:0 auto}b , i{x:1}", "beautify"),
            "a {\n  color: red;\n  margin: 0 auto\n}\n\nb, i {\n  x: 1\n}"
        );
        assert_eq!(
            run(css, "a {\n  color : red ;\n}\n/* c */", "minify"),
            "a{color:red}"
        );
    }

    #[test]
    fn formats_javascript() {
        assert_eq!(
            run(
                javascript,
                "function f(a,b){if(a===b){return \"x;y\"}else{return a+b;}}",
                "beautify"
            ),
            "function f(a, b) {\n  if(a === b) {\n    return \"x;y\"\n  } else {\n    return a + b;\n  }\n}"
        );
        assert_eq!(
            run(javascript, "let a = 1; // one\nlet b = 'x  y';", "minify"),
            "let a=1;let b='x  y';"
        );
    }

    #[test]
    fn formats_sql() {
        assert_eq!(
            run(
                sql,
                "select a, b from t where x = 'a, b' and y > 2 order by a",
                "beautify"
            ),
            "SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  x = 'a, b'\n  AND y > 2\nORDER BY\n  a"
        );
    }

    #[test]
    fn javascript_objects() {
        assert_eq!(
            run(
                js_object,
                "const o = { a: 'x', \"b\": [1, 2.5,], // c\n c: undefined, d: true };",
                ""
            ),
            "{\n  \"a\": \"x\",\n  \"b\": [\n    1,\n    2.5\n  ],\n  \"c\": null,\n  \"d\": true\n}"
        );
        let mut rng = Rng(1);
        assert!(js_object("{a: }", &mut Opts::new("", &mut rng)).is_err());
    }

    #[test]
    fn csv() {
        assert_eq!(
            run(
                json_to_csv,
                r#"[{"n":"a, b","x":{"y":1}},{"n":"c","z":[1,2]}]"#,
                "comma"
            ),
            "n,x.y,z\n\"a, b\",1,\nc,,\"1, 2\""
        );
    }
}
