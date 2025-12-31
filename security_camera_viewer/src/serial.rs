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
