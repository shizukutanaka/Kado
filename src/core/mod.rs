//! SDFカーネル中核 (Phase 1)。
//!
//! - [`math`]: 決定的な f64 ベクトル演算。
//! - [`sdf`]: SDF木 ([`Sdf`]) とその解析的評価。プリミティブ・ブーリアン・
//!   smooth blend・変換を含む。
//! - [`measure`]: 光線に沿った表面交差の測定 (sphere tracing・問299)。
//!   穴径・肉厚・面間距離を1回の問い合わせで返す。
//!
//! 決定性方針 (ADR-003 / 問5): すべて f64・固定評価順序。fast-math 相当は
//! 用いない。同一バイナリ・同一arch内でバイト同一を保証する。

pub mod math;
pub mod measure;
pub mod sdf;

pub use math::Vec3;
pub use measure::{ray_crossings, spans, Crossing};
pub use sdf::Sdf;
