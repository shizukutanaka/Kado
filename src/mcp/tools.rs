//! MCP ツール定義 (Phase 0.5/2/3)。

use crate::core::{Sdf, Vec3};
use crate::extract::polygonize;
use crate::io::{gltf, html, stl, threemf};
use crate::mcp::json::{self, Value};
use crate::render::{draw_axes, render, Camera};
use crate::script::eval_any;
use crate::verify::{validate, validate_with_field};

// ── リソース上限 (問18: 無境界パラメータによる OOM/panic DoS を防ぐ) ─────────────
// polygonize は (res+1)^3 個の f64 を確保するため、res を上限で抑える。
const MAX_RESOLUTION: usize = 256; // 257^3 f64 ≈ 136 MiB ×2バッファ
const MAX_IMAGE_DIM: usize = 4096; // 4096^2 px ×3byte ≈ 48 MiB

/// `resolution` 引数を安全な範囲 `[1, MAX_RESOLUTION]` に収める。
/// 非有限・負・0・過大値はすべて安全側へ丸め、`polygonize` の panic/OOM を防ぐ (問18)。
fn arg_resolution(args: &Value, default: usize) -> usize {
    match args.get("resolution").and_then(|v| v.as_f64()) {
        Some(f) if f.is_finite() => (f as usize).clamp(1, MAX_RESOLUTION),
        _ => default,
    }
}

/// 画像寸法引数を安全な範囲 `[1, MAX_IMAGE_DIM]` に収める (問18)。
fn arg_dim(args: &Value, key: &str, default: usize) -> usize {
    match args.get(key).and_then(|v| v.as_f64()) {
        Some(f) if f.is_finite() => (f as usize).clamp(1, MAX_IMAGE_DIM),
        _ => default,
    }
}

// ── ツールスキーマ ────────────────────────────────────────────────────────────

