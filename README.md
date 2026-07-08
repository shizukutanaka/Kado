# Kado

**AI-First のローカル幾何モデリングエンジン (SDF カーネル)。**

Kado は、AI エージェントが 3D 形状を**生成・検証・製造用出力**するために呼び出す、
ローカル完結の幾何エンジンです。SDF（符号付き距離場）木を正本とし、スクリプトから
決定的にメッシュを抽出・DFM 検証・各種フォーマット出力します。

## 設計の柱

| 柱 | 意味 |
|----|------|
| 🔒 **外部送信ゼロ** | ネットワーク I/O を持たない。MCP は stdio のみ。 |
| 📦 **単一自己完結バイナリ** | コアは Rust std のみ（外部 crate ゼロ・ADR-003）。 |
| 🎯 **決定的出力** | 同一バイナリ・同一 arch でバイト同一。`digest()` で観測可能。 |
| 🖥️ **全機能ヘッドレス** | CLI / MCP（stdio）で完結。GUI 非搭載。 |

座標の **1 単位 = 1 mm**。DFM 閾値・3MF 宣言単位はすべて mm で一貫します。

## クイックスタート

```sh
# ビルド (外部依存ゼロ・std のみ)
cargo build --release

# 動作確認
./target/release/kado selftest
# → selftest ok: f(origin) = -1

# デモモデルを STL 出力
./target/release/kado export demo.stl

# スクリプトからメッシュ統計を表示
echo '{"op":"sphere","r":1.0}' > scene.json
./target/release/kado run scene.json

# 製造性 (DFM) 検証: 最小肉厚 0.8mm・最大オーバーハング 45°
./target/release/kado check scene.json 0.8 45
```

### CLI コマンド

| コマンド | 機能 |
|----------|------|
| `version` | バージョン表示 |
| `selftest` | 最小 SDF 評価の動作確認 |
| `export [scene.json] <out.stl\|.glb\|.3mf\|.html>` | メッシュ出力（拡張子で形式選択） |
| `screenshot [scene.json] <out.png> [view]` | PNG スクリーンショット |
| `run <scene.json> [resolution]` | メッシュ統計表示 |
| `check <scene.json> [min_wall_mm] [max_overhang_deg] [resolution]` | DFM 検証 |
| `mcp` | MCP サーバ（stdio）起動 |
| `help` / `--help` / `-h` | 使い方一覧を表示 |

## MCP サーバ（AI エージェント向け）

Kado は MCP（Model Context Protocol, JSON-RPC 2.0）サーバとして AI に道具を提供します。

```sh
./target/release/kado mcp   # stdin/stdout で JSON-RPC を待ち受け
```

| ツール | 機能 |
|--------|------|
| `run_script` | DSL/JSON スクリプトを評価しシーン正本を更新 |
| `eval` | 1 点の符号付き距離を返す |
| `validate` | DFM 検証レポート（肉厚・オーバーハング・多様体・体積） |
| `screenshot` | シーンを PNG レンダリング（base64・7 視点） |
| `export` | STL/GLB/3MF/HTML をプロジェクト直下へ書き出し |
| `get_scene` | 現在のシーン正本（スクリプト）を返す |
| `undo_script` | 1 段階 undo |
| `help` | DSL/ツールのリファレンス |

書き込みは**プロジェクト直下のみ**に制限（パストラバーサル・絶対パス拒否）。

## 形状スクリプト（DSL）

JSON 形式とテキスト DSL の両方を受け付けます（自動判別）。

```jsonc
// 球と直方体を smooth_union でブレンドし、円柱で穴を開ける
{"op":"difference",
 "a":{"op":"smooth_union","k":0.2,
      "a":{"op":"sphere","r":1.0},
      "b":{"op":"cuboid","x":0.8,"y":0.8,"z":0.8}},
 "b":{"op":"cylinder","r":0.3,"h":2.0}}
```

同じ形状をテキスト DSL で:

```
difference(smooth_union(sphere(1.0), cuboid(0.8, 0.8, 0.8), 0.2), cylinder(0.3, 2.0))
```

### 利用可能な演算子

- **プリミティブ**: `sphere` `cuboid` `cylinder` `torus` `cone` `capsule` `rounded_box` `ellipsoid` `prism`
- **CSG**: `union` `intersection` `difference` `smooth_union` `smooth_intersection` `smooth_difference`
- **変換**: `translate` `scale` `scale_xyz` `offset` `shell` `repeat` `mirror_x/y/z` `rotate_x/y/z` `rotate` `cut` `flatten`

`cut` は平面で形状を切る半空間カット、`flatten` はその最頻用途（FDM 印刷の**平坦な底面**づくり）の意図明示型ショートカットです:

```jsonc
// 球の下半分を削り z=0 に平らな底を作る (印刷可能なドーム)。これが推奨:
{"op":"flatten","at":0,"shape":{"op":"sphere","r":1.0}}

// 汎用カット (任意平面)。flatten と等価な書き方:
{"op":"cut","nx":0,"ny":0,"nz":-1,"offset":0,"shape":{"op":"sphere","r":1.0}}
```

`cut` は `dot(p,(nx,ny,nz)) <= offset` の側を残します（法線が指す側を切り落とす）。
`flatten(at)` は `z >= at` を残す安全な別名で、法線方向の取り違えを防ぎます。

## アーキテクチャ

```
スクリプト(DSL/JSON) ──parse──▶ SDF木(core::Sdf) ──polygonize──▶ Mesh
   (正本)                       (決定的射影)        (Marching Tetrahedra)
                                    │                    │
                                    │                    ├─▶ verify (DFM検証)
                                    │                    └─▶ io (STL/GLB/3MF/HTML)
                                    └─▶ render (ラスタライザ→PNG)
```

| モジュール | 責務 |
|-----------|------|
| `core` | Vec3 演算 / SDF 木 / eval / aabb / sampling_box |
| `extract` | Marching Tetrahedra / Mesh / 多様体検査 / 体積 |
| `render` | ラスタライザ / Image / PNG エンコード |
| `io` | STL / glTF(GLB) / 3MF / HTML / ZIP |
| `script` | DSL パーサ / JSON 評価器 / リソース予算 |
| `verify` | DFM 検証（肉厚・オーバーハング・多様体・体積） |
| `mcp` | JSON-RPC 2.0 サーバ / ツール / セッション |

詳細な契約・不変条件・非目標は [`docs/SPEC.md`](docs/SPEC.md) を参照。

## 決定性の範囲

「同一リリースバイナリ・同一 OS/arch」内で**バイト同一**を保証します。
クロスプラットフォーム間は**数値的に同等だがビット同一は非保証**（libm 差・FP 縮約順序差）。
`Mesh::digest()`（FNV-1a 64bit）で再現性を短いハッシュ 1 つで検証できます。

## 開発

```sh
cargo test                              # 全テスト (288 ユニット + 6 統合)
cargo clippy --all-targets -- -D warnings   # Lint (警告ゼロ)
cargo fmt                               # 整形
```

品質ゲート: 全テスト合格・clippy 警告ゼロ。設計判断は [`docs/adr/`](docs/adr/)、
継続的なソクラテス問答による吟味議事録は [`docs/socratic-review.md`](docs/socratic-review.md) に記録。

## ライセンス

MIT
