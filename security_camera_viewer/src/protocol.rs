use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Cursor};

/// MJPEG Protocol Constants
pub const SYNC_WORD: u32 = 0xCAFEBABE;
pub const MJPEG_HEADER_SIZE: usize = 12; // sync_word(4) + sequence(4) + jpeg_size(4)
pub const CRC_SIZE: usize = 2;
pub const MIN_PACKET_SIZE: usize = MJPEG_HEADER_SIZE + CRC_SIZE; // 14 bytes

/// Metrics Protocol Constants (Phase 4.1 extension)
pub const METRICS_SYNC_WORD: u32 = 0xCAFEBEEF;
pub const METRICS_PACKET_SIZE: usize = 58; // Total size including CRC (Phase 9.2: +8 bytes for health metrics)

/// Batch Protocol Constants (Phase 7.2a: Multi-frame batching)
pub const MJPEG_BATCH_SYNC_WORD: u32 = 0xCAFEBABF;
pub const BATCH_HEADER_SIZE: usize = 16; // sync(4) + batch_seq(4) + frame_count(4) + total_size(4)
pub const FRAME_META_SIZE: usize = 8;    // frame_seq(4) + frame_size(4)

/// MJPEG Packet Header (12 bytes)
#[derive(Debug, Clone)]
pub struct MjpegHeader {
    pub sync_word: u32,      // 0xCAFEBABE
    pub sequence: u32,       // Frame sequence number
    pub jpeg_size: u32,      // JPEG data size in bytes
}

impl MjpegHeader {
    /// Parse MJPEG header from buffer
    pub fn parse(buf: &[u8]) -> io::Result<Self> {
        if buf.len() < MJPEG_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("Buffer too small for MJPEG header: {} bytes", buf.len()),
            ));
        }

        let mut cursor = Cursor::new(buf);

        let sync_word = cursor.read_u32::<LittleEndian>()?;
        if sync_word != SYNC_WORD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid sync word: 0x{:08X}, expected 0x{:08X}",
                        sync_word, SYNC_WORD),
            ));
        }

        let sequence = cursor.read_u32::<LittleEndian>()?;
        let jpeg_size = cursor.read_u32::<LittleEndian>()?;

        // Validate JPEG size (max 512 KB as per spec)
        if jpeg_size > 524288 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JPEG size too large: {} bytes (max 512 KB)", jpeg_size),
            ));
        }

        Ok(MjpegHeader {
            sync_word,
            sequence,
            jpeg_size,
        })
    }

    /// Get total packet size (header + JPEG data + CRC)
    pub fn total_size(&self) -> usize {
        MJPEG_HEADER_SIZE + self.jpeg_size as usize + CRC_SIZE
    }
}

/// Complete MJPEG Packet
#[derive(Debug, Clone)]
pub struct MjpegPacket {
    pub header: MjpegHeader,
    pub jpeg_data: Vec<u8>,
    pub crc16: u16,
}

impl MjpegPacket {
    /// Parse MJPEG packet from buffer
    pub fn parse(buf: &[u8]) -> io::Result<Self> {
        // Parse header
        let header = MjpegHeader::parse(buf)?;

        let total_size = header.total_size();
        if buf.len() < total_size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("Buffer too small for complete packet: {} bytes, need {} bytes",
                        buf.len(), total_size),
            ));
        }

        // Extract JPEG data
        let jpeg_start = MJPEG_HEADER_SIZE;
        let jpeg_end = jpeg_start + header.jpeg_size as usize;
        let jpeg_data = buf[jpeg_start..jpeg_end].to_vec();

        // Extract CRC16
        let crc_offset = jpeg_end;
        let mut crc_cursor = Cursor::new(&buf[crc_offset..]);
        let crc16 = crc_cursor.read_u16::<LittleEndian>()?;

        // Verify CRC16-CCITT
        let calculated_crc = calculate_crc16_ccitt(&buf[0..jpeg_end]);
        if calculated_crc != crc16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("CRC mismatch: expected 0x{:04X}, got 0x{:04X}",
                        crc16, calculated_crc),
            ));
        }

        Ok(MjpegPacket {
            header,
            jpeg_data,
            crc16,
        })
    }

    /// Verify if JPEG data has valid JPEG markers
    ///
    /// Checks for:
    /// - SOI (Start of Image): 0xFF 0xD8 at start
    /// - EOI (End of Image): 0xFF 0xD9 at end
    ///
    /// Accepts both JFIF format (FF D8 FF E0) and bare JPEG format (FF D8 FF DB)
    pub fn is_valid_jpeg(&self) -> bool {
        if self.jpeg_data.len() < 4 {
            return false;
        }

        // Check for JPEG SOI marker (0xFF 0xD8) at start
        let has_soi = self.jpeg_data[0] == 0xFF && self.jpeg_data[1] == 0xD8;

        // Check for JPEG EOI marker (0xFF 0xD9) at end
        let len = self.jpeg_data.len();
        let has_eoi = len >= 2 &&
                      self.jpeg_data[len - 2] == 0xFF &&
                      self.jpeg_data[len - 1] == 0xD9;

        has_soi && has_eoi
    }
}

