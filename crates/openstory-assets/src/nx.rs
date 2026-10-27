//! Bounded random-access reader for the PKG4 NX container used by OpenStory.
//!
//! The format stores compact node records and separate offset tables for strings,
//! bitmaps, and audio. Reading by offset keeps the multi-gigabyte asset set lazy.

use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
use std::io;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::{Arc, Mutex};

const NX_MAGIC: u32 = 0x3447_4B50;
const HEADER_SIZE: u64 = 52;
const NODE_RECORD_SIZE: u64 = 20;
const TABLE_ENTRY_SIZE: u64 = 8;
const MAX_DECODED_BITMAP_SIZE: usize = 512 * 1024 * 1024;
const MAX_COMPRESSED_BITMAP_SIZE: usize = 256 * 1024 * 1024;
const MAX_AUDIO_SIZE: usize = 256 * 1024 * 1024;
const NODE_CACHE_CAPACITY: usize = 65_536;
const STRING_CACHE_CAPACITY_ENTRIES: usize = 65_536;
const STRING_CACHE_CAPACITY_BYTES: usize = 16 * 1024 * 1024;
const BITMAP_CACHE_CAPACITY_ENTRIES: usize = 4_096;
const BITMAP_CACHE_CAPACITY_BYTES: usize = 16 * 1024 * 1024;
const SOURCE_CACHE_BLOCK_SIZE: usize = 64 * 1024;
const SOURCE_CACHE_CAPACITY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum NxError {
    Io(io::Error),
    InvalidHeader(&'static str),
    InvalidNodeType(u16),
    InvalidIndex {
        table: &'static str,
        index: u32,
        count: u32,
    },
    InvalidNodeRange {
        start: u32,
        count: u16,
        total: u32,
    },
    OutOfBounds {
        offset: u64,
        length: u64,
        file_length: u64,
    },
    BitmapTooLarge(usize),
    AudioTooLarge(usize),
    InvalidBitmapData(String),
}

impl fmt::Display for NxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "NX I/O error: {error}"),
            Self::InvalidHeader(message) => write!(f, "invalid NX header: {message}"),
            Self::InvalidNodeType(kind) => write!(f, "unknown NX node type {kind}"),
            Self::InvalidIndex {
                table,
                index,
                count,
            } => {
                write!(
                    f,
                    "NX {table} index {index} is outside table of {count} entries"
                )
            }
            Self::InvalidNodeRange {
                start,
                count,
                total,
            } => write!(
                f,
                "NX child range {start}..{} exceeds {total} nodes",
                u64::from(*start) + u64::from(*count)
            ),
            Self::OutOfBounds {
                offset,
                length,
                file_length,
            } => write!(
                f,
                "NX read {offset}..{} exceeds file length {file_length}",
                offset.saturating_add(*length)
            ),
            Self::BitmapTooLarge(length) => {
                write!(
                    f,
                    "NX bitmap decoded size {length} exceeds the safety limit"
                )
            }
            Self::AudioTooLarge(length) => {
                write!(f, "NX audio size {length} exceeds the safety limit")
            }
            Self::InvalidBitmapData(message) => write!(f, "invalid NX bitmap: {message}"),
        }
    }
}

impl Error for NxError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for NxError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// A source capable of bounded random reads. Native builds use positional file reads;
/// browser asset providers can implement the same shape over a range cache.
pub trait RandomAccessSource: Send + Sync {
    fn len(&self) -> u64;
    fn read_at(&self, offset: u64, output: &mut [u8]) -> io::Result<usize>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// An immutable in-memory NX source for browser-delivered archives and tools.
///
/// Browser code can fetch or otherwise stage an archive through its platform APIs,
/// then pass the bytes here; the NX reader still performs bounded reads and lazy
/// decoding through the same `RandomAccessSource` interface as native files.
#[derive(Clone, Debug)]
pub struct MemorySource {
    bytes: Arc<[u8]>,
}

impl MemorySource {
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }
}

impl RandomAccessSource for MemorySource {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read_at(&self, offset: u64, output: &mut [u8]) -> io::Result<usize> {
        let Ok(offset) = usize::try_from(offset) else {
            return Ok(0);
        };
        if offset >= self.bytes.len() || output.is_empty() {
            return Ok(0);
        }
        let count = output.len().min(self.bytes.len() - offset);
        output[..count].copy_from_slice(&self.bytes[offset..offset + count]);
        Ok(count)
    }
}