pub fn tool_list() -> Value {
    json::arr([
        tool_def(
            "screenshot",
            "Render the current SDF scene as a PNG screenshot. Returns base64-encoded PNG.",
            &[
                (
                    "view",
                    "string",
                    "Camera: front|back|right|left|top|bottom|iso (default: iso)",
                    false,
                ),
                (
                    "width",
                    "integer",
                    "Image width in pixels (default: 512)",
                    false,
                ),
                (
                    "height",
                    "integer",
                    "Image height in pixels (default: 512)",
                    false,
                ),
                (
                    "resolution",
                    "integer",
                    "Mesh resolution cells/axis (default: 48; increase for smoother output)",
                    false,
                ),
                (
                    "samples",
                    "integer",
                    "Anti-aliasing supersample factor 1-4 (default: 2; higher = smoother edges)",
                    false,
                ),
                (
                    "axes",
                    "boolean",
                    "Overlay an X(red)/Y(green)/Z(blue) orientation gnomon (default: true)",
                    false,
                ),
            ],
        ),
        tool_def(
            "export",
            "Export the current scene to a mesh file. Format is chosen by the path extension: \
             \".glb\" writes binary glTF 2.0 (indexed, viewable in browsers/Blender); \
             \".3mf\" writes 3MF (modern 3D-printing standard with mm units); \
             \".html\" writes a self-contained offline WebGL viewer (drag to orbit); \
             any other extension writes binary STL. Returns the output file path.",
            &[
                (
                    "path",
                    "string",
                    "Output file path; .glb=glTF, .3mf=3MF, .html=viewer, otherwise STL (default: kado-export.stl)",
                    false,
                ),
                (
                    "resolution",
                    "integer",
                    "Mesh resolution cells/axis (default: 64)",
                    false,
                ),
            ],
        ),
        tool_def(
            "eval",
            "Evaluate the SDF signed distance at a point. Negative = inside, positive = outside.",
            &[
                ("x", "number", "X coordinate", true),
                ("y", "number", "Y coordinate", true),
                ("z", "number", "Z coordinate", true),
            ],
        ),
        tool_def(
            "run_script",
            "Evaluate a KadoScene script and set it as the active scene. Returns a summary. \
             Accepts either JSON (starts with '{') or the compact text DSL, e.g. \
             difference(sphere(1), cylinder(0.3, 2)).",
            &[
                (
                    "script",
                    "string",
                    "KadoScene script: JSON object, or text DSL like union(sphere(1), cuboid(0.8))",
                    true,
                ),
                (
                    "resolution",
                    "integer",
                    "Mesh resolution cells/axis for summary (default: 32)",
                    false,
                ),
            ],
        ),
        tool_def(
            "validate",
            "Validate the current scene mesh for manufacturability (DFM). Returns a structured \
             JSON report: {ok, triangles, manifold, volume, bbox, dims_mm, digest, \
             issues:[{severity:\"error\"|\"warning\", code, cause, hints:[]}]}. \
             All issue codes: EMPTY_MESH (no geometry), NON_MANIFOLD (self-intersecting faces), \
             OPEN_MESH (boundary edges, unprintable), MULTIPLE_BODIES (separate shells), \
             THIN_WALL (local section < min_wall_mm), OVERHANG (angle > max_overhang_deg), \
             SUSPICIOUS_SCALE (overall size < min_wall, likely wrong units). \
             Overhang is measured against build_dir (default +Z). \
             If your printer builds along a different axis, set build_dir to get correct results.",
            &[
                (
                    "resolution",
                    "integer",
                    "Mesh resolution cells/axis (default: 48)",
                    false,
                ),
                (
                    "min_wall_mm",
                    "number",
                    "Minimum wall thickness threshold (0 to skip, default: 0.5)",
                    false,
                ),
                (
                    "max_overhang_deg",
                    "number",
                    "Maximum overhang angle in degrees from horizontal (0 to skip, default: 45)",
                    false,
                ),
                (
                    "build_dir",
                    "string",
                    "FDM build direction for overhang check: \"z\" (default +Z up), \"-z\", \
                     \"x\", \"-x\", \"y\", \"-y\", or a JSON array [dx,dy,dz]. \
                     Governs which faces are considered overhanging (問68).",
                    false,
                ),
            ],
        ),
        tool_def(
            "get_scene",
            "Return the KadoScene JSON script that produced the current active scene, \
             along with sampling bounds. Allows AI agents to inspect state before modifying it (問26).",
            &[],
        ),
        tool_def(
            "undo_script",
            "Restore the scene to the state before the last run_script call (single-level undo). \
             If run_script was never called, or undo was already applied, returns an error. \
             Useful when a bad script overwrites a valid scene and the AI needs to recover \
             without restarting the session (問67).",
            &[],
        ),
        tool_def(
            "help",
            "Return the KadoScene JSON format reference: all available op codes, \
             their parameters, and example scripts. Call this first when unfamiliar with the format.",
            &[],
        ),
    ])
}

fn tool_def(name: &str, description: &str, params: &[(&str, &str, &str, bool)]) -> Value {
    let mut props = std::collections::BTreeMap::new();
    let mut required = vec![];
    for &(pname, ptype, pdesc, preq) in params {
        props.insert(
            pname.to_string(),
            json::obj([("type", json::s(ptype)), ("description", json::s(pdesc))]),
        );
        if preq {
            required.push(json::s(pname));
        }
    }
    json::obj([
        ("name", json::s(name)),
        ("description", json::s(description)),
        (
            "inputSchema",
            Value::Object({
                let mut m = std::collections::BTreeMap::new();
                m.insert("type".into(), json::s("object"));
                m.insert("properties".into(), Value::Object(props));
                m.insert("required".into(), Value::Array(required));
                m
            }),
        ),
    ])
}

// ── ツール実行 ────────────────────────────────────────────────────────────────

pub struct ToolResult {
    pub content: Vec<Value>,
    pub is_error: bool,
}

impl ToolResult {
    fn text(s: impl Into<String>) -> Self {
        ToolResult {
            content: vec![json::obj([("type", json::s("text")), ("text", json::s(s))])],
            is_error: false,
        }
    }
    fn error(s: impl Into<String>) -> Self {
        ToolResult {
            content: vec![json::obj([("type", json::s("text")), ("text", json::s(s))])],
            is_error: true,
        }
    }
    fn image(b64: String) -> Self {
        ToolResult {
            content: vec![json::obj([
                ("type", json::s("image")),
                ("data", json::s(b64)),
                ("mimeType", json::s("image/png")),
            ])],
            is_error: false,
        }
    }
}

