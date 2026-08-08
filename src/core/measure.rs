//! 光線に沿った表面交差の測定 (問299)。
//!
//! # なぜ必要か (第一原理)
//!
//! Kado の存在理由は「AI が人手なしで製造可能な部品を作り切ること」であり、KPI に
//! **平均ツール呼出 ≤15/タスク** が含まれる (Plan.md §7)。しかし個別形状の寸法
//! (穴径・肉厚・面間距離) を測る手段が `eval` (1点1呼出) しかなく、AI が符号の
//! 二分探索を手組みすると寸法1本で約30呼出を要し、KPI を単独で破っていた。
//! 本モジュールはその探索をエンジン側へ畳み、**1呼出で寸法を返す**。
//!
//! # アルゴリズム: sphere tracing (Hart 1996)
//!
//! John C. Hart, *"Sphere Tracing: A Geometric Method for the Antialiased Ray
//! Tracing of Implicit Surfaces"*, The Visual Computer 12(10), 1996.
//! <https://graphics.stanford.edu/courses/cs348b-20-spring-content/uploads/hart.pdf>
//!
//! sphere tracing は「距離関数が真の距離の**下界**を返す」ことのみを要求し、
//! 現在地から `|d|` だけ進んでも**表面を跨がない**ことを保証する。Kado の SDF は
//! まさにこの条件を満たす——合成形状で大きさは真の距離を上回らない
//! (`src/core/sdf.rs` の型コメント「大きさは常に真の距離以下 (安全側の過小評価)」)。
//! したがって固定ステップ行進と違い、**ステップより薄い特徴を飛び越す危険がない**。
//!
//! 符号は合成形状でも常に厳密 (同上) なので、交差点は符号変化として正確に捉えられる。
//!
//! # 決定性 (問5)
//!
//! ステップ列は `|d|` のみで決まり、反復上限・収束閾値は固定定数。f64・FMA 不使用・
//! 超越関数なしのため、同一入力は同一出力 (バイト同一) を与える。

use super::math::Vec3;
use super::sdf::Sdf;

/// 表面交差の収束判定距離 (mm)。これ以下の |距離| を表面上とみなす。
/// mm 単位系 (SPEC C5) で 1nm 相当——製造用途の測定精度として十分に細かく、
/// f64 の桁落ちより十分粗い。
///
/// `verify::check` の肉厚探針も同じ前進床を使う (問300) ため crate 内へ公開する。
pub(crate) const SURFACE_EPS: f64 = 1e-6;

/// sphere tracing の1ステップ幅 (問300 で `verify::check` と共有)。
///
/// `|d|` は真の距離の**下界**なのでこれだけ進んでも表面を跨がない (Hart 1996)。
/// 表面近傍では `|d|→0` で停滞するため下限 `SURFACE_EPS` を課して必ず前進させる
/// (無限ループ防止)。内部を深さ `t` で進むとき `|d| = t` となり歩幅が倍々に増えるため、
/// 表面から出発しても対数回で任意距離に到達する。
pub(crate) fn sphere_trace_step(d: f64) -> f64 {
    d.abs().max(SURFACE_EPS)
}

/// sphere tracing の最大反復回数 (リソース上限・SECURITY §4)。
/// 表面に沿ってかすめる光線 (grazing) では収束が遅くなるため上限で打ち切る。
const MAX_STEPS: usize = 10_000;

/// 1本の光線が返す交差点の最大数 (リソース上限)。
pub const MAX_CROSSINGS: usize = 64;

/// 光線と表面の交差点。
#[derive(Clone, Debug, PartialEq)]
pub struct Crossing {
    /// 光線始点からの距離 (mm)。`dir` は単位化されるため実距離。
    pub distance: f64,
    /// 交差点の座標。
    pub point: Vec3,
    /// 立体へ**入る**交差か (外→内)。false なら出る交差 (内→外)。
    pub entering: bool,
}

