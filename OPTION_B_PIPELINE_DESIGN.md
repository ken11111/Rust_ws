# Option B: 完全パイプライン設計（3 スレッド + 2 キュー）

**作成日**: 2025-12-31
**ステータス**: 📋 未実装（将来の改善案）
**前提条件**: 高速通信（WiFi, Ethernet, USB バルク転送など）

---

## 📋 概要

**目的**: Spresense Phase 1.5 パイプラインと同じ構成を PC 側でも実装し、完全な並列処理を実現する。

**対象シナリオ**:
- WiFi 通信（50-100 Mbps）に移行後
- Ethernet 通信（100 Mbps+）に移行後
- USB バルク転送（480 Mbps）に移行後

**現状（Option A）との違い**:
- Option A: 2 スレッド（Serial + Decode が同一スレッド）
- **Option B: 3 スレッド**（Serial, Decode, GUI が独立）

---

## 🎯 なぜ現状では未実装か

### 現状の USB CDC-ACM の制限

```
Serial 読み込み: 48ms (95.5%)  ← ボトルネック
JPEG デコード:    2.3ms (4.5%)
GUI 処理:         2-3ms (5.0%)
───────────────────────────────
Total: 50.3ms/frame = 19.9 fps

Option B 実装後の理論値:
Critical Path: max(48, 2.3, 2-3) = 48ms
FPS: 20.8 fps (+0.9 fps, +4.6% のみ)
```

**結論**: USB CDC-ACM では Serial がボトルネックすぎて、Option B の効果が小さい。

### WiFi 移行後の期待性能

```
WiFi (50 Mbps) の場合:
JPEG サイズ: 54 KB
Serial 読み込み: 54,000 × 8 / 50,000,000 = 8.6ms

Option A (2 スレッド):
Serial (8.6ms) → Decode (2.3ms) = 10.9ms/frame = 91.7 fps

Option B (3 スレッド):
max(8.6, 2.3, 2.3) = 8.6ms/frame = 116.3 fps

改善: 91.7 → 116.3 fps (+27%, +24.6 fps) ← 有意な改善！
```

**結論**: WiFi など高速通信では、Option B の効果が大きくなる。

---

## 🏗️ アーキテクチャ設計

### 全体構成

```
┌─────────────────┐      ┌─────────────────┐      ┌─────────────────┐
│ Serial Thread   │      │ Decode Thread   │      │ GUI Thread      │
│ Priority: 110   │      │ Priority: 105   │      │ Priority: 100   │
├─────────────────┤      ├─────────────────┤      ├─────────────────┤
│ 1. Network recv │      │ 4. Pull JPEG    │      │ 7. Pull RGBA    │
│    (WiFi/Eth)   │      │    from queue   │      │    from queue   │
│                 │      │                 │      │                 │
│ 2. Parse packet │      │ 5. Decode JPEG  │      │ 8. Upload       │
│    (MJPEG)      │      │    to RGBA      │      │    texture      │
│                 │      │                 │      │                 │
│ 3. Push to      │      │ 6. Push to      │      │ 9. Render       │
│    JPEG queue   │      │    RGBA queue   │      │    (60 FPS)     │
└────────┬────────┘      └────────┬────────┘      └─────────────────┘
         │                        │
         ↓                        ↓
    [JPEG Queue]            [RGBA Queue]
    ┌──────────┐            ┌──────────┐
    │ Action Q │            │ Action Q │
    │ Empty Q  │            │ Empty Q  │
    │ Depth: 3 │            │ Depth: 3 │
    └──────────┘            └──────────┘
```

### スレッド間同期

```
Serial Thread                 Decode Thread               GUI Thread
─────────────                 ─────────────               ──────────
while running {               while running {             while running {
  1. recv_data()                4. lock(jpeg_mutex)         7. lock(rgba_mutex)
                                5. wait for JPEG              8. wait for RGBA
  2. lock(jpeg_mutex)              (cond_wait)                  (cond_wait)
  3. push JPEG                  6. unlock(jpeg_mutex)       9. unlock(rgba_mutex)
     (action_q)
                                7. decode JPEG              10. upload texture
  4. signal decode
     (cond_signal)              8. lock(rgba_mutex)         11. render
                                9. push RGBA
  5. unlock(jpeg_mutex)            (action_q)
                                10. signal GUI
  6. lock(jpeg_mutex)               (cond_signal)
  7. pull empty JPEG
     (empty_q)                  11. unlock(rgba_mutex)
  8. unlock(jpeg_mutex)
                                12. lock(rgba_mutex)
                                13. pull empty RGBA
                                    (empty_q)
                                14. unlock(rgba_mutex)
}                             }                           }
```

