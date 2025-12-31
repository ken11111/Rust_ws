# Spresense メトリクス送信プロトコル仕様

**バージョン**: 1.0
**作成日**: 2025-12-31
**対象フェーズ**: Phase 4.1 拡張
**ステータス**: 設計完了（実装前）

---

## 📋 概要

### 目的

Spresense 側の性能メトリクスを PC 側に送信し、CSV に記録することで、24 時間テストなどの長時間動作における Spresense 側の挙動を正確に評価可能にする。

### 背景

**Phase 4.1 の問題点**:
- PC 側でパケットシーケンス番号から Spresense FPS を推定していた
- この方式は **PC の受信レート** を測定しており、**Spresense の送信レート** ではない
- 例: Spresense が 30 fps で送信していても、シリアル通信のボトルネック (48ms) により PC が 20 fps でしか受信できない場合、計算結果は 20 fps となる

**ユーザー要求**:
1. GUI 上の Spresense FPS 表示を完全削除
2. CSV の PC 推定 Spresense FPS も削除
3. Spresense 側の実測メトリクスを PC に送信し CSV に記録

### 解決策

**二重パケットプロトコル**:
- **MJPEG パケット** (既存): 画像データ転送
- **メトリクスパケット** (新規): Spresense 性能データ転送

---

## 🔧 パケット設計

### パケット種別の識別

| パケット種別 | Sync Word | サイズ | 送信頻度 |
|--------------|-----------|--------|----------|
| MJPEG        | 0xCAFEBABE | 可変 (20-100KB) | 30 fps (33ms間隔) |
| Metrics      | 0xCAFEBEEF | 固定 (38 bytes) | 1秒間隔 or 30フレームごと |

### メトリクスパケット構造

```c
#define METRICS_SYNC_WORD 0xCAFEBEEF

typedef struct {
    // Header (8 bytes)
    uint32_t sync_word;        // 0xCAFEBEEF (識別用)
    uint32_t sequence;         // メトリクスパケットのシーケンス番号

    // Metrics Data (28 bytes)
    uint32_t timestamp_ms;     // Spresense 起動からの時刻 (ミリ秒)
    uint32_t camera_frames;    // カメラから取得した累積フレーム数
    uint32_t usb_packets;      // USB に送信した累積パケット数
    uint32_t action_q_depth;   // 現在の action queue 深度 (0-3)
    uint32_t avg_packet_size;  // 平均パケットサイズ (bytes)
    uint32_t errors;           // 累積エラー回数
    uint32_t reserved;         // 将来の拡張用 (0 で埋める)

    // Integrity Check (2 bytes)
    uint16_t crc16;            // CRC-16-CCITT (sync_word から reserved まで)
} __attribute__((packed)) metrics_packet_t;

// Total size: 38 bytes
```

### フィールド詳細

| フィールド | 型 | サイズ | 説明 |
|------------|-----|--------|------|
| `sync_word` | uint32_t | 4B | 0xCAFEBEEF (メトリクスパケット識別用) |
| `sequence` | uint32_t | 4B | メトリクスパケット番号 (0から開始、1ずつ増加) |
| `timestamp_ms` | uint32_t | 4B | Spresense 起動からの経過時間 (ミリ秒) |
| `camera_frames` | uint32_t | 4B | カメラから取得した総フレーム数 |
| `usb_packets` | uint32_t | 4B | USB に送信した総パケット数 (MJPEG + Metrics) |
| `action_q_depth` | uint32_t | 4B | 現在の action queue 深度 (0-3) |
| `avg_packet_size` | uint32_t | 4B | MJPEG パケットの平均サイズ (bytes) |
| `errors` | uint32_t | 4B | 累積エラー回数 (camera timeout, USB error など) |
| `reserved` | uint32_t | 4B | 将来の拡張用 (現在は 0) |
| `crc16` | uint16_t | 2B | CRC-16-CCITT (データ整合性チェック) |

### CRC-16 計算範囲

```
[sync_word (4B)] [sequence (4B)] ... [reserved (4B)] [crc16 (2B)]
 ←────────────────── CRC 計算範囲 (36 bytes) ──────────────────→
```

