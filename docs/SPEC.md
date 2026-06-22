# Kado 仕様書 (SPEC)

> 状態: ドラフト ｜ 日付: 2026-06-20 ｜ 対象バージョン: 0.0.1
> 上位文書: `Plan.md`（要件）, `docs/adr/ADR-003`（言語決定）, `docs/socratic-review.md`（設計吟味議事録）
> 本書の位置づけ: Plan.md の要件と問答で確定した不変条件を、**実装済みの観測可能な振る舞い**として明文化する。
> 「何を保証するか（契約）」と「保証しないか（非目標）」を区別し、テストで固定された事実のみを記す。

---

## 1. 概要

Kado は **AI-First のローカル幾何エンジン**である。SDF（符号付き距離場）木を正本とし、
スクリプト（DSL/JSON）から決定的にメッシュを抽出・検証・出力する。MCP サーバとして
AI エージェントに「形状を作る・検証する・書き出す」道具を提供する。

### 1.1 設計の柱（Plan.md 由来の横断制約）

| # | 制約 | 本書での扱い |
|---|------|-------------|
| C1 | **外部送信ゼロ** | §7.1 — ネットワーク I/O を持たない |
| C2 | **単一自己完結バイナリ** | §2 — コアは std のみ（ADR-003） |
| C3 | **決定的出力** | §6 — 同一バイナリ・同一 arch でバイト同一 |
| C4 | **全機能ヘッドレス動作** | §5 — CLI / MCP（stdio）で完結 |
| C5 | **単位 = ミリメートル** | §4.4 — 座標 1 単位 = 1 mm |

---

## 2. アーキテクチャ

```
スクリプト(DSL/JSON)  ──parse──▶  SDF木(core::Sdf)  ──polygonize──▶  Mesh
   (正本)                          (決定的射影)         (Marching Tetrahedra)
                                       │                      │
                                       │                      ├─▶ verify::check (DFM検証)
                                       │                      └─▶ io (STL/GLB/3MF/HTML)
                                       └─▶ render (ラスタライザ→PNG)
```

| モジュール | 責務 | 外部依存 |
|-----------|------|---------|
| `core` | Vec3 演算 / SDF 木 / eval / aabb / sampling_box | std のみ |
| `extract` | Marching Tetrahedra / Mesh / 多様体検査 / 体積 | std のみ |
| `render` | ラスタライザ / Image / PNG エンコード | std のみ |
| `io` | STL / glTF(GLB) / 3MF / HTML / ZIP | std のみ |
| `script` | DSL パーサ / JSON 評価器 / リソース予算 | std のみ |
| `verify` | DFM 検証（肉厚・オーバーハング・多様体・体積） | std のみ |
| `mcp` | JSON-RPC 2.0 サーバ / ツール / セッション | std のみ |
| `cli` | エントリポイント | std のみ |

**依存方針（ADR-003 / 問4）**: コアは外部 crate を持ち込まない。テスト補助に限り devDependencies を許容。

---

## 3. SDF 木（core::Sdf）

### 3.1 プリミティブ

| 種別 | 構築子 | パラメータ | 備考 |
|------|--------|-----------|------|
| 球 | `sphere(r)` | r > 0 | |
| 直方体 | `cuboid(half)` | 各半幅 > 0 | half は半幅ベクトル |
| 円柱 | `cylinder(r, half_height)` | r > 0, hh > 0 | 第2引数は**半高** |
| トーラス | `torus(major, minor)` | minor < major | ring torus のみ（§4.2） |
| 円錐 | `cone(r, h)` | r > 0, h > 0 | |
| カプセル | `capsule(half_height, r)` | hh ≥ 0, r > 0 | **引数順は (hh, r)**。hh=0 は球 |
| 角丸箱 | `rounded_box(half, r)` | half > 0, r > 0 | |
| 楕円体 | `ellipsoid(radii)` | 各半径 > 0 | IQ 近似（軸上は厳密） |

### 3.2 CSG（集合演算）

| 種別 | 構築子 | eval 定義 |
|------|--------|----------|
| 和 | `union(a, b)` | `min(da, db)` |
| 積 | `intersection(a, b)` | `max(da, db)` |
| 差 | `difference(a, b)` | `max(da, -db)` |
| 平滑和 | `smooth_union(a, b, k)` | k > 0 の多項式ブレンド |
| 平滑積 | `smooth_intersection(a, b, k)` | k > 0 |
| 平滑差 | `smooth_difference(a, b, k)` | k > 0 |

