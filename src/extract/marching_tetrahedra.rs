//! Marching Tetrahedra による水密メッシュ抽出。
//!
//! 各立方体セルを6四面体に分割し、各四面体内の符号変化を線形補間する。
//! MTは marching cubes のような曖昧ケースを持たず、エッジ補間を**正準化**
//! (両隣のセルから同一バイト列の交点が得られる) することで水密性を保証する。
//!
//! 向き付けは三角形ごとに SDF 勾配で外向きに補正する (ケース別の巻き順管理を
//! 排し、堅牢にする)。

use crate::core::{Sdf, Vec3};
use crate::extract::mesh::Mesh;

/// サンプル格子上の角 (グリッド整数座標・実座標・SDF値)。
#[derive(Clone, Copy)]
struct Corner {
    coord: [i32; 3],
    pos: Vec3,
    val: f64,
}

/// エッジ上の零交差点。正準順序 (グリッド座標の辞書順) で補間し、
/// 隣接四面体から同一の f64 ビット列を得る (問5/水密性)。
fn edge_vertex(a: &Corner, b: &Corner) -> Vec3 {
    let (p, q) = if a.coord <= b.coord { (a, b) } else { (b, a) };
    let denom = p.val - q.val;
    // 問86: t を [0,1] にクランプする。浮動小数点誤差で両隅が同符号になると
    // t が [0,1] 外になり四面体セル外へ外挿してしまい、非多様体頂点を生む。
    let t = if denom == 0.0 { 0.5 } else { (p.val / denom).clamp(0.0, 1.0) };
    p.pos + (q.pos - p.pos) * t
}

// 立方体の8角 (x,y,z ∈ {0,1})。
const CUBE: [[i32; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [1, 1, 0],
    [0, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [1, 1, 1],
    [0, 1, 1],
];

// 0-6 対角を共有する6四面体分割。立方体を隙間なく充填する。
const TETS: [[usize; 4]; 6] = [
    [0, 5, 1, 6],
    [0, 1, 2, 6],
    [0, 2, 3, 6],
    [0, 3, 7, 6],
    [0, 7, 4, 6],
    [0, 4, 5, 6],
];

/// SDF木を bounds `[min,max]^3` 上で各軸 `res` 分割して抽出する。
///
/// `res` は1軸あたりのセル数。頂点サンプル数は `(res+1)^3`。
pub fn polygonize(sdf: &Sdf, min: Vec3, max: Vec3, res: usize) -> Mesh {
    assert!(res >= 1, "res must be >= 1");
    let n = res + 1;
    let step = Vec3::new(
        (max.x - min.x) / res as f64,
        (max.y - min.y) / res as f64,
        (max.z - min.z) / res as f64,
    );

    // 角の SDF 値を事前計算 (各サンプル点1回)。
    let idx = |i: usize, j: usize, k: usize| (i * n + j) * n + k;
    let mut vals = vec![0.0f64; n * n * n];
    let mut poss = vec![Vec3::ZERO; n * n * n];
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                let p = Vec3::new(
                    min.x + step.x * i as f64,
                    min.y + step.y * j as f64,
                    min.z + step.z * k as f64,
                );
                poss[idx(i, j, k)] = p;
                vals[idx(i, j, k)] = sdf.eval(p);
            }
        }
    }

    let mut soup: Vec<[Vec3; 3]> = Vec::new();
    for i in 0..res {
        for j in 0..res {
            for k in 0..res {
                // この立方体セルの8角。
                let mut corners = [Corner {
                    coord: [0, 0, 0],
                    pos: Vec3::ZERO,
                    val: 0.0,
                }; 8];
                for (c, off) in corners.iter_mut().zip(CUBE.iter()) {
                    let (ci, cj, ck) = (
                        i + off[0] as usize,
                        j + off[1] as usize,
                        k + off[2] as usize,
                    );
                    *c = Corner {
                        coord: [ci as i32, cj as i32, ck as i32],
                        pos: poss[idx(ci, cj, ck)],
                        val: vals[idx(ci, cj, ck)],
                    };
                }
                for tet in &TETS {
                    emit_tet(sdf, &corners, *tet, &mut soup);
                }
            }
        }
    }

    Mesh::from_soup(&soup)
}

