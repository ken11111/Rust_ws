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

        // Phase 1: ヘッダー読み込み (14 bytes)
        // [SYNC_WORD:4][SEQUENCE:4][JPEG_SIZE:4][RESERVED:2]
        while total_read < 14 {
            match self.stream.read(&mut buffer[total_read..]) {
                Ok(0) => {
                    error!("Connection closed by Spresense");
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Connection closed by server",
                    ));
                }
                Ok(n) => {
                    total_read += n;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                    warn!("Read timeout while reading header");
                    return Err(e);
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    return Err(e);
                }
            }
        }

        // Phase 2: JPEG sizeを抽出 (リトルエンディアン)
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

        // Phase 3: JPEG data + CRC読み込み
        let remaining = jpeg_size + 2; // JPEG + CRC16
        let mut read_so_far = 0;

        while read_so_far < remaining {
            match self.stream.read(&mut buffer[total_read..]) {
                Ok(0) => {
                    error!("Connection closed while reading JPEG data");
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Connection closed while reading JPEG",
                    ));
                }
                Ok(n) => {
                    total_read += n;
                    read_so_far += n;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                    warn!("Read timeout while reading JPEG data");
                    return Err(e);
                }
                Err(e) => {
                    error!("Read error while reading JPEG: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(total_read)
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