### 3.3 変換・修飾

| 種別 | 構築子 | 備考 |
|------|--------|------|
| 平行移動 | `translate(v)` | |
| スケール | `scale(s)` | s > 0（距離場保存）。s≤0 は eval 層で拒否 |
| オフセット | `offset(amount)` | 正=膨張 / 負=収縮（両方許可） |
| シェル | `shell(thickness)` | thickness > 0。t=0 は \|d\|（eval 層で拒否） |
| 反復 | `repeat_n(period, count)` | 有限反復。count=0 または period=0 で軸無効 |
| 鏡映 | `mirror_x/y/z()` | `child.eval(\|x\|,..)`：+側を−側へ反射 |
| 回転 | `rotate_x/y/z(angle)` | 剛体・距離保存。angle はラジアン |
| カット | `cut(normal, offset)` | 平面 `dot(p,n)=offset` で半空間と交差。`dot(p,n)≤offset` 側を残す。法線は単位化。断面用。AABB は子の AABB（材料を削るのみ） |
| 平坦化 | `flatten(at)` | FDM 印刷の平坦底面。z=at（既定 0）で底を切り z≥at を残す。`cut` の最頻用ケースの意図明示型別名（法線方向の取り違え回避）。内部で `cut((0,0,−1),−at)` に lower |

### 3.4 中核メソッドの契約

- **`eval(p) -> f64`**: 点 p における符号付き距離。内部負・外部正・表面 0。
- **`aabb() -> (Vec3, Vec3)`**: 軸整列境界箱。非重複 CSG では**反転しうる**（lo > hi）。
- **`sampling_box() -> (Vec3, Vec3)`**: 抽出用境界。**常に正規化**（lo ≤ hi）し余白を付加。反転 AABB を安全に吸収する。

---

## 4. 不変条件（テストで固定済み）

本節は `docs/socratic-review.md`（問1–202）で確定し、ユニットテストで固定された契約を要約する。

### 4.1 距離場の正しさ
- 球面・各プリミティブの表面で eval ≈ 0、軸上距離は厳密。
- 楕円体の極端縦横比（1000:0.001:0.5）でも eval は有限かつ符号は厳密。
- 平滑 CSG は k → 0（1e-300 まで）で NaN/Inf を生まない（clamp 吸収）。

### 4.2 幾何的妥当性の事前拒否
- r ≤ 0 / 半幅 ≤ 0 / scale ≤ 0 / shell ≤ 0 は**評価前にエラー**（無音の空メッシュを防ぐ）。
- torus は minor ≥ major（horn/spindle）を拒否（自己交差＝非多様体を防ぐ）。
- 非有限パラメータ（1e400→inf）はパーサ層で拒否。

### 4.3 抽出・メッシュの健全性
- Marching Tetrahedra の 6 四面体は単位立方体を**隙間なく充填**（体積和 = 1.0）。
- `edge_vertex` は t を [0,1] にクランプし、結果は必ず線分上（セル外へ外挿しない）。
- 単一三角形は boundary=3, nonmanifold=0（`is_edge_manifold` は開境界も検出するため false）。
- `body_components` は反復呼び出しで決定的（HashMap 反復順に非依存）。

### 4.4 単位とスケール
- 座標 1 単位 = 1 mm。検証レポートは `dims_mm` を明示。

---

## 5. MCP サーバ（道具）

トランスポートは **stdio + Content-Length フレーミング**（LSP スタイル）。
プロトコルは JSON-RPC 2.0、`protocolVersion = 2024-11-05`。

### 5.1 ツール一覧

| ツール | 機能 | 主要引数 |
|--------|------|---------|
| `run_script` | DSL/JSON スクリプトを評価しシーン正本を更新 | `script` |
| `eval` | 1 点の符号付き距離を返す | `x, y, z` |
| `validate` | DFM 検証レポート（§5.2） | `resolution, min_wall_mm, max_overhang_deg, build_dir` |
| `screenshot` | シーンを PNG レンダリング（base64） | `view, width, height, samples, axes` |
| `export` | STL/GLB/3MF をプロジェクト直下へ書き出し | `path, resolution` |
| `get_scene` | 現在のシーン正本（スクリプト）を返す | — |
| `undo_script` | 1 段階 undo（`prev_scene`） | — |
| `help` | DSL/ツールのリファレンス | — |

