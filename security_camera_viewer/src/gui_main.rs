mod protocol;
mod serial;
mod tcp_connection;  // Phase 7: WiFi support
mod metrics;
mod ring_buffer;
mod motion_detector;
mod mp4_recorder;

use eframe::egui;
use log::{error, info, warn};
use serial::SerialConnection;
use tcp_connection::TcpConnection;  // Phase 7: WiFi support
use protocol::Packet;
use metrics::{MetricsLogger, PerformanceMetrics, SpresenseFpsCalculator, SpresenseCameraFpsCalculator};
use ring_buffer::{RingBuffer, JpegFrame};
use motion_detector::{MotionDetector, MotionDetectionConfig};
use mp4_recorder::Mp4Recorder;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use chrono;

// Phase 3: Recording functionality constants
const MAX_RECORDING_SIZE: u64 = 1_000_000_000;  // 1 GB
const RECORDING_DIR: &str = "./recordings";

// Phase 3/5: Recording state management
/// 録画フォーマット (Phase 6)
#[derive(Debug, Clone, Copy, PartialEq)]
enum RecordingFormat {
    /// MJPEG形式（Phase 3-5）
    Mjpeg,
    /// MP4形式（Phase 6）
    Mp4,
}

impl Default for RecordingFormat {
    fn default() -> Self {
        RecordingFormat::Mp4  // Phase 6以降はMP4をデフォルトに
    }
}

// Phase 7: Transport type selection (USB Serial vs WiFi TCP)
#[derive(Debug, Clone, Copy, PartialEq)]
enum TransportType {
    /// USB Serial connection (Phase 1-6)
    UsbSerial,
    /// WiFi TCP connection (Phase 7)
    WiFi,
}

impl Default for TransportType {
    fn default() -> Self {
        TransportType::UsbSerial  // USB Serial is default
    }
}

enum RecordingState {
    Idle,
    /// 手動録画 (Phase 3)
    ManualRecording {
        filepath: PathBuf,
        start_time: Instant,
        frame_count: u32,
        total_bytes: u64,
        format: RecordingFormat,  // Phase 6: 録画フォーマット
    },
    /// 動き検知録画 (Phase 5)
    MotionRecording {
        filepath: PathBuf,
        start_time: Instant,
        frame_count: u32,
        total_bytes: u64,
        motion_active: bool,           // 現在動き検知中か
        countdown_frames: u32,         // ポスト録画残りフレーム数
        format: RecordingFormat,  // Phase 6: 録画フォーマット
    },
}

#[derive(Debug, Clone)]
enum AppMessage {
    NewFrame(Vec<u8>),  // Legacy - will be replaced by DecodedFrame
    DecodedFrame { width: u32, height: u32, pixels: Vec<u8> },  // RGBA8 decoded image
    ConnectionStatus(String),
    Stats {
        fps: f32,
        spresense_fps: f32,  // Spresense-side FPS (from sequence numbers)
        frame_count: u64,
        errors: u32,
        decode_time_ms: f32,
        serial_read_time_ms: f32,
        texture_upload_time_ms: f32,
        jpeg_size_kb: f32,  // JPEG size in KB
    },
    SpresenseMetrics {  // Phase 4.1: Spresense-side metrics
        timestamp_ms: u32,
        camera_frames: u32,
        camera_fps: f32,
        usb_packets: u32,
        action_q_depth: u32,
        avg_packet_size: u32,
        errors: u32,
    },
    JpegFrame(Vec<u8>),  // Phase 3: JPEG frame data for recording
}

struct CameraApp {
    // Communication
    rx: Receiver<AppMessage>,
    tx: Sender<AppMessage>,

    // State
    current_frame: Option<egui::TextureHandle>,
    connection_status: String,
    is_running: Arc<Mutex<bool>>,
    is_recording: Arc<AtomicBool>,  // Phase 3: Recording state shared with capture thread

    // Statistics
    fps: f32,
    spresense_fps: f32,
    frame_count: u64,
    error_count: u32,
    decode_time_ms: f32,
    serial_read_time_ms: f32,
    texture_upload_time_ms: f32,
    jpeg_size_kb: f32,

    // Phase 4.1: Spresense-side metrics
    spresense_camera_frames: Option<u32>,
    spresense_camera_fps: Option<f32>,
    spresense_action_q_depth: Option<u32>,
    spresense_errors: Option<u32>,

    // Phase 3: Recording functionality
    recording_state: RecordingState,
    recording_file: Option<Arc<Mutex<File>>>,
    recording_dir: PathBuf,

    // Phase 5: Motion detection recording
    motion_config: MotionDetectionConfig,
    motion_detector: MotionDetector,
    ring_buffer: RingBuffer,
    last_motion_time: Option<Instant>,

    // Phase 6: MP4 recording
    recording_format: RecordingFormat,
    mp4_recorder: Option<Mp4Recorder>,

    // Settings
    port_path: String,
    auto_detect: bool,

    // Phase 7: WiFi settings
    transport_type: TransportType,
    wifi_host: String,
    wifi_port: u16,
}

