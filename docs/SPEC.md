# Kado 仕様書 (SPEC)

> 状態: ドラフト ｜ 日付: 2026-08-18 ｜ 対象バージョン: 0.2.0
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
| C3 | **決定的出力** | §6 — 同一バイナリ・同一 arch でバイト同一。**スレッド数にも非依存**（問312） |
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
| 平滑和 | `smooth_union(a, b, k)` | k > 0 の多項式ブレンド（**丸い**フィレット） |
| 平滑積 | `smooth_intersection(a, b, k)` | k > 0 |
| 平滑差 | `smooth_difference(a, b, k)` | k > 0 |
| 面取り和 | `chamfer_union(a, b, k)` | k > 0。**平面（45°）**の面取り（IQ/hg_sdf）。丸い smooth_* の角度版（問285） |
| 面取り積 | `chamfer_intersection(a, b, k)` | k > 0 |
| 面取り差 | `chamfer_difference(a, b, k)` | k > 0 |

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
プロトコルは JSON-RPC 2.0。`protocolVersion` は **2025-11-25 / 2025-06-18 / 2024-11-05
を版交渉**（クライアントが要求した対応版を返し、未対応なら最新を返す・問251/問286）。
最新安定版 **2025-11-25** を既定とし、`serverInfo.description`（同版で Implementation
に追加された任意フィールド）を宣言する。同版が明確化した「入力検証エラーは Protocol
Error でなく Tool Execution Error（`isError:true`）で返す」規約に Kado は問106 以来
適合している。各ツールは `annotations`（`readOnlyHint` / `destructiveHint` /
`idempotentHint` / `openWorldHint`）を宣言し、クライアント/LLM が読み取り専用ツールと
状態変更ツールを区別できる（問251）。validate レポートは `bed_contact_area`（build_dir 最下層の接地
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
| `measure` | **光線に沿った表面交差**を返し、隣接交差間の距離（span）＝穴径・肉厚・面間距離を 1 呼出で与える（問299）。sphere tracing（Hart 1996）で薄い特徴も飛び越さない。SDF の**符号**のみに依拠するため距離は厳密（`eval` の大きさは合成形状で下界に過ぎない）。**`complete` を必ず返す**——上限で打ち切られた場合 `false` となり、結果が不完全であることを AI が判別できる（問301） | `from[3], dir[3], max_distance` |
| `validate` | DFM 検証レポート（§5.2）。戻り値 JSON: `{ok, triangles, manifold, volume, volume_reliable, surface_area, bbox, dims_mm, center_of_mass, measured_min_wall, body_count, cavity_count, bed_contact_area, aspect_ratio, digest, issues:[{severity, code, cause, hints, location}]}`。`measured_min_wall` は閾値と独立に常に測定する実測最小肉厚（問247）。`body_count`/`cavity_count` は中実ボディ数/内部空洞数（非水密は null・問248）。`bed_contact_area` は造形プレート接地面積（点接地は ~0・問252）。`aspect_ratio` は実測の細長さ比で、閾値 `max_aspect_ratio`（既定 8・0 でスキップ）の発火と独立に常に公開（問305） | `resolution, min_wall_mm, max_overhang_deg, max_aspect_ratio, build_dir` |
| `screenshot` | シーンを PNG レンダリング（base64）。`axes=true`（既定）のとき X=赤/Y=緑/Z=青のグノモンに **mm 目盛り**（1/10/100mm を軸長から決定的に選択・問288）を刻み、応答に目盛り間隔を記した text を添えて AI が画像から寸法を概算できるようにする | `view, width, height, samples, axes, projection` |
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
| `HIGH_ASPECT_RATIO` | warn | ビルド高さ / 横方向最大寸法が `max_aspect_ratio`（既定 8）超（FDM 印刷中の揺れリスク）。UNSTABLE と相補的な製造プロセス安定性軸。**安全な比は絶対寸法・ノズル径・材料に依存する**ため閾値は調整可能（0 でスキップ）。実測比は閾値と独立に `aspect_ratio` として常に公開（問305） |
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
| glTF（GLB） | io/gltf | インデックス付き＋境界（accessor min/max）＋**頂点法線**（面積重み付き平滑法線・問290）＋**既定マット材質**（metallic=0/roughness=0.6/中立グレー・問291; ビューアの暗い既定材質フォールバックを避け素直な陰影で表示） |
| 3MF | io/threemf | 製造（単位 mm 宣言） |
| HTML | io/html | 自己完結ビューア |
| PNG | render/image, render/deflate | スクリーンショット（PNG 行フィルタ None/Sub/Up を行ごとに最小絶対値和で選択・問287、決定的 DEFLATE：固定 Huffman＋距離{1,3}限定 RLE と無圧縮の小さい方を採用・問281。全 None 版とも比較し小さい方を採る＝旧実装より悪化しない・外部依存ゼロ） |

