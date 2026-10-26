use crate::{HEADER_LENGTH, PacketError, decode_plain_header};

/// Reassembles the legacy four-byte length header and packet body across arbitrary
/// transport reads, returning every complete body in stream order.
#[derive(Clone, Debug, Default)]
pub struct FrameDecoder {
    header: [u8; HEADER_LENGTH],
    header_length: usize,
    expected_body_length: Option<usize>,
    body: Vec<u8>,
}

impl FrameDecoder {
    /// Adds a stream chunk and emits all complete packet bodies it contains.
    /// Incomplete headers and bodies stay buffered for the next call.
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

                match decode_plain_header(self.header) {
                    Ok(length) => self.expected_body_length = Some(length),
                    Err(error) => {
                        self.reset();
                        return Err(error);
                    }
                }
            }

            let expected = self
                .expected_body_length
                .expect("body length set after a complete header");
            let count = (expected - self.body.len()).min(input.len());
            self.body.extend_from_slice(&input[..count]);
            input = &input[count..];

            if self.body.len() == expected {
                packets.push(std::mem::take(&mut self.body));
                self.expected_body_length = None;
                self.header_length = 0;
            }
        }

        Ok(packets)
    }

    /// Drops any partial header or body, for example after reconnecting a socket.
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