/// Bounded block cache for a random-access source.
///
/// This is useful for large local archives and for platform sources backed by an
/// already-populated browser/network range cache. Reads are immutable and safe to
/// share across archive decoder threads.
pub struct CachedSource {
    source: Arc<dyn RandomAccessSource>,
    block_size: usize,
    cache: Mutex<BlockCache>,
}

struct BlockCache {
    blocks: HashMap<u64, CachedBlock>,
    access_order: VecDeque<(u64, u64)>,
    cached_bytes: usize,
    access_clock: u64,
    capacity_bytes: usize,
    max_entries: usize,
}

struct CachedBlock {
    bytes: Arc<[u8]>,
    last_access: u64,
}

impl CachedSource {
    /// Wraps a source with an LRU block cache. `capacity_bytes` must be at least one
    /// block; the cache never retains more than this bound.
    pub fn new(
        source: Arc<dyn RandomAccessSource>,
        block_size: usize,
        capacity_bytes: usize,
    ) -> io::Result<Self> {
        if block_size == 0 || capacity_bytes < block_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "random-access cache needs a nonzero block size and room for one block",
            ));
        }
        Ok(Self {
            source,
            block_size,
            cache: Mutex::new(BlockCache {
                blocks: HashMap::new(),
                access_order: VecDeque::new(),
                cached_bytes: 0,
                access_clock: 0,
                capacity_bytes,
                max_entries: capacity_bytes / block_size,
            }),
        })
    }

    fn block(&self, index: u64) -> io::Result<Arc<[u8]>> {
        if let Some(bytes) = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(index)
        {
            return Ok(bytes);
        }

        let start = index
            .checked_mul(self.block_size as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "cache offset overflow"))?;
        let requested = self
            .source
            .len()
            .saturating_sub(start)
            .min(self.block_size as u64) as usize;
        let mut bytes = vec![0; requested];
        let mut read = 0;
        while read < requested {
            let offset = start.checked_add(read as u64).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "cache offset overflow")
            })?;
            let count = self.source.read_at(offset, &mut bytes[read..])?;
            if count > requested - read {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "random-access source returned more bytes than requested",
                ));
            }
            if count == 0 {
                break;
            }
            read += count;
        }
        bytes.truncate(read);
        let bytes: Arc<[u8]> = bytes.into();

        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(cache.insert(index, bytes))
    }
}

impl RandomAccessSource for CachedSource {
    fn len(&self) -> u64 {
        self.source.len()
    }

    fn read_at(&self, offset: u64, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || offset >= self.source.len() {
            return Ok(0);
        }

        let available = self.source.len().saturating_sub(offset);
        let requested = usize::try_from(available.min(output.len() as u64)).unwrap_or(output.len());
        let mut copied = 0;
        while copied < requested {
            let absolute = offset.checked_add(copied as u64).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "cache offset overflow")
            })?;
            let index = absolute / self.block_size as u64;
            let within_block = (absolute % self.block_size as u64) as usize;
            let block = self.block(index)?;
            if within_block >= block.len() {
                break;
            }
            let count = (requested - copied).min(block.len() - within_block);
            output[copied..copied + count]
                .copy_from_slice(&block[within_block..within_block + count]);
            copied += count;
        }
        Ok(copied)
    }
}

impl BlockCache {
    fn get(&mut self, index: u64) -> Option<Arc<[u8]>> {
        let bytes = self.blocks.get(&index)?.bytes.clone();
        let last_access = self.next_access();
        if let Some(cached) = self.blocks.get_mut(&index) {
            cached.last_access = last_access;
        }
        self.access_order.push_back((last_access, index));
        self.compact_access_order();
        Some(bytes)
    }

    fn insert(&mut self, index: u64, bytes: Arc<[u8]>) -> Arc<[u8]> {
        if let Some(cached) = self.get(index) {
            return cached;
        }
        if bytes.is_empty() || bytes.len() > self.capacity_bytes {
            return bytes;
        }
        while self.cached_bytes.saturating_add(bytes.len()) > self.capacity_bytes {
            let Some((access, evicted)) = self.access_order.pop_front() else {
                break;
            };
            if self
                .blocks
                .get(&evicted)
                .is_some_and(|cached| cached.last_access == access)
                && let Some(cached) = self.blocks.remove(&evicted)
            {
                self.cached_bytes = self.cached_bytes.saturating_sub(cached.bytes.len());
            }
        }
        let last_access = self.next_access();
        self.cached_bytes += bytes.len();
        self.access_order.push_back((last_access, index));
        self.blocks.insert(
            index,
            CachedBlock {
                bytes: bytes.clone(),
                last_access,
            },
        );
        self.compact_access_order();
        bytes
    }