---

## 📦 データ構造設計

### 1. Frame Buffer 構造体

```rust
/// Frame buffer for pipeline processing
#[derive(Debug)]
pub struct FrameBuffer {
    /// Buffer ID (0-5)
    pub id: usize,

    /// JPEG data (compressed)
    pub jpeg_data: Vec<u8>,

    /// RGBA data (decompressed)
    pub rgba_data: Vec<u8>,

    /// Image dimensions
    pub width: u32,
    pub height: u32,

    /// Sequence number
    pub sequence: u32,

    /// Timestamp
    pub timestamp: std::time::Instant,
}

impl FrameBuffer {
    /// Create a new frame buffer
    pub fn new(id: usize, max_jpeg_size: usize, width: u32, height: u32) -> Self {
        Self {
            id,
            jpeg_data: vec![0u8; max_jpeg_size],
            rgba_data: vec![0u8; (width * height * 4) as usize],
            width,
            height,
            sequence: 0,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Reset buffer for reuse
    pub fn reset(&mut self) {
        self.sequence = 0;
        self.jpeg_data.clear();
        self.rgba_data.clear();
    }
}
```

### 2. Frame Queue 構造体

```rust
use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;

/// Thread-safe frame queue with action/empty queues
pub struct FrameQueue {
    /// Action queue (filled frames)
    action_queue: Arc<Mutex<VecDeque<Arc<FrameBuffer>>>>,

    /// Empty queue (available buffers)
    empty_queue: Arc<Mutex<VecDeque<Arc<FrameBuffer>>>>,

    /// Condition variable for signaling
    cond_var: Arc<Condvar>,

    /// Maximum queue depth
    max_depth: usize,
}

impl FrameQueue {
    /// Create a new frame queue with initial buffers
    pub fn new(max_depth: usize, max_jpeg_size: usize, width: u32, height: u32) -> Self {
        let mut empty_queue = VecDeque::new();

        // Allocate initial buffers
        for i in 0..max_depth {
            let buffer = Arc::new(FrameBuffer::new(i, max_jpeg_size, width, height));
            empty_queue.push_back(buffer);
        }

        Self {
            action_queue: Arc::new(Mutex::new(VecDeque::new())),
            empty_queue: Arc::new(Mutex::new(empty_queue)),
            cond_var: Arc::new(Condvar::new()),
            max_depth,
        }
    }

    /// Pull an empty buffer (blocking)
    pub fn pull_empty(&self) -> Option<Arc<FrameBuffer>> {
        let mut empty_q = self.empty_queue.lock().unwrap();

        // Wait if no empty buffers available
        while empty_q.is_empty() {
            empty_q = self.cond_var.wait(empty_q).unwrap();
        }

        empty_q.pop_front()
    }

    /// Push a filled buffer to action queue
    pub fn push_action(&self, buffer: Arc<FrameBuffer>) {
        let mut action_q = self.action_queue.lock().unwrap();
        action_q.push_back(buffer);

        // Signal waiting thread
        self.cond_var.notify_one();
    }

    /// Pull a filled buffer from action queue (blocking)
    pub fn pull_action(&self) -> Option<Arc<FrameBuffer>> {
        let mut action_q = self.action_queue.lock().unwrap();

        // Wait if no filled buffers available
        while action_q.is_empty() {
            action_q = self.cond_var.wait(action_q).unwrap();
        }

        action_q.pop_front()
    }

    /// Return an empty buffer to empty queue
    pub fn push_empty(&self, buffer: Arc<FrameBuffer>) {
        let mut empty_q = self.empty_queue.lock().unwrap();
        empty_q.push_back(buffer);

        // Signal waiting thread
        self.cond_var.notify_one();
    }

    /// Get current action queue depth
    pub fn action_depth(&self) -> usize {
        self.action_queue.lock().unwrap().len()
    }

    /// Get current empty queue depth
    pub fn empty_depth(&self) -> usize {
        self.empty_queue.lock().unwrap().len()
    }
}
```

### 3. Pipeline Context 構造体

