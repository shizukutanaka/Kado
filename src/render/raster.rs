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
    /// 背景色 [R,G,B]。
    pub bg: [u8; 3],
    /// 拡散光のキーライト方向 (単位ベクトル、ワールド座標)。
    pub light_dir: Vec3,
    /// メッシュの拡散色 [R,G,B]。
    pub diffuse: [u8; 3],
    /// Ambient 強度 [0.0, 1.0]。
    pub ambient: f64,
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
    let proj = perspective(cam.fov_y, aspect, 0.01, 1000.0);
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
        let has_foreground = img.pixels.chunks(3).any(|c| c != &cam.bg);
        assert!(has_foreground, "rendered image is entirely background");
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
        assert_eq!((a.width, a.height), (32, 32), "downsample halves dimensions");
        assert_eq!(a.pixels, b.pixels, "SSAA path must be deterministic");
        let has_fg = a.pixels.chunks(3).any(|c| c != &cam.bg);
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
            let has_foreground = img.pixels.chunks(3).any(|c| c != &cam.bg);
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
        };
        let (w, h) = (200, 100);
        let (sx, sy, clip_w) = project_to_screen(&cam, cam.target, w, h);
        assert!(clip_w > 0.0, "target must be in front of camera (clip w>0): {clip_w}");
        assert!((sx - w as f64 / 2.0).abs() < 1e-9, "target screen-x must be center {}, got {sx}", w / 2);
        assert!((sy - h as f64 / 2.0).abs() < 1e-9, "target screen-y must be center {}, got {sy}", h / 2);
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
            assert!((v.length() - 1.0).abs() < 1e-12, "{name} must be unit length, got {}", v.length());
        }
        assert!(r.dot(u).abs() < 1e-12, "r ⊥ u: {}", r.dot(u));
        assert!(r.dot(nf).abs() < 1e-12, "r ⊥ f: {}", r.dot(nf));
        assert!(u.dot(nf).abs() < 1e-12, "u ⊥ f: {}", u.dot(nf));
    }
}
