//! Tools on markup: HTML and XML formatting and escaping, XML and JSON,
//! HTML and Markdown.

use super::Opts;
use super::Output;
use super::code::json_text;
use hypereffect::{Map, Value};

/// A piece of markup.
#[derive(Debug, PartialEq)]
enum Token<'a> {
    /// `<name a="b">`, and whether it closes itself (`<br/>`).
    Open(&'a str, Vec<(&'a str, String)>, bool),
    Close(&'a str),
    Text(&'a str),
    /// A comment, doctype, processing instruction or CDATA section, verbatim.
    Other(&'a str),
}

fn tokenize(src: &str) -> Result<Vec<Token<'_>>, String> {
    let mut out = Vec::new();
    let mut rest = src;
    while !rest.is_empty() {
        let Some(start) = rest.find('<') else {
            out.push(Token::Text(rest));
            break;
        };
        if start > 0 {
            out.push(Token::Text(&rest[..start]));
        }
        rest = &rest[start..];
        let end_of = |close: &str| {
            rest.find(close)
                .map(|i| i + close.len())
                .ok_or_else(|| format!("Missing “{close}”."))
        };
        let end = if rest.starts_with("<!--") {
            end_of("-->")?
        } else if rest.starts_with("<![CDATA[") {
            end_of("]]>")?
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            end_of(">")?
        } else {
            // A tag: find the `>` outside of quoted attribute values.
            let mut quote = None;
            let mut end = None;
            for (i, c) in rest.char_indices().skip(1) {
                match (quote, c) {
                    (Some(q), c) if c == q => quote = None,
                    (Some(_), _) => {}
                    (None, '"' | '\'') => quote = Some(c),
                    (None, '>') => {
                        end = Some(i + 1);
                        break;
                    }
                    _ => {}
                }
            }
            end.ok_or("A tag is not closed with “>”.")?
        };
        let raw = &rest[..end];
        rest = &rest[end..];
        if raw.starts_with("<!") || raw.starts_with("<?") {
            out.push(Token::Other(raw));
        } else if let Some(name) = raw.strip_prefix("</") {
            out.push(Token::Close(name.trim_end_matches('>').trim()));
        } else {
            let inner = raw[1..raw.len() - 1].trim();
            let closed = inner.ends_with('/');
            let inner = inner.trim_end_matches('/').trim();
            let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
            let name = &inner[..name_end];
            if name.is_empty() {
                // A lone `<`, as in `a < b`: text.
                out.push(Token::Text(raw));
                continue;
            }
            let tag = Token::Open(name, attributes(&inner[name_end..]), closed);
            let raw_text = matches!(
                name.to_ascii_lowercase().as_str(),
                "script" | "style" | "pre" | "textarea"
            );
            out.push(tag);
            if raw_text && !closed {
                // Their content is not markup: keep it to the closing tag.
                let close = format!("</{name}");
                let i = rest
                    .to_ascii_lowercase()
                    .find(&close.to_ascii_lowercase())
                    .unwrap_or(rest.len());
                if i > 0 {
                    out.push(Token::Other(&rest[..i]));
                }
                rest = &rest[i..];
            }
        }
    }
    Ok(out)
}

fn attributes(mut s: &str) -> Vec<(&str, String)> {
    let mut out = Vec::new();
    loop {
        s = s.trim_start();
        if s.is_empty() {
            return out;
        }
        let end = s
            .find(|c: char| c == '=' || c.is_whitespace())
            .unwrap_or(s.len());
        let name = &s[..end];
        s = s[end..].trim_start();
        let Some(after) = s.strip_prefix('=') else {
            out.push((name, String::new()));
            continue;
        };
        s = after.trim_start();
        let (value, rest) = match s.chars().next() {
            Some(q @ ('"' | '\'')) => {
                let close = s[1..].find(q).map_or(s.len(), |i| i + 1);
                (&s[1..close], s.get(close + 1..).unwrap_or(""))
            }
            _ => {
                let e = s.find(char::is_whitespace).unwrap_or(s.len());
                (&s[..e], &s[e..])
            }
        };
        out.push((name, unescape(value)));
        s = rest;
    }
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];
const INLINE: &[&str] = &[
    "a", "abbr", "b", "bdi", "bdo", "cite", "code", "data", "dfn", "em", "i", "kbd", "mark", "q",
    "s", "samp", "small", "span", "strong", "sub", "sup", "time", "u", "var", "label",
];

fn is_void(name: &str) -> bool {
    VOID.contains(&name.to_ascii_lowercase().as_str())
}

fn open_tag(name: &str, attrs: &[(&str, String)], closed: bool, xml: bool) -> String {
    let mut s = format!("<{name}");
    for (k, v) in attrs {
        if v.is_empty() && !xml {
            s.push_str(&format!(" {k}"));
        } else {
            s.push_str(&format!(" {k}=\"{}\"", escape_attr(v)));
        }
    }
    s.push_str(if closed && xml { "/>" } else { ">" });
    s
}

/// Indents markup one element per line; in HTML, inline elements stay on
/// the line of their text.
fn format_markup(src: &str, indent: usize, xml: bool, minify: bool) -> Result<Output, String> {
    let tokens = tokenize(src)?;
    let pad = |depth: usize| " ".repeat(depth * indent);
    let mut out = String::new();
    let mut depth = 0usize;
    let mut line = String::new();
    let flush = |out: &mut String, line: &mut String, depth: usize| {
        let l = line.trim();
        if !l.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&pad(depth));
            out.push_str(l);
        }
        line.clear();
    };
    let inline = |name: &str| !xml && INLINE.contains(&name.to_ascii_lowercase().as_str());
    // Whether the element at `i` holds only text and inline elements, so it
    // can stay on one line.
    let inline_content = |i: usize| {
        let mut depth = 0usize;
        for t in &tokens[i + 1..] {
            match t {
                Token::Text(_) => {}
                Token::Open(name, _, closed) => {
                    if !inline(name) && !is_void(name) {
                        return false;
                    }
                    if !*closed && !is_void(name) {
                        depth += 1;
                    }
                }
                Token::Close(_) if depth == 0 => return true,
                Token::Close(_) => depth -= 1,
                Token::Other(_) => return false,
            }
        }
        false
    };
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Text(text) => {
                let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if minify {
                    out.push_str(&collapsed);
                    continue;
                }
                if !collapsed.is_empty() {
                    if text.starts_with(char::is_whitespace) && !line.is_empty() {
                        line.push(' ');
                    }
                    line.push_str(&collapsed);
                    if text.ends_with(char::is_whitespace) {
                        line.push(' ');
                    }
                }
            }
            Token::Other(raw) => {
                if minify {
                    if !raw.starts_with("<!--") {
                        out.push_str(raw);
                    }
                    continue;
                }
                flush(&mut out, &mut line, depth);
                let mut first = true;
                for l in raw.lines().filter(|l| !l.trim().is_empty()) {
                    if !first || !out.is_empty() {
                        out.push('\n');
                    }
                    first = false;
                    out.push_str(&pad(depth));
                    out.push_str(l.trim());
                }
            }
            Token::Open(name, attrs, closed) => {
                let closed = *closed || (!xml && is_void(name));
                let tag = open_tag(name, attrs, closed, xml);
                if minify {
                    out.push_str(&tag);
                } else if inline(name) {
                    line.push_str(&tag);
                } else {
                    flush(&mut out, &mut line, depth);
                    line.push_str(&tag);
                    // A tag with only text and inline children stays on one line.
                    let simple = !closed && inline_content(i);
                    if !simple {
                        flush(&mut out, &mut line, depth);
                    }
                    if !closed {
                        depth += 1;
                    }
                    if simple {
                        // Kept open until the close tag flushes the line.
                        line.insert(0, '\u{0}');
                    }
                }
            }
            Token::Close(name) => {
                let tag = format!("</{name}>");
                if minify {
                    out.push_str(&tag);
                } else if inline(name) {
                    line.push_str(&tag);
                } else {
                    depth = depth.saturating_sub(1);
                    if line.starts_with('\u{0}') {
                        line.remove(0);
                        line.push_str(&tag);
                        flush(&mut out, &mut line, depth);
                    } else {
                        flush(&mut out, &mut line, depth + 1);
                        line.push_str(&tag);
                        flush(&mut out, &mut line, depth);
                    }
                }
            }
        }
    }
    let rest = line.trim_start_matches('\u{0}').to_owned();
    line = rest;
    flush(&mut out, &mut line, depth);
    Ok(out.into())
}