    fn compact_access_order(&mut self) {
        if self.access_order.len() > self.max_entries.saturating_mul(2) {
            self.access_order.retain(|(access, index)| {
                self.blocks
                    .get(index)
                    .is_some_and(|cached| cached.last_access == *access)
            });
        }
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut recency: Vec<_> = self
                .blocks
                .iter()
                .map(|(index, cached)| (*index, cached.last_access))
                .collect();
            recency.sort_unstable_by_key(|(_, last_access)| *last_access);
            self.access_order.clear();
            for (access, (index, _)) in recency.into_iter().enumerate() {
                let access = access as u64;
                if let Some(cached) = self.blocks.get_mut(&index) {
                    cached.last_access = access;
                }
                self.access_order.push_back((access, index));
            }
            self.access_clock = self.blocks.len() as u64;
        }
        self.access_clock += 1;
        self.access_clock
    }
}

#[derive(Debug)]
#[cfg(not(target_arch = "wasm32"))]
struct FileSource {
    file: File,
    length: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl FileSource {
    fn open(path: &Path) -> Result<Self, NxError> {
        let file = File::open(path)?;
        let length = file.metadata()?.len();
        Ok(Self { file, length })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RandomAccessSource for FileSource {
    fn len(&self) -> u64 {
        self.length
    }

    fn read_at(&self, offset: u64, output: &mut [u8]) -> io::Result<usize> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt;
            self.file.read_at(output, offset)
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileExt;
            self.file.seek_read(output, offset)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (offset, output);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "random-access file reads unavailable",
            ))
        }
    }
}

#[derive(Clone, Debug)]
struct Header {
    node_count: u32,
    node_offset: u64,
    string_count: u32,
    string_offset: u64,
    bitmap_count: u32,
    bitmap_offset: u64,
    audio_count: u32,
    audio_offset: u64,
}

/// Public summary of an NX file's top-level tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NxHeader {
    pub node_count: u32,
    pub string_count: u32,
    pub bitmap_count: u32,
    pub audio_count: u32,
}

#[derive(Clone, Debug)]
struct NodeRecord {
    name: u32,
    children: u32,
    child_count: u16,
    kind: u16,
    value: [u8; 8],
}

struct NodeCache {
    records: HashMap<u32, CachedNode>,
    access_order: VecDeque<(u64, u32)>,
    access_clock: u64,
}

struct CachedNode {
    record: Arc<NodeRecord>,
    last_access: u64,
}

impl NodeCache {
    fn new() -> Self {
        Self {
            records: HashMap::new(),
            access_order: VecDeque::new(),
            access_clock: 0,
        }
    }

    fn get(&mut self, index: &u32) -> Option<Arc<NodeRecord>> {
        let record = self.records.get(index)?.record.clone();
        let last_access = self.next_access();
        if let Some(cached) = self.records.get_mut(index) {
            cached.last_access = last_access;
        }
        self.access_order.push_back((last_access, *index));
        self.compact_access_order();
        Some(record)
    }

    fn insert(&mut self, index: u32, record: Arc<NodeRecord>) -> Arc<NodeRecord> {
        if let Some(cached) = self.get(&index) {
            return cached;
        }
        while self.records.len() >= NODE_CACHE_CAPACITY {
            let Some((access, evicted)) = self.access_order.pop_front() else {
                break;
            };
            if self
                .records
                .get(&evicted)
                .is_some_and(|cached| cached.last_access == access)
            {
                self.records.remove(&evicted);
            }
        }
        let last_access = self.next_access();
        self.access_order.push_back((last_access, index));
        self.records.insert(
            index,
            CachedNode {
                record: record.clone(),
                last_access,
            },
        );
        self.compact_access_order();
        record
    }

    fn compact_access_order(&mut self) {
        if self.access_order.len() > NODE_CACHE_CAPACITY * 2 {
            self.access_order.retain(|(access, index)| {
                self.records
                    .get(index)
                    .is_some_and(|cached| cached.last_access == *access)
            });
        }
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut recency: Vec<_> = self
                .records
                .iter()
                .map(|(index, cached)| (*index, cached.last_access))
                .collect();
            recency.sort_unstable_by_key(|(_, last_access)| *last_access);
            self.access_order.clear();
            for (access, (index, _)) in recency.into_iter().enumerate() {
                let access = access as u64;
                if let Some(cached) = self.records.get_mut(&index) {
                    cached.last_access = access;
                }
                self.access_order.push_back((access, index));
            }
            self.access_clock = self.records.len() as u64;
        }
        self.access_clock += 1;
        self.access_clock
    }
}

