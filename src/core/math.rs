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
