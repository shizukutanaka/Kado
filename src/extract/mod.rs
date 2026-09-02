//! メッシュ抽出。
//!
//! SDF木をサンプリングして三角形メッシュを得る。抽出アルゴリズムには
//! **marching tetrahedra** を採用する (ADR-001)。MTは6四面体分割に曖昧
//! ケースが無く、隣接セル間でエッジ補間を正準化することで**水密**メッシュを
//! 構築時から保証する。
//!
//! 当初の計画では特徴保持・面数削減を狙う manifold dual contouring (DC) への
//! 将来的な置換を想定していたが (問11)、MT のまま v0.1.0 を含む全リリースを
//! 通過し、391本のテスト (問281時点) がその水密性・決定性保証の上に構築
//! された。DC の利点 (鋭利エッジ保持・三角形数削減) を要求する具体的な
//! ユースケースがこれまで生じていないため、DC は「未着手のTODO」ではなく
//! 「必要性が具体化するまで着手しない」判断に転じている (ADR-001・問282)。

pub mod marching_tetrahedra;
pub mod mesh;

pub use marching_tetrahedra::polygonize;
pub use mesh::Mesh;

/// `polygonize` に渡せる解像度の上限 (問18 / 問325)。
///
/// `polygonize` は `(res+1)^3` 個の `f64` を確保する。257³ × 8 byte ≈ 136 MiB で、
/// 2 バッファ分を見込んでここで抑える (無境界パラメータによる OOM/panic DoS を防ぐ)。
///
/// 問325 以前は `mcp/tools.rs` の private 定数と CLI の直値 `256` が**二重に**存在し、
/// 片方を変えるともう片方が黙ってずれる構造だった。この上限が守っているのは
/// 本モジュールのサンプル格子メモリなので、所有者である `extract` に一本化する
/// (`MAX_STL_TRIANGLES` が `io/stl.rs` に、`MAX_CROSSINGS` が `core/measure.rs` に
/// あるのと同じ慣例)。
pub const MAX_RESOLUTION: usize = 256;