struct StringCache {
    values: HashMap<u32, CachedString>,
    access_order: VecDeque<(u64, u32)>,
    cached_bytes: usize,
    access_clock: u64,
}

struct CachedString {
    bytes: Arc<[u8]>,
    last_access: u64,
}

impl StringCache {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            access_order: VecDeque::new(),
            cached_bytes: 0,
            access_clock: 0,
        }
    }

    fn get(&mut self, index: &u32) -> Option<Arc<[u8]>> {
        let bytes = self.values.get(index)?.bytes.clone();
        let last_access = self.next_access();
        if let Some(cached) = self.values.get_mut(index) {
            cached.last_access = last_access;
        }
        self.access_order.push_back((last_access, *index));
        self.compact_access_order();
        Some(bytes)
    }

    fn insert(&mut self, index: u32, value: Arc<[u8]>) -> Arc<[u8]> {
        if let Some(cached) = self.get(&index) {
            return cached;
        }
        if value.len() > STRING_CACHE_CAPACITY_BYTES {
            return value;
        }
        while self.values.len() >= STRING_CACHE_CAPACITY_ENTRIES
            || self.cached_bytes.saturating_add(value.len()) > STRING_CACHE_CAPACITY_BYTES
        {
            let Some((access, evicted)) = self.access_order.pop_front() else {
                break;
            };
            if self
                .values
                .get(&evicted)
                .is_some_and(|cached| cached.last_access == access)
                && let Some(cached) = self.values.remove(&evicted)
            {
                self.cached_bytes = self.cached_bytes.saturating_sub(cached.bytes.len());
            }
        }
        let last_access = self.next_access();
        self.cached_bytes += value.len();
        self.access_order.push_back((last_access, index));
        self.values.insert(
            index,
            CachedString {
                bytes: value.clone(),
                last_access,
            },
        );
        self.compact_access_order();
        value
    }

    fn compact_access_order(&mut self) {
        if self.access_order.len() > STRING_CACHE_CAPACITY_ENTRIES * 2 {
            self.access_order.retain(|(access, index)| {
                self.values
                    .get(index)
                    .is_some_and(|cached| cached.last_access == *access)
            });
        }
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut recency: Vec<_> = self
                .values
                .iter()
                .map(|(index, cached)| (*index, cached.last_access))
                .collect();
            recency.sort_unstable_by_key(|(_, last_access)| *last_access);
            self.access_order.clear();
            for (access, (index, _)) in recency.into_iter().enumerate() {
                let access = access as u64;
                if let Some(cached) = self.values.get_mut(&index) {
                    cached.last_access = access;
                }
                self.access_order.push_back((access, index));
            }
            self.access_clock = self.values.len() as u64;
        }
        self.access_clock += 1;
        self.access_clock
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct BitmapKey {
    index: u32,
    width: u16,
    height: u16,
}

/// Mutex-protected LRU cache with both decoded-byte and entry-count bounds.
struct BitmapCache {
    values: HashMap<BitmapKey, CachedBitmap>,
    access_order: VecDeque<(u64, BitmapKey)>,
    cached_bytes: usize,
    access_clock: u64,
}

struct CachedBitmap {
    pixels: Arc<[u8]>,
    last_access: u64,
}