### 5.2 検証レポートの issue コード

| コード | 種別 | 意味 |
|--------|------|------|
| `EMPTY_MESH` | error | 三角形ゼロ（形状が空） |
| `OPEN_MESH` | error | 開境界（非水密） |
| `NON_MANIFOLD` | error | 3 面以上共有エッジ（自己交差等） |
| `NEGATIVE_VOLUME` | error | 向き反転（体積が負） |
| `MULTIPLE_BODIES` | warn | 複数の独立ボディ |
| `THIN_WALL` | warn | 最小肉厚 < 閾値 |
| `OVERHANG` | warn | オーバーハング角 > 閾値 |
| `SUSPICIOUS_SCALE` | warn | 部品が自身の最小肉厚より小さい |

---

## 6. 決定性（C3）

- **f64 固定**: すべて f64。`mul_add`(FMA) は使わず素朴な `a*b + c`（プラットフォーム差回避）。
- **演算順序固定**: dot/cross 等は固定順序。並列リダクションはツリー縮約の順序固定。
- **超越関数**: std libm に固定。クロスプラットフォームは数値同等だが**ビット同一は保証しない**（同一 arch 内のみバイト同一）。
- **観測可能性**: `Mesh::digest()`（FNV-1a 64bit）で再現性を短いハッシュ 1 つで検証可能。
- **エンコード決定性**: STL/GLB/PNG は同一メッシュからバイト同一。

---

## 7. セキュリティ

### 7.1 外部送信ゼロ（C1）
- ネットワークソケットを開かない。MCP は stdio のみ。

### 7.2 書き込みサンドボックス（問15）
- `export` の書き込み先は**プロジェクト直下のみ**。
- 絶対パス・`..` トラバーサル・ルート/プレフィックス付きパスを拒否。
- 空・空白のみのパスを拒否。
- Unix ではバックスラッシュは literal ファイル名として扱われ脱出不能（パス構造で判定）。

### 7.3 DSL サンドボックス
- ホワイトリスト AST：既知の演算子のみ。未知演算子はトップ/入れ子いずれも拒否。
- 引数アリティ厳密：不足/過剰（0 引数含む）はエラー。
- import・任意コード実行なし。

### 7.4 リソース上限（DoS 防御）

| 上限 | 値 | 場所 |
|------|-----|------|
| `MAX_SOURCE_BYTES` | 1 MiB | script/eval |
| `MAX_NODES` | 50,000 | script/eval（共有予算） |
| `MAX_DEPTH`（scene） | 64 | script/eval |
| `MAX_DEPTH`（JSON） | 128 | mcp/json |
| `MAX_REPEAT` | 256 | script/eval |
| `MAX_MESSAGE_BYTES` | 16 MiB | mcp/server（確保前に検査） |
| `MAX_RESOLUTION` | 256 | mcp/tools |
| `MAX_IMAGE_DIM` | 4096 | mcp/tools |

---

## 8. 出力フォーマット

| 形式 | モジュール | 用途 |
|------|-----------|------|
| STL（binary） | io/stl | 汎用メッシュ |
| glTF（GLB） | io/gltf | インデックス付き＋境界（accessor min/max） |
| 3MF | io/threemf | 製造（単位 mm 宣言） |
| HTML | io/html | 自己完結ビューア |
| PNG | render/image | スクリーンショット（deflate store・外部依存ゼロ） |

---

## 9. 非目標（保証しないこと）

- **クロスプラットフォームのビット同一**: 同一 arch 内のみ保証（超越関数の libm 差）。
- **薄肉の完全検出**: `min_wall_probe` はステップ（diag/256）より薄い壁を見落としうる（安全側の補助）。
- **楕円体の軸外厳密距離**: IQ 近似（符号と軸上距離のみ厳密）。
- **メッシュ CSG**: 採用しない（数値破綻回避のため SDF 経由）。
- **無限反復**: `repeat` は count による有限反復のみ。

---

## 10. 品質ゲート

- `cargo test`: 270 テスト（267 ユニット + 3 統合）合格。
- `cargo clippy --all-targets -- -D warnings`: 警告ゼロ。
- ソクラテス問答（`docs/socratic-review.md`）: 問1–202 を継続的に吟味・固定。

---

> 本書は実装の進展（問答ループ v84–v94+）に追従して更新する。
> 各契約の根拠は `docs/socratic-review.md` の対応する問番号を参照。