/// 既定シーン (デモ形状)。`run_script` 実行前の初期状態。
/// 問78: 旧デモは union/difference の組み合わせでシャープなエッジを持つ形状だった。
/// smooth_union に変更し、SDF の最大の特長である有機的ブレンドをデモとして示す。
pub fn default_scene() -> Sdf {
    Sdf::sphere(1.0).smooth_union(Sdf::cuboid(Vec3::splat(0.8)), 0.2)
}

/// MCP セッション状態。**正本はスクリプトが評価した [`Sdf`] 木**であり、
/// `run_script` がこれを更新し、他の全ツールがこれを読む (問2/問12)。
/// 固定のハードコード形状が事実上の正本になる退行を防ぐ。
pub struct Session {
    /// 現在アクティブな SDF シーン。`run_script` で差し替えられる。
    pub scene: Sdf,
    /// 最後に評価した KadoScene JSON スクリプト。
    /// `run_script` で更新; `get_scene` でAIが読み返せる (問26)。
    /// 未設定 (デフォルトシーン) の場合は None。
    pub script: Option<String>,
    /// 一つ前のシーン (undo 用, 問67)。`run_script` が更新前に保存する。
    /// `undo_script` 呼び出し後はクリアされる (single-level undo)。
    pub prev_scene: Option<Sdf>,
    /// 一つ前のスクリプト (undo 用, 問67)。
    pub prev_script: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Session {
            scene: default_scene(),
            script: None,
            prev_scene: None,
            prev_script: None,
        }
    }
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn call_tool(session: &mut Session, name: &str, args: &Value) -> ToolResult {
    match name {
        "screenshot" => tool_screenshot(session, args),
        "export" => tool_export(session, args),
        "eval" => tool_eval(session, args),
        "run_script" => tool_run_script(session, args),
        "validate" => tool_validate(session, args),
        "get_scene" => tool_get_scene(session),
        "undo_script" => tool_undo_script(session),
        "help" => tool_help(),
        other => ToolResult::error(format!("unknown tool: {other}")),
    }
}

fn tool_screenshot(session: &Session, args: &Value) -> ToolResult {
    let view = args.get("view").and_then(|v| v.as_str()).unwrap_or("iso");
    let width = arg_dim(args, "width", 512);
    let height = arg_dim(args, "height", 512);
    let res = arg_resolution(args, 48);
    // SSAA 係数 (問56)。スーパーサンプルバッファが MAX_IMAGE_DIM を超えないよう
    // クランプし OOM ガード (問18) を維持する。
    let samples = arg_samples(args, width, height);

    let scene = &session.scene;
    let (lo_b, hi_b) = scene.sampling_box();
    let mesh = polygonize(scene, lo_b, hi_b, res);
    if mesh.triangles.is_empty() {
        return ToolResult::error("mesh is empty — scene may be outside the bounding box");
    }
    let (lo, hi) = mesh.bounds().unwrap();
    let presets = Camera::presets(lo, hi);
    // 問71: 未知のビュー名はサイレントに "iso" フォールバックするのではなく、
    // 明示エラーを返す。AI が無効な view を指定した場合に気づけるようにする。
    let cam = match presets.iter().find(|(n, _)| *n == view) {
        Some((_, c)) => c.clone(),
        None => {
            let valid: Vec<&str> = presets.iter().map(|(n, _)| *n).collect();
            return ToolResult::error(format!(
                "unknown view '{view}'; valid views: {}",
                valid.join(", ")
            ));
        }
    };

    // スーパーサンプルして縮小 (アンチエイリアス)。
    let big = render(&mesh, &cam, width * samples, height * samples);
    let mut img = big.downsample(samples);
    // 向きの基準として座標軸グノモンを重ねる (問66; axes=false で無効化)。
    let show_axes = args.get("axes").and_then(|v| v.as_bool()).unwrap_or(true);
    if show_axes {
        let center = (lo + hi) * 0.5;
        let length = (hi - lo).length() * 0.35;
        draw_axes(&mut img, &cam, center, length);
    }
    ToolResult::image(base64_encode(&img.encode_png()))
}

