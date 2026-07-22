# コントリビューションガイド

Kado は明示的な設計制約の上に成り立っています。以下は「知らないと踏む」非自明な
ルールです。コードを書く前に目を通してください。

## 鉄則

### 1. 外部 crate を追加しない（ADR-003 / 問4）
コア（`core` `extract` `render` `io` `script` `verify` `mcp`）は **Rust std のみ** で
実装します。`Cargo.toml` の `[dependencies]` は空であり、CI の `no-external-deps`
ジョブが `cargo tree` で依存ツリーの空を強制します。新しい crate が必要に見えたら、
まず std で書けないか、あるいは設計を見直せないかを検討してください。
追加が本当に必要な場合は SPEC での個別承認が要ります。

### 2. 決定性を壊さない（問5）
同一バイナリ・同一 arch でバイト同一の出力を保証します:
- すべて `f64`。`mul_add`（FMA）は使わず素朴な `a*b + c`（プラットフォーム差回避）。
- 演算順序を固定する。`HashMap` の反復順序に出力を依存させない
  （`body_components` 等はカウントのみ使い順序非依存）。
- 超越関数は std libm に固定。クロスプラットフォームは数値同等だがビット同一は
  非保証（同一 arch 内のみ保証）。
- エンコーダ（STL/GLB/PNG/3MF/HTML）は同一メッシュからバイト同一であること。

### 3. バグ修正は必ずテストで固定する
本プロジェクトはソクラテス問答（[`docs/socratic-review.md`](docs/socratic-review.md)）
による継続的な不変条件の発見・固定で成り立っています。バグを直したら、その
バグを再現する回帰テストを必ず追加してください。「直した」ではなく「二度と
起きないことを観測可能にした」が完了の定義です。

### 4. 幾何的に無効な入力は評価前に拒否する
`r<=0` / `scale<=0` / `shell<=0` / `torus minor>=major` などは、無音の空メッシュや
非多様体を生む前に `script::eval` 層で明示エラーにします。サイレント故障を作らない。

### 5. 信頼できない入力としてリソース上限を守る
新しい再帰・確保・ループを追加するときは、既存の上限
（[`SECURITY.md`](SECURITY.md) §4）に従い、病的入力でクラッシュしないことを確認。

## 品質ゲート

PR を出す前に、ローカルで以下がすべて通ること:

```sh
cargo test --all-targets                        # 全テスト合格
cargo clippy --all-targets -- -D warnings       # 警告ゼロ
cargo fmt --all -- --check                       # rustfmt 標準に一致
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps  # rustdoc 警告ゼロ
cargo build --release                           # リリースビルド成功
```

CI（[`docs/ci.yml`](docs/ci.yml)、要 `.github/workflows/` への配置）が
ubuntu/macos/windows で同じゲートを実行します。

### `cargo fmt` について
かつては意図的な手整形を採用し `cargo fmt` を強制していませんでしたが、
**v151（問295）で全ツリーを rustfmt 標準へ正規化し、方針を転換しました**。
現在ツリー全体が `cargo fmt --all -- --check` を差分ゼロで通ります。以後は
これを品質ゲートの一部として維持してください（CI でも強制）。経緯と判断理由は
[`docs/socratic-review.md`](docs/socratic-review.md) 問295 を参照。

## アーキテクチャと契約

- 全体像とモジュール責務: [`README.md`](README.md)
- 契約・不変条件・非目標: [`docs/SPEC.md`](docs/SPEC.md)
- 設計判断の記録: [`docs/adr/`](docs/adr/)
- 吟味議事録（問1〜）: [`docs/socratic-review.md`](docs/socratic-review.md)

## コミットメッセージ

変更の「何を」だけでなく「なぜ」を書いてください。不変条件を固定するコミットは、
対応する問番号（例: 問232）を参照すると経緯が追えます。
