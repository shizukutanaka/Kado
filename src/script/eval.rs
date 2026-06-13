//! KadoScene JSON 評価器。
//!
//! # 形式
//!
//! ```json
//! { "op": "difference",
//!   "a": { "op": "union",
//!          "a": { "op": "sphere", "r": 1.0 },
//!          "b": { "op": "cuboid", "x": 0.8, "y": 0.8, "z": 0.8 } },
//!   "b": { "op": "cylinder", "r": 0.3, "h": 2.0 } }
//! ```
//!
//! 演算子一覧 (op 文字列):
//! プリミティブ: sphere, cuboid, cylinder, torus, cone, capsule, rounded_box
//! ブーリアン:   union, intersection, difference, smooth_union, smooth_difference
//! 変形:         translate, scale, offset, shell, repeat, mirror_x, mirror_y, mirror_z

use crate::core::{Sdf, Vec3};
use crate::mcp::json::{parse, Value};

#[derive(Debug)]
pub struct ScriptError {
    pub message: String,
}

impl ScriptError {
    fn new(s: impl Into<String>) -> ScriptError {
        ScriptError { message: s.into() }
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// スクリプトソースの最大バイト数 (リソース上限・問16)。
const MAX_SOURCE_BYTES: usize = 1 << 20; // 1 MiB
/// SDF木の最大ノード数 (リソース上限・問16)。
const MAX_NODES: usize = 50_000;
/// SDF木の最大深さ (リソース上限・問16)。パーサ側の深さ上限と二重防御。
const MAX_DEPTH: usize = 64;

/// 評価予算 (DoS 防止のリソース上限・問16)。
struct Budget {
    nodes: usize,
}

/// JSON 文字列を KadoScene として評価して [`Sdf`] 木を返す。
///
/// セキュリティ (Plan リスク E): ソースサイズ・ノード数・深さに上限を課す。
pub fn eval_scene(source: &str) -> Result<Sdf, ScriptError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(ScriptError::new(format!(
            "script too large ({} bytes > {MAX_SOURCE_BYTES})",
            source.len()
        )));
    }
    let v = parse(source).map_err(|e| ScriptError::new(format!("JSON parse error: {e}")))?;
    let mut budget = Budget { nodes: 0 };
    build(&v, 0, &mut budget)
}

fn build(v: &Value, depth: usize, budget: &mut Budget) -> Result<Sdf, ScriptError> {
    if depth > MAX_DEPTH {
        return Err(ScriptError::new(format!(
            "scene tree too deep (> {MAX_DEPTH})"
        )));
    }
    budget.nodes += 1;
    if budget.nodes > MAX_NODES {
        return Err(ScriptError::new(format!(
            "scene tree too large (> {MAX_NODES} nodes)"
        )));
    }
    let op = v
        .get("op")
        .and_then(|x| x.as_str())
        .ok_or_else(|| ScriptError::new("missing \"op\" field"))?;

    match op {
        // ── プリミティブ ──────────────────────────────────────────────────────
        "sphere" => {
            let r = req_f64(v, "r")?;
            Ok(Sdf::sphere(r))
        }
        "cuboid" => {
            let x = opt_f64(v, "x").or_else(|| opt_f64(v, "s")).unwrap_or(1.0);
            let y = opt_f64(v, "y").or_else(|| opt_f64(v, "s")).unwrap_or(x);
            let z = opt_f64(v, "z").or_else(|| opt_f64(v, "s")).unwrap_or(x);
            Ok(Sdf::cuboid(Vec3::new(x, y, z)))
        }
        "cylinder" => {
            let r = req_f64(v, "r")?;
            let h = req_f64(v, "h")?;
            Ok(Sdf::cylinder(r, h))
        }
        "torus" => {
            let major = req_f64(v, "major")?;
            let minor = req_f64(v, "minor")?;
            Ok(Sdf::torus(major, minor))
        }
        "cone" => {
            let r = req_f64(v, "r")?;
            let h = req_f64(v, "h")?;
            Ok(Sdf::cone(r, h))
        }
        "capsule" => {
            let h = req_f64(v, "h")?;
            let r = req_f64(v, "r")?;
            Ok(Sdf::capsule(h, r))
        }
        "rounded_box" => {
            let x = req_f64(v, "x")?;
            let y = opt_f64(v, "y").unwrap_or(x);
            let z = opt_f64(v, "z").unwrap_or(x);
            let r = req_f64(v, "r")?;
            Ok(Sdf::rounded_box(Vec3::new(x, y, z), r))
        }

        // ── ブーリアン ────────────────────────────────────────────────────────
        "union" => {
            let a = build(req_child(v, "a")?, depth + 1, budget)?;
            let b = build(req_child(v, "b")?, depth + 1, budget)?;
            Ok(a.union(b))
        }
        "intersection" => {
            let a = build(req_child(v, "a")?, depth + 1, budget)?;
            let b = build(req_child(v, "b")?, depth + 1, budget)?;
            Ok(a.intersection(b))
        }
        "difference" => {
            let a = build(req_child(v, "a")?, depth + 1, budget)?;
            let b = build(req_child(v, "b")?, depth + 1, budget)?;
            Ok(a.difference(b))
        }
        "smooth_union" => {
            let a = build(req_child(v, "a")?, depth + 1, budget)?;
            let b = build(req_child(v, "b")?, depth + 1, budget)?;
            let k = opt_f64(v, "k").unwrap_or(0.3);
            Ok(a.smooth_union(b, k))
        }
        "smooth_difference" => {
            let a = build(req_child(v, "a")?, depth + 1, budget)?;
            let b = build(req_child(v, "b")?, depth + 1, budget)?;
            let k = opt_f64(v, "k").unwrap_or(0.3);
            Ok(a.smooth_difference(b, k))
        }

        // ── 変形 ──────────────────────────────────────────────────────────────
        "translate" => {
            let child = build(req_child(v, "shape")?, depth + 1, budget)?;
            let tx = opt_f64(v, "x").unwrap_or(0.0);
            let ty = opt_f64(v, "y").unwrap_or(0.0);
            let tz = opt_f64(v, "z").unwrap_or(0.0);
            Ok(child.translate(Vec3::new(tx, ty, tz)))
        }
        "scale" => {
            let child = build(req_child(v, "shape")?, depth + 1, budget)?;
            let s = req_f64(v, "s")?;
            Ok(child.scale(s))
        }
        "offset" => {
            let child = build(req_child(v, "shape")?, depth + 1, budget)?;
            let a = req_f64(v, "amount")?;
            Ok(child.offset(a))
        }
        "shell" => {
            let child = build(req_child(v, "shape")?, depth + 1, budget)?;
            let t = req_f64(v, "thickness")?;
            Ok(child.shell(t))
        }
        "repeat" => {
            let child = build(req_child(v, "shape")?, depth + 1, budget)?;
            let px = opt_f64(v, "x").unwrap_or(0.0);
            let py = opt_f64(v, "y").unwrap_or(0.0);
            let pz = opt_f64(v, "z").unwrap_or(0.0);
            Ok(child.repeat(Vec3::new(px, py, pz)))
        }
        "mirror_x" => Ok(build(req_child(v, "shape")?, depth + 1, budget)?.mirror_x()),
        "mirror_y" => Ok(build(req_child(v, "shape")?, depth + 1, budget)?.mirror_y()),
        "mirror_z" => Ok(build(req_child(v, "shape")?, depth + 1, budget)?.mirror_z()),

        other => Err(ScriptError::new(format!("unknown op: \"{other}\""))),
    }
}

