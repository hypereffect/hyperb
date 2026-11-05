//! Calculators, math, number systems and statistics.

use super::Output;
use super::calc::numbers;
use super::{Opts, Tool, fmt, grid, table, tool, verdict};

// Expressions

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Postfix(char, Box<Expr>),
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Result<Expr, String> {
        let mut l = self.term()?;
        while let Some(c @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            l = Expr::Bin(c as char, Box::new(l), Box::new(self.term()?));
        }
        Ok(l)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut l = self.unary()?;
        loop {
            match self.peek() {
                Some(c @ (b'*' | b'/' | b'%')) => {
                    if c == b'%' && self.percent_ends() {
                        self.i += 1;
                        l = Expr::Postfix('%', Box::new(l));
                        continue;
                    }
                    self.i += 1;
                    l = Expr::Bin(c as char, Box::new(l), Box::new(self.unary()?));
                }
                Some(b'(' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.') => {
                    // Implicit multiplication: 2pi, 3(4), (1+i)(1-i).
                    l = Expr::Bin('*', Box::new(l), Box::new(self.power()?));
                }
                _ => return Ok(l),
            }
        }
    }

    /// Whether the `%` here is a percent sign: nothing that could be an
    /// operand follows it.
    fn percent_ends(&self) -> bool {
        let mut j = self.i + 1;
        while self.s.get(j).is_some_and(|c| c.is_ascii_whitespace()) {
            j += 1;
        }
        !matches!(
            self.s.get(j),
            Some(b'0'..=b'9' | b'.' | b'(' | b'a'..=b'z' | b'A'..=b'Z')
        )
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(b'+') => {
                self.i += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Expr, String> {
        let base = self.postfix()?;
        if self.peek() == Some(b'^') || self.s[self.i..].starts_with(b"**") {
            self.i += if self.s[self.i] == b'^' { 1 } else { 2 };
            return Ok(Expr::Bin('^', Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.atom()?;
        while self.peek() == Some(b'!') {
            self.i += 1;
            e = Expr::Postfix('!', Box::new(e));
        }
        Ok(e)
    }

    fn atom(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(b'(') => {
                self.i += 1;
                let e = self.expr()?;
                if self.peek() != Some(b')') {
                    return Err("A “(” is not closed.".into());
                }
                self.i += 1;
                Ok(e)
            }
            Some(b'0'..=b'9' | b'.') => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| c.is_ascii_digit() || *c == b'.' || *c == b'_')
                {
                    self.i += 1;
                }
                // An exponent, but not the start of a name such as `e`.
                if matches!(self.s.get(self.i), Some(b'e' | b'E'))
                    && self.s.get(self.i + 1).is_some_and(|c| {
                        c.is_ascii_digit()
                            || ((*c == b'-' || *c == b'+')
                                && self.s.get(self.i + 2).is_some_and(u8::is_ascii_digit))
                    })
                {
                    self.i += 2;
                    while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                        self.i += 1;
                    }
                }
                let t = std::str::from_utf8(&self.s[start..self.i])
                    .unwrap()
                    .replace('_', "");
                t.parse()
                    .map(Expr::Num)
                    .map_err(|_| format!("“{t}” is not a number."))
            }
            Some(b'a'..=b'z' | b'A'..=b'Z') => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                {
                    self.i += 1;
                }
                let name = std::str::from_utf8(&self.s[start..self.i])
                    .unwrap()
                    .to_owned();
                if self.peek() == Some(b'(') {
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(b')') {
                        loop {
                            args.push(self.expr()?);
                            match self.peek() {
                                Some(b',' | b';') => self.i += 1,
                                _ => break,
                            }
                        }
                    }
                    if self.peek() != Some(b')') {
                        return Err(format!("“{name}(” is not closed."));
                    }
                    self.i += 1;
                    return Ok(Expr::Call(name.to_lowercase(), args));
                }
                Ok(Expr::Var(name))
            }
            Some(c) => Err(format!("Unexpected “{}”.", c as char)),
            None => Err("The expression ends too soon.".into()),
        }
    }
}

fn parse_expr(s: &str) -> Result<Expr, String> {
    let s = s
        .replace(['×', '·'], "*")
        .replace('÷', "/")
        .replace('−', "-")
        .replace('π', "pi")
        .replace('√', "sqrt");
    let mut p = Parser {
        s: s.as_bytes(),
        i: 0,
    };
    let e = p.expr()?;
    if p.peek().is_some() {
        return Err(format!("Unexpected “{}”.", &s[p.i..]));
    }
    Ok(e)
}

/// Numbers an expression can be evaluated in.
trait Num: Sized + Clone {
    fn from_f64(v: f64) -> Result<Self, String>;
    fn var(name: &str, vars: &[(String, Self)]) -> Result<Self, String>;
    fn bin(op: char, a: Self, b: Self) -> Result<Self, String>;
    fn neg(a: Self) -> Result<Self, String>;
    fn call(name: &str, args: Vec<Self>, degrees: bool) -> Result<Self, String>;
    fn postfix(op: char, a: Self) -> Result<Self, String>;
}

fn eval<N: Num>(e: &Expr, vars: &[(String, N)], degrees: bool) -> Result<N, String> {
    match e {
        Expr::Num(v) => N::from_f64(*v),
        Expr::Var(name) => N::var(name, vars),
        Expr::Neg(a) => N::neg(eval(a, vars, degrees)?),
        Expr::Bin(op, a, b) => N::bin(*op, eval(a, vars, degrees)?, eval(b, vars, degrees)?),
        Expr::Postfix(op, a) => N::postfix(*op, eval(a, vars, degrees)?),
        Expr::Call(name, args) => {
            let args = args
                .iter()
                .map(|a| eval(a, vars, degrees))
                .collect::<Result<Vec<_>, _>>()?;
            N::call(name, args, degrees)
        }
    }
}

fn gamma(x: f64) -> f64 {
    if x < 0.5 {
        return std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x));
    }
    ln_gamma(x).exp()
}

/// The log of the gamma function, by Lanczos.
pub fn ln_gamma(x: f64) -> f64 {
    const G: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.5203681218851,
        -1259.1392167224028,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507343278686905,
        -0.13857109526572012,
        9.984_369_578_019_572e-6,
        1.5056327351493116e-7,
    ];
    let x = x - 1.0;
    let t = x + 7.5;
    let mut a = G[0];
    for (i, g) in G.iter().enumerate().skip(1) {
        a += g / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

fn arity(name: &str, args: &[impl Sized], n: usize) -> Result<(), String> {
    if args.len() == n {
        Ok(())
    } else {
        Err(format!(
            "{name}() takes {n} argument{}.",
            if n == 1 { "" } else { "s" }
        ))
    }
}

impl Num for f64 {
    fn from_f64(v: f64) -> Result<f64, String> {
        Ok(v)
    }

    fn var(name: &str, vars: &[(String, f64)]) -> Result<f64, String> {
        if let Some((_, v)) = vars.iter().rev().find(|(n, _)| n == name) {
            return Ok(*v);
        }
        match name {
            "pi" | "PI" => Ok(std::f64::consts::PI),
            "e" | "E" => Ok(std::f64::consts::E),
            "tau" => Ok(std::f64::consts::TAU),
            "phi" => Ok(1.618_033_988_749_895),
            "inf" | "infinity" => Ok(f64::INFINITY),
            _ => Err(format!("“{name}” is not defined.")),
        }
    }

    fn bin(op: char, a: f64, b: f64) -> Result<f64, String> {
        Ok(match op {
            '+' => a + b,
            '-' => a - b,
            '*' => a * b,
            '/' if b == 0.0 => return Err("Division by zero.".into()),
            '/' => a / b,
            '%' => a.rem_euclid(b),
            _ => a.powf(b),
        })
    }

    fn neg(a: f64) -> Result<f64, String> {
        Ok(-a)
    }

    fn postfix(op: char, a: f64) -> Result<f64, String> {
        Ok(if op == '%' {
            a / 100.0
        } else {
            gamma(a + 1.0).round_if_whole(a)
        })
    }

    fn call(name: &str, a: Vec<f64>, degrees: bool) -> Result<f64, String> {
        let to = |x: f64| if degrees { x.to_radians() } else { x };
        let from = |x: f64| if degrees { x.to_degrees() } else { x };
        if matches!(
            name,
            "min" | "max" | "sum" | "avg" | "mean" | "gcd" | "lcm" | "hypot"
        ) {
            if a.is_empty() {
                return Err(format!("{name}() needs numbers."));
            }
            return Ok(match name {
                "min" => a.iter().copied().fold(f64::INFINITY, f64::min),
                "max" => a.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                "sum" => a.iter().sum(),
                "hypot" => a.iter().map(|x| x * x).sum::<f64>().sqrt(),
                "gcd" => a.iter().map(|x| x.abs().round() as u128).fold(0, gcd) as f64,
                "lcm" => a
                    .iter()
                    .map(|x| x.abs().round() as u128)
                    .fold(1, |l, x| if x == 0 { 0 } else { l / gcd(l, x) * x })
                    as f64,
                _ => a.iter().sum::<f64>() / a.len() as f64,
            });
        }
        if matches!(
            name,
            "log" | "pow" | "root" | "atan2" | "ncr" | "npr" | "choose" | "mod" | "round"
        ) && a.len() == 2
        {
            let (x, y) = (a[0], a[1]);
            return Ok(match name {
                "log" => x.ln() / y.ln(),
                "pow" => x.powf(y),
                "root" => x.powf(1.0 / y),
                "atan2" => from(x.atan2(y)),
                "mod" => x.rem_euclid(y),
                "round" => (x * 10f64.powf(y)).round() / 10f64.powf(y),
                "npr" => (gamma(x + 1.0) / gamma(x - y + 1.0)).round(),
                _ => (gamma(x + 1.0) / (gamma(y + 1.0) * gamma(x - y + 1.0))).round(),
            });
        }
        arity(name, &a, 1)?;
        let x = a[0];
        Ok(match name {
            "sqrt" => x.sqrt(),
            "cbrt" => x.cbrt(),
            "abs" => x.abs(),
            "sin" => to(x).sin(),
            "cos" => to(x).cos(),
            "tan" => to(x).tan(),
            "asin" | "arcsin" => from(x.asin()),
            "acos" | "arccos" => from(x.acos()),
            "atan" | "arctan" => from(x.atan()),
            "sinh" => x.sinh(),
            "cosh" => x.cosh(),
            "tanh" => x.tanh(),
            "asinh" => x.asinh(),
            "acosh" => x.acosh(),
            "atanh" => x.atanh(),
            "sec" => 1.0 / to(x).cos(),
            "csc" => 1.0 / to(x).sin(),
            "cot" => 1.0 / to(x).tan(),
            "ln" => x.ln(),
            "log" | "log10" | "lg" => x.log10(),
            "log2" | "lb" => x.log2(),
            "exp" => x.exp(),
            "floor" => x.floor(),
            "ceil" => x.ceil(),
            "round" => x.round(),
            "trunc" => x.trunc(),
            "sign" | "sgn" => x.signum(),
            "fact" | "factorial" => gamma(x + 1.0).round_if_whole(x),
            "gamma" => gamma(x),
            "deg" => x.to_degrees(),
            "rad" => x.to_radians(),
            _ => return Err(format!("There is no function “{name}”.")),
        })
    }
}

trait RoundWhole {
    fn round_if_whole(self, of: f64) -> f64;
}

impl RoundWhole for f64 {
    fn round_if_whole(self, of: f64) -> f64 {
        if of.fract() == 0.0 {
            self.round()
        } else {
            self
        }
    }
}

pub fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Evaluates each line; `name = expression` defines a variable and `ans` is
/// the previous result.
fn calculator(text: &str, o: &mut Opts) -> Result<Output, String> {
    let degrees = o.mode == "deg";
    let mut vars: Vec<(String, f64)> = Vec::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        if l.is_empty() {
            continue;
        }
        let (name, expr) = match l.split_once('=') {
            Some((n, e))
                if n.trim().chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !n.trim().is_empty() =>
            {
                (Some(n.trim()), e)
            }
            _ => (None, l),
        };
        let v = eval(&parse_expr(expr)?, &vars, degrees).map_err(|e| format!("{e} (in “{l}”)"))?;
        vars.push(("ans".into(), v));
        if let Some(n) = name {
            vars.push((n.to_owned(), v));
        }
        let shown = if v.is_nan() {
            "not a number".into()
        } else {
            fmt(v, o.number.max(1))
        };
        out.push((l.to_owned(), shown));
    }
    Ok(table(
        &out.iter()
            .map(|(l, v)| (l.as_str(), v.clone()))
            .collect::<Vec<_>>(),
    ))
}

