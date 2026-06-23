//! 製造可能性 (DFM) 検査と構造化エラー。
//!
//! 各検査は [`KadoError`] のリストを返す。エラー ≒ 製造上の問題。
//! `fix_hints` は AI エージェントが自己修正ループを回すためのヒント (Plan §3)。

use crate::core::{Sdf, Vec3};
use crate::extract::Mesh;
use crate::mcp::json::{self, Value};

/// validator が emit しうる全 issue code の正準リスト (問103: 単一の真実源)。
///
/// 新しい issue code を追加するときは必ずここにも追加すること。
/// `issue_codes_are_fully_documented` テストが、この全コードが MCP の help と
/// validate スキーマの双方に記載されていることを強制する (文書ドリフト防止)。
pub const ALL_ISSUE_CODES: &[&str] = &[
    "EMPTY_MESH",
    "OPEN_MESH",
    "NON_MANIFOLD",
    "NEGATIVE_VOLUME",
    "MULTIPLE_BODIES",
    "THIN_WALL",
    "SUSPICIOUS_SCALE",
    "OVERHANG",
    "UNSTABLE",
    "HIGH_ASPECT_RATIO",
];

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
    /// 問題が特定の空間座標に関連する場合の位置ヒント (問242)。
    /// OVERHANG: 最悪三角形の重心。THIN_WALL: プローブが最小肉厚を検出した頂点。
    /// UNSTABLE: 重心 (= 問題の発生点)。空間的でない問題は None。
    /// AI エージェントは `location` で「どこを直すか」を直接参照できる。
    pub location: Option<Vec3>,
}

impl KadoError {
    fn error(code: &'static str, cause: impl Into<String>, hints: &[&str]) -> KadoError {
        KadoError {
            severity: Severity::Error,
            code,
            cause: cause.into(),
            fix_hints: hints.iter().map(|s| s.to_string()).collect(),
            location: None,
        }
    }
    fn warn(code: &'static str, cause: impl Into<String>, hints: &[&str]) -> KadoError {
        KadoError {
            severity: Severity::Warning,
            code,
            cause: cause.into(),
            fix_hints: hints.iter().map(|s| s.to_string()).collect(),
            location: None,
        }
    }
    /// 空間位置ヒントを付加する。Builder パターン (問242)。
    pub fn with_location(mut self, loc: Vec3) -> Self {
        self.location = Some(loc);
        self
    }
}

// ── 検証レポート ──────────────────────────────────────────────────────────────

#[derive(Debug)]
#[derive(Clone)]
pub struct Report {
    /// 符号付き体積 (mm³ または 任意単位)。
    pub volume: f64,
    /// メッシュ表面積 (mm² または 任意単位の二乗, 問244)。
    /// FDM 造形時間・材料費の主要因。体積と並ぶ基本幾何量として公開する。
    /// 体積と異なり開境界メッシュでも常に意味を持つ (三角形面積の単純和)。
    pub surface_area: f64,
    /// 軸整列バウンディングボックス [min, max]。
    pub bbox: Option<(Vec3, Vec3)>,
    /// 三角形数。
    pub triangle_count: usize,
    /// edge-manifold (水密) かどうか。
    pub is_manifold: bool,
    /// 正準メッシュ内容ダイジェスト (FNV-1a 64bit, 問61)。再現性検証用。
    pub digest: u64,
    /// 一様密度を仮定した重心 (問239)。空メッシュ・退化メッシュは None。
    /// UNSTABLE 判定の根拠数値として公開し、AI/利用者が「どこに重心があるか」を
    /// 直接確認できるようにする。発散定理ベースの計算 (center_of_mass と同系統)。
    pub center_of_mass: Option<Vec3>,
    /// DFM 問題リスト。
    pub issues: Vec<KadoError>,
}

impl Report {
    /// 問題なし (エラー0) なら true。
    pub fn is_ok(&self) -> bool {
        self.issues.iter().all(|e| e.severity != Severity::Error)
    }

