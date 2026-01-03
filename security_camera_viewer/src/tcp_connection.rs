/// TCP接続モジュール (Phase 7)
///
/// SpresenseのTCPサーバーに接続し、MJPEGパケットを受信する。
/// SerialConnection互換のインターフェースを提供。

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs, SocketAddr};
use std::time::Duration;
use log::{info, warn, error};

/// TCP接続管理
pub struct TcpConnection {
    stream: TcpStream,
    host: String,
    port: u16,
    peer_addr: SocketAddr,
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

        // タイムアウト設定
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        // Nagleアルゴリズム無効化 (低遅延優先)
        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            host: host.to_string(),
            port,
            peer_addr,
        })
    }

    /// MJPEGパケットを読み込む (SerialConnection::read_packet()互換)
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
    pub fn read_packet(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut total_read = 0;

        const MJPEG_SYNC_WORD: u32 = 0xCAFEBABE;
        const METRICS_SYNC_WORD: u32 = 0xCAFEBEEF;
        const MAX_SYNC_ATTEMPTS: usize = 100000; // 最大100KBスキップ

        // Phase 1: Sync word検索 - 同期が取れるまで1バイトずつ読む
        let mut sync_buffer = [0u8; 4];
        let mut sync_attempts = 0;

        loop {
            // 4バイト読み込み
            if total_read == 0 {
                // 最初の4バイト
                for i in 0..4 {
                    match self.stream.read(&mut sync_buffer[i..i+1]) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Connection closed by server",
                            ));
                        }
                        Ok(_) => {}
                        Err(e) => return Err(e),
                    }
                }
            } else {
                // 1バイトスライドして新しい1バイトを読む
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
            }

            let sync_word = u32::from_le_bytes(sync_buffer);

            // 正しいsync wordを発見
            if sync_word == MJPEG_SYNC_WORD || sync_word == METRICS_SYNC_WORD {
                // sync wordをバッファにコピー
                buffer[0..4].copy_from_slice(&sync_buffer);
                total_read = 4;

                if sync_attempts > 0 {
                    warn!("Sync word found after {} bytes skipped", sync_attempts);
                }
                break;
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

        // Phase 2: Sync wordを判定
        let sync_word = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);

        match sync_word {
            MJPEG_SYNC_WORD => {
                // MJPEGパケット: 残りのヘッダー + JPEG data + CRC
                // ヘッダー残り10バイト読み込み
                while total_read < 14 {
                    match self.stream.read(&mut buffer[total_read..]) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Connection closed while reading MJPEG header",
                            ));
                        }
                        Ok(n) => total_read += n,
                        Err(e) => return Err(e),
                    }
                }

                // JPEG sizeを抽出
                let jpeg_size = u32::from_le_bytes([
                    buffer[8],
                    buffer[9],
                    buffer[10],
                    buffer[11],
                ]) as usize;

                // サイズ妥当性チェック
                if jpeg_size == 0 || jpeg_size > 150_000 {
                    error!("Invalid JPEG size: {} bytes", jpeg_size);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid JPEG size: {}", jpeg_size),
                    ));
                }

                // JPEG data + CRC読み込み
                let remaining = jpeg_size + 2;
                let mut read_so_far = 0;

                while read_so_far < remaining {
                    match self.stream.read(&mut buffer[total_read..]) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Connection closed while reading JPEG data",
                            ));
                        }
                        Ok(n) => {
                            total_read += n;
                            read_so_far += n;
                        }
                        Err(e) => return Err(e),
                    }
                }

                Ok(total_read)
            }

            METRICS_SYNC_WORD => {
                // Metricsパケット: 固定38バイト
                const METRICS_PACKET_SIZE: usize = 38;

                while total_read < METRICS_PACKET_SIZE {
                    match self.stream.read(&mut buffer[total_read..]) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "Connection closed while reading metrics packet",
                            ));
                        }
                        Ok(n) => total_read += n,
                        Err(e) => return Err(e),
                    }
                }

                Ok(total_read)
            }

            _ => {
                error!("Unknown sync word: 0x{:08X}", sync_word);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unknown sync word: 0x{:08X}", sync_word),
                ))
            }
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
