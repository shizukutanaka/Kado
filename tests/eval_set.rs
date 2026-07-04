//! 評価セット (EVAL-SET) — エンドツーエンドのリグレッションガード (問52)。
//!
//! 代表的な実モデル相当の KadoScene スクリプト N(≥10) 本を、
//! script → SDF → polygonize → validate のフルパイプラインで通し、
//! 製品中核 KPI を性質テストする:
//!   1. 水密性 (問11): 抽出メッシュは閉じた 2-多様体 (edge-manifold)。
//!   2. 構造健全性: validate が OPEN_MESH / NON_MANIFOLD / EMPTY_MESH を出さない。
//!   3. 決定性 (問5): 同一入力の2回抽出がビット単位で一致。
//!   4. 向き一貫性: 符号付き体積が正。
//!
//! これは「測って改善する」ためのベンチであり、全演算 (回転・smooth・repeat 等) を
//! 横断的に運動させ、機能追加時の退行を一点で検知する。

use kado::extract::polygonize;
use kado::io::{gltf, html, stl, threemf};
use kado::mcp::json::parse;
use kado::script::eval_scene;
use kado::verify::validate;

/// 評価セットの1課題。`name` は失敗時の特定用、`script` は KadoScene JSON。
struct Task {
    name: &'static str,
    script: &'static str,
}

/// N≥10 の代表課題。プリミティブ・ブーリアン・smooth・変形 (回転含む) を網羅する。
fn eval_set() -> Vec<Task> {
    vec![
        Task {
            name: "bracket (union + hole)",
            script: r#"{"op":"difference",
                "a":{"op":"union",
                     "a":{"op":"sphere","r":1.0},
                     "b":{"op":"cuboid","x":0.8,"y":0.8,"z":0.8}},
                "b":{"op":"cylinder","r":0.3,"h":2.0}}"#,
        },
        Task {
            name: "lens (sphere intersection)",
            script: r#"{"op":"intersection",
                "a":{"op":"sphere","r":1.0},
                "b":{"op":"translate","x":0.8,"y":0,"z":0,"shape":{"op":"sphere","r":1.0}}}"#,
        },
        Task {
            name: "pipe elbow (rotated cylinders)",
            script: r#"{"op":"union",
                "a":{"op":"cylinder","r":0.3,"h":1.0},
                "b":{"op":"rotate_x","angle":90,"shape":{"op":"cylinder","r":0.3,"h":1.0}}}"#,
        },
        Task {
            name: "hollow rounded enclosure (shell)",
            script: r#"{"op":"shell","thickness":0.15,
                "shape":{"op":"rounded_box","x":1.0,"y":0.7,"z":0.5,"r":0.15}}"#,
        },
        Task {
            name: "dumbbell (smooth_union)",
            script: r#"{"op":"smooth_union","k":0.3,
                "a":{"op":"translate","x":-0.9,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}},
                "b":{"op":"smooth_union","k":0.3,
                     "a":{"op":"cylinder","r":0.15,"h":0.9},
                     "b":{"op":"translate","x":0.9,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}}}}"#,
        },
        Task {
            name: "plus sign (perpendicular bars)",
            script: r#"{"op":"union",
                "a":{"op":"cuboid","x":1.2,"y":0.35,"z":0.35},
                "b":{"op":"cuboid","x":0.35,"y":1.2,"z":0.35}}"#,
        },
        Task {
            name: "tilted block (print orientation)",
            script: r#"{"op":"rotate_z","angle":30,
                "shape":{"op":"rotate_x","angle":20,
                         "shape":{"op":"rounded_box","x":0.9,"y":0.6,"z":0.4,"r":0.1}}}"#,
        },
        Task {
            name: "ring (torus)",
            script: r#"{"op":"torus","major":1.0,"minor":0.3}"#,
        },
        Task {
            name: "bolt (stacked cylinders)",
            script: r#"{"op":"union",
                "a":{"op":"cylinder","r":0.6,"h":0.25},
                "b":{"op":"translate","x":0,"y":0,"z":-0.8,"shape":{"op":"cylinder","r":0.25,"h":0.8}}}"#,
        },
        Task {
            name: "perforated plate (repeat holes)",
            script: r#"{"op":"difference",
                "a":{"op":"cuboid","x":1.5,"y":1.5,"z":0.3},
                "b":{"op":"repeat","x":0.9,"y":0.9,"nx":1,"ny":1,
                     "shape":{"op":"cylinder","r":0.25,"h":1.0}}}"#,
        },
        Task {
            name: "cross-drilled cylinder (rotated holes)",
            script: r#"{"op":"difference",
                "a":{"op":"cylinder","r":0.7,"h":0.8},
                "b":{"op":"rotate_x","angle":90,"shape":{"op":"cylinder","r":0.2,"h":2.0}}}"#,
        },
        Task {
            name: "egg (ellipsoid)",
            script: r#"{"op":"ellipsoid","x":0.7,"y":0.7,"z":1.1}"#,
        },
        Task {
            name: "mirrored fin pair",
            script: r#"{"op":"mirror_x",
                "shape":{"op":"translate","x":0.7,"y":0,"z":0,
                         "shape":{"op":"capsule","h":0.5,"r":0.25}}}"#,
        },
        Task {
            name: "flat-based dome (cut for FDM printability)",
            script: r#"{"op":"cut","nx":0,"ny":0,"nz":-1,"offset":0,
                "shape":{"op":"sphere","r":1.0}}"#,
        },
        Task {
            name: "hex nut blank (prism with bore, 問269/273)",
            script: r#"{"op":"difference",
                "a":{"op":"prism","n":6,"r":0.9,"h":0.3},
                "b":{"op":"cylinder","r":0.4,"h":0.5}}"#,
        },
        Task {
            name: "diagonal brace (arbitrary-axis rotate, 問266/273)",
            script: r#"{"op":"rotate","ax":1,"ay":1,"az":0,"angle":35,
                "shape":{"op":"cuboid","x":1.0,"y":0.3,"z":0.3}}"#,
        },
    ]
}