impl BitmapCache {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            access_order: VecDeque::new(),
            cached_bytes: 0,
            access_clock: 0,
        }
    }

    fn get(&mut self, key: &BitmapKey) -> Option<Arc<[u8]>> {
        let pixels = self.values.get(key)?.pixels.clone();
        let last_access = self.next_access();
        if let Some(cached) = self.values.get_mut(key) {
            cached.last_access = last_access;
        }
        self.access_order.push_back((last_access, *key));
        self.compact_access_order();
        Some(pixels)
    }

    fn insert(&mut self, key: BitmapKey, pixels: Arc<[u8]>) -> Arc<[u8]> {
        if let Some(cached) = self.get(&key) {
            return cached;
        }
        if pixels.len() > BITMAP_CACHE_CAPACITY_BYTES {
            return pixels;
        }
        while self.values.len() >= BITMAP_CACHE_CAPACITY_ENTRIES
            || self.cached_bytes.saturating_add(pixels.len()) > BITMAP_CACHE_CAPACITY_BYTES
        {
            let Some((access, evicted)) = self.access_order.pop_front() else {
                break;
            };
            if self
                .values
                .get(&evicted)
                .is_some_and(|cached| cached.last_access == access)
                && let Some(cached) = self.values.remove(&evicted)
            {
                self.cached_bytes = self.cached_bytes.saturating_sub(cached.pixels.len());
            }
        }
        let last_access = self.next_access();
        self.cached_bytes += pixels.len();
        self.access_order.push_back((last_access, key));
        self.values.insert(
            key,
            CachedBitmap {
                pixels: pixels.clone(),
                last_access,
            },
        );
        self.compact_access_order();
        pixels
    }

    fn compact_access_order(&mut self) {
        if self.access_order.len() > BITMAP_CACHE_CAPACITY_ENTRIES * 2 {
            self.access_order.retain(|(access, key)| {
                self.values
                    .get(key)
                    .is_some_and(|cached| cached.last_access == *access)
            });
        }
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            // Preserve the existing recency order before the monotonic clock wraps.
            let mut recency: Vec<_> = self
                .values
                .iter()
                .map(|(key, cached)| (*key, cached.last_access))
                .collect();
            recency.sort_unstable_by_key(|(_, last_access)| *last_access);
            self.access_order.clear();
            for (access, (key, _)) in recency.into_iter().enumerate() {
                let access = access as u64;
                if let Some(cached) = self.values.get_mut(&key) {
                    cached.last_access = access;
                }
                self.access_order.push_back((access, key));
            }
            self.access_clock = self.values.len() as u64;
        }
        self.access_clock += 1;
        self.access_clock
    }
}

struct ArchiveInner {
    source: Arc<dyn RandomAccessSource>,
    header: Header,
    node_cache: Mutex<NodeCache>,
    string_cache: Mutex<StringCache>,
    bitmap_cache: Mutex<BitmapCache>,
}

/// An opened PKG4 NX file. Records and values load on demand into bounded caches.
#[derive(Clone)]
pub struct NxArchive {
    inner: Arc<ArchiveInner>,
}

impl fmt::Debug for NxArchive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NxArchive")
            .field("header", &self.header())
            .field("length", &self.inner.source.len())
            .finish()
    }
}