/// SSAA 係数を `[1, 4]` に収め、かつ `dim * samples <= MAX_IMAGE_DIM` を保証する (問56/問18)。
fn arg_samples(args: &Value, width: usize, height: usize) -> usize {
    let requested = match args.get("samples").and_then(|v| v.as_f64()) {
        Some(f) if f.is_finite() => (f as usize).clamp(1, 4),
        _ => 2,
    };
    let cap_w = (MAX_IMAGE_DIM / width.max(1)).max(1);
    let cap_h = (MAX_IMAGE_DIM / height.max(1)).max(1);
    requested.min(cap_w).min(cap_h)
}

fn tool_export(session: &Session, args: &Value) -> ToolResult {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("kado-export.stl");
    let res = arg_resolution(args, 64);

    // MCP 書き込みポリシー (Plan リスク T / C9): プロジェクトdir限定・パストラバーサル拒否。
    let safe = match sandbox_write_path(path) {
        Ok(p) => p,
        Err(e) => return ToolResult::error(format!("rejected output path: {e}")),
    };

    let scene = &session.scene;
    let (lo_b, hi_b) = scene.sampling_box();
    let mesh = polygonize(scene, lo_b, hi_b, res);
    if mesh.triangles.is_empty() {
        return ToolResult::error(
            "mesh is empty — scene may be outside the bounding box; nothing exported",
        );
    }
    // 拡張子で形式を選択: .glb→GLB, .3mf→3MF, .html→HTMLビューア, 他→STL (問54/55/57)。
    let lower = path.to_lowercase();
    let (fmt, write_res) = if lower.ends_with(".glb") {
        ("GLB", gltf::write_glb(&mesh, &safe))
    } else if lower.ends_with(".3mf") {
        ("3MF", threemf::write_3mf(&mesh, &safe))
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        ("HTML", html::write_html(&mesh, &safe))
    } else {
        ("STL", stl::write_binary(&mesh, &safe))
    };
    match write_res {
        Ok(()) => {
            // 問72: 相対パスだけでは MCP サーバーの CWD が不明な AI はファイルの場所を
            // 特定できない。書き込み後に canonicalize で絶対パスを解決して返す。
            let abs_path = std::fs::canonicalize(&safe)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| safe.display().to_string());
            // 非多様体メッシュは 3D プリントで問題になる可能性がある。明示警告を付加する。
            let manifold = mesh.is_edge_manifold();
            let manifold_note = if manifold {
                String::new()
            } else {
                " [WARNING: non-manifold mesh — may cause issues when 3D printing; \
                  consider increasing resolution or checking for open boundaries]"
                    .to_string()
            };
            ToolResult::text(format!(
                "exported {fmt}: {abs_path} ({} triangles, manifold={manifold}){manifold_note}",
                mesh.triangles.len(),
            ))
        }
        Err(e) => ToolResult::error(format!("export failed: {e}")),
    }
}

/// MCP 書き込みのサンドボックス検査 (問15)。プロジェクトdir (CWD) 配下の相対パスのみ許可し、
/// 絶対パス・`..` によるパストラバーサル・ルート/プレフィックス脱出を拒否する。
/// ファイル存在に依存しないため `canonicalize` は使わず、パス構造のみで判定する。
fn sandbox_write_path(requested: &str) -> Result<std::path::PathBuf, String> {
    use std::path::{Component, Path};
    if requested.trim().is_empty() {
        return Err("empty output path".into());
    }
    let p = Path::new(requested);
    if p.is_absolute() {
        return Err(format!(
            "absolute paths are not permitted (project-dir only): {requested}"
        ));
    }
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                return Err(format!(
                    "path traversal (\"..\") is not permitted: {requested}"
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("rooted paths are not permitted: {requested}"));
            }
            _ => {}
        }
    }
    Ok(p.to_path_buf())
}

