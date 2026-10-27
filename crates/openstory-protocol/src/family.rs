//! Bounded v83 family information, pedigree, and request packets.

use crate::{PacketError, PacketReader, PacketWriter};

pub const OPCODE_FAMILY_CHART_RESULT: u16 = 94;
pub const OPCODE_FAMILY_INFO: u16 = 95;
pub const OPCODE_FAMILY_RESULT: u16 = 96;
pub const OPCODE_FAMILY_JOIN_REQUEST: u16 = 97;
pub const OPCODE_FAMILY_JOIN_REQUEST_RESULT: u16 = 98;
pub const OPCODE_FAMILY_JOIN_ACCEPTED: u16 = 99;
pub const OPCODE_FAMILY_PRIVILEGE_LIST: u16 = 100;
pub const OPCODE_FAMILY_REP_GAIN: u16 = 101;
pub const OPCODE_FAMILY_NOTIFY_LOGIN: u16 = 102;
pub const OPCODE_FAMILY_SUMMON_REQUEST: u16 = 104;

pub const OPCODE_OPEN_FAMILY_PEDIGREE: u16 = 145;
pub const OPCODE_OPEN_FAMILY: u16 = 146;
pub const OPCODE_ADD_FAMILY: u16 = 147;
pub const OPCODE_SEPARATE_FAMILY_BY_SENIOR: u16 = 148;
pub const OPCODE_SEPARATE_FAMILY_BY_JUNIOR: u16 = 149;
pub const OPCODE_ACCEPT_FAMILY: u16 = 150;
pub const OPCODE_USE_FAMILY: u16 = 151;
pub const OPCODE_CHANGE_FAMILY_MESSAGE: u16 = 152;
pub const OPCODE_FAMILY_SUMMON_RESPONSE: u16 = 153;