// Exact fractions

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ratio(i128, i128);

impl Ratio {
    fn new(n: i128, d: i128) -> Result<Ratio, String> {
        if d == 0 {
            return Err("Division by zero.".into());
        }
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        let s = if d < 0 { -1 } else { 1 };
        Ok(Ratio(s * n / g.max(1), s * d / g.max(1)))
    }
}

fn big() -> String {
    "The numbers grow too large for exact fractions.".into()
}

impl Num for Ratio {
    fn from_f64(v: f64) -> Result<Ratio, String> {
        // Decimals as written: 0.25 is 1/4.
        let s = format!("{v}");
        let (w, f) = s.split_once('.').unwrap_or((&s, ""));
        let d = 10i128.checked_pow(f.len() as u32).ok_or_else(big)?;
        let n: i128 = format!("{w}{f}").parse().map_err(|_| big())?;
        Ratio::new(n, d)
    }

    fn var(name: &str, _: &[(String, Ratio)]) -> Result<Ratio, String> {
        Err(format!("“{name}” cannot be an exact fraction."))
    }

    fn bin(op: char, a: Ratio, b: Ratio) -> Result<Ratio, String> {
        let m = |x: i128, y: i128| x.checked_mul(y).ok_or_else(big);
        match op {
            '+' => Ratio::new(
                m(a.0, b.1)?.checked_add(m(b.0, a.1)?).ok_or_else(big)?,
                m(a.1, b.1)?,
            ),
            '-' => Ratio::new(
                m(a.0, b.1)?.checked_sub(m(b.0, a.1)?).ok_or_else(big)?,
                m(a.1, b.1)?,
            ),
            '*' => Ratio::new(m(a.0, b.0)?, m(a.1, b.1)?),
            '/' => Ratio::new(m(a.0, b.1)?, m(a.1, b.0)?),
            '^' if b.1 == 1 && b.0.abs() < 128 => {
                let e = b.0.unsigned_abs() as u32;
                let (n, d) = (
                    a.0.checked_pow(e).ok_or_else(big)?,
                    a.1.checked_pow(e).ok_or_else(big)?,
                );
                if b.0 < 0 {
                    Ratio::new(d, n)
                } else {
                    Ratio::new(n, d)
                }
            }
            _ => Err("Exact fractions take + − × ÷ and whole powers only.".into()),
        }
    }

    fn neg(a: Ratio) -> Result<Ratio, String> {
        Ok(Ratio(-a.0, a.1))
    }

    fn call(name: &str, _: Vec<Ratio>, _: bool) -> Result<Ratio, String> {
        Err(format!("{name}() has no exact fraction."))
    }

    fn postfix(op: char, a: Ratio) -> Result<Ratio, String> {
        match op {
            '%' => Ratio::new(a.0, a.1.checked_mul(100).ok_or_else(big)?),
            _ => Err("Exact fractions have no factorials.".into()),
        }
    }
}

fn mixed(r: Ratio) -> String {
    if r.1 == 1 || r.0.abs() < r.1 {
        return String::new();
    }
    format!(
        "{}{} {}/{}",
        if r.0 < 0 { "-" } else { "" },
        r.0.abs() / r.1,
        r.0.abs() % r.1,
        r.1
    )
}

fn fractions(_: &str, o: &mut Opts) -> Result<Output, String> {
    let r: Ratio = eval(&parse_expr(o.field(0))?, &[], false)?;
    let mut rows = vec![(
        "Result",
        if r.1 == 1 {
            r.0.to_string()
        } else {
            format!("{}/{}", r.0, r.1)
        },
    )];
    if !mixed(r).is_empty() {
        rows.push(("Mixed number", mixed(r)));
    }
    rows.push(("Decimal", fmt(r.0 as f64 / r.1 as f64, 10)));
    Ok(table(&rows))
}

/// The simplest fraction within `eps` of `x`, by continued fractions.
fn to_fraction(x: f64, eps: f64) -> (i128, i128) {
    let (mut h0, mut h1, mut k0, mut k1) = (0i128, 1i128, 1i128, 0i128);
    let mut v = x;
    for _ in 0..64 {
        let a = v.floor() as i128;
        let (h2, k2) = (a * h1 + h0, a * k1 + k0);
        (h0, h1, k0, k1) = (h1, h2, k1, k2);
        if (x - h1 as f64 / k1 as f64).abs() < eps || (v - a as f64).abs() < 1e-12 {
            break;
        }
        v = 1.0 / (v - a as f64);
    }
    (h1, k1)
}

fn simplify(_: &str, o: &mut Opts) -> Result<Output, String> {
    let s = o.field(0);
    if let Some((n, d)) = s.split_once('/') {
        let (n, d) = (
            n.trim()
                .parse::<i128>()
                .map_err(|_| "Write a fraction such as 42/56.")?,
            d.trim()
                .parse::<i128>()
                .map_err(|_| "Write a fraction such as 42/56.")?,
        );
        let r = Ratio::new(n, d)?;
        let g = gcd(n.unsigned_abs(), d.unsigned_abs());
        let mut rows = vec![
            ("Simplified", format!("{}/{}", r.0, r.1)),
            ("Divided by", g.to_string()),
            ("Decimal", fmt(n as f64 / d as f64, 10)),
        ];
        if !mixed(r).is_empty() {
            rows.push(("Mixed number", mixed(r)));
        }
        rows.push((
            "Percent",
            format!("{}%", fmt(n as f64 / d as f64 * 100.0, 6)),
        ));
        return Ok(table(&rows));
    }
    let x = super::parse_number(s).ok_or("Write a fraction such as 42/56, or a decimal.")?;
    let (n, d) = to_fraction(x, 1e-9);
    Ok(table(&[
        ("Fraction", format!("{n}/{d}")),
        ("Mixed number", mixed(Ratio(n, d))),
    ]))
}

// Complex numbers

#[derive(Clone, Copy, Debug, PartialEq)]
struct C(f64, f64);

impl C {
    fn mul(self, b: C) -> C {
        C(self.0 * b.0 - self.1 * b.1, self.0 * b.1 + self.1 * b.0)
    }
    fn div(self, b: C) -> Result<C, String> {
        let d = b.0 * b.0 + b.1 * b.1;
        if d == 0.0 {
            return Err("Division by zero.".into());
        }
        Ok(C(
            (self.0 * b.0 + self.1 * b.1) / d,
            (self.1 * b.0 - self.0 * b.1) / d,
        ))
    }
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
    fn arg(self) -> f64 {
        self.1.atan2(self.0)
    }
    fn ln(self) -> C {
        C(self.abs().ln(), self.arg())
    }
    fn exp(self) -> C {
        let r = self.0.exp();
        C(r * self.1.cos(), r * self.1.sin())
    }
    fn pow(self, b: C) -> C {
        if self == C(0.0, 0.0) {
            return C(0.0, 0.0);
        }
        self.ln().mul(b).exp()
    }
}

impl Num for C {
    fn from_f64(v: f64) -> Result<C, String> {
        Ok(C(v, 0.0))
    }
    fn var(name: &str, _: &[(String, C)]) -> Result<C, String> {
        match name {
            "i" | "j" => Ok(C(0.0, 1.0)),
            "pi" => Ok(C(std::f64::consts::PI, 0.0)),
            "e" => Ok(C(std::f64::consts::E, 0.0)),
            _ => Err(format!("“{name}” is not defined.")),
        }
    }
    fn bin(op: char, a: C, b: C) -> Result<C, String> {
        Ok(match op {
            '+' => C(a.0 + b.0, a.1 + b.1),
            '-' => C(a.0 - b.0, a.1 - b.1),
            '*' => a.mul(b),
            '/' => a.div(b)?,
            '^' => a.pow(b),
            _ => return Err("Complex numbers have no remainder.".into()),
        })
    }
    fn neg(a: C) -> Result<C, String> {
        Ok(C(-a.0, -a.1))
    }
    fn postfix(_: char, _: C) -> Result<C, String> {
        Err("Complex numbers have no factorial or percent.".into())
    }
    fn call(name: &str, a: Vec<C>, _: bool) -> Result<C, String> {
        arity(name, &a, 1)?;
        let z = a[0];
        let i = C(0.0, 1.0);
        Ok(match name {
            "abs" | "mod" => C(z.abs(), 0.0),
            "arg" | "angle" => C(z.arg(), 0.0),
            "conj" => C(z.0, -z.1),
            "re" | "real" => C(z.0, 0.0),
            "im" | "imag" => C(z.1, 0.0),
            "sqrt" => z.pow(C(0.5, 0.0)),
            "exp" => z.exp(),
            "ln" | "log" => z.ln(),
            "sin" => C(z.0.sin() * z.1.cosh(), z.0.cos() * z.1.sinh()),
            "cos" => C(z.0.cos() * z.1.cosh(), -z.0.sin() * z.1.sinh()),
            "tan" => C(z.0.sin() * z.1.cosh(), z.0.cos() * z.1.sinh())
                .div(C(z.0.cos() * z.1.cosh(), -z.0.sin() * z.1.sinh()))?,
            "cis" => i.mul(z).exp(),
            _ => return Err(format!("There is no complex function “{name}”.")),
        })
    }
}

fn show_complex(z: C) -> String {
    let clean = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v };
    let (a, b) = (clean(z.0), clean(z.1));
    match (a, b) {
        (a, 0.0) => fmt(a, 10),
        (0.0, b) => format!("{}i", fmt(b, 10)),
        (a, b) => format!(
            "{} {} {}i",
            fmt(a, 10),
            if b < 0.0 { "−" } else { "+" },
            fmt(b.abs(), 10)
        ),
    }
}

fn complex(_: &str, o: &mut Opts) -> Result<Output, String> {
    let z: C = eval(&parse_expr(o.field(0))?, &[], false)?;
    Ok(table(&[
        ("Result", show_complex(z)),
        ("Modulus", fmt(z.abs(), 10)),
        (
            "Argument",
            format!(
                "{} rad ({}°)",
                fmt(z.arg(), 10),
                fmt(z.arg().to_degrees(), 6)
            ),
        ),
        (
            "Polar",
            format!("{} ∠ {}°", fmt(z.abs(), 6), fmt(z.arg().to_degrees(), 4)),
        ),
        ("Conjugate", show_complex(C(z.0, -z.1))),
    ]))
}