fn tool_eval(session: &Session, args: &Value) -> ToolResult {
    let x = args.get("x").and_then(|v| v.as_f64());
    let y = args.get("y").and_then(|v| v.as_f64());
    let z = args.get("z").and_then(|v| v.as_f64());
    match (x, y, z) {
        (Some(x), Some(y), Some(z)) => {
            // 問69: 非有限座標 (Infinity, NaN) は SDF 演算内で伝播し NaN 結果を生む。
            // AI が `1e999` (→ +Inf) を送ると SDF 全体が無意味な値を返す。早期拒否する。
            if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                return ToolResult::error(format!(
                    "coordinates must be finite: x={x}, y={y}, z={z} \
                     (Infinity and NaN are not valid SDF query points)"
                ));
            }
            let d = session.scene.eval(Vec3::new(x, y, z));
            // SDF 結果の非有限チェック: 正常な SDF 木では起こらないが防御的に検出する。
            if !d.is_finite() {
                return ToolResult::error(format!(
                    "SDF evaluation produced non-finite result {d} at ({x},{y},{z}) \
                     — this may indicate a degenerate shape in the scene"
                ));
            }
            ToolResult::text(format!("{d:.6}"))
        }
        _ => ToolResult::error("x, y, z are required numeric fields"),
    }
}

fn tool_run_script(session: &mut Session, args: &Value) -> ToolResult {
    let src = match args.get("script").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return ToolResult::error("\"script\" field is required"),
    };
    let res = arg_resolution(args, 32);

    // JSON ({...}) とテキスト DSL を自動判別 (問59)。
    let sdf = match eval_any(&src) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("script error: {e}")),
    };
    // 問67: 上書き前に現在の状態を保存し、undo_script で1段戻れるようにする。
    session.prev_scene = Some(session.scene.clone());
    session.prev_script = session.script.clone();
    // スクリプトが正本 (問2/問12): 評価結果とスクリプトをセッションに保存する。
    // スクリプトを保存しておくことでAIがget_sceneで読み返せる (問26)。
    let (lo_b, hi_b) = sdf.sampling_box();
    let mesh = polygonize(&sdf, lo_b, hi_b, res);
    let report = validate(&mesh, 0.0, 0.0);
    session.scene = sdf;
    session.script = Some(src);
    // 問46: エラーがある場合はコードを明示してAI自己修正ループを補助する。
    // 問81: 警告 (MULTIPLE_BODIES 等) もサイレントにせず列挙する。
    // "scene updated" だけでは AI が切断ボディを見逃す可能性がある。
    // is_error=false のまま (スクリプト自体は有効) だが問題を可視化する。
    let all_codes: Vec<&str> = report.issues.iter().map(|e| e.code).collect();
    let prefix = if all_codes.is_empty() {
        "scene updated".to_string()
    } else {
        format!("scene updated (check issues: {})", all_codes.join(", "))
    };
    ToolResult::text(format!("{prefix} — {}", report.summary()))
}

fn tool_undo_script(session: &mut Session) -> ToolResult {
    // 問67: 前のシーンを復元する (single-level undo)。
    // prev_scene が None = undo 不可 (まだ run_script が呼ばれていない、または既に undo 済み)。
    match session.prev_scene.take() {
        Some(prev_sdf) => {
            let prev_script = session.prev_script.take();
            session.scene = prev_sdf;
            session.script = prev_script;
            let (lo, hi) = session.scene.sampling_box();
            let script_info = match &session.script {
                Some(s) => format!("script={s}"),
                None => "script=(default scene)".to_string(),
            };
            ToolResult::text(format!(
                "undo ok — {script_info}\n\
                 bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}]",
                lo.x, lo.y, lo.z, hi.x, hi.y, hi.z
            ))
        }
        None => ToolResult::error(
            "nothing to undo — no previous script in this session (undo is single-level)",
        ),
    }
}

fn tool_help() -> ToolResult {
    ToolResult::text(KADOSCENE_HELP)
}

const KADOSCENE_HELP: &str = r#"# KadoScene JSON Format Reference

All scripts are a single JSON object with an "op" field.
Parameters marked (req) are required; others are optional with their default shown.

## Primitives

sphere        {"op":"sphere","r":1.0}
              r (req): radius > 0

cuboid        {"op":"cuboid","x":1.0,"y":1.0,"z":1.0}
              x,y,z (req): half-extents > 0 (or use "s" for uniform)

cylinder      {"op":"cylinder","r":0.5,"h":1.0}
              r (req): radius > 0; h (req): half-height > 0

torus         {"op":"torus","major":1.0,"minor":0.25}
              major (req): ring radius > 0; minor (req): tube radius > 0

cone          {"op":"cone","r":0.5,"h":1.5}
              apex at z=0, base at z=-h; r (req) > 0; h (req) > 0