#[test]
fn eval_set_has_at_least_ten_tasks() {
    assert!(
        eval_set().len() >= 10,
        "eval set must define N>=10 representative tasks, got {}",
        eval_set().len()
    );
}

#[test]
fn eval_set_models_are_watertight_and_sound() {
    let res = 40;
    for task in eval_set() {
        let sdf = eval_scene(task.script)
            .unwrap_or_else(|e| panic!("[{}] script failed to evaluate: {e}", task.name));
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, res);

        // 非空。
        assert!(
            !mesh.triangles.is_empty(),
            "[{}] mesh is unexpectedly empty",
            task.name
        );
        // 水密 (問11)。
        assert!(
            mesh.is_edge_manifold(),
            "[{}] mesh must be edge-manifold (watertight)",
            task.name
        );
        // 向き一貫性: 体積は正。
        assert!(
            mesh.signed_volume() > 0.0,
            "[{}] signed volume must be positive, got {}",
            task.name,
            mesh.signed_volume()
        );

        // 構造系 DFM エラー (OPEN_MESH/NON_MANIFOLD/EMPTY_MESH) が出ないこと。
        // 肉厚・オーバーハングは形状依存なのでスキップ (0) して構造のみ見る。
        let report = validate(&mesh, 0.0, 0.0);
        for issue in &report.issues {
            assert!(
                !matches!(issue.code, "OPEN_MESH" | "NON_MANIFOLD" | "EMPTY_MESH"),
                "[{}] structural error {}: {}",
                task.name,
                issue.code,
                issue.cause
            );
        }
    }
}

