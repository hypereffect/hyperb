//! Tools on plain text: cleaning, sorting, counting, extracting and numbers.

use super::Output;
use super::{Opts, table, verdict};
use regex_lite::{Regex, RegexBuilder};
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

fn lines(items: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    items
        .into_iter()
        .map(|s| s.as_ref().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The words of `text`: runs of letters and digits, with inner apostrophes.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '’'))
        .map(|w| w.trim_matches(|c| c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Every match of `re` in `text`, one per line, without repeats if `unique`.
fn extract(re: &Regex, text: &str, unique: bool, group: usize) -> Output {
    let mut seen = HashSet::new();
    super::list(
        re.captures_iter(text)
            .filter_map(|c| c.get(group).map(|m| m.as_str()))
            .filter(|m| !unique || seen.insert(*m)),
    )
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid pattern")
}

// Text

pub fn case(text: &str, o: &mut Opts) -> Result<Output, String> {
    let capitalize = |w: &str| {
        let mut c = w.chars();
        c.next().map_or(String::new(), |f| {
            f.to_uppercase()
                .chain(c.flat_map(char::to_lowercase))
                .collect()
        })
    };
    let joined = |sep: &str, f: &dyn Fn(usize, &str) -> String| {
        text.lines()
            .map(|l| {
                words(l)
                    .iter()
                    .enumerate()
                    .map(|(i, w)| f(i, w))
                    .collect::<Vec<_>>()
                    .join(sep)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    Ok(super::snippet(match o.mode {
        "lower" => text.to_lowercase(),
        "title" => {
            let mut out = String::new();
            let mut start = true;
            for c in text.chars() {
                if start && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    start = false;
                } else {
                    out.extend(c.to_lowercase());
                    start =
                        start && !c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '"';
                }
            }
            out
        }
        "sentence" => {
            let mut out = String::new();
            let mut start = true;
            for c in text.chars() {
                if start && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    start = false;
                } else {
                    out.extend(c.to_lowercase());
                    if matches!(c, '.' | '!' | '?' | '\n') {
                        start = true;
                    }
                }
            }
            out
        }
        "alternating" => {
            let mut upper = false;
            text.chars()
                .flat_map(|c| {
                    if c.is_alphabetic() {
                        upper = !upper;
                    }
                    let v: Vec<char> = if upper {
                        c.to_lowercase().collect()
                    } else {
                        c.to_uppercase().collect()
                    };
                    v
                })
                .collect()
        }
        "inverse" => text
            .chars()
            .flat_map(|c| {
                let v: Vec<char> = if c.is_uppercase() {
                    c.to_lowercase().collect()
                } else {
                    c.to_uppercase().collect()
                };
                v
            })
            .collect(),
        "camel" => joined("", &|i, w| {
            if i == 0 {
                w.to_lowercase()
            } else {
                capitalize(w)
            }
        }),
        "pascal" => joined("", &|_, w| capitalize(w)),
        "snake" => joined("_", &|_, w| w.to_lowercase()),
        "kebab" => joined("-", &|_, w| w.to_lowercase()),
        "constant" => joined("_", &|_, w| w.to_uppercase()),
        _ => text.to_uppercase(),
    }))
}

pub fn find_replace(text: &str, o: &mut Opts) -> Result<Output, String> {
    if o.first.is_empty() {
        return Err("Type what to find.".into());
    }
    let pattern = match o.mode {
        "regex" => o.first.to_owned(),
        _ => regex_lite::escape(o.first),
    };
    let re = RegexBuilder::new(&pattern)
        .case_insensitive(o.flag)
        .build()
        .map_err(|e| format!("Not a regular expression: {e}"))?;
    let found = re.find_iter(text).count();
    Ok(Output::from(match o.mode {
        "regex" => re.replace_all(text, o.second).into_owned(),
        _ => re
            .replace_all(text, regex_lite::NoExpand(o.second))
            .into_owned(),
    })
    .note(format!("{found} replaced")))
}

pub fn remove_accents(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(unaccented(text).into())
}

/// Any script as ASCII: accented letters, Cyrillic, Greek, kana and Han.
pub fn ascii(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(deunicode::deunicode(text).into())
}

/// `text` without accents and diacritics.
pub fn unaccented(text: &str) -> String {
    text.nfd()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .flat_map(|c| {
            // Letters that are not a base letter and a mark.
            let s = match c {
                'ß' => "ss",
                'æ' => "ae",
                'Æ' => "AE",
                'œ' => "oe",
                'Œ' => "OE",
                'ø' => "o",
                'Ø' => "O",
                'đ' => "d",
                'Đ' => "D",
                'ł' => "l",
                'Ł' => "L",
                'ı' => "i",
                'þ' => "th",
                'Þ' => "Th",
                'ð' => "d",
                'Ð' => "D",
                _ => return vec![c],
            };
            s.chars().collect()
        })
        .collect()
}

fn separator(mode: &str) -> &'static str {
    match mode {
        "newline" => "\n",
        "space" => " ",
        _ => ",",
    }
}

pub fn duplicate_keywords(text: &str, o: &mut Opts) -> Result<Output, String> {
    let sep = separator(o.mode);
    let mut seen = HashSet::new();
    let kept: Vec<&str> = text
        .split(|c: char| {
            if sep == "," {
                c == ','
            } else {
                c.is_whitespace() && (sep == " " || c == '\n')
            }
        })
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .filter(|k| {
            seen.insert(if o.flag {
                k.to_lowercase()
            } else {
                k.to_string()
            })
        })
        .collect();
    Ok(super::snippet(kept.join(if sep == "," {
        ", "
    } else {
        sep
    })))
}

pub fn duplicate_lines(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut seen = HashSet::new();
    let key = |l: &str| {
        let l = l.trim();
        if o.flag {
            l.to_lowercase()
        } else {
            l.to_owned()
        }
    };
    let kept: Vec<&str> = match o.mode {
        "all" => {
            let mut counts = HashMap::new();
            for l in text.lines() {
                *counts.entry(key(l)).or_insert(0) += 1;
            }
            text.lines().filter(|l| counts[&key(l)] == 1).collect()
        }
        _ => text.lines().filter(|l| seen.insert(key(l))).collect(),
    };
    Ok(super::snippet(lines(kept)))
}

fn is_emoji(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF   // pictographs, emoticons, transport, flags, symbols
        | 0x2600..=0x27BF   // miscellaneous symbols and dingbats
        | 0x2B00..=0x2BFF   // arrows and stars
        | 0x2300..=0x23FF   // technical: watch, hourglass…
        | 0xFE00..=0xFE0F   // variation selectors
        | 0x200D            // zero-width joiner
        | 0x20E3            // keycap
        | 0xE0020..=0xE007F // tags
        | 0x3030 | 0x303D | 0x3297 | 0x3299 | 0x00A9 | 0x00AE | 0x2122
        | 0x2194..=0x21AA | 0x24C2 | 0x25AA..=0x25FE)
}

/// Collapses the runs of spaces that removing things leaves.
fn tidy(text: &str) -> String {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn remove_emojis(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(tidy(
        &text.chars().filter(|c| !is_emoji(*c)).collect::<String>(),
    )))
}

pub fn remove_empty_lines(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(lines(
        text.lines().filter(|l| !l.trim().is_empty()),
    )))
}