pub fn html(text: &str, o: &mut Opts) -> Result<Output, String> {
    format_markup(text, o.number, false, o.mode == "minify")
}

pub fn xml(text: &str, o: &mut Opts) -> Result<Output, String> {
    // Only well-formed XML is formatted.
    parse_xml(text)?;
    format_markup(text, o.number, true, o.mode == "minify")
}

// Escaping

pub fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Decodes character references: the XML five, a few common HTML ones and
/// numeric ones. Unknown ones stay as they are.
pub fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e <= 12) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        let c = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            "copy" => Some('©'),
            "reg" => Some('®'),
            "trade" => Some('™'),
            "hellip" => Some('…'),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "lsquo" => Some('‘'),
            "rsquo" => Some('’'),
            "ldquo" => Some('“'),
            "rdquo" => Some('”'),
            "euro" => Some('€'),
            _ => name
                .strip_prefix("#x")
                .or_else(|| name.strip_prefix("#X"))
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn html_escape(text: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(match o.mode {
        "unescape" => unescape(text),
        _ => escape_html(text),
    }
    .into())
}

pub fn xml_escape(text: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(match o.mode {
        "unescape" => unescape(text),
        _ => text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;"),
    }
    .into())
}

// XML and JSON

/// An XML element.
#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    text: String,
}

