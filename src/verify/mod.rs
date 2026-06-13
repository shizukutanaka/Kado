//! 検証API (Phase 3)。
//!
//! メッシュ・SDF両面から製造可能性 (DFM) を検査する。
//! 全結果は構造化エラー [`KadoError`] で返す (Plan §3)。

pub mod check;

pub use check::{validate, KadoError, Report, Severity};