/// Metrics Packet (Phase 4.1 extension, Phase 7 TCP stats, Phase 7.3.3 drop stats, Phase 9.2 health, 58 bytes)
#[derive(Debug, Clone)]
pub struct MetricsPacket {
    pub sequence: u32,                  // Metrics packet sequence number
    pub timestamp_ms: u32,              // Spresense uptime in milliseconds
    pub camera_frames: u32,             // Total camera frames captured
    pub usb_packets: u32,               // Total USB packets sent
    pub action_q_depth: u32,            // Current action queue depth (0-3)
    pub avg_packet_size: u32,           // Average MJPEG packet size (bytes)
    pub errors: u32,                    // Total error count
    pub tcp_avg_send_us: u32,           // Average TCP send time (microseconds, Phase 7)
    pub tcp_max_send_us: u32,           // Maximum TCP send time (microseconds, Phase 7)
    pub dropped_frames: u32,            // Total dropped frames (Phase 7.3.3)
    pub drop_events: u32,               // Number of drop events (Phase 7.3.3)
    pub tcp_health_moving_avg_ms: u32,  // TCP health moving average send time (Phase 9.2)
    pub tcp_health_total_spikes: u32,   // TCP health total spike count (Phase 9.2)
}

impl MetricsPacket {
    /// Parse Metrics packet from buffer
    pub fn parse(buf: &[u8]) -> io::Result<Self> {
        if buf.len() < METRICS_PACKET_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("Buffer too small for metrics packet: {} bytes", buf.len()),
            ));
        }

        let mut cursor = Cursor::new(buf);

        // Read and verify sync word
        let sync_word = cursor.read_u32::<LittleEndian>()?;
        if sync_word != METRICS_SYNC_WORD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid metrics sync word: 0x{:08X}, expected 0x{:08X}",
                        sync_word, METRICS_SYNC_WORD),
            ));
        }

        // Read all fields
        let sequence = cursor.read_u32::<LittleEndian>()?;
        let timestamp_ms = cursor.read_u32::<LittleEndian>()?;
        let camera_frames = cursor.read_u32::<LittleEndian>()?;
        let usb_packets = cursor.read_u32::<LittleEndian>()?;
        let action_q_depth = cursor.read_u32::<LittleEndian>()?;
        let avg_packet_size = cursor.read_u32::<LittleEndian>()?;
        let errors = cursor.read_u32::<LittleEndian>()?;
        let tcp_avg_send_us = cursor.read_u32::<LittleEndian>()?;         // Phase 7: TCP stats
        let tcp_max_send_us = cursor.read_u32::<LittleEndian>()?;         // Phase 7: TCP stats
        let dropped_frames = cursor.read_u32::<LittleEndian>()?;          // Phase 7.3.3: Frame drop stats
        let drop_events = cursor.read_u32::<LittleEndian>()?;             // Phase 7.3.3: Frame drop stats
        let tcp_health_moving_avg_ms = cursor.read_u32::<LittleEndian>()?; // Phase 9.2: Health metrics
        let tcp_health_total_spikes = cursor.read_u32::<LittleEndian>()?;  // Phase 9.2: Health metrics
        let crc16 = cursor.read_u16::<LittleEndian>()?;

        // Verify CRC (56 bytes: all fields except crc16 itself)
        let calculated_crc = calculate_crc16_ccitt(&buf[0..56]);
        if calculated_crc != crc16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Metrics CRC mismatch: expected 0x{:04X}, got 0x{:04X}",
                        crc16, calculated_crc),
            ));
        }

        Ok(MetricsPacket {
            sequence,
            timestamp_ms,
            camera_frames,
            usb_packets,
            action_q_depth,
            avg_packet_size,
            errors,
            tcp_avg_send_us,
            tcp_max_send_us,
            dropped_frames,
            drop_events,
            tcp_health_moving_avg_ms,
            tcp_health_total_spikes,
        })
    }
}

