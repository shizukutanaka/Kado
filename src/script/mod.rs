//! スクリプト評価器 (Phase 2)。
//!
//! 正本 (source of truth) はスクリプトであり、SDF 木はその決定的射影 (問2)。
//!
//! 現時点の実装は **KadoScene JSON 形式** — 宣言的な JSON ツリーで SDF 木を記述する。
//! テキスト DSL (Phase 2 最終目標) は BACKLOG に記録し、まず JSON 形式で
//! AIエージェントが扱いやすいスクリプト体制を確立する。
//!
//! セキュリティ (Plan リスク E): import 不可, 任意コード実行不可 (JSON 宣言のみ)。

pub mod eval;

pub use eval::{eval_scene, ScriptError};
