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
    pub fn presets(lo: Vec3, hi: Vec3) -> Vec<(&'static str, Camera)> {
        let center = (lo + hi) * 0.5;
        let diag = (hi - lo).length();
        let dist = diag * 1.8;
        let fov = std::f64::consts::FRAC_PI_4;
        let up = Vec3::new(0.0, 0.0, 1.0);
        let light = Vec3::new(0.577, 0.577, 0.577); // 等方
        let diffuse = [220u8, 210, 200];
        let ambient = 0.25;
        let bg = [40u8, 44, 52];

        let cam = |eye: Vec3| Camera {
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
            ("front", cam(Vec3::new(0.0, -1.0, 0.0))),
            ("back", cam(Vec3::new(0.0, 1.0, 0.0))),
            ("right", cam(Vec3::new(1.0, 0.0, 0.0))),
            ("left", cam(Vec3::new(-1.0, 0.0, 0.0))),
            ("top", cam(Vec3::new(0.0, 0.0, 1.0))),
            ("bottom", cam(Vec3::new(0.0, 0.0, -1.0))),
            ("iso", cam(Vec3::new(0.707, -0.707, 0.707))),
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
        let mesh = sphere_mesh();
        let (lo, hi) = mesh.bounds().unwrap();
        let (_, cam) = &Camera::presets(lo, hi)[6]; // iso
        assert_eq!(
            render(&mesh, cam, 32, 32).pixels,
            render(&mesh, cam, 32, 32).pixels
        );
    }
}
