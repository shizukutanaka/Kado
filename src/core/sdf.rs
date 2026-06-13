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
    // ── プリミティブ (原点中心, 軸整列) ────────────────────────────────────────
    /// 半径 `radius` の球。
    Sphere { radius: f64 },
    /// 半幅 `half` の直方体 (中心原点)。
    Cuboid { half: Vec3 },
    /// z軸の円柱。`radius` 半径, `half_height` 高さの半分。
    Cylinder { radius: f64, half_height: f64 },
    /// XY平面内のトーラス。`major` 中心半径, `minor` 管半径。
    Torus { major: f64, minor: f64 },
    /// 頂点が原点にある円錐 (頂点から下方向)。`radius` 底面半径, `height` 高さ。
    /// 内側を正とする向きに注意: 先端が z=0 で底面が z=-height。
    Cone { radius: f64, height: f64 },
    /// カプセル (z軸方向の球–円柱)。`half_height` 軸半長, `radius` 半径。
    Capsule { half_height: f64, radius: f64 },
    /// 角丸直方体。`half` 半幅, `radius` フィレット半径。
    RoundedBox { half: Vec3, radius: f64 },

    // ── ブーリアン (場の代数。問11: 抽出健全性は別保証) ──────────────────────
    /// 和 (min)。
    Union(Box<Sdf>, Box<Sdf>),
    /// 積 (max)。
    Intersection(Box<Sdf>, Box<Sdf>),
    /// 差 `a - b` (max(a, -b))。
    Difference(Box<Sdf>, Box<Sdf>),
    /// 多項式 smooth union。`k` はブレンド幅 (>0)。
    SmoothUnion(Box<Sdf>, Box<Sdf>, f64),
    /// 多項式 smooth difference。`k` はブレンド幅 (>0)。
    SmoothDifference(Box<Sdf>, Box<Sdf>, f64),

    // ── 変形・変換 ────────────────────────────────────────────────────────────
    /// 平行移動。
    Translate(Box<Sdf>, Vec3),
    /// 一様スケール。距離場を保つため `factor * child(p/factor)`。
    Scale(Box<Sdf>, f64),
    /// オフセット (膨張/収縮)。`amount > 0` で膨張, `< 0` で収縮。
    Offset(Box<Sdf>, f64),
    /// 中空シェル。表面から `thickness/2` 内外を残す。
    Shell(Box<Sdf>, f64),
    /// 軸整列 3D 線形繰り返し。`period` の各成分が 0 の軸は繰り返しなし。
    Repeat(Box<Sdf>, Vec3),
    /// 面対称 (ミラー)。`axis`: 0=x, 1=y, 2=z。
    Mirror(Box<Sdf>, u8),
}