const MAX_FAMILY_ENTRIES: usize = 100;
const MAX_FAMILY_ENTITLEMENTS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyEntitlementUsage {
    pub ordinal: i32,
    pub used: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyInfo {
    pub current_reputation: i32,
    pub total_reputation: i32,
    pub todays_reputation: i32,
    pub junior_count: i16,
    pub juniors_allowed: i16,
    pub unknown: i16,
    pub leader_id: i32,
    pub name: String,
    pub message: String,
    pub entitlement_usage: Vec<FamilyEntitlementUsage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyChartEntry {
    pub character_id: i32,
    pub parent_id: i32,
    pub job: i16,
    pub level: i8,
    pub online: bool,
    pub current_reputation: i32,
    pub total_reputation: i32,
    pub reputation_to_senior: i32,
    pub todays_reputation: i32,
    pub channel: i32,
    pub time_online: i32,
    pub name: String,
}

/// Two opaque integer fields retained from the legacy pedigree trailer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyChartCounter {
    pub id_or_flag: i32,
    pub count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyChart {
    pub viewed_character_id: i32,
    pub entries: Vec<FamilyChartEntry>,
    pub member_counters: Vec<FamilyChartCounter>,
    pub entitlement_usage: Vec<FamilyEntitlementUsage>,
    pub add_button: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyPrivilege {
    pub kind: u8,
    pub reputation_cost: i32,
    pub usage_limit: i32,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FamilyEvent {
    Info(FamilyInfo),
    Chart(FamilyChart),
    Result { result_code: i32, mesos: i32 },
    JoinRequest { character_id: i32, name: String },
    JoinRequestResult { accepted: bool, name: String },
    Joined { name: String, status: i32 },
    Privileges(Vec<FamilyPrivilege>),
    ReputationGained { amount: i32, from: String },
    MemberPresenceChanged { logged_in: bool, name: String },
    SummonRequest { from: String, family_name: String },
}

/// Parses the legacy family's receive opcodes. Unrecognized opcodes return
/// `Ok(None)` so callers can share dispatch code with other social packets.
pub fn parse_family_packet(bytes: &[u8]) -> Result<Option<FamilyEvent>, PacketError> {
    let mut packet = PacketReader::new(bytes);
    let opcode = packet.read_u16()?;
    let event = match opcode {
        OPCODE_FAMILY_CHART_RESULT => Some(FamilyEvent::Chart(parse_family_chart(&mut packet)?)),
        OPCODE_FAMILY_INFO => Some(FamilyEvent::Info(parse_family_info(&mut packet)?)),
        OPCODE_FAMILY_RESULT => Some(FamilyEvent::Result {
            result_code: packet.read_i32()?,
            mesos: packet.read_i32()?,
        }),
        OPCODE_FAMILY_JOIN_REQUEST => Some(FamilyEvent::JoinRequest {
            character_id: packet.read_i32()?,
            name: packet.read_string()?,
        }),
        OPCODE_FAMILY_JOIN_REQUEST_RESULT => Some(FamilyEvent::JoinRequestResult {
            // The legacy handler treats every nonzero byte as accepted.
            accepted: packet.read_u8()? != 0,
            name: packet.read_string()?,
        }),
        OPCODE_FAMILY_JOIN_ACCEPTED => Some(FamilyEvent::Joined {
            name: packet.read_string()?,
            status: packet.read_i32()?,
        }),
        OPCODE_FAMILY_PRIVILEGE_LIST => {
            Some(FamilyEvent::Privileges(parse_privileges(&mut packet)?))
        }
        OPCODE_FAMILY_REP_GAIN => Some(FamilyEvent::ReputationGained {
            amount: packet.read_i32()?,
            from: packet.read_string()?,
        }),
        OPCODE_FAMILY_NOTIFY_LOGIN => Some(FamilyEvent::MemberPresenceChanged {
            // InPacket::read_bool() is true only for the exact byte value 1.
            logged_in: packet.read_bool()?,
            name: packet.read_string()?,
        }),
        OPCODE_FAMILY_SUMMON_REQUEST => Some(FamilyEvent::SummonRequest {
            from: packet.read_string()?,
            family_name: packet.read_string()?,
        }),
        _ => None,
    };
    Ok(event)
}

fn parse_family_info(packet: &mut PacketReader<'_>) -> Result<FamilyInfo, PacketError> {
    let current_reputation = packet.read_i32()?;
    let total_reputation = packet.read_i32()?;
    let todays_reputation = packet.read_i32()?;
    let junior_count = packet.read_i16()?;
    let juniors_allowed = packet.read_i16()?;
    let unknown = packet.read_i16()?;
    let leader_id = packet.read_i32()?;
    let name = packet.read_string()?;
    let message = packet.read_string()?;
    let entitlement_usage = if packet.is_empty() {
        Vec::new()
    } else {
        read_entitlement_usage(packet, MAX_FAMILY_ENTITLEMENTS)?
    };
    Ok(FamilyInfo {
        current_reputation,
        total_reputation,
        todays_reputation,
        junior_count,
        juniors_allowed,
        unknown,
        leader_id,
        name,
        message,
        entitlement_usage,
    })
}

fn parse_family_chart(packet: &mut PacketReader<'_>) -> Result<FamilyChart, PacketError> {
    let viewed_character_id = packet.read_i32()?;
    let count = read_family_count(packet, "chart entries", MAX_FAMILY_ENTRIES)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(FamilyChartEntry {
            character_id: packet.read_i32()?,
            parent_id: packet.read_i32()?,
            job: packet.read_i16()?,
            level: packet.read_i8()?,
            online: packet.read_bool()?,
            current_reputation: packet.read_i32()?,
            total_reputation: packet.read_i32()?,
            reputation_to_senior: packet.read_i32()?,
            todays_reputation: packet.read_i32()?,
            channel: packet.read_i32()?,
            time_online: packet.read_i32()?,
            name: packet.read_string()?,
        });
    }

    let mut chart = FamilyChart {
        viewed_character_id,
        entries,
        member_counters: Vec::new(),
        entitlement_usage: Vec::new(),
        add_button: None,
    };

    // Cosmic appends optional UI metadata after the pedigree entries. Older
    // v83-compatible servers may stop after the entries, so retain its absence.
    if !packet.is_empty() {
        let count = read_family_count(packet, "chart member counters", MAX_FAMILY_ENTRIES)?;
        chart.member_counters.reserve(count);
        for _ in 0..count {
            chart.member_counters.push(FamilyChartCounter {
                id_or_flag: packet.read_i32()?,
                count: packet.read_i32()?,
            });
        }
        if !packet.is_empty() {
            chart.entitlement_usage = read_entitlement_usage(packet, MAX_FAMILY_ENTITLEMENTS)?;
        }
        if !packet.is_empty() {
            chart.add_button = Some(packet.read_i16()?);
        }
    }
    Ok(chart)
}

fn parse_privileges(packet: &mut PacketReader<'_>) -> Result<Vec<FamilyPrivilege>, PacketError> {
    let count = read_family_count(packet, "privileges", MAX_FAMILY_ENTITLEMENTS)?;
    let mut privileges = Vec::with_capacity(count);
    for _ in 0..count {
        privileges.push(FamilyPrivilege {
            kind: packet.read_u8()?,
            reputation_cost: packet.read_i32()?,
            usage_limit: packet.read_i32()?,
            name: packet.read_string()?,
            description: packet.read_string()?,
        });
    }
    Ok(privileges)
}

fn read_entitlement_usage(
    packet: &mut PacketReader<'_>,
    maximum: usize,
) -> Result<Vec<FamilyEntitlementUsage>, PacketError> {
    let count = read_family_count(packet, "entitlement usage", maximum)?;
    let mut usages = Vec::with_capacity(count);
    for _ in 0..count {
        usages.push(FamilyEntitlementUsage {
            ordinal: packet.read_i32()?,
            used: packet.read_i32()?,
        });
    }
    Ok(usages)
}

fn read_family_count(
    packet: &mut PacketReader<'_>,
    section: &'static str,
    maximum: usize,
) -> Result<usize, PacketError> {
    let count = packet.read_i32()?;
    if count < 0 || count as usize > maximum {
        return Err(PacketError::InvalidFamilyCount { section, count });
    }
    Ok(count as usize)
}

/// Requests the local character's family information (`OPEN_FAMILY`).
pub fn open_family_packet() -> Vec<u8> {
    PacketWriter::new(OPCODE_OPEN_FAMILY).into_bytes()
}

/// Requests the named character's pedigree chart (`OPEN_FAMILY_PEDIGREE`).
pub fn family_pedigree_request_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_OPEN_FAMILY_PEDIGREE);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

/// Invites a named character to become a junior (`ADD_FAMILY`).
pub fn family_add_junior_packet(name: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_ADD_FAMILY);
    packet.write_string(name)?;
    Ok(packet.into_bytes())
}

/// Accepts a join request from the specified character (`ACCEPT_FAMILY`).
pub fn family_accept_request_packet(inviter_id: i32) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_ACCEPT_FAMILY);
    packet.write_i32(inviter_id);
    packet.write_string("")?;
    packet.write_u8(1);
    Ok(packet.into_bytes())
}

