//! スクリプト評価器 (Phase 2)。
//!
//! 正本 (source of truth) はスクリプトであり、SDF 木はその決定的射影 (問2)。
//!
//! 2 つの表層構文を提供する (どちらも同一の KadoScene [`Value`] 木へ落ち、
//! 意味論・検証・上限は共有される):
//! - **KadoScene JSON** ([`eval_scene`]) — 宣言的 JSON ツリー。
//! - **テキスト DSL** ([`eval_dsl`]) — 簡潔な関数呼び出し式 (問59)。
//! [`eval_any`] は先頭文字で両者を自動判別する。
//!
//! セキュリティ (Plan リスク E): import 不可, 任意コード実行不可。

pub mod dsl;
pub mod eval;

pub use dsl::{eval_dsl, parse_dsl};
pub use eval::{eval_scene, eval_value, ScriptError};

/// スクリプトを自動判別して評価する。先頭の非空白が `{` なら JSON、
/// それ以外はテキスト DSL とみなす (問59)。
pub fn eval_any(source: &str) -> Result<crate::core::Sdf, ScriptError> {
    if source.trim_start().starts_with('{') {
        eval_scene(source)
    } else {
        eval_dsl(source)
    }
}
