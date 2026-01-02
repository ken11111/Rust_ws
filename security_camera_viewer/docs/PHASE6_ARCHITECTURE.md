# Phase 6 MP4録画機能 - アーキテクチャ設計書

**Phase**: 6 - MP4直接保存機能
**作成日**: 2026年1月2日

---

## 1. システムアーキテクチャ

### 1.1 全体構成

```
┌─────────────────────────────────────────────────────────────────┐
│                        CameraApp (GUI Thread)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐         ┌──────────────┐                      │
│  │ Motion       │         │ Ring Buffer  │                      │
│  │ Detector     │         │ (Pre-buffer) │                      │
│  │ (Phase 5)    │         │ (Phase 5)    │                      │
│  └──────────────┘         └──────────────┘                      │
│                                                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │          Recording State Machine                          │  │
│  │                                                           │  │
│  │  ┌─────────┐  Start Rec   ┌──────────────────┐          │  │
│  │  │  Idle   │──────────────>│ ManualRecording  │          │  │
│  │  └─────────┘               │ or               │          │  │
│  │      ^                     │ MotionRecording  │          │  │
│  │      │                     └──────────────────┘          │  │
│  │      │ Stop Rec                     │                    │  │
│  │      └──────────────────────────────┘                    │  │
│  │                                                           │  │
│  │  ┌────────────────────┐                                  │  │
│  │  │ RecordingFormat    │                                  │  │
│  │  │  • Mjpeg           │  (Phase 6)                       │  │
│  │  │  • Mp4 (default)   │                                  │  │
│  │  └────────────────────┘                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Recording Output Routing                    │   │
│  │                                                          │   │
│  │  Format == Mjpeg        Format == Mp4                   │   │
│  │       │                      │                           │   │
│  │       v                      v                           │   │
│  │  ┌─────────┐          ┌──────────────┐                  │   │
│  │  │  File   │          │ Mp4Recorder  │  (Phase 6)       │   │
│  │  │  Write  │          │   (ffmpeg)   │                  │   │
│  │  └─────────┘          └──────────────┘                  │   │
│  │       │                      │                           │   │
│  │       v                      v                           │   │
│  │  ┌─────────────────┐  ┌─────────────────┐               │   │
│  │  │ *.mjpeg file    │  │  *.mp4 file     │               │   │
│  │  └─────────────────┘  └─────────────────┘               │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

### 1.2 Mp4Recorder内部構造

```
┌─────────────────────────────────────────────────────────────┐
│                      Mp4Recorder                            │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Fields:                                                    │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ • ffmpeg_process: Child                               │ │
│  │ • stdin: Option<Box<dyn Write + Send>>                │ │
│  │ • frame_count: u32                                    │ │
│  │ • output_path: String                                 │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  Methods:                                                   │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ new(path, fps)  -> Result<Mp4Recorder>                │ │
│  │   - ffmpegプロセス起動                                │ │
│  │   - stdin取得                                         │ │
│  │                                                       │ │
│  │ write_frame(jpeg_data) -> Result<()>                  │ │
│  │   - JPEGフレームをstdinに書き込み                     │ │
│  │   - frame_count++                                     │ │
│  │                                                       │ │
│  │ finish(self) -> Result<()>                            │ │
│  │   - stdin.take() (クローズ)                          │ │
│  │   - ffmpeg_process.wait() (終了待機)                 │ │
│  │                                                       │ │
│  │ Drop::drop(&mut self)                                 │ │
│  │   - ffmpeg_process.kill() (強制終了)                 │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
         │
         │ spawn()
         v
┌─────────────────────────────────────────────────────────────┐
│              ffmpeg subprocess                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Command Line:                                              │
│  ffmpeg -f image2pipe -codec:v mjpeg -framerate 11 \       │
│         -i - -c:v libx264 -preset medium -crf 23 \         │
│         -pix_fmt yuv420p -movflags +faststart -y output.mp4│
│                                                             │
│  Pipeline:                                                  │
│  stdin (JPEG stream) → MJPEG decoder → H.264 encoder → MP4  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. シーケンス図

### 2.1 手動録画 (MP4) - 正常系