impl CameraApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            rx,
            tx,
            current_frame: None,
            connection_status: "Not connected".to_string(),
            is_running: Arc::new(Mutex::new(false)),
            is_recording: Arc::new(AtomicBool::new(false)),
            fps: 0.0,
            spresense_fps: 0.0,
            frame_count: 0,
            error_count: 0,
            decode_time_ms: 0.0,
            serial_read_time_ms: 0.0,
            texture_upload_time_ms: 0.0,
            jpeg_size_kb: 0.0,
            spresense_camera_frames: None,
            spresense_camera_fps: None,
            spresense_action_q_depth: None,
            spresense_errors: None,
            recording_state: RecordingState::Idle,
            recording_file: None,
            recording_dir: PathBuf::from(RECORDING_DIR),
            motion_config: MotionDetectionConfig::default(),
            motion_detector: MotionDetector::default(),
            ring_buffer: RingBuffer::from_seconds(10, 11),  // 10秒@11fps
            last_motion_time: None,
            recording_format: RecordingFormat::default(),
            mp4_recorder: None,
            port_path: "/dev/ttyACM0".to_string(),
            auto_detect: true,
            // Phase 7: WiFi settings
            transport_type: TransportType::default(),
            wifi_host: "192.168.1.100".to_string(),
            wifi_port: 8888,
        }
    }

    fn start_capture(&mut self) {
        if *self.is_running.lock().unwrap() {
            warn!("Capture already running");
            return;
        }

        *self.is_running.lock().unwrap() = true;

        let tx = self.tx.clone();
        let is_running = self.is_running.clone();
        let is_recording = self.is_recording.clone();
        let port_path = self.port_path.clone();
        let auto_detect = self.auto_detect;
        // Phase 7: WiFi configuration
        let transport_type = self.transport_type;
        let wifi_host = self.wifi_host.clone();
        let wifi_port = self.wifi_port;

        thread::spawn(move || {
            capture_thread(tx, is_running, is_recording, port_path, auto_detect, transport_type, wifi_host, wifi_port);
        });
    }

    fn stop_capture(&mut self) {
        *self.is_running.lock().unwrap() = false;
        self.connection_status = "Stopped".to_string();

        // Phase 3/5: Auto-stop recording when capture stops
        if matches!(self.recording_state, RecordingState::ManualRecording { .. } | RecordingState::MotionRecording { .. }) {
            if let Err(e) = self.stop_recording() {
                error!("Failed to auto-stop recording: {}", e);
            }
        }
    }

    // Phase 3/5: Recording methods
    fn start_manual_recording(&mut self) -> io::Result<()> {
        // Check if already recording
        if matches!(self.recording_state, RecordingState::ManualRecording { .. } | RecordingState::MotionRecording { .. }) {
            warn!("Recording already in progress");
            return Ok(());
        }

        // Create recording directory if it doesn't exist
        std::fs::create_dir_all(&self.recording_dir)?;

        // Generate filename with timestamp (Phase 6: dynamic extension)
        let now = chrono::Local::now();
        let extension = match self.recording_format {
            RecordingFormat::Mjpeg => "mjpeg",
            RecordingFormat::Mp4 => "mp4",
        };
        let filename = format!("manual_{}.{}", now.format("%Y%m%d_%H%M%S"), extension);
        let filepath = self.recording_dir.join(&filename);

        // Phase 6: Create recorder based on format
        match self.recording_format {
            RecordingFormat::Mjpeg => {
                let file = File::create(&filepath)?;
                info!("Started manual MJPEG recording to: {:?}", filepath);
                self.recording_file = Some(Arc::new(Mutex::new(file)));
            }
            RecordingFormat::Mp4 => {
                let recorder = Mp4Recorder::new(&filepath, 11)?;  // 11 fps
                info!("Started manual MP4 recording to: {:?}", filepath);
                self.mp4_recorder = Some(recorder);
            }
        }

        // Update state
        self.recording_state = RecordingState::ManualRecording {
            filepath: filepath.clone(),
            start_time: Instant::now(),
            frame_count: 0,
            total_bytes: 0,
            format: self.recording_format,
        };

        self.is_recording.store(true, Ordering::Relaxed);

        Ok(())
    }

    // Phase 5: Motion detection recording
    fn start_motion_recording(&mut self) -> io::Result<()> {
        // Check if already recording
        if matches!(self.recording_state, RecordingState::ManualRecording { .. } | RecordingState::MotionRecording { .. }) {
            return Ok(());  // Already recording, skip silently
        }

        // Create recording directory
        std::fs::create_dir_all(&self.recording_dir)?;

        // Generate filename with timestamp (Phase 6: dynamic extension)
        let now = chrono::Local::now();
        let extension = match self.recording_format {
            RecordingFormat::Mjpeg => "mjpeg",
            RecordingFormat::Mp4 => "mp4",
        };
        let filename = format!("motion_{}.{}", now.format("%Y%m%d_%H%M%S"), extension);
        let filepath = self.recording_dir.join(&filename);

        // Phase 6: Create recorder and write pre-buffer based on format
        let (pre_frames, pre_bytes) = match self.recording_format {
            RecordingFormat::Mjpeg => {
                let mut file = File::create(&filepath)?;
                let (frames, bytes) = self.ring_buffer.flush_to_file(&mut file)?;
                info!("Started motion MJPEG recording to: {:?}", filepath);
                info!("  Pre-buffer: {} frames, {:.2} MB", frames, bytes as f32 / 1_000_000.0);
                self.recording_file = Some(Arc::new(Mutex::new(file)));
                (frames, bytes)
            }
            RecordingFormat::Mp4 => {
                let mut recorder = Mp4Recorder::new(&filepath, 11)?;

                // Write pre-buffer frames to MP4
                let mut pre_frame_count = 0;
                let mut pre_byte_count = 0;

                // Ring bufferから各フレームを取得してMP4に書き込む
                // Note: ring_bufferはflush_to_fileでしか一括取得できないため、
                // 一時的にMJPEGファイルを経由する必要がある
                // TODO: より効率的な実装（ring_bufferにイテレータを追加）
                let temp_file_path = std::env::temp_dir().join(format!("prebuffer_{}.mjpeg", now.format("%Y%m%d_%H%M%S")));
                let mut temp_file = File::create(&temp_file_path)?;
                let (frames, bytes) = self.ring_buffer.flush_to_file(&mut temp_file)?;
                drop(temp_file);

                // TODO: MJPEGファイルを読み込んで個別フレームとしてMP4に書き込む処理
                // 現在の実装では、プリバッファはスキップ（MP4の場合）
                warn!("MP4 motion recording: pre-buffer not yet implemented, starting from current frame");

                std::fs::remove_file(temp_file_path)?;

                info!("Started motion MP4 recording to: {:?}", filepath);
                self.mp4_recorder = Some(recorder);
                (0, 0)  // プリバッファは未実装
            }
        };

        // Update state
        self.recording_state = RecordingState::MotionRecording {
            filepath: filepath.clone(),
            start_time: Instant::now(),
            frame_count: pre_frames as u32,
            total_bytes: pre_bytes as u64,
            motion_active: true,
            countdown_frames: self.motion_config.post_record_seconds * 11,  // 11 fps
            format: self.recording_format,
        };

        self.is_recording.store(true, Ordering::Relaxed);
        self.last_motion_time = Some(Instant::now());

        Ok(())
    }

    fn stop_recording(&mut self) -> io::Result<()> {
        // Check if recording (manual or motion)
        match &self.recording_state {
            RecordingState::ManualRecording { filepath, start_time, frame_count, total_bytes, format } |
            RecordingState::MotionRecording { filepath, start_time, frame_count, total_bytes, format, .. } => {
                let duration = start_time.elapsed();
                let is_motion = matches!(self.recording_state, RecordingState::MotionRecording { .. });

                info!("Stopped {} recording: {:?}", if is_motion { "motion" } else { "manual" }, filepath);
                info!("  Duration: {:.1}s", duration.as_secs_f32());
                info!("  Frames: {}", frame_count);
                info!("  Size: {:.2} MB", *total_bytes as f32 / 1_000_000.0);

                // Phase 6: Close recorder based on format
                match format {
                    RecordingFormat::Mjpeg => {
                        self.recording_file = None;
                    }
                    RecordingFormat::Mp4 => {
                        if let Some(recorder) = self.mp4_recorder.take() {
                            recorder.finish()?;
                        }
                    }
                }

                // Update state
                self.recording_state = RecordingState::Idle;
                self.is_recording.store(false, Ordering::Relaxed);
            }
            RecordingState::Idle => {
                warn!("No recording in progress");
            }
        }

        Ok(())
    }

    fn write_frame(&mut self, jpeg_data: &[u8]) -> io::Result<()> {
        // Check if recording (manual or motion)
        match &mut self.recording_state {
            RecordingState::ManualRecording { total_bytes, frame_count, format, .. } |
            RecordingState::MotionRecording { total_bytes, frame_count, format, .. } => {
                // Check size limit
                if *total_bytes + jpeg_data.len() as u64 > MAX_RECORDING_SIZE {
                    warn!("Recording size limit reached ({} MB), stopping", MAX_RECORDING_SIZE / 1_000_000);
                    self.stop_recording()?;
                    return Ok(());
                }

                // Phase 6: Write to appropriate recorder based on format
                match format {
                    RecordingFormat::Mjpeg => {
                        // Write JPEG data to MJPEG file
                        if let Some(ref file) = self.recording_file {
                            let mut file_guard = file.lock().unwrap();
                            file_guard.write_all(jpeg_data)?;
                            // Note: flush() removed to reduce GUI thread blocking
                            // File will be flushed automatically on close or periodically by OS
                        }
                    }
                    RecordingFormat::Mp4 => {
                        // Write JPEG frame to MP4 encoder
                        if let Some(ref mut recorder) = self.mp4_recorder {
                            recorder.write_frame(jpeg_data)?;
                        }
                    }
                }

                // Update counters
                *total_bytes += jpeg_data.len() as u64;
                *frame_count += 1;
            }
            RecordingState::Idle => {
                // Not recording, do nothing
            }
        }

        Ok(())
    }

    fn process_messages(&mut self, ctx: &egui::Context) {
        // Process all pending messages
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AppMessage::NewFrame(jpeg_data) => {
                    // Legacy path - decode JPEG in GUI thread (slower)
                    if let Ok(img) = image::load_from_memory(&jpeg_data) {
                        let size = [img.width() as usize, img.height() as usize];
                        let pixels = img.to_rgba8();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_raw(),
                        );

                        if let Some(texture) = &mut self.current_frame {
                            texture.set(color_image, egui::TextureOptions::LINEAR);
                        } else {
                            self.current_frame = Some(ctx.load_texture(
                                "camera_frame",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                }
                AppMessage::DecodedFrame { width, height, pixels } => {
                    // Fast path - receive pre-decoded RGBA data
                    let size = [width as usize, height as usize];
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        &pixels,
                    );

                    if let Some(texture) = &mut self.current_frame {
                        texture.set(color_image, egui::TextureOptions::LINEAR);
                    } else {
                        self.current_frame = Some(ctx.load_texture(
                            "camera_frame",
                            color_image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }

                    // Phase 5: Motion detection
                    if self.motion_config.enabled {
                        use image::RgbaImage;

                        // Convert pixels Vec<u8> to RgbaImage
                        if let Some(rgba_img) = RgbaImage::from_raw(width, height, pixels) {
                            // Detect motion
                            let motion_detected = self.motion_detector.detect(&rgba_img);

                            // Handle motion detection states
                            match &mut self.recording_state {
                                RecordingState::Idle => {
                                    if motion_detected {
                                        // Start motion recording
                                        if let Err(e) = self.start_motion_recording() {
                                            error!("Failed to start motion recording: {}", e);
                                        }
                                    }
                                }
                                RecordingState::MotionRecording { motion_active, countdown_frames, .. } => {
                                    if motion_detected {
                                        // Motion continues - reset countdown
                                        *motion_active = true;
                                        *countdown_frames = self.motion_config.post_record_seconds * 11;
                                        self.last_motion_time = Some(Instant::now());
                                    } else {
                                        // No motion - countdown
                                        *motion_active = false;
                                        if *countdown_frames > 0 {
                                            *countdown_frames -= 1;
                                        } else {
                                            // Countdown finished - stop recording
                                            if let Err(e) = self.stop_recording() {
                                                error!("Failed to stop motion recording: {}", e);
                                            }
                                        }
                                    }
                                }
                                RecordingState::ManualRecording { .. } => {
                                    // Manual recording in progress - don't interfere
                                }
                            }
                        }
                    }
                }
                AppMessage::ConnectionStatus(status) => {
                    self.connection_status = status;
                }
                AppMessage::Stats { fps, spresense_fps, frame_count, errors, decode_time_ms, serial_read_time_ms, texture_upload_time_ms, jpeg_size_kb } => {
                    self.fps = fps;
                    self.spresense_fps = spresense_fps;
                    self.frame_count = frame_count;
                    self.error_count = errors;
                    self.decode_time_ms = decode_time_ms;
                    self.serial_read_time_ms = serial_read_time_ms;
                    self.texture_upload_time_ms = texture_upload_time_ms;
                    self.jpeg_size_kb = jpeg_size_kb;
                }
                AppMessage::SpresenseMetrics { timestamp_ms: _, camera_frames, camera_fps, usb_packets: _, action_q_depth, avg_packet_size: _, errors } => {
                    // Phase 4.1: Update Spresense-side metrics
                    self.spresense_camera_frames = Some(camera_frames);
                    self.spresense_camera_fps = Some(camera_fps);
                    self.spresense_action_q_depth = Some(action_q_depth);
                    self.spresense_errors = Some(errors);
                }
                AppMessage::JpegFrame(jpeg_data) => {
                    // Phase 5: Add to ring buffer (if motion detection enabled)
                    if self.motion_config.enabled {
                        self.ring_buffer.push(JpegFrame {
                            jpeg_data: jpeg_data.clone(),
                            timestamp: Instant::now(),
                        });
                    }

                    // Phase 3/5: Write JPEG frame to recording file
                    if let Err(e) = self.write_frame(&jpeg_data) {
                        error!("Failed to write recording frame: {}", e);
                    }
                }
            }
        }
    }
}

impl eframe::App for CameraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Phase 5: Sync motion detector config
        self.motion_detector.update_config(self.motion_config.clone());

        // Update ring buffer capacity if pre_record_seconds changed
        let expected_capacity = (self.motion_config.pre_record_seconds * 11) as usize;
        if self.ring_buffer.capacity() != expected_capacity {
            self.ring_buffer = RingBuffer::from_seconds(self.motion_config.pre_record_seconds, 11);
        }

        // Process incoming messages
        self.process_messages(ctx);

        // Request continuous repaint for smooth video
        ctx.request_repaint();

        // Top panel - Controls
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("📷 Spresense Security Camera Viewer");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_running = *self.is_running.lock().unwrap();

                    // Capture controls
                    if is_running {
                        if ui.button("⏹ Stop").clicked() {
                            self.stop_capture();
                        }
                    } else {
                        if ui.button("▶ Start").clicked() {
                            self.start_capture();
                        }
                    }

                    ui.separator();

                    // Phase 3/5: Recording controls
                    let is_recording = matches!(self.recording_state,
                        RecordingState::ManualRecording { .. } | RecordingState::MotionRecording { .. });

                    if is_recording {
                        if ui.button("⏺ Stop Rec").clicked() {
                            if let Err(e) = self.stop_recording() {
                                error!("Failed to stop recording: {}", e);
                            }
                        }

                        // Display recording status
                        match &self.recording_state {
                            RecordingState::ManualRecording { start_time, frame_count, total_bytes, .. } => {
                                let duration = start_time.elapsed().as_secs();
                                let size_mb = *total_bytes as f32 / 1_000_000.0;
                                ui.label(format!("🔴 MANUAL {}:{:02} | {:.1}MB | {} frames",
                                               duration / 60, duration % 60, size_mb, frame_count));
                            }
                            RecordingState::MotionRecording { start_time, frame_count, total_bytes, motion_active, countdown_frames, .. } => {
                                let duration = start_time.elapsed().as_secs();
                                let size_mb = *total_bytes as f32 / 1_000_000.0;
                                let motion_indicator = if *motion_active { "🔴 MOTION" } else { "⏱️  POST" };
                                ui.label(format!("{} {}:{:02} | {:.1}MB | {} frames | {}f left",
                                               motion_indicator, duration / 60, duration % 60, size_mb, frame_count, countdown_frames));
                            }
                            _ => {}
                        }
                    } else {
                        // Phase 6: Recording format selector
                        ui.label("Format:");
                        ui.radio_value(&mut self.recording_format, RecordingFormat::Mp4, "MP4");
                        ui.radio_value(&mut self.recording_format, RecordingFormat::Mjpeg, "MJPEG");

                        if ui.button("⏺ Start Rec").clicked() {
                            if let Err(e) = self.start_manual_recording() {
                                error!("Failed to start recording: {}", e);
                            }
                        }
                    }

                    ui.separator();

                    ui.label(format!("Status: {}", self.connection_status));
                });
            });
        });

        // Bottom panel - Statistics
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("📊 PC: {:.1} fps", self.fps));
                ui.separator();
                ui.label(format!("📡 Spresense: {:.1} fps", self.spresense_fps));
                ui.separator();
                ui.label(format!("🎬 Frames: {}", self.frame_count));
                ui.separator();
                ui.label(format!("❌ Errors: {}", self.error_count));
                ui.separator();
                ui.label(format!("⏱ Decode: {:.1}ms", self.decode_time_ms));
                ui.separator();
                ui.label(format!("📨 Serial: {:.1}ms", self.serial_read_time_ms));
                ui.separator();
                ui.label(format!("🖼 Texture: {:.1}ms", self.texture_upload_time_ms));
                ui.separator();
                ui.label(format!("📦 JPEG: {:.1}KB", self.jpeg_size_kb));
                ui.separator();

                // Phase 4.1: Spresense-side metrics from Metrics packets
                if let Some(cam_fps) = self.spresense_camera_fps {
                    ui.label(format!("📷 Cam FPS: {:.1}", cam_fps));
                } else {
                    ui.label("📷 Cam FPS: --");
                }
                ui.separator();
                if let Some(q_depth) = self.spresense_action_q_depth {
                    ui.label(format!("📊 Q Depth: {}", q_depth));
                } else {
                    ui.label("📊 Q Depth: --");
                }
                ui.separator();
                if let Some(errors) = self.spresense_errors {
                    ui.label(format!("⚠ Sp Errors: {}", errors));
                } else {
                    ui.label("⚠ Sp Errors: --");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/");
                });
            });
        });

        // Side panel - Settings
        egui::SidePanel::left("side_panel").max_width(250.0).show(ctx, |ui| {
            ui.heading("⚙ Settings");
            ui.separator();

            // Phase 7: Transport type selection
            ui.label("Connection Type:");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.transport_type, TransportType::UsbSerial, "USB Serial");
                ui.radio_value(&mut self.transport_type, TransportType::WiFi, "WiFi");
            });

            ui.add_space(5.0);

            // USB Serial settings
            if self.transport_type == TransportType::UsbSerial {
                ui.checkbox(&mut self.auto_detect, "Auto-detect Spresense");
                if !self.auto_detect {
                    ui.label("Serial Port:");
                    ui.text_edit_singleline(&mut self.port_path);
                }
            }
            // WiFi settings
            else if self.transport_type == TransportType::WiFi {
                ui.label("Spresense IP Address:");
                ui.text_edit_singleline(&mut self.wifi_host);

                ui.add_space(3.0);
                ui.label("Port:");
                ui.add(egui::DragValue::new(&mut self.wifi_port).clamp_range(1..=65535));
            }

            ui.separator();

            if let Some(texture) = &self.current_frame {
                ui.label(format!("Resolution: {}x{}", texture.size()[0], texture.size()[1]));
            }

            ui.separator();

            // Phase 5: Motion Detection Settings
            ui.heading("🔍 Motion Detection");
            ui.separator();

            ui.checkbox(&mut self.motion_config.enabled, "Enable Motion Detection");

            if self.motion_config.enabled {
                ui.add_space(5.0);

                // Sensitivity slider
                ui.label("Sensitivity:");
                ui.add(egui::Slider::new(&mut self.motion_config.sensitivity, 0.0..=1.0)
                    .text(""));
                ui.label(format!("  {:.0}%", self.motion_config.sensitivity * 100.0));

                ui.add_space(5.0);

                // Minimum motion area
                ui.label("Min Motion Area:");
                ui.add(egui::Slider::new(&mut self.motion_config.min_motion_area, 0.1..=10.0)
                    .text("%"));

                ui.add_space(5.0);

                // Pre-record seconds
                ui.label("Pre-record (sec):");
                ui.add(egui::Slider::new(&mut self.motion_config.pre_record_seconds, 5..=30)
                    .text("s"));

                ui.add_space(5.0);

                // Post-record seconds
                ui.label("Post-record (sec):");
                ui.add(egui::Slider::new(&mut self.motion_config.post_record_seconds, 10..=60)
                    .text("s"));

                ui.add_space(5.0);

                // Motion detector stats
                let stats = self.motion_detector.stats();
                if stats.total_frames > 0 {
                    ui.label(format!("📊 Detection: {:.1}%", stats.detection_rate));
                }

                // Ring buffer status
                ui.label(format!("💾 Buffer: {}/{} frames ({:.1}%)",
                    self.ring_buffer.len(),
                    self.ring_buffer.capacity(),
                    self.ring_buffer.usage_ratio() * 100.0));

                if let Some(age) = self.ring_buffer.oldest_frame_age_secs() {
                    ui.label(format!("⏱️  Oldest: {:.1}s ago", age));
                }
            }

            ui.separator();
            ui.label("💡 Tips:");
            if self.transport_type == TransportType::UsbSerial {
                ui.label("• Connect Spresense via USB");
            } else {
                ui.label("• Connect Spresense to WiFi");
                ui.label("• Enter Spresense IP address");
            }
            ui.label("• Click Start to begin");
            ui.label("• Motion rec = auto start");
        });

        // Central panel - Video display
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if let Some(texture) = &self.current_frame {
                    // Calculate scaling to fit panel
                    let available_size = ui.available_size();
                    let img_size = texture.size_vec2();
                    let scale = (available_size.x / img_size.x).min(available_size.y / img_size.y);
                    let display_size = img_size * scale * 0.95; // 95% to leave some margin

                    ui.add(egui::Image::new(texture).fit_to_exact_size(display_size));
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading("No camera feed");
                        ui.label("Click 'Start' to begin capturing");
                        ui.add_space(20.0);
                        ui.label("📷");
                    });
                }
            });
        });
    }
}

