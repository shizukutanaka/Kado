//! スクリプト評価器 (Phase 2)。
//!
//! 正本 (source of truth) はスクリプトであり、SDF 木はその決定的射影 (問2)。
//!
//! 2 つの表層構文を提供する (どちらも同一の KadoScene [`Value`](crate::mcp::json::Value) 木へ落ち、
//! 意味論・検証・上限は共有される):
//! - **KadoScene JSON** ([`eval_scene`]) — 宣言的 JSON ツリー。
//! - **テキスト DSL** ([`eval_dsl`]) — 簡潔な関数呼び出し式 (問59)。
//!
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Vec3;

    /// 同じシーンを JSON と DSL で書いたとき、eval_any 経由で同一の場になることを確認。
    fn assert_eval_any_agrees(json: &str, dsl: &str) {
        let a = eval_any(json).unwrap_or_else(|e| panic!("eval_any(JSON) failed: {e}"));
        let b = eval_any(dsl).unwrap_or_else(|e| panic!("eval_any(DSL) failed: {e}"));
        for p in [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.5, 0.3, -0.2),
            Vec3::new(1.2, -0.7, 0.9),
        ] {
            assert!(
                (a.eval(p) - b.eval(p)).abs() < 1e-12,
                "eval_any dispatch mismatch at {p:?}: JSON={} DSL={}",
                a.eval(p),
                b.eval(p)
            );
        }
    }

    #[test]
    fn dispatches_json_and_dsl_to_same_field() {
        // 問122: eval_any は MCP run_script・CLI の公開入口。先頭文字による
        // JSON/DSL 振り分けが両表現を同じ場へ落とすことを公開 API レベルで固定する。
        assert_eval_any_agrees(r#"{"op":"sphere","r":1.5}"#, "sphere(1.5)");
        assert_eval_any_agrees(
            r#"{"op":"difference","a":{"op":"sphere","r":1.0},"b":{"op":"cylinder","r":0.3,"h":2.0}}"#,
            "difference(sphere(1.0), cylinder(0.3, 2.0))",
        );
    }

    #[test]
    fn leading_whitespace_before_brace_is_still_json() {
        // trim_start を使うため、JSON の前の空白・改行があっても JSON と判別される。
        // もし trim を忘れると "  {...}" が DSL 経路へ行き識別子パースで失敗する。
        let sphere =
            eval_any("  \n\t{\"op\":\"sphere\",\"r\":2.0}").expect("leading-ws JSON must parse");
        // 半径2の球: 原点で -2。
        assert!((sphere.eval(Vec3::ZERO) - (-2.0)).abs() < 1e-12);
    }

    #[test]
    fn identifier_start_routes_to_dsl() {
        // 識別子始まり → DSL 経路。JSON パーサに渡すと即座に失敗するため、
        // 正しく DSL として評価できること自体が振り分けの証拠。
        let s = eval_any("cuboid(0.5)").expect("DSL must parse via eval_any");
        // 半幅0.5の立方体: 原点で -0.5。
        assert!((s.eval(Vec3::ZERO) - (-0.5)).abs() < 1e-12);
    }

    #[test]
    fn malformed_input_returns_error_not_panic() {
        // どちらの経路でも不正入力は Err を返しパニックしない (公開入口の堅牢性)。
        assert!(
            eval_any("{not valid json").is_err(),
            "broken JSON must error"
        );
        assert!(
            eval_any("nonexistent_fn(1)").is_err(),
            "unknown DSL fn must error"
        );
        assert!(eval_any("").is_err(), "empty input must error, not panic");
    }
}
