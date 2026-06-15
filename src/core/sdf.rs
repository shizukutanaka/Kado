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
    /// 楕円体。`radii` は各軸の半径 (半軸長, すべて > 0)。
    /// 内外の符号は厳密 (内部 (x/a)²+(y/b)²+(z/c)² < 1)、軸上の距離も厳密だが、
    /// 軸外の距離値は IQ 近似 (Lipschitz ≈ 1)。非一様スケールと違い距離場が壊れず
    /// 水密抽出を保てる (符号が厳密なため)。
    Ellipsoid { radii: Vec3 },

    // ── ブーリアン (場の代数。問11: 抽出健全性は別保証) ──────────────────────
    /// 和 (min)。
    Union(Box<Sdf>, Box<Sdf>),
    /// 積 (max)。
    Intersection(Box<Sdf>, Box<Sdf>),
    /// 差 `a - b` (max(a, -b))。
    Difference(Box<Sdf>, Box<Sdf>),
    /// 多項式 smooth union。`k` はブレンド幅 (>0)。
    SmoothUnion(Box<Sdf>, Box<Sdf>, f64),
    /// 多項式 smooth intersection。`k` はブレンド幅 (>0)。
    SmoothIntersection(Box<Sdf>, Box<Sdf>, f64),
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
    /// 軸整列 3D **有限**繰り返し。`period` の各成分が 0、または `count` が 0 の軸は
    /// 繰り返さない。`count[axis]` は原点の両側へのコピー数 (合計 `2*count+1` 個)。
    /// 無限格子は有限メッシュ化・有限BBox化できないため、必ず有限回数で囲う (問21)。
    Repeat(Box<Sdf>, Vec3, [u32; 3]),
    /// 面対称 (ミラー)。`axis`: 0=x, 1=y, 2=z。
    Mirror(Box<Sdf>, u8),
    /// 軸周り回転。`axis`: 0=x, 1=y, 2=z。`angle` はラジアン。
    /// 剛体変換ゆえ距離場は厳密に保たれる (スケール変化なし)。
    /// 決定性 (問5): sin/cos は同一バイナリ・同一arch内で確定的 (sqrt と同じ保証水準)。
    Rotate(Box<Sdf>, u8, f64),
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
                // 厳密な有限円錐 (IQ "Cone - exact")。
                // 先端 z=0, 底面 z=-height, 底面半径 radius (FDM 印刷向けに下向き)。
                let h = *height;
                let r = *radius;
                // (radial, axial) 平面での厳密距離。q は底面エッジ頂点 (r, -h)。
                let w = (p.x.hypot(p.y), p.z);
                let q = (r, -h);
                let dot_wq = w.0 * q.0 + w.1 * q.1;
                let dot_qq = q.0 * q.0 + q.1 * q.1;
                let t1 = clamp(dot_wq / dot_qq, 0.0, 1.0);
                let a = (w.0 - q.0 * t1, w.1 - q.1 * t1);
                let t2 = clamp(w.0 / q.0, 0.0, 1.0);
                let b = (w.0 - q.0 * t2, w.1 - q.1);
                let k = q.1.signum();
                let d = (a.0 * a.0 + a.1 * a.1).min(b.0 * b.0 + b.1 * b.1);
                let s = (k * (w.0 * q.1 - w.1 * q.0)).max(k * (w.1 - q.1));
                d.sqrt() * s.signum()
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

            Sdf::Ellipsoid { radii } => {
                // IQ 近似楕円体距離。符号と軸上距離は厳密、軸外は近似。
                let k0 = Vec3::new(p.x / radii.x, p.y / radii.y, p.z / radii.z).length();
                let k1 = Vec3::new(
                    p.x / (radii.x * radii.x),
                    p.y / (radii.y * radii.y),
                    p.z / (radii.z * radii.z),
                )
                .length();
                if k1 == 0.0 {
                    // 中心 (p=0)。最近接表面距離は最小半軸 (内部なので負)。
                    -radii.x.min(radii.y).min(radii.z)
                } else {
                    k0 * (k0 - 1.0) / k1
                }
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

            Sdf::SmoothIntersection(a, b, k) => {
                let da = a.eval(p);
                let db = b.eval(p);
                let h = clamp(0.5 - 0.5 * (db - da) / k, 0.0, 1.0);
                mix(db, da, h) + k * h * (1.0 - h)
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

            Sdf::Repeat(child, period, count) => {
                // 有限繰り返し (IQ opLimitedRepetition): セル番号を [-n, n] にクランプ。
                let snap = |v: f64, per: f64, n: u32| -> f64 {
                    if per == 0.0 || n == 0 {
                        v
                    } else {
                        let r = (v / per).round().clamp(-(n as f64), n as f64);
                        v - per * r
                    }
                };
                let q = Vec3::new(
                    snap(p.x, period.x, count[0]),
                    snap(p.y, period.y, count[1]),
                    snap(p.z, period.z, count[2]),
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

            Sdf::Rotate(child, axis, angle) => {
                // 形状を +angle 回した場合、点を -angle で逆回転してから子を評価する
                // (剛体・距離保存)。
                child.eval(rotate_point(p, *axis, -*angle))
            }
        }
    }

    // ── バウンディングボックス (問14: サンプリング領域を形状から導出) ────────────

    /// 保守的な軸整列バウンディングボックス `(min, max)` を解析的に推定する。
    ///
    /// メッシュ抽出のサンプリング領域を形状そのものから導くために使う。これにより
    /// 固定境界 (±2 など) によるスクリプト形状の暗黙クリッピングを防ぐ (問14)。
    /// `Repeat` は無限範囲なので 1 セル分の子ボックスで近似する (要注意ケース)。
    pub fn aabb(&self) -> (Vec3, Vec3) {
        match self {
            Sdf::Sphere { radius } => (Vec3::splat(-radius), Vec3::splat(*radius)),
            Sdf::Cuboid { half } => (-*half, *half),
            Sdf::Cylinder {
                radius,
                half_height,
            } => (
                Vec3::new(-radius, -radius, -half_height),
                Vec3::new(*radius, *radius, *half_height),
            ),
            Sdf::Torus { major, minor } => {
                let r = major + minor;
                (Vec3::new(-r, -r, -minor), Vec3::new(r, r, *minor))
            }
            Sdf::Cone { radius, height } => (
                Vec3::new(-radius, -radius, -height),
                Vec3::new(*radius, *radius, 0.0),
            ),
            Sdf::Capsule {
                half_height,
                radius,
            } => {
                let z = half_height + radius;
                (
                    Vec3::new(-radius, -radius, -z),
                    Vec3::new(*radius, *radius, z),
                )
            }
            Sdf::RoundedBox { half, radius } => {
                let e = *half + Vec3::splat(*radius);
                (-e, e)
            }
            Sdf::Ellipsoid { radii } => (-*radii, *radii),
            // 和: 子ボックスの和集合。smooth は k 分だけ膨らみうるので余裕を足す。
            Sdf::Union(a, b) => union_box(a.aabb(), b.aabb()),
            Sdf::SmoothUnion(a, b, k) => {
                let (lo, hi) = union_box(a.aabb(), b.aabb());
                (lo - Vec3::splat(*k), hi + Vec3::splat(*k))
            }
            // 積: 子ボックスの積集合 (重なり)。smooth は k 分の膨らみを許容。
            Sdf::Intersection(a, b) => {
                let (alo, ahi) = a.aabb();
                let (blo, bhi) = b.aabb();
                (alo.max(blo), ahi.min(bhi))
            }
            Sdf::SmoothIntersection(a, b, k) => {
                let (alo, ahi) = a.aabb();
                let (blo, bhi) = b.aabb();
                let e = Vec3::splat(*k);
                (alo.max(blo) - e, ahi.min(bhi) + e)
            }
            // 差 a-b ⊆ a。smooth は k 分の膨らみを許容。
            Sdf::Difference(a, _) => a.aabb(),
            Sdf::SmoothDifference(a, _, k) => {
                let (lo, hi) = a.aabb();
                (lo - Vec3::splat(*k), hi + Vec3::splat(*k))
            }
            Sdf::Translate(c, o) => {
                let (lo, hi) = c.aabb();
                (lo + *o, hi + *o)
            }
            Sdf::Scale(c, f) => {
                let (lo, hi) = c.aabb();
                (lo * *f, hi * *f) // factor > 0 前提 (距離場保存スケール)
            }
            Sdf::Offset(c, amount) => {
                let (lo, hi) = c.aabb();
                // 問84: 符号付き拡張。amount>0→膨張, amount<0→収縮。
                // max(0)クランプでは負 amount 時の AABB が子のまま (保守的すぎ)。
                // 過侵食 (lo2 > hi2) を正規化する。
                let e = Vec3::splat(*amount);
                let lo2 = lo - e;
                let hi2 = hi + e;
                (lo2.min(hi2), lo2.max(hi2))
            }
            Sdf::Shell(c, _) => c.aabb(),
            Sdf::Repeat(c, period, count) => {
                let (lo, hi) = c.aabb();
                let ext = Vec3::new(
                    count[0] as f64 * period.x.abs(),
                    count[1] as f64 * period.y.abs(),
                    count[2] as f64 * period.z.abs(),
                );
                (lo - ext, hi + ext)
            }
            Sdf::Mirror(c, axis) => mirror_box(c.aabb(), *axis),
            // 子 aabb の 8 隅を +angle で回し、その軸整列 bbox を取る (保守的)。
            Sdf::Rotate(c, axis, angle) => rotate_box(c.aabb(), *axis, *angle),
        }
    }

    /// 抽出用サンプリング境界。`aabb` に表面が境界で切れないよう余白を足す。
    ///
    /// AABB が反転 (lo > hi、例えば重なりのない SmoothIntersection) の場合は
    /// 空メッシュを生む最小ボックスを返す (問40)。`polygonize` が負ステップで
    /// 無駄な評価をするのを防ぐ。
    pub fn sampling_box(&self) -> (Vec3, Vec3) {
        let (lo, hi) = self.aabb();
        // 反転ボックスの正規化: Intersection 系で非重複の場合に発生する。
        let (lo, hi) = (lo.min(hi), lo.max(hi));
        let diag = (hi - lo).length();
        let m = (0.05 * diag).max(1e-3);
        let e = Vec3::splat(m);
        (lo - e, hi + e)
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
    /// 各軸の半径 `radii` を持つ楕円体。
    pub fn ellipsoid(radii: Vec3) -> Sdf {
        Sdf::Ellipsoid { radii }
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
    pub fn smooth_intersection(self, other: Sdf, k: f64) -> Sdf {
        Sdf::SmoothIntersection(Box::new(self), Box::new(other), k)
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
        Sdf::Repeat(Box::new(self), period, [1, 1, 1])
    }
    /// 各軸のコピー数を指定する有限繰り返し。`count[a]` は原点の両側へのコピー数。
    pub fn repeat_n(self, period: Vec3, count: [u32; 3]) -> Sdf {
        Sdf::Repeat(Box::new(self), period, count)
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
    /// x 軸周りに `angle` ラジアン回転。
    pub fn rotate_x(self, angle: f64) -> Sdf {
        Sdf::Rotate(Box::new(self), 0, angle)
    }
    /// y 軸周りに `angle` ラジアン回転。
    pub fn rotate_y(self, angle: f64) -> Sdf {
        Sdf::Rotate(Box::new(self), 1, angle)
    }
    /// z 軸周りに `angle` ラジアン回転。
    pub fn rotate_z(self, angle: f64) -> Sdf {
        Sdf::Rotate(Box::new(self), 2, angle)
    }
}

/// 点 `p` を指定軸周りに `angle` ラジアン回転する (右手系)。
/// 固定演算順序で記述し決定性を保つ (問5、FMA 不使用)。
fn rotate_point(p: Vec3, axis: u8, angle: f64) -> Vec3 {
    let s = angle.sin();
    let c = angle.cos();
    match axis {
        0 => Vec3::new(p.x, c * p.y - s * p.z, s * p.y + c * p.z),
        1 => Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z),
        2 => Vec3::new(c * p.x - s * p.y, s * p.x + c * p.y, p.z),
        _ => p,
    }
}

/// 軸整列ボックスの 8 隅を `angle` 回転し、その軸整列バウンディングボックスを返す。
fn rotate_box((lo, hi): (Vec3, Vec3), axis: u8, angle: f64) -> (Vec3, Vec3) {
    let corners = [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ];
    let mut mn = Vec3::splat(f64::INFINITY);
    let mut mx = Vec3::splat(f64::NEG_INFINITY);
    for corner in corners {
        let r = rotate_point(corner, axis, angle);
        mn = mn.min(r);
        mx = mx.max(r);
    }
    (mn, mx)
}

// ── AABB ヘルパ ────────────────────────────────────────────────────────────────

fn union_box(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> (Vec3, Vec3) {
    (a.0.min(b.0), a.1.max(b.1))
}

/// 指定軸について面対称化したボックス。対称面 (=0) の両側に広がる。
fn mirror_box((lo, hi): (Vec3, Vec3), axis: u8) -> (Vec3, Vec3) {
    let ext = |l: f64, h: f64| l.abs().max(h.abs());
    match axis {
        0 => {
            let e = ext(lo.x, hi.x);
            (Vec3::new(-e, lo.y, lo.z), Vec3::new(e, hi.y, hi.z))
        }
        1 => {
            let e = ext(lo.y, hi.y);
            (Vec3::new(lo.x, -e, lo.z), Vec3::new(hi.x, e, hi.z))
        }
        2 => {
            let e = ext(lo.z, hi.z);
            (Vec3::new(lo.x, lo.y, -e), Vec3::new(hi.x, hi.y, e))
        }
        _ => (lo, hi),
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
    fn ellipsoid_sign_and_axis_distances() {
        // 問53: 符号は厳密、軸上の距離は厳密。
        let e = Sdf::ellipsoid(Vec3::new(2.0, 1.0, 0.5));
        // 中心は内部 (負)。最小半軸 0.5 → 距離 -0.5。
        assert!((e.eval(Vec3::ZERO) - (-0.5)).abs() < EPS, "center: {}", e.eval(Vec3::ZERO));
        // 軸上の表面点は 0。
        assert!(e.eval(Vec3::new(2.0, 0.0, 0.0)).abs() < EPS, "x surface");
        assert!(e.eval(Vec3::new(0.0, 1.0, 0.0)).abs() < EPS, "y surface");
        assert!(e.eval(Vec3::new(0.0, 0.0, 0.5)).abs() < EPS, "z surface");
        // 軸上の外側距離は厳密 (x=3 → 距離 1)。
        assert!((e.eval(Vec3::new(3.0, 0.0, 0.0)) - 1.0).abs() < EPS, "x exterior");
        // 内側の符号。
        assert!(e.eval(Vec3::new(1.0, 0.0, 0.0)) < 0.0, "inside x");
        // 軸外の点でも符号は厳密: (1.5, 0.5, 0) は (1.5/2)²+(0.5/1)² = 0.5625+0.25 < 1 → 内部。
        assert!(e.eval(Vec3::new(1.5, 0.5, 0.0)) < 0.0, "off-axis inside");
        // (2, 0.9, 0): (1)²+(0.81) > 1 → 外部。
        assert!(e.eval(Vec3::new(2.0, 0.9, 0.0)) > 0.0, "off-axis outside");
    }

    #[test]
    fn ellipsoid_aabb_matches_radii() {
        let e = Sdf::ellipsoid(Vec3::new(2.0, 1.0, 0.5));
        let (lo, hi) = e.aabb();
        assert_eq!(hi, Vec3::new(2.0, 1.0, 0.5));
        assert_eq!(lo, Vec3::new(-2.0, -1.0, -0.5));
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
        // 既定 repeat は片側1コピー (合計3個/軸)。範囲内では周期的。
        let s = Sdf::sphere(0.3).repeat(Vec3::new(2.0, 2.0, 2.0));
        // (0,0,0) と (2,0,0) は同じセル像 → 同じ値
        assert!((s.eval(Vec3::ZERO) - s.eval(Vec3::new(2.0, 0.0, 0.0))).abs() < EPS);
    }

    #[test]
    fn repeat_is_bounded_not_infinite() {
        // 問21: 有限繰り返し。x軸 片側1 (=3コピー) のみ。
        let s = Sdf::sphere(0.3).repeat_n(Vec3::new(2.0, 2.0, 2.0), [1, 0, 0]);
        // 中心と隣接コピー中心 (±2) は球の内部 (負)。
        assert!(s.eval(Vec3::ZERO) < 0.0);
        assert!(s.eval(Vec3::new(2.0, 0.0, 0.0)) < 0.0);
        assert!(s.eval(Vec3::new(-2.0, 0.0, 0.0)) < 0.0);
        // 4セル目 (x=8) にはコピーが無い → 外側 (正)。無限繰り返しなら負になるはず。
        assert!(
            s.eval(Vec3::new(8.0, 0.0, 0.0)) > 0.0,
            "bounded repeat must not tile infinitely"
        );
        // y軸は count=0 なので繰り返さない。
        assert!(s.eval(Vec3::new(0.0, 2.0, 0.0)) > 0.0);
        // AABB は有限で、x方向に period*count だけ広がる。
        let (lo, hi) = s.aabb();
        assert!((hi.x - (0.3 + 2.0)).abs() < 1e-9, "hi.x={}", hi.x);
        assert!((lo.x + (0.3 + 2.0)).abs() < 1e-9, "lo.x={}", lo.x);
    }

    #[test]
    fn repeat_count_is_per_side_total_is_two_n_plus_one() {
        // 問96: count は「片側」のコピー数 → 1軸あたり合計 2*count+1 個。
        // nx=2, period 2 → x ∈ {-4,-2,0,2,4} の5コピー。x=6 (3個目/側) は存在しない。
        let s = Sdf::sphere(0.3).repeat_n(Vec3::new(2.0, 0.0, 0.0), [2, 0, 0]);
        // 2個目/側 (x=4) は内部 (コピーが存在)。
        assert!(
            s.eval(Vec3::new(4.0, 0.0, 0.0)) < 0.0,
            "2nd copy per side (x=4) must exist: {}",
            s.eval(Vec3::new(4.0, 0.0, 0.0))
        );
        // 3個目/側 (x=6) は存在しない → 外部。これが per-side=2 (合計5) の証拠。
        assert!(
            s.eval(Vec3::new(6.0, 0.0, 0.0)) > 0.0,
            "no 3rd copy per side (x=6) → count is per-side, total=2n+1: {}",
            s.eval(Vec3::new(6.0, 0.0, 0.0))
        );
        // AABB も両側に count*period = 4 広がる (hi.x = 0.3 + 4)。
        let (lo, hi) = s.aabb();
        assert!((hi.x - (0.3 + 4.0)).abs() < 1e-9, "hi.x={}", hi.x);
        assert!((lo.x + (0.3 + 4.0)).abs() < 1e-9, "lo.x={}", lo.x);
    }

    #[test]
    fn rotation_is_rigid_and_preserves_distance_field() {
        // 問51: 回転は剛体変換。回転形状を回転点で評価すると元の場と一致する。
        // S = rotate_z(child, θ) のとき S(R_θ x) == child(x) が任意 x で成り立つ。
        use std::f64::consts::FRAC_PI_3;
        let child = Sdf::cuboid(Vec3::new(1.0, 0.5, 0.3));
        let theta = FRAC_PI_3;
        let rotated = child.clone().rotate_z(theta);
        for p in grid() {
            // p を +θ 回した点で回転形状を評価 → 子の p での評価と一致。
            let rp = super::rotate_point(p, 2, theta);
            assert!(
                (rotated.eval(rp) - child.eval(p)).abs() < EPS,
                "rotation must preserve the distance field at {p:?}"
            );
        }
    }

    #[test]
    fn rotation_of_sphere_is_invariant() {
        // 球は回転不変。任意角でも同一の場。
        let s = Sdf::sphere(1.0);
        let r = Sdf::sphere(1.0).rotate_y(0.9);
        for p in grid() {
            assert!((s.eval(p) - r.eval(p)).abs() < EPS);
        }
    }

    #[test]
    fn rotate_z_90deg_swaps_extents_in_aabb() {
        // 問51: 細長い x 箱を z 周りに 90° 回すと aabb の x/y 範囲が入れ替わる。
        use std::f64::consts::FRAC_PI_2;
        let bar = Sdf::cuboid(Vec3::new(2.0, 0.5, 0.5));
        let rotated = bar.rotate_z(FRAC_PI_2);
        let (lo, hi) = rotated.aabb();
        // 回転後: x 半幅 ≈ 0.5, y 半幅 ≈ 2.0。
        assert!((hi.x - 0.5).abs() < 1e-9, "hi.x={}", hi.x);
        assert!((hi.y - 2.0).abs() < 1e-9, "hi.y={}", hi.y);
        assert!((lo.x + 0.5).abs() < 1e-9, "lo.x={}", lo.x);
        assert!((lo.y + 2.0).abs() < 1e-9, "lo.y={}", lo.y);
    }

    #[test]
    fn rotation_is_deterministic() {
        // 問5: sin/cos を含む回転も同一バイナリ内でビット決定的。
        let tree = Sdf::cylinder(0.4, 1.0).rotate_x(0.7).rotate_z(1.3);
        for p in grid() {
            assert_eq!(tree.eval(p).to_bits(), tree.eval(p).to_bits());
        }
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
    fn smooth_intersection_is_upper_bound_of_hard_intersection() {
        // smooth_intersection(a, b, k) >= intersection(a, b) everywhere (blend region "extends" into each shape).
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(0.5, 0.0, 0.0));
        let hard = a.clone().intersection(b.clone());
        let smooth = a.smooth_intersection(b, 0.3);
        for p in grid() {
            assert!(
                smooth.eval(p) >= hard.eval(p) - 1e-12,
                "smooth_intersection must be >= hard at {p:?}"
            );
        }
    }

    #[test]
    fn smooth_intersection_converges_to_hard_as_k_shrinks() {
        // smooth_intersection converges to hard intersection as k→0.
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(0.5, 0.0, 0.0));
        let hard = a.clone().intersection(b.clone());
        let tight = a.smooth_intersection(b, 1e-6);
        for p in grid() {
            assert!(
                (tight.eval(p) - hard.eval(p)).abs() < 1e-4,
                "smooth_intersection(k→0) must converge to hard at {p:?}"
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
    fn smooth_difference_is_upper_bound_of_hard_difference() {
        // smooth_difference(a, b, k) >= difference(a, b) everywhere (blend region extends).
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(0.6);
        let hard = a.clone().difference(b.clone());
        let soft = a.smooth_difference(b, 0.3);
        for p in grid() {
            assert!(
                soft.eval(p) >= hard.eval(p) - EPS,
                "smooth_diff must be >= hard at {p:?}: soft={} hard={}",
                soft.eval(p),
                hard.eval(p)
            );
        }
    }

    #[test]
    fn smooth_difference_converges_to_hard_as_k_shrinks() {
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(0.6);
        let hard = a.clone().difference(b.clone());
        let soft = a.smooth_difference(b, 1e-6);
        for p in grid() {
            assert!(
                (soft.eval(p) - hard.eval(p)).abs() < 1e-4,
                "smooth_diff(k→0) must converge to hard at {p:?}"
            );
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

    #[test]
    fn cone_surface_and_sign() {
        // 先端 z=0, 底面 z=-2, 底面半径 1。
        let c = Sdf::cone(1.0, 2.0);
        // 先端は表面 (点)。
        assert!(c.eval(Vec3::ZERO).abs() < EPS, "apex on surface");
        // 内部 (軸上, 中ほど) は負。
        assert!(c.eval(Vec3::new(0.0, 0.0, -1.0)) < 0.0, "interior negative");
        // 側面上の点: z=-1 では半径 0.5。(0.5, 0, -1) は母線上 → 表面。
        assert!(
            c.eval(Vec3::new(0.5, 0.0, -1.0)).abs() < EPS,
            "lateral surface zero: got {}",
            c.eval(Vec3::new(0.5, 0.0, -1.0))
        );
        // 底面ディスク内部の点 (0.4, 0, -2) は底面上 → 表面。
        assert!(
            c.eval(Vec3::new(0.4, 0.0, -2.0)).abs() < EPS,
            "base cap zero: got {}",
            c.eval(Vec3::new(0.4, 0.0, -2.0))
        );
        // 遠方は正。
        assert!(c.eval(Vec3::new(5.0, 0.0, -1.0)) > 0.0, "exterior positive");
    }

    #[test]
    fn sampling_box_is_never_inverted() {
        // 問40: 非重複 SmoothIntersection の AABB は反転する (lo > hi) が、
        // sampling_box は lo <= hi を保証し polygonize が安全に空メッシュを返す。
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(10.0, 0.0, 0.0));
        let si = a.smooth_intersection(b, 0.3);
        let (slo, shi) = si.sampling_box();
        assert!(
            slo.x <= shi.x && slo.y <= shi.y && slo.z <= shi.z,
            "sampling_box must never be inverted, got lo={slo:?} hi={shi:?}"
        );
        // 実際に空メッシュになることも確認。
        let mesh = crate::extract::polygonize(&si, slo, shi, 8);
        assert!(
            mesh.triangles.is_empty(),
            "non-overlapping smooth_intersection must produce empty mesh"
        );
    }

    #[test]
    fn aabb_encloses_surface_samples() {
        // 代表ツリーで AABB が表面サンプルを内包することを確認 (問14)。
        let tree = Sdf::sphere(1.0)
            .union(Sdf::cuboid(Vec3::splat(0.8)))
            .difference(Sdf::cylinder(0.3, 2.0))
            .translate(Vec3::new(0.5, -0.3, 0.2));
        let (lo, hi) = tree.aabb();
        // バウンディングボックスの外では SDF は厳密に正 (内包性)。
        let outside = [
            Vec3::new(lo.x - 0.5, 0.0, 0.0),
            Vec3::new(hi.x + 0.5, 0.0, 0.0),
            Vec3::new(0.0, lo.y - 0.5, 0.0),
            Vec3::new(0.0, hi.y + 0.5, 0.0),
            Vec3::new(0.0, 0.0, lo.z - 0.5),
            Vec3::new(0.0, 0.0, hi.z + 0.5),
        ];
        for p in outside {
            assert!(
                tree.eval(p) > 0.0,
                "point {p:?} outside aabb must be exterior"
            );
        }
        // sampling_box は aabb を内包する。
        let (slo, shi) = tree.sampling_box();
        assert!(slo.x <= lo.x && slo.y <= lo.y && slo.z <= lo.z);
        assert!(shi.x >= hi.x && shi.y >= hi.y && shi.z >= hi.z);
    }

    #[test]
    fn offset_negative_aabb_tightens_not_stays_at_child_size() {
        // 問84: offset(-r) は形状を収縮させる。
        // 修正前は AABB = child の AABB (保守的すぎ)。
        // 修正後は AABB が収縮した形状に合わせてタイトになる。
        let child = Sdf::sphere(1.0);
        let (child_lo, child_hi) = child.aabb();

        let shrunk = child.offset(-0.4);
        let (lo, hi) = shrunk.aabb();
        // 収縮後 AABB は子の AABB より小さくなければならない。
        assert!(
            lo.x > child_lo.x,
            "shrunk AABB lo.x must be > child lo.x: lo.x={}, child_lo.x={}",
            lo.x, child_lo.x
        );
        assert!(
            hi.x < child_hi.x,
            "shrunk AABB hi.x must be < child hi.x: hi.x={}, child_hi.x={}",
            hi.x, child_hi.x
        );
        // AABB は等方 (sphere): 各軸 ±(1.0 - 0.4) = ±0.6。
        assert!((lo.x - (-0.6)).abs() < 1e-12, "expected lo.x=-0.6, got {}", lo.x);
        assert!((hi.x - 0.6).abs() < 1e-12, "expected hi.x=0.6, got {}", hi.x);

        // AABB は依然 isosurface を内包する (表面点 (0.6,0,0) は AABB 内または境界)。
        let surface = Vec3::new(0.6, 0.0, 0.0);
        assert!(
            surface.x >= lo.x && surface.x <= hi.x,
            "surface point must be within AABB: {:?} in [{:?}, {:?}]",
            surface, lo, hi
        );

        // 過侵食 (amount > 子半径) では lo2 > hi2 → min/max で正規化されるため有限。
        let over_eroded = Sdf::sphere(1.0).offset(-1.5);
        let (elo, ehi) = over_eroded.aabb();
        assert!(elo.x.is_finite() && ehi.x.is_finite(), "over-eroded AABB must be finite");
        assert!(elo.x <= ehi.x, "over-eroded AABB must not be inverted: {elo:?} {ehi:?}");
    }
}