```rust
use std::sync::atomic::{AtomicBool, Ordering};

/// Pipeline context shared across threads
pub struct PipelineContext {
    /// JPEG queue (Serial → Decode)
    pub jpeg_queue: Arc<FrameQueue>,

    /// RGBA queue (Decode → GUI)
    pub rgba_queue: Arc<FrameQueue>,

    /// Running flag
    pub running: Arc<AtomicBool>,

    /// Frame statistics
    pub stats: Arc<Mutex<PipelineStats>>,
}

#[derive(Debug, Default)]
pub struct PipelineStats {
    pub total_frames: u64,
    pub serial_thread_fps: f32,
    pub decode_thread_fps: f32,
    pub gui_thread_fps: f32,
    pub jpeg_queue_depth_avg: f32,
    pub rgba_queue_depth_avg: f32,
}

impl PipelineContext {
    pub fn new(max_depth: usize, max_jpeg_size: usize, width: u32, height: u32) -> Self {
        Self {
            jpeg_queue: Arc::new(FrameQueue::new(max_depth, max_jpeg_size, width, height)),
            rgba_queue: Arc::new(FrameQueue::new(max_depth, max_jpeg_size, width, height)),
            running: Arc::new(AtomicBool::new(true)),
            stats: Arc::new(Mutex::new(PipelineStats::default())),
        }
    }

    pub fn shutdown(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}
```

---

## 🔧 スレッド実装

### 1. Serial Thread（Network 受信 + JPEG キューへ）

```rust
use std::thread;

/// Serial/Network thread: Receive JPEG data and push to queue
pub fn serial_thread(
    ctx: Arc<PipelineContext>,
    mut serial: SerialConnection,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("serial_thread".to_string())
        .spawn(move || {
            log::info!("Serial thread started");

            let mut frame_count = 0u64;
            let mut last_stats = std::time::Instant::now();

            while ctx.is_running() {
                // 1. Pull empty buffer from JPEG queue
                let buffer = match ctx.jpeg_queue.pull_empty() {
                    Some(b) => b,
                    None => continue,
                };

                // 2. Receive JPEG data
                let start = std::time::Instant::now();
                match serial.read_packet() {
                    Ok(packet) => {
                        // Copy JPEG data to buffer
                        let mut buffer_mut = Arc::make_mut(&mut buffer.clone());
                        buffer_mut.jpeg_data = packet.jpeg_data;
                        buffer_mut.sequence = packet.sequence;
                        buffer_mut.timestamp = std::time::Instant::now();

                        // 3. Push to JPEG action queue
                        ctx.jpeg_queue.push_action(buffer);

                        frame_count += 1;

                        // Update statistics
                        let elapsed = start.elapsed().as_secs_f32();
                        if elapsed >= 1.0 {
                            let fps = frame_count as f32 / elapsed;
                            let mut stats = ctx.stats.lock().unwrap();
                            stats.serial_thread_fps = fps;
                            stats.jpeg_queue_depth_avg = ctx.jpeg_queue.action_depth() as f32;

                            frame_count = 0;
                            last_stats = std::time::Instant::now();
                        }
                    }
                    Err(e) => {
                        log::error!("Serial read error: {}", e);
                        // Return buffer to empty queue
                        ctx.jpeg_queue.push_empty(buffer);
                    }
                }
            }

            log::info!("Serial thread stopped");
        })
        .expect("Failed to spawn serial thread")
}
```

### 2. Decode Thread（JPEG デコード + RGBA キューへ）

```rust
/// Decode thread: Decode JPEG to RGBA and push to queue
pub fn decode_thread(ctx: Arc<PipelineContext>) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("decode_thread".to_string())
        .spawn(move || {
            log::info!("Decode thread started");

            let mut frame_count = 0u64;
            let mut last_stats = std::time::Instant::now();

            while ctx.is_running() {
                // 1. Pull filled JPEG buffer
                let jpeg_buffer = match ctx.jpeg_queue.pull_action() {
                    Some(b) => b,
                    None => continue,
                };

                // 2. Pull empty RGBA buffer
                let rgba_buffer = match ctx.rgba_queue.pull_empty() {
                    Some(b) => b,
                    None => {
                        // Return JPEG buffer if RGBA buffer unavailable
                        ctx.jpeg_queue.push_empty(jpeg_buffer);
                        continue;
                    }
                };

                // 3. Decode JPEG to RGBA
                let start = std::time::Instant::now();
                match image::load_from_memory(&jpeg_buffer.jpeg_data) {
                    Ok(img) => {
                        let rgba = img.to_rgba8();

                        // Copy to RGBA buffer
                        let mut rgba_buf_mut = Arc::make_mut(&mut rgba_buffer.clone());
                        rgba_buf_mut.rgba_data = rgba.into_raw();
                        rgba_buf_mut.width = img.width();
                        rgba_buf_mut.height = img.height();
                        rgba_buf_mut.sequence = jpeg_buffer.sequence;
                        rgba_buf_mut.timestamp = jpeg_buffer.timestamp;

                        // 4. Push to RGBA action queue
                        ctx.rgba_queue.push_action(rgba_buffer);

                        // 5. Return JPEG buffer to empty queue
                        ctx.jpeg_queue.push_empty(jpeg_buffer);

                        frame_count += 1;

                        // Update statistics
                        let elapsed = start.elapsed().as_secs_f32();
                        if elapsed >= 1.0 {
                            let fps = frame_count as f32 / elapsed;
                            let mut stats = ctx.stats.lock().unwrap();
                            stats.decode_thread_fps = fps;
                            stats.rgba_queue_depth_avg = ctx.rgba_queue.action_depth() as f32;

                            frame_count = 0;
                            last_stats = std::time::Instant::now();
                        }
                    }
                    Err(e) => {
                        log::error!("JPEG decode error: {}", e);
                        // Return buffers
                        ctx.jpeg_queue.push_empty(jpeg_buffer);
                        ctx.rgba_queue.push_empty(rgba_buffer);
                    }
                }
            }

            log::info!("Decode thread stopped");
        })
        .expect("Failed to spawn decode thread")
}
```

