/// Phase 8: PC側パイプライン最適化モジュール
///
/// 3スレッドパイプラインアーキテクチャ:
/// - TCP Reader Thread: パケット読み込み専用 (200ms/frame)
/// - JPEG Decoder Thread: デコード専用 (40ms/frame)
/// - GUI Thread: テクスチャ更新専用 (20ms/frame)
///
/// パイプライン処理により、各段階が並列動作可能:
/// - Frame N: TCP読み込み中
/// - Frame N-1: JPEGデコード中
/// - Frame N-2: GUI表示中
///
/// 期待効果:
/// - スループット: 4.2 fps → 5.0 fps (+19%)
/// - レイテンシ: 240ms → 200ms (-17%)

use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError, RecvTimeoutError};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::io;
use log::{info, warn, error, debug};
use byteorder::{ByteOrder, LittleEndian};

use crate::protocol::{Packet, MjpegPacket, MetricsPacket, BatchPacket};
use crate::serial::SerialConnection;
use crate::tcp_connection::{TcpConnection, ConnectionState};

/// パイプラインチャネル容量
const PACKET_CHANNEL_CAPACITY: usize = 3;  // TCP→Decoder (約150KB)

/// TCP/Serial接続の抽象化
pub enum Connection {
    Serial(SerialConnection),
    Tcp(TcpConnection),
}

impl Connection {
    /// パケットを読み込み (ブロッキング)
    /// Phase 9: TCP接続の場合は自動再接続対応
    pub fn read_packet(&mut self) -> io::Result<Packet> {
        match self {
            Connection::Serial(serial) => serial.read_packet(),
            Connection::Tcp(tcp) => {
                let mut buffer = vec![0u8; 250_000];

                // Phase 9: 自動再接続対応読み込み
                let size = tcp.read_packet_with_reconnect(&mut buffer)?;
                buffer.truncate(size);

                if size < 4 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Packet too small",
                    ));
                }

                let sync_word = LittleEndian::read_u32(&buffer[0..4]);

                match sync_word {
                    crate::protocol::SYNC_WORD => {
                        let packet = MjpegPacket::parse(&buffer)?;
                        Ok(Packet::Mjpeg(packet))
                    }
                    crate::protocol::METRICS_SYNC_WORD => {
                        let packet = MetricsPacket::parse(&buffer)?;
                        Ok(Packet::Metrics(packet))
                    }
                    crate::protocol::MJPEG_BATCH_SYNC_WORD => {
                        let packet = BatchPacket::parse(&buffer)?;
                        Ok(Packet::Batch(packet))
                    }
                    _ => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unknown sync word: 0x{:08X}", sync_word),
                    )),
                }
            }
        }
    }

    /// バッファをフラッシュ
    pub fn flush(&mut self) -> io::Result<()> {
        match self {
            Connection::Serial(serial) => serial.flush(),
            Connection::Tcp(_) => Ok(()),
        }
    }

    /// 接続情報を取得
    pub fn connection_info(&self) -> String {
        match self {
            Connection::Serial(_) => "USB Serial".to_string(),
            Connection::Tcp(tcp) => tcp.connection_info(),
        }
    }

    /// Phase 9: 接続状態を取得
    pub fn connection_state(&self) -> ConnectionState {
        match self {
            Connection::Serial(_) => ConnectionState::Connected,  // Serialは常時接続扱い
            Connection::Tcp(tcp) => tcp.connection_state(),
        }
    }

    /// Phase 9: 再接続回数を取得
    pub fn reconnect_count(&self) -> u32 {
        match self {
            Connection::Serial(_) => 0,  // Serialは再接続なし
            Connection::Tcp(tcp) => tcp.reconnect_count(),
        }
    }
}

/// パケットチャネル用構造体 (TCP→Decoder)
#[derive(Debug)]
pub struct RawPacket {
    pub packet: Packet,
    pub read_time_ms: f32,
    pub timestamp: Instant,
}

/// デコード済みフレーム構造体 (Decoder→GUI)
#[derive(Debug)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,  // RGBA8
    pub sequence: u32,
    pub jpeg_size: usize,
    pub decode_time_ms: f32,
    pub read_time_ms: f32,
    pub timestamp: Instant,
}

