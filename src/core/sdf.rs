//! SDF木 ([`Sdf`]) と解析的評価。
//!
//! 設計 (問1/問2/問11):
//! - 採用カーネルは案C (SDF/陰関数)。ブーリアンは min/max であり**場の代数として
//!   失敗しない**。出力メッシュの健全性は別途 extract 層で保証する (問11)。
//! - 木は enum で表現し、これは正本スクリプトの構文木の決定的射影に対応する (問2)。
//! - プリミティブ寸法は解析的に厳密。距離関数は符号付き距離 (内部負・外部正) を返す。
//!
//! 距離関数の式は Inigo Quilez の標準SDFに準拠。

use super::math::{clamp, mix, Vec3};

/// SDF木。各ノードは点 `p` における符号付き距離 (内部 < 0, 表面 = 0, 外部 > 0)
/// を [`Sdf::eval`] で返す。
#[derive(Clone, Debug, PartialEq)]
pub enum Sdf {
    // --- プリミティブ (原点中心, 軸整列) ---
    /// 半径 `radius` の球。
    Sphere { radius: f64 },
    /// 半幅 `half` の直方体 (中心原点)。各成分は辺長の半分。
    Cuboid { half: Vec3 },
    /// z軸まわりの円柱。`radius` 半径, `half_height` 高さの半分。
    Cylinder { radius: f64, half_height: f64 },

    // --- ブーリアン (場の代数。問11: 抽出健全性は別保証) ---
    /// 和 (min)。
    Union(Box<Sdf>, Box<Sdf>),
    /// 積 (max)。
    Intersection(Box<Sdf>, Box<Sdf>),
    /// 差 `a - b` (max(a, -b))。
    Difference(Box<Sdf>, Box<Sdf>),
    /// 多項式 smooth union。`k` はブレンド幅 (>0)。
    SmoothUnion(Box<Sdf>, Box<Sdf>, f64),

    // --- 変換 ---
    /// 平行移動。子を `offset` だけ移動する。
    Translate(Box<Sdf>, Vec3),
    /// 一様スケール。`factor` > 0。距離場を保つため `factor * child(p/factor)`。
    Scale(Box<Sdf>, f64),
}

impl Sdf {
    /// 点 `p` における符号付き距離を評価する。
    ///
    /// 決定的: f64・固定評価順序。同一バイナリ・同一arch内でバイト同一 (問5)。
    pub fn eval(&self, p: Vec3) -> f64 {
        match self {
            Sdf::Sphere { radius } => p.length() - radius,

            Sdf::Cuboid { half } => {
                // q = |p| - half;  外側距離 + 内側距離
                let q = p.abs() - *half;
                let outside = q.max_scalar(0.0).length();
                let inside = q.max_component().min(0.0);
                outside + inside
            }

            Sdf::Cylinder {
                radius,
                half_height,
            } => {
                // 半径方向と軸方向の2D距離。
                let radial = (p.x * p.x + p.y * p.y).sqrt() - radius;
                let axial = p.z.abs() - half_height;
                let outside = (radial.max(0.0).powi(2) + axial.max(0.0).powi(2)).sqrt();
                let inside = radial.max(axial).min(0.0);
                outside + inside
            }

            Sdf::Union(a, b) => a.eval(p).min(b.eval(p)),

            Sdf::Intersection(a, b) => a.eval(p).max(b.eval(p)),

            Sdf::Difference(a, b) => a.eval(p).max(-b.eval(p)),

            Sdf::SmoothUnion(a, b, k) => {
                let da = a.eval(p);
                let db = b.eval(p);
                // 多項式 smooth min (IQ)。
                let h = clamp(0.5 + 0.5 * (db - da) / k, 0.0, 1.0);
                mix(db, da, h) - k * h * (1.0 - h)
            }

            Sdf::Translate(child, offset) => child.eval(p - *offset),

            Sdf::Scale(child, factor) => factor * child.eval(p / *factor),
        }
    }

    // --- 構築ヘルパ (正本スクリプトの構文木構築に対応, 問2) ---

    pub fn sphere(radius: f64) -> Sdf {
        Sdf::Sphere { radius }
    }

    pub fn cuboid(half: Vec3) -> Sdf {
        Sdf::Cuboid { half }
    }

    pub fn cylinder(radius: f64, half_height: f64) -> Sdf {
        Sdf::Cylinder {
            radius,
            half_height,
        }
    }

    pub fn union(self, other: Sdf) -> Sdf {
        Sdf::Union(Box::new(self), Box::new(other))
    }

    pub fn intersection(self, other: Sdf) -> Sdf {
        Sdf::Intersection(Box::new(self), Box::new(other))
    }

    pub fn difference(self, other: Sdf) -> Sdf {
        Sdf::Difference(Box::new(self), Box::new(other))
    }

    pub fn smooth_union(self, other: Sdf, k: f64) -> Sdf {
        Sdf::SmoothUnion(Box::new(self), Box::new(other), k)
    }