    /// 体積が信頼できるか (問65)。発散定理による体積は**閉じた**メッシュでのみ意味を持つ。
    /// 開境界 (OPEN_MESH) や空メッシュでは符号付き体積は無意味なので、AI/利用者が
    /// 誤って信頼しないよう明示する。
    pub fn volume_reliable(&self) -> bool {
        self.is_manifold && self.triangle_count > 0
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
        let (lo, hi) = self.bbox.unwrap_or((Vec3::ZERO, Vec3::ZERO));
        // 寸法を明示する (問62: 単位はミリメートル, 1 unit = 1 mm)。
        let d = hi - lo;
        // 体積は閉じたメッシュでのみ有効 (問65)。
        let vol_note = if self.volume_reliable() { "" } else { "(unreliable: not closed)" };
        // 重心は存在するときのみ付加する (問239)。
        let com_str = self.center_of_mass.map_or(String::new(), |c| {
            format!(" com=[{:.3},{:.3},{:.3}]", c.x, c.y, c.z)
        });
        format!(
            "triangles={} manifold={} volume={:.3}{vol_note} area={:.3} \
             bbox=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] \
             dims_mm=[{:.3}x{:.3}x{:.3}] \
             digest={:016x} errors={errors} warnings={warnings}{com_str}",
            self.triangle_count,
            self.is_manifold,
            self.volume,
            self.surface_area,
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

    /// 機械可読な構造化レポート (問63)。AI が `code` で分岐し数値指標を直接読めるよう、
    /// 自由文字列ではなく JSON で返す。`mcp::json` を汎用 JSON ユーティリティとして用いる。
    pub fn to_json(&self) -> Value {
        let (lo, hi) = self.bbox.unwrap_or((Vec3::ZERO, Vec3::ZERO));
        let d = hi - lo;
        let vec3 = |v: Vec3| json::arr([json::n(v.x), json::n(v.y), json::n(v.z)]);
        let issues: Vec<Value> = self
            .issues
            .iter()
            .map(|e| {
                json::obj([
                    // 問82: Debug 形式 "Error"/"Warning" (PascalCase) の代わりに
                    // 小文字 "error"/"warning" を使う。AI が文字列比較しやすい標準形式。
                    ("severity", json::s(match e.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        Severity::Info => "info",
                    })),
                    ("code", json::s(e.code)),
                    ("cause", json::s(e.cause.clone())),
                    (
                        "hints",
                        Value::Array(e.fix_hints.iter().map(|h| json::s(h.clone())).collect()),
                    ),
                    // 問242: 空間位置ヒント。None なら null。
                    ("location", e.location.map_or(json::NULL, vec3)),
                ])
            })
            .collect();
        let bbox = if self.bbox.is_some() {
            json::obj([("min", vec3(lo)), ("max", vec3(hi))])
        } else {
            json::NULL
        };
        // 重心: Some なら [x,y,z] 配列、None なら null (問239)。
        let com_json = self
            .center_of_mass
            .map_or(json::NULL, vec3);
        json::obj([
            ("ok", json::b(self.is_ok())),
            ("triangles", json::n(self.triangle_count as f64)),
            ("manifold", json::b(self.is_manifold)),
            ("volume", json::n(self.volume)),
            ("volume_reliable", json::b(self.volume_reliable())),
            ("surface_area", json::n(self.surface_area)),
            ("bbox", bbox),
            ("dims_mm", vec3(d)),
            ("center_of_mass", com_json),
            ("digest", json::s(format!("{:016x}", self.digest))),
            ("issues", Value::Array(issues)),
        ])
    }
}

// ── メインエントリポイント ────────────────────────────────────────────────────

/// メッシュを検証して [`Report`] を返す (メッシュのみ。肉厚は 2V/SA 平均)。
///
/// `min_wall_mm` は最小肉厚チェックの閾値 (0以下でスキップ)。
/// `max_overhang_deg` は最大オーバーハング角度 (度; 0以下でスキップ)。
/// ビルド方向は +Z (= デフォルト: 重力と反対方向)。
pub fn validate(mesh: &Mesh, min_wall_mm: f64, max_overhang_deg: f64) -> Report {
    validate_with_field(mesh, None, min_wall_mm, max_overhang_deg, Vec3::new(0.0, 0.0, 1.0))
}

/// SDF 場を併用して検証する。`sdf` を渡すと肉厚チェックに**内向きレイ探針**
/// (問58) を併用し、2V/SA 平均が見落とす**局所的な薄肉** (太い本体に付く細いリブ等)
/// を検出できる。`sdf=None` のときは [`validate`] と同じ (平均のみ)。
///
/// `build_dir`: ビルド方向ベクトル (正規化不要; 零ベクトルならオーバーハング検査をスキップ)。
/// デフォルトは `Vec3::new(0,0,1)` (+Z 上向き = 多くの FDM プリンタの方向)。
/// 問68: この軸が暗黙だったため、AI が誤った方向でオーバーハングを評価していた。
pub fn validate_with_field(
    mesh: &Mesh,
    sdf: Option<&Sdf>,
    min_wall_mm: f64,
    max_overhang_deg: f64,
    build_dir: Vec3,
) -> Report {
    let volume = mesh.signed_volume();
    let surface_area = mesh.surface_area();
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
            surface_area,
            bbox,
            triangle_count: tri_count,
            is_manifold,
            digest,
            center_of_mass: None, // 空メッシュに COM なし。
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
            let probe_t = probe.map(|(t, _)| t);
            let probe_loc = probe.map(|(_, loc)| loc);
            let thin = probe_t.map_or(mean, |p| p.min(mean));
            let method = if probe.is_some() {
                "min of 2V/SA mean and inward-ray probe"
            } else {
                "2V/SA average"
            };
            if thin < min_wall_mm {
                // 探針が平均より薄い値を検出した場合のみ位置ヒントを付与 (問242)。
                // 探針位置は「直す必要がある薄肉の表面点」を示す。
                let thin_loc = probe_t.and_then(|p| if p <= mean { probe_loc } else { None });
                let mut issue = KadoError::error(
                    "THIN_WALL",
                    format!(
                        "estimated wall thickness {thin:.3} < {min_wall_mm:.3} \
                         ({method}; a pass does not guarantee no local thin features)"
                    ),
                    &[
                        "Increase wall thickness via offset() or larger primitives",
                        "Reduce min_wall_mm threshold if intentional",
                    ],
                );
                if let Some(loc) = thin_loc {
                    issue = issue.with_location(loc);
                }
                issues.push(issue);
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

    // 6. オーバーハング検査 (問68: build_dir で方向を明示。以前は +Z 暗黙固定だった)
    //    dot(n̂, bd̂) = +1 → 完全にビルド方向向き (上向き), -1 → 完全に逆 (下向き・最悪オーバーハング)。
    if max_overhang_deg > 0.0 {
        let bd_len = build_dir.length();
        if bd_len > 1e-12 {
            let bd = build_dir * (1.0 / bd_len);
            let max_cos = (90.0_f64 - max_overhang_deg).to_radians().cos();
            // 問237: ビルド方向への投影範囲を求める。最下層 (= ベッド接地面) は
            // 下向きでもベッドが支えるためオーバーハングではない。flatten/cut で作った
            // 平坦底面を誤って OVERHANG 警告する偽陽性を防ぐ。
            let mut min_proj = f64::INFINITY;
            let mut max_proj = f64::NEG_INFINITY;
            for v in &mesh.vertices {
                let pr = v.dot(bd);
                min_proj = min_proj.min(pr);
                max_proj = max_proj.max(pr);
            }
            let height = max_proj - min_proj;
            // (a) ベッド接地許容: 平坦底面は厳密に min_proj。薄い最下層 (1%) を吸収。
            let bed_eps = (height * 0.01).max(1e-9);
            let mut worst: f64 = 0.0;
            let mut worst_centroid = Vec3::ZERO; // 問242: 最悪三角形の重心。
            for tri in &mesh.triangles {
                let a = mesh.vertices[tri[0] as usize];
                let b = mesh.vertices[tri[1] as usize];
                let c = mesh.vertices[tri[2] as usize];
                let n = (b - a).cross(c - a);
                let len = n.length();
                if len < 1e-15 {
                    continue;
                }
                // n̂ と bd̂ の内積: 負値 = ビルド方向と逆向き (下向き面 = オーバーハング)。
                let nd = n.dot(bd) / len;
                if nd >= 0.0 {
                    continue; // 上向き・水平面はオーバーハングでない。
                }
                // (a) ベッド接地面 (最下層) は支持されるため除外。
                let centroid = (a + b + c) * (1.0 / 3.0);
                let above_bed = centroid.dot(bd) - min_proj;
                if above_bed <= bed_eps {
                    continue;
                }
                // (b) 重心の真下・ベッド直上に形状材料があれば、ベッドから材料が立ち上がって
                //     この面を支えている → オーバーハングでない (SDF 併用時のみ)。
                //     重心を min_proj+bed_eps の高さへ落とした点で判定する (固定ステップだと
                //     低い面でベッド下へ突き抜けるため、ベッド直上の固定高さで標本化する)。
                //     これにより平坦底面と壁の鋭い凸エッジ (flatten/cut の rim) の遷移三角形
                //     (直下に底面材料あり) を支持済みと正しく扱う。物理的に正しい支持判定。
                if let Some(field) = sdf {
                    let sample = centroid - bd * (above_bed - bed_eps);
                    if field.eval(sample) < 0.0 {
                        continue;
                    }
                }
                if nd < worst {
                    worst = nd;
                    worst_centroid = centroid; // 問242: 最悪三角形の重心を更新。
                }
            }
            if worst < -max_cos {
                // オーバーハング角度 = 水平からの角度 = asin(-worst) (問38)。
                let deg = (-worst).asin().to_degrees();
                issues.push(
                    KadoError::warn(
                        "OVERHANG",
                        format!(
                            "overhang angle {deg:.1}° from horizontal exceeds {max_overhang_deg:.1}° \
                             (build direction [{:.2},{:.2},{:.2}])",
                            bd.x, bd.y, bd.z
                        ),
                        &[
                            "Add support structures or redesign with chamfer/fillet",
                            "Rotate the model to minimize overhangs",
                        ],
                    )
                    .with_location(worst_centroid), // 問242: 最悪三角形の重心を位置ヒントとして付与。
                );
            }
        }
    }

    // 7. 安定性検査 (問238: 新視点「物理挙動」)。製造可能性 (DFM) とは別軸で、
    //    「作った後に物理的に成立するか」を問う。重心 (COM) がベース接地面の
    //    足元 (フットプリント) から外れると、印刷後/取り扱い時に転倒する。
    //    フットプリント bbox は真の支持多角形 (凸包) の外接近似なので、「bbox の外」は
    //    転倒の十分条件 → 偽陽性のない保守的警告になる (「内」は安定を保証しない)。
    //
    // COM は UNSTABLE 判定の根拠として Report にも公開する (問239)。
    // AI/利用者が「重心がどこにあるか」を直接確認できるようにする。
    let center_of_mass = mesh.center_of_mass();
    {
        let bd_len = build_dir.length();
        if bd_len > 1e-12 {
            if let Some(com) = center_of_mass {
                let bd = build_dir * (1.0 / bd_len);
                let mut min_proj = f64::INFINITY;
                let mut max_proj = f64::NEG_INFINITY;
                for v in &mesh.vertices {
                    let pr = v.dot(bd);
                    min_proj = min_proj.min(pr);
                    max_proj = max_proj.max(pr);
                }
                let bed_eps = ((max_proj - min_proj) * 0.02).max(1e-9);
                // bd に垂直な平面での横方向成分。接地頂点の横方向 bbox を支持域とする。
                let lat = |p: Vec3| p - bd * p.dot(bd);
                let mut lo = Vec3::splat(f64::INFINITY);
                let mut hi = Vec3::splat(f64::NEG_INFINITY);
                let mut n_contact = 0u32;
                for v in &mesh.vertices {
                    if v.dot(bd) - min_proj <= bed_eps {
                        let l = lat(*v);
                        lo = lo.min(l);
                        hi = hi.max(l);
                        n_contact += 1;
                    }
                }
                if n_contact > 0 {
                    let cl = lat(com);
                    // フットプリント対角の 2% を境界ジッタ許容とし、明確に外れた場合のみ警告。
                    let tol = ((hi - lo).length() * 0.02).max(1e-9);
                    let outside = cl.x < lo.x - tol
                        || cl.x > hi.x + tol
                        || cl.y < lo.y - tol
                        || cl.y > hi.y + tol
                        || cl.z < lo.z - tol
                        || cl.z > hi.z + tol;
                    if outside {
                        issues.push(
                            KadoError::warn(
                                "UNSTABLE",
                                format!(
                                    "center of mass projects outside the base footprint \
                                     (build direction [{:.2},{:.2},{:.2}]) — the part may tip over",
                                    bd.x, bd.y, bd.z
                                ),
                                &[
                                    "Widen the base, or lower/center the center of mass",
                                    "Reorient the part so the COM sits over the support footprint",
                                ],
                            )
                            .with_location(com), // 問242: COM が問題の空間的起点。
                        );
                    }
                }
            }
        }
    }

    // 8. 高アスペクト比検査 (問241: 印刷プロセス中の揺れリスク)。
    //    UNSTABLE (印刷後の転倒: 物理挙動) と相補的な新視点 — 製造プロセスの安定性。
    //    FDM では高く細い形状がノズル通過時に振動し、層間剥離や転倒を引き起こす。
    //    高さ / 横方向最大寸法 > 8 は業界ガイドラインの目安。
    //    bd に垂直な平面での頂点群バウンディングボックスを横方向尺度とする。
    const HIGH_ASPECT_RATIO_THRESHOLD: f64 = 8.0;
    {
        let bd_len = build_dir.length();
        if bd_len > 1e-12 {
            let bd = build_dir * (1.0 / bd_len);
            let lat = |p: Vec3| p - bd * p.dot(bd);
            let mut min_p = f64::INFINITY;
            let mut max_p = f64::NEG_INFINITY;
            let mut lat_lo = Vec3::splat(f64::INFINITY);
            let mut lat_hi = Vec3::splat(f64::NEG_INFINITY);
            for v in &mesh.vertices {
                let pr = v.dot(bd);
                min_p = min_p.min(pr);
                max_p = max_p.max(pr);
                let l = lat(*v);
                lat_lo = lat_lo.min(l);
                lat_hi = lat_hi.max(l);
            }
            let height = max_p - min_p;
            // bd に垂直な平面での最大横幅 (bbox の最長辺)。
            let lateral_max = (lat_hi - lat_lo).max_component();
            if height > 0.0 && lateral_max > 0.0 {
                let ratio = height / lateral_max;
                if ratio > HIGH_ASPECT_RATIO_THRESHOLD {
                    issues.push(KadoError::warn(
                        "HIGH_ASPECT_RATIO",
                        format!(
                            "build height {height:.1} mm / lateral size {lateral_max:.1} mm = \
                             aspect ratio {ratio:.1} (threshold {HIGH_ASPECT_RATIO_THRESHOLD:.0}) \
                             — tall thin parts sway under the print nozzle and may delaminate \
                             (build direction [{:.2},{:.2},{:.2}])",
                            bd.x, bd.y, bd.z
                        ),
                        &[
                            "Reorient the part to print along the longest axis (rotate 90°)",
                            "Add a wider brim or raft for better bed adhesion",
                            "Reduce print speed for tall thin features",
                        ],
                    ));
                }
            }
        }
    }

    Report {
        volume,
        surface_area,
        bbox,
        triangle_count: tri_count,
        is_manifold,
        digest,
        center_of_mass,
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
    let surface_area = mesh.surface_area();
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
/// 内向きレイ探針による最小肉厚の推定と、最小を検出した表面頂点の位置。
/// 戻り値: `Some((thickness_mm, surface_vertex))` または `None`。
/// surface_vertex は AI エージェントが「どこを修正するか」を特定するための空間ヒント (問242)。
pub(crate) fn min_wall_probe(sdf: &Sdf, mesh: &Mesh, lo: Vec3, hi: Vec3) -> Option<(f64, Vec3)> {
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
    let mut min_loc = Vec3::ZERO;
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
                    min_loc = p; // 最小を検出した表面頂点 (問242)。
                }
                break;
            }
            if d < 0.0 {
                went_inside = true;
            }
            prev_d = d;
        }
    }
    min_t.is_finite().then_some((min_t, min_loc))
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
    fn volume_is_marked_unreliable_for_open_mesh() {
        // 問65: 閉じたメッシュは体積信頼可、クリップで開いたメッシュは不可。
        let s = Sdf::sphere(1.0);
        let closed = polygonize(&s, Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        let r = validate(&closed, 0.0, 0.0);
        assert!(r.volume_reliable(), "closed mesh volume must be reliable");
        assert!(r.to_json().to_string().contains("\"volume_reliable\":true"));

        // z=0 でクリップして開境界を作る。
        let open = polygonize(&s, Vec3::new(-1.5, -1.5, -1.5), Vec3::new(1.5, 1.5, 0.0), 24);
        let r = validate(&open, 0.0, 0.0);
        assert!(!r.volume_reliable(), "open mesh volume must be flagged unreliable");
        assert!(r.summary().contains("unreliable"), "summary must warn: {}", r.summary());
    }

    #[test]
    fn to_json_is_machine_readable_and_carries_codes() {
        // 問63: 構造化レポートが再パース可能で、code/severity/指標を保持する。
        use crate::mcp::json::parse;
        // 薄肉になる穴あき球で THIN_WALL を誘発。
        let model = Sdf::sphere(1.0).difference(Sdf::cylinder(0.9, 2.0));
        let (lo, hi) = model.sampling_box();
        let mesh = polygonize(&model, lo, hi, 40);
        let report = validate(&mesh, 0.5, 0.0);
        let v = report.to_json();
        // 文字列化 → 再パースして往復が壊れないこと。
        let reparsed = parse(&v.to_string()).expect("report JSON must be valid");
        assert_eq!(reparsed, v, "to_json must round-trip through parse");
        // 必須フィールド。
        assert!(v.get("ok").and_then(|x| x.as_bool()).is_some());
        assert!(v.get("digest").and_then(|x| x.as_str()).is_some());
        assert!(v.get("dims_mm").and_then(|x| x.as_array()).is_some());
        let issues = v.get("issues").and_then(|x| x.as_array()).unwrap();
        // 少なくとも1つの issue が code/severity/hints を持つ。
        assert!(
            issues
                .iter()
                .all(|e| e.get("code").is_some() && e.get("severity").is_some()),
            "every issue must carry a machine-readable code and severity"
        );
        // ok は issues のエラー有無と整合 (問82: severity は小文字 "error"/"warning")。
        let has_error = issues
            .iter()
            .any(|e| e.get("severity").and_then(|s| s.as_str()) == Some("error"));
        assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(!has_error));
    }

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
    fn summary_dims_mm_uses_exactly_three_decimal_places() {
        // 問219: summary は dims_mm=[{:.3}x{:.3}x{:.3}] で 3 桁固定精度。
        // summary_reports_physical_dimensions_in_mm は "dims_mm=[2." 接頭辞のみ確認し、
        // 精度桁数 (3 桁) を検証していなかった。各成分が厳密に 3 桁小数であることを固定。
        let mesh = polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        let s = validate(&mesh, 0.0, 0.0).summary();
        let start = s.find("dims_mm=[").expect("dims_mm must be present") + "dims_mm=[".len();
        let end = s[start..].find(']').expect("dims_mm must close with ]") + start;
        let body = &s[start..end]; // 例: "2.000x2.000x2.000"
        let parts: Vec<&str> = body.split('x').collect();
        assert_eq!(parts.len(), 3, "dims_mm must have 3 components: {body}");
        for part in parts {
            let dot = part.find('.').unwrap_or_else(|| panic!("each dim must have a decimal point: {part}"));
            let decimals = &part[dot + 1..];
            assert_eq!(
                decimals.len(),
                3,
                "each dim must have exactly 3 decimal places, got '{part}' ({} decimals)",
                decimals.len()
            );
        }
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
        let (t, _loc) = min_wall_probe(&sdf, &mesh, lo, hi).expect("probe must return a value");
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
        let (t, _loc) = min_wall_probe(&sdf, &mesh, lo, hi).unwrap();
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
        let (probe, probe_loc) = min_wall_probe(&sdf, &mesh, lo, hi).unwrap();
        assert!(
            mean > 0.2,
            "2V/SA mean should be dominated by the body (>0.2), got {mean}"
        );
        assert!(
            probe < 0.18,
            "probe must catch the thin fin (~0.1), got {probe}"
        );
        // 問242: プローブ位置がフィン領域 (|y| < 0.15) にあることを確認。
        assert!(
            probe_loc.y.abs() < 0.15,
            "probe location must be in the thin fin (|y|<0.15), got {probe_loc:?}"
        );

        // 場併用 validate は THIN_WALL を報告し、メッシュのみは見逃す (閾値 0.2)。
        let with_field = validate_with_field(&mesh, Some(&sdf), 0.2, 0.0, Vec3::new(0.0, 0.0, 1.0));
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

    #[test]
    fn overhang_check_respects_build_direction() {
        // 問68: build_dir パラメータが OVERHANG 検出に使われていることを確認する。
        //
        // 証明戦略: 球は全方向に面を持つため、+Z / +X ビルドともに
        // OVERHANG を検出する (両者を区別するためではない)。
        // しかし「どの面が問題か」はビルド方向で決まるため、
        // エラーメッセージに含まれる build_dir ベクトルが異なる値になる。
        // これが build_dir が実際に使われている証拠となる。
        //
        // 追加テスト: max_overhang_deg=0 → オーバーハング検査がスキップされ、
        //   build_dir の値に関係なく OVERHANG が報告されないことで、
        //   build_dir の零長判定分岐も正しく動作することを確認する。
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 24);

        // +Z ビルド, 閾値 45°: 南半球の面 (nz < -cos45°) が検出される。
        let r_z = validate_with_field(&mesh, None, 0.0, 45.0, Vec3::new(0.0, 0.0, 1.0));
        // +X ビルド, 閾値 45°: 西半球の面 (nX < -cos45°) が検出される。
        let r_x = validate_with_field(&mesh, None, 0.0, 45.0, Vec3::new(1.0, 0.0, 0.0));

        // 両ビルドとも OVERHANG を検出する (球は全方向に南半球を持つ)。
        let ov_z = r_z.issues.iter().find(|e| e.code == "OVERHANG").expect(
            "+Z build must detect sphere underside overhang",
        );
        let ov_x = r_x.issues.iter().find(|e| e.code == "OVERHANG").expect(
            "+X build must detect sphere -X-facing overhang",
        );

        // 問68 の核心: エラーメッセージが build_dir ベクトルを含む。
        assert!(
            ov_z.cause.contains("build direction"),
            "+Z report must name build direction: {}",
            ov_z.cause
        );
        assert!(
            ov_x.cause.contains("build direction"),
            "+X report must name build direction: {}",
            ov_x.cause
        );
        // 各レポートが異なる build_dir を埋め込んでいることを確認。
        // +Z: "build direction [0.00,0.00,1.00]"
        // +X: "build direction [1.00,0.00,0.00]"
        assert!(
            ov_z.cause.contains("[0.00,0.00,1.00]"),
            "+Z report must embed (0,0,1) direction: {}",
            ov_z.cause
        );
        assert!(
            ov_x.cause.contains("[1.00,0.00,0.00]"),
            "+X report must embed (1,0,0) direction: {}",
            ov_x.cause
        );

        // max_overhang_deg=0 → OVERHANG 検査スキップ (build_dir は無関係)。
        let r_skip = validate_with_field(&mesh, None, 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            !r_skip.issues.iter().any(|e| e.code == "OVERHANG"),
            "max_overhang_deg=0 must disable overhang check entirely"
        );
    }