/// 光線 `from + t*dir` (t ∈ [0, max_distance]) が SDF 表面と交わる点を、
/// 始点に近い順に返す (問299)。
///
/// `dir` は内部で単位化する。返る `distance` は始点からの実距離 (mm) なので、
/// 隣接する交差点の距離差がそのまま**穴径・肉厚・面間距離**になる。
///
/// # エラー
/// 非有限な `from`/`dir`/`max_distance`、ゼロ長 `dir`、非正の `max_distance` を拒否する
/// (幾何的に無効な入力は評価前に拒否する・CONTRIBUTING §4)。
///
/// # 既知の限界 (誠実な表明)
/// 反復上限 `MAX_STEPS` に達した場合、それ以降の交差は返らない (表面をかすめる
/// 光線で起こりうる)。交差数は `MAX_CROSSINGS` で打ち切る。
pub fn ray_crossings(
    sdf: &Sdf,
    from: Vec3,
    dir: Vec3,
    max_distance: f64,
) -> Result<Vec<Crossing>, String> {
    if !from.x.is_finite() || !from.y.is_finite() || !from.z.is_finite() {
        return Err(format!("ray origin must be finite, got {from:?}"));
    }
    if !dir.x.is_finite() || !dir.y.is_finite() || !dir.z.is_finite() {
        return Err(format!("ray direction must be finite, got {dir:?}"));
    }
    if !max_distance.is_finite() || max_distance <= 0.0 {
        return Err(format!(
            "max_distance must be finite and > 0, got {max_distance}"
        ));
    }
    let len = dir.length();
    if len == 0.0 {
        return Err("ray direction must not be the zero vector".into());
    }
    let unit = dir * (1.0 / len);

    let mut crossings = Vec::new();
    let mut t = 0.0_f64;
    // 始点の内外。以後この符号が変わる位置が交差点。
    let mut inside = sdf.eval(from) < 0.0;

    for _ in 0..MAX_STEPS {
        if t > max_distance || crossings.len() >= MAX_CROSSINGS {
            break;
        }
        let p = from + unit * t;
        let d = sdf.eval(p);
        // 非有限は退化形状の兆候。無音で誤った測定を返さず打ち切る。
        if !d.is_finite() {
            break;
        }
        let now_inside = d < 0.0;
        if now_inside != inside {
            // 符号が変わった = 直前のステップ内に表面がある。二分探索で厳密化する。
            // sphere tracing のステップは表面を跨がないが、SURFACE_EPS 以内へ寄った後の
            // 微小前進で跨ぐため、区間 [t - last_step, t] を挟み込む。
            let lo = (t - SURFACE_EPS * 4.0).max(0.0);
            let hit = bisect_crossing(sdf, from, unit, lo, t, inside);
            crossings.push(Crossing {
                distance: hit,
                point: from + unit * hit,
                entering: !inside, // 直前が外なら「入る」交差。
            });
            inside = now_inside;
        }
        t += sphere_trace_step(d);
    }
    Ok(crossings)
}

/// 区間 `[lo, hi]` に符号変化が1つあるとして、二分探索で交差点の t を返す。
/// 反復回数は固定 (決定性)。`inside_at_lo` は lo 側が立体内部かどうか。
///
/// 符号のみに依拠するため、合成形状で大きさが下界に過ぎない場合でも収束先は
/// **真の表面**である (問299)。`verify::check` の肉厚探針も利用する (問300)。
pub(crate) fn bisect_crossing(
    sdf: &Sdf,
    from: Vec3,
    unit: Vec3,
    lo: f64,
    hi: f64,
    inside_at_lo: bool,
) -> f64 {
    let (mut a, mut b) = (lo, hi);
    // 60 回で区間幅は 2^-60 倍。f64 の相対精度に十分到達する。
    for _ in 0..60 {
        let mid = (a + b) * 0.5;
        let inside_mid = sdf.eval(from + unit * mid) < 0.0;
        if inside_mid == inside_at_lo {
            a = mid;
        } else {
            b = mid;
        }
    }
    (a + b) * 0.5
}