fn parse_xml(src: &str) -> Result<Node, String> {
    let mut stack = vec![Node::default()];
    for t in tokenize(src)? {
        match t {
            Token::Open(name, attrs, closed) => {
                let node = Node {
                    name: name.to_owned(),
                    attrs: attrs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
                    ..Node::default()
                };
                if closed {
                    stack.last_mut().unwrap().children.push(node);
                } else {
                    stack.push(node);
                }
            }
            Token::Close(name) => {
                let node = stack.pop().filter(|n| n.name == name && !n.name.is_empty());
                let node = node.ok_or_else(|| format!("“</{name}>” closes nothing."))?;
                stack.last_mut().unwrap().children.push(node);
            }
            Token::Text(t) => stack.last_mut().unwrap().text.push_str(&unescape(t)),
            Token::Other(raw) => {
                if let Some(data) = raw.strip_prefix("<![CDATA[") {
                    stack
                        .last_mut()
                        .unwrap()
                        .text
                        .push_str(data.trim_end_matches("]]>"));
                }
            }
        }
    }
    let mut root = stack.pop().unwrap();
    if !stack.is_empty() {
        return Err(format!("“<{}>” is never closed.", root.name));
    }
    if root.children.len() != 1 || !root.text.trim().is_empty() {
        return Err("XML needs exactly one root element.".into());
    }
    Ok(root.children.remove(0))
}

/// An element as JSON: attributes as `@name`, text as `#text` (or the value
/// itself when there is nothing else), and repeated children as arrays.
fn node_json(n: &Node) -> Value {
    let text = n.text.trim();
    if n.attrs.is_empty() && n.children.is_empty() {
        return Value::String(text.to_owned());
    }
    let mut m = Map::new();
    for (k, v) in &n.attrs {
        m.insert(format!("@{k}"), Value::String(v.clone()));
    }
    for c in &n.children {
        let v = node_json(c);
        match m.get_mut(&c.name) {
            Some(Value::Array(a)) => a.push(v),
            Some(prev) => *prev = Value::Array(vec![prev.take(), v]),
            None => {
                m.insert(c.name.clone(), v);
            }
        }
    }
    if !text.is_empty() {
        m.insert("#text".into(), Value::String(text.to_owned()));
    }
    Value::Object(m)
}

pub fn xml_to_json(text: &str, o: &mut Opts) -> Result<Output, String> {
    let root = parse_xml(text)?;
    let mut m = Map::new();
    m.insert(root.name.clone(), node_json(&root));
    Ok(json_text(&Value::Object(m), o.number).into())
}