// Phase 7: Connection abstraction for Serial/TCP
enum Connection {
    Serial(SerialConnection),
    Tcp(TcpConnection),
}

impl Connection {
    fn read_packet(&mut self) -> io::Result<Packet> {
        match self {
            Connection::Serial(serial) => serial.read_packet(),
            Connection::Tcp(tcp) => {
                // Read raw packet from TCP
                let mut buffer = vec![0u8; 150_000];
                let size = tcp.read_packet(&mut buffer)?;
                buffer.truncate(size);

                // Parse packet based on sync word
                if size < 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Packet too small",
                    ));
                }

                use byteorder::{ByteOrder, LittleEndian};
                let sync_word = LittleEndian::read_u32(&buffer[0..4]);

                match sync_word {
                    protocol::SYNC_WORD => {
                        // MJPEG packet
                        let mjpeg_packet = protocol::MjpegPacket::parse(&buffer)?;
                        Ok(Packet::Mjpeg(mjpeg_packet))
                    }
                    protocol::METRICS_SYNC_WORD => {
                        // Metrics packet
                        let metrics_packet = protocol::MetricsPacket::parse(&buffer)?;
                        Ok(Packet::Metrics(metrics_packet))
                    }
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unknown sync word: 0x{:08X}", sync_word),
                    )),
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Connection::Serial(serial) => serial.flush(),
            Connection::Tcp(_) => Ok(()), // TCP doesn't need flush
        }
    }

    fn connection_info(&self) -> String {
        match self {
            Connection::Serial(_) => "USB Serial".to_string(),
            Connection::Tcp(tcp) => tcp.connection_info(),
        }
    }
}