capsule       {"op":"capsule","h":0.5,"r":0.3}
              h (req): half-height >= 0; r (req): radius > 0

rounded_box   {"op":"rounded_box","x":0.8,"y":0.8,"z":0.8,"r":0.1}
              x,y,z (req): half-extents > 0; r (req): corner radius > 0

ellipsoid     {"op":"ellipsoid","x":2.0,"y":1.0,"z":0.5}
              x,y,z (req): per-axis radii > 0 (or "s" for uniform = sphere)

## Boolean Operations

union         {"op":"union","a":<sdf>,"b":<sdf>}
intersection  {"op":"intersection","a":<sdf>,"b":<sdf>}
difference    {"op":"difference","a":<sdf>,"b":<sdf>}  (a minus b)

smooth_union         {"op":"smooth_union","a":<sdf>,"b":<sdf>,"k":0.3}
smooth_intersection  {"op":"smooth_intersection","a":<sdf>,"b":<sdf>,"k":0.3}
smooth_difference    {"op":"smooth_difference","a":<sdf>,"b":<sdf>,"k":0.3}
              k: blend radius (default 0.3)

## Transforms

translate     {"op":"translate","x":1.0,"y":0.0,"z":0.0,"shape":<sdf>}
scale         {"op":"scale","s":2.0,"shape":<sdf>}          s > 0
offset        {"op":"offset","amount":0.1,"shape":<sdf>}    inflates/deflates
shell         {"op":"shell","thickness":0.1,"shape":<sdf>}  thickness > 0
mirror_x      {"op":"mirror_x","shape":<sdf>}
mirror_y      {"op":"mirror_y","shape":<sdf>}
mirror_z      {"op":"mirror_z","shape":<sdf>}
rotate_x      {"op":"rotate_x","angle":90,"shape":<sdf>}  angle in DEGREES
rotate_y      {"op":"rotate_y","angle":45,"shape":<sdf>}
rotate_z      {"op":"rotate_z","angle":30,"shape":<sdf>}
repeat        {"op":"repeat","x":2.0,"nx":2,"shape":<sdf>}
              period per axis (x/y/z), count per axis (nx/ny/nz, default 1)

## Example: sphere with a cylindrical hole

{"op":"difference",
 "a":{"op":"sphere","r":1.5},
 "b":{"op":"cylinder","r":0.4,"h":2.0}}

## Compact text DSL (alternative to JSON)

run_script also accepts a concise function-call syntax (token-efficient). The same
hole example:

  difference(sphere(1.5), cylinder(0.4, 2.0))

DSL arg order mirrors the constructors:
  sphere(r) · cuboid(s) or cuboid(x,y,z) · cylinder(r,h) · torus(major,minor)
  cone(r,h) · capsule(h,r) · rounded_box(s,r) or (x,y,z,r) · ellipsoid(s) or (x,y,z)
  union/intersection/difference(a,b) · smooth_*(a,b[,k])
  translate(x,y,z,shape) · scale(s,shape) · offset(amount,shape) · shell(t,shape)
  rotate_x/y/z(deg,shape) · mirror_x/y/z(shape) · repeat(px,py,pz[,nx,ny,nz],shape)

## Workflow

1. Call run_script with your KadoScene JSON or text DSL.
2. Call screenshot to preview (valid views: front|back|right|left|top|bottom|iso).
3. Call validate for DFM; export to save STL/GLB/3MF/HTML.
4. Call get_scene to read back the current script if needed (also reports undo availability).
5. If a run_script went wrong, call undo_script to restore the previous scene (single-level).

## validate build_dir parameter

The validate tool checks overhang relative to a build direction (default +Z = gravity up).
If your 3D printer builds along a different axis, specify build_dir:
  validate(build_dir="z")   same as default (+Z up)
  validate(build_dir="-z")  build head-down (inverted)
  validate(build_dir="y")   build along Y axis

## validate issue codes (問79)

Branch on issue.code to categorize results:
  EMPTY_MESH        — no triangles (script may be outside bounding box)
  NON_MANIFOLD      — self-intersecting faces (boolean degeneracy)
  OPEN_MESH         — boundary edges present (shape not watertight; cannot print)
  MULTIPLE_BODIES   — disconnected shells (may need to merge or orient separately)
  THIN_WALL         — local wall < min_wall_mm (SDF-ray probe)
  OVERHANG          — surface > max_overhang_deg from horizontal (support required)
  SUSPICIOUS_SCALE  — max dimension < min_wall_mm (likely authored in wrong units)

