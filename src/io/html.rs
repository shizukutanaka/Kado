//! 自己完結 HTML ビューア書き出し (Plan §3)。
//!
//! 単一の .html にメッシュ (頂点 + インデックス) を埋め込み、外部リソースを一切
//! 参照しない WebGL2 ビューアを同梱する (外部送信ゼロ / 問4)。ブラウザで開けば
//! オフラインでドラッグ回転・ホイールズームできる。決定的 (固定精度整形, 問5)。
//!
//! シェーディングはフラグメントの画面空間微分 (dFdx/dFdy) から面法線を求める
//! ため、法線バッファ不要で頂点位置とインデックスのみを転送する。

use crate::core::Vec3;
use crate::extract::Mesh;
use std::fmt::Write;

/// メッシュを自己完結 HTML 文字列にエンコードする。
pub fn encode_html(mesh: &Mesh) -> String {
    let (positions, indices) = mesh_arrays(mesh);
    let (lo, hi) = mesh.bounds().unwrap_or((Vec3::ZERO, Vec3::ZERO));
    let c = (lo + hi) * 0.5;
    // 問216: 非有限頂点を含むメッシュでは bounds 由来の center/radius が NaN/Inf に
    // なりうる。center は 0.0 へ、radius は (Inf.max(1e-3)=Inf を通すため) 明示的に
    // 非有限を 1e-3 フォールバックへサニタイズする (3MF の finite_coord と同方針)。
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let raw_radius = (hi - lo).length() * 0.5;
    let radius = if raw_radius.is_finite() { raw_radius.max(1e-3) } else { 1e-3 };
    TEMPLATE
        .replace("/*POSITIONS*/", &positions)
        .replace("/*INDICES*/", &indices)
        .replace(
            "/*CENTER*/",
            &format!("[{:.4},{:.4},{:.4}]", finite(c.x), finite(c.y), finite(c.z)),
        )
        .replace("/*RADIUS*/", &format!("{radius:.4}"))
}

/// メッシュを HTML ファイルに書き出す。
pub fn write_html(mesh: &Mesh, path: &std::path::Path) -> std::io::Result<()> {
    std::fs::write(path, encode_html(mesh))
}

/// 埋め込み用の `(positions, indices)` を固定精度 (4桁) のカンマ区切りで作る。
fn mesh_arrays(mesh: &Mesh) -> (String, String) {
    // 問216: 非有限座標 (NaN/Inf) は `{:.4}` で "NaN"/"inf" 文字列になり、
    // 埋め込み JS の MESH.positions 配列が構文エラーになる。3MF の finite_coord と
    // 同様に 0.0 へサニタイズしてビューアが壊れないようにする。
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    let mut positions = String::with_capacity(mesh.vertices.len() * 24);
    for (i, v) in mesh.vertices.iter().enumerate() {
        if i > 0 {
            positions.push(',');
        }
        let _ = write!(
            positions,
            "{:.4},{:.4},{:.4}",
            finite(v.x),
            finite(v.y),
            finite(v.z)
        );
    }
    let mut indices = String::with_capacity(mesh.triangles.len() * 18);
    for (i, t) in mesh.triangles.iter().enumerate() {
        if i > 0 {
            indices.push(',');
        }
        let _ = write!(indices, "{},{},{}", t[0], t[1], t[2]);
    }
    (positions, indices)
}

