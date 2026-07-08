# Kado 仕様書 (SPEC)

> 状態: ドラフト ｜ 日付: 2026-06-20 ｜ 対象バージョン: 0.1.0
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
| 正多角形プリズム | `prism(sides, radius, half_height)` | sides ≥ 3（整数）, radius > 0, hh > 0 | Z軸押し出し・外接円半径基準。**厳密 SDF**（IQ sdRegularPolygon + 厳密押し出し）。n→∞ で cylinder に収束（問269） |

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
| 任意軸回転 | `rotate_axis(axis, angle)` | Rodrigues の回転公式。axis は内部で単位化（ゼロ軸は eval 層で拒否）。`axis=(1,0,0)/(0,1,0)/(0,0,1)` で `rotate_x/y/z` と数式的に一致（問266） |
| 非一様スケール | `scale_xyz(s)` | s の各成分 > 0（eval 層で拒否）。距離場は厳密には保たないが**符号は常に厳密・大きさは常に真の距離以下（保守的過小評価）・結果の場は厳密に Lipschitz=1**（証明は socratic-review.md 問276。楕円体の「Lipschitz≈1」より強い保証） |
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
- 座標 1 単位 = 1 mm。検証レポートは `dims_mm` を明示。`volume` は mm³、`surface_area`/
  `bed_contact_area` は mm²。材料/質量見積もり: `mass_g = volume/1000 × density`
  （PLA~1.24 等）を help が案内する（問253）。solid volume は infill 前の上限値。

---

## 5. MCP サーバ（道具）

トランスポートは **stdio + Content-Length フレーミング**（LSP スタイル）。
プロトコルは JSON-RPC 2.0。`protocolVersion` は **2025-06-18 / 2024-11-05 を版交渉**
（クライアントが要求した対応版を返し、未対応なら最新を返す・問251）。
各ツールは `annotations`（`readOnlyHint` / `destructiveHint` / `idempotentHint` /
`openWorldHint`）を宣言し、クライアント/LLM が読み取り専用ツールと状態変更ツールを
区別できる（問251）。validate レポートは `bed_contact_area`（build_dir 最下層の接地
面積; 反り・剥がれ抵抗の指標）を測定値として公開する（問252）。

`id` を持つリクエストは必ず応答を受け取る（`method` 欠落/非文字列でも無応答のまま
放置されない・問261）。`id` を持たない通知形メッセージのみ JSON-RPC 2.0 の慣例通り
無応答。1通のフレームの本文が不正（非UTF-8・不正JSON）でも**セッション全体は
終了しない**——ストリーム上のフレーム区切り（Content-Length）自体は健全なので
Parse error を返して次のメッセージを処理し続ける（問264）。フレーミング自体が
壊れている場合（Content-Length 欠落・上限超過・EOF）のみ再同期不能として接続を終了する。

| エラーコード | 意味 |
|--------------|------|
| `-32600` | Invalid Request（`method` 欠落または非文字列。問261） |
| `-32601` | Method not found（未知のトップレベル JSON-RPC メソッド） |
| `-32700` | Parse error（フレーム本文が非UTF-8または不正JSON。セッションは継続する・問264） |

ツール呼び出し (`tools/call`) 自体のエラー（未知ツール名・引数不正等）はこの
JSON-RPC エラーではなく、`result.isError=true` を伴うツール結果として返る（問106）。

### 5.1 ツール一覧

| ツール | 機能 | 主要引数 |
|--------|------|---------|
| `run_script` | DSL/JSON スクリプトを評価しシーン正本を更新 | `script` |
| `eval` | 1 点の符号付き距離を返す | `x, y, z` |
| `validate` | DFM 検証レポート（§5.2）。戻り値 JSON: `{ok, triangles, manifold, volume, volume_reliable, surface_area, bbox, dims_mm, center_of_mass, measured_min_wall, body_count, cavity_count, bed_contact_area, digest, issues:[{severity, code, cause, hints, location}]}`。`measured_min_wall` は閾値と独立に常に測定する実測最小肉厚（問247）。`body_count`/`cavity_count` は中実ボディ数/内部空洞数（非水密は null・問248）。`bed_contact_area` は造形プレート接地面積（点接地は ~0・問252） | `resolution, min_wall_mm, max_overhang_deg, build_dir` |
| `screenshot` | シーンを PNG レンダリング（base64） | `view, width, height, samples, axes, projection` |
| `export` | STL/GLB/3MF をプロジェクト直下へ書き出し | `path, resolution` |
| `get_scene` | 現在のシーン正本（スクリプト）を返す | — |
| `undo_script` | 1 段階 undo（`prev_scene`） | — |
| `help` | DSL/ツールのリファレンス | — |

