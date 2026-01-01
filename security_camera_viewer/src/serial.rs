use serialport::{SerialPort, SerialPortType};
use std::io::{self, Read};
use std::time::Duration;
use log::{debug, info, error};
use crate::protocol::{
    MjpegPacket, MjpegHeader, MetricsPacket, Packet,
    MJPEG_HEADER_SIZE, SYNC_WORD, METRICS_SYNC_WORD, METRICS_PACKET_SIZE
};
use byteorder::{LittleEndian, ReadBytesExt};

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


    /// Read a complete packet from serial port (MJPEG or Metrics)
    /// Phase 4.1 extension: Returns Packet enum that can be either type
    pub fn read_packet(&mut self) -> io::Result<Packet> {
        // Read sync word first (4 bytes) to determine packet type
        let mut sync_buf = [0u8; 4];
        self.read_exact(&mut sync_buf)?;

        let mut cursor = std::io::Cursor::new(&sync_buf);
        let sync_word = cursor.read_u32::<LittleEndian>()?;

        debug!("Read sync word: 0x{:08X}", sync_word);

        match sync_word {
            SYNC_WORD => {
                // MJPEG packet
                debug!("Detected MJPEG packet");

                // Read rest of header (8 more bytes: sequence + jpeg_size)
                let mut rest_header = [0u8; 8];
                self.read_exact(&mut rest_header)?;

                // Reconstruct full header
                let mut header_buf = [0u8; MJPEG_HEADER_SIZE];
                header_buf[0..4].copy_from_slice(&sync_buf);
                header_buf[4..12].copy_from_slice(&rest_header);

                let header = MjpegHeader::parse(&header_buf)?;

                debug!("MJPEG header: seq={}, jpeg_size={} bytes",
                       header.sequence, header.jpeg_size);

                // Validate JPEG size
                if header.jpeg_size > 524288 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("JPEG size too large: {} bytes", header.jpeg_size),
                    ));
                }

                // Allocate buffer for complete packet
                let total_size = header.total_size();
                let mut packet_buf = vec![0u8; total_size];
                packet_buf[..MJPEG_HEADER_SIZE].copy_from_slice(&header_buf);

                // Read JPEG data + CRC
                let remaining_size = header.jpeg_size as usize + 2;
                self.read_exact(&mut packet_buf[MJPEG_HEADER_SIZE..total_size])?;

                // Parse and verify complete packet
                let mjpeg_packet = MjpegPacket::parse(&packet_buf)?;
                Ok(Packet::Mjpeg(mjpeg_packet))
            }

            METRICS_SYNC_WORD => {
                // Metrics packet
                debug!("Detected Metrics packet");

                // Read rest of packet (34 more bytes: METRICS_PACKET_SIZE - 4)
                let mut packet_buf = [0u8; METRICS_PACKET_SIZE];
                packet_buf[0..4].copy_from_slice(&sync_buf);
                self.read_exact(&mut packet_buf[4..METRICS_PACKET_SIZE])?;

                // Parse and verify metrics packet
                let metrics_packet = MetricsPacket::parse(&packet_buf)?;

                info!("Metrics packet: seq={}, cam_frames={}, usb_pkts={}, q_depth={}, errors={}",
                      metrics_packet.sequence,
                      metrics_packet.camera_frames,
                      metrics_packet.usb_packets,
                      metrics_packet.action_q_depth,
                      metrics_packet.errors);

                Ok(Packet::Metrics(metrics_packet))
            }

            _ => {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unknown sync word: 0x{:08X}", sync_word),
                ))
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