/// 自己完結 HTML テンプレート。プレースホルダはコメント形式にして、置換前でも
/// JS として構文上有効に保つ (node --check 等での検証を容易にするため)。
const TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<meta name="viewport" content="width=device-width, initial-scale=1"/>
<title>Kado viewer</title>
<style>
  html,body{margin:0;height:100%;overflow:hidden;background:#282c34;font-family:sans-serif}
  #c{width:100vw;height:100vh;display:block}
  #h{position:fixed;left:8px;bottom:8px;color:#aab;font-size:12px;opacity:.7}
</style>
</head>
<body>
<canvas id="c"></canvas>
<div id="h">drag: orbit &nbsp; wheel: zoom</div>
<script>
"use strict";
const MESH = {positions:[/*POSITIONS*/], indices:[/*INDICES*/], center:/*CENTER*/, radius:/*RADIUS*/};
const canvas = document.getElementById("c");
const gl = canvas.getContext("webgl2");
if (!gl) { document.body.innerHTML = "<p style='color:#fff'>WebGL2 not supported</p>"; }
const VS = `#version 300 es
in vec3 p; uniform mat4 mvp; uniform mat4 mv; out vec3 vp;
void main(){ vp = (mv * vec4(p,1.0)).xyz; gl_Position = mvp * vec4(p,1.0); }`;
const FS = `#version 300 es
precision highp float; in vec3 vp; out vec4 o;
void main(){
  vec3 n = normalize(cross(dFdx(vp), dFdy(vp)));
  vec3 L = normalize(vec3(0.4,0.6,0.8));
  float d = max(dot(n,L), 0.0);
  vec3 col = vec3(0.86,0.82,0.78) * (0.25 + 0.75*d);
  o = vec4(col, 1.0);
}`;
function sh(type, src){ const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); return s; }
const prog = gl.createProgram();
gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS));
gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS));
gl.linkProgram(prog); gl.useProgram(prog);
const pos = new Float32Array(MESH.positions);
const idx = new Uint32Array(MESH.indices);
const vbo = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
gl.bufferData(gl.ARRAY_BUFFER, pos, gl.STATIC_DRAW);
const loc = gl.getAttribLocation(prog, "p");
gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 3, gl.FLOAT, false, 0, 0);
const ibo = gl.createBuffer(); gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ibo);
gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);
gl.enable(gl.DEPTH_TEST);
let theta = 0.7, phi = 1.0, dist = MESH.radius * 3.0;
const ctr = MESH.center;
let drag = false, lx = 0, ly = 0;
canvas.addEventListener("mousedown", e => { drag = true; lx = e.clientX; ly = e.clientY; });
window.addEventListener("mouseup", () => { drag = false; });
window.addEventListener("mousemove", e => {
  if (!drag) return;
  theta -= (e.clientX - lx) * 0.01;
  phi = Math.max(0.05, Math.min(3.09, phi - (e.clientY - ly) * 0.01));
  lx = e.clientX; ly = e.clientY; draw();
});
canvas.addEventListener("wheel", e => { e.preventDefault(); dist *= Math.exp(e.deltaY * 0.001); draw(); }, {passive:false});
function mul(a,b){ const r = new Float32Array(16);
  for (let i=0;i<4;i++) for (let j=0;j<4;j++){ let s=0; for (let k=0;k<4;k++) s += a[i*4+k]*b[k*4+j]; r[i*4+j]=s; } return r; }
function transpose(m){ const r = new Float32Array(16);
  for (let i=0;i<4;i++) for (let j=0;j<4;j++) r[j*4+i] = m[i*4+j]; return r; }
function persp(f,asp,n,fa){ const t = 1/Math.tan(f/2);
  return new Float32Array([t/asp,0,0,0, 0,t,0,0, 0,0,(fa+n)/(n-fa),2*fa*n/(n-fa), 0,0,-1,0]); }