// Calculators

fn percentage(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (a, b) = (o.num(0)?, o.num(1)?);
    let (label, value) = match o.mode {
        "what-percent" => (
            format!("{} of {}", fmt(a, 6), fmt(b, 6)),
            format!("{}%", fmt(a / b * 100.0, 6)),
        ),
        "change" => (
            format!(
                "From {} to {}, a {}",
                fmt(a, 6),
                fmt(b, 6),
                if b >= a { "rise" } else { "fall" }
            ),
            format!("{}%", fmt(((b - a) / a * 100.0).abs(), 6)),
        ),
        "whole" => (
            format!("{} is {}% of", fmt(a, 6), fmt(b, 6)),
            fmt(a / b * 100.0, 6),
        ),
        "difference" => (
            format!("Difference between {} and {}", fmt(a, 6), fmt(b, 6)),
            format!("{}%", fmt((a - b).abs() / ((a + b) / 2.0) * 100.0, 6)),
        ),
        _ => (
            format!("{}% of {}", fmt(a, 6), fmt(b, 6)),
            fmt(a / 100.0 * b, 6),
        ),
    };
    Ok(table(&[(&label, value)]))
}

fn average(text: &str, o: &mut Opts) -> Result<Output, String> {
    if o.mode == "weighted" {
        let mut sum = 0.0;
        let mut weights = 0.0;
        for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let v = numbers(l)?;
            let [x, w] = v[..] else {
                return Err(format!("“{l}” needs a value and a weight."));
            };
            sum += x * w;
            weights += w;
        }
        return Ok(table(&[
            ("Weighted average", fmt(sum / weights, 6)),
            ("Total weight", fmt(weights, 6)),
        ]));
    }
    let v = numbers(text)?;
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let mut rows = vec![
        ("Mean", fmt(mean, 6)),
        ("Median", fmt(median(&v), 6)),
        ("Mode", mode(&v)),
    ];
    if v.iter().all(|x| *x > 0.0) {
        rows.push((
            "Geometric mean",
            fmt((v.iter().map(|x| x.ln()).sum::<f64>() / n).exp(), 6),
        ));
        rows.push((
            "Harmonic mean",
            fmt(n / v.iter().map(|x| 1.0 / x).sum::<f64>(), 6),
        ));
    }
    rows.push(("Count", fmt(n, 0)));
    rows.push(("Sum", fmt(v.iter().sum(), 6)));
    Ok(table(&rows))
}

const WEIGHT_UNITS: &[(&str, f64)] = &[
    ("mg", 0.001),
    ("g", 1.0),
    ("kg", 1000.0),
    ("oz", 28.349523),
    ("lb", 453.59237),
    ("lbs", 453.59237),
];
const VOLUME_UNITS: &[(&str, f64)] = &[
    ("ml", 0.001),
    ("cl", 0.01),
    ("dl", 0.1),
    ("l", 1.0),
    ("floz", 0.0295735),
    ("fl oz", 0.0295735),
    ("pt", 0.473176),
    ("pint", 0.473176),
    ("qt", 0.946353),
    ("gal", 3.78541),
    ("cup", 0.236588),
    ("tbsp", 0.0147868),
    ("tsp", 0.00492892),
    ("m3", 1000.0),
];

/// A package size such as `500 g`, `1.5kg`, `12 oz`, `2 L` or `6`.
fn package(s: &str, kind: &str) -> Result<f64, String> {
    let t = s.trim().to_lowercase().replace(',', ".");
    let split = t.find(|c: char| c.is_alphabetic()).unwrap_or(t.len());
    let n: f64 = t[..split]
        .trim()
        .parse()
        .map_err(|_| format!("“{s}” is not a size such as 500 g."))?;
    let unit = t[split..].trim();
    let table: &[(&str, f64)] = match kind {
        "volume" => VOLUME_UNITS,
        "weight" => WEIGHT_UNITS,
        _ => return Ok(n),
    };
    if unit.is_empty() {
        return Ok(n * table.iter().find(|u| u.1 == 1.0).map_or(1.0, |u| u.1));
    }
    let unit = unit.trim_end_matches('.');
    table
        .iter()
        .find(|u| u.0 == unit || (unit.len() > 2 && unit.trim_end_matches('s') == u.0))
        .map(|u| n * u.1)
        .ok_or_else(|| {
            format!(
                "“{unit}” is not a unit of {kind}; use {}.",
                table.iter().map(|u| u.0).collect::<Vec<_>>().join(", ")
            )
        })
}

fn unit_price(text: &str, o: &mut Opts) -> Result<Output, String> {
    let kind = o.mode;
    let sym = o.field(0);
    let (per, label) = match kind {
        "volume" => (1.0, "per litre"),
        "count" => (1.0, "per item"),
        _ => (100.0, "per 100 g"),
    };
    let mut items = Vec::new();
    for (n, l) in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        let parts: Vec<&str> = l.split([',', ';', '\t', '|']).map(str::trim).collect();
        let (name, price, size) = match parts.as_slice() {
            [name, price, size, ..] => (name.to_string(), *price, *size),
            [price, size] => (format!("Item {}", n + 1), *price, *size),
            _ => {
                return Err(format!(
                    "“{l}” needs a name, a price and a size, separated by commas."
                ));
            }
        };
        let price =
            super::parse_number(price).ok_or_else(|| format!("“{price}” is not a price."))?;
        let amount = package(size, kind)?;
        if amount <= 0.0 {
            return Err(format!("“{size}” is not a size."));
        }
        items.push((name, price, size.to_owned(), price / amount * per));
    }
    if items.is_empty() {
        return Err("Write one item per line: name, price, size.".into());
    }
    items.sort_by(|a, b| a.3.total_cmp(&b.3));
    let best = items[0].3;
    let mut out = Vec::new();
    for (i, (name, price, size, unit)) in items.iter().enumerate() {
        let tag = if i == 0 {
            "Best value".to_owned()
        } else if best > 0.0 {
            format!("+{}%", fmt((unit / best - 1.0) * 100.0, 1))
        } else {
            String::new()
        };
        out.push(vec![
            name.clone(),
            format!("{sym}{} for {size}", fmt(*price, 2)),
            format!("{sym}{} {label}", fmt(*unit, 3)),
            tag,
        ]);
    }
    let mut result = grid(&["", "Price", "Unit price", "Compared"], out);
    if items.len() > 1 {
        let worst = items.last().unwrap();
        result = result.note(format!(
            "The best value saves {}% over “{}”.",
            fmt((1.0 - best / worst.3) * 100.0, 1),
            worst.0
        ));
    }
    Ok(result)
}

// Math

fn poly_roots(coef: &[f64]) -> Vec<C> {
    // Durand–Kerner on the monic polynomial.
    let n = coef.len() - 1;
    let a: Vec<f64> = coef.iter().map(|c| c / coef[0]).collect();
    let eval = |z: C| {
        a.iter().fold(C(0.0, 0.0), |acc, c| {
            C::bin('+', acc.mul(z), C(*c, 0.0)).unwrap()
        })
    };
    let mut roots: Vec<C> = (0..n).map(|k| C(0.4, 0.9).pow(C(k as f64, 0.0))).collect();
    for _ in 0..500 {
        let mut moved = 0.0f64;
        for i in 0..n {
            let mut d = C(1.0, 0.0);
            for j in 0..n {
                if i != j {
                    d = d.mul(C(roots[i].0 - roots[j].0, roots[i].1 - roots[j].1));
                }
            }
            let step = eval(roots[i]).div(d).unwrap_or(C(0.0, 0.0));
            roots[i] = C(roots[i].0 - step.0, roots[i].1 - step.1);
            moved = moved.max(step.abs());
        }
        if moved < 1e-14 {
            break;
        }
    }
    roots
}

fn equation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mut coef = numbers(&o.field(0).replace(',', " "))?;
    while coef.first() == Some(&0.0) {
        coef.remove(0);
    }
    if coef.len() < 2 {
        return Err(
            "Write the coefficients from the highest power down, such as 1, -3, 2 for x² − 3x + 2."
                .into(),
        );
    }
    if coef.len() > 13 {
        return Err("At most degree 12.".into());
    }
    let n = coef.len() - 1;
    let mut terms = Vec::new();
    for (i, c) in coef.iter().enumerate() {
        let p = n - i;
        if *c == 0.0 {
            continue;
        }
        let x = match p {
            0 => String::new(),
            1 => "x".into(),
            p => format!("x^{p}"),
        };
        let c_abs = if c.abs() == 1.0 && p > 0 {
            String::new()
        } else {
            fmt(c.abs(), 6)
        };
        terms.push(format!(
            "{}{c_abs}{x}",
            if *c < 0.0 {
                "− "
            } else if terms.is_empty() {
                ""
            } else {
                "+ "
            }
        ));
    }
    let mut out = vec![("Equation".to_owned(), format!("{} = 0", terms.join(" ")))];
    if n == 2 {
        let (a, b, c) = (coef[0], coef[1], coef[2]);
        out.push(("Discriminant".into(), fmt(b * b - 4.0 * a * c, 6)));
        out.push((
            "Vertex".into(),
            format!(
                "({}, {})",
                fmt(-b / (2.0 * a), 6),
                fmt(c - b * b / (4.0 * a), 6)
            ),
        ));
    }
    let mut roots = poly_roots(&coef);
    roots.sort_by(|a, b| a.1.abs().total_cmp(&b.1.abs()).then(a.0.total_cmp(&b.0)));
    for (i, r) in roots.iter().enumerate() {
        let r = C(
            if r.0.abs() < 1e-10 { 0.0 } else { r.0 },
            if r.1.abs() < 1e-9 { 0.0 } else { r.1 },
        );
        out.push((
            format!("x{}", i + 1),
            show_complex(C((r.0 * 1e9).round() / 1e9, (r.1 * 1e9).round() / 1e9)),
        ));
    }
    Ok(table(
        &out.iter()
            .map(|(l, v)| (l.as_str(), v.clone()))
            .collect::<Vec<_>>(),
    ))
}

type Matrix = Vec<Vec<f64>>;

fn parse_matrix(s: &str) -> Result<Matrix, String> {
    let m: Matrix = s
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| numbers(&l.replace([',', '[', ']', '|'], " ")))
        .collect::<Result<_, _>>()?;
    if m.is_empty() || m.iter().any(|r| r.len() != m[0].len()) {
        return Err("A matrix needs rows of the same length.".into());
    }
    Ok(m)
}

