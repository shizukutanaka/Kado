//! ソフトウェアラスタライザ (Phase 0.5 / Phase 4)。
//!
//! 外部依存ゼロで PNG 生成まで完結する (問4 / ADR-003)。
//! - [`image`]: RGB 画像バッファ + 決定的 PNG エンコーダ (deflate store)。
//! - [`raster`]: z バッファ・フラットシェーディング・パースペクティブ投影。

pub mod image;
pub mod raster;

pub use image::Image;
pub use raster::{render, Camera};