impl NxArchive {
    /// Opens one NX file without reading its complete contents into memory.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(path: impl AsRef<Path>) -> Result<Self, NxError> {
        let source = Arc::new(FileSource::open(path.as_ref())?);
        let source =
            CachedSource::new(source, SOURCE_CACHE_BLOCK_SIZE, SOURCE_CACHE_CAPACITY_BYTES)?;
        Self::from_source(Arc::new(source))
    }

    pub fn from_source(source: Arc<dyn RandomAccessSource>) -> Result<Self, NxError> {
        let file_length = source.len();
        if file_length < HEADER_SIZE {
            return Err(NxError::InvalidHeader("file is shorter than 52 bytes"));
        }
        let mut raw = [0_u8; HEADER_SIZE as usize];
        read_exact_at(source.as_ref(), 0, &mut raw)?;
        let magic = u32_at(&raw, 0);
        if magic != NX_MAGIC {
            return Err(NxError::InvalidHeader("expected PKG4 magic"));
        }

        let header = Header {
            node_count: u32_at(&raw, 4),
            node_offset: u64_at(&raw, 8),
            string_count: u32_at(&raw, 16),
            string_offset: u64_at(&raw, 20),
            bitmap_count: u32_at(&raw, 28),
            bitmap_offset: u64_at(&raw, 32),
            audio_count: u32_at(&raw, 40),
            audio_offset: u64_at(&raw, 44),
        };

        check_region(
            header.node_offset,
            u64::from(header.node_count),
            NODE_RECORD_SIZE,
            file_length,
        )?;
        check_region(
            header.string_offset,
            u64::from(header.string_count),
            TABLE_ENTRY_SIZE,
            file_length,
        )?;
        check_region(
            header.bitmap_offset,
            u64::from(header.bitmap_count),
            TABLE_ENTRY_SIZE,
            file_length,
        )?;
        check_region(
            header.audio_offset,
            u64::from(header.audio_count),
            TABLE_ENTRY_SIZE,
            file_length,
        )?;
        if header.node_count == 0 {
            return Err(NxError::InvalidHeader("node table has no root node"));
        }

        Ok(Self {
            inner: Arc::new(ArchiveInner {
                source,
                header,
                node_cache: Mutex::new(NodeCache::new()),
                string_cache: Mutex::new(StringCache::new()),
                bitmap_cache: Mutex::new(BitmapCache::new()),
            }),
        })
    }

    pub fn header(&self) -> NxHeader {
        NxHeader {
            node_count: self.inner.header.node_count,
            string_count: self.inner.header.string_count,
            bitmap_count: self.inner.header.bitmap_count,
            audio_count: self.inner.header.audio_count,
        }
    }

    pub fn root(&self) -> NxNode {
        NxNode {
            archive: self.clone(),
            index: 0,
        }
    }

    fn node_record(&self, index: u32) -> Result<Arc<NodeRecord>, NxError> {
        if index >= self.inner.header.node_count {
            return Err(NxError::InvalidIndex {
                table: "node",
                index,
                count: self.inner.header.node_count,
            });
        }
        if let Some(record) = lock_recover(&self.inner.node_cache).get(&index) {
            return Ok(record);
        }

        let offset = self
            .inner
            .header
            .node_offset
            .checked_add(u64::from(index) * NODE_RECORD_SIZE)
            .ok_or(NxError::InvalidHeader("node offset overflow"))?;
        let mut bytes = [0_u8; NODE_RECORD_SIZE as usize];
        read_exact_at(self.inner.source.as_ref(), offset, &mut bytes)?;
        let record = Arc::new(NodeRecord {
            name: u32_at(&bytes, 0),
            children: u32_at(&bytes, 4),
            child_count: u16_at(&bytes, 8),
            kind: u16_at(&bytes, 10),
            value: bytes[12..20].try_into().expect("fixed-size NX node value"),
        });
        if record.name >= self.inner.header.string_count {
            return Err(NxError::InvalidIndex {
                table: "string",
                index: record.name,
                count: self.inner.header.string_count,
            });
        }
        let end = u64::from(record.children) + u64::from(record.child_count);
        if end > u64::from(self.inner.header.node_count) {
            return Err(NxError::InvalidNodeRange {
                start: record.children,
                count: record.child_count,
                total: self.inner.header.node_count,
            });
        }

        Ok(lock_recover(&self.inner.node_cache).insert(index, record))
    }

    fn string_bytes(&self, index: u32) -> Result<Arc<[u8]>, NxError> {
        if index >= self.inner.header.string_count {
            return Err(NxError::InvalidIndex {
                table: "string",
                index,
                count: self.inner.header.string_count,
            });
        }
        if let Some(value) = lock_recover(&self.inner.string_cache).get(&index) {
            return Ok(value);
        }

        let table_slot = self
            .inner
            .header
            .string_offset
            .checked_add(u64::from(index) * TABLE_ENTRY_SIZE)
            .ok_or(NxError::InvalidHeader("string table offset overflow"))?;
        let record_offset = read_u64_at(self.inner.source.as_ref(), table_slot)?;
        let mut length_bytes = [0_u8; 2];
        read_exact_at(self.inner.source.as_ref(), record_offset, &mut length_bytes)?;
        let length = u64::from(u16::from_le_bytes(length_bytes));
        let data_offset = record_offset
            .checked_add(2)
            .ok_or(NxError::InvalidHeader("string offset overflow"))?;
        check_bounds(data_offset, length, self.inner.source.len())?;
        let mut bytes = vec![0; length as usize];
        read_exact_at(self.inner.source.as_ref(), data_offset, &mut bytes)?;
        let value: Arc<[u8]> = bytes.into();
        Ok(lock_recover(&self.inner.string_cache).insert(index, value))
    }

    fn table_data_offset(
        &self,
        table: &'static str,
        index: u32,
        count: u32,
        offset: u64,
    ) -> Result<u64, NxError> {
        if index >= count {
            return Err(NxError::InvalidIndex {
                table,
                index,
                count,
            });
        }
        let slot = offset
            .checked_add(u64::from(index) * TABLE_ENTRY_SIZE)
            .ok_or(NxError::InvalidHeader("data table offset overflow"))?;
        read_u64_at(self.inner.source.as_ref(), slot)
    }
}

#[derive(Clone, Debug)]
pub struct NxNode {
    archive: NxArchive,
    index: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    None,
    Integer(i64),
    Real(f64),
    String(String),
    Vector { x: i32, y: i32 },
    Bitmap { index: u32, width: u16, height: u16 },
    Audio { index: u32, length: u32 },
}

