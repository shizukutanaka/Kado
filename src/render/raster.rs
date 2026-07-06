//! ソフトウェアラスタライザ。
//!
//! - パースペクティブ投影 (または正射影)
//! - z バッファによる隠面除去
//! - フラットシェーディング (Lambertian + ambient)
//! - SDF 勾配から解析的法線を計算 (メッシュ面法線は粗いため)
//! - 背景色・光源方向はカメラ設定で制御

use crate::core::Vec3;
use crate::extract::Mesh;
use crate::render::Image;

/// カメラ・レンダリング設定。
#[derive(Clone, Debug)]
pub struct Camera {
    /// カメラ位置 (ワールド座標)。
    pub eye: Vec3,
    /// 注視点。
    pub target: Vec3,
    /// 上方向。
    pub up: Vec3,
    /// 垂直視野角 (ラジアン)。
    pub fov_y: f64,
    /// 背景色 \[R,G,B\]。
    pub bg: [u8; 3],
    /// 拡散光のキーライト方向 (単位ベクトル、ワールド座標)。
    pub light_dir: Vec3,
    /// メッシュの拡散色 \[R,G,B\]。
    pub diffuse: [u8; 3],
    /// Ambient 強度 [0.0, 1.0]。
    pub ambient: f64,
    /// true なら正射影 (technical drawing 向け・寸法が歪まない)、false なら透視投影
    /// (既定・問267)。前後関係を写実的に見せたい screenshot には透視投影が向くが、
    /// front/back/right/left/top/bottom/iso という名前は本来エンジニアリング図面の
    /// 多面図・等角投影法に由来し、これらは伝統的に**正射影**で描かれる
    /// (透視投影は寸法比率を歪めるため、DFM の目視確認には不利)。
    /// 既定は false (透視投影) とし、既存の screenshot 挙動を変えない。
    pub ortho: bool,
}

impl Camera {
    /// 標準 6 視点 + 等角のプリセットを返す (Plan §4 Phase 4)。
    ///
    /// モデルの BBox を渡すと自動的に eye・target を配置する。
    ///
    /// 注意 (問45): "top"/"bottom" の視線方向は Z 軸と平行なため、共通の
    /// up=(0,0,1) を使うと look_at の cross(f, up) がゼロベクトルになり縮退する。
    /// これらの視点だけ up=(0,1,0) を用いて非縮退な行列を保証する。
    pub fn presets(lo: Vec3, hi: Vec3) -> Vec<(&'static str, Camera)> {
        let center = (lo + hi) * 0.5;
        let diag = (hi - lo).length();
        let dist = diag * 1.8;
        let fov = std::f64::consts::FRAC_PI_4;
        let up_z = Vec3::new(0.0, 0.0, 1.0); // 横向き視点の上方向
        let up_y = Vec3::new(0.0, 1.0, 0.0); // top/bottom: eye ∥ z → up_y で縮退回避 (問45)
        let light = Vec3::new(0.577, 0.577, 0.577); // 等方
        let diffuse = [220u8, 210, 200];
        let ambient = 0.25;
        let bg = [40u8, 44, 52];

        let cam = |eye: Vec3, up: Vec3| Camera {
            eye: center + eye * (dist / eye.length()),
            target: center,
            up,
            fov_y: fov,
            bg,
            light_dir: light,
            diffuse,
            ambient,
            ortho: false, // 既定は透視投影 (問267)。orthographic は screenshot 引数で opt-in。
        };
        vec![
            ("front", cam(Vec3::new(0.0, -1.0, 0.0), up_z)),
            ("back", cam(Vec3::new(0.0, 1.0, 0.0), up_z)),
            ("right", cam(Vec3::new(1.0, 0.0, 0.0), up_z)),
            ("left", cam(Vec3::new(-1.0, 0.0, 0.0), up_z)),
            ("top", cam(Vec3::new(0.0, 0.0, 1.0), up_y)),
            ("bottom", cam(Vec3::new(0.0, 0.0, -1.0), up_y)),
            ("iso", cam(Vec3::new(0.707, -0.707, 0.707), up_z)),
        ]
    }
}