## repeat requires explicit period when count is set

If you specify nx/ny/nz, the matching x/y/z period MUST be positive:
  {"op":"repeat","nx":3,"shape":...}          ERROR: x period required
  {"op":"repeat","x":2.0,"nx":3,"shape":...}  OK: 7 copies (3 each side + center)
  {"op":"repeat","x":2.0,"shape":...}         OK: default count=1 (3 copies: -1, 0, +1)
"#;

fn tool_get_scene(session: &Session) -> ToolResult {
    // 問80: sampling_box は実際の形状 AABB より ~5% 広い (polygonize 用の余白を含む)。
    // AI が eval クエリ領域を設定する際に過大な範囲を使うことを防ぐため、ラベルを明示する。
    let (lo, hi) = session.scene.sampling_box();
    let bounds_info = format!(
        "sampling_bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] (includes ~5% margin beyond shape AABB)",
        lo.x, lo.y, lo.z, hi.x, hi.y, hi.z
    );
    // 問74: undo_script の可否を明示する。AI が undo を試みる前に確認できる。
    let undo_info = if session.prev_scene.is_some() {
        "undo_available=true"
    } else {
        "undo_available=false"
    };
    match &session.script {
        Some(script) => ToolResult::text(format!(
            "script={script}\n{bounds_info}\n{undo_info}"
        )),
        // 問83: デフォルトシーンは Rust コードで定義され、対応するスクリプト文字列がない。
        // AI がデフォルトシーンを再現したい場合のため、等価な DSL 文字列を案内する。
        None => ToolResult::text(format!(
            "script=(default scene — no run_script call yet; \
             to reproduce: smooth_union(sphere(1.0),cuboid(0.8),0.2))\n\
             {bounds_info}\n{undo_info}"
        )),
    }
}

/// ビルド方向を args から解釈する (問68)。
/// 文字列: "x"/"+x"/"-x"/"y"/"+y"/"-y"/"z"/"+z"/"-z"。
/// 数値配列: [dx, dy, dz]。省略時: +Z (FDM 標準)。
fn arg_build_dir(args: &Value) -> Vec3 {
    if let Some(s) = args.get("build_dir").and_then(|v| v.as_str()) {
        match s.trim() {
            "x" | "+x" => Vec3::new(1.0, 0.0, 0.0),
            "-x" => Vec3::new(-1.0, 0.0, 0.0),
            "y" | "+y" => Vec3::new(0.0, 1.0, 0.0),
            "-y" => Vec3::new(0.0, -1.0, 0.0),
            "-z" => Vec3::new(0.0, 0.0, -1.0),
            _ => Vec3::new(0.0, 0.0, 1.0), // "z" / "+z" / 未知
        }
    } else if let Some(arr) = args.get("build_dir").and_then(|v| v.as_array()) {
        // 問85: 要素数が 3 未満なら z=1.0 のサイレント補完をせずに +Z デフォルトへ
        // フォールバックする。[1,0] を渡して x-build を意図したAIが対角 [1,0,1] で
        // オーバーハング解析される誤りを防ぐ。
        if arr.len() >= 3 {
            Vec3::new(
                arr[0].as_f64().unwrap_or(0.0),
                arr[1].as_f64().unwrap_or(0.0),
                arr[2].as_f64().unwrap_or(0.0),
            )
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        }
    } else {
        Vec3::new(0.0, 0.0, 1.0)
    }
}

fn tool_validate(session: &Session, args: &Value) -> ToolResult {
    let res = arg_resolution(args, 48);
    let min_wall = args
        .get("min_wall_mm")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let max_overhang = args
        .get("max_overhang_deg")
        .and_then(|v| v.as_f64())
        .unwrap_or(45.0);
    // 問68: ビルド方向を明示受け取り (デフォルト +Z)。
    let build_dir = arg_build_dir(args);

    let scene = &session.scene;
    let (lo_b, hi_b) = scene.sampling_box();
    let mesh = polygonize(scene, lo_b, hi_b, res);
    // SDF を渡し、局所薄肉の内向きレイ探針を有効化する (問58)。
    let report = validate_with_field(&mesh, Some(scene), min_wall, max_overhang, build_dir);
    // 機械可読な構造化 JSON を返す (問63): AI が code で分岐し指標を直接読める。
    ToolResult::text(report.to_json().to_string())
}

