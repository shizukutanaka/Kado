//! インデックス付き三角形メッシュと健全性チェック (問11)。

use crate::core::Vec3;
use std::collections::HashMap;

/// インデックス付き三角形メッシュ。
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

/// 頂点座標を f64 ビット列で正準キー化する (正準補間により共有点は
/// バイト一致するため、重複が確実に統合され水密性が成立する, 問5/問11)。
fn vertex_key(v: Vec3) -> [u64; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

impl Mesh {
    /// 三角形スープ (各三角形が3頂点座標を持つ) から重複頂点を統合して構築。
    pub fn from_soup(soup: &[[Vec3; 3]]) -> Mesh {
        let mut mesh = Mesh::default();
        let mut index: HashMap<[u64; 3], u32> = HashMap::new();
        for tri in soup {
            let mut idx = [0u32; 3];
            for (slot, &p) in idx.iter_mut().zip(tri.iter()) {
                let key = vertex_key(p);
                *slot = *index.entry(key).or_insert_with(|| {
                    mesh.vertices.push(p);
                    (mesh.vertices.len() - 1) as u32
                });
            }
            // 退化三角形 (面積ゼロ) は捨てる。
            if idx[0] != idx[1] && idx[1] != idx[2] && idx[0] != idx[2] {
                mesh.triangles.push(idx);
            }
        }
        mesh
    }

    /// すべての無向エッジがちょうど2つの三角形に共有されるか
    /// (= 閉じた2-多様体・水密)。
    pub fn is_edge_manifold(&self) -> bool {
        let (boundary, nonmanifold) = self.edge_defects();
        boundary == 0 && nonmanifold == 0
    }

    /// 無向エッジ (頂点インデックス昇順ペア) ごとの共有三角形数。
    /// `edge_defects`/`first_boundary_edge_midpoint`/`first_nonmanifold_edge_midpoint` が共有する
    /// 内部集計 (問257: 重複計算を避ける)。
    fn edge_counts(&self) -> HashMap<(u32, u32), u32> {
        let mut counts: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &self.triangles {
            for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let key = if a < b { (a, b) } else { (b, a) };
                *counts.entry(key).or_insert(0) += 1;
            }
        }
        counts
    }

    /// 条件 `pred(c)` を満たすエッジのうち、最小頂点インデックスのものの中点を返す。
    /// HashMap 走査後に最小キーを選ぶため決定的 (問257/ADR-003)。
    fn first_edge_midpoint_where(
        &self,
        counts: &HashMap<(u32, u32), u32>,
        pred: impl Fn(u32) -> bool,
    ) -> Option<Vec3> {
        let mut best: Option<(u32, u32)> = None;
        for (&(a, b), &c) in counts {
            if pred(c) {
                best = Some(match best {
                    None => (a, b),
                    Some(curr) if (a, b) < curr => (a, b),
                    Some(curr) => curr,
                });
            }
        }
        best.map(|(a, b)| {
            let va = self.vertices[a as usize];
            let vb = self.vertices[b as usize];
            (va + vb) * 0.5
        })
    }

    /// エッジ欠陥の内訳 `(開境界エッジ数, 非多様体接合エッジ数)`。
    ///
    /// - 開境界 (1面共有): 表面が閉じていない。サンプリング境界によるクリップや
    ///   ゼロ厚フィーチャが主因 (問25)。
    /// - 非多様体接合 (3面以上共有): 自己交差・座標一致による接合。
    ///
    /// 両者は是正策が異なる (前者=境界拡大、後者=解像度/形状分離) ため区別する。
    pub fn edge_defects(&self) -> (usize, usize) {
        let counts = self.edge_counts();
        let mut boundary = 0;
        let mut nonmanifold = 0;
        for &c in counts.values() {
            if c == 1 {
                boundary += 1;
            } else if c > 2 {
                nonmanifold += 1;
            }
        }
        (boundary, nonmanifold)
    }

    /// 開境界エッジ (1面のみ共有) が存在する場合、最小頂点インデックスのエッジ中点を返す。
    /// なければ `None` (問258)。
    pub fn first_boundary_edge_midpoint(&self) -> Option<Vec3> {
        let counts = self.edge_counts();
        self.first_edge_midpoint_where(&counts, |c| c == 1)
    }

    /// 非多様体エッジ (3面以上共有) が存在する場合、最小頂点インデックスのエッジ中点を返す。
    /// なければ `None` (問257)。
    pub fn first_nonmanifold_edge_midpoint(&self) -> Option<Vec3> {
        let counts = self.edge_counts();
        self.first_edge_midpoint_where(&counts, |c| c > 2)
    }

    /// 符号付き体積 (発散定理)。メッシュの向きが外向きに揃っている前提。
    pub fn signed_volume(&self) -> f64 {
        let mut v = 0.0;
        for t in &self.triangles {
            let a = self.vertices[t[0] as usize];
            let b = self.vertices[t[1] as usize];
            let c = self.vertices[t[2] as usize];
            v += a.dot(b.cross(c)) / 6.0;
        }
        v
    }

    /// 全三角形の面積和 = メッシュ表面積 (問244)。
    ///
    /// 各三角形 (a,b,c) の面積 `|(b-a)×(c-a)|/2` を固定順序の f64 加算で合計する
    /// (決定的・問5/ADR-003)。FDM プリントでは表面積が造形時間・材料費の主要因であり、
    /// 体積と並ぶ基本幾何量。`mean_wall_thickness` の 2V/SA 計算もこれを用いる
    /// (公式の二重定義を避ける)。
    pub fn surface_area(&self) -> f64 {
        let mut area = 0.0;
        for t in &self.triangles {
            let a = self.vertices[t[0] as usize];
            let b = self.vertices[t[1] as usize];
            let c = self.vertices[t[2] as usize];
            area += (b - a).cross(c - a).length() * 0.5;
        }
        area
    }

    /// 囲まれた立体の重心 (center of mass)。一様密度を仮定する (問238)。
    ///
    /// 発散定理と同系統: 各三角形と原点が成す四面体の符号付き体積 `v_t = a·(b×c)/6` と
    /// その重心 `(a+b+c)/4` (原点が第4頂点) を体積重み付き平均する。
    /// `Σ(v_t · (a+b+c)/4) / Σ(v_t)`。決定的 (固定順序の f64 加算・問5)。
    ///
    /// 体積が極小 (退化・空) なら `None`。`signed_volume` が負 (向き反転) でも
    /// 比は符号が打ち消し合い正しい重心を返す。
    pub fn center_of_mass(&self) -> Option<Vec3> {
        let mut vol = 0.0;
        let mut acc = Vec3::ZERO;
        for t in &self.triangles {
            let a = self.vertices[t[0] as usize];
            let b = self.vertices[t[1] as usize];
            let c = self.vertices[t[2] as usize];
            let v_t = a.dot(b.cross(c)) / 6.0;
            // 四面体 (原点,a,b,c) の重心 = (a+b+c)/4。
            acc = acc + (a + b + c) * (v_t * 0.25);
            vol += v_t;
        }
        if vol.abs() < 1e-12 {
            return None;
        }
        Some(acc * (1.0 / vol))
    }

    /// 正準メッシュの内容ダイジェスト (FNV-1a 64bit) を返す (問61)。
    ///
    /// 決定性 (問5) を**観測可能**にする: 同一バイナリ・同一arch・同一スクリプト・
    /// 同一解像度なら同じダイジェストになり、第三者が短いハッシュ1つで再現性を検証
    /// できる (ファイル全体の比較が不要)。頂点ビット列・三角形索引をメッシュ順
    /// (これ自体が決定的) で食わせる。
    pub fn digest(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64; // FNV offset basis
        for v in &self.vertices {
            h = fnv1a(h, &v.x.to_bits().to_le_bytes());
            h = fnv1a(h, &v.y.to_bits().to_le_bytes());
            h = fnv1a(h, &v.z.to_bits().to_le_bytes());
        }
        // 頂点数と三角形数も混ぜて長さ差を確実に反映する。
        h = fnv1a(h, &(self.vertices.len() as u64).to_le_bytes());
        for t in &self.triangles {
            for &i in t {
                h = fnv1a(h, &i.to_le_bytes());
            }
        }
        h
    }

    /// 連結成分を符号付き体積で分類し `(中実ボディ数, 内部空洞数)` を返す (問60)。
    ///
    /// 「水密」は「単一造形物」を意味しない: 離れた2形状の和は2つの閉殻 (= 2ボディ)
    /// になりうる。一方、中空シェルは外殻(+)と内殻(−)の2成分だが**1ボディ+1空洞**で
    /// 正常。よって面の連結成分を符号付き体積で分類し、正=ボディ・負=空洞と数える。
    ///
    /// 頂点を三角形エッジで結ぶ Union-Find で成分を求める。決定的 (問5)。
    pub fn body_components(&self) -> (usize, usize) {
        let n = self.vertices.len();
        if n == 0 || self.triangles.is_empty() {
            return (0, 0);
        }
        let mut parent: Vec<u32> = (0..n as u32).collect();
        for t in &self.triangles {
            let r = uf_find(&mut parent, t[0]);
            uf_union(&mut parent, r, t[1]);
            uf_union(&mut parent, r, t[2]);
        }
        // 成分ごとに符号付き体積を集計。
        let mut vols: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for t in &self.triangles {
            let root = uf_find(&mut parent, t[0]);
            let a = self.vertices[t[0] as usize];
            let b = self.vertices[t[1] as usize];
            let c = self.vertices[t[2] as usize];
            *vols.entry(root).or_insert(0.0) += a.dot(b.cross(c)) / 6.0;
        }
        // 退化成分 (面積ゼロ近傍) を無視する相対しきい値。
        let max_abs = vols.values().fold(0.0f64, |m, &v| m.max(v.abs()));
        let eps = (max_abs * 1e-6).max(f64::MIN_POSITIVE);
        let bodies = vols.values().filter(|&&v| v > eps).count();
        let cavities = vols.values().filter(|&&v| v < -eps).count();
        (bodies, cavities)
    }

    /// 軸整列バウンディングボックス (min, max)。空なら None。
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut it = self.vertices.iter();
        let first = *it.next()?;
        let (mut lo, mut hi) = (first, first);
        for &v in it {
            lo = lo.min(v);
            hi = hi.max(v);
        }
        Some((lo, hi))
    }
}

