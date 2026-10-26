#![forbid(unsafe_code)]

//! Bounded access to native or platform-provided NX data and its typed node tree.

mod directory;
mod nx;

pub use directory::{AssetDirectory, AssetDirectoryError, REQUIRED_NX_FILES};
pub use nx::{
    Bitmap, MemorySource, NodeValue, NxArchive, NxError, NxHeader, NxNode, RandomAccessSource,
};
