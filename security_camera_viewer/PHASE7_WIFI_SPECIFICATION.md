# Phase 7: WiFi通信対応 - 仕様書

**Phase**: 7 - WiFi通信対応
**作成日**: 2026年1月2日
**優先度**: 高 (ユーザー要望)
**前提Phase**: Phase 1-6完了

---

## 1. 概要

### 1.1 目的

現在のUSB CDC通信をWiFi (TCP/IP)通信に置き換え、ワイヤレスでの映像伝送を実現する。

**現状**:
```
[Spresense] ←─ USB CDC ─→ [PC]
            (有線、12Mbps)
```

**Phase 7実装後**:
```
[Spresense] ←─ WiFi TCP/IP ─→ [PC]
            (無線、最大150Mbps理論値)
```

### 1.2 ユーザー要求 (既存資料より)

**要求仕様書 (01_REQUIREMENTS.md) からの抜粋**:
- Q4回答: 「最終的にはWiFi、テストのために最初はUSB接続から始める」
- Q5回答: 「RTSPでお願いします」

**Phase 1.5計画 (PHASE1_5_UPDATE_PLAN.md) からの抜粋**:
- Phase 2-B項目: WiFi対応 (優先度: 低 → **Phase 7で高に格上げ**)

### 1.3 設計方針

**Option A: RTSP対応** (ユーザー要望)
- ✅ 標準プロトコル
- ✅ VLCなどの既存プレーヤーで再生可能
- ❌ 実装複雑 (RTSPサーバー実装必要)
- ❌ Spresense上でRTSPライブラリ対応が不明

**Option B: TCP/IPソケット + MJPEGプロトコル継続** ✅ **推奨**
- ✅ 既存のMJPEGプロトコルを再利用
- ✅ 実装が最もシンプル
- ✅ 段階的移行が可能
- ❌ 専用クライアント必要 (既存のRustアプリ使用)

**Option C: HTTP MJPEG Streaming**
- ✅ ブラウザで直接表示可能
- ✅ 実装比較的簡単
- ❌ HTTPヘッダーオーバーヘッド
- ❌ Spresense上でHTTPサーバー実装必要

**Phase 7採用**: **Option B** (TCP/IP + MJPEGプロトコル)
**Phase 8検討**: Option A (RTSP) または Option C (HTTP MJPEG)

---

## 2. ハードウェア要件

### 2.1 Spresense WiFi拡張ボード

**必要ハードウェア**:
- Spresense メインボード (既存)
- Spresense カメラボード (既存)
- **iS110B WiFi Add-onボード** (新規購入必要)

**iS110B仕様**:
- WiFi規格: IEEE 802.11 b/g/n
- 周波数: 2.4GHz
- 理論速度: 最大150Mbps
- セキュリティ: WPA/WPA2
- インターフェース: SPI経由でSpresenseと接続

**接続構成**:
```
┌─────────────────────────────────────┐
│  Spresense メインボード             │
│  ┌─────────────┐                    │
│  │   CXD5602   │                    │
│  │   (ARM M4)  │                    │
│  └──┬──────┬───┘                    │
│     │      │                        │
│   SPI    USB CDC                    │
│     │      │                        │
├─────┼──────┼────────────────────────┤
│     │      │                        │
│  ┌──▼──┐   │                        │
│  │iS110B│  │ (Phase 7でSPI使用)     │
│  │WiFi │  │                        │
│  └──┬──┘   │                        │
│     │      │                        │
│   WiFi   USB (Phase 1-6互換保持)    │
│     │      │                        │
└─────┼──────┼────────────────────────┘
      │      │
      │      └──> PC (USB CDC)
      │
      └──> WiFi AP ──> PC (WiFi)
```

### 2.2 ネットワーク構成

**開発・テスト環境**:
```
┌─────────────┐      WiFi       ┌──────────────┐
│  Spresense  │◄───────────────►│  WiFi Router │
│  (Station)  │  192.168.1.100  │  (AP Mode)   │
└─────────────┘                 └──────┬───────┘
                                       │
                                       │ LAN/WiFi
                                       │
                                ┌──────▼───────┐
                                │  PC (Client) │
                                │192.168.1.200 │
                                └──────────────┘
```