/// Frame metadata within batch (Phase 7.2a)
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    pub frame_sequence: u32,  // Individual frame sequence number
    pub frame_size: u32,      // JPEG data size for this frame
}

/// Batch Header (Phase 7.2a: Multi-frame batching, 16 bytes)
#[derive(Debug, Clone)]
pub struct BatchHeader {
    pub sync_word: u32,       // 0xCAFEBABF
    pub batch_sequence: u32,  // Batch sequence number
    pub frame_count: u32,     // Number of frames in this batch (1-3)
    pub total_size: u32,      // Total size of all JPEG data
}

impl BatchHeader {
    /// Parse batch header from buffer
    pub fn parse(buf: &[u8]) -> io::Result<Self> {
        if buf.len() < BATCH_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("Buffer too small for batch header: {} bytes", buf.len()),
            ));
        }

        let mut cursor = Cursor::new(buf);

        let sync_word = cursor.read_u32::<LittleEndian>()?;
        if sync_word != MJPEG_BATCH_SYNC_WORD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid batch sync word: 0x{:08X}, expected 0x{:08X}",
                        sync_word, MJPEG_BATCH_SYNC_WORD),
            ));
        }

        let batch_sequence = cursor.read_u32::<LittleEndian>()?;
        let frame_count = cursor.read_u32::<LittleEndian>()?;
        let total_size = cursor.read_u32::<LittleEndian>()?;

        // Validate frame count (1-3 frames per batch)
        if frame_count == 0 || frame_count > 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid frame count: {} (must be 1-3)", frame_count),
            ));
        }

        // Validate total size
        if total_size > 200_000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Batch total size too large: {} bytes", total_size),
            ));
        }

        Ok(BatchHeader {
            sync_word,
            batch_sequence,
            frame_count,
            total_size,
        })
    }
}

/// Single frame within a batch packet
#[derive(Debug, Clone)]
pub struct BatchFrame {
    pub metadata: FrameMetadata,
    pub jpeg_data: Vec<u8>,
}

/// Complete Batch Packet (Phase 7.2a)
#[derive(Debug, Clone)]
pub struct BatchPacket {
    pub header: BatchHeader,
    pub frames: Vec<BatchFrame>,
    pub crc16: u16,
}

impl BatchPacket {
    /// Parse batch packet from buffer
    pub fn parse(buf: &[u8]) -> io::Result<Self> {
        // Parse header
        let header = BatchHeader::parse(buf)?;

        let mut offset = BATCH_HEADER_SIZE;
        let mut frames = Vec::with_capacity(header.frame_count as usize);

        // Parse each frame
        for _ in 0..header.frame_count {
            // Read frame metadata
            if offset + FRAME_META_SIZE > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Buffer too small for frame metadata",
                ));
            }

            let mut cursor = Cursor::new(&buf[offset..]);
            let frame_sequence = cursor.read_u32::<LittleEndian>()?;
            let frame_size = cursor.read_u32::<LittleEndian>()?;
            offset += FRAME_META_SIZE;

            // Read JPEG data
            if offset + frame_size as usize > buf.len() {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("Buffer too small for JPEG data: {} bytes", frame_size),
                ));
            }

            let jpeg_data = buf[offset..offset + frame_size as usize].to_vec();
            offset += frame_size as usize;

            frames.push(BatchFrame {
                metadata: FrameMetadata {
                    frame_sequence,
                    frame_size,
                },
                jpeg_data,
            });
        }

        // Read CRC
        if offset + CRC_SIZE > buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Buffer too small for CRC",
            ));
        }

        let mut crc_cursor = Cursor::new(&buf[offset..]);
        let crc16 = crc_cursor.read_u16::<LittleEndian>()?;

        // Verify CRC (all data except CRC itself)
        let calculated_crc = calculate_crc16_ccitt(&buf[0..offset]);
        if calculated_crc != crc16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Batch CRC mismatch: expected 0x{:04X}, got 0x{:04X}",
                        crc16, calculated_crc),
            ));
        }

        Ok(BatchPacket {
            header,
            frames,
            crc16,
        })
    }

    /// Get the total packet size
    pub fn total_size(&self) -> usize {
        BATCH_HEADER_SIZE
            + self.frames.len() * FRAME_META_SIZE
            + self.frames.iter().map(|f| f.jpeg_data.len()).sum::<usize>()
            + CRC_SIZE
    }
}