CRC は `sync_word` から `reserved` までの 36 バイトを対象とする。

---

## 🖥️ Spresense 側実装

### ファイル構成

```
apps/examples/security_camera/
├── mjpeg_protocol.h          # メトリクスパケット構造体定義を追加
├── camera_app_main.c         # メトリクス収集とパケット送信
└── mjpeg_packet.c            # send_metrics_packet() 実装
```

### mjpeg_protocol.h への追加

```c
/* Metrics packet definition */
#define METRICS_SYNC_WORD 0xCAFEBEEF

typedef struct {
    uint32_t sync_word;
    uint32_t sequence;
    uint32_t timestamp_ms;
    uint32_t camera_frames;
    uint32_t usb_packets;
    uint32_t action_q_depth;
    uint32_t avg_packet_size;
    uint32_t errors;
    uint32_t reserved;
    uint16_t crc16;
} __attribute__((packed)) metrics_packet_t;

/* Function prototypes */
int send_metrics_packet(int usb_fd, const metrics_packet_t *metrics);
```

### メトリクス収集 (camera_app_main.c)

```c
/* Global metrics counters */
static uint32_t g_metrics_sequence = 0;
static uint32_t g_camera_frames = 0;
static uint32_t g_usb_packets = 0;
static uint32_t g_total_packet_size = 0;
static uint32_t g_errors = 0;

/* Metrics transmission interval */
#define METRICS_INTERVAL_MS 1000  // 1 second
static uint32_t last_metrics_time_ms = 0;

void camera_thread_func(void *arg) {
    while (!shutdown_requested) {
        // 1. Camera capture
        ret = camera_get_frame(&frame);
        if (ret == OK) {
            g_camera_frames++;
        } else {
            g_errors++;
        }

        // 2. MJPEG packing
        packet_size = mjpeg_pack_frame(...);
        g_total_packet_size += packet_size;

        // 3. Enqueue to USB thread
        push_action_queue(packet);

        // 4. Check if metrics should be sent
        uint32_t now_ms = get_uptime_ms();
        if (now_ms - last_metrics_time_ms >= METRICS_INTERVAL_MS) {
            send_metrics_now();
            last_metrics_time_ms = now_ms;
        }
    }
}

void send_metrics_now(void) {
    metrics_packet_t metrics;

    metrics.sync_word = METRICS_SYNC_WORD;
    metrics.sequence = g_metrics_sequence++;
    metrics.timestamp_ms = get_uptime_ms();
    metrics.camera_frames = g_camera_frames;
    metrics.usb_packets = g_usb_packets;
    metrics.action_q_depth = get_action_queue_depth();
    metrics.avg_packet_size = (g_usb_packets > 0)
        ? (g_total_packet_size / g_usb_packets)
        : 0;
    metrics.errors = g_errors;
    metrics.reserved = 0;

    // Calculate CRC
    metrics.crc16 = crc16_ccitt((uint8_t*)&metrics,
                                 sizeof(metrics) - sizeof(uint16_t));

    // Send via USB (bypassing queue, direct write)
    send_metrics_packet(usb_fd, &metrics);
    g_usb_packets++;
}
```

### メトリクス送信関数 (mjpeg_packet.c)

```c
int send_metrics_packet(int usb_fd, const metrics_packet_t *metrics) {
    ssize_t written = 0;
    ssize_t total = sizeof(metrics_packet_t);
    const uint8_t *buf = (const uint8_t*)metrics;

    while (written < total) {
        ssize_t ret = write(usb_fd, buf + written, total - written);
        if (ret < 0) {
            if (errno == EINTR) continue;
            return -1;  // USB error
        }
        written += ret;
    }

    return 0;
}
```

### 送信タイミングの選択肢

| オプション | 間隔 | 利点 | 欠点 |
|-----------|------|------|------|
| **Option A: 1秒間隔** | 1000ms | 一定間隔、CSV 1行/秒と同期 | フレーム数と非同期 |
| **Option B: 30フレームごと** | ~1000ms (30fps時) | フレーム数と同期 | FPS変動時に間隔が変わる |

**推奨**: Option A (1秒間隔)
- CSV の統計更新と同期しやすい
- 安定した間隔でメトリクス取得