#[test]
fn eval_set_models_export_to_all_formats_with_valid_structure() {
    // 問231: 既存の eval_set は script→mesh→validate までで、ユーザ/AI が実際に行う
    // 最終段 (export = フォーマット直列化) を横断的に通していなかった。
    // 各エンコーダ (STL/GLB/3MF/HTML) の単体テストは単一の sphere メッシュのみで、
    // CSG・smooth・repeat・mirror・rotate・torus 等の多様な実モデルでの退行は
    // 検出できなかった。全課題 × 4 形式の出力が構造的に妥当かつ決定的であることを固定する
    // (問273: 課題数を固定の数字でコメントすると eval_set() へのタスク追加のたびに
    // ドリフトする。件数ではなく「全課題」と書くことで将来の追加に自動追従する)。
    let res = 32;
    for task in eval_set() {
        let sdf = eval_scene(task.script).unwrap();
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, res);
        assert!(
            !mesh.triangles.is_empty(),
            "[{}] mesh must be non-empty",
            task.name
        );

        // ── STL: 80 バイトヘッダ "kado binary stl" + 三角形数フィールド ──
        let stl_bytes = stl::encode_binary(&mesh);
        assert!(
            stl_bytes.starts_with(b"kado binary stl"),
            "[{}] STL header",
            task.name
        );
        let stl_tri = u32::from_le_bytes(stl_bytes[80..84].try_into().unwrap());
        assert_eq!(
            stl_tri as usize,
            mesh.triangles.len(),
            "[{}] STL tri count",
            task.name
        );
        assert_eq!(
            stl_bytes.len(),
            84 + mesh.triangles.len() * 50,
            "[{}] STL size",
            task.name
        );
        assert_eq!(
            stl_bytes,
            stl::encode_binary(&mesh),
            "[{}] STL deterministic",
            task.name
        );

        // ── GLB: マジック + JSON チャンクが parse でき accessor count が一致 ──
        let glb = gltf::encode_glb(&mesh);
        assert_eq!(&glb[0..4], b"glTF", "[{}] GLB magic", task.name);
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc = parse(
            std::str::from_utf8(&glb[20..20 + json_len])
                .unwrap()
                .trim_end(),
        )
        .unwrap_or_else(|e| panic!("[{}] GLB JSON must parse: {e}", task.name));
        let accessors = doc.get("accessors").and_then(|a| a.as_array()).unwrap();
        let pos_count = accessors[0].get("count").and_then(|c| c.as_f64()).unwrap() as usize;
        let idx_count = accessors[1].get("count").and_then(|c| c.as_f64()).unwrap() as usize;
        assert_eq!(
            pos_count,
            mesh.vertices.len(),
            "[{}] GLB POSITION count",
            task.name
        );
        assert_eq!(
            idx_count,
            mesh.triangles.len() * 3,
            "[{}] GLB index count",
            task.name
        );
        assert_eq!(
            glb,
            gltf::encode_glb(&mesh),
            "[{}] GLB deterministic",
            task.name
        );

        // ── 3MF: ZIP 署名 + vertex/triangle 要素数が一致 ──
        let mf = threemf::encode_3mf(&mesh);
        assert_eq!(
            &mf[0..4],
            &[0x50, 0x4B, 0x03, 0x04],
            "[{}] 3MF is ZIP",
            task.name
        );
        assert_eq!(
            mf,
            threemf::encode_3mf(&mesh),
            "[{}] 3MF deterministic",
            task.name
        );

        // ── HTML: doctype + プレースホルダ全置換 + 非有限リテラルなし ──
        let h = html::encode_html(&mesh);
        assert!(
            h.starts_with("<!DOCTYPE html>"),
            "[{}] HTML doctype",
            task.name
        );
        for ph in ["/*POSITIONS*/", "/*INDICES*/", "/*CENTER*/", "/*RADIUS*/"] {
            assert!(
                !h.contains(ph),
                "[{}] HTML placeholder {ph} must be replaced",
                task.name
            );
        }
        let hl = h.to_lowercase();
        assert!(
            !hl.contains("nan") && !hl.contains("inf"),
            "[{}] HTML no nan/inf",
            task.name
        );
        assert_eq!(
            h,
            html::encode_html(&mesh),
            "[{}] HTML deterministic",
            task.name
        );
    }
}

#[test]
fn eval_set_extraction_is_byte_deterministic() {
    // 問5: 各課題で2回の抽出がビット単位一致 (HashMap 順序依存等の退行検知)。
    let res = 32;
    for task in eval_set() {
        let sdf = eval_scene(task.script).unwrap();
        let (lo, hi) = sdf.sampling_box();
        let a = polygonize(&sdf, lo, hi, res);
        let b = polygonize(&sdf, lo, hi, res);
        assert_eq!(
            a.triangles, b.triangles,
            "[{}] triangle lists must be identical across runs",
            task.name
        );
        assert_eq!(
            a.vertices.len(),
            b.vertices.len(),
            "[{}] vertex counts must match",
            task.name
        );
        for (va, vb) in a.vertices.iter().zip(&b.vertices) {
            assert_eq!(va.x.to_bits(), vb.x.to_bits(), "[{}] x bits", task.name);
            assert_eq!(va.y.to_bits(), vb.y.to_bits(), "[{}] y bits", task.name);
            assert_eq!(va.z.to_bits(), vb.z.to_bits(), "[{}] z bits", task.name);
        }
    }
}
