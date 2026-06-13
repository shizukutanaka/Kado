//! 製造可能性 (DFM) 検査と構造化エラー。
//!
//! 各検査は [`KadoError`] のリストを返す。エラー ≒ 製造上の問題。
//! `fix_hints` は AI エージェントが自己修正ループを回すためのヒント (Plan §3)。

use crate::core::Vec3;
use crate::extract::Mesh;

// ── 構造化エラー ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug)]
pub struct KadoError {
    pub severity: Severity,
    pub code: &'static str,
    pub cause: String,
    pub fix_hints: Vec<String>,
}

impl KadoError {
    fn error(code: &'static str, cause: impl Into<String>, hints: &[&str]) -> KadoError {
        KadoError {
            severity: Severity::Error,
            code,
            cause: cause.into(),
            fix_hints: hints.iter().map(|s| s.to_string()).collect(),
        }
    }
    fn warn(code: &'static str, cause: impl Into<String>, hints: &[&str]) -> KadoError {
        KadoError {
            severity: Severity::Warning,
            code,
            cause: cause.into(),
            fix_hints: hints.iter().map(|s| s.to_string()).collect(),
        }
    }
}

// ── 検証レポート ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct Report {
    /// 符号付き体積 (mm³ または 任意単位)。
    pub volume: f64,
    /// 軸整列バウンディングボックス [min, max]。
    pub bbox: Option<(Vec3, Vec3)>,
    /// 三角形数。
    pub triangle_count: usize,
    /// edge-manifold (水密) かどうか。
    pub is_manifold: bool,
    /// DFM 問題リスト。
    pub issues: Vec<KadoError>,
}

impl Report {
    /// 問題なし (エラー0) なら true。
    pub fn is_ok(&self) -> bool {
        self.issues.iter().all(|e| e.severity != Severity::Error)
    }

    /// 人間可読なサマリー。
    pub fn summary(&self) -> String {
        let errors = self
            .issues
            .iter()
            .filter(|e| e.severity == Severity::Error)
            .count();
        let warnings = self
            .issues
            .iter()
            .filter(|e| e.severity == Severity::Warning)
            .count();
        let (lo, hi) = self
            .bbox
            .map(|(a, b)| (a, b))
            .unwrap_or((Vec3::ZERO, Vec3::ZERO));
        format!(
            "triangles={} manifold={} volume={:.3} \
             bbox=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] \
             errors={errors} warnings={warnings}",
            self.triangle_count, self.is_manifold, self.volume, lo.x, lo.y, lo.z, hi.x, hi.y, hi.z,
        )
    }
}

// ── メインエントリポイント ────────────────────────────────────────────────────

