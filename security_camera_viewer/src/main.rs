mod protocol;
mod serial;

use clap::Parser;
use log::{debug, info, warn, error};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use anyhow::{Result, Context};
use serial::SerialConnection;
use protocol::Packet;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Serial port path (e.g., /dev/ttyACM0)
    /// If not specified, auto-detection will be attempted
    #[arg(short, long)]
    port: Option<String>,

    /// Output directory for JPEG frames
    #[arg(short, long, default_value = "output")]
    output: String,

    /// Save as individual JPEG files instead of continuous stream
    #[arg(long)]
    individual_files: bool,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,

    /// List available serial ports and exit
    #[arg(short, long)]
    list: bool,

    /// Maximum number of frames to capture (0 = unlimited)
    #[arg(long, default_value = "0")]
    max_frames: u64,

    /// Maximum number of consecutive errors before exit
    #[arg(long, default_value = "10")]
    max_errors: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logger
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    info!("Security Camera Viewer (MJPEG) v{}", env!("CARGO_PKG_VERSION"));
    info!("==========================================");

    // List ports mode
    if args.list {
        return SerialConnection::list_ports()
            .context("Failed to list serial ports");
    }

    // Connect to serial port
    let mut serial = if let Some(port) = args.port {
        info!("Connecting to specified port: {}", port);
        SerialConnection::open(&port, 115200)
            .context(format!("Failed to open port {}", port))?
    } else {
        info!("Auto-detecting Spresense device...");
        SerialConnection::auto_detect()
            .context("Failed to auto-detect Spresense device")?
    };

    info!("Connected successfully");

    // Prepare output
    let output_path = PathBuf::from(&args.output);
    let mut stream_file = if args.individual_files {
        // Create output directory for individual JPEG files
        fs::create_dir_all(&output_path)
            .context(format!("Failed to create output directory: {}", args.output))?;
        info!("Output directory: {}", args.output);
        info!("Mode: Individual JPEG files");
        None
    } else {
        // Create single MJPEG stream file
        let stream_path = output_path.with_extension("mjpeg");
        let file = File::create(&stream_path)
            .context(format!("Failed to create output file: {:?}", stream_path))?;
        info!("Output file: {:?}", stream_path);
        info!("Mode: MJPEG stream");
        Some(file)
    };

    // Flush any existing data in the buffer
    info!("Flushing receive buffer...");
    serial.flush()?;

    // Start receiving frames
    info!("==========================================");
    info!("Receiving MJPEG frames...");
    info!("Press Ctrl+C to stop");
    info!("==========================================");

    let mut frame_count = 0u64;
    let mut packet_count = 0u64;
    let mut error_count = 0u32;
    let mut total_bytes = 0u64;
    let mut jpeg_errors = 0u32;

    loop {
        // Check max frames limit
        if args.max_frames > 0 && frame_count >= args.max_frames {
            info!("Reached maximum frame count ({})", args.max_frames);
            break;
        }

        match serial.read_packet() {
            Ok(Packet::Mjpeg(packet)) => {
                error_count = 0; // Reset error count on success
                packet_count += 1;
                frame_count += 1;

                debug!("Packet #{}: seq={}, jpeg_size={} bytes, crc=0x{:04X}",
                       packet_count,
                       packet.header.sequence,
                       packet.header.jpeg_size,
                       packet.crc16);

                let jpeg_size = packet.jpeg_data.len();

                // Verify JPEG validity
                if !packet.is_valid_jpeg() {
                    warn!("Frame #{}: Invalid JPEG markers detected!", frame_count);

                    // Print first and last bytes for diagnosis
                    if args.verbose {
                        if jpeg_size >= 16 {
                            let hex_dump: Vec<String> = packet.jpeg_data[..16]
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect();
                            warn!("  First 16 bytes: {}", hex_dump.join(" "));
                            warn!("  Expected SOI marker: FF D8");
                        }
                        if jpeg_size >= 4 {
                            let last_bytes: Vec<String> = packet.jpeg_data[jpeg_size.saturating_sub(4)..]
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect();
                            warn!("  Last {} bytes: {}", last_bytes.len(), last_bytes.join(" "));
                            warn!("  Expected EOI marker: FF D9");
                        }
                    }

                    jpeg_errors += 1;
                } else if args.verbose && frame_count <= 3 {
                    // Show first valid JPEG markers (JFIF or bare JPEG format)
                    if jpeg_size >= 4 {
                        let format_type = if jpeg_size >= 4 && packet.jpeg_data[2] == 0xFF {
                            match packet.jpeg_data[3] {
                                0xE0 => "JFIF",
                                0xE1 => "EXIF",
                                0xDB => "Bare JPEG (DQT)",
                                _ => "Unknown",
                            }
                        } else {
                            "Unknown"
                        };
                        debug!("  Valid JPEG ({}): {:02X} {:02X} {:02X} {:02X} ...",
                              format_type,
                              packet.jpeg_data[0], packet.jpeg_data[1],
                              packet.jpeg_data[2], packet.jpeg_data[3]);
                    }
                }
                total_bytes += jpeg_size as u64;

                // Save JPEG data
                if args.individual_files {
                    // Save as individual file
                    let filename = output_path.join(format!("frame_{:06}.jpg", frame_count));
                    match File::create(&filename) {
                        Ok(mut file) => {
                            file.write_all(&packet.jpeg_data)
                                .context(format!("Failed to write JPEG file: {:?}", filename))?;
                            debug!("Saved: {:?}", filename);
                        }
                        Err(e) => {
                            error!("Failed to create file {:?}: {}", filename, e);
                            jpeg_errors += 1;
                        }
                    }
                } else {
                    // Append to stream file
                    if let Some(ref mut file) = stream_file {
                        file.write_all(&packet.jpeg_data)
                            .context("Failed to write to MJPEG stream")?;
                    }
                }

                // Log progress every 30 frames (1 second at 30fps)
                if frame_count % 30 == 0 {
                    info!("Progress: {} frames, {} packets, {:.2} MB, {} JPEG errors",
                          frame_count,
                          packet_count,
                          total_bytes as f64 / 1_048_576.0,
                          jpeg_errors);
                }
            }

            Ok(Packet::Metrics(metrics)) => {
                // Phase 4.1: Metrics packets - just log and continue (CLI viewer doesn't display them)
                error_count = 0; // Reset error count on success
                packet_count += 1;
                debug!("Metrics packet: seq={}, cam_frames={}, usb_pkts={}, q_depth={}, errors={}",
                       metrics.sequence,
                       metrics.camera_frames,
                       metrics.usb_packets,
                       metrics.action_q_depth,
                       metrics.errors);
            }

            Err(e) => {
                error_count += 1;

                // Distinguish between timeout and real errors
                if e.kind() == std::io::ErrorKind::TimedOut {
                    debug!("Read timeout ({}), retrying...", error_count);
                } else {
                    error!("Packet read error ({}): {}", error_count, e);
                }

                if error_count >= args.max_errors {
                    error!("Too many consecutive errors ({}), exiting", error_count);
                    break;
                }
            }
        }
    }

    // Final statistics
    info!("==========================================");
    info!("Reception Summary:");
    info!("  Total frames: {}", frame_count);
    info!("  Total packets: {}", packet_count);
    info!("  Total data: {:.2} MB", total_bytes as f64 / 1_048_576.0);
    info!("  JPEG errors: {}", jpeg_errors);
    if frame_count > 0 {
        info!("  Average frame size: {:.2} KB",
              (total_bytes as f64 / frame_count as f64) / 1024.0);
    }
    info!("==========================================");

    if args.individual_files {
        info!("JPEG files saved to: {}", args.output);
        info!("View with: feh {} or eog {}", args.output, args.output);
    } else {
        let stream_path = PathBuf::from(&args.output).with_extension("mjpeg");
        info!("MJPEG stream saved to: {:?}", stream_path);
        info!("Play with: ffplay {:?} or vlc {:?}", stream_path, stream_path);
        info!("Or extract frames with: ffmpeg -i {:?} frame_%04d.jpg", stream_path);
    }

    if frame_count > 0 {
        info!("Success! {} frames captured.", frame_count);
    } else {
        warn!("Warning: No frames received.");
    }

    Ok(())
}