fn xml_name(key: &str) -> String {
    let name: String = key
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "_-.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.starts_with(|c: char| c.is_alphabetic() || c == '_') {
        name
    } else {
        format!("_{name}")
    }
}

fn value_xml(name: &str, v: &Value, depth: usize, indent: usize, out: &mut String) {
    let pad = " ".repeat(depth * indent);
    let nl = if indent > 0 { "\n" } else { "" };
    match v {
        Value::Array(items) => {
            for item in items {
                value_xml(name, item, depth, indent, out);
            }
        }
        Value::Object(m) => {
            let attrs: String = m
                .iter()
                .filter_map(|(k, v)| {
                    let a = k.strip_prefix('@')?;
                    let v = match v {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    };
                    Some(format!(" {}=\"{}\"", xml_name(a), escape_attr(&v)))
                })
                .collect();
            let text = m.get("#text").map(|t| match t {
                Value::String(s) => s.clone(),
                v => v.to_string(),
            });
            let children: Vec<_> = m
                .iter()
                .filter(|(k, _)| !k.starts_with(['@', '#']))
                .collect();
            if children.is_empty() {
                let text = text.unwrap_or_default();
                if text.is_empty() {
                    out.push_str(&format!("{pad}<{name}{attrs}/>{nl}"));
                } else {
                    out.push_str(&format!(
                        "{pad}<{name}{attrs}>{}</{name}>{nl}",
                        xml_text(&text)
                    ));
                }
                return;
            }
            out.push_str(&format!("{pad}<{name}{attrs}>{nl}"));
            if let Some(t) = text {
                out.push_str(&format!(
                    "{}{}{nl}",
                    " ".repeat((depth + 1) * indent),
                    xml_text(&t)
                ));
            }
            for (k, v) in children {
                value_xml(&xml_name(k), v, depth + 1, indent, out);
            }
            out.push_str(&format!("{pad}</{name}>{nl}"));
        }
        Value::Null => out.push_str(&format!("{pad}<{name}/>{nl}")),
        Value::String(s) => out.push_str(&format!("{pad}<{name}>{}</{name}>{nl}", xml_text(s))),
        v => out.push_str(&format!("{pad}<{name}>{v}</{name}>{nl}")),
    }
}

fn xml_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn json_to_xml(text: &str, o: &mut Opts) -> Result<Output, String> {
    let v: Value = serde_json_parse(text)?;
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    out.push(if o.number > 0 { '\n' } else { ' ' });
    match &v {
        Value::Object(m) if m.len() == 1 && !m.keys().next().unwrap().starts_with(['@', '#']) => {
            let (k, v) = m.iter().next().unwrap();
            if v.is_array() {
                value_xml("root", &Value::Object(m.clone()), 0, o.number, &mut out);
            } else {
                value_xml(&xml_name(k), v, 0, o.number, &mut out);
            }
        }
        _ => value_xml("root", &v, 0, o.number, &mut out),
    }
    Ok(out.trim_end().to_owned().into())
}

fn serde_json_parse(text: &str) -> Result<Value, String> {
    hypereffect::serde_json::from_str(text).map_err(|e| format!("Not JSON: {e}."))
}

// HTML and Markdown