/// Metricsメッセージ (パイプライン経由)
#[derive(Debug, Clone)]
pub struct MetricsMessage {
    pub timestamp_ms: u32,
    pub camera_frames: u32,
    pub camera_fps: f32,
    pub usb_packets: u32,
    pub action_q_depth: u32,
    pub avg_packet_size: u32,
    pub errors: u32,
    pub tcp_avg_send_us: u32,
    pub tcp_max_send_us: u32,
    pub dropped_frames: u32,
    pub drop_events: u32,
    // Phase 9.2: TCP health metrics
    pub tcp_health_moving_avg_ms: u32,
    pub tcp_health_total_spikes: u32,
}

/// パイプライン統計
#[derive(Debug, Default)]
pub struct PipelineStats {
    pub frames_read: AtomicU64,
    pub frames_decoded: AtomicU64,
    pub frames_displayed: AtomicU64,
    pub read_errors: AtomicU64,
    pub decode_errors: AtomicU64,
    pub channel_full_count: AtomicU64,
}

/// パイプライン出力メッセージ (GUI向け)
#[derive(Debug)]
pub enum PipelineMessage {
    Frame(DecodedFrame),
    Metrics(MetricsMessage),
    JpegData(Vec<u8>),  // 録画用
    Stats {
        fps: f32,
        spresense_fps: f32,
        frame_count: u64,
        errors: u32,
        decode_time_ms: f32,
        read_time_ms: f32,
        jpeg_size_kb: f32,
    },
    ConnectionStatus(String),
    Error(String),
}

/// Phase 8 パイプラインマネージャー
pub struct Pipeline {
    /// TCP Reader Thread
    reader_handle: Option<JoinHandle<()>>,
    /// JPEG Decoder Thread
    decoder_handle: Option<JoinHandle<()>>,
    /// GUI向け出力チャネル
    pub output_rx: Receiver<PipelineMessage>,
    /// 停止フラグ
    is_running: Arc<AtomicBool>,
    /// 統計情報
    pub stats: Arc<PipelineStats>,
}

impl Pipeline {
    /// パイプラインを作成・開始
    pub fn new(
        connection: Connection,
        is_recording: Arc<AtomicBool>,
    ) -> Self {
        let is_running = Arc::new(AtomicBool::new(true));
        let stats = Arc::new(PipelineStats::default());

        // チャネル作成 (bounded)
        let (packet_tx, packet_rx) = mpsc::sync_channel::<RawPacket>(PACKET_CHANNEL_CAPACITY);
        let (output_tx, output_rx) = mpsc::channel::<PipelineMessage>();

        // TCP Reader Thread起動
        let reader_handle = {
            let is_running = is_running.clone();
            let stats = stats.clone();
            let output_tx = output_tx.clone();
            let is_recording = is_recording.clone();

            thread::Builder::new()
                .name("tcp-reader".to_string())
                .spawn(move || {
                    tcp_reader_thread(
                        connection,
                        packet_tx,
                        output_tx,
                        is_running,
                        stats,
                        is_recording,
                    );
                })
                .expect("Failed to spawn TCP reader thread")
        };

        // JPEG Decoder Thread起動
        let decoder_handle = {
            let is_running = is_running.clone();
            let stats = stats.clone();
            let output_tx_decoder = output_tx.clone();

            thread::Builder::new()
                .name("jpeg-decoder".to_string())
                .spawn(move || {
                    jpeg_decoder_thread(
                        packet_rx,
                        output_tx_decoder,
                        is_running,
                        stats,
                    );
                })
                .expect("Failed to spawn JPEG decoder thread")
        };

        Pipeline {
            reader_handle: Some(reader_handle),
            decoder_handle: Some(decoder_handle),
            output_rx,
            is_running,
            stats,
        }
    }

