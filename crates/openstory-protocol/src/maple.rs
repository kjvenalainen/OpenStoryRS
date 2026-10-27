use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};

use crate::{HEADER_LENGTH, MIN_PACKET_BODY_LENGTH, PacketError};

const MAPLE_VERSION: u16 = 83;
const AES_KEY: [u8; 32] = [
    0x13, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0xB4, 0x00, 0x00, 0x00,
    0x1B, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x33, 0x00, 0x00, 0x00, 0x52, 0x00, 0x00, 0x00,
];

/// v83 encrypted frame bodies are limited by their 16-bit length XOR header.
pub const MAX_ENCRYPTED_PACKET_LENGTH: usize = u16::MAX as usize;

/// Reassembles encrypted v83 frames across arbitrary TCP read boundaries.
pub struct MapleFrameDecoder {
    cipher: MapleCipher,
    header: [u8; HEADER_LENGTH],
    header_length: usize,
    expected_body_length: Option<usize>,
    body: Vec<u8>,
}

impl MapleFrameDecoder {
    pub fn new(cipher: MapleCipher) -> Self {
        Self {
            cipher,
            header: [0; HEADER_LENGTH],
            header_length: 0,
            expected_body_length: None,
            body: Vec::new(),
        }
    }

    pub fn push(&mut self, mut input: &[u8]) -> Result<Vec<Vec<u8>>, PacketError> {
        let mut packets = Vec::new();
        while !input.is_empty() {
            if self.expected_body_length.is_none() {
                let count = (HEADER_LENGTH - self.header_length).min(input.len());
                let end = self.header_length + count;
                self.header[self.header_length..end].copy_from_slice(&input[..count]);
                self.header_length = end;
                input = &input[count..];
                if self.header_length < HEADER_LENGTH {
                    break;
                }

                match self.cipher.decode_header(self.header) {
                    Ok(length) => self.expected_body_length = Some(length),
                    Err(error) => {
                        self.reset();
                        return Err(error);
                    }
                }
            }

            let expected = self
                .expected_body_length
                .expect("body length set after a complete encrypted header");
            let count = (expected - self.body.len()).min(input.len());
            self.body.extend_from_slice(&input[..count]);
            input = &input[count..];
            if self.body.len() == expected {
                self.cipher.decrypt_body(&mut self.body);
                packets.push(std::mem::take(&mut self.body));
                self.expected_body_length = None;
                self.header_length = 0;
            }
        }
        Ok(packets)
    }

    pub fn encode_frame(&mut self, body: &[u8]) -> Result<Vec<u8>, PacketError> {
        self.cipher.encode_frame(body)
    }

    pub fn reset(&mut self) {
        self.header = [0; HEADER_LENGTH];
        self.header_length = 0;
        self.expected_body_length = None;
        self.body.clear();
    }

    pub fn has_partial_packet(&self) -> bool {
        self.header_length != 0 || self.expected_body_length.is_some()
    }
}

/// Stateful v83 MapleStory cipher. Send and receive IVs advance independently.
/// A value belongs to one connection and must not be shared between sessions.
pub struct MapleCipher {
    send_iv: [u8; 4],
    receive_iv: [u8; 4],
    aes: Aes256,
}

impl MapleCipher {
    /// Reads the v83 IVs from the standard 16-byte handshake prefix.
    pub fn from_handshake(handshake: &[u8]) -> Result<Self, PacketError> {
        if handshake.len() < 16 {
            return Err(PacketError::InvalidHandshakeLength(handshake.len()));
        }
        let send_iv = handshake[7..11]
            .try_into()
            .expect("handshake length checked");
        let receive_iv = handshake[11..15]
            .try_into()
            .expect("handshake length checked");
        Ok(Self::from_ivs(send_iv, receive_iv))
    }

    pub fn from_ivs(send_iv: [u8; 4], receive_iv: [u8; 4]) -> Self {
        let aes = Aes256::new_from_slice(&AES_KEY).expect("Maple AES key has a fixed valid length");
        Self {
            send_iv,
            receive_iv,
            aes,
        }
    }

    pub fn send_iv(&self) -> [u8; 4] {
        self.send_iv
    }

    pub fn receive_iv(&self) -> [u8; 4] {
        self.receive_iv
    }

    /// Checks a four-byte v83 header against the current receive IV and returns
    /// its bounded encrypted body length. The IV advances only after body decrypt.
    pub fn decode_header(&self, header: [u8; 4]) -> Result<usize, PacketError> {
        let first = u16::from_le_bytes([header[0], header[1]]);
        let expected = receive_version_mask(self.receive_iv);
        if first != expected {
            return Err(PacketError::InvalidEncryptedHeader {
                expected,
                actual: first,
            });
        }

        let second = u16::from_le_bytes([header[2], header[3]]);
        let length = usize::from(first ^ second);
        if !(MIN_PACKET_BODY_LENGTH..=MAX_ENCRYPTED_PACKET_LENGTH).contains(&length) {
            return Err(PacketError::InvalidFrameLength(length));
        }
        Ok(length)
    }

