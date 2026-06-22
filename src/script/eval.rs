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
//! プリミティブ: sphere, cuboid, cylinder, torus, cone, capsule, rounded_box, ellipsoid
//! ブーリアン:   union, intersection, difference,
//!               smooth_union, smooth_intersection, smooth_difference
//! 変形:         translate, scale, offset, shell, repeat, mirror_x, mirror_y, mirror_z,
//!               rotate_x, rotate_y, rotate_z (angle は度)

use crate::core::{Sdf, Vec3};
use crate::mcp::json::{parse, Value};

#[derive(Debug)]
pub struct ScriptError {
    pub message: String,
}

impl ScriptError {
    pub(crate) fn new(s: impl Into<String>) -> ScriptError {
        ScriptError { message: s.into() }
    }

    /// 親ノードの文脈 (`op.key`) を先頭に付け、失敗位置のパスを積む (問64)。
    /// 木を巻き戻しながら `difference.a > union.b > sphere: ...` のように経路を構築する。
    pub(crate) fn at(self, op: &str, key: &str) -> ScriptError {
        ScriptError::new(format!("{op}.{key} > {}", self.message))
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
    eval_value(&v)
}

/// 既に構築済みの KadoScene [`Value`] 木を評価して [`Sdf`] を返す。
///
/// JSON とテキスト DSL は同じ `Value` 木へ落ちるため、意味論・検証・リソース上限を
/// ここで一元的に適用する (フロントエンド非依存)。
pub fn eval_value(v: &Value) -> Result<Sdf, ScriptError> {
    let mut budget = Budget { nodes: 0 };
    build(v, 0, &mut budget)
}

/// DSL 用の最大ソースバイト数 (JSON と共通の上限・問16)。
pub(crate) const DSL_MAX_SOURCE_BYTES: usize = MAX_SOURCE_BYTES;
/// DSL 用の最大ネスト深さ (問16)。
pub(crate) const DSL_MAX_DEPTH: usize = MAX_DEPTH;

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
            // 問77: minor >= major → horn (minor=major) or spindle (minor>major) torus。
            // どちらも自己交差して非多様体メッシュになり 3D 印刷が失敗する。
            // ring torus の数学的必要条件は minor < major。
            if minor >= major {
                return Err(ScriptError::new(format!(
                    "torus minor radius {minor} must be < major radius {major} \
                     (minor=major → horn torus (self-touching at center); \
                      minor>major → spindle torus (self-intersecting, non-manifold))"
                )));
            }
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
        "ellipsoid" => {
            // 各軸の半径。"s" で一様 (=球) 指定も可。すべて > 0。
            let x = opt_f64(v, "x").or_else(|| opt_f64(v, "s"));
            let x = match x {
                Some(x) if x > 0.0 => x,
                Some(x) => {
                    return Err(ScriptError::new(format!(
                        "ellipsoid radius \"x\" must be > 0, got {x}"
                    )))
                }
                None => return Err(ScriptError::new("missing or non-numeric field \"x\"")),
            };
            let y = opt_f64(v, "y").or_else(|| opt_f64(v, "s")).unwrap_or(x);
            let z = opt_f64(v, "z").or_else(|| opt_f64(v, "s")).unwrap_or(x);
            for (n, val) in [("y", y), ("z", z)] {
                if val <= 0.0 {
                    return Err(ScriptError::new(format!(
                        "ellipsoid radius \"{n}\" must be > 0, got {val}"
                    )));
                }
            }
            Ok(Sdf::ellipsoid(Vec3::new(x, y, z)))
        }

