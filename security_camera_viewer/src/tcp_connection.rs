/// TCP接続モジュール (Phase 7 + Phase 9 Auto-reconnect)
///
/// SpresenseのTCPサーバーに接続し、MJPEGパケットを受信する。
/// SerialConnection互換のインターフェースを提供。
/// Phase 9: 自動再接続機能を追加。

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs, SocketAddr};
use std::time::Duration;
use std::thread;
use log::{info, warn, error};

/// Phase 9.1: 再接続設定（バックオフ付き）
const RECONNECT_MAX_ATTEMPTS: u32 = 10;
const RECONNECT_BASE_WAIT_SECS: u64 = 5;    // 基本待機時間
const RECONNECT_BACKOFF_SECS: u64 = 2;      // 試行ごとの追加待機時間

/// Phase 9: 接続状態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
}

/// TCP接続管理（Phase 7.3: ステートフル読み取り対応 + Phase 9: 自動再接続）
pub struct TcpConnection {
    stream: TcpStream,
    host: String,
    port: u16,
    peer_addr: SocketAddr,
    // Phase 7.3: ステートフル読み取り用フィールド
    sync_established: bool,     // 初回sync完了フラグ
    internal_buffer: Vec<u8>,   // 内部バッファ（TCP read()のタイミング吸収用）
    buffer_pos: usize,          // 内部バッファの現在位置
    // Phase 9: 自動再接続用フィールド
    state: ConnectionState,     // 接続状態
    reconnect_count: u32,       // 再接続回数（累積）
}

