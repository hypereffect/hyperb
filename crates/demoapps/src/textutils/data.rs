//! Data formats (CSV, YAML, .env) and sorting.

use super::Output;
use super::code::{json_text, lenient_json};
use super::{Opts, Tool, grid, markup, text, tool, verdict};
use hypereffect::serde_json::{self, Map, Value};
use std::cmp::Ordering;

// CSV

/// The delimiter of CSV text: the one that the first line has most of.
fn sniff(text: &str, mode: &str) -> char {
    match mode {
        "comma" => ',',
        "semicolon" => ';',
        "tab" => '\t',
        "pipe" => '|',
        _ => {
            let first = text.lines().next().unwrap_or("");
            [',', ';', '\t', '|']
                .into_iter()
                .max_by_key(|d| first.matches(*d).count())
                .unwrap_or(',')
        }
    }
}

/// Rows of cells, with quoted cells and doubled quotes.
pub fn parse_csv(text: &str, delim: char) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut line = 1;
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                c => {
                    line += (c == '\n') as usize;
                    cell.push(c);
                }
            }
            continue;
        }
        match c {
            '"' if cell.trim().is_empty() => {
                cell.clear();
                quoted = true;
            }
            '\r' => {}
            '\n' => {
                line += 1;
                row.push(std::mem::take(&mut cell));
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            c if c == delim => row.push(std::mem::take(&mut cell)),
            c => cell.push(c),
        }
    }
    if quoted {
        return Err(format!("A quoted cell is never closed (line {line})."));
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    if rows.is_empty() {
        return Err("There are no rows.".into());
    }
    Ok(rows)
}

fn cell_value(s: &str, typed: bool) -> Value {
    if typed {
        let t = s.trim();
        if let Ok(n) = t.parse::<i64>() {
            return Value::from(n);
        }
        if let Some(n) = t.parse::<f64>().ok().and_then(serde_json::Number::from_f64) {
            return Value::Number(n);
        }
        match t {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            "" | "null" => return Value::Null,
            _ => {}
        }
    }
    Value::String(s.to_owned())
}

fn csv_to_json(text: &str, o: &mut Opts) -> Result<Output, String> {
    let rows = parse_csv(text, sniff(text, o.mode))?;
    let typed = o.check(0);
    let out: Vec<Value> = if o.check(1) {
        rows.iter()
            .map(|r| Value::Array(r.iter().map(|c| cell_value(c, typed)).collect()))
            .collect()
    } else {
        let header = &rows[0];
        rows[1..]
            .iter()
            .map(|r| {
                let mut m = Map::new();
                for (i, h) in header.iter().enumerate() {
                    m.insert(
                        h.trim().to_owned(),
                        r.get(i).map_or(Value::Null, |c| cell_value(c, typed)),
                    );
                }
                Value::Object(m)
            })
            .collect()
    };
    Ok(json_text(&Value::Array(out), o.number).into())
}

fn csv_to_markdown(text: &str, o: &mut Opts) -> Result<Output, String> {
    let rows = parse_csv(text, sniff(text, o.mode))?;
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let esc = |s: &str| s.trim().replace('|', "\\|").replace('\n', "<br>");
    let width: Vec<usize> = (0..cols)
        .map(|i| {
            rows.iter()
                .map(|r| r.get(i).map_or(0, |c| esc(c).chars().count()))
                .max()
                .unwrap_or(0)
                .max(3)
        })
        .collect();
    let line = |r: &Vec<String>| {
        let cells: Vec<String> = (0..cols)
            .map(|i| {
                let c = esc(r.get(i).map_or("", |c| c.as_str()));
                format!("{c}{}", " ".repeat(width[i] - c.chars().count()))
            })
            .collect();
        format!("| {} |", cells.join(" | "))
    };
    let mut out = vec![line(&rows[0])];
    out.push(format!(
        "|{}|",
        width
            .iter()
            .map(|w| format!(" {} ", "-".repeat(*w)))
            .collect::<Vec<_>>()
            .join("|")
    ));
    out.extend(rows[1..].iter().map(line));
    Ok(out.join("\n").into())
}