**本番環境**:
- Spresense: WiFi Station Mode (DHCP or 固定IP)
- PC: WiFi Client (同一ネットワーク)
- ポート: TCP 8888 (設定可能)

---

## 3. Spresense側実装

### 3.1 WiFi設定構成

**新規ファイル**: `apps/examples/security_camera/wifi_config.h`

```c
#ifndef __WIFI_CONFIG_H
#define __WIFI_CONFIG_H

// WiFi接続設定
#define WIFI_SSID           "YourSSID"          // 要設定
#define WIFI_PASSWORD       "YourPassword"      // 要設定
#define WIFI_SECURITY       WIFI_SECURITY_WPA2  // WPA2

// TCP/IPサーバー設定
#define TCP_SERVER_PORT     8888                // TCPポート
#define TCP_SERVER_BACKLOG  1                   // 接続待ちキュー

// IPアドレス設定
#define USE_DHCP            1                   // 1=DHCP, 0=固定IP
#define STATIC_IP_ADDR      "192.168.1.100"
#define STATIC_NETMASK      "255.255.255.0"
#define STATIC_GATEWAY      "192.168.1.1"

// タイムアウト設定
#define WIFI_CONNECT_TIMEOUT_SEC    30          // WiFi接続タイムアウト
#define TCP_ACCEPT_TIMEOUT_SEC      60          // クライアント接続待ちタイムアウト
#define TCP_SEND_TIMEOUT_SEC        5           // TCP送信タイムアウト

#endif /* __WIFI_CONFIG_H */
```

### 3.2 WiFi管理モジュール

**新規ファイル**: `apps/examples/security_camera/wifi_manager.c`

