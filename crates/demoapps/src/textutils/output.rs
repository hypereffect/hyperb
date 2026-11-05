//! What a tool answers: a few kinds of blocks that the form shows as
//! elements, each in the way that suits it.

use super::super::img;
use hypereffect::vtree::{self, Node};
use std::collections::HashSet;

/// The most items a list or a table shows; the rest are counted.
const MOST: usize = 300;

pub enum Block {
    /// Labelled values: the answer of most calculators.
    Facts(Vec<(String, String)>),
    /// Rows under a header, such as a schedule.
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// Text to copy or to work on again, monospaced.
    Code(String),
    /// Prose.
    Text(String),
    /// Items, one per line, such as matches or generated names.
    List(Vec<String>),
    /// Whether the input passed a check, and why.
    Verdict {
        ok: bool,
        text: String,
    },
    Image {
        png: Vec<u8>,
        alt: String,
    },
    /// A secondary remark.
    Note(String),
}

#[derive(Default)]
pub struct Output {
    pub blocks: Vec<Block>,
}

impl Output {
    pub fn facts(mut self, rows: &[(&str, String)]) -> Self {
        self.blocks.push(Block::Facts(
            rows.iter()
                .map(|(l, v)| ((*l).to_owned(), v.clone()))
                .collect(),
        ));
        self
    }

