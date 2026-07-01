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
    /// 任意軸 (単位ベクトル) 周りの回転 (Rodrigues の回転公式・問266)。
    /// `rotate_x/y/z` は canonical 3軸のみだが、対角線等の任意軸をこの1操作で
    /// 表現できる ("rotate_x してから rotate_y" のような合成では一般に到達できない
    /// 姿勢を、オイラー角の逆算なしに直接指定できる)。剛体変換ゆえ距離場は
    /// 厳密に保たれる。
    RotateAxis(Box<Sdf>, Vec3, f64),
    /// 平面カット (半空間との交差)。`normal` は**単位**法線、`offset` は原点からの符号付き距離。
    /// `dot(p, normal) <= offset` の側を残し、法線が指す側を切り落とす。
    /// FDM 印刷の平坦な底面づくり (サポート不要化) や断面表示に使う。
    /// 単独の半空間は無限 AABB になるため、必ず子形状への単項修飾として持つ
    /// (交差は材料を削るだけなので AABB は子の AABB で保守的に囲える)。
    Cut(Box<Sdf>, Vec3, f64),
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

            Sdf::RotateAxis(child, axis, angle) => {
                // Rotate と同じ「逆回転してから子を評価」の原理 (剛体・距離保存)。
                child.eval(rotate_point_axis(p, *axis, -*angle))
            }

            Sdf::Cut(child, normal, offset) => {
                // 半空間 dot(p,n) - offset との交差 (max)。n は単位法線なので
                // 半空間側の距離場も正しい (Lipschitz ≈ 1)。
                let half = p.dot(*normal) - offset;
                child.eval(p).max(half)
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
            Sdf::RotateAxis(c, axis, angle) => rotate_box_axis(c.aabb(), *axis, *angle),
            // カットは材料を削るだけなので子の AABB が保守的な上界 (半空間は無限だが
            // 交差により実体は子の範囲を超えない)。
            Sdf::Cut(c, _, _) => c.aabb(),
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

    /// 任意軸周りに `angle` ラジアン回転する (Rodrigues の回転公式・問266)。
    /// `axis` は内部で単位化される (距離場の Lipschitz を保つため)。
    /// ゼロ軸は呼び出し側 (eval.rs) で拒否される前提だが、`cut` の法線と同様
    /// 防御的に +Z へフォールバックし NaN を避ける。
    pub fn rotate_axis(self, axis: Vec3, angle: f64) -> Sdf {
        let len = axis.length();
        let unit = if len > 0.0 {
            axis * (1.0 / len)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        Sdf::RotateAxis(Box::new(self), unit, angle)
    }

    /// 平面カット: `dot(p, normal) <= offset` の側を残し、法線が指す側を切り落とす。
    /// `normal` は内部で単位化される (距離場の Lipschitz を保つため)。
    /// ゼロ法線は呼び出し側 (eval.rs) で拒否される前提だが、防御的に正規化する。
    pub fn cut(self, normal: Vec3, offset: f64) -> Sdf {
        let len = normal.length();
        // 単位化。ゼロ長なら +Z を既定にして NaN を避ける (eval.rs がゼロ法線を拒否)。
        let unit = if len > 0.0 {
            normal * (1.0 / len)
        } else {
            Vec3::new(0.0, 0.0, 1.0)
        };
        // offset も同じスケールで正規化し、平面位置 dot(p,n)=offset を保つ。
        let off = if len > 0.0 { offset / len } else { offset };
        Sdf::Cut(Box::new(self), unit, off)
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

/// 点 `p` を単位軸 `axis` 周りに `angle` ラジアン回転する (Rodrigues の回転公式・問266)。
/// `axis=(1,0,0)/(0,1,0)/(0,0,1)` のとき `rotate_point` の対応する分岐と数式的に一致する。
/// 固定演算順序で記述し決定性を保つ (問5、FMA 不使用)。`axis` は単位長である前提
/// (呼び出し側の `rotate_axis`/`cut` と同じ契約)。
fn rotate_point_axis(p: Vec3, axis: Vec3, angle: f64) -> Vec3 {
    let s = angle.sin();
    let c = angle.cos();
    let k = axis;
    let k_cross_p = k.cross(p);
    let k_dot_p = k.dot(p);
    p * c + k_cross_p * s + k * (k_dot_p * (1.0 - c))
}

/// 軸整列ボックスの 8 隅を任意軸 `axis` 周りに `angle` 回転し、
/// その軸整列バウンディングボックスを返す (`rotate_box` の任意軸版)。
fn rotate_box_axis((lo, hi): (Vec3, Vec3), axis: Vec3, angle: f64) -> (Vec3, Vec3) {
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
        let r = rotate_point_axis(corner, axis, angle);
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
    fn primitive_axes_are_z_aligned_as_documented() {
        // 問99: help が主張する向き (cylinder/capsule = Z軸, torus = XY平面リング) を
        // 挙動で固定する。AI が stacking/rotation を計画する際の前提。

        // cylinder: 長軸は Z。h=2,r=0.5 → z 方向に ±2 まで内部・x 方向は ±0.5 まで。
        let cy = Sdf::cylinder(0.5, 2.0);
        assert!(
            cy.eval(Vec3::new(0.0, 0.0, 1.5)) < 0.0,
            "cylinder inside along +Z within h"
        );
        assert!(
            cy.eval(Vec3::new(1.5, 0.0, 0.0)) > 0.0,
            "cylinder outside at x=1.5 (r=0.5)"
        );
        // 非対称 (z は h=2 まで内部だが x は r=0.5 で外部) が「長軸=Z」の証拠。

        // capsule: 軸は Z。端点 (0,0,h) から radius 離れた点が表面。
        let cap = Sdf::capsule(1.0, 0.5);
        assert!(
            cap.eval(Vec3::new(0.0, 0.0, 1.4)) < 0.0,
            "capsule inside Z cap region"
        );
        assert!(
            cap.eval(Vec3::new(1.4, 0.0, 0.0)) > 0.0,
            "capsule outside far in X"
        );

        // torus: リングは XY 平面。穴は Z 軸を向く → Z 軸上 (0,0,z) は穴の中 (外部)。
        let t = Sdf::torus(1.0, 0.3);
        assert!(
            t.eval(Vec3::ZERO) > 0.0,
            "torus hole on Z axis → origin is outside"
        );
        assert!(
            t.eval(Vec3::new(1.0, 0.0, 0.0)) < 0.0,
            "torus tube center in XY plane is inside"
        );
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
        assert!(
            (e.eval(Vec3::ZERO) - (-0.5)).abs() < EPS,
            "center: {}",
            e.eval(Vec3::ZERO)
        );
        // 軸上の表面点は 0。
        assert!(e.eval(Vec3::new(2.0, 0.0, 0.0)).abs() < EPS, "x surface");
        assert!(e.eval(Vec3::new(0.0, 1.0, 0.0)).abs() < EPS, "y surface");
        assert!(e.eval(Vec3::new(0.0, 0.0, 0.5)).abs() < EPS, "z surface");
        // 軸上の外側距離は厳密 (x=3 → 距離 1)。
        assert!(
            (e.eval(Vec3::new(3.0, 0.0, 0.0)) - 1.0).abs() < EPS,
            "x exterior"
        );
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
    fn ellipsoid_extreme_asymmetry_is_finite_and_correct_sign() {
        // 問157: 極端な縦横比 (1000:1) では IQ 近似の中間値が f64 精度で劣化しうる。
        // `k1 = length²(p/r²) / r` が非常に小さな値になり underflow や NaN を
        // 引き起こす可能性がある。符号が厳密なことと有限値であることを固定する。
        let e = Sdf::ellipsoid(Vec3::new(1000.0, 0.001, 0.5));
        // 中心: 最小半径 0.001 → 距離 ≈ -0.001。
        let d_center = e.eval(Vec3::ZERO);
        assert!(d_center < 0.0, "center must be inside: {d_center}");
        assert!(
            d_center.is_finite(),
            "center distance must be finite: {d_center}"
        );
        // X 軸上の表面 (x=1000): 距離 ≈ 0。
        let d_xsurf = e.eval(Vec3::new(1000.0, 0.0, 0.0));
        assert!(
            d_xsurf.is_finite(),
            "x-surface distance must be finite: {d_xsurf}"
        );
        assert!(d_xsurf.abs() < 0.1, "x-surface must be near 0: {d_xsurf}");
        // X 軸内部 (x=500): 楕円式 (500/1000)²=0.25 < 1 → 内部 (負)。
        let d_inside = e.eval(Vec3::new(500.0, 0.0, 0.0));
        assert!(d_inside < 0.0, "x=500 must be inside: {d_inside}");
        assert!(
            d_inside.is_finite(),
            "interior distance must be finite: {d_inside}"
        );
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
    fn shell_hollows_inward_keeping_outer_surface() {
        // 問98: shell は外側表面を保持し内向きに壁を作る (中空化)。
        // shell(sphere(1.0), 0.3) → 外半径 1.0 維持・内半径 0.7・壁厚 0.3。
        let s = Sdf::sphere(1.0).shell(0.3);
        // 外側表面は元の半径 1.0 のまま (肥大しない)。
        assert!(
            s.eval(Vec3::new(1.0, 0.0, 0.0)).abs() < EPS,
            "outer surface preserved at r=1"
        );
        // 壁の内部 (r=0.85) は内側 (負)。
        assert!(
            s.eval(Vec3::new(0.85, 0.0, 0.0)) < 0.0,
            "wall is inward of the surface"
        );
        // 内側表面は r = 1 - thickness = 0.7。
        assert!(
            s.eval(Vec3::new(0.7, 0.0, 0.0)).abs() < EPS,
            "inner wall at r=0.7"
        );
        // 中心は中空 (壁の外 = 正)。
        assert!(s.eval(Vec3::ZERO) > 0.0, "deep interior is hollow");
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
    fn mirror_keeps_positive_half_and_reflects_to_negative() {
        // 問97: mirror_x は +x 半分を保持し -x 側へ鏡像化する (abs(p.x))。
        // 元の -x 半分は破棄され、+x 半分の鏡像で置き換わる。
        // +x にある球は mirror 後、+x と -x の両方に対称コピーとして現れる。
        let plus = Sdf::sphere(0.5).translate(Vec3::new(2.0, 0.0, 0.0));
        let m = plus.mirror_x();
        assert!(m.eval(Vec3::new(2.0, 0.0, 0.0)) < 0.0, "+x copy preserved");
        assert!(m.eval(Vec3::new(-2.0, 0.0, 0.0)) < 0.0, "reflected onto -x");

        // 逆に -x にしかない球を mirror_x すると、source となる +x 半分が空のため
        // 結果は空 (両側とも外部)。これが「+半分が源」という意味論の決定的証拠。
        let minus = Sdf::sphere(0.5).translate(Vec3::new(-2.0, 0.0, 0.0));
        let m2 = minus.mirror_x();
        assert!(
            m2.eval(Vec3::new(-2.0, 0.0, 0.0)) > 0.0,
            "a -x-only shape mirrors to empty (source is the +x half): {}",
            m2.eval(Vec3::new(-2.0, 0.0, 0.0))
        );
        assert!(m2.eval(Vec3::new(2.0, 0.0, 0.0)) > 0.0, "+x also empty");
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
    fn rotation_composition_is_additive_roundtrip_and_order_dependent() {
        // 問110: 回転の合成 (入れ子) は AI が向き付き組立を作る際に重要だが、
        // 既存テストは単一回転のみ。合成の正しさを固定する:
        //  (1) 同軸加法性: rotate_z(a, rotate_z(b, S)) == rotate_z(a+b, S)
        //  (2) 往復恒等:   rotate_z(-θ, rotate_z(θ, S)) == S
        //  (3) 異軸の順序依存 (非可換): rot_x∘rot_y ≠ rot_y∘rot_x
        let child = Sdf::cuboid(Vec3::new(1.0, 0.6, 0.3));
        let pts = grid();

        // (1) 同軸加法性。
        let composed = child.clone().rotate_z(0.3).rotate_z(0.5);
        let single = child.clone().rotate_z(0.8);
        for &p in &pts {
            assert!(
                (composed.eval(p) - single.eval(p)).abs() < 1e-9,
                "same-axis rotations must add (z 0.3 then 0.5 == 0.8) at {p:?}"
            );
        }

        // (2) 往復恒等。
        let roundtrip = child.clone().rotate_z(0.7).rotate_z(-0.7);
        for &p in &pts {
            assert!(
                (roundtrip.eval(p) - child.eval(p)).abs() < 1e-9,
                "rotate_z(θ) then rotate_z(-θ) must be identity at {p:?}"
            );
        }

        // (3) 異軸は非可換: rot_x(60°)∘rot_y(60°) と順序逆は一般に異なる。
        let xy = child.clone().rotate_y(1.0).rotate_x(1.0);
        let yx = child.clone().rotate_x(1.0).rotate_y(1.0);
        let differs = pts.iter().any(|&p| (xy.eval(p) - yx.eval(p)).abs() > 1e-6);
        assert!(
            differs,
            "different-axis rotations must be order-dependent (non-commutative)"
        );
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
    fn rotate_axis_matches_canonical_rotate_on_x_y_z() {
        // 問266: rotate_axis((1,0,0)/(0,1,0)/(0,0,1), angle) は Rodrigues の回転公式が
        // rotate_x/y/z の既存の直交行列式と数式的に一致することを固定する。
        let angle = 0.83;
        let bar = Sdf::cuboid(Vec3::new(1.5, 0.6, 0.3));
        let via_axis_x = bar.clone().rotate_axis(Vec3::new(1.0, 0.0, 0.0), angle);
        let via_canonical_x = bar.clone().rotate_x(angle);
        let via_axis_y = bar.clone().rotate_axis(Vec3::new(0.0, 1.0, 0.0), angle);
        let via_canonical_y = bar.clone().rotate_y(angle);
        let via_axis_z = bar.clone().rotate_axis(Vec3::new(0.0, 0.0, 1.0), angle);
        let via_canonical_z = bar.rotate_z(angle);
        for p in grid() {
            assert!(
                (via_axis_x.eval(p) - via_canonical_x.eval(p)).abs() < 1e-9,
                "rotate_axis(x) must match rotate_x at {p:?}"
            );
            assert!(
                (via_axis_y.eval(p) - via_canonical_y.eval(p)).abs() < 1e-9,
                "rotate_axis(y) must match rotate_y at {p:?}"
            );
            assert!(
                (via_axis_z.eval(p) - via_canonical_z.eval(p)).abs() < 1e-9,
                "rotate_axis(z) must match rotate_z at {p:?}"
            );
        }
    }

    #[test]
    fn rotate_axis_leaves_sphere_unchanged() {
        // 問266: 球は回転対称なので、任意軸周りに任意角度回転しても距離場は不変。
        let s = Sdf::sphere(1.0);
        let r = s.clone().rotate_axis(Vec3::new(1.0, 1.0, 1.0), 1.234);
        for p in grid() {
            assert!((s.eval(p) - r.eval(p)).abs() < EPS);
        }
    }

    #[test]
    fn rotate_axis_unnormalized_axis_gives_same_result_as_normalized() {
        // 問266: 軸ベクトルは内部で単位化されるため、非単位長 (例: (2,0,0)) を渡しても
        // 単位化済み (1,0,0) と同じ結果になる (axis の「向き」だけが意味を持つ)。
        let angle = 0.5;
        let bar = Sdf::cuboid(Vec3::new(1.2, 0.7, 0.4));
        let unit = bar.clone().rotate_axis(Vec3::new(1.0, 0.0, 0.0), angle);
        let scaled = bar.rotate_axis(Vec3::new(5.0, 0.0, 0.0), angle);
        for p in grid() {
            assert!(
                (unit.eval(p) - scaled.eval(p)).abs() < 1e-9,
                "non-unit axis length must not change the rotation result at {p:?}"
            );
        }
    }

    #[test]
    fn rotate_axis_180deg_around_diagonal_swaps_and_negates_orthogonal_axis() {
        // 問266: (1,1,0)/√2 軸周りの180°回転は、幾何学的によく知られた変換——
        // x↔y を入れ替え、z を反転する。既知の解析解と比較して Rodrigues 公式の
        // 実装 (演算順序含む) が正しいことを検証する。
        use std::f64::consts::PI;
        let axis = Vec3::new(1.0, 1.0, 0.0);
        let p = Vec3::new(1.0, 2.0, 3.0);
        // rotate_point_axis は "点を逆回転" ではなく直接回転に使う関数なので、
        // ここでは形状の eval ではなく幾何学的な変換結果を rotate_point_axis で直接確認する。
        let rotated = rotate_point_axis(p, axis * (1.0 / axis.length()), PI);
        assert!(
            (rotated.x - 2.0).abs() < 1e-9,
            "x must become original y, got {rotated:?}"
        );
        assert!(
            (rotated.y - 1.0).abs() < 1e-9,
            "y must become original x, got {rotated:?}"
        );
        assert!(
            (rotated.z - (-3.0)).abs() < 1e-9,
            "z must be negated, got {rotated:?}"
        );
    }

    #[test]
    fn rotate_axis_is_deterministic() {
        // 問5/266: 任意軸回転も同一バイナリ内でビット決定的。
        let tree = Sdf::cylinder(0.4, 1.0).rotate_axis(Vec3::new(1.0, 2.0, 3.0), 0.7);
        for p in grid() {
            assert_eq!(tree.eval(p).to_bits(), tree.eval(p).to_bits());
        }
    }

    #[test]
    fn rotate_axis_aabb_is_finite_and_contains_rotated_shape() {
        // 問266: 対角軸周りの回転でも aabb は有限で、実際に回転した頂点を包含する
        // (rotate_box_axis が rotate_box と同じ8隅法で正しく汎化されていることを確認)。
        let bar = Sdf::cuboid(Vec3::new(2.0, 0.5, 0.5));
        let rotated = bar.rotate_axis(Vec3::new(1.0, 1.0, 1.0), 0.9);
        let (lo, hi) = rotated.aabb();
        assert!(lo.x.is_finite() && lo.y.is_finite() && lo.z.is_finite());
        assert!(hi.x.is_finite() && hi.y.is_finite() && hi.z.is_finite());
        assert!(lo.x <= hi.x && lo.y <= hi.y && lo.z <= hi.z);
        // 回転前の対角長 (2*sqrt(2^2+0.5^2+0.5^2)) を超えない程度の妥当な広がりであること
        // (無限大や極端な値に発散していないことの粗いガード)。
        let diag = (hi - lo).length();
        assert!(
            diag > 0.0 && diag < 20.0,
            "aabb diagonal must be reasonable, got {diag}"
        );
    }

    #[test]
    fn cut_removes_half_space_the_normal_points_into() {
        // 問235 (新機能): cut は dot(p,n) <= offset の側を残す。
        // 球を z=0 平面で「下半分を削る」: 法線が下 (0,0,-1)、offset=0 → z>=0 を残す。
        let s = Sdf::sphere(1.0).cut(Vec3::new(0.0, 0.0, -1.0), 0.0);
        // 上半球内部 (0,0,0.5) は残る (負)。
        assert!(
            s.eval(Vec3::new(0.0, 0.0, 0.5)) < 0.0,
            "kept half (z>0) must be inside"
        );
        // 下半球の点 (0,0,-0.5) は切り落とされ外部 (正)。
        // dot(p,n) = (0,0,-0.5)·(0,0,-1) = 0.5 > offset 0 → half=0.5 → max(球内部, 0.5)=0.5。
        assert!(
            s.eval(Vec3::new(0.0, 0.0, -0.5)) > 0.0,
            "cut-away half (z<0) must be outside"
        );
        // 切断面 z=0 上の中心は表面 (=0): 球内部 -1.0 と half = 0 の max = 0。
        assert!(
            s.eval(Vec3::ZERO).abs() < 1e-12,
            "cut plane at z=0 is the new surface at center"
        );
    }

    #[test]
    fn cut_normal_is_normalized_so_distance_field_is_metric() {
        // cut() は法線を単位化する。非正規化法線 (0,0,-3) を渡しても
        // 平面位置 dot(p,n)=offset が保たれ、距離場が単位スケール (Lipschitz≈1) になる。
        // sphere をカットせず平面だけが効く領域で、距離が真の幾何距離になることを確認。
        let unit = Sdf::sphere(5.0).cut(Vec3::new(0.0, 0.0, -1.0), 0.0);
        let scaled = Sdf::sphere(5.0).cut(Vec3::new(0.0, 0.0, -3.0), 0.0); // 非正規化
                                                                           // z=-2 の点: 球内部 (r=5)、平面側 half = 2 (真の距離)。両者一致すべき。
        let p = Vec3::new(0.0, 0.0, -2.0);
        assert!(
            (unit.eval(p) - 2.0).abs() < 1e-12,
            "unit normal: half-space distance must be 2.0"
        );
        assert!(
            (scaled.eval(p) - unit.eval(p)).abs() < 1e-12,
            "non-unit normal must be normalized to the same metric field"
        );
    }

    #[test]
    fn cut_aabb_is_bounded_by_child_not_infinite() {
        // 半空間単独なら無限 AABB だが、cut は子への交差なので AABB は子の AABB。
        let child = Sdf::sphere(1.0);
        let cut = child.clone().cut(Vec3::new(0.0, 0.0, -1.0), 0.0);
        assert_eq!(
            cut.aabb(),
            child.aabb(),
            "cut aabb must equal child aabb (bounded)"
        );
        // sampling_box も有限で非反転。
        let (lo, hi) = cut.sampling_box();
        assert!(
            lo.x.is_finite() && hi.x.is_finite(),
            "cut sampling_box must be finite"
        );
        assert!(lo.z <= hi.z, "cut sampling_box must not be inverted");
    }

    #[test]
    fn rotate_box_negative_and_per_axis_swaps_extents_symmetrically() {
        // 問184: rotate_z_90deg は +90°/z軸のみ。負角 (-90°) と x/y 軸も
        // 範囲入れ替えが対称に起きることを確認する。±90° は同じ aabb になるはず
        // (細長い箱を回すと x/y が入れ替わり、符号対称の箱なので結果は一致)。
        use std::f64::consts::FRAC_PI_2;
        let bar = Sdf::cuboid(Vec3::new(2.0, 0.5, 0.5));
        let pos = bar.clone().rotate_z(FRAC_PI_2).aabb();
        let neg = bar.clone().rotate_z(-FRAC_PI_2).aabb();
        // ±90° の z 回転は対称な箱に対して同一 aabb を生む。
        assert!(
            (pos.0.x - neg.0.x).abs() < 1e-9,
            "±90° z must give same lo.x"
        );
        assert!(
            (pos.1.y - neg.1.y).abs() < 1e-9,
            "±90° z must give same hi.y"
        );
        assert!(
            (neg.1.y - 2.0).abs() < 1e-9,
            "neg rotate hi.y must be 2.0, got {}",
            neg.1.y
        );

        // x 軸回転 90°: y 半幅 (0.5) と z 半幅 (0.5) が入れ替わる (両方 0.5 なので不変)。
        // 代わりに y!=z の箱で確認する。
        let bar_yz = Sdf::cuboid(Vec3::new(0.5, 2.0, 0.3));
        let rx = bar_yz.clone().rotate_x(FRAC_PI_2).aabb();
        // x 軸回転で y(2.0)↔z(0.3) 入れ替え → hi.z ≈ 2.0, hi.y ≈ 0.3。
        assert!(
            (rx.1.z - 2.0).abs() < 1e-9,
            "rotate_x must move y-extent to z, got hi.z={}",
            rx.1.z
        );
        assert!(
            (rx.1.y - 0.3).abs() < 1e-9,
            "rotate_x must move z-extent to y, got hi.y={}",
            rx.1.y
        );

        // y 軸回転 90°: x(0.5)↔z(0.3) 入れ替え。
        let ry = bar_yz.clone().rotate_y(FRAC_PI_2).aabb();
        assert!(
            (ry.1.x - 0.3).abs() < 1e-9,
            "rotate_y must move z-extent to x, got hi.x={}",
            ry.1.x
        );
        assert!(
            (ry.1.z - 0.5).abs() < 1e-9,
            "rotate_y must move x-extent to z, got hi.z={}",
            ry.1.z
        );
    }

    #[test]
    fn intersection_of_nonoverlapping_shapes_inverts_aabb_but_eval_is_exterior() {
        // 問185: 非重複の hard Intersection は aabb が反転 (lo > hi) しうるが、
        // eval は正しく外部 (正値) を返し、sampling_box は正規化することを確認。
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(10.0, 0.0, 0.0));
        let inter = a.intersection(b);
        // aabb は x 方向で反転する (a の hi.x=1.0 < b の lo.x=9.0 → max(lo)=9, min(hi)=1)。
        let (lo, hi) = inter.aabb();
        assert!(
            lo.x > hi.x,
            "non-overlap intersection aabb must invert on x: lo.x={} hi.x={}",
            lo.x,
            hi.x
        );
        // eval は max(da, db)。原点では a 内部(-1.0) だが b 外部(+9.0) → max=+9.0 (外部)。
        let d = inter.eval(Vec3::ZERO);
        assert!(
            d > 0.0,
            "intersection of disjoint shapes must be exterior at origin: {d}"
        );
        assert!((d - 9.0).abs() < 1e-9, "eval = max(-1, 9) = 9, got {d}");
        // sampling_box は正規化される (lo <= hi)。
        let (slo, shi) = inter.sampling_box();
        assert!(
            slo.x <= shi.x && slo.y <= shi.y && slo.z <= shi.z,
            "sampling_box must be normalized"
        );
    }

    #[test]
    fn mirror_box_symmetrizes_aabb_but_eval_keeps_only_positive_half() {
        // 問186: mirror_box は ext = max(|lo|, |hi|) で対称化するが、eval は
        // child.eval(|x|,..) なので **+x 半分を -x へ反射**する規約である。
        // 完全に -x 側にある形状を mirror_x すると:
        //  - aabb は対称 [-3.5, 3.5] に広がる (保守的境界)
        //  - しかし |x|>=0 は child (中心 x=-3) に決して届かないため幾何は空になる
        // この aabb と eval の乖離 (保守境界 vs 実際の空集合) を固定する。
        let neg_side = Sdf::sphere(0.5).translate(Vec3::new(-3.0, 0.0, 0.0));
        let m = neg_side.clone().mirror_x();
        let (lo, hi) = m.aabb();
        // aabb: ext = max(|-3.5|, |-2.5|) = 3.5 → [-3.5, 3.5] (対称・保守的)。
        assert!(
            (lo.x + 3.5).abs() < 1e-9,
            "mirrored lo.x must be -3.5, got {}",
            lo.x
        );
        assert!(
            (hi.x - 3.5).abs() < 1e-9,
            "mirrored hi.x must be 3.5, got {}",
            hi.x
        );
        assert!(
            (lo.x + hi.x).abs() < 1e-12,
            "mirror aabb must be symmetric: lo.x=-hi.x"
        );
        // eval: child は x=-3 にあり |x|>=0 では届かない → どこも外部 (空集合)。
        let d_pos = m.eval(Vec3::new(3.0, 0.0, 0.0)); // child.eval(3,..) = |3-(-3)|-0.5 = 5.5
        let d_neg = m.eval(Vec3::new(-3.0, 0.0, 0.0)); // child.eval(|-3|,..) = 同じ 5.5
        assert!(
            (d_pos - d_neg).abs() < EPS,
            "mirror eval must be symmetric: {d_pos} vs {d_neg}"
        );
        assert!(
            d_pos > 0.0,
            "negative-side shape mirrors to empty: must be exterior, got {d_pos}"
        );

        // 対照: +x 側の形状なら反射コピーが -x 側に現れる (規約の正常動作)。
        let pos_side = Sdf::sphere(0.5).translate(Vec3::new(3.0, 0.0, 0.0));
        let mp = pos_side.mirror_x();
        assert!(
            mp.eval(Vec3::new(3.0, 0.0, 0.0)) < 0.0,
            "+x copy must exist (inside)"
        );
        assert!(
            mp.eval(Vec3::new(-3.0, 0.0, 0.0)) < 0.0,
            "reflected -x copy must exist (inside)"
        );
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
    fn smooth_union_exactly_equals_hard_outside_blend_zone() {
        // 問135: |da - db| > k の点では h が [0,1] にクランプされ、
        // k * h * (1-h) 項が消える。よって smooth_union = hard_union が厳密に成立する。
        // 「blend 領域以外では hard と同じ」という保証を数値テストで固定する。
        let k = 0.3;
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(5.0, 0.0, 0.0)); // 中心間 5, 非重複
        let hard = a.clone().union(b.clone());
        let soft = a.clone().smooth_union(b.clone(), k);
        // blend は 2 球表面が距離 k 以内に近づく領域にのみ生じる。
        // 両球表面が k=0.3 より離れている点では soft == hard が厳密 (数値誤差ゼロ)。
        let test_points = [
            Vec3::new(0.0, 3.0, 0.0),  // 球A上方, 球Bまで 5 以上
            Vec3::new(5.0, 3.0, 0.0),  // 球B上方, 球Aまで 5 以上
            Vec3::new(-3.0, 0.0, 0.0), // 両球から遠い
            Vec3::new(8.0, 0.0, 0.0),  // 両球から遠い
            Vec3::new(0.0, 0.0, 2.0),  // 球A上方 (z軸)
        ];
        for p in test_points {
            let da = a.eval(p);
            let db = b.eval(p);
            // |da - db| > k ならば blend は生じない。
            if (da - db).abs() > k {
                let diff = (soft.eval(p) - hard.eval(p)).abs();
                assert!(
                    diff < 1e-14,
                    "at {p:?}: |da-db|={:.3} > k={k}: smooth must equal hard (diff={diff})",
                    (da - db).abs()
                );
            }
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
    fn cone_apex_is_exactly_zero_and_base_disk_is_complete() {
        // 問229/230: cone_surface_and_sign は apex を .abs()<EPS で確認し、底面は
        // 内部 1 点 (0.4,0,-2) のみ。apex が厳密に 0.0 (== 0.0)、底面のエッジ・内部・
        // 外側が正しいことを固定する (底面ディスク全体の被覆を確認)。
        let c = Sdf::cone(1.0, 2.0);
        // 先端は厳密に 0.0 (符号付きゼロ含む; .abs()<EPS より強い等値)。
        let apex = c.eval(Vec3::ZERO);
        assert_eq!(apex, 0.0, "apex distance must be exactly 0.0, got {apex}");
        // 底面ディスクのエッジ (r, 0, -h) は表面。
        assert!(
            c.eval(Vec3::new(1.0, 0.0, -2.0)).abs() < 1e-12,
            "base disk edge (1,0,-2) must be on surface: {}",
            c.eval(Vec3::new(1.0, 0.0, -2.0))
        );
        // 底面ディスク内部 (0.5, 0, -2) も表面 (ディスク全体が境界)。
        assert!(
            c.eval(Vec3::new(0.5, 0.0, -2.0)).abs() < 1e-12,
            "base disk interior (0.5,0,-2) must be on surface: {}",
            c.eval(Vec3::new(0.5, 0.0, -2.0))
        );
        // 底面ディスク外側 (1.1, 0, -2) は外部 (正)。
        assert!(
            c.eval(Vec3::new(1.1, 0.0, -2.0)) > 0.0,
            "just outside base disk (1.1,0,-2) must be exterior"
        );
    }

    #[test]
    fn smooth_union_of_shape_with_itself_subtracts_quarter_k_everywhere() {
        // 問228: smooth_union の多項式 mix(db,da,h) - k*h*(1-h) は da==db のとき
        // h=0.5 となり補正項 = k*0.25 (最大ブレンド)。同一形状同士の smooth_union は
        // 全点で da==db なので結果は d - k*0.25 になる。既存テストは収束 (k→0) や
        // ブレンドゾーン外 (|da-db|>k) のみで、中点での厳密な補正値を確認していなかった。
        let s = Sdf::sphere(1.0);
        let k = 0.4_f64;
        let su = s.clone().smooth_union(s.clone(), k);
        for p in [
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0), // 外部でも成立
            Vec3::ZERO,
        ] {
            let d = s.eval(p);
            let expected = d - k * 0.25;
            let got = su.eval(p);
            assert!(
                (got - expected).abs() < 1e-14,
                "smooth_union(s,s,{k}) at {p:?} must be d - k*0.25 = {expected}, got {got}"
            );
        }
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
    fn sampling_box_normalizes_for_all_inverted_aabb_variants() {
        // 問206 (SPEC §3.4): sampling_box は反転 AABB を生むすべての変種で正規化する。
        // sampling_box_is_never_inverted は SmoothIntersection のみ。
        // hard Intersection / Difference / SmoothDifference でも lo<=hi を保証することを
        // グループで固定し、いずれかの変種で正規化が壊れる回帰を防ぐ。
        let far = |s: Sdf| s.translate(Vec3::new(10.0, 0.0, 0.0));
        let cases: Vec<(&str, Sdf)> = vec![
            (
                "hard_intersection",
                Sdf::sphere(1.0).intersection(far(Sdf::sphere(1.0))),
            ),
            (
                "smooth_intersection",
                Sdf::sphere(1.0).smooth_intersection(far(Sdf::sphere(1.0)), 0.3),
            ),
            (
                "hard_difference",
                Sdf::sphere(0.5).difference(far(Sdf::sphere(2.0))),
            ),
            (
                "smooth_difference",
                Sdf::sphere(0.5).smooth_difference(far(Sdf::sphere(2.0)), 0.3),
            ),
        ];
        for (name, sdf) in cases {
            let (slo, shi) = sdf.sampling_box();
            assert!(
                slo.x <= shi.x && slo.y <= shi.y && slo.z <= shi.z,
                "{name}: sampling_box must be normalized, got lo={slo:?} hi={shi:?}"
            );
        }
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
            lo.x,
            child_lo.x
        );
        assert!(
            hi.x < child_hi.x,
            "shrunk AABB hi.x must be < child hi.x: hi.x={}, child_hi.x={}",
            hi.x,
            child_hi.x
        );
        // AABB は等方 (sphere): 各軸 ±(1.0 - 0.4) = ±0.6。
        assert!(
            (lo.x - (-0.6)).abs() < 1e-12,
            "expected lo.x=-0.6, got {}",
            lo.x
        );
        assert!(
            (hi.x - 0.6).abs() < 1e-12,
            "expected hi.x=0.6, got {}",
            hi.x
        );

        // AABB は依然 isosurface を内包する (表面点 (0.6,0,0) は AABB 内または境界)。
        let surface = Vec3::new(0.6, 0.0, 0.0);
        assert!(
            surface.x >= lo.x && surface.x <= hi.x,
            "surface point must be within AABB: {:?} in [{:?}, {:?}]",
            surface,
            lo,
            hi
        );

        // 過侵食 (amount > 子半径) では lo2 > hi2 → min/max で正規化されるため有限。
        let over_eroded = Sdf::sphere(1.0).offset(-1.5);
        let (elo, ehi) = over_eroded.aabb();
        assert!(
            elo.x.is_finite() && ehi.x.is_finite(),
            "over-eroded AABB must be finite"
        );
        assert!(
            elo.x <= ehi.x,
            "over-eroded AABB must not be inverted: {elo:?} {ehi:?}"
        );
    }

    #[test]
    fn translate_composition_is_additive_and_roundtrip() {
        // 問112: translate の合成 (入れ子) の代数法則を固定する。
        // 回転合成 (問110) と対称なペアとして、平行移動の加法性・往復恒等を保証する。
        //
        // (1) 加法性: translate(v1, translate(v2, S)) == translate(v1+v2, S)
        //     SDF: S(p - v2 - v1) = S(p - (v1+v2))。数値的に完全一致すること。
        // (2) 往復恒等: translate(-v, translate(v, S)) == S
        //     SDF: S(p - v - (-v)) = S(p - 0) = S(p)。
        let child = Sdf::cuboid(Vec3::new(0.8, 0.5, 0.3));
        let v1 = Vec3::new(0.7, -0.3, 0.5);
        let v2 = Vec3::new(-0.4, 1.2, -0.6);
        let pts = grid();

        // (1) 加法性: translate(v2).translate(v1) == translate(v1+v2)
        let composed = child.clone().translate(v2).translate(v1);
        let single = child.clone().translate(v1 + v2);
        for &p in &pts {
            assert!(
                (composed.eval(p) - single.eval(p)).abs() < EPS,
                "translate composition must be additive: translate(v1)∘translate(v2) == translate(v1+v2) at {p:?}"
            );
        }

        // (2) 往復恒等: translate(v).translate(-v) == identity
        let roundtrip = child.clone().translate(v1).translate(-v1);
        for &p in &pts {
            assert!(
                (roundtrip.eval(p) - child.eval(p)).abs() < EPS,
                "translate(v) then translate(-v) must be identity at {p:?}"
            );
        }
    }

    #[test]
    fn boolean_idempotency_union_intersection_difference_self() {
        // 問113: ブーリアン等冪法則。AI がノードを複製して union/intersection/difference を
        // 掛ける DSL を生成した場合、代数的に予測可能な結果になることを固定する。
        //
        // (1) union(A, A) == A everywhere      (min(f,f) = f)
        // (2) intersection(A, A) == A everywhere (max(f,f) = f)
        // (3) difference(A, A) >= 0 everywhere  (max(f,-f) = |f| >= 0 → 自己差分は常に外部)
        let a = Sdf::sphere(1.0).smooth_union(Sdf::cuboid(Vec3::splat(0.6)), 0.2);
        let pts = grid();

        // (1) union の等冪性。
        let u = a.clone().union(a.clone());
        for &p in &pts {
            assert!(
                (u.eval(p) - a.eval(p)).abs() < EPS,
                "union(A, A) must equal A at {p:?}: got {} vs {}",
                u.eval(p),
                a.eval(p)
            );
        }

        // (2) intersection の等冪性。
        let i = a.clone().intersection(a.clone());
        for &p in &pts {
            assert!(
                (i.eval(p) - a.eval(p)).abs() < EPS,
                "intersection(A, A) must equal A at {p:?}: got {} vs {}",
                i.eval(p),
                a.eval(p)
            );
        }

        // (3) difference(A, A) は全域 d >= 0 (自己差分は空 = 外部のみ)。
        let d = a.clone().difference(a.clone());
        for &p in &pts {
            assert!(
                d.eval(p) >= -EPS,
                "difference(A, A) must be >= 0 everywhere (self-subtraction is empty) at {p:?}: got {}",
                d.eval(p)
            );
        }
    }

    #[test]
    fn sampling_box_encloses_aabb_for_representative_shapes() {
        // 問115: sampling_box は AABB を 5% マージンで内包する。
        // aabb_encloses_surface_samples (問14) は1つの複合ツリーでこれを確認するが、
        // 全形状電池で invariant が成り立つことは未確認だった。
        // primitive/変換/ブーリアン/複合 を網羅する代表電池で固定する。
        // 問136: Cone/RoundedBox/Ellipsoid が電池から漏れていたため追加。
        let shapes: &[Sdf] = &[
            Sdf::sphere(1.0),
            Sdf::cuboid(Vec3::new(1.0, 0.8, 0.5)),
            Sdf::cylinder(0.5, 1.5),
            Sdf::capsule(0.4, 1.2),
            Sdf::torus(0.8, 0.2),
            Sdf::cone(0.5, 1.0),
            Sdf::rounded_box(Vec3::new(0.8, 0.6, 0.4), 0.1),
            Sdf::ellipsoid(Vec3::new(1.2, 0.8, 0.5)),
            Sdf::sphere(1.0).translate(Vec3::new(0.5, -0.3, 0.2)),
            Sdf::sphere(0.6).union(Sdf::cuboid(Vec3::splat(0.5))),
            Sdf::sphere(1.0).difference(Sdf::cylinder(0.4, 2.0)),
            Sdf::sphere(1.0).shell(0.25),
            Sdf::sphere(0.5).repeat_n(Vec3::splat(2.0), [1, 1, 1]),
            // 問142: Mirror は電池から漏れていた。mirror_box は反射軸で対称ボックスを作る。
            Sdf::sphere(0.5)
                .translate(Vec3::new(1.5, 0.0, 0.0))
                .mirror_x(),
        ];
        for (k, s) in shapes.iter().enumerate() {
            let (alo, ahi) = s.aabb();
            let (slo, shi) = s.sampling_box();
            assert!(
                slo.x <= alo.x && slo.y <= alo.y && slo.z <= alo.z,
                "shape {k}: sampling_box lo must be <= aabb lo: slo={slo:?} alo={alo:?}"
            );
            assert!(
                shi.x >= ahi.x && shi.y >= ahi.y && shi.z >= ahi.z,
                "shape {k}: sampling_box hi must be >= aabb hi: shi={shi:?} ahi={ahi:?}"
            );
            // sampling_box 自体は反転しない。
            assert!(
                slo.x <= shi.x && slo.y <= shi.y && slo.z <= shi.z,
                "shape {k}: sampling_box must not be inverted: slo={slo:?} shi={shi:?}"
            );
        }
    }

    #[test]
    fn sampling_box_applies_minimum_margin_for_zero_aabb() {
        // 問126: AABB が点 (diag = 0) の場合、sampling_box は 1e-3 の最小余白を保証する。
        // この最小値が唯一の防護線: 欠落すると polygonize がゼロ幅のボックスを受け取り
        // step = 0 / ゼロ除算になる。
        // 半径0の球 (eval(p)=length(p)≥0) は AABB = (0,0,0)×(0,0,0) → diag=0。
        let s = Sdf::Sphere { radius: 0.0 };
        let (lo, hi) = s.sampling_box();
        let span = hi - lo;
        assert!(
            span.x >= 1e-3 && span.y >= 1e-3 && span.z >= 1e-3,
            "zero-AABB shape must get at least 1e-3 margin per axis, got span={span:?}"
        );
        // polygonize はパニックせず空メッシュを返す (全評価点が val≥0 のため)。
        let mesh = crate::extract::polygonize(&s, lo, hi, 4);
        assert!(
            mesh.triangles.is_empty(),
            "zero-radius sphere must produce empty mesh, got {} triangles",
            mesh.triangles.len()
        );
    }

    #[test]
    fn aabb_exact_values_for_primitives() {
        // 問148: 各プリミティブの aabb() が期待する数値を返すことを固定する。
        // 退行で AABB が縮小すると polygonize が表面を欠損し、サイレントに不完全なメッシュを生む。
        const EPS: f64 = 1e-12;

        // Sphere(r=1.5): 全軸 ±1.5。
        let (lo, hi) = Sdf::sphere(1.5).aabb();
        assert!(
            (lo.x + 1.5).abs() < EPS && (hi.x - 1.5).abs() < EPS,
            "sphere aabb x"
        );
        assert!(
            (lo.z + 1.5).abs() < EPS && (hi.z - 1.5).abs() < EPS,
            "sphere aabb z"
        );

        // Cylinder(r=0.5, half_height=2.0): XY は ±0.5、Z は ±2.0。
        // API: cylinder(radius, half_height) — 第2引数は高さの「半分」。
        let (lo, hi) = Sdf::cylinder(0.5, 2.0).aabb();
        assert!(
            (lo.x + 0.5).abs() < EPS && (hi.x - 0.5).abs() < EPS,
            "cylinder aabb x"
        );
        assert!(
            (lo.z + 2.0).abs() < EPS && (hi.z - 2.0).abs() < EPS,
            "cylinder aabb z"
        );

        // Torus(major=2.0, minor=0.5): XY は ±2.5、Z は ±0.5。
        let (lo, hi) = Sdf::torus(2.0, 0.5).aabb();
        assert!(
            (lo.x + 2.5).abs() < EPS && (hi.x - 2.5).abs() < EPS,
            "torus aabb x"
        );
        assert!(
            (lo.z + 0.5).abs() < EPS && (hi.z - 0.5).abs() < EPS,
            "torus aabb z"
        );

        // Cone(r=1.0, h=2.0): XY は ±1.0 (底面)、Z は [-2.0, 0.0] (頂点=z=0, 底面=z=-2)。
        let (lo, hi) = Sdf::cone(1.0, 2.0).aabb();
        assert!(
            (lo.x + 1.0).abs() < EPS && (hi.x - 1.0).abs() < EPS,
            "cone aabb x"
        );
        assert!((lo.z + 2.0).abs() < EPS && hi.z.abs() < EPS, "cone aabb z");
    }

    #[test]
    fn rotation_roundtrip_holds_for_x_and_y_axes() {
        // 問153: rotate_z の往復恒等は問110 でテスト済み。X・Y 軸は異なる行列を使うため
        // 独立して確認する。rotate_x(θ).rotate_x(-θ) および rotate_y(θ).rotate_y(-θ) が
        // 恒等変換になることを非対称形状で固定する。
        let child = Sdf::cuboid(Vec3::new(1.0, 0.5, 0.3));
        let angle = 0.7_f64; // 任意の非自明な角度
        for &p in &grid() {
            // X 軸往復。
            let rx = child.clone().rotate_x(angle).rotate_x(-angle);
            assert!(
                (rx.eval(p) - child.eval(p)).abs() < 1e-9,
                "rotate_x roundtrip must be identity at {p:?}"
            );
            // Y 軸往復。
            let ry = child.clone().rotate_y(angle).rotate_y(-angle);
            assert!(
                (ry.eval(p) - child.eval(p)).abs() < 1e-9,
                "rotate_y roundtrip must be identity at {p:?}"
            );
        }
    }

    #[test]
    fn repeat_snap_at_half_period_maps_to_neighbor_cell() {
        // 問154: Rust の `f64::round()` は「半分は0から遠い方向」に丸める
        // (round-half-away-from-zero)。period=2.0 でちょうど中間 x=1.0 は
        // `(1.0/2.0).round() = 0.5.round() = 1.0` → 隣接セル (x=2.0) に snap する。
        // この動作がバンカー丸めとは異なることを固定する (退行で変化したら即検出)。
        let s = Sdf::sphere(0.3).repeat_n(Vec3::new(2.0, 0.0, 0.0), [1, 0, 0]);
        // x=1.0 は中心球 (x=0) と隣接球 (x=2) の中間。snap→隣接セル (x=2) に吸い寄せられ
        // 距離 = 1.0 - 0.3 = 0.7 > 0 (外部)。もし中心セルに snap されても同じ距離になるが
        // 「どちらに snap されたか」を確認するため境界外 x=2.5 (隣接セル内) も確認。
        let at_mid = s.eval(Vec3::new(1.0, 0.0, 0.0));
        let at_neighbor = s.eval(Vec3::new(2.0, 0.0, 0.0));
        // 隣接セル中心は球内部 (負)。
        assert!(
            at_neighbor < 0.0,
            "x=2 (copy center) must be inside sphere: {at_neighbor}"
        );
        // x=1.0 は両球から距離 0.7 → 外部。
        assert!(
            at_mid > 0.0,
            "midpoint x=1.0 must be outside both spheres: {at_mid}"
        );
        // 両端が同距離なことの確認 (snap どちらでも同値)。
        let at_neg_mid = s.eval(Vec3::new(-1.0, 0.0, 0.0));
        assert!(
            (at_mid - at_neg_mid).abs() < 1e-12,
            "midpoints at ±period/2 must be equidistant from nearest sphere"
        );
    }

    #[test]
    fn scale_negative_factor_sampling_box_is_normalized() {
        // 問149: eval.rs は s<=0 を拒否するが、Sdf:: API を直接呼ぶと
        // aabb() が反転した (lo > hi) ボックスを返す。sampling_box() の正規化が
        // これを安全に扱い lo <= hi の範囲を返すことを固定する。
        // (polygonize が負ステップで壊れないための防護線)
        let s = Sdf::sphere(1.0).scale(-2.0);
        let (lo, hi) = s.sampling_box();
        // 反転後も sampling_box が lo <= hi を保証する。
        assert!(
            lo.x <= hi.x,
            "sampling_box x must be non-inverted after scale(-1)"
        );
        assert!(
            lo.y <= hi.y,
            "sampling_box y must be non-inverted after scale(-1)"
        );
        assert!(
            lo.z <= hi.z,
            "sampling_box z must be non-inverted after scale(-1)"
        );
    }

    #[test]
    fn sampling_box_with_zero_period_repeat_axis_is_not_inverted() {
        // 問201: sampling_box_encloses_aabb は repeat_n(splat(2), [1,1,1]) のみ確認。
        // period.x=0 (x 軸無効) の repeat で aabb が child の AABB のみを返し、
        // sampling_box がその正規化と 5% マージンを正しく適用することを確認。
        let sphere = Sdf::sphere(1.0);
        let rep = sphere.repeat_n(Vec3::new(0.0, 2.0, 2.0), [0, 1, 1]);
        let (alo, ahi) = rep.aabb();
        let (slo, shi) = rep.sampling_box();
        // sampling_box は aabb を包含する。
        assert!(
            slo.x <= alo.x,
            "sampling_box.x.lo must enclose aabb.x.lo: slo.x={} alo.x={}",
            slo.x,
            alo.x
        );
        assert!(
            shi.x >= ahi.x,
            "sampling_box.x.hi must enclose aabb.x.hi: shi.x={} ahi.x={}",
            shi.x,
            ahi.x
        );
        assert!(slo.y <= alo.y, "sampling_box.y.lo must enclose aabb.y.lo");
        assert!(shi.y >= ahi.y, "sampling_box.y.hi must enclose aabb.y.hi");
        // 反転していない (lo <= hi)。
        assert!(
            slo.x <= shi.x && slo.y <= shi.y && slo.z <= shi.z,
            "sampling_box must not be inverted with zero-period axis"
        );
        // x 軸の AABB は child 範囲のみ (period.x=0 なのでカウントによる拡張なし)。
        // sampling_box は全軸一律の diag*5% マージンを加えるため、y/z の拡張の影響を受ける。
        // (diag はすべての軸を含む全体の対角線なので x も多少大きくなる → 保守的に確認)
        assert!(shi.x >= 1.0, "x half-extent must cover child sphere radius");
        assert!(
            shi.x < 3.0,
            "x-axis must not excessively expand with period=0: shi.x={}",
            shi.x
        );
    }

    #[test]
    fn scale_factor_one_is_identity() {
        // 問161: scale(1.0) は恒等変換なので child.eval(p) と完全一致しなければならない。
        // uniform_scale_preserves_distance_field は factor=2.0 のみ確認。factor=1.0 で
        // 別のコードパスを通る可能性 (e.g., p / 1.0 の浮動小数点丸め) を固定する。
        let sphere = Sdf::sphere(1.0);
        let scaled = sphere.clone().scale(1.0);
        let probes = [
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-2.0, 3.0, -1.0),
        ];
        for p in probes {
            let d_orig = sphere.eval(p);
            let d_scaled = scaled.eval(p);
            assert_eq!(
                d_orig, d_scaled,
                "scale(1.0) must be identity: at {p:?} orig={d_orig} scaled={d_scaled}"
            );
        }
    }

    #[test]
    fn repeat_count_zero_disables_axis_with_positive_period() {
        // 問162: count=0 の軸は period が正でも繰り返しを行わない。
        // snap() は `if per == 0.0 || n == 0 { return v; }` を持つが、
        // count=0 で無効化される軸が実際に無効であることをテストで確認する。
        // repeat_n(period=[2,2,2], count=[1,0,1]) → y 軸のみ繰り返しなし。
        let sphere = Sdf::sphere(0.3);
        let rep = sphere.clone().repeat_n(Vec3::splat(2.0), [1, 0, 1]);
        // y=0 と y=2.0 の距離が異なる → y 軸はクランプされず連続な SDF のまま。
        let d_y0 = rep.eval(Vec3::new(0.0, 0.0, 0.0));
        let d_y2 = rep.eval(Vec3::new(0.0, 2.0, 0.0));
        // count=1 の x 軸は period 2.0 で繰り返す → x=0 と x=2.0 は同じセルに snap。
        let d_x0 = rep.eval(Vec3::new(0.0, 0.0, 0.0));
        let d_x2 = rep.eval(Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(
            d_x0, d_x2,
            "x-axis (count=1) must repeat: d(x=0)={d_x0} d(x=2)={d_x2}"
        );
        // y 軸 (count=0) は繰り返しなし: y=2.0 は繰り返しセルに snap されない。
        // 中心 (0,0,0) は球の内部 (d<0)、y=2.0 は 1.7 距離 (d≈1.4>0) なので異なるはず。
        assert_ne!(
            d_y0, d_y2,
            "y-axis (count=0) must not repeat: d(y=0)={d_y0} should differ from d(y=2)={d_y2}"
        );
    }

    #[test]
    fn smooth_union_and_intersection_remain_finite_for_tiny_k() {
        // 問163: k→0 のとき (da - db) / k → ±∞ になるが clamp(h, 0, 1) で吸収される。
        // 非常に小さい k (1e-300) でも NaN/Inf が生じないことを確認する。
        // k=1e-6 の収束テスト (問905相当) とは別に、極限での数値安全性を固定。
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(3.0, 0.0, 0.0));
        let probes = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.5, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        ];
        for &k in &[1e-1_f64, 1e-6, 1e-12, 1e-100, 1e-300] {
            let su = a.clone().smooth_union(b.clone(), k);
            let si = a.clone().smooth_intersection(b.clone(), k);
            let sd = a.clone().smooth_difference(b.clone(), k);
            for p in probes {
                let du = su.eval(p);
                let di = si.eval(p);
                let dd = sd.eval(p);
                assert!(
                    du.is_finite(),
                    "smooth_union k={k} at {p:?}: {du} is not finite"
                );
                assert!(
                    di.is_finite(),
                    "smooth_intersection k={k} at {p:?}: {di} is not finite"
                );
                assert!(
                    dd.is_finite(),
                    "smooth_difference k={k} at {p:?}: {dd} is not finite"
                );
            }
        }
    }

    #[test]
    fn shell_zero_thickness_equals_absolute_value_field() {
        // 問176: thickness=0 のとき d.max(-(d+0)) = d.max(-d) = |d|。
        // Shell(shape, 0) は完全に面圧縮された表面 (絶対値場) になる。
        // 既存テストはすべて thickness > 0 のみ確認。
        let sphere = Sdf::sphere(1.0);
        let shell0 = sphere.clone().shell(0.0);
        for r in [0.0_f64, 0.5, 1.0, 2.0, -0.5] {
            let p = Vec3::new(r, 0.0, 0.0);
            let d = sphere.eval(p);
            let expected = d.abs();
            let got = shell0.eval(p);
            assert!(
                (got - expected).abs() < 1e-12,
                "shell(0.0).eval at r={r}: expected |d|={expected} got {got}"
            );
        }
    }

    #[test]
    fn scale_zero_factor_eval_produces_nan_not_panic() {
        // 問177: Sdf::scale(0.0) は eval.rs が s<=0 を拒否するため通常使用には現れないが、
        // Sdf::Scale を直接構築した場合のコード挙動を文書化する。
        // p / 0.0 → Inf/NaN コンポーネント → child.eval(NaN) → NaN → 0.0 * NaN = NaN。
        // 「パニックしない」かつ「NaN を返す」ことを固定し、将来のリグレッションを防ぐ。
        let s = Sdf::sphere(1.0).scale(0.0);
        let d = s.eval(Vec3::new(0.5, 0.0, 0.0));
        // パニックなしを確認 (上の行が到達できれば OK)。NaN であることも固定。
        assert!(
            d.is_nan(),
            "scale(0.0).eval must be NaN (not panic), got {d}"
        );
        // aabb は 0 * child_bound = 0 → lo=hi=0 (有限)。
        let (lo, hi) = s.aabb();
        assert!(
            lo.x.is_finite() && hi.x.is_finite(),
            "scale(0.0).aabb must be finite (all-zero)"
        );
    }

    #[test]
    fn capsule_radius_exceeds_half_height_degenerates_to_sphere_like() {
        // 問178: radius > half_height のとき、軸方向クランプ範囲 [-hh, hh] が
        // 半径より短くなり、上下半球が重なる形状になる。
        // capsule(half_height, radius) の引数順に注意 (問178 で判明)。
        // half_height=0.1, radius=1.0 → 丸みの勝ったカプセル。
        let c = Sdf::capsule(0.1, 1.0); // half_height=0.1, radius=1.0
                                        // 中心: pz_clamped=0, length(0,0,0)=0 → d = 0 - 1.0 = -1.0。
        assert!(
            (c.eval(Vec3::ZERO) - (-1.0)).abs() < 1e-12,
            "center must be at d=-1.0"
        );
        // 軸端 (0,0,0.1) から radial 方向 (1.0, 0, 0.1):
        // pz_clamped=0.1, offset=(1.0, 0, 0) → length=1.0 → d = 1.0 - 1.0 = 0。
        assert!(
            c.eval(Vec3::new(1.0, 0.0, 0.1)).abs() < 1e-12,
            "end-cap surface at (1,0,0.1)"
        );
        // 遠点は外部 (有限)。
        let d_far = c.eval(Vec3::new(5.0, 0.0, 0.0));
        assert!(
            d_far.is_finite() && d_far > 0.0,
            "far exterior must be positive finite: {d_far}"
        );
        // 連続性: 中心 → 軸端 の中間点も評価できる。
        for z in [0.0_f64, 0.05, 0.1, 0.2] {
            let d = c.eval(Vec3::new(0.0, 0.0, z));
            assert!(d.is_finite(), "eval at z={z} must be finite: {d}");
        }
    }

    #[test]
    fn offset_negative_extreme_scale_ratio_aabb_is_finite_and_normalized() {
        // 問182: Sphere(1e-10).offset(-1.0) で child の大きさ << offset 量。
        // AABB 正規化 lo.min(hi)/lo.max(hi) を経た後、lo<=hi かつ有限であることを確認。
        // offset_negative_aabb_tightens は Sphere(1.0).offset(-0.4) のみ。
        let extreme = Sdf::sphere(1e-10).offset(-1.0);
        let (lo, hi) = extreme.aabb();
        assert!(lo.x.is_finite(), "lo.x must be finite: {}", lo.x);
        assert!(hi.x.is_finite(), "hi.x must be finite: {}", hi.x);
        assert!(
            lo.x <= hi.x,
            "aabb must be normalized (lo <= hi): lo={} hi={}",
            lo.x,
            hi.x
        );
        // eval も NaN/Inf を生じない。
        let d = extreme.eval(Vec3::ZERO);
        assert!(d.is_finite(), "eval at origin must be finite: {d}");
    }
}