/// Unified Packet type that can be MJPEG, Batch, or Metrics
#[derive(Debug, Clone)]
pub enum Packet {
    Mjpeg(MjpegPacket),
    Batch(BatchPacket),
    Metrics(MetricsPacket),
}

/// Calculate CRC-16-CCITT (Polynomial 0x1021, Initial 0xFFFF)
///
/// This matches the Spresense implementation in the MJPEG protocol.
pub fn calculate_crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_ccitt() {
        // Test with known CRC-16-CCITT values
        let data = b"123456789";
        let crc = calculate_crc16_ccitt(data);
        // Expected CRC-16-CCITT for "123456789" is 0x29B1
        assert_eq!(crc, 0x29B1);
    }

    #[test]
    fn test_sync_word_validation() {
        let mut buf = vec![0u8; MJPEG_HEADER_SIZE];
        let mut cursor = Cursor::new(&mut buf);

        use byteorder::WriteBytesExt;
        cursor.write_u32::<LittleEndian>(SYNC_WORD).unwrap();
        cursor.write_u32::<LittleEndian>(1).unwrap(); // sequence
        cursor.write_u32::<LittleEndian>(100).unwrap(); // jpeg_size

        let header = MjpegHeader::parse(&buf).unwrap();
        assert_eq!(header.sync_word, SYNC_WORD);
        assert_eq!(header.sequence, 1);
        assert_eq!(header.jpeg_size, 100);
    }

    #[test]
    fn test_invalid_sync_word() {
        let mut buf = vec![0u8; MJPEG_HEADER_SIZE];
        let mut cursor = Cursor::new(&mut buf);

        use byteorder::WriteBytesExt;
        cursor.write_u32::<LittleEndian>(0xDEADBEEF).unwrap(); // Wrong sync word

        let result = MjpegHeader::parse(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_jpeg_size_limit() {
        let mut buf = vec![0u8; MJPEG_HEADER_SIZE];
        let mut cursor = Cursor::new(&mut buf);

        use byteorder::WriteBytesExt;
        cursor.write_u32::<LittleEndian>(SYNC_WORD).unwrap();
        cursor.write_u32::<LittleEndian>(1).unwrap();
        cursor.write_u32::<LittleEndian>(1_000_000).unwrap(); // > 512 KB

        let result = MjpegHeader::parse(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_bare_jpeg_format() {
        // Test bare JPEG format (FF D8 FF DB) without JFIF/EXIF headers
        let mut jpeg_data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xDB, // DQT marker
            0x00, 0x04, // DQT length
            0x00, 0x00, // DQT data
        ];
        // Add some padding
        jpeg_data.extend_from_slice(&[0xFF; 100]);
        // Add EOI
        jpeg_data.push(0xFF);
        jpeg_data.push(0xD9);

        let packet = MjpegPacket {
            header: MjpegHeader {
                sync_word: SYNC_WORD,
                sequence: 0,
                jpeg_size: jpeg_data.len() as u32,
            },
            jpeg_data: jpeg_data.clone(),
            crc16: calculate_crc16_ccitt(&jpeg_data),
        };

        assert!(packet.is_valid_jpeg(), "Bare JPEG format should be valid");
    }

    #[test]
    fn test_jfif_jpeg_format() {
        // Test JFIF JPEG format (FF D8 FF E0)
        let jpeg_data = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xE0, // APP0 (JFIF)
            0x00, 0x10, // APP0 length
            0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
            0x01, 0x01, // Version 1.1
            0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            0xFF, 0xD9, // EOI
        ];

        let packet = MjpegPacket {
            header: MjpegHeader {
                sync_word: SYNC_WORD,
                sequence: 0,
                jpeg_size: jpeg_data.len() as u32,
            },
            jpeg_data: jpeg_data.clone(),
            crc16: calculate_crc16_ccitt(&jpeg_data),
        };

        assert!(packet.is_valid_jpeg(), "JFIF JPEG format should be valid");
    }
}