/// メッシュを `(width, height)` ピクセルの PNG 画像としてラスタライズする。
pub fn render(mesh: &Mesh, cam: &Camera, width: usize, height: usize) -> Image {
    let mut img = Image::new(width, height, cam.bg);
    // 問259: width/height==0 は xmax=(...).min(width-1) 等の usize 減算がアンダーフローし、
    // 巨大な走査範囲で zbuf 添字が範囲外パニックする。draw_axes (同ファイル) は既に
    // 同じガードを持つが render はこれまで無防備だった。MCP 経由 (arg_dim, 問18) は
    // [1, MAX_IMAGE_DIM] にクランプ済みで到達しないが、render は pub fn であり
    // 呼び出し側の契約に依存しない防御が一貫性の観点で必要。
    if width == 0 || height == 0 {
        return img;
    }
    let mut zbuf = vec![f32::MAX; width * height];

    // カメラ行列を構築。
    let (view, proj) = build_matrices(cam, width, height);

    // 事前計算: 頂点をクリップ空間へ変換。
    let clip: Vec<[f64; 4]> = mesh
        .vertices
        .iter()
        .map(|&v| mat4_mul_vec4(&proj, mat4_mul_vec4(&view, [v.x, v.y, v.z, 1.0])))
        .collect();

    let ndc_to_screen = |x: f64, y: f64| -> (f64, f64) {
        (
            (x + 1.0) * 0.5 * (width as f64),
            (1.0 - y) * 0.5 * (height as f64),
        )
    };

    for tri in &mesh.triangles {
        let c0 = clip[tri[0] as usize];
        let c1 = clip[tri[1] as usize];
        let c2 = clip[tri[2] as usize];

        // 粗い near/far クリップ (w > 0 必須)。
        if c0[3] <= 0.0 || c1[3] <= 0.0 || c2[3] <= 0.0 {
            continue;
        }

        // NDC。
        let n = |c: [f64; 4]| [c[0] / c[3], c[1] / c[3], c[2] / c[3]];
        let (n0, n1, n2) = (n(c0), n(c1), n(c2));

        // スクリーン座標。
        let (x0, y0) = ndc_to_screen(n0[0], n0[1]);
        let (x1, y1) = ndc_to_screen(n1[0], n1[1]);
        let (x2, y2) = ndc_to_screen(n2[0], n2[1]);

        // フラットシェーディング: 面法線 (ワールド座標)。
        let v0 = mesh.vertices[tri[0] as usize];
        let v1 = mesh.vertices[tri[1] as usize];
        let v2 = mesh.vertices[tri[2] as usize];
        let face_n = (v1 - v0).cross(v2 - v0);
        let face_n_len = face_n.length();
        if face_n_len == 0.0 {
            continue;
        }
        let face_n = face_n * (1.0 / face_n_len);
        let ldot = face_n.dot(cam.light_dir).max(0.0);
        let intensity = (cam.ambient + (1.0 - cam.ambient) * ldot).min(1.0);
        let color = [
            (cam.diffuse[0] as f64 * intensity) as u8,
            (cam.diffuse[1] as f64 * intensity) as u8,
            (cam.diffuse[2] as f64 * intensity) as u8,
        ];

        // バウンディングボックスでスキャン範囲を限定。
        let xmin = x0.min(x1).min(x2).max(0.0) as usize;
        let xmax = (x0.max(x1).max(x2).ceil() as usize).min(width - 1);
        let ymin = y0.min(y1).min(y2).max(0.0) as usize;
        let ymax = (y0.max(y1).max(y2).ceil() as usize).min(height - 1);

        let denom = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2);
        if denom.abs() < 1e-12 {
            continue;
        }
        let inv_denom = 1.0 / denom;

        for py in ymin..=ymax {
            for px in xmin..=xmax {
                let (fx, fy) = (px as f64 + 0.5, py as f64 + 0.5);
                let w0 = ((y1 - y2) * (fx - x2) + (x2 - x1) * (fy - y2)) * inv_denom;
                let w1 = ((y2 - y0) * (fx - x2) + (x0 - x2) * (fy - y2)) * inv_denom;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = (w0 * n0[2] + w1 * n1[2] + w2 * n2[2]) as f32;
                let idx = py * width + px;
                if z < zbuf[idx] {
                    zbuf[idx] = z;
                    img.set(px, py, color);
                }
            }
        }
    }
    img
}

// ── 座標軸グノモン (問66) ───────────────────────────────────────────────────────

