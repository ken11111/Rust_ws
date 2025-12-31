use serialport::{SerialPort, SerialPortType};
use std::io::{self, Read};
use std::time::Duration;
use log::{debug, info, error};
use crate::protocol::{MjpegPacket, MjpegHeader, MJPEG_HEADER_SIZE};

pub struct SerialConnection {
    port: Box<dyn SerialPort>,
}

impl SerialConnection {
    /// Open a serial port with the specified parameters
    pub fn open(port_name: &str, baud_rate: u32) -> io::Result<Self> {
        info!("Opening serial port: {} @ {} bps", port_name, baud_rate);

        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()
            .map_err(|e| {
                error!("Failed to open serial port {}: {}", port_name, e);
                io::Error::new(io::ErrorKind::NotFound, e.to_string())
            })?;

        info!("Serial port opened successfully");

        Ok(SerialConnection { port })
    }

    /// Auto-detect Spresense device by VID/PID
    pub fn auto_detect() -> io::Result<Self> {
        info!("Auto-detecting Spresense device...");

        let ports = serialport::available_ports()
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

        debug!("Found {} serial ports", ports.len());

        for port in &ports {
            debug!("  Port: {} - {:?}", port.port_name, port.port_type);

            match &port.port_type {
                SerialPortType::UsbPort(info) => {
                    debug!("    USB: VID={:04X} PID={:04X}", info.vid, info.pid);
                    // Spresense VID/PID: 0x054C/0x0BC2
                    if info.vid == 0x054C && info.pid == 0x0BC2 {
                        info!("Found Spresense device: {}", port.port_name);
                        return Self::open(&port.port_name, 115200);
                    }
                }
                _ => {}
            }
        }

        error!("Spresense device not found");
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Spresense device not found (VID=054C, PID=0BC2)",
        ))
    }

    /// List all available serial ports
    pub fn list_ports() -> io::Result<()> {
        let ports = serialport::available_ports()
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;

        if ports.is_empty() {
            info!("No serial ports found");
            return Ok(());
        }

        info!("Available serial ports:");
        for port in ports {
            match &port.port_type {
                SerialPortType::UsbPort(info) => {
                    info!("  {} - USB (VID={:04X} PID={:04X})",
                          port.port_name, info.vid, info.pid);
                    if let Some(ref manufacturer) = info.manufacturer {
                        info!("    Manufacturer: {}", manufacturer);
                    }
                    if let Some(ref product) = info.product {
                        info!("    Product: {}", product);
                    }
                }
                SerialPortType::BluetoothPort => {
                    info!("  {} - Bluetooth", port.port_name);
                }
                SerialPortType::PciPort => {
                    info!("  {} - PCI", port.port_name);
                }
                SerialPortType::Unknown => {
                    info!("  {} - Unknown", port.port_name);
                }
            }
        }

        Ok(())
    }

    /// Read raw bytes from serial port
    pub fn read_bytes(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.port.read(buf)
    }

    /// Read exact number of bytes from serial port
    pub fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.port.read_exact(buf)
    }

    /// Find sync word (0xCAFEBABE) in the byte stream
    ///
    /// Reads bytes one at a time until the sync word is found.
    /// This is used to recover from sync errors.
    pub fn find_sync_word(&mut self) -> io::Result<()> {
        use crate::protocol::SYNC_WORD;

        let mut buf = [0u8; 4];
        error!("🔍 Searching for sync word 0x{:08X}...", SYNC_WORD);

        // Initialize buffer with first 4 bytes
        match self.read_exact(&mut buf) {
            Ok(()) => {}
            Err(e) => {
                error!("Failed to read initial 4 bytes: {} (kind: {:?})", e, e.kind());
                return Err(e);
            }
        }

        let mut bytes_read = 4;
        const MAX_SEARCH_BYTES: usize = 100_000; // 100KB safety limit

        loop {
            // Check if current 4 bytes match sync word
            let current_word = u32::from_le_bytes(buf);
            if current_word == SYNC_WORD {
                error!("✅ Sync word found after reading {} bytes", bytes_read);
                return Ok(());
            }

            // Log progress every 1000 bytes
            if bytes_read % 1000 == 0 {
                error!("  Still searching... {} bytes read (current: 0x{:08X})", bytes_read, current_word);
            }

            // Safety check: prevent infinite loop
            if bytes_read >= MAX_SEARCH_BYTES {
                error!("❌ Sync word not found after {} bytes - giving up", MAX_SEARCH_BYTES);
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!("Sync word not found after {} bytes", MAX_SEARCH_BYTES),
                ));
            }

            // Shift buffer left by 1 byte and read new byte
            buf[0] = buf[1];
            buf[1] = buf[2];
            buf[2] = buf[3];

            let mut new_byte = [0u8; 1];
            match self.port.read_exact(&mut new_byte) {
                Ok(()) => {
                    buf[3] = new_byte[0];
                }
                Err(e) => {
                    error!("Failed to read byte at position {}: {} (kind: {:?})", bytes_read, e, e.kind());
                    return Err(e);
                }
            }

            bytes_read += 1;
        }
    }

    /// Read a complete MJPEG packet from serial port
    pub fn read_packet(&mut self) -> io::Result<MjpegPacket> {
        // Read header first (12 bytes)
        let mut header_buf = [0u8; MJPEG_HEADER_SIZE];

        debug!("Reading MJPEG header ({} bytes)...", MJPEG_HEADER_SIZE);
        self.read_exact(&mut header_buf)?;

        debug!("Header bytes: {:02X?}", &header_buf[..12]);

        let header = MjpegHeader::parse(&header_buf)?;

        debug!("Parsed header: sync=0x{:08X}, seq={}, jpeg_size={} bytes",
               header.sync_word, header.sequence, header.jpeg_size);

        // Validate JPEG size (additional safety check)
        if header.jpeg_size > 524288 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JPEG size too large: {} bytes", header.jpeg_size),
            ));
        }

        // Allocate buffer for complete packet
        let total_size = header.total_size();
        let mut packet_buf = vec![0u8; total_size];

        // Copy header
        packet_buf[..MJPEG_HEADER_SIZE].copy_from_slice(&header_buf);

        // Read JPEG data + CRC (jpeg_size + 2 bytes)
        let remaining_size = header.jpeg_size as usize + 2;
        debug!("Reading JPEG data + CRC ({} bytes)...", remaining_size);
        self.read_exact(&mut packet_buf[MJPEG_HEADER_SIZE..total_size])?;

        // Parse and verify complete packet
        MjpegPacket::parse(&packet_buf)
    }

    /// Read packet body after sync word has been found
    ///
    /// This is called after find_sync_word() has successfully located the sync word.
    /// The sync word has already been consumed, so we only read the remaining parts.
    fn read_packet_after_sync(&mut self) -> io::Result<MjpegPacket> {
        use crate::protocol::SYNC_WORD;

        // Read sequence and JPEG size (8 bytes)
        let mut header_tail = [0u8; 8];
        self.read_exact(&mut header_tail)?;

        let sequence = u32::from_le_bytes([header_tail[0], header_tail[1], header_tail[2], header_tail[3]]);
        let jpeg_size = u32::from_le_bytes([header_tail[4], header_tail[5], header_tail[6], header_tail[7]]);

        debug!("After sync: seq={}, jpeg_size={} bytes", sequence, jpeg_size);

        // Validate JPEG size
        if jpeg_size > 524288 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JPEG size too large: {} bytes", jpeg_size),
            ));
        }

        // Reconstruct full header (12 bytes)
        let mut header_buf = [0u8; MJPEG_HEADER_SIZE];
        header_buf[0..4].copy_from_slice(&SYNC_WORD.to_le_bytes());
        header_buf[4..12].copy_from_slice(&header_tail);

        // Allocate buffer for complete packet
        let total_size = MJPEG_HEADER_SIZE + jpeg_size as usize + 2;
        let mut packet_buf = vec![0u8; total_size];

        // Copy header
        packet_buf[..MJPEG_HEADER_SIZE].copy_from_slice(&header_buf);

        // Read JPEG data + CRC
        let remaining_size = jpeg_size as usize + 2;
        debug!("Reading JPEG data + CRC ({} bytes)...", remaining_size);
        self.read_exact(&mut packet_buf[MJPEG_HEADER_SIZE..total_size])?;

        // Parse and verify complete packet
        MjpegPacket::parse(&packet_buf)
    }

    /// Read MJPEG packet with automatic error recovery
    ///
    /// This function wraps read_packet() and adds:
    /// - Sync word search on parse errors
    /// - JPEG marker validation
    /// - Automatic retry on recoverable errors
    pub fn read_packet_with_recovery(&mut self) -> io::Result<MjpegPacket> {
        const MAX_RETRIES: usize = 3;
        let mut retry_count = 0;
        let mut need_sync = false;

        loop {
            // If we need to resync, find the sync word first
            if need_sync {
                error!("Attempting to resync (retry {}/{})...", retry_count, MAX_RETRIES);
                match self.find_sync_word() {
                    Ok(()) => {
                        error!("✅ Resync successful, reading packet after sync word...");
                        need_sync = false;
                    }
                    Err(ref e) => {
                        error!("Resync failed: {} (kind: {:?})", e, e.kind());
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Resync failed: {}", e),
                        ));
                    }
                }
            }

            // Read packet (or packet body if we just synced)
            let packet_result = if need_sync == false && retry_count > 0 {
                // We just found sync word, read the rest
                self.read_packet_after_sync()
            } else {
                // Normal read (includes sync word)
                self.read_packet()
            };

            match packet_result {
                Ok(packet) => {
                    // Validate JPEG markers (SOI and EOI)
                    if packet.is_valid_jpeg() {
                        // Reset retry count on success
                        if retry_count > 0 {
                            info!("Successfully recovered after {} retries", retry_count);
                        }
                        return Ok(packet);
                    } else {
                        error!("Invalid JPEG markers detected (no SOI/EOI), seq={}, size={}",
                               packet.header.sequence, packet.jpeg_data.len());
                        if packet.jpeg_data.len() >= 4 {
                            error!("  First 4 bytes: {:02X?}", &packet.jpeg_data[..4]);
                        }
                        if packet.jpeg_data.len() >= 4 {
                            error!("  Last 4 bytes: {:02X?}", &packet.jpeg_data[packet.jpeg_data.len().saturating_sub(4)..]);
                        }

                        retry_count += 1;
                        if retry_count >= MAX_RETRIES {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Too many invalid JPEG packets",
                            ));
                        }
                        need_sync = true;
                        continue;
                    }
                }
                Err(e) => {
                    match e.kind() {
                        // Recoverable errors - try to resync
                        io::ErrorKind::InvalidData => {
                            error!("Packet parse error (retry {}/{}): {} (kind: {:?})",
                                   retry_count + 1, MAX_RETRIES, e, e.kind());

                            retry_count += 1;
                            if retry_count >= MAX_RETRIES {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!("Failed to recover after {} retries", MAX_RETRIES),
                                ));
                            }
                            need_sync = true;
                            continue;
                        }
                        // Non-recoverable errors - propagate immediately
                        io::ErrorKind::TimedOut => {
                            error!("Serial read timeout: {}", e);
                            return Err(e);
                        }
                        _ => {
                            error!("Non-recoverable error: {} (kind: {:?})", e, e.kind());
                            return Err(e);
                        }
                    }
                }
            }
        }
    }

    /// Flush the receive buffer
    pub fn flush(&mut self) -> io::Result<()> {
        // Read and discard all available data
        let mut discard_buf = [0u8; 1024];
        loop {
            match self.port.read(&mut discard_buf) {
                Ok(0) => break,
                Ok(n) => {
                    debug!("Flushed {} bytes from receive buffer", n);
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Set timeout for read operations
    pub fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        self.port.set_timeout(timeout)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_ports() {
        // This test just ensures the function doesn't panic
        let _ = SerialConnection::list_ports();
    }
}