impl TcpConnection {
    /// 新しいTCP接続を作成
    ///
    /// # Arguments
    /// * `host` - Spresenseのホスト名またはIPアドレス
    /// * `port` - TCPポート番号 (デフォルト: 8888)
    ///
    /// # Returns
    /// 成功時は`TcpConnection`インスタンス、失敗時はエラー
    ///
    /// # Example
    /// ```
    /// let conn = TcpConnection::new("192.168.1.100", 8888)?;
    /// ```
    pub fn new(host: &str, port: u16) -> io::Result<Self> {
        info!("Connecting to TCP server {}:{}...", host, port);

        let addr = format!("{}:{}", host, port);
        let socket_addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid address"))?;

        // 接続タイムアウト10秒
        let stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))?;

        let peer_addr = stream.peer_addr()?;
        info!("Connected to Spresense: {}", peer_addr);

        // タイムアウト設定 (Phase 7.2a: Increased to 30s for batching + initialization)
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;

        // Nagleアルゴリズム無効化 (低遅延優先)
        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            host: host.to_string(),
            port,
            peer_addr,
            // Phase 7.3: ステートフル読み取り初期化
            sync_established: false,
            internal_buffer: Vec::with_capacity(256_000),  // 250KB内部バッファ
            buffer_pos: 0,
            // Phase 9: 自動再接続初期化
            state: ConnectionState::Connected,
            reconnect_count: 0,
        })
    }

    /// MJPEGパケットを読み込む (Phase 7.3: ステートフル読み取り対応)
    ///
    /// # Arguments
    /// * `buffer` - 読み込みバッファ (最小150KB推奨)
    ///
    /// # Returns
    /// 成功時は読み込んだバイト数、失敗時はエラー
    ///
    /// # Errors
    /// - 接続が切断された場合: `UnexpectedEof`
    /// - タイムアウト: `TimedOut`
    /// - その他のI/Oエラー
    ///
    /// # Phase 7.3 改善内容
    /// - 初回のみsync word検索、以降はパケットサイズ情報に基づいて読み取り
    /// - 内部バッファでTCP read()のタイミングのばらつきを吸収
    /// - 期待される成功率: 99.85% (従来21% → 大幅改善)
    pub fn read_packet(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        const MJPEG_SYNC_WORD: u32 = 0xCAFEBABE;
        const MJPEG_BATCH_SYNC_WORD: u32 = 0xCAFEBABF;
        const METRICS_SYNC_WORD: u32 = 0xCAFEBEEF;

        // Step 1: 初回のみsync word検索
        if !self.sync_established {
            self.find_initial_sync()?;
            self.sync_established = true;
            info!("Initial sync established");
        }

        // Step 2: 内部バッファに十分なデータがあるか確認（最低14バイト必要）
        self.ensure_buffer_has(14)?;

        // Step 3: sync wordを読み取り（すでに同期済みなので、現在位置から読むだけ）
        let sync_word = u32::from_le_bytes([
            self.internal_buffer[self.buffer_pos],
            self.internal_buffer[self.buffer_pos + 1],
            self.internal_buffer[self.buffer_pos + 2],
            self.internal_buffer[self.buffer_pos + 3],
        ]);

        // Step 4: パケットタイプに応じてサイズを決定
        let packet_size = match sync_word {
            MJPEG_SYNC_WORD => {
                // MJPEG packet: ヘッダー14バイト + JPEG data + CRC 2バイト
                let jpeg_size = u32::from_le_bytes([
                    self.internal_buffer[self.buffer_pos + 8],
                    self.internal_buffer[self.buffer_pos + 9],
                    self.internal_buffer[self.buffer_pos + 10],
                    self.internal_buffer[self.buffer_pos + 11],
                ]) as usize;

                // サイズ妥当性チェック
                if jpeg_size == 0 || jpeg_size > 150_000 {
                    error!("Invalid JPEG size: {} bytes", jpeg_size);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid JPEG size: {}", jpeg_size),
                    ));
                }

                // Header 12 bytes (sync 4 + seq 4 + size 4) + JPEG data + CRC 2 bytes
                12 + jpeg_size + 2
            }

            MJPEG_BATCH_SYNC_WORD => {
                // Batch packet: ヘッダー16バイト + frame metadata + JPEG data + CRC
                // まず16バイト確保
                self.ensure_buffer_has(16)?;

                let frame_count = u32::from_le_bytes([
                    self.internal_buffer[self.buffer_pos + 8],
                    self.internal_buffer[self.buffer_pos + 9],
                    self.internal_buffer[self.buffer_pos + 10],
                    self.internal_buffer[self.buffer_pos + 11],
                ]) as usize;

                let total_size = u32::from_le_bytes([
                    self.internal_buffer[self.buffer_pos + 12],
                    self.internal_buffer[self.buffer_pos + 13],
                    self.internal_buffer[self.buffer_pos + 14],
                    self.internal_buffer[self.buffer_pos + 15],
                ]) as usize;

                // 妥当性チェック
                if frame_count == 0 || frame_count > 3 {
                    error!("Invalid batch frame count: {}", frame_count);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid batch frame count: {}", frame_count),
                    ));
                }

                if total_size == 0 || total_size > 200_000 {
                    error!("Invalid batch total size: {} bytes", total_size);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid batch total size: {}", total_size),
                    ));
                }

                // バッチパケット全体のサイズ
                16 + frame_count * 8 + total_size + 2
            }

            METRICS_SYNC_WORD => {
                // Metricsパケット: 固定58バイト (Phase 9.2: +8 bytes for health metrics)
                // 4*14 fields + 2 CRC = 58 bytes
                58
            }

            _ => {
                // 同期ずれの可能性 - バッファ内で次のsync wordを検索
                warn!("Unknown sync word: 0x{:08X}, searching for resync...", sync_word);

                // バッファ内で次の有効なsync wordを探す
                let resync_result = self.resync_in_buffer()?;
                if !resync_result {
                    // バッファ内に見つからない - ストリームから再検索
                    self.sync_established = false;
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unknown sync word: 0x{:08X}", sync_word),
                    ));
                }

                // 再帰的に再試行（バッファ位置が更新されている）
                return self.read_packet(buffer);
            }
        };

        // Step 5: パケット全体を内部バッファに確保
        self.ensure_buffer_has(packet_size)?;

        // Step 6: パケットを出力bufferにコピー
        if buffer.len() < packet_size {
            error!("Output buffer too small: {} < {}", buffer.len(), packet_size);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Output buffer too small: {} < {}", buffer.len(), packet_size),
            ));
        }

        buffer[..packet_size].copy_from_slice(
            &self.internal_buffer[self.buffer_pos..self.buffer_pos + packet_size]
        );

        // Step 7: バッファ位置を進める
        self.buffer_pos += packet_size;

        Ok(packet_size)
    }

    /// 初回sync word検索 (Phase 7.3)
    ///
    /// TCP streamから読み込み、最初のsync wordを探す。
    /// 見つかったら、大量のデータを内部バッファに読み込んで待ち行列を構築する。
    fn find_initial_sync(&mut self) -> io::Result<()> {
        const MJPEG_SYNC_WORD: u32 = 0xCAFEBABE;
        const MJPEG_BATCH_SYNC_WORD: u32 = 0xCAFEBABF;
        const METRICS_SYNC_WORD: u32 = 0xCAFEBEEF;
        const MAX_SYNC_ATTEMPTS: usize = 100_000;
        const INITIAL_BUFFER_SIZE: usize = 150_000; // 150KB事前読み込み

        info!("Searching for initial sync word...");

        let mut sync_buffer = [0u8; 4];
        let mut sync_attempts = 0;
        let mut discarded_bytes = Vec::new(); // スキップしたバイトを保存

        // 最初の4バイトを読む
        self.stream.read_exact(&mut sync_buffer)?;

        loop {
            let sync_word = u32::from_le_bytes(sync_buffer);

            // 正しいsync wordを発見
            if sync_word == MJPEG_SYNC_WORD
                || sync_word == MJPEG_BATCH_SYNC_WORD
                || sync_word == METRICS_SYNC_WORD {

                if sync_attempts > 0 {
                    warn!("Initial sync word found after {} bytes skipped", sync_attempts);
                }

                // 内部バッファをクリア
                self.internal_buffer.clear();
                self.buffer_pos = 0;

                // sync wordを内部バッファに格納
                self.internal_buffer.extend_from_slice(&sync_buffer);

                // 重要: 大量のデータを事前に読み込んで待ち行列を構築
                // これにより、TCP read()のタイミングに関係なくパケット境界を保てる
                let mut temp_buffer = vec![0u8; INITIAL_BUFFER_SIZE];
                let mut total_read = 0;

                // 最低150KBまたはタイムアウトまで読み込む
                while total_read < INITIAL_BUFFER_SIZE {
                    match self.stream.read(&mut temp_buffer[total_read..]) {
                        Ok(0) => {
                            // 接続切断 - 読み込んだ分だけ使う
                            warn!("Connection closed during initial buffer fill (got {} bytes)", total_read);
                            break;
                        }
                        Ok(n) => {
                            total_read += n;
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                            // タイムアウト - 読み込んだ分だけ使う
                            warn!("Timeout during initial buffer fill (got {} bytes)", total_read);
                            break;
                        }
                        Err(e) => return Err(e),
                    }
                }

                // 読み込んだデータを内部バッファに追加
                self.internal_buffer.extend_from_slice(&temp_buffer[..total_read]);

                info!("Initial buffer filled: {} bytes (sync word + {} bytes data)",
                      self.internal_buffer.len(), total_read);

                return Ok(());
            }

            // 1バイトスライドして新しい1バイトを読む
            discarded_bytes.push(sync_buffer[0]);
            sync_buffer[0] = sync_buffer[1];
            sync_buffer[1] = sync_buffer[2];
            sync_buffer[2] = sync_buffer[3];

            match self.stream.read(&mut sync_buffer[3..4]) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Connection closed while searching sync word",
                    ));
                }
                Ok(_) => {}
                Err(e) => return Err(e),
            }

            sync_attempts += 1;
            if sync_attempts > MAX_SYNC_ATTEMPTS {
                error!("Failed to find sync word after {} attempts", MAX_SYNC_ATTEMPTS);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Sync word not found",
                ));
            }
        }
    }

    /// 内部バッファに最低限必要なバイト数を確保 (Phase 7.3)
    ///
    /// # Arguments
    /// * `min_bytes` - 必要な最小バイト数
    ///
    /// # Returns
    /// 成功時はOk(()), 失敗時はエラー
    fn ensure_buffer_has(&mut self, min_bytes: usize) -> io::Result<()> {
        let available = self.internal_buffer.len() - self.buffer_pos;

        if available >= min_bytes {
            // すでに十分なデータがある
            return Ok(());
        }

        // 未消費データを先頭に移動
        if self.buffer_pos > 0 {
            let remaining = self.internal_buffer.len() - self.buffer_pos;
            if remaining > 0 {
                // copy_withinは重複領域のコピーを安全に行う
                self.internal_buffer.copy_within(self.buffer_pos.., 0);
                self.internal_buffer.truncate(remaining);
            } else {
                self.internal_buffer.clear();
            }
            self.buffer_pos = 0;
        }

        // 追加読み込み（最低min_bytesまで読む）
        let mut temp_buf = vec![0u8; 65536];  // 64KB一時バッファ

        while self.internal_buffer.len() < min_bytes {
            match self.stream.read(&mut temp_buf) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("Connection closed (need {} bytes, have {})",
                                min_bytes, self.internal_buffer.len()),
                    ));
                }
                Ok(n) => {
                    self.internal_buffer.extend_from_slice(&temp_buf[..n]);
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    /// バッファ内で次の有効なsync wordを検索 (Phase 8: 同期復帰改善)
    ///
    /// unknown sync wordが検出された場合、バッファ内で次の有効なsync wordを探す。
    /// 見つかった場合はbuffer_posを更新してtrue、見つからない場合はfalseを返す。
    fn resync_in_buffer(&mut self) -> io::Result<bool> {
        const MJPEG_SYNC_WORD: u32 = 0xCAFEBABE;
        const MJPEG_BATCH_SYNC_WORD: u32 = 0xCAFEBABF;
        const METRICS_SYNC_WORD: u32 = 0xCAFEBEEF;
        const MAX_SEARCH_BYTES: usize = 200_000;  // 最大200KB検索

        let available = self.internal_buffer.len() - self.buffer_pos;

        // 最低4バイト必要
        if available < 4 {
            return Ok(false);
        }

        let search_limit = std::cmp::min(available - 3, MAX_SEARCH_BYTES);
        let mut found_offset = None;

        // 1バイトずつスライドしながら検索
        for offset in 1..search_limit {
            let pos = self.buffer_pos + offset;
            let sync_word = u32::from_le_bytes([
                self.internal_buffer[pos],
                self.internal_buffer[pos + 1],
                self.internal_buffer[pos + 2],
                self.internal_buffer[pos + 3],
            ]);

            if sync_word == MJPEG_SYNC_WORD
                || sync_word == MJPEG_BATCH_SYNC_WORD
                || sync_word == METRICS_SYNC_WORD
            {
                found_offset = Some(offset);
                break;
            }
        }

        if let Some(offset) = found_offset {
            warn!("Resync found valid sync word at offset {} bytes", offset);
            self.buffer_pos += offset;
            Ok(true)
        } else {
            warn!("No valid sync word found in {} bytes", search_limit);
            Ok(false)
        }
    }

    /// 接続情報を取得
    pub fn connection_info(&self) -> String {
        format!("TCP {}:{} ({})", self.host, self.port, self.peer_addr)
    }

    /// ホスト名を取得
    pub fn host(&self) -> &str {
        &self.host
    }

    /// ポート番号を取得
    pub fn port(&self) -> u16 {
        self.port
    }

    /// ピアアドレスを取得
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// 接続が有効かチェック
    pub fn is_connected(&self) -> bool {
        // TCP接続の状態確認 (簡易)
        self.stream.peer_addr().is_ok()
    }

    /// Phase 9: 接続状態を取得
    pub fn connection_state(&self) -> ConnectionState {
        self.state
    }

    /// Phase 9: 再接続回数を取得
    pub fn reconnect_count(&self) -> u32 {
        self.reconnect_count
    }

    /// Phase 9: 再接続を試行
    ///
    /// TCP切断を検出した場合に呼び出す。
    /// 最大RECONNECT_MAX_ATTEMPTS回まで再接続を試行する。
    ///
    /// # Returns
    /// 成功時はOk(()), 失敗時（上限到達等）はエラー
    pub fn reconnect(&mut self) -> io::Result<()> {
        for attempt in 1..=RECONNECT_MAX_ATTEMPTS {
            self.state = ConnectionState::Reconnecting;

            // Phase 9.1: バックオフ付き待機
            // wait_secs = base + (attempt - 1) * backoff
            let wait_secs = RECONNECT_BASE_WAIT_SECS +
                            (attempt as u64 - 1) * RECONNECT_BACKOFF_SECS;

            warn!("TCP disconnected, waiting {} secs before reconnect ({}/{})",
                  wait_secs, attempt, RECONNECT_MAX_ATTEMPTS);

            // 待機（GS2200Mのリソース解放を待つ）
            thread::sleep(Duration::from_secs(wait_secs));

            // 接続試行
            let addr = format!("{}:{}", self.host, self.port);
            let socket_addr = match addr.to_socket_addrs() {
                Ok(mut addrs) => match addrs.next() {
                    Some(addr) => addr,
                    None => {
                        error!("Reconnect attempt {} failed: Invalid address", attempt);
                        continue;
                    }
                },
                Err(e) => {
                    error!("Reconnect attempt {} failed: {}", attempt, e);
                    continue;
                }
            };

            match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10)) {
                Ok(stream) => {
                    // 設定適用
                    if let Err(e) = stream.set_read_timeout(Some(Duration::from_secs(30))) {
                        warn!("Failed to set read timeout: {}", e);
                    }
                    if let Err(e) = stream.set_write_timeout(Some(Duration::from_secs(30))) {
                        warn!("Failed to set write timeout: {}", e);
                    }
                    if let Err(e) = stream.set_nodelay(true) {
                        warn!("Failed to set nodelay: {}", e);
                    }

                    // peer_addrを取得
                    let peer_addr = match stream.peer_addr() {
                        Ok(addr) => addr,
                        Err(e) => {
                            error!("Failed to get peer address: {}", e);
                            continue;
                        }
                    };

                    // 接続更新
                    self.stream = stream;
                    self.peer_addr = peer_addr;

                    // sync状態リセット（再同期が必要）
                    self.sync_established = false;
                    self.internal_buffer.clear();
                    self.buffer_pos = 0;

                    // 状態更新
                    self.state = ConnectionState::Connected;
                    self.reconnect_count += 1;

                    info!("Reconnected to Spresense: {} (attempt {})", self.peer_addr, attempt);
                    return Ok(());
                }
                Err(e) => {
                    error!("Reconnect attempt {} failed: {}", attempt, e);
                }
            }
        }

        self.state = ConnectionState::Disconnected;
        Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!("Failed to reconnect after {} attempts", RECONNECT_MAX_ATTEMPTS)
        ))
    }

    /// Phase 9: 自動再接続対応パケット読み込み
    ///
    /// 切断検出時に自動で再接続を試行し、成功したら読み込みを継続する。
    ///
    /// # Arguments
    /// * `buffer` - 読み込みバッファ (最小150KB推奨)
    ///
    /// # Returns
    /// 成功時は読み込んだバイト数、失敗時はエラー
    pub fn read_packet_with_reconnect(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.read_packet(buffer) {
                Ok(n) => return Ok(n),
                Err(e) => {
                    // 切断検出
                    let is_disconnect = matches!(e.kind(),
                        io::ErrorKind::UnexpectedEof |
                        io::ErrorKind::ConnectionReset |
                        io::ErrorKind::ConnectionAborted |
                        io::ErrorKind::BrokenPipe
                    );

                    if is_disconnect {
                        warn!("TCP disconnect detected: {}", e);

                        // 再接続試行
                        match self.reconnect() {
                            Ok(()) => {
                                info!("Reconnect successful, resuming read");
                                // 再接続成功 → ループ継続（再度read_packet）
                                continue;
                            }
                            Err(reconnect_err) => {
                                error!("Reconnect failed: {}", reconnect_err);
                                return Err(reconnect_err);
                            }
                        }
                    }

                    // その他のエラーはそのまま返す
                    return Err(e);
                }
            }
        }
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        // Graceful shutdown
        if let Err(e) = self.stream.shutdown(std::net::Shutdown::Both) {
            warn!("TCP shutdown error: {}", e);
        }
        info!("TCP connection closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Spresenseサーバーが起動している場合のみ実行
    fn test_tcp_connection() {
        // テスト用: ローカルホストのSpresenseサーバーに接続
        let conn = TcpConnection::new("127.0.0.1", 8888);

        match conn {
            Ok(c) => {
                println!("Connected: {}", c.connection_info());
                assert!(c.is_connected());
            }
            Err(e) => {
                println!("Connection failed (expected if no server): {}", e);
            }
        }
    }

    #[test]
    fn test_invalid_address() {
        let result = TcpConnection::new("invalid.host.name.that.does.not.exist", 8888);
        assert!(result.is_err());
    }
}