### 5.2 検証レポートの issue コード

| コード | 種別 | 意味 |
|--------|------|------|
| `EMPTY_MESH` | error | 三角形ゼロ（形状が空） |
| `OPEN_MESH` | error | 開境界（非水密）。location = 最小インデックスの境界エッジ中点（問258） |
| `NON_MANIFOLD` | error | 3 面以上共有エッジ（自己交差等） |
| `NEGATIVE_VOLUME` | warn | 向き反転（体積が負）。**閉じたメッシュでのみ判定**（開境界では signed_volume が無意味なため抑制し、OPEN_MESH のみ出す。問245） |
| `MULTIPLE_BODIES` | warn | 複数の独立ボディ |
| `THIN_WALL` | warn | 最小肉厚 < 閾値。`SUSPICIOUS_SCALE` が発火した場合は抑制される（問256: スケールミス修正が優先） |
| `OVERHANG` | warn | オーバーハング角 > 閾値（ベッド接地面・直下に材料がある面は支持済みとして除外）。最悪角度に加え総表面積に占めるオーバーハング**面積割合**を報告し、AI が比例的に対応できる（問249）。fix_hints に**ブリッジ vs カンチレバーの区別**を案内：両端支持の水平天井（ブリッジ）は数 mm の短スパンならサポート不要；一端支持（カンチレバー）や長スパンは要サポート（問254） |
| `SUSPICIOUS_SCALE` | warn | 部品全体が `min_wall_mm` より小さい（単位/スケールの誤りが濃厚）。発火時は `THIN_WALL` を抑制する（問256） |
| `UNSTABLE` | warn | 重心がベース接地面の足元から外れる（転倒する物理挙動）。`issue.location` = COM 座標 |
| `HIGH_ASPECT_RATIO` | warn | ビルド高さ / 横方向最大寸法 > 8（FDM 印刷中の揺れリスク）。UNSTABLE と相補的な製造プロセス安定性軸 |
| `ENCLOSED_CAVITY` | info | 外部に開口のない完全密閉の内部空洞（SLA で未硬化樹脂・FDM で除去不能サポートを閉じ込める）。中空シェルと密閉トラップはメッシュ上区別不能のため Info（`is_ok` を倒さない・問246） |

### 5.3 issue の `location` フィールド（問242/243）

各 issue は `location: [x,y,z] | null` を持つ。AI エージェントが「どこを直すか」を直接参照できる。

| issue コード | location の意味 |
|-------------|----------------|
| `OVERHANG` | 最悪三角形の重心座標 |
| `THIN_WALL` | プローブが最小肉厚を検出した表面頂点 |
| `UNSTABLE` | 重心（COM）座標 = `Report.center_of_mass` と同一 |
| `NON_MANIFOLD` | 最小頂点インデックスの非多様体エッジ中点（決定的・問257） |
| `OPEN_MESH` | 最小頂点インデックスの開境界エッジ中点（決定的・問258） |
| `HIGH_ASPECT_RATIO` | ビルド方向最上位 10% の頂点重心（揺れの起点・問255） |
| その他 | `null`（空間的意味を持たない issue） |

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

MCP 経由の画像寸法は `[1, MAX_IMAGE_DIM]` にクランプされ 0 に到達しないが、
`render::render()` 自体も pub fn として `width==0`/`height==0` を防御する
（`draw_axes` と同じガード。呼び出し側の契約に依存しない防御・問259）。

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

- `cargo test`: 379 テスト（368 ライブラリユニット + 5 CLI ユニット + 6 統合）合格。
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: rustdoc 警告ゼロ（問275）。
- `cargo clippy --all-targets -- -D warnings`: 警告ゼロ。
- ソクラテス問答（`docs/socratic-review.md`）: 問1–202 を継続的に吟味・固定。

---

> 本書は実装の進展（問答ループ v84–v94+）に追従して更新する。
> 各契約の根拠は `docs/socratic-review.md` の対応する問番号を参照。
