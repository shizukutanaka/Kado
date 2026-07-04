//! 敵対的モデルの水密性ストレステスト (問232)。
//!
//! eval_set (tests/eval_set.rs) が代表的な「正常系」モデルを扱うのに対し、
//! 本スイートは Marching Tetrahedra の限界に近い**敵対的入力**を集中的に運動させる:
//!   - 薄壁シェル (壁厚がステップ幅に迫る)
//!   - 深いネスト CSG
//!   - 極端アスペクト比のプリミティブ
//!   - ほぼ接する形状の smooth_union (鞍点・ゼロ勾配)
//!   - 細い穴 / 薄板
//!   - 変換の合成 (回転 + 反復)
//!
//! 不変条件: これらの難所でも抽出メッシュは **edge-manifold (水密)** かつ
//! **符号付き体積が正** (向き一貫) でなければならない。複数解像度で確認し、
//! 解像度依存の退行 (低解像度での非多様体化など) を検知する。
//!
//! 注 (既知の限界, SPEC §9): ステップ (diag/res) より薄いフィーチャは
//! **体積が過少**になりうるが、**水密性は保たれる**こと自体が契約である。

use kado::extract::polygonize;
use kado::script::eval_scene;

struct Model {
    name: &'static str,
    script: &'static str,
}

fn adversarial_models() -> Vec<Model> {
    vec![
        Model {
            name: "thin shell (wall 0.08mm)",
            script: r#"{"op":"shell","thickness":0.08,"shape":{"op":"sphere","r":1.0}}"#,
        },
        Model {
            name: "deep nested union (5 spheres)",
            script: r#"{"op":"union",
                "a":{"op":"union",
                     "a":{"op":"union",
                          "a":{"op":"union",
                               "a":{"op":"sphere","r":0.5},
                               "b":{"op":"translate","x":0.3,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}}},
                          "b":{"op":"translate","x":0.6,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}}},
                     "b":{"op":"translate","x":0.9,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}}},
                "b":{"op":"translate","x":1.2,"y":0,"z":0,"shape":{"op":"sphere","r":0.5}}}"#,
        },
        Model {
            name: "extreme aspect capsule (h=3 r=0.05)",
            script: r#"{"op":"capsule","h":3.0,"r":0.05}"#,
        },
        Model {
            name: "kissing spheres smooth_union (k=0.05)",
            script: r#"{"op":"smooth_union","k":0.05,
                "a":{"op":"translate","x":-1.0,"y":0,"z":0,"shape":{"op":"sphere","r":1.0}},
                "b":{"op":"translate","x":1.0,"y":0,"z":0,"shape":{"op":"sphere","r":1.0}}}"#,
        },
        Model {
            name: "thin perforated plate (hole r=0.9 in z=0.1 plate)",
            script: r#"{"op":"difference",
                "a":{"op":"cuboid","x":1.0,"y":1.0,"z":0.1},
                "b":{"op":"cylinder","r":0.9,"h":1.0}}"#,
        },
        Model {
            name: "rotated repeat lattice",
            script: r#"{"op":"rotate_z","angle":33,
                "shape":{"op":"repeat","x":0.8,"nx":2,"shape":{"op":"sphere","r":0.25}}}"#,
        },
        Model {
            name: "thin-walled high-n prism shell (問269/274)",
            // regular_polygon_2d の角度フォールディングは n が大きいほどセクター境界を
            // 多く持つ。薄壁 (シャープなコーナーは滑らかな球面より抽出が難しい) と
            // 組み合わせ、既存の thin shell (球) では検出できない角部特有の退行を狙う。
            script: r#"{"op":"shell","thickness":0.08,
                "shape":{"op":"prism","n":24,"r":1.0,"h":1.0}}"#,
        },
        Model {
            name: "extreme aspect capsule on arbitrary diagonal axis (問266/274)",
            // 既存の "extreme aspect capsule" は軸整列 (Z軸) のまま。任意軸回転と
            // 組み合わせることで、rotate_box_axis の AABB が極端な形状を正しく包含し
            // (過小だと sampling_box が形状を切り欠き OPEN_MESH になる)、かつ
            // 非軸整列の抽出でも水密性が保たれることを確認する。
            script: r#"{"op":"rotate","ax":1,"ay":1,"az":1,"angle":25,
                "shape":{"op":"capsule","h":3.0,"r":0.05}}"#,
        },
    ]
}

#[test]
fn adversarial_models_stay_watertight_across_resolutions() {
    for &res in &[24usize, 48, 64] {
        for m in adversarial_models() {
            let sdf = eval_scene(m.script)
                .unwrap_or_else(|e| panic!("[{}] script failed to evaluate: {e}", m.name));
            let (lo, hi) = sdf.sampling_box();
            let mesh = polygonize(&sdf, lo, hi, res);

            // 敵対的でも非空であること (これらは全て十分なサイズを持つ)。
            assert!(
                !mesh.triangles.is_empty(),
                "[{}] res={res}: mesh unexpectedly empty",
                m.name
            );
            // 水密 (問11/SPEC §4.3): 難所でも edge-manifold を維持する。
            assert!(
                mesh.is_edge_manifold(),
                "[{}] res={res}: mesh must remain edge-manifold (watertight)",
                m.name
            );
            // 向き一貫性: 符号付き体積は正。
            assert!(
                mesh.signed_volume() > 0.0,
                "[{}] res={res}: signed volume must be positive, got {}",
                m.name,
                mesh.signed_volume()
            );
        }
    }
}

#[test]
fn under_resolved_thin_shell_stays_watertight_even_when_volume_is_underestimated() {
    // SPEC §9 の既知の限界を明示的に固定する: ステップより薄い壁は体積が過少に
    // なりうるが、水密性は保たれる。壁厚 0.08mm のシェルを低解像度 (res=20,
    // step = diag/20 ≈ 0.17 > 0.08) で抽出しても edge-manifold であることを確認。
    let sdf =
        eval_scene(r#"{"op":"shell","thickness":0.08,"shape":{"op":"sphere","r":1.0}}"#).unwrap();
    let (lo, hi) = sdf.sampling_box();
    let mesh = polygonize(&sdf, lo, hi, 20);
    assert!(
        !mesh.triangles.is_empty(),
        "under-resolved shell still produces triangles"
    );
    assert!(
        mesh.is_edge_manifold(),
        "under-resolved thin shell must STILL be watertight (the contract is manifold, not volume accuracy)"
    );
}