        // ── ブーリアン ────────────────────────────────────────────────────────
        "union" => {
            let a = build_child(v, "a", op, depth, budget)?;
            let b = build_child(v, "b", op, depth, budget)?;
            Ok(a.union(b))
        }
        "intersection" => {
            let a = build_child(v, "a", op, depth, budget)?;
            let b = build_child(v, "b", op, depth, budget)?;
            Ok(a.intersection(b))
        }
        "difference" => {
            let a = build_child(v, "a", op, depth, budget)?;
            let b = build_child(v, "b", op, depth, budget)?;
            Ok(a.difference(b))
        }
        "smooth_union" => {
            let a = build_child(v, "a", op, depth, budget)?;
            let b = build_child(v, "b", op, depth, budget)?;
            let k = opt_f64(v, "k").unwrap_or(0.3);
            // 問75: k=0 → 除算ゼロで NaN; k<0 → AABB が縮小しメッシュが欠損する。
            if k <= 0.0 {
                return Err(ScriptError::new(format!(
                    "smooth_union \"k\" blend radius must be > 0, got {k} \
                     (k=0 causes division by zero; use union for hard boundary)"
                )));
            }
            Ok(a.smooth_union(b, k))
        }
        "smooth_intersection" => {
            let a = build_child(v, "a", op, depth, budget)?;
            let b = build_child(v, "b", op, depth, budget)?;
            let k = opt_f64(v, "k").unwrap_or(0.3);
            if k <= 0.0 {
                return Err(ScriptError::new(format!(
                    "smooth_intersection \"k\" blend radius must be > 0, got {k} \
                     (use intersection for hard boundary)"
                )));
            }
            Ok(a.smooth_intersection(b, k))
        }
        "smooth_difference" => {
            let a = build_child(v, "a", op, depth, budget)?;
            let b = build_child(v, "b", op, depth, budget)?;
            let k = opt_f64(v, "k").unwrap_or(0.3);
            if k <= 0.0 {
                return Err(ScriptError::new(format!(
                    "smooth_difference \"k\" blend radius must be > 0, got {k} \
                     (use difference for hard boundary)"
                )));
            }
            Ok(a.smooth_difference(b, k))
        }

        // ── 変形 ──────────────────────────────────────────────────────────────
        "translate" => {
            let child = build_child(v, "shape", op, depth, budget)?;
            let tx = opt_f64(v, "x").unwrap_or(0.0);
            let ty = opt_f64(v, "y").unwrap_or(0.0);
            let tz = opt_f64(v, "z").unwrap_or(0.0);
            Ok(child.translate(Vec3::new(tx, ty, tz)))
        }
        "scale" => {
            let child = build_child(v, "shape", op, depth, budget)?;
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
            let child = build_child(v, "shape", op, depth, budget)?;
            let a = req_f64(v, "amount")?;
            Ok(child.offset(a))
        }
        "shell" => {
            let child = build_child(v, "shape", op, depth, budget)?;
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
            let child = build_child(v, "shape", op, depth, budget)?;
            let px = opt_f64(v, "x").unwrap_or(0.0);
            let py = opt_f64(v, "y").unwrap_or(0.0);
            let pz = opt_f64(v, "z").unwrap_or(0.0);
            // 問70: count が明示指定されているのに対応する period が正でない場合はサイレント縮退
            // (= period=0 → snap が v を素通し → タイルなし) を起こす。エラーとして明示する。
            // count が指定されていない (= 既定 1) の場合は period=0 で「その軸は繰り返さない」
            // という既存の慣例と互換なため、チェックしない。
            for (axis, period, count_key) in [("x", px, "nx"), ("y", py, "ny"), ("z", pz, "nz")] {
                if let Some(cnt) = opt_f64(v, count_key) {
                    if cnt.is_finite() && cnt > 0.0 && period <= 0.0 {
                        return Err(ScriptError::new(format!(
                            "repeat: \"{count_key}\"={cnt} requires a positive \"{axis}\" period \
                             (omitting \"{axis}\" or setting it to 0 silently disables repetition on that axis)"
                        )));
                    }
                }
            }
            // 各軸のコピー数 (片側)。既定1。非有限/負は1へ、過大はエラー (問76)。
            // サイレントクランプでは AI が要求したコピー数が反映されたか判断できない。
            let mut counts = [1u32; 3];
            for (i, key) in ["nx", "ny", "nz"].iter().enumerate() {
                if let Some(f) = opt_f64(v, key) {
                    if !f.is_finite() || f < 0.0 {
                        return Err(ScriptError::new(format!(
                            "repeat \"{key}\" must be a non-negative integer, got {f}"
                        )));
                    }
                    let cnt = f as u32;
                    if cnt > MAX_REPEAT {
                        return Err(ScriptError::new(format!(
                            "repeat \"{key}\"={cnt} exceeds the maximum of {MAX_REPEAT} \
                             (total copies = 2*n+1 per axis; use smaller count to stay within limits)"
                        )));
                    }
                    counts[i] = cnt;
                }
            }
            Ok(child.repeat_n(Vec3::new(px, py, pz), counts))
        }
        "mirror_x" => Ok(build_child(v, "shape", op, depth, budget)?.mirror_x()),
        "mirror_y" => Ok(build_child(v, "shape", op, depth, budget)?.mirror_y()),
        "mirror_z" => Ok(build_child(v, "shape", op, depth, budget)?.mirror_z()),

