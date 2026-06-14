//! 製造可能性 (DFM) 検査と構造化エラー。
//!
//! 各検査は [`KadoError`] のリストを返す。エラー ≒ 製造上の問題。
//! `fix_hints` は AI エージェントが自己修正ループを回すためのヒント (Plan §3)。

use crate::core::{Sdf, Vec3};
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
    /// 正準メッシュ内容ダイジェスト (FNV-1a 64bit, 問61)。再現性検証用。
    pub digest: u64,
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
        // 寸法を明示する (問62: 単位はミリメートル, 1 unit = 1 mm)。
        let d = hi - lo;
        format!(
            "triangles={} manifold={} volume={:.3} \
             bbox=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] \
             dims_mm=[{:.3}x{:.3}x{:.3}] \
             digest={:016x} errors={errors} warnings={warnings}",
            self.triangle_count,
            self.is_manifold,
            self.volume,
            lo.x,
            lo.y,
            lo.z,
            hi.x,
            hi.y,
            hi.z,
            d.x,
            d.y,
            d.z,
            self.digest,
        )
    }
}

// ── メインエントリポイント ────────────────────────────────────────────────────

/// メッシュを検証して [`Report`] を返す (メッシュのみ。肉厚は 2V/SA 平均)。
///
/// `min_wall_mm` は最小肉厚チェックの閾値 (0以下でスキップ)。
/// `max_overhang_deg` は最大オーバーハング角度 (度; 0以下でスキップ)。
pub fn validate(mesh: &Mesh, min_wall_mm: f64, max_overhang_deg: f64) -> Report {
    validate_with_field(mesh, None, min_wall_mm, max_overhang_deg)
}