    /// Encrypts one packet body and prefixes its v83 length header.
    pub fn encode_frame(&mut self, body: &[u8]) -> Result<Vec<u8>, PacketError> {
        validate_encrypted_length(body.len())?;
        let first = send_version_mask(self.send_iv);
        let second = first ^ body.len() as u16;
        let mut frame = Vec::with_capacity(4 + body.len());
        frame.extend_from_slice(&first.to_le_bytes());
        frame.extend_from_slice(&second.to_le_bytes());
        frame.extend_from_slice(body);
        self.encrypt_body(&mut frame[4..]);
        Ok(frame)
    }

    /// Decrypts a body after validating that its size matches the header.
    pub fn decode_frame(
        &mut self,
        header: [u8; 4],
        encrypted_body: &[u8],
    ) -> Result<Vec<u8>, PacketError> {
        let expected = self.decode_header(header)?;
        if encrypted_body.len() != expected {
            return Err(PacketError::InvalidFrameLength(encrypted_body.len()));
        }
        let mut body = encrypted_body.to_vec();
        self.decrypt_body(&mut body);
        Ok(body)
    }

    pub fn encrypt_body(&mut self, body: &mut [u8]) {
        maple_encrypt(body);
        aes_ofb(&self.aes, body, &mut self.send_iv);
    }

    pub fn decrypt_body(&mut self, body: &mut [u8]) {
        aes_ofb(&self.aes, body, &mut self.receive_iv);
        maple_decrypt(body);
    }
}

fn validate_encrypted_length(length: usize) -> Result<(), PacketError> {
    if !(MIN_PACKET_BODY_LENGTH..=MAX_ENCRYPTED_PACKET_LENGTH).contains(&length) {
        return Err(PacketError::InvalidFrameLength(length));
    }
    Ok(())
}

fn send_version_mask(iv: [u8; 4]) -> u16 {
    u16::from_le_bytes([iv[2], iv[3]]) ^ MAPLE_VERSION
}

fn receive_version_mask(iv: [u8; 4]) -> u16 {
    // The server writes the header in network byte order and uses the
    // complemented version; after little-endian decoding this is the mask.
    u16::from_le_bytes([iv[2], iv[3]]) ^ (u16::MAX - MAPLE_VERSION)
}

fn maple_encrypt(bytes: &mut [u8]) {
    for _ in 0..3 {
        let mut remember = 0_u8;
        let mut data_len = bytes.len() as u8;
        for byte in bytes.iter_mut() {
            let current = (byte.rotate_left(3).wrapping_add(data_len)) ^ remember;
            remember = current;
            *byte = (!current.rotate_right(u32::from(data_len))).wrapping_add(0x48);
            data_len = data_len.wrapping_sub(1);
        }

        remember = 0;
        data_len = bytes.len() as u8;
        for byte in bytes.iter_mut().rev() {
            let current = (byte.rotate_left(4).wrapping_add(data_len)) ^ remember;
            remember = current;
            *byte = (current ^ 0x13).rotate_right(3);
            data_len = data_len.wrapping_sub(1);
        }
    }
}

fn maple_decrypt(bytes: &mut [u8]) {
    for _ in 0..3 {
        let mut remember = 0_u8;
        let mut data_len = bytes.len() as u8;
        for byte in bytes.iter_mut().rev() {
            let current = byte.rotate_left(3) ^ 0x13;
            *byte = (current ^ remember).wrapping_sub(data_len).rotate_right(4);
            remember = current;
            data_len = data_len.wrapping_sub(1);
        }

        remember = 0;
        data_len = bytes.len() as u8;
        for byte in bytes.iter_mut() {
            let current = (!(byte.wrapping_sub(0x48))).rotate_left(u32::from(data_len));
            *byte = (current ^ remember).wrapping_sub(data_len).rotate_right(3);
            remember = current;
            data_len = data_len.wrapping_sub(1);
        }
    }
}

fn aes_ofb(aes: &Aes256, bytes: &mut [u8], iv: &mut [u8; 4]) {
    let mut block_length = 0x5B0;
    let mut offset = 0;

    while offset < bytes.len() {
        let mut stream = [0; 16];
        for (index, byte) in stream.iter_mut().enumerate() {
            *byte = iv[index % iv.len()];
        }

        let remaining = (bytes.len() - offset).min(block_length);
        for relative in 0..remaining {
            let stream_offset = relative % 16;
            if stream_offset == 0 {
                let mut block = GenericArray::clone_from_slice(&stream);
                aes.encrypt_block(&mut block);
                stream.copy_from_slice(&block);
            }
            bytes[offset + relative] ^= stream[stream_offset];
        }

        offset += block_length;
        block_length = 0x5B4;
    }

    update_iv(iv);
}