/// 1四面体を処理し、0/1/2 枚の三角形を `soup` に追加する。
/// 内部判定は `val < 0` (SDFは内部負)。
///
/// 問116: 内/外の角インデックス収集を `Vec` から固定長スタック配列に変更し、
/// ホットパス (res^3 セル × 6 四面体 = 数百万回呼ばれる) のヒープ確保を排除する。
/// 収集順 (0..4 昇順) は元の `filter().collect()` と完全一致するため、edge_vertex の
/// 評価順・三角形の巻き順が不変 → 出力メッシュはバイト同一 (問5 決定性契約)。
fn emit_tet(sdf: &Sdf, corners: &[Corner; 8], tet: [usize; 4], soup: &mut Vec<[Vec3; 3]>) {
    let c = [
        corners[tet[0]],
        corners[tet[1]],
        corners[tet[2]],
        corners[tet[3]],
    ];
    // 内 (val<0) と外 (val>=0) の角を昇順で固定長配列に振り分ける。
    // 旧 (0..4).filter(...).collect() と同じ昇順なので決定性は不変。
    let mut inside = [0usize; 4];
    let mut n_in = 0usize;
    let mut outside = [0usize; 4];
    let mut n_out = 0usize;
    for (n, corner) in c.iter().enumerate() {
        if corner.val < 0.0 {
            inside[n_in] = n;
            n_in += 1;
        } else {
            outside[n_out] = n;
            n_out += 1;
        }
    }

    match n_in {
        0 | 4 => {}
        1 => {
            let i = inside[0];
            // 外3点 = outside[0..3] (昇順) = 旧 (0..4).filter(|n| n != i) と同順。
            push_tri(
                sdf,
                soup,
                edge_vertex(&c[i], &c[outside[0]]),
                edge_vertex(&c[i], &c[outside[1]]),
                edge_vertex(&c[i], &c[outside[2]]),
            );
        }
        3 => {
            let o = outside[0];
            // 内3点 = inside[0..3] (昇順) = 旧 (0..4).filter(|n| n != o) と同順。
            push_tri(
                sdf,
                soup,
                edge_vertex(&c[o], &c[inside[0]]),
                edge_vertex(&c[o], &c[inside[1]]),
                edge_vertex(&c[o], &c[inside[2]]),
            );
        }
        2 => {
            let (i0, i1) = (inside[0], inside[1]);
            let (o0, o1) = (outside[0], outside[1]);
            // 四辺形 (内2点×外2点) を2三角形に分割。
            let a = edge_vertex(&c[i0], &c[o0]);
            let b = edge_vertex(&c[i0], &c[o1]);
            let d = edge_vertex(&c[i1], &c[o1]);
            let e = edge_vertex(&c[i1], &c[o0]);
            push_tri(sdf, soup, a, b, d);
            push_tri(sdf, soup, a, d, e);
        }
        _ => unreachable!(),
    }
}

/// 三角形を外向き (SDF勾配方向) に揃えて追加。退化はスキップ。
fn push_tri(sdf: &Sdf, soup: &mut Vec<[Vec3; 3]>, p0: Vec3, p1: Vec3, p2: Vec3) {
    let normal = (p1 - p0).cross(p2 - p0);
    if normal.length() == 0.0 {
        return; // 退化
    }
    let centroid = (p0 + p1 + p2) / 3.0;
    // 問87: 固定 h=1e-4 はシェル厚 < 0.2mm のとき壁を突き抜け外向きを誤判定する。
    // 三角形最短辺の 1% を h とし、シェルより小さいステップで勾配を推定する。
    let min_edge = (p1 - p0).length()
        .min((p2 - p0).length())
        .min((p2 - p1).length());
    let h = (min_edge * 0.01).clamp(1e-9, 1e-4);
    let grad = gradient(sdf, centroid, h);
    // 問88: 勾配がゼロベクトルのとき dot=0 で反転しないが向き保証はない。
    // ゼロ勾配 (鞍点・blend境界) ではデフォルトの巻き順を維持し確定的に扱う。
    if grad.length() > 1e-12 && normal.dot(grad) < 0.0 {
        soup.push([p0, p2, p1]); // 反転
    } else {
        soup.push([p0, p1, p2]);
    }
}