fn show_matrix(m: &Matrix) -> String {
    let cells: Vec<Vec<String>> = m
        .iter()
        .map(|r| {
            r.iter()
                .map(|v| fmt(if v.abs() < 1e-12 { 0.0 } else { *v }, 6).replace(',', ""))
                .collect()
        })
        .collect();
    let w = cells.iter().flatten().map(|c| c.len()).max().unwrap_or(1);
    cells
        .iter()
        .map(|r| {
            format!(
                "[ {} ]",
                r.iter()
                    .map(|c| format!("{c:>w$}"))
                    .collect::<Vec<_>>()
                    .join("  ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reduced row echelon form, and the determinant's factor and the rank.
fn rref(mut m: Matrix) -> (Matrix, f64, usize) {
    let (rows, cols) = (m.len(), m[0].len());
    let mut det = 1.0;
    let mut r = 0;
    for c in 0..cols {
        if r == rows {
            break;
        }
        let Some(p) = (r..rows)
            .max_by(|&a, &b| m[a][c].abs().total_cmp(&m[b][c].abs()))
            .filter(|&p| m[p][c].abs() > 1e-12)
        else {
            det = 0.0;
            continue;
        };
        if p != r {
            m.swap(p, r);
            det = -det;
        }
        let pivot = m[r][c];
        det *= pivot;
        for v in m[r].iter_mut() {
            *v /= pivot;
        }
        for i in 0..rows {
            if i != r {
                let f = m[i][c];
                for j in 0..cols {
                    m[i][j] -= f * m[r][j];
                }
            }
        }
        r += 1;
    }
    (m, det, r)
}

fn matrix(text: &str, o: &mut Opts) -> Result<Output, String> {
    let (a_text, b_text) = match text.split_once("\n===") {
        Some((a, b)) => (a, Some(b.trim_start_matches('='))),
        None => (text, None),
    };
    let a = parse_matrix(a_text)?;
    let b = b_text.map(parse_matrix).transpose()?;
    let need_b = || {
        b.clone()
            .ok_or("Put the second matrix after a line of ===.")
    };
    let square = |m: &Matrix| {
        if m.len() == m[0].len() {
            Ok(())
        } else {
            Err("That needs a square matrix.".to_string())
        }
    };
    Ok(match o.mode {
        "det" => {
            square(&a)?;
            let (_, det, rank) = rref(a.clone());
            table(&[(
                "Determinant",
                fmt(if rank < a.len() { 0.0 } else { det }, 10),
            )])
        }
        "inverse" => {
            square(&a)?;
            let n = a.len();
            let aug: Matrix = a
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    r.iter()
                        .copied()
                        .chain((0..n).map(|j| if i == j { 1.0 } else { 0.0 }))
                        .collect()
                })
                .collect();
            let (m, _, _) = rref(aug);
            if (0..n).any(|i| (m[i][i] - 1.0).abs() > 1e-9) {
                return Err("The matrix is singular: it has no inverse.".into());
            }
            show_matrix(&m.iter().map(|r| r[n..].to_vec()).collect()).into()
        }
        "transpose" => show_matrix(
            &(0..a[0].len())
                .map(|j| a.iter().map(|r| r[j]).collect())
                .collect(),
        )
        .into(),
        "rank" => table(&[("Rank", rref(a).2.to_string())]),
        "rref" => show_matrix(&rref(a).0).into(),
        "solve" => {
            let n = a.len();
            if a[0].len() != n + 1 {
                return Err(
                    "Write the augmented matrix: each equation's coefficients, then its constant."
                        .into(),
                );
            }
            let (m, _, rank) = rref(a);
            if rank < n {
                return Err("The system has no single solution.".into());
            }
            let names: Vec<String> = (1..=n).map(|i| format!("x{i}")).collect();
            table(
                &(0..n)
                    .map(|i| (names[i].as_str(), fmt(m[i][n], 10)))
                    .collect::<Vec<_>>(),
            )
        }
        "multiply" => {
            let b = need_b()?;
            if a[0].len() != b.len() {
                return Err(
                    "The first matrix needs as many columns as the second has rows.".into(),
                );
            }
            show_matrix(
                &a.iter()
                    .map(|r| {
                        (0..b[0].len())
                            .map(|j| r.iter().zip(&b).map(|(x, br)| x * br[j]).sum())
                            .collect()
                    })
                    .collect(),
            )
            .into()
        }
        op => {
            let b = need_b()?;
            if a.len() != b.len() || a[0].len() != b[0].len() {
                return Err("The matrices need the same size.".into());
            }
            let s = if op == "subtract" { -1.0 } else { 1.0 };
            show_matrix(
                &a.iter()
                    .zip(&b)
                    .map(|(x, y)| x.iter().zip(y).map(|(p, q)| p + s * q).collect())
                    .collect(),
            )
            .into()
        }
    })
}

/// A big natural number, in base 10⁹ digits, least significant first.
#[derive(Clone)]
struct Big(Vec<u64>);

impl Big {
    const BASE: u64 = 1_000_000_000;

    fn mul_small(&mut self, k: u64) {
        let mut carry = 0;
        for d in self.0.iter_mut() {
            let v = *d * k + carry;
            *d = v % Self::BASE;
            carry = v / Self::BASE;
        }
        while carry > 0 {
            self.0.push(carry % Self::BASE);
            carry /= Self::BASE;
        }
    }

    fn add(&self, b: &Big) -> Big {
        let mut out = Vec::with_capacity(self.0.len().max(b.0.len()) + 1);
        let mut carry = 0;
        for i in 0..self.0.len().max(b.0.len()) {
            let v = self.0.get(i).unwrap_or(&0) + b.0.get(i).unwrap_or(&0) + carry;
            out.push(v % Self::BASE);
            carry = v / Self::BASE;
        }
        if carry > 0 {
            out.push(carry);
        }
        Big(out)
    }

    fn digits(&self) -> String {
        let mut s = self.0.last().unwrap_or(&0).to_string();
        for d in self.0.iter().rev().skip(1) {
            s.push_str(&format!("{d:09}"));
        }
        s
    }
}

fn factorial(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.int(0)?;
    if !(0..=3000).contains(&n) {
        return Err("Factorials go from 0! to 3000! here.".into());
    }
    let mut f = Big(vec![1]);
    for k in 2..=n as u64 {
        f.mul_small(k);
    }
    let d = f.digits();
    let digits = d.len();
    Ok(Output::default().code(d).note(format!(
        "{n}! has {digits} digit{}.",
        if digits == 1 { "" } else { "s" }
    )))
}

fn fibonacci(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.int(0)?;
    if !(0..=10000).contains(&n) {
        return Err("Up to the 10,000th Fibonacci number.".into());
    }
    let (mut a, mut b) = (Big(vec![0]), Big(vec![1]));
    let mut list = Vec::new();
    for i in 0..n {
        if o.flag && i < 200 {
            list.push(a.digits());
        }
        let c = a.add(&b);
        a = b;
        b = c;
    }
    let d = a.digits();
    let digits = d.len();
    let out = Output::default().code(d).note(format!(
        "F({n}) has {digits} digit{}.",
        if digits == 1 { "" } else { "s" }
    ));
    Ok(if o.flag {
        out.text(list.join(", "))
    } else {
        out
    })
}

fn gcd_lcm(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v: Vec<u128> = numbers(&o.field(0).replace(',', " "))?
        .iter()
        .map(|x| {
            if x.fract() == 0.0 && *x >= 0.0 && *x < 1e18 {
                Ok(*x as u128)
            } else {
                Err(format!("“{x}” is not a whole number."))
            }
        })
        .collect::<Result<_, _>>()?;
    let g = v.iter().copied().fold(0, gcd);
    let l = v.iter().try_fold(1u128, |l, x| {
        if *x == 0 {
            Some(0)
        } else {
            (l / gcd(l, *x)).checked_mul(*x)
        }
    });
    Ok(table(&[
        ("Greatest common divisor", g.to_string()),
        (
            "Least common multiple",
            l.map_or("too large".into(), |l| l.to_string()),
        ),
        ("Coprime", if g == 1 { "yes" } else { "no" }.into()),
    ]))
}

fn logarithm(_: &str, o: &mut Opts) -> Result<Output, String> {
    let x = o.positive(0)?;
    let base = o.opt_num(1)?.unwrap_or(10.0);
    if base <= 0.0 || base == 1.0 {
        return Err("A base is positive and not 1.".into());
    }
    let label = format!("log base {}", fmt(base, 6));
    Ok(table(&[
        (label.as_str(), fmt(x.ln() / base.ln(), 12)),
        ("Natural (ln)", fmt(x.ln(), 12)),
        ("Base 10", fmt(x.log10(), 12)),
        ("Base 2", fmt(x.log2(), 12)),
    ]))
}

fn exponent(_: &str, o: &mut Opts) -> Result<Output, String> {
    let b = o.num(0)?;
    let e = o.num(1)?;
    let v = b.powf(e);
    let mut rows = vec![(
        "Result",
        if v.is_finite() {
            fmt(v, 12)
        } else {
            "too large".into()
        },
    )];
    if b > 0.0 {
        let l = e * b.log10();
        rows.push((
            "Scientific",
            format!(
                "{}e{}",
                fmt(
                    10f64.powf(
                        l.fract()
                            + if l < 0.0 && l.fract() != 0.0 {
                                1.0
                            } else {
                                0.0
                            }
                    ),
                    10
                ),
                (l.floor()) as i64
            ),
        ));
    }
    if e != 0.0 {
        rows.push(("Root (1/exponent)", fmt(b.powf(1.0 / e), 12)));
    }
    Ok(table(&rows))
}

fn magic_square(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(3);
    let mut m = vec![vec![0usize; n]; n];
    if n % 2 == 1 {
        // Siamese method.
        let (mut i, mut j) = (0, n / 2);
        for k in 1..=n * n {
            m[i][j] = k;
            let (ni, nj) = ((i + n - 1) % n, (j + 1) % n);
            if m[ni][nj] != 0 {
                i = (i + 1) % n;
            } else {
                (i, j) = (ni, nj);
            }
        }
    } else if n % 4 == 0 {
        for i in 0..n {
            for j in 0..n {
                let k = i * n + j + 1;
                let keep = (i % 4 == j % 4) || ((i % 4) + (j % 4) == 3);
                m[i][j] = if keep { n * n + 1 - k } else { k };
            }
        }
    } else {
        // LUX method for singly even sizes.
        let h = n / 2;
        let mut sub = vec![vec![0usize; h]; h];
        let (mut i, mut j) = (0, h / 2);
        for k in 1..=h * h {
            sub[i][j] = k;
            let (ni, nj) = ((i + h - 1) % h, (j + 1) % h);
            if sub[ni][nj] != 0 {
                i = (i + 1) % h;
            } else {
                (i, j) = (ni, nj);
            }
        }
        let q = h * h;
        for i in 0..h {
            for j in 0..h {
                m[i][j] = sub[i][j];
                m[i + h][j + h] = sub[i][j] + q;
                m[i][j + h] = sub[i][j] + 2 * q;
                m[i + h][j] = sub[i][j] + 3 * q;
            }
        }
        let k = (n - 2) / 4;
        for i in 0..h {
            let cols: Vec<usize> = if i == h / 2 {
                (1..=k).collect()
            } else {
                (0..k).collect()
            };
            for &j in cols.iter().chain((n - k + 1..n).collect::<Vec<_>>().iter()) {
                let t = m[i][j];
                m[i][j] = m[i + h][j];
                m[i + h][j] = t;
            }
        }
    }
    let w = (n * n).to_string().len();
    let rows: Vec<String> = m
        .iter()
        .map(|r| {
            r.iter()
                .map(|v| format!("{v:>w$}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    Ok(Output::default().code(rows.join("\n")).note(format!(
        "Every row, column and diagonal adds up to {}.",
        n * (n * n + 1) / 2
    )))
}

fn pascal(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(1);
    let mut rows: Vec<Vec<u128>> = vec![vec![1]];
    for i in 1..n {
        let prev = &rows[i - 1];
        let mut r = vec![1u128];
        for k in 1..i {
            r.push(prev[k - 1] + prev[k]);
        }
        r.push(1);
        rows.push(r);
    }
    let lines: Vec<String> = rows
        .iter()
        .map(|r| r.iter().map(u128::to_string).collect::<Vec<_>>().join(" "))
        .collect();
    let width = lines.last().map_or(0, |l| l.len());
    Ok(lines
        .iter()
        .map(|l| format!("{:^width$}", l).trim_end().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn pow_mod(mut a: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1;
    a %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, a, m);
        }
        a = mul_mod(a, a, m);
        e >>= 1;
    }
    r
}

/// Miller–Rabin with bases that are exact for every 64-bit number.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for p in [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n % p == 0 {
            return n == p;
        }
    }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'outer: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = pow_mod(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mul_mod(x, x, n);
            if x == n - 1 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn pollard(n: u64) -> u64 {
    if n % 2 == 0 {
        return 2;
    }
    for c in 1.. {
        let f = |x: u64| (mul_mod(x, x, n) + c) % n;
        let (mut x, mut y, mut d) = (2u64, 2u64, 1u64);
        while d == 1 {
            x = f(x);
            y = f(f(y));
            d = gcd(x.abs_diff(y) as u128, n as u128) as u64;
        }
        if d != n {
            return d;
        }
    }
    n
}

pub fn factorize(n: u64, out: &mut Vec<u64>) {
    if n == 1 {
        return;
    }
    if is_prime(n) {
        out.push(n);
        return;
    }
    for p in [2u64, 3, 5, 7, 11, 13] {
        if n % p == 0 {
            out.push(p);
            return factorize(n / p, out);
        }
    }
    let d = pollard(n);
    factorize(d, out);
    factorize(n / d, out);
}

fn whole(o: &Opts, i: usize) -> Result<u64, String> {
    let s = o.field(i).replace([',', '_', ' '], "");
    s.parse::<u64>().map_err(|_| {
        format!(
            "“{}” is not a whole number from 0 to 18,446,744,073,709,551,615.",
            o.field(i)
        )
    })
}

fn prime_check(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = whole(o, 0)?;
    let prime = is_prime(n);
    let next = (n.saturating_add(1)..).find(|k| is_prime(*k)).unwrap_or(0);
    let prev = (2..n).rev().find(|k| is_prime(*k));
    let mut f = Vec::new();
    factorize(n.max(1), &mut f);
    f.sort();
    Ok(verdict(
        prime,
        if prime {
            format!("{n} is prime.")
        } else if n < 2 {
            format!("{n} is not prime.")
        } else {
            format!(
                "{n} is not prime: {n} = {}.",
                f.iter().map(u64::to_string).collect::<Vec<_>>().join(" × ")
            )
        },
    )
    .facts(&[
        ("Next prime", next.to_string()),
        (
            "Previous prime",
            prev.map_or("none".into(), |p| p.to_string()),
        ),
    ]))
}

fn prime_factors(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = whole(o, 0)?;
    if n < 2 {
        return Err("Numbers below 2 have no prime factors.".into());
    }
    let mut f = Vec::new();
    factorize(n, &mut f);
    f.sort();
    let mut powers: Vec<(u64, u32)> = Vec::new();
    for p in f {
        match powers.last_mut() {
            Some((q, e)) if *q == p => *e += 1,
            _ => powers.push((p, 1)),
        }
    }
    let sup = |e: u32| {
        e.to_string()
            .chars()
            .map(|c| "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(c as usize - '0' as usize).unwrap())
            .collect::<String>()
    };
    let count: u64 = powers.iter().map(|(_, e)| *e as u64 + 1).product();
    let mut rows = vec![
        (
            "Factors",
            powers
                .iter()
                .map(|(p, e)| {
                    if *e > 1 {
                        format!("{p}{}", sup(*e))
                    } else {
                        p.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" × "),
        ),
        ("Number of divisors", count.to_string()),
    ];
    if count <= 500 {
        let mut divs = vec![1u64];
        for (p, e) in &powers {
            let mut more = Vec::new();
            for d in &divs {
                let mut x = *d;
                for _ in 0..*e {
                    x *= p;
                    more.push(x);
                }
            }
            divs.extend(more);
        }
        divs.sort();
        rows.push((
            "Divisors",
            divs.iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }
    Ok(table(&rows))
}

fn sci_notation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    if v == 0.0 {
        return Ok("0".into());
    }
    let e = v.abs().log10().floor() as i32;
    let m = v / 10f64.powi(e);
    let eng = e.div_euclid(3) * 3;
    let sup: String = e
        .to_string()
        .chars()
        .map(|c| match c {
            '-' => '⁻',
            c => "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(c as usize - '0' as usize).unwrap(),
        })
        .collect();
    let digits = o.number.max(1);
    Ok(table(&[
        ("Scientific", format!("{} × 10{sup}", fmt(m, digits))),
        ("E notation", format!("{:.*}e{e}", digits, m)),
        (
            "Engineering",
            format!("{} × 10^{eng}", fmt(v / 10f64.powi(eng), digits)),
        ),
        ("Decimal", fmt(v, 15)),
    ]))
}

fn combinations(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.int(0)?;
    let k = o.int(1)?;
    if n < 0 || k < 0 || k > n || n > 1000 {
        return Err("Use 0 ≤ k ≤ n ≤ 1000.".into());
    }
    let mut c = Big(vec![1]);
    // C(n, k) exactly: multiply, then divide by small numbers in big form.
    let mut num: Vec<u64> = ((n - k + 1)..=n).map(|x| x as u64).collect();
    for d in 2..=k as u64 {
        let mut d = d;
        for x in num.iter_mut() {
            let g = gcd(*x as u128, d as u128) as u64;
            *x /= g;
            d /= g;
            if d == 1 {
                break;
            }
        }
    }
    for x in &num {
        c.mul_small(*x);
    }
    let mut p = Big(vec![1]);
    for x in (n - k + 1)..=n {
        p.mul_small(x as u64);
    }
    let mut rep = Big(vec![1]);
    let mut rnum: Vec<u64> = (n..n + k).map(|x| x as u64).collect();
    for d in 2..=k as u64 {
        let mut d = d;
        for x in rnum.iter_mut() {
            let g = gcd(*x as u128, d as u128) as u64;
            *x /= g;
            d /= g;
            if d == 1 {
                break;
            }
        }
    }
    for x in &rnum {
        rep.mul_small(*x);
    }
    Ok(table(&[
        ("Combinations C(n, k)", c.digits()),
        ("Permutations P(n, k)", p.digits()),
        ("With repetition", rep.digits()),
        (
            "Arrangements n^k",
            if (n as f64).powi(k as i32) < 1e18 {
                (n as u128).pow(k as u32).to_string()
            } else {
                format!("{:.6e}", (n as f64).powi(k as i32))
            },
        ),
    ]))
}

fn truth_table(_: &str, o: &mut Opts) -> Result<Output, String> {
    let src = o.field(0);
    let norm = src
        .replace("<->", " ≡ ")
        .replace("<=>", " ≡ ")
        .replace("->", " → ")
        .replace("=>", " → ")
        .replace("&&", " ∧ ")
        .replace("||", " ∨ ")
        .replace(['&', '∧', '*'], " ∧ ")
        .replace(['|', '∨', '+'], " ∨ ")
        .replace(['!', '¬', '~'], " ¬ ")
        .replace('^', " ⊕ ")
        .replace('(', " ( ")
        .replace(')', " ) ");
    let tokens: Vec<String> = norm
        .split_whitespace()
        .map(|t| match t.to_lowercase().as_str() {
            "and" => "∧".into(),
            "or" => "∨".into(),
            "not" => "¬".into(),
            "xor" => "⊕".into(),
            "implies" => "→".into(),
            "iff" => "≡".into(),
            _ => t.to_owned(),
        })
        .collect();
    let mut vars: Vec<String> = tokens
        .iter()
        .filter(|t| {
            t.chars().all(|c| c.is_alphanumeric())
                && !matches!(t.as_str(), "0" | "1" | "true" | "false")
        })
        .cloned()
        .collect();
    vars.sort();
    vars.dedup();
    if vars.is_empty() || vars.len() > 8 {
        return Err("Use one to eight variables, such as a and (b or not c).".into());
    }
    fn parse(t: &[String], i: &mut usize, vals: &[(String, bool)]) -> Result<bool, String> {
        // Precedence: ¬ > ∧ > ⊕ > ∨ > → > ≡.
        fn level(
            t: &[String],
            i: &mut usize,
            vals: &[(String, bool)],
            lv: usize,
        ) -> Result<bool, String> {
            const OPS: [&str; 5] = ["≡", "→", "∨", "⊕", "∧"];
            if lv == OPS.len() {
                return unary(t, i, vals);
            }
            let mut l = level(t, i, vals, lv + 1)?;
            while t.get(*i).is_some_and(|x| x == OPS[lv]) {
                *i += 1;
                let r = level(t, i, vals, if OPS[lv] == "→" { lv } else { lv + 1 })?;
                l = match OPS[lv] {
                    "≡" => l == r,
                    "→" => !l || r,
                    "∨" => l || r,
                    "⊕" => l != r,
                    _ => l && r,
                };
                if OPS[lv] == "→" {
                    break;
                }
            }
            Ok(l)
        }
        fn unary(t: &[String], i: &mut usize, vals: &[(String, bool)]) -> Result<bool, String> {
            match t.get(*i).map(String::as_str) {
                Some("¬") => {
                    *i += 1;
                    Ok(!unary(t, i, vals)?)
                }
                Some("(") => {
                    *i += 1;
                    let v = level(t, i, vals, 0)?;
                    if t.get(*i).map(String::as_str) != Some(")") {
                        return Err("A “(” is not closed.".into());
                    }
                    *i += 1;
                    Ok(v)
                }
                Some("1" | "true") => {
                    *i += 1;
                    Ok(true)
                }
                Some("0" | "false") => {
                    *i += 1;
                    Ok(false)
                }
                Some(v) => {
                    let val = vals
                        .iter()
                        .find(|(n, _)| n == v)
                        .map(|x| x.1)
                        .ok_or_else(|| format!("Unexpected “{v}”."))?;
                    *i += 1;
                    Ok(val)
                }
                None => Err("The expression ends too soon.".into()),
            }
        }
        level(t, i, vals, 0)
    }
    let mut out = vec![format!("{} │ {}", vars.join(" "), src)];
    out.push(format!(
        "{}─┼─{}",
        "─".repeat(vars.join(" ").chars().count()),
        "─".repeat(src.chars().count().max(3))
    ));
    let (mut trues, n) = (0, 1usize << vars.len());
    for row in 0..n {
        let vals: Vec<(String, bool)> = vars
            .iter()
            .enumerate()
            .map(|(k, v)| (v.clone(), row >> (vars.len() - 1 - k) & 1 == 0))
            .collect();
        let mut i = 0;
        let r = parse(&tokens, &mut i, &vals)?;
        if i < tokens.len() {
            return Err(format!("Unexpected “{}”.", tokens[i]));
        }
        trues += r as usize;
        let cells: Vec<String> = vars
            .iter()
            .zip(&vals)
            .map(|(name, (_, b))| {
                format!(
                    "{:^w$}",
                    if *b { "T" } else { "F" },
                    w = name.chars().count()
                )
            })
            .collect();
        out.push(format!(
            "{} │ {}",
            cells.join(" "),
            if r { "T" } else { "F" }
        ));
    }
    Ok(Output::default().code(out.join("\n")).note(match trues {
        0 => "A contradiction: never true.".into(),
        t if t == n => "A tautology: always true.".into(),
        t => format!("True in {t} of {n} rows."),
    }))
}

// Numbers

const DIGITS: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

fn to_base(mut n: u128, base: u32) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut s = Vec::new();
    while n > 0 {
        s.push(DIGITS.as_bytes()[(n % base as u128) as usize]);
        n /= base as u128;
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

fn base_convert(_: &str, o: &mut Opts) -> Result<Output, String> {
    let raw = o.field(0).replace(['_', ' '], "");
    let (neg, raw) = match raw.strip_prefix('-') {
        Some(r) => (true, r.to_owned()),
        None => (false, raw),
    };
    let from: u32 = if o.mode == "custom" {
        o.num(1)? as u32
    } else {
        o.mode.parse().unwrap_or(10)
    };
    if !(2..=36).contains(&from) {
        return Err("A base goes from 2 to 36.".into());
    }
    let digits = match from {
        16 => raw.trim_start_matches("0x").trim_start_matches("0X"),
        2 => raw.trim_start_matches("0b"),
        8 => raw.trim_start_matches("0o"),
        _ => &raw,
    };
    let n = u128::from_str_radix(&digits.to_lowercase(), from)
        .map_err(|_| format!("“{digits}” is not a number in base {from}."))?;
    let sign = if neg { "-" } else { "" };
    let mut rows = vec![
        ("Binary", format!("{sign}{}", to_base(n, 2))),
        ("Octal", format!("{sign}{}", to_base(n, 8))),
        ("Decimal", format!("{sign}{n}")),
        (
            "Hexadecimal",
            format!("{sign}{}", to_base(n, 16).to_uppercase()),
        ),
        ("Base 36", format!("{sign}{}", to_base(n, 36))),
    ];
    if let Some(to) = o
        .opt_num(2)?
        .map(|v| v as u32)
        .filter(|b| (2..=36).contains(b))
    {
        rows.insert(0, ("Your base", format!("{sign}{}", to_base(n, to))));
    }
    Ok(table(&rows))
}

const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];
const SCALES: [&str; 7] = [
    "",
    "thousand",
    "million",
    "billion",
    "trillion",
    "quadrillion",
    "quintillion",
];

fn below_thousand(n: u64) -> String {
    let (h, r) = (n / 100, n % 100);
    let mut parts = Vec::new();
    if h > 0 {
        parts.push(format!("{} hundred", ONES[h as usize]));
    }
    if r > 0 {
        parts.push(if r < 20 {
            ONES[r as usize].to_owned()
        } else if r % 10 == 0 {
            TENS[(r / 10) as usize].to_owned()
        } else {
            format!("{}-{}", TENS[(r / 10) as usize], ONES[(r % 10) as usize])
        });
    }
    parts.join(" and ")
}

pub fn words(n: u64) -> String {
    if n == 0 {
        return "zero".into();
    }
    let mut groups = Vec::new();
    let mut m = n;
    while m > 0 {
        groups.push(m % 1000);
        m /= 1000;
    }
    groups
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, g)| **g > 0)
        .map(|(i, g)| {
            format!(
                "{}{}",
                below_thousand(*g),
                if i > 0 {
                    format!(" {}", SCALES[i])
                } else {
                    String::new()
                }
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
        .replace(", ", " ")
}

fn ordinal_words(w: &str) -> String {
    let irregular = [
        ("one", "first"),
        ("two", "second"),
        ("three", "third"),
        ("five", "fifth"),
        ("eight", "eighth"),
        ("nine", "ninth"),
        ("twelve", "twelfth"),
    ];
    let (head, last) = match w.rfind([' ', '-']) {
        Some(i) => (&w[..=i], &w[i + 1..]),
        None => ("", w),
    };
    let last = irregular
        .iter()
        .find(|(a, _)| *a == last)
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|| {
            if let Some(stem) = last.strip_suffix('y') {
                format!("{stem}ieth")
            } else {
                format!("{last}th")
            }
        });
    format!("{head}{last}")
}

fn number_words(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    if v.abs() >= 1e19 {
        return Err("Up to quintillions.".into());
    }
    let whole = v.abs().trunc() as u64;
    let cents = ((v.abs() - v.abs().trunc()) * 100.0).round() as u64;
    let sign = if v < 0.0 { "minus " } else { "" };
    let w = words(whole);
    let decimal = format!("{}", v.abs()).split_once('.').map(|(_, f)| {
        f.chars()
            .map(|c| ONES[c as usize - '0' as usize])
            .collect::<Vec<_>>()
            .join(" ")
    });
    let mut rows = vec![(
        "Words",
        format!(
            "{sign}{w}{}",
            decimal.map_or(String::new(), |d| format!(" point {d}"))
        ),
    )];
    rows.push(("Ordinal", format!("{sign}{}", ordinal_words(&w))));
    rows.push((
        "On a cheque",
        format!("{}{} and {cents:02}/100", sign, capitalize(&w)),
    ));
    let unit = o.field(1);
    if !unit.is_empty() {
        rows.push((
            "As money",
            format!(
                "{sign}{w} {unit}{}",
                if cents > 0 {
                    format!(" and {} cents", words(cents))
                } else {
                    String::new()
                }
            ),
        ));
    }
    Ok(table(&rows))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}

fn bitwise(_: &str, o: &mut Opts) -> Result<Output, String> {
    let parse = |s: &str| -> Result<i64, String> {
        let t = s.trim().replace('_', "");
        let r = if let Some(h) = t.strip_prefix("0x") {
            i64::from_str_radix(h, 16)
        } else if let Some(b) = t.strip_prefix("0b") {
            i64::from_str_radix(b, 2)
        } else {
            t.parse()
        };
        r.map_err(|_| format!("“{s}” is not a whole number (decimal, 0x hex or 0b binary)."))
    };
    let a = parse(o.field(0))?;
    let b = parse(if o.field(1).is_empty() {
        "0"
    } else {
        o.field(1)
    })?;
    let r = match o.mode {
        "or" => a | b,
        "xor" => a ^ b,
        "not" => !a,
        "shl" => a.checked_shl(b as u32).unwrap_or(0),
        "shr" => a.checked_shr(b as u32).unwrap_or(0),
        "nand" => !(a & b),
        _ => a & b,
    };
    let bits = |v: i64| {
        let s = format!("{:b}", v as u64);
        if v >= 0 {
            format!("{:0>8}", format!("{v:b}"))
        } else {
            s
        }
    };
    Ok(table(&[
        ("Decimal", r.to_string()),
        ("Hex", format!("0x{:X}", r)),
        ("Binary", bits(r)),
        ("A", format!("{} ({})", bits(a), a)),
        ("B", format!("{} ({})", bits(b), b)),
    ]))
}

fn twos_complement(_: &str, o: &mut Opts) -> Result<Output, String> {
    let bits: u32 = o.mode.parse().unwrap_or(8);
    let s = o.field(0).replace([' ', '_'], "");
    let max = 1i128 << bits;
    let v: i128 = if o.choice(1) == "binary" {
        let u =
            i128::from_str_radix(&s, 2).map_err(|_| "Write the bits as 0s and 1s.".to_string())?;
        if s.len() as u32 > bits {
            return Err(format!("That is more than {bits} bits."));
        }
        if u >= max / 2 { u - max } else { u }
    } else {
        s.parse().map_err(|_| "Write a whole number.".to_string())?
    };
    if v < -max / 2 || v >= max / 2 {
        return Err(format!("{bits} bits hold {} to {}.", -max / 2, max / 2 - 1));
    }
    let u = if v < 0 { v + max } else { v } as u128;
    Ok(table(&[
        ("Decimal", v.to_string()),
        (
            "Two's complement",
            format!("{:0>width$}", format!("{u:b}"), width = bits as usize),
        ),
        (
            "Hex",
            format!("0x{:0>width$X}", u, width = bits as usize / 4),
        ),
        (
            "One's complement",
            format!(
                "{:0>width$}",
                format!("{:b}", (!u) & (max as u128 - 1)),
                width = bits as usize
            ),
        ),
    ]))
}

fn ieee754(_: &str, o: &mut Opts) -> Result<Output, String> {
    let s = o.field(0);
    let double = o.mode == "64";
    let v: f64 = if let Some(h) = s.strip_prefix("0x") {
        if double {
            f64::from_bits(u64::from_str_radix(h, 16).map_err(|_| "That is not hex.")?)
        } else {
            f32::from_bits(u32::from_str_radix(h, 16).map_err(|_| "That is not hex.")?) as f64
        }
    } else {
        super::parse_number(s)
            .or_else(|| s.parse().ok())
            .ok_or("Write a number, or 0x and its bits in hex.")?
    };
    let (bits, sign, exp, man, ebits, hex, exact) = if double {
        let b = v.to_bits();
        (
            format!("{b:064b}"),
            b >> 63,
            (b >> 52) & 0x7ff,
            b & ((1 << 52) - 1),
            11,
            format!("0x{b:016X}"),
            format!("{v:e}"),
        )
    } else {
        let f = v as f32;
        let b = f.to_bits() as u64;
        (
            format!("{b:032b}"),
            b >> 31,
            (b >> 23) & 0xff,
            b & ((1 << 23) - 1),
            8,
            format!("0x{b:08X}"),
            format!("{} (stored as {:e})", v, f),
        )
    };
    let bias = if double { 1023 } else { 127 };
    Ok(table(&[
        ("Value", exact),
        ("Hex", hex),
        (
            "Bits",
            format!(
                "{} {} {}",
                &bits[..1],
                &bits[1..1 + ebits],
                &bits[1 + ebits..]
            ),
        ),
        (
            "Sign",
            if sign == 1 {
                "negative".into()
            } else {
                "positive".into()
            },
        ),
        ("Exponent", format!("{exp} (2^{})", exp as i64 - bias)),
        ("Mantissa", format!("0x{man:X}")),
    ]))
}

// Statistics

fn sorted(v: &[f64]) -> Vec<f64> {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    s
}

pub fn median(v: &[f64]) -> f64 {
    quantile(&sorted(v), 0.5)
}

fn quantile(s: &[f64], q: f64) -> f64 {
    let pos = q * (s.len() - 1) as f64;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
}

fn mode(v: &[f64]) -> String {
    let mut counts: Vec<(f64, usize)> = Vec::new();
    for x in v {
        match counts.iter_mut().find(|c| c.0 == *x) {
            Some(c) => c.1 += 1,
            None => counts.push((*x, 1)),
        }
    }
    let top = counts.iter().map(|c| c.1).max().unwrap_or(0);
    if top <= 1 {
        return "none".into();
    }
    counts
        .iter()
        .filter(|c| c.1 == top)
        .map(|c| fmt(c.0, 6))
        .collect::<Vec<_>>()
        .join(", ")
        + &format!(" ({top}×)")
}

fn stats(text: &str, _: &mut Opts) -> Result<Output, String> {
    let v = numbers(text)?;
    let n = v.len() as f64;
    let s = sorted(&v);
    let mean = v.iter().sum::<f64>() / n;
    let ss: f64 = v.iter().map(|x| (x - mean).powi(2)).sum();
    let var_s = if n > 1.0 { ss / (n - 1.0) } else { 0.0 };
    let var_p = ss / n;
    let sd = var_s.sqrt();
    let m3: f64 = v.iter().map(|x| (x - mean).powi(3)).sum::<f64>() / n;
    let m4: f64 = v.iter().map(|x| (x - mean).powi(4)).sum::<f64>() / n;
    let (q1, q3) = (quantile(&s, 0.25), quantile(&s, 0.75));
    Ok(table(&[
        ("Count", fmt(n, 0)),
        ("Sum", fmt(v.iter().sum(), 6)),
        ("Mean", fmt(mean, 6)),
        ("Median", fmt(quantile(&s, 0.5), 6)),
        ("Mode", mode(&v)),
        ("Minimum", fmt(s[0], 6)),
        ("Maximum", fmt(s[s.len() - 1], 6)),
        ("Range", fmt(s[s.len() - 1] - s[0], 6)),
        ("Q1 / Q3", format!("{} / {}", fmt(q1, 6), fmt(q3, 6))),
        ("Interquartile range", fmt(q3 - q1, 6)),
        ("Sample variance", fmt(var_s, 6)),
        ("Population variance", fmt(var_p, 6)),
        ("Sample std. deviation", fmt(sd, 6)),
        ("Population std. deviation", fmt(var_p.sqrt(), 6)),
        ("Standard error", fmt(sd / n.sqrt(), 6)),
        (
            "Coefficient of variation",
            if mean != 0.0 {
                format!("{}%", fmt(sd / mean.abs() * 100.0, 3))
            } else {
                "—".into()
            },
        ),
        (
            "Skewness",
            if var_p > 0.0 {
                fmt(m3 / var_p.powf(1.5), 4)
            } else {
                "—".into()
            },
        ),
        (
            "Excess kurtosis",
            if var_p > 0.0 {
                fmt(m4 / var_p.powi(2) - 3.0, 4)
            } else {
                "—".into()
            },
        ),
    ]))
}

/// The regularized lower incomplete gamma function P(a, x).
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        let (mut sum, mut term, mut ap) = (1.0 / a, 1.0 / a, a);
        for _ in 0..500 {
            ap += 1.0;
            term *= x / ap;
            sum += term;
            if term.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        return sum * (-x + a * x.ln() - ln_gamma(a)).exp();
    }
    // A continued fraction for Q, by Lentz's method.
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / 1e-300;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..500 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = b + an / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    1.0 - (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

pub fn normal_cdf(z: f64) -> f64 {
    let p = gamma_p(0.5, z * z / 2.0) / 2.0;
    if z >= 0.0 { 0.5 + p } else { 0.5 - p }
}

/// The z of a probability, by bisection.
fn normal_quantile(p: f64) -> f64 {
    let (mut lo, mut hi) = (-40.0, 40.0);
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if normal_cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn z_score(_: &str, o: &mut Opts) -> Result<Output, String> {
    let x = o.num(0)?;
    let mean = o.num(1)?;
    let sd = o.positive(2)?;
    let z = (x - mean) / sd;
    let below = normal_cdf(z);
    Ok(table(&[
        ("z-score", fmt(z, 4)),
        ("Below (percentile)", format!("{}%", fmt(below * 100.0, 4))),
        ("Above", format!("{}%", fmt((1.0 - below) * 100.0, 4))),
        ("Two-tailed p", fmt(2.0 * (1.0 - normal_cdf(z.abs())), 6)),
    ]))
}

fn normal(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mean = o.num(0)?;
    let sd = o.positive(1)?;
    let a = o.opt_num(2)?;
    let b = o.opt_num(3)?;
    let cdf = |x: f64| normal_cdf((x - mean) / sd);
    let mut rows = Vec::new();
    match (a, b) {
        (Some(a), Some(b)) => rows.push(("P(a < X < b)", fmt((cdf(b) - cdf(a)).abs(), 8))),
        (Some(a), None) => {
            let pdf = (-((a - mean) / sd).powi(2) / 2.0).exp()
                / (sd * (2.0 * std::f64::consts::PI).sqrt());
            rows.push(("P(X < a)", fmt(cdf(a), 8)));
            rows.push(("P(X > a)", fmt(1.0 - cdf(a), 8)));
            rows.push(("Density at a", fmt(pdf, 8)));
        }
        _ => {}
    }
    rows.push((
        "68% interval",
        format!("{} – {}", fmt(mean - sd, 4), fmt(mean + sd, 4)),
    ));
    rows.push((
        "95% interval",
        format!(
            "{} – {}",
            fmt(mean + normal_quantile(0.025) * sd, 4),
            fmt(mean + normal_quantile(0.975) * sd, 4)
        ),
    ));
    rows.push((
        "99% interval",
        format!(
            "{} – {}",
            fmt(mean + normal_quantile(0.005) * sd, 4),
            fmt(mean + normal_quantile(0.995) * sd, 4)
        ),
    ));
    Ok(table(&rows))
}

fn ranks(v: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]));
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
            j += 1;
        }
        for k in i..=j {
            r[idx[k]] = (i + j) as f64 / 2.0 + 1.0;
        }
        i = j + 1;
    }
    r
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let cov: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let (sx, sy) = (
        x.iter().map(|a| (a - mx).powi(2)).sum::<f64>().sqrt(),
        y.iter().map(|b| (b - my).powi(2)).sum::<f64>().sqrt(),
    );
    cov / (sx * sy)
}

fn correlation(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let v = numbers(&l.replace(',', " "))?;
        let [a, b] = v[..] else {
            return Err(format!("“{l}” needs an x and a y."));
        };
        x.push(a);
        y.push(b);
    }
    if x.len() < 3 {
        return Err("Write at least three x, y pairs.".into());
    }
    let r = pearson(&x, &y);
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let slope = x
        .iter()
        .zip(&y)
        .map(|(a, b)| (a - mx) * (b - my))
        .sum::<f64>()
        / x.iter().map(|a| (a - mx).powi(2)).sum::<f64>();
    let strength = match r.abs() {
        a if a >= 0.8 => "very strong",
        a if a >= 0.6 => "strong",
        a if a >= 0.4 => "moderate",
        a if a >= 0.2 => "weak",
        _ => "very weak or none",
    };
    Ok(table(&[
        (
            "Pearson r",
            format!(
                "{} ({strength}, {})",
                fmt(r, 4),
                if r >= 0.0 { "positive" } else { "negative" }
            ),
        ),
        ("r²", fmt(r * r, 4)),
        ("Spearman ρ", fmt(pearson(&ranks(&x), &ranks(&y)), 4)),
        (
            "Regression line",
            format!(
                "y = {} x {} {}",
                fmt(slope, 6),
                if my - slope * mx < 0.0 { "−" } else { "+" },
                fmt((my - slope * mx).abs(), 6)
            ),
        ),
        ("Pairs", fmt(n, 0)),
    ]))
}

fn chi_square(text: &str, o: &mut Opts) -> Result<Output, String> {
    let rows: Vec<Vec<f64>> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| numbers(&l.replace(',', " ")))
        .collect::<Result<_, _>>()?;
    if rows.is_empty() || rows.iter().any(|r| r.len() != rows[0].len()) {
        return Err("Write rows of the same length.".into());
    }
    let (chi, df) = if o.mode == "fit" {
        if rows.len() != 2 {
            return Err(
                "Write the observed counts on one line and the expected on the next.".into(),
            );
        }
        let total_o: f64 = rows[0].iter().sum();
        let total_e: f64 = rows[1].iter().sum();
        let chi: f64 = rows[0]
            .iter()
            .zip(&rows[1])
            .map(|(ob, ex)| {
                let e = ex * total_o / total_e;
                (ob - e).powi(2) / e
            })
            .sum();
        (chi, rows[0].len() as f64 - 1.0)
    } else {
        let total: f64 = rows.iter().flatten().sum();
        let rs: Vec<f64> = rows.iter().map(|r| r.iter().sum()).collect();
        let cs: Vec<f64> = (0..rows[0].len())
            .map(|j| rows.iter().map(|r| r[j]).sum())
            .collect();
        let mut chi = 0.0;
        for (i, r) in rows.iter().enumerate() {
            for (j, ob) in r.iter().enumerate() {
                let e = rs[i] * cs[j] / total;
                chi += (ob - e).powi(2) / e;
            }
        }
        (chi, ((rows.len() - 1) * (rows[0].len() - 1)) as f64)
    };
    if df < 1.0 {
        return Err("That gives no degrees of freedom.".into());
    }
    let p = 1.0 - gamma_p(df / 2.0, chi / 2.0);
    Ok(table(&[
        ("χ²", fmt(chi, 4)),
        ("Degrees of freedom", fmt(df, 0)),
        ("p-value", fmt(p, 6)),
        (
            "At 5%",
            if p < 0.05 {
                "significant: reject the null hypothesis"
            } else {
                "not significant"
            }
            .into(),
        ),
    ]))
}

fn frequency(text: &str, o: &mut Opts) -> Result<Output, String> {
    let v = numbers(text)?;
    let s = sorted(&v);
    let (lo, hi) = (s[0], s[s.len() - 1]);
    let bins = o.number.max(1);
    let width = if hi > lo {
        (hi - lo) / bins as f64
    } else {
        1.0
    };
    let mut counts = vec![0usize; bins];
    for x in &v {
        let i = (((x - lo) / width) as usize).min(bins - 1);
        counts[i] += 1;
    }
    let max = *counts.iter().max().unwrap_or(&1);
    let mut cum = 0;
    let mut out = Vec::new();
    for (i, c) in counts.iter().enumerate() {
        cum += c;
        out.push(vec![
            format!(
                "{} – {}",
                fmt(lo + width * i as f64, 3),
                fmt(lo + width * (i + 1) as f64, 3)
            ),
            c.to_string(),
            format!("{:.1}%", *c as f64 * 100.0 / v.len() as f64),
            format!("{:.1}%", cum as f64 * 100.0 / v.len() as f64),
            "█".repeat((c * 20).div_ceil(max.max(1))),
        ]);
    }
    Ok(grid(&["Range", "Count", "Share", "Cumulative", ""], out))
}

fn probability(_: &str, o: &mut Opts) -> Result<Output, String> {
    let read = |i: usize| -> Result<f64, String> {
        let raw = o.field(i);
        let v = super::parse_number(raw).ok_or_else(|| format!("“{raw}” is not a probability."))?;
        let v = if raw.ends_with('%') || v > 1.0 {
            v / 100.0
        } else {
            v
        };
        if (0.0..=1.0).contains(&v) {
            Ok(v)
        } else {
            Err("A probability goes from 0 to 1 (or 0% to 100%).".into())
        }
    };
    let (a, b) = (read(0)?, read(1)?);
    let p = |v: f64| format!("{} ({}%)", fmt(v, 6), fmt(v * 100.0, 4));
    Ok(table(&[
        ("P(A and B)", p(a * b)),
        ("P(A or B)", p(a + b - a * b)),
        ("P(exactly one)", p(a + b - 2.0 * a * b)),
        ("P(neither)", p((1.0 - a) * (1.0 - b))),
        ("P(not A)", p(1.0 - a)),
        ("P(not B)", p(1.0 - b)),
        ("", "For independent events.".into()),
    ]))
}

// The menus

pub const CALC: &[Tool] = &[
    tool("calculator", "Scientific calculator", "Evaluates expressions line by line: sin, cos, sqrt, log, ln, ^, !, %, pi, e, and variables (x = 2).", calculator)
        .text("Expressions, one per line")
        .selects(&[("Angles in", &[("rad", "Radians"), ("deg", "Degrees")])])
        .slider("Decimals", 1.0, 15.0, 10.0),
    tool("unit-price", "Unit price comparator", "Ranks products by price per 100 g, per litre or per item, and shows the best value.", unit_price)
        .text("Name, price, size on each line (such as: Big pack, 4.99, 1.5 kg)")
        .fields(&[("Currency symbol", "$")])
        .selects(&[("Compare by", &[("weight", "Weight (per 100 g)"), ("volume", "Volume (per litre)"), ("count", "Count (per item)")])]),
    tool("percentage", "Percentage calculator", "Percentages of, as a share, changes and differences.", percentage)
        .form()
        .fields(&[("X", "15"), ("Y", "80")])
        .selects(&[(
            "Find",
            &[("of", "X% of Y"), ("what-percent", "X is what % of Y"), ("change", "% change from X to Y"), ("whole", "X is Y% of what"), ("difference", "% difference of X and Y")],
        )]),
    tool("average", "Average calculator", "Mean, median, mode, geometric and harmonic means; or a weighted average.", average)
        .text("Numbers (or value, weight on each line)")
        .selects(&[("Average", &[("plain", "Plain"), ("weighted", "Weighted")])]),
    tool("fractions", "Fraction calculator", "Exact arithmetic on fractions: 1/2 + 3/4 × 2.", fractions).form().fields(&[("Expression", "1/2 + 3/4 * 2/3")]),
    tool("complex", "Complex numbers", "Arithmetic and functions of complex numbers: (3 + 4i) × (1 − 2i), sqrt(-4).", complex)
        .form()
        .fields(&[("Expression", "(3 + 4i) * (1 - 2i)")]),
];

pub const MATH: &[Tool] = &[
    tool("equation", "Equation solver", "Every root, real and complex, of a polynomial equation of any degree.", equation)
        .form()
        .fields(&[("Coefficients, highest power first", "1, -6, 11, -6")]),
    tool("matrix", "Matrix calculator", "Determinant, inverse, transpose, rank, row reduction, linear systems, and two-matrix sums and products.", matrix)
        .text("A matrix, a row per line (a second one after a line of ===)")
        .selects(&[(
            "Operation",
            &[("det", "Determinant"), ("inverse", "Inverse"), ("transpose", "Transpose"), ("rank", "Rank"), ("rref", "Row echelon form"), ("solve", "Solve the linear system"), ("multiply", "A × B"), ("add", "A + B"), ("subtract", "A − B")],
        )]),
    tool("fraction-simplify", "Fraction simplifier", "Simplifies a fraction, or finds the fraction of a decimal.", simplify).form().fields(&[("Fraction or decimal", "42/56")]),
    tool("exponent", "Exponent calculator", "Powers and roots.", exponent).form().fields(&[("Base", "2"), ("Exponent", "64")]),
    tool("logarithm", "Logarithm calculator", "Logarithms in any base.", logarithm).form().fields(&[("Number", "1000"), ("Base", "10")]),
    tool("factorial", "Factorial", "n! exactly, however many digits.", factorial).form().fields(&[("n", "25")]),
    tool("fibonacci", "Fibonacci numbers", "The nth Fibonacci number, exactly.", fibonacci)
        .form()
        .fields(&[("n", "100")])
        .checks(&["List the sequence"]),
    tool("combinations", "Combinations and permutations", "nCr, nPr and more, exactly.", combinations).form().fields(&[("n", "52"), ("k", "5")]),
    tool("gcd-lcm", "GCD and LCM", "The greatest common divisor and least common multiple.", gcd_lcm).form().fields(&[("Numbers", "12, 18, 30")]),
    tool("prime-check", "Prime checker", "Whether a number is prime, and its neighbouring primes.", prime_check).form().fields(&[("Number", "1000003")]),
    tool("prime-factors", "Prime factorization", "Prime factors and divisors of a number.", prime_factors).form().fields(&[("Number", "360")]),
    tool("magic-square", "Magic square", "A magic square of any size.", magic_square).form().slider("Size", 3.0, 16.0, 4.0),
    tool("pascal", "Pascal's triangle", "Rows of Pascal's triangle.", pascal).form().slider("Rows", 1.0, 30.0, 10.0),
    tool("sci-notation", "Scientific notation", "A number in scientific, E and engineering notation.", sci_notation)
        .form()
        .fields(&[("Number", "0.000123456")])
        .slider("Decimals", 1.0, 12.0, 4.0),
    tool("truth-table", "Truth table", "The truth table of a logic expression: and, or, not, xor, ->, <->.", truth_table)
        .form()
        .fields(&[("Expression", "(a and b) -> not c")]),
];

pub const NUMBERS: &[Tool] = &[
    tool(
        "base",
        "Number base converter",
        "Binary, octal, decimal, hex and any base from 2 to 36.",
        base_convert,
    )
    .form()
    .fields(&[
        ("Number", "255"),
        ("From base (custom)", "7"),
        ("Also to base", ""),
    ])
    .selects(&[(
        "From",
        &[
            ("10", "Decimal"),
            ("2", "Binary"),
            ("8", "Octal"),
            ("16", "Hexadecimal"),
            ("custom", "Custom base"),
        ],
    )]),
    tool(
        "number-words",
        "Number to words",
        "A number in English words, as an ordinal, and as on a cheque.",
        number_words,
    )
    .form()
    .fields(&[("Number", "1234.56"), ("Currency (optional)", "dollars")]),
    tool(
        "roman",
        "Decimal to Roman numerals",
        "Each number (1–3999) as Roman numerals.",
        super::encode::decimal_to_roman,
    )
    .text("Numbers, one per line"),
    tool(
        "roman-decimal",
        "Roman numerals to decimal",
        "Each Roman numeral as a number.",
        super::encode::roman_to_decimal,
    )
    .text("Roman numerals, one per line"),
    tool(
        "bitwise",
        "Bitwise calculator",
        "AND, OR, XOR, NOT and shifts, in decimal, hex and binary.",
        bitwise,
    )
    .form()
    .fields(&[("A", "0b1100"), ("B", "0b1010")])
    .selects(&[(
        "Operation",
        &[
            ("and", "A AND B"),
            ("or", "A OR B"),
            ("xor", "A XOR B"),
            ("nand", "A NAND B"),
            ("not", "NOT A"),
            ("shl", "A << B"),
            ("shr", "A >> B"),
        ],
    )]),
    tool(
        "twos-complement",
        "Two's complement",
        "Signed numbers as two's complement bits, and back.",
        twos_complement,
    )
    .form()
    .fields(&[("Value", "-42")])
    .selects(&[
        (
            "Bits",
            &[("8", "8"), ("16", "16"), ("32", "32"), ("64", "64")],
        ),
        (
            "The value is",
            &[("decimal", "Decimal"), ("binary", "Binary")],
        ),
    ]),
    tool(
        "ieee754",
        "IEEE 754 floating point",
        "How a number is stored as a float or double.",
        ieee754,
    )
    .form()
    .fields(&[("Number, or 0x bits", "0.1")])
    .selects(&[(
        "Precision",
        &[("64", "Double (64 bits)"), ("32", "Single (32 bits)")],
    )]),
];

pub const STATS: &[Tool] = &[
    tool(
        "stats",
        "Descriptive statistics",
        "Mean, median, mode, quartiles, variance, standard deviation, skewness and more.",
        stats,
    )
    .text("Numbers"),
    tool(
        "z-score",
        "Z-score",
        "A value's z-score and percentile.",
        z_score,
    )
    .form()
    .fields(&[
        ("Value", "130"),
        ("Mean", "100"),
        ("Standard deviation", "15"),
    ]),
    tool(
        "normal",
        "Normal distribution",
        "Probabilities and intervals of a normal distribution.",
        normal,
    )
    .form()
    .fields(&[
        ("Mean", "100"),
        ("Standard deviation", "15"),
        ("a", "115"),
        ("b (optional)", ""),
    ]),
    tool(
        "correlation",
        "Correlation and regression",
        "Pearson and Spearman correlation and the regression line.",
        correlation,
    )
    .text("x, y on each line"),
    tool(
        "chi-square",
        "Chi-square test",
        "Goodness of fit or independence, with the p-value.",
        chi_square,
    )
    .text("Counts: a row per line")
    .selects(&[(
        "Test",
        &[
            ("independence", "Independence (a contingency table)"),
            ("fit", "Goodness of fit (observed, then expected)"),
        ],
    )]),
    tool(
        "frequency-distribution",
        "Frequency distribution",
        "Counts and shares of numbers in equal bins, with a histogram.",
        frequency,
    )
    .text("Numbers")
    .slider("Bins", 2.0, 30.0, 8.0),
    tool(
        "probability",
        "Probability calculator",
        "And, or, neither and more for two independent events.",
        probability,
    )
    .form()
    .fields(&[("P(A)", "0.5"), ("P(B)", "30%")]),
];

#[cfg(test)]
mod tests {
    use super::super::Rng;
    use super::*;

    fn calc(s: &str) -> f64 {
        eval(&parse_expr(s).unwrap(), &[], false).unwrap()
    }

    #[test]
    fn expressions() {
        assert_eq!(calc("1 + 2 * 3"), 7.0);
        assert_eq!(calc("-2^2"), -4.0);
        assert_eq!(calc("2^3^2"), 512.0);
        assert_eq!(calc("2pi / pi"), 2.0);
        assert_eq!(calc("5!"), 120.0);
        assert_eq!(calc("50% * 20"), 10.0);
        assert_eq!(calc("10 % 3"), 1.0);
        assert_eq!(calc("1.5e3 + e - e"), 1500.0);
        assert_eq!(calc("max(1, 4, 2)"), 4.0);
        let r: Ratio = eval(&parse_expr("1/2 + 3/4 * 2/3").unwrap(), &[], false).unwrap();
        assert_eq!(r, Ratio(1, 1));
        let z: C = eval(&parse_expr("(3 + 4i) * (1 - 2i)").unwrap(), &[], false).unwrap();
        assert_eq!(z, C(11.0, -2.0));
    }

    #[test]
    fn number_theory() {
        assert!(is_prime(1_000_000_007));
        assert!(!is_prime(1_000_000_007 * 3));
        let mut f = Vec::new();
        factorize(600851475143, &mut f);
        f.sort();
        assert_eq!(f, vec![71, 839, 1471, 6857]);
        assert_eq!(
            words(1234567),
            "one million two hundred and thirty-four thousand five hundred and sixty-seven"
        );
        assert_eq!(ordinal_words("twenty-one"), "twenty-first");
        assert_eq!(to_fraction(0.75, 1e-9), (3, 4));
    }

    #[test]
    fn unit_prices() {
        let mut rng = Rng(1);
        let mut o = Opts::new("weight", &mut rng).with(&["$"], &["weight"], &[]);
        let out = unit_price("Small, 2.00, 250 g\nBig, 5.00, 1 kg\nMid, 3, 500g", &mut o)
            .unwrap()
            .plain();
        let first = out.lines().nth(1).unwrap();
        assert!(
            first.starts_with("Big") && first.contains("Best value"),
            "{out}"
        );
        assert!(out.contains("+60%"), "{out}");
    }

    #[test]
    fn statistics() {
        assert!((normal_cdf(1.96) - 0.975).abs() < 1e-4);
        assert!((gamma_p(1.0, 1.0) - (1.0 - (-1.0f64).exp())).abs() < 1e-12);
        assert_eq!(median(&[3.0, 1.0, 2.0, 10.0]), 2.5);
        let roots = poly_roots(&[1.0, -6.0, 11.0, -6.0]);
        let mut re: Vec<f64> = roots.iter().map(|r| (r.0 * 1e6).round() / 1e6).collect();
        re.sort_by(f64::total_cmp);
        assert_eq!(re, vec![1.0, 2.0, 3.0]);
    }
}
