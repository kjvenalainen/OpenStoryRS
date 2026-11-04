# Finish OpenStoryRS Backend Parity

## Summary

Bring the Rust client's non-UI behavior to parity with the current OpenStory C++
client and its v83 server contract. Use native macOS as the first runtime
verification target; keep browser delivery and its gateway out of scope.

The current implementation already covers NX reading, encrypted sessions, login,
physics, inventory, quests, combat, map actors, and many social and commerce
packets. It is not yet complete: combat and status behavior have known gaps, some
original packet families—including MTS and bot inventory—need parity assessment,
and many implemented flows have not been verified against a live server. Gameplay
state also remains largely in the native client instead of the core crate.

## Implementation

1. **Establish a parity inventory.** Compare the original's registered incoming
   handlers, outgoing packet builders, gameplay, character, and data systems
   against Rust. Record each behavior as implemented and verified, implemented
   but unverified, missing, intentionally ignored by the original, or dependent
   on a server contract. Track packet variants and state transitions, including
   MTS and bot inventory, rather than counting opcode names alone.
2. **Complete protocol and session behavior.** Port missing original handler
   variants and request builders into `openstory-protocol`; apply their events to
   state without requiring new screens. Cover login, redirects, channel changes,
   reconnects, field entry, inventory, quests, social systems, shops, storage,
   cash operations, MTS, and bot inventory as identified by the inventory. Keep
   packet reads bounded and reject malformed input without partial state changes
   or panics.
3. **Complete gameplay and data parity.** Port remaining original behavior for
   character state, movement and map transitions, actors and controllers, items
   and equipment, skills, buffs, diseases, combat formulas and mitigation,
   targeting, pets, and quest state. Preserve server-authoritative inventory,
   HP, and transaction results. Compare the existing documented fixes with the
   actual v83 wire format instead of reintroducing known C++ defects.
4. **Move backend ownership into the appropriate crates.** Incrementally move
   platform-independent state, simulation, and rules from the native client into
   `openstory-core`; keep NX access in `openstory-assets`, serialization and
   transport-independent events in `openstory-protocol`, and native transport and
   application composition in `openstory-client`. Expose typed commands and events
   for existing UI consumers without redesigning screens or changing the logical
   canvas.
5. **Resolve compatibility dependencies explicitly.** Validate server-dependent
   behavior against the supplied v83 server and its scripts. Keep the documented
   token-for-mesos shop operation disabled until its server-side contract is
   corrected or verified; do not fabricate client behavior for missing server
   capabilities. Record any remaining server limitation with the exact affected
   flow.

## Verification and completion

- Add focused protocol fixtures for each newly supported packet variant,
  including truncated input, invalid lengths, unknown variants, and
  representative outgoing bytes.
- Add deterministic core tests for movement, state transitions, combat, buffs,
  item restrictions, and authoritative updates; compare representative cases
  with the original and NX data.
- Exercise native end-to-end flows: login through map entry, channel transfer,
  movement, combat, inventory and item use, quests, social actions, and
  supported shop/storage transactions. Verify failure and disconnect paths as
  well as successful responses.
- Run `cargo fmt --all -- --check`, `cargo check --workspace`, and
  `cargo test --workspace`; verify the supported native build. Update the port
  notes with evidence and remaining server-dependent limitations.
- Consider the backend complete when every original non-UI behavior in the
  inventory is either verified or explicitly documented as an original no-op or
  unavailable server capability, and all applicable checks pass.

## Assumptions

- “Backend” means client-side protocol, asset access, domain state, and
  simulation; the server itself and UI presentation are outside the implementation
  scope.
- Existing UI may consume newly exposed state and events, but no new screens or
  visual parity work are required.
- Read NX assets from the existing data directory and treat the adjacent OpenStory
  checkout as read-only.