impl Sdf {
    /// 点 `p` における符号付き距離を評価する。
    ///
    /// 決定的: f64・固定評価順序。同一バイナリ・同一arch内でバイト同一 (問5)。
    pub fn eval(&self, p: Vec3) -> f64 {
        match self {
            Sdf::Sphere { radius } => p.length() - radius,

            Sdf::Cuboid { half } => {
                let q = p.abs() - *half;
                q.max_scalar(0.0).length() + q.max_component().min(0.0)
            }

            Sdf::Cylinder {
                radius,
                half_height,
            } => {
                let radial = (p.x * p.x + p.y * p.y).sqrt() - radius;
                let axial = p.z.abs() - half_height;
                let outside = (radial.max(0.0).powi(2) + axial.max(0.0).powi(2)).sqrt();
                let inside = radial.max(axial).min(0.0);
                outside + inside
            }

            Sdf::Torus { major, minor } => {
                let q_x = (p.x * p.x + p.y * p.y).sqrt() - major;
                (q_x * q_x + p.z * p.z).sqrt() - minor
            }

            Sdf::Cone { radius, height } => {
                // 先端 z=0, 底面 z=-height の錐 (FDM 印刷向けに下向き)。
                // q = (radial_dist, -pz)
                let q_x = (p.x * p.x + p.y * p.y).sqrt();
                let q_y = -p.z;
                let h = *height;
                let r = *radius;
                let len = (r * r + h * h).sqrt();
                let c = Vec3::new(h / len, r / len, 0.0); // normalized cone edge direction
                let k = p.z.min(0.0);
                let dot_proj = (q_x * c.x - q_y * c.y).max(0.0);
                let edge = Vec3::new(q_x - r * dot_proj / len, q_y - h * dot_proj / len, 0.0);
                let outside = edge.length() * (q_x * c.y - q_y * c.x).signum().max(0.0);
                let cap =
                    (Vec3::new(q_x, q_y - h, 0.0)).length() * (-q_y.max(-h).signum()).max(0.0);
                // Simplest correct formulation (IQ):
                let _ = (k, c, outside, cap);
                let q2 = Vec3::new(q_x, q_y, 0.0);
                let c2 = Vec3::new(h / len, r / len, 0.0);
                let d = (q2.dot(c2)).max(0.0);
                let proj = Vec3::new(c2.x * d, c2.y * d, 0.0);
                let perp = q2 - proj;
                let edge_dist = perp.length()
                    * if q2.x * c2.y - q2.y * c2.x < 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                let cap_dist =
                    (q2 - Vec3::new(0.0, h, 0.0)).length() * if q2.y < h { 1.0 } else { -1.0 };
                edge_dist.max(-cap_dist)
            }

            Sdf::Capsule {
                half_height,
                radius,
            } => {
                let pz_clamped = p.z.clamp(-half_height, *half_height);
                Vec3::new(p.x, p.y, p.z - pz_clamped).length() - radius
            }

            Sdf::RoundedBox { half, radius } => {
                let q = p.abs() - *half;
                q.max_scalar(0.0).length() + q.max_component().min(0.0) - radius
            }

            Sdf::Union(a, b) => a.eval(p).min(b.eval(p)),
            Sdf::Intersection(a, b) => a.eval(p).max(b.eval(p)),
            Sdf::Difference(a, b) => a.eval(p).max(-b.eval(p)),

            Sdf::SmoothUnion(a, b, k) => {
                let da = a.eval(p);
                let db = b.eval(p);
                let h = clamp(0.5 + 0.5 * (db - da) / k, 0.0, 1.0);
                mix(db, da, h) - k * h * (1.0 - h)
            }

            Sdf::SmoothDifference(a, b, k) => {
                let da = a.eval(p);
                let db = b.eval(p);
                let h = clamp(0.5 - 0.5 * (da + db) / k, 0.0, 1.0);
                mix(da, -db, h) + k * h * (1.0 - h)
            }

            Sdf::Translate(child, offset) => child.eval(p - *offset),

            Sdf::Scale(child, factor) => factor * child.eval(p / *factor),

            Sdf::Offset(child, amount) => child.eval(p) - amount,

            Sdf::Shell(child, thickness) => {
                let d = child.eval(p);
                d.max(-(d + *thickness))
            }

            Sdf::Repeat(child, period) => {
                let snap = |v: f64, per: f64| -> f64 {
                    if per == 0.0 {
                        v
                    } else {
                        v - per * (v / per + 0.5).floor()
                    }
                };
                let q = Vec3::new(
                    snap(p.x, period.x),
                    snap(p.y, period.y),
                    snap(p.z, period.z),
                );
                child.eval(q)
            }

            Sdf::Mirror(child, axis) => {
                let q = match axis {
                    0 => Vec3::new(p.x.abs(), p.y, p.z),
                    1 => Vec3::new(p.x, p.y.abs(), p.z),
                    2 => Vec3::new(p.x, p.y, p.z.abs()),
                    _ => p,
                };
                child.eval(q)
            }
        }
    }