```c
#include <nuttx/config.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

#include "wifi_config.h"
#include "wifi_manager.h"

// WiFi管理構造体
typedef struct {
    int wifi_fd;                // WiFiデバイスFD
    int server_fd;              // TCPサーバーソケット
    int client_fd;              // TCPクライアントソケット
    bool wifi_connected;        // WiFi接続状態
    bool client_connected;      // クライアント接続状態
    struct sockaddr_in server_addr;
    struct sockaddr_in client_addr;
} wifi_manager_t;

static wifi_manager_t g_wifi_mgr;

/**
 * WiFi初期化
 */
int wifi_manager_init(void)
{
    memset(&g_wifi_mgr, 0, sizeof(wifi_manager_t));
    g_wifi_mgr.server_fd = -1;
    g_wifi_mgr.client_fd = -1;

    // WiFiドライバ初期化
    // TODO: iS110B固有の初期化処理

    return 0;
}

/**
 * WiFi接続
 */
int wifi_manager_connect(void)
{
    // WiFi Station Mode設定
    // TODO: iS110B WiFi接続API呼び出し

    // DHCP or 固定IP設定
#if USE_DHCP
    // DHCP取得
#else
    // 固定IP設定
#endif

    g_wifi_mgr.wifi_connected = true;
    printf("WiFi connected: %s\n", WIFI_SSID);

    return 0;
}

/**
 * TCPサーバー起動
 */
int wifi_manager_start_server(void)
{
    int ret;

    // ソケット作成
    g_wifi_mgr.server_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (g_wifi_mgr.server_fd < 0) {
        fprintf(stderr, "socket() failed: %d\n", errno);
        return -errno;
    }

    // SO_REUSEADDR設定
    int reuse = 1;
    setsockopt(g_wifi_mgr.server_fd, SOL_SOCKET, SO_REUSEADDR,
               &reuse, sizeof(reuse));

    // サーバーアドレス設定
    memset(&g_wifi_mgr.server_addr, 0, sizeof(struct sockaddr_in));
    g_wifi_mgr.server_addr.sin_family = AF_INET;
    g_wifi_mgr.server_addr.sin_port = htons(TCP_SERVER_PORT);
    g_wifi_mgr.server_addr.sin_addr.s_addr = INADDR_ANY;

    // bind
    ret = bind(g_wifi_mgr.server_fd,
               (struct sockaddr *)&g_wifi_mgr.server_addr,
               sizeof(struct sockaddr_in));
    if (ret < 0) {
        fprintf(stderr, "bind() failed: %d\n", errno);
        close(g_wifi_mgr.server_fd);
        return -errno;
    }

    // listen
    ret = listen(g_wifi_mgr.server_fd, TCP_SERVER_BACKLOG);
    if (ret < 0) {
        fprintf(stderr, "listen() failed: %d\n", errno);
        close(g_wifi_mgr.server_fd);
        return -errno;
    }

    printf("TCP server listening on port %d\n", TCP_SERVER_PORT);

    return 0;
}

/**
 * クライアント接続待ち
 */
int wifi_manager_accept_client(void)
{
    socklen_t client_len = sizeof(struct sockaddr_in);

    printf("Waiting for client connection...\n");

    g_wifi_mgr.client_fd = accept(g_wifi_mgr.server_fd,
                                   (struct sockaddr *)&g_wifi_mgr.client_addr,
                                   &client_len);

    if (g_wifi_mgr.client_fd < 0) {
        fprintf(stderr, "accept() failed: %d\n", errno);
        return -errno;
    }

    g_wifi_mgr.client_connected = true;

    char client_ip[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &g_wifi_mgr.client_addr.sin_addr, client_ip, sizeof(client_ip));
    printf("Client connected: %s:%d\n", client_ip,
           ntohs(g_wifi_mgr.client_addr.sin_port));

    return 0;
}

/**
 * データ送信 (USB transport代替)
 */
ssize_t wifi_manager_send(const uint8_t *data, size_t size)
{
    if (!g_wifi_mgr.client_connected) {
        return -ENOTCONN;
    }

    ssize_t sent = send(g_wifi_mgr.client_fd, data, size, 0);

    if (sent < 0) {
        if (errno == EPIPE || errno == ENOTCONN) {
            // クライアント切断
            fprintf(stderr, "Client disconnected\n");
            g_wifi_mgr.client_connected = false;
            close(g_wifi_mgr.client_fd);
            g_wifi_mgr.client_fd = -1;
        }
        return -errno;
    }

    return sent;
}

/**
 * WiFi切断
 */
void wifi_manager_disconnect(void)
{
    if (g_wifi_mgr.client_fd >= 0) {
        close(g_wifi_mgr.client_fd);
        g_wifi_mgr.client_fd = -1;
    }

    if (g_wifi_mgr.server_fd >= 0) {
        close(g_wifi_mgr.server_fd);
        g_wifi_mgr.server_fd = -1;
    }

    // WiFi切断
    // TODO: iS110B WiFi切断API呼び出し

    g_wifi_mgr.wifi_connected = false;
    g_wifi_mgr.client_connected = false;
}

/**
 * 接続状態取得
 */
bool wifi_manager_is_connected(void)
{
    return g_wifi_mgr.wifi_connected && g_wifi_mgr.client_connected;
}
```

### 3.3 メインアプリケーション統合

**変更ファイル**: `apps/examples/security_camera/camera_app_main.c`

```c
// Phase 7: WiFi or USB選択
#ifdef CONFIG_SECURITY_CAMERA_USE_WIFI
  #include "wifi_manager.h"
  #define transport_init()    wifi_manager_init()
  #define transport_connect() wifi_manager_connect()
  #define transport_start()   wifi_manager_start_server(); \
                               wifi_manager_accept_client()
  #define transport_send(data, size) wifi_manager_send(data, size)
  #define transport_disconnect() wifi_manager_disconnect()
  #define transport_is_connected() wifi_manager_is_connected()
#else
  #include "usb_transport.h"
  #define transport_init()    usb_transport_init()
  #define transport_connect() usb_transport_open()
  #define transport_start()   (0)  // USB is always ready
  #define transport_send(data, size) usb_transport_send_bytes(data, size)
  #define transport_disconnect() usb_transport_close()
  #define transport_is_connected() (usb_fd >= 0)
#endif

int main(int argc, FAR char *argv[])
{
    int ret;

    // Transport初期化 (WiFi or USB)
    ret = transport_init();
    if (ret < 0) {
        fprintf(stderr, "Transport init failed: %d\n", ret);
        return EXIT_FAILURE;
    }

    // Transport接続
    ret = transport_connect();
    if (ret < 0) {
        fprintf(stderr, "Transport connect failed: %d\n", ret);
        goto error;
    }

    // サーバー起動 (WiFiのみ)
    ret = transport_start();
    if (ret < 0) {
        fprintf(stderr, "Transport start failed: %d\n", ret);
        goto error;
    }

    // カメラ初期化 (既存)
    ret = camera_manager_init();
    // ...

    // メインループ (Phase 2パイプライン)
    ret = camera_threads_init();
    // ...

    return EXIT_SUCCESS;

error:
    transport_disconnect();
    return EXIT_FAILURE;
}
```