pub fn html_to_markdown(text: &str, _: &mut Opts) -> Result<Output, String> {
    let tokens = tokenize(text)?;
    let mut out = String::new();
    // The open lists, `None` for bullets or the next number.
    let mut lists: Vec<Option<usize>> = Vec::new();
    let mut links: Vec<String> = Vec::new();
    let mut pre = false;
    let mut skip: i32 = 0;
    let block = |out: &mut String| {
        let trimmed = out.trim_end_matches([' ', '\n']).len();
        out.truncate(trimmed);
        if !out.is_empty() {
            out.push_str("\n\n");
        }
    };
    let line = |out: &mut String| {
        let trimmed = out.trim_end_matches(' ').len();
        out.truncate(trimmed);
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
    };
    for t in &tokens {
        match t {
            Token::Open(name, attrs, _) => {
                let attr = |k: &str| {
                    attrs
                        .iter()
                        .find(|(n, _)| n.eq_ignore_ascii_case(k))
                        .map_or("", |(_, v)| v.as_str())
                };
                match name.to_ascii_lowercase().as_str() {
                    "script" | "style" | "head" | "title" => skip += 1,
                    h @ ("h1" | "h2" | "h3" | "h4" | "h5" | "h6") => {
                        block(&mut out);
                        out.push_str(&"#".repeat((h.as_bytes()[1] - b'0') as usize));
                        out.push(' ');
                    }
                    "p" | "div" | "section" | "article" | "table" => block(&mut out),
                    "tr" => line(&mut out),
                    "td" | "th" => out.push_str(" | "),
                    "br" => out.push_str("  \n"),
                    "hr" => {
                        block(&mut out);
                        out.push_str("---\n\n");
                    }
                    "strong" | "b" => out.push_str("**"),
                    "em" | "i" => out.push('*'),
                    "del" | "s" => out.push_str("~~"),
                    "code" if !pre => out.push('`'),
                    "pre" => {
                        block(&mut out);
                        out.push_str("```\n");
                        pre = true;
                    }
                    "blockquote" => {
                        block(&mut out);
                        out.push_str("> ");
                    }
                    "ul" => {
                        if lists.is_empty() {
                            block(&mut out);
                        }
                        lists.push(None);
                    }
                    "ol" => {
                        if lists.is_empty() {
                            block(&mut out);
                        }
                        lists.push(Some(attr("start").parse().unwrap_or(1)));
                    }
                    "li" => {
                        line(&mut out);
                        out.push_str(&"   ".repeat(lists.len().saturating_sub(1)));
                        match lists.last_mut() {
                            Some(Some(n)) => {
                                out.push_str(&format!("{n}. "));
                                *n += 1;
                            }
                            _ => out.push_str("- "),
                        }
                    }
                    "a" => {
                        out.push('[');
                        links.push(attr("href").to_owned());
                    }
                    "img" => out.push_str(&format!("![{}]({})", attr("alt"), attr("src"))),
                    _ => {}
                }
            }
            Token::Close(name) => match name.to_ascii_lowercase().as_str() {
                "script" | "style" | "head" | "title" => skip = skip.saturating_sub(1),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "div" | "section" | "article"
                | "blockquote" | "table" => block(&mut out),
                "tr" => out.push_str(" |"),
                "strong" | "b" => out.push_str("**"),
                "em" | "i" => out.push('*'),
                "del" | "s" => out.push_str("~~"),
                "code" if !pre => out.push('`'),
                "pre" => {
                    line(&mut out);
                    out.push_str("```");
                    block(&mut out);
                    pre = false;
                }
                "ul" | "ol" => {
                    lists.pop();
                    if lists.is_empty() {
                        block(&mut out);
                    }
                }
                "a" => {
                    let href = links.pop().unwrap_or_default();
                    out.push_str(&format!("]({href})"));
                }
                _ => {}
            },
            Token::Text(t) if skip == 0 => {
                let t = unescape(t);
                if pre {
                    out.push_str(&t);
                } else {
                    let collapsed = t.split_whitespace().collect::<Vec<_>>().join(" ");
                    if t.starts_with(char::is_whitespace)
                        && !out.ends_with([' ', '\n', '['])
                        && !out.is_empty()
                    {
                        out.push(' ');
                    }
                    out.push_str(&collapsed);
                    if t.ends_with(char::is_whitespace) && !collapsed.is_empty() {
                        out.push(' ');
                    }
                }
            }
            Token::Other(raw) if pre => out.push_str(&unescape(raw)),
            _ => {}
        }
    }
    Ok(out
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
        .into())
}

