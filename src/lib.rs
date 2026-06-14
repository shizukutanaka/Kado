//! Kado — AI-First local geometry engine.
//!
//! 設計制約 (Plan.md): 外部送信ゼロ / 単一自己完結バイナリ / 決定的出力 /
//! 全機能ヘッドレス動作。コアは std のみで実装する (ADR-003 / 問4)。
//!
//! 正本 (source of truth) はスクリプト(DSL)であり、SDF木はその構文木の
//! 決定的な射影である (問2)。本クレートは現時点でその射影=評価対象である
//! [`core::Sdf`] 木を提供する。
//!
//! 単位の取り決め (問62): **座標の1単位 = 1ミリメートル**。DFM 閾値 (最小肉厚など)・
//! 3MF 出力の宣言単位はすべて mm で一貫する。検証レポートは寸法を `dims_mm` として明示し、
//! AI/利用者が実寸を把握できるようにする。

pub mod core;
pub mod extract;
pub mod io;
pub mod mcp;
pub mod render;
pub mod script;
pub mod verify;
