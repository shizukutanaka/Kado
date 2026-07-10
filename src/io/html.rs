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
    let radius = if raw_radius.is_finite() {
        raw_radius.max(1e-3)
    } else {
        1e-3
    };
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
  #c{width:100vw;height:100vh;display:block;touch-action:none}
  #h{position:fixed;left:8px;bottom:8px;color:#aab;font-size:12px;opacity:.7}
  #err{position:fixed;left:0;top:0;right:0;padding:16px;color:#fff;background:#402020;font:14px sans-serif;white-space:pre-wrap}
</style>
</head>
<body>
<canvas id="c"></canvas>
<div id="h">drag / touch: orbit &nbsp; wheel / pinch: zoom &nbsp; double-click: reset view</div>
<script>
"use strict";
const MESH = {positions:[/*POSITIONS*/], indices:[/*INDICES*/], center:/*CENTER*/, radius:/*RADIUS*/};
const canvas = document.getElementById("c");
function fail(msg) {
  const d = document.createElement("div");
  d.id = "err"; d.textContent = msg;
  document.body.appendChild(d);
}
const gl = canvas.getContext("webgl2");
if (!gl) {
  fail("WebGL2 is not supported by this browser. Try a recent Chrome, Firefox, Safari, or Edge.");
} else {
  try {
    run(gl);
  } catch (err) {
    fail("Viewer failed to start: " + err.message);
  }
}
function run(gl) {
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
function sh(type, src){
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
    throw new Error("shader compile error: " + gl.getShaderInfoLog(s));
  }
  return s;
}
const prog = gl.createProgram();
gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS));
gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS));
gl.linkProgram(prog);
if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
  throw new Error("program link error: " + gl.getProgramInfoLog(prog));
}
gl.useProgram(prog);
const pos = new Float32Array(MESH.positions);
const idx = new Uint32Array(MESH.indices);
const vbo = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
gl.bufferData(gl.ARRAY_BUFFER, pos, gl.STATIC_DRAW);
const loc = gl.getAttribLocation(prog, "p");
gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 3, gl.FLOAT, false, 0, 0);
const ibo = gl.createBuffer(); gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ibo);
gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, idx, gl.STATIC_DRAW);
gl.enable(gl.DEPTH_TEST);
const DEFAULT_THETA = 0.7, DEFAULT_PHI = 1.0, DEFAULT_DIST = MESH.radius * 3.0;
let theta = DEFAULT_THETA, phi = DEFAULT_PHI, dist = DEFAULT_DIST;
const ctr = MESH.center;
const MIN_DIST = MESH.radius * 0.6, MAX_DIST = MESH.radius * 30.0;
function clampDist(d){ return Math.max(MIN_DIST, Math.min(MAX_DIST, d)); }
function resetView(){ theta = DEFAULT_THETA; phi = DEFAULT_PHI; dist = DEFAULT_DIST; draw(); }
let drag = false, lx = 0, ly = 0;
canvas.addEventListener("mousedown", e => { drag = true; lx = e.clientX; ly = e.clientY; });
window.addEventListener("mouseup", () => { drag = false; });
window.addEventListener("mousemove", e => {
  if (!drag) return;
  theta -= (e.clientX - lx) * 0.01;
  phi = Math.max(0.05, Math.min(3.09, phi - (e.clientY - ly) * 0.01));
  lx = e.clientX; ly = e.clientY; draw();
});
canvas.addEventListener("dblclick", resetView);
canvas.addEventListener("wheel", e => { e.preventDefault(); dist = clampDist(dist * Math.exp(e.deltaY * 0.001)); draw(); }, {passive:false});
function touchPoints(e){ return Array.from(e.touches).map(t => ({x:t.clientX, y:t.clientY})); }
let prevTouches = null;
canvas.addEventListener("touchstart", e => { e.preventDefault(); prevTouches = touchPoints(e); }, {passive:false});
canvas.addEventListener("touchend", e => { prevTouches = e.touches.length ? touchPoints(e) : null; }, {passive:false});
canvas.addEventListener("touchcancel", () => { prevTouches = null; }, {passive:false});
canvas.addEventListener("touchmove", e => {
  e.preventDefault();
  const cur = touchPoints(e);
  if (prevTouches && prevTouches.length === 1 && cur.length === 1) {
    theta -= (cur[0].x - prevTouches[0].x) * 0.01;
    phi = Math.max(0.05, Math.min(3.09, phi - (cur[0].y - prevTouches[0].y) * 0.01));
    draw();
  } else if (prevTouches && prevTouches.length === 2 && cur.length === 2) {
    const pd = Math.hypot(prevTouches[0].x - prevTouches[1].x, prevTouches[0].y - prevTouches[1].y);
    const cd = Math.hypot(cur[0].x - cur[1].x, cur[0].y - cur[1].y);
    if (pd > 0) { dist = clampDist(dist * (pd / cd)); draw(); }
  }
  prevTouches = cur;
}, {passive:false});
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
  const near = Math.max(MESH.radius*0.01, dist - MESH.radius*2.0);
  const far = dist + MESH.radius*2.0;
  const pr = persp(0.8, w/h, near, far);
  const mvp = mul(pr, mv);
  gl.uniformMatrix4fv(gl.getUniformLocation(prog,"mvp"), false, transpose(mvp));
  gl.uniformMatrix4fv(gl.getUniformLocation(prog,"mv"), false, transpose(mv));
  gl.drawElements(gl.TRIANGLES, idx.length, gl.UNSIGNED_INT, 0);
}
window.addEventListener("resize", draw); draw();
}
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
        polygonize(
            &Sdf::cuboid(Vec3::splat(1.0)),
            Vec3::splat(-1.5),
            Vec3::splat(1.5),
            8,
        )
    }

    #[test]
    fn html_has_self_contained_viewer_markers() {
        let html = encode_html(&cube_mesh());
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("getContext(\"webgl2\")"), "must use WebGL2");
        assert!(html.contains("drawElements"), "must draw the mesh");
        // 外部リソースを参照しない (オフライン / 外部送信ゼロ)。
        assert!(
            !html.contains("http://") && !html.contains("https://"),
            "no external URLs"
        );
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
    fn html_supports_touch_input() {
        // 問284: モバイル/タブレット (マウスを持たない端末) でも操作できるよう、
        // touchstart/touchmove/touchend でのオービット・ピンチズームに対応する。
        let html = encode_html(&cube_mesh());
        assert!(html.contains("touchstart"), "must handle touchstart");
        assert!(html.contains("touchmove"), "must handle touchmove");
        assert!(html.contains("touchend"), "must handle touchend");
        assert!(html.contains("touch-action:none"), "must disable native touch gestures on canvas so custom orbit/pinch doesn't fight the browser");
    }

    #[test]
    fn html_checks_shader_compile_and_link_status() {
        // 問284: 旧実装は gl.compileShader/linkProgram の成否を確認せず、失敗時に
        // 空白キャンバスのまま無言で壊れていた (シェーダエラーがコンソールにすら
        // 出ない)。COMPILE_STATUS/LINK_STATUS を確認し、失敗時は画面に理由を
        // 表示する (プロジェクト全体の「サイレント失敗させない」方針と整合)。
        let html = encode_html(&cube_mesh());
        assert!(
            html.contains("COMPILE_STATUS"),
            "must check shader compile status"
        );
        assert!(
            html.contains("LINK_STATUS"),
            "must check program link status"
        );
        assert!(
            html.contains("getShaderInfoLog"),
            "must surface the compiler's error message"
        );
        assert!(
            html.contains("getProgramInfoLog"),
            "must surface the linker's error message"
        );
        assert!(
            html.contains("#err{"),
            "must style a visible error surface, not just console.error"
        );
        assert!(
            html.contains("d.id = \"err\""),
            "must create and show the error element on failure"
        );
    }

    #[test]
    fn html_clamps_zoom_distance() {
        // 問284: 旧実装は wheel イベントで dist を無制限に掛け続けており、
        // ズームアウトし過ぎると far クリップ面 (旧: 固定 radius*20) を超えて
        // 対象物が消える、ズームインし過ぎると near クリップ面を割り込むという
        // 退化ケースがあった。MIN_DIST/MAX_DIST でのクランプに加え、near/far
        // 自体を現在の dist に追従させることで、クランプ範囲内なら常に対象物が
        // 可視のままであることを保証する。
        let html = encode_html(&cube_mesh());
        assert!(html.contains("MIN_DIST"), "must clamp zoom-in distance");
        assert!(html.contains("MAX_DIST"), "must clamp zoom-out distance");
        assert!(
            html.contains("clampDist"),
            "clamp must be applied consistently to wheel and pinch"
        );
        // near/far は固定値ではなく dist に追従させる (旧実装の固定 far=radius*20 は
        // dist が離れるほど対象物が far を超えて消える退化バグだった)。
        assert!(
            html.contains("dist - MESH.radius*2.0") && html.contains("dist + MESH.radius*2.0"),
            "near/far must track the current distance, not stay fixed"
        );
    }

    #[test]
    fn html_supports_double_click_reset() {
        // 問284: ドラッグ/ズームで見失った視点を戻す手段がリロードしかなかった。
        // ダブルクリックで初期視点に戻せるようにする。
        let html = encode_html(&cube_mesh());
        assert!(
            html.contains("dblclick"),
            "must support double-click to reset the view"
        );
        assert!(
            html.contains("resetView"),
            "reset must restore the original theta/phi/dist"
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
            assert!(
                !html.contains(ph),
                "placeholder {ph} must be replaced even for empty mesh"
            );
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
        // 問284: テンプレート自体が (シェーダエラー表示のため) `getShaderInfoLog`
        // 等の正当な API 名を含むようになり、小文字化した全文検索では
        // "info" → "inf" に誤ヒットする。埋め込みデータ行 (`const MESH = ...;`)
        // だけを対象に、数値リテラルとしての "nan"/"inf" 混入を検査する。
        let mesh_line = html
            .lines()
            .find(|l| l.trim_start().starts_with("const MESH ="))
            .expect("MESH data line must exist");
        let lower = mesh_line.to_lowercase();
        assert!(
            !lower.contains("nan"),
            "sanitized MESH data must not contain 'nan' literal"
        );
        assert!(
            !lower.contains("inf"),
            "sanitized MESH data must not contain 'inf' literal"
        );
        // 非有限は 0.0000 に置換される。
        assert!(
            html.contains("0.0000"),
            "non-finite coords must become 0.0000"
        );
    }
}
