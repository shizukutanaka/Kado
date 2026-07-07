//! KadoScene テキスト DSL (問59)。
//!
//! JSON より簡潔な関数呼び出し式でシーンを記述する。例:
//! ```text
//! difference(union(sphere(1), cuboid(0.8)), cylinder(0.3, 2))
//! translate(2, 0, 0, rotate_z(45, cylinder(0.3, 1)))
//! ```
//!
//! 設計: DSL は**表層構文**であり、解析結果は JSON と同一の KadoScene [`Value`] 木へ
//! 落ちる。意味論・検証・リソース上限は [`eval_value`] に
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

    let obj = |pairs: Vec<(&'static str, Value)>| {
        Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    };

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
        // prism(n, r, h): 正多角形プリズム (問269)。
        "prism" => {
            want(3)?;
            Ok(obj(vec![
                ("op", json::s("prism")),
                ("n", a(0)),
                ("r", a(1)),
                ("h", a(2)),
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
        // scale_xyz(sx, sy, sz, shape): 非一様スケール (問276)。
        "scale_xyz" => {
            want(4)?;
            Ok(obj(vec![
                ("op", json::s("scale_xyz")),
                ("sx", a(0)),
                ("sy", a(1)),
                ("sz", a(2)),
                ("shape", a(3)),
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
        // rotate(ax, ay, az, angle, shape): 任意軸周り回転 (問266)。
        // rotate_x/y/z の canonical 3軸だけでは表現できない対角線等の軸を
        // オイラー角の逆算なしに1操作で指定できる。
        "rotate" => {
            want(5)?;
            Ok(obj(vec![
                ("op", json::s("rotate")),
                ("ax", a(0)),
                ("ay", a(1)),
                ("az", a(2)),
                ("angle", a(3)),
                ("shape", a(4)),
            ]))
        }
        "mirror_x" | "mirror_y" | "mirror_z" => {
            want(1)?;
            Ok(obj(vec![("op", json::s(name)), ("shape", a(0))]))
        }
        // cut(nx,ny,nz, shape) [offset=0] または cut(nx,ny,nz, offset, shape)。
        // dot(p,(nx,ny,nz)) <= offset の側を残す (shape は最後の引数)。
        "cut" => match args.len() {
            4 => Ok(obj(vec![
                ("op", json::s("cut")),
                ("nx", a(0)),
                ("ny", a(1)),
                ("nz", a(2)),
                ("shape", a(3)),
            ])),
            5 => Ok(obj(vec![
                ("op", json::s("cut")),
                ("nx", a(0)),
                ("ny", a(1)),
                ("nz", a(2)),
                ("offset", a(3)),
                ("shape", a(4)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"cut\" expects 4 (nx,ny,nz,shape) or 5 (nx,ny,nz,offset,shape) args, got {n}"
            ))),
        },
        // flatten(shape) [at=0] または flatten(at, shape)。z=at で底を切り z>=at を残す。
        "flatten" => match args.len() {
            1 => Ok(obj(vec![("op", json::s("flatten")), ("shape", a(0))])),
            2 => Ok(obj(vec![
                ("op", json::s("flatten")),
                ("at", a(0)),
                ("shape", a(1)),
            ])),
            n => Err(ScriptError::new(format!(
                "\"flatten\" expects 1 (shape) or 2 (at,shape) args, got {n}"
            ))),
        },
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
    fn all_ops_parse_identically_in_dsl_and_json() {
        // 問104: JSON 評価器とテキスト DSL は同一の op 集合を扱い、各 op で同一の場を
        // 生まなければならない。片方にしか無い op があると、AI が一方の記法で書いた
        // ときだけ "unknown" になる隠れた非対称が生じる。全 op の DSL↔JSON 等価性を
        // 固定し、将来 op を片側だけに追加する退行を検知する。
        // プリミティブ (9; prism 問269 追加)
        assert_same("sphere(1.0)", r#"{"op":"sphere","r":1.0}"#);
        assert_same("cuboid(0.8)", r#"{"op":"cuboid","x":0.8,"y":0.8,"z":0.8}"#);
        assert_same("cylinder(0.5, 1.0)", r#"{"op":"cylinder","r":0.5,"h":1.0}"#);
        assert_same(
            "torus(1.0, 0.25)",
            r#"{"op":"torus","major":1.0,"minor":0.25}"#,
        );
        assert_same("cone(0.5, 1.0)", r#"{"op":"cone","r":0.5,"h":1.0}"#);
        assert_same("capsule(0.5, 0.3)", r#"{"op":"capsule","h":0.5,"r":0.3}"#);
        assert_same(
            "rounded_box(0.8, 0.1)",
            r#"{"op":"rounded_box","x":0.8,"y":0.8,"z":0.8,"r":0.1}"#,
        );
        assert_same(
            "ellipsoid(2, 1, 0.5)",
            r#"{"op":"ellipsoid","x":2,"y":1,"z":0.5}"#,
        );
        assert_same(
            "prism(6, 1.0, 0.5)",
            r#"{"op":"prism","n":6,"r":1.0,"h":0.5}"#,
        );

        // ブーリアン (6): a,b はそれぞれ離れた球で領域差が出るようにする。
        let a = r#"{"op":"sphere","r":1}"#;
        let b = r#"{"op":"translate","x":0.5,"y":0,"z":0,"shape":{"op":"sphere","r":1}}"#;
        let da = "sphere(1)";
        let db = "translate(0.5, 0, 0, sphere(1))";
        assert_same(
            &format!("union({da}, {db})"),
            &format!(r#"{{"op":"union","a":{a},"b":{b}}}"#),
        );
        assert_same(
            &format!("intersection({da}, {db})"),
            &format!(r#"{{"op":"intersection","a":{a},"b":{b}}}"#),
        );
        assert_same(
            &format!("difference({da}, {db})"),
            &format!(r#"{{"op":"difference","a":{a},"b":{b}}}"#),
        );
        assert_same(
            &format!("smooth_union({da}, {db}, 0.3)"),
            &format!(r#"{{"op":"smooth_union","k":0.3,"a":{a},"b":{b}}}"#),
        );
        assert_same(
            &format!("smooth_intersection({da}, {db}, 0.3)"),
            &format!(r#"{{"op":"smooth_intersection","k":0.3,"a":{a},"b":{b}}}"#),
        );
        assert_same(
            &format!("smooth_difference({da}, {db}, 0.3)"),
            &format!(r#"{{"op":"smooth_difference","k":0.3,"a":{a},"b":{b}}}"#),
        );

        // 変形 (13; rotate 問266・scale_xyz 問276 追加)
        assert_same(
            "translate(1, 0.5, -0.5, sphere(1))",
            r#"{"op":"translate","x":1,"y":0.5,"z":-0.5,"shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "scale(2, sphere(1))",
            r#"{"op":"scale","s":2,"shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "scale_xyz(2, 1, 0.5, sphere(1))",
            r#"{"op":"scale_xyz","sx":2,"sy":1,"sz":0.5,"shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "offset(0.1, sphere(1))",
            r#"{"op":"offset","amount":0.1,"shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "shell(0.1, sphere(1))",
            r#"{"op":"shell","thickness":0.1,"shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "repeat(2, 2, 2, sphere(0.3))",
            r#"{"op":"repeat","x":2,"y":2,"z":2,"shape":{"op":"sphere","r":0.3}}"#,
        );
        for axis in ["x", "y", "z"] {
            assert_same(
                &format!("mirror_{axis}(translate(1,0,0, sphere(0.3)))"),
                &format!(
                    r#"{{"op":"mirror_{axis}","shape":{{"op":"translate","x":1,"y":0,"z":0,"shape":{{"op":"sphere","r":0.3}}}}}}"#
                ),
            );
            assert_same(
                &format!("rotate_{axis}(45, cuboid(1, 0.5, 0.3))"),
                &format!(
                    r#"{{"op":"rotate_{axis}","angle":45,"shape":{{"op":"cuboid","x":1,"y":0.5,"z":0.3}}}}"#
                ),
            );
        }
        // rotate: 任意軸周り回転 (問266)。
        assert_same(
            "rotate(1, 1, 0, 45, cuboid(1, 0.5, 0.3))",
            r#"{"op":"rotate","ax":1,"ay":1,"az":0,"angle":45,
               "shape":{"op":"cuboid","x":1,"y":0.5,"z":0.3}}"#,
        );

        // cut: 4 引数 (offset 省略) と 5 引数 (offset 明示) の両形式。
        assert_same(
            "cut(0, 0, -1, sphere(1))",
            r#"{"op":"cut","nx":0,"ny":0,"nz":-1,"shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "cut(0, 0, -1, 0.5, sphere(1))",
            r#"{"op":"cut","nx":0,"ny":0,"nz":-1,"offset":0.5,"shape":{"op":"sphere","r":1}}"#,
        );
        // flatten: 1 引数 (at=0) と 2 引数 (at 明示)。
        assert_same(
            "flatten(sphere(1))",
            r#"{"op":"flatten","shape":{"op":"sphere","r":1}}"#,
        );
        assert_same(
            "flatten(0.3, sphere(1))",
            r#"{"op":"flatten","at":0.3,"shape":{"op":"sphere","r":1}}"#,
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

    #[test]
    fn bare_numeric_top_level_is_rejected() {
        // 問151: DSL の parse_expr は数値も有効構文として受け入れる (引数文脈のため)。
        // しかしトップレベルの裸の数値 "1.5" は eval_value で "missing op field" になる。
        // AI エージェントが "1.5" や "-3.14" を形状式として送った場合に
        // 明確なエラーが返ることを固定する。
        assert!(
            eval_dsl("1.5").is_err(),
            "bare positive number must be rejected"
        );
        assert!(eval_dsl("0").is_err(), "bare zero must be rejected");
        assert!(
            eval_dsl("-3.14").is_err(),
            "bare negative number must be rejected"
        );
    }

    #[test]
    fn unknown_operator_rejected_both_at_top_and_nested() {
        // 問155: 未知の演算子はトップレベルだけでなく入れ子でも拒否される。
        // 既存テストは `wobble(1)` (トップレベル) のみ確認。
        // `union(wobble(1), sphere(1))` のように有効な呼び出しの引数に
        // 未知演算子が含まれる場合もエラーが伝播することを固定する。
        assert!(
            eval_dsl("sphire(1)").is_err(),
            "typo at top level must be rejected"
        );
        assert!(
            eval_dsl("union(sphire(1), sphere(1))").is_err(),
            "typo nested in union must be rejected"
        );
        assert!(
            eval_dsl("translate(0,0,0, wobble(1))").is_err(),
            "unknown op nested in translate must be rejected"
        );
    }

    #[test]
    fn function_call_with_zero_arguments_is_rejected() {
        // 問175: 既知の演算子でも引数 0 個の呼び出し sphere() は
        // want(n) ガードで "expects N argument(s), got 0" になり拒否される。
        // AI が引数を忘れた呼び出しを送っても黙って既定値で評価しないことを固定。
        let err = eval_dsl("sphere()").expect_err("sphere() with no args must be rejected");
        assert!(
            err.to_string().contains("got 0"),
            "zero-arg error must report 'got 0', message: {err}"
        );
        // 他の固定アリティ演算子も同様に拒否される。
        assert!(eval_dsl("cuboid()").is_err(), "cuboid() must be rejected");
        assert!(eval_dsl("union()").is_err(), "union() must be rejected");
        assert!(
            eval_dsl("translate()").is_err(),
            "translate() must be rejected"
        );
    }

    #[test]
    fn rounded_box_wrong_arity_gives_clear_error() {
        // 問195: rounded_box は 2 (uniform+r) または 4 (x,y,z,r) 引数のみ有効。
        // 1 引数や 3 引数は "expects 2 or 4 args, got N" エラーになる。
        // function_call_with_zero_arguments_is_rejected は 0 引数のみ確認しており
        // 中間アリティの多アリティ演算子 (rounded_box) の拒否は未固定だった。
        let err1 = eval_dsl("rounded_box(0.5)").expect_err("1-arg rounded_box must be rejected");
        assert!(
            err1.to_string().contains("got 1"),
            "1-arg error must mention 'got 1': {err1}"
        );
        let err3 =
            eval_dsl("rounded_box(1.0, 0.8, 0.6)").expect_err("3-arg rounded_box must be rejected");
        assert!(
            err3.to_string().contains("got 3"),
            "3-arg error must mention 'got 3': {err3}"
        );
        // 有効なアリティは通る (回帰防止)。
        assert!(
            eval_dsl("rounded_box(1.0, 0.1)").is_ok(),
            "2-arg rounded_box must succeed"
        );
        assert!(
            eval_dsl("rounded_box(1.0, 0.8, 0.6, 0.1)").is_ok(),
            "4-arg rounded_box must succeed"
        );
    }
}