---

## 9. 非目標（保証しないこと）

- **クロスプラットフォームのビット同一**: 同一 arch 内のみ保証（超越関数の libm 差）。
- **オーバーハングの支持判定**: 下向き面の支持は、**面のすぐ下**（`bed_eps`）の1点標本が
  材料内かで判定する（問303）。スライサ文献の定義「**下方から支持されていない**部分」に
  対応する。SDF を渡さない `validate`（mesh-only）では判定不可のため、rim 等が
  オーバーハング候補として残る（既知の制限）。標本は1点なので、面のすぐ下だけを見る——
  斜めに逃げる支持柱など、真下にない支持は考慮しない（安全側＝過検出寄り）。
- **陰影の物理的正しさ（ガンマ空間シェーディング）**: `render` は Lambertian + ambient の
  輝度係数を **sRGB 値へ直接乗じる**（`diffuse * intensity`）。光の伝播はリニア空間の
  演算なので、これは物理的には誤りで、正しくは sRGB→リニア→乗算→sRGB の順を踏む。
  実測差（diffuse=220）: intensity 0.25 で現在 55 に対し正しくは約 117（全域の 24%）、
  0.60 で 132 対 174。**陰の面が本来より暗く沈み階調が失われる**。
  修正すると全 screenshot の出力バイトが変わる一方、VLM の形状認識が実際に向上するかは
  未計測であり、物理的正しさと視覚的有用性は自動的には一致しない。よって**意図的に
  据え置き**、テスト `shading_is_computed_in_gamma_space_by_deliberate_choice` で
  現行挙動を契約として固定している（問307）。出力を変える判断は利用者に委ねる。
- **`volume` / `surface_area` の厳密性**: いずれも**抽出メッシュ由来の離散近似**であり、
  内接多面体近似のため**常にわずかに過小評価**する（安全側＝材料を過大に見積もらない）。
  既定解像度 48 での実測誤差（問306 で測定・回帰テストで固定）:
  体積は sphere -0.119% / cylinder -0.121% / cuboid -0.051%（材料費見積もりに十分）。
  表面積は sphere -0.061% / cylinder -0.831% / cuboid -1.024% と**収束が遅く**、
  箱状形状では解像度 128 でも -0.44% 程度で頭打ちになる（内接多面体近似の系統的性質）。
  体積は解像度 16→128 で -1.07%→-0.02% と単調に収束する。厳密値が必要な場合は
  `resolution` を上げる。SDF に対する解析的な体積計算は非目標。
- **薄肉の完全検出**: `min_wall_probe` は **sphere tracing**（Hart 1996・問300）で前進するため
  **どれほど薄い壁も跨いで見落とすことはない**（`|d|` が真の距離の下界なので表面を跨がない）。
  残る限界は2点: (a) 探針は抽出メッシュの頂点から出るため、**抽出セルより薄い壁**では
  メッシュ自身が表面を再現しきれず値に誤差が乗る（検出はされるが公称厚とずれる）、
  (b) 探針数の上限（30,000 頂点）で間引くため、間引かれた頂点にのみ現れる薄肉は逃しうる。
  測定の定義は **ray 法**（法線方向の対面までの距離）であり、rolling ball 法（内接最大球）とは
  鋭角コーナー付近で値が異なる。