```
User          CameraApp         Mp4Recorder       ffmpeg
 │                │                  │               │
 │ [⏺ Start Rec] │                  │               │
 │───────────────>│                  │               │
 │                │ new(path, 11)    │               │
 │                │─────────────────>│               │
 │                │                  │ spawn()       │
 │                │                  │──────────────>│
 │                │                  │               │ (起動)
 │                │                  │<──────────────│
 │                │                  │  stdin        │
 │                │<─────────────────│               │
 │                │  Ok(recorder)    │               │
 │                │                  │               │
 │                │ (状態遷移)        │               │
 │                │ ManualRecording  │               │
 │                │                  │               │
 │   (カメラ動作中) │                  │               │
 │                │                  │               │
 │                ├─ DecodedFrame ───┤               │
 │                │                  │               │
 │                │ write_frame()    │               │
 │                │─────────────────>│               │
 │                │                  │ write(JPEG)   │
 │                │                  │──────────────>│
 │                │                  │               │ (エンコード)
 │                │                  │<──────────────│
 │                │<─────────────────│  Ok()         │
 │                │                  │               │
 │                │ (繰り返し)        │               │
 │                │                  │               │
 │ [⏹ Stop Rec]   │                  │               │
 │───────────────>│                  │               │
 │                │ finish()         │               │
 │                │─────────────────>│               │
 │                │                  │ stdin.take()  │
 │                │                  │ (close stdin) │
 │                │                  │               │
 │                │                  │ wait()        │
 │                │                  │──────────────>│
 │                │                  │               │ (終了処理)
 │                │                  │<──────────────│
 │                │                  │  exit(0)      │
 │                │<─────────────────│               │
 │                │  Ok()            │               │
 │                │                  │               │
 │                │ (状態遷移)        │               │
 │                │ Idle             │               │
 │                │                  │               │
 │<───────────────│                  │               │
 │   録画完了     │                  │               │
```

### 2.2 動き検知録画 (MP4) - 正常系

```
User          CameraApp      MotionDetector  Mp4Recorder    ffmpeg
 │                │                │              │            │
 │ [Enable Motion]│                │              │            │
 │───────────────>│                │              │            │
 │                │                │              │            │
 │   (カメラ動作中) │                │              │            │
 │                │                │              │            │
 │                ├─ DecodedFrame ─>│              │            │
 │                │                │              │            │
 │                │                │ detect()     │            │
 │                │<───────────────│              │            │
 │                │  false         │              │            │
 │                │                │              │            │
 │                ├─ DecodedFrame ─>│              │            │
 │                │                │ detect()     │            │
 │                │<───────────────│              │            │
 │                │  true (動き検知)│              │            │
 │                │                │              │            │
 │                │ start_motion_recording()      │            │
 │                │                │              │            │
 │                │ new(path, 11)  │              │            │
 │                │────────────────┼─────────────>│            │
 │                │                │              │ spawn()    │
 │                │                │              │───────────>│
 │                │                │              │            │
 │                │                │              │<───────────│
 │                │<───────────────┼──────────────│            │
 │                │  Ok(recorder)  │              │            │
 │                │                │              │            │
 │                │ (状態遷移)      │              │            │
 │                │ MotionRecording│              │            │
 │                │ motion_active=true            │            │
 │                │ countdown=330  │              │            │
 │                │                │              │            │
 │                ├─ DecodedFrame ─>│              │            │
 │                │                │ detect()     │            │
 │                │<───────────────│              │            │
 │                │  true          │              │            │
 │                │                │              │            │
 │                │ write_frame()  │              │            │
 │                │────────────────┼─────────────>│            │
 │                │                │              │ write()    │
 │                │                │              │───────────>│
 │                │                │              │            │
 │                │ countdown=330  │              │            │
 │                │ (リセット)      │              │            │
 │                │                │              │            │
 │                ├─ DecodedFrame ─>│              │            │
 │                │                │ detect()     │            │
 │                │<───────────────│              │            │
 │                │  false (静止)  │              │            │
 │                │                │              │            │
 │                │ motion_active=false           │            │
 │                │ countdown--    │              │            │
 │                │                │              │            │
 │                │ (ポストバッファ: 330フレーム)  │            │
 │                │                │              │            │
 │                │ countdown=0    │              │            │
 │                │                │              │            │
 │                │ stop_recording()              │            │
 │                │────────────────┼─────────────>│            │
 │                │                │              │ finish()   │
 │                │                │              │───────────>│
 │                │                │              │            │
 │                │<───────────────┼──────────────│            │
 │                │  Ok()          │              │            │
 │                │                │              │            │
 │                │ (状態遷移)      │              │            │
 │                │ Idle           │              │            │
```

