//! Legacy v83 character-selection identifiers for macOS.
//!
//! The old client sends a MAC string and a derived HWID when selecting a
//! character or handling a PIC. Explicit environment overrides remain
//! available for servers that require configured values.

#[cfg(target_os = "macos")]
use std::process::Command;

#[derive(Debug)]
pub(super) struct HardwareInfo {
    pub(super) macs: String,
    pub(super) hwid: String,
}

pub(super) fn legacy_hardware_info() -> HardwareInfo {
    #[cfg(target_os = "macos")]
    {
        return macos_hardware_info();
    }

    #[cfg(not(target_os = "macos"))]
    {
        HardwareInfo {
            macs: String::new(),
            hwid: String::new(),
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_hardware_info() -> HardwareInfo {
    let host_uuid = read_host_uuid();
    let host_hash = host_uuid
        .as_deref()
        .map(fnv1a)
        .filter(|hash| *hash != 0)
        .unwrap_or_else(|| {
            Command::new("/bin/hostname")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| fnv1a(&output.stdout))
                .unwrap_or(0)
        });

    let mac = read_primary_mac().unwrap_or_else(|| {
        [
            0x02,
            0x00,
            (host_hash >> 24) as u8,
            (host_hash >> 16) as u8,
            (host_hash >> 8) as u8,
            host_hash as u8,
        ]
    });

    let macs = format_mac(&mac, '-');
    let mac_hwid = format_mac(&mac, '\0').replace('\0', "");
    let serial = if host_hash == 0 {
        fnv1a(&mac)
    } else {
        host_hash
    };
    let serial = format!("{serial:08X}");
    let reversed_serial = format!(
        "{}{}{}{}",
        &serial[6..8],
        &serial[4..6],
        &serial[2..4],
        &serial[0..2]
    );

    HardwareInfo {
        macs,
        hwid: format!("{mac_hwid}_{reversed_serial}"),
    }
}

#[cfg(target_os = "macos")]
fn read_primary_mac() -> Option<[u8; 6]> {
    let output = Command::new("/sbin/ifconfig").arg("-a").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_interface = "";
    let mut first_mac = None;
    for line in text.lines() {
        if !line.starts_with([' ', '\t'])
            && let Some((interface, _)) = line.split_once(':')
        {
            current_interface = interface.trim();
        }
        if current_interface == "lo0" || !line.trim_start().starts_with("ether ") {
            continue;
        }
        let Some(mac) = parse_mac(line.trim_start().trim_start_matches("ether ")) else {
            continue;
        };
        if current_interface == "en0" {
            return Some(mac);
        }
        first_mac.get_or_insert(mac);
    }
    first_mac
}

#[cfg(target_os = "macos")]
fn read_host_uuid() -> Option<Vec<u8>> {
    let output = Command::new("/usr/sbin/ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let uuid = text
        .lines()
        .find(|line| line.contains("IOPlatformUUID"))?
        .split('"')
        .nth(3)?;
    let bytes = uuid
        .bytes()
        .filter(|byte| *byte != b'-')
        .collect::<Vec<_>>();
    if bytes.len() != 32 {
        return None;
    }
    let mut decoded = Vec::with_capacity(16);
    for pair in bytes.chunks_exact(2) {
        decoded.push((hex_value(pair[0])? << 4) | hex_value(pair[1])?);
    }
    Some(decoded)
}

#[cfg(target_os = "macos")]
fn parse_mac(value: &str) -> Option<[u8; 6]> {
    let bytes = value
        .trim()
        .split(':')
        .map(|part| u8::from_str_radix(part, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    let mac: [u8; 6] = bytes.try_into().ok()?;
    (mac != [0; 6]).then_some(mac)
}

#[cfg(target_os = "macos")]
fn format_mac(mac: &[u8; 6], separator: char) -> String {
    mac.iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(&separator.to_string())
}

#[cfg(target_os = "macos")]
fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(16_777_619)
    })
}

#[cfg(target_os = "macos")]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
