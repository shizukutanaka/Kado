//! KadoScene テキスト DSL (問59)。
//!
//! JSON より簡潔な関数呼び出し式でシーンを記述する。例:
//! ```text
//! difference(union(sphere(1), cuboid(0.8)), cylinder(0.3, 2))
//! translate(2, 0, 0, rotate_z(45, cylinder(0.3, 1)))
//! ```
//!
//! 設計: DSL は**表層構文**であり、解析結果は JSON と同一の KadoScene [`Value`] 木へ
//! 落ちる。意味論・検証・リソース上限は [`eval_value`](super::eval::eval_value) に
//! 一元化され、DSL 側は構文解析と引数→キー対応のみを担う (重複ゼロ)。
//!
//! セキュリティ (Plan リスク E): ソースサイズ・ネスト深さに上限。任意コード実行なし。

use crate::core::Sdf;
use crate::mcp::json::{self, Value};
use crate::script::eval::{eval_value, ScriptError, DSL_MAX_DEPTH, DSL_MAX_SOURCE_BYTES};

/// テキスト DSL 文字列を評価して [`Sdf`] を返す。
pub fn eval_dsl(source: &str) -> Result<Sdf, ScriptError> {
    let v = parse_dsl(source)?;
    eval_value(&v)
}

/// テキスト DSL を KadoScene [`Value`] 木へ解析する (評価はしない)。
pub fn parse_dsl(source: &str) -> Result<Value, ScriptError> {
    if source.len() > DSL_MAX_SOURCE_BYTES {
        return Err(ScriptError::new(format!(
            "script too large ({} bytes > {DSL_MAX_SOURCE_BYTES})",
            source.len()
        )));
    }
    let mut p = Parser {
        s: source.as_bytes(),
        pos: 0,
        depth: 0,
    };
    p.skip_ws();
    let v = p.parse_expr()?;
    p.skip_ws();
    if p.pos != p.s.len() {
        return Err(ScriptError::new(format!(
            "trailing characters at position {}",
            p.pos
        )));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<Value, ScriptError> {
        self.depth += 1;
        if self.depth > DSL_MAX_DEPTH {
            return Err(ScriptError::new(format!(
                "expression nested too deep (> {DSL_MAX_DEPTH})"
            )));
        }
        self.skip_ws();
        let r = match self.peek() {
            Some(c) if c == b'-' || c == b'+' || c == b'.' || c.is_ascii_digit() => {
                self.parse_number()
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => self.parse_call(),
            other => Err(ScriptError::new(format!(
                "unexpected {:?} at position {}",
                other.map(|b| b as char),
                self.pos
            ))),
        };
        self.depth -= 1;
        r
    }

    fn parse_number(&mut self) -> Result<Value, ScriptError> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'-' | b'+')) {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.pos]).unwrap_or("");
        let val: f64 = text
            .parse()
            .map_err(|_| ScriptError::new(format!("invalid number \"{text}\"")))?;
        // 非有限を拒否 (問20): inf/NaN が距離場へ伝播するのを防ぐ。
        if !val.is_finite() {
            return Err(ScriptError::new(format!("non-finite number \"{text}\"")));
        }
        Ok(json::n(val))
    }

    fn parse_ident(&mut self) -> String {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_') {
            self.pos += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.pos]).into_owned()
    }

    fn parse_call(&mut self) -> Result<Value, ScriptError> {
        let name = self.parse_ident();
        self.skip_ws();
        if self.peek() != Some(b'(') {
            return Err(ScriptError::new(format!(
                "expected '(' after \"{name}\" at position {}",
                self.pos
            )));
        }
        self.pos += 1; // consume '('
        let mut args = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b')') {
            self.pos += 1;
        } else {
            loop {
                args.push(self.parse_expr()?);
                self.skip_ws();
                match self.peek() {
                    Some(b',') => {
                        self.pos += 1;
                    }
                    Some(b')') => {
                        self.pos += 1;
                        break;
                    }
                    other => {
                        return Err(ScriptError::new(format!(
                            "expected ',' or ')' in \"{name}\" args, got {:?} at {}",
                            other.map(|b| b as char),
                            self.pos
                        )))
                    }
                }
            }
        }
        build_call(&name, args)
    }
}

