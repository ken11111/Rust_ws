mod protocol;
mod serial;
mod metrics;

use eframe::egui;
use log::{error, info, warn};
use serial::SerialConnection;
use metrics::{MetricsLogger, PerformanceMetrics, SpresenseFpsCalculator};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

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
}

struct CameraApp {
    // Communication
    rx: Receiver<AppMessage>,
    tx: Sender<AppMessage>,

    // State
    current_frame: Option<egui::TextureHandle>,
    connection_status: String,
    is_running: Arc<Mutex<bool>>,

    // Statistics
    fps: f32,
    spresense_fps: f32,
    frame_count: u64,
    error_count: u32,
    decode_time_ms: f32,
    serial_read_time_ms: f32,
    texture_upload_time_ms: f32,
    jpeg_size_kb: f32,

    // Settings
    port_path: String,
    auto_detect: bool,
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
            fps: 0.0,
            spresense_fps: 0.0,
            frame_count: 0,
            error_count: 0,
            decode_time_ms: 0.0,
            serial_read_time_ms: 0.0,
            texture_upload_time_ms: 0.0,
            jpeg_size_kb: 0.0,
            port_path: "/dev/ttyACM0".to_string(),
            auto_detect: true,
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
        let port_path = self.port_path.clone();
        let auto_detect = self.auto_detect;

        thread::spawn(move || {
            capture_thread(tx, is_running, port_path, auto_detect);
        });
    }

    fn stop_capture(&mut self) {
        *self.is_running.lock().unwrap() = false;
        self.connection_status = "Stopped".to_string();
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
            }
        }
    }
}

impl eframe::App for CameraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/");
                });
            });
        });

        // Side panel - Settings
        egui::SidePanel::left("side_panel").max_width(250.0).show(ctx, |ui| {
            ui.heading("⚙ Settings");
            ui.separator();

            ui.checkbox(&mut self.auto_detect, "Auto-detect Spresense");

            if !self.auto_detect {
                ui.label("Serial Port:");
                ui.text_edit_singleline(&mut self.port_path);
            }

            ui.separator();

            if let Some(texture) = &self.current_frame {
                ui.label(format!("Resolution: {}x{}", texture.size()[0], texture.size()[1]));
            }

            ui.separator();
            ui.label("💡 Tips:");
            ui.label("• Connect Spresense via USB");
            ui.label("• Click Start to begin");
            ui.label("• Press Stop to pause");
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

fn capture_thread(
    tx: Sender<AppMessage>,
    is_running: Arc<Mutex<bool>>,
    port_path: String,
    auto_detect: bool,
) {
    info!("Capture thread started");

    // Connect to serial port
    let mut serial = if auto_detect {
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

    // Flush buffer
    if let Err(e) = serial.flush() {
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

    let mut frame_count = 0u64;
    let mut error_count = 0u32;
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

    while *is_running.lock().unwrap() {
        // Measure serial read time
        let read_start = Instant::now();
        let read_result = serial.read_packet();
        let serial_read_time_ms = read_start.elapsed().as_secs_f32() * 1000.0;

        match read_result {
            Ok(packet) => {
                // Reset consecutive error count on successful read
                error_count = 0;
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

                // Option A: Decode JPEG in capture thread (not GUI thread)
                let decode_start = Instant::now();
                match image::load_from_memory(&packet.jpeg_data) {
                    Ok(img) => {
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
                        error!("Failed to decode JPEG: {}", e);
                        error_count += 1;
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
                        errors: error_count,
                        decode_time_ms: avg_decode_time_ms,
                        serial_read_time_ms: avg_serial_read_time_ms,
                        texture_upload_time_ms: 0.0,  // Measured in GUI thread
                        jpeg_size_kb: avg_jpeg_size_kb,
                    }).ok();

                    // Log metrics to CSV
                    if let Some(ref logger) = metrics_logger {
                        let metrics = PerformanceMetrics {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs_f64(),
                            pc_fps: fps,
                            spresense_fps: avg_spresense_fps,
                            frame_count,
                            error_count,
                            decode_time_ms: avg_decode_time_ms,
                            serial_read_time_ms: avg_serial_read_time_ms,
                            texture_upload_time_ms: 0.0,
                            jpeg_size_kb: avg_jpeg_size_kb,
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
            Err(e) => {
                if e.kind() != std::io::ErrorKind::TimedOut {
                    error_count += 1;
                    error!("Packet read error (after recovery attempts): {}", e);

                    if error_count >= 10 {
                        error!("Too many consecutive errors ({}), stopping capture thread", error_count);
                        tx.send(AppMessage::ConnectionStatus("Too many errors".to_string())).ok();
                        break;
                    }

                    // Brief pause before retry to allow device to recover
                    std::thread::sleep(std::time::Duration::from_millis(10));
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
