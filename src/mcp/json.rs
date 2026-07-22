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
                // 問117: 非有限値 (NaN/±Inf) は JSON に表現できない。Rust の Display は
                // "NaN"/"inf"/"-inf" を吐くが、これは**不正な JSON** であり、MCP 応答を
                // 受け取る AI クライアントのパーサを壊す (本パーサ自身も問20 で拒否する)。
                // パーサ側 (入力) は問20 で遮断済みだが、出力側 (内部計算の伝播) は無防備
                // だった。serde_json と同じく null に落とし、応答が常に valid JSON である
                // ことを保証する (決定性のある安全側の縮退)。
                if !n.is_finite() {
                    f.write_str("null")
                } else if n.fract() == 0.0 && n.abs() < 1e15 {
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
            c if (c as u32) < 0x20 => {
                let _ = fmt::write(&mut out, format_args!("\\u{:04x}", c as u32));
            }
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

/// UTF-8 リードバイトからシーケンス長 (バイト数) を返す (問49)。
/// 不正なリードバイトは 1 として扱い、進行を保証して無限ループを防ぐ。
fn utf8_seq_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else if lead >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

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
            Some(b't') => self.parse_literal(b"true", Value::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", Value::Bool(false)),
            Some(b'n') => self.parse_literal(b"null", Value::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            other => Err(format!("unexpected {:?} at pos {}", other, self.pos)),
        }
    }

    /// `true`/`false`/`null` リテラルを厳密に照合する (問50)。
    /// バイト列が一致しない・末尾を越える場合はエラーにし、`nXYZ` のような
    /// 不正入力を `null` として無音受理する退行と pos 溢れを防ぐ。
    fn parse_literal(&mut self, kw: &'static [u8], val: Value) -> Result<Value, String> {
        let end = self.pos + kw.len();
        if end <= self.src.len() && &self.src[self.pos..end] == kw {
            self.pos = end;
            Ok(val)
        } else {
            Err(format!(
                "invalid literal at pos {} (expected \"{}\")",
                self.pos,
                std::str::from_utf8(kw).unwrap()
            ))
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
                        // 問167: 未知のエスケープは厳密に拒否する。以前は
                        // `s.push(other.unwrap_or(b'?') as char)` だったが advance を
                        // 呼ばないため、次ループで同じ文字が `Some(c)` 経由で再 push され
                        // `\q` → "qq" のように文字が二重化するバグがあった。
                        // malformed literal / bad \u escape と同様にエラーとする。
                        Some(c) => {
                            return Err(format!(
                                "invalid escape \\{} at pos {}",
                                c as char, self.pos
                            ));
                        }
                        None => return Err("unterminated string: trailing backslash".into()),
                    }
                }
                Some(c) => {
                    // 問49: 生バイトを `c as char` で push すると Latin-1 解釈になり
                    // マルチバイト UTF-8 (日本語・絵文字等) が破壊される。
                    // `self.src` は &str 由来で正当な UTF-8 なので、リード文字から
                    // シーケンス長を判定し、文字単位でコピーする。
                    let len = utf8_seq_len(c);
                    let end = (self.pos + len).min(self.src.len());
                    match std::str::from_utf8(&self.src[self.pos..end]) {
                        Ok(chunk) => s.push_str(chunk),
                        // 不正な UTF-8 (本来到達しないが防御的に) は置換文字へ。
                        Err(_) => s.push('\u{FFFD}'),
                    }
                    self.pos = end;
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
    fn malformed_numbers_are_rejected_not_panicked() {
        // 問297: parse_number は数値スパン (ASCII の -0-9.eE+ のみ) に対し
        // `from_utf8(...).unwrap()` を使う。構造的に安全だが、不正な数値入力が
        // パニックせず必ず Err を返すことを観測可能な不変条件として固定する
        // (untrusted な MCP 入力経路のクラッシュ防御・SECURITY §4)。
        for bad in [
            "-",     // 単独マイナス
            "-.",    // 数字のないマイナス小数点
            ".",     // 単独ドット (数値開始でない)
            "1e",    // 指数部が空
            "1e+",   // 符号のみで指数桁なし
            "1e-",   // 同上
            "-e5",   // 仮数部が空
            "1..2",  // ドット2つ (末尾 ".2" が余剰トークンになる)
            "01e",   // 指数欠損
            "1.2.3", // ドット過多
        ] {
            let r = parse(bad);
            assert!(
                r.is_err(),
                "malformed number {bad:?} must be rejected (got {r:?}), never panic"
            );
        }
        // 妥当な数値は通る (境界を狭めすぎていないことの確認)。
        // 注: バッキングの Rust `f64::from_str` は "1." や "01" を許容する寛容性を持つ
        // (厳密 JSON より緩い) が、パニックせず値を返す点では安全。ここでは明確に妥当な
        // 数値のみを OK として固定する。
        for ok in [
            "0",
            "-0",
            "1.5",
            "1e3",
            "1E3",
            "-2.5e-3",
            "1.0e+2",
            "123456789",
        ] {
            assert!(parse(ok).is_ok(), "valid number {ok:?} must parse");
        }
    }

    #[test]
    fn malformed_literals_are_rejected() {
        // 問50: true/false/null は厳密照合。誤綴り・途中切れを無音受理しない。
        assert!(parse("nul").is_err(), "truncated null must be rejected");
        assert!(parse("nXYZ").is_err(), "misspelled null must be rejected");
        assert!(parse("tru").is_err(), "truncated true must be rejected");
        assert!(parse("fals").is_err(), "truncated false must be rejected");
        assert!(parse("truely").is_err(), "trailing chars must be rejected");
        // 正しいリテラルは通る。
        assert_eq!(parse("true").unwrap(), Value::Bool(true));
        assert_eq!(parse("false").unwrap(), Value::Bool(false));
        assert_eq!(parse("null").unwrap(), Value::Null);
        // 配列内でも厳密に照合される。
        assert!(
            parse("[nul,1]").is_err(),
            "malformed literal in array must fail"
        );
        assert_eq!(
            parse("[true,null,false]").unwrap(),
            arr([Value::Bool(true), Value::Null, Value::Bool(false)])
        );
    }

    #[test]
    fn empty_and_whitespace_only_input_is_rejected() {
        // 問166: 空文字列・空白のみの入力は値が存在しないので
        // parse_value_inner の peek が None になり "unexpected" エラーになる。
        // MCP フレームが空ボディを送っても黙って null 等に化けないことを固定。
        assert!(parse("").is_err(), "empty input must be rejected");
        assert!(
            parse("   ").is_err(),
            "whitespace-only input must be rejected"
        );
        assert!(
            parse("\n\t ").is_err(),
            "whitespace-only input must be rejected"
        );
        // エラーメッセージが pos 起点を含むこと (デバッグ可能性)。
        assert!(
            parse("").unwrap_err().contains("unexpected"),
            "empty input error must mention 'unexpected', got: {}",
            parse("").unwrap_err()
        );
    }

    #[test]
    fn array_with_leading_or_middle_comma_is_rejected() {
        // 問168: trailing_comma_is_rejected は [1,2,] のみ確認。
        // 先頭コンマ [,1] や中間コンマ [1,,2] も値欠落として拒否されることを固定。
        // parse_value が ',' を unexpected として弾く経路。
        assert!(parse("[,1]").is_err(), "leading comma must be rejected");
        assert!(
            parse("[1,,2]").is_err(),
            "middle double-comma must be rejected"
        );
        assert!(parse("[ , ]").is_err(), "comma-only array must be rejected");
        // 回帰: 正常な配列は通る。
        assert_eq!(
            parse("[1,2]").unwrap(),
            arr([Value::Number(1.0), Value::Number(2.0)])
        );
    }

    #[test]
    fn invalid_escape_is_rejected_not_silently_doubled() {
        // 問167: 未知エスケープ \q は以前 `other` 分岐で advance せずに push され、
        // 次ループで同じ文字が再 push されて "aqqb" のように二重化していた (バグ)。
        // 修正後は malformed literal 等と同様に厳密にエラーとなる。
        let err = parse(r#""a\qb""#).unwrap_err();
        assert!(
            err.contains("invalid escape"),
            "unknown escape \\q must be rejected, got: {err}"
        );
        // 有効なエスケープは引き続き正しく機能する (回帰防止)。
        assert_eq!(
            parse(r#""a\nb""#).unwrap().as_str(),
            Some("a\nb"),
            "\\n must still work"
        );
        assert_eq!(
            parse(r#""a\\b""#).unwrap().as_str(),
            Some("a\\b"),
            "\\\\ must still work"
        );
        assert_eq!(
            parse(r#""a\"b""#).unwrap().as_str(),
            Some("a\"b"),
            "\\\" must still work"
        );

        // バックスラッシュ直後が EOF の場合も明確にエラー (黙って壊れた値を返さない)。
        let backslash_eof = "\"a\\"; // 開きクオート + a + バックスラッシュ + EOF
        let err2 = parse(backslash_eof).unwrap_err();
        assert!(
            err2.contains("trailing backslash") || err2.contains("unterminated"),
            "backslash at EOF must error, got: {err2}"
        );
    }

    #[test]
    fn multibyte_utf8_strings_roundtrip() {
        // 問49: マルチバイト UTF-8 (日本語・絵文字・アクセント記号) が
        // パース→シリアライズで破壊されないこと。MCP クライアントは UTF-8 JSON を送る。
        for original in &[
            "日本語のテスト",
            "emoji 🎲 mix",
            "café résumé",
            "混在 ascii 123",
        ] {
            let parsed = parse(&format!(r#""{original}""#)).unwrap();
            assert_eq!(
                parsed.as_str(),
                Some(*original),
                "multibyte string must parse intact: {original}"
            );
            // ラウンドトリップ (値→文字列→値) も一致。
            let v = s(*original);
            assert_eq!(parse(&v.to_string()).unwrap(), v);
        }

        // オブジェクトのキー・値ともに UTF-8 を保持すること。
        let obj_v = obj([("名前", s("立方体 🧊"))]);
        let reparsed = parse(&obj_v.to_string()).unwrap();
        assert_eq!(obj_v, reparsed, "UTF-8 keys and values must roundtrip");
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

    #[test]
    fn nonfinite_numbers_serialize_as_valid_json_null() {
        // 問117: パーサは問20 で非有限を拒否するが、シリアライザは無防備だった。
        // 内部計算 (体積・寸法・角度等) が NaN/±Inf を生み Value::Number に入ると、
        // Display は "NaN"/"inf"/"-inf" を吐く — これは**不正な JSON** であり、
        // MCP 応答を受け取る AI クライアントのパーサを壊す。null に落として
        // 応答が常に valid JSON であることを保証する。
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let serialized = Value::Number(bad).to_string();
            assert_eq!(
                serialized, "null",
                "non-finite {bad} must serialize as null, got {serialized}"
            );
            // 出力が valid JSON として再パースできること (= クライアントが壊れない)。
            assert!(
                parse(&serialized).is_ok(),
                "non-finite serialization must be valid JSON"
            );
        }

        // 非有限が混入したオブジェクト全体も valid JSON のままであること。
        let v = obj([
            ("vol", n(f64::NAN)),
            ("dim", n(f64::INFINITY)),
            ("ok", n(1.5)),
        ]);
        let serialized = v.to_string();
        let reparsed = parse(&serialized).expect("object with non-finite must stay valid JSON");
        // 有限値は保持され、非有限は null になる。
        assert_eq!(reparsed.get("ok").and_then(|x| x.as_f64()), Some(1.5));
        assert!(reparsed.get("vol").map(|x| x.is_null()).unwrap_or(false));
        assert!(reparsed.get("dim").map(|x| x.is_null()).unwrap_or(false));
    }

    #[test]
    fn negative_zero_serializes_as_zero_and_is_numerically_equivalent() {
        // 問226: Display は n.fract()==0.0 && n.abs()<1e15 で整数出力するため
        // -0.0 は "0" になり符号ビットが失われる。-0.0 == +0.0 なので幾何・算術に
        // 影響はなく、出力は決定的 (-0.0 は常に "0")。この意図された良性挙動を固定する。
        let serialized = Value::Number(-0.0_f64).to_string();
        assert_eq!(
            serialized, "0",
            "-0.0 must serialize as \"0\" (sign-bit loss is benign)"
        );
        // 再パースは +0.0 になり、数値的に 0.0 と等価。
        let reparsed = parse(&serialized).unwrap().as_f64().unwrap();
        assert_eq!(reparsed, 0.0, "re-parsed value must equal 0.0");
        // 出力は決定的 (同一入力で同一出力)。
        assert_eq!(Value::Number(-0.0).to_string(), serialized);
    }

    #[test]
    fn scientific_notation_input_roundtrips_bit_identically_via_decimal() {
        // 問227: パーサは科学記法 (1.5e-3) を受理するが Display は十進 (0.0015) で出力する。
        // 文字列形式は変わるが f64 のビット列は保存される (Rust の Display↔parse 往復保証)。
        // AI が科学記法を送っても数値が壊れないことを固定する。
        for input in ["1.5e-3", "2.0e5", "1e-10", "3.14e2", "6.022e23"] {
            let parsed = parse(&format!(r#"{{"n":{input}}}"#))
                .unwrap()
                .get("n")
                .and_then(|v| v.as_f64())
                .unwrap();
            let serialized = Value::Number(parsed).to_string();
            let reparsed = parse(&serialized).unwrap().as_f64().unwrap();
            assert_eq!(
                parsed.to_bits(),
                reparsed.to_bits(),
                "scientific {input}: {parsed} → \"{serialized}\" → {reparsed} must be bit-identical"
            );
        }
    }

    #[test]
    fn trailing_comma_is_rejected() {
        // 問144: JSON 仕様は配列・オブジェクトの末尾カンマを禁止する。
        // 現パーサはカンマ後に再度 parse_value() を呼ぶ仕組み上自然に拒否するが、
        // この動作が退行で壊れないことを明示的に固定する。
        assert!(
            parse("[1,2,]").is_err(),
            "trailing comma in array must be rejected"
        );
        assert!(
            parse(r#"{"a":1,}"#).is_err(),
            "trailing comma in object must be rejected"
        );
        // 正常系 (末尾カンマなし) は通る。
        assert!(parse("[1,2]").is_ok());
        assert!(parse(r#"{"a":1}"#).is_ok());
    }

    #[test]
    fn duplicate_object_keys_last_wins() {
        // 問145: JSON オブジェクトに重複キーが含まれる場合、BTreeMap::insert により
        // 後の値が前の値を無音で上書きする (last-wins)。
        // MCP リクエストに重複フィールドが混入した際の動作を明示的に固定する。
        let v = parse(r#"{"a":1,"b":2,"a":99}"#).expect("must parse");
        // 後の "a":99 が前の "a":1 を上書きしていること。
        assert_eq!(
            v.get("a").and_then(|x| x.as_f64()),
            Some(99.0),
            "duplicate key must keep last value"
        );
        assert_eq!(v.get("b").and_then(|x| x.as_f64()), Some(2.0));
    }

    #[test]
    fn unicode_surrogate_escape_becomes_replacement_char() {
        // 問147: \uXXXX エスケープで UTF-16 サロゲートコードポイント (U+D800-U+DFFF) を
        // 渡すと char::from_u32 が None を返し U+FFFD (置換文字) に変換される。
        // RFC 8259 はサロゲートを許容するが char としては無効なため置換が安全側。
        // この動作がパニックせず定義済みであることを固定する。
        let lone = parse(r#""\uD800""#).expect("must not panic on lone surrogate");
        assert_eq!(
            lone.as_str(),
            Some("\u{FFFD}"),
            "lone surrogate must become replacement char"
        );
        // UTF-16 サロゲートペア符号化: 😀 は U+1F600 (😀) の UTF-16 表現。
        // 各 \uXXXX は独立処理され結合されず、2つの U+FFFD になる。
        // (実際の 😀 をリテラルで入れると UTF-8 経路になるため \u エスケープで入力する)
        let pair = parse("\"\\uD83D\\uDE00\"").expect("must not panic on surrogate pair");
        assert_eq!(
            pair.as_str(),
            Some("\u{FFFD}\u{FFFD}"),
            "surrogate pair must become two replacement chars (not combined emoji)"
        );
    }
}