fn table_format(text: &str, o: &mut Opts) -> Result<Output, String> {
    let rows = parse_csv(text, sniff(text, o.mode))?;
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let width: Vec<usize> = (0..cols)
        .map(|i| {
            rows.iter()
                .map(|r| r.get(i).map_or(0, |c| c.trim().chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let numeric = |i: usize| {
        rows[1..].iter().all(|r| {
            r.get(i)
                .is_none_or(|c| c.trim().is_empty() || super::parse_number(c).is_some())
        })
    };
    let line = |r: &Vec<String>, head: bool| {
        let cells: Vec<String> = (0..cols)
            .map(|i| {
                let c = r.get(i).map_or("", |c| c.trim());
                let pad = " ".repeat(width[i] - c.chars().count());
                if numeric(i) && !head {
                    format!("{pad}{c}")
                } else {
                    format!("{c}{pad}")
                }
            })
            .collect();
        format!("│ {} │", cells.join(" │ "))
    };
    let rule = |l: &str, m: &str, r: &str| {
        format!(
            "{l}{}{r}",
            width
                .iter()
                .map(|w| "─".repeat(w + 2))
                .collect::<Vec<_>>()
                .join(m)
        )
    };
    let mut out = vec![rule("┌", "┬", "┐"), line(&rows[0], true)];
    if !o.flag {
        out.push(rule("├", "┼", "┤"));
    }
    out.extend(rows[1..].iter().map(|r| line(r, false)));
    out.push(rule("└", "┴", "┘"));
    Ok(out.join("\n").into())
}

fn csv_validate(text: &str, o: &mut Opts) -> Result<Output, String> {
    let delim = sniff(text, o.mode);
    let rows = parse_csv(text, delim)?;
    let cols = rows[0].len();
    let mut problems = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for h in &rows[0] {
        if h.trim().is_empty() {
            problems.push("The header has an empty column name.".to_owned());
        } else if !seen.insert(h.trim().to_lowercase()) {
            problems.push(format!("The column “{}” appears twice.", h.trim()));
        }
    }
    for (i, r) in rows.iter().enumerate().skip(1) {
        if r.len() != cols {
            problems.push(format!(
                "Row {}: {} cells, but the header has {cols}.",
                i + 1,
                r.len()
            ));
        }
        let empty = r.iter().filter(|c| c.trim().is_empty()).count();
        if empty > 0 && o.flag {
            problems.push(format!("Row {}: {empty} empty cells.", i + 1));
        }
    }
    let name = match delim {
        '\t' => "tabs".to_owned(),
        d => format!("“{d}”"),
    };
    let summary = format!(
        "{} rows of {cols} columns, separated by {name}.",
        rows.len() - 1
    );
    if problems.is_empty() {
        return Ok(verdict(true, format!("Valid: {summary}")));
    }
    let n = problems.len();
    Ok(verdict(
        false,
        format!("{n} problem{}. {summary}", if n == 1 { "" } else { "s" }),
    )
    .list(problems))
}

fn duplicate_finder(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut counts: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let key = if o.flag {
            l.trim().to_lowercase()
        } else {
            l.trim().to_owned()
        };
        if key.is_empty() {
            continue;
        }
        match counts.iter_mut().find(|c| c.0 == key) {
            Some(c) => c.1.push(i + 1),
            None => counts.push((key, vec![i + 1])),
        }
    }
    let dups: Vec<Vec<String>> = counts
        .iter()
        .filter(|c| c.1.len() > 1)
        .map(|(k, at)| {
            vec![
                k.clone(),
                format!("{}×", at.len()),
                at.iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            ]
        })
        .collect();
    let dups_len = dups.len();
    if dups.is_empty() {
        return Ok(verdict(true, "No duplicates."));
    }
    Ok(grid(&["Value", "Times", "Lines"], dups).note(format!("{} repeated values", dups_len)))
}

// YAML

fn yaml_key(k: &str) -> String {
    let plain = !k.is_empty()
        && k.chars()
            .all(|c| c.is_alphanumeric() || "_-./ ".contains(c))
        && !k.starts_with([' ', '-'])
        && !k.ends_with(' ');
    if plain {
        k.to_owned()
    } else {
        serde_json::to_string(k).unwrap()
    }
}

fn yaml_string(s: &str) -> String {
    let special = s.is_empty()
        || s.starts_with(|c: char| "-?:,[]{}#&*!|>'\"%@` ".contains(c))
        || s.ends_with(' ')
        || s.contains(": ")
        || s.contains(" #")
        || s.contains('\n')
        || matches!(
            s.to_lowercase().as_str(),
            "true" | "false" | "null" | "yes" | "no" | "on" | "off" | "~"
        )
        || s.parse::<f64>().is_ok();
    if special {
        serde_json::to_string(s).unwrap()
    } else {
        s.to_owned()
    }
}

fn to_yaml(v: &Value, indent: usize, out: &mut String) {
    let pad = " ".repeat(indent);
    match v {
        Value::Object(m) if !m.is_empty() => {
            for (k, v) in m {
                out.push_str(&format!("{pad}{}:", yaml_key(k)));
                yaml_child(v, indent, out);
            }
        }
        Value::Array(a) if !a.is_empty() => {
            for v in a {
                out.push_str(&format!("{pad}-"));
                match v {
                    Value::Object(m) if !m.is_empty() => {
                        // The first key goes on the dash's line.
                        let mut inner = String::new();
                        to_yaml(v, indent + 2, &mut inner);
                        out.push(' ');
                        out.push_str(inner.trim_start());
                    }
                    Value::Array(a) if !a.is_empty() => {
                        out.push('\n');
                        to_yaml(v, indent + 2, out);
                    }
                    v => {
                        out.push(' ');
                        out.push_str(&yaml_scalar(v));
                        out.push('\n');
                    }
                }
            }
        }
        v => {
            out.push_str(&pad);
            out.push_str(&yaml_scalar(v));
            out.push('\n');
        }
    }
}

fn yaml_child(v: &Value, indent: usize, out: &mut String) {
    match v {
        Value::Object(m) if !m.is_empty() => {
            out.push('\n');
            to_yaml(v, indent + 2, out);
        }
        Value::Array(a) if !a.is_empty() => {
            out.push('\n');
            to_yaml(v, indent + 2, out);
        }
        Value::String(s) if s.contains('\n') => {
            out.push_str(" |\n");
            for l in s.trim_end_matches('\n').lines() {
                out.push_str(&format!("{}{l}\n", " ".repeat(indent + 2)));
            }
        }
        v => {
            out.push(' ');
            out.push_str(&yaml_scalar(v));
            out.push('\n');
        }
    }
}

fn yaml_scalar(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::String(s) => yaml_string(s),
        Value::Object(_) => "{}".into(),
        Value::Array(_) => "[]".into(),
        v => v.to_string(),
    }
}

fn json_to_yaml(text: &str, _: &mut Opts) -> Result<Output, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("Not JSON: {e}."))?;
    let mut out = String::new();
    to_yaml(&v, 0, &mut out);
    Ok(out.trim_end().to_owned().into())
}

