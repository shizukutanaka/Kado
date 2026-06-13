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
            name: "mirrored fin pair",
            script: r#"{"op":"mirror_x",
                "shape":{"op":"translate","x":0.7,"y":0,"z":0,
                         "shape":{"op":"capsule","h":0.5,"r":0.25}}}"#,
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