function look(ex,ey,ez,cx,cy,cz){
  let fx=cx-ex, fy=cy-ey, fz=cz-ez; const fl=Math.hypot(fx,fy,fz); fx/=fl; fy/=fl; fz/=fl;
  let ux=0, uy=0, uz=1; if (Math.abs(fz) > 0.999){ ux=0; uy=1; uz=0; }
  let rx=fy*uz-fz*uy, ry=fz*ux-fx*uz, rz=fx*uy-fy*ux; const rl=Math.hypot(rx,ry,rz); rx/=rl; ry/=rl; rz/=rl;
  const nx=ry*fz-rz*fy, ny=rz*fx-rx*fz, nz=rx*fy-ry*fx;
  return new Float32Array([rx,ry,rz,-(rx*ex+ry*ey+rz*ez), nx,ny,nz,-(nx*ex+ny*ey+nz*ez), -fx,-fy,-fz,(fx*ex+fy*ey+fz*ez), 0,0,0,1]);
}
function draw(){
  const w = canvas.width = canvas.clientWidth, h = canvas.height = canvas.clientHeight;
  gl.viewport(0,0,w,h); gl.clearColor(0.16,0.17,0.20,1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  const ex = ctr[0] + dist*Math.sin(phi)*Math.cos(theta);
  const ey = ctr[1] + dist*Math.sin(phi)*Math.sin(theta);
  const ez = ctr[2] + dist*Math.cos(phi);
  const mv = look(ex,ey,ez, ctr[0],ctr[1],ctr[2]);
  const pr = persp(0.8, w/h, MESH.radius*0.05, MESH.radius*20.0);
  const mvp = mul(pr, mv);
  gl.uniformMatrix4fv(gl.getUniformLocation(prog,"mvp"), false, transpose(mvp));
  gl.uniformMatrix4fv(gl.getUniformLocation(prog,"mv"), false, transpose(mv));
  gl.drawElements(gl.TRIANGLES, idx.length, gl.UNSIGNED_INT, 0);
}
window.addEventListener("resize", draw); draw();
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Sdf;
    use crate::extract::polygonize;

    fn cube_mesh() -> Mesh {
        polygonize(&Sdf::cuboid(Vec3::splat(1.0)), Vec3::splat(-1.5), Vec3::splat(1.5), 8)
    }

    #[test]
    fn html_has_self_contained_viewer_markers() {
        let html = encode_html(&cube_mesh());
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("getContext(\"webgl2\")"), "must use WebGL2");
        assert!(html.contains("drawElements"), "must draw the mesh");
        // 外部リソースを参照しない (オフライン / 外部送信ゼロ)。
        assert!(!html.contains("http://") && !html.contains("https://"), "no external URLs");
        assert!(!html.contains("src="), "no external script/img src");
    }

    #[test]
    fn html_embeds_full_index_data() {
        let mesh = cube_mesh();
        let (_, indices) = mesh_arrays(&mesh);
        let count = indices.split(',').count();
        assert_eq!(
            count,
            mesh.triangles.len() * 3,
            "all triangle indices must be embedded"
        );
    }

    #[test]
    fn html_is_deterministic() {
        let m = cube_mesh();
        assert_eq!(encode_html(&m), encode_html(&m));
    }

    #[test]
    fn placeholders_are_all_replaced() {
        let html = encode_html(&cube_mesh());
        for ph in ["/*POSITIONS*/", "/*INDICES*/", "/*CENTER*/", "/*RADIUS*/"] {
            assert!(!html.contains(ph), "placeholder {ph} must be replaced");
        }
    }

    #[test]
    fn empty_mesh_produces_valid_html_with_nonzero_radius() {
        // 問132: 空メッシュ (bounds が None) で encode_html がパニックせず、
        // WebGL の persp() の near 平面が 0 にならないよう radius ≥ 1e-3 を保証する。
        // near = MESH.radius * 0.05 が 0 だと投影行列の (2,3) 要素が 0 になり
        // 全 z 座標が NaN/Inf になる (数値崩壊)。
        let html = encode_html(&Mesh::default());
        // すべてのプレースホルダが置換されている。
        for ph in ["/*POSITIONS*/", "/*INDICES*/", "/*CENTER*/", "/*RADIUS*/"] {
            assert!(!html.contains(ph), "placeholder {ph} must be replaced even for empty mesh");
        }
        // radius は 1e-3 フォールバック (4桁固定精度: "0.0010")。0 ではない。
        assert!(
            html.contains("radius:0.0010"),
            "empty mesh must embed fallback radius 1e-3 (0.0010), not zero"
        );
        // center はゼロ原点 (空 bounds のフォールバック)。
        assert!(
            html.contains("center:[0.0000,0.0000,0.0000]"),
            "empty mesh center must fall back to origin"
        );
    }

    #[test]
    fn html_sanitizes_nonfinite_coordinates() {
        // 問216 (バグ修正): mesh_arrays は以前 {:.4} で非有限座標を出力していたため
        // NaN/Inf 頂点が "NaN"/"inf" 文字列になり MESH.positions が構文エラーになった。
        // 3MF の finite_coord と同様に 0.0 へサニタイズする修正を固定する。
        let bad = Mesh {
            vertices: vec![
                Vec3::new(f64::NAN, 0.0, 0.0),
                Vec3::new(f64::INFINITY, 1.0, 0.0),
                Vec3::new(0.0, f64::NEG_INFINITY, 1.0),
            ],
            triangles: vec![[0, 1, 2]],
        };
        let html = encode_html(&bad);
        let lower = html.to_lowercase();
        // 埋め込み JS に "nan"/"inf" リテラルが現れてはならない。
        // (テンプレ固定文字列に nan/inf を含まないことは別途確認済みの前提)
        assert!(!lower.contains("nan"), "sanitized HTML must not contain 'nan' literal");
        assert!(!lower.contains("inf"), "sanitized HTML must not contain 'inf' literal");
        // 非有限は 0.0000 に置換される。
        assert!(html.contains("0.0000"), "non-finite coords must become 0.0000");
    }
}