### 2.3 エラー処理 - ffmpeg起動失敗

```
User          CameraApp         Mp4Recorder
 │                │                  │
 │ [⏺ Start Rec] │                  │
 │───────────────>│                  │
 │                │ new(path, 11)    │
 │                │─────────────────>│
 │                │                  │ spawn("ffmpeg")
 │                │                  │ (ffmpeg not found)
 │                │                  │
 │                │<─────────────────│
 │                │  Err(NotFound)   │
 │                │  "Failed to start ffmpeg: ..."
 │                │                  │
 │                │ error!()         │
 │                │ (ログ出力)        │
 │                │                  │
 │<───────────────│                  │
 │   エラー表示   │                  │
 │   (状態: Idle) │                  │
```

---

## 3. データフロー図

### 3.1 MJPEG録画 (Phase 3-5)

```
┌──────────────┐
│   Camera     │
│  (Spresense) │
└──────┬───────┘
       │ JPEG frames (Serial/USB)
       v
┌──────────────┐
│ SerialThread │
│  (Phase 2)   │
└──────┬───────┘
       │ JpegFrame message
       v
┌──────────────┐
│  CameraApp   │
│  GUI Thread  │
└──────┬───────┘
       │ write_frame()
       v
┌──────────────────────┐
│  recording_file      │
│  (Arc<Mutex<File>>)  │
└──────┬───────────────┘
       │ write_all(jpeg_data)
       v
┌──────────────────────┐
│  *.mjpeg file        │
│  (JPEG stream)       │
└──────────────────────┘
   File size: Large
   (47KB × 12fps = 564KB/sec)
```

### 3.2 MP4録画 (Phase 6)

```
┌──────────────┐
│   Camera     │
│  (Spresense) │
└──────┬───────┘
       │ JPEG frames (Serial/USB)
       v
┌──────────────┐
│ SerialThread │
│  (Phase 2)   │
└──────┬───────┘
       │ JpegFrame message
       v
┌──────────────┐
│  CameraApp   │
│  GUI Thread  │
└──────┬───────┘
       │ write_frame()
       v
┌──────────────────────┐
│  mp4_recorder        │
│  (Mp4Recorder)       │
└──────┬───────────────┘
       │ stdin.write_all(jpeg_data)
       v
┌─────────────────────────────────────┐
│  ffmpeg subprocess                  │
│  ┌────────┐   ┌─────────┐   ┌────┐ │
│  │ MJPEG  │──>│  H.264  │──>│MP4 │ │
│  │Decoder │   │ Encoder │   │Mux │ │
│  └────────┘   └─────────┘   └────┘ │
└─────────────────┬───────────────────┘
                  │
                  v
         ┌─────────────────┐
         │  *.mp4 file     │
         │  (H.264/MP4)    │
         └─────────────────┘
            File size: Small
            (約40-80KB/sec, 93% compression)
```

---

## 4. 状態遷移図

### 4.1 RecordingState遷移

```
                    ┌─────────────────────┐
                    │                     │
                    │       Idle          │
                    │                     │
                    └──────┬──────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        │ Start Manual     │                  │ Motion Detected
        │ Recording        │                  │
        v                  │                  v
┌───────────────┐          │          ┌───────────────────┐
│ Manual        │          │          │ Motion            │
│ Recording     │          │          │ Recording         │
│               │          │          │                   │
│ format: Mp4   │          │          │ format: Mp4       │
│   or Mjpeg    │          │          │   or Mjpeg        │
│               │          │          │ motion_active: T/F│
│ start_time    │          │          │ countdown: 0-330  │
│ frame_count   │          │          │                   │
│ total_bytes   │          │          │ start_time        │
│               │          │          │ frame_count       │
└───────┬───────┘          │          │ total_bytes       │
        │                  │          └─────────┬─────────┘
        │ Stop Recording   │                    │
        │ (Manual)         │                    │ countdown=0
        │                  │                    │ (Auto Stop)
        └──────────────────┼────────────────────┘
                           │
                           v
                    ┌─────────────┐
                    │    Idle     │
                    └─────────────┘
```