/// FNV-1a 64bit の1ステップ (バイト列を畳み込む)。
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

// ── Union-Find (経路圧縮) ────────────────────────────────────────────────────
fn uf_find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        parent[x as usize] = parent[parent[x as usize] as usize];
        x = parent[x as usize];
    }
    x
}

fn uf_union(parent: &mut [u32], a: u32, b: u32) {
    let ra = uf_find(parent, a);
    let rb = uf_find(parent, b);
    if ra != rb {
        // 小さい root を親にして決定性を保つ (問5: HashMap 等の順序に依存しない)。
        let (hi, lo) = if ra < rb { (rb, ra) } else { (ra, rb) };
        parent[hi as usize] = lo;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Sdf;
    use crate::extract::polygonize;

    #[test]
    fn from_soup_with_empty_input_returns_empty_mesh() {
        // 問141: from_soup(&[]) は空の Mesh を返す (パニックしない)。
        // body_components/signed_volume/bounds など全ての下流関数がこの状態を受け入れる。
        let m = Mesh::from_soup(&[]);
        assert!(
            m.vertices.is_empty(),
            "empty soup must yield empty vertex list"
        );
        assert!(
            m.triangles.is_empty(),
            "empty soup must yield empty triangle list"
        );
        // 下流関数が empty mesh を正しく処理する。
        assert_eq!(
            m.body_components(),
            (0, 0),
            "empty mesh has no bodies or cavities"
        );
        assert_eq!(m.signed_volume(), 0.0, "empty mesh has zero volume");
        assert!(m.bounds().is_none(), "empty mesh has no bounds");
        assert!(
            m.is_edge_manifold(),
            "empty mesh is trivially manifold (no edges)"
        );
        assert!(
            m.center_of_mass().is_none(),
            "empty mesh has no center of mass"
        );
        assert_eq!(m.surface_area(), 0.0, "empty mesh has zero surface area");
    }

    #[test]
    fn surface_area_of_unit_cube_is_six() {
        // 問244: 一辺 2 (±1) の立方体の表面積は 6 面 × (2×2) = 24。marching tetrahedra は
        // エッジ・コーナーを僅かに面取りするため真値より少し小さく出る (内接近似) が、
        // 5% 以内に収束する。面積和が正で 24 を超えないこと (面取り=面積減) を確認。
        use crate::core::Sdf;
        use crate::extract::polygonize;
        let m = polygonize(
            &Sdf::cuboid(Vec3::splat(1.0)),
            Vec3::splat(-1.5),
            Vec3::splat(1.5),
            32,
        );
        let area = m.surface_area();
        assert!(
            area <= 24.0 + 1e-6,
            "beveled cube area must not exceed 24, got {area}"
        );
        assert!(
            (area - 24.0).abs() / 24.0 < 0.05,
            "±1 cube area within 5% of 24, got {area}"
        );
    }

    #[test]
    fn surface_area_of_sphere_approximates_four_pi() {
        // 問244: 半径 1 の球の表面積は 4πr² ≈ 12.566。内接多面体なので真値より僅かに
        // 小さいが、十分な解像度で 5% 以内に収束する。
        use crate::core::Sdf;
        use crate::extract::polygonize;
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 48);
        let exact = 4.0 * std::f64::consts::PI;
        assert!(
            m.surface_area() <= exact,
            "inscribed polyhedron area must not exceed 4π"
        );
        assert!(
            (m.surface_area() - exact).abs() / exact < 0.05,
            "sphere area within 5% of 4π≈{exact:.3}, got {}",
            m.surface_area()
        );
    }

    #[test]
    fn center_of_mass_of_centered_sphere_is_origin() {
        // 問238: 原点中心の球の重心は原点。発散定理ベースの COM が対称形状で
        // 厳密に中心へ来ることを確認する (一様密度)。
        use crate::core::Sdf;
        use crate::extract::polygonize;
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 32);
        let com = m
            .center_of_mass()
            .expect("solid sphere has a center of mass");
        assert!(
            com.length() < 1e-2,
            "centered sphere COM must be ~origin, got {com:?}"
        );
    }

    #[test]
    fn center_of_mass_shifts_toward_translated_mass() {
        // 平行移動した球の重心はその中心へ移る。COM が質量分布を正しく追うことを確認。
        use crate::core::{Sdf, Vec3 as V};
        use crate::extract::polygonize;
        let shifted = Sdf::sphere(0.8).translate(V::new(2.0, 0.0, 0.0));
        let (lo, hi) = shifted.sampling_box();
        let m = polygonize(&shifted, lo, hi, 32);
        let com = m.center_of_mass().unwrap();
        assert!(
            (com.x - 2.0).abs() < 0.05,
            "COM.x must track the +2 shift, got {}",
            com.x
        );
        assert!(
            com.y.abs() < 0.05 && com.z.abs() < 0.05,
            "off-axis COM must stay ~0: {com:?}"
        );
    }

    #[test]
    fn center_of_mass_of_hemisphere_sits_above_flat_base() {
        // 問238: 平坦底面ドーム (上半球) の重心は底面 (z=0) より上、かつ理論値
        // 3R/8 = 0.375 付近にあることを確認する (安定性判定の土台)。
        use crate::core::{Sdf, Vec3 as V};
        use crate::extract::polygonize;
        let dome = Sdf::sphere(1.0).cut(V::new(0.0, 0.0, -1.0), 0.0);
        let (lo, hi) = dome.sampling_box();
        let m = polygonize(&dome, lo, hi, 48);
        let com = m.center_of_mass().unwrap();
        assert!(
            com.z > 0.0,
            "hemisphere COM must be above the flat base, got z={}",
            com.z
        );
        assert!(
            (com.z - 0.375).abs() < 0.03,
            "hemisphere COM.z ~ 3R/8 = 0.375, got {}",
            com.z
        );
        assert!(
            com.x.abs() < 0.02 && com.y.abs() < 0.02,
            "axisymmetric COM lateral ~0: {com:?}"
        );
    }

    #[test]
    fn digest_is_deterministic_and_geometry_sensitive() {
        // 問61: 同一メッシュは同一ダイジェスト、異なる形状は (ほぼ確実に) 異なる。
        let a = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 20);
        let b = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 20);
        assert_eq!(a.digest(), b.digest(), "same mesh must share digest");

        let c = polygonize(&Sdf::sphere(1.01), Vec3::splat(-1.5), Vec3::splat(1.5), 20);
        assert_ne!(
            a.digest(),
            c.digest(),
            "different geometry must change digest"
        );
        // 解像度差も反映される。
        let d = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        assert_ne!(
            a.digest(),
            d.digest(),
            "different resolution must change digest"
        );
    }

    #[test]
    fn single_solid_is_one_body_no_cavity() {
        let m = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        assert_eq!(m.body_components(), (1, 0));
    }

    #[test]
    fn two_disjoint_solids_are_two_bodies() {
        // 問60: 離れた2球の和は水密だが 2 ボディ。
        let a = Sdf::sphere(0.6).translate(Vec3::new(-1.2, 0.0, 0.0));
        let b = Sdf::sphere(0.6).translate(Vec3::new(1.2, 0.0, 0.0));
        let model = a.union(b);
        let (lo, hi) = model.sampling_box();
        let m = polygonize(&model, lo, hi, 40);
        assert!(m.is_edge_manifold(), "each shell is watertight");
        assert_eq!(m.body_components(), (2, 0), "two separate solids");
    }

    #[test]
    fn hollow_shell_is_one_body_one_cavity() {
        // 中空シェルは外殻(+)と内殻(−) → 1 ボディ + 1 空洞。誤って 2 ボディとしない。
        let model = Sdf::sphere(1.0).shell(0.25);
        let (lo, hi) = model.sampling_box();
        let m = polygonize(&model, lo, hi, 48);
        let (bodies, cavities) = m.body_components();
        assert_eq!(bodies, 1, "hollow shell is a single solid body");
        assert_eq!(cavities, 1, "with one internal cavity");
    }

    #[test]
    fn three_disjoint_solids_are_three_bodies() {
        // 問127: Union-Find が N > 2 の独立成分を正しく識別する。
        // 1ボディ・2ボディの既存テストを超え、経路圧縮が多成分でも正しいことを確認する。
        // (uf_find は経路分割を用いており、成分数が増えるほど圧縮の正確さが重要になる。)
        let a = Sdf::sphere(0.5).translate(Vec3::new(-3.0, 0.0, 0.0));
        let b = Sdf::sphere(0.5);
        let c = Sdf::sphere(0.5).translate(Vec3::new(3.0, 0.0, 0.0));
        let model = a.union(b).union(c);
        let (lo, hi) = model.sampling_box();
        let m = polygonize(&model, lo, hi, 32);
        assert!(
            m.is_edge_manifold(),
            "three disjoint spheres must be individually watertight"
        );
        assert_eq!(
            m.body_components(),
            (3, 0),
            "three disjoint spheres are three separate solid bodies"
        );
    }

    #[test]
    fn triangle_indices_are_in_bounds_and_nondegenerate() {
        // 問109: 全ての出力経路 (STL/GLB/3MF/HTML/レンダラ/体積/検証) は
        // `mesh.vertices[t[i] as usize]` で頂点を引く。インデックスが範囲外だと全経路が
        // パニックする。また from_soup は退化三角形 (重複インデックス) を捨てる契約。
        // 代表形状群でこの基礎不変条件を固定する。
        let shapes = [
            Sdf::sphere(1.0),
            Sdf::cuboid(Vec3::splat(0.8)),
            Sdf::sphere(1.0).difference(Sdf::cylinder(0.4, 2.0)),
            Sdf::sphere(1.0).smooth_union(Sdf::cuboid(Vec3::splat(0.7)), 0.2),
            Sdf::sphere(1.0).shell(0.25),
        ];
        for (k, s) in shapes.iter().enumerate() {
            let (lo, hi) = s.sampling_box();
            let m = polygonize(s, lo, hi, 24);
            let n = m.vertices.len() as u32;
            for (ti, t) in m.triangles.iter().enumerate() {
                // 範囲内。
                assert!(
                    t[0] < n && t[1] < n && t[2] < n,
                    "shape {k} tri {ti} index out of bounds: {t:?} (n={n})"
                );
                // 非退化 (3 頂点が相異)。from_soup の契約。
                assert!(
                    t[0] != t[1] && t[1] != t[2] && t[0] != t[2],
                    "shape {k} tri {ti} is degenerate (duplicate index): {t:?}"
                );
            }
        }
    }

    #[test]
    fn body_components_is_deterministic_under_repeated_calls() {
        // 問179: body_components は Union-Find 後の HashMap 集計を使う。
        // HashMap の反復順序は呼び出しごとに異なりうるが、**カウント** (bodies, cavities) は
        // 不変であることを複数回呼び出しで確認する。問5 の決定性の保証をテストで固定。
        let a = Sdf::sphere(0.5).translate(Vec3::new(-1.5, 0.0, 0.0));
        let b = Sdf::sphere(0.5).translate(Vec3::new(1.5, 0.0, 0.0));
        let model = a.union(b);
        let (lo, hi) = model.sampling_box();
        let m = polygonize(&model, lo, hi, 32);
        let first = m.body_components();
        for i in 1..=4 {
            let again = m.body_components();
            assert_eq!(
                first, again,
                "body_components call {i} must equal first call: {first:?} vs {again:?}"
            );
        }
    }

    #[test]
    fn edge_defects_single_triangle_has_three_boundary_edges() {
        // 問196: 三角形 1 枚のメッシュは辺が各 1 回しか現れない → boundary=3, nonmanifold=0。
        // from_soup_with_empty_input は空メッシュのみ、hollow_shell は多三角形のみ確認。
        // edge_defects は c==1 (境界) と c>2 (非多様体) を分けるが、
        // 最小ケース (三角形 1 枚) での境界辺カウントが未確認。
        let tri = Mesh::from_soup(&[[
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]]);
        assert_eq!(
            tri.triangles.len(),
            1,
            "single triangle soup must yield 1 triangle"
        );
        let (boundary, nonmanifold) = tri.edge_defects();
        assert_eq!(
            boundary, 3,
            "single triangle must have 3 boundary edges, got {boundary}"
        );
        assert_eq!(
            nonmanifold, 0,
            "single triangle has no shared edges → no non-manifold, got {nonmanifold}"
        );
        // is_edge_manifold は boundary==0 かつ nonmanifold==0 のときのみ true。
        // 単一三角形は boundary=3 なので is_edge_manifold は false (開境界あり)。
        assert!(
            !tri.is_edge_manifold(),
            "single open triangle must NOT be edge-manifold (has 3 boundary edges)"
        );
    }

    #[test]
    fn first_nonmanifold_edge_midpoint_is_deterministic_and_correct() {
        // 問257: 3枚の三角形がエッジ (v0,v1) を共有するメッシュで、
        // first_nonmanifold_edge_midpoint が最小インデックスのエッジ中点 (0.5,0,0) を返す。
        let mut mesh = Mesh::default();
        mesh.vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),  // v0
            Vec3::new(1.0, 0.0, 0.0),  // v1
            Vec3::new(0.0, 1.0, 0.0),  // v2
            Vec3::new(0.0, 0.0, 1.0),  // v3
            Vec3::new(0.0, -1.0, 0.0), // v4
        ];
        mesh.triangles = vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]];
        let mid = mesh
            .first_nonmanifold_edge_midpoint()
            .expect("3-shared-edge mesh must return Some");
        assert!(
            (mid.x - 0.5).abs() < 1e-9 && mid.y.abs() < 1e-9 && mid.z.abs() < 1e-9,
            "midpoint of edge (v0,v1) must be (0.5,0,0), got {:?}",
            mid
        );
        // 非多様体エッジがない正常なメッシュでは None。
        let clean = Mesh::from_soup(&[[
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]]);
        assert!(
            clean.first_nonmanifold_edge_midpoint().is_none(),
            "manifold mesh must return None"
        );
    }

    #[test]
    fn first_boundary_edge_midpoint_is_deterministic_and_correct() {
        // 問258: 単一三角形は3辺すべてが境界エッジ (1面のみ共有)。
        // 最小インデックスのエッジは (v0,v1) → 中点 (0.5,0,0)。
        let tri = Mesh::from_soup(&[[
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]]);
        let mid = tri
            .first_boundary_edge_midpoint()
            .expect("open triangle must return Some");
        assert!(
            (mid.x - 0.5).abs() < 1e-9 && mid.y.abs() < 1e-9 && mid.z.abs() < 1e-9,
            "midpoint of edge (v0,v1) must be (0.5,0,0), got {:?}",
            mid
        );
        // 開境界のない水密メッシュ (正四面体) では None。
        let tet = Mesh::from_soup(&[
            [
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, 1.0, -1.0),
            ],
            [
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(-1.0, -1.0, 1.0),
            ],
            [
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, -1.0),
            ],
            [
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, 1.0, -1.0),
            ],
        ]);
        assert!(
            tet.is_edge_manifold(),
            "regular tetrahedron soup must be watertight"
        );
        assert!(
            tet.first_boundary_edge_midpoint().is_none(),
            "watertight mesh must return None for boundary edge"
        );
    }
}
