# WASM platform seams

This is an architecture audit, not a claim that OpenStoryRS currently runs in a
browser. The immediate delivery target remains native macOS; this document records
the existing portable contracts, a new asset-delivery adapter seam, and the work
still required before browser deployment.

## Asset delivery seam

`openstory-assets` exposes `RandomAccessSource`, a bounded random-read contract
(`crates/openstory-assets/src/nx.rs:119-128`). Native `FileSource` is compiled only
outside `wasm32` (`nx.rs:377-419`), while `NxArchive::from_source` accepts the shared
contract (`nx.rs:790-796`). `MemorySource` already supports fully staged archives
(`nx.rs:130-160`), and `CachedSource` bounds cached source blocks (`nx.rs:172-294`).

`RangeBackedSource` extends that boundary for large archives that should not be
downloaded or retained as one `MemorySource`:

- It stores only requested pages, with a configurable LRU byte cap and a bounded
  pending-plus-in-flight request queue (`crates/openstory-assets/src/range_source.rs`).
- A missing read returns `io::ErrorKind::WouldBlock` and records the page request.
  The host drains/coalesces requests with `take_requests`, fetches the ranges using
  its async API, and supplies the exact response through `provide_range`.
- `request_range` supports prefetching the first page before calling
  `NxArchive::from_source`, because the NX archive header must be present at open.
- On a failed fetch, the host calls `release_request`; after retry/backoff, a later
  read can enqueue the missing page again.
- Range alignment, exact response length, conflicting overlap with cached pages,
  cache size, and outstanding work are bounded/validated. The source itself contains
  no `web_sys`, JS values, fetch implementation, filesystem API, or network client.

The current NX decoder is synchronous. `NxArchive::from_source` returns an error if
its initial header read is missing, so a browser host must preload that header page
and retry construction. For subsequent misses, the host must service requests and
retry the relevant lazy node/decode operation. An async archive API or a suspending
asset task queue has not been implemented.

The asset archive set is large (the existing local NX files total about 3.8 GB, per
`PORT_PLAN.md:18-20`). Browser delivery still needs a policy for selecting permitted
archives, hosting them, CORS and HTTP Range behavior, cache persistence, and bandwidth
limits. The new source is only the reusable random-access/cache boundary.

## Existing portable contracts and current blockers

- `openstory-protocol` exports platform-neutral `SessionEvent` and `SessionTransport`
  (`crates/openstory-protocol/src/session.rs:1-31`, re-exported in `lib.rs:202`). Core
  and protocol sources have no `TcpStream`, filesystem, or thread dependencies;
  protocol uses `Ipv4Addr` as a packet data type (`protocol/src/login.rs:154-161`).
- The native transport is isolated in `crates/openstory-client/src/session.rs:1-18`
  and directly uses TCP sockets, OS threads, and standard channels. Its comments
  already identify the adapter replacement point.
- The current client module and native renderer are both gated out on `wasm32`
  (`crates/openstory-client/src/main.rs:3-7`, `crates/openstory-render/src/lib.rs:5-9`).
  The client has only a browser placeholder entry point (`main.rs:51012-51015`).
- `openstory-client/Cargo.toml:19-27` separates native-only window/presentation/audio
  dependencies from target-independent crates. Browser-compatible equivalents or
  adapters for audio, persistent settings, browser canvas/window lifecycle, image
  conversion, and logging remain to be designed and wired.
- `AssetDirectory` still resolves paths through environment variables, current
  directories, executable paths, and `std::fs` (`crates/openstory-assets/src/directory.rs:1-6,91-120`).
  The browser host must provide an archive manifest and construct sources without
  calling that native directory resolver.
- `NxArchive::open` remains native-only (`crates/openstory-assets/src/nx.rs:780-788`).
  Browser composition should use `from_source` with a preloaded `RangeBackedSource`.

## Next browser integration steps

1. Add a wasm entry point that owns a canvas/window lifecycle and can retry pending
   asset reads without blocking the browser event loop.
2. Add an HTTP Range/fetch provider that validates server `Content-Range` and asset
   identity before calling `provide_range`; select only the NX archives needed for
   the requested v83 experience and persist fetched pages where browser storage
   permits.
3. Implement a WebSocket gateway for the v83 login/channel TCP protocol. It must
   preserve the Maple handshake/cipher, channel redirect flow, framing, and packet
   ordering. The `SessionTransport` contract is already the shared seam; no gateway
   or browser transport exists yet.
4. Share renderer and input logic behind browser-capable APIs, supply browser audio
   and storage adapters, and move the client composition currently inside the native
   `main.rs` module into platform-neutral orchestration.
5. Once those adapters exist, compile and exercise the browser target. This work
   deliberately does not build or test WASM while the macOS target is being delivered.
