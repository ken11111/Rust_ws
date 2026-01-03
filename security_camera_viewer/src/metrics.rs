use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Performance metrics data structure
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub timestamp: f64,           // Unix timestamp (seconds.milliseconds)
    pub pc_fps: f32,              // PC-side FPS (calculated)
    pub spresense_fps: f32,       // Spresense-side FPS (from sequence numbers)
    pub frame_count: u64,         // Total frames received
    pub error_count: u32,         // Total errors
    pub decode_time_ms: f32,      // JPEG decode time
    pub serial_read_time_ms: f32, // Serial read time
    pub texture_upload_time_ms: f32, // Texture upload time
    pub jpeg_size_kb: f32,        // JPEG size in KB
    // Phase 4.1: Spresense-side metrics
    pub spresense_camera_frames: u32,  // Spresense camera frames captured
    pub spresense_camera_fps: f32,     // Spresense camera FPS (from Metrics packet)
    pub spresense_usb_packets: u32,    // Spresense USB packets sent
    pub action_q_depth: u32,           // Pipeline queue depth (0-3)
    pub spresense_errors: u32,         // Spresense error count
    // Phase 7: TCP performance metrics
    pub tcp_avg_send_ms: f32,          // Average TCP send time (milliseconds)
    pub tcp_max_send_ms: f32,          // Maximum TCP send time (milliseconds)
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            timestamp: Self::current_timestamp(),
            pc_fps: 0.0,
            spresense_fps: 0.0,
            frame_count: 0,
            error_count: 0,
            decode_time_ms: 0.0,
            serial_read_time_ms: 0.0,
            texture_upload_time_ms: 0.0,
            jpeg_size_kb: 0.0,
            // Phase 4.1: Spresense-side metrics
            spresense_camera_frames: 0,
            spresense_camera_fps: 0.0,
            spresense_usb_packets: 0,
            action_q_depth: 0,
            spresense_errors: 0,
            // Phase 7: TCP performance metrics
            tcp_avg_send_ms: 0.0,
            tcp_max_send_ms: 0.0,
        }
    }

    fn current_timestamp() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}

/// CSV metrics logger
pub struct MetricsLogger {
    file: Arc<Mutex<File>>,
    log_path: PathBuf,
}

impl MetricsLogger {
    /// Create a new metrics logger with timestamped filename
    pub fn new(output_dir: &str) -> io::Result<Self> {
        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;

        // Generate timestamped filename
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        let timestamp = chrono::DateTime::<chrono::Utc>::from(UNIX_EPOCH + now)
            .format("%Y%m%d_%H%M%S");

        let log_path = PathBuf::from(output_dir)
            .join(format!("metrics_{}.csv", timestamp));

        let mut file = File::create(&log_path)?;

        // Write CSV header (Phase 4.1: Added Spresense-side metrics)
        writeln!(
            file,
            "timestamp,pc_fps,spresense_fps,frame_count,error_count,\
             decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb,\
             spresense_camera_frames,spresense_camera_fps,spresense_usb_packets,action_q_depth,spresense_errors,\
             tcp_avg_send_ms,tcp_max_send_ms"
        )?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            log_path,
        })
    }

    /// Log a metrics sample to CSV (Phase 4.1: Added Spresense-side metrics)
    pub fn log(&self, metrics: &PerformanceMetrics) -> io::Result<()> {
        let mut file = self.file.lock().unwrap();

        writeln!(
            file,
            "{:.3},{:.2},{:.2},{},{},{:.2},{:.2},{:.2},{:.2},{},{:.2},{},{},{},{:.2},{:.2}",
            metrics.timestamp,
            metrics.pc_fps,
            metrics.spresense_fps,
            metrics.frame_count,
            metrics.error_count,
            metrics.decode_time_ms,
            metrics.serial_read_time_ms,
            metrics.texture_upload_time_ms,
            metrics.jpeg_size_kb,
            metrics.spresense_camera_frames,
            metrics.spresense_camera_fps,
            metrics.spresense_usb_packets,
            metrics.action_q_depth,
            metrics.spresense_errors,
            metrics.tcp_avg_send_ms,
            metrics.tcp_max_send_ms,
        )?;

        file.flush()?;
        Ok(())
    }

    /// Get the log file path
    pub fn path(&self) -> &PathBuf {
        &self.log_path
    }
}

