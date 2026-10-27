//! Bounded random-access source for assets delivered in asynchronous byte ranges.
//!
//! The source contains no browser or networking APIs. A platform adapter drains
//! [`RangeBackedSource::take_requests`], fetches each range asynchronously, and
//! supplies the bytes with [`RangeBackedSource::provide_range`]. Synchronous NX
//! readers can then retry the same operation after the missing pages arrive.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::RandomAccessSource;

const MAX_PENDING_RANGE_PAGES: usize = 16_384;

/// One page-aligned byte range requested from the platform asset provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetRangeRequest {
    pub offset: u64,
    pub length: u64,
}

/// A bounded, lazy source suitable for browser-managed asset delivery.
///
/// Pages are immutable once loaded. Reads of absent pages return `WouldBlock` and
/// queue their page indices for the platform adapter. The adapter must complete
/// or release every request before retrying a failed decode. Both queued requests
/// and retained page bytes have explicit bounds.
pub struct RangeBackedSource {
    length: u64,
    page_size: usize,
    max_cached_bytes: usize,
    max_cached_pages: usize,
    state: Mutex<RangeState>,
}

struct RangeState {
    pages: HashMap<u64, CachedPage>,
    access_order: VecDeque<(u64, u64)>,
    pending: VecDeque<u64>,
    pending_pages: HashSet<u64>,
    in_flight_pages: HashSet<u64>,
    cached_bytes: usize,
    access_clock: u64,
}

struct CachedPage {
    bytes: Arc<[u8]>,
    last_access: u64,
}