- **楕円体の軸外厳密距離**: IQ 近似（符号と軸上距離のみ厳密）。
- **メッシュ CSG**: 採用しない（数値破綻回避のため SDF 経由）。
- **STL インポートの SDF 化**: `validate-stl`（CLI）は外部 binary STL を**検証専用**で
  読み込むが、mesh→SDF 再構成はしない。インポートしたメッシュは SDF シーン正本に
  ならない（ADR-001「SDF が唯一の正本」を保存・問296）。MCP には出さない。
- **`measure` の完全性**: 反復上限（10,000 ステップ）または交差数上限（64）に達した光線は
  以降を走査しない。薄い特徴の見落としは sphere tracing により**構造的に起きない**
  （`|d|` は真の距離の下界なので表面を跨がない・Hart 1996）が、上限打ち切りは起こりうる——
  特に**面に沿って滑る光線**は `|d|=0` で歩幅が最小値に張り付き完走できない（この
  収束の遅さは Keinert et al., *Enhanced Sphere Tracing*, 2014 が扱った既知の性質）。
  ただし打ち切りは**サイレントではない**: 応答の `complete=false` と `warning` で明示され、
  AI は「交差なし」と「未走査」を区別できる（問301）。
- **無限反復**: `repeat` は count による有限反復のみ。
- **面取り/平滑ブーリアンの内部厳密距離**: `chamfer_*`/`smooth_*` は**符号・表面（ゼロ等位面）は厳密**だが、内部の距離場は真のメトリックからずれる（面取り平面/ブレンド項が深部で min/max に勝ち続けるため）。抽出は符号のみ使用するため水密性に影響しない。`offset`/`shell` を面取り・平滑結果に適用する場合は近似（問285）。

---

## 10. 品質ゲート

- `cargo test --all-targets`: 全テスト合格（ライブラリユニット + CLI ユニット + CLI/MCP の実バイナリ E2E）。
  本数は `cargo test` が常に表示するので、ここでは重複して持たない（問324: 本セッション中に
  手で同期した数値が繰り返しずれた）。
- `cargo clippy --all-targets -- -D warnings`: 警告ゼロ。
- `cargo fmt --all -- --check`: rustfmt 標準に一致（v151/問295 で全ツリー正規化）。
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: rustdoc 警告ゼロ（問275）。
- `cargo build --release`: リリースプロファイル（LTO・単一 codegen-unit・panic=abort）成功。
- ソクラテス問答（`docs/socratic-review.md`）: 問1–331 を継続的に吟味・固定。

**KPI 実測値**（問309・Plan.md §7 と突き合わせ）:

| KPI | 目標 | 実測 | |
|---|---|---|---|
| 単一バイナリ起動 | ≤100ms | **4ms** | ✅ |
| screenshot | ≤2秒 | **84ms** | ✅ |
| テスト数 | 300+ | **大幅超過**（正確な本数は `cargo test --all-targets` が表示） | ✅ |
| 静的解析警告 | 0 | **0** | ✅ |
| PII 収集 | ゼロ | ゼロ（ネットワーク I/O なし） | ✅ |
| ツール呼出/タスク | ≤15 | **3〜4**（固定の道具列。実測平均ではなく**予算内であることの確認**） | ✅ |
| ツールチェーン完走率 | ≥80% | **100%（17/17）** — **スクリプトを与えられた状態から**実 MCP バイナリで完走（問311/331） | ✅ |
| 著述ミスからの回復可能率 | 100% | **100%（12/12）** — 現実的な誤り全件が名指し＋選択肢つきのエラーを返す（問331） | ✅ |

