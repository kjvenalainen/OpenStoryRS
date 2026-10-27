//! Party Search client requests supported by the Cosmic v83 server.

use crate::packet::PacketWriter;

/// Legacy client register request. Cosmic accepts it as an empty-payload stub;
/// partyless players are made available automatically by the server.
pub const OPCODE_PARTY_SEARCH_REGISTER: u16 = 220;

/// Starts leader-driven matching. Matching candidates receive regular party invites.
pub const OPCODE_PARTY_SEARCH_START: u16 = 222;

/// Cosmic's documented maximum difference between the minimum and maximum levels.
pub const MAX_PARTY_SEARCH_LEVEL_SPAN: i32 = 30;
pub const MAX_PARTY_MEMBERS: i32 = 6;
pub const ALL_PARTY_SEARCH_JOBS: i32 = 0x03FF_FFFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartySearchCriteria {
    pub min_level: i32,
    pub max_level: i32,
    pub members: i32,
    pub jobs_mask: i32,
}

impl PartySearchCriteria {
    /// Makes the default ±10 level criteria around the local player's level.
    pub fn around_level(level: i32) -> Self {
        let level = level.clamp(1, 200);
        Self {
            min_level: (level - 10).max(1),
            max_level: (level + 10).min(200),
            members: MAX_PARTY_MEMBERS,
            jobs_mask: ALL_PARTY_SEARCH_JOBS,
        }
    }

    /// Clamps a user-entered range to valid level limits and the server's 30-level span.
    pub fn normalized(min_level: i32, max_level: i32, members: i32, jobs_mask: i32) -> Self {
        let min_level = min_level.clamp(1, 200);
        let max_level = max_level.clamp(
            min_level,
            (min_level + MAX_PARTY_SEARCH_LEVEL_SPAN).min(200),
        );
        Self {
            min_level,
            max_level,
            members: members.clamp(1, MAX_PARTY_MEMBERS),
            jobs_mask,
        }
    }
}

/// Encodes the legacy register request. This is a no-op on Cosmic v83, but is
/// retained for server forks that still implement the operation.
pub fn party_search_register_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_PARTY_SEARCH_REGISTER).into_bytes()
}

/// Encodes a leader's level/member/job filters for the v83 party matcher.
pub fn party_search_start_packet(criteria: PartySearchCriteria) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_PARTY_SEARCH_START);
    packet.write_i32(criteria.min_level);
    packet.write_i32(criteria.max_level);
    packet.write_i32(criteria.members);
    packet.write_i32(criteria.jobs_mask);
    packet.into_bytes()
}