/// メッシュを検証して [`Report`] を返す。
///
/// `min_wall_mm` は最小肉厚チェックの閾値 (0以下でスキップ)。
/// `max_overhang_deg` は最大オーバーハング角度 (度; 0以下でスキップ)。
pub fn validate(mesh: &Mesh, min_wall_mm: f64, max_overhang_deg: f64) -> Report {
    let volume = mesh.signed_volume();
    let bbox = mesh.bounds();
    let is_manifold = mesh.is_edge_manifold();
    let tri_count = mesh.triangles.len();
    let mut issues = vec![];

    // 1. 非多様体メッシュ (致命的)
    if !is_manifold {
        issues.push(KadoError::error(
            "NON_MANIFOLD",
            "mesh is not edge-manifold (non-watertight)",
            &[
                "Increase mesh resolution (higher res value)",
                "Check for self-intersecting geometry in the SDF tree",
            ],
        ));
    }

    // 2. 空メッシュ
    if tri_count == 0 {
        issues.push(KadoError::error(
            "EMPTY_MESH",
            "mesh has no triangles — bounding box may not contain the shape",
            &["Expand the bounding box or check primitive parameters"],
        ));
        return Report {
            volume,
            bbox,
            triangle_count: tri_count,
            is_manifold,
            issues,
        };
    }

    // 3. 負体積 (裏返し)
    if volume < 0.0 {
        issues.push(KadoError::warn(
            "NEGATIVE_VOLUME",
            format!("signed volume is negative ({volume:.3}), mesh may be inverted"),
            &["Check SDF field orientation; inner surface should have negative SDF"],
        ));
    }

    // 4. 肉厚チェック (2V/SA による平均肉厚。問23: 「平均」であり最小ではない)
    if min_wall_mm > 0.0 {
        if let Some((lo, hi)) = bbox {
            let thin = mean_wall_thickness(mesh, lo, hi);
            if thin < min_wall_mm {
                issues.push(KadoError::error(
                    "THIN_WALL",
                    format!(
                        "estimated mean wall thickness {thin:.3} < {min_wall_mm:.3} \
                         (2V/SA average; a pass does not guarantee no local thin features)"
                    ),
                    &[
                        "Increase wall thickness via offset() or larger primitives",
                        "Reduce min_wall_mm threshold if intentional",
                    ],
                ));
            }
        }
    }

    // 5. オーバーハング検査 (z 軸上向きビルド前提)
    if max_overhang_deg > 0.0 {
        let max_cos = (90.0_f64 - max_overhang_deg).to_radians().cos();
        let mut worst: f64 = 0.0;
        for tri in &mesh.triangles {
            let a = mesh.vertices[tri[0] as usize];
            let b = mesh.vertices[tri[1] as usize];
            let c = mesh.vertices[tri[2] as usize];
            let n = (b - a).cross(c - a);
            let len = n.length();
            if len < 1e-15 {
                continue;
            }
            let nz = n.z / len;
            if nz < worst {
                worst = nz;
            }
        }
        if worst < -max_cos {
            let deg = worst.acos().to_degrees();
            issues.push(KadoError::warn(
                "OVERHANG",
                format!("overhang angle {deg:.1}° exceeds max {max_overhang_deg:.1}°"),
                &[
                    "Add support structures or redesign with chamfer/fillet",
                    "Rotate the model to minimize overhangs",
                ],
            ));
        }
    }

    Report {
        volume,
        bbox,
        triangle_count: tri_count,
        is_manifold,
        issues,
    }
}

/// 2V/SA (体積÷表面積×2) で **平均** 肉厚を推定する (問23)。
///
/// 注意: これは平均であって**最小ではない**。塊状の本体に細い1リブが付く形状では
/// 2V/SA が本体に支配されて大きく出るため、リブの薄さを見逃しうる。よって
/// 「閾値未満 = 薄肉」は有効な検出だが、「閾値以上 = 薄肉なし」は保証しない。
/// 真の最小肉厚 (medial axis 等) は別途要実装 (BACKLOG)。メッシュのみからの安価な近似。
fn mean_wall_thickness(mesh: &Mesh, lo: Vec3, hi: Vec3) -> f64 {
    if mesh.triangles.is_empty() {
        return (hi - lo).length();
    }
    let surface_area: f64 = mesh
        .triangles
        .iter()
        .map(|t| {
            let a = mesh.vertices[t[0] as usize];
            let b = mesh.vertices[t[1] as usize];
            let c = mesh.vertices[t[2] as usize];
            (b - a).cross(c - a).length() * 0.5
        })
        .sum();
    if surface_area < 1e-15 {
        return (hi - lo).length();
    }
    2.0 * mesh.signed_volume().abs() / surface_area
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Sdf;
    use crate::extract::polygonize;

    #[test]
    fn sphere_passes_validation() {
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 32);
        let r = validate(&mesh, 0.05, 45.0);
        assert!(r.is_manifold, "sphere must be manifold");
        assert!(r.volume > 0.0, "volume must be positive");
        assert!(
            r.is_ok(),
            "sphere should have no errors; got {:?}",
            r.issues
        );
    }

    #[test]
    fn holed_model_passes_validation() {
        let model = Sdf::sphere(1.0).difference(Sdf::cylinder(0.4, 2.0));
        let mesh = polygonize(&model, Vec3::splat(-1.5), Vec3::splat(1.5), 40);
        let r = validate(&mesh, 0.01, 0.0);
        assert!(r.is_manifold);
        assert!(r.is_ok(), "holed model errors: {:?}", r.issues);
    }

    #[test]
    fn summary_format() {
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 16);
        let r = validate(&mesh, 0.0, 0.0);
        let s = r.summary();
        assert!(s.contains("manifold=true"));
        assert!(s.contains("errors=0"));
    }
}
