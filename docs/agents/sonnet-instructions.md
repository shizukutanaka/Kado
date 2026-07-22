# Sonnet 級モデルへの指示書 — 仕様が明確なタスク

前提: [`../../CLAUDE.md`](../../CLAUDE.md) の共通ガードレールを読んでいること。
本書は**判断の余地が小さい、良く仕様化されたタスク**（テスト追加・docs 同期・
既知バグの修正・メタテスト指摘の解消・CHANGELOG 記入など）向けの手順書。
設計判断が要るタスクは着手せず、[`opus-instructions.md`](opus-instructions.md) 側の
判断が要る旨を報告して停止する。

## 手順（この順に厳守）

### 1. 範囲を確認する
- 変更対象ファイルを特定する。**新規ファイルを作る前に `Glob`/`Grep` で同目的の
  既存ファイルがないか必ず検索**（問295: 既存 `docs/ci.yml` を見落として重複作成した）。
- 変更が SPEC / ADR / feature-triage の記述と矛盾しないか確認。矛盾するなら
  **実装せず**、問として `docs/socratic-review.md` に記録して停止・報告（エスカレーション）。

### 2. 「表面積」を漏れなく更新する（CLAUDE.md §2 の再掲）
- **DSL 演算子**: `sdf.rs` + `eval.rs`(+`ALL_DSL_OPS`) + `dsl.rs` + `KADOSCENE_HELP` + `README.md` の5点。
- **MCP ツール**: `tool_list` + `call_tool` + `tool_annotations` の3点。
- **issue コード**: `ALL_ISSUE_CODES` + `KADOSCENE_HELP` + 発火テスト。
- どれか漏れると対応するメタテストが落ちる。落ちたら「足りない箇所を教わった」と考える。

### 3. テストを書く（バグ修正なら回帰テスト必須）
- バグ修正: **そのバグを再現するテスト**を追加し、修正前に落ち・修正後に通ることを確認
  （CONTRIBUTING §3。「直した」ではなく「二度と起きないことを観測可能にした」が完了）。
- 決定性が関わるなら「2回実行バイト一致」を固定。
- 既存テストの命名・スタイル（`#[cfg(test)] mod tests`・日本語コメントで問番号参照）に合わせる。

### 4. 品質ゲートを全通過させる
```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build --release
```
- clippy が出たら**その指摘に沿って直す**（`#[allow]` で握りつぶさない）。
- fmt が差分を出したら `cargo fmt` で整える（v151/問295 で rustfmt 標準に統一済み）。

### 5. 記録する
- `docs/socratic-review.md`: 最新問番号の続きを採番し、問い→実装→検証を簡潔に記載。
- `CHANGELOG.md` の Unreleased（追加/修正）に1項目。問番号を添える。
- 必要なら `docs/feature-triage.md` の個数表（`ALL_DSL_OPS` 等の数）を実態に合わせる。

### 6. コミット
- メッセージは「何を」だけでなく「なぜ」。対応する問番号を参照。
- モデル識別子（`claude-*`）を書かない。指定ブランチへ push。

## エスカレーション基準（実装を止めて報告する）

以下に該当したら勝手に進めず、状況と選択肢を提示して指示を仰ぐ:
- SPEC / ADR / feature-triage / 決定性 / std-only のいずれかと矛盾する変更が必要。
- 恣意的な数値閾値を導入しないと実装できない。
- 外部 crate が必要に見える。
- 変更が「表面積」全点に及ぶ大規模改修になる、または巨大4ファイルの分割を要する。
- ユーザー方針が未確定の項目（STL インポート等）に触れる。