impl NxNode {
    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn name(&self) -> Result<String, NxError> {
        let record = self.archive.node_record(self.index)?;
        let bytes = self.archive.string_bytes(record.name)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    pub fn value(&self) -> Result<NodeValue, NxError> {
        let record = self.archive.node_record(self.index)?;
        let value = match record.kind {
            0 => NodeValue::None,
            1 => NodeValue::Integer(i64::from_le_bytes(record.value)),
            2 => NodeValue::Real(f64::from_le_bytes(record.value)),
            3 => {
                let index = u32::from_le_bytes(record.value[..4].try_into().expect("four bytes"));
                let bytes = self.archive.string_bytes(index)?;
                NodeValue::String(String::from_utf8_lossy(&bytes).into_owned())
            }
            4 => NodeValue::Vector {
                x: i32::from_le_bytes(record.value[..4].try_into().expect("four bytes")),
                y: i32::from_le_bytes(record.value[4..].try_into().expect("four bytes")),
            },
            5 => NodeValue::Bitmap {
                index: u32::from_le_bytes(record.value[..4].try_into().expect("four bytes")),
                width: u16::from_le_bytes(record.value[4..6].try_into().expect("two bytes")),
                height: u16::from_le_bytes(record.value[6..8].try_into().expect("two bytes")),
            },
            6 => NodeValue::Audio {
                index: u32::from_le_bytes(record.value[..4].try_into().expect("four bytes")),
                length: u32::from_le_bytes(record.value[4..].try_into().expect("four bytes")),
            },
            kind => return Err(NxError::InvalidNodeType(kind)),
        };
        Ok(value)
    }

    pub fn child(&self, name: &str) -> Result<Option<Self>, NxError> {
        let parent = self.archive.node_record(self.index)?;
        let sought = name.as_bytes();
        let mut low = 0_u32;
        let mut high = u32::from(parent.child_count);
        while low < high {
            let middle = low + (high - low) / 2;
            let child_index = parent.children + middle;
            let child = self.archive.node_record(child_index)?;
            let child_name = self.archive.string_bytes(child.name)?;
            match child_name.as_ref().cmp(sought) {
                std::cmp::Ordering::Less => low = middle + 1,
                std::cmp::Ordering::Greater => high = middle,
                std::cmp::Ordering::Equal => {
                    return Ok(Some(Self {
                        archive: self.archive.clone(),
                        index: child_index,
                    }));
                }
            }
        }
        Ok(None)
    }

    pub fn children(&self) -> Result<Vec<Self>, NxError> {
        let parent = self.archive.node_record(self.index)?;
        let first = parent.children;
        let end = first + u32::from(parent.child_count);
        Ok((first..end)
            .map(|index| Self {
                archive: self.archive.clone(),
                index,
            })
            .collect())
    }

    pub fn resolve(&self, path: &str) -> Result<Option<Self>, NxError> {
        let mut node = self.clone();
        for part in path.split('/').filter(|part| !part.is_empty()) {
            let Some(child) = node.child(part)? else {
                return Ok(None);
            };
            node = child;
        }
        Ok(Some(node))
    }

    pub fn bitmap(&self) -> Result<Option<Bitmap>, NxError> {
        let NodeValue::Bitmap {
            index,
            width,
            height,
        } = self.value()?
        else {
            return Ok(None);
        };
        if index >= self.archive.inner.header.bitmap_count {
            return Err(NxError::InvalidIndex {
                table: "bitmap",
                index,
                count: self.archive.inner.header.bitmap_count,
            });
        }
        Ok(Some(Bitmap {
            archive: self.archive.clone(),
            index,
            width,
            height,
        }))
    }

    pub fn audio_bytes(&self) -> Result<Option<Vec<u8>>, NxError> {
        let NodeValue::Audio { index, length } = self.value()? else {
            return Ok(None);
        };
        let offset = self.archive.table_data_offset(
            "audio",
            index,
            self.archive.inner.header.audio_count,
            self.archive.inner.header.audio_offset,
        )?;
        if length as usize > MAX_AUDIO_SIZE {
            return Err(NxError::AudioTooLarge(length as usize));
        }
        check_bounds(offset, u64::from(length), self.archive.inner.source.len())?;
        let mut bytes = vec![0; length as usize];
        read_exact_at(self.archive.inner.source.as_ref(), offset, &mut bytes)?;
        Ok(Some(bytes))
    }
}

#[derive(Clone, Debug)]
pub struct Bitmap {
    archive: NxArchive,
    index: u32,
    width: u16,
    height: u16,
}

impl Bitmap {
    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Decodes the NX LZ4 block into its raw four-byte-per-pixel buffer.
    pub fn decode_bgra(&self) -> Result<Vec<u8>, NxError> {
        Ok(self.decode_bgra_shared()?.as_ref().to_vec())
    }