    pub fn table<S: Into<String>>(
        mut self,
        head: &[&str],
        rows: impl IntoIterator<Item = Vec<S>>,
    ) -> Self {
        self.blocks.push(Block::Table {
            head: head.iter().map(|h| (*h).to_owned()).collect(),
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(Into::into).collect())
                .collect(),
        });
        self
    }

    pub fn code(mut self, text: impl Into<String>) -> Self {
        self.blocks.push(Block::Code(text.into()));
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.blocks.push(Block::Text(text.into()));
        self
    }

    pub fn list(mut self, items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.blocks
            .push(Block::List(items.into_iter().map(Into::into).collect()));
        self
    }

    pub fn verdict(mut self, ok: bool, text: impl Into<String>) -> Self {
        self.blocks.push(Block::Verdict {
            ok,
            text: text.into(),
        });
        self
    }

    pub fn image(mut self, png: Vec<u8>, alt: impl Into<String>) -> Self {
        self.blocks.push(Block::Image {
            png,
            alt: alt.into(),
        });
        self
    }

    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.blocks.push(Block::Note(text.into()));
        self
    }

    /// Appends the blocks of `other`.
    pub fn then(mut self, other: Output) -> Self {
        self.blocks.extend(other.blocks);
        self
    }

    /// The text the user may take as the next input: the first code block,
    /// or else the first list.
    pub fn reusable(&self) -> Option<String> {
        let code = self.blocks.iter().find_map(|b| match b {
            Block::Code(c) => Some(c.clone()),
            _ => None,
        });
        code.or_else(|| {
            self.blocks.iter().find_map(|b| match b {
                Block::List(items) => Some(items.join("\n")),
                _ => None,
            })
        })
    }

    /// The output as plain text, for tests and image descriptions.
    pub fn plain(&self) -> String {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::Facts(rows) => Some(aligned(rows)),
                Block::Table { head, rows } => {
                    let mut all = vec![head.clone()];
                    all.extend(rows.iter().cloned());
                    Some(columns(&all))
                }
                Block::Code(t) | Block::Text(t) | Block::Note(t) => Some(t.clone()),
                Block::List(items) => Some(items.join("\n")),
                Block::Verdict { ok, text } => {
                    Some(format!("{} {text}", if *ok { "✓" } else { "✗" }))
                }
                Block::Image { .. } => None,
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The elements of the output, under ids that start with `prefix`.
    /// `counts` adds the size of code blocks, for tools on text.
    pub fn view(&self, prefix: &str, counts: bool) -> Vec<Node> {
        let mut ids = Ids::default();
        let mut nodes = Vec::new();
        for b in &self.blocks {
            match b {
                Block::Facts(rows) if rows.len() == 1 => {
                    let (label, value) = &rows[0];
                    let id = ids.take(&format!("{prefix}-{}", slug(label)));
                    nodes.push(
                        vtree::column(id.as_str()).gap("none").children([
                            vtree::text(format!("{id}-label"))
                                .variant("caption")
                                .text(label.as_str()),
                            vtree::text(format!("{id}-value"))
                                .variant("h2")
                                .text(value.as_str()),
                        ]),
                    );
                }
                Block::Facts(rows) => {
                    let list = ids.take(&format!("{prefix}-facts"));
                    nodes.push(vtree::list(list.as_str()).children(rows.iter().map(
                        |(label, value)| {
                            if label.is_empty() {
                                let id = ids.take(&format!("{list}-{}", slug(value)));
                                return vtree::text(id).variant("caption").text(value.as_str());
                            }
                            let id = ids.take(&format!("{list}-{}", slug(label)));
                            vtree::row(id.as_str())
                                .justify("between")
                                .wrap(true)
                                .children([
                                    vtree::text(format!("{id}-label")).text(label.as_str()),
                                    vtree::text(format!("{id}-value"))
                                        .variant("h3")
                                        .text(value.as_str()),
                                ])
                        },
                    )));
                }
                Block::Table { head, rows } => {
                    let list = ids.take(&format!("{prefix}-table"));
                    let shown = rows.iter().take(MOST).map(|row| {
                        let first = row.first().map_or("", String::as_str);
                        let id = ids.take(&format!("{list}-{}", slug(first)));
                        let rest: Vec<String> = row
                            .iter()
                            .zip(head.iter())
                            .skip(1)
                            .filter(|(v, _)| !v.is_empty())
                            .map(|(v, h)| {
                                if h.is_empty() {
                                    v.clone()
                                } else {
                                    format!("{h}: {v}")
                                }
                            })
                            .collect();
                        let lead = match head.first() {
                            Some(h)
                                if !h.is_empty() && first.chars().all(|c| c.is_ascii_digit()) =>
                            {
                                format!("{h} {first}")
                            }
                            _ => first.to_owned(),
                        };
                        vtree::column(id.as_str()).gap("none").children([
                            vtree::text(format!("{id}-name")).text(lead),
                            vtree::text(format!("{id}-values"))
                                .variant("caption")
                                .text(rest.join(" · ")),
                        ])
                    });
                    nodes.push(vtree::list(list.as_str()).children(shown));
                    nodes.extend(more(&mut ids, prefix, rows.len()));
                }
                Block::Code(text) => {
                    let id = ids.take(&format!("{prefix}-code"));
                    if text.is_empty() {
                        nodes.push(
                            vtree::text(id)
                                .variant("caption")
                                .text("The result is empty."),
                        );
                        continue;
                    }
                    nodes.push(vtree::text(id.as_str()).variant("code").text(text.as_str()));
                    if counts {
                        let lines = text.lines().count();
                        let chars = text.chars().count();
                        nodes.push(vtree::badge(format!("{id}-size")).variant("info").text(
                            format!(
                                "{lines} line{} · {chars} character{}",
                                if lines == 1 { "" } else { "s" },
                                if chars == 1 { "" } else { "s" },
                            ),
                        ));
                    }
                }
                Block::Text(text) => {
                    nodes.push(vtree::text(ids.take(&format!("{prefix}-text"))).text(text.as_str()))
                }
                Block::List(items) => {
                    let list = ids.take(&format!("{prefix}-list"));
                    let shown = items.iter().take(MOST).map(|item| {
                        vtree::text(ids.take(&format!("{list}-{}", slug(item)))).text(item.as_str())
                    });
                    nodes.push(
                        vtree::list(list.as_str())
                            .empty("Nothing found.")
                            .children(shown),
                    );
                    nodes.extend(more(&mut ids, prefix, items.len()));
                }
                Block::Verdict { ok, text } => nodes.push(
                    vtree::notice(ids.take(&format!("{prefix}-verdict")))
                        .variant(if *ok { "success" } else { "failure" })
                        .text(text.as_str()),
                ),
                Block::Image { png, alt } => nodes.push(
                    vtree::image(ids.take(&format!("{prefix}-image")))
                        .variant("full")
                        .src(img::data_uri(png))
                        .alt(alt.as_str()),
                ),
                Block::Note(text) => nodes.push(
                    vtree::text(ids.take(&format!("{prefix}-note")))
                        .variant("caption")
                        .text(text.as_str()),
                ),
            }
        }
        nodes
    }
}

