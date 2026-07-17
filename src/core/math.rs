//! 決定的な 3D ベクトル演算 (f64)。
//!
//! 決定性 (ADR-003 / 問5): すべて f64・演算順序を固定。`mul_add` (FMA) は
//! プラットフォーム差を生むため**意図的に使わず**、素朴な `a*b + c` で記述する。

/// 3次元ベクトル。SDF評価の座標・半径・半幅などに用いる。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }

    /// 全成分同値のベクトル。
    #[inline]
    pub const fn splat(v: f64) -> Self {
        Vec3 { x: v, y: v, z: v }
    }

    #[inline]
    pub fn dot(self, o: Vec3) -> f64 {
        // 固定順序の加算 (問5)。
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// ユークリッド長。
    #[inline]
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// 外積。固定演算順序 (問5)。
    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    /// 成分ごとの絶対値。
    #[inline]
    pub fn abs(self) -> Vec3 {
        Vec3::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// 成分ごとに `min` をとる。
    #[inline]
    pub fn min(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x.min(o.x), self.y.min(o.y), self.z.min(o.z))
    }

    /// 成分ごとに `max` をとる。
    #[inline]
    pub fn max(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x.max(o.x), self.y.max(o.y), self.z.max(o.z))
    }

    /// 成分ごとに下限でクランプ (max(self, lo))。
    #[inline]
    pub fn max_scalar(self, lo: f64) -> Vec3 {
        Vec3::new(self.x.max(lo), self.y.max(lo), self.z.max(lo))
    }

    /// 3成分の最大値。
    #[inline]
    pub fn max_component(self) -> f64 {
        self.x.max(self.y).max(self.z)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn mul(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Vec3;
    #[inline]
    fn div(self, s: f64) -> Vec3 {
        Vec3::new(self.x / s, self.y / s, self.z / s)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Vec3;
    #[inline]
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// スカラ線形補間 `a + (b-a)*t`。
#[inline]
pub fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// `[lo, hi]` へのクランプ。
#[inline]
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Vec3 定数 ──────────────────────────────────────────────────────────────

    #[test]
    fn zero_constant_is_all_zeros() {
        // 問119: Vec3::ZERO はあらゆる計算の基点として信頼される定数。
        assert_eq!(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(Vec3::ZERO.length(), 0.0);
    }

    #[test]
    fn splat_makes_all_components_equal() {
        let v = Vec3::splat(3.0);
        assert_eq!(v.x, 3.0);
        assert_eq!(v.y, 3.0);
        assert_eq!(v.z, 3.0);
    }

    // ── ドット積 ───────────────────────────────────────────────────────────────

    #[test]
    fn dot_self_equals_squared_length() {
        // dot(v,v) = |v|² は length² の定義。ピタゴラス triple (3,4,0) → 9+16 = 25。
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert_eq!(v.dot(v), 25.0);
        assert_eq!(v.length(), 5.0);
    }

    #[test]
    fn dot_is_commutative() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, -1.0, 2.0);
        assert_eq!(a.dot(b), b.dot(a));
    }

    #[test]
    fn dot_orthogonal_is_zero() {
        // X 軸と Y 軸は直交 → dot = 0。SDF の多くの判定は直交性に依存する。
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        assert_eq!(x.dot(y), 0.0);
        assert_eq!(y.dot(z), 0.0);
        assert_eq!(x.dot(z), 0.0);
    }

    // ── 外積 ──────────────────────────────────────────────────────────────────

    #[test]
    fn cross_gives_right_hand_rule() {
        // 問5 決定性: 固定演算順序。X × Y = +Z (右手系)。
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = Vec3::new(0.0, 0.0, 1.0);
        let r = x.cross(y);
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.z, 1.0, "X × Y must equal +Z (right-hand rule)");
        // Y × X = -Z (反可換性)。
        let r2 = y.cross(x);
        assert_eq!(r2.z, -1.0, "Y × X must equal -Z (anti-commutative)");
        // Y × Z = +X。
        assert_eq!(y.cross(z), x);
        // Z × X = +Y。
        assert_eq!(z.cross(x), y);
    }

    #[test]
    fn cross_self_is_zero() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let r = v.cross(v);
        assert_eq!(r, Vec3::ZERO, "v × v must be zero vector");
    }

    #[test]
    fn cross_length_equals_area_of_parallelogram() {
        // |a × b| = |a||b|sin θ。直角の場合 sin90°=1 なので |a||b|。
        let a = Vec3::new(3.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 4.0, 0.0);
        let area = a.cross(b).length();
        assert!(
            (area - 12.0).abs() < 1e-12,
            "area of 3×4 rectangle = 12, got {area}"
        );
    }

    // ── 算術演算子 ────────────────────────────────────────────────────────────

    #[test]
    fn add_sub_are_component_wise() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn mul_div_are_scalar_scale() {
        let v = Vec3::new(2.0, 4.0, 6.0);
        assert_eq!(v * 2.0, Vec3::new(4.0, 8.0, 12.0));
        assert_eq!(v / 2.0, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn neg_flips_all_components() {
        let v = Vec3::new(1.0, -2.0, 3.0);
        assert_eq!(-v, Vec3::new(-1.0, 2.0, -3.0));
    }

    // ── 成分別操作 ────────────────────────────────────────────────────────────

    #[test]
    fn abs_takes_absolute_value_per_component() {
        let v = Vec3::new(-3.0, 0.0, 2.0);
        assert_eq!(v.abs(), Vec3::new(3.0, 0.0, 2.0));
    }

    #[test]
    fn min_max_are_component_wise() {
        let a = Vec3::new(1.0, 5.0, 2.0);
        let b = Vec3::new(3.0, 2.0, 4.0);
        assert_eq!(a.min(b), Vec3::new(1.0, 2.0, 2.0));
        assert_eq!(a.max(b), Vec3::new(3.0, 5.0, 4.0));
    }

    #[test]
    fn max_scalar_clamps_below_floor() {
        let v = Vec3::new(-1.0, 0.0, 3.0);
        assert_eq!(v.max_scalar(0.5), Vec3::new(0.5, 0.5, 3.0));
    }

    #[test]
    fn max_component_returns_largest() {
        assert_eq!(Vec3::new(1.0, 5.0, 3.0).max_component(), 5.0);
        assert_eq!(Vec3::new(-3.0, -1.0, -2.0).max_component(), -1.0);
    }

    // ── スカラヘルパ ──────────────────────────────────────────────────────────

    #[test]
    fn mix_interpolates_linearly() {
        // t=0 → a, t=1 → b, t=0.5 → midpoint。
        assert_eq!(mix(2.0, 8.0, 0.0), 2.0);
        assert_eq!(mix(2.0, 8.0, 1.0), 8.0);
        assert_eq!(mix(2.0, 8.0, 0.5), 5.0);
        // 外挿 (t > 1) も数式どおり動くことを確認 (クランプなし)。
        assert_eq!(mix(0.0, 1.0, 2.0), 2.0);
    }

    #[test]
    fn clamp_is_bounded_to_lo_hi() {
        assert_eq!(clamp(0.5, 0.0, 1.0), 0.5, "in-range");
        assert_eq!(clamp(-1.0, 0.0, 1.0), 0.0, "below lo");
        assert_eq!(clamp(2.0, 0.0, 1.0), 1.0, "above hi");
        assert_eq!(clamp(0.0, 0.0, 1.0), 0.0, "at lo");
        assert_eq!(clamp(1.0, 0.0, 1.0), 1.0, "at hi");
    }

    #[test]
    fn cross_collinear_vectors_yield_zero() {
        // 問156: cross_self_is_zero は v×v のみ確認。v×(kv) (k≠1) も平行なのでゼロになる。
        // stl.rs の face_normal がこの性質で退化三角形を検出するため固定する。
        let v = Vec3::new(3.0, 4.0, 0.0);
        // 正方向スケール。
        assert_eq!(v.cross(v * 2.0), Vec3::ZERO, "v × 2v must be zero");
        // 負方向スケール (逆平行)。
        assert_eq!(v.cross(v * -1.0), Vec3::ZERO, "v × -v must be zero");
        // 任意の非ゼロスケール。
        assert_eq!(v.cross(v * 0.001), Vec3::ZERO, "v × 0.001v must be zero");
    }
}
