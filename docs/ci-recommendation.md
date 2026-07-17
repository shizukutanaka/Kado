# CI 推奨定義 (問289)

Kado の品質ゲート (`docs/SPEC.md` §品質ゲート: `cargo test` / `cargo clippy
--all-targets -- -D warnings` / rustdoc 無警告 / `cargo fmt --check`) は
これまで手動運用だった。市販水準では push/PR ごとに自動で回すのが望ましい。

Kado は **std のみ・実行時外部依存ゼロ** (ADR-003) なので、CI には追加の
サービスもキャッシュも不要 — 安定版 Rust ツールチェインだけで完結する。

## 適用方法

下記を `.github/workflows/ci.yml` として**リポジトリ所有者がコミット**すること。
（Kado の開発は GitHub App 経由で行われており、App には `workflows` 権限が
無いため、Claude からは `.github/workflows/` を push できない。手動での追加を
お願いします。）

```yaml
# Kado CI — enforces the quality gates documented in docs/SPEC.md.
# Kado is std-only with zero runtime dependencies (ADR-003), so CI needs no
# extra services or caches beyond a stable Rust toolchain.
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

jobs:
  quality-gates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain (pinned to rust-version in Cargo.toml)
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: "1.94"
          components: rustfmt, clippy

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy (warnings are errors)
        run: cargo clippy --all-targets -- -D warnings

      - name: Tests
        run: cargo test --all

      - name: Rustdoc (warnings are errors)
        env:
          RUSTDOCFLAGS: "-D warnings"
        run: cargo doc --no-deps

      - name: Release build (deterministic profile)
        run: cargo build --release
```

## 出力フォーマットの外部相互運用性 (独立検証)

内部テスト (io/stl.rs, io/zip.rs, io/threemf.rs, io/gltf.rs) に加え、
書き出した STL/3MF/GLB が**Kado 以外の標準ツール**で開けることを確認済み
(問289):

- **STL**: Python `struct` で `84 + n*50` のレイアウトを復元、全レコードの
  属性バイト = 0、ファイル長が三角形数と厳密一致。
- **3MF**: Python `zipfile.testzip()` が全エントリの CRC-32 を**独自実装で
  再計算**して検証 (Kado の CRC と一致) し、`xml.etree` が `3D/3dmodel.model`
  を整形式 XML としてパース、`unit="millimeter"` と頂点/三角形数を確認。
- **GLB**: `glTF` マジック・version 2・総長一致・先頭 JSON チャンクの
  パース・`asset.version == "2.0"` を確認。

この相互運用性検証は `cargo test` の外 (Python 依存) で行うため CI には
含めていないが、リリース前の手動確認手順として記録する。