// ── base64 (RFC 4648, std のみ) ───────────────────────────────────────────────

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(T[((n >> 18) & 0x3F) as usize]);
        out.push(T[((n >> 12) & 0x3F) as usize]);
        out.push(if chunk.len() > 1 {
            T[((n >> 6) & 0x3F) as usize]
        } else {
            b'='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 0x3F) as usize]
        } else {
            b'='
        });
    }
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_accepts_project_relative_paths() {
        assert!(sandbox_write_path("out.stl").is_ok());
        assert!(sandbox_write_path("sub/dir/out.stl").is_ok());
        assert!(sandbox_write_path("./out.stl").is_ok());
    }

    #[test]
    fn sandbox_rejects_traversal_and_absolute() {
        // 問15: パストラバーサル・絶対パスを拒否する。
        assert!(sandbox_write_path("../escape.stl").is_err());
        assert!(sandbox_write_path("a/../../escape.stl").is_err());
        assert!(sandbox_write_path("/etc/passwd").is_err());
        assert!(sandbox_write_path("/tmp/x.stl").is_err());
        assert!(sandbox_write_path("").is_err());
    }

    #[test]
    fn export_tool_rejects_unsafe_path() {
        // 経路全体: run_script で正本を設定し export が脱出パスを拒否する。
        let mut session = Session::new();
        let args = json::obj([("path", json::s("../../escape.stl"))]);
        let r = call_tool(&mut session, "export", &args);
        assert!(r.is_error, "export must reject traversal path");
    }

    #[test]
    fn resolution_is_clamped_to_safe_range() {
        // 問18: 過大・0・負・非有限の resolution を安全側へ丸め OOM/panic を防ぐ。
        assert_eq!(
            arg_resolution(&json::obj([("resolution", json::n(1e9))]), 48),
            MAX_RESOLUTION
        );
        assert_eq!(
            arg_resolution(&json::obj([("resolution", json::n(0.0))]), 48),
            1
        );
        assert_eq!(
            arg_resolution(&json::obj([("resolution", json::n(-5.0))]), 48),
            1
        );
        assert_eq!(arg_resolution(&json::obj([]), 48), 48);
        assert_eq!(
            arg_resolution(&json::obj([("resolution", json::n(64.0))]), 48),
            64
        );
    }

    #[test]
    fn image_dims_are_clamped() {
        assert_eq!(
            arg_dim(&json::obj([("width", json::n(1e9))]), "width", 512),
            MAX_IMAGE_DIM
        );
        assert_eq!(
            arg_dim(&json::obj([("width", json::n(0.0))]), "width", 512),
            1
        );
        assert_eq!(arg_dim(&json::obj([]), "width", 512), 512);
        // 負の値も安全に下限 1 へ丸められること。
        assert_eq!(
            arg_dim(&json::obj([("width", json::n(-10.0))]), "width", 512),
            1
        );
    }

    #[test]
    fn run_script_surfaces_empty_mesh_issue_code() {
        // 問46: スクリプト自体は有効だが mesh が空になる場合、
        // is_error=false のまま応答テキストに EMPTY_MESH コードを含むこと。
        // AI 自己修正ループが "scene updated" を成功とみなさないようにする。
        let mut s = Session::new();
        // offset(-100) で isosurface が sampling box の外に出て EMPTY_MESH になる。
        let args = json::obj([(
            "script",
            json::s(r#"{"op":"offset","amount":-100.0,"shape":{"op":"sphere","r":1.0}}"#),
        )]);
        let r = call_tool(&mut s, "run_script", &args);
        // スクリプト自体は有効なので is_error=false。
        assert!(!r.is_error, "valid script must not set is_error");
        let text = r.content[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            text.contains("EMPTY_MESH"),
            "run_script must surface EMPTY_MESH in response, got: {text}"
        );
        // "scene updated" は依然として含まれる (セッションは更新済み)。
        assert!(
            text.contains("scene updated"),
            "session must still be marked as updated: {text}"
        );
    }
}