    #[test]
    fn flat_printable_base_is_not_flagged_as_overhang() {
        // 問237: flatten/cut で作った平坦底面 (法線 -Z・最下層) はベッドが支えるため
        // オーバーハングではない。以前は下向き面を一律に OVERHANG 警告する偽陽性があった。
        // 平坦底面ドーム (球を z=0 でカット) は +Z ビルドで OVERHANG を出さないこと。
        // flatten(at=0) ≡ cut(normal=(0,0,-1), offset=0): z>=0 を残す平坦底面ドーム。
        // SDF を渡すと直下材料判定 (b) が rim の遷移三角形を支持済みと正しく扱う。
        let dome = Sdf::sphere(1.0).cut(Vec3::new(0.0, 0.0, -1.0), 0.0);
        let (lo, hi) = dome.sampling_box();
        let mesh = polygonize(&dome, lo, hi, 32);
        let r = validate_with_field(&mesh, Some(&dome), 0.0, 45.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            !r.issues.iter().any(|e| e.code == "OVERHANG"),
            "flat base + bed-supported rim must NOT be flagged as overhang: {:?}",
            r.issues.iter().map(|e| &e.cause).collect::<Vec<_>>()
        );

        // 偽陰性を作っていないことの対照: 平坦化していない球 (真の底面オーバーハング・
        // 下に材料なし) は依然 OVERHANG を検出する。
        let sphere = Sdf::sphere(1.0);
        let (slo, shi) = sphere.sampling_box();
        let smesh = polygonize(&sphere, slo, shi, 32);
        let rs = validate_with_field(&smesh, Some(&sphere), 0.0, 45.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            rs.issues.iter().any(|e| e.code == "OVERHANG"),
            "a real curved underside (un-flattened sphere) must still be flagged"
        );
    }