impl RangeBackedSource {
    /// Creates a lazy source for an asset of `length` bytes.
    ///
    /// The byte cache must fit at least one full page. The final page may be
    /// shorter when the asset length is not divisible by `page_size`.
    pub fn new(length: u64, page_size: usize, max_cached_bytes: usize) -> io::Result<Self> {
        if page_size == 0 || max_cached_bytes < page_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "range source needs a nonzero page size and room for one full page",
            ));
        }

        Ok(Self {
            length,
            page_size,
            max_cached_bytes,
            max_cached_pages: max_cached_bytes / page_size,
            state: Mutex::new(RangeState {
                pages: HashMap::new(),
                access_order: VecDeque::new(),
                pending: VecDeque::new(),
                pending_pages: HashSet::new(),
                in_flight_pages: HashSet::new(),
                cached_bytes: 0,
                access_clock: 0,
            }),
        })
    }

    /// Returns up to `max_ranges` coalesced, page-aligned requests and marks their
    /// pages in flight. Each request contains no more than `max_pages_per_range`
    /// pages; this is useful for bounding browser fetch sizes.
    pub fn take_requests(
        &self,
        max_ranges: usize,
        max_pages_per_range: usize,
    ) -> Vec<AssetRangeRequest> {
        if max_ranges == 0 || max_pages_per_range == 0 {
            return Vec::new();
        }

        let page_limit = max_pages_per_range.min(self.max_cached_pages);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut requests = Vec::with_capacity(max_ranges.min(state.pending.len()));

        while requests.len() < max_ranges {
            let Some(first) = pop_pending_page(&mut state) else {
                break;
            };
            state.in_flight_pages.insert(first);

            let mut last = first;
            let mut page_count = 1;
            while page_count < page_limit {
                let Some(next) = last.checked_add(1) else {
                    break;
                };
                if !state.pending_pages.remove(&next) {
                    break;
                }
                state.in_flight_pages.insert(next);
                last = next;
                page_count += 1;
            }

            let offset = first.saturating_mul(self.page_size as u64);
            let end = last
                .saturating_add(1)
                .saturating_mul(self.page_size as u64)
                .min(self.length);
            requests.push(AssetRangeRequest {
                offset,
                length: end.saturating_sub(offset),
            });
        }
        state.compact_pending();

        requests
    }

    /// Queues a page-aligned range for proactive loading, including the NX header
    /// page needed before [`crate::NxArchive::from_source`] can open an archive.
    pub fn request_range(&self, request: AssetRangeRequest) -> io::Result<()> {
        let pages = self.request_pages(request)?;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let missing = pages
            .clone()
            .filter(|page| {
                !state.pages.contains_key(page)
                    && !state.pending_pages.contains(page)
                    && !state.in_flight_pages.contains(page)
            })
            .count();
        if state
            .pending_pages
            .len()
            .saturating_add(state.in_flight_pages.len())
            .saturating_add(missing)
            > MAX_PENDING_RANGE_PAGES
        {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "asset range request queue is full",
            ));
        }
        for page_index in pages {
            if !state.pages.contains_key(&page_index)
                && !state.pending_pages.contains(&page_index)
                && !state.in_flight_pages.contains(&page_index)
            {
                state.pending_pages.insert(page_index);
                state.pending.push_back(page_index);
            }
        }
        Ok(())
    }

    /// Installs the exact response bytes for a request returned by
    /// [`Self::take_requests`]. `request.offset` must be page aligned, and a
    /// non-final range must end on a page boundary.
    pub fn provide_range(&self, request: AssetRangeRequest, bytes: &[u8]) -> io::Result<()> {
        let pages = self.request_pages(request)?;
        if bytes.len() as u64 != request.length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "asset range response has {} bytes; expected {}",
                    bytes.len(),
                    request.length
                ),
            ));
        }

        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Check all overlaps before modifying the cache so a conflicting response
        // cannot leave a partially installed range behind.
        let mut cursor = 0;
        for page_index in pages.clone() {
            let expected = self.page_length(page_index);
            let end = cursor + expected;
            if let Some(existing) = state.pages.get(&page_index)
                && existing.bytes.as_ref() != &bytes[cursor..end]
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "asset provider returned different bytes for an immutable page",
                ));
            }
            cursor = end;
        }

        cursor = 0;
        for page_index in pages {
            let page_length = self.page_length(page_index);
            let end = cursor + page_length;
            if !state.pages.contains_key(&page_index) {
                let page: Arc<[u8]> = Arc::from(bytes[cursor..end].to_vec());
                state.insert(
                    page_index,
                    page,
                    self.max_cached_bytes,
                    self.max_cached_pages,
                );
            } else {
                state.touch(page_index, self.max_cached_pages);
            }
            state.pending_pages.remove(&page_index);
            state.in_flight_pages.remove(&page_index);
            cursor = end;
        }
        state.compact_pending();

        Ok(())
    }

    /// Releases a failed request so a later read can request those pages again.
    /// This lets the platform adapter apply its own retry/backoff policy.
    pub fn release_request(&self, request: AssetRangeRequest) -> io::Result<()> {
        let pages = self.request_pages(request)?;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for page in pages {
            state.in_flight_pages.remove(&page);
        }
        Ok(())
    }

    /// Number of immutable pages currently retained by the bounded cache.
    pub fn cached_page_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pages
            .len()
    }

    /// Total bytes currently retained by the bounded page cache.
    pub fn cached_byte_count(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .cached_bytes
    }

    fn page_length(&self, page_index: u64) -> usize {
        let page_start = page_index.saturating_mul(self.page_size as u64);
        self.length
            .saturating_sub(page_start)
            .min(self.page_size as u64) as usize
    }

    fn request_pages(&self, request: AssetRangeRequest) -> io::Result<Range<u64>> {
        let page_size = self.page_size as u64;
        let end = request
            .offset
            .checked_add(request.length)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "asset range overflow"))?;
        if request.length == 0
            || request.offset % page_size != 0
            || end > self.length
            || (end != self.length && end % page_size != 0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "asset range must be non-empty, page aligned, and within the asset",
            ));
        }
        let first = request.offset / page_size;
        let last = end.div_ceil(page_size);
        Ok(first..last)
    }
}

impl RandomAccessSource for RangeBackedSource {
    fn len(&self) -> u64 {
        self.length
    }