    /// Decodes an NX bitmap once per archive and shares the bounded cached pixels.
    pub fn decode_bgra_shared(&self) -> Result<Arc<[u8]>, NxError> {
        let key = BitmapKey {
            index: self.index,
            width: self.width,
            height: self.height,
        };
        let cached_pixels = {
            let mut cache = lock_recover(&self.archive.inner.bitmap_cache);
            cache.get(&key)
        };
        if let Some(pixels) = cached_pixels {
            return Ok(pixels);
        }
        let offset = self.archive.table_data_offset(
            "bitmap",
            self.index,
            self.archive.inner.header.bitmap_count,
            self.archive.inner.header.bitmap_offset,
        )?;
        let decoded_len = usize::from(self.width)
            .checked_mul(usize::from(self.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(NxError::BitmapTooLarge(usize::MAX))?;
        if decoded_len > MAX_DECODED_BITMAP_SIZE {
            return Err(NxError::BitmapTooLarge(decoded_len));
        }

        let compressed_length = read_u32_at(self.archive.inner.source.as_ref(), offset)? as usize;
        if compressed_length > MAX_COMPRESSED_BITMAP_SIZE {
            return Err(NxError::BitmapTooLarge(compressed_length));
        }
        let data_offset = offset
            .checked_add(4)
            .ok_or(NxError::InvalidHeader("bitmap offset overflow"))?;
        check_bounds(
            data_offset,
            compressed_length as u64,
            self.archive.inner.source.len(),
        )?;
        let mut compressed = vec![0; compressed_length];
        read_exact_at(
            self.archive.inner.source.as_ref(),
            data_offset,
            &mut compressed,
        )?;
        let mut pixels = vec![0; decoded_len];
        let actual = lz4_flex::block::decompress_into(&compressed, &mut pixels)
            .map_err(|error| NxError::InvalidBitmapData(error.to_string()))?;
        if actual != decoded_len {
            return Err(NxError::InvalidBitmapData(format!(
                "decoded {actual} bytes but dimensions require {decoded_len}"
            )));
        }
        let pixels: Arc<[u8]> = pixels.into();
        Ok(lock_recover(&self.archive.inner.bitmap_cache).insert(key, pixels))
    }
}

fn read_exact_at(
    source: &dyn RandomAccessSource,
    mut offset: u64,
    mut output: &mut [u8],
) -> Result<(), NxError> {
    let total_length = output.len() as u64;
    check_bounds(offset, total_length, source.len())?;
    while !output.is_empty() {
        let read = source.read_at(offset, output)?;
        if read == 0 {
            return Err(NxError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "short NX read",
            )));
        }
        if read > output.len() {
            return Err(NxError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                "random-access source returned too many bytes",
            )));
        }
        offset += read as u64;
        output = &mut output[read..];
    }
    Ok(())
}

fn read_u32_at(source: &dyn RandomAccessSource, offset: u64) -> Result<u32, NxError> {
    let mut bytes = [0; 4];
    read_exact_at(source, offset, &mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64_at(source: &dyn RandomAccessSource, offset: u64) -> Result<u64, NxError> {
    let mut bytes = [0; 8];
    read_exact_at(source, offset, &mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn check_region(offset: u64, count: u64, stride: u64, file_length: u64) -> Result<(), NxError> {
    let length = count
        .checked_mul(stride)
        .ok_or(NxError::InvalidHeader("table length overflow"))?;
    check_bounds(offset, length, file_length)
}

fn check_bounds(offset: u64, length: u64, file_length: u64) -> Result<(), NxError> {
    let Some(end) = offset.checked_add(length) else {
        return Err(NxError::OutOfBounds {
            offset,
            length,
            file_length,
        });
    };
    if end > file_length {
        return Err(NxError::OutOfBounds {
            offset,
            length,
            file_length,
        });
    }
    Ok(())
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("bounded NX field"),
    )
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("bounded NX field"),
    )
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("bounded NX field"),
    )
}

fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