pub fn remove_extra_spaces(text: &str, o: &mut Opts) -> Result<Output, String> {
    if o.mode == "json" {
        // Whitespace outside of strings goes; JSON need not be valid.
        let mut out = String::new();
        let (mut quoted, mut escaped) = (false, false);
        for c in text.chars() {
            if quoted {
                out.push(c);
                match (escaped, c) {
                    (true, _) => escaped = false,
                    (_, '\\') => escaped = true,
                    (_, '"') => quoted = false,
                    _ => {}
                }
            } else if c == '"' {
                quoted = true;
                out.push(c);
            } else if !c.is_whitespace() {
                out.push(c);
            }
        }
        return Ok(super::snippet(out));
    }
    Ok(super::snippet(
        text.lines()
            .map(|l| {
                let mut out = String::new();
                for word in l.split([' ', '\t', '\u{a0}']).filter(|w| !w.is_empty()) {
                    // No space before punctuation.
                    if !out.is_empty() && !word.starts_with([',', '.', ';', ':', '!', '?', ')']) {
                        out.push(' ');
                    }
                    out.push_str(word);
                }
                out
            })
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

pub fn remove_hashtags(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(tidy(
        &regex(r"(^|\s)#\w+").replace_all(text, "$1"),
    )))
}

pub fn remove_spaces(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(
        text.chars().filter(|c| !c.is_whitespace()).collect(),
    ))
}

pub fn remove_line_breaks(text: &str, o: &mut Opts) -> Result<Output, String> {
    let join = |t: &str| match o.mode {
        "nothing" => t.lines().map(str::trim).collect::<String>(),
        _ => t.split_whitespace().collect::<Vec<_>>().join(" "),
    };
    Ok(super::snippet(match o.mode {
        "paragraphs" => regex(r"\n\s*\n")
            .split(text)
            .map(join)
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => join(text),
    }))
}

pub fn lines_containing(text: &str, o: &mut Opts) -> Result<Output, String> {
    if o.first.is_empty() {
        return Err("Type the text the lines should contain.".into());
    }
    let needle = if o.flag {
        o.first.to_lowercase()
    } else {
        o.first.to_owned()
    };
    let keep = o.mode == "keep";
    Ok(super::snippet(lines(text.lines().filter(|l| {
        let found = if o.flag {
            l.to_lowercase().contains(&needle)
        } else {
            l.contains(&needle)
        };
        found == keep
    }))))
}

pub fn reverse(text: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(match o.mode {
        "words" => lines(
            text.lines()
                .map(|l| l.split_whitespace().rev().collect::<Vec<_>>().join(" ")),
        ),
        "lines" => lines(text.lines().rev()),
        "each" => lines(text.lines().map(|l| {
            l.split_whitespace()
                .map(|w| w.chars().rev().collect::<String>())
                .collect::<Vec<_>>()
                .join(" ")
        })),
        _ => text.chars().rev().collect(),
    }))
}

pub fn sort_lines(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut v: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let key = |l: &str| {
        if o.flag {
            l.to_lowercase()
        } else {
            l.to_owned()
        }
    };
    match o.mode {
        "desc" => v.sort_by_key(|l| std::cmp::Reverse(key(l))),
        "length" => v.sort_by_key(|l| (l.chars().count(), key(l))),
        "random" => o.rng.shuffle(&mut v),
        "reverse" => v.reverse(),
        _ => v.sort_by_key(|l| key(l)),
    }
    Ok(super::snippet(lines(v)))
}