// ── arg helpers ───────────────────────────────────────────────────────────────

fn req_f64(v: &Value, key: &str) -> Result<f64, ScriptError> {
    v.get(key)
        .and_then(|x| x.as_f64())
        .ok_or_else(|| ScriptError::new(format!("missing or non-numeric field \"{key}\"")))
}

fn opt_f64(v: &Value, key: &str) -> Option<f64> {
    v.get(key)?.as_f64()
}

fn req_child<'a>(v: &'a Value, key: &str) -> Result<&'a Value, ScriptError> {
    v.get(key)
        .ok_or_else(|| ScriptError::new(format!("missing child \"{key}\"")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Vec3;

    #[test]
    fn simple_sphere() {
        let s = eval_scene(r#"{"op":"sphere","r":1.0}"#).unwrap();
        let expected = Sdf::sphere(1.0);
        // 同一評価 (決定性)
        let p = Vec3::new(1.0, 0.0, 0.0);
        assert!((s.eval(p) - expected.eval(p)).abs() < 1e-12);
    }

    #[test]
    fn demo_bracket_scene() {
        let src = r#"{
          "op": "difference",
          "a": {
            "op": "union",
            "a": {"op": "sphere", "r": 1.0},
            "b": {"op": "cuboid", "x": 0.8, "y": 0.8, "z": 0.8}
          },
          "b": {"op": "cylinder", "r": 0.3, "h": 2.0}
        }"#;
        let s = eval_scene(src).unwrap();
        // 穴の中心は外側 (正)
        assert!(s.eval(Vec3::ZERO) > 0.0);
        // 穴から外れた内部は内側 (負)
        assert!(s.eval(Vec3::new(0.5, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn translate_via_script() {
        let src = r#"{"op":"translate","x":2.0,"y":0.0,"z":0.0,"shape":{"op":"sphere","r":1.0}}"#;
        let s = eval_scene(src).unwrap();
        let direct = Sdf::sphere(1.0).translate(Vec3::new(2.0, 0.0, 0.0));
        let p = Vec3::new(2.0, 1.0, 0.0);
        assert!((s.eval(p) - direct.eval(p)).abs() < 1e-12);
    }

    #[test]
    fn unknown_op_returns_error() {
        let r = eval_scene(r#"{"op":"unknown_primitive"}"#);
        assert!(r.is_err());
    }

    #[test]
    fn missing_field_returns_error() {
        let r = eval_scene(r#"{"op":"sphere"}"#); // missing r
        assert!(r.is_err());
    }

    #[test]
    fn over_deep_scene_is_rejected() {
        // 問16: 過度にネストしたスクリプトはリソース上限で拒否され、
        // スタックオーバーフローを起こさない (パーサ深さ上限 or 木深さ上限)。
        let depth = 200; // パーサ MAX_DEPTH(128) と build MAX_DEPTH(64) の両方を超える
        let mut src = String::new();
        for _ in 0..depth {
            src.push_str(r#"{"op":"translate","shape":"#);
        }
        src.push_str(r#"{"op":"sphere","r":1.0}"#);
        for _ in 0..depth {
            src.push('}');
        }
        assert!(
            eval_scene(&src).is_err(),
            "over-deep scene must be rejected"
        );
    }

    #[test]
    fn oversized_source_is_rejected() {
        let big = format!("{}{}", " ".repeat(MAX_SOURCE_BYTES + 1), "{}");
        let r = eval_scene(&big);
        assert!(r.is_err());
        assert!(r.unwrap_err().message.contains("too large"));
    }
}