/// `origin` を起点に X=赤・Y=緑・Z=青の軸線を `length` だけ描き、向きの基準を与える。
/// オーバーレイ (深度無視) で最後に重ねる。AI/人間が向き・鏡像・座標系を判読できる。
pub fn draw_axes(img: &mut Image, cam: &Camera, origin: Vec3, length: f64) {
    let (w, h) = (img.width, img.height);
    if w == 0 || h == 0 {
        return;
    }
    let (view, proj) = build_matrices(cam, w, h);
    let project = |p: Vec3| -> Option<(f64, f64)> {
        let c = mat4_mul_vec4(&proj, mat4_mul_vec4(&view, [p.x, p.y, p.z, 1.0]));
        if c[3] <= 1e-9 {
            return None;
        }
        let x = c[0] / c[3];
        let y = c[1] / c[3];
        Some(((x + 1.0) * 0.5 * w as f64, (1.0 - y) * 0.5 * h as f64))
    };
    let o = match project(origin) {
        Some(o) => o,
        None => return,
    };
    let axes = [
        (Vec3::new(length, 0.0, 0.0), [235u8, 70, 70]), // X 赤
        (Vec3::new(0.0, length, 0.0), [70, 200, 70]),   // Y 緑
        (Vec3::new(0.0, 0.0, length), [90, 130, 245]),  // Z 青
    ];
    for (dir, color) in axes {
        if let Some(end) = project(origin + dir) {
            draw_line(img, o, end, color);
        }
    }
}

/// 2点間に色付き線を描く (DDA, 2px 太, 画面外はクリップ)。
fn draw_line(img: &mut Image, a: (f64, f64), b: (f64, f64), color: [u8; 3]) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as usize;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let x = (a.0 + dx * t).round() as isize;
        let y = (a.1 + dy * t).round() as isize;
        // 2px 太さで視認性を上げる。
        for (ox, oy) in [(0isize, 0isize), (1, 0), (0, 1)] {
            let (px, py) = (x + ox, y + oy);
            if px >= 0 && py >= 0 && (px as usize) < img.width && (py as usize) < img.height {
                img.set(px as usize, py as usize, color);
            }
        }
    }
}

// ── 行列ヘルパ ─────────────────────────────────────────────────────────────────

fn build_matrices(cam: &Camera, w: usize, h: usize) -> ([f64; 16], [f64; 16]) {
    let view = look_at(cam.eye, cam.target, cam.up);
    let aspect = w as f64 / h as f64;
    let proj = if cam.ortho {
        // 問267: eye-target 距離における透視投影相当の視野半高を正射影の
        // 視野半高として使う。同じ eye/target/fov_y のまま ortho を切り替えても
        // 対象の見かけの大きさが概ね揃う (ズーム感の連続性)。
        let dist = (cam.eye - cam.target).length();
        let half_height = dist * (cam.fov_y / 2.0).tan();
        orthographic(half_height, aspect, 0.01, 1000.0)
    } else {
        perspective(cam.fov_y, aspect, 0.01, 1000.0)
    };
    (view, proj)
}

fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> [f64; 16] {
    let f = normalize(center - eye);
    let r = normalize(f.cross(up));
    let u = r.cross(f);
    // Column-major (OpenGL convention)。mat4_mul_vec4 は row-major で実装。
    // ここでは row-major で直接作る。
    [
        r.x,
        r.y,
        r.z,
        -r.dot(eye),
        u.x,
        u.y,
        u.z,
        -u.dot(eye),
        -f.x,
        -f.y,
        -f.z,
        f.dot(eye),
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn perspective(fov_y: f64, aspect: f64, near: f64, far: f64) -> [f64; 16] {
    let f = 1.0 / (fov_y / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) * nf,
        2.0 * far * near * nf,
        0.0,
        0.0,
        -1.0,
        0.0,
    ]
}

/// 正射影行列 (問267)。透視投影と違い w は常に1のまま (near/far は NDC z に
/// 線形写像されるのみ)。`render` の透視除算 `[0]/[3]` は恒等になり寸法比率を保つ。
fn orthographic(half_height: f64, aspect: f64, near: f64, far: f64) -> [f64; 16] {
    let half_width = half_height * aspect;
    let nf = 1.0 / (near - far);
    [
        1.0 / half_width,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 / half_height,
        0.0,
        0.0,
        0.0,
        0.0,
        2.0 * nf,
        (far + near) * nf,
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

fn mat4_mul_vec4(m: &[f64; 16], v: [f64; 4]) -> [f64; 4] {
    [
        m[0] * v[0] + m[1] * v[1] + m[2] * v[2] + m[3] * v[3],
        m[4] * v[0] + m[5] * v[1] + m[6] * v[2] + m[7] * v[3],
        m[8] * v[0] + m[9] * v[1] + m[10] * v[2] + m[11] * v[3],
        m[12] * v[0] + m[13] * v[1] + m[14] * v[2] + m[15] * v[3],
    ]
}

fn normalize(v: Vec3) -> Vec3 {
    let len = v.length();
    if len < 1e-15 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        v * (1.0 / len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Sdf;
    use crate::extract::polygonize;

    fn sphere_mesh() -> Mesh {
        polygonize(&Sdf::sphere(1.0), Vec3::splat(-1.5), Vec3::splat(1.5), 24)
    }

    #[test]
    fn render_produces_non_blank_image() {
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        let (_, cam) = &presets[0]; // front
        let img = render(&mesh, cam, 64, 64);
        // 少なくとも1画素がメッシュ色 (背景色以外) であること。
        let has_foreground = img.pixels.chunks(3).any(|c| c != cam.bg);
        assert!(has_foreground, "rendered image is entirely background");
    }

    #[test]
    fn render_with_zero_dimension_returns_empty_image_without_panicking() {
        // 問259: width/height==0 は width-1/height-1 の usize アンダーフローで
        // zbuf 添字が範囲外パニックしうる。draw_axes と同じガードを render にも適用する。
        // MCP 経由では arg_dim (問18) が [1, MAX_IMAGE_DIM] にクランプするため到達しないが、
        // render は pub fn であり呼び出し側の契約に依存しない防御が必要。
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        let (_, cam) = &presets[0];
        let img_w0 = render(&mesh, cam, 0, 64);
        assert_eq!(
            img_w0.pixels.len(),
            0,
            "width=0 must yield an empty pixel buffer"
        );
        let img_h0 = render(&mesh, cam, 64, 0);
        assert_eq!(
            img_h0.pixels.len(),
            0,
            "height=0 must yield an empty pixel buffer"
        );
        let img_both0 = render(&mesh, cam, 0, 0);
        assert_eq!(
            img_both0.pixels.len(),
            0,
            "0×0 must yield an empty pixel buffer"
        );
    }

    #[test]
    fn render_is_deterministic() {
        // 問41: インデックスではなく名前でカメラを取得し順序変更に耐える。
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        let (_, cam) = presets.iter().find(|(n, _)| *n == "iso").unwrap();
        assert_eq!(
            render(&mesh, cam, 32, 32).pixels,
            render(&mesh, cam, 32, 32).pixels
        );
    }

    #[test]
    fn supersampled_render_is_deterministic_and_non_blank() {
        // 問56: 2× スーパーサンプル → ダウンサンプルした画像も決定的で前景を持つ。
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        let (_, cam) = presets.iter().find(|(n, _)| *n == "iso").unwrap();
        let a = render(&mesh, cam, 64, 64).downsample(2);
        let b = render(&mesh, cam, 64, 64).downsample(2);
        assert_eq!(
            (a.width, a.height),
            (32, 32),
            "downsample halves dimensions"
        );
        assert_eq!(a.pixels, b.pixels, "SSAA path must be deterministic");
        let has_fg = a.pixels.chunks(3).any(|c| c != cam.bg);
        assert!(has_fg, "SSAA image must contain foreground");
    }

    #[test]
    fn axes_gnomon_draws_rgb_orientation_lines() {
        // 問66: グノモン描画後、画像に赤・緑・青の軸色画素が現れる。
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        let (_, cam) = presets.iter().find(|(n, _)| *n == "iso").unwrap();
        let mut img = render(&mesh, cam, 96, 96);
        let center = (lo + hi) * 0.5;
        draw_axes(&mut img, cam, center, (hi - lo).length() * 0.4);
        let has = |c: [u8; 3]| img.pixels.chunks(3).any(|px| px == c);
        assert!(has([235, 70, 70]), "X axis (red) must be drawn");
        assert!(has([70, 200, 70]), "Y axis (green) must be drawn");
        assert!(has([90, 130, 245]), "Z axis (blue) must be drawn");
    }

    #[test]
    fn axes_overlay_is_deterministic() {
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        let (_, cam) = presets.iter().find(|(n, _)| *n == "iso").unwrap();
        let mut a = render(&mesh, cam, 64, 64);
        let mut b = render(&mesh, cam, 64, 64);
        let center = (lo + hi) * 0.5;
        draw_axes(&mut a, cam, center, 1.0);
        draw_axes(&mut b, cam, center, 1.0);
        assert_eq!(a.pixels, b.pixels);
    }

    #[test]
    fn top_and_bottom_views_render_non_blank() {
        // 問45: "top"/"bottom" は eye ∥ up_z だと look_at が縮退しブランク画像になる。
        // up_y に切り替えることで非縮退な行列を得て、球が写ることを確認する。
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let presets = Camera::presets(lo, hi);
        for name in &["top", "bottom"] {
            let (_, cam) = presets.iter().find(|(n, _)| n == name).unwrap();
            let img = render(&mesh, cam, 32, 32);
            let has_foreground = img.pixels.chunks(3).any(|c| c != cam.bg);
            assert!(
                has_foreground,
                "{name} view must render non-blank (degenerate look_at if up ∥ forward)"
            );
        }
    }

    // ── 投影数値の正しさ (問123) ──────────────────────────────────────────────

    /// テスト用: build_matrices と同じ経路でワールド点をスクリーン座標へ射影する。
    /// render() の ndc_to_screen と一致させ、パイプライン全体を数値で検証する。
    fn project_to_screen(cam: &Camera, p: Vec3, w: usize, h: usize) -> (f64, f64, f64) {
        let (view, proj) = build_matrices(cam, w, h);
        let c = mat4_mul_vec4(&proj, mat4_mul_vec4(&view, [p.x, p.y, p.z, 1.0]));
        let (ndc_x, ndc_y) = (c[0] / c[3], c[1] / c[3]);
        let sx = (ndc_x + 1.0) * 0.5 * w as f64;
        let sy = (1.0 - ndc_y) * 0.5 * h as f64;
        (sx, sy, c[3]) // c[3] = clip w (>0 なら前方)
    }

    #[test]
    fn camera_target_projects_to_screen_center() {
        // 問123: 「非ブランク・決定的」だけでは投影の正しさは保証されない。
        // 転置ミス・符号反転でも安定した誤画像が出る。パイプラインの数学的核心を固定する:
        // カメラ注視点 (target) はちょうどスクリーン中央へ投影されなければならない。
        // (look_at が target を視線軸へ、perspective が軸を NDC 原点へ写すため)
        let cam = Camera {
            eye: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            fov_y: std::f64::consts::FRAC_PI_4,
            bg: [0, 0, 0],
            light_dir: Vec3::new(0.0, 0.0, 1.0),
            diffuse: [200, 200, 200],
            ambient: 0.2,
            ortho: false,
        };
        let (w, h) = (200, 100);
        let (sx, sy, clip_w) = project_to_screen(&cam, cam.target, w, h);
        assert!(
            clip_w > 0.0,
            "target must be in front of camera (clip w>0): {clip_w}"
        );
        assert!(
            (sx - w as f64 / 2.0).abs() < 1e-9,
            "target screen-x must be center {}, got {sx}",
            w / 2
        );
        assert!(
            (sy - h as f64 / 2.0).abs() < 1e-9,
            "target screen-y must be center {}, got {sy}",
            h / 2
        );
    }

    #[test]
    fn point_above_target_projects_above_center() {
        // 問123: up=(0,1,0) のとき、target の真上 (+Y) の点はスクリーン中央より上
        // (= スクリーン y が小さい、原点左上ゆえ) に投影される。これが up 方向と
        // y 反転の両方を検証する。符号が反転していればこのテストが落ちる。
        let cam = Camera {
            eye: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::new(0.0, 1.0, 0.0),
            fov_y: std::f64::consts::FRAC_PI_4,
            bg: [0, 0, 0],
            light_dir: Vec3::new(0.0, 0.0, 1.0),
            diffuse: [200, 200, 200],
            ambient: 0.2,
            ortho: false,
        };
        let (w, h) = (100, 100);
        let (_, center_y, _) = project_to_screen(&cam, cam.target, w, h);
        let (_, above_y, clip_w) = project_to_screen(&cam, Vec3::new(0.0, 1.0, 0.0), w, h);
        assert!(clip_w > 0.0, "point must be in front");
        assert!(
            above_y < center_y,
            "a point above the target must project above center (smaller screen-y): above={above_y} center={center_y}"
        );
    }

    #[test]
    fn look_at_basis_is_orthonormal() {
        // 問123: look_at が生成するビュー行列の上 3×3 (回転部) は正規直交基底でなければ
        // ならない。さもなくば剛体でない変換になり形状が歪む。各行の長さ=1、相互直交を確認。
        let cam_eye = Vec3::new(3.0, -2.0, 4.0);
        let cam_target = Vec3::new(0.5, 0.5, 0.0);
        let view = look_at(cam_eye, cam_target, Vec3::new(0.0, 0.0, 1.0));
        // row0=r, row1=u, row2=-f (各先頭3成分)。
        let row = |i: usize| Vec3::new(view[i * 4], view[i * 4 + 1], view[i * 4 + 2]);
        let (r, u, nf) = (row(0), row(1), row(2));
        for (name, v) in [("r", r), ("u", u), ("-f", nf)] {
            assert!(
                (v.length() - 1.0).abs() < 1e-12,
                "{name} must be unit length, got {}",
                v.length()
            );
        }
        assert!(r.dot(u).abs() < 1e-12, "r ⊥ u: {}", r.dot(u));
        assert!(r.dot(nf).abs() < 1e-12, "r ⊥ f: {}", r.dot(nf));
        assert!(u.dot(nf).abs() < 1e-12, "u ⊥ f: {}", u.dot(nf));
    }

    /// 最小構成のテストカメラ (eye→target を見る)。
    fn test_camera(eye: Vec3, target: Vec3) -> Camera {
        Camera {
            eye,
            target,
            up: Vec3::new(0.0, 1.0, 0.0),
            fov_y: std::f64::consts::FRAC_PI_4,
            bg: [0, 0, 0],
            light_dir: Vec3::new(0.577, 0.577, 0.577),
            diffuse: [200, 200, 200],
            ambient: 0.2,
            ortho: false,
        }
    }

    #[test]
    fn triangle_behind_camera_is_culled() {
        // 問207: クリップ空間 w <= 0 (near 面の裏) の三角形は描画前に continue で除外される。
        // 既存テストはすべて前方の sphere のみ。カメラ背後の三角形が全背景になることを固定。
        // eye=(0,0,1) は -z 方向を見る。z=3 の三角形は eye より後方 (背面) → w<=0 で除外。
        let cam = test_camera(Vec3::new(0.0, 0.0, 1.0), Vec3::ZERO);
        let mesh = Mesh {
            vertices: vec![
                Vec3::new(0.0, 0.0, 3.0),
                Vec3::new(1.0, 0.0, 3.0),
                Vec3::new(0.0, 1.0, 3.0),
            ],
            triangles: vec![[0, 1, 2]],
        };
        let img = render(&mesh, &cam, 32, 32);
        // 全画素が背景 (前景なし)。
        assert!(
            img.pixels.chunks(3).all(|c| c == cam.bg),
            "behind-camera triangle must be fully culled (all background)"
        );
    }

    #[test]
    fn degenerate_collinear_triangle_is_not_rendered() {
        // 問208: face_n_len == 0.0 (共線=面積ゼロ) の三角形は continue で除外される。
        // polygonize は退化三角形を生まないため、この経路は手動メッシュでのみ到達する。
        let cam = test_camera(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let mesh = Mesh {
            vertices: vec![
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0), // 3 点共線 → 面法線ゼロ
            ],
            triangles: vec![[0, 1, 2]],
        };
        let img = render(&mesh, &cam, 32, 32);
        assert!(
            img.pixels.chunks(3).all(|c| c == cam.bg),
            "degenerate (collinear) triangle must not render (zero face normal)"
        );
    }

    #[test]
    fn perspective_matrix_depth_coefficients_are_exact() {
        // 問209: 透視行列の z 関連係数 (proj[10], proj[11], proj[14]) の数値を固定する。
        // 既存テストは最終スクリーン座標のみ確認し、係数の符号入れ替え等を検出できない。
        let fov = std::f64::consts::FRAC_PI_4;
        let near = 0.01;
        let far = 1000.0;
        let proj = perspective(fov, 1.0, near, far);
        let nf = 1.0 / (near - far); // near-far < 0 なので nf < 0
        assert!(
            (proj[10] - (far + near) * nf).abs() < 1e-15,
            "proj[10] must be (far+near)/(near-far), got {}",
            proj[10]
        );
        assert!(
            (proj[11] - 2.0 * far * near * nf).abs() < 1e-14,
            "proj[11] must be 2*far*near/(near-far), got {}",
            proj[11]
        );
        assert_eq!(proj[14], -1.0, "proj[14] (w from z) must be -1");
        // f = 1/tan(fov/2)。fov=π/4 → tan(π/8) ≈ 0.41421 → f ≈ 2.41421。aspect=1。
        let f = 1.0 / (fov / 2.0).tan();
        assert!(
            (proj[0] - f).abs() < 1e-12,
            "proj[0] = f/aspect must equal f at aspect=1"
        );
        assert!((proj[5] - f).abs() < 1e-12, "proj[5] must equal f");
    }

    #[test]
    fn orthographic_matrix_coefficients_are_exact() {
        // 問267: 正射影行列の係数を固定する。透視投影と異なり w 行は [0,0,0,1]
        // (透視除算が恒等になる) でなければならない。
        let half_height = 2.0;
        let aspect = 1.5;
        let near = 0.01;
        let far = 1000.0;
        let proj = orthographic(half_height, aspect, near, far);
        let half_width = half_height * aspect;
        assert!(
            (proj[0] - 1.0 / half_width).abs() < 1e-15,
            "proj[0] must be 1/half_width, got {}",
            proj[0]
        );
        assert!(
            (proj[5] - 1.0 / half_height).abs() < 1e-15,
            "proj[5] must be 1/half_height, got {}",
            proj[5]
        );
        let nf = 1.0 / (near - far);
        assert!(
            (proj[10] - 2.0 * nf).abs() < 1e-15,
            "proj[10] must be 2/(near-far), got {}",
            proj[10]
        );
        assert!(
            (proj[11] - (far + near) * nf).abs() < 1e-15,
            "proj[11] must be (far+near)/(near-far), got {}",
            proj[11]
        );
        // w 行は恒等 (正射影には透視除算が無い)。
        assert_eq!(
            &proj[12..16],
            &[0.0, 0.0, 0.0, 1.0],
            "w row must be [0,0,0,1]"
        );
    }

    #[test]
    fn orthographic_projection_maps_near_and_far_plane_to_ndc_bounds() {
        // 問267: near 面は NDC z=-1、far 面は NDC z=+1 に写像される
        // (透視除算が w=1 で恒等のため、そのままの値が最終 NDC z になる)。
        let (near, far) = (0.5, 50.0);
        let proj = orthographic(1.0, 1.0, near, far);
        // 視点空間で camera から -near 離れた点 (view space z = -near)。
        let at_near = mat4_mul_vec4(&proj, [0.0, 0.0, -near, 1.0]);
        assert!(
            (at_near[2] / at_near[3] - (-1.0)).abs() < 1e-9,
            "near plane must map to NDC z=-1, got {}",
            at_near[2] / at_near[3]
        );
        let at_far = mat4_mul_vec4(&proj, [0.0, 0.0, -far, 1.0]);
        assert!(
            (at_far[2] / at_far[3] - 1.0).abs() < 1e-9,
            "far plane must map to NDC z=+1, got {}",
            at_far[2] / at_far[3]
        );
    }

    #[test]
    fn orthographic_screenshot_of_cuboid_produces_non_blank_image() {
        // 問267: Camera.ortho=true で render() を呼んでも (w=1 の透視除算恒等が
        // 正しく処理され) パニックせず、前景を含む画像を生成する。
        let sdf = Sdf::cuboid(Vec3::splat(1.0));
        let mesh = polygonize(&sdf, Vec3::splat(-1.5), Vec3::splat(1.5), 24);
        let (lo, hi) = mesh.bounds().unwrap();
        let mut presets = Camera::presets(lo, hi);
        let (_, cam) = presets.iter_mut().find(|(n, _)| *n == "front").unwrap();
        cam.ortho = true;
        let img = render(&mesh, cam, 64, 64);
        let has_foreground = img.pixels.chunks(3).any(|c| c != cam.bg);
        assert!(
            has_foreground,
            "orthographic render must not be entirely background"
        );
    }

    #[test]
    fn orthographic_keeps_parallel_edges_parallel_unlike_perspective() {
        // 問267: 正射影の特徴——奥行きが違っても平行なエッジは画面上でも平行のまま
        // (透視投影は奥行きに応じて収束させ、歪める)。
        // 立方体の手前の辺と奥の辺 (X方向, 両方 Y=Z=1 の高さ/奥行きにある2本) を front
        // ビューから見て、画面上の水平方向の長さがほぼ一致することを確認する
        // (透視だと奥の辺がカメラから遠い分だけ短く見える)。
        let cam_front = Vec3::new(0.0, -1.0, 0.0);
        let target = Vec3::ZERO;
        let mut cam = Camera {
            eye: target + cam_front * 5.0,
            target,
            up: Vec3::new(0.0, 0.0, 1.0),
            fov_y: std::f64::consts::FRAC_PI_4,
            bg: [0, 0, 0],
            light_dir: Vec3::new(0.0, 0.0, 1.0),
            diffuse: [200, 200, 200],
            ambient: 0.2,
            ortho: true,
        };
        let (w, h) = (200, 200);
        // 手前の辺 (y=-1, 近い) の両端。
        let near_left = project_to_screen(&cam, Vec3::new(-1.0, -1.0, 0.0), w, h);
        let near_right = project_to_screen(&cam, Vec3::new(1.0, -1.0, 0.0), w, h);
        // 奥の辺 (y=+1, 遠い) の両端。
        let far_left = project_to_screen(&cam, Vec3::new(-1.0, 1.0, 0.0), w, h);
        let far_right = project_to_screen(&cam, Vec3::new(1.0, 1.0, 0.0), w, h);
        let near_width = (near_right.0 - near_left.0).abs();
        let far_width = (far_right.0 - far_left.0).abs();
        assert!(
            (near_width - far_width).abs() < 1e-6,
            "orthographic must render equal-depth-independent widths: near={near_width}, far={far_width}"
        );

        // 対照: 透視投影では奥の辺が手前より画面上で狭く見える (収束)。
        cam.ortho = false;
        let p_near_left = project_to_screen(&cam, Vec3::new(-1.0, -1.0, 0.0), w, h);
        let p_near_right = project_to_screen(&cam, Vec3::new(1.0, -1.0, 0.0), w, h);
        let p_far_left = project_to_screen(&cam, Vec3::new(-1.0, 1.0, 0.0), w, h);
        let p_far_right = project_to_screen(&cam, Vec3::new(1.0, 1.0, 0.0), w, h);
        let p_near_width = (p_near_right.0 - p_near_left.0).abs();
        let p_far_width = (p_far_right.0 - p_far_left.0).abs();
        assert!(
            p_far_width < p_near_width,
            "perspective must render the far edge narrower than the near edge: \
             near={p_near_width}, far={p_far_width}"
        );
    }

    #[test]
    fn normalize_zero_length_vector_falls_back_to_z_axis() {
        // 問210: normalize は len < 1e-15 のとき (0,0,1) へフォールバックする。
        // 閾値の上下で挙動が分かれることを固定 (look_at の縮退回避に依存)。
        // 閾値以上 → 正規化される。
        let above = normalize(Vec3::new(1e-14, 0.0, 0.0));
        assert!(
            above.x > 0.5,
            "len=1e-14 (>threshold) must normalize toward x, got {above:?}"
        );
        assert!(
            (above.length() - 1.0).abs() < 1e-9,
            "normalized vector must be unit length"
        );
        // 閾値未満 → (0,0,1) フォールバック。
        let below = normalize(Vec3::new(1e-16, 0.0, 0.0));
        assert_eq!(
            below,
            Vec3::new(0.0, 0.0, 1.0),
            "len=1e-16 (<threshold) must fall back to Z"
        );
        // 完全ゼロベクトルもフォールバック。
        assert_eq!(
            normalize(Vec3::ZERO),
            Vec3::new(0.0, 0.0, 1.0),
            "zero vector must fall back to Z"
        );
    }
}