---

## 🖥️ PC 側実装

### ファイル構成

```
src/
├── protocol.rs               # Packet enum と MetricsPacket 定義
├── capture.rs                # read_packet() 修正
├── metrics.rs                # CSV format 拡張
└── gui_main.rs               # Spresense FPS 表示削除
```

### protocol.rs への追加

```rust
pub const METRICS_SYNC_WORD: u32 = 0xCAFEBEEF;

#[derive(Debug, Clone)]
pub struct MetricsPacket {
    pub sequence: u32,
    pub timestamp_ms: u32,
    pub camera_frames: u32,
    pub usb_packets: u32,
    pub action_q_depth: u32,
    pub avg_packet_size: u32,
    pub errors: u32,
}

#[derive(Debug)]
pub enum Packet {
    Mjpeg(MjpegPacket),
    Metrics(MetricsPacket),
}
```

### capture.rs の read_packet() 修正

```rust
pub fn read_packet(port: &mut Box<dyn SerialPort>) -> Result<Packet> {
    // 1. Read sync word
    let sync_word = read_u32(port)?;

    match sync_word {
        MJPEG_SYNC_WORD => {
            // 2a. Read MJPEG packet
            let sequence = read_u32(port)?;
            let jpeg_size = read_u32(port)?;
            let reserved = read_u32(port)?;
            let crc = read_u16(port)?;

            // Verify header CRC
            verify_header_crc(sync_word, sequence, jpeg_size, reserved, crc)?;

            // Read JPEG data
            let mut jpeg_data = vec![0u8; jpeg_size as usize];
            port.read_exact(&mut jpeg_data)?;

            // Verify data CRC
            let data_crc = read_u16(port)?;
            verify_data_crc(&jpeg_data, data_crc)?;

            Ok(Packet::Mjpeg(MjpegPacket {
                header: PacketHeader { sequence, jpeg_size },
                jpeg_data,
            }))
        }

        METRICS_SYNC_WORD => {
            // 2b. Read Metrics packet
            let sequence = read_u32(port)?;
            let timestamp_ms = read_u32(port)?;
            let camera_frames = read_u32(port)?;
            let usb_packets = read_u32(port)?;
            let action_q_depth = read_u32(port)?;
            let avg_packet_size = read_u32(port)?;
            let errors = read_u32(port)?;
            let _reserved = read_u32(port)?;
            let crc = read_u16(port)?;

            // Verify CRC (36 bytes)
            // TODO: Implement CRC verification

            Ok(Packet::Metrics(MetricsPacket {
                sequence,
                timestamp_ms,
                camera_frames,
                usb_packets,
                action_q_depth,
                avg_packet_size,
                errors,
            }))
        }

        _ => Err(anyhow!("Invalid sync word: 0x{:08X}", sync_word)),
    }
}
```

### gui_main.rs の修正

#### Spresense FPS 表示の削除

```rust
// BEFORE (削除)
pub struct CameraApp {
    // ...
    spresense_fps: f32,  // 削除
    spresense_fps_calc: SpresenseFpsCalculator,  // 削除
}

enum AppMessage {
    Stats {
        fps: f32,
        spresense_fps: f32,  // 削除
        // ...
    },
}

// AFTER (修正後)
pub struct CameraApp {
    // ...
    spresense_metrics: Option<SpresenseMetrics>,  // 追加
}

#[derive(Clone)]
pub struct SpresenseMetrics {
    pub timestamp_ms: u32,
    pub camera_frames: u32,
    pub camera_fps: f32,  // camera_frames から計算
    pub usb_packets: u32,
    pub action_q_depth: u32,
    pub avg_packet_size: u32,
    pub errors: u32,
}

enum AppMessage {
    Stats {
        fps: f32,
        // spresense_fps 削除
        // ...
    },
    SpresenseMetrics(SpresenseMetrics),  // 追加
}
```

#### capture_thread の修正