### 3.4 NuttX Kconfig設定

**変更ファイル**: `apps/examples/security_camera/Kconfig`

```kconfig
config SECURITY_CAMERA_USE_WIFI
	bool "Use WiFi transport instead of USB"
	default n
	select NETUTILS_DHCPC if SECURITY_CAMERA_WIFI_USE_DHCP
	select NET_TCP
	select NET_IPv4
	---help---
		Enable WiFi (TCP/IP) transport instead of USB CDC.
		Requires iS110B WiFi Add-on board.

if SECURITY_CAMERA_USE_WIFI

config SECURITY_CAMERA_WIFI_SSID
	string "WiFi SSID"
	default "YourSSID"

config SECURITY_CAMERA_WIFI_PASSWORD
	string "WiFi Password"
	default "YourPassword"

config SECURITY_CAMERA_WIFI_SECURITY
	int "WiFi Security (0=Open, 2=WPA2)"
	default 2

config SECURITY_CAMERA_TCP_PORT
	int "TCP Server Port"
	default 8888

config SECURITY_CAMERA_WIFI_USE_DHCP
	bool "Use DHCP for IP address"
	default y

if !SECURITY_CAMERA_WIFI_USE_DHCP

config SECURITY_CAMERA_STATIC_IP
	string "Static IP Address"
	default "192.168.1.100"

config SECURITY_CAMERA_NETMASK
	string "Netmask"
	default "255.255.255.0"

config SECURITY_CAMERA_GATEWAY
	string "Gateway"
	default "192.168.1.1"

endif # !SECURITY_CAMERA_WIFI_USE_DHCP

endif # SECURITY_CAMERA_USE_WIFI
```

---

## 4. PC側実装 (Rust)

### 4.1 TCP接続モジュール

**新規ファイル**: `src/tcp_connection.rs`

```rust
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// TCP接続管理
pub struct TcpConnection {
    stream: TcpStream,
    host: String,
    port: u16,
}

impl TcpConnection {
    /// 新しいTCP接続を作成
    pub fn new(host: &str, port: u16) -> io::Result<Self> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect_timeout(
            &addr.to_socket_addrs()?.next().unwrap(),
            Duration::from_secs(10),
        )?;

        // タイムアウト設定
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        // Nagleアルゴリズム無効化 (低遅延優先)
        stream.set_nodelay(true)?;

        Ok(Self {
            stream,
            host: host.to_string(),
            port,
        })
    }

    /// パケット読み込み (SerialConnection::read_packet()互換)
    pub fn read_packet(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut total_read = 0;

        // ヘッダー読み込み (14 bytes)
        while total_read < 14 {
            let n = self.stream.read(&mut buffer[total_read..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Connection closed by server",
                ));
            }
            total_read += n;
        }

        // ヘッダーパース (既存のprotocol.rsロジック使用)
        // JPEG size抽出
        let jpeg_size = u32::from_le_bytes([
            buffer[8], buffer[9], buffer[10], buffer[11],
        ]) as usize;

        // JPEG data + CRC読み込み
        let remaining = jpeg_size + 2; // JPEG + CRC16
        let mut read_so_far = 0;

        while read_so_far < remaining {
            let n = self.stream.read(&mut buffer[total_read..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Connection closed while reading JPEG",
                ));
            }
            total_read += n;
            read_so_far += n;
        }

        Ok(total_read)
    }

    /// 接続情報取得
    pub fn connection_info(&self) -> String {
        format!("TCP {}:{}", self.host, self.port)
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        // Graceful shutdown
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}
```

