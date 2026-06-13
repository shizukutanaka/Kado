//! MCP ツール定義 (Phase 0.5/2/3)。

use crate::core::{Sdf, Vec3};
use crate::extract::polygonize;
use crate::io::stl;
use crate::mcp::json::{self, Value};
use crate::render::{render, Camera};
use crate::script::eval_scene;
use crate::verify::validate;

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
            ],
        ),
        tool_def(
            "export",
            "Export the current scene as a binary STL file. Returns the output file path.",
            &[
                (
                    "path",
                    "string",
                    "Output file path (default: kado-export.stl)",
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

/// 現時点はデモ形状固定。Phase 2 で DSL スクリプトから構築する (問2)。
pub fn active_scene() -> Sdf {
    Sdf::sphere(1.0)
        .union(Sdf::cuboid(Vec3::splat(0.8)))
        .difference(Sdf::cylinder(0.3, 2.0))
}

pub fn call_tool(name: &str, args: &Value) -> ToolResult {
    match name {
        "screenshot" => tool_screenshot(args),
        "export" => tool_export(args),
        "eval" => tool_eval(args),
        "run_script" => tool_run_script(args),
        "validate" => tool_validate(args),
        other => ToolResult::error(format!("unknown tool: {other}")),
    }
}

fn tool_screenshot(args: &Value) -> ToolResult {
    let view = args.get("view").and_then(|v| v.as_str()).unwrap_or("iso");
    let width = args
        .get("width")
        .and_then(|v| v.as_f64())
        .map(|f| f as usize)
        .unwrap_or(512);
    let height = args
        .get("height")
        .and_then(|v| v.as_f64())
        .map(|f| f as usize)
        .unwrap_or(512);

    let scene = active_scene();
    let mesh = polygonize(&scene, Vec3::splat(-2.0), Vec3::splat(2.0), 48);
    if mesh.triangles.is_empty() {
        return ToolResult::error("mesh is empty — scene may be outside the bounding box");
    }
    let (lo, hi) = mesh.bounds().unwrap();
    let presets = Camera::presets(lo, hi);
    let cam = presets
        .iter()
        .find(|(n, _)| *n == view)
        .unwrap_or(&presets[6])
        .1
        .clone();

    let img = render(&mesh, &cam, width, height);
    ToolResult::image(base64_encode(&img.encode_png()))
}

fn tool_export(args: &Value) -> ToolResult {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("kado-export.stl");
    let res = args
        .get("resolution")
        .and_then(|v| v.as_f64())
        .map(|f| f as usize)
        .unwrap_or(64);

    let scene = active_scene();
    let mesh = polygonize(&scene, Vec3::splat(-2.0), Vec3::splat(2.0), res);
    match stl::write_binary(&mesh, std::path::Path::new(path)) {
        Ok(()) => ToolResult::text(format!(
            "exported: {path} ({} triangles, manifold={})",
            mesh.triangles.len(),
            mesh.is_edge_manifold()
        )),
        Err(e) => ToolResult::error(format!("export failed: {e}")),
    }
}

fn tool_eval(args: &Value) -> ToolResult {
    let x = args.get("x").and_then(|v| v.as_f64());
    let y = args.get("y").and_then(|v| v.as_f64());
    let z = args.get("z").and_then(|v| v.as_f64());
    match (x, y, z) {
        (Some(x), Some(y), Some(z)) => {
            let d = active_scene().eval(Vec3::new(x, y, z));
            ToolResult::text(format!("{d:.6}"))
        }
        _ => ToolResult::error("x, y, z are required numeric fields"),
    }
}

fn tool_run_script(args: &Value) -> ToolResult {
    let src = match args.get("script").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return ToolResult::error("\"script\" field is required"),
    };
    let res = args
        .get("resolution")
        .and_then(|v| v.as_f64())
        .map(|f| f as usize)
        .unwrap_or(32);

    let sdf = match eval_scene(&src) {
        Ok(s) => s,
        Err(e) => return ToolResult::error(format!("script error: {e}")),
    };
    let mesh = polygonize(&sdf, Vec3::splat(-4.0), Vec3::splat(4.0), res);
    let report = validate(&mesh, 0.0, 0.0);
    ToolResult::text(format!("script ok — {}", report.summary()))
}

fn tool_validate(args: &Value) -> ToolResult {
    let res = args
        .get("resolution")
        .and_then(|v| v.as_f64())
        .map(|f| f as usize)
        .unwrap_or(48);
    let min_wall = args
        .get("min_wall_mm")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let max_overhang = args
        .get("max_overhang_deg")
        .and_then(|v| v.as_f64())
        .unwrap_or(45.0);

    let scene = active_scene();
    let mesh = polygonize(&scene, Vec3::splat(-2.0), Vec3::splat(2.0), res);
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
