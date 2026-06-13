//! 最小 JSON パーサー / シリアライザ (std のみ)。
//!
//! MCP プロトコルで必要な値域に絞って実装する:
//! 数値は f64 単一型、オブジェクトキーは String。
//! 外部クレート不要 (ADR-003 / 問4)。

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let Value::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        if let Value::Array(a) = self {
            Some(a)
        } else {
            None
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

// ── 構築ヘルパ ────────────────────────────────────────────────────────────────

pub fn obj(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn arr(items: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(items.into_iter().collect())
}

pub fn s(v: impl Into<String>) -> Value {
    Value::String(v.into())
}
pub fn n(v: f64) -> Value {
    Value::Number(v)
}
pub fn b(v: bool) -> Value {
    Value::Bool(v)
}
pub const NULL: Value = Value::Null;

// ── シリアライザ ──────────────────────────────────────────────────────────────

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            Value::String(s) => {
                f.write_str("\"")?;
                for c in s.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\n' => f.write_str("\\n")?,
                        '\r' => f.write_str("\\r")?,
                        '\t' => f.write_str("\\t")?,
                        c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
                        c => write!(f, "{c}")?,
                    }
                }
                f.write_str("\"")
            }
            Value::Array(a) => {
                f.write_str("[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Value::Object(m) => {
                f.write_str("{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "\"{}\":{v}", escape(k))?;
                }
                f.write_str("}")
            }
        }
    }
}

fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => { let _ = fmt::write(&mut out, format_args!("\\u{:04x}", c as u32)); }
            c => out.push(c),
        }
    }
    out
}

// ── パーサー ──────────────────────────────────────────────────────────────────

pub fn parse(input: &str) -> Result<Value, String> {
    let mut p = Parser {
        src: input.as_bytes(),
        pos: 0,
        depth: 0,
    };
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.src.len() {
        return Err(format!("trailing garbage at pos {}", p.pos));
    }
    Ok(v)
}