/// Inline Markdown: code, images, links, bold, italics and strikethrough.
fn inline(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let find = |from: usize, pat: &str| {
        let p: Vec<char> = pat.chars().collect();
        (from..chars.len()).find(|&j| chars[j..].starts_with(&p))
    };
    let text = |a: usize, b: usize| chars[a..b].iter().collect::<String>();
    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            out.push_str(&escape_html(&chars[i + 1].to_string()));
            i += 2;
        } else if c == '`' {
            if let Some(end) = find(i + 1, "`") {
                out.push_str(&format!("<code>{}</code>", escape_html(&text(i + 1, end))));
                i = end + 1;
            } else {
                out.push('`');
                i += 1;
            }
        } else if (c == '!' && chars.get(i + 1) == Some(&'[')) || c == '[' {
            let image = c == '!';
            let open = if image { i + 1 } else { i };
            let link = find(open + 1, "](").and_then(|m| Some((m, find(m + 2, ")")?)));
            match link {
                Some((mid, end)) => {
                    let label = text(open + 1, mid);
                    let href = escape_attr(&text(mid + 2, end));
                    if image {
                        out.push_str(&format!(
                            "<img src=\"{href}\" alt=\"{}\">",
                            escape_attr(&label)
                        ));
                    } else {
                        out.push_str(&format!("<a href=\"{href}\">{}</a>", inline(&label)));
                    }
                    i = end + 1;
                }
                None => {
                    out.push(c);
                    i += 1;
                }
            }
        } else if (c == '*' || c == '_') && chars.get(i + 1) == Some(&c) {
            bold = !bold;
            out.push_str(if bold { "<strong>" } else { "</strong>" });
            i += 2;
        } else if c == '~' && chars.get(i + 1) == Some(&'~') {
            strike = !strike;
            out.push_str(if strike { "<del>" } else { "</del>" });
            i += 2;
        } else if c == '*' || (c == '_' && (i == 0 || !chars[i - 1].is_alphanumeric() || italic)) {
            italic = !italic;
            out.push_str(if italic { "<em>" } else { "</em>" });
            i += 1;
        } else {
            out.push_str(&escape_html(&c.to_string()));
            i += 1;
        }
    }
    for (open, tag) in [(strike, "</del>"), (italic, "</em>"), (bold, "</strong>")] {
        if open {
            out.push_str(tag);
        }
    }
    out
}

fn list_item(line: &str) -> Option<(bool, usize, &str)> {
    let indent = line.len() - line.trim_start().len();
    let t = line.trim_start();
    if let Some(rest) = t
        .strip_prefix(['-', '*', '+'])
        .and_then(|r| r.strip_prefix(' '))
    {
        return Some((false, indent, rest));
    }
    let digits = t.find(|c: char| !c.is_ascii_digit())?;
    let rest = t[digits..].strip_prefix(['.', ')'])?.strip_prefix(' ')?;
    (digits > 0).then_some((true, indent, rest))
}

pub fn markdown_to_html(text: &str, _: &mut Opts) -> Result<Output, String> {
    markdown_html(text).map(Output::from)
}