    fn read_at(&self, offset: u64, output: &mut [u8]) -> io::Result<usize> {
        if offset >= self.length || output.is_empty() {
            return Ok(0);
        }

        let available = self.length.saturating_sub(offset);
        let requested = usize::try_from(available.min(output.len() as u64)).unwrap_or(output.len());
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut copied = 0;

        while copied < requested {
            let absolute = offset.checked_add(copied as u64).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "asset read offset overflow")
            })?;
            let page_index = absolute / self.page_size as u64;
            let within_page = (absolute % self.page_size as u64) as usize;
            let Some(page) = state.pages.get(&page_index).map(|page| page.bytes.clone()) else {
                queue_missing_page(&mut state, page_index);
                if copied == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "asset range is not available yet",
                    ));
                }
                break;
            };
            state.touch(page_index, self.max_cached_pages);

            if within_page >= page.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "cached asset page is shorter than its declared range",
                ));
            }
            let count = (requested - copied).min(page.len() - within_page);
            output[copied..copied + count].copy_from_slice(&page[within_page..within_page + count]);
            copied += count;
        }

        Ok(copied)
    }
}

impl RangeState {
    fn touch(&mut self, page_index: u64, max_pages: usize) {
        if !self.pages.contains_key(&page_index) {
            return;
        }
        let access = self.next_access();
        if let Some(page) = self.pages.get_mut(&page_index) {
            page.last_access = access;
        }
        self.access_order.push_back((access, page_index));
        self.compact_access_order(max_pages);
    }

    fn insert(&mut self, page_index: u64, bytes: Arc<[u8]>, max_bytes: usize, max_pages: usize) {
        while self.cached_bytes.saturating_add(bytes.len()) > max_bytes {
            let Some((access, evicted)) = self.access_order.pop_front() else {
                break;
            };
            if self
                .pages
                .get(&evicted)
                .is_some_and(|page| page.last_access == access)
                && let Some(page) = self.pages.remove(&evicted)
            {
                self.cached_bytes = self.cached_bytes.saturating_sub(page.bytes.len());
            }
        }

        let access = self.next_access();
        self.cached_bytes += bytes.len();
        self.access_order.push_back((access, page_index));
        self.pages.insert(
            page_index,
            CachedPage {
                bytes,
                last_access: access,
            },
        );
        self.compact_access_order(max_pages);
    }

    fn compact_access_order(&mut self, max_pages: usize) {
        if self.access_order.len() > max_pages.saturating_mul(2) {
            self.access_order.retain(|(access, index)| {
                self.pages
                    .get(index)
                    .is_some_and(|page| page.last_access == *access)
            });
        }
    }

    fn compact_pending(&mut self) {
        if self.pending.len() > self.pending_pages.len().saturating_mul(2) {
            self.pending
                .retain(|page_index| self.pending_pages.contains(page_index));
        }
    }

    fn next_access(&mut self) -> u64 {
        if self.access_clock == u64::MAX {
            let mut recency: Vec<_> = self
                .pages
                .iter()
                .map(|(index, page)| (*index, page.last_access))
                .collect();
            recency.sort_unstable_by_key(|(_, last_access)| *last_access);
            self.access_order.clear();
            for (access, (index, _)) in recency.into_iter().enumerate() {
                let access = access as u64;
                if let Some(page) = self.pages.get_mut(&index) {
                    page.last_access = access;
                }
                self.access_order.push_back((access, index));
            }
            self.access_clock = self.pages.len() as u64;
        }
        self.access_clock += 1;
        self.access_clock
    }
}

fn pop_pending_page(state: &mut RangeState) -> Option<u64> {
    while let Some(page) = state.pending.pop_front() {
        if state.pending_pages.remove(&page)
            && !state.in_flight_pages.contains(&page)
            && !state.pages.contains_key(&page)
        {
            return Some(page);
        }
    }
    None
}

fn queue_missing_page(state: &mut RangeState, page_index: u64) {
    if state
        .pending_pages
        .len()
        .saturating_add(state.in_flight_pages.len())
        < MAX_PENDING_RANGE_PAGES
        && !state.pending_pages.contains(&page_index)
        && !state.in_flight_pages.contains(&page_index)
        && !state.pages.contains_key(&page_index)
    {
        state.pending_pages.insert(page_index);
        state.pending.push_back(page_index);
    }
}