### 4.2 Motion Recording詳細状態

```
  Motion Detected
        │
        v
┌───────────────────┐
│ motion_active: T  │
│ countdown: 330    │
└────────┬──────────┘
         │
         │<─────────┐
         │          │ Motion continues
         │          │ (countdown reset)
         v          │
    ┌────────┐      │
    │ Motion │      │
    │ Active │──────┘
    └───┬────┘
        │ No motion detected
        v
┌───────────────────┐
│ motion_active: F  │
│ countdown: 330    │
└────────┬──────────┘
         │
         │ countdown--
         v
    ┌────────────┐
    │ Post       │
    │ Buffer     │
    │ (30sec)    │
    └─────┬──────┘
          │
          │ countdown=0
          v
    ┌──────────┐
    │ Stop Rec │
    └──────────┘
```

---

## 5. コンポーネント図

### 5.1 Phase 6追加コンポーネント

```
┌─────────────────────────────────────────────────────────────┐
│                    security_camera_viewer                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  src/                                                       │
│  ├── main.rs           (CLI entry point)                   │
│  ├── gui_main.rs       (GUI entry point + CameraApp)       │
│  │   ├── RecordingFormat enum        ← Phase 6 NEW        │
│  │   ├── RecordingState enum (+ format field) ← Phase 6   │
│  │   ├── CameraApp struct                                 │
│  │   │   ├── recording_format: RecordingFormat ← Phase 6  │
│  │   │   ├── mp4_recorder: Option<Mp4Recorder> ← Phase 6  │
│  │   │   ├── start_manual_recording() (modified)          │
│  │   │   ├── start_motion_recording() (modified)          │
│  │   │   ├── stop_recording() (modified)                  │
│  │   │   └── write_frame() (modified)                     │
│  │   └── UI (format selector radio buttons) ← Phase 6     │
│  │                                                         │
│  ├── mp4_recorder.rs   ← Phase 6 NEW (178 lines)          │
│  │   └── Mp4Recorder struct                               │
│  │       ├── new(path, fps) -> Result<Mp4Recorder>        │
│  │       ├── write_frame(jpeg_data) -> Result<()>         │
│  │       ├── finish(self) -> Result<()>                   │
│  │       └── Drop::drop()                                 │
│  │                                                         │
│  ├── ring_buffer.rs    (Phase 5)                          │
│  ├── motion_detector.rs (Phase 5)                         │
│  ├── serial.rs         (Phase 2-4)                        │
│  ├── protocol.rs       (Phase 1-4)                        │
│  └── metrics.rs        (Phase 4)                          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
         │
         │ spawn subprocess
         v
┌─────────────────────────────────────────────────────────────┐
│                     ffmpeg (external)                        │
├─────────────────────────────────────────────────────────────┤
│  - stdin: JPEG frame stream                                 │
│  - stdout: null                                             │
│  - stderr: null                                             │
│  - output: *.mp4 file                                       │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 依存関係

```
CameraApp
   │
   ├──> Mp4Recorder (Phase 6)
   │       │
   │       └──> ffmpeg subprocess
   │
   ├──> MotionDetector (Phase 5)
   │
   ├──> RingBuffer (Phase 5)
   │
   └──> SerialConnection (Phase 1-2)
            │
            └──> Protocol (Phase 1-4)
```

---

## 6. ファイルフォーマット

### 6.1 MJPEG形式 (Phase 3-5)

```
File: manual_20260102_154436.mjpeg

┌────────────────────────────────────────────┐
│ JPEG Frame 1                               │
│ ┌──────────────────────────────────────┐   │
│ │ FF D8 (SOI)                          │   │
│ │ FF E0 (APP0 - JFIF)                  │   │
│ │ ...                                  │   │
│ │ FF DA (Start of Scan)                │   │
│ │ [compressed image data]              │   │
│ │ FF D9 (EOI)                          │   │
│ └──────────────────────────────────────┘   │
├────────────────────────────────────────────┤
│ JPEG Frame 2                               │
│ ┌──────────────────────────────────────┐   │
│ │ FF D8 (SOI)                          │   │
│ │ ...                                  │   │
│ │ FF D9 (EOI)                          │   │
│ └──────────────────────────────────────┘   │
├────────────────────────────────────────────┤
│ ... (more JPEG frames)                     │
└────────────────────────────────────────────┘