fn markdown_html(text: &str) -> Result<String, String> {
    let mut out: Vec<String> = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let mut lists: Vec<(bool, usize)> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let flush = |out: &mut Vec<String>, para: &mut Vec<String>| {
        if !para.is_empty() {
            out.push(format!("<p>{}</p>", para.join("\n")));
            para.clear();
        }
    };
    let close_lists = |out: &mut Vec<String>, lists: &mut Vec<(bool, usize)>, to: usize| {
        while lists.len() > to {
            let (ordered, _) = lists.pop().unwrap();
            let tag = format!("</li></{}>", if ordered { "ol" } else { "ul" });
            match out.last_mut() {
                Some(last) => last.push_str(&tag),
                None => out.push(tag),
            }
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim();
        if t.starts_with("```") {
            flush(&mut out, &mut para);
            close_lists(&mut out, &mut lists, 0);
            let lang = t.trim_start_matches('`').trim();
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code.push(escape_html(lines[i]));
                i += 1;
            }
            let class = if lang.is_empty() {
                String::new()
            } else {
                format!(" class=\"language-{}\"", escape_attr(lang))
            };
            out.push(format!(
                "<pre><code{class}>{}</code></pre>",
                code.join("\n")
            ));
        } else if t.is_empty() {
            flush(&mut out, &mut para);
            close_lists(&mut out, &mut lists, 0);
        } else if let Some(level) = t
            .find(|c| c != '#')
            .filter(|&n| (1..=6).contains(&n) && t[n..].starts_with(' '))
        {
            flush(&mut out, &mut para);
            close_lists(&mut out, &mut lists, 0);
            out.push(format!(
                "<h{level}>{}</h{level}>",
                inline(t[level..].trim().trim_end_matches('#').trim())
            ));
        } else if t.len() >= 3
            && (t.chars().all(|c| c == '-' || c == ' ')
                || t.chars().all(|c| c == '*' || c == ' ')
                || t.chars().all(|c| c == '_'))
            && t.chars().filter(|c| !c.is_whitespace()).count() >= 3
            && list_item(line).is_none()
        {
            flush(&mut out, &mut para);
            close_lists(&mut out, &mut lists, 0);
            out.push("<hr>".into());
        } else if let Some(quote) = t.strip_prefix('>') {
            flush(&mut out, &mut para);
            close_lists(&mut out, &mut lists, 0);
            let mut quoted = vec![quote.trim_start().to_owned()];
            while i + 1 < lines.len() && lines[i + 1].trim().starts_with('>') {
                i += 1;
                quoted.push(lines[i].trim()[1..].trim_start().to_owned());
            }
            let inner = markdown_html(&quoted.join("\n"))?;
            out.push(format!("<blockquote>\n{inner}\n</blockquote>"));
        } else if let Some((ordered, indent, rest)) = list_item(line) {
            flush(&mut out, &mut para);
            match lists.last() {
                Some(&(_, last)) if indent > last => {
                    lists.push((ordered, indent));
                    out.push(format!("<{}>", if ordered { "ol" } else { "ul" }));
                }
                Some(_) => {
                    let keep = lists
                        .iter()
                        .position(|&(_, n)| n >= indent)
                        .map_or(lists.len(), |p| p + 1);
                    close_lists(&mut out, &mut lists, keep);
                    out.push("</li>".into());
                }
                None => {
                    lists.push((ordered, indent));
                    out.push(format!("<{}>", if ordered { "ol" } else { "ul" }));
                }
            }
            out.push(format!("<li>{}", inline(rest)));
        } else if !lists.is_empty() {
            // A continuation of the list item.
            if let Some(last) = out.last_mut() {
                last.push(' ');
                last.push_str(&inline(t));
            }
        } else {
            let hard = line.ends_with("  ");
            para.push(format!("{}{}", inline(t), if hard { "<br>" } else { "" }));
        }
        i += 1;
    }
    flush(&mut out, &mut para);
    close_lists(&mut out, &mut lists, 0);
    // A list's items close where the next one opens.
    let joined = out.join("\n");
    Ok(joined.replace("\n</li>\n<li>", "</li>\n<li>"))
}

pub fn preserve_spaces(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(text
        .lines()
        .map(|l| {
            let mut out = String::new();
            let mut prev_space = true;
            for c in escape_html(l).chars() {
                match c {
                    ' ' if prev_space => out.push_str("&nbsp;"),
                    '\t' => {
                        out.push_str("&nbsp;&nbsp;&nbsp;&nbsp;");
                        prev_space = true;
                        continue;
                    }
                    c => out.push(c),
                }
                prev_space = c == ' ';
            }
            // A trailing space would be dropped.
            if out.ends_with(' ') {
                out.pop();
                out.push_str("&nbsp;");
            }
            out
        })
        .collect::<Vec<_>>()
        .join("<br>\n")
        .into())
}

/// Document text, as pasted from a word processor, as HTML: paragraphs,
/// headings (short lines without a final stop, followed by a blank line),
/// and bulleted or numbered lists.
pub fn word_to_html(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut out = Vec::new();
    fn bullet(l: &str) -> Option<&str> {
        l.strip_prefix(['•', '◦', '▪', '·', '-', '*', '–', '●', '○', '■'])
            .map(str::trim_start)
    }
    fn number(l: &str) -> Option<&str> {
        let d = l.find(|c: char| !c.is_ascii_digit())?;
        (d > 0)
            .then(|| l[d..].strip_prefix(['.', ')']))?
            .map(str::trim_start)
    }
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    for p in paragraphs {
        let lines: Vec<&str> = p.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if lines.iter().all(|l| bullet(l).is_some()) {
            out.push("<ul>".to_owned());
            out.extend(
                lines
                    .iter()
                    .map(|l| format!("  <li>{}</li>", escape_html(bullet(l).unwrap()))),
            );
            out.push("</ul>".to_owned());
        } else if lines.iter().all(|l| number(l).is_some()) {
            out.push("<ol>".to_owned());
            out.extend(
                lines
                    .iter()
                    .map(|l| format!("  <li>{}</li>", escape_html(number(l).unwrap()))),
            );
            out.push("</ol>".to_owned());
        } else if lines.len() == 1
            && lines[0].chars().count() < 70
            && !lines[0].ends_with(['.', ',', ';', ':', '!', '?'])
        {
            out.push(format!("<h2>{}</h2>", escape_html(lines[0])));
        } else {
            let body: Vec<String> = lines.iter().map(|l| escape_html(l)).collect();
            out.push(format!("<p>{}</p>", body.join("<br>\n")));
        }
    }
    Ok(out.join("\n").into())
}

