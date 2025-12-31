use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Cursor};

/// MJPEG Protocol Constants
pub const SYNC_WORD: u32 = 0xCAFEBABE;
pub const MJPEG_HEADER_SIZE: usize = 12; // sync_word(4) + sequence(4) + jpeg_size(4)
pub const CRC_SIZE: usize = 2;
pub const MIN_PACKET_SIZE: usize = MJPEG_HEADER_SIZE + CRC_SIZE; // 14 bytes

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
