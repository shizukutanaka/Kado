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
//! ブーリアン:   union, intersection, difference,
//!               smooth_union, smooth_intersection, smooth_difference
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
/// 繰り返しコピー数の片側上限 (リソース上限・問21/問16)。
const MAX_REPEAT: u32 = 256;

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
            let r = req_positive_f64(v, "r")?;
            Ok(Sdf::sphere(r))
        }
        "cuboid" => {
            let x = opt_f64(v, "x").or_else(|| opt_f64(v, "s")).unwrap_or(1.0);
            let y = opt_f64(v, "y").or_else(|| opt_f64(v, "s")).unwrap_or(x);
            let z = opt_f64(v, "z").or_else(|| opt_f64(v, "s")).unwrap_or(x);
            // 半辺長 <= 0 は interior なしのデジェネレート形状 → EMPTY_MESH でサイレント失敗 (問28)。
            for (n, val) in [("x", x), ("y", y), ("z", z)] {
                if val <= 0.0 {
                    return Err(ScriptError::new(format!(
                        "cuboid half-extent \"{n}\" must be > 0, got {val}"
                    )));
                }
            }
            Ok(Sdf::cuboid(Vec3::new(x, y, z)))
        }
        "cylinder" => {
            let r = req_positive_f64(v, "r")?;
            let h = req_positive_f64(v, "h")?;
            Ok(Sdf::cylinder(r, h))
        }
        "torus" => {
            let major = req_positive_f64(v, "major")?;
            let minor = req_positive_f64(v, "minor")?;
            Ok(Sdf::torus(major, minor))
        }
        "cone" => {
            let r = req_positive_f64(v, "r")?;
            let h = req_positive_f64(v, "h")?;
            Ok(Sdf::cone(r, h))
        }
        "capsule" => {
            // h=0 → 球体 (有効)。r は必ず正。
            let h = req_f64(v, "h")?;
            if h < 0.0 {
                return Err(ScriptError::new(format!(
                    "capsule half-height \"h\" must be >= 0, got {h}"
                )));
            }
            let r = req_positive_f64(v, "r")?;
            Ok(Sdf::capsule(h, r))
        }
        "rounded_box" => {
            let x = req_positive_f64(v, "x")?;
            let y = opt_f64(v, "y").unwrap_or(x);
            let z = opt_f64(v, "z").unwrap_or(x);
            for (n, val) in [("y", y), ("z", z)] {
                if val <= 0.0 {
                    return Err(ScriptError::new(format!(
                        "rounded_box half-extent \"{n}\" must be > 0, got {val}"
                    )));
                }
            }
            let r = req_positive_f64(v, "r")?;
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
        "smooth_intersection" => {
            let a = build(req_child(v, "a")?, depth + 1, budget)?;
            let b = build(req_child(v, "b")?, depth + 1, budget)?;
            let k = opt_f64(v, "k").unwrap_or(0.3);
            Ok(a.smooth_intersection(b, k))
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
            // s<=0 は距離場を破壊する (s=0→0除算でNaN, s<0→内外反転)。拒否する (問20)。
            if s <= 0.0 {
                return Err(ScriptError::new(format!(
                    "scale factor must be > 0, got {s}"
                )));
            }
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
            // t<=0 は geometrically invalid: t=0 → |d| (内部なし), t<0 → 無意味。
            // scale<=0 と同様に拒否する (問27)。
            if t <= 0.0 {
                return Err(ScriptError::new(format!(
                    "shell thickness must be > 0, got {t}"
                )));
            }
            Ok(child.shell(t))
        }
        "repeat" => {
            let child = build(req_child(v, "shape")?, depth + 1, budget)?;
            let px = opt_f64(v, "x").unwrap_or(0.0);
            let py = opt_f64(v, "y").unwrap_or(0.0);
            let pz = opt_f64(v, "z").unwrap_or(0.0);
            // 各軸のコピー数 (片側)。既定1。非有限/負は1へ、過大は上限へ丸める (問21/問16)。
            let n = |key: &str| -> u32 {
                opt_f64(v, key)
                    .filter(|f| f.is_finite() && *f >= 0.0)
                    .map(|f| (f as u32).min(MAX_REPEAT))
                    .unwrap_or(1)
            };
            Ok(child.repeat_n(Vec3::new(px, py, pz), [n("nx"), n("ny"), n("nz")]))
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

/// `req_f64` + `> 0` チェック。`r=0` のプリミティブは内部がなく EMPTY_MESH でサイレント失敗するため (問28)。
fn req_positive_f64(v: &Value, key: &str) -> Result<f64, ScriptError> {
    let f = req_f64(v, key)?;
    if f <= 0.0 {
        return Err(ScriptError::new(format!(
            "\"{key}\" must be > 0, got {f}"
        )));
    }
    Ok(f)
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

    #[test]
    fn zero_or_negative_scale_is_rejected() {
        // 問20: s<=0 は距離場を破壊するため拒否 (無音の不正メッシュを防ぐ)。
        let z = r#"{"op":"scale","s":0.0,"shape":{"op":"sphere","r":1.0}}"#;
        assert!(eval_scene(z).is_err(), "zero scale must be rejected");
        let neg = r#"{"op":"scale","s":-2.0,"shape":{"op":"sphere","r":1.0}}"#;
        assert!(eval_scene(neg).is_err(), "negative scale must be rejected");
        let ok = r#"{"op":"scale","s":2.0,"shape":{"op":"sphere","r":1.0}}"#;
        assert!(eval_scene(ok).is_ok(), "positive scale must pass");
    }

    #[test]
    fn non_finite_param_is_rejected_via_parser() {
        // 問20: 1e400 (→ +inf) を含むスクリプトはパーサ段で拒否される。
        let r = eval_scene(r#"{"op":"sphere","r":1e400}"#);
        assert!(r.is_err(), "non-finite radius must be rejected");
    }

    #[test]
    fn zero_or_negative_primitive_dimensions_are_rejected() {
        // 問28: r=0/負は eval エラーなく受理されると EMPTY_MESH でサイレント失敗する。
        // scale/shell と同様に入力段で拒否して明確なエラーを返す。
        assert!(eval_scene(r#"{"op":"sphere","r":0.0}"#).is_err(), "r=0 sphere");
        assert!(eval_scene(r#"{"op":"sphere","r":-1.0}"#).is_err(), "r<0 sphere");
        assert!(eval_scene(r#"{"op":"sphere","r":1.0}"#).is_ok(), "r>0 sphere");

        assert!(eval_scene(r#"{"op":"cylinder","r":0.0,"h":1.0}"#).is_err(), "r=0 cylinder");
        assert!(eval_scene(r#"{"op":"cylinder","r":1.0,"h":0.0}"#).is_err(), "h=0 cylinder");
        assert!(eval_scene(r#"{"op":"cylinder","r":1.0,"h":1.0}"#).is_ok());

        assert!(eval_scene(r#"{"op":"cone","r":0.0,"h":1.0}"#).is_err(), "r=0 cone");
        assert!(eval_scene(r#"{"op":"cone","r":1.0,"h":0.0}"#).is_err(), "h=0 cone");

        assert!(eval_scene(r#"{"op":"torus","major":0.0,"minor":0.1}"#).is_err(), "major=0 torus");
        assert!(eval_scene(r#"{"op":"torus","major":1.0,"minor":0.0}"#).is_err(), "minor=0 torus");

        assert!(eval_scene(r#"{"op":"capsule","h":-1.0,"r":0.5}"#).is_err(), "h<0 capsule");
        assert!(eval_scene(r#"{"op":"capsule","h":0.0,"r":0.5}"#).is_ok(), "h=0 capsule is sphere");
        assert!(eval_scene(r#"{"op":"capsule","h":1.0,"r":0.0}"#).is_err(), "r=0 capsule");

        assert!(eval_scene(r#"{"op":"cuboid","x":0.0,"y":1.0,"z":1.0}"#).is_err(), "x=0 cuboid");
        assert!(eval_scene(r#"{"op":"cuboid","x":1.0,"y":-1.0,"z":1.0}"#).is_err(), "y<0 cuboid");

        assert!(eval_scene(r#"{"op":"rounded_box","x":0.0,"r":0.1}"#).is_err(), "x=0 rounded_box");
        assert!(eval_scene(r#"{"op":"rounded_box","x":1.0,"r":0.0}"#).is_err(), "r=0 rounded_box");
        assert!(eval_scene(r#"{"op":"rounded_box","x":1.0,"r":0.1}"#).is_ok());
    }

    #[test]
    fn zero_or_negative_shell_thickness_is_rejected() {
        // 問27: thickness<=0 は scale<=0 と同様に幾何的に無効。拒否して無音の不正メッシュを防ぐ。
        let z = r#"{"op":"shell","thickness":0.0,"shape":{"op":"sphere","r":1.0}}"#;
        assert!(eval_scene(z).is_err(), "zero thickness must be rejected");
        let neg = r#"{"op":"shell","thickness":-0.1,"shape":{"op":"sphere","r":1.0}}"#;
        assert!(eval_scene(neg).is_err(), "negative thickness must be rejected");
        let ok = r#"{"op":"shell","thickness":0.1,"shape":{"op":"sphere","r":1.0}}"#;
        assert!(eval_scene(ok).is_ok(), "positive thickness must pass");
    }

    #[test]
    fn smooth_operations_via_script() {
        // 問35: smooth_{union,intersection,difference} の eval レベルを網羅的に検証する。
        let sphere_a = r#"{"op":"sphere","r":1.0}"#;
        let sphere_b = r#"{"op":"translate","x":0.5,"y":0,"z":0,"shape":{"op":"sphere","r":1.0}}"#;

        // smooth_union: 重なり中心は両球の内部 → 負。
        let src_u = format!(r#"{{"op":"smooth_union","k":0.3,"a":{sphere_a},"b":{sphere_b}}}"#);
        let su = eval_scene(&src_u).unwrap();
        assert!(su.eval(Vec3::new(0.25, 0.0, 0.0)) < 0.0, "smooth_union center inside");
        // 遠方は外側 → 正。
        assert!(su.eval(Vec3::new(5.0, 0.0, 0.0)) > 0.0, "smooth_union far outside");

        // smooth_intersection: 両球の重なり領域の中心は内側 → 負。
        let src_i = format!(r#"{{"op":"smooth_intersection","k":0.3,"a":{sphere_a},"b":{sphere_b}}}"#);
        let si = eval_scene(&src_i).unwrap();
        assert!(si.eval(Vec3::new(0.25, 0.0, 0.0)) < 0.0, "smooth_intersection overlap inside");
        // 一方の球だけにある点は外側 → 正。
        assert!(si.eval(Vec3::new(-1.5, 0.0, 0.0)) > 0.0, "smooth_intersection non-overlap outside");

        // smooth_difference a-b: a 内 b 外の領域 → 負。
        let src_d = format!(r#"{{"op":"smooth_difference","k":0.3,"a":{sphere_a},"b":{sphere_b}}}"#);
        let sd = eval_scene(&src_d).unwrap();
        // a の左端 (-0.9, 0, 0) は a 内 b 外 → 負。
        assert!(sd.eval(Vec3::new(-0.9, 0.0, 0.0)) < 0.0, "smooth_diff inside a minus b");
        // b の中心付近 (0.5, 0, 0) は b 内 → 削除済み → 正。
        assert!(sd.eval(Vec3::new(0.5, 0.0, 0.0)) > 0.0, "smooth_diff inside b is removed");
    }

    #[test]
    fn mirror_operations_via_script() {
        // mirror_x/y/z が対称性を正しく生成することを確認する。
        // mirror_x: shape が x>0 に偏っていても x<0 側にも現れる。
        let src = r#"{"op":"mirror_x","shape":{"op":"translate","x":1.0,"y":0.0,"z":0.0,"shape":{"op":"sphere","r":0.3}}}"#;
        let s = eval_scene(src).unwrap();
        let p_pos = Vec3::new(1.0, 0.0, 0.0);
        let p_neg = Vec3::new(-1.0, 0.0, 0.0);
        assert!(s.eval(p_pos) < 0.0, "mirrored shape should be inside at +x");
        assert!(s.eval(p_neg) < 0.0, "mirrored shape should be inside at -x");
        // 対称性: 評価値も一致する。
        assert!(
            (s.eval(p_pos) - s.eval(p_neg)).abs() < 1e-12,
            "mirror must be symmetric"
        );
    }

    #[test]
    fn repeat_script_is_bounded() {
        // 問21: スクリプトの repeat は有限。nx=1 (x方向3コピー)、他軸は既定だが period 0 で無効。
        let src = r#"{"op":"repeat","x":2.0,"nx":1,"shape":{"op":"sphere","r":0.3}}"#;
        let s = eval_scene(src).unwrap();
        assert!(s.eval(Vec3::ZERO) < 0.0);
        assert!(s.eval(Vec3::new(2.0, 0.0, 0.0)) < 0.0);
        // 範囲外 (4セル目) は外側 → 無限タイルでない。
        assert!(s.eval(Vec3::new(8.0, 0.0, 0.0)) > 0.0);
    }
}
