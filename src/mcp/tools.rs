//! MCP ツール定義 (Phase 0.5/2/3)。

use crate::core::{Sdf, Vec3};
use crate::extract::polygonize;
use crate::io::{gltf, stl};
use crate::mcp::json::{self, Value};
use crate::render::{render, Camera};
use crate::script::eval_scene;
use crate::verify::{validate, Severity};

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
            ],
        ),
        tool_def(
            "export",
            "Export the current scene to a mesh file. Format is chosen by the path extension: \
             \".glb\" writes binary glTF 2.0 (indexed, viewable in browsers/Blender); \
             any other extension writes binary STL. Returns the output file path.",
            &[
                (
                    "path",
                    "string",
                    "Output file path; .glb for glTF, otherwise STL (default: kado-export.stl)",
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
            "Evaluate a KadoScene JSON script and set it as the active scene. Returns a summary.",
            &[
                ("script", "string", "KadoScene JSON scene description", true),
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
            "Validate the current scene mesh for manufacturability (DFM). Returns a structured report.",
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
                    "Maximum overhang angle in degrees (0 to skip, default: 45)",
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
pub fn default_scene() -> Sdf {
    Sdf::sphere(1.0)
        .union(Sdf::cuboid(Vec3::splat(0.8)))
        .difference(Sdf::cylinder(0.3, 2.0))
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
}

impl Default for Session {
    fn default() -> Self {
        Session {
            scene: default_scene(),
            script: None,
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
        "help" => tool_help(),
        other => ToolResult::error(format!("unknown tool: {other}")),
    }
}

fn tool_screenshot(session: &Session, args: &Value) -> ToolResult {
    let view = args.get("view").and_then(|v| v.as_str()).unwrap_or("iso");
    let width = arg_dim(args, "width", 512);
    let height = arg_dim(args, "height", 512);
    let res = arg_resolution(args, 48);

    let scene = &session.scene;
    let (lo_b, hi_b) = scene.sampling_box();
    let mesh = polygonize(scene, lo_b, hi_b, res);
    if mesh.triangles.is_empty() {
        return ToolResult::error("mesh is empty — scene may be outside the bounding box");
    }
    let (lo, hi) = mesh.bounds().unwrap();
    let presets = Camera::presets(lo, hi);
    let cam = presets
        .iter()
        .find(|(n, _)| *n == view)
        .or_else(|| presets.iter().find(|(n, _)| *n == "iso"))
        .unwrap_or(&presets[0])
        .1
        .clone();

    let img = render(&mesh, &cam, width, height);
    ToolResult::image(base64_encode(&img.encode_png()))
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
    // 拡張子 .glb → GLB (インデックス付き・閲覧容易)、それ以外 → STL (問54)。
    let is_glb = path.to_lowercase().ends_with(".glb");
    let write_res = if is_glb {
        gltf::write_glb(&mesh, &safe)
    } else {
        stl::write_binary(&mesh, &safe)
    };
    let fmt = if is_glb { "GLB" } else { "STL" };
    match write_res {
        Ok(()) => ToolResult::text(format!(
            "exported {}: {} ({} triangles, manifold={})",
            fmt,
            safe.display(),
            mesh.triangles.len(),
            mesh.is_edge_manifold()
        )),
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
            let d = session.scene.eval(Vec3::new(x, y, z));
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

    let sdf = match eval_scene(&src) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("script error: {e}")),
    };
    // スクリプトが正本 (問2/問12): 評価結果とスクリプトをセッションに保存する。
    // スクリプトを保存しておくことでAIがget_sceneで読み返せる (問26)。
    let (lo_b, hi_b) = sdf.sampling_box();
    let mesh = polygonize(&sdf, lo_b, hi_b, res);
    let report = validate(&mesh, 0.0, 0.0);
    session.scene = sdf;
    session.script = Some(src);
    // 問46: エラーがある場合はコードを明示してAI自己修正ループを補助する。
    // is_error=false のまま (スクリプト自体は有効) だが問題を可視化する。
    let prefix = if report.is_ok() {
        "scene updated".to_string()
    } else {
        let codes: Vec<&str> = report
            .issues
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .map(|e| e.code)
            .collect();
        if codes.is_empty() {
            "scene updated".to_string()
        } else {
            format!("scene updated (check issues: {})", codes.join(", "))
        }
    };
    ToolResult::text(format!("{prefix} — {}", report.summary()))
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

## Workflow

1. Call run_script with your KadoScene JSON.
2. Call screenshot to preview; validate for DFM; export to save STL.
3. Call get_scene to read back the current script if needed.
"#;

fn tool_get_scene(session: &Session) -> ToolResult {
    let (lo, hi) = session.scene.sampling_box();
    let bounds_info = format!(
        "bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}]",
        lo.x, lo.y, lo.z, hi.x, hi.y, hi.z
    );
    match &session.script {
        Some(script) => ToolResult::text(format!("script={script}\n{bounds_info}")),
        None => ToolResult::text(format!(
            "script=(default scene — no run_script call yet)\n{bounds_info}"
        )),
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

    let scene = &session.scene;
    let (lo_b, hi_b) = scene.sampling_box();
    let mesh = polygonize(scene, lo_b, hi_b, res);
    let report = validate(&mesh, min_wall, max_overhang);
    let status = if report.is_ok() { "PASS" } else { "FAIL" };
    let mut lines = vec![format!("[{status}] {}", report.summary())];
    for issue in &report.issues {
        lines.push(format!(
            "  [{:?}] {} — {}",
            issue.severity, issue.code, issue.cause
        ));
        for hint in &issue.fix_hints {
            lines.push(format!("    hint: {hint}"));
        }
    }
    ToolResult::text(lines.join("\n"))
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