/// The part of a YAML line before a comment.
fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut prev = ' ';
    for (i, c) in line.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') if prev == ' ' || prev == ':' || i == 0 => quote = Some(c),
            (None, '#') if prev == ' ' || i == 0 => return &line[..i],
            _ => {}
        }
        prev = c;
    }
    line
}

struct Yaml {
    lines: Vec<(usize, String, usize)>,
    i: usize,
}

/// The position of a mapping's `:` outside quotes and brackets.
fn key_colon(s: &str) -> Option<usize> {
    let mut quote = None;
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') if i == 0 => quote = Some(c),
            (None, '[' | '{') => depth += 1,
            (None, ']' | '}') => depth -= 1,
            (None, ':') if depth == 0 && (s[i + 1..].is_empty() || s[i + 1..].starts_with(' ')) => {
                return Some(i);
            }
            _ => {}
        }
    }
    None
}

fn yaml_plain(s: &str) -> Value {
    let s = s.trim();
    match s {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    if let Ok(n) = s.parse::<i64>() {
        return Value::from(n);
    }
    if let Some(h) = s
        .strip_prefix("0x")
        .and_then(|h| i64::from_str_radix(h, 16).ok())
    {
        return Value::from(h);
    }
    if let Some(n) = s
        .parse::<f64>()
        .ok()
        .filter(|f| f.is_finite())
        .and_then(serde_json::Number::from_f64)
    {
        return Value::Number(n);
    }
    if let Some(q) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return serde_json::from_str(&format!("\"{q}\"")).unwrap_or(Value::String(q.to_owned()));
    }
    if let Some(q) = s.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        return Value::String(q.replace("''", "'"));
    }
    if s.starts_with('[') || s.starts_with('{') {
        let mut pos = 0;
        if let Ok(v) = flow(s.as_bytes(), s, &mut pos) {
            return v;
        }
    }
    Value::String(s.to_owned())
}

/// A flow collection: `[a, b]`, `{k: v}`.
fn flow(b: &[u8], s: &str, pos: &mut usize) -> Result<Value, ()> {
    let skip = |pos: &mut usize| {
        while *pos < b.len() && b[*pos] == b' ' {
            *pos += 1;
        }
    };
    skip(pos);
    let scalar = |pos: &mut usize, stop: &[u8]| {
        let start = *pos;
        if b.get(*pos).is_some_and(|c| *c == b'"' || *c == b'\'') {
            let q = b[*pos];
            *pos += 1;
            while *pos < b.len() && b[*pos] != q {
                *pos += 1 + (b[*pos] == b'\\' && q == b'"') as usize;
            }
            *pos += 1;
        } else {
            while *pos < b.len() && !stop.contains(&b[*pos]) {
                *pos += 1;
            }
        }
        yaml_plain(s.get(start..(*pos).min(b.len())).unwrap_or(""))
    };
    match b.get(*pos) {
        Some(b'[') => {
            *pos += 1;
            let mut v = Vec::new();
            loop {
                skip(pos);
                match b.get(*pos) {
                    Some(b']') => {
                        *pos += 1;
                        return Ok(Value::Array(v));
                    }
                    Some(b',') => *pos += 1,
                    Some(b'[' | b'{') => v.push(flow(b, s, pos)?),
                    Some(_) => v.push(scalar(pos, b",]")),
                    None => return Err(()),
                }
            }
        }
        Some(b'{') => {
            *pos += 1;
            let mut m = Map::new();
            loop {
                skip(pos);
                match b.get(*pos) {
                    Some(b'}') => {
                        *pos += 1;
                        return Ok(Value::Object(m));
                    }
                    Some(b',') => *pos += 1,
                    Some(_) => {
                        let k = scalar(pos, b":,}");
                        let k = match k {
                            Value::String(s) => s,
                            v => v.to_string(),
                        };
                        if b.get(*pos) != Some(&b':') {
                            return Err(());
                        }
                        *pos += 1;
                        skip(pos);
                        let v = if matches!(b.get(*pos), Some(b'[' | b'{')) {
                            flow(b, s, pos)?
                        } else {
                            scalar(pos, b",}")
                        };
                        m.insert(k, v);
                    }
                    None => return Err(()),
                }
            }
        }
        _ => Err(()),
    }
}