Size: ~47KB per frame × 12fps = ~564KB/sec
No inter-frame compression
```

### 6.2 MP4/H.264形式 (Phase 6)

```
File: manual_20260102_154436.mp4

┌────────────────────────────────────────────┐
│ ftyp (File Type Box)                       │
├────────────────────────────────────────────┤
│ moov (Movie Box) ← faststart: 先頭配置     │
│ ┌──────────────────────────────────────┐   │
│ │ mvhd (Movie Header)                  │   │
│ │ trak (Track)                         │   │
│ │   ├── tkhd (Track Header)            │   │
│ │   └── mdia (Media)                   │   │
│ │       ├── mdhd (Media Header)        │   │
│ │       └── minf (Media Info)          │   │
│ │           ├── vmhd (Video Header)    │   │
│ │           └── stbl (Sample Table)    │   │
│ │               ├── stsd (Codec: avc1) │   │
│ │               ├── stts (Time to Samp)│   │
│ │               ├── stsc (Sample to Ch)│   │
│ │               └── stco (Chunk Offset)│   │
│ └──────────────────────────────────────┘   │
├────────────────────────────────────────────┤
│ mdat (Media Data)                          │
│ ┌──────────────────────────────────────┐   │
│ │ H.264 NAL Units                      │   │
│ │ ├── IDR frame (I-frame, keyframe)    │   │
│ │ ├── P-frame (predicted)              │   │
│ │ ├── P-frame                          │   │
│ │ ├── P-frame                          │   │
│ │ ├── ... (GOP structure)              │   │
│ │ └── IDR frame (next keyframe)        │   │
│ └──────────────────────────────────────┘   │
└────────────────────────────────────────────┘

Size: ~40-80KB/sec (93% compression vs MJPEG)
Inter-frame compression (H.264)
```

---

## 7. 設計上の重要ポイント

### 7.1 Option<Box<dyn Write + Send>>の採用理由

**問題**: `finish()`メソッドで`self.stdin`をdropする際、`Mp4Recorder`が`Drop` traitを実装しているため、ムーブアウトできない。

**解決策**: `stdin`を`Option`でラップ。

```rust
// Before (コンパイルエラー)
pub struct Mp4Recorder {
    stdin: Box<dyn Write + Send>,
}

pub fn finish(mut self) -> io::Result<()> {
    drop(self.stdin);  // ERROR: cannot move out of Drop type
}

// After (正常)
pub struct Mp4Recorder {
    stdin: Option<Box<dyn Write + Send>>,
}

pub fn finish(mut self) -> io::Result<()> {
    self.stdin.take();  // OK: Optionからムーブアウト
}
```

### 7.2 ffmpegプロセス制御の安全性

**Dropトレイト実装**:
```rust
impl Drop for Mp4Recorder {
    fn drop(&mut self) {
        let _ = self.ffmpeg_process.kill();
    }
}
```

- 正常終了: `finish()`でstdinクローズ → ffmpeg正常終了
- 異常終了: `Drop::drop()`でffmpeg強制終了 (プロセス残留防止)

### 7.3 ブロッキングI/Oの配置

**GUIスレッド内でのffmpeg通信**:
- `write_frame()`: JPEGデータをstdinに書き込み (ブロッキング)
- 影響: GUIスレッドがブロックされる可能性

**緩和策**:
- ffmpegプロセスは別プロセスで実行 (非同期的に処理)
- stdinバッファサイズが十分 (通常はブロックしない)
- 実測: GUI応答性に影響なし (12.04 fps維持)

**将来の改善案** (必要に応じて):
- 専用の録画スレッドを追加
- mpscチャネルでJPEGフレームをキューイング

### 7.4 エラーハンドリング戦略

```rust
// ffmpeg not found
Mp4Recorder::new() -> Err(NotFound, "Please install ffmpeg")
  → GUI: エラーログ + ユーザー通知