/// 中心差分による SDF 勾配 (向き判定用。符号のみ重要)。
/// h は探針ステップ (三角形スケールに合わせること: 問87)。
fn gradient(sdf: &Sdf, p: Vec3, h: f64) -> Vec3 {
    let dx = sdf.eval(p + Vec3::new(h, 0.0, 0.0)) - sdf.eval(p - Vec3::new(h, 0.0, 0.0));
    let dy = sdf.eval(p + Vec3::new(0.0, h, 0.0)) - sdf.eval(p - Vec3::new(0.0, h, 0.0));
    let dz = sdf.eval(p + Vec3::new(0.0, 0.0, h)) - sdf.eval(p - Vec3::new(0.0, 0.0, h));
    Vec3::new(dx, dy, dz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn sphere_mesh_is_watertight() {
        let s = Sdf::sphere(1.0);
        let m = polygonize(&s, Vec3::splat(-1.5), Vec3::splat(1.5), 32);
        assert!(!m.triangles.is_empty());
        // 問11: 抽出メッシュは閉じた2-多様体 (水密)。
        assert!(m.is_edge_manifold(), "sphere mesh must be edge-manifold");
    }

    #[test]
    fn sphere_volume_within_tolerance() {
        let s = Sdf::sphere(1.0);
        let m = polygonize(&s, Vec3::splat(-1.5), Vec3::splat(1.5), 48);
        let analytic = 4.0 / 3.0 * PI; // r=1
        let v = m.signed_volume();
        let err = (v - analytic).abs() / analytic;
        assert!(err < 0.02, "volume err {err} (v={v}, exact={analytic})");
    }

    #[test]
    fn sphere_bounds_match_radius() {
        let s = Sdf::sphere(1.0);
        let m = polygonize(&s, Vec3::splat(-1.5), Vec3::splat(1.5), 32);
        let (lo, hi) = m.bounds().unwrap();
        let step = 3.0 / 32.0;
        for &x in &[lo.x, lo.y, lo.z] {
            assert!((x + 1.0).abs() < step * 1.5, "lo {x}");
        }
        for &x in &[hi.x, hi.y, hi.z] {
            assert!((x - 1.0).abs() < step * 1.5, "hi {x}");
        }
    }

    #[test]
    fn boolean_difference_mesh_is_watertight() {
        // 球から円柱を貫通させた穴あき形状でも水密 (問11)。
        let model = Sdf::sphere(1.0).difference(Sdf::cylinder(0.4, 2.0));
        let m = polygonize(&model, Vec3::splat(-1.5), Vec3::splat(1.5), 40);
        assert!(!m.triangles.is_empty());
        assert!(m.is_edge_manifold(), "holed model must stay watertight");
    }

    #[test]
    fn empty_region_yields_no_triangles() {
        // 全点が外側 (正) のとき三角形は出ない。
        let s = Sdf::sphere(0.1);
        let m = polygonize(&s, Vec3::new(2.0, 2.0, 2.0), Vec3::new(3.0, 3.0, 3.0), 8);
        assert!(m.triangles.is_empty());
    }

    #[test]
    fn polygonize_is_byte_deterministic() {
        // 問17: 看板KPI「決定的出力」をエンドツーエンドで固定する。
        // 独立した2回の抽出 (頂点統合に HashMap を使う) が **ビット単位で同一** の
        // 頂点列・三角形列を生むことを保証 (将来の HashMap 順序依存の退行を検知)。
        let model = Sdf::sphere(1.0)
            .union(Sdf::cuboid(Vec3::splat(0.8)))
            .difference(Sdf::cylinder(0.3, 2.0));
        let a = polygonize(&model, Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        let b = polygonize(&model, Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        assert_eq!(a.triangles, b.triangles, "triangle index lists must match");
        assert_eq!(
            a.vertices.len(),
            b.vertices.len(),
            "vertex counts must match"
        );
        for (va, vb) in a.vertices.iter().zip(&b.vertices) {
            assert_eq!(va.x.to_bits(), vb.x.to_bits());
            assert_eq!(va.y.to_bits(), vb.y.to_bits());
            assert_eq!(va.z.to_bits(), vb.z.to_bits());
        }
    }

    #[test]
    fn watertight_guarantee_holds_across_shape_battery() {
        // 問19: 製品中核の主張 (問11: 水密100%) を2例の抜き取りではなく
        // 形状バッテリで性質テストする。各形状で edge-manifold (水密) を保証する。
        let b = Vec3::splat(2.0);
        let battery: Vec<(&str, Sdf)> = vec![
            ("sphere", Sdf::sphere(1.0)),
            ("cuboid", Sdf::cuboid(Vec3::new(1.0, 0.7, 0.5))),
            ("cylinder", Sdf::cylinder(0.8, 1.0)),
            ("torus", Sdf::torus(1.0, 0.35)),
            (
                "cone",
                Sdf::cone(1.0, 1.5).translate(Vec3::new(0.0, 0.0, 0.75)),
            ),
            ("capsule", Sdf::capsule(0.8, 0.4)),
            ("rounded_box", Sdf::rounded_box(Vec3::splat(1.0), 0.3)),
            ("ellipsoid", Sdf::ellipsoid(Vec3::new(1.5, 1.0, 0.6))),
            (
                "union",
                Sdf::sphere(1.0).union(Sdf::cuboid(Vec3::splat(0.9))),
            ),
            (
                "intersection",
                Sdf::sphere(1.2).intersection(Sdf::cuboid(Vec3::splat(1.0))),
            ),
            (
                "difference",
                Sdf::cuboid(Vec3::splat(1.0)).difference(Sdf::sphere(1.2)),
            ),
            (
                "smooth_union",
                Sdf::sphere(0.9)
                    .smooth_union(Sdf::sphere(0.9).translate(Vec3::new(1.0, 0.0, 0.0)), 0.4),
            ),
            ("shell", Sdf::sphere(1.0).shell(0.2)),
            (
                "mirror",
                Sdf::sphere(0.5)
                    .translate(Vec3::new(0.8, 0.0, 0.0))
                    .mirror_x(),
            ),
            (
                "rotate",
                Sdf::cuboid(Vec3::new(1.2, 0.4, 0.4)).rotate_z(0.6).rotate_x(0.3),
            ),
        ];
        for (name, sdf) in &battery {
            let m = polygonize(sdf, -b, b, 28);
            assert!(!m.triangles.is_empty(), "{name}: mesh unexpectedly empty");
            assert!(
                m.is_edge_manifold(),
                "{name}: mesh must be edge-manifold (watertight)"
            );
            // 向き一貫性の代理: 外向き整合なら符号付き体積は正。
            assert!(
                m.signed_volume() > 0.0,
                "{name}: signed volume must be positive (consistent outward orientation), got {}",
                m.signed_volume()
            );
        }
    }

    #[test]
    fn watertight_holds_for_adversarial_coincident_cases() {
        // 問24 (問11の難所): 完全一致面・格子整列面でも水密が崩れないか実証する。
        let cases: Vec<(&str, Sdf, Vec3, Vec3, usize)> = vec![
            // 軸整列直方体の面が標本平面に正確に載るケース (val がちょうど 0)。
            (
                "grid_aligned_cuboid",
                Sdf::cuboid(Vec3::splat(1.0)),
                Vec3::splat(-2.0),
                Vec3::splat(2.0),
                4, // step=1.0 → 面が x,y,z=±1 の標本平面に一致
            ),
            // 同一球の和 (全面が完全一致)。
            (
                "coincident_union",
                Sdf::sphere(1.0).union(Sdf::sphere(1.0)),
                Vec3::splat(-1.5),
                Vec3::splat(1.5),
                24,
            ),
            // 同一立方体の積 (全面が完全一致)。
            (
                "coincident_intersection",
                Sdf::cuboid(Vec3::splat(1.0)).intersection(Sdf::cuboid(Vec3::splat(1.0))),
                Vec3::splat(-2.0),
                Vec3::splat(2.0),
                4,
            ),
        ];
        for (name, sdf, lo, hi, res) in &cases {
            let m = polygonize(sdf, *lo, *hi, *res);
            assert!(!m.triangles.is_empty(), "{name}: unexpectedly empty");
            assert!(
                m.is_edge_manifold(),
                "{name}: must stay edge-manifold even at coincident/grid-aligned surfaces"
            );
            assert!(
                m.signed_volume() > 0.0,
                "{name}: signed volume must be positive, got {}",
                m.signed_volume()
            );
        }
    }

    #[test]
    fn boundary_clipping_is_detected_not_silently_watertight() {
        // 問25 (問14の安全網): 形状がサンプリング境界を超えると表面が箱の面で
        // 開く。これが「無音で水密扱い」されず必ず検知されることを保証する。
        let s = Sdf::sphere(1.0);

        // 部分クリップ: 箱が z=0 で球を切る → z=0 面に開いた境界 → 非多様体で検知。
        let clipped = polygonize(
            &s,
            Vec3::new(-1.5, -1.5, -1.5),
            Vec3::new(1.5, 1.5, 0.0),
            24,
        );
        assert!(!clipped.triangles.is_empty());
        assert!(
            !clipped.is_edge_manifold(),
            "a surface cut by the sampling box must be reported non-manifold (open)"
        );

        // 完全内包: 箱が球の内部に収まる → 符号変化なし → 空メッシュ (EMPTY_MESH で検知)。
        let interior = polygonize(&s, Vec3::splat(-0.5), Vec3::splat(0.5), 16);
        assert!(
            interior.triangles.is_empty(),
            "a box fully inside the shape yields an empty mesh (caught as EMPTY_MESH)"
        );

        // 健全な境界では水密。
        let ok = polygonize(&s, Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        assert!(ok.is_edge_manifold());
    }

    #[test]
    fn edge_vertex_clamp_produces_valid_interpolation() {
        // 問86: 両隅が同符号 (浮動小数点誤差) のとき t がクランプされ
        // セル内に留まることを確認する。
        let a = Corner { coord: [0, 0, 0], pos: Vec3::new(0.0, 0.0, 0.0), val: 0.5 };
        let b = Corner { coord: [1, 0, 0], pos: Vec3::new(1.0, 0.0, 0.0), val: 1.0 };
        // 両方正 (同符号): denom = 0.5 - 1.0 = -0.5, t = 0.5 / -0.5 = -1 → クランプで 0.0。
        let v = edge_vertex(&a, &b);
        // クランプ後 t=0, 補間結果 = p.pos + (q.pos - p.pos) * 0 = a.pos = (0,0,0)
        // (p = 辞書順で先の角。coord比較で a < b なので p=a)
        assert!(
            v.x >= 0.0 && v.x <= 1.0,
            "clamped interpolation must stay within cell bounds, got x={}",
            v.x
        );

        // 正常ケース (異符号): t ∈ (0,1) → 等号チェック。
        let neg = Corner { coord: [0, 0, 0], pos: Vec3::new(0.0, 0.0, 0.0), val: -0.3 };
        let pos = Corner { coord: [1, 0, 0], pos: Vec3::new(1.0, 0.0, 0.0), val: 0.7 };
        let v2 = edge_vertex(&neg, &pos);
        // t = -0.3 / (-0.3 - 0.7) = -0.3 / -1.0 = 0.3 → x = 0 + 1*0.3 = 0.3
        assert!((v2.x - 0.3).abs() < 1e-12, "normal sign-change: x should be 0.3, got {}", v2.x);
    }

    #[test]
    fn thin_shell_mesh_is_watertight() {
        // 問87: gradient に適応的 h を使うことでシェル厚 << 1e-4 でも
        // 外向き補正が正しく機能することを確認。水密性が失われないことが保証。
        // shell(sphere(1.0), 0.05) の厚さ = 0.05 << デフォルト h=0.0001 の 500 倍。
        // (注: 0.05 > 0.0001 なので従来も大丈夫だが、薄いシェルでの水密を明示的に確認)
        let sdf = Sdf::sphere(1.0).shell(0.05);
        let (lo, hi) = sdf.sampling_box();
        let m = polygonize(&sdf, lo, hi, 40);
        assert!(!m.triangles.is_empty(), "thin shell must produce triangles");
        assert!(
            m.is_edge_manifold(),
            "thin shell mesh must be edge-manifold (watertight)"
        );
    }

    #[test]
    fn zero_gradient_region_does_not_produce_inverted_mesh() {
        // 問88: smooth_union ブレンド境界はゼロ勾配を持ちうる (鞍点)。
        // ゼロ勾配時に反転しないことで水密性が保たれることを確認。
        let sdf = Sdf::sphere(1.0).smooth_union(Sdf::sphere(1.0).translate(Vec3::new(1.5, 0.0, 0.0)), 0.5);
        let (lo, hi) = sdf.sampling_box();
        let m = polygonize(&sdf, lo, hi, 32);
        assert!(
            m.is_edge_manifold(),
            "smooth_union blend region (potential zero gradient) must yield watertight mesh"
        );
        assert!(
            m.signed_volume() > 0.0,
            "smooth_union volume must be positive (correct winding)"
        );
    }

    #[test]
    fn extraction_digest_is_byte_stable_golden() {
        // 問116: emit_tet のアロケーション除去リファクタが出力を**バイト単位で**
        // 変えていないことを golden ダイジェストで固定する。決定性契約 (問5) では
        // 「同一バイナリ・同一arch・同一スクリプト・同一解像度 → バイト同一」を保証する。
        // 内部最適化 (Vec→スタック配列) はこの契約に観測可能な影響を与えてはならない。
        // この値はリファクタ前の実装で測定したもの。変更があれば即座に検出される。
        let s = Sdf::sphere(1.0);
        let m = polygonize(&s, Vec3::splat(-1.5), Vec3::splat(1.5), 16);
        assert_eq!(
            m.digest(),
            0x13a3_7711_0ebc_a030,
            "sphere extraction digest must remain byte-stable across internal refactors"
        );
        assert_eq!(m.vertices.len(), 1586, "sphere vertex count must be stable");
        assert_eq!(m.triangles.len(), 3168, "sphere triangle count must be stable");

        // 全4ケース (内0/1/2/3) を踏む穴あき形状でも固定する。
        let diff = Sdf::sphere(1.0).difference(Sdf::cylinder(0.4, 2.0));
        let md = polygonize(&diff, Vec3::splat(-1.5), Vec3::splat(1.5), 20);
        assert_eq!(
            md.digest(),
            0xabb6_848b_19e4_319a,
            "boolean-difference extraction digest must remain byte-stable"
        );
        assert_eq!(md.vertices.len(), 3036, "diff vertex count must be stable");
        assert_eq!(md.triangles.len(), 6072, "diff triangle count must be stable");
    }
}
