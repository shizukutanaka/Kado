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
