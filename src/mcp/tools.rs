//! MCP ツール定義 (Phase 0.5/2/3)。

use crate::core::measure::{ray_crossings, spans};
use crate::core::{Sdf, Vec3};
use crate::extract::polygonize;
use crate::io::ExportFormat;
use crate::mcp::json::{self, Value};
use crate::render::{draw_axes, render, Camera};
use crate::script::eval_any;
use crate::verify::{validate, validate_full, DEFAULT_MAX_ASPECT_RATIO};

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
                    "Overlay an X(red)/Y(green)/Z(blue) orientation gnomon with mm scale ticks \
                     (default: true). When on, the response includes a text note with the tick \
                     spacing so you can estimate dimensions from the image.",
                    false,
                ),
                (
                    "projection",
                    "string",
                    "perspective|orthographic (default: perspective). Orthographic keeps true \
                     dimensional proportions (no perspective distortion) — matches the \
                     engineering-drawing convention that front/back/right/left/top/bottom/iso \
                     names imply; useful for judging proportions/alignment rather than a \
                     photo-realistic look.",
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
            "Evaluate the SDF signed distance at a point, in millimeters (Kado: 1 unit = 1 mm). \
             Negative = inside, positive = outside, ~0 = on the surface. The magnitude is exact \
             for primitives but a conservative lower bound on the true distance for \
             composite/smoothed shapes (union/difference/smooth_* yield a Lipschitz-bounded \
             field), so treat it as a safe under-estimate when measuring clearances or wall gaps.",
            &[
                ("x", "number", "X coordinate", true),
                ("y", "number", "Y coordinate", true),
                ("z", "number", "Z coordinate", true),
            ],
        ),
        tool_def(
            "measure",
            "Measure real dimensions by casting a ray through the scene. Returns every \
             surface crossing along the ray plus the span between consecutive crossings — \
             so a span IS the hole diameter / wall thickness / face-to-face distance in mm. \
             USE THIS INSTEAD OF probing with many eval calls: one measure call replaces a \
             hand-rolled bisection search, and unlike eval's magnitude (a conservative lower \
             bound on composites) these distances are exact, because they are found by \
             bisecting the SDF *sign*, which is always exact. Example: to check an M3 \
             clearance hole drilled along Z in a plate, cast a ray across it \
             (from=[-50,0,0], dir=[1,0,0]); crossings are solid→hole→solid and the middle \
             span is the hole diameter (expect 3.2). Uses sphere tracing (Hart 1996), so \
             arbitrarily thin features are never stepped over. ALWAYS CHECK the returned \
             \"complete\" flag: if it is false the ray was cut short (typically because it \
             grazes along a face instead of crossing it) and the crossings/spans are \
             INCOMPLETE — re-aim the ray rather than trusting the numbers.",
            &[
                (
                    "from",
                    "array",
                    "Ray origin [x,y,z] in mm (required)",
                    true,
                ),
                (
                    "dir",
                    "array",
                    "Ray direction [dx,dy,dz]; normalized internally, so returned distances \
                     are true millimeters (required)",
                    true,
                ),
                (
                    "max_distance",
                    "number",
                    "How far along the ray to search, in mm (default: scene diagonal × 3)",
                    false,
                ),
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
             JSON report: {ok, triangles, manifold, volume, volume_reliable, \
             surface_area, bbox:{min:[x,y,z],max:[x,y,z]}|null, dims_mm:[x,y,z], \
             center_of_mass:[x,y,z]|null, measured_min_wall:number|null, \
             body_count:int|null, cavity_count:int|null, bed_contact_area:number|null, \
             digest, resolution:int, \
             issues:[{severity:\"error\"|\"warning\"|\"info\", code, cause, hints:[], \
             location:[x,y,z]|null}]}. \
             Only \"error\" severity sets ok=false; \"warning\"/\"info\" are advisory \
             (ENCLOSED_CAVITY is \"info\"). resolution echoes the effective (clamped) \
             extraction resolution used for this report. \
             Units: 1 coordinate unit = 1 mm, so volume is mm³, surface_area mm², \
             bed_contact_area mm². Estimate material: mass_g = volume/1000 × density \
             (PLA~1.24, ABS~1.04, PETG~1.27, resin~1.1 g/cm³) — see help for cost/infill notes. \
             location gives the 3-D coordinates of the problem (e.g. worst overhang centroid, \
             thinnest wall vertex, or center of mass for UNSTABLE) so the AI can zoom in. \
             All issue codes: EMPTY_MESH (no geometry), NON_MANIFOLD (self-intersecting faces), \
             OPEN_MESH (boundary edges, unprintable), NEGATIVE_VOLUME (inverted/inside-out mesh), \
             MULTIPLE_BODIES (separate shells), \
             THIN_WALL (local section < min_wall_mm), OVERHANG (angle > max_overhang_deg), \
             SUSPICIOUS_SCALE (overall size < min_wall, likely wrong units), \
             UNSTABLE (center of mass falls outside the base footprint → tips over), \
             HIGH_ASPECT_RATIO (build height / lateral size over max_aspect_ratio, \
             default 8 — sways during printing; the measured value is always in aspect_ratio), \
             ENCLOSED_CAVITY (info: fully-sealed internal void traps resin/support — needs a drain hole). \
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
                    "Minimum wall thickness threshold in mm (0 to skip, default: 0.5). \
                     Rule of thumb: set to ~1.25× nozzle diameter (0.5 mm suits a 0.4 mm FDM \
                     nozzle); resin/SLA can go thinner but ~1 mm walls are more robust",
                    false,
                ),
                (
                    "max_overhang_deg",
                    "number",
                    "Maximum overhang angle in degrees from horizontal (0 to skip, default: 45)",
                    false,
                ),
                (
                    "max_aspect_ratio",
                    "number",
                    "Slenderness threshold: build height / lateral size (0 to skip, default: 8). \
                     The safe value depends on ABSOLUTE size, nozzle and material, not the ratio \
                     alone — FDM practice puts a 0.3mm-wide wire at ~7mm tall (ratio ~23) and a \
                     1.5mm-wide one at ~30mm (ratio ~20) before waving. The default 8 is \
                     deliberately conservative for complex parts; raise it when your features are \
                     thick. The measured ratio is ALWAYS returned as `aspect_ratio`, whether or \
                     not it trips this threshold",
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
        ("annotations", tool_annotations(name)),
    ])
}

/// MCP ツール注釈 (問251; 2025-03-26+ 仕様)。クライアント/LLM が「副作用なしで安全に
/// 呼べる読み取り専用ツールか」「シーン状態を変更するか」を判断できるようにする。
/// 注釈は信頼できない助言 (untrusted hint) であり権限の保証ではない (仕様)。
/// Kado は外部実体と相互作用しないため openWorldHint は常に false (閉世界・決定的)。
/// destructiveHint/idempotentHint は readOnlyHint=false のときのみ意味を持つ。
fn tool_annotations(name: &str) -> Value {
    // (title, read_only, destructive, idempotent)
    let (title, read_only, destructive, idempotent) = match name {
        "eval" => ("Evaluate signed distance at a point", true, false, true),
        "measure" => ("Measure dimensions along a ray", true, false, true),
        "validate" => ("Validate manufacturability (DFM)", true, false, true),
        "get_scene" => ("Get the current scene script", true, false, true),
        "help" => ("DSL and tool reference", true, false, true),
        "screenshot" => ("Render the scene to a PNG", true, false, true),
        // export はファイルを書くが additive (シーン状態は不変, 同一シーン→同一ファイル)。
        "export" => ("Export the mesh to a file", false, false, true),
        // run_script はシーン正本を置換する (undo 可能だが additive ではない)。
        "run_script" => ("Run a script, replacing the scene", false, true, false),
        "undo_script" => ("Undo the last script change", false, true, false),
        _ => (name, false, true, false),
    };
    let mut m = std::collections::BTreeMap::new();
    m.insert("title".into(), json::s(title));
    m.insert("readOnlyHint".into(), json::b(read_only));
    m.insert("openWorldHint".into(), json::b(false));
    if !read_only {
        m.insert("destructiveHint".into(), json::b(destructive));
        m.insert("idempotentHint".into(), json::b(idempotent));
    }
    Value::Object(m)
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
    /// 画像に説明テキストを添える (問288: 軸目盛りの寸法凡例など)。
    /// MCP のツール結果は複数コンテンツを許すため、text と image を並べて返す。
    fn image_with_text(b64: String, note: impl Into<String>) -> Self {
        ToolResult {
            content: vec![
                json::obj([("type", json::s("text")), ("text", json::s(note))]),
                json::obj([
                    ("type", json::s("image")),
                    ("data", json::s(b64)),
                    ("mimeType", json::s("image/png")),
                ]),
            ],
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
        "measure" => tool_measure(session, args),
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
    let mut cam = match presets.iter().find(|(n, _)| *n == view) {
        Some((_, c)) => c.clone(),
        None => {
            let valid: Vec<&str> = presets.iter().map(|(n, _)| *n).collect();
            return ToolResult::error(format!(
                "unknown view '{view}'; valid views: {}",
                valid.join(", ")
            ));
        }
    };
    // 問267: front/back/right/left/top/bottom/iso はエンジニアリング図面の多面図・
    // 等角投影法に由来し、伝統的に正射影 (寸法比率が歪まない) で描かれる。
    // 既定は既存挙動を維持するため透視投影のまま、opt-in で切り替える。
    let projection = args
        .get("projection")
        .and_then(|v| v.as_str())
        .unwrap_or("perspective");
    cam.ortho = match projection {
        "perspective" => false,
        "orthographic" => true,
        other => {
            return ToolResult::error(format!(
                "unknown projection '{other}'; valid: perspective, orthographic"
            ));
        }
    };

    // スーパーサンプルして縮小 (アンチエイリアス)。
    let big = render(&mesh, &cam, width * samples, height * samples);
    let mut img = big.downsample(samples);
    // 向きの基準として座標軸グノモンを重ねる (問66; axes=false で無効化)。
    let show_axes = args.get("axes").and_then(|v| v.as_bool()).unwrap_or(true);
    let tick_step = if show_axes {
        let center = (lo + hi) * 0.5;
        let length = (hi - lo).length() * 0.35;
        draw_axes(&mut img, &cam, center, length)
    } else {
        0.0
    };
    let png = base64_encode(&img.encode_png());
    if show_axes && tick_step > 0.0 {
        // 軸目盛りの寸法凡例を添える (問288): AI が画像から寸法を概算できる。
        ToolResult::image_with_text(
            png,
            format!(
                "Axis gnomon at the model center: X=red, Y=green, Z=blue (1 unit = 1 mm). \
                 Tick marks are spaced every {tick_step} mm along each axis — use them to \
                 estimate dimensions from the image."
            ),
        )
    } else {
        ToolResult::image(png)
    }
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
    // 拡張子で形式を選択 (問124: CLI と共有する単一の真実源 io::ExportFormat)。
    let format = ExportFormat::from_path(path);
    let fmt = format.label();
    let write_res = format.write(&mesh, &safe);
    match write_res {
        Ok(()) => {
            // 問72: 相対パスだけでは MCP サーバーの CWD が不明な AI はファイルの場所を
            // 特定できない。書き込み後に canonicalize で絶対パスを解決して返す。
            let abs_path = std::fs::canonicalize(&safe)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| safe.display().to_string());
            // 問92: manifold 真偽だけでは構造 DFM を網羅しない。watertight でも
            // MULTIPLE_BODIES (離れた複数ボディ) や NEGATIVE_VOLUME (裏返し) は
            // manifold=true のまま見逃される。run_script (問81) と同じ閾値非依存の
            // 構造チェック validate(mesh, 0, 0) を出力解像度で実行し issue code を併記する。
            // これにより「manifold=true ⇒ DFM 合格」という AI の誤認を防ぐ。
            let report = validate(&mesh, 0.0, 0.0);
            let manifold = report.is_manifold;
            let codes: Vec<&str> = report.issues.iter().map(|e| e.code).collect();
            let dfm_note = if codes.is_empty() {
                String::new()
            } else {
                format!(
                    " [structural DFM issues at this resolution: {}; \
                      run validate(resolution={res}) for full DFM incl. thin walls/overhang]",
                    codes.join(", ")
                )
            };
            // 問91: 出力ファイルの再現性同一性を記録・検証できるよう digest と
            // resolution を併記する (問61/問90 と同じ契約)。三角形数だけでは
            // 異なる形状が同数になりうるため弱い指標。digest が正準な内容同一性。
            ToolResult::text(format!(
                "exported {fmt}: {abs_path} ({} triangles, manifold={manifold}, \
                 resolution={res}, digest={:016x}){dfm_note}",
                mesh.triangles.len(),
                report.digest,
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

/// `[x,y,z]` の数値3要素配列を取り出す (問299)。
fn arg_vec3(args: &Value, key: &str) -> Result<Vec3, String> {
    let a = args
        .get(key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("\"{key}\" must be an array of 3 numbers [x,y,z]"))?;
    if a.len() != 3 {
        return Err(format!(
            "\"{key}\" must have exactly 3 elements, got {}",
            a.len()
        ));
    }
    let mut c = [0.0f64; 3];
    for (i, slot) in c.iter_mut().enumerate() {
        *slot = a[i]
            .as_f64()
            .ok_or_else(|| format!("\"{key}\"[{i}] must be a number"))?;
    }
    Ok(Vec3::new(c[0], c[1], c[2]))
}

/// 光線に沿った表面交差を返す (問299)。第一原理: AI が寸法を1呼出で測れるようにし、
/// KPI「平均ツール呼出 ≤15/タスク」(Plan.md §7) を守れるようにする。
fn tool_measure(session: &Session, args: &Value) -> ToolResult {
    let from = match arg_vec3(args, "from") {
        Ok(v) => v,
        Err(e) => return ToolResult::error(e),
    };
    let dir = match arg_vec3(args, "dir") {
        Ok(v) => v,
        Err(e) => return ToolResult::error(e),
    };
    // 既定の探索距離: シーン対角の3倍 (外から撃って通り抜けるのに十分)。
    let (lo, hi) = session.scene.sampling_box();
    let default_max = ((hi - lo).length() * 3.0).max(1.0);
    let max_distance = match args.get("max_distance").and_then(|v| v.as_f64()) {
        Some(v) => v,
        None => default_max,
    };
    let m = match ray_crossings(&session.scene, from, dir, max_distance) {
        Ok(c) => c,
        Err(e) => return ToolResult::error(e),
    };
    let sp = spans(&m.crossings);
    let items: Vec<Value> = m
        .crossings
        .iter()
        .map(|c| {
            json::obj([
                ("distance", json::n(c.distance)),
                (
                    "point",
                    json::arr([json::n(c.point.x), json::n(c.point.y), json::n(c.point.z)]),
                ),
                ("entering", json::b(c.entering)),
            ])
        })
        .collect();
    let mut pairs = vec![
        ("crossings", Value::Array(items)),
        ("spans", json::arr(sp.iter().map(|v| json::n(*v)))),
        ("count", json::n(m.crossings.len() as f64)),
        // 問301: 完走したかを必ず伝える。サイレントな打ち切りを作らない。
        ("complete", json::b(m.complete)),
    ];
    if !m.complete {
        // 不完全なときは理由と対処を添える (AI が誤った寸法を確信して読むのを防ぐ)。
        pairs.push((
            "warning",
            json::s(
                "ray was cut short before reaching max_distance — results are INCOMPLETE and \
                 there may be further crossings. This usually means the ray grazes along a \
                 surface (a ray sliding on a face advances only ~1e-6mm per step). Aim the ray \
                 so it crosses surfaces transversely, or reduce max_distance.",
            ),
        ));
    }
    ToolResult::text(
        Value::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()).to_string(),
    )
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
    // 問93: summary は digest を含むが、このチェックは res=32 (validate 既定48・
    // export 既定64 より粗い)。解像度を開示しないと AI は (a) summary の digest が
    // validate/export の digest と一致しない理由を説明できず、(b) 粗い res の
    // 「issue なし」を確定的と誤認する。check_resolution を併記し、authoritative な
    // DFM は validate を使うよう案内する (問90/91/92 と同じ解像度透明性)。
    ToolResult::text(format!(
        "{prefix} — {} check_resolution={res} \
         (quick check; validate/export use higher res by default — digests differ across resolutions)",
        report.summary()
    ))
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

## Units & coordinates (問62/問95)

All lengths are in MILLIMETERS (1 coordinate unit = 1 mm). e.g. sphere(10) is a
10 mm-radius ball, not arbitrary units. +Z is "up" for FDM build direction by default.
Authoring at the wrong scale triggers a SUSPICIOUS_SCALE warning from validate.

## Primitives

sphere        {"op":"sphere","r":1.0}
              r (req): radius > 0

cuboid        {"op":"cuboid","x":1.0,"y":1.0,"z":1.0}
              x,y,z (req): half-extents > 0 (or use "s" for uniform)

cylinder      {"op":"cylinder","r":0.5,"h":1.0}
              axis along Z, centered at origin (spans z=-h..+h);
              r (req): radius > 0; h (req): half-height > 0

torus         {"op":"torus","major":1.0,"minor":0.25}
              ring lies in the XY plane, hole faces Z;
              major (req): ring radius > 0; minor (req): tube radius > 0 AND < major
              (minor >= major self-intersects: horn/spindle torus, non-manifold)

cone          {"op":"cone","r":0.5,"h":1.5}
              axis along Z: apex at z=0, base at z=-h; r (req) > 0; h (req) > 0

capsule       {"op":"capsule","h":0.5,"r":0.3}
              axis along Z (z=-h..+h plus radius hemispherical caps);
              h (req): half-height >= 0; r (req): radius > 0

rounded_box   {"op":"rounded_box","x":0.8,"y":0.8,"z":0.8,"r":0.1}
              x,y,z (req): half-extents > 0; r (req): corner radius > 0

ellipsoid     {"op":"ellipsoid","x":2.0,"y":1.0,"z":0.5}
              x,y,z (req): per-axis radii > 0 (or "s" for uniform = sphere)

prism         {"op":"prism","n":6,"r":1.0,"h":1.0}
              regular n-gon prism, axis along Z (spans z=-h..+h). Exact SDF
              (not an approximation), so extraction is watertight like any
              other primitive. Useful for hex nuts, bolt heads, and other
              polygonal FDM parts. n (req): integer sides >= 3;
              r (req): circumradius > 0; h (req): half-height > 0.
              Large n approaches a cylinder(r,h).

## Boolean Operations

union         {"op":"union","a":<sdf>,"b":<sdf>}
intersection  {"op":"intersection","a":<sdf>,"b":<sdf>}
difference    {"op":"difference","a":<sdf>,"b":<sdf>}  (a minus b)

smooth_union         {"op":"smooth_union","a":<sdf>,"b":<sdf>,"k":0.3}
smooth_intersection  {"op":"smooth_intersection","a":<sdf>,"b":<sdf>,"k":0.3}
smooth_difference    {"op":"smooth_difference","a":<sdf>,"b":<sdf>,"k":0.3}  (a minus b, blended)
              k: blend radius > 0 (default 0.3; k<=0 rejected — use the hard
              union/intersection/difference op for a sharp boundary)
              smooth_* make a ROUND (filleted) transition.

chamfer_union        {"op":"chamfer_union","a":<sdf>,"b":<sdf>,"k":0.3}
chamfer_intersection {"op":"chamfer_intersection","a":<sdf>,"b":<sdf>,"k":0.3}
chamfer_difference   {"op":"chamfer_difference","a":<sdf>,"b":<sdf>,"k":0.3}  (a minus b, chamfered)
              k: chamfer width > 0 (default 0.3; k<=0 rejected — use the hard op).
              chamfer_* make a FLAT (45deg beveled) transition — the angular
              counterpart to smooth_* (round). Use for print-bed bevels,
              deburred edges, and assembly clearances.

## Transforms

translate     {"op":"translate","x":1.0,"y":0.0,"z":0.0,"shape":<sdf>}
scale         {"op":"scale","s":2.0,"shape":<sdf>}          s > 0 (uniform, exact)
              one factor for all axes; distance field stays exact (Lipschitz=1).
scale_xyz     {"op":"scale_xyz","sx":2.0,"sy":1.0,"sz":0.5,"shape":<sdf>}
              sx,sy,sz > 0 (non-uniform). Distance is NOT exact off the
              smallest-scale axis, but sign is always correct, magnitude is
              always a safe UNDERESTIMATE of the true distance (never reports
              a wall as thicker than it is), and the field stays Lipschitz=1
              (same guarantee tier as every other primitive). For shapes with
              per-axis extents as a primitive, cuboid/ellipsoid/rounded_box's
              own x/y/z params are usually simpler than composing scale_xyz.
offset        {"op":"offset","amount":0.1,"shape":<sdf>}    inflates/deflates
shell         {"op":"shell","thickness":0.1,"shape":<sdf>}  thickness > 0.
              Hollows the solid INWARD: keeps the outer surface and carves a cavity,
              leaving a wall of `thickness` just inside the surface. Outer size is
              unchanged. e.g. shell(sphere(1.0),0.2) → hollow ball, outer r=1, inner r=0.8.
mirror_x      {"op":"mirror_x","shape":<sdf>}  Makes the shape symmetric about the
mirror_y      {"op":"mirror_y","shape":<sdf>}  axis=0 plane: it KEEPS the positive-axis
mirror_z      {"op":"mirror_z","shape":<sdf>}  half and reflects it onto the negative
              half (the shape's original negative-axis half is REPLACED, not kept).
              To mirror a part to both sides, place it on the +axis side first.
rotate_x      {"op":"rotate_x","angle":90,"shape":<sdf>}  angle in DEGREES
rotate_y      {"op":"rotate_y","angle":45,"shape":<sdf>}
rotate_z      {"op":"rotate_z","angle":30,"shape":<sdf>}
rotate        {"op":"rotate","ax":1,"ay":1,"az":0,"angle":45,"shape":<sdf>}
              rotate around an arbitrary axis (ax,ay,az; auto-normalized, must be
              non-zero), angle in DEGREES. Use this instead of composing
              rotate_x/y/z when you need a diagonal axis — chaining canonical
              rotations does NOT equal a single arbitrary-axis rotation in
              general (Euler composition order matters).
repeat        {"op":"repeat","x":2.0,"nx":2,"shape":<sdf>}
              period per axis (x/y/z); count per axis (nx/ny/nz, default 1).
              count is copies PER SIDE of the origin → total = 2*count+1 per axis.
              e.g. nx=2 gives 5 copies along x (2 left + center + 2 right).
cut           {"op":"cut","nx":0,"ny":0,"nz":1,"offset":0.5,"shape":<sdf>}
              plane cut: keeps dot(p,(nx,ny,nz)) <= offset, removes the half the
              normal points into. offset defaults to 0 (plane through origin).
              Cross-section: thin slab via two cuts.
flatten       {"op":"flatten","at":0,"shape":<sdf>}
              FDM printable base: cuts a flat bottom at z=at (default 0), keeping
              z>=at. Safe, intent-named shortcut for the common cut case (avoids
              the nz=+1/-1 normal-direction mistake).

## Example: sphere with a cylindrical hole

{"op":"difference",
 "a":{"op":"sphere","r":1.5},
 "b":{"op":"cylinder","r":0.4,"h":2.0}}

## Example: chamfer (flat 45deg bevel) vs smooth (round fillet)

Both blend a cylindrical boss onto a base plate. chamfer_* makes a FLAT bevel
at the joint; smooth_* makes a ROUND fillet. Same k = blend width:

  chamfer_union(cuboid(1.0, 1.0, 0.3), cylinder(0.4, 1.2), 0.2)   // 45deg bevel
  smooth_union(cuboid(1.0, 1.0, 0.3), cylinder(0.4, 1.2), 0.2)    // rounded fillet

Use chamfer_* for print-bed relief, deburred edges, and press-fit clearances;
use smooth_* for organic/rounded transitions.

## Compact text DSL (alternative to JSON)

run_script also accepts a concise function-call syntax (token-efficient). The same
hole example:

  difference(sphere(1.5), cylinder(0.4, 2.0))

DSL arg order mirrors the constructors:
  sphere(r) · cuboid(s) or cuboid(x,y,z) · cylinder(r,h) · torus(major,minor)
  cone(r,h) · capsule(h,r) · rounded_box(s,r) or (x,y,z,r) · ellipsoid(s) or (x,y,z)
  prism(n,r,h)
  union/intersection/difference(a,b) · smooth_*(a,b[,k]) · chamfer_*(a,b[,k])
  translate(x,y,z,shape) · scale(s,shape) · scale_xyz(sx,sy,sz,shape)
  offset(amount,shape) · shell(t,shape)
  rotate_x/y/z(deg,shape) · rotate(ax,ay,az,deg,shape) · mirror_x/y/z(shape)
  repeat(px,py,pz[,nx,ny,nz],shape)
  cut(nx,ny,nz,shape) or cut(nx,ny,nz,offset,shape) · flatten(shape) or flatten(at,shape)

## Measuring real dimensions (measure tool)

validate reports WHOLE-MODEL numbers (dims_mm, volume). To check a SPECIFIC feature
— a hole diameter, a wall thickness, a face-to-face distance — cast a ray with
measure and read the spans. Do NOT hand-roll a bisection search with many eval
calls: one measure call replaces ~30 evals, and eval's magnitude is only a lower
bound on composites while measure's distances are exact (it bisects the SDF sign,
which is always exact).

  measure(from=[-50,0,0], dir=[1,0,0])

returns {crossings:[{distance,point,entering}...], spans:[...], count, complete}. Each
span is the length between consecutive crossings, in mm.

ALWAYS check "complete". If false, the ray stopped before max_distance and the result
is INCOMPLETE (more crossings may exist) — a "warning" field explains why. The usual
cause is a ray that grazes ALONG a surface instead of crossing it: with distance ~0 the
tracer advances only ~1e-6mm per step and cannot finish. Re-aim so the ray hits
surfaces transversely. complete=false with zero crossings does NOT mean "nothing there".

Recipe — verify an M3 clearance hole (Ø3.2) drilled along Z through a plate:

  run_script: difference(cuboid(20.0, 20.0, 2.0), cylinder(1.6, 10.0))
  measure(from=[-50,0,0], dir=[1,0,0])
  -> spans = [18.4, 3.2, 18.4]   solid, HOLE (=3.2 diameter), solid

Recipe — wall thickness: aim the ray through the wall along its normal; the span
between the two crossings is the thickness. Ray direction is normalized, so
distances are true millimeters regardless of the vector's length.

## Workflow

1. Call run_script with your KadoScene JSON or text DSL.
2. Call screenshot to preview (valid views: front|back|right|left|top|bottom|iso).
3. Call measure to check specific dimensions (hole Ø, wall thickness) against intent.
4. Call validate for DFM; export to save STL/GLB/3MF/HTML.
4. Call get_scene to read back the current script if needed (also reports undo availability).
5. If a run_script went wrong, call undo_script to restore the previous scene (single-level).

## validate build_dir parameter

The validate tool checks overhang relative to a build direction (default +Z = gravity up).
If your 3D printer builds along a different axis, specify build_dir:
  validate(build_dir="z")   same as default (+Z up)
  validate(build_dir="-z")  build head-down (inverted)
  validate(build_dir="y")   build along Y axis

## validate issue codes (問79)

Each issue object has {severity, code, cause, hints:[], location:[x,y,z]|null}.
location is the 3-D coordinates of the problem (null for non-spatial issues):
  OVERHANG        → centroid of the worst-angled triangle
  THIN_WALL       → surface vertex where the thinnest wall was probed
  UNSTABLE        → center of mass (same as report.center_of_mass)
  others          → null

Branch on issue.code to categorize results:
  EMPTY_MESH        — no triangles (script may be outside bounding box)
  NON_MANIFOLD      — self-intersecting faces (boolean degeneracy)
  OPEN_MESH         — boundary edges present (shape not watertight; cannot print)
  NEGATIVE_VOLUME   — signed volume < 0 (mesh inverted/inside-out; check orientation)
  MULTIPLE_BODIES   — disconnected shells (may need to merge or orient separately)
  THIN_WALL         — local wall < min_wall_mm (SDF-ray probe); location = thin vertex
  OVERHANG          — surface > max_overhang_deg from horizontal; location = worst face centroid
  SUSPICIOUS_SCALE  — max dimension < min_wall_mm (likely authored in wrong units)
  UNSTABLE          — center of mass outside the base footprint; location = COM coords
  HIGH_ASPECT_RATIO — build height / lateral size exceeds max_aspect_ratio (default 8;
                      sways during printing, risk of delamination). The safe ratio depends on
                      ABSOLUTE size: a 0.3mm-wide wire tolerates ~7mm tall (ratio ~23), a 1.5mm
                      one ~30mm (ratio ~20). Raise max_aspect_ratio for thick features, 0 skips.
                      The measured ratio is ALWAYS returned as `aspect_ratio`.
  ENCLOSED_CAVITY   — info: fully-sealed internal void (traps resin/support; add a drain hole for SLA)

## printability rules of thumb (問250; FDM/resin community practice)

Use these to set thresholds and interpret results (adjust per printer/material):
  min wall          ≈ nozzle diameter (commonly 0.4 mm FDM); aim ≥2× nozzle (~0.8 mm) for strength
  min hole/pin      ≳ nozzle diameter; small holes print undersized — design holes oversized
  feature width     prefer multiples of nozzle width (0.8 mm FDM prints cleaner than 1.0 mm)
  overhang          ≤45° self-supporting; up to ~60° with tuning; beyond needs support
  bridge vs overhang  a flat ceiling supported at BOTH ends is a bridge (spans a gap) — short
                    bridges (a few mm) print without support; a one-sided cantilever or a long
                    span sags. validate's OVERHANG flags both flat ceilings and cantilevers as
                    steep angles; judge by span length (dims_mm) whether support is truly needed
  resin/SLA         walls thinner-capable but ~1 mm+ is robust; ~3 mm around screw holes;
                    hollow parts need a drain hole (see ENCLOSED_CAVITY)
  report.measured_min_wall  the actual thinnest wall — compare to your nozzle to judge margin
  bed adhesion      larger build-plate contact resists warping/peeling; small footprint + tall
                    part risks detachment (compare report.bed_contact_area to the part size)
  report.bed_contact_area   area touching the build plate (build_dir lowest layer); ~0 = point
                    contact (e.g. a sphere) which needs a raft/brim or a flat-cut base

## material, weight & cost estimation (問253)

Coordinates are millimeters (1 unit = 1 mm), so report.volume is in mm³ and
report.surface_area in mm². To estimate filament/resin use from a validate report:
  solid mass (g)    = volume / 1000 × density        (mm³→cm³ is /1000)
  density (g/cm³)   PLA ~1.24, ABS ~1.04, PETG ~1.27, Nylon ~1.14, resin ~1.10
  cost              ≈ mass_g × price_per_gram (e.g. ~2.4 yen/g for 2400 yen/kg PLA)
  IMPORTANT         report.volume is the SOLID volume — an UPPER BOUND on material.
                    FDM prints are usually 15–30% infill + a few perimeters, so actual
                    filament is far less; multiply by your infill fraction for a real
                    estimate. A thin shell uses ≈ surface_area × wall_thickness of material.
                    print time also scales with volume/infill and surface_area (perimeters).

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
        Some(script) => ToolResult::text(format!("script={script}\n{bounds_info}\n{undo_info}")),
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
        // 問263: 要素数が3以上でも、非数値要素 (文字列・真偽値・null 等) を
        // `unwrap_or(0.0)` で0扱いにすると [1,0,"up"] が静かに [1,0,0] という
        // 別のビルド方向になり、AI の意図しない軸でオーバーハング解析されてしまう
        // (問85 と同じ「部分的な誤り訂正」の危険)。全要素が数値のときのみ採用し、
        // 1つでも数値でなければ配列全体が短すぎる場合と同じ +Z デフォルトへ倒す。
        if arr.len() >= 3 {
            match (arr[0].as_f64(), arr[1].as_f64(), arr[2].as_f64()) {
                (Some(x), Some(y), Some(z)) => Vec3::new(x, y, z),
                _ => Vec3::new(0.0, 0.0, 1.0),
            }
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
    // 問305: 細長さ閾値。安全値はプリンタ・材料・絶対寸法に依存するため調整可能にする。
    let max_aspect = args
        .get("max_aspect_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(DEFAULT_MAX_ASPECT_RATIO);
    // 問68: ビルド方向を明示受け取り (デフォルト +Z)。
    let build_dir = arg_build_dir(args);

    let scene = &session.scene;
    let (lo_b, hi_b) = scene.sampling_box();
    let mesh = polygonize(scene, lo_b, hi_b, res);
    // SDF を渡し、局所薄肉の内向きレイ探針を有効化する (問58)。
    let report = validate_full(
        &mesh,
        Some(scene),
        min_wall,
        max_overhang,
        build_dir,
        max_aspect,
    );
    // 機械可読な構造化 JSON を返す (問63): AI が code で分岐し指標を直接読める。
    // 問90: digest の決定性契約 (問61) は「同一解像度」が前提だが、report 単体には
    // 解像度が含まれず digest が再現性検証に使えなかった。resolution を併記する。
    let mut report_json = report.to_json();
    if let Value::Object(ref mut map) = report_json {
        map.insert("resolution".to_string(), json::n(res as f64));
    }
    ToolResult::text(report_json.to_string())
}

// ── base64 (RFC 4648, std のみ) ───────────────────────────────────────────────

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
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
    fn every_declared_tool_is_dispatched_and_explicitly_annotated() {
        // 問294: ツールは3箇所に並ぶ — tool_list (スキーマ)・call_tool (ディスパッチ)・
        // tool_annotations (安全ヒント)。これらがドリフトすると、問292 の DSL と同型の
        // 「宣言したのに呼べない / 別物として注釈される」表面積バグが生じる。
        // tool_list を真実源に、全ツールが (1) call_tool でディスパッチされ
        // (=「unknown tool」にならない)、(2) tool_annotations に明示エントリを持つ
        // (=フォールバック `_ => (name, ...)` に落ちない) ことを固定する。
        let list = tool_list();
        let names: Vec<String> = list
            .as_array()
            .expect("tool_list is an array")
            .iter()
            .map(|t| {
                t.get("name")
                    .and_then(|v| v.as_str())
                    .expect("each tool has a name")
                    .to_string()
            })
            .collect();
        assert_eq!(names.len(), 9, "tool_list must declare 9 tools (SPEC §5)");

        for name in &names {
            // (2) 注釈の網羅: フォールバックは title=name を返すので、title != name なら
            // 明示エントリがある。
            let ann = tool_annotations(name);
            let title = ann.get("title").and_then(|v| v.as_str()).unwrap_or("");
            assert_ne!(
                title, name,
                "tool `{name}` has no explicit tool_annotations entry (hit the fallback, \
                 so it would be mislabeled destructive/non-read-only)"
            );

            // (1) ディスパッチの網羅: call_tool が「unknown tool」を返さない。
            // export はファイルを書くので、パストラバーサルを渡して**ディスパッチ後に**
            // サンドボックスで弾かせる (ファイルを作らず、かつ unknown tool でもない)。
            let mut s = Session::new();
            let args = if name == "export" {
                json::obj([("path", json::s("../should-be-rejected.stl"))])
            } else {
                json::obj([])
            };
            let r = call_tool(&mut s, name, &args);
            let txt = r
                .content
                .first()
                .and_then(|c| c.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert!(
                !txt.contains(&format!("unknown tool: {name}")),
                "tool `{name}` is declared in tool_list but not dispatched in call_tool"
            );
        }
    }

    /// tool_list から指定ツールの annotations オブジェクトを取り出す。
    fn annotations_of(name: &str) -> Value {
        let list = tool_list();
        let arr = list.as_array().expect("tool_list is an array");
        let tool = arr
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(name))
            .unwrap_or_else(|| panic!("tool {name} not found"));
        tool.get("annotations")
            .cloned()
            .unwrap_or_else(|| panic!("tool {name} has no annotations"))
    }

    #[test]
    fn read_only_tools_declare_read_only_hint() {
        // 問251: 副作用のないツールは readOnlyHint=true を宣言し、クライアント/LLM が
        // 確認なしで安全に呼べることを示す。
        for name in ["eval", "validate", "get_scene", "help", "screenshot"] {
            let ann = annotations_of(name);
            assert_eq!(
                ann.get("readOnlyHint").and_then(|b| b.as_bool()),
                Some(true),
                "{name} must be readOnlyHint=true"
            );
            // 読み取り専用なら destructiveHint は意味を持たないため省略する (MCP 仕様)。
            assert!(
                ann.get("destructiveHint").is_none(),
                "{name} (read-only) must omit destructiveHint"
            );
            // Kado は閉世界 (外部実体と相互作用しない)。
            assert_eq!(
                ann.get("openWorldHint").and_then(|b| b.as_bool()),
                Some(false),
                "{name} must be openWorldHint=false (closed world)"
            );
        }
    }

    #[test]
    fn state_mutating_tools_declare_not_read_only() {
        // 問251: シーン正本を変更する run_script/undo_script は readOnlyHint=false かつ
        // destructiveHint=true (置換は additive ではない)、idempotentHint=false。
        for name in ["run_script", "undo_script"] {
            let ann = annotations_of(name);
            assert_eq!(
                ann.get("readOnlyHint").and_then(|b| b.as_bool()),
                Some(false),
                "{name} must be readOnlyHint=false"
            );
            assert_eq!(
                ann.get("destructiveHint").and_then(|b| b.as_bool()),
                Some(true),
                "{name} must be destructiveHint=true"
            );
            assert_eq!(
                ann.get("idempotentHint").and_then(|b| b.as_bool()),
                Some(false),
                "{name} must be idempotentHint=false"
            );
        }
    }

    #[test]
    fn export_is_additive_and_idempotent() {
        // 問251: export はファイルを書く (readOnly=false) が additive・冪等
        // (同一シーン→同一ファイル) なので destructive=false, idempotent=true。
        let ann = annotations_of("export");
        assert_eq!(
            ann.get("readOnlyHint").and_then(|b| b.as_bool()),
            Some(false)
        );
        assert_eq!(
            ann.get("destructiveHint").and_then(|b| b.as_bool()),
            Some(false)
        );
        assert_eq!(
            ann.get("idempotentHint").and_then(|b| b.as_bool()),
            Some(true)
        );
    }

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
        // 問139: 空文字列は拒否されるが、空白のみも拒否されなければならない。
        // trim() → empty() で検査するため "   " も Err になる。
        assert!(
            sandbox_write_path("   ").is_err(),
            "whitespace-only path must be rejected"
        );
        assert!(
            sandbox_write_path("\t\n").is_err(),
            "whitespace-only path must be rejected"
        );
    }

    #[test]
    fn sandbox_applies_uniformly_across_all_export_formats() {
        // 問204 (SPEC §7.2): サンドボックスは拡張子非依存。STL だけでなく
        // GLB/3MF/HTML すべてのエクスポートパスが同一の制約 (絶対/トラバーサル拒否) を
        // 受けることを固定する。将来 1 形式だけパスチェックを飛ばす回帰を防ぐ。
        for ext in ["stl", "glb", "3mf", "html"] {
            // 正常なプロジェクト相対パスは許可。
            assert!(
                sandbox_write_path(&format!("model.{ext}")).is_ok(),
                "valid .{ext} path must be accepted"
            );
            assert!(
                sandbox_write_path(&format!("sub/dir/model.{ext}")).is_ok(),
                "nested .{ext} path must be accepted"
            );
            // トラバーサル・絶対パスは形式を問わず拒否。
            assert!(
                sandbox_write_path(&format!("../escape.{ext}")).is_err(),
                ".{ext} traversal must be rejected"
            );
            assert!(
                sandbox_write_path(&format!("/tmp/escape.{ext}")).is_err(),
                ".{ext} absolute path must be rejected"
            );
        }
    }

    #[test]
    fn get_scene_reports_script_bounds_and_undo_state() {
        // 問205 (SPEC §5.1): get_scene ツールは個別テストがなかった。
        // 初期 (デフォルトシーン) では undo_available=false、run_script 後は
        // 現在のスクリプトと undo_available=true を報告することを固定する。
        let mut s = Session::new();
        // 初期状態: prev_scene なし → undo_available=false。
        let r0 = call_tool(&mut s, "get_scene", &json::obj([]));
        assert!(!r0.is_error, "get_scene must succeed on default scene");
        let t0 = r0.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        assert!(
            t0.contains("undo_available=false"),
            "default scene must report undo unavailable: {t0}"
        );
        assert!(
            t0.contains("sampling_bounds="),
            "must report sampling bounds: {t0}"
        );

        // run_script でシーン更新 → script= に反映、undo_available=true。
        let run = json::obj([("script", json::s(r#"{"op":"sphere","r":1.5}"#))]);
        assert!(
            !call_tool(&mut s, "run_script", &run).is_error,
            "run_script must succeed"
        );
        let r1 = call_tool(&mut s, "get_scene", &json::obj([]));
        let t1 = r1.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        assert!(
            t1.contains("sphere"),
            "get_scene must echo current script: {t1}"
        );
        assert!(
            t1.contains("1.5"),
            "get_scene must include script params: {t1}"
        );
        assert!(
            t1.contains("undo_available=true"),
            "after run_script undo must be available: {t1}"
        );
    }

    #[test]
    fn screenshot_accepts_all_seven_documented_views_and_rejects_unknown() {
        // 問224: help (line 662) は front|back|right|left|top|bottom|iso を有効ビューとして
        // 宣言するが、各ビューが実際に screenshot で成功することを確認するテストがなかった。
        // Camera::presets の 7 名すべてが画像を生成し、未知ビューが明示エラーになることを固定。
        let mut s = Session::new();
        for view in ["front", "back", "right", "left", "top", "bottom", "iso"] {
            let args = json::obj([
                ("view", json::s(view)),
                ("width", json::n(32.0)),
                ("height", json::n(32.0)),
                ("resolution", json::n(16.0)),
            ]);
            let r = call_tool(&mut s, "screenshot", &args);
            assert!(
                !r.is_error,
                "screenshot(view={view}) must succeed for a documented view"
            );
        }
        // 未知ビューはサイレントフォールバックせず明示エラー (問71)。
        let bad = json::obj([("view", json::s("diagonal"))]);
        let rb = call_tool(&mut s, "screenshot", &bad);
        assert!(rb.is_error, "unknown view must produce an explicit error");
        let txt = rb.content[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            txt.contains("unknown view"),
            "error must name the problem: {txt}"
        );
    }

    #[test]
    fn screenshot_with_axes_includes_tick_spacing_note_and_image() {
        // 問288: axes=true (既定) のとき、応答は image に加えて目盛り間隔を記した
        // text を含み、AI が画像から寸法を概算できる。
        let mut s = Session::new();
        let args = json::obj([
            ("width", json::n(64.0)),
            ("height", json::n(64.0)),
            ("resolution", json::n(24.0)),
        ]);
        let r = call_tool(&mut s, "screenshot", &args);
        assert!(!r.is_error);
        let has_image = r.content.iter().any(|c| {
            c.get("type").and_then(|v| v.as_str()) == Some("image")
                && c.get("mimeType").and_then(|v| v.as_str()) == Some("image/png")
        });
        assert!(has_image, "response must include the PNG image");
        let note = r
            .content
            .iter()
            .find_map(|c| c.get("text").and_then(|v| v.as_str()))
            .unwrap_or("");
        assert!(
            note.contains("Tick marks") && note.contains("mm"),
            "axes response must note the mm tick spacing, got: {note}"
        );
    }

    #[test]
    fn screenshot_without_axes_is_image_only() {
        // 問288: axes=false のときは目盛りが無いので text 凡例も付けず、従来通り image のみ。
        let mut s = Session::new();
        let args = json::obj([
            ("width", json::n(64.0)),
            ("height", json::n(64.0)),
            ("resolution", json::n(24.0)),
            ("axes", json::b(false)),
        ]);
        let r = call_tool(&mut s, "screenshot", &args);
        assert!(!r.is_error);
        assert_eq!(r.content.len(), 1, "no-axes response must be image-only");
        assert_eq!(
            r.content[0].get("type").and_then(|v| v.as_str()),
            Some("image")
        );
    }

    #[test]
    fn screenshot_accepts_perspective_and_orthographic_projection_and_rejects_unknown() {
        // 問267: projection 引数で正射影/透視投影を切り替えられる。既定 (省略) は
        // 従来通り透視投影で成功し、"perspective"/"orthographic" も成功する。
        // 未知の値は screenshot の view と同じくサイレントフォールバックせず明示エラー。
        let mut s = Session::new();
        let base_args = |projection: Option<&str>| {
            let mut pairs = vec![
                ("view", json::s("front")),
                ("width", json::n(32.0)),
                ("height", json::n(32.0)),
                ("resolution", json::n(16.0)),
            ];
            if let Some(p) = projection {
                pairs.push(("projection", json::s(p)));
            }
            json::obj(pairs)
        };
        for projection in [None, Some("perspective"), Some("orthographic")] {
            let r = call_tool(&mut s, "screenshot", &base_args(projection));
            assert!(
                !r.is_error,
                "screenshot(projection={projection:?}) must succeed"
            );
        }
        let bad = call_tool(&mut s, "screenshot", &base_args(Some("fisheye")));
        assert!(
            bad.is_error,
            "unknown projection must produce an explicit error"
        );
        let txt = bad.content[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            txt.contains("unknown projection"),
            "error must name the problem: {txt}"
        );
    }

    #[test]
    fn sandbox_backslash_path_is_literal_filename_on_unix_not_traversal() {
        // 問189: Unix では '\\' はパス区切りではなくファイル名の一文字。
        // "a\\..\\escape.stl" は単一の Normal コンポーネントになり、ParentDir として
        // 解釈されないため脱出しない (プロジェクト直下に変な名前のファイルができるだけ)。
        // この「バックスラッシュは literal = 安全」という platform 契約を固定する。
        #[cfg(unix)]
        {
            // 単一の奇妙なファイル名として受理される (脱出なし)。
            let r = sandbox_write_path("a\\..\\escape.stl");
            assert!(
                r.is_ok(),
                "on Unix backslash is a literal filename char, not traversal"
            );
            // 正規のスラッシュ traversal は引き続き拒否される (回帰防止)。
            assert!(sandbox_write_path("a/../escape.stl").is_err());
        }
    }

    #[test]
    fn arg_build_dir_short_array_falls_back_to_plus_z_not_partial_fill() {
        // 問183: build_dir が 3 要素未満の配列のとき、欠けた成分を 0 補完して
        // [1,0] → [1,0,1] のような対角ビルドにせず、+Z デフォルトへフォールバックする。
        // (問85 の契約をテストで固定。AI が x-build を意図した [1,0] が
        //  誤って斜めビルド方向で解析される事故を防ぐ)
        let two = json::obj([("build_dir", json::arr([json::n(1.0), json::n(0.0)]))]);
        assert_eq!(
            arg_build_dir(&two),
            Vec3::new(0.0, 0.0, 1.0),
            "2-element build_dir must fall back to +Z (not partial-filled to [1,0,1])"
        );
        // 1 要素も同様。
        let one = json::obj([("build_dir", json::arr([json::n(1.0)]))]);
        assert_eq!(
            arg_build_dir(&one),
            Vec3::new(0.0, 0.0, 1.0),
            "1-element must fall back to +Z"
        );
        // 完全な 3 要素はそのまま使われる (回帰: 正常経路を壊さない)。
        let three = json::obj([(
            "build_dir",
            json::arr([json::n(1.0), json::n(0.0), json::n(0.0)]),
        )]);
        assert_eq!(
            arg_build_dir(&three),
            Vec3::new(1.0, 0.0, 0.0),
            "valid 3-element build_dir must be used verbatim"
        );
    }

    #[test]
    fn arg_build_dir_array_with_non_numeric_element_falls_back_to_plus_z() {
        // 問263: 3要素あっても1つが非数値 (文字列) なら、unwrap_or(0.0) で
        // その要素だけ0扱いにして [1,0,"up"] → [1,0,0] のような別方向を静かに
        // 作らず、問85/183 と同じ +Z デフォルトへ倒す。
        let mixed = json::obj([(
            "build_dir",
            json::arr([json::n(1.0), json::n(0.0), json::s("up")]),
        )]);
        assert_eq!(
            arg_build_dir(&mixed),
            Vec3::new(0.0, 0.0, 1.0),
            "build_dir with a non-numeric element must fall back to +Z, not zero-fill it"
        );
        // null/bool 要素も同様。
        let with_null = json::obj([(
            "build_dir",
            json::arr([json::n(1.0), json::NULL, json::n(0.0)]),
        )]);
        assert_eq!(
            arg_build_dir(&with_null),
            Vec3::new(0.0, 0.0, 1.0),
            "build_dir with a null element must fall back to +Z"
        );
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
    fn export_reports_digest_and_resolution_matching_validate() {
        // 問91: export 応答は出力の再現性同一性 (digest + resolution) を含み、
        // 同一解像度の validate と同じ digest になる (ツール間整合)。
        let mut session = Session::new();
        let fname = "kado-test-export-q91.stl";
        let args = json::obj([("path", json::s(fname)), ("resolution", json::n(24.0))]);
        let r = call_tool(&mut session, "export", &args);
        // テスト後の後始末 (成否に関わらず削除)。
        let _cleanup = std::fs::remove_file(fname);
        assert!(!r.is_error, "export must succeed");
        let text = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        assert!(
            text.contains("resolution=24"),
            "export response must report resolution (問91): {text}"
        );
        // export の digest を抽出。
        let digest_hex = text
            .split("digest=")
            .nth(1)
            .and_then(|s| s.split(')').next())
            .map(|s| s.trim())
            .expect("export response must contain digest");

        // 同一解像度の validate が同じ digest を報告する (ツール間整合)。
        let val_args = json::obj([("resolution", json::n(24.0))]);
        let vr = call_tool(&mut session, "validate", &val_args);
        let vtext = vr.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        let vjson = crate::mcp::json::parse(vtext).unwrap();
        let vdigest = vjson.get("digest").and_then(|x| x.as_str()).unwrap();
        assert_eq!(
            digest_hex, vdigest,
            "export and validate digests must match at the same resolution: \
             export={digest_hex} validate={vdigest}"
        );
    }

    #[test]
    fn export_surfaces_multiple_bodies_not_just_manifold() {
        // 問92: watertight な複数ボディは manifold=true のまま MULTIPLE_BODIES を
        // 隠す。export が構造 DFM issue code を併記し、AI が「manifold=true ⇒ DFM合格」
        // と誤認しないことを保証する (run_script 問81 と同じ閾値非依存チェック)。
        let mut session = Session::new();
        // 離れた2球 → 各殻は water­tight (manifold=true) だが 2 ボディ。
        let script = r#"union(translate(-2,0,0,sphere(0.6)),translate(2,0,0,sphere(0.6)))"#;
        let run_args = json::obj([("script", json::s(script))]);
        let rr = call_tool(&mut session, "run_script", &run_args);
        assert!(!rr.is_error, "two-sphere script must be valid");

        let fname = "kado-test-export-q92.stl";
        let args = json::obj([("path", json::s(fname)), ("resolution", json::n(32.0))]);
        let r = call_tool(&mut session, "export", &args);
        let _cleanup = std::fs::remove_file(fname);
        assert!(!r.is_error, "export must succeed");
        let text = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        // manifold は true のはず (各殻は閉じている)。
        assert!(
            text.contains("manifold=true"),
            "each shell is watertight → manifold=true: {text}"
        );
        // しかし MULTIPLE_BODIES が併記されていなければならない (問92 の核心)。
        assert!(
            text.contains("MULTIPLE_BODIES"),
            "export must surface MULTIPLE_BODIES even when manifold=true (問92): {text}"
        );
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
        let text = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
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

    #[test]
    fn validate_schema_documents_every_severity_the_validator_emits() {
        // 問299: validate のスキーマ説明は AI が読む契約そのもの。実際に emit されうる
        // severity 文字列が全て記載されていなければ、AI に嘘の列挙を渡すことになる。
        // 実際 "info" (ENCLOSED_CAVITY が使う) が漏れていた。Severity enum の全変種が
        // スキーマ説明に現れることを固定し、将来 severity を増やしたときの漏れを検知する。
        let tools = tool_list();
        let desc = tools
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("validate"))
            .and_then(|t| t.get("description"))
            .and_then(|d| d.as_str())
            .expect("validate tool must have a description");
        // Severity enum の JSON 表現 (check.rs の to_json と同じ文字列) を網羅する。
        for sev in ["error", "warning", "info"] {
            assert!(
                desc.contains(&format!("\"{sev}\"")),
                "validate schema must document severity \"{sev}\" (問299)"
            );
        }
        // 実行時に注入されるフィールドも文書化されていること。
        assert!(
            desc.contains("resolution"),
            "validate schema must document the runtime-injected `resolution` field (問299)"
        );
    }

    #[test]
    fn issue_codes_are_fully_documented() {
        // 問103: validator が emit しうる全 issue code (ALL_ISSUE_CODES, 単一の真実源) は、
        // MCP の help と validate スキーマ説明の双方に記載されていなければならない。
        // NEGATIVE_VOLUME がスキーマの "All issue codes:" から漏れていた退行を防ぎ、
        // 将来コード追加時の文書ドリフトを検知する (問79 の完全版)。
        let help = KADOSCENE_HELP;
        // validate スキーマ説明を取得。
        let tools = tool_list();
        let validate_desc = tools
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("validate"))
            .and_then(|t| t.get("description"))
            .and_then(|d| d.as_str())
            .expect("validate tool must have a description");

        for code in crate::verify::ALL_ISSUE_CODES {
            assert!(
                help.contains(code),
                "issue code '{code}' must be documented in KADOSCENE_HELP (問103)"
            );
            assert!(
                validate_desc.contains(code),
                "issue code '{code}' must be listed in the validate tool schema (問103)"
            );
        }
    }

    #[test]
    fn dsl_ops_are_fully_documented() {
        // 問271: eval.rs の build() が実際に受理する全 op (ALL_DSL_OPS, 単一の
        // 真実源) は KADOSCENE_HELP (AI が読む唯一のリファレンス) に記載されて
        // いなければならない。問103 (ALL_ISSUE_CODES) と同じ文書ドリフト防止
        // パターンを演算子名にも適用する。eval.rs 側の対になるテスト
        // (all_dsl_ops_are_documented_in_module_comment) はモジュール冒頭コメントを
        // 検証する。
        let help = KADOSCENE_HELP;
        for op in crate::script::eval::ALL_DSL_OPS {
            assert!(
                help.contains(op),
                "DSL op '{op}' must be documented in KADOSCENE_HELP (問271)"
            );
        }
    }

    #[test]
    fn help_worked_examples_are_valid_scripts() {
        // 問298: KADOSCENE_HELP の「動く例」は AI が最初に読み、模倣する。例が構文的に
        // 壊れる (op 名変更・引数順変更で無効化する) と AI が誤った雛形を学ぶ。
        // help に載せた実例が実際に eval できることを固定し、例の腐敗を防ぐ。
        // JSON / テキスト DSL 両形式・chamfer/smooth を横断する代表例を検証する。
        // 1行のテキスト DSL 例: help 本文に**逐語**で載っており、かつ eval できること。
        let dsl_examples = [
            "chamfer_union(cuboid(1.0, 1.0, 0.3), cylinder(0.4, 1.2), 0.2)",
            "smooth_union(cuboid(1.0, 1.0, 0.3), cylinder(0.4, 1.2), 0.2)",
            "difference(sphere(1.5), cylinder(0.4, 2.0))",
        ];
        for src in dsl_examples {
            assert!(
                eval_any(src).is_ok(),
                "documented DSL example must eval cleanly: {src}"
            );
            assert!(
                KADOSCENE_HELP.contains(src),
                "DSL example must appear verbatim in KADOSCENE_HELP: {src}"
            );
        }
        // 複数行 JSON 例 (sphere with a hole): eval できること (整形は help 側の体裁)。
        assert!(
            eval_any(
                r#"{"op":"difference","a":{"op":"sphere","r":1.5},"b":{"op":"cylinder","r":0.4,"h":2.0}}"#
            )
            .is_ok(),
            "documented JSON hole example must eval cleanly"
        );
    }

    #[test]
    fn readme_operator_list_is_fully_documented() {
        // 問278: README.md の「利用可能な演算子」一覧も KADOSCENE_HELP・eval.rs
        // 冒頭コメントと同じ ALL_DSL_OPS に対して検証する。実際に prism (問269)・
        // rotate 任意軸 (問266)・scale_xyz (問276) が複数ラウンド分このリストから
        // 漏れていた退行 (問278) を検知する。include_str! でリポジトリルートの
        // README.md をコンパイル時に読み込む (問271 の自己参照パターンと同型、
        // 対象がクレート外のファイルである点のみ異なる)。
        let readme = include_str!("../../README.md");
        let heading = "利用可能な演算子";
        let heading_pos = readme
            .find(heading)
            .expect("README must have an operator list section");
        // 見出し直後には空行 (heading→list の区切り) があるため、まずそれを
        // 飛び越してから箇条書きリストの終端 (次の空行) を探す。
        let list_start = readme[heading_pos..]
            .find("\n\n")
            .map(|i| heading_pos + i + 2)
            .expect("README operator section must have a blank line after the heading");
        let list_end = readme[list_start..]
            .find("\n\n")
            .map(|i| list_start + i)
            .unwrap_or(readme.len());
        let section = &readme[list_start..list_end];
        // README は mirror_x/y/z・rotate_x/y/z のような同系列 op を "op_x/y/z" と
        // 圧縮表記する (ドリフトではなく正当な慣例)。個別名か圧縮形のどちらかが
        // あれば「記載済み」とみなす。
        for op in crate::script::eval::ALL_DSL_OPS {
            let compact = op
                .strip_suffix("_x")
                .or_else(|| op.strip_suffix("_y"))
                .or_else(|| op.strip_suffix("_z"))
                .map(|prefix| format!("{prefix}_x/y/z"));
            let documented =
                section.contains(op) || compact.as_deref().is_some_and(|c| section.contains(c));
            assert!(
                documented,
                "DSL op '{op}' must be listed in README.md's operator section \
                 (as itself or as a compact op_x/y/z group, 問278)"
            );
        }
    }

    #[test]
    fn every_advertised_tool_is_dispatchable() {
        // 問102: tools/list が広告する全ツールは call_tool で必ずディスパッチされなければ
        // ならない。広告されているのに未実装だと AI は「unknown tool」という混乱する
        // エラーを受け取る (リストとディスパッチの構造的整合性)。ツール追加時に
        // 配線忘れを検知する回帰ガード。
        let names: Vec<String> = tool_list()
            .as_array()
            .expect("tool_list is an array")
            .iter()
            .map(|t| t.get("name").and_then(|n| n.as_str()).unwrap().to_string())
            .collect();
        assert!(!names.is_empty(), "tool_list must advertise tools");

        for name in &names {
            let mut session = Session::new();
            // export は副作用 (ファイル書込) があるため一意な一時パスを与え後始末する。
            let args = if name == "export" {
                json::obj([("path", json::s("kado-test-q102-dispatch.stl"))])
            } else {
                // 他ツールは引数省略でも「unknown tool」以外を返すはず
                // (eval は arg エラー、screenshot/validate/get_scene 等は既定で動作)。
                json::obj([])
            };
            let r = call_tool(&mut session, name, &args);
            if name == "export" {
                let _ = std::fs::remove_file("kado-test-q102-dispatch.stl");
            }
            // 結果がエラーでも良いが、「unknown tool」だけは出てはならない
            // (= リストにあるのにディスパッチされていない)。
            let text = r
                .content
                .first()
                .and_then(|c| c.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert!(
                !text.contains("unknown tool"),
                "advertised tool '{name}' must be dispatched by call_tool, got: {text}"
            );
        }
    }

    #[test]
    fn run_script_discloses_check_resolution() {
        // 問93: run_script の summary は digest を含むが res=32 のチェック。
        // 解像度を開示しないと AI が digest の不一致を説明できず、粗い「issue なし」を
        // 確定的と誤認する。既定 (32) と明示指定の両方で check_resolution が出ることを確認。
        let mut s = Session::new();
        // 既定解像度 (32)。
        let args = json::obj([("script", json::s("sphere(1.0)"))]);
        let r = call_tool(&mut s, "run_script", &args);
        assert!(!r.is_error);
        let text = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        assert!(
            text.contains("check_resolution=32"),
            "run_script must disclose its (default 32) check resolution (問93): {text}"
        );
        // 明示指定 (16) も反映される。
        let args2 = json::obj([
            ("script", json::s("sphere(1.0)")),
            ("resolution", json::n(16.0)),
        ]);
        let r2 = call_tool(&mut s, "run_script", &args2);
        let text2 = r2.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        assert!(
            text2.contains("check_resolution=16"),
            "run_script must disclose the explicit check resolution (問93): {text2}"
        );
    }

    #[test]
    fn help_documents_evaluator_constraints() {
        // 問89: help は評価器が課す制約を正確に記載しなければならない。
        // 問77 (torus minor < major) と問75 (smooth k > 0) は評価器で拒否されるが、
        // help に未記載だと AI が予測できないエラーに遭遇する。help/評価器の同期を保証する。
        let help = KADOSCENE_HELP;
        // 問77: torus minor < major 制約。
        assert!(
            help.contains("< major"),
            "help must document torus minor < major constraint (問77)"
        );
        // 問75: smooth k > 0 制約。
        assert!(
            help.contains("k: blend radius > 0"),
            "help must document smooth k > 0 constraint (問75)"
        );

        // 問95: 著作リファレンス (help) は座標の単位 (mm) を述べなければならない。
        // AI が寸法を指定する基礎情報であり、欠落すると scale が不定になる。
        assert!(
            help.contains("MILLIMETERS") && help.contains("1 mm"),
            "help must state the mm unit convention for authoring (問95)"
        );

        // 問100/276: scale が一様限定であることを help が明示しなければならない
        // (AI が per-axis scale を期待して誤用しないため)。問276 で scale_xyz
        // (非一様) が追加されたため、文言は "UNIFORM only" という単独の警告から
        // 「scale=一様/scale_xyz=非一様」という対比の説明へ変わったが、
        // 「scale が一様であること」自体を明示する要件 (問100 の本質) は変わらない。
        assert!(
            help.contains("uniform"),
            "help must state scale is uniform (問100/276)"
        );
        // 問276: scale_xyz の安全性保証 (符号厳密・保守的過小評価・Lipschitz=1) を
        // help が明示しなければならない (AI が非一様スケールの信頼性を判断する材料)。
        assert!(
            help.contains("scale_xyz") && help.contains("UNDERESTIMATE"),
            "help must document scale_xyz's conservative-underestimate guarantee (問276)"
        );

        // 評価器が実際にこれらを拒否することを確認 (help の主張が現実と一致)。
        assert!(
            eval_any(r#"{"op":"torus","major":1.0,"minor":1.0}"#).is_err(),
            "evaluator must reject torus minor >= major as help claims"
        );
        assert!(
            eval_any(r#"{"op":"smooth_union","a":{"op":"sphere","r":1.0},"b":{"op":"sphere","r":1.0},"k":0.0}"#).is_err(),
            "evaluator must reject smooth k<=0 as help claims"
        );
    }

    #[test]
    fn help_documents_material_and_unit_estimation() {
        // 問253: report.volume の単位 (mm³) は内部コメントにしかなく AI 出力に無かった。
        // help が単位と材料/質量/コスト見積もりの式を述べ、AI が volume から
        // フィラメント/レジン量を概算できることを保証する。
        let help = KADOSCENE_HELP;
        // 単位の明示 (mm³) と質量式。
        assert!(
            help.contains("mm³") && help.contains("density"),
            "help must state volume unit (mm³) and the mass formula"
        );
        // 代表的な密度 (PLA) が含まれる。
        assert!(
            help.contains("PLA"),
            "help must list common filament densities (e.g. PLA)"
        );
        // solid volume は上限であるという正直な注意 (infill)。
        assert!(
            help.to_lowercase().contains("infill"),
            "help must note solid volume is an upper bound (infill caveat)"
        );
    }

    #[test]
    fn validate_reports_resolution_alongside_digest() {
        // 問90: digest の決定性契約 (問61) は同一解像度が前提。report に resolution が
        // 無いと digest を後で再現できない。validate 応答が resolution を含むことを保証する。
        use crate::mcp::json::parse;
        let mut s = Session::new();
        let args = json::obj([("resolution", json::n(40.0))]);
        let r = call_tool(&mut s, "validate", &args);
        assert!(!r.is_error, "validate must succeed");
        let text = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
        let v = parse(text).expect("validate output must be valid JSON");
        // resolution と digest が両方含まれること。
        assert_eq!(
            v.get("resolution").and_then(|x| x.as_f64()),
            Some(40.0),
            "validate JSON must report the resolution used (問90): {text}"
        );
        assert!(
            v.get("digest").and_then(|x| x.as_str()).is_some(),
            "validate JSON must still report digest: {text}"
        );
    }

    #[test]
    fn base64_matches_rfc4648_vectors_including_padding() {
        // 問107: screenshot は PNG を base64 で返すため、パディング (= / ==) を含む
        // エンコードが正確でなければ AI/クライアントが画像をデコードできない。
        // RFC 4648 §10 の正準テストベクタで全パディングケース (0/1/2 余りバイト) を固定する。
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // バイナリ安全性: 上位ビット (>0x7F) でも符号拡張等で壊れないこと。
        assert_eq!(base64_encode(&[0xFF]), "/w==");
        assert_eq!(base64_encode(&[0xFF, 0xFF, 0xFF]), "////");
        assert_eq!(base64_encode(&[0x00]), "AA==");
        // 出力長は常に 4 の倍数 (パディングで埋まる)。
        for n in 0..20usize {
            let data = vec![0xABu8; n];
            assert_eq!(
                base64_encode(&data).len() % 4,
                0,
                "base64 length must be a multiple of 4 (n={n})"
            );
        }
    }

    #[test]
    fn base64_encode_long_input_uses_valid_alphabet_and_is_deterministic() {
        // 問200: base64_matches_rfc4648 は最長 8 バイトのみ。1000 バイトのような
        // 長い入力が複数の chunk(3) を跨いでも有効な Base64 文字のみ出力されることと
        // 決定的であることを確認する (ADR-003 決定性の保証)。
        let data: Vec<u8> = (0u8..=255).cycle().take(1000).collect();
        let b64 = base64_encode(&data);
        // 長さは 4 の倍数。
        assert_eq!(
            b64.len() % 4,
            0,
            "length must be multiple of 4 for 1000-byte input"
        );
        // 期待長: ceil(1000/3)*4 = 334*4 = 1336。
        assert_eq!(b64.len(), 1336, "expected length 1336 for 1000 bytes");
        // すべての文字が Base64 アルファベット内 (A-Z, a-z, 0-9, +, /, =)。
        for &ch in b64.as_bytes() {
            assert!(
                ch.is_ascii_alphabetic()
                    || ch.is_ascii_digit()
                    || ch == b'+'
                    || ch == b'/'
                    || ch == b'=',
                "invalid Base64 character: {}",
                ch as char
            );
        }
        // 末尾のパディング文字 '=' は出力末尾のみに現れる (中間に = はない)。
        let trimmed = b64.trim_end_matches('=');
        assert!(
            !trimmed.contains('='),
            "padding '=' must only appear at end"
        );
        // 決定性: 同一入力で同一出力。
        assert_eq!(base64_encode(&data), b64, "base64 must be deterministic");
    }

    #[test]
    fn default_scene_is_structurally_sound_for_first_impression() {
        // 問111: AI が接続直後 (run_script 前) に validate を呼ぶとデフォルトシーンを
        // 検証する。「デフォルトは健全なデモ」という前提を固定し、将来 default_scene を
        // 変更しても構造的に壊れたデモを出荷しないことを保証する。
        // 閾値依存の OVERHANG/THIN_WALL (閉形状なら下面は常に overhang) は対象外とし、
        // 構造的健全性 (manifold/単一ボディ/正体積/開境界なし) のみを固定する。
        use crate::mcp::json::parse;
        let mut s = Session::new();
        // 構造チェックのみ: min_wall=0, max_overhang=0 で閾値系をスキップ。
        let args = json::obj([
            ("min_wall_mm", json::n(0.0)),
            ("max_overhang_deg", json::n(0.0)),
            ("resolution", json::n(48.0)),
        ]);
        let r = call_tool(&mut s, "validate", &args);
        assert!(!r.is_error, "validate must run on the default scene");
        let v = parse(r.content[0].get("text").and_then(|t| t.as_str()).unwrap()).unwrap();

        assert_eq!(
            v.get("manifold").and_then(|x| x.as_bool()),
            Some(true),
            "default scene must be watertight (manifold)"
        );
        assert!(
            v.get("volume").and_then(|x| x.as_f64()).unwrap_or(-1.0) > 0.0,
            "default scene must have positive volume (correct orientation)"
        );
        // 構造エラーが無いこと: OPEN_MESH/NON_MANIFOLD/EMPTY_MESH/NEGATIVE_VOLUME/MULTIPLE_BODIES。
        let codes: Vec<&str> = v
            .get("issues")
            .and_then(|i| i.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|e| e.get("code").and_then(|c| c.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        for bad in [
            "OPEN_MESH",
            "NON_MANIFOLD",
            "EMPTY_MESH",
            "NEGATIVE_VOLUME",
            "MULTIPLE_BODIES",
        ] {
            assert!(
                !codes.contains(&bad),
                "default scene must not have structural issue {bad}; got {codes:?}"
            );
        }
    }

    #[test]
    fn run_script_to_validate_digest_is_deterministic_across_sessions() {
        // 問105: 再現性契約 (問5/問61/問90) を MCP ツール経路で end-to-end に固定する。
        // polygonize_is_byte_deterministic は抽出単体を見るが、ここでは
        // run_script → validate という実際の AI 利用経路を、独立した2セッションで通し、
        // 同一スクリプト・同一解像度なら同一 digest になることを保証する。
        use crate::mcp::json::parse;
        let script = "difference(smooth_union(sphere(1.0), cuboid(0.7), 0.2), cylinder(0.3, 2.0))";
        let digest_via_fresh_session = || -> String {
            let mut s = Session::new();
            let run = json::obj([("script", json::s(script))]);
            assert!(
                !call_tool(&mut s, "run_script", &run).is_error,
                "script must be valid"
            );
            let val = json::obj([("resolution", json::n(36.0))]);
            let r = call_tool(&mut s, "validate", &val);
            let text = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
            parse(text)
                .unwrap()
                .get("digest")
                .and_then(|x| x.as_str())
                .unwrap()
                .to_string()
        };
        let d1 = digest_via_fresh_session();
        let d2 = digest_via_fresh_session();
        assert_eq!(
            d1, d2,
            "same script at same resolution must yield identical digest across \
             independent sessions (MCP-path reproducibility, 問105): {d1} vs {d2}"
        );

        // 同一セッションで validate を2回呼んでも digest は不変 (validate は非破壊)。
        let mut s = Session::new();
        let run = json::obj([("script", json::s(script))]);
        call_tool(&mut s, "run_script", &run);
        let val = json::obj([("resolution", json::n(36.0))]);
        let read = |s: &mut Session| -> String {
            let r = call_tool(s, "validate", &val);
            let t = r.content[0].get("text").and_then(|v| v.as_str()).unwrap();
            parse(t)
                .unwrap()
                .get("digest")
                .and_then(|x| x.as_str())
                .unwrap()
                .to_string()
        };
        assert_eq!(
            read(&mut s),
            read(&mut s),
            "validate must be non-mutating/repeatable"
        );
    }

    #[test]
    fn eval_schema_discloses_units_and_lower_bound() {
        // 問94: eval の戻り値は mm 単位の符号付き距離だが、合成/平滑形状では
        // 真の距離の保守的下界 (Lipschitz 場) にすぎない。AI がクリアランス計測で
        // 過信しないよう、スキーマ説明が単位と下界性を開示することを保証する。
        let tools = tool_list();
        let arr = tools.as_array().expect("tool_list returns an array");
        let eval_desc = arr
            .iter()
            .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("eval"))
            .and_then(|t| t.get("description"))
            .and_then(|d| d.as_str())
            .expect("eval tool must exist with a description");
        assert!(
            eval_desc.contains("mm"),
            "eval schema must state units are mm (問94): {eval_desc}"
        );
        assert!(
            eval_desc.contains("lower bound"),
            "eval schema must disclose magnitude is a conservative lower bound (問94): {eval_desc}"
        );

        // 例外なくプリミティブでは厳密 (下界主張のアンカー)。球面上の点で距離 ≈ 0。
        let mut s = Session::new();
        let run = json::obj([("script", json::s("sphere(1.0)"))]);
        assert!(!call_tool(&mut s, "run_script", &run).is_error);
        let q = json::obj([
            ("x", json::n(2.0)),
            ("y", json::n(0.0)),
            ("z", json::n(0.0)),
        ]);
        let r = call_tool(&mut s, "eval", &q);
        let d: f64 = r.content[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        // (2,0,0) は半径1球の外、表面まで厳密に 1.0 mm。
        assert!(
            (d - 1.0).abs() < 1e-9,
            "sphere eval must be exact for a primitive: expected 1.0, got {d}"
        );
    }

    #[test]
    fn undo_script_restores_scene_then_exhausts_single_level() {
        // 問137: undo_script は1段戻りのみ対応し、2回目の呼び出しはエラーを返す。
        // undo が全く呼び出されていない状態 (初期) もエラー。
        // MCP ツールとして実装されているが、専用ユニットテストが存在しなかった。
        let mut s = Session::new();

        // (1) run_script 前の undo → エラー。
        let r = call_tool(&mut s, "undo_script", &json::obj([]));
        assert!(r.is_error, "undo before any run_script must return error");

        // (2) run_script で sphere に変更。
        let sphere_script = json::obj([("script", json::s(r#"{"op":"sphere","r":2.0}"#))]);
        let r = call_tool(&mut s, "run_script", &sphere_script);
        assert!(!r.is_error, "run_script must succeed");
        // シーンが sphere(r=2) に変わっている: 原点の SDF 値 ≈ -2。
        let d_after = s.scene.eval(Vec3::ZERO);
        assert!((d_after - (-2.0)).abs() < 1e-9, "scene must be sphere(r=2)");

        // (3) undo → デフォルトシーンに戻る。
        let r = call_tool(&mut s, "undo_script", &json::obj([]));
        assert!(!r.is_error, "first undo must succeed");
        // デフォルトシーンのf(0)は sphere-cuboid smooth_union で負 (内部)。
        let d_restored = s.scene.eval(Vec3::ZERO);
        assert!(
            d_restored < 0.0,
            "undo must restore a scene with negative SDF at origin"
        );
        // undo 応答に "undo ok" が含まれる。
        let text = r.content[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            text.contains("undo ok"),
            "undo response must say 'undo ok': {text}"
        );

        // (4) 2回目の undo → エラー (single-level)。
        let r = call_tool(&mut s, "undo_script", &json::obj([]));
        assert!(
            r.is_error,
            "second undo must return error (single-level undo exhausted)"
        );
        let err = r.content[0]
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            err.contains("nothing to undo"),
            "double undo error must say 'nothing to undo': {err}"
        );
    }

    #[test]
    fn undo_script_after_failed_run_does_not_corrupt_undo_state() {
        // 問137b: 失敗した run_script は undo 状態を変えない。
        // 実装: eval_any が Err なら early return するため prev_scene は更新されない。
        // よって undo_script は失敗した run_script の「前」ではなく、
        // その前の成功した run_script の前の状態に戻る。
        let mut s = Session::new();

        // (1) デフォルトシーンの SDF を記録 (sphere-cuboid smooth_union)。
        let d_default = s.scene.eval(Vec3::ZERO);

        // (2) sphere(r=1.5) に変更。prev_scene = default_scene。
        let sphere_script = json::obj([("script", json::s(r#"{"op":"sphere","r":1.5}"#))]);
        call_tool(&mut s, "run_script", &sphere_script);

        // (3) 失敗する run_script。early return により prev_scene = default_scene のまま。
        let bad = json::obj([("script", json::s("nonexistent_fn()"))]);
        let r = call_tool(&mut s, "run_script", &bad);
        assert!(
            r.is_error,
            "bad script must return is_error=true, got text: {}",
            r.content[0]
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("(no text)")
        );
        // scene は sphere のまま変わっていない。
        let d_sphere = s.scene.eval(Vec3::ZERO);
        assert!(
            (d_sphere - (-1.5)).abs() < 1e-9,
            "failed script must not change scene"
        );

        // (4) undo → default_scene に戻る (prev_scene は sphere 前のものが残っている)。
        let r = call_tool(&mut s, "undo_script", &json::obj([]));
        assert!(!r.is_error, "undo after failed run_script must succeed");
        let d_after_undo = s.scene.eval(Vec3::ZERO);
        assert!(
            (d_after_undo - d_default).abs() < 1e-9,
            "undo must restore to pre-sphere (default) state: expected {d_default} got {d_after_undo}"
        );
    }

    #[test]
    fn arg_samples_invariant_dim_times_samples_fits_max_image_dim() {
        // 問131: `dim * samples ≤ MAX_IMAGE_DIM` の不変条件が成り立つことをテスト。
        // arg_dim は [1, MAX_IMAGE_DIM] にクランプされるが、arg_samples のキャップ戦略に
        // 専用テストがなかった。img 生成時は width*samples × height*samples のバッファを
        // 確保するため、両軸とも MAX_IMAGE_DIM を超えてはならない。
        let no_req = json::obj([]);
        let max_req = json::obj([("samples", json::n(4.0))]);

        // デフォルト: samples=2
        assert_eq!(
            arg_samples(&no_req, 512, 512),
            2,
            "default samples must be 2"
        );

        // 代表的な大寸法での不変条件確認。
        for (w, h) in [
            (1, 1),
            (1024, 768),
            (2048, 1),
            (1, 2048),
            (MAX_IMAGE_DIM, 1),
            (1, MAX_IMAGE_DIM),
            (MAX_IMAGE_DIM, MAX_IMAGE_DIM),
        ] {
            let s = arg_samples(&max_req, w, h);
            assert!(
                w * s <= MAX_IMAGE_DIM,
                "width * samples must not exceed MAX_IMAGE_DIM: {w}*{s}={}>{}",
                w * s,
                MAX_IMAGE_DIM
            );
            assert!(
                h * s <= MAX_IMAGE_DIM,
                "height * samples must not exceed MAX_IMAGE_DIM: {h}*{s}={}>{}",
                h * s,
                MAX_IMAGE_DIM
            );
        }

        // MAX_IMAGE_DIM×MAX_IMAGE_DIM では samples=4 が要求されても 1 にキャップされる。
        assert_eq!(
            arg_samples(&max_req, MAX_IMAGE_DIM, MAX_IMAGE_DIM),
            1,
            "max-size canvas must cap samples to 1"
        );
    }

    #[test]
    fn arg_samples_actually_reduces_when_dimension_limits_it() {
        // 問160: arg_samples_invariant は「積が MAX_IMAGE_DIM 以内」の不変条件を確認するが、
        // 「クランプが実際に減少を生じさせた」ことは未確認。
        // 不変条件テストは外側の arg_dim ガードにも依存するため、
        // arg_samples の min(cap_w) 節が削除されても不変条件テストは通過しうる。
        // ここでは戻り値の具体値を固定してクランプ削除を即検出する。
        let req4 = json::obj([("samples", json::n(4.0))]);

        // width=2048, height=2048, requested=4 →
        // cap_w = 4096/2048 = 2, cap_h = 4096/2048 = 2 → result = min(4,2,2) = 2。
        assert_eq!(
            arg_samples(&req4, 2048, 2048),
            2,
            "samples must reduce from 4 to 2 when 2048×2048 canvas"
        );
        // 非対称: width=512, height=4096 →
        // cap_w = 4096/512 = 8, cap_h = 4096/4096 = 1 → result = min(4,8,1) = 1。
        assert_eq!(
            arg_samples(&req4, 512, MAX_IMAGE_DIM),
            1,
            "height=MAX_IMAGE_DIM must force samples to 1"
        );
        // 十分小さい寸法: 1024×1024 → cap=4 → クランプ不発で 4 のまま。
        assert_eq!(
            arg_samples(&req4, 1024, 1024),
            4,
            "samples must remain 4 when 1024×1024 allows it"
        );
    }
}