    #[test]
    fn top_heavy_offset_part_is_flagged_unstable_but_centered_dome_is_not() {
        // 問238 (新視点・物理挙動): 重心がベース足元から外れる部品は転倒する。
        // 対称な平坦底面ドームは安定 (COM が底面中央上) → 警告なし。
        let dome = Sdf::sphere(1.0).cut(Vec3::new(0.0, 0.0, -1.0), 0.0);
        let (lo, hi) = dome.sampling_box();
        let dome_mesh = polygonize(&dome, lo, hi, 32);
        let r_dome = validate_with_field(&dome_mesh, Some(&dome), 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            !r_dome.issues.iter().any(|e| e.code == "UNSTABLE"),
            "centered flat-based dome must be stable: {:?}",
            r_dome.issues.iter().map(|e| &e.cause).collect::<Vec<_>>()
        );

        // トップヘビーで偏った部品: 小さな脚 (底) の上に、横へ大きくずれた重い塊を載せる。
        // 重心が脚のフットプリントから外れ転倒する → UNSTABLE。
        let foot = Sdf::cuboid(Vec3::new(0.15, 0.15, 0.15)); // 原点付近の小さな脚
        let head = Sdf::sphere(0.6).translate(Vec3::new(1.5, 0.0, 0.6)); // 横へ大きくずれた重い頭
        let tower = foot.union(head);
        // z=最下面で平坦化し脚を接地させる。
        let (tlo, thi) = tower.sampling_box();
        let tower_mesh = polygonize(&tower, tlo, thi, 48);
        let r_tower = validate_with_field(&tower_mesh, Some(&tower), 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            r_tower.issues.iter().any(|e| e.code == "UNSTABLE"),
            "a part whose mass hangs far to one side of its base must be UNSTABLE: {:?}",
            r_tower.issues.iter().map(|e| &e.cause).collect::<Vec<_>>()
        );
    }