/// SDF 場を併用して検証する。`sdf` を渡すと肉厚チェックに**内向きレイ探針**
/// (問58) を併用し、2V/SA 平均が見落とす**局所的な薄肉** (太い本体に付く細いリブ等)
/// を検出できる。`sdf=None` のときは [`validate`] と同じ (平均のみ)。
pub fn validate_with_field(
    mesh: &Mesh,
    sdf: Option<&Sdf>,
    min_wall_mm: f64,
    max_overhang_deg: f64,
) -> Report {
    let volume = mesh.signed_volume();
    let bbox = mesh.bounds();
    let (boundary_edges, nonmanifold_edges) = mesh.edge_defects();
    let is_manifold = boundary_edges == 0 && nonmanifold_edges == 0;
    let tri_count = mesh.triangles.len();
    let digest = mesh.digest();
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
            digest,
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

    // 4.5 複数ボディ検出 (問60): 水密でも独立した中実成分が複数あれば「単一造形物」でない。
    //     成分を符号付き体積で分類し、正=ボディ・負=空洞。空洞 (中空シェル) は正常なので
    //     ボディ数>1 のときのみ警告する (分割は意図的なこともあるため Error でなく Warning)。
    if is_manifold {
        let (bodies, cavities) = mesh.body_components();
        if bodies > 1 {
            issues.push(KadoError::warn(
                "MULTIPLE_BODIES",
                format!(
                    "watertight mesh contains {bodies} separate solid bodies \
                     ({cavities} internal cavit{}); this may be unintended \
                     (e.g. a gap that should have connected)",
                    if cavities == 1 { "y" } else { "ies" }
                ),
                &[
                    "If a single part was intended, bridge the gap (overlap shapes or add a connector)",
                    "If multiple parts are intended, this warning can be ignored",
                ],
            ));
        }
    }

    // 5. 肉厚チェック。2V/SA 平均 (問23) と、SDF があれば内向きレイ探針 (問58) の
    //    小さい方を採る。探針は局所的な薄肉を捉え、平均が太い本体に支配されて
    //    リブの薄さを見逃す弱点を補う。なお「閾値以上 = 薄肉なし」は依然非保証。
    if min_wall_mm > 0.0 {
        if let Some((lo, hi)) = bbox {
            let mean = mean_wall_thickness(mesh, lo, hi);
            let probe = sdf.and_then(|s| min_wall_probe(s, mesh, lo, hi));
            let thin = probe.map_or(mean, |p| p.min(mean));
            let method = if probe.is_some() {
                "min of 2V/SA mean and inward-ray probe"
            } else {
                "2V/SA average"
            };
            if thin < min_wall_mm {
                issues.push(KadoError::error(
                    "THIN_WALL",
                    format!(
                        "estimated wall thickness {thin:.3} < {min_wall_mm:.3} \
                         ({method}; a pass does not guarantee no local thin features)"
                    ),
                    &[
                        "Increase wall thickness via offset() or larger primitives",
                        "Reduce min_wall_mm threshold if intentional",
                    ],
                ));
            }

            // 5.5 スケール健全性 (問62): Kado 座標は mm (1 unit = 1 mm)。最大寸法が
            //     ユーザ自身の最小肉厚閾値すら下回るなら、形状全体が1壁より小さい =
            //     ほぼ確実に単位/スケールの誤り。絶対値でなく閾値相対なので恣意的でない。
            let max_dim = (hi - lo).max_component();
            if max_dim > 0.0 && max_dim < min_wall_mm {
                issues.push(KadoError::warn(
                    "SUSPICIOUS_SCALE",
                    format!(
                        "largest dimension {max_dim:.3} mm is smaller than the min wall \
                         {min_wall_mm:.3} mm — likely a units/scale error \
                         (Kado coordinates are millimeters; 1 unit = 1 mm)"
                    ),
                    &[
                        "Scale the model up (scale()) if it was authored in other units",
                        "Verify intended size: the whole part is currently sub-wall-thickness",
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
        digest,
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

/// 内向きレイ探針による**局所**肉厚の最小推定 (問58)。
///
/// 各表面頂点から内向き法線 (-∇SDF) 方向へ固定ステップで距離場を辿り、反対側の壁
/// (SDF が負→非負へ戻る点) までの距離を肉厚とみなし、全探針の最小を返す。2V/SA 平均が
/// 見落とす局所薄肉 (太い本体の細いリブ等) を捉える。探針数は上限で抑える。
///
/// 限界: ステップより薄い壁 (< diag/256) は跨いで見落としうる。よって検出は有効だが
/// 非検出は薄肉皆無を保証しない (平均と同じく安全側の補助)。
pub(crate) fn min_wall_probe(sdf: &Sdf, mesh: &Mesh, lo: Vec3, hi: Vec3) -> Option<f64> {
    let diag = (hi - lo).length();
    let v = mesh.vertices.len();
    if diag <= 0.0 || v == 0 {
        return None;
    }
    let step = diag / 256.0;
    let max_dist = diag * 1.2;
    // 過剰計算を避けるため探針数を上限で間引く。
    let cap = 30_000usize;
    let stride = v.div_ceil(cap);

    let mut min_t = f64::INFINITY;
    let mut i = 0;
    while i < v {
        let p = mesh.vertices[i];
        i += stride;
        let g = sdf_gradient(sdf, p);
        let gl = g.length();
        if gl < 1e-12 {
            continue;
        }
        let inward = g * (-1.0 / gl);
        // 内側 (負) を確認してから最初の 0 跨ぎを探す。
        let mut t = 0.0;
        let mut prev_d = sdf.eval(p);
        let mut went_inside = prev_d < 0.0;
        while t < max_dist {
            t += step;
            let d = sdf.eval(p + inward * t);
            if went_inside && d >= 0.0 {
                // prev_d (<0) と d (>=0) を線形補間して交点距離を求める。
                let cross = (t - step) + (-prev_d) / (d - prev_d) * step;
                if cross > 0.0 && cross < min_t {
                    min_t = cross;
                }
                break;
            }
            if d < 0.0 {
                went_inside = true;
            }
            prev_d = d;
        }
    }
    min_t.is_finite().then_some(min_t)
}

/// 中心差分による SDF 勾配 (外向き)。内向き法線は `-gradient`。
fn sdf_gradient(sdf: &Sdf, p: Vec3) -> Vec3 {
    let h = 1e-4;
    Vec3::new(
        sdf.eval(p + Vec3::new(h, 0.0, 0.0)) - sdf.eval(p - Vec3::new(h, 0.0, 0.0)),
        sdf.eval(p + Vec3::new(0.0, h, 0.0)) - sdf.eval(p - Vec3::new(0.0, h, 0.0)),
        sdf.eval(p + Vec3::new(0.0, 0.0, h)) - sdf.eval(p - Vec3::new(0.0, 0.0, h)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Sdf;
    use crate::extract::polygonize;

    #[test]
    fn summary_reports_physical_dimensions_in_mm() {
        // 問62: 要約に dims_mm が含まれ、実寸 (= bbox の幅) を反映する。
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        let s = validate(&mesh, 0.0, 0.0).summary();
        assert!(s.contains("dims_mm="), "summary must expose physical dims: {s}");
        // 半径1の球 → 約 2x2x2 mm。
        assert!(s.contains("dims_mm=[2."), "diameter ~2mm expected: {s}");
    }

    #[test]
    fn suspicious_scale_warns_when_part_smaller_than_its_own_min_wall() {
        // 問62: 最大寸法が min_wall すら下回る = 単位/スケール誤りの可能性。
        // 直径 0.2mm の球に min_wall=0.5mm を課すと SUSPICIOUS_SCALE。
        let tiny = Sdf::sphere(0.1);
        let (lo, hi) = tiny.sampling_box();
        let r = validate(&polygonize(&tiny, lo, hi, 16), 0.5, 0.0);
        assert!(
            r.issues.iter().any(|e| e.code == "SUSPICIOUS_SCALE"),
            "sub-wall-sized part must warn SUSPICIOUS_SCALE"
        );

        // 通常サイズ (直径 2mm) では出ない。
        let ok = Sdf::sphere(1.0);
        let (lo, hi) = ok.sampling_box();
        let r = validate(&polygonize(&ok, lo, hi, 24), 0.5, 0.0);
        assert!(
            !r.issues.iter().any(|e| e.code == "SUSPICIOUS_SCALE"),
            "normal-sized part must not warn"
        );
    }

    #[test]
    fn multiple_bodies_warns_but_hollow_shell_does_not() {
        // 問60: 離れた2球は MULTIPLE_BODIES 警告。中空シェル (1ボディ+1空洞) は出さない。
        let two = Sdf::sphere(0.6)
            .translate(Vec3::new(-1.2, 0.0, 0.0))
            .union(Sdf::sphere(0.6).translate(Vec3::new(1.2, 0.0, 0.0)));
        let (lo, hi) = two.sampling_box();
        let r = validate(&polygonize(&two, lo, hi, 40), 0.0, 0.0);
        assert!(
            r.issues.iter().any(|e| e.code == "MULTIPLE_BODIES"),
            "two disjoint solids must warn MULTIPLE_BODIES"
        );

        let shell = Sdf::sphere(1.0).shell(0.25);
        let (lo, hi) = shell.sampling_box();
        let r = validate(&polygonize(&shell, lo, hi, 48), 0.0, 0.0);
        assert!(
            !r.issues.iter().any(|e| e.code == "MULTIPLE_BODIES"),
            "hollow shell is one body + one cavity, must NOT warn"
        );
    }

    #[test]
    fn probe_measures_shell_thickness() {
        // 問58: 厚さ 0.2 のシェルの最小肉厚 ≈ 0.2 を内向きレイ探針が測れる。
        let sdf = Sdf::sphere(1.0).shell(0.2);
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 48);
        let t = min_wall_probe(&sdf, &mesh, lo, hi).expect("probe must return a value");
        assert!(
            (t - 0.2).abs() < 0.06,
            "shell thickness probe should be ~0.2, got {t}"
        );
    }

    #[test]
    fn probe_reports_large_thickness_for_solid_sphere() {
        // 中実球には薄肉がない。探針はおおむね直径 (≈2.0) を返し、薄肉と誤判定しない。
        let sdf = Sdf::sphere(1.0);
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 40);
        let t = min_wall_probe(&sdf, &mesh, lo, hi).unwrap();
        assert!(t > 1.0, "solid sphere min wall should be large (~2.0), got {t}");
    }

    #[test]
    fn probe_catches_local_thin_fin_that_mean_misses() {
        // 問58 の核心: 太い本体 (2×2×2) に薄いフィン (厚さ 0.1) が付く形状。
        // 2V/SA 平均は本体に支配されて薄肉を見逃すが、探針はフィンの 0.1 を捉える。
        let body = Sdf::cuboid(Vec3::new(1.0, 1.0, 1.0));
        let fin = Sdf::cuboid(Vec3::new(1.8, 0.05, 0.8));
        let sdf = body.union(fin);
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 48);

        let mean = mean_wall_thickness(&mesh, lo, hi);
        let probe = min_wall_probe(&sdf, &mesh, lo, hi).unwrap();
        assert!(
            mean > 0.2,
            "2V/SA mean should be dominated by the body (>0.2), got {mean}"
        );
        assert!(
            probe < 0.18,
            "probe must catch the thin fin (~0.1), got {probe}"
        );

        // 場併用 validate は THIN_WALL を報告し、メッシュのみは見逃す (閾値 0.2)。
        let with_field = validate_with_field(&mesh, Some(&sdf), 0.2, 0.0);
        let mesh_only = validate(&mesh, 0.2, 0.0);
        assert!(
            with_field.issues.iter().any(|e| e.code == "THIN_WALL"),
            "field-aware validate must flag the thin fin"
        );
        assert!(
            !mesh_only.issues.iter().any(|e| e.code == "THIN_WALL"),
            "mesh-only mean check misses the thin fin (demonstrates added value)"
        );
    }

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
        // 問61: 再現性検証のためダイジェストが要約に含まれる。
        assert!(
            s.contains(&format!("digest={:016x}", r.digest)),
            "summary must expose the content digest: {s}"
        );
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
