#![forbid(unsafe_code)]

//! Cross-platform rendering foundation.

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{BgraImage, BlendMode, GpuRenderer, GpuTexture, RendererError, SpriteDraw};