    pub fn translate(self, offset: Vec3) -> Sdf {
        Sdf::Translate(Box::new(self), offset)
    }

    pub fn scale(self, factor: f64) -> Sdf {
        Sdf::Scale(Box::new(self), factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 決定的サンプリング格子: [-2,2]^3 を分割した点列。乱数を使わず再現可能。
    fn grid() -> Vec<Vec3> {
        let mut pts = Vec::new();
        let n = 13;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let f = |idx: usize| -2.0 + 4.0 * (idx as f64) / ((n - 1) as f64);
                    pts.push(Vec3::new(f(i), f(j), f(k)));
                }
            }
        }
        pts
    }

    const EPS: f64 = 1e-12;

    #[test]
    fn sphere_is_analytically_exact() {
        // プリミティブ寸法は解析的に厳密 (問1)。
        let s = Sdf::sphere(1.0);
        assert!((s.eval(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < EPS);
        assert!((s.eval(Vec3::ZERO) - (-1.0)).abs() < EPS);
        assert!(s.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS); // 表面
    }

    #[test]
    fn cuboid_surface_and_inside() {
        let c = Sdf::cuboid(Vec3::splat(1.0));
        assert!(c.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS); // 面上
        assert!((c.eval(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < EPS); // 外側
        assert!((c.eval(Vec3::ZERO) - (-1.0)).abs() < EPS); // 中心の内側距離
    }

    #[test]
    fn cylinder_radial_and_axial() {
        let cy = Sdf::cylinder(1.0, 2.0);
        assert!((cy.eval(Vec3::new(3.0, 0.0, 0.0)) - 2.0).abs() < EPS); // 半径方向外
        assert!((cy.eval(Vec3::new(0.0, 0.0, 4.0)) - 2.0).abs() < EPS); // 軸方向外
        assert!(cy.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS); // 側面上
    }

    #[test]
    fn union_equals_min_everywhere() {
        // 性質テスト: union ≡ min (問11: 場の代数)。
        let a = Sdf::sphere(1.0);
        let b = Sdf::cuboid(Vec3::splat(0.8)).translate(Vec3::new(0.5, 0.0, 0.0));
        let u = a.clone().union(b.clone());
        for p in grid() {
            let expect = a.eval(p).min(b.eval(p));
            assert!((u.eval(p) - expect).abs() < EPS, "at {:?}", p);
        }
    }

    #[test]
    fn intersection_and_difference_identities() {
        let a = Sdf::sphere(1.2);
        let b = Sdf::cuboid(Vec3::splat(1.0));
        let inter = a.clone().intersection(b.clone());
        let diff = a.clone().difference(b.clone());
        for p in grid() {
            assert!((inter.eval(p) - a.eval(p).max(b.eval(p))).abs() < EPS);
            assert!((diff.eval(p) - a.eval(p).max(-b.eval(p))).abs() < EPS);
        }
    }

    #[test]
    fn smooth_union_is_lower_bound_of_hard_union() {
        // smooth min ≤ hard min が全点で成り立つ。
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(1.5, 0.0, 0.0));
        let hard = a.clone().union(b.clone());
        let soft = a.clone().smooth_union(b.clone(), 0.3);
        for p in grid() {
            assert!(soft.eval(p) <= hard.eval(p) + EPS, "at {:?}", p);
        }
    }

    #[test]
    fn smooth_union_converges_to_hard_as_k_shrinks() {
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(1.5, 0.0, 0.0));
        let hard = a.clone().union(b.clone());
        let soft = a.clone().smooth_union(b.clone(), 1e-6);
        for p in grid() {
            assert!((soft.eval(p) - hard.eval(p)).abs() < 1e-4, "at {:?}", p);
        }
    }

    #[test]
    fn translate_shifts_field() {
        let s = Sdf::sphere(1.0);
        let t = Sdf::sphere(1.0).translate(Vec3::new(2.0, 0.0, 0.0));
        for p in grid() {
            assert!((t.eval(p) - s.eval(p - Vec3::new(2.0, 0.0, 0.0))).abs() < EPS);
        }
    }

    #[test]
    fn uniform_scale_preserves_distance_field() {
        // 一様スケール後も真の符号付き距離: 半径2の球と等価。
        let scaled = Sdf::sphere(1.0).scale(2.0);
        let direct = Sdf::sphere(2.0);
        for p in grid() {
            assert!((scaled.eval(p) - direct.eval(p)).abs() < EPS, "at {:?}", p);
        }
    }

    #[test]
    fn evaluation_is_deterministic() {
        // 同一入力 → バイト同一 (問5)。
        let tree = Sdf::sphere(1.0)
            .union(Sdf::cuboid(Vec3::splat(0.7)))
            .difference(Sdf::cylinder(0.3, 2.0))
            .translate(Vec3::new(0.1, -0.2, 0.3));
        for p in grid() {
            assert_eq!(tree.eval(p).to_bits(), tree.eval(p).to_bits());
        }
    }
}