/// ネスト深さの上限 (リソース上限・問16)。これを超える入力は拒否し、
/// 再帰下降パーサのスタックオーバーフロー (= DoS) を防ぐ。
const MAX_DEPTH: usize = 128;

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }
    fn advance(&mut self) {
        self.pos += 1;
    }
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.advance();
        }
    }
    fn expect(&mut self, b: u8) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(c) if c == b => {
                self.advance();
                Ok(())
            }
            other => Err(format!(
                "expected '{}' got {:?} at pos {}",
                b as char, other, self.pos
            )),
        }
    }

    /// 全ての再帰はここを通る。深さ上限を一点で強制する (問16)。
    fn parse_value(&mut self) -> Result<Value, String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(format!(
                "nesting too deep (> {MAX_DEPTH}) at pos {}",
                self.pos
            ));
        }
        let r = self.parse_value_inner();
        self.depth -= 1;
        r
    }

    fn parse_value_inner(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => self.parse_string().map(Value::String),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b't') => {
                self.pos += 4;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.pos += 5;
                Ok(Value::Bool(false))
            }
            Some(b'n') => {
                self.pos += 4;
                Ok(Value::Null)
            }
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            other => Err(format!("unexpected {:?} at pos {}", other, self.pos)),
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err("unterminated string".into()),
                Some(b'"') => {
                    self.advance();
                    return Ok(s);
                }
                Some(b'\\') => {
                    self.advance();
                    match self.peek() {
                        Some(b'"') => {
                            s.push('"');
                            self.advance();
                        }
                        Some(b'\\') => {
                            s.push('\\');
                            self.advance();
                        }
                        Some(b'n') => {
                            s.push('\n');
                            self.advance();
                        }
                        Some(b'r') => {
                            s.push('\r');
                            self.advance();
                        }
                        Some(b't') => {
                            s.push('\t');
                            self.advance();
                        }
                        Some(b'/') => {
                            s.push('/');
                            self.advance();
                        }
                        Some(b'u') => {
                            self.advance();
                            let hex: String = (0..4)
                                .map(|_| {
                                    let c = self.peek().unwrap_or(b'0') as char;
                                    self.advance();
                                    c
                                })
                                .collect();
                            let cp = u32::from_str_radix(&hex, 16)
                                .map_err(|_| format!("bad \\u escape: {hex}"))?;
                            s.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                        }
                        other => s.push(other.unwrap_or(b'?') as char),
                    }
                }
                Some(c) => {
                    s.push(c as char);
                    self.advance();
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.advance();
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
        }
        if self.peek() == Some(b'.') {
            self.advance();
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance();
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        let val: f64 = s
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        // 非有限値の混入を一点で拒否 (問20): 例 1e400 は f64 で +inf に丸められ、
        // SDF へ伝播すると無音の不正メッシュを生む。ここで遮断する。
        if !val.is_finite() {
            return Err(format!("number out of range (non-finite): {s}"));
        }
        Ok(Value::Number(val))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut arr = vec![];
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.advance();
            return Ok(Value::Array(arr));
        }
        loop {
            arr.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.advance();
                }
                Some(b']') => {
                    self.advance();
                    break;
                }
                other => return Err(format!("expected ',' or ']' got {:?}", other)),
            }
        }
        Ok(Value::Array(arr))
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.advance();
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.expect(b':')?;
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.advance();
                }
                Some(b'}') => {
                    self.advance();
                    break;
                }
                other => return Err(format!("expected ',' or '}}' got {:?}", other)),
            }
        }
        Ok(Value::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object() {
        let v = obj([
            ("a", n(1.0)),
            ("b", s("hello")),
            ("c", b(true)),
            ("d", NULL),
        ]);
        let s = v.to_string();
        let v2 = parse(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn nested_array() {
        let v = arr([n(1.0), arr([n(2.0), n(3.0)]), s("x")]);
        assert_eq!(parse(&v.to_string()).unwrap(), v);
    }

    #[test]
    fn string_escaping() {
        let v = s("hello\nworld\t\"quoted\"");
        let s = v.to_string();
        let v2 = parse(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn get_and_as_str() {
        let v = obj([("key", s("val"))]);
        assert_eq!(v.get("key").and_then(|v| v.as_str()), Some("val"));
        assert!(v.get("missing").is_none());
    }

    #[test]
    fn deeply_nested_input_is_rejected_not_overflowed() {
        // 問16: 病的にネストした入力でスタックを溢れさせず、エラーで拒否する。
        let depth = MAX_DEPTH + 50;
        let src = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let r = parse(&src);
        assert!(r.is_err(), "over-deep input must be rejected");
        assert!(r.unwrap_err().contains("nesting too deep"));
    }

    #[test]
    fn moderately_nested_input_still_parses() {
        let depth = 20;
        let src = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse(&src).is_ok(), "shallow nesting must still parse");
    }

    #[test]
    fn non_finite_number_is_rejected() {
        // 問20: 1e400 は f64 で +inf に丸められる。パーサで拒否する。
        let r = parse("1e400");
        assert!(r.is_err(), "overflowing number must be rejected");
        assert!(r.unwrap_err().contains("non-finite"));
        assert!(parse("-1e400").is_err());
        // 通常の数は通る。
        assert!(parse("1.5e3").is_ok());
    }

    #[test]
    fn object_key_escape_matches_string_value_escape() {
        // 問32: escape() (オブジェクトキー) と Display (文字列値) の制御文字処理が一致するか。
        // キーに \r, \t, 制御文字が含まれる場合の一貫性を確認する。
        let v = Value::Object({
            let mut m = std::collections::BTreeMap::new();
            m.insert("key\twith\ttabs".to_string(), s("val\twith\ttabs"));
            m.insert("key\rwith\rCR".to_string(), s("val\rwith\rCR"));
            m
        });
        let serialized = v.to_string();
        // シリアライズ後に再パースしてラウンドトリップ一致を確認。
        let reparsed = parse(&serialized).unwrap();
        assert_eq!(v, reparsed, "object keys with control chars must roundtrip");
        // 出力が生の制御文字を含まないことを確認。
        assert!(
            !serialized.chars().any(|c| c == '\t' || c == '\r'),
            "serialized output must not contain raw control chars"
        );
    }
}