    #[test]
    fn issue_severity_serializes_as_lowercase_valid_value() {
        // 問114: validate が emit する全 issue の severity JSON 表現が小文字の
        // "error"/"warning"/"info" のいずれかであることを固定する。
        //
        // 問82 で to_json の lowercase を確認したが、その確認は1形状・1issue のみ。
        // 実際に severity バリアントを持ちうる全コードが、代表形状電池で全て
        // 正当な小文字値にシリアライズされることを回帰ガードする。
        //
        // 実装側 check.rs:148 が Severity::Error => "error" のように固定しているが
        // 将来の enum バリアント追加や serialization 経路変更で大文字化するリスクがある。
        use crate::mcp::json::parse;

        // 多様な issue を誘発する形状/パラメータの電池。
        type MeshFactory = Box<dyn Fn() -> crate::extract::Mesh>;
        let shapes_params: &[(&str, MeshFactory)] = &[
            // EMPTY_MESH: 非重複 smooth_intersection
            ("empty", Box::new(|| {
                let a = Sdf::sphere(1.0);
                let b = Sdf::sphere(1.0).translate(Vec3::new(10.0, 0.0, 0.0));
                let (lo, hi) = a.clone().smooth_intersection(b.clone(), 0.3).sampling_box();
                crate::extract::polygonize(&a.smooth_intersection(b, 0.3), lo, hi, 8)
            })),
            // OPEN_MESH: クリップ
            ("open", Box::new(|| {
                crate::extract::polygonize(
                    &Sdf::sphere(1.0),
                    Vec3::new(-1.5, -1.5, -1.5),
                    Vec3::new(1.5, 1.5, 0.0),
                    24,
                )
            })),
            // THIN_WALL + 通常: 穴あき球
            ("thinwall", Box::new(|| {
                let m = Sdf::sphere(1.0).difference(Sdf::cylinder(0.9, 2.0));
                let (lo, hi) = m.sampling_box();
                crate::extract::polygonize(&m, lo, hi, 40)
            })),
            // SUSPICIOUS_SCALE: 極小形状
            ("tiny", Box::new(|| {
                let tiny = Sdf::sphere(0.1);
                let (lo, hi) = tiny.sampling_box();
                crate::extract::polygonize(&tiny, lo, hi, 16)
            })),
            // 正常: 球
            ("ok", Box::new(|| {
                let s = Sdf::sphere(1.0);
                let (lo, hi) = s.sampling_box();
                crate::extract::polygonize(&s, lo, hi, 24)
            })),
        ];

        const VALID_VALUES: &[&str] = &["error", "warning", "info"];

        for (label, make_mesh) in shapes_params {
            let mesh = make_mesh();
            // min_wall=0.5 で THIN_WALL, max_overhang=0 で OVERHANG スキップ。
            let report = validate(&mesh, 0.5, 0.0);
            let v = report.to_json();
            let json_str = v.to_string();
            let reparsed = parse(&json_str).expect("report JSON must be valid");
            let issues = reparsed.get("issues").and_then(|x| x.as_array()).unwrap();
            for issue in issues {
                let sev = issue
                    .get("severity")
                    .and_then(|s| s.as_str())
                    .unwrap_or("<missing>");
                assert!(
                    VALID_VALUES.contains(&sev),
                    "shape '{label}': severity must be lowercase valid value, got '{sev}'"
                );
                // 大文字でないことを明示確認 (大文字バリアント "Error"/"Warning" が混入しない)。
                assert_eq!(
                    sev,
                    sev.to_lowercase(),
                    "shape '{label}': severity must be all-lowercase, got '{sev}'"
                );
            }
        }
    }

