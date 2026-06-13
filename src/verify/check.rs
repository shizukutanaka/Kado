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
    let (boundary_edges, nonmanifold_edges) = mesh.edge_defects();
    let is_manifold = boundary_edges == 0 && nonmanifold_edges == 0;
    let tri_count = mesh.triangles.len();
    let mut issues = vec![];

    // 1. 開境界 (致命的): 表面が閉じていない。原因別にヒントを出す (問25/問3)。
    //    「解像度を上げよ」は開境界 (クリップ) には逆効果なので、境界拡大を案内する。
    if boundary_edges > 0 {
        issues.push(KadoError::error(
            "OPEN_MESH",
            format!(
                "surface is not closed: {boundary_edges} open boundary edge(s) \
                 (shape likely clipped by the sampling bounds, or has a zero-thickness feature)"
            ),
            &[
                "Enlarge the sampling/bounding region so the shape is fully enclosed",
                "Avoid zero-thickness walls; add offset()/shell thickness",
            ],
        ));
    }

    // 2. 非多様体接合 (致命的): 3面以上が同一エッジを共有 (自己交差・座標一致)。
    if nonmanifold_edges > 0 {
        issues.push(KadoError::error(
            "NON_MANIFOLD",
            format!(
                "{nonmanifold_edges} non-manifold edge(s) shared by >2 faces \
                 (self-intersection or coincident surfaces)"
            ),
            &[
                "Increase mesh resolution (higher res value)",
                "Separate coincident or self-intersecting geometry in the SDF tree",
            ],
        ));
    }

    // 3. 空メッシュ
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

    // 4. 負体積 (裏返し)
    if volume < 0.0 {
        issues.push(KadoError::warn(
            "NEGATIVE_VOLUME",
            format!("signed volume is negative ({volume:.3}), mesh may be inverted"),
            &["Check SDF field orientation; inner surface should have negative SDF"],
        ));
    }

    // 5. 肉厚チェック (2V/SA による平均肉厚。問23: 「平均」であり最小ではない)
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

    // 6. オーバーハング検査 (z 軸上向きビルド前提)
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
            // `worst` は最も大きい下向き法線の nz 成分 (負)。
            // オーバーハング角度 = 水平からの角度 = asin(-worst) (問38: acos(nz) は違う慣例)。
            // max_overhang_deg は「水平から」なので単位を揃える。
            let deg = (-worst).asin().to_degrees();
            issues.push(KadoError::warn(
                "OVERHANG",
                format!("overhang angle {deg:.1}° from horizontal exceeds max {max_overhang_deg:.1}°"),
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

    #[test]
    fn overhang_angle_reported_from_horizontal_not_from_z_axis() {
        // 問38: OVERHANG エラーは「水平からの角度」で報告されるべき。
        // max_overhang_deg=45 の規定と同じ慣例 (0°=垂直, 90°=水平面)。
        // 球にオーバーハング検査を適用すると、底半球の下向き法線が検出される。
        // 底点での nz = -1 → overhang = asin(1) = 90° from horizontal。
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        let r = validate(&mesh, 0.0, 45.0);
        let ov = r.issues.iter().find(|e| e.code == "OVERHANG");
        if let Some(e) = ov {
            // 角度は 45–90° の間にあるはず (水平換算)。
            // "XXX.X° from horizontal" の形式か確認。
            assert!(
                e.cause.contains("from horizontal"),
                "overhang must report angle from horizontal: {}",
                e.cause
            );
            // 角度が 90 以下であることを確認 (acos の誤用がないこと)。
            // フォーマット: "overhang angle XX.X° from horizontal ..."
            let deg: f64 = e
                .cause
                .split_whitespace()
                .nth(2)  // "overhang angle XX.X° ..."
                .map(|s| s.trim_end_matches('°'))
                .and_then(|s| s.parse().ok())
                .unwrap_or(999.0);
            assert!(
                deg <= 90.0,
                "overhang angle must be <= 90° from horizontal (FDM convention), got {deg}"
            );
        }
    }

    #[test]
    fn clipped_mesh_reports_open_mesh_with_bounds_hint() {
        // 問25/問3: クリップで開いたメッシュは OPEN_MESH として境界拡大を案内し、
        // 「解像度を上げよ」という誤誘導 (NON_MANIFOLD) にしない。
        let s = Sdf::sphere(1.0);
        let mesh = polygonize(
            &s,
            Vec3::new(-1.5, -1.5, -1.5),
            Vec3::new(1.5, 1.5, 0.0),
            24,
        );
        let r = validate(&mesh, 0.0, 0.0);
        assert!(!r.is_ok(), "clipped mesh must fail validation");
        let open = r.issues.iter().find(|e| e.code == "OPEN_MESH");
        assert!(open.is_some(), "clipping must be reported as OPEN_MESH");
        let hints = &open.unwrap().fix_hints;
        assert!(
            hints
                .iter()
                .any(|h| h.to_lowercase().contains("enclos")
                    || h.to_lowercase().contains("bounding")),
            "OPEN_MESH hint must guide enlarging bounds, got {hints:?}"
        );
    }
}