### 3. GUI Thread（RGBA 受信 + レンダリング）

```rust
impl eframe::App for CameraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaint
        ctx.request_repaint();

        // Pull RGBA buffer from queue (non-blocking)
        if let Some(rgba_buffer) = self.pipeline_ctx.rgba_queue.pull_action() {
            let start = std::time::Instant::now();

            // Create texture from RGBA data
            let size = [rgba_buffer.width as usize, rgba_buffer.height as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                &rgba_buffer.rgba_data,
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

            let texture_upload_time = start.elapsed().as_secs_f32() * 1000.0;

            // Update statistics
            self.frame_count += 1;
            self.texture_upload_time_ms = texture_upload_time;

            // Return buffer to empty queue
            self.pipeline_ctx.rgba_queue.push_empty(rgba_buffer);
        }

        // Render GUI (same as before)
        self.render_ui(ctx);
    }
}
```

---

## 📊 メモリ使用量

### バッファ構成

| 種類 | 個数 | サイズ/個 | 合計 |
|------|------|-----------|------|
| JPEG バッファ | 3 | 100 KB | 300 KB |
| RGBA バッファ | 3 | 1.2 MB | 3.6 MB |
| **合計** | **6** | - | **3.9 MB** |

### 比較

| 実装 | メモリ使用量 |
|------|-------------|
| Option A（現状） | ~1.2 MB |
| **Option B** | **~3.9 MB** |

**増加**: +2.7 MB（許容範囲）

---

## 🚀 実装手順（将来）

### Phase 1: 基礎実装

1. ✅ `FrameBuffer` 構造体実装
2. ✅ `FrameQueue` 構造体実装
3. ✅ `PipelineContext` 構造体実装

### Phase 2: スレッド実装

4. ✅ Serial Thread 実装
5. ✅ Decode Thread 実装
6. ✅ GUI Thread 修正

### Phase 3: テスト

7. ✅ ユニットテスト（Queue 操作）
8. ✅ 統合テスト（3 スレッド動作）
9. ✅ 性能テスト（FPS 測定）

### Phase 4: 最適化

10. ✅ Queue Depth チューニング
11. ✅ スレッド優先度調整
12. ✅ メモリ使用量削減

---

## 📈 期待される性能（WiFi 移行後）

### 前提条件

- **通信方式**: WiFi 802.11n（50 Mbps）
- **JPEG サイズ**: 54 KB
- **解像度**: VGA (640×480)

### Option A vs Option B

| 項目 | Option A | Option B | 改善 |
|------|---------|---------|------|
| Serial 読み込み | 8.6 ms | 8.6 ms | - |
| JPEG デコード | 2.3 ms（直列） | 2.3 ms（並列）| - |
| GUI 処理 | 2.3 ms（直列） | 2.3 ms（並列）| - |
| **Total** | **10.9 ms** | **8.6 ms** | **-21%** |
| **FPS** | **91.7** | **116.3** | **+27%** |

**結論**: WiFi では Option B が有意に高速

---

## ⚠️ 注意事項

### 1. デッドロック防止

**ロック順序を統一**:
```rust
// 常にこの順序でロック
1. jpeg_mutex
2. rgba_mutex
```

