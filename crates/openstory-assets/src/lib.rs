#![forbid(unsafe_code)]

//! Bounded access to native or platform-provided NX data and its typed node tree.

mod directory;
mod nx;
mod range_source;

pub use directory::{AssetDirectory, AssetDirectoryError, REQUIRED_NX_FILES};
pub use nx::{
    Bitmap, CachedSource, MemorySource, NodeValue, NxArchive, NxError, NxHeader, NxNode,
    RandomAccessSource,
};
pub use range_source::{AssetRangeRequest, RangeBackedSource};