impl Yaml {
    fn new(text: &str) -> Result<Yaml, String> {
        let mut lines = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            if raw.contains('\t') && raw.trim_start_matches(' ').starts_with('\t') {
                return Err(format!(
                    "Line {}: YAML is indented with spaces, not tabs.",
                    n + 1
                ));
            }
            let t = raw.trim_end();
            if t == "---" || t == "..." || t.starts_with('%') {
                continue;
            }
            let indent = t.len() - t.trim_start().len();
            lines.push((indent, t.trim_start().to_owned(), n + 1));
        }
        Ok(Yaml { lines, i: 0 })
    }

    /// The next line with content, skipping comments; block scalars keep
    /// their own lines.
    fn peek(&mut self) -> Option<(usize, String)> {
        while let Some((ind, l, _)) = self.lines.get(self.i) {
            let c = strip_comment(l).trim_end();
            if c.is_empty() {
                self.i += 1;
                continue;
            }
            return Some((*ind, c.to_owned()));
        }
        None
    }

    fn block_scalar(&mut self, header: &str, parent: usize) -> Value {
        let fold = header.starts_with('>');
        let mut body: Vec<String> = Vec::new();
        let mut indent = None;
        while let Some((ind, l, _)) = self.lines.get(self.i) {
            if l.is_empty() {
                body.push(String::new());
                self.i += 1;
                continue;
            }
            if *ind <= parent {
                break;
            }
            let base = *indent.get_or_insert(*ind);
            body.push(format!("{}{l}", " ".repeat(ind.saturating_sub(base))));
            self.i += 1;
        }
        while body.last().is_some_and(String::is_empty) {
            body.pop();
        }
        let mut s = if fold {
            body.split(|l| l.is_empty())
                .map(|p| p.join(" "))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            body.join("\n")
        };
        if !header.contains('-') {
            s.push('\n');
        }
        Value::String(s)
    }

    fn value(&mut self, indent: usize) -> Result<Value, String> {
        let Some((ind, line)) = self.peek() else {
            return Ok(Value::Null);
        };
        if ind < indent {
            return Ok(Value::Null);
        }
        if line == "-" || line.starts_with("- ") {
            return self.sequence(ind);
        }
        if key_colon(&line).is_some() {
            return self.mapping(ind);
        }
        self.i += 1;
        Ok(yaml_plain(&line))
    }

    fn sequence(&mut self, indent: usize) -> Result<Value, String> {
        let mut v = Vec::new();
        while let Some((ind, line)) = self.peek() {
            if ind != indent || !(line == "-" || line.starts_with("- ")) {
                break;
            }
            let rest = line[1..].trim_start().to_owned();
            if rest.is_empty() {
                self.i += 1;
                v.push(self.value(indent + 1)?);
            } else if key_colon(&rest).is_some() || rest.starts_with("- ") {
                // `- key: value`: the item continues at the key's column.
                let col = indent + (line.len() - rest.len());
                self.lines[self.i] = (col, rest, self.lines[self.i].2);
                v.push(self.value(col)?);
            } else if rest.starts_with('|') || rest.starts_with('>') {
                self.i += 1;
                v.push(self.block_scalar(&rest, indent));
            } else {
                self.i += 1;
                v.push(yaml_plain(&rest));
            }
        }
        Ok(Value::Array(v))
    }

    fn mapping(&mut self, indent: usize) -> Result<Value, String> {
        let mut m = Map::new();
        while let Some((ind, line)) = self.peek() {
            if ind < indent {
                break;
            }
            let n = self.lines[self.i].2;
            if ind > indent {
                return Err(format!("Line {n}: unexpected indentation."));
            }
            let Some(colon) = key_colon(&line) else {
                return Err(format!("Line {n}: expected “key: value”."));
            };
            let key = match yaml_plain(&line[..colon]) {
                Value::String(s) => s,
                v => v.to_string(),
            };
            let rest = line[colon + 1..].trim().to_owned();
            self.i += 1;
            let value = if rest.is_empty() {
                match self.peek() {
                    Some((i2, l2))
                        if i2 > indent || (i2 == indent && (l2 == "-" || l2.starts_with("- "))) =>
                    {
                        self.value(i2)?
                    }
                    _ => Value::Null,
                }
            } else if rest.starts_with('|') || rest.starts_with('>') {
                self.block_scalar(&rest, indent)
            } else {
                yaml_plain(&rest)
            };
            m.insert(key, value);
        }
        Ok(Value::Object(m))
    }
}

