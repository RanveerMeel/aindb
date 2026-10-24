//! Strict JSON (RFC 8259 subset) used by the typed protocol form.
//!
//! Stricter than RFC 8259 on purpose (SPEC §5.3, ADR-0004):
//! * no floating point or exponents — decimals travel as strings;
//! * integers are canonical (no leading zeros, no `-0`) and fit in `i128`;
//! * duplicate object keys, lone surrogates and trailing data are errors;
//! * nesting depth is bounded.
//!
//! The encoder produces compact output with object keys in the order given.

use crate::DecodeError;

/// Maximum nesting depth of arrays and objects.
pub const MAX_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i128),
    Str(String),
    Arr(Vec<Json>),
    /// Keys are unique; order is as written.
    Obj(Vec<(String, Json)>),
}

pub fn parse(bytes: &[u8]) -> Result<Json, DecodeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| DecodeError::Invalid("input is not UTF-8".into()))?;
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
        path: Vec::new(),
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    /// Path of object keys / array indexes, for error messages.
    path: Vec<String>,
}

impl Parser<'_> {
    fn here(&self) -> String {
        if self.path.is_empty() {
            "$".into()
        } else {
            format!("$.{}", self.path.join("."))
        }
    }

    fn err(&self, what: &str) -> DecodeError {
        DecodeError::Invalid(format!("{what} at byte {} ({})", self.i, self.here()))
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.s.get(self.i) {
            self.i += 1;
        }
    }

    fn eat(&mut self, lit: &[u8]) -> bool {
        if self.s[self.i..].starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, DecodeError> {
        if depth > MAX_DEPTH {
            return Err(DecodeError::LimitExceeded(format!(
                "nesting deeper than {MAX_DEPTH}"
            )));
        }
        match self.s.get(self.i) {
            None => Err(self.err("unexpected end of input")),
            Some(b'n') if self.eat(b"null") => Ok(Json::Null),
            Some(b't') if self.eat(b"true") => Ok(Json::Bool(true)),
            Some(b'f') if self.eat(b"false") => Ok(Json::Bool(false)),
            Some(b'"') => self.string().map(Json::Str),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(_) => Err(self.err("unexpected character")),
        }
    }

    fn number(&mut self) -> Result<Json, DecodeError> {
        let start = self.i;
        if self.s[self.i] == b'-' {
            self.i += 1;
        }
        let digits = self.i;
        while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
        }
        if let Some(b'.' | b'e' | b'E') = self.s.get(self.i) {
            return Err(DecodeError::NonCanonical(format!(
                "{}: floating-point numbers are not allowed; use {{\"$decimal\": \"…\"}}",
                self.here()
            )));
        }
        let lit = std::str::from_utf8(&self.s[start..self.i]).expect("ASCII");
        let body = &self.s[digits..self.i];
        if body.is_empty() {
            return Err(self.err("invalid number"));
        }
        if (body.len() > 1 && body[0] == b'0') || lit == "-0" {
            return Err(DecodeError::NonCanonical(format!(
                "{}: integer {lit}",
                self.here()
            )));
        }
        lit.parse::<i128>().map(Json::Int).map_err(|_| {
            DecodeError::LimitExceeded(format!("{}: integer out of range", self.here()))
        })
    }

    fn hex4(&mut self) -> Result<u32, DecodeError> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or_else(|| self.err("truncated \\u escape"))?;
        let h = std::str::from_utf8(h).map_err(|_| self.err("bad \\u escape"))?;
        let v = u32::from_str_radix(h, 16).map_err(|_| self.err("bad \\u escape"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, DecodeError> {
        self.i += 1; // opening quote
        let mut out = String::new();
        loop {
            let start = self.i;
            while let Some(&c) = self.s.get(self.i) {
                if c == b'"' || c == b'\\' || c < 0x20 {
                    break;
                }
                self.i += 1;
            }
            out.push_str(std::str::from_utf8(&self.s[start..self.i]).expect("input is UTF-8"));
            match self.s.get(self.i) {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let esc = *self
                        .s
                        .get(self.i)
                        .ok_or_else(|| self.err("truncated escape"))?;
                    self.i += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if !self.eat(b"\\u") {
                                    return Err(self.err("lone high surrogate"));
                                }
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(self.err("invalid low surrogate"));
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(cp).ok_or_else(|| self.err("lone surrogate"))?);
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                }
                Some(_) => return Err(self.err("control character in string")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, DecodeError> {
        self.i += 1;
        let mut items = Vec::new();
        self.ws();
        if self.eat(b"]") {
            return Ok(Json::Arr(items));
        }
        loop {
            self.path.push(items.len().to_string());
            self.ws();
            items.push(self.value(depth + 1)?);
            self.path.pop();
            self.ws();
            if self.eat(b"]") {
                return Ok(Json::Arr(items));
            }
            if !self.eat(b",") {
                return Err(self.err("expected ',' or ']'"));
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, DecodeError> {
        self.i += 1;
        let mut members: Vec<(String, Json)> = Vec::new();
        self.ws();
        if self.eat(b"}") {
            return Ok(Json::Obj(members));
        }
        loop {
            self.ws();
            if self.s.get(self.i) != Some(&b'"') {
                return Err(self.err("expected object key"));
            }
            let key = self.string()?;
            if members.iter().any(|(k, _)| *k == key) {
                self.path.push(key);
                return Err(DecodeError::DuplicateKey(self.here()));
            }
            self.ws();
            if !self.eat(b":") {
                return Err(self.err("expected ':'"));
            }
            self.ws();
            self.path.push(key.clone());
            let v = self.value(depth + 1)?;
            self.path.pop();
            members.push((key, v));
            self.ws();
            if self.eat(b"}") {
                return Ok(Json::Obj(members));
            }
            if !self.eat(b",") {
                return Err(self.err("expected ',' or '}'"));
            }
        }
    }
}

/// Compact encoding. Strings escape only what JSON requires.
pub fn encode(v: &Json, out: &mut String) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Int(n) => out.push_str(&n.to_string()),
        Json::Str(s) => encode_str(s, out),
        Json::Arr(items) => {
            out.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode(x, out);
            }
            out.push(']');
        }
        Json::Obj(members) => {
            out.push('{');
            for (i, (k, x)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_str(k, out);
                out.push(':');
                encode(x, out);
            }
            out.push('}');
        }
    }
}

fn encode_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Json, DecodeError> {
        parse(s.as_bytes())
    }

    #[test]
    fn accepts_strict_json() {
        assert_eq!(
            p(r#" {"a": [1, -2, true, null, "xé😀\n"]} "#).unwrap(),
            Json::Obj(vec![(
                "a".into(),
                Json::Arr(vec![
                    Json::Int(1),
                    Json::Int(-2),
                    Json::Bool(true),
                    Json::Null,
                    Json::Str("xé😀\n".into())
                ])
            )])
        );
    }

    #[test]
    fn rejects_non_canonical_and_malformed() {
        assert!(matches!(p("1.5"), Err(DecodeError::NonCanonical(_))));
        assert!(matches!(p("1e3"), Err(DecodeError::NonCanonical(_))));
        assert!(matches!(p("01"), Err(DecodeError::NonCanonical(_))));
        assert!(matches!(p("-0"), Err(DecodeError::NonCanonical(_))));
        assert!(matches!(p(r#"{"a":1,"a":2}"#), Err(DecodeError::DuplicateKey(k)) if k == "$.a"));
        assert_eq!(p("{} x"), Err(DecodeError::TrailingBytes));
        assert!(matches!(p(r#""\ud800""#), Err(DecodeError::Invalid(_))));
        assert!(matches!(p("\"a\u{1}\""), Err(DecodeError::Invalid(_))));
        assert!(matches!(p("[1,]"), Err(DecodeError::Invalid(_))));
        assert!(matches!(p("{'a':1}"), Err(DecodeError::Invalid(_))));
        assert!(matches!(
            p(&"[".repeat(MAX_DEPTH + 2)),
            Err(DecodeError::LimitExceeded(_))
        ));
        assert!(matches!(
            p(&"9".repeat(60)),
            Err(DecodeError::LimitExceeded(_))
        ));
        assert!(matches!(parse(b"\"\xff\""), Err(DecodeError::Invalid(_))));
    }

    #[test]
    fn encode_round_trips() {
        let src = r#"{"k":["a\"b\\c\u0001",-7,false,null,{}]}"#;
        let v = p(src).unwrap();
        let mut out = String::new();
        encode(&v, &mut out);
        assert_eq!(out, src);
        assert_eq!(p(&out).unwrap(), v);
    }
}