        // 回転の "angle" は**度** (degree) で受け取り内部でラジアンへ変換する (CAD 慣習)。
        "rotate_x" => {
            let child = build_child(v, "shape", op, depth, budget)?;
            Ok(child.rotate_x(req_f64(v, "angle")?.to_radians()))
        }
        "rotate_y" => {
            let child = build_child(v, "shape", op, depth, budget)?;
            Ok(child.rotate_y(req_f64(v, "angle")?.to_radians()))
        }
        "rotate_z" => {
            let child = build_child(v, "shape", op, depth, budget)?;
            Ok(child.rotate_z(req_f64(v, "angle")?.to_radians()))
        }

        // 平面カット: dot(p,(nx,ny,nz)) <= offset の側を残す。法線は非ゼロ必須。
        "cut" => {
            let child = build_child(v, "shape", op, depth, budget)?;
            let nx = req_f64(v, "nx")?;
            let ny = req_f64(v, "ny")?;
            let nz = req_f64(v, "nz")?;
            // offset は省略可 (既定 0 = 原点を通る平面)。
            let offset = opt_f64(v, "offset").unwrap_or(0.0);
            let normal = Vec3::new(nx, ny, nz);
            if normal.length() == 0.0 {
                return Err(ScriptError::new(
                    "cut normal (nx,ny,nz) must be non-zero".to_string(),
                ));
            }
            if !offset.is_finite() {
                return Err(ScriptError::new(format!(
                    "cut \"offset\" must be finite, got {offset}"
                )));
            }
            Ok(child.cut(normal, offset))
        }

        // flatten: FDM 印刷の平坦底面づくり (cut の最頻用ケースの安全な別名)。
        // z = at の平面で底を切り、z >= at の側 (上) を残す。`cut` の法線方向の
        // 取り違え (nz=+1 と -1 の混同) を避けるための意図明示型 op。
        // 内部的には cut(normal=(0,0,-1), offset=-at) に lower する (新 variant 不要)。
        "flatten" => {
            let child = build_child(v, "shape", op, depth, budget)?;
            let at = opt_f64(v, "at").unwrap_or(0.0);
            if !at.is_finite() {
                return Err(ScriptError::new(format!(
                    "flatten \"at\" must be finite, got {at}"
                )));
            }
            // keep z >= at  ⟺  dot(p,(0,0,-1)) <= -at
            Ok(child.cut(Vec3::new(0.0, 0.0, -1.0), -at))
        }

        other => Err(ScriptError::new(format!("unknown op: \"{other}\""))),
    }
}

// ── arg helpers ───────────────────────────────────────────────────────────────