    /// パイプラインを停止
    pub fn stop(&mut self) {
        info!("Stopping pipeline...");
        self.is_running.store(false, Ordering::SeqCst);

        // スレッドの終了を待機
        if let Some(handle) = self.reader_handle.take() {
            if let Err(e) = handle.join() {
                error!("TCP reader thread join error: {:?}", e);
            }
        }

        if let Some(handle) = self.decoder_handle.take() {
            if let Err(e) = handle.join() {
                error!("JPEG decoder thread join error: {:?}", e);
            }
        }

        info!("Pipeline stopped");
    }

    /// パイプラインが動作中か確認
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// GUI向けメッセージを非ブロッキング受信
    pub fn try_recv(&self) -> Option<PipelineMessage> {
        self.output_rx.try_recv().ok()
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        self.stop();
    }
}

/// TCP Reader Thread
///
/// - TCPからパケットを読み込み (ブロッキング I/O)
/// - Metricsパケットは直接output_txへ送信
/// - MJPEGパケットはpacket_txへ送信 (Decoderスレッドへ)
fn tcp_reader_thread(
    mut connection: Connection,
    packet_tx: SyncSender<RawPacket>,
    output_tx: mpsc::Sender<PipelineMessage>,
    is_running: Arc<AtomicBool>,
    stats: Arc<PipelineStats>,
    is_recording: Arc<AtomicBool>,
) {
    info!("TCP Reader Thread started");
    let conn_info = connection.connection_info();
    output_tx.send(PipelineMessage::ConnectionStatus(format!("Connected: {}", conn_info))).ok();

    // 接続バッファをフラッシュ
    if let Err(e) = connection.flush() {
        error!("Failed to flush connection: {}", e);
    }

    // 統計用
    let mut frame_count = 0u64;
    let mut error_count = 0u32;
    let mut last_stats_time = Instant::now();
    let mut frames_since_last_stats = 0u32;
    let mut total_read_time_ms = 0.0f32;
    let mut total_jpeg_size = 0u64;

    // Spresense FPS計算用
    let mut last_sequence = 0u32;
    let mut sequence_diff_sum = 0u32;
    let mut sequence_samples = 0u32;

    while is_running.load(Ordering::SeqCst) {
        let read_start = Instant::now();
        let result = connection.read_packet();
        let read_time_ms = read_start.elapsed().as_secs_f32() * 1000.0;

        match result {
            Ok(Packet::Mjpeg(packet)) => {
                stats.frames_read.fetch_add(1, Ordering::Relaxed);
                frame_count += 1;
                frames_since_last_stats += 1;
                total_read_time_ms += read_time_ms;
                total_jpeg_size += packet.jpeg_data.len() as u64;

                // Spresense FPS計算 (シーケンス番号の差分から)
                if last_sequence > 0 && packet.header.sequence > last_sequence {
                    let diff = packet.header.sequence - last_sequence;
                    sequence_diff_sum += diff;
                    sequence_samples += 1;
                }
                last_sequence = packet.header.sequence;

                // 録画用にJPEGデータを送信
                if is_recording.load(Ordering::Relaxed) {
                    output_tx.send(PipelineMessage::JpegData(packet.jpeg_data.clone())).ok();
                }

                // パケットをDecoderスレッドへ送信
                let raw_packet = RawPacket {
                    packet: Packet::Mjpeg(packet),
                    read_time_ms,
                    timestamp: Instant::now(),
                };

                match packet_tx.try_send(raw_packet) {
                    Ok(_) => {}
                    Err(TrySendError::Full(pkt)) => {
                        // チャネル満杯 - ブロッキング送信
                        stats.channel_full_count.fetch_add(1, Ordering::Relaxed);
                        debug!("Packet channel full, blocking...");
                        if packet_tx.send(pkt).is_err() {
                            error!("Decoder thread disconnected");
                            break;
                        }
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        error!("Decoder thread disconnected");
                        break;
                    }
                }
            }

            Ok(Packet::Batch(batch)) => {
                // バッチパケット - 各フレームを個別に送信
                for frame in batch.frames {
                    stats.frames_read.fetch_add(1, Ordering::Relaxed);
                    frame_count += 1;
                    frames_since_last_stats += 1;
                    total_jpeg_size += frame.jpeg_data.len() as u64;

                    // Spresense FPS計算
                    if last_sequence > 0 && frame.metadata.frame_sequence > last_sequence {
                        let diff = frame.metadata.frame_sequence - last_sequence;
                        sequence_diff_sum += diff;
                        sequence_samples += 1;
                    }
                    last_sequence = frame.metadata.frame_sequence;

                    // 録画用
                    if is_recording.load(Ordering::Relaxed) {
                        output_tx.send(PipelineMessage::JpegData(frame.jpeg_data.clone())).ok();
                    }

                    // MJPEGパケットとして変換して送信
                    let mjpeg_packet = MjpegPacket {
                        header: crate::protocol::MjpegHeader {
                            sync_word: crate::protocol::SYNC_WORD,
                            sequence: frame.metadata.frame_sequence,
                            jpeg_size: frame.metadata.frame_size,
                        },
                        jpeg_data: frame.jpeg_data,
                        crc16: 0,  // CRCは検証済み
                    };

                    let raw_packet = RawPacket {
                        packet: Packet::Mjpeg(mjpeg_packet),
                        read_time_ms: read_time_ms / batch.header.frame_count as f32,
                        timestamp: Instant::now(),
                    };

                    match packet_tx.try_send(raw_packet) {
                        Ok(_) => {}
                        Err(TrySendError::Full(pkt)) => {
                            stats.channel_full_count.fetch_add(1, Ordering::Relaxed);
                            if packet_tx.send(pkt).is_err() {
                                error!("Decoder thread disconnected");
                                return;
                            }
                        }
                        Err(TrySendError::Disconnected(_)) => {
                            error!("Decoder thread disconnected");
                            return;
                        }
                    }
                }
                total_read_time_ms += read_time_ms;
            }

            Ok(Packet::Metrics(metrics)) => {
                // Metricsパケット - 直接GUIへ送信
                info!("📊 Metrics: seq={}, cam_frames={}, usb_pkts={}, q_depth={}, dropped={}, tcp_avg={}ms",
                      metrics.sequence, metrics.camera_frames, metrics.usb_packets,
                      metrics.action_q_depth, metrics.dropped_frames,
                      metrics.tcp_avg_send_us / 1000);

                // Camera FPS計算 (前回からの差分)
                let camera_fps = if sequence_samples > 0 {
                    let avg_diff = sequence_diff_sum as f32 / sequence_samples as f32;
                    if avg_diff > 0.0 { 30.0 / avg_diff } else { 0.0 }  // 30fps基準
                } else {
                    0.0
                };

                let msg = MetricsMessage {
                    timestamp_ms: metrics.timestamp_ms,
                    camera_frames: metrics.camera_frames,
                    camera_fps,
                    usb_packets: metrics.usb_packets,
                    action_q_depth: metrics.action_q_depth,
                    avg_packet_size: metrics.avg_packet_size,
                    errors: metrics.errors,
                    tcp_avg_send_us: metrics.tcp_avg_send_us,
                    tcp_max_send_us: metrics.tcp_max_send_us,
                    dropped_frames: metrics.dropped_frames,
                    drop_events: metrics.drop_events,
                    tcp_health_moving_avg_ms: metrics.tcp_health_moving_avg_ms,  // Phase 9.2
                    tcp_health_total_spikes: metrics.tcp_health_total_spikes,    // Phase 9.2
                };

                output_tx.send(PipelineMessage::Metrics(msg)).ok();
            }

            Err(e) => {
                match e.kind() {
                    io::ErrorKind::UnexpectedEof |
                    io::ErrorKind::ConnectionReset |
                    io::ErrorKind::ConnectionAborted |
                    io::ErrorKind::BrokenPipe |
                    io::ErrorKind::NotConnected => {
                        error!("Connection closed: {}", e);
                        output_tx.send(PipelineMessage::ConnectionStatus(
                            format!("Disconnected: {}", e)
                        )).ok();
                        break;
                    }
                    io::ErrorKind::TimedOut => {
                        // タイムアウトは継続
                        debug!("Read timeout, continuing...");
                    }
                    _ => {
                        error!("Read error: {}", e);
                        error_count += 1;
                        stats.read_errors.fetch_add(1, Ordering::Relaxed);

                        if error_count >= 10 {
                            error!("Too many read errors, stopping");
                            output_tx.send(PipelineMessage::Error(
                                format!("Too many errors: {}", e)
                            )).ok();
                            break;
                        }

                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
        }

        // 統計送信 (1秒ごと)
        let elapsed = last_stats_time.elapsed().as_secs_f32();
        if elapsed >= 1.0 && frames_since_last_stats > 0 {
            let fps = frames_since_last_stats as f32 / elapsed;
            let avg_read_time = total_read_time_ms / frames_since_last_stats as f32;
            let avg_jpeg_size_kb = (total_jpeg_size as f32 / frames_since_last_stats as f32) / 1024.0;

            let spresense_fps = if sequence_samples > 0 {
                let avg_diff = sequence_diff_sum as f32 / sequence_samples as f32;
                if avg_diff > 0.0 { 30.0 / avg_diff } else { 0.0 }
            } else {
                0.0
            };

            output_tx.send(PipelineMessage::Stats {
                fps,
                spresense_fps,
                frame_count,
                errors: error_count,
                decode_time_ms: 0.0,  // Decoderスレッドで計測
                read_time_ms: avg_read_time,
                jpeg_size_kb: avg_jpeg_size_kb,
            }).ok();

            // リセット
            frames_since_last_stats = 0;
            total_read_time_ms = 0.0;
            total_jpeg_size = 0;
            sequence_diff_sum = 0;
            sequence_samples = 0;
            last_stats_time = Instant::now();
        }
    }

    info!("TCP Reader Thread stopped (frames: {})", frame_count);
}

/// JPEG Decoder Thread
///
/// - packet_rxからパケットを受信
/// - JPEGデコード (image::load_from_memory)
/// - RGBA8に変換
/// - output_txへ直接送信 (PipelineMessage::Frame)
fn jpeg_decoder_thread(
    packet_rx: Receiver<RawPacket>,
    output_tx: mpsc::Sender<PipelineMessage>,
    is_running: Arc<AtomicBool>,
    stats: Arc<PipelineStats>,
) {
    info!("JPEG Decoder Thread started");

    let mut consecutive_errors = 0u32;

    while is_running.load(Ordering::SeqCst) {
        // パケット受信 (タイムアウト付き)
        let packet = match packet_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(p) => p,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                info!("Packet channel disconnected");
                break;
            }
        };

        // MJPEGパケットのみ処理
        let (mjpeg, read_time_ms) = match packet.packet {
            Packet::Mjpeg(m) => (m, packet.read_time_ms),
            _ => continue,
        };

        // JPEGデコード
        let decode_start = Instant::now();
        match image::load_from_memory(&mjpeg.jpeg_data) {
            Ok(img) => {
                let decode_time_ms = decode_start.elapsed().as_secs_f32() * 1000.0;
                consecutive_errors = 0;

                // RGBA8変換
                let rgba = img.to_rgba8();
                let width = img.width();
                let height = img.height();
                let pixels = rgba.into_raw();

                let frame = DecodedFrame {
                    width,
                    height,
                    pixels,
                    sequence: mjpeg.header.sequence,
                    jpeg_size: mjpeg.jpeg_data.len(),
                    decode_time_ms,
                    read_time_ms,
                    timestamp: packet.timestamp,
                };

                stats.frames_decoded.fetch_add(1, Ordering::Relaxed);

                // フレームをGUIへ直接送信
                if output_tx.send(PipelineMessage::Frame(frame)).is_err() {
                    info!("Output channel disconnected");
                    break;
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                stats.decode_errors.fetch_add(1, Ordering::Relaxed);

                if consecutive_errors == 5 {
                    warn!("5 consecutive JPEG decode errors");
                } else if consecutive_errors >= 10 {
                    error!("10+ consecutive JPEG decode errors: {}", e);
                }

                // エラーでも継続 (フレームスキップ)
            }
        }
    }

    info!("JPEG Decoder Thread stopped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stats() {
        let stats = PipelineStats::default();
        stats.frames_read.fetch_add(1, Ordering::Relaxed);
        assert_eq!(stats.frames_read.load(Ordering::Relaxed), 1);
    }
}