**タイムアウト設定**:
```rust
let timeout = Duration::from_secs(1);
let result = cond_var.wait_timeout(mutex, timeout);
```

### 2. バッファリーク防止

**すべてのバッファを追跡**:
```rust
fn verify_buffer_count(ctx: &PipelineContext) {
    let jpeg_action = ctx.jpeg_queue.action_depth();
    let jpeg_empty = ctx.jpeg_queue.empty_depth();
    let rgba_action = ctx.rgba_queue.action_depth();
    let rgba_empty = ctx.rgba_queue.empty_depth();

    assert_eq!(jpeg_action + jpeg_empty, 3, "JPEG buffer leak!");
    assert_eq!(rgba_action + rgba_empty, 3, "RGBA buffer leak!");
}
```

### 3. Graceful Shutdown

```rust
impl Drop for PipelineContext {
    fn drop(&mut self) {
        // Signal shutdown
        self.shutdown();

        // Wait for all threads to finish
        // (handled by JoinHandle)

        // Verify no buffer leaks
        verify_buffer_count(self);
    }
}
```

---

## 📚 参考実装

### Spresense 側（Phase 1.5）

参考ファイル:
- `/home/ken/Spr_ws/GH_wk_test/apps/examples/security_camera/frame_queue.c`
- `/home/ken/Spr_ws/GH_wk_test/apps/examples/security_camera/frame_queue.h`
- `/home/ken/Spr_ws/GH_wk_test/apps/examples/security_camera/camera_threads.c`

**実装の参考ポイント**:
1. Queue 操作の排他制御（mutex + condition variable）
2. Buffer プールの管理
3. Producer-Consumer パターン

### Rust 並行プログラミング

参考資料:
- [The Rust Programming Language - Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [std::sync::mpsc](https://doc.rust-lang.org/std/sync/mpsc/)
- [crossbeam チャンネル](https://docs.rs/crossbeam/)

---

## 🎯 実装トリガー

### いつ Option B を実装すべきか

以下の条件を**すべて**満たす場合:

1. ✅ **通信が高速化**:
   - WiFi (50+ Mbps)
   - Ethernet (100+ Mbps)
   - USB バルク転送 (480 Mbps)

2. ✅ **Serial 時間が短縮**:
   - Serial 読み込み < 10 ms

3. ✅ **並列化の効果が見込める**:
   - Serial 時間とデコード時間が同程度
   - または、デコード時間 > 5 ms

### 判断基準

```
Serial 時間 / Decode 時間 の比率:

> 10: Option B 不要（Serial がボトルネック）
5-10: Option B 検討（小さな改善）
2-5:  Option B 推奨（有意な改善）
< 2:  Option B 必須（大きな改善）

現状（USB CDC-ACM）: 48 / 2.3 = 20.9 → 不要
WiFi (50 Mbps):      8.6 / 2.3 = 3.7  → 推奨
WiFi (100 Mbps):     4.3 / 2.3 = 1.9  → 必須
```

---

## ✅ チェックリスト（実装時）

### 設計フェーズ

- [ ] Queue Depth の決定（推奨: 3）
- [ ] バッファサイズの決定
- [ ] スレッド優先度の決定
- [ ] タイムアウト値の決定

### 実装フェーズ

- [ ] FrameBuffer 構造体実装
- [ ] FrameQueue 構造体実装
- [ ] PipelineContext 構造体実装
- [ ] Serial Thread 実装
- [ ] Decode Thread 実装
- [ ] GUI Thread 修正

### テストフェーズ

- [ ] ユニットテスト（Queue 操作）
- [ ] デッドロックテスト
- [ ] バッファリークテスト
- [ ] 性能テスト（FPS 測定）
- [ ] 長時間動作テスト（24 時間）

### ドキュメントフェーズ

- [ ] API ドキュメント作成
- [ ] パフォーマンス測定結果記録
- [ ] トラブルシューティングガイド作成

---

## 📝 まとめ

**Option B は将来の高速通信移行時に有効な設計**です。

**現状（USB CDC-ACM）**:
- ❌ 効果小（+4.6%）
- ❌ 実装コスト高
- **結論**: 実装不要

**WiFi 移行後**:
- ✅ 効果大（+27%）
- ✅ 実装コスト正当化
- **結論**: 実装推奨

このドキュメントを基に、WiFi など高速通信に移行した際に Option B を実装してください。

---

**作成者**: Claude Code (Sonnet 4.5)
**作成日**: 2025-12-31
**バージョン**: 1.0
**ステータス**: 📋 設計完了・実装待ち