```rust
fn capture_thread(/* ... */) {
    // ...
    loop {
        match read_packet(&mut port) {
            Ok(Packet::Mjpeg(packet)) => {
                // 既存の MJPEG 処理
                // ...

                // Spresense FPS 計算は削除
                // let spresense_fps = spresense_fps_calc.update(packet.header.sequence);
            }

            Ok(Packet::Metrics(metrics)) => {
                // メトリクスパケット受信
                let spresense_metrics = SpresenseMetrics {
                    timestamp_ms: metrics.timestamp_ms,
                    camera_frames: metrics.camera_frames,
                    camera_fps: calculate_spresense_fps(&metrics),  // 実装必要
                    usb_packets: metrics.usb_packets,
                    action_q_depth: metrics.action_q_depth,
                    avg_packet_size: metrics.avg_packet_size,
                    errors: metrics.errors,
                };

                tx.send(AppMessage::SpresenseMetrics(spresense_metrics)).ok();
            }

            Err(e) => {
                error_count += 1;
                // ...
            }
        }
    }
}

fn calculate_spresense_fps(metrics: &MetricsPacket) -> f32 {
    // 前回のメトリクスとの差分から FPS を計算
    // camera_frames_delta / time_delta_seconds
    // 実装詳細は後述
    0.0  // Placeholder
}
```

### metrics.rs の CSV フォーマット拡張

```rust
pub struct PerformanceMetrics {
    // PC 側メトリクス
    pub timestamp: f64,
    pub pc_fps: f32,
    pub frame_count: u64,
    pub error_count: u32,
    pub decode_time_ms: f32,
    pub serial_read_time_ms: f32,
    pub texture_upload_time_ms: f32,
    pub jpeg_size_kb: f32,

    // Spresense 側メトリクス (Option で追加)
    pub spresense_timestamp_ms: Option<u32>,
    pub spresense_camera_frames: Option<u32>,
    pub spresense_camera_fps: Option<f32>,
    pub spresense_usb_packets: Option<u32>,
    pub spresense_action_q_depth: Option<u32>,
    pub spresense_avg_packet_size: Option<u32>,
    pub spresense_errors: Option<u32>,
}

impl MetricsLogger {
    pub fn new(metrics_dir: &str) -> io::Result<Self> {
        // ...
        writeln!(file, "timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb,spresense_timestamp_ms,spresense_camera_frames,spresense_camera_fps,spresense_usb_packets,spresense_action_q_depth,spresense_avg_packet_size,spresense_errors")?;
        // ...
    }

    pub fn log(&self, metrics: &PerformanceMetrics) -> io::Result<()> {
        let mut file = self.file.lock().unwrap();
        write!(
            file,
            "{:.3},{:.2},{},{},{:.2},{:.2},{:.2},{:.2}",
            metrics.timestamp,
            metrics.pc_fps,
            metrics.frame_count,
            metrics.error_count,
            metrics.decode_time_ms,
            metrics.serial_read_time_ms,
            metrics.texture_upload_time_ms,
            metrics.jpeg_size_kb,
        )?;

        // Spresense メトリクス (Option)
        if let Some(ts) = metrics.spresense_timestamp_ms {
            write!(file, ",{}", ts)?;
        } else {
            write!(file, ",")?;
        }
        if let Some(frames) = metrics.spresense_camera_frames {
            write!(file, ",{}", frames)?;
        } else {
            write!(file, ",")?;
        }
        if let Some(fps) = metrics.spresense_camera_fps {
            write!(file, ",{:.2}", fps)?;
        } else {
            write!(file, ",")?;
        }
        if let Some(packets) = metrics.spresense_usb_packets {
            write!(file, ",{}", packets)?;
        } else {
            write!(file, ",")?;
        }
        if let Some(depth) = metrics.spresense_action_q_depth {
            write!(file, ",{}", depth)?;
        } else {
            write!(file, ",")?;
        }
        if let Some(size) = metrics.spresense_avg_packet_size {
            write!(file, ",{}", size)?;
        } else {
            write!(file, ",")?;
        }
        if let Some(errors) = metrics.spresense_errors {
            writeln!(file, ",{}", errors)?;
        } else {
            writeln!(file, ",")?;
        }

        file.flush()?;
        Ok(())
    }
}
```

---

## 📊 CSV フォーマット

### 新しい CSV ヘッダー