### 4.2 GUI統合

**変更ファイル**: `src/gui_main.rs`

```rust
// Phase 7: Transport選択
use crate::serial::SerialConnection;
use crate::tcp_connection::TcpConnection;

enum TransportType {
    Serial(SerialConnection),
    Tcp(TcpConnection),
}

impl TransportType {
    fn read_packet(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self {
            TransportType::Serial(conn) => conn.read_packet(buffer),
            TransportType::Tcp(conn) => conn.read_packet(buffer),
        }
    }

    fn connection_info(&self) -> String {
        match self {
            TransportType::Serial(conn) => conn.port_name().to_string(),
            TransportType::Tcp(conn) => conn.connection_info(),
        }
    }
}

// CameraApp構造体に追加
struct CameraApp {
    // ...
    transport_type: Arc<Mutex<Option<TransportType>>>,
    use_wifi: bool,  // 設定: WiFi or USB
    wifi_host: String,
    wifi_port: u16,
}

// 接続処理
fn start_capture(&mut self) {
    let transport = if self.use_wifi {
        // WiFi (TCP)接続
        match TcpConnection::new(&self.wifi_host, self.wifi_port) {
            Ok(tcp) => TransportType::Tcp(tcp),
            Err(e) => {
                error!("Failed to connect to {}:{}: {}", self.wifi_host, self.wifi_port, e);
                return;
            }
        }
    } else {
        // USB (Serial)接続
        match SerialConnection::auto_detect() {
            Ok(serial) => TransportType::Serial(serial),
            Err(e) => {
                error!("Failed to detect serial port: {}", e);
                return;
            }
        }
    };

    *self.transport_type.lock().unwrap() = Some(transport);

    // 既存のキャプチャスレッド起動
    // ...
}
```

### 4.3 設定UI追加

```rust
// GUI設定パネル
ui.horizontal(|ui| {
    ui.label("Transport:");
    ui.radio_value(&mut self.use_wifi, false, "USB");
    ui.radio_value(&mut self.use_wifi, true, "WiFi");
});

if self.use_wifi {
    ui.horizontal(|ui| {
        ui.label("Host:");
        ui.text_edit_singleline(&mut self.wifi_host);
    });
    ui.horizontal(|ui| {
        ui.label("Port:");
        ui.add(egui::DragValue::new(&mut self.wifi_port).clamp_range(1..=65535));
    });
}
```

---

## 5. プロトコル仕様 (変更なし)

Phase 7では、**既存のMJPEGプロトコルをそのまま使用**します。

**パケットフォーマット** (Phase 1-6と同一):
```
┌────────────────────────────────────────────────────────────┐
│ MJPEG Packet (Phase 1-6と同一)                             │
├────────────────────────────────────────────────────────────┤
│ [SYNC_WORD: 4] 0xCAFEBABE                                  │
│ [SEQUENCE: 4]  シーケンス番号                              │
│ [JPEG_SIZE: 4] JPEGデータサイズ                            │
│ [RESERVED: 2]  予約領域                                    │
│ [JPEG_DATA: N] JPEGフレームデータ (N = JPEG_SIZE bytes)    │
│ [CRC16: 2]     CRC-16-CCITT                                │
└────────────────────────────────────────────────────────────┘
Total: 14 + N + 2 bytes
```

**変更点**:
- 伝送路: USB CDC → **TCP/IP (WiFi)**のみ
- プロトコル: 変更なし
- CRC検証: 継続 (TCPにもCRCあるが、念のため)

---

## 6. パフォーマンス予測

### 6.1 帯域幅比較

| 項目 | USB CDC (Phase 1-6) | WiFi (Phase 7) |
|------|---------------------|----------------|
| 理論帯域 | 12 Mbps | 150 Mbps |
| 実効帯域 | ~8 Mbps | ~50-80 Mbps |
| 現在のFPS | 12.04 fps (VGA) | - |
| 必要帯域 | 4.8 Mbps (MJPEG) | 4.8 Mbps (同じ) |
| **予測FPS** | 12.04 fps | **12-15 fps** |