/// 隣接する交差点の距離差 (= 通過した各区間の長さ) を返す (問299)。
/// 穴径・肉厚・面間距離はこの値をそのまま読めばよい。
pub fn spans(crossings: &[Crossing]) -> Vec<f64> {
    crossings
        .windows(2)
        .map(|w| w[1].distance - w[0].distance)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 解析解と照合する許容誤差。二分探索は f64 精度まで収束するが、
    /// SDF 自体の近似 (合成形状の下界性) を考慮して緩めに取る。
    const TOL: f64 = 1e-5;

    #[test]
    fn sphere_diameter_matches_analytic_solution() {
        // 半径 r の球の中心を通る光線 → 交差2点、距離差 = 2r (解析解)。
        for r in [0.5, 1.0, 2.5, 10.0] {
            let s = Sdf::sphere(r);
            let cs = ray_crossings(
                &s,
                Vec3::new(-100.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                1000.0,
            )
            .unwrap();
            assert_eq!(
                cs.len(),
                2,
                "a ray through a sphere must cross twice (r={r})"
            );
            assert!(cs[0].entering, "first crossing enters the solid");
            assert!(!cs[1].entering, "second crossing exits the solid");
            let d = spans(&cs)[0];
            assert!(
                (d - 2.0 * r).abs() < TOL,
                "sphere diameter must be 2r={}, measured {d}",
                2.0 * r
            );
        }
    }

    #[test]
    fn hole_diameter_in_a_plate_matches_analytic_solution() {
        // 第一原理の旗艦ユースケース: 板に開けた既知径の穴を1回で測る (問299)。
        // 40x40x4mm の板に半径 1.6mm (M3 クリアランス Ø3.2) の穴。
        let plate = Sdf::cuboid(Vec3::new(20.0, 20.0, 2.0));
        let drill = Sdf::cylinder(1.6, 10.0);
        let part = plate.difference(drill);
        // 板の中心 (z=0) を x 方向に貫く光線: 材料→穴→材料 で4交差。
        let cs = ray_crossings(
            &part,
            Vec3::new(-50.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            200.0,
        )
        .unwrap();
        assert_eq!(
            cs.len(),
            4,
            "solid→hole→solid must produce 4 crossings, got {cs:?}"
        );
        let sp = spans(&cs);
        // sp = [左側の肉厚, 穴径, 右側の肉厚]
        assert!(
            (sp[1] - 3.2).abs() < TOL,
            "M3 clearance hole must measure Ø3.2, got {}",
            sp[1]
        );
        // 左右の肉厚は対称 (20 - 1.6 = 18.4)。
        assert!((sp[0] - 18.4).abs() < TOL, "left wall {} != 18.4", sp[0]);
        assert!((sp[2] - 18.4).abs() < TOL, "right wall {} != 18.4", sp[2]);
    }

    #[test]
    fn plate_thickness_measured_through_the_face() {
        // 肉厚測定: 4mm 板 (half-extent 2.0) を z 方向に貫く。
        let plate = Sdf::cuboid(Vec3::new(20.0, 20.0, 2.0));
        let cs = ray_crossings(
            &plate,
            Vec3::new(0.0, 0.0, -50.0),
            Vec3::new(0.0, 0.0, 1.0),
            200.0,
        )
        .unwrap();
        assert_eq!(cs.len(), 2);
        assert!((spans(&cs)[0] - 4.0).abs() < TOL, "plate must be 4mm thick");
    }

    #[test]
    fn ray_starting_inside_the_solid_reports_exit_first() {
        // 始点が内部なら最初の交差は「出る」交差。
        let s = Sdf::sphere(1.0);
        let cs = ray_crossings(&s, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), 10.0).unwrap();
        assert_eq!(cs.len(), 1, "from the centre outward there is one crossing");
        assert!(!cs[0].entering, "leaving the solid is an exit crossing");
        assert!((cs[0].distance - 1.0).abs() < TOL, "exit at r=1");
    }

    #[test]
    fn missing_the_shape_returns_no_crossings() {
        // 形状を外す光線は空 (エラーではない: 「交差なし」は正当な測定結果)。
        let s = Sdf::sphere(1.0);
        let cs = ray_crossings(
            &s,
            Vec3::new(-100.0, 50.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            1000.0,
        )
        .unwrap();
        assert!(cs.is_empty(), "a ray that misses must report no crossings");
    }

    #[test]
    fn max_distance_bounds_the_search() {
        // max_distance より遠い交差は返らない。
        let s = Sdf::sphere(1.0);
        let cs = ray_crossings(
            &s,
            Vec3::new(-10.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            5.0,
        )
        .unwrap();
        assert!(
            cs.is_empty(),
            "sphere at distance 9 must not be found within max_distance=5"
        );
    }

    #[test]
    fn direction_is_normalized_so_distances_are_true_millimetres() {
        // 非単位ベクトルを渡しても距離は実 mm (単位化される)。
        let s = Sdf::sphere(1.0);
        let a = ray_crossings(
            &s,
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            100.0,
        )
        .unwrap();
        let b = ray_crossings(
            &s,
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(7.3, 0.0, 0.0), // 同方向・長さ違い
            100.0,
        )
        .unwrap();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert!(
                (x.distance - y.distance).abs() < TOL,
                "distances must not depend on |dir|"
            );
        }
    }

    #[test]
    fn invalid_inputs_are_rejected_explicitly() {
        // 幾何的に無効な入力は評価前に明示エラー (CONTRIBUTING §4)。
        let s = Sdf::sphere(1.0);
        let ok_dir = Vec3::new(1.0, 0.0, 0.0);
        assert!(ray_crossings(&s, Vec3::new(f64::NAN, 0.0, 0.0), ok_dir, 10.0).is_err());
        assert!(ray_crossings(&s, Vec3::ZERO, Vec3::new(f64::INFINITY, 0.0, 0.0), 10.0).is_err());
        assert!(ray_crossings(&s, Vec3::ZERO, Vec3::ZERO, 10.0).is_err());
        assert!(ray_crossings(&s, Vec3::ZERO, ok_dir, 0.0).is_err());
        assert!(ray_crossings(&s, Vec3::ZERO, ok_dir, -1.0).is_err());
        assert!(ray_crossings(&s, Vec3::ZERO, ok_dir, f64::NAN).is_err());
    }

    #[test]
    fn measurement_is_deterministic() {
        // 問5: 同一入力 → 同一出力 (バイト同一)。
        let part = Sdf::cuboid(Vec3::splat(5.0)).difference(Sdf::sphere(2.0));
        let run = || {
            ray_crossings(
                &part,
                Vec3::new(-20.0, 0.3, -0.7),
                Vec3::new(1.0, 0.0, 0.0),
                100.0,
            )
            .unwrap()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn thin_feature_is_not_stepped_over() {
        // sphere tracing の要点 (Hart 1996): ステップは |d| なので、どれほど薄い特徴でも
        // 飛び越さない。固定ステップ行進なら見落とす 0.05mm の薄板を検出できることを固定する。
        let thin = Sdf::cuboid(Vec3::new(10.0, 10.0, 0.025)); // 厚さ 0.05mm
        let cs = ray_crossings(
            &thin,
            Vec3::new(0.0, 0.0, -20.0),
            Vec3::new(0.0, 0.0, 1.0),
            100.0,
        )
        .unwrap();
        assert_eq!(cs.len(), 2, "a 0.05mm plate must still be detected");
        assert!(
            (spans(&cs)[0] - 0.05).abs() < TOL,
            "thin plate thickness must measure 0.05, got {}",
            spans(&cs)[0]
        );
    }
}
