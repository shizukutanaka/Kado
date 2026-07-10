//! ソフトウェアラスタライザ (Phase 0.5 / Phase 4)。
//!
//! 外部依存ゼロで PNG 生成まで完結する (問4 / ADR-003)。
//! - [`image`]: RGB 画像バッファ + 決定的 PNG エンコーダ (RLE限定 deflate・問281)。
//! - [`raster`]: z バッファ・フラットシェーディング・パースペクティブ投影。
//! - `deflate` (非公開): 決定的 DEFLATE 圧縮 (RFC 1951 固定 Huffman・RLE限定・問281)。

mod deflate;
pub mod image;
pub mod raster;

pub use image::Image;
pub use raster::{draw_axes, render, Camera};
