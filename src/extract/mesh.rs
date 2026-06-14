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

    /// エッジ欠陥の内訳 `(開境界エッジ数, 非多様体接合エッジ数)`。
    ///
    /// - 開境界 (1面共有): 表面が閉じていない。サンプリング境界によるクリップや
    ///   ゼロ厚フィーチャが主因 (問25)。
    /// - 非多様体接合 (3面以上共有): 自己交差・座標一致による接合。
    ///
    /// 両者は是正策が異なる (前者=境界拡大、後者=解像度/形状分離) ため区別する。
    pub fn edge_defects(&self) -> (usize, usize) {
        let mut counts: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &self.triangles {
            for &(a, b) in &[(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let key = if a < b { (a, b) } else { (b, a) };
                *counts.entry(key).or_insert(0) += 1;
            }
        }
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
}