fn update_iv(iv: &mut [u8; 4]) {
    let mut mixed: [u8; 4] = [0xF2, 0x53, 0x50, 0xC6];
    for &iv_byte in iv.iter() {
        mixed[0] = mixed[0]
            .wrapping_add(MAPLE_BYTES[mixed[1] as usize])
            .wrapping_sub(iv_byte);
        mixed[1] = mixed[1].wrapping_sub(mixed[2] ^ MAPLE_BYTES[iv_byte as usize]);
        mixed[2] ^= MAPLE_BYTES[mixed[3] as usize].wrapping_add(iv_byte);
        mixed[3] = mixed[3]
            .wrapping_add(MAPLE_BYTES[iv_byte as usize])
            .wrapping_sub(mixed[0]);
        mixed = u32::from_le_bytes(mixed).rotate_left(3).to_le_bytes();
    }
    *iv = mixed;
}

const MAPLE_BYTES: [u8; 256] = [
    0xEC, 0x3F, 0x77, 0xA4, 0x45, 0xD0, 0x71, 0xBF, 0xB7, 0x98, 0x20, 0xFC, 0x4B, 0xE9, 0xB3, 0xE1,
    0x5C, 0x22, 0xF7, 0x0C, 0x44, 0x1B, 0x81, 0xBD, 0x63, 0x8D, 0xD4, 0xC3, 0xF2, 0x10, 0x19, 0xE0,
    0xFB, 0xA1, 0x6E, 0x66, 0xEA, 0xAE, 0xD6, 0xCE, 0x06, 0x18, 0x4E, 0xEB, 0x78, 0x95, 0xDB, 0xBA,
    0xB6, 0x42, 0x7A, 0x2A, 0x83, 0x0B, 0x54, 0x67, 0x6D, 0xE8, 0x65, 0xE7, 0x2F, 0x07, 0xF3, 0xAA,
    0x27, 0x7B, 0x85, 0xB0, 0x26, 0xFD, 0x8B, 0xA9, 0xFA, 0xBE, 0xA8, 0xD7, 0xCB, 0xCC, 0x92, 0xDA,
    0xF9, 0x93, 0x60, 0x2D, 0xDD, 0xD2, 0xA2, 0x9B, 0x39, 0x5F, 0x82, 0x21, 0x4C, 0x69, 0xF8, 0x31,
    0x87, 0xEE, 0x8E, 0xAD, 0x8C, 0x6A, 0xBC, 0xB5, 0x6B, 0x59, 0x13, 0xF1, 0x04, 0x00, 0xF6, 0x5A,
    0x35, 0x79, 0x48, 0x8F, 0x15, 0xCD, 0x97, 0x57, 0x12, 0x3E, 0x37, 0xFF, 0x9D, 0x4F, 0x51, 0xF5,
    0xA3, 0x70, 0xBB, 0x14, 0x75, 0xC2, 0xB8, 0x72, 0xC0, 0xED, 0x7D, 0x68, 0xC9, 0x2E, 0x0D, 0x62,
    0x46, 0x17, 0x11, 0x4D, 0x6C, 0xC4, 0x7E, 0x53, 0xC1, 0x25, 0xC7, 0x9A, 0x1C, 0x88, 0x58, 0x2C,
    0x89, 0xDC, 0x02, 0x64, 0x40, 0x01, 0x5D, 0x38, 0xA5, 0xE2, 0xAF, 0x55, 0xD5, 0xEF, 0x1A, 0x7C,
    0xA7, 0x5B, 0xA6, 0x6F, 0x86, 0x9F, 0x73, 0xE6, 0x0A, 0xDE, 0x2B, 0x99, 0x4A, 0x47, 0x9C, 0xDF,
    0x09, 0x76, 0x9E, 0x30, 0x0E, 0xE4, 0xB2, 0x94, 0xA0, 0x3B, 0x34, 0x1D, 0x28, 0x0F, 0x36, 0xE3,
    0x23, 0xB4, 0x03, 0xD8, 0x90, 0xC8, 0x3C, 0xFE, 0x5E, 0x32, 0x24, 0x50, 0x1F, 0x3A, 0x43, 0x8A,
    0x96, 0x41, 0x74, 0xAC, 0x52, 0x33, 0xF0, 0xD9, 0x29, 0x80, 0xB1, 0x16, 0xD3, 0xAB, 0x91, 0xB9,
    0x84, 0x7F, 0x61, 0x1E, 0xCF, 0xC5, 0xD1, 0x56, 0x3D, 0xCA, 0xF4, 0x05, 0xC6, 0xE5, 0x08, 0x49,
];