    #[test]
    fn zero_build_dir_silently_skips_overhang_check_without_crash() {
        // 問143: build_dir = Vec3::ZERO は bd_len ≤ 1e-12 により OVERHANG 検査が
        // スキップされる (doc: "零ベクトルならスキップ")。
        // テストがなかったためパニックしないこと・OVERHANG が出ないことを固定する。
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 16);
        // max_overhang_deg > 0 だが build_dir = ZERO → スキップ。
        let r = validate_with_field(&mesh, None, 0.0, 45.0, Vec3::ZERO);
        assert!(
            !r.issues.iter().any(|e| e.code == "OVERHANG"),
            "zero build_dir must skip overhang check without producing OVERHANG issue"
        );
        // very small (非ゼロ) build_dir も同様。
        let r2 = validate_with_field(&mesh, None, 0.0, 45.0, Vec3::new(0.0, 0.0, 1e-14));
        assert!(
            !r2.issues.iter().any(|e| e.code == "OVERHANG"),
            "sub-threshold build_dir must also skip overhang check"
        );
    }

    #[test]
    fn validate_is_consistent_with_validate_with_field_default_args() {
        // 問138: `validate(mesh, w, o)` は `validate_with_field(mesh, None, w, o, Vec3(0,0,1))`
        // の薄いラッパーである。両経路が同一の Report を生成することをテストで固定する。
        // ラッパーが誤って異なる build_dir や sdf を渡した場合、この回帰テストが検知する。
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 24);

        let r_short = validate(&mesh, 0.5, 45.0);
        let r_long = validate_with_field(&mesh, None, 0.5, 45.0, Vec3::new(0.0, 0.0, 1.0));

        // 基本指標: 全て同一でなければならない。
        assert_eq!(r_short.triangle_count, r_long.triangle_count, "triangle_count must match");
        assert_eq!(r_short.is_manifold, r_long.is_manifold, "is_manifold must match");
        assert_eq!(r_short.digest, r_long.digest, "digest must match");
        // issues の数と code も同一。
        assert_eq!(
            r_short.issues.len(),
            r_long.issues.len(),
            "issue count must match: short={:?} long={:?}",
            r_short.issues.iter().map(|e| &e.code).collect::<Vec<_>>(),
            r_long.issues.iter().map(|e| &e.code).collect::<Vec<_>>()
        );
        for (a, b) in r_short.issues.iter().zip(r_long.issues.iter()) {
            assert_eq!(a.code, b.code, "issue codes must match");
            assert_eq!(a.severity, b.severity, "issue severities must match");
        }
    }

    #[test]
    fn empty_mesh_volume_is_never_reliable() {
        // 問130: 空メッシュ (三角形ゼロ) は is_manifold=true になりうるが、
        // volume_reliable() は false を返さなければならない。
        // volume_reliable = is_manifold && triangle_count > 0 の triangle_count > 0
        // ガードが削除されると空メッシュを「体積信頼可」と誤判定する。
        // 非重複の SmoothIntersection は空メッシュを生む標準的なパス (問40)。
        let a = Sdf::sphere(1.0);
        let b = Sdf::sphere(1.0).translate(Vec3::new(10.0, 0.0, 0.0));
        let si = a.smooth_intersection(b, 0.3);
        let (lo, hi) = si.sampling_box();
        let mesh = polygonize(&si, lo, hi, 8);
        assert!(
            mesh.triangles.is_empty(),
            "precondition: non-overlapping smooth_intersection must yield empty mesh"
        );
        let report = validate(&mesh, 0.0, 0.0);
        assert!(
            !report.volume_reliable(),
            "empty mesh must not claim volume is reliable (triangle_count=0 guard must hold)"
        );
        assert!(
            report.issues.iter().any(|e| e.code == "EMPTY_MESH"),
            "empty mesh must produce an EMPTY_MESH issue"
        );
    }

    #[test]
    fn volume_reliable_conjunction_requires_both_conditions() {
        // 問159: volume_reliable() = is_manifold && triangle_count > 0。
        // 両条件は独立した論理積なので、片方だけ真の場合に false を返すことを
        // Report を直接構築して固定する (validate() は常に両者を整合させるため、
        // この単体テストなしではガードの片側削除が無音で通る)。
        let base = Report {
            volume: 1.0,
            surface_area: 6.0,
            bbox: Some((Vec3::ZERO, Vec3::splat(1.0))),
            triangle_count: 100,
            is_manifold: true,
            digest: 42,
            center_of_mass: None,
            issues: vec![],
        };
        // 正常系: 両条件 true → reliable。
        assert!(base.volume_reliable(), "both conditions true → reliable");
        // is_manifold だけ true, triangle_count=0 → unreliable (triangle_count > 0 ガード)。
        assert!(
            !Report { triangle_count: 0, ..base.clone() }.volume_reliable(),
            "triangle_count=0 with is_manifold=true must be unreliable"
        );
        // triangle_count > 0 だけ true, is_manifold=false → unreliable。
        assert!(
            !Report { is_manifold: false, ..base }.volume_reliable(),
            "non-manifold with triangle_count>0 must be unreliable"
        );
    }

    #[test]
    fn sdf_gradient_points_outward_on_sphere_surface() {
        // 問193: sdf_gradient は中央差分 h=1e-4 で勾配を計算するが、
        // 専用の単体テストが存在しなかった。球面上の点での勾配は
        // 外向き単位ベクトルに近い (長さ ≈ 2*h*1/1 ≈ 2e-4 ではなく、
        // 中央差分なので grad ≈ 2h*1/(2h) の正規化前: dfx ≈ 2h, 実際には長さ≈1)。
        // なお sdf_gradient は非正規化勾配を返す (中央差分の差分値そのもの)。
        let sphere = Sdf::sphere(1.0);
        let p = Vec3::new(1.0, 0.0, 0.0); // 球面上の点
        let g = sdf_gradient(&sphere, p);
        let len = g.length();
        // 球 SDF の勾配は至るところ ≈ 1.0 なので中央差分は (d(1+h) - d(1-h)) / (2h) ≈ 1.0/1.
        // 実際の返り値は差分値 (2h * 1) ≈ 2e-4 ではなく差のみ (= 2h * |∇|)。
        // ∇sphere = 1.0 なので len ≈ 2*1e-4 * 1.0... いや、この実装は h=1e-4 で
        // sdf(p+h) - sdf(p-h) を各軸で計算: 約 2h * ∂d/∂x_i。
        // x方向: (|1+h|-1) - (|1-h|-1) = h - (-h) = 2h ≈ 2e-4 (for exact sphere).
        // 長さ ≈ 2e-4 (x成分のみ非ゼロ)。
        assert!(len > 1e-6, "gradient at sphere surface must be nonzero: len={len}");
        assert!(len.is_finite(), "gradient must be finite: len={len}");
        // x 方向が主成分 (球面外向き法線 = +x 方向)。
        let gx_frac = g.x.abs() / len;
        assert!(gx_frac > 0.99, "gradient at (1,0,0) must point mostly in x-direction: gx_frac={gx_frac}");
        // 内部点 (0,0,0) でも有限。
        let g_center = sdf_gradient(&sphere, Vec3::ZERO);
        assert!(g_center.length().is_finite(), "gradient at center must be finite");
    }

    #[test]
    fn min_wall_probe_degenerate_bbox_returns_none() {
        // 問194: min_wall_probe は `if diag <= 0.0 || v == 0 { return None; }` の
        // 早期リターンを持つが、これをテストするケースが存在しなかった。
        // ゼロ対角 (lo=hi) と空メッシュ (v=0) の両方で None を確認。
        use crate::extract::Mesh;
        let sphere = Sdf::sphere(1.0);
        // ケース1: lo == hi → diag = 0 → None。
        let non_empty = {
            let mut m = Mesh::default();
            m.vertices.push(Vec3::new(1.0, 0.0, 0.0));
            m
        };
        let r1 = min_wall_probe(&sphere, &non_empty, Vec3::ZERO, Vec3::ZERO);
        assert!(r1.is_none(), "zero-extent bbox must return None, got {:?}", r1);

        // ケース2: v == 0 (頂点なし) → None。
        let empty_mesh = Mesh::default();
        let r2 = min_wall_probe(&sphere, &empty_mesh, Vec3::splat(-2.0), Vec3::splat(2.0));
        assert!(r2.is_none(), "empty mesh must return None, got {:?}", r2);
    }

    #[test]
    fn overhang_issue_carries_spatial_location() {
        // 問242: OVERHANG issue は最悪三角形の重心を location フィールドに持つ。
        // 球 (+Z ビルド、閾値 45°) では南半球の最下点付近が最悪。
        // 位置の z 成分が負 (下半球) であること、JSON に location が含まれることを固定する。
        use crate::mcp::json::parse;
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 32);
        let r = validate_with_field(&mesh, Some(&sphere), 0.0, 45.0, Vec3::new(0.0, 0.0, 1.0));

        let ov = r.issues.iter().find(|e| e.code == "OVERHANG")
            .expect("sphere must have OVERHANG issue");
        let loc = ov.location.expect("OVERHANG issue must carry a spatial location");
        // 最悪オーバーハング (z = -1.0 付近) は南半球に位置する。
        assert!(loc.z < -0.5, "OVERHANG location must be in lower hemisphere, got {loc:?}");
        // 球は原点中心なので lateral は小さいはず。
        assert!(loc.x.abs() < 1.5 && loc.y.abs() < 1.5, "location within sphere bounds: {loc:?}");

        // JSON にも location が含まれる。
        let json_str = r.to_json().to_string();
        let doc = parse(&json_str).unwrap();
        let issues = doc.get("issues").and_then(|x| x.as_array()).unwrap();
        let ov_json = issues.iter().find(|e| {
            e.get("code").and_then(|c| c.as_str()) == Some("OVERHANG")
        }).expect("OVERHANG must be in JSON issues");
        let loc_arr = ov_json.get("location").and_then(|l| l.as_array())
            .expect("OVERHANG JSON issue must have location array");
        assert_eq!(loc_arr.len(), 3, "location must be [x,y,z]");
        let lz = loc_arr[2].as_f64().unwrap();
        assert!(lz < -0.5, "JSON location z must be in lower hemisphere, got {lz}");
    }

    #[test]
    fn thin_wall_issue_carries_probe_location_in_fin() {
        // 問242: THIN_WALL issue (SDF プローブが検出) は、最小肉厚を見つけた
        // 表面頂点を location に持つ。フィン形状ではその点がフィン領域にある。
        let body = Sdf::cuboid(Vec3::new(1.0, 1.0, 1.0));
        let fin = Sdf::cuboid(Vec3::new(1.8, 0.05, 0.8));
        let sdf = body.union(fin);
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 48);

        let r = validate_with_field(&mesh, Some(&sdf), 0.2, 0.0, Vec3::new(0.0, 0.0, 1.0));
        let tw = r.issues.iter().find(|e| e.code == "THIN_WALL")
            .expect("body+fin must trigger THIN_WALL with threshold 0.2");
        let loc = tw.location.expect("THIN_WALL (probe-detected) must carry a location");
        // フィンの y 半幅は 0.05 → |y| < 0.15 の領域がフィン。
        assert!(
            loc.y.abs() < 0.15,
            "THIN_WALL location must be in the thin fin (|y|<0.15), got {loc:?}"
        );
    }

    #[test]
    fn unstable_issue_carries_com_as_location() {
        // 問242: UNSTABLE issue は重心 (COM) を location に持つ。
        // AI エージェントが issue.location だけを見て重心位置を知れることを確認。
        let foot = Sdf::cuboid(Vec3::new(0.15, 0.15, 0.15));
        let head = Sdf::sphere(0.6).translate(Vec3::new(1.5, 0.0, 0.6));
        let tower = foot.union(head);
        let (lo, hi) = tower.sampling_box();
        let mesh = polygonize(&tower, lo, hi, 48);
        let r = validate_with_field(&mesh, Some(&tower), 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));

        let us = r.issues.iter().find(|e| e.code == "UNSTABLE")
            .expect("top-heavy tower must be UNSTABLE");
        let loc = us.location.expect("UNSTABLE issue must carry COM as location");
        // 重心は頭 (大きな球) 側に引き寄せられるので x > 0.5。
        assert!(loc.x > 0.5, "UNSTABLE location (COM) must be toward the heavy head, got {loc:?}");
        // Report.center_of_mass と一致する (同一 COM)。
        let com = r.center_of_mass.expect("report must have center_of_mass");
        assert!((loc.x - com.x).abs() < 1e-9, "issue.location must equal report.center_of_mass");
    }

    #[test]
    fn issues_without_spatial_context_have_null_location_in_json() {
        // 問242: 空間的でない issue (EMPTY_MESH, OPEN_MESH 等) の JSON location は null。
        use crate::mcp::json::{parse, Value};
        // OPEN_MESH を誘発 (クリップ)。
        let s = Sdf::sphere(1.0);
        let mesh = polygonize(&s, Vec3::new(-1.5,-1.5,-1.5), Vec3::new(1.5,1.5,0.0), 24);
        let r = validate(&mesh, 0.0, 0.0);
        let doc = parse(&r.to_json().to_string()).unwrap();
        let issues = doc.get("issues").and_then(|x| x.as_array()).unwrap();
        // 全 issue に location キーが存在し、OPEN_MESH は null。
        for issue in issues {
            let code = issue.get("code").and_then(|c| c.as_str()).unwrap_or("");
            let loc = issue.get("location").expect("every issue JSON must have a location key");
            if code == "OPEN_MESH" {
                assert_eq!(*loc, Value::Null, "OPEN_MESH location must be null");
            }
        }
    }

    #[test]
    fn to_json_carries_all_schema_fields() {
        // 問243: validate の JSON レポートスキーマは MCP ツール定義と SPEC §5 で文書化されている。
        // フィールドの追加・削除時に文書と実装がずれる「スキーマドリフト」を防ぐために、
        // 期待される全フィールドの存在を固定する回帰テスト。
        //
        // 対象フィールド (SPEC §5.1 / tools.rs validate description より):
        //   レポート: ok, triangles, manifold, volume, volume_reliable, surface_area,
        //             bbox, dims_mm, center_of_mass, digest, issues
        //   各 issue: severity, code, cause, hints, location (v110 追加)
        use crate::mcp::json::parse;
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 24);
        // overhang あり → issues に OVERHANG (location = Some) が含まれることを確認。
        let report = validate_with_field(&mesh, Some(&sphere), 0.0, 45.0, Vec3::new(0.0, 0.0, 1.0));
        let v = report.to_json();
        let doc = parse(&v.to_string()).expect("report JSON must parse");

        // レポートレベル全フィールド。
        for field in ["ok", "triangles", "manifold", "volume", "volume_reliable",
                      "surface_area", "bbox", "dims_mm", "center_of_mass", "digest", "issues"] {
            assert!(
                doc.get(field).is_some(),
                "report JSON must have field '{field}'"
            );
        }

        // issues 配列の各要素が全フィールドを持つ。
        let issues = doc.get("issues").and_then(|x| x.as_array()).unwrap();
        assert!(!issues.is_empty(), "sphere with overhang check must have at least one issue");
        for issue in issues {
            for field in ["severity", "code", "cause", "hints", "location"] {
                assert!(
                    issue.get(field).is_some(),
                    "issue JSON must have field '{field}' (missing in: {})",
                    issue.get("code").and_then(|c| c.as_str()).unwrap_or("?")
                );
            }
        }
    }

    #[test]
    fn report_exposes_center_of_mass_in_json_and_summary() {
        // 問239: validate は COM を計算して UNSTABLE 判定に使うが、その座標を Report
        // に公開していなかった。AI エージェントが「重心がどこか」を直接読めるよう
        // center_of_mass フィールドを追加した。
        //
        // 正常系: 中実球の COM は原点付近。JSON に center_of_mass: [x,y,z]、
        // summary に com=[...] が含まれ、かつ to_json が往復可能。
        use crate::mcp::json::parse;
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 32);
        let report = validate(&mesh, 0.0, 0.0);

        // center_of_mass フィールドが Some であること。
        let com = report.center_of_mass.expect("solid sphere must have a center of mass");
        assert!(com.length() < 0.05, "centered sphere COM must be near origin, got {com:?}");

        // to_json に "center_of_mass" キーが存在し、3要素配列。
        let v = report.to_json();
        let json_str = v.to_string();
        let reparsed = parse(&json_str).expect("report JSON must round-trip");
        assert_eq!(reparsed, v, "to_json must round-trip");
        let com_arr = v.get("center_of_mass")
            .and_then(|x| x.as_array())
            .expect("center_of_mass must be a JSON array for a solid mesh");
        assert_eq!(com_arr.len(), 3, "center_of_mass array must have 3 components");
        let cx = com_arr[0].as_f64().expect("center_of_mass[0] must be a number");
        assert!(cx.abs() < 0.05, "COM.x near 0 for centered sphere, got {cx}");

        // summary に "com=" が含まれる。
        let s = report.summary();
        assert!(s.contains("com="), "summary must expose center_of_mass: {s}");

        // 異常系: 空メッシュの COM は null。
        use crate::extract::Mesh;
        let empty = Mesh::default();
        let r_empty = validate(&empty, 0.0, 0.0);
        assert!(
            r_empty.center_of_mass.is_none(),
            "empty mesh must have center_of_mass = None"
        );
        let v_empty = r_empty.to_json();
        let com_empty = v_empty.get("center_of_mass").expect("key must exist even if null");
        assert!(
            *com_empty == crate::mcp::json::Value::Null,
            "empty mesh center_of_mass in JSON must be null"
        );
    }

    #[test]
    fn report_exposes_surface_area_in_json_and_summary() {
        // 問244: validate は肉厚推定 (2V/SA) で表面積を計算しながら破棄していた。
        // 表面積は FDM 造形時間・材料費の主要因であり、体積と並ぶ基本幾何量。
        // Report.surface_area として公開し、AI がコスト/時間を見積もれるようにする。
        //
        // 正常系: 半径 1 の中実球の表面積は 4πr² ≈ 12.566。テッセレーション近似なので
        // 真値より僅かに小さい (内接多面体) が、十分な解像度で 10% 以内。
        use crate::mcp::json::parse;
        let sphere = Sdf::sphere(1.0);
        let (lo, hi) = sphere.sampling_box();
        let mesh = polygonize(&sphere, lo, hi, 48);
        let report = validate(&mesh, 0.0, 0.0);

        let exact = 4.0 * std::f64::consts::PI; // 4πr², r=1
        assert!(
            report.surface_area > 0.0 && report.surface_area <= exact * 1.02,
            "sphere surface area must be positive and not exceed 4π (inscribed polyhedron), got {}",
            report.surface_area
        );
        assert!(
            (report.surface_area - exact).abs() / exact < 0.1,
            "tessellated sphere area must be within 10% of 4π≈{exact:.3}, got {}",
            report.surface_area
        );
        // Report.surface_area は Mesh::surface_area と一致 (単一の真実源)。
        assert_eq!(report.surface_area, mesh.surface_area(), "report area must equal mesh area");

        // to_json に "surface_area" キーが存在し往復可能。
        let v = report.to_json();
        let reparsed = parse(&v.to_string()).expect("report JSON must round-trip");
        assert_eq!(reparsed, v, "to_json must round-trip");
        let area = v.get("surface_area").and_then(|x| x.as_f64()).expect("surface_area must be a number");
        assert_eq!(area, report.surface_area, "JSON surface_area must equal report value");

        // summary に "area=" が含まれる。
        assert!(report.summary().contains("area="), "summary must expose area: {}", report.summary());

        // 開境界 (クリップ) でも表面積は意味を持つ (体積と異なり常に有効)。
        let clipped = polygonize(&sphere, Vec3::new(-1.5,-1.5,-1.5), Vec3::new(1.5,1.5,0.0), 32);
        let r_clip = validate(&clipped, 0.0, 0.0);
        assert!(
            r_clip.surface_area > 0.0,
            "open mesh must still report a positive surface area, got {}",
            r_clip.surface_area
        );
    }

    #[test]
    fn tall_thin_cylinder_is_flagged_high_aspect_ratio() {
        // 問241: 半径 0.2, 半高 2.5 → 高さ 5.0 mm / 横幅 0.4 mm = 比率 12.5 > 8。
        // 印刷中にノズルが当たると揺れて層間剥離する典型的な形状。
        let sdf = Sdf::cylinder(0.2, 2.5);
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 32);
        let r = validate_with_field(&mesh, None, 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            r.issues.iter().any(|e| e.code == "HIGH_ASPECT_RATIO"),
            "tall thin cylinder (h=5, w=0.4, ratio=12.5) must flag HIGH_ASPECT_RATIO: {:?}",
            r.issues.iter().map(|e| (&e.code, &e.cause)).collect::<Vec<_>>()
        );
        let issue = r.issues.iter().find(|e| e.code == "HIGH_ASPECT_RATIO").unwrap();
        assert!(
            issue.cause.contains("build direction"),
            "HIGH_ASPECT_RATIO issue must name build direction: {}",
            issue.cause
        );
    }

    #[test]
    fn wide_flat_part_does_not_flag_aspect_ratio() {
        // 問241 の対照: 扁平な形状はアスペクト比が低く HIGH_ASPECT_RATIO なし。
        // half=(2,2,0.1) → 高さ 0.2 / 横幅 4.0 = 比率 0.05 << 8。
        let sdf = Sdf::cuboid(Vec3::new(2.0, 2.0, 0.1));
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 32);
        let r = validate_with_field(&mesh, None, 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            !r.issues.iter().any(|e| e.code == "HIGH_ASPECT_RATIO"),
            "wide flat part must NOT flag HIGH_ASPECT_RATIO: {:?}",
            r.issues.iter().map(|e| &e.code).collect::<Vec<_>>()
        );
    }

    #[test]
    fn aspect_ratio_check_respects_build_direction() {
        // 問241: build_dir が HIGH_ASPECT_RATIO 判定に使われていることを確認。
        // 横長の直方体 half=(2,0.2,0.2) は印刷方向で結果が変わる:
        //   +X ビルド: 高さ=4, 横幅≈0.4 → 比率≈10 > 8 → HIGH_ASPECT_RATIO
        //   +Z ビルド: 高さ=0.4, 横幅≈4 → 比率≈0.1 < 8 → なし
        // この対称性が build_dir パラメータの実際の機能を証明する。
        let sdf = Sdf::cuboid(Vec3::new(2.0, 0.2, 0.2));
        let (lo, hi) = sdf.sampling_box();
        let mesh = polygonize(&sdf, lo, hi, 32);

        // 長軸方向 (+X) へのビルド: 高くて細い → 警告あり。
        let r_x = validate_with_field(&mesh, None, 0.0, 0.0, Vec3::new(1.0, 0.0, 0.0));
        assert!(
            r_x.issues.iter().any(|e| e.code == "HIGH_ASPECT_RATIO"),
            "printing a bar along its long axis must flag HIGH_ASPECT_RATIO"
        );

        // 短軸方向 (+Z) へのビルド: 低くて広い → 警告なし。
        let r_z = validate_with_field(&mesh, None, 0.0, 0.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            !r_z.issues.iter().any(|e| e.code == "HIGH_ASPECT_RATIO"),
            "printing the same bar along its short axis must NOT flag HIGH_ASPECT_RATIO"
        );
    }

    #[test]
    fn mesh_only_validate_flags_dome_rim_overhang_that_field_aware_suppresses() {
        // 問240: validate(mesh, w, o) は sdf=None のため、OVERHANG チェックの
        // (b) 段階「直下に材料あり → 支持済み除外」が機能しない。
        // flatten/cut で作った平坦底面ドームの rim 遷移三角形は：
        //   - field-aware (Some(&sdf)): SDF が「直下に底面材料あり」と判定 → 除外 → 警告なし
        //   - mesh-only (None):         SDF チェック不可 → rim が OVERHANG 候補のまま → 警告あり
        //
        // このテストは制限 (メッシュのみでは支持判定不可) を**意図的に文書化**する。
        // validate_with_field の SDF 引数が意味を持つことを回帰ガードする。
        let dome = Sdf::sphere(1.0).cut(Vec3::new(0.0, 0.0, -1.0), 0.0);
        let (lo, hi) = dome.sampling_box();
        let mesh = polygonize(&dome, lo, hi, 32);

        // field-aware: rim は支持済みと判定され OVERHANG なし (問237 で保証済み)。
        let r_field = validate_with_field(&mesh, Some(&dome), 0.0, 45.0, Vec3::new(0.0, 0.0, 1.0));
        assert!(
            !r_field.issues.iter().any(|e| e.code == "OVERHANG"),
            "field-aware validate must NOT flag flat-base dome rim as overhang: {:?}",
            r_field.issues.iter().map(|e| &e.cause).collect::<Vec<_>>()
        );

        // mesh-only: SDF がないため rim の支持判定ができず OVERHANG 警告が出る (既知制限)。
        let r_mesh = validate(&mesh, 0.0, 45.0);
        assert!(
            r_mesh.issues.iter().any(|e| e.code == "OVERHANG"),
            "mesh-only validate MUST flag flat-base dome rim as overhang \
             (known limitation: no SDF → cannot determine support). \
             If this fails, the rim exclusion may have been incorrectly backported to mesh-only."
        );
    }
}