/// 子ノードを取得して再帰評価し、失敗時に親文脈 (`op.key`) をパスへ積む (問64)。
fn build_child(
    v: &Value,
    key: &str,
    op: &str,
    depth: usize,
    budget: &mut Budget,
) -> Result<Sdf, ScriptError> {
    let child = req_child(v, key).map_err(|e| e.at(op, key))?;
    build(child, depth + 1, budget).map_err(|e| e.at(op, key))
}

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
    fn nested_error_reports_path_to_failing_node() {
        // 問64: 深い木の失敗はパス付きで報告される (どのノードが原因か特定可能)。
        // difference.a > union.b > sphere の r=0 が原因。
        let src = r#"{"op":"difference",
            "a":{"op":"union","a":{"op":"sphere","r":1},"b":{"op":"sphere","r":0}},
            "b":{"op":"cylinder","r":0.3,"h":2}}"#;
        let e = eval_scene(src).unwrap_err();
        assert!(
            e.message.contains("difference.a > union.b >"),
            "error must carry the path to the failing node, got: {}",
            e.message
        );
        assert!(e.message.contains("must be > 0"), "and the leaf cause: {}", e.message);
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
    fn node_budget_is_shared_and_enforced_for_wide_trees() {
        // 問146: MAX_NODES=50,000 はノード数で DoS を止める。
        // eval_scene では MAX_SOURCE_BYTES が先に発動するため、Value 木を
        // プログラムで生成して eval_value を直接呼ぶことで MAX_NODES を確認する。
        //
        // budget は `&mut Budget` (共有) なのでサブツリー全体の合計がカウントされる。
        // depth=17 の完全二分木は 2^17-1=131,071 ノードを持ち MAX_NODES を超える。
        use crate::mcp::json::{self, Value};
        fn balanced_union(depth: usize) -> Value {
            if depth == 0 {
                return json::obj([("op", json::s("sphere")), ("r", json::n(0.5))]);
            }
            json::obj([
                ("op", json::s("union")),
                ("a", balanced_union(depth - 1)),
                ("b", balanced_union(depth - 1)),
            ])
        }
        // 深さ17 (131,071 ノード) → MAX_NODES 超過で拒否されること。
        let big_tree = balanced_union(17);
        let r = eval_value(&big_tree);
        assert!(r.is_err(), "wide tree exceeding MAX_NODES must be rejected");
        assert!(
            r.unwrap_err().message.contains("too large"),
            "error must mention 'too large'"
        );
        // 深さ14 (16,383 ノード) → MAX_NODES 以内で受理されること。
        let small_tree = balanced_union(14);
        assert!(
            eval_value(&small_tree).is_ok(),
            "tree within MAX_NODES must be accepted"
        );
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

        assert!(eval_scene(r#"{"op":"ellipsoid","x":0.0,"y":1.0,"z":1.0}"#).is_err(), "x=0 ellipsoid");
        assert!(eval_scene(r#"{"op":"ellipsoid","x":1.0,"y":-1.0,"z":1.0}"#).is_err(), "y<0 ellipsoid");
    }

    #[test]
    fn ellipsoid_via_script() {
        // 問53: 各軸半径指定。
        let s = eval_scene(r#"{"op":"ellipsoid","x":2.0,"y":1.0,"z":0.5}"#).unwrap();
        let direct = Sdf::ellipsoid(Vec3::new(2.0, 1.0, 0.5));
        let p = Vec3::new(0.7, 0.3, 0.2);
        assert!((s.eval(p) - direct.eval(p)).abs() < 1e-12);
        // "s" で一様指定 → 球相当。
        let uni = eval_scene(r#"{"op":"ellipsoid","s":1.5}"#).unwrap();
        assert!(uni.eval(Vec3::new(1.5, 0.0, 0.0)).abs() < 1e-12, "uniform ellipsoid surface");
        // x 欠落はエラー。
        assert!(eval_scene(r#"{"op":"ellipsoid","y":1.0,"z":1.0}"#).is_err());
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
    fn cut_via_script_flattens_base_and_rejects_zero_normal() {
        // 問235: cut で印刷用の平坦な底面を作る。z=0 平面で下半分 (z<0) を削る。
        let src = r#"{"op":"cut","nx":0,"ny":0,"nz":-1,"offset":0,
            "shape":{"op":"sphere","r":1.0}}"#;
        let s = eval_scene(src).unwrap();
        assert!(s.eval(Vec3::new(0.0, 0.0, 0.5)) < 0.0, "upper half kept");
        assert!(s.eval(Vec3::new(0.0, 0.0, -0.5)) > 0.0, "lower half cut away");

        // offset 省略時は 0 (原点を通る平面)。
        let no_off = eval_scene(r#"{"op":"cut","nx":0,"ny":0,"nz":-1,"shape":{"op":"sphere","r":1.0}}"#).unwrap();
        assert!(no_off.eval(Vec3::new(0.0, 0.0, -0.5)) > 0.0, "default offset 0 cuts at origin");

        // ゼロ法線は拒否 (退化平面)。
        assert!(
            eval_scene(r#"{"op":"cut","nx":0,"ny":0,"nz":0,"shape":{"op":"sphere","r":1.0}}"#).is_err(),
            "zero normal must be rejected"
        );
        // 法線成分の欠落はエラー。
        assert!(
            eval_scene(r#"{"op":"cut","nx":0,"ny":0,"shape":{"op":"sphere","r":1.0}}"#).is_err(),
            "missing nz must be rejected"
        );
    }

    #[test]
    fn flatten_keeps_above_plane_and_equals_explicit_cut() {
        // 問236: flatten は印刷用の平坦底面 (z>=at を残す) を意図明示型で作る。
        // flatten(at) は cut(normal=(0,0,-1), offset=-at) と完全一致しなければならない
        // (法線方向の取り違えを避ける安全な別名)。
        let flat = eval_scene(r#"{"op":"flatten","at":0,"shape":{"op":"sphere","r":1.0}}"#).unwrap();
        // z>=0 は残る、z<0 は削られる。
        assert!(flat.eval(Vec3::new(0.0, 0.0, 0.5)) < 0.0, "z>0 kept");
        assert!(flat.eval(Vec3::new(0.0, 0.0, -0.5)) > 0.0, "z<0 cut away");
        assert!(flat.eval(Vec3::ZERO).abs() < 1e-12, "z=0 is the flat base surface");

        // at 省略時は 0。
        let no_at = eval_scene(r#"{"op":"flatten","shape":{"op":"sphere","r":1.0}}"#).unwrap();
        assert!(no_at.eval(Vec3::new(0.0, 0.0, -0.5)) > 0.0, "default at=0");

        // at=0.3 で底を上げる: z>=0.3 を残す。
        let raised = eval_scene(r#"{"op":"flatten","at":0.3,"shape":{"op":"sphere","r":1.0}}"#).unwrap();
        assert!(raised.eval(Vec3::new(0.0, 0.0, 0.5)) < 0.0, "z>0.3 kept");
        assert!(raised.eval(Vec3::new(0.0, 0.0, 0.1)) > 0.0, "z<0.3 cut away");

        // flatten(at) == cut((0,0,-1), -at) を多点でビット一致確認。
        let explicit = eval_scene(r#"{"op":"cut","nx":0,"ny":0,"nz":-1,"offset":-0.3,"shape":{"op":"sphere","r":1.0}}"#).unwrap();
        for p in [Vec3::ZERO, Vec3::new(0.2, -0.1, 0.5), Vec3::new(0.0, 0.0, -0.7), Vec3::new(0.5, 0.5, 0.3)] {
            assert_eq!(
                raised.eval(p).to_bits(),
                explicit.eval(p).to_bits(),
                "flatten(0.3) must equal cut((0,0,-1),-0.3) bit-for-bit at {p:?}"
            );
        }
        // 非有限 at は拒否 (パーサが 1e999→inf を遮断: 問20)。
        assert!(
            eval_scene(r#"{"op":"flatten","at":1e999,"shape":{"op":"sphere","r":1.0}}"#).is_err(),
            "non-finite at must be rejected"
        );
    }

    #[test]
    fn rotate_operations_via_script() {
        // 問51: rotate_z 90° は z 軸長尺の円柱を x 軸長尺へ向け直す。
        // angle はスクリプトでは度。h=2 の円柱を z 周りでなく y 周りに回すと軸が倒れる。
        let src = r#"{"op":"rotate_y","angle":90,"shape":{"op":"cylinder","r":0.3,"h":2.0}}"#;
        let s = eval_scene(src).unwrap();
        // 元の円柱は z 軸に沿う (z=±2 付近まで内部)。y 周り 90° 回転後は x 軸に沿う。
        // 回転後、(1.5, 0, 0) は軸上 (元の (0,0,-1.5) 相当) → 内部。
        assert!(
            s.eval(Vec3::new(1.5, 0.0, 0.0)) < 0.0,
            "after rotate_y 90°, cylinder axis lies along x: {}",
            s.eval(Vec3::new(1.5, 0.0, 0.0))
        );
        // 元の軸方向 z=1.5 はもはや内部でない (軸が倒れたため)。
        assert!(
            s.eval(Vec3::new(0.0, 0.0, 1.5)) > 0.0,
            "z-axis is no longer the cylinder axis after rotation"
        );

        // angle 欠落はエラー。
        assert!(
            eval_scene(r#"{"op":"rotate_x","shape":{"op":"sphere","r":1.0}}"#).is_err(),
            "missing angle must be rejected"
        );

        // 0° 回転は恒等。
        let id = eval_scene(r#"{"op":"rotate_z","angle":0,"shape":{"op":"cuboid","x":1,"y":0.5,"z":0.5}}"#).unwrap();
        let direct = Sdf::cuboid(Vec3::new(1.0, 0.5, 0.5));
        let p = Vec3::new(0.7, 0.2, 0.1);
        assert!((id.eval(p) - direct.eval(p)).abs() < 1e-12, "0° rotation must be identity");
    }

    #[test]
    fn rotate_accepts_negative_and_over_360_degree_angles() {
        // 問215: angle は req_f64 で範囲制限なし。負角や 360° 超も to_radians() で
        // 正しく扱われる。rotate_operations_via_script は 0°/90° のみ確認していた。
        // -90° は +270° と、450° は +90° と同じ回転になる (mod 360 の周期性)。
        let cyl = r#"{"op":"cylinder","r":0.3,"h":2.0}"#;
        let p = Vec3::new(1.5, 0.0, 0.0);
        // 負角 -90° と +270° は同一回転。
        let neg = eval_scene(&format!(r#"{{"op":"rotate_y","angle":-90,"shape":{cyl}}}"#)).unwrap();
        let pos270 = eval_scene(&format!(r#"{{"op":"rotate_y","angle":270,"shape":{cyl}}}"#)).unwrap();
        assert!(
            (neg.eval(p) - pos270.eval(p)).abs() < 1e-9,
            "-90° must equal 270°: {} vs {}", neg.eval(p), pos270.eval(p)
        );
        // 450° と 90° は同一回転。
        let big = eval_scene(&format!(r#"{{"op":"rotate_y","angle":450,"shape":{cyl}}}"#)).unwrap();
        let small = eval_scene(&format!(r#"{{"op":"rotate_y","angle":90,"shape":{cyl}}}"#)).unwrap();
        assert!(
            (big.eval(p) - small.eval(p)).abs() < 1e-9,
            "450° must equal 90°: {} vs {}", big.eval(p), small.eval(p)
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

    #[test]
    fn repeat_count_without_period_is_rejected() {
        // 問70: nx を明示したのに x 周期を省略 → サイレント縮退ではなくエラー。
        // count=3 なのに period=0 → タイルなしで元の球1個になる縮退を防ぐ。
        let bad = r#"{"op":"repeat","nx":3,"shape":{"op":"sphere","r":0.5}}"#;
        assert!(
            eval_scene(bad).is_err(),
            "repeat nx=3 with no x period must be an error, not silent degeneration"
        );

        // ny を明示、nz は既定 (= 省略扱い) → ny のみエラー、nz はチェックなし。
        let bad_y = r#"{"op":"repeat","y":0.0,"ny":2,"shape":{"op":"sphere","r":0.5}}"#;
        assert!(
            eval_scene(bad_y).is_err(),
            "repeat ny=2 with y=0 period must be an error"
        );

        // period を正しく指定すれば OK。
        let good = r#"{"op":"repeat","x":2.0,"nx":2,"shape":{"op":"sphere","r":0.5}}"#;
        assert!(
            eval_scene(good).is_ok(),
            "repeat with explicit positive period must succeed"
        );

        // count を省略 (既定 1) かつ period=0 は「その軸は繰り返さない」 → エラー不要。
        let default_ok = r#"{"op":"repeat","x":2.0,"nx":1,"shape":{"op":"sphere","r":0.3}}"#;
        assert!(
            eval_scene(default_ok).is_ok(),
            "repeat with explicit nx=1 and positive period must succeed"
        );
    }

    #[test]
    fn repeat_with_explicit_zero_counts_degenerates_to_single_shape() {
        // 問174: 明示的に nx=ny=nz=0 を与え period を正にした場合、
        // 検証 (line 300 の cnt>0.0 ガード) は cnt=0 なので通過し、エラーにならない。
        // snap() は n==0 で軸を無効化するため、全軸 count=0 は繰り返しなしの
        // 単一形状に縮退する。この「明示ゼロ = 無効化」契約を固定する。
        let zero_all = r#"{"op":"repeat","x":2.0,"nx":0,"y":2.0,"ny":0,"z":2.0,"nz":0,
                           "shape":{"op":"sphere","r":0.3}}"#;
        let sdf = eval_scene(zero_all).expect("explicit zero counts must not error");
        // 結果は素の sphere(0.3) と同じ距離場 (繰り返しが無効化されている)。
        let bare = Sdf::sphere(0.3);
        for p in [
            Vec3::ZERO,
            Vec3::new(2.0, 0.0, 0.0), // 繰り返しがあれば d<0、なければ d>0。
            Vec3::new(1.0, 1.0, 1.0),
        ] {
            assert_eq!(
                sdf.eval(p),
                bare.eval(p),
                "all-zero-count repeat must equal bare sphere at {p:?}"
            );
        }
    }

    #[test]
    fn torus_minor_ge_major_is_rejected() {
        // 問77: minor >= major → 自己交差 (horn/spindle torus) → 非多様体メッシュ → 印刷不可。
        // 数学的 ring torus の要件 (minor < major) をスクリプト評価段階で強制する。

        // minor = major → horn torus
        let horn = r#"{"op":"torus","major":1.0,"minor":1.0}"#;
        assert!(eval_scene(horn).is_err(), "minor=major (horn torus) must be rejected");

        // minor > major → spindle torus
        let spindle = r#"{"op":"torus","major":0.5,"minor":0.8}"#;
        let err = eval_scene(spindle);
        assert!(err.is_err(), "minor>major (spindle torus) must be rejected");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("spindle") || msg.contains("non-manifold"),
            "error must explain the self-intersection: {msg}"
        );

        // minor < major → 有効な ring torus
        let ring = r#"{"op":"torus","major":1.0,"minor":0.3}"#;
        assert!(eval_scene(ring).is_ok(), "minor<major ring torus must be accepted");
    }

    #[test]
    fn repeat_count_over_max_is_rejected_not_silently_clamped() {
        // 問76: count > MAX_REPEAT をサイレントクランプするのではなく明示エラーにする。
        // AI が nx=500 を指定したのに 256 コピーしか生成されないことに気づけない問題を防ぐ。
        let too_many = r#"{"op":"repeat","x":1.0,"nx":300,"shape":{"op":"sphere","r":0.3}}"#;
        let err = eval_scene(too_many);
        assert!(err.is_err(), "repeat nx=300 > MAX_REPEAT must be an error, not clamped silently");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("256") || msg.contains("maximum"),
            "error must mention the max limit: {msg}"
        );

        // MAX_REPEAT(256) 以内は OK。
        let at_max = r#"{"op":"repeat","x":1.0,"nx":256,"shape":{"op":"sphere","r":0.3}}"#;
        assert!(eval_scene(at_max).is_ok(), "repeat nx=256 (= MAX_REPEAT) must be accepted");

        // 負のカウントもエラー。
        let neg_count = r#"{"op":"repeat","x":1.0,"nx":-1,"shape":{"op":"sphere","r":0.3}}"#;
        assert!(eval_scene(neg_count).is_err(), "repeat nx=-1 must be rejected");
    }

    #[test]
    fn smooth_k_zero_or_negative_is_rejected() {
        // 問75: smooth_* の k=0 は除算ゼロで NaN を生む; k<0 は AABB を縮小し
        // メッシュが欠損する。スクリプト検証段階で拒否することを確認する。
        let a = r#"{"op":"sphere","r":1.0}"#;
        let b = r#"{"op":"sphere","r":0.8}"#;

        // k=0 はエラー。
        let zero_u = format!(r#"{{"op":"smooth_union","k":0,"a":{a},"b":{b}}}"#);
        assert!(eval_scene(&zero_u).is_err(), "smooth_union k=0 must be rejected (NaN risk)");

        let zero_i = format!(r#"{{"op":"smooth_intersection","k":0,"a":{a},"b":{b}}}"#);
        assert!(eval_scene(&zero_i).is_err(), "smooth_intersection k=0 must be rejected");

        let zero_d = format!(r#"{{"op":"smooth_difference","k":0,"a":{a},"b":{b}}}"#);
        assert!(eval_scene(&zero_d).is_err(), "smooth_difference k=0 must be rejected");

        // k<0 もエラー。
        let neg_u = format!(r#"{{"op":"smooth_union","k":-0.5,"a":{a},"b":{b}}}"#);
        assert!(eval_scene(&neg_u).is_err(), "smooth_union k<0 must be rejected (AABB shrinks)");

        // k>0 は有効。
        let pos_u = format!(r#"{{"op":"smooth_union","k":0.2,"a":{a},"b":{b}}}"#);
        assert!(eval_scene(&pos_u).is_ok(), "smooth_union positive k must succeed");
    }

    #[test]
    fn cone_negative_radius_or_height_is_rejected() {
        // 問191: zero_or_negative_primitive_dimensions_are_rejected は cone r=0/h=0 を確認するが
        // 負値 (r<0, h<0) は別のコードパス (f <= 0.0 の負側) を通る。
        // req_positive_f64 の f<=0.0 ガードが負値でも発動することを固定する。
        assert!(
            eval_scene(r#"{"op":"cone","r":-1.0,"h":1.0}"#).is_err(),
            "cone r<0 must be rejected by req_positive_f64"
        );
        assert!(
            eval_scene(r#"{"op":"cone","r":1.0,"h":-1.0}"#).is_err(),
            "cone h<0 must be rejected by req_positive_f64"
        );
        assert!(
            eval_scene(r#"{"op":"cone","r":-0.001,"h":-0.001}"#).is_err(),
            "cone r<0 h<0 must be rejected"
        );
        // 回帰: 正値は有効。
        assert!(
            eval_scene(r#"{"op":"cone","r":1.0,"h":2.0}"#).is_ok(),
            "cone with positive r and h must succeed"
        );
    }

    #[test]
    fn offset_negative_amount_shrinks_sphere_correctly() {
        // 問190: offset は req_f64 (not req_positive_f64) を使い、負値を意図的に許可する。
        // 正値は shape を膨張、負値は収縮する。どちらも valid。
        // eval.rs の文書コメント (line 621: "inflates/deflates") を固定する。
        let inflated = eval_scene(r#"{"op":"offset","amount":0.5,"shape":{"op":"sphere","r":1.0}}"#)
            .expect("positive offset must succeed");
        let deflated = eval_scene(r#"{"op":"offset","amount":-0.5,"shape":{"op":"sphere","r":1.0}}"#)
            .expect("negative offset must succeed (deflation)");
        let p = crate::core::Vec3::new(1.5, 0.0, 0.0);
        // 元の sphere(1.0) で x=1.5 は外部 (d=0.5)。
        // offset(+0.5): d = 0.5 - 0.5 = 0.0 (表面になる)。
        assert!(inflated.eval(p).abs() < 1e-12, "offset(+0.5) must bring x=1.5 to surface, got {}", inflated.eval(p));
        // offset(-0.5): d = 0.5 - (-0.5) = 1.0 (さらに外側)。
        assert!((deflated.eval(p) - 1.0).abs() < 1e-12, "offset(-0.5) must push x=1.5 further out, got {}", deflated.eval(p));
    }
}