pub fn counter(text: &str, _: &mut Opts) -> Result<Output, String> {
    let all = words(text);
    let mut counts = HashMap::new();
    for w in &all {
        *counts.entry(w.to_lowercase()).or_insert(0) += 1;
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let sentences = regex(r"[^.!?\s][^.!?]*(?:[.!?]+|$)")
        .find_iter(text)
        .filter(|s| s.as_str().chars().any(char::is_alphanumeric))
        .count();
    let paragraphs = regex(r"\n\s*\n")
        .split(text)
        .filter(|p| !p.trim().is_empty())
        .count();
    // At 238 words a minute, an adult's average silent reading speed.
    let reading = (all.len() as f64 / 238.0 * 60.0).round() as u64;
    let rows = [
        ("Characters", text.chars().count().to_string()),
        (
            "Characters without spaces",
            text.chars()
                .filter(|c| !c.is_whitespace())
                .count()
                .to_string(),
        ),
        ("Words", all.len().to_string()),
        ("Unique words", ranked.len().to_string()),
        ("Sentences", sentences.to_string()),
        ("Paragraphs", paragraphs.to_string()),
        ("Lines", text.lines().count().to_string()),
        (
            "Reading time",
            format!("{}m {:02}s", reading / 60, reading % 60),
        ),
    ];
    let out = table(&rows);
    if ranked.is_empty() {
        return Ok(out);
    }
    Ok(out.note("Most used words").table(
        &["Word", "Times"],
        ranked
            .iter()
            .take(5)
            .map(|(word, count)| vec![word.clone(), format!("{count}×")]),
    ))
}

pub fn upside_down(text: &str, _: &mut Opts) -> Result<Output, String> {
    const FROM: &str =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,!?'\"()[]{}<>&_;";
    const TO: &str =
        "ɐqɔpǝɟƃɥᴉɾʞlɯuodbɹsʇnʌʍxʎz∀ꓭƆꓷƎℲ⅁HIſꓘ˥WNOԀῸꓤS⊥∩ΛMX⅄Z0ƖᄅƐㄣϛ9ㄥ86˙'¡¿,„)(][}{><⅋‾؛";
    let map: HashMap<char, char> = FROM.chars().zip(TO.chars()).collect();
    Ok(super::snippet(
        text.lines()
            .rev()
            .map(|l| {
                l.chars()
                    .rev()
                    .map(|c| *map.get(&c).unwrap_or(&c))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

// Extractors

pub fn capitalized(text: &str, o: &mut Opts) -> Result<Output, String> {
    let all_caps = o.mode == "caps";
    let mut seen = HashSet::new();
    Ok(super::list(words(text).into_iter().filter(|w| {
        let mut letters = w.chars().filter(|c| c.is_alphabetic());
        let ok = if all_caps {
            w.chars().filter(|c| c.is_alphabetic()).count() > 1 && letters.all(char::is_uppercase)
        } else {
            w.chars().next().is_some_and(char::is_uppercase)
        };
        ok && (!o.flag || seen.insert(*w))
    })))
}

const MONTHS: &str = "(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|june?|july?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)";

pub fn dates(text: &str, o: &mut Opts) -> Result<Output, String> {
    let re = regex(&format!(
        r"(?i)\b(\d{{4}}[-/.]\d{{1,2}}[-/.]\d{{1,2}}|\d{{1,2}}[-/.]\d{{1,2}}[-/.]\d{{2,4}}|{MONTHS}\.? \d{{1,2}}(?:st|nd|rd|th)?,? \d{{4}}|\d{{1,2}}(?:st|nd|rd|th)? (?:of )?{MONTHS}\.?,? \d{{4}}|{MONTHS} \d{{4}})\b"
    ));
    Ok(extract(&re, text, o.flag, 1))
}

pub fn emails(text: &str, o: &mut Opts) -> Result<Output, String> {
    let re = regex(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}");
    Ok(extract(&re, text, o.flag, 0))
}

pub fn quotes(text: &str, o: &mut Opts) -> Result<Output, String> {
    let pattern = match o.mode {
        "double" => r#""([^"\n]*)"|“([^”\n]*)”|«([^»\n]*)»"#,
        // Single quotes only around a phrase, so apostrophes are not taken.
        "single" => r"(?:^|[^\w])'([^'\n]+)'(?:[^\w]|$)|‘([^’\n]*)’",
        _ => r#""([^"\n]*)"|“([^”\n]*)”|«([^»\n]*)»|(?:^|[^\w])'([^'\n]+)'(?:[^\w]|$)|‘([^’\n]*)’"#,
    };
    let mut seen = HashSet::new();
    Ok(super::list(
        regex(pattern)
            .captures_iter(text)
            .filter_map(|c| c.iter().skip(1).flatten().next().map(|m| m.as_str()))
            .filter(|m| !o.flag || seen.insert(*m)),
    ))
}

pub fn urls(text: &str, o: &mut Opts) -> Result<Output, String> {
    let re = regex(r#"(?i)\b(?:(?:https?|ftp)://|www\.)[^\s<>"'`]+"#);
    let mut seen = HashSet::new();
    Ok(super::list(
        re.find_iter(text)
            .map(|m| {
                m.as_str()
                    .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']'])
            })
            .filter(|m| !o.flag || seen.insert(*m)),
    ))
}

pub fn hashtags(text: &str, o: &mut Opts) -> Result<Output, String> {
    let re = regex(r"(?:^|[^\w&#])(#\w*[^\W\d]\w*)");
    Ok(extract(&re, text, o.flag, 1))
}

pub fn ips(text: &str, o: &mut Opts) -> Result<Output, String> {
    let v4 = r"\b(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\b";
    let v6 = r"(?i)(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}|(?:[0-9a-f]{1,4}:){1,7}:|(?:[0-9a-f]{1,4}:){1,6}:[0-9a-f]{1,4}|(?:[0-9a-f]{1,4}:){1,5}(?::[0-9a-f]{1,4}){1,2}|(?:[0-9a-f]{1,4}:){1,4}(?::[0-9a-f]{1,4}){1,3}|(?:[0-9a-f]{1,4}:){1,3}(?::[0-9a-f]{1,4}){1,4}|(?:[0-9a-f]{1,4}:){1,2}(?::[0-9a-f]{1,4}){1,5}|[0-9a-f]{1,4}:(?::[0-9a-f]{1,4}){1,6}|::(?:[0-9a-f]{1,4}:){0,6}[0-9a-f]{1,4}";
    let pattern = match o.mode {
        "v4" => v4.to_owned(),
        "v6" => format!(r"(?:^|[^\w:])({v6})(?:[^\w:]|$)"),
        _ => format!(r"(?:^|[^\w:])({v4}|{v6})(?:[^\w:]|$)"),
    };
    let group = if o.mode == "v4" { 0 } else { 1 };
    // Overlapping boundaries: run on each word so neighbours are not eaten.
    let re = regex(&pattern);
    let mut seen = HashSet::new();
    Ok(super::list(
        text.split_whitespace()
            .flat_map(|w| {
                re.captures_iter(w)
                    .filter_map(|c| c.get(group).map(|m| m.as_str().to_owned()))
                    .collect::<Vec<_>>()
            })
            .filter(|m| !o.flag || seen.insert(m.clone())),
    ))
}

/// Characters that do not show: zero-width ones, invisible operators and
/// marks, odd spaces, and the bidirectional controls.
fn is_hidden(c: char) -> bool {
    matches!(c as u32,
        0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F
        | 0xFEFF | 0x00AD | 0x180E | 0x034F | 0x061C | 0xFFF9..=0xFFFB
        | 0xE0000..=0xE007F)
}

fn is_odd_space(c: char) -> bool {
    matches!(
        c as u32,
        0x00A0 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

pub fn hidden(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut removed = 0;
    let out: String = text
        .chars()
        .filter_map(|c| {
            if is_hidden(c) {
                removed += 1;
                None
            } else if is_odd_space(c) {
                removed += 1;
                Some(' ')
            } else {
                Some(c)
            }
        })
        .collect();
    Ok(Output::from(out).note(format!("{removed} hidden characters found")))
}

pub fn special(text: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(
        text.chars()
            .filter(|c| {
                c.is_whitespace()
                    || match o.mode {
                        "letters" => c.is_alphabetic(),
                        "ascii" => c.is_ascii_alphanumeric(),
                        "punctuation" => c.is_alphanumeric() || ".,;:!?'\"-()".contains(*c),
                        _ => c.is_alphanumeric(),
                    }
            })
            .collect(),
    ))
}

// Numbers

fn number_re() -> Regex {
    regex(r"[-+]?(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?(?:[eE][-+]?\d+)?|[-+]?\.\d+")
}

pub fn extract_numbers(text: &str, o: &mut Opts) -> Result<Output, String> {
    let found: Vec<&str> = number_re().find_iter(text).map(|m| m.as_str()).collect();
    Ok(super::snippet(found.join(separator(o.mode))))
}

pub fn remove_numbers(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(tidy(&regex(r"\d").replace_all(text, ""))))
}

pub fn sort_numbers(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut found: Vec<(f64, &str)> = number_re()
        .find_iter(text)
        .filter_map(|m| Some((m.as_str().replace(',', "").parse().ok()?, m.as_str())))
        .collect();
    if found.is_empty() {
        return Err("There are no numbers in the text.".into());
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    if o.mode == "desc" {
        found.reverse();
    }
    if o.flag {
        found.dedup_by(|a, b| a.0 == b.0);
    }
    Ok(super::snippet(
        found.iter().map(|(_, s)| *s).collect::<Vec<_>>().join("\n"),
    ))
}

// More text

pub fn char_frequency(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut counts: HashMap<char, usize> = HashMap::new();
    for c in text.chars().filter(|c| !c.is_whitespace() || o.flag) {
        let c = if o.check(1) {
            c.to_lowercase().next().unwrap_or(c)
        } else {
            c
        };
        *counts.entry(c).or_insert(0) += 1;
    }
    let total: usize = counts.values().sum();
    let mut ranked: Vec<(char, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(super::grid(
        &["Character", "Count", "Share"],
        ranked.iter().map(|(c, n)| {
            let shown = match c {
                ' ' => "space".to_owned(),
                '\n' => "newline".to_owned(),
                '\t' => "tab".to_owned(),
                c => c.to_string(),
            };
            vec![
                shown,
                n.to_string(),
                format!("{:.2}%", *n as f64 * 100.0 / total as f64),
            ]
        }),
    ))
}

pub fn word_frequency(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for w in words(text) {
        let w = w.to_lowercase();
        if !o.flag || !STOPWORDS.contains(&w.as_str()) {
            *counts.entry(w).or_insert(0) += 1;
        }
    }
    let total: usize = counts.values().sum();
    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(super::grid(
        &["Word", "Count", "Share"],
        ranked.iter().map(|(w, n)| {
            vec![
                w.clone(),
                n.to_string(),
                format!("{:.2}%", *n as f64 * 100.0 / total as f64),
            ]
        }),
    ))
}

/// The lines of `a` and `b`, as `  `, `- ` and `+ ` lines, from the longest
/// common subsequence.
fn diff_lines<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<(char, &'a str)> {
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            out.push((' ', a[i]));
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            out.push(('+', b[j]));
            j += 1;
        } else {
            out.push(('-', a[i]));
            i += 1;
        }
    }
    out
}

pub fn diff(text: &str, o: &mut Opts) -> Result<Output, String> {
    let (a, b) = text
        .split_once("\n===\n")
        .ok_or("Put a line with only “===” between the two texts.")?;
    let split = |t: &'_ str| -> Vec<String> {
        if o.mode == "words" {
            t.split_whitespace().map(str::to_owned).collect()
        } else {
            t.lines().map(str::to_owned).collect()
        }
    };
    let (a, b) = (split(a), split(b));
    let (a, b): (Vec<&str>, Vec<&str>) = (
        a.iter().map(String::as_str).collect(),
        b.iter().map(String::as_str).collect(),
    );
    if a.len() * b.len() > 4_000_000 {
        return Err("The texts are too long to compare.".into());
    }
    let d = diff_lines(&a, &b);
    let (added, removed) = (
        d.iter().filter(|x| x.0 == '+').count(),
        d.iter().filter(|x| x.0 == '-').count(),
    );
    let note = format!("{added} added, {removed} removed");
    if o.mode == "words" {
        return Ok(super::snippet(
            d.iter()
                .map(|(k, w)| match k {
                    '+' => format!("{{+{w}+}}"),
                    '-' => format!("[-{w}-]"),
                    _ => w.to_string(),
                })
                .collect::<Vec<_>>()
                .join(" "),
        )
        .note(note));
    }
    Ok(Output::from(lines(
        d.iter()
            .filter(|(k, _)| !o.flag || *k != ' ')
            .map(|(k, l)| format!("{k} {l}")),
    ))
    .note(note))
}

/// Letters in another Unicode alphabet: the offsets of `A`, `a` and `0`.
fn alphabet(c: char, upper: u32, lower: u32, digit: Option<u32>) -> char {
    let shift = |base: u32, from: char| char::from_u32(base + (c as u32 - from as u32));
    let r = match c {
        'A'..='Z' => shift(upper, 'A'),
        'a'..='z' => shift(lower, 'a'),
        '0'..='9' => digit.and_then(|d| shift(d, '0')),
        _ => None,
    };
    r.unwrap_or(c)
}

pub fn fancy(text: &str, o: &mut Opts) -> Result<Output, String> {
    // Letters that Unicode placed elsewhere, before the gaps they leave.
    let holes = |c: char, s: char| -> char {
        match (o.mode, s) {
            ("italic", 'h') => 'ℎ',
            ("script", 'B') => 'ℬ',
            ("script", 'E') => 'ℰ',
            ("script", 'F') => 'ℱ',
            ("script", 'H') => 'ℋ',
            ("script", 'I') => 'ℐ',
            ("script", 'L') => 'ℒ',
            ("script", 'M') => 'ℳ',
            ("script", 'R') => 'ℛ',
            ("script", 'e') => 'ℯ',
            ("script", 'g') => 'ℊ',
            ("script", 'o') => 'ℴ',
            ("fraktur", 'C') => 'ℭ',
            ("fraktur", 'H') => 'ℌ',
            ("fraktur", 'I') => 'ℑ',
            ("fraktur", 'R') => 'ℜ',
            ("fraktur", 'Z') => 'ℨ',
            ("double", 'C') => 'ℂ',
            ("double", 'H') => 'ℍ',
            ("double", 'N') => 'ℕ',
            ("double", 'P') => 'ℙ',
            ("double", 'Q') => 'ℚ',
            ("double", 'R') => 'ℝ',
            ("double", 'Z') => 'ℤ',
            _ => c,
        }
    };
    const SMALL: &str = "ᴀʙᴄᴅᴇꜰɢʜɪᴊᴋʟᴍɴᴏᴘǫʀꜱᴛᴜᴠᴡxʏᴢ";
    Ok(super::snippet(
        text.chars()
            .map(|c| {
                let mapped = match o.mode {
                    "italic" => alphabet(c, 0x1D434, 0x1D44E, None),
                    "bold-italic" => alphabet(c, 0x1D468, 0x1D482, None),
                    "script" => alphabet(c, 0x1D49C, 0x1D4B6, None),
                    "fraktur" => alphabet(c, 0x1D504, 0x1D51E, None),
                    "double" => alphabet(c, 0x1D538, 0x1D552, Some(0x1D7D8)),
                    "mono" => alphabet(c, 0x1D670, 0x1D68A, Some(0x1D7F6)),
                    "sans" => alphabet(c, 0x1D5D4, 0x1D5EE, Some(0x1D7EC)),
                    "circled" => alphabet(c, 0x24B6, 0x24D0, None),
                    "fullwidth" => match c {
                        '!'..='~' => char::from_u32(c as u32 + 0xFEE0).unwrap_or(c),
                        ' ' => '\u{3000}',
                        c => c,
                    },
                    "small" => match c {
                        'a'..='z' => SMALL.chars().nth(c as usize - 'a' as usize).unwrap_or(c),
                        c => c,
                    },
                    "strike" => return format!("{c}\u{336}"),
                    "underline" => return format!("{c}\u{332}"),
                    _ => alphabet(c, 0x1D400, 0x1D41A, Some(0x1D7CE)),
                };
                holes(mapped, c).to_string()
            })
            .collect(),
    ))
}

pub fn line_numbers(text: &str, o: &mut Opts) -> Result<Output, String> {
    let start = o.opt_num(0)?.unwrap_or(1.0) as i64;
    let sep = if o.second.is_empty() { ". " } else { o.second };
    let count = text.lines().count() as i64;
    let width = (start + count).to_string().len();
    let mut n = start;
    Ok(super::snippet(lines(text.lines().map(|l| {
        if o.flag && l.trim().is_empty() {
            return l.to_owned();
        }
        let s = format!("{n:>width$}{sep}{l}");
        n += 1;
        s
    }))))
}

const LOREM: &str = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur excepteur sint occaecat cupidatat non proident sunt in culpa qui officia deserunt mollit anim id est laborum";

pub fn lorem(_: &str, o: &mut Opts) -> Result<Output, String> {
    let vocab: Vec<&str> = LOREM.split(' ').collect();
    let n = o.number.max(1);
    let sentence = |rng: &mut super::Rng| {
        let len = 6 + rng.below(10);
        let mut s: Vec<String> = (0..len)
            .map(|_| vocab[rng.below(vocab.len())].to_owned())
            .collect();
        s[0] = capitalize(&s[0]);
        if len > 9 {
            let comma = 3 + rng.below(len - 6);
            s[comma].push(',');
        }
        format!("{}.", s.join(" "))
    };
    let start = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";
    Ok(super::snippet(match o.mode {
        "words" => {
            let mut w: Vec<&str> = (0..n)
                .map(|i| {
                    if o.flag && i < vocab.len() {
                        vocab[i]
                    } else {
                        vocab[o.rng.below(vocab.len())]
                    }
                })
                .collect();
            if w.is_empty() {
                w.push("lorem");
            }
            capitalize(&w.join(" "))
        }
        "sentences" => {
            let mut s: Vec<String> = (0..n).map(|_| sentence(o.rng)).collect();
            if o.flag {
                s[0] = start.into();
            }
            s.join(" ")
        }
        _ => {
            let mut p: Vec<String> = (0..n)
                .map(|_| {
                    let count = 4 + o.rng.below(4);
                    (0..count)
                        .map(|_| sentence(o.rng))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect();
            if o.flag {
                p[0] = format!("{start} {}", p[0]);
            }
            p.join("\n\n")
        }
    }))
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    c.next()
        .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

pub fn slug(text: &str, o: &mut Opts) -> Result<Output, String> {
    let sep = if o.mode == "underscore" { "_" } else { "-" };
    let plain = unaccented(text);
    Ok(super::snippet(lines(plain.lines().map(|l| {
        let l = if o.flag {
            l.to_owned()
        } else {
            l.to_lowercase()
        };
        l.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join(sep)
    }))))
}

// Writing

pub const STOPWORDS: &[&str] = &[
    "a",
    "about",
    "above",
    "after",
    "again",
    "against",
    "all",
    "am",
    "an",
    "and",
    "any",
    "are",
    "as",
    "at",
    "be",
    "because",
    "been",
    "before",
    "being",
    "below",
    "between",
    "both",
    "but",
    "by",
    "can",
    "could",
    "did",
    "do",
    "does",
    "doing",
    "down",
    "during",
    "each",
    "few",
    "for",
    "from",
    "further",
    "had",
    "has",
    "have",
    "having",
    "he",
    "her",
    "here",
    "hers",
    "herself",
    "him",
    "himself",
    "his",
    "how",
    "i",
    "if",
    "in",
    "into",
    "is",
    "it",
    "its",
    "itself",
    "just",
    "me",
    "more",
    "most",
    "my",
    "myself",
    "no",
    "nor",
    "not",
    "now",
    "of",
    "off",
    "on",
    "once",
    "only",
    "or",
    "other",
    "our",
    "ours",
    "ourselves",
    "out",
    "over",
    "own",
    "same",
    "she",
    "should",
    "so",
    "some",
    "such",
    "than",
    "that",
    "the",
    "their",
    "theirs",
    "them",
    "themselves",
    "then",
    "there",
    "these",
    "they",
    "this",
    "those",
    "through",
    "to",
    "too",
    "under",
    "until",
    "up",
    "very",
    "was",
    "we",
    "were",
    "what",
    "when",
    "where",
    "which",
    "while",
    "who",
    "whom",
    "why",
    "will",
    "with",
    "would",
    "you",
    "your",
    "yours",
    "yourself",
    "yourselves",
    "it's",
    "don't",
    "i'm",
    "can't",
    "also",
    "into",
    "one",
    "may",
    "might",
    "must",
];

/// Syllables in an English word, by vowel groups with the usual exceptions.
pub fn syllables(word: &str) -> usize {
    let w: String = word
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect();
    if w.is_empty() {
        return 0;
    }
    if w.len() <= 3 {
        return 1;
    }
    let vowel = |c: char| "aeiouy".contains(c);
    let chars: Vec<char> = w.chars().collect();
    let mut count = 0;
    let mut prev = false;
    for &c in &chars {
        let v = vowel(c);
        if v && !prev {
            count += 1;
        }
        prev = v;
    }
    // A silent final e, but not in "-le" after a consonant.
    if w.ends_with('e') && !w.ends_with("le") && !w.ends_with("ee") && count > 1 {
        count -= 1;
    }
    if (w.ends_with("es") || w.ends_with("ed"))
        && !w.ends_with("ted")
        && !w.ends_with("ded")
        && count > 1
        && !vowel(chars[chars.len() - 3])
    {
        count -= 1;
    }
    count.max(1)
}

fn sentences(text: &str) -> Vec<&str> {
    regex(r"[^.!?\n]+[.!?]*")
        .find_iter(text)
        .map(|m| m.as_str().trim())
        .filter(|s| s.chars().any(char::is_alphabetic))
        .collect()
}

pub fn readability(text: &str, _: &mut Opts) -> Result<Output, String> {
    let w = words(text);
    let s = sentences(text).len().max(1) as f64;
    if w.is_empty() {
        return Err("There are no words to score.".into());
    }
    let n = w.len() as f64;
    let syl: usize = w.iter().map(|w| syllables(w)).sum();
    let complex = w.iter().filter(|w| syllables(w) >= 3).count() as f64;
    let letters = w
        .iter()
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).count())
        .sum::<usize>() as f64;
    let syl = syl as f64;
    let ease = 206.835 - 1.015 * (n / s) - 84.6 * (syl / n);
    let grade = 0.39 * (n / s) + 11.8 * (syl / n) - 15.59;
    let fog = 0.4 * ((n / s) + 100.0 * complex / n);
    let smog = 1.043 * (complex * 30.0 / s).sqrt() + 3.1291;
    let cli = 0.0588 * (letters / n * 100.0) - 0.296 * (s / n * 100.0) - 15.8;
    let ari = 4.71 * (letters / n) + 0.5 * (n / s) - 21.43;
    let level = match ease {
        e if e >= 90.0 => "very easy (5th grade)",
        e if e >= 80.0 => "easy (6th grade)",
        e if e >= 70.0 => "fairly easy (7th grade)",
        e if e >= 60.0 => "plain English (8th–9th grade)",
        e if e >= 50.0 => "fairly difficult (high school)",
        e if e >= 30.0 => "difficult (college)",
        _ => "very difficult (graduate)",
    };
    Ok(super::table(&[
        ("Flesch reading ease", format!("{ease:.1} — {level}")),
        ("Flesch–Kincaid grade", format!("{grade:.1}")),
        ("Gunning fog index", format!("{fog:.1}")),
        ("SMOG index", format!("{smog:.1}")),
        ("Coleman–Liau index", format!("{cli:.1}")),
        ("Automated readability", format!("{ari:.1}")),
    ])
    .facts(&[
        ("Sentences", format!("{s}")),
        ("Words", format!("{n}")),
        ("Syllables", format!("{syl}")),
        ("Words per sentence", format!("{:.1}", n / s)),
        ("Syllables per word", format!("{:.2}", syl / n)),
        (
            "Complex words",
            format!("{complex} ({:.1}%)", complex * 100.0 / n),
        ),
    ]))
}

pub fn syllable_count(text: &str, _: &mut Opts) -> Result<Output, String> {
    let w = words(text);
    let total: usize = w.iter().map(|w| syllables(w)).sum();
    Ok(table(&[("Syllables", total.to_string())]).table(
        &["Word", "Syllables"],
        w.iter()
            .map(|w| vec![w.to_string(), syllables(w).to_string()]),
    ))
}

pub fn passive(text: &str, _: &mut Opts) -> Result<Output, String> {
    let re = regex(
        r"(?i)\b(?:am|is|are|was|were|be|been|being|get|gets|got|gotten)\s+(?:\w+ly\s+)?(\w+(?:ed|en|wn|lt|pt|ung|ought|aught|ade|ung|id|one))\b",
    );
    let found = sentences(text)
        .into_iter()
        .filter_map(|s| {
            re.find(s)
                .map(|m| vec![s.to_owned(), format!("“{}”", m.as_str())])
        })
        .collect::<Vec<_>>();
    if found.is_empty() {
        return Ok(verdict(true, "No passive voice found."));
    }
    let n = found.len();
    Ok(verdict(
        false,
        format!(
            "{n} sentence{} in the passive voice.",
            if n == 1 { "" } else { "s" }
        ),
    )
    .table(&["Sentence", "Passive"], found))
}

pub fn adverbs(text: &str, _: &mut Opts) -> Result<Output, String> {
    const NOT: &[&str] = &[
        "only", "family", "early", "reply", "apply", "supply", "fly", "july", "italy", "rely",
        "holy", "ugly", "belly", "jelly", "bully", "rally", "ally", "lily", "silly", "daily",
        "friendly", "lovely", "lonely", "likely", "lively", "costly", "elderly", "orderly",
    ];
    let mut counts: Vec<(String, usize)> = Vec::new();
    for w in words(text) {
        let l = w.to_lowercase();
        if l.ends_with("ly") && l.len() > 4 && !NOT.contains(&l.as_str()) {
            match counts.iter_mut().find(|c| c.0 == l) {
                Some(c) => c.1 += 1,
                None => counts.push((l, 1)),
            }
        }
    }
    let total: usize = counts.iter().map(|c| c.1).sum();
    let all = words(text).len().max(1);
    Ok(table(&[
        ("Adverbs", total.to_string()),
        ("Words", all.to_string()),
        (
            "Share",
            format!("{:.1}%", total as f64 * 100.0 / all as f64),
        ),
    ])
    .table(
        &["Adverb", "Times"],
        counts.iter().map(|(w, n)| vec![w.clone(), n.to_string()]),
    ))
}

const CLICHES: &[&str] = &[
    "at the end of the day",
    "avoid like the plague",
    "better late than never",
    "beat around the bush",
    "bite the bullet",
    "break the ice",
    "by the same token",
    "calm before the storm",
    "come to a head",
    "cut to the chase",
    "dead as a doornail",
    "every cloud has a silver lining",
    "few and far between",
    "fit as a fiddle",
    "for all intents and purposes",
    "go the extra mile",
    "hit the nail on the head",
    "in the nick of time",
    "it goes without saying",
    "last but not least",
    "low-hanging fruit",
    "move the needle",
    "needle in a haystack",
    "only time will tell",
    "out of the box",
    "think outside the box",
    "par for the course",
    "piece of cake",
    "plenty of fish in the sea",
    "read between the lines",
    "the bottom line",
    "time will tell",
    "tip of the iceberg",
    "touch base",
    "under the weather",
    "when all is said and done",
    "win-win",
    "game changer",
    "paradigm shift",
    "at this point in time",
    "in this day and age",
    "all walks of life",
    "back to square one",
    "ballpark figure",
    "blessing in disguise",
    "easier said than done",
    "head over heels",
    "on the same page",
    "level playing field",
    "the elephant in the room",
    "let the cat out of the bag",
    "once in a blue moon",
    "a perfect storm",
    "take it to the next level",
    "the writing on the wall",
    "slowly but surely",
    "the whole nine yards",
    "throw caution to the wind",
    "you can't judge a book by its cover",
    "think on your feet",
    "circle back",
    "deep dive",
    "synergy",
];

pub fn cliches(text: &str, _: &mut Opts) -> Result<Output, String> {
    let lower = text.to_lowercase();
    let found: Vec<Vec<String>> = CLICHES
        .iter()
        .filter_map(|c| {
            let n = lower.matches(c).count();
            (n > 0).then(|| vec![(*c).to_owned(), format!("{n}×")])
        })
        .collect();
    if found.is_empty() {
        return Ok(verdict(true, "No clichés found."));
    }
    Ok(super::grid(&["Cliché", "Times"], found))
}

pub fn repetition(text: &str, o: &mut Opts) -> Result<Output, String> {
    let w: Vec<String> = words(text).iter().map(|w| w.to_lowercase()).collect();
    let n = o.number.max(1);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for g in w.windows(n) {
        if n == 1 && (STOPWORDS.contains(&g[0].as_str()) || g[0].len() < 3) {
            continue;
        }
        *counts.entry(g.join(" ")).or_insert(0) += 1;
    }
    let mut ranked: Vec<(String, usize)> = counts.into_iter().filter(|c| c.1 > 1).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if ranked.is_empty() {
        return Ok(verdict(true, "Nothing is repeated."));
    }
    Ok(super::grid(
        &["Words", "Times"],
        ranked
            .iter()
            .take(50)
            .map(|(p, c)| vec![p.clone(), format!("{c}×")]),
    ))
}

pub fn sentence_length(text: &str, o: &mut Opts) -> Result<Output, String> {
    let limit = o.number.max(1);
    let all = sentences(text);
    if all.is_empty() {
        return Err("There are no sentences.".into());
    }
    let lens: Vec<usize> = all.iter().map(|s| words(s).len()).collect();
    let avg = lens.iter().sum::<usize>() as f64 / lens.len() as f64;
    let long = lens.iter().filter(|n| **n > limit).count();
    Ok(table(&[
        ("Sentences", lens.len().to_string()),
        ("Words on average", format!("{avg:.1}")),
        ("Longest", format!("{} words", lens.iter().max().unwrap())),
        (&format!("Over {limit} words"), long.to_string()),
    ])
    .table(
        &["Words", "Sentence", ""],
        all.iter().zip(&lens).map(|(s, n)| {
            vec![
                n.to_string(),
                s.to_string(),
                if *n > limit {
                    "too long".into()
                } else {
                    String::new()
                },
            ]
        }),
    ))
}

const TRANSITIONS: &[&str] = &[
    "accordingly",
    "additionally",
    "afterward",
    "also",
    "although",
    "as a result",
    "besides",
    "but",
    "consequently",
    "conversely",
    "finally",
    "first",
    "for example",
    "for instance",
    "furthermore",
    "hence",
    "however",
    "in addition",
    "in conclusion",
    "in contrast",
    "in fact",
    "in other words",
    "in short",
    "indeed",
    "instead",
    "likewise",
    "meanwhile",
    "moreover",
    "nevertheless",
    "next",
    "nonetheless",
    "on the other hand",
    "otherwise",
    "second",
    "similarly",
    "since",
    "so",
    "subsequently",
    "then",
    "therefore",
    "thus",
    "to summarize",
    "ultimately",
    "whereas",
    "yet",
    "because",
    "even though",
    "in the meantime",
    "to illustrate",
    "above all",
    "after all",
];

pub fn transitions(text: &str, _: &mut Opts) -> Result<Output, String> {
    let all = sentences(text);
    let mut found: Vec<(String, usize)> = Vec::new();
    let mut with = 0;
    for s in &all {
        let l = format!(" {} ", s.to_lowercase().replace(',', " "));
        let mut any = false;
        for t in TRANSITIONS {
            if l.contains(&format!(" {t} ")) {
                any = true;
                match found.iter_mut().find(|f| f.0 == *t) {
                    Some(f) => f.1 += 1,
                    None => found.push((t.to_string(), 1)),
                }
            }
        }
        with += any as usize;
    }
    let share = with as f64 * 100.0 / all.len().max(1) as f64;
    found.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(verdict(share >= 25.0, format!("{with} of {} sentences ({share:.0}%) have a transition word; 25% or more reads smoothly.", all.len()))
        .table(&["Transition", "Times"], found.iter().map(|(t, n)| vec![t.clone(), n.to_string()])))
}

pub fn lexical_density(text: &str, _: &mut Opts) -> Result<Output, String> {
    let w = words(text);
    if w.is_empty() {
        return Err("There are no words.".into());
    }
    let lexical = w
        .iter()
        .filter(|w| !STOPWORDS.contains(&w.to_lowercase().as_str()))
        .count();
    let unique: HashSet<String> = w.iter().map(|w| w.to_lowercase()).collect();
    Ok(super::table(&[
        (
            "Lexical density",
            format!("{:.1}%", lexical as f64 * 100.0 / w.len() as f64),
        ),
        ("Lexical words", lexical.to_string()),
        ("Function words", (w.len() - lexical).to_string()),
        (
            "Type–token ratio",
            format!("{:.1}%", unique.len() as f64 * 100.0 / w.len() as f64),
        ),
    ]))
}

pub fn keyword_density(text: &str, o: &mut Opts) -> Result<Output, String> {
    let w: Vec<String> = words(text).iter().map(|w| w.to_lowercase()).collect();
    let total = w.len().max(1);
    let mut out = Output::default();
    for n in 1..=3 {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for g in w.windows(n) {
            if o.flag
                && (STOPWORDS.contains(&g[0].as_str()) || STOPWORDS.contains(&g[n - 1].as_str()))
            {
                continue;
            }
            *counts.entry(g.join(" ")).or_insert(0) += 1;
        }
        let mut ranked: Vec<(String, usize)> =
            counts.into_iter().filter(|c| n == 1 || c.1 > 1).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        if ranked.is_empty() {
            continue;
        }
        out = out.table(
            &[
                ["Word", "Two-word phrase", "Three-word phrase"][n - 1],
                "Count",
                "Density",
            ],
            ranked.iter().take(10).map(|(k, c)| {
                vec![
                    k.clone(),
                    c.to_string(),
                    format!("{:.2}%", *c as f64 * 100.0 * n as f64 / total as f64),
                ]
            }),
        );
    }
    Ok(out)
}

pub fn keywords(text: &str, o: &mut Opts) -> Result<Output, String> {
    // Words weighed by frequency and length, without the common ones.
    let mut scores: HashMap<String, f64> = HashMap::new();
    for w in words(text) {
        let l = w.to_lowercase();
        if l.len() < 3 || STOPWORDS.contains(&l.as_str()) || l.chars().all(|c| c.is_numeric()) {
            continue;
        }
        *scores.entry(l).or_insert(0.0) +=
            1.0 + (w.chars().next().is_some_and(char::is_uppercase) as u8 as f64) * 0.5;
    }
    let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(super::list(
        ranked.iter().take(o.number.max(1)).map(|k| k.0.as_str()),
    ))
}

pub fn tokens(text: &str, _: &mut Opts) -> Result<Output, String> {
    let chars = text.chars().count();
    let w = words(text).len();
    let pieces = regex(r"\w+|[^\w\s]").find_iter(text).count();
    // Byte-pair tokenizers average about four characters of English a token.
    let estimate = ((chars as f64 / 4.0).max(w as f64 * 1.3)).round();
    Ok(super::table(&[
        ("Estimated tokens", format!("≈ {estimate}")),
        ("Characters", chars.to_string()),
        ("Words", w.to_string()),
        ("Words and symbols", pieces.to_string()),
        (
            "",
            "An estimate: each model's tokenizer counts a little differently.".into(),
        ),
    ]))
}

pub fn citation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let authors: Vec<&str> = o
        .field(0)
        .split(';')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .collect();
    let (title, year, source, url) = (o.field(1), o.field(2), o.field(3), o.field(4));
    if title.is_empty() {
        return Err("Fill in “Title”.".into());
    }
    // "First Last" → ("Last", "First").
    let split = |a: &str| -> (String, String) {
        match a.split_once(',') {
            Some((last, first)) => (last.trim().to_owned(), first.trim().to_owned()),
            None => match a.rsplit_once(' ') {
                Some((first, last)) => (last.to_owned(), first.to_owned()),
                None => (a.to_owned(), String::new()),
            },
        }
    };
    let initials = |first: &str| {
        first
            .split_whitespace()
            .map(|p| format!("{}.", p.chars().next().unwrap_or(' ')))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let year = if year.is_empty() {
        "n.d.".to_owned()
    } else {
        year.to_owned()
    };
    let tail = |sep: &str| {
        let mut t = String::new();
        if !source.is_empty() {
            t.push_str(&format!("{sep}{source}."));
        }
        if !url.is_empty() {
            t.push_str(&format!(" {url}"));
        }
        t
    };
    Ok(super::snippet(match o.mode {
        "mla" => {
            let names = match authors.as_slice() {
                [] => String::new(),
                [a] => {
                    let (l, f) = split(a);
                    format!("{l}, {f}. ")
                }
                [a, b] => {
                    let (l, f) = split(a);
                    format!("{l}, {f}, and {b}. ")
                }
                [a, ..] => {
                    let (l, f) = split(a);
                    format!("{l}, {f}, et al. ")
                }
            };
            format!(
                "{names}“{title}.”{}, {year}.",
                tail(" ").trim_end_matches('.')
            )
        }
        "chicago" => {
            let names: Vec<String> = authors
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let (l, f) = split(a);
                    if i == 0 {
                        format!("{l}, {f}")
                    } else {
                        format!("{f} {l}")
                    }
                })
                .collect();
            let names = match names.len() {
                0 => String::new(),
                1 => format!("{}. ", names[0]),
                _ => format!(
                    "{}, and {}. ",
                    names[..names.len() - 1].join(", "),
                    names[names.len() - 1]
                ),
            };
            format!("{names}{year}. “{title}.”{}", tail(" "))
        }
        "harvard" => {
            let names: Vec<String> = authors
                .iter()
                .map(|a| {
                    let (l, f) = split(a);
                    format!("{l}, {}", initials(&f))
                })
                .collect();
            let names = match names.len() {
                0 => String::new(),
                1 => names[0].clone(),
                _ => format!(
                    "{} and {}",
                    names[..names.len() - 1].join(", "),
                    names[names.len() - 1]
                ),
            };
            let url = if url.is_empty() {
                String::new()
            } else {
                format!(" Available at: {url}")
            };
            format!(
                "{names} ({year}) {title}.{}{url}",
                if source.is_empty() {
                    String::new()
                } else {
                    format!(" {source}.")
                }
            )
        }
        _ => {
            let names: Vec<String> = authors
                .iter()
                .map(|a| {
                    let (l, f) = split(a);
                    format!("{l}, {}", initials(&f))
                })
                .collect();
            let names = match names.len() {
                0 => String::new(),
                1 => format!("{} ", names[0]),
                _ => format!(
                    "{}, & {} ",
                    names[..names.len() - 1].join(", "),
                    names[names.len() - 1]
                ),
            };
            format!("{names}({year}). {title}.{}", tail(" "))
        }
    }))
}

// The menus

use super::{Tool, tool};

const CASES: super::Menu = &[
    ("upper", "UPPER CASE"),
    ("lower", "lower case"),
    ("title", "Title Case"),
    ("sentence", "Sentence case"),
    ("alternating", "aLtErNaTiNg"),
    ("inverse", "iNVERSE"),
    ("camel", "camelCase"),
    ("pascal", "PascalCase"),
    ("snake", "snake_case"),
    ("kebab", "kebab-case"),
    ("constant", "CONSTANT_CASE"),
];
const SEPARATORS: super::Menu = &[
    ("comma", "Commas"),
    ("newline", "Lines"),
    ("space", "Spaces"),
];
const UNIQUE: &[&str] = &["Only the first of each"];

pub const TEXT: &[Tool] = &[
    tool(
        "counter",
        "Text counter",
        "Characters, words, sentences, lines, reading time and the most used words.",
        counter,
    ),
    tool(
        "case",
        "Case converter",
        "Upper, lower, title, sentence, camel, snake and more cases.",
        case,
    )
    .selects(&[("Case", CASES)]),
    tool(
        "find-replace",
        "Find and replace",
        "Replace text or a regular expression everywhere.",
        find_replace,
    )
    .fields(&[("Find", ""), ("Replace with", "")])
    .selects(&[(
        "Search for",
        &[
            ("text", "Plain text"),
            ("regex", "Regular expression ($1 for groups)"),
        ],
    )])
    .checks(&["Ignore case"]),
    tool(
        "remove-accents",
        "Remove accents",
        "Strips accents and diacritics: “déjà vu” becomes “deja vu”.",
        remove_accents,
    ),
    tool(
        "transliterate",
        "Transliterate to ASCII",
        "Turns accented, Cyrillic, Greek, kana and CJK text into ASCII: “Привет” becomes “Privet”.",
        ascii,
    ),
    tool(
        "remove-duplicate-keywords",
        "Remove duplicate keywords",
        "Keeps the first of each keyword in a list.",
        duplicate_keywords,
    )
    .selects(&[("Keywords separated by", SEPARATORS)])
    .checks(&["Ignore case"]),
    tool(
        "remove-duplicate-lines",
        "Remove duplicate lines",
        "Deletes repeated lines.",
        duplicate_lines,
    )
    .selects(&[(
        "Keep",
        &[
            ("first", "The first of each"),
            ("all", "Only lines never repeated"),
        ],
    )])
    .checks(&["Ignore case"]),
    tool(
        "remove-emojis",
        "Remove emojis",
        "Removes emoji and pictographs.",
        remove_emojis,
    ),
    tool(
        "remove-empty-lines",
        "Remove empty lines",
        "Deletes blank lines.",
        remove_empty_lines,
    ),
    tool(
        "remove-extra-spaces",
        "Remove extra spaces",
        "Collapses runs of spaces, in text or JSON.",
        remove_extra_spaces,
    )
    .selects(&[(
        "Text is",
        &[
            ("text", "Plain text"),
            ("json", "JSON (all whitespace outside strings)"),
        ],
    )]),
    tool(
        "remove-hashtags",
        "Remove hashtags",
        "Strips #hashtags from text.",
        remove_hashtags,
    ),
    tool(
        "remove-line-breaks",
        "Remove line breaks",
        "Joins lines into one.",
        remove_line_breaks,
    )
    .selects(&[(
        "Line breaks become",
        &[
            ("space", "Spaces"),
            ("nothing", "Nothing"),
            ("paragraphs", "Spaces, keeping paragraphs"),
        ],
    )]),
    tool(
        "remove-spaces",
        "Remove spaces",
        "Deletes all whitespace.",
        remove_spaces,
    ),
    tool(
        "lines-containing",
        "Remove lines containing",
        "Removes, or keeps only, the lines that contain some text.",
        lines_containing,
    )
    .fields(&[("Lines containing", "")])
    .selects(&[(
        "Action",
        &[("remove", "Remove them"), ("keep", "Keep only them")],
    )])
    .checks(&["Ignore case"]),
    tool(
        "reverse",
        "Reverse text",
        "Reverses characters, words or lines.",
        reverse,
    )
    .selects(&[(
        "Reverse",
        &[
            ("chars", "Characters"),
            ("words", "Word order"),
            ("each", "Each word"),
            ("lines", "Line order"),
        ],
    )]),
    tool(
        "sort-lines",
        "Sort lines",
        "Sorts lines alphabetically, in reverse, by length or at random.",
        sort_lines,
    )
    .selects(&[(
        "Order",
        &[
            ("asc", "A to Z"),
            ("desc", "Z to A"),
            ("length", "By length"),
            ("random", "Random"),
            ("reverse", "Reverse"),
        ],
    )])
    .checks(&["Ignore case"]),
    tool(
        "upside-down",
        "Upside-down text",
        "Flips text upside down: ɹǝʍol.",
        upside_down,
    ),
    tool(
        "char-frequency",
        "Character frequency",
        "How often each character appears.",
        char_frequency,
    )
    .checks(&["Count whitespace", "Ignore case"]),
    tool(
        "word-frequency",
        "Word frequency",
        "How often each word appears.",
        word_frequency,
    )
    .checks(&["Skip common words"]),
    tool(
        "diff",
        "Text diff",
        "Compares two texts line by line or word by word. Separate them with a line of “===”.",
        diff,
    )
    .text("Original, then a line of ===, then the changed text")
    .selects(&[("Compare", &[("lines", "Lines"), ("words", "Words")])])
    .checks(&["Only changes"]),
    tool(
        "fancy",
        "Fancy text",
        "Bold, italic, script, gothic and other Unicode letters.",
        fancy,
    )
    .selects(&[(
        "Style",
        &[
            ("bold", "𝐁𝐨𝐥𝐝"),
            ("italic", "𝐼𝑡𝑎𝑙𝑖𝑐"),
            ("bold-italic", "𝑩𝒐𝒍𝒅 𝒊𝒕𝒂𝒍𝒊𝒄"),
            ("sans", "𝗦𝗮𝗻𝘀 𝗯𝗼𝗹𝗱"),
            ("script", "𝒮𝒸𝓇𝒾𝓅𝓉"),
            ("fraktur", "𝔉𝔯𝔞𝔨𝔱𝔲𝔯"),
            ("double", "𝔻𝕠𝕦𝕓𝕝𝕖"),
            ("mono", "𝙼𝚘𝚗𝚘"),
            ("circled", "Ⓒⓘⓡⓒⓛⓔⓓ"),
            ("fullwidth", "Ｆｕｌｌ"),
            ("small", "ꜱᴍᴀʟʟ ᴄᴀᴘꜱ"),
            ("strike", "S̶t̶r̶i̶k̶e̶"),
            ("underline", "U̲n̲d̲e̲r̲"),
        ],
    )]),
    tool(
        "line-numbers",
        "Add line numbers",
        "Numbers each line.",
        line_numbers,
    )
    .fields(&[("Start at", "1"), ("Separator", ". ")])
    .checks(&["Skip empty lines"]),
    tool("lorem", "Lorem ipsum", "Placeholder Latin text.", lorem)
        .form()
        .selects(&[(
            "Generate",
            &[
                ("paragraphs", "Paragraphs"),
                ("sentences", "Sentences"),
                ("words", "Words"),
            ],
        )])
        .checks(&["Start with “Lorem ipsum”"])
        .slider("How many", 1.0, 50.0, 3.0),
    tool(
        "slug",
        "Slug generator",
        "Turns titles into URL slugs.",
        slug,
    )
    .selects(&[(
        "Separator",
        &[("dash", "Dashes"), ("underscore", "Underscores")],
    )])
    .checks(&["Keep the case"]),
];

pub const WRITING: &[Tool] = &[
    tool(
        "readability",
        "Readability score",
        "Flesch reading ease, grade levels, fog and SMOG indexes.",
        readability,
    ),
    tool(
        "syllables",
        "Syllable counter",
        "Syllables per word, and in all.",
        syllable_count,
    ),
    tool(
        "passive",
        "Passive voice finder",
        "Sentences that seem to use the passive voice.",
        passive,
    ),
    tool(
        "adverbs",
        "Adverb highlighter",
        "The -ly adverbs, and how many there are.",
        adverbs,
    ),
    tool(
        "cliches",
        "Cliché finder",
        "Common clichés and buzzwords in the text.",
        cliches,
    ),
    tool(
        "repetition",
        "Repetition finder",
        "Words and phrases used again and again.",
        repetition,
    )
    .slider("Words per phrase", 1.0, 5.0, 1.0),
    tool(
        "sentence-length",
        "Sentence length",
        "Words per sentence, flagging the long ones.",
        sentence_length,
    )
    .slider("Long after", 10.0, 50.0, 25.0),
    tool(
        "transitions",
        "Transition words",
        "How many sentences use transition words.",
        transitions,
    ),
    tool(
        "lexical-density",
        "Lexical density",
        "The share of content words, and of distinct words.",
        lexical_density,
    ),
    tool(
        "keyword-density",
        "Keyword density",
        "The most used words and phrases, as a share of the text.",
        keyword_density,
    )
    .checks(&["Skip common words"]),
    tool(
        "keywords",
        "Keyword extractor",
        "The words that best describe the text.",
        keywords,
    )
    .slider("Keywords", 1.0, 30.0, 10.0),
    tool(
        "tokens",
        "Token counter",
        "An estimate of the tokens a language model would see.",
        tokens,
    ),
    tool(
        "citation",
        "Citation formatter",
        "APA, MLA, Chicago or Harvard references.",
        citation,
    )
    .form()
    .fields(&[
        ("Authors (separated by ;)", "Ada Lovelace; Charles Babbage"),
        ("Title", "Sketch of the Analytical Engine"),
        ("Year", "1843"),
        ("Publisher or site", "Taylor's Scientific Memoirs"),
        ("URL", ""),
    ])
    .selects(&[(
        "Style",
        &[
            ("apa", "APA"),
            ("mla", "MLA"),
            ("chicago", "Chicago"),
            ("harvard", "Harvard"),
        ],
    )]),
];

pub const EXTRACT: &[Tool] = &[
    tool(
        "capitalized",
        "Extract capitalized words",
        "Words starting with a capital, or in all capitals.",
        capitalized,
    )
    .selects(&[(
        "Words",
        &[
            ("first", "Starting with a capital"),
            ("caps", "IN ALL CAPITALS"),
        ],
    )])
    .checks(UNIQUE),
    tool(
        "dates",
        "Extract dates",
        "Dates in many formats: 2024-03-05, 5/3/2024, March 5th, 2024…",
        dates,
    )
    .checks(UNIQUE),
    tool(
        "emails",
        "Extract emails",
        "Email addresses in any text, CSV, HTML or JSON.",
        emails,
    )
    .checks(UNIQUE),
    tool(
        "quotes",
        "Extract quoted text",
        "What is between quotation marks.",
        quotes,
    )
    .selects(&[(
        "Quotes",
        &[("all", "Any"), ("double", "Double"), ("single", "Single")],
    )])
    .checks(UNIQUE),
    tool(
        "urls",
        "Extract URLs",
        "Web addresses in any text, CSV, HTML or JSON.",
        urls,
    )
    .checks(UNIQUE),
    tool(
        "hashtags",
        "Extract hashtags",
        "The #hashtags in a text.",
        hashtags,
    )
    .checks(UNIQUE),
    tool(
        "ips",
        "Extract IP addresses",
        "IPv4 and IPv6 addresses.",
        ips,
    )
    .selects(&[(
        "Addresses",
        &[("all", "IPv4 and IPv6"), ("v4", "IPv4"), ("v6", "IPv6")],
    )])
    .checks(UNIQUE),
    tool(
        "hidden",
        "Remove hidden characters",
        "Zero-width, invisible and odd space characters, as AI text often has.",
        hidden,
    ),
    tool(
        "special",
        "Remove special characters",
        "Keeps only letters, digits and the characters you choose.",
        special,
    )
    .selects(&[(
        "Keep",
        &[
            ("alnum", "Letters, digits and spaces"),
            ("punctuation", "Also basic punctuation"),
            ("letters", "Letters and spaces"),
            ("ascii", "ASCII letters, digits and spaces"),
        ],
    )]),
    tool(
        "extract-numbers",
        "Extract numbers",
        "The numbers in a text.",
        extract_numbers,
    )
    .selects(&[(
        "Separate with",
        &[
            ("newline", "Lines"),
            ("comma", "Commas"),
            ("space", "Spaces"),
        ],
    )]),
    tool(
        "remove-numbers",
        "Remove numbers",
        "Deletes every digit.",
        remove_numbers,
    ),
];

#[cfg(test)]
mod tests {
    use super::super::{Opts, Rng};
    use super::*;

    fn run(f: fn(&str, &mut Opts) -> Result<Output, String>, text: &str, mode: &str) -> String {
        let mut rng = Rng(1);
        let mut o = Opts::new(mode, &mut rng);
        f(text, &mut o).unwrap().plain()
    }

    #[test]
    fn cases() {
        let t = "hello brave new-world";
        assert_eq!(run(case, t, "title"), "Hello Brave New-World");
        assert_eq!(run(case, t, "camel"), "helloBraveNewWorld");
        assert_eq!(run(case, t, "snake"), "hello_brave_new_world");
        assert_eq!(run(case, "one. two? three", "sentence"), "One. Two? Three");
    }

    #[test]
    fn extractors() {
        let t = "Write to ana@example.org or (bob.k@mail.co.uk), see https://x.org/a?b=1. #news #42 and C#";
        assert_eq!(run(emails, t, ""), "ana@example.org\nbob.k@mail.co.uk");
        assert_eq!(run(urls, t, ""), "https://x.org/a?b=1");
        assert_eq!(run(hashtags, t, ""), "#news");
        assert_eq!(
            run(
                ips,
                "from 10.0.0.1, to ::1 and fe80::1ff:fe23:4567:890a not 999.1.1.1",
                ""
            ),
            "10.0.0.1\n::1\nfe80::1ff:fe23:4567:890a"
        );
        assert_eq!(
            run(dates, "On 2024-03-05, 5/3/2024 and March 5th, 2024.", ""),
            "2024-03-05\n5/3/2024\nMarch 5th, 2024"
        );
        assert_eq!(
            run(quotes, r#"He said "hi" and 'bye', don't"#, "all"),
            "hi\nbye"
        );
    }

    #[test]
    fn cleaning() {
        assert_eq!(run(remove_accents, "Ça déjà Straße", ""), "Ca deja Strasse");
        assert_eq!(
            run(ascii, "Ça déjà, Привет, 北亰", ""),
            "Ca deja, Privet, Bei Jing"
        );
        assert_eq!(run(remove_emojis, "hi 👋🏽 there ❤️!", ""), "hi there !");
        assert_eq!(
            run(hidden, "a\u{200b}b\u{a0}c", ""),
            "ab c\n\n2 hidden characters found"
        );
        assert_eq!(run(remove_extra_spaces, "  a   b ,c ", "text"), "a b,c");
        assert_eq!(run(remove_spaces, " a b\tc\nd ", ""), "abcd");
        assert_eq!(
            run(sort_numbers, "10, -2, 3.5 and 1,000", "asc"),
            "-2\n3.5\n10\n1,000"
        );
        assert_eq!(run(upside_down, "ab", ""), "qɐ");
    }

    #[test]
    fn counts() {
        let out = run(counter, "rose rose tulip fern", "");
        assert!(
            out.lines()
                .any(|l| l.starts_with("Words:") && l.trim_end().ends_with('4'))
        );
        assert!(
            out.lines()
                .any(|l| l.starts_with("Unique words:") && l.trim_end().ends_with('3'))
        );
        assert!(
            out.lines()
                .any(|l| l.starts_with("rose ") && l.ends_with(" 2×")),
            "{out}"
        );
    }
}