fn yaml_to_json(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut y = Yaml::new(text)?;
    let v = y.value(0)?;
    if let Some((_, l)) = y.peek() {
        let n = y.lines[y.i].2;
        return Err(format!("Line {n}: cannot read “{l}”."));
    }
    Ok(json_text(&v, o.number).into())
}

fn json_repair(text: &str, o: &mut Opts) -> Result<Output, String> {
    let v = lenient_json(text)?;
    let valid = serde_json::from_str::<Value>(text).is_ok();
    Ok(Output::from(json_text(&v, o.number)).note(if valid {
        "It was valid already."
    } else {
        "Repaired."
    }))
}

fn env_to_json(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut m = Map::new();
    for (n, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let l = l.strip_prefix("export ").unwrap_or(l);
        let (k, v) = l
            .split_once('=')
            .ok_or_else(|| format!("Line {}: expected KEY=value.", n + 1))?;
        let v = v.trim();
        let v = if let Some(q) = v.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            q.replace("\\n", "\n").replace("\\\"", "\"")
        } else if let Some(q) = v.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            q.to_owned()
        } else {
            strip_comment(v).trim().to_owned()
        };
        m.insert(k.trim().to_owned(), Value::String(v));
    }
    Ok(match o.mode {
        "env" => m
            .iter()
            .map(|(k, v)| {
                let v = v.as_str().unwrap_or("");
                if v.contains([' ', '#', '"', '\n']) {
                    format!("{k}={}", serde_json::to_string(v).unwrap())
                } else {
                    format!("{k}={v}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        "yaml" => {
            let mut out = String::new();
            to_yaml(&Value::Object(m), 0, &mut out);
            out.trim_end().to_owned()
        }
        _ => json_text(&Value::Object(m), 2),
    }
    .into())
}

// Sorting

/// Natural order: digits compare as numbers, so "file2" comes before "file10".
pub fn natural(a: &str, b: &str) -> Ordering {
    let chunks = |s: &str| -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let d = c.is_ascii_digit();
            match out.last_mut() {
                Some((dd, t)) if *dd == d => t.push(c),
                _ => out.push((d, c.to_string())),
            }
        }
        out
    };
    let (ca, cb) = (chunks(&a.to_lowercase()), chunks(&b.to_lowercase()));
    for (x, y) in ca.iter().zip(&cb) {
        let o = match (x.0, y.0) {
            (true, true) => {
                let (tx, ty) = (x.1.trim_start_matches('0'), y.1.trim_start_matches('0'));
                tx.len().cmp(&ty.len()).then(tx.cmp(ty))
            }
            _ => x.1.cmp(&y.1),
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    ca.len().cmp(&cb.len()).then(a.cmp(b))
}

fn sorted_lines(
    text: &str,
    o: &Opts,
    key: impl Fn(&str) -> Option<Vec<f64>>,
    what: &str,
) -> Result<Output, String> {
    let mut keyed: Vec<(Vec<f64>, &str)> = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        keyed.push((key(l).ok_or_else(|| format!("“{l}” is not {what}."))?, l));
    }
    keyed.sort_by(|a, b| {
        a.0.iter()
            .zip(&b.0)
            .map(|(x, y)| x.total_cmp(y))
            .find(|o| *o != Ordering::Equal)
            .unwrap_or(a.0.len().cmp(&b.0.len()))
    });
    if o.mode == "desc" {
        keyed.reverse();
    }
    if o.flag {
        keyed.dedup_by(|a, b| a.1 == b.1);
    }
    Ok(keyed
        .iter()
        .map(|k| k.1)
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn natural_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut v: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    v.sort_by(|a, b| natural(a, b));
    if o.mode == "desc" {
        v.reverse();
    }
    if o.flag {
        v.dedup();
    }
    Ok(v.join("\n").into())
}

pub fn ip_key(s: &str) -> Option<Vec<f64>> {
    let s = s.split('/').next()?;
    if s.contains(':') {
        let ip: std::net::Ipv6Addr = s.parse().ok()?;
        return Some(
            std::iter::once(1.0)
                .chain(ip.segments().iter().map(|x| *x as f64))
                .collect(),
        );
    }
    let ip: std::net::Ipv4Addr = s.parse().ok()?;
    Some(
        std::iter::once(0.0)
            .chain(ip.octets().iter().map(|x| *x as f64))
            .collect(),
    )
}

fn ip_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    sorted_lines(text, o, ip_key, "an IP address")
}

fn version_key(s: &str) -> Option<Vec<f64>> {
    let t = s.trim_start_matches(['v', 'V']);
    let (core, pre) = t.split_once('-').map_or((t, None), |(c, p)| (c, Some(p)));
    let core = core.split('+').next()?;
    let mut k: Vec<f64> = core
        .split('.')
        .map(|p| p.parse::<f64>().ok())
        .collect::<Option<_>>()?;
    k.resize(4, 0.0);
    // A pre-release sorts before its release.
    k.push(if pre.is_some() { 0.0 } else { 1.0 });
    if let Some(p) = pre {
        for part in p.split('.') {
            k.push(
                part.parse()
                    .unwrap_or_else(|_| part.bytes().fold(0.0, |a, b| a * 256.0 + b as f64)),
            );
        }
    }
    Some(k)
}

fn version_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    sorted_lines(text, o, version_key, "a version number")
}

