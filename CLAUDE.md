# CLAUDE.md — Kado で作業する Claude への共通ガードレール

このファイルは Claude Code が全セッションで自動読込する。Kado は明示的な設計制約と
ソクラテス問答（問1〜）の上に成り立つ。**セッションを跨ぐと固有規律が失われる**ため、
作業前に本ファイルと下記を必ず読むこと。

## 0. 作業開始前の必読（順に）

1. [`CONTRIBUTING.md`](CONTRIBUTING.md) — 鉄則（外部 crate 禁止・決定性・事前拒否）
2. [`docs/SPEC.md`](docs/SPEC.md) — 契約・不変条件・非目標・品質ゲート
3. [`docs/feature-triage.md`](docs/feature-triage.md) — 何を作らないか（過剰機能の判定）
4. [`docs/socratic-review.md`](docs/socratic-review.md) — 全設計判断の経緯（問1〜）

タスクの性質で読み分ける:
- **設計判断を伴う**（新機能・研究調査・ADR 起票）→ [`docs/agents/opus-instructions.md`](docs/agents/opus-instructions.md)
- **仕様が明確**（テスト追加・docs 同期・既知バグ修正）→ [`docs/agents/sonnet-instructions.md`](docs/agents/sonnet-instructions.md)

## 1. 鉄則（破ると設計が壊れる）

- **外部 crate 禁止**（ADR-003 / 問4）。コアは std のみ。`Cargo.toml` の
  `[dependencies]` は空。DEFLATE/PNG/glTF/3MF/JSON はすべて自前実装。
- **決定性を壊さない**（問5）。全 `f64`・`mul_add`(FMA) 不使用・演算順序固定・
  `HashMap` 反復順に出力を依存させない・超越関数は std libm 固定。エンコーダは
  同一メッシュからバイト同一。
- **幾何無効入力は評価前に拒否**（問題を無音の空メッシュにしない）。`r<=0` 等は
  `script::eval` 層で明示エラー。
- **恣意的閾値を持ち込まない**。DFM 等の正しさに関わる数値は根拠を要する
  （表示上の既定値＝材質色などは別。問291 参照）。
- **リソース上限を守る**（[`SECURITY.md`](SECURITY.md) §4）。新しい再帰/確保/ループは
  病的入力でクラッシュしないこと。

## 2. 「表面積の不整合」チェックリスト（問292/294 の教訓）

同じ概念が複数箇所に並ぶ。**片方だけに足すとバグる**。追加時は全点セットで更新し、
メタテストが落ちたら不足箇所を教えてくれる。

- **DSL 演算子を追加** → 5点セット:
  `src/core/sdf.rs`（enum+eval+aabb+constructor）/ `src/script/eval.rs`（JSON 評価+`ALL_DSL_OPS`）/
  `src/script/dsl.rs`（テキスト DSL の `build_call` ディスパッチ）/ `KADOSCENE_HELP`（`tools.rs`）/ `README.md` の演算子一覧。
  ガード: `every_documented_op_is_dispatchable_in_text_dsl`・`all_ops_parse_identically_in_dsl_and_json`・`dsl_ops_are_fully_documented`。
- **MCP ツールを追加** → 3点セット:
  `tool_list`（スキーマ）/ `call_tool`（ディスパッチ）/ `tool_annotations`（安全ヒント、`tools.rs`）。
  ガード: `every_declared_tool_is_dispatched_and_explicitly_annotated`。
- **DFM issue コードを追加** → `ALL_ISSUE_CODES`（`check.rs`）+ `KADOSCENE_HELP` + そのコードを発火させる回帰テスト。
- **新規ファイルを作る前に必ず既存を検索**（問295 の教訓: 既存 `docs/ci.yml` を
  見落として重複を作った）。`Glob`/`Grep` で同目的のファイルがないか確認する。

## 3. 完了の定義

「テスト緑」では完了ではない。**ユーザーが実際に叩く経路で E2E 検証する**（問292/293）:
- MCP 機能 → 実 `kado mcp` バイナリを stdio で叩く（`tests/mcp_workflow.rs` が雛形）。
- 出力形式 → Kado 以外の標準ツールで開通確認（`docs/SPEC.md` §10）。
- バグ修正 → **そのバグを再現する回帰テストを必ず追加**（CONTRIBUTING §3）。

## 4. ラウンドの型

1. `docs/socratic-review.md` の最新問番号の続き（例: 問N）を採番。
2. 実装 + テスト（決定性・回帰ガードを含む）。
3. 品質ゲートを全通過させる（§5）。
4. `docs/socratic-review.md` に問N エントリ（問い→設計判断→実装→検証、出典 URL）。
5. `CHANGELOG.md`（Unreleased）・必要なら `docs/feature-triage.md` を更新。
6. commit（メッセージに「なぜ」と問N を書く）→ push。ブランチは指定に従う。

## 5. 品質ゲート（push 前に全通過）

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check           # v151/問295 で rustfmt 標準に統一
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build --release
```

## 6. 禁止事項

- `.github/workflows/` への push（GitHub App に `workflows` 権限なし。CI は
  `docs/ci.yml` に置き、管理者が `git mv` で有効化する）。
- fmt 方針の蒸し返し（問295 で rustfmt 標準に決着済み）。
- 4巨大ファイル（`core/sdf.rs`・`verify/check.rs`・`mcp/tools.rs`・`script/eval.rs`）の
  分割提案（テスト最厚・分割は決定性 churn の方が大きく、意図的に据え置き）。
- モデル識別子（`claude-*` 等）をコミット/PR/コード/成果物へ書くこと（チャット限定）。
- feature-triage の判断軸を通さない機能追加。

## 7. backlog

- **STL インポート**: 問296 で**検証専用**として実装済み（`validate-stl` CLI・
  ADR-001 を保存し SDF 正本にはしない・MCP には出さない）。SDF 再構成は引き続き
  非目標。現時点でユーザー方針待ちの未解決候補はなし（`docs/feature-triage.md`）。
