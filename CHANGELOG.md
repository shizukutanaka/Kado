# Changelog

このプロジェクトの注目すべき変更をすべて記録する。
形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に、
バージョニングは [Semantic Versioning](https://semver.org/lang/ja/) に従う。

## [Unreleased]

### 追加

- 非一様スケール `scale_xyz(sx,sy,sz)`: 符号厳密・保守的過小評価・
  Lipschitz=1 を証明済みの安全な近似 (問276)
- PNG screenshot 応答の実圧縮: 決定的 DEFLATE (RFC 1951, 固定 Huffman・
  候補距離 {1,3} 限定 RLE) を std-only で新規実装。無圧縮 stored との
  小さい方を採用するため退行の余地なし。実測で raw 比 8.3% まで縮小 (問281)

### 修正

- `docs/SPEC.md` の対象バージョンヘッダが 0.0.1 のまま更新漏れしていたのを
  0.1.0 へ修正 (問277)
- `README.md` の演算子一覧から `prism`・任意軸 `rotate`・`scale_xyz` が
  漏れていたのを修正し、README を含む3箇所の演算子一覧の文書完全性テストを
  新設 (問278)
- CHANGELOG.md 自身が、まだ作成されていない git tag / GitHub Release への
  リンクを先回りして記載していたのを削除 (問280)
- `Plan.md` が「存在する」前提で書いていた `ADR-001`（カーネル選定）・
  `ADR-002`（GUI非搭載）が実際には一度も作成されていなかったのを
  `docs/adr/` に追加。`src/extract/mod.rs` の「Marching Tetrahedra は
  暫定実装・dual contouring に置換予定」というコメントも、実際には
  MT のまま全リリースを通過し置換の必要性が生じていない実態に更新 (問282)

## [0.1.0] - 2026-07-06

初の対外品質リリース。AI エージェントが 3D 形状を生成・検証・製造出力するための
ローカル完結幾何エンジン。

### 幾何カーネル (SDF)

- プリミティブ 9 種: `sphere` / `cuboid` / `cylinder` / `torus` / `cone` /
  `capsule` / `rounded_box` / `ellipsoid` (IQ 近似・符号厳密) /
  `prism` (正多角形・厳密 SDF)
- CSG 6 種: `union` / `intersection` / `difference` + smooth 版 3 種
- 変形 12 種: `translate` / `scale` / `offset` / `shell` / `repeat` (有限反復) /
  `mirror_x/y/z` / `rotate_x/y/z` / `rotate` (任意軸・Rodrigues) /
  `cut` (平面) / `flatten` (FDM 平坦底面)
- 決定的メッシュ抽出 (Marching Tetrahedra): 同一バイナリ・同一 arch で
  バイト同一、`digest()` (FNV-1a) で再現性を観測可能

### 製造性検証 (DFM)

- 11 種の issue コード (`EMPTY_MESH` 〜 `ENCLOSED_CAVITY`)。
  空間的な issue は全て問題箇所の 3D 座標 (`location`) 付き
- 実測値レポート: 体積 (mm³)・表面積 (mm²)・実測最小肉厚・重心・
  ボディ/空洞数・ベッド接地面積
- SDF 内向きレイ探針による局所薄肉検出、ビルド方向対応オーバーハング解析

### 入出力

- 形状スクリプト: JSON とテキスト DSL の 2 表層構文 (自動判別・意味論は単一)
- エクスポート 4 形式: binary STL / GLB (glTF 2.0) / 3MF (mm 単位宣言) /
  自己完結 HTML ビューア — すべて決定的エンコード
- PNG スクリーンショット (7 視点プリセット・SSAA・透視/正射影・座標軸表示)

### インターフェース

- CLI: `version` / `selftest` / `export` / `screenshot` / `run` / `check` /
  `mcp` / `help`
- MCP サーバ (JSON-RPC 2.0, stdio, プロトコル版交渉 2025-06-18/2024-11-05):
  `run_script` / `eval` / `validate` / `screenshot` / `export` / `get_scene` /
  `undo_script` / `help` の 8 ツール (tool annotations 付き)

### セキュリティ・堅牢性

- 外部送信ゼロ (ネットワーク I/O なし)・外部 crate ゼロ (Rust std のみ)
- 書き込みサンドボックス (プロジェクト直下のみ・パストラバーサル拒否)
- DoS 上限: ソース 1MiB・50,000 ノード・深さ 64・メッセージ 16MiB・
  解像度 256・画像 4096px
- 不正な MCP フレーム本文 1 通でセッションが落ちない (Parse error で継続)
- テスト 371 本 (単体 + 統合 eval-set/敵対的ストレス) 全通過・
  clippy / rustfmt / rustdoc 警告ゼロ

<!-- 問280: バージョン比較/リリースページへのリンクは、実際に git tag と
     GitHub Release が作成されてから追加する。存在しないタグ/リリースへの
     リンクを先に書くと 404 になるため、ここでは意図的に省略する。 -->
