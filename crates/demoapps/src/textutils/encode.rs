//! Encoders, hashes and Roman numerals.

use super::Opts;
use super::Output;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use sha2::Digest;

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(5) {
        let mut buf = [0u8; 5];
        buf[..chunk.len()].copy_from_slice(chunk);
        let bits = buf.iter().fold(0u64, |acc, b| acc << 8 | *b as u64);
        let symbols = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            if i < symbols {
                out.push(BASE32[(bits >> (35 - i * 5) & 31) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn base32_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut bits = 0u64;
    let mut count = 0;
    let mut out = Vec::new();
    for c in text.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let value = BASE32
            .iter()
            .position(|b| *b as char == c.to_ascii_uppercase())
            .ok_or_else(|| format!("“{c}” is not a Base32 character."))?;
        bits = bits << 5 | value as u64;
        count += 5;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    Ok(out)
}

/// basE91's alphabet, by Joachim Henke.
const BASE91: &[u8; 91] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,./:;<=>?@[]^_`{|}~\"";

fn base91_encode(bytes: &[u8]) -> String {
    let (mut queue, mut nbits) = (0u32, 0u32);
    let mut out = String::new();
    for b in bytes {
        queue |= (*b as u32) << nbits;
        nbits += 8;
        if nbits > 13 {
            let mut value = queue & 8191;
            if value > 88 {
                queue >>= 13;
                nbits -= 13;
            } else {
                value = queue & 16383;
                queue >>= 14;
                nbits -= 14;
            }
            out.push(BASE91[(value % 91) as usize] as char);
            out.push(BASE91[(value / 91) as usize] as char);
        }
    }
    if nbits > 0 {
        out.push(BASE91[(queue % 91) as usize] as char);
        if nbits > 7 || queue > 90 {
            out.push(BASE91[(queue / 91) as usize] as char);
        }
    }
    out
}

fn base91_decode(text: &str) -> Result<Vec<u8>, String> {
    let (mut queue, mut nbits, mut value) = (0u32, 0u32, None::<u32>);
    let mut out = Vec::new();
    for c in text.chars().filter(|c| !c.is_whitespace()) {
        let d = BASE91
            .iter()
            .position(|b| *b as char == c)
            .ok_or_else(|| format!("“{c}” is not a basE91 character."))? as u32;
        match value.take() {
            None => value = Some(d),
            Some(v) => {
                let v = v + d * 91;
                queue |= v << nbits;
                nbits += if v & 8191 > 88 { 13 } else { 14 };
                while nbits > 7 {
                    out.push(queue as u8);
                    queue >>= 8;
                    nbits -= 8;
                }
            }
        }
    }
    if let Some(v) = value {
        out.push((queue | v << nbits) as u8);
    }
    Ok(out)
}

/// Base96's alphabet: the printable ASCII characters, with space and tab as
/// the last two symbols. The input is not trimmed, so both are significant.
const BASE96: &[u8; 96] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ \t";

/// The number of Base96 digits a block of `n` bytes needs.
const BASE96_DIGITS: [usize; 8] = [0, 2, 3, 4, 5, 7, 8, 9];

/// The number of bytes a Base96 block of `digits` digits holds.
fn base96_bytes(digits: usize) -> Option<usize> {
    match digits {
        2 => Some(1),
        3 => Some(2),
        4 => Some(3),
        5 => Some(4),
        7 => Some(5),
        8 => Some(6),
        9 => Some(7),
        _ => None,
    }
}

/// Base96: bytes as 96 printable ASCII symbols, seven bytes to nine digits.
pub fn base96_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(7) * 9);
    for chunk in bytes.chunks(7) {
        let digits = BASE96_DIGITS[chunk.len()];
        let value = chunk.iter().fold(0u128, |n, b| n << 8 | *b as u128);
        let mut symbol = [BASE96[0]; 9];
        let mut value = value;
        for i in (0..digits).rev() {
            symbol[i] = BASE96[(value % 96) as usize];
            value /= 96;
        }
        out.push_str(std::str::from_utf8(&symbol[..digits]).unwrap_or_default());
    }
    out
}

/// The bytes of Base96 text. Line breaks are ignored, but spaces and tabs are
/// part of the alphabet.
pub fn base96_decode(text: &str) -> Result<Vec<u8>, String> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|b| *b != b'\r' && *b != b'\n')
        .collect();
    let mut out = Vec::with_capacity(digits.len() * 7 / 9);
    let mut at = 0;
    while at < digits.len() {
        let take = (digits.len() - at).min(9);
        let count = base96_bytes(take)
            .ok_or_else(|| format!("A Base96 block of {take} digits is not valid."))?;
        let mut value = 0u128;
        for &b in &digits[at..at + take] {
            let d = BASE96
                .iter()
                .position(|c| *c == b)
                .ok_or_else(|| format!("“{}” is not a Base96 character.", b as char))?;
            value = value * 96 + d as u128;
        }
        if value >= 1u128 << (count * 8) {
            return Err("A Base96 block is larger than the bytes it stands for.".into());
        }
        for i in (0..count).rev() {
            out.push((value >> (i * 8)) as u8);
        }
        at += take;
    }
    Ok(out)
}

fn base96(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => utf8(base96_decode(text)?),
        _ => Ok(super::snippet(base96_encode(text.as_bytes()))),
    }
}

fn utf8(bytes: Vec<u8>) -> Result<Output, String> {
    String::from_utf8(bytes)
        .map(Output::from)
        .map_err(|_| "The decoded bytes are not UTF-8 text.".into())
}

pub fn base32(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => utf8(base32_decode(text)?),
        _ => Ok(super::snippet(base32_encode(text.as_bytes()))),
    }
}

pub fn base64(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => {
            let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            let clean = clean.replace('-', "+").replace('_', "/");
            let clean = clean.trim_end_matches('=');
            let padded = format!("{clean}{}", "=".repeat((4 - clean.len() % 4) % 4));
            utf8(
                STANDARD
                    .decode(padded)
                    .map_err(|e| format!("Not Base64: {e}."))?,
            )
        }
        _ => Ok(super::snippet(STANDARD.encode(text))),
    }
}

pub fn base91(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => utf8(base91_decode(text)?),
        _ => Ok(super::snippet(base91_encode(text.as_bytes()))),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Every digest the hash tools offer, by name.
pub fn digests(b: &[u8]) -> Vec<(&'static str, String)> {
    vec![
        ("MD5", format!("{:x}", md5::compute(b))),
        ("SHA-1", hex(&sha1::Sha1::digest(b))),
        ("SHA-224", hex(&sha2::Sha224::digest(b))),
        ("SHA-256", hex(&sha2::Sha256::digest(b))),
        ("SHA-384", hex(&sha2::Sha384::digest(b))),
        ("SHA-512", hex(&sha2::Sha512::digest(b))),
        ("SHA3-256", hex(&sha3::Sha3_256::digest(b))),
        ("SHA3-384", hex(&sha3::Sha3_384::digest(b))),
        ("SHA3-512", hex(&sha3::Sha3_512::digest(b))),
        ("Keccak-256", hex(&sha3::Keccak256::digest(b))),
        ("BLAKE2b-512", hex(&blake2::Blake2b512::digest(b))),
        ("BLAKE2s-256", hex(&blake2::Blake2s256::digest(b))),
        ("RIPEMD-160", hex(&ripemd::Ripemd160::digest(b))),
        ("CRC-32", format!("{:08x}", crc32fast::hash(b))),
    ]
}

pub fn hash(text: &str, o: &mut Opts) -> Result<Output, String> {
    let all = digests(text.as_bytes());
    let upper = |s: &String| if o.flag { s.to_uppercase() } else { s.clone() };
    Ok(match all.iter().find(|(name, _)| *name == o.mode) {
        Some((name, digest)) => super::table(&[(name, upper(digest))]),
        None => super::table(&all.iter().map(|(n, d)| (*n, upper(d))).collect::<Vec<_>>()),
    })
}

pub fn rot13(text: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(super::snippet(
        text.chars()
            .map(|c| match c {
                'a'..='z' => ((c as u8 - b'a' + 13) % 26 + b'a') as char,
                'A'..='Z' => ((c as u8 - b'A' + 13) % 26 + b'A') as char,
                _ => c,
            })
            .collect(),
    ))
}

pub fn url(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => {
            let bytes = text.as_bytes();
            let mut out = Vec::new();
            let mut i = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'%' => {
                        let byte = text
                            .get(i + 1..i + 3)
                            .and_then(|h| u8::from_str_radix(h, 16).ok())
                            .ok_or("A “%” must be followed by two hex digits.")?;
                        out.push(byte);
                        i += 3;
                    }
                    b'+' => {
                        out.push(b' ');
                        i += 1;
                    }
                    b => {
                        out.push(b);
                        i += 1;
                    }
                }
            }
            utf8(out)
        }
        _ => Ok(super::snippet(
            text.bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        (b as char).to_string()
                    }
                    _ => format!("%{b:02X}"),
                })
                .collect(),
        )),
    }
}

const ROMAN: [(u32, &str); 13] = [
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

fn to_roman(mut n: u32) -> String {
    let mut out = String::new();
    for (value, symbol) in ROMAN {
        while n >= value {
            out.push_str(symbol);
            n -= value;
        }
    }
    out
}

fn from_roman(s: &str) -> Option<u32> {
    let upper = s.to_ascii_uppercase();
    let mut rest = upper.as_str();
    let mut n = 0;
    for (value, symbol) in ROMAN {
        while let Some(r) = rest.strip_prefix(symbol) {
            n += value;
            rest = r;
        }
    }
    // Only the canonical spelling: "IIII" or "VX" do not count.
    (rest.is_empty() && n > 0 && to_roman(n) == upper).then_some(n)
}

/// Applies `f` to each non-empty line, keeping the empty ones.
fn per_line(text: &str, f: impl Fn(&str) -> Result<String, String>) -> Result<Output, String> {
    text.lines()
        .map(|l| {
            let l = l.trim();
            if l.is_empty() {
                Ok(String::new())
            } else {
                f(l)
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.join("\n").into())
}

pub fn decimal_to_roman(text: &str, _: &mut Opts) -> Result<Output, String> {
    per_line(text, |l| match l.parse::<u32>() {
        Ok(n @ 1..=3999) => Ok(to_roman(n)),
        _ => Err(format!("“{l}” is not a number from 1 to 3999.")),
    })
}

pub fn roman_to_decimal(text: &str, _: &mut Opts) -> Result<Output, String> {
    per_line(text, |l| {
        from_roman(l)
            .map(|n| n.to_string())
            .ok_or_else(|| format!("“{l}” is not a Roman numeral."))
    })
}

// More encoders

fn ascii85_encode(bytes: &[u8]) -> String {
    let mut out = String::from("<~");
    for chunk in bytes.chunks(4) {
        let mut buf = [0u8; 4];
        buf[..chunk.len()].copy_from_slice(chunk);
        let n = u32::from_be_bytes(buf);
        if n == 0 && chunk.len() == 4 {
            out.push('z');
            continue;
        }
        let mut digits = [0u8; 5];
        let mut v = n;
        for d in digits.iter_mut().rev() {
            *d = (v % 85) as u8 + b'!';
            v /= 85;
        }
        out.extend(digits[..chunk.len() + 1].iter().map(|d| *d as char));
    }
    out.push_str("~>");
    out
}

fn ascii85_decode(text: &str) -> Result<Vec<u8>, String> {
    let t = text.trim();
    let t = t.strip_prefix("<~").unwrap_or(t);
    let t = t.strip_suffix("~>").unwrap_or(t);
    let mut out = Vec::new();
    let mut group: Vec<u32> = Vec::new();
    for c in t.chars().filter(|c| !c.is_whitespace()) {
        if c == 'z' && group.is_empty() {
            out.extend([0; 4]);
            continue;
        }
        if !('!'..='u').contains(&c) {
            return Err(format!("“{c}” is not an Ascii85 character."));
        }
        group.push(c as u32 - 33);
        if group.len() == 5 {
            let n = group
                .iter()
                .try_fold(0u32, |a, d| a.checked_mul(85)?.checked_add(*d))
                .ok_or("An Ascii85 group is too large.")?;
            out.extend(n.to_be_bytes());
            group.clear();
        }
    }
    if !group.is_empty() {
        let len = group.len();
        group.resize(5, 84);
        let n = group.iter().fold(0u64, |a, d| a * 85 + *d as u64) as u32;
        out.extend(&n.to_be_bytes()[..len - 1]);
    }
    Ok(out)
}

fn ascii85(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => utf8(ascii85_decode(text)?),
        _ => Ok(super::snippet(ascii85_encode(text.as_bytes()))),
    }
}

/// Bytes from hex digits, with or without `0x` and spaces.
pub fn unhex(s: &str) -> Result<Vec<u8>, String> {
    let t: String = s
        .trim()
        .trim_start_matches("0x")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-')
        .collect();
    if t.len() % 2 == 1 {
        return Err("Hex needs an even number of digits.".into());
    }
    (0..t.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&t[i..i + 2], 16)
                .map_err(|_| format!("“{}” is not hex.", &t[i..i + 2]))
        })
        .collect()
}

fn hex_tool(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => utf8(unhex(text)?),
        _ => {
            let h: Vec<String> = text
                .bytes()
                .map(|b| {
                    if o.check(1) {
                        format!("{b:02X}")
                    } else {
                        format!("{b:02x}")
                    }
                })
                .collect();
            Ok(super::snippet(h.join(if o.flag { " " } else { "" })))
        }
    }
}

fn binary(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => {
            let bits: String = text.chars().filter(|c| *c == '0' || *c == '1').collect();
            if bits.len() % 8 != 0 {
                return Err("Binary text needs groups of eight bits.".into());
            }
            utf8(
                (0..bits.len())
                    .step_by(8)
                    .map(|i| u8::from_str_radix(&bits[i..i + 8], 2).unwrap())
                    .collect(),
            )
        }
        _ => Ok(super::snippet(
            text.bytes()
                .map(|b| format!("{b:08b}"))
                .collect::<Vec<_>>()
                .join(" "),
        )),
    }
}

fn string_escape(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "unescape" => {
            let t = text.trim();
            let quoted = if t.starts_with('"') && t.ends_with('"') && t.len() > 1 {
                t.to_owned()
            } else {
                format!("\"{}\"", t.replace('"', "\\\""))
            };
            // JavaScript escapes that JSON lacks.
            let quoted = quoted.replace("\\'", "'").replace("\\0", "\\u0000");
            let v: String = hypereffect::serde_json::from_str(&quoted)
                .map_err(|e| format!("Not an escaped string: {e}."))?;
            Ok(super::snippet(v))
        }
        _ => {
            let s = hypereffect::serde_json::to_string(text).unwrap();
            Ok(super::snippet(if o.flag {
                s
            } else {
                s[1..s.len() - 1].to_owned()
            }))
        }
    }
}

const MORSE: &[(char, &str)] = &[
    ('A', ".-"),
    ('B', "-..."),
    ('C', "-.-."),
    ('D', "-.."),
    ('E', "."),
    ('F', "..-."),
    ('G', "--."),
    ('H', "...."),
    ('I', ".."),
    ('J', ".---"),
    ('K', "-.-"),
    ('L', ".-.."),
    ('M', "--"),
    ('N', "-."),
    ('O', "---"),
    ('P', ".--."),
    ('Q', "--.-"),
    ('R', ".-."),
    ('S', "..."),
    ('T', "-"),
    ('U', "..-"),
    ('V', "...-"),
    ('W', ".--"),
    ('X', "-..-"),
    ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('?', "..--.."),
    ('\'', ".----."),
    ('!', "-.-.--"),
    ('/', "-..-."),
    ('(', "-.--."),
    (')', "-.--.-"),
    ('&', ".-..."),
    (':', "---..."),
    (';', "-.-.-."),
    ('=', "-...-"),
    ('+', ".-.-."),
    ('-', "-....-"),
    ('_', "..--.-"),
    ('"', ".-..-."),
    ('$', "...-..-"),
    ('@', ".--.-."),
];

fn morse(text: &str, o: &mut Opts) -> Result<Output, String> {
    match o.mode {
        "decode" => Ok(super::snippet(
            text.trim()
                .split(['/', '|'])
                .map(|w| {
                    w.split_whitespace()
                        .map(|code| {
                            let code = code.replace(['•', '·'], ".").replace(['–', '—', '_'], "-");
                            MORSE.iter().find(|m| m.1 == code).map_or('�', |m| m.0)
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join(" "),
        )),
        _ => {
            let plain: String = super::text::unaccented(text);
            Ok(super::snippet(
                plain
                    .split_whitespace()
                    .map(|w| {
                        w.chars()
                            .filter_map(|c| {
                                MORSE
                                    .iter()
                                    .find(|m| m.0 == c.to_ascii_uppercase())
                                    .map(|m| m.1)
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(" / "),
            ))
        }
    }
}

// Punycode (RFC 3492)

const BASE: u32 = 36;
const TMIN: u32 = 1;
const TMAX: u32 = 26;

fn adapt(mut delta: u32, points: u32, first: bool) -> u32 {
    delta = if first { delta / 700 } else { delta / 2 };
    delta += delta / points;
    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + (BASE - TMIN + 1) * delta / (delta + 38)
}

fn digit(d: u32) -> char {
    (if d < 26 {
        b'a' + d as u8
    } else {
        b'0' + (d - 26) as u8
    }) as char
}

fn puny_encode(input: &str) -> Option<String> {
    let chars: Vec<u32> = input.chars().map(|c| c as u32).collect();
    let mut out: String = input.chars().filter(char::is_ascii).collect();
    let basic = out.len() as u32;
    let mut h = basic;
    if basic > 0 {
        out.push('-');
    }
    let (mut n, mut delta, mut bias) = (128u32, 0u32, 72u32);
    while (h as usize) < chars.len() {
        let m = *chars.iter().filter(|c| **c >= n).min()?;
        delta = delta.checked_add((m - n).checked_mul(h + 1)?)?;
        n = m;
        for &c in &chars {
            if c < n {
                delta = delta.checked_add(1)?;
            }
            if c == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        TMIN
                    } else if k >= bias + TMAX {
                        TMAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    out.push(digit(t + (q - t) % (BASE - t)));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                out.push(digit(q));
                bias = adapt(delta, h + 1, h == basic);
                delta = 0;
                h += 1;
            }
        }
        delta += 1;
        n += 1;
    }
    Some(out)
}

fn puny_decode(input: &str) -> Option<String> {
    let (basic, rest) = match input.rfind('-') {
        Some(i) => (&input[..i], &input[i + 1..]),
        None => ("", input),
    };
    let mut out: Vec<u32> = basic.chars().map(|c| c as u32).collect();
    let (mut n, mut i, mut bias) = (128u32, 0u32, 72u32);
    let mut it = rest.chars().peekable();
    while it.peek().is_some() {
        let old = i;
        let mut w = 1u32;
        let mut k = BASE;
        loop {
            let c = it.next()?;
            let d = match c {
                'a'..='z' => c as u32 - 'a' as u32,
                'A'..='Z' => c as u32 - 'A' as u32,
                '0'..='9' => c as u32 - '0' as u32 + 26,
                _ => return None,
            };
            i = i.checked_add(d.checked_mul(w)?)?;
            let t = if k <= bias {
                TMIN
            } else if k >= bias + TMAX {
                TMAX
            } else {
                k - bias
            };
            if d < t {
                break;
            }
            w = w.checked_mul(BASE - t)?;
            k += BASE;
        }
        let len = out.len() as u32 + 1;
        bias = adapt(i - old, len, old == 0);
        n = n.checked_add(i / len)?;
        i %= len;
        out.insert(i as usize, n);
        i += 1;
    }
    out.into_iter().map(char::from_u32).collect()
}

fn punycode(text: &str, o: &mut Opts) -> Result<Output, String> {
    let decode = o.mode == "decode";
    Ok(super::snippet(
        text.lines()
            .map(|line| {
                line.trim()
                    .split('.')
                    .map(|label| {
                        if decode {
                            match label.strip_prefix("xn--") {
                                Some(p) => puny_decode(p).unwrap_or_else(|| label.to_owned()),
                                None => label.to_owned(),
                            }
                        } else if label.is_ascii() {
                            label.to_owned()
                        } else {
                            puny_encode(&label.to_lowercase())
                                .map_or_else(|| label.to_owned(), |p| format!("xn--{p}"))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

fn unicode(text: &str, o: &mut Opts) -> Result<Output, String> {
    let non_ascii = o.flag;
    let keep = |c: char| non_ascii && c.is_ascii();
    Ok(super::snippet(match o.mode {
        "decode" => {
            let re = regex_lite::Regex::new(r"\\u\{([0-9a-fA-F]+)\}|\\u([0-9a-fA-F]{4})|\\U([0-9a-fA-F]{8})|U\+([0-9a-fA-F]{4,6})|&#x([0-9a-fA-F]+);|&#([0-9]+);").unwrap();
            let decoded = re.replace_all(text, |c: &regex_lite::Captures| {
                let (hex, dec) = (
                    c.get(1).or(c.get(2)).or(c.get(3)).or(c.get(4)).or(c.get(5)),
                    c.get(6),
                );
                let n = match (hex, dec) {
                    (Some(h), _) => u32::from_str_radix(h.as_str(), 16).ok(),
                    (_, Some(d)) => d.as_str().parse().ok(),
                    _ => None,
                };
                n.and_then(char::from_u32)
                    .map_or(c[0].to_owned(), |c| c.to_string())
            });
            decoded.into_owned()
        }
        "js" => text
            .encode_utf16()
            .map(|u| match char::from_u32(u as u32) {
                Some(c) if keep(c) => c.to_string(),
                _ => format!("\\u{u:04x}"),
            })
            .collect(),
        "es6" => text
            .chars()
            .map(|c| {
                if keep(c) {
                    c.to_string()
                } else {
                    format!("\\u{{{:x}}}", c as u32)
                }
            })
            .collect(),
        "html" => text
            .chars()
            .map(|c| {
                if keep(c) {
                    c.to_string()
                } else {
                    format!("&#x{:X};", c as u32)
                }
            })
            .collect(),
        "css" => text
            .chars()
            .map(|c| {
                if keep(c) {
                    c.to_string()
                } else {
                    format!("\\{:X} ", c as u32)
                }
            })
            .collect(),
        _ => text
            .chars()
            .map(|c| format!("U+{:04X}", c as u32))
            .collect::<Vec<_>>()
            .join(" "),
    }))
}

fn cipher(text: &str, o: &mut Opts) -> Result<Output, String> {
    let decode = o.choice(1) == "decode";
    let shift_char = |c: char, k: i32| -> char {
        let (base, n) = match c {
            'a'..='z' => (b'a', 26),
            'A'..='Z' => (b'A', 26),
            _ => return c,
        };
        let k = if decode { -k } else { k };
        (((c as u8 - base) as i32 + k).rem_euclid(n) as u8 + base) as char
    };
    Ok(super::snippet(match o.mode {
        "vigenere" => {
            let key: Vec<i32> = o
                .first
                .chars()
                .filter(char::is_ascii_alphabetic)
                .map(|c| (c.to_ascii_lowercase() as u8 - b'a') as i32)
                .collect();
            if key.is_empty() {
                return Err("Type a key of letters.".into());
            }
            let mut i = 0;
            text.chars()
                .map(|c| {
                    if c.is_ascii_alphabetic() {
                        let r = shift_char(c, key[i % key.len()]);
                        i += 1;
                        r
                    } else {
                        c
                    }
                })
                .collect()
        }
        "atbash" => text
            .chars()
            .map(|c| match c {
                'a'..='z' => (b'z' - (c as u8 - b'a')) as char,
                'A'..='Z' => (b'Z' - (c as u8 - b'A')) as char,
                c => c,
            })
            .collect(),
        "rot47" => text
            .chars()
            .map(|c| {
                if ('!'..='~').contains(&c) {
                    (((c as u8 - 33 + 47) % 94) + 33) as char
                } else {
                    c
                }
            })
            .collect(),
        "a1z26" if decode => text
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|n| {
                n.parse::<u8>()
                    .ok()
                    .filter(|n| (1..=26).contains(n))
                    .map_or('?', |n| (b'a' + n - 1) as char)
            })
            .collect(),
        "a1z26" => text
            .split_whitespace()
            .map(|w| {
                w.chars()
                    .filter(char::is_ascii_alphabetic)
                    .map(|c| (c.to_ascii_lowercase() as u8 - b'a' + 1).to_string())
                    .collect::<Vec<_>>()
                    .join("-")
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => text
            .chars()
            .map(|c| shift_char(c, o.number as i32))
            .collect(),
    }))
}

/// HMAC over a hash with `block`-byte blocks.
fn hmac(key: &[u8], msg: &[u8], block: usize, h: fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let mut k = if key.len() > block {
        h(key)
    } else {
        key.to_vec()
    };
    k.resize(block, 0);
    let inner: Vec<u8> = k
        .iter()
        .map(|b| b ^ 0x36)
        .chain(msg.iter().copied())
        .collect();
    let outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).chain(h(&inner)).collect();
    h(&outer)
}

fn hmac_tool(text: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.first.as_bytes();
    let (block, h): (usize, fn(&[u8]) -> Vec<u8>) = match o.mode {
        "SHA-1" => (64, |b| sha1::Sha1::digest(b).to_vec()),
        "SHA-512" => (128, |b| sha2::Sha512::digest(b).to_vec()),
        "SHA-384" => (128, |b| sha2::Sha384::digest(b).to_vec()),
        "MD5" => (64, |b| md5::compute(b).0.to_vec()),
        _ => (64, |b| sha2::Sha256::digest(b).to_vec()),
    };
    let mac = hmac(key, text.as_bytes(), block, h);
    Ok(super::table(&[(
        &format!(
            "HMAC-{}",
            if o.mode.is_empty() { "SHA-256" } else { o.mode }
        ),
        match o.choice(1) {
            "base64" => STANDARD.encode(&mac),
            _ => hex(&mac),
        },
    )]))
}

fn hash_compare(_: &str, o: &mut Opts) -> Result<Output, String> {
    let clean = |s: &str| s.trim().trim_start_matches("0x").to_lowercase();
    let (a, b) = (clean(o.field(0)), clean(o.field(1)));
    if a.is_empty() || b.is_empty() {
        return Err("Fill in both hashes.".into());
    }
    let kind = |h: &str| match (h.chars().all(|c| c.is_ascii_hexdigit()), h.len()) {
        (true, 8) => "CRC-32",
        (true, 32) => "MD5 (or another 128-bit hash)",
        (true, 40) => "SHA-1 or RIPEMD-160",
        (true, 56) => "SHA-224 or SHA3-224",
        (true, 64) => "SHA-256, SHA3-256, Keccak-256 or BLAKE2s",
        (true, 96) => "SHA-384 or SHA3-384",
        (true, 128) => "SHA-512, SHA3-512 or BLAKE2b",
        _ => "not a hex digest of a known length",
    };
    let same = a == b;
    let first_diff = a
        .chars()
        .zip(b.chars())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    Ok(verdict(
        same,
        if same {
            "The hashes are the same.".into()
        } else {
            format!("They differ, from character {}.", first_diff + 1)
        },
    )
    .facts(&[
        ("First looks like", kind(&a).into()),
        ("Second looks like", kind(&b).into()),
    ]))
}

// Blockchain

fn keccak(b: &[u8]) -> [u8; 32] {
    sha3::Keccak256::digest(b).into()
}

/// A decimal amount of `decimals` decimals as an integer of the smallest unit.
fn to_base(amount: &str, decimals: u32) -> Result<u128, String> {
    let t = amount.trim().replace([',', '_', ' '], "");
    let (whole, frac) = t.split_once('.').unwrap_or((&t, ""));
    if whole.is_empty() && frac.is_empty()
        || !whole
            .chars()
            .chain(frac.chars())
            .all(|c| c.is_ascii_digit())
    {
        return Err(format!("“{amount}” is not a positive amount."));
    }
    if frac.trim_end_matches('0').len() > decimals as usize {
        return Err(format!("“{amount}” has more than {decimals} decimals."));
    }
    let frac = format!("{frac:0<width$}", width = decimals as usize);
    let digits = format!("{whole}{}", &frac[..decimals as usize]);
    digits
        .trim_start_matches('0')
        .parse::<u128>()
        .or(if digits.trim_matches('0').is_empty() {
            Ok(0)
        } else {
            Err("The amount is too large.".to_string())
        })
}

fn from_base(n: u128, decimals: u32) -> String {
    let s = format!("{n:0>width$}", width = decimals as usize + 1);
    let (whole, frac) = s.split_at(s.len() - decimals as usize);
    let frac = frac.trim_end_matches('0');
    let whole = crate::common::group(whole);
    if frac.is_empty() {
        whole
    } else {
        format!("{whole}.{frac}")
    }
}

const ETH_UNITS: &[(&str, u32)] = &[
    ("wei", 0),
    ("kwei", 3),
    ("mwei", 6),
    ("gwei", 9),
    ("szabo", 12),
    ("finney", 15),
    ("ether", 18),
];

fn wei(_: &str, o: &mut Opts) -> Result<Output, String> {
    let from = ETH_UNITS
        .iter()
        .find(|u| u.0 == o.mode)
        .unwrap_or(&ETH_UNITS[6])
        .1;
    let base = to_base(o.field(0), from)?;
    Ok(super::table(
        &ETH_UNITS
            .iter()
            .map(|(n, d)| (*n, from_base(base, *d)))
            .collect::<Vec<_>>(),
    ))
}

fn erc20(_: &str, o: &mut Opts) -> Result<Output, String> {
    let decimals = o.int(1)?;
    if !(0..=36).contains(&decimals) {
        return Err("Decimals go from 0 to 36.".into());
    }
    let d = decimals as u32;
    Ok(if o.mode == "base" {
        let base: u128 = o
            .field(0)
            .replace([',', '_'], "")
            .parse()
            .map_err(|_| "Base units are a whole number.".to_string())?;
        super::table(&[("Tokens", from_base(base, d))])
    } else {
        super::table(&[("Base units", to_base(o.field(0), d)?.to_string())])
    })
}

fn selector(_: &str, o: &mut Opts) -> Result<Output, String> {
    let sig: String = o.field(0).chars().filter(|c| !c.is_whitespace()).collect();
    if !sig.contains('(') || !sig.ends_with(')') {
        return Err("Write a signature such as transfer(address,uint256).".into());
    }
    let h = keccak(sig.as_bytes());
    Ok(super::table(&[
        ("Signature", sig),
        ("Function selector", format!("0x{}", hex(&h[..4]))),
        ("Event topic", format!("0x{}", hex(&h))),
    ]))
}

fn namehash(_: &str, o: &mut Opts) -> Result<Output, String> {
    let name = o.field(0).to_lowercase();
    let mut node = [0u8; 32];
    if !name.is_empty() {
        for label in name.rsplit('.') {
            let mut buf = node.to_vec();
            buf.extend(keccak(label.as_bytes()));
            node = keccak(&buf);
        }
    }
    let first = name.split('.').next().unwrap_or("");
    Ok(super::table(&[
        ("Name", name.clone()),
        ("Namehash", format!("0x{}", hex(&node))),
        (
            "Label hash",
            format!("0x{}", hex(&keccak(first.as_bytes()))),
        ),
    ]))
}

fn checksum_address(addr: &[u8]) -> String {
    let lower = hex(addr);
    let h = hex(&keccak(lower.as_bytes()));
    let mixed: String = lower
        .chars()
        .zip(h.chars())
        .map(|(c, d)| {
            if c.is_ascii_alphabetic() && d.to_digit(16).unwrap() >= 8 {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    format!("0x{mixed}")
}

fn address(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a = o.field(0);
    let body = a.strip_prefix("0x").ok_or("An address starts with 0x.")?;
    if body.len() != 40 || !body.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(verdict(
            false,
            format!(
                "An address is 0x and 40 hex digits; this has {}.",
                body.len()
            ),
        ));
    }
    let good = checksum_address(&unhex(body)?);
    let mixed = body.chars().any(|c| c.is_ascii_lowercase())
        && body.chars().any(|c| c.is_ascii_uppercase());
    let (ok, why) = if !mixed {
        (true, "Valid, without a checksum (all one case).")
    } else if good == a {
        (true, "Valid, with a correct EIP-55 checksum.")
    } else {
        (
            false,
            "The EIP-55 checksum is wrong: a letter's case is off, or the address has a typo.",
        )
    };
    Ok(verdict(ok, why).facts(&[("Checksummed", good)]))
}

fn create2(_: &str, o: &mut Opts) -> Result<Output, String> {
    let deployer = unhex(o.field(0))?;
    let salt = unhex(o.field(1))?;
    let code = o.field(2);
    if deployer.len() != 20 {
        return Err("The deployer is a 20-byte address.".into());
    }
    if salt.len() > 32 {
        return Err("The salt is at most 32 bytes.".into());
    }
    let mut salt32 = vec![0u8; 32 - salt.len()];
    salt32.extend(salt);
    let code_hash = if o.mode == "code" {
        keccak(&unhex(code)?)
    } else {
        let h = unhex(code)?;
        h.try_into()
            .map_err(|_| "The init code hash is 32 bytes.".to_string())?
    };
    let mut buf = vec![0xff];
    buf.extend(&deployer);
    buf.extend(&salt32);
    buf.extend(code_hash);
    Ok(super::table(&[
        ("Address", checksum_address(&keccak(&buf)[12..])),
        ("Salt", format!("0x{}", hex(&salt32))),
        ("Init code hash", format!("0x{}", hex(&code_hash))),
    ]))
}

fn calldata(_: &str, o: &mut Opts) -> Result<Output, String> {
    let data = unhex(o.field(0))?;
    let zeros = data.iter().filter(|b| **b == 0).count();
    let gas = zeros * 4 + (data.len() - zeros) * 16;
    let price = o.opt_num(1)?.unwrap_or(0.0);
    let mut rows = vec![
        (
            "Bytes",
            format!(
                "{} ({zeros} zero, {} non-zero)",
                data.len(),
                data.len() - zeros
            ),
        ),
        ("Calldata gas", super::fmt(gas as f64, 0)),
        ("With the 21,000 base", super::fmt(gas as f64 + 21000.0, 0)),
    ];
    if price > 0.0 {
        rows.push((
            "Calldata cost",
            format!("{} ETH", super::fmt(gas as f64 * price * 1e-9, 9)),
        ));
    }
    Ok(super::table(&rows))
}

fn gas(_: &str, o: &mut Opts) -> Result<Output, String> {
    let units = o.positive(0)?;
    let base = o.num(1)?;
    let tip = o.opt_num(2)?.unwrap_or(0.0);
    let eth = units * (base + tip) * 1e-9;
    let mut rows = vec![
        ("Fee", format!("{} ETH", super::fmt(eth, 9))),
        ("Gwei", super::fmt(units * (base + tip), 2)),
    ];
    if let Some(p) = o.opt_num(3)? {
        rows.push(("In money", super::fmt(eth * p, 2)));
    }
    Ok(super::table(&rows))
}

fn vanity(_: &str, o: &mut Opts) -> Result<Output, String> {
    let pattern = o.field(0).trim_start_matches("0x");
    if !pattern.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("A vanity pattern is hex digits.".into());
    }
    let speed = o.positive(1)?;
    let letters = pattern.chars().filter(char::is_ascii_alphabetic).count() as i32;
    let mut tries = 16f64.powi(pattern.len() as i32);
    if o.flag {
        tries *= 2f64.powi(letters);
    }
    let human = |s: f64| super::dates::duration_words(s);
    Ok(super::table(&[
        ("Difficulty", format!("1 in {}", super::fmt(tries, 0))),
        (
            "50% chance after",
            format!(
                "{} tries, {}",
                super::fmt(tries * std::f64::consts::LN_2, 0),
                human(tries * std::f64::consts::LN_2 / speed)
            ),
        ),
        ("90% chance after", human(tries * 10f64.ln() / speed)),
    ]))
}

fn merkle(text: &str, o: &mut Opts) -> Result<Output, String> {
    let h: fn(&[u8]) -> Vec<u8> = if o.mode == "sha256" {
        |b| sha2::Sha256::digest(b).to_vec()
    } else {
        |b| keccak(b).to_vec()
    };
    let leaves: Vec<Vec<u8>> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            if o.check(1) {
                unhex(l)
            } else {
                Ok(h(l.as_bytes()))
            }
        })
        .collect::<Result<_, _>>()?;
    if leaves.is_empty() {
        return Err("Write one leaf per line.".into());
    }
    let mut levels = vec![leaves];
    while levels.last().unwrap().len() > 1 {
        let next: Vec<Vec<u8>> = levels
            .last()
            .unwrap()
            .chunks(2)
            .map(|p| match p {
                [a, b] => {
                    let (x, y) = if o.flag && b < a { (b, a) } else { (a, b) };
                    h(&[x.as_slice(), y.as_slice()].concat())
                }
                [a] => a.clone(),
                _ => unreachable!(),
            })
            .collect();
        levels.push(next);
    }
    let root = hex(&levels.last().unwrap()[0]);
    let rows = levels.iter().enumerate().rev().flat_map(|(i, level)| {
        level
            .iter()
            .map(move |n| vec![format!("Level {i}"), format!("0x{}", hex(n))])
    });
    Ok(super::table(&[("Root", format!("0x{root}"))]).table(&["", "Hash"], rows))
}

fn block_estimate(_: &str, o: &mut Opts) -> Result<Output, String> {
    let current = o.num(0)?;
    let secs = o.positive(2)?;
    let now = o.now()? / 1000.0;
    let target = o.field(1);
    if let Some(block) = super::parse_number(target).filter(|n| *n < 1e10) {
        let at = now + (block - current) * secs;
        Ok(super::table(&[
            ("Block", super::fmt(block, 0)),
            ("Expected", super::dates::format_unix(at)),
            ("In", super::dates::duration_words((at - now).abs())),
        ]))
    } else {
        let at = super::dates::parse_datetime(target)
            .ok_or("The target is a block number or a date such as 2030-01-01 12:00.")?;
        let block = current + (at - now) / secs;
        Ok(super::table(&[
            ("At", super::dates::format_unix(at)),
            ("Expected block", super::fmt(block.round(), 0)),
        ]))
    }
}

// The menus

use super::{Tool, tool, verdict};

const CODEC: super::Menu = &[("encode", "Encode"), ("decode", "Decode")];

pub const ENCODE: &[Tool] = &[
    tool(
        "base64",
        "Base64",
        "Encodes text as Base64, or decodes it (URL-safe too).",
        base64,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "base32",
        "Base32",
        "Encodes text as Base32 (RFC 4648), or decodes it.",
        base32,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "base91",
        "Base91",
        "Encodes text as basE91, or decodes it.",
        base91,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "base96",
        "Base96",
        "Encodes bytes as Base96 printable ASCII, or decodes it.",
        base96,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "ascii85",
        "Ascii85 / Base85",
        "Encodes text as Ascii85, or decodes it.",
        ascii85,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "hex",
        "Hex",
        "Text as hexadecimal bytes (UTF-8), or back.",
        hex_tool,
    )
    .selects(&[("Direction", CODEC)])
    .checks(&["Spaces between bytes", "Capital letters"]),
    tool("binary", "Binary", "Text as bits (UTF-8), or back.", binary)
        .selects(&[("Direction", CODEC)]),
    tool(
        "url",
        "URL encoder",
        "Percent-encodes text for URLs, or decodes it.",
        url,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "html-escape",
        "HTML escape",
        "Replaces <, >, &, quotes with HTML entities, or back.",
        super::markup::html_escape,
    )
    .selects(&[(
        "Direction",
        &[("escape", "Escape"), ("unescape", "Unescape")],
    )]),
    tool(
        "xml-escape",
        "XML escape",
        "Replaces the five XML special characters with entities, or back.",
        super::markup::xml_escape,
    )
    .selects(&[(
        "Direction",
        &[("escape", "Escape"), ("unescape", "Unescape")],
    )]),
    tool(
        "string-escape",
        "String escape",
        "Escapes text for a JSON or JavaScript string, or unescapes \\n, \\t, \\u…",
        string_escape,
    )
    .selects(&[(
        "Direction",
        &[("escape", "Escape"), ("unescape", "Unescape")],
    )])
    .checks(&["Add quotes"]),
    tool(
        "unicode",
        "Unicode escapes",
        "Code points, or \\u, \\u{…}, &#x; and CSS escapes, and back.",
        unicode,
    )
    .selects(&[(
        "Write as",
        &[
            ("points", "U+0041 code points"),
            ("js", "\\u0041 (JavaScript, JSON)"),
            ("es6", "\\u{41} (ES6, Rust)"),
            ("html", "&#x41; (HTML)"),
            ("css", "\\41 (CSS)"),
            ("decode", "Decode any of them"),
        ],
    )])
    .checks(&["Keep ASCII as it is"]),
    tool(
        "punycode",
        "Punycode",
        "International domain names as xn-- ASCII, or back.",
        punycode,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "morse",
        "Morse code",
        "Text as Morse code, or back; words separated by /.",
        morse,
    )
    .selects(&[("Direction", CODEC)]),
    tool(
        "rot13",
        "ROT13",
        "Shifts letters by 13; running it twice gives the text back.",
        rot13,
    ),
    tool(
        "cipher",
        "Text cipher",
        "Caesar, Vigenère, Atbash, ROT47 and A1Z26 ciphers.",
        cipher,
    )
    .fields(&[("Key (Vigenère)", "lemon")])
    .selects(&[
        (
            "Cipher",
            &[
                ("caesar", "Caesar"),
                ("vigenere", "Vigenère"),
                ("atbash", "Atbash"),
                ("rot47", "ROT47"),
                ("a1z26", "A1Z26 (letters as numbers)"),
            ],
        ),
        ("Direction", CODEC),
    ])
    .slider("Caesar shift", 1.0, 25.0, 3.0),
    tool(
        "hmac",
        "HMAC generator",
        "A keyed hash (HMAC) of the text.",
        hmac_tool,
    )
    .fields(&[("Secret key", "secret")])
    .selects(&[
        (
            "Hash",
            &[
                ("SHA-256", "SHA-256"),
                ("SHA-512", "SHA-512"),
                ("SHA-384", "SHA-384"),
                ("SHA-1", "SHA-1"),
                ("MD5", "MD5"),
            ],
        ),
        ("Output", &[("hex", "Hex"), ("base64", "Base64")]),
    ]),
];

pub const HASH: &[Tool] = &[
    tool(
        "hash",
        "Hash calculator",
        "MD5, SHA-1, SHA-2, SHA-3, Keccak, BLAKE2, RIPEMD-160 and CRC-32 of the text.",
        hash,
    )
    .selects(&[(
        "Algorithm",
        &[
            ("all", "All of them"),
            ("MD5", "MD5"),
            ("SHA-1", "SHA-1"),
            ("SHA-224", "SHA-224"),
            ("SHA-256", "SHA-256"),
            ("SHA-384", "SHA-384"),
            ("SHA-512", "SHA-512"),
            ("SHA3-256", "SHA3-256"),
            ("SHA3-384", "SHA3-384"),
            ("SHA3-512", "SHA3-512"),
            ("Keccak-256", "Keccak-256"),
            ("BLAKE2b-512", "BLAKE2b-512"),
            ("BLAKE2s-256", "BLAKE2s-256"),
            ("RIPEMD-160", "RIPEMD-160"),
            ("CRC-32", "CRC-32"),
        ],
    )])
    .checks(&["Capital letters"]),
    tool(
        "hash-compare",
        "Hash comparison",
        "Whether two hashes are the same, and what kind they look like.",
        hash_compare,
    )
    .form()
    .fields(&[
        ("First hash", "5d41402abc4b2a76b9719d911017c592"),
        ("Second hash", "5D41402ABC4B2A76B9719D911017C592"),
    ]),
];

pub const BLOCKCHAIN: &[Tool] = &[
    tool(
        "wei",
        "Wei, Gwei and Ether converter",
        "An amount of ether in every unit.",
        wei,
    )
    .form()
    .fields(&[("Amount", "1.5")])
    .selects(&[(
        "Unit",
        &[
            ("ether", "Ether"),
            ("gwei", "Gwei"),
            ("wei", "Wei"),
            ("finney", "Finney"),
            ("szabo", "Szabo"),
            ("mwei", "Mwei"),
            ("kwei", "Kwei"),
        ],
    )]),
    tool(
        "erc20",
        "ERC-20 unit converter",
        "Token amounts to base units and back, for any number of decimals.",
        erc20,
    )
    .form()
    .fields(&[("Amount", "12.5"), ("Decimals", "6")])
    .selects(&[("From", &[("tokens", "Tokens"), ("base", "Base units")])]),
    tool(
        "selector",
        "Function selector",
        "The 4-byte selector and event topic of a Solidity signature.",
        selector,
    )
    .form()
    .fields(&[("Signature", "transfer(address,uint256)")]),
    tool(
        "namehash",
        "ENS namehash",
        "The ENS namehash and label hash of a name.",
        namehash,
    )
    .form()
    .fields(&[("Name", "vitalik.eth")]),
    tool(
        "address",
        "Wallet address validator",
        "Checks an Ethereum address and its EIP-55 checksum.",
        address,
    )
    .form()
    .fields(&[("Address", "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed")]),
    tool(
        "create2",
        "CREATE2 address",
        "The address a contract gets from CREATE2.",
        create2,
    )
    .form()
    .fields(&[
        ("Deployer", "0x0000000000000000000000000000000000000000"),
        ("Salt (hex)", "0x00"),
        ("Init code, or its hash", "0x00"),
    ])
    .selects(&[(
        "Third field is",
        &[("code", "Init code"), ("hash", "Init code hash")],
    )]),
    tool(
        "calldata",
        "Calldata cost",
        "The gas that transaction data costs: 4 a zero byte, 16 the others.",
        calldata,
    )
    .form()
    .fields(&[
        ("Calldata (hex)", "0xa9059cbb000000000000000000000000ab"),
        ("Gas price (gwei)", "20"),
    ]),
    tool(
        "gas",
        "Gas fee estimator",
        "A transaction's fee from its gas and the gas price.",
        gas,
    )
    .form()
    .fields(&[
        ("Gas used", "21000"),
        ("Base fee (gwei)", "20"),
        ("Priority fee (gwei)", "1"),
        ("ETH price (optional)", ""),
    ]),
    tool(
        "vanity",
        "Vanity address estimator",
        "How long finding an address with a given prefix takes.",
        vanity,
    )
    .form()
    .fields(&[("Prefix (hex)", "c0ffee"), ("Keys per second", "1000000")])
    .checks(&["Case-sensitive (checksum)"]),
    tool(
        "merkle",
        "Merkle tree",
        "The Merkle root and levels of a list of leaves.",
        merkle,
    )
    .text("One leaf per line")
    .selects(&[("Hash", &[("keccak", "Keccak-256"), ("sha256", "SHA-256")])])
    .checks(&["Sort pairs (OpenZeppelin)", "Leaves are hex hashes already"]),
    tool(
        "block-estimate",
        "Block estimator",
        "When a block comes, or which block a date falls on.",
        block_estimate,
    )
    .form()
    .fields(&[
        ("Current block", "21000000"),
        ("Target block or date", "21100000"),
        ("Seconds per block", "12"),
    ]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_round_trip() {
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI======");
        assert_eq!(base32_encode(b"f"), "MY======");
        assert_eq!(base32_decode("MZXW6YTBOI======").unwrap(), b"foobar");
    }

    #[test]
    fn base91_round_trip() {
        assert_eq!(base91_encode(b"Hello, World!"), ">OwJh>}AQ;r@@Y?F");
        let text = "Ünïcödé and some longer text, 0123456789";
        assert_eq!(
            base91_decode(&base91_encode(text.as_bytes())).unwrap(),
            text.as_bytes()
        );
    }

    #[test]
    fn base96_round_trip() {
        let all: Vec<u8> = (0..=255).collect();
        for bytes in [
            b"".as_slice(),
            b"\0".as_slice(),
            b"\0\0\0\x01hello".as_slice(),
            b"Hello, World! \t".as_slice(),
            &[0xff; 7],
            &[0xff; 8],
            &all,
        ] {
            let text = base96_encode(bytes);
            assert_eq!(base96_decode(&text).unwrap(), bytes, "{text:?}");
        }
        assert!(base96_decode("_").is_err());
    }

    #[test]
    fn roman() {
        assert_eq!(to_roman(1994), "MCMXCIV");
        assert_eq!(from_roman("mcmxciv"), Some(1994));
        assert_eq!(from_roman("IIII"), None);
        assert_eq!(from_roman("VX"), None);
    }
}