```csv
timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb,spresense_timestamp_ms,spresense_camera_frames,spresense_camera_fps,spresense_usb_packets,spresense_action_q_depth,spresense_avg_packet_size,spresense_errors
```

### カラム定義

| カラム | 型 | 単位 | 説明 | データ元 |
|--------|-----|------|------|----------|
| `timestamp` | float | 秒 | Unix タイムスタンプ | PC |
| `pc_fps` | float | fps | PC 受信・表示 FPS | PC |
| `frame_count` | integer | フレーム | PC 受信フレーム数 | PC |
| `error_count` | integer | 回 | PC 側エラー回数 | PC |
| `decode_time_ms` | float | ms | JPEG デコード時間 | PC |
| `serial_read_time_ms` | float | ms | シリアル読み込み時間 | PC |
| `texture_upload_time_ms` | float | ms | テクスチャアップロード時間 | PC |
| `jpeg_size_kb` | float | KB | JPEG データサイズ | PC |
| `spresense_timestamp_ms` | integer | ms | Spresense 起動からの時刻 | **Spresense** |
| `spresense_camera_frames` | integer | フレーム | Spresense カメラフレーム数 | **Spresense** |
| `spresense_camera_fps` | float | fps | Spresense カメラ FPS | **Spresense** |
| `spresense_usb_packets` | integer | パケット | Spresense USB 送信数 | **Spresense** |
| `spresense_action_q_depth` | integer | 個 | Action queue 深度 | **Spresense** |
| `spresense_avg_packet_size` | integer | bytes | 平均パケットサイズ | **Spresense** |
| `spresense_errors` | integer | 回 | Spresense 側エラー回数 | **Spresense** |

### サンプルデータ

```csv
timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb,spresense_timestamp_ms,spresense_camera_frames,spresense_camera_fps,spresense_usb_packets,spresense_action_q_depth,spresense_avg_packet_size,spresense_errors
1735650622.145,19.8,120,0,2.3,48.2,0.0,53.1,5120,150,30.1,151,2,54231,0
1735650623.147,19.9,140,0,2.2,47.8,0.0,52.9,6121,180,29.9,181,1,53987,0
1735650624.149,20.1,160,0,2.4,48.5,0.0,53.4,7123,210,30.0,211,2,54102,0
```

**注目ポイント**:
- `spresense_camera_frames` (150, 180, 210) vs `frame_count` (120, 140, 160)
  - Spresense が 30 fps で送信、PC が 20 fps で受信していることが明確
- `spresense_camera_fps` (30.1, 29.9, 30.0)
  - Spresense の実測 FPS が記録される
- `spresense_action_q_depth` (2, 1, 2)
  - キューの状態が監視可能

---

## 🧪 テスト手順

### Phase 1: Spresense 側実装

1. `mjpeg_protocol.h` に構造体追加
2. `camera_app_main.c` にメトリクス収集追加
3. `mjpeg_packet.c` に `send_metrics_packet()` 追加
4. ビルド・フラッシュ
5. シリアルコンソールでメトリクス送信を確認

**検証**:
```bash
# Spresense のログ
Sent metrics: seq=0, camera_frames=30, usb_packets=31
Sent metrics: seq=1, camera_frames=60, usb_packets=62
```

### Phase 2: PC 側実装

1. `src/protocol.rs` に `Packet` enum と `MetricsPacket` 追加
2. `src/capture.rs` の `read_packet()` 修正
3. `src/gui_main.rs` から Spresense FPS 表示削除
4. `src/metrics.rs` の CSV フォーマット拡張
5. ビルド

**検証**:
```bash
RUST_LOG=info cargo run --release --features gui

# ログに以下が表示されることを確認
[INFO] Received Metrics packet: seq=0, camera_frames=30, fps=30.1
[INFO] Received Metrics packet: seq=1, camera_frames=60, fps=29.9
```

### Phase 3: 統合テスト

1. Spresense と PC を接続
2. GUI アプリケーション起動
3. 30 秒動作
4. CSV ファイル確認

**検証項目**:
- ✅ CSV に Spresense メトリクスが記録される
- ✅ GUI から Spresense FPS 表示が削除されている
- ✅ `spresense_camera_fps` が 29-31 fps
- ✅ `frame_count` < `spresense_camera_frames` (シリアルボトルネック検証)