/// Leaves the current family as a junior (`SEPARATE_FAMILY_BY_JUNIOR`).
pub fn family_leave_packet(entry_id: i32) -> Vec<u8> {
    let mut packet = PacketWriter::new(OPCODE_SEPARATE_FAMILY_BY_JUNIOR);
    packet.write_i32(entry_id);
    packet.into_bytes()
}

/// Uses a family ability. Teleport and summon abilities include a target name;
/// reputation abilities omit it, matching the legacy handler's payload shape.
pub fn family_use_packet(
    ability_type: i32,
    target_name: Option<&str>,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_USE_FAMILY);
    packet.write_i32(ability_type);
    if let Some(name) = target_name {
        packet.write_string(name)?;
    }
    Ok(packet.into_bytes())
}

/// Updates the family motto / precepts (`CHANGE_FAMILY_MESSAGE`).
pub fn family_precepts_packet(message: &str) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_CHANGE_FAMILY_MESSAGE);
    packet.write_string(message)?;
    Ok(packet.into_bytes())
}

/// Accepts or declines a family summon (`FAMILY_SUMMON_RESPONSE`).
pub fn family_summon_response_packet(
    family_name: &str,
    accept: bool,
) -> Result<Vec<u8>, PacketError> {
    let mut packet = PacketWriter::new(OPCODE_FAMILY_SUMMON_RESPONSE);
    packet.write_string(family_name)?;
    packet.write_bool(accept);
    Ok(packet.into_bytes())
}