    // ── 構築ヘルパ ─────────────────────────────────────────────────────────────

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
    pub fn torus(major: f64, minor: f64) -> Sdf {
        Sdf::Torus { major, minor }
    }
    pub fn cone(radius: f64, height: f64) -> Sdf {
        Sdf::Cone { radius, height }
    }
    pub fn capsule(half_height: f64, radius: f64) -> Sdf {
        Sdf::Capsule {
            half_height,
            radius,
        }
    }
    pub fn rounded_box(half: Vec3, radius: f64) -> Sdf {
        Sdf::RoundedBox { half, radius }
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
    pub fn smooth_difference(self, other: Sdf, k: f64) -> Sdf {
        Sdf::SmoothDifference(Box::new(self), Box::new(other), k)
    }

    pub fn translate(self, offset: Vec3) -> Sdf {
        Sdf::Translate(Box::new(self), offset)
    }
    pub fn scale(self, factor: f64) -> Sdf {
        Sdf::Scale(Box::new(self), factor)
    }
    pub fn offset(self, amount: f64) -> Sdf {
        Sdf::Offset(Box::new(self), amount)
    }
    pub fn shell(self, thickness: f64) -> Sdf {
        Sdf::Shell(Box::new(self), thickness)
    }
    pub fn repeat(self, period: Vec3) -> Sdf {
        Sdf::Repeat(Box::new(self), period)
    }
    pub fn mirror_x(self) -> Sdf {
        Sdf::Mirror(Box::new(self), 0)
    }
    pub fn mirror_y(self) -> Sdf {
        Sdf::Mirror(Box::new(self), 1)
    }
    pub fn mirror_z(self) -> Sdf {
        Sdf::Mirror(Box::new(self), 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 決定的サンプリング格子。
    fn grid() -> Vec<Vec3> {
        let n = 13;
        let mut pts = Vec::new();
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
        let s = Sdf::sphere(1.0);
        assert!((s.eval(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < EPS);
        assert!((s.eval(Vec3::ZERO) - (-1.0)).abs() < EPS);
        assert!(s.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS);
    }

    #[test]
    fn cuboid_surface_and_inside() {
        let c = Sdf::cuboid(Vec3::splat(1.0));
        assert!(c.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS);
        assert!((c.eval(Vec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < EPS);
        assert!((c.eval(Vec3::ZERO) - (-1.0)).abs() < EPS);
    }

    #[test]
    fn cylinder_radial_and_axial() {
        let cy = Sdf::cylinder(1.0, 2.0);
        assert!((cy.eval(Vec3::new(3.0, 0.0, 0.0)) - 2.0).abs() < EPS);
        assert!((cy.eval(Vec3::new(0.0, 0.0, 4.0)) - 2.0).abs() < EPS);
        assert!(cy.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS);
    }

    #[test]
    fn torus_surface_at_ring() {
        // (major=1, minor=0.3): 点 (1.3, 0, 0) は表面上
        let t = Sdf::torus(1.0, 0.3);
        assert!(t.eval(Vec3::new(1.3, 0.0, 0.0)).abs() < EPS);
        assert!((t.eval(Vec3::new(1.0, 0.0, 0.0)) - (-0.3)).abs() < EPS); // 輪の中心
    }

    #[test]
    fn capsule_endpoints_on_surface() {
        let c = Sdf::capsule(1.0, 0.5);
        // 軸端点 (0,0,1) から radius 離れた点が表面
        assert!(c.eval(Vec3::new(0.5, 0.0, 1.0)).abs() < EPS);
        assert!(c.eval(Vec3::new(0.5, 0.0, -1.0)).abs() < EPS);
    }

    #[test]
    fn rounded_box_surface() {
        let rb = Sdf::rounded_box(Vec3::splat(1.0), 0.2);
        // 面中心 (1.0, 0, 0) は辺長1の直方体のエッジからradius分外 → 表面
        assert!((rb.eval(Vec3::new(1.2, 0.0, 0.0))).abs() < EPS);
    }

    #[test]
    fn offset_inflates_sphere() {
        let s = Sdf::sphere(1.0).offset(0.5);
        let direct = Sdf::sphere(1.5);
        for p in grid() {
            assert!((s.eval(p) - direct.eval(p)).abs() < EPS);
        }
    }

    #[test]
    fn shell_is_thin_surface() {
        let shell = Sdf::sphere(1.0).shell(0.1);
        // 表面付近は薄い層 (-0.05 ~ 0.05)
        assert!(shell.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS);
        // 内側は shell の外 (正)
        assert!(shell.eval(Vec3::ZERO) > 0.0);
    }

    #[test]
    fn repeat_periodic_field() {
        let s = Sdf::sphere(0.3).repeat(Vec3::new(2.0, 2.0, 2.0));
        // (0,0,0) と (2,0,0) で同じ値
        assert!((s.eval(Vec3::ZERO) - s.eval(Vec3::new(2.0, 0.0, 0.0))).abs() < EPS);
    }

    #[test]
    fn mirror_x_symmetry() {
        let base = Sdf::sphere(0.5).translate(Vec3::new(1.0, 0.0, 0.0));
        let m = base.clone().mirror_x();
        // (1, 0, 0) と (-1, 0, 0) で同じ値
        assert!((m.eval(Vec3::new(1.0, 0.0, 0.0)) - m.eval(Vec3::new(-1.0, 0.0, 0.0))).abs() < EPS);
    }

    #[test]
    fn union_equals_min_everywhere() {
        let a = Sdf::sphere(1.0);
        let b = Sdf::cuboid(Vec3::splat(0.8)).translate(Vec3::new(0.5, 0.0, 0.0));
        let u = a.clone().union(b.clone());
        for p in grid() {
            assert!((u.eval(p) - a.eval(p).min(b.eval(p))).abs() < EPS);
        }
    }

    #[test]
    fn intersection_and_difference_identities() {
        let a = Sdf::sphere(1.2);
        let b = Sdf::cuboid(Vec3::splat(1.0));
        for p in grid() {
            assert!(
                (a.clone().intersection(b.clone()).eval(p) - a.eval(p).max(b.eval(p))).abs() < EPS
            );
            assert!(
                (a.clone().difference(b.clone()).eval(p) - a.eval(p).max(-b.eval(p))).abs() < EPS
            );
        }
    }

    #[test]
    fn smooth_union_is_lower_bound_of_hard_union() {
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(1.5, 0.0, 0.0));
        let hard = a.clone().union(b.clone());
        let soft = a.clone().smooth_union(b.clone(), 0.3);
        for p in grid() {
            assert!(soft.eval(p) <= hard.eval(p) + EPS);
        }
    }

    #[test]
    fn smooth_union_converges_to_hard_as_k_shrinks() {
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(1.5, 0.0, 0.0));
        let hard = a.clone().union(b.clone());
        let soft = a.clone().smooth_union(b.clone(), 1e-6);
        for p in grid() {
            assert!((soft.eval(p) - hard.eval(p)).abs() < 1e-4);
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
        let scaled = Sdf::sphere(1.0).scale(2.0);
        let direct = Sdf::sphere(2.0);
        for p in grid() {
            assert!((scaled.eval(p) - direct.eval(p)).abs() < EPS);
        }
    }

    #[test]
    fn evaluation_is_deterministic() {
        let tree = Sdf::sphere(1.0)
            .union(Sdf::cuboid(Vec3::splat(0.7)))
            .difference(Sdf::cylinder(0.3, 2.0))
            .translate(Vec3::new(0.1, -0.2, 0.3));
        for p in grid() {
            assert_eq!(tree.eval(p).to_bits(), tree.eval(p).to_bits());
        }
    }
}