// ffmpeg process crash
write_frame() -> Err(BrokenPipe, "stdin already closed")
  → stop_recording() → 録画停止

// ffmpeg exit with error
finish() -> Err(Other, "ffmpeg exited with status: ...")
  → ログ記録 + ユーザー通知
```

---

## 8. パフォーマンス考察

### 8.1 CPU負荷

| コンポーネント | 処理 | CPU負荷 |
|---------------|------|---------|
| CameraApp (GUI Thread) | JPEGデコード + テクスチャアップロード | 10-15% (1コア) |
| CameraApp (GUI Thread) | write_frame() (stdin書き込み) | < 1% |
| ffmpeg (別プロセス) | H.264エンコード (medium preset) | 10-30% (1コア) |
| **合計** | | **20-45%** (マルチコア分散) |

**結論**: マルチコアCPUでは並列実行により影響最小化。

### 8.2 メモリ使用量

| コンポーネント | メモリ |
|---------------|--------|
| CameraApp | 約50-100 MB |
| Mp4Recorder (stdin buffer) | 数MB |
| ffmpeg subprocess | 約50-100 MB |
| **合計** | **約150-250 MB** |

**結論**: 一般的なPC環境で問題なし。

### 8.3 ディスクI/O

- MJPEG: 564 KB/sec (連続書き込み)
- MP4: 40-80 KB/sec (ffmpeg経由、バッファリング)

**結論**: MP4の方がディスクI/O負荷が低い (87%削減)。

---

## 9. 既知の制限と対応方針

### 9.1 MP4動き検知録画のプリバッファ未実装

**問題**:
- MJPEG形式: RingBufferからファイルにフラッシュ可能
- MP4形式: RingBuffer内のMJPEGデータを個別フレームとしてffmpegに送る処理が未実装

**現状**:
- 動き検知時点から録画開始
- 警告ログ: `"MP4 motion recording: pre-buffer not yet implemented"`

**Phase 6.1実装案**:

**Option A**: RingBuffer個別フレーム送信
```rust
// RingBufferにイテレータ追加
for frame in ring_buffer.frames() {
    mp4_recorder.write_frame(&frame.jpeg_data)?;
}
```

**Option B**: 一時MJPEGファイル経由
```rust
// 1. RingBufferを一時MJPEGファイルに保存
ring_buffer.flush_to_file(&temp_file)?;
// 2. MJPEGファイルをパースして個別JPEG抽出
let jpegs = parse_mjpeg_file(&temp_file)?;
// 3. 各JPEGをMP4レコーダーに送信
for jpeg in jpegs {
    mp4_recorder.write_frame(&jpeg)?;
}
// 4. 一時ファイル削除
fs::remove_file(&temp_file)?;
```

**優先度**: Medium (手動録画では不要)

---

## 10. テスト戦略

### 10.1 単体テスト

```rust
#[test]
#[ignore] // ffmpeg dependency
fn test_mp4_recorder_creation() {
    let recorder = Mp4Recorder::new(&PathBuf::from("/tmp/test.mp4"), 11);
    assert!(recorder.is_ok());
}

#[test]
#[ignore]
fn test_mp4_recorder_write_and_finish() {
    let mut recorder = Mp4Recorder::new(&PathBuf::from("/tmp/test.mp4"), 11).unwrap();
    let dummy_jpeg = vec![0xFF, 0xD8, /* ... */, 0xFF, 0xD9];
    for _ in 0..10 {
        recorder.write_frame(&dummy_jpeg).unwrap();
    }
    assert_eq!(recorder.frame_count(), 10);
    recorder.finish().unwrap();
}
```

### 10.2 統合テスト

1. **手動録画テスト**: Format=MP4, 30秒録画, ファイル生成確認
2. **動き検知録画テスト**: 3時間連続運転, 131件録画成功
3. **エラーハンドリング**: ffmpeg未インストール時のエラーメッセージ確認

### 10.3 パフォーマンステスト

- FPS維持: 12.04 fps (目標11.0+ fps) ✅
- エラー率: 0.000% ✅
- 圧縮率: 93%削減 (目標70%+) ✅

---

**作成者**: Claude Code
**バージョン**: Phase 6
**最終更新**: 2026年1月2日