/// A plain answer is text to copy or work on again.
impl From<String> for Output {
    fn from(text: String) -> Self {
        Output::default().code(text)
    }
}

impl From<&str> for Output {
    fn from(text: &str) -> Self {
        Output::default().code(text)
    }
}

/// A caption counting what a list or a table leaves out.
fn more(ids: &mut Ids, prefix: &str, len: usize) -> Option<Node> {
    (len > MOST).then(|| {
        vtree::text(ids.take(&format!("{prefix}-more")))
            .variant("caption")
            .text(format!("And {} more, not shown.", len - MOST))
    })
}

/// Ids made unique by a counter.
#[derive(Default)]
struct Ids(HashSet<String>);

impl Ids {
    fn take(&mut self, base: &str) -> String {
        let mut id = base.to_owned();
        let mut n = 1;
        while !self.0.insert(id.clone()) {
            n += 1;
            id = format!("{base}-{n}");
        }
        id
    }
}

/// An id part from a label: lowercase ASCII words joined by `-`, at most
/// 40 characters, or `item` when nothing is left. Subscript and superscript
/// digits become digits, and Greek letters their names.
pub fn slug(label: &str) -> String {
    let mut out = String::new();
    for c in label.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if let Some(d) = "₀₁₂₃₄₅₆₇₈₉"
            .chars()
            .position(|s| s == c)
            .or_else(|| "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|s| s == c))
        {
            out.push(char::from(b'0' + d as u8));
        } else if let Some(name) = greek(c) {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
            out.push_str(name);
            out.push('-');
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    let out = out.trim_end_matches('-');
    if out.is_empty() {
        "item".into()
    } else {
        out.into()
    }
}

fn greek(c: char) -> Option<&'static str> {
    const NAMES: [&str; 24] = [
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigma", "tau", "upsilon", "phi",
        "chi", "psi", "omega",
    ];
    // Final sigma sits between rho and sigma.
    let i = match c {
        'α'..='ρ' => c as usize - 'α' as usize,
        'ς' | 'σ' => 17,
        'τ'..='ω' => c as usize - 'α' as usize - 1,
        'µ' => 11,
        'Ω' => 23,
        _ => return None,
    };
    Some(NAMES[i])
}

/// Aligned `label: value` lines.
fn aligned(rows: &[(String, String)]) -> String {
    let width = rows
        .iter()
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(0);
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

/// Rows in left-aligned columns.
fn columns(rows: &[Vec<String>]) -> String {
    let n = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..n)
        .map(|i| {
            rows.iter()
                .filter_map(|r| r.get(i))
                .map(|c| c.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    rows.iter()
        .map(|r| {
            r.iter()
                .enumerate()
                .map(|(i, c)| format!("{c}{}", " ".repeat(widths[i] - c.chars().count())))
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("Rate (%)"), "rate");
        assert_eq!(slug("Down payment, %"), "down-payment");
        assert_eq!(slug("ΔT"), "delta-t");
        assert_eq!(slug("C₁ (mol/L)"), "c1-mol-l");
        assert_eq!(slug("λ"), "lambda");
        assert_eq!(slug("—"), "item");
    }

    #[test]
    fn plain_text() {
        let o = Output::default()
            .facts(&[("Net", "100".into()), ("Tax", "20".into())])
            .note("Rounded.");
        assert_eq!(o.plain(), "Net: 100\nTax: 20\n\nRounded.");
        let t = Output::default().table(&["Year", "Balance"], [vec!["1", "900"], vec!["2", "0"]]);
        assert_eq!(t.plain(), "Year  Balance\n1     900\n2     0");
    }

    #[test]
    fn ids_are_unique() {
        let o = Output::default().list(["a", "a", "A"]).list(["b"]);
        let mut seen = HashSet::new();
        fn walk(n: &Node, seen: &mut HashSet<String>) {
            assert!(seen.insert(n.id().to_owned()), "{}", n.id());
            n.children.iter().for_each(|c| walk(c, seen));
        }
        for n in o.view("result", false) {
            walk(&n, &mut seen);
        }
        assert!(seen.contains("result-list-a-2") && seen.contains("result-list-2-b"));
    }
}