---

## 📈 24 時間テストでの活用

### 評価項目

1. **Spresense 安定性**:
   ```bash
   # Spresense FPS の変動
   awk -F',' 'NR>1 {sum+=$11; count++} END {print "Avg:", sum/count}' metrics.csv
   ```
   **期待**: 29.5-30.5 fps (変動 < 3%)

2. **パケットドロップ検出**:
   ```bash
   # frame_count vs spresense_camera_frames のギャップ
   awk -F',' 'NR>1 {gap=$10-$3; print gap}' metrics.csv | tail -1
   ```
   **期待**: ギャップが一定または緩やかに増加 (PC 側の受信能力 20 fps)

3. **Queue 深度の推移**:
   ```bash
   # action_q_depth の分布
   awk -F',' 'NR>1 {print $13}' metrics.csv | sort | uniq -c
   ```
   **期待**: 0-3 の範囲で安定分布 (頻繁な 0 はバッファ飢餓、頻繁な 3 は USB ボトルネック)

4. **エラー発生状況**:
   ```bash
   # Spresense 側エラー
   awk -F',' 'NR>1 {print $15}' metrics.csv | tail -1
   ```
   **期待**: 0 (エラーなし)

---

## 🔍 トラブルシューティング

### 問題 1: メトリクスパケットが受信されない

**確認**:
```bash
# Spresense ログ
Sent metrics: seq=X  # 送信されているか

# PC ログ
[INFO] Received Metrics packet  # 受信されているか
```

**原因候補**:
1. Sync word のエンディアン不一致
2. CRC 計算ミス
3. シリアルバッファオーバーフロー

### 問題 2: CSV に Spresense データが記録されない

**確認**:
```bash
# CSV の末尾カラムが空
tail -5 metrics.csv
```

**原因**:
- メトリクスパケット受信後の `AppMessage::SpresenseMetrics` 送信漏れ
- CSV ログ時の Option 処理ミス

### 問題 3: Spresense FPS が異常値

**確認**:
```bash
# spresense_camera_fps が 0 または 1000+ fps
```

**原因**:
- `calculate_spresense_fps()` のロジックバグ
- 初回メトリクス時の分母 0

---

## 📚 参照

### 関連ドキュメント
- `PHASE4_TEST_GUIDE.md` - Phase 4 テスト手順
- `METRICS_GUIDE.md` - メトリクス測定ガイド (旧版、要更新)
- `/home/ken/Spr_ws/GH_wk_test/docs/security_camera/01_specifications/06_SOFTWARE_SPEC_PC_RUST.md` - PC ソフトウェア仕様書

### 実装ファイル
- **Spresense**: `/home/ken/Spr_ws/GH_wk_test/apps/examples/security_camera/`
- **PC**: `/home/ken/Rust_ws/security_camera_viewer/src/`

---

## ✅ チェックリスト

### Spresense 側
- [ ] `metrics_packet_t` 構造体定義
- [ ] メトリクス収集ロジック実装
- [ ] `send_metrics_packet()` 実装
- [ ] CRC-16 計算実装
- [ ] 1 秒間隔の送信タイマー実装
- [ ] ビルド・フラッシュ

### PC 側
- [ ] `Packet` enum 定義
- [ ] `MetricsPacket` 構造体定義
- [ ] `read_packet()` 修正
- [ ] Spresense FPS 表示削除 (GUI)
- [ ] Spresense FPS 削除 (CSV)
- [ ] CSV フォーマット拡張
- [ ] `calculate_spresense_fps()` 実装
- [ ] ビルド

### テスト
- [ ] Spresense 単体テスト (メトリクス送信確認)
- [ ] PC 単体テスト (メトリクス受信確認)
- [ ] 統合テスト (30 秒動作)
- [ ] CSV データ確認
- [ ] 24 時間テスト準備

---

**作成者**: Claude Code (Sonnet 4.5)
**レビュー状態**: 設計完了、実装前
**次のステップ**: Spresense 側実装 → PC 側実装 → 統合テスト