/// `name(args...)` を KadoScene [`Value`] オブジェクトへ対応づける。
/// 位置引数→キー名の対応のみを行い、値の妥当性は `eval_value` 側で検査する。
fn build_call(name: &str, args: Vec<Value>) -> Result<Value, ScriptError> {
    // 引数個数の検査ヘルパ。
    let want = |n: usize| -> Result<(), ScriptError> {
        if args.len() == n {
            Ok(())
        } else {
            Err(ScriptError::new(format!(
                "\"{name}\" expects {n} argument(s), got {}",
                args.len()
            )))
        }
    };
    let a = |i: usize| args[i].clone();

    let obj = |pairs: Vec<(&'static str, Value)>| Value::Object(
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
    );

    match name {
        // ── プリミティブ ──
        "sphere" => {
            want(1)?;
            Ok(obj(vec![("op", json::s("sphere")), ("r", a(0))]))
        }
        "cuboid" => match args.len() {
            1 => Ok(obj(vec![
                ("op", json::s("cuboid")),
                ("x", a(0)),
                ("y", a(0)),
                ("z", a(0)),
            ])),
            3 => Ok(obj(vec![
                ("op", json::s("cuboid")),
                ("x", a(0)),
                ("y", a(1)),
                ("z", a(2)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"cuboid\" expects 1 (uniform) or 3 args, got {n}"
            ))),
        },
        "cylinder" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s("cylinder")),
                ("r", a(0)),
                ("h", a(1)),
            ]))
        }
        "torus" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s("torus")),
                ("major", a(0)),
                ("minor", a(1)),
            ]))
        }
        "cone" => {
            want(2)?;
            Ok(obj(vec![("op", json::s("cone")), ("r", a(0)), ("h", a(1))]))
        }
        "capsule" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s("capsule")),
                ("h", a(0)),
                ("r", a(1)),
            ]))
        }
        "rounded_box" => match args.len() {
            2 => Ok(obj(vec![
                ("op", json::s("rounded_box")),
                ("x", a(0)),
                ("y", a(0)),
                ("z", a(0)),
                ("r", a(1)),
            ])),
            4 => Ok(obj(vec![
                ("op", json::s("rounded_box")),
                ("x", a(0)),
                ("y", a(1)),
                ("z", a(2)),
                ("r", a(3)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"rounded_box\" expects 2 (uniform+r) or 4 (x,y,z,r) args, got {n}"
            ))),
        },
        "ellipsoid" => match args.len() {
            1 => Ok(obj(vec![
                ("op", json::s("ellipsoid")),
                ("x", a(0)),
                ("y", a(0)),
                ("z", a(0)),
            ])),
            3 => Ok(obj(vec![
                ("op", json::s("ellipsoid")),
                ("x", a(0)),
                ("y", a(1)),
                ("z", a(2)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"ellipsoid\" expects 1 (uniform) or 3 args, got {n}"
            ))),
        },

        // ── ブーリアン ──
        "union" | "intersection" | "difference" => {
            want(2)?;
            Ok(obj(vec![("op", json::s(name)), ("a", a(0)), ("b", a(1))]))
        }
        "smooth_union" | "smooth_intersection" | "smooth_difference" => match args.len() {
            2 => Ok(obj(vec![("op", json::s(name)), ("a", a(0)), ("b", a(1))])),
            3 => Ok(obj(vec![
                ("op", json::s(name)),
                ("a", a(0)),
                ("b", a(1)),
                ("k", a(2)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"{name}\" expects 2 or 3 (with k) args, got {n}"
            ))),
        },

        // ── 変形 (shape は末尾引数) ──
        "translate" => {
            want(4)?;
            Ok(obj(vec![
                ("op", json::s("translate")),
                ("x", a(0)),
                ("y", a(1)),
                ("z", a(2)),
                ("shape", a(3)),
            ]))
        }
        "scale" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s("scale")),
                ("s", a(0)),
                ("shape", a(1)),
            ]))
        }
        "offset" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s("offset")),
                ("amount", a(0)),
                ("shape", a(1)),
            ]))
        }
        "shell" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s("shell")),
                ("thickness", a(0)),
                ("shape", a(1)),
            ]))
        }
        "rotate_x" | "rotate_y" | "rotate_z" => {
            want(2)?;
            Ok(obj(vec![
                ("op", json::s(name)),
                ("angle", a(0)),
                ("shape", a(1)),
            ]))
        }
        "mirror_x" | "mirror_y" | "mirror_z" => {
            want(1)?;
            Ok(obj(vec![("op", json::s(name)), ("shape", a(0))]))
        }
        "repeat" => match args.len() {
            // repeat(px,py,pz, shape) — 各軸 count 既定 1。
            4 => Ok(obj(vec![
                ("op", json::s("repeat")),
                ("x", a(0)),
                ("y", a(1)),
                ("z", a(2)),
                ("shape", a(3)),
            ])),
            // repeat(px,py,pz, nx,ny,nz, shape)。
            7 => Ok(obj(vec![
                ("op", json::s("repeat")),
                ("x", a(0)),
                ("y", a(1)),
                ("z", a(2)),
                ("nx", a(3)),
                ("ny", a(4)),
                ("nz", a(5)),
                ("shape", a(6)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"repeat\" expects 4 (px,py,pz,shape) or 7 (+nx,ny,nz) args, got {n}"
            ))),
        },

        other => Err(ScriptError::new(format!("unknown function \"{other}\""))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Vec3;
    use crate::script::eval::eval_scene;

    /// DSL と等価 JSON が同一の場を生むことを確認する。
    fn assert_same(dsl: &str, json: &str) {
        let a = eval_dsl(dsl).unwrap_or_else(|e| panic!("DSL failed: {} ({e})", dsl));
        let b = eval_scene(json).unwrap();
        for p in [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.5, 0.3, -0.2),
            Vec3::new(1.2, -0.7, 0.9),
            Vec3::new(-1.0, 1.0, 1.0),
        ] {
            assert!(
                (a.eval(p) - b.eval(p)).abs() < 1e-12,
                "DSL {dsl} != JSON at {p:?}: {} vs {}",
                a.eval(p),
                b.eval(p)
            );
        }
    }

    #[test]
    fn primitives_match_json() {
        assert_same("sphere(1.0)", r#"{"op":"sphere","r":1.0}"#);
        assert_same("cuboid(0.8)", r#"{"op":"cuboid","x":0.8,"y":0.8,"z":0.8}"#);
        assert_same(
            "cuboid(1, 0.5, 0.3)",
            r#"{"op":"cuboid","x":1,"y":0.5,"z":0.3}"#,
        );
        assert_same("cylinder(0.3, 2)", r#"{"op":"cylinder","r":0.3,"h":2}"#);
        assert_same("torus(1, 0.25)", r#"{"op":"torus","major":1,"minor":0.25}"#);
        assert_same(
            "ellipsoid(2, 1, 0.5)",
            r#"{"op":"ellipsoid","x":2,"y":1,"z":0.5}"#,
        );
    }

    #[test]
    fn nested_booleans_and_transforms_match_json() {
        assert_same(
            "difference(union(sphere(1), cuboid(0.8)), cylinder(0.3, 2))",
            r#"{"op":"difference","a":{"op":"union","a":{"op":"sphere","r":1},
               "b":{"op":"cuboid","x":0.8,"y":0.8,"z":0.8}},
               "b":{"op":"cylinder","r":0.3,"h":2}}"#,
        );
        assert_same(
            "translate(2, 0, 0, rotate_z(90, cylinder(0.3, 1)))",
            r#"{"op":"translate","x":2,"y":0,"z":0,
               "shape":{"op":"rotate_z","angle":90,"shape":{"op":"cylinder","r":0.3,"h":1}}}"#,
        );
        assert_same(
            "smooth_union(sphere(1), translate(1,0,0, sphere(1)), 0.3)",
            r#"{"op":"smooth_union","k":0.3,"a":{"op":"sphere","r":1},
               "b":{"op":"translate","x":1,"y":0,"z":0,"shape":{"op":"sphere","r":1}}}"#,
        );
    }

    #[test]
    fn negative_and_decimal_numbers_parse() {
        assert_same(
            "translate(-1.5, 0, 0, sphere(0.5))",
            r#"{"op":"translate","x":-1.5,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}}"#,
        );
    }

    #[test]
    fn whitespace_is_ignored() {
        assert_same(
            "  union(\n  sphere(1) ,\tcuboid(0.5)\n)  ",
            r#"{"op":"union","a":{"op":"sphere","r":1},"b":{"op":"cuboid","x":0.5,"y":0.5,"z":0.5}}"#,
        );
    }

    #[test]
    fn malformed_dsl_is_rejected() {
        assert!(eval_dsl("sphere(").is_err(), "unclosed paren");
        assert!(eval_dsl("sphere 1)").is_err(), "missing paren");
        assert!(eval_dsl("sphere(1) extra").is_err(), "trailing");
        assert!(eval_dsl("wobble(1)").is_err(), "unknown function");
        assert!(eval_dsl("cylinder(1)").is_err(), "wrong arity");
        assert!(eval_dsl("").is_err(), "empty");
        assert!(eval_dsl("sphere(1e400)").is_err(), "non-finite");
    }

    #[test]
    fn invalid_values_rejected_by_shared_semantics() {
        // 検証は eval_value 共有: DSL でも r<=0 や scale<=0 が同じく弾かれる。
        assert!(eval_dsl("sphere(0)").is_err(), "r=0 via shared check");
        assert!(eval_dsl("sphere(-1)").is_err(), "r<0");
        assert!(
            eval_dsl("scale(0, sphere(1))").is_err(),
            "scale=0 via shared check"
        );
    }

    #[test]
    fn deeply_nested_dsl_is_rejected_not_overflowed() {
        // 問16: 病的ネストを上限で拒否しスタックを溢れさせない。
        let n = DSL_MAX_DEPTH + 50;
        let mut s = String::new();
        for _ in 0..n {
            s.push_str("translate(0,0,0,");
        }
        s.push_str("sphere(1)");
        for _ in 0..n {
            s.push(')');
        }
        assert!(eval_dsl(&s).is_err(), "over-deep DSL must be rejected");
    }
}