fn capture_thread(
    tx: Sender<AppMessage>,
    is_running: Arc<Mutex<bool>>,
    is_recording: Arc<AtomicBool>,
    port_path: String,
    auto_detect: bool,
    transport_type: TransportType,  // Phase 7: WiFi support
    wifi_host: String,
    wifi_port: u16,
) {
    info!("Capture thread started (transport: {:?})", transport_type);

    // Phase 7: Connect based on transport type
    let mut connection = match transport_type {
        TransportType::UsbSerial => {
            // USB Serial connection (Phase 1-6)
            let serial = if auto_detect {
                tx.send(AppMessage::ConnectionStatus("Connecting (auto-detect)...".to_string())).ok();
                match SerialConnection::auto_detect() {
                    Ok(s) => {
                        tx.send(AppMessage::ConnectionStatus("Connected".to_string())).ok();
                        s
                    }
                    Err(e) => {
                        error!("Failed to auto-detect: {}", e);
                        tx.send(AppMessage::ConnectionStatus(format!("Error: {}", e))).ok();
                        return;
                    }
                }
            } else {
                tx.send(AppMessage::ConnectionStatus(format!("Connecting to {}...", port_path))).ok();
                match SerialConnection::open(&port_path, 115200) {
                    Ok(s) => {
                        tx.send(AppMessage::ConnectionStatus("Connected".to_string())).ok();
                        s
                    }
                    Err(e) => {
                        error!("Failed to open port: {}", e);
                        tx.send(AppMessage::ConnectionStatus(format!("Error: {}", e))).ok();
                        return;
                    }
                }
            };
            Connection::Serial(serial)
        }
        TransportType::WiFi => {
            // WiFi TCP connection (Phase 7)
            tx.send(AppMessage::ConnectionStatus(format!("Connecting to {}:{}...", wifi_host, wifi_port))).ok();
            match TcpConnection::new(&wifi_host, wifi_port) {
                Ok(tcp) => {
                    let info = tcp.connection_info();
                    tx.send(AppMessage::ConnectionStatus(format!("Connected: {}", info))).ok();
                    info!("TCP connected: {}", info);
                    Connection::Tcp(tcp)
                }
                Err(e) => {
                    error!("Failed to connect to TCP server: {}", e);
                    tx.send(AppMessage::ConnectionStatus(format!("Error: {}", e))).ok();
                    return;
                }
            }
        }
    };

    // Flush buffer
    if let Err(e) = connection.flush() {
        error!("Failed to flush: {}", e);
    }

    // Initialize metrics logger
    let metrics_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("metrics");

    let metrics_logger = match MetricsLogger::new(metrics_dir.to_str().unwrap_or("metrics")) {
        Ok(logger) => {
            info!("📊 Metrics logging to: {:?}", logger.path());
            tx.send(AppMessage::ConnectionStatus(
                format!("Logging to: {:?}", logger.path())
            )).ok();
            Some(logger)
        }
        Err(e) => {
            error!("Failed to create metrics logger: {}", e);
            tx.send(AppMessage::ConnectionStatus(
                format!("Warning: Metrics logging disabled ({})", e)
            )).ok();
            None
        }
    };

    // Initialize Spresense FPS calculator (30-frame window)
    let mut spresense_fps_calc = SpresenseFpsCalculator::new(30);

    // Initialize Spresense Camera FPS calculator (from Metrics packets)
    let mut spresense_camera_fps_calc = SpresenseCameraFpsCalculator::new();

    let mut frame_count = 0u64;

    // Phase 4.1.1: Separate error counters for better diagnostics
    let mut packet_error_count = 0u32;        // Serial packet read errors
    let mut jpeg_decode_error_count = 0u32;   // JPEG decode errors
    let mut consecutive_jpeg_errors = 0u32;   // Consecutive JPEG errors
    let mut last_stats_time = Instant::now();
    let mut frames_since_last_stats = 0u32;

    // Performance tracking
    let mut total_decode_time_ms = 0.0f32;
    let mut total_serial_read_time_ms = 0.0f32;
    let mut total_jpeg_size_bytes = 0u64;
    let mut avg_decode_time_ms = 0.0f32;
    let mut avg_serial_read_time_ms = 0.0f32;
    let mut avg_jpeg_size_kb = 0.0f32;
    let mut avg_spresense_fps = 0.0f32;

    // Phase 4.1: Spresense-side metrics (latest values from Metrics packets)
    let mut spresense_camera_frames = 0u32;
    let mut spresense_camera_fps = 0.0f32;
    let mut spresense_usb_packets = 0u32;
    let mut spresense_action_q_depth = 0u32;
    let mut spresense_errors = 0u32;

    while *is_running.lock().unwrap() {
        // Measure read time (serial or TCP)
        let read_start = Instant::now();
        let read_result = connection.read_packet();
        let serial_read_time_ms = read_start.elapsed().as_secs_f32() * 1000.0;

        match read_result {
            Ok(Packet::Mjpeg(packet)) => {
                // MJPEG packet - process as video frame
                // Reset packet error count on successful read
                packet_error_count = 0;
                frame_count += 1;
                frames_since_last_stats += 1;

                // Update Spresense FPS from packet sequence number
                let current_spresense_fps = spresense_fps_calc.update(packet.header.sequence);

                // Debug: Log sequence number for first few frames
                if frame_count <= 5 {
                    info!("Frame {}: sequence={}, spresense_fps={:.1}",
                          frame_count, packet.header.sequence, current_spresense_fps);
                }

                // Accumulate serial read time and JPEG size
                total_serial_read_time_ms += serial_read_time_ms;
                let jpeg_size_bytes = packet.jpeg_data.len();
                total_jpeg_size_bytes += jpeg_size_bytes as u64;

                // Phase 3: Send JPEG data for recording ONLY when recording is active
                // This prevents message queue congestion and Metrics packet delay
                if is_recording.load(Ordering::Relaxed) {
                    tx.send(AppMessage::JpegFrame(packet.jpeg_data.clone())).ok();
                }

                // Option A: Decode JPEG in capture thread (not GUI thread)
                let decode_start = Instant::now();
                match image::load_from_memory(&packet.jpeg_data) {
                    Ok(img) => {
                        // Phase 4.1.1: Reset consecutive JPEG errors on success
                        consecutive_jpeg_errors = 0;

                        let decode_time_ms = decode_start.elapsed().as_secs_f32() * 1000.0;
                        total_decode_time_ms += decode_time_ms;

                        // Convert to RGBA8
                        let rgba = img.to_rgba8();
                        let width = img.width();
                        let height = img.height();
                        let pixels = rgba.into_raw();

                        // Send pre-decoded frame to GUI (fast path - no decode in GUI thread)
                        tx.send(AppMessage::DecodedFrame {
                            width,
                            height,
                            pixels,
                        }).ok();
                    }
                    Err(e) => {
                        // Phase 4.1.1: Enhanced JPEG decode error handling
                        error!("Failed to decode JPEG: {}", e);

                        // Update error counters
                        jpeg_decode_error_count += 1;
                        consecutive_jpeg_errors += 1;

                        // Warn on consecutive errors
                        if consecutive_jpeg_errors == 5 {
                            warn!("5 consecutive JPEG decode errors detected - possible Spresense compression issue");
                        } else if consecutive_jpeg_errors >= 10 {
                            error!("10+ consecutive JPEG decode errors - check Spresense JPEG encoder");
                        }

                        // Skip this frame (do not send to GUI, previous frame remains displayed)
                        // This allows continuous operation despite JPEG errors
                    }
                }

                // Update statistics every second
                let now = Instant::now();
                let elapsed = now.duration_since(last_stats_time).as_secs_f32();
                if elapsed >= 1.0 {
                    let fps = frames_since_last_stats as f32 / elapsed;

                    // Calculate averages
                    avg_decode_time_ms = total_decode_time_ms / frames_since_last_stats as f32;
                    avg_serial_read_time_ms = total_serial_read_time_ms / frames_since_last_stats as f32;
                    avg_jpeg_size_kb = (total_jpeg_size_bytes as f32 / frames_since_last_stats as f32) / 1024.0;
                    avg_spresense_fps = spresense_fps_calc.current_fps();

                    // Debug: Log stats calculation
                    info!("Stats: PC FPS={:.1}, Spresense FPS={:.1}, Frames={}",
                          fps, avg_spresense_fps, frame_count);

                    tx.send(AppMessage::Stats {
                        fps,
                        spresense_fps: avg_spresense_fps,
                        frame_count,
                        errors: jpeg_decode_error_count,  // Phase 4.1.1: Show JPEG decode errors only
                        decode_time_ms: avg_decode_time_ms,
                        serial_read_time_ms: avg_serial_read_time_ms,
                        texture_upload_time_ms: 0.0,  // Measured in GUI thread
                        jpeg_size_kb: avg_jpeg_size_kb,
                    }).ok();

                    // Log metrics to CSV (Phase 4.1: Added Spresense-side metrics)
                    if let Some(ref logger) = metrics_logger {
                        let metrics = PerformanceMetrics {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs_f64(),
                            pc_fps: fps,
                            spresense_fps: avg_spresense_fps,
                            frame_count,
                            error_count: jpeg_decode_error_count,  // Phase 4.1.1: JPEG decode errors
                            decode_time_ms: avg_decode_time_ms,
                            serial_read_time_ms: avg_serial_read_time_ms,
                            texture_upload_time_ms: 0.0,
                            jpeg_size_kb: avg_jpeg_size_kb,
                            // Phase 4.1: Spresense-side metrics from Metrics packets
                            spresense_camera_frames,
                            spresense_camera_fps,
                            spresense_usb_packets,
                            action_q_depth: spresense_action_q_depth,
                            spresense_errors,
                        };

                        if let Err(e) = logger.log(&metrics) {
                            error!("Failed to log metrics: {}", e);
                        }
                    }

                    // Reset accumulators
                    frames_since_last_stats = 0;
                    total_decode_time_ms = 0.0;
                    total_serial_read_time_ms = 0.0;
                    total_jpeg_size_bytes = 0;
                    last_stats_time = now;
                }
            }
            Ok(Packet::Metrics(metrics)) => {
                // Phase 4.1: Metrics packet - update Spresense-side metrics
                packet_error_count = 0;  // Reset on successful read

                // Calculate Spresense camera FPS from Metrics packet
                let camera_fps = spresense_camera_fps_calc.update(metrics.timestamp_ms, metrics.camera_frames);

                // Store latest Spresense metrics for CSV logging
                spresense_camera_frames = metrics.camera_frames;
                spresense_camera_fps = camera_fps;
                spresense_usb_packets = metrics.usb_packets;
                spresense_action_q_depth = metrics.action_q_depth;
                spresense_errors = metrics.errors;

                info!("Received Spresense metrics: frames={}, fps={:.1}, usb_pkts={}, q_depth={}, errors={}",
                      metrics.camera_frames, camera_fps, metrics.usb_packets,
                      metrics.action_q_depth, metrics.errors);

                tx.send(AppMessage::SpresenseMetrics {
                    timestamp_ms: metrics.timestamp_ms,
                    camera_frames: metrics.camera_frames,
                    camera_fps,
                    usb_packets: metrics.usb_packets,
                    action_q_depth: metrics.action_q_depth,
                    avg_packet_size: metrics.avg_packet_size,
                    errors: metrics.errors,
                }).ok();
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::TimedOut {
                    // Phase 4.1.1: Track packet read errors separately
                    packet_error_count += 1;
                    error!("Packet read error: {}", e);

                    if packet_error_count >= 10 {
                        error!("Too many consecutive packet errors ({}), stopping capture thread", packet_error_count);
                        tx.send(AppMessage::ConnectionStatus("Too many packet errors".to_string())).ok();
                        break;
                    }

                    // Brief pause before retry to allow device to recover
                    std::thread::sleep(std::time::Duration::from_millis(10));
                } else {
                    // Timeout is not counted as an error (device may be slow)
                    // Continue to next iteration
                }
            }
        }
    }

    info!("Capture thread stopped");
    tx.send(AppMessage::ConnectionStatus("Stopped".to_string())).ok();
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])  // Optimized for VGA 640×480 display
            .with_min_inner_size([960.0, 640.0]),  // Minimum to fit VGA comfortably
        ..Default::default()
    };

    eframe::run_native(
        "Spresense Security Camera",
        options,
        Box::new(|cc| Box::new(CameraApp::new(cc))),
    )
}