/// A size such as `1.5 MB`, `200KiB` or `3 GB` in bytes.
pub fn size_bytes(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', "");
    let split = t.find(|c: char| c.is_alphabetic()).unwrap_or(t.len());
    let n: f64 = t[..split].trim().parse().ok()?;
    let unit = t[split..].trim().to_lowercase();
    let k = match unit.as_str() {
        "" | "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" => 1e3,
        "kib" => 1024.0,
        "m" | "mb" => 1e6,
        "mib" => 1024f64.powi(2),
        "g" | "gb" => 1e9,
        "gib" => 1024f64.powi(3),
        "t" | "tb" => 1e12,
        "tib" => 1024f64.powi(4),
        "p" | "pb" => 1e15,
        "pib" => 1024f64.powi(5),
        _ => return None,
    };
    Some(n * k)
}

fn size_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    sorted_lines(text, o, |s| size_bytes(s).map(|b| vec![b]), "a file size")
}

fn date_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    sorted_lines(
        text,
        o,
        |s| super::dates::parse_datetime(s).map(|t| vec![t]),
        "a date",
    )
}

fn domain_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut v: Vec<(String, &str)> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let host = l
                .split("://")
                .last()
                .unwrap_or(l)
                .split(['/', '?', '#', ':'])
                .next()
                .unwrap_or(l)
                .to_lowercase();
            let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
            // By the registrable name first: example.com before www.example.org.
            let key = host.rsplit('.').collect::<Vec<_>>().join(".");
            (if o.check(1) { host } else { key }, l)
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)));
    if o.mode == "desc" {
        v.reverse();
    }
    if o.flag {
        v.dedup_by(|a, b| a.1 == b.1);
    }
    Ok(v.iter().map(|x| x.1).collect::<Vec<_>>().join("\n").into())
}