#[cfg(test)]
mod tests {
    use super::super::{Opts, Rng};
    use super::*;

    fn run(f: fn(&str, &mut Opts) -> Result<Output, String>, text: &str, mode: &str) -> String {
        let mut rng = Rng(1);
        let mut o = Opts::new(mode, &mut rng);
        o.number = 2;
        f(text, &mut o).unwrap().plain()
    }

    #[test]
    fn formats_xml() {
        assert_eq!(
            run(xml, "<a x='1'><b>hi</b><c/></a>", "beautify"),
            "<a x=\"1\">\n  <b>hi</b>\n  <c/>\n</a>"
        );
        assert_eq!(
            run(xml, "<a>\n  <b> hi </b>\n</a>", "minify"),
            "<a><b>hi</b></a>"
        );
    }

    #[test]
    fn formats_html() {
        assert_eq!(
            run(
                html,
                "<div><p>Hello <b>you</b></p><br><ul><li>x</li></ul></div>",
                "beautify"
            ),
            "<div>\n  <p>Hello <b>you</b></p>\n  <br>\n  <ul>\n    <li>x</li>\n  </ul>\n</div>"
        );
    }

    #[test]
    fn xml_and_json() {
        let json = run(xml_to_json, "<r><i id=\"1\">a</i><i>b</i><n>3</n></r>", "");
        assert_eq!(
            json,
            "{\n  \"r\": {\n    \"i\": [\n      {\n        \"@id\": \"1\",\n        \"#text\": \"a\"\n      },\n      \"b\"\n    ],\n    \"n\": \"3\"\n  }\n}"
        );
        assert_eq!(
            run(json_to_xml, r#"{"r": {"i": ["a", "b"], "n": 3}}"#, ""),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<r>\n  <i>a</i>\n  <i>b</i>\n  <n>3</n>\n</r>"
        );
        let mut rng = Rng(1);
        assert!(xml_to_json("<a><b></a>", &mut Opts::new("", &mut rng)).is_err());
    }

    #[test]
    fn markdown() {
        assert_eq!(
            run(
                markdown_to_html,
                "# Hi\n\nSome *em* and **bold**, `x<y`.\n\n- one\n- [two](http://t)\n",
                ""
            ),
            "<h1>Hi</h1>\n<p>Some <em>em</em> and <strong>bold</strong>, <code>x&lt;y</code>.</p>\n<ul>\n<li>one</li>\n<li><a href=\"http://t\">two</a></li></ul>"
        );
        assert_eq!(
            run(
                html_to_markdown,
                "<h2>Hi</h2><p>Some <em>em</em> and <a href=\"/x\">a link</a>.</p><ol><li>one</li><li>two</li></ol>",
                ""
            ),
            "## Hi\n\nSome *em* and [a link](/x).\n\n1. one\n2. two"
        );
    }

    #[test]
    fn escapes() {
        assert_eq!(
            run(html_escape, "<a href=\"x\">&</a>", "escape"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
        assert_eq!(
            run(html_escape, "&lt;&#65;&#x42;&copy;&bogus;", "unescape"),
            "<AB©&bogus;"
        );
        assert_eq!(
            run(preserve_spaces, "a  b\n c", ""),
            "a &nbsp;b<br>\n&nbsp;c"
        );
    }
}