**改善見込み**:
- WiFiは帯域に余裕があるため、HD (1280×720) への移行も可能
- 遅延: USB < 10ms、WiFi 10-50ms (ネットワーク環境依存)

### 6.2 メモリ使用量 (Spresense)

| 項目 | Phase 6 (USB) | Phase 7 (WiFi) | 増加 |
|------|---------------|----------------|------|
| カメラバッファ | 794 KB | 794 KB | 0 KB |
| WiFiスタック | 0 KB | ~100 KB | +100 KB |
| TCPバッファ | 0 KB | ~50 KB | +50 KB |
| **合計** | 794 KB | **944 KB** | **+150 KB** |

**使用率**: 944 KB / 1536 KB = **61.5%** (許容範囲)

---

## 7. 移行計画

### 7.1 段階的移行

**Phase 7.0: 準備**
- iS110B WiFi Add-onボード購入
- NuttX WiFiドライバ確認
- 開発環境準備

**Phase 7.1: Spresense側実装**
- wifi_manager.c/h実装
- Kconfig設定追加
- ビルド・動作確認 (WiFi接続のみ)

**Phase 7.2: PC側実装**
- tcp_connection.rs実装
- GUI統合
- ビルド・動作確認 (TCP接続のみ)

**Phase 7.3: 統合テスト**
- WiFi経由でのMJPEGストリーミング
- FPS測定
- エラー率確認
- 長時間動作テスト

**Phase 7.4: USB互換性維持**
- USB/WiFi切替機能
- 設定UI
- ドキュメント整備

### 7.2 後方互換性

```c
// Spresense: ビルド時選択
make menuconfig
  → CONFIG_SECURITY_CAMERA_USE_WIFI=y/n

// PC Rust: 実行時選択
./security_camera_gui
  → GUI: Transport: (•) USB  ( ) WiFi
```

**両対応**: USBビルドとWiFiビルドの両方を提供

---

## 8. テスト計画

### 8.1 機能テスト

| テスト項目 | 期待結果 |
|-----------|---------|
| WiFi接続 | SSID接続成功、IP取得 |
| TCPサーバー起動 | ポート8888でlisten |
| PC接続 | TCP接続成功 |
| MJPEGストリーミング | フレーム受信、表示正常 |
| 切断・再接続 | エラーハンドリング正常 |

### 8.2 パフォーマンステスト

| 項目 | 目標 |
|------|------|
| FPS | ≥ 12 fps (USB同等以上) |
| 遅延 | < 100ms |
| エラー率 | < 0.1% |
| 連続動作 | 3時間以上 |

### 8.3 ストレステスト

- WiFi電波強度変化
- WiFi APリブート
- 長時間動作 (24時間)
- 複数クライアント接続 (拒否確認)

---

## 9. リスクと対策

### 9.1 WiFi接続不安定

**リスク**: WiFi電波環境により接続が不安定
**対策**:
- 再接続ロジック実装
- TCP KeepAlive設定
- エラーログ詳細化

### 9.2 遅延増加

**リスク**: WiFi遅延がUSBより大きい
**対策**:
- TCP_NODELAY設定 (Nagle無効)
- バッファサイズ最適化
- 遅延測定・表示

### 9.3 iS110Bドライバ不明

**リスク**: iS110BのNuttXドライバ対応状況不明
**対策**:
- 事前調査 (Spresense SDK examples確認)
- 代替WiFiモジュール検討 (ESP32等)
- USBフォールバック維持

---

## 10. Phase 8以降の展望

### Phase 8: RTSP対応 (ユーザー要望)

- RTSPサーバー実装 (Spresense)
- VLC等での直接再生
- 標準プロトコル対応

### Phase 9: HD (1280×720) 対応

- WiFi帯域を活用した高解像度
- H.264ハードウェアエンコード活用

### Phase 10: マルチクライアント対応

- 複数PC同時接続
- ブロードキャスト配信

---

**作成者**: Claude Code
**承認**: Pending
**次回レビュー**: iS110B購入・動作確認後