**この KPI が測っていないもの（問331）:** 上記の完走率は
`eval_set()` に**人間が書いて置いた**スクリプトを逐語で流している。答えるのは
「正しいスクリプトを与えられた状態からツール層が完走できるか」であって、
**「AI が意図からスクリプトを書けるか」ではない**——後者こそ無人運用の難所である。
著述の測定にはライブのモデルが要り、外部依存ゼロ・オフライン・決定的という
本プロジェクトの制約と両立しない。**測れないものを測れると書かない**のが正しい扱いで、
問331 以前は「無人完走率」という名前がその区別を消していた。
著述の失敗から**回復**できるかは代わりに
`recoverable_error_rate_over_realistic_authoring_mistakes` が測る（12 種の現実的な
誤り——op 名のタイポ・引数の数違い・キー名の誤り・型違い・範囲外・未対応形式など——が
すべて `isError` かつ原因の名指しと有効な選択肢／`help` への誘導を含むこと）。

旗艦 DoD「M3穴付きブラケットを自然言語→検証済み STL まで無人完走」は
`tests/mcp_workflow.rs::flagship_dod_m3_bracket_completes_within_the_tool_call_budget`
として**実行可能なテスト**になっている（run_script→measure→validate→export の
4 呼出で完走し、穴径 Ø3.2 を実測確認、出力 STL を binary STL としてデコードして
水密性まで検証する）。ただしここでもスクリプトはテストが与えており、
**「自然言語→」の部分は測定範囲外**である（同上）。

CI 定義は `docs/ci.yml`（ubuntu/macos/windows マトリクス + `no-external-deps` 検証）。
Claude Code の GitHub App は `workflows` 権限を持たないため、リポジトリ管理者が
`git mv docs/ci.yml .github/workflows/ci.yml` で有効化する。

### 出力フォーマットの外部相互運用性（リリース前検証・問289/問324）

内部テスト（`io/stl`・`io/zip`・`io/threemf`・`io/gltf`）に加え、書き出した
STL/3MF/GLB/HTML が **Kado 以外の標準ツール** で開けることを確認する。

```sh
python3 scripts/interop-check.py
```

問324 以前この節は**散文としてしか存在せず**、実行可能なものがリポジトリに無かった
（`find . -name '*.py'` が 0 件）。手順が書いてあることと、それが実行されたことは別である。
`scripts/check.sh` には組み込まない（Python 依存のため CI には含めない・従来の判断を維持）。

要点は Kado の実装を**一切参照せず独立にデコードする**こと。Kado のバグを Kado の
コードで見逃さないため、Python 標準ライブラリだけで復元する:

- **STL**: `struct` で `84 + n*50` レイアウトを復元・全レコードの属性=0・
  ファイル長が三角形数と厳密一致・全 float が有限。
- **3MF**: `zipfile.testzip()` が全エントリの CRC-32 を独自実装で再計算して検証し、
  `xml.etree` が `3D/3dmodel.model` を整形式 XML としてパース・`unit="millimeter"`・
  頂点/三角形数・三角形の頂点参照が範囲内であることを確認。
- **GLB**: `glTF` マジック・version 2・総長一致・先頭 JSON チャンクのパース・
  `asset.version == "2.0"`・`NORMAL` アクセサが単位長（問290）を確認。
- **HTML**: 外部リソース参照が無いこと。§10 は従来 HTML に触れていなかったが、
  **C1「外部送信ゼロ」は出力側でも担保が要る**（問324）——書き出した HTML が
  外部 URL を参照していれば、開いた時点でネットワークへ出る。ソースにソケットが
  無いこと（問316）だけでは、生成物経由の流出を防げない。

**実測（問324・v0.2.0）**: 4 形式すべて PASS。GLB 法線の単位長からの最大偏差は
`4.50e-08`。各形式を1バイト改変すると対応する検査が実際に FAIL することも確認済み
（通るだけの検査は、検査が機能している証拠にならない）。

---

> 本書は実装の進展（問答ループ v84–v94+）に追従して更新する。
> 各契約の根拠は `docs/socratic-review.md` の対応する問番号を参照。