/// Spresense FPS calculator
///
/// Calculates Spresense-side send rate from packet sequence numbers
pub struct SpresenseFpsCalculator {
    last_sequence: Option<u32>,
    last_timestamp: Option<f64>,
    sequence_window: Vec<(u32, f64)>,  // (sequence, timestamp) pairs
    window_size: usize,
}

/// Spresense Camera FPS calculator
///
/// Calculates Spresense-side camera FPS from Metrics packets
/// Uses timestamp_ms and camera_frames fields
pub struct SpresenseCameraFpsCalculator {
    last_camera_frames: Option<u32>,
    last_timestamp_ms: Option<u32>,
}

impl SpresenseCameraFpsCalculator {
    pub fn new() -> Self {
        Self {
            last_camera_frames: None,
            last_timestamp_ms: None,
        }
    }

    /// Update with new Metrics packet data
    /// Returns Spresense camera FPS
    pub fn update(&mut self, timestamp_ms: u32, camera_frames: u32) -> f32 {
        if let (Some(last_frames), Some(last_ts)) = (self.last_camera_frames, self.last_timestamp_ms) {
            let frame_delta = if camera_frames >= last_frames {
                camera_frames - last_frames
            } else {
                // Handle wraparound (unlikely but possible)
                (u32::MAX - last_frames) + camera_frames + 1
            };

            let time_delta_ms = if timestamp_ms >= last_ts {
                timestamp_ms - last_ts
            } else {
                // Handle wraparound
                (u32::MAX - last_ts) + timestamp_ms + 1
            };

            let time_delta_sec = time_delta_ms as f32 / 1000.0;

            // Store current values for next calculation
            self.last_camera_frames = Some(camera_frames);
            self.last_timestamp_ms = Some(timestamp_ms);

            if time_delta_sec > 0.0 {
                return frame_delta as f32 / time_delta_sec;
            }
        } else {
            // First call - just store values
            self.last_camera_frames = Some(camera_frames);
            self.last_timestamp_ms = Some(timestamp_ms);
        }

        0.0
    }
}

impl SpresenseFpsCalculator {
    pub fn new(window_size: usize) -> Self {
        Self {
            last_sequence: None,
            last_timestamp: None,
            sequence_window: Vec::with_capacity(window_size),
            window_size,
        }
    }

    /// Update with new packet sequence number
    /// Returns current Spresense FPS estimate
    pub fn update(&mut self, sequence: u32) -> f32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        // Add to window
        self.sequence_window.push((sequence, now));

        // Trim window to size
        if self.sequence_window.len() > self.window_size {
            self.sequence_window.remove(0);
        }

        // Calculate FPS from window
        if self.sequence_window.len() >= 2 {
            let first = self.sequence_window.first().unwrap();
            let last = self.sequence_window.last().unwrap();

            let time_delta = last.1 - first.1;
            let sequence_delta = if last.0 >= first.0 {
                last.0 - first.0
            } else {
                // Handle sequence number wraparound (unlikely but possible)
                (u32::MAX - first.0) + last.0 + 1
            };

            if time_delta > 0.0 {
                return sequence_delta as f32 / time_delta as f32;
            }
        }

        0.0
    }

    /// Get the last calculated FPS
    pub fn current_fps(&self) -> f32 {
        if self.sequence_window.len() >= 2 {
            let first = self.sequence_window.first().unwrap();
            let last = self.sequence_window.last().unwrap();

            let time_delta = last.1 - first.1;
            let sequence_delta = if last.0 >= first.0 {
                last.0 - first.0
            } else {
                (u32::MAX - first.0) + last.0 + 1
            };

            if time_delta > 0.0 {
                return sequence_delta as f32 / time_delta as f32;
            }
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spresense_fps_calculator() {
        let mut calc = SpresenseFpsCalculator::new(30);

        // Simulate 30 fps (33.33 ms per frame)
        for i in 0..30 {
            calc.update(i);
            std::thread::sleep(std::time::Duration::from_millis(33));
        }

        let fps = calc.current_fps();
        assert!(fps > 25.0 && fps < 35.0, "FPS should be around 30, got {}", fps);
    }

    #[test]
    fn test_sequence_wraparound() {
        let mut calc = SpresenseFpsCalculator::new(10);

        // Test near wraparound point
        calc.update(u32::MAX - 5);
        std::thread::sleep(std::time::Duration::from_millis(10));
        calc.update(u32::MAX - 4);
        std::thread::sleep(std::time::Duration::from_millis(10));
        calc.update(u32::MAX - 3);

        let fps = calc.current_fps();
        assert!(fps > 0.0, "FPS should be calculated even near wraparound");
    }
}