fn csv_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    let delim = sniff(text, "auto");
    let rows = parse_csv(text, delim)?;
    let header = &rows[0];
    let col = o.field(0);
    let i = header
        .iter()
        .position(|h| h.trim().eq_ignore_ascii_case(col))
        .or_else(|| {
            col.parse::<usize>()
                .ok()
                .filter(|n| *n >= 1 && *n <= header.len())
                .map(|n| n - 1)
        })
        .ok_or_else(|| {
            format!(
                "There is no column “{col}”; the columns are {}.",
                header.join(", ")
            )
        })?;
    let mut body: Vec<&Vec<String>> = rows[1..].iter().collect();
    let get = |r: &Vec<String>| r.get(i).cloned().unwrap_or_default();
    body.sort_by(|a, b| {
        let (x, y) = (get(a), get(b));
        match (super::parse_number(&x), super::parse_number(&y)) {
            (Some(p), Some(q)) => p.total_cmp(&q),
            _ => natural(&x, &y),
        }
    });
    if o.mode == "desc" {
        body.reverse();
    }
    let d = delim.to_string();
    let line = |r: &Vec<String>| {
        r.iter()
            .map(|c| {
                if c.contains([delim, '"', '\n']) {
                    format!("\"{}\"", c.replace('"', "\"\""))
                } else {
                    c.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(&d)
    };
    Ok(std::iter::once(header)
        .chain(body)
        .map(line)
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn sort_keys(v: &mut Value, desc: bool) {
    match v {
        Value::Object(m) => {
            let mut entries: Vec<(String, Value)> = std::mem::take(m).into_iter().collect();
            entries.sort_by(|a, b| {
                if desc {
                    natural(&b.0, &a.0)
                } else {
                    natural(&a.0, &b.0)
                }
            });
            for (k, mut v) in entries {
                sort_keys(&mut v, desc);
                m.insert(k, v);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|v| sort_keys(v, desc)),
        _ => {}
    }
}

fn json_sort(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut v: Value = serde_json::from_str(text).map_err(|e| format!("Not JSON: {e}."))?;
    let desc = o.mode == "desc";
    let by = o.field(0);
    if by.is_empty() {
        sort_keys(&mut v, desc);
    } else {
        let Value::Array(a) = &mut v else {
            return Err("To sort by a key, the JSON must be an array of objects.".into());
        };
        let key = |x: &Value| {
            x.pointer(&format!("/{}", by.replace('.', "/")))
                .cloned()
                .unwrap_or(Value::Null)
        };
        a.sort_by(|x, y| match (key(x), key(y)) {
            (Value::Number(p), Value::Number(q)) => p
                .as_f64()
                .unwrap_or(0.0)
                .total_cmp(&q.as_f64().unwrap_or(0.0)),
            (Value::String(p), Value::String(q)) => natural(&p, &q),
            (p, q) => p.to_string().cmp(&q.to_string()),
        });
        if desc {
            a.reverse();
        }
    }
    Ok(json_text(&v, 2).into())
}

// The menus

const DELIMITERS: super::Menu = &[
    ("auto", "Detect"),
    ("comma", "Commas"),
    ("semicolon", "Semicolons"),
    ("tab", "Tabs"),
    ("pipe", "Pipes"),
];
const ORDER: super::Menu = &[("asc", "Ascending"), ("desc", "Descending")];
const DEDUP: &[&str] = &["Remove duplicates"];

pub const CONVERT: &[Tool] = &[
    tool("js-json", "JavaScript object to JSON", "Turns a JavaScript object literal (unquoted keys, single quotes, comments, trailing commas) into JSON.", super::code::js_object)
        .text("JavaScript object")
        .slider("Indent", 0.0, 8.0, 2.0),
    tool("json-csv", "JSON to CSV", "An array of objects as CSV, with nested keys flattened.", super::code::json_to_csv)
        .text("JSON")
        .selects(&[("Delimiter", &[("comma", "Commas"), ("semicolon", "Semicolons"), ("tab", "Tabs")])]),
    tool("csv-json", "CSV to JSON", "CSV rows as JSON objects keyed by the header.", csv_to_json)
        .text("CSV")
        .selects(&[("Delimiter", DELIMITERS)])
        .checks(&["Numbers and booleans as such", "Arrays instead of objects"])
        .slider("Indent", 0.0, 8.0, 2.0),
    tool("csv-markdown", "CSV to Markdown table", "A CSV as a Markdown table.", csv_to_markdown).text("CSV").selects(&[("Delimiter", DELIMITERS)]),
    tool("table", "Table formatter", "A CSV as an aligned text table.", table_format)
        .text("CSV")
        .selects(&[("Delimiter", DELIMITERS)])
        .checks(&["No line under the header"]),
    tool("json-xml", "JSON to XML", "JSON as XML; “@name” keys become attributes and “#text” text.", markup::json_to_xml)
        .text("JSON")
        .slider("Indent", 0.0, 8.0, 2.0),
    tool("xml-json", "XML to JSON", "XML as JSON; attributes become “@name” keys and repeated elements arrays.", markup::xml_to_json)
        .text("XML")
        .slider("Indent", 0.0, 8.0, 2.0),
    tool("json-yaml", "JSON to YAML", "JSON as YAML.", json_to_yaml).text("JSON"),
    tool("yaml-json", "YAML to JSON", "YAML (mappings, lists, scalars, flow collections and block text) as JSON.", yaml_to_json)
        .text("YAML")
        .slider("Indent", 0.0, 8.0, 2.0),
    tool("json-repair", "JSON repair", "Fixes almost-JSON: single quotes, unquoted keys, comments and trailing commas.", json_repair)
        .text("Broken JSON")
        .slider("Indent", 0.0, 8.0, 2.0),
    tool("env", "Env file parser", "A .env file as JSON or YAML, or tidied.", env_to_json)
        .text(".env")
        .selects(&[("Output", &[("json", "JSON"), ("yaml", "YAML"), ("env", "Tidy .env")])]),
    tool("html-markdown", "HTML to Markdown", "HTML as Markdown.", markup::html_to_markdown).text("HTML"),
    tool("markdown-html", "Markdown to HTML", "Markdown as HTML.", markup::markdown_to_html).text("Markdown"),
    tool("preserve-spaces", "Preserve spaces in HTML", "Spaces and line breaks as &nbsp; and <br>, so HTML keeps them.", markup::preserve_spaces),
    tool("word-html", "Document text to HTML", "Text pasted from a word processor as HTML paragraphs, headings and lists.", markup::word_to_html),
    tool("csv-validate", "CSV validator", "Checks that every row has as many cells as the header.", csv_validate)
        .text("CSV")
        .selects(&[("Delimiter", DELIMITERS)])
        .checks(&["Report empty cells"]),
    tool("duplicates", "Duplicate finder", "The values (lines) that appear more than once, and where.", duplicate_finder).checks(&["Ignore case"]),
];

pub const SORT: &[Tool] = &[
    tool(
        "natural-sort",
        "List sorter",
        "Sorts a list in natural order, so “item 2” comes before “item 10”.",
        natural_sort,
    )
    .text("One item per line")
    .selects(&[("Order", ORDER)])
    .checks(DEDUP),
    tool(
        "sort-numbers",
        "Number sorter",
        "Sorts the numbers in a text.",
        text::sort_numbers,
    )
    .selects(&[("Order", ORDER)])
    .checks(DEDUP),
    tool(
        "ip-sort",
        "IP address sorter",
        "Sorts IPv4 and IPv6 addresses numerically.",
        ip_sort,
    )
    .text("One address per line")
    .selects(&[("Order", ORDER)])
    .checks(DEDUP),
    tool(
        "version-sort",
        "Version number sorter",
        "Sorts version numbers: 1.2 < 1.10 < 2.0-beta < 2.0.",
        version_sort,
    )
    .text("One version per line")
    .selects(&[("Order", ORDER)])
    .checks(DEDUP),
    tool(
        "size-sort",
        "File size sorter",
        "Sorts sizes such as 1.5 MB, 200 KiB and 3 GB.",
        size_sort,
    )
    .text("One size per line")
    .selects(&[("Order", ORDER)])
    .checks(DEDUP),
    tool(
        "date-sort",
        "Date and time sorter",
        "Sorts dates in ISO, US or European formats.",
        date_sort,
    )
    .text("One date per line")
    .selects(&[("Order", ORDER)])
    .checks(DEDUP),
    tool(
        "domain-sort",
        "Domain and URL sorter",
        "Sorts URLs by domain, grouping subdomains.",
        domain_sort,
    )
    .text("One URL per line")
    .selects(&[("Order", ORDER)])
    .checks(&["Remove duplicates", "Sort by the full host name"]),
    tool(
        "csv-sort",
        "CSV column sorter",
        "Sorts CSV rows by a column, by name or number.",
        csv_sort,
    )
    .text("CSV")
    .fields(&[("Column", "1")])
    .selects(&[("Order", ORDER)]),
    tool(
        "json-sort",
        "JSON sorter",
        "Sorts JSON object keys, or an array of objects by a key.",
        json_sort,
    )
    .text("JSON")
    .fields(&[("Sort array by key (empty: sort keys)", "")])
    .selects(&[("Order", ORDER)]),
];

#[cfg(test)]
mod tests {
    use super::super::Rng;
    use super::*;

    fn run(f: super::super::Run, text: &str, mode: &str) -> String {
        let mut rng = Rng(1);
        let mut o = Opts::new(mode, &mut rng);
        o.number = 2;
        f(text, &mut o).unwrap().plain()
    }

    #[test]
    fn csv() {
        assert_eq!(
            parse_csv("a,\"b,\"\"c\"\"\"\n1,2\n", ',').unwrap(),
            vec![vec!["a", "b,\"c\""], vec!["1", "2"]]
        );
        assert_eq!(
            run(csv_to_json, "n;x\na;1", "auto"),
            "[\n  {\n    \"n\": \"a\",\n    \"x\": \"1\"\n  }\n]"
        );
        assert_eq!(
            run(csv_to_markdown, "a,b\n1,22", "auto"),
            "| a   | b   |\n| --- | --- |\n| 1   | 22  |"
        );
    }

    #[test]
    fn yaml_round_trip() {
        let json = r#"{"name":"x","list":[1,"two",{"k":true,"m":null}],"nested":{"a":"b: c","t":"line1\nline2"}}"#;
        let yaml = run(json_to_yaml, json, "");
        assert_eq!(
            yaml,
            "name: x\nlist:\n  - 1\n  - two\n  - k: true\n    m: null\nnested:\n  a: \"b: c\"\n  t: |\n    line1\n    line2"
        );
        let back: Value = serde_json::from_str(&run(yaml_to_json, &yaml, "")).unwrap();
        let mut want: Value = serde_json::from_str(json).unwrap();
        want["nested"]["t"] = json!("line1\nline2\n");
        assert_eq!(back, want);
        assert_eq!(
            run(
                yaml_to_json,
                "# c\na: [1, b, {c: d}] # x\nb:\n- 'it''s'\n- \"q\\n\"\n",
                ""
            ),
            "{\n  \"a\": [\n    1,\n    \"b\",\n    {\n      \"c\": \"d\"\n    }\n  ],\n  \"b\": [\n    \"it's\",\n    \"q\\n\"\n  ]\n}"
        );
    }

    use hypereffect::json;

    #[test]
    fn sorting() {
        assert_eq!(run(natural_sort, "a10\na2\nA1", "asc"), "A1\na2\na10");
        assert_eq!(
            run(ip_sort, "10.0.0.10\n10.0.0.9\n::1", "asc"),
            "10.0.0.9\n10.0.0.10\n::1"
        );
        assert_eq!(
            run(version_sort, "2.0\n1.10\n2.0-beta\n1.2", "asc"),
            "1.2\n1.10\n2.0-beta\n2.0"
        );
        assert_eq!(
            run(size_sort, "1 GB\n200 KiB\n1.5MB", "asc"),
            "200 KiB\n1.5MB\n1 GB"
        );
        assert_eq!(
            run(domain_sort, "https://b.org/x\nwww.a.com\nmail.a.com", "asc"),
            "www.a.com\nmail.a.com\nhttps://b.org/x"
        );
    }
}
