# Phase 3 録画機能 仕様書

**作成日**: 2026-01-01
**バージョン**: 1.0
**前提**: Phase 2パイプライン + Phase 4.1メトリクス完了

---

## 📋 概要

### 目的

セキュリティカメラシステムに録画機能を追加し、受信したMJPEGストリームを
ファイルに保存できるようにする。

### スコープ

**Phase 3.0 Step 3の実装範囲:**
- ✅ GUI録画開始/停止ボタン
- ✅ MJPEGストリーム形式での保存
- ✅ タイムスタンプ付きファイル名
- ✅ 基本的な容量管理

**Phase 3.0後の拡張候補:**
- 個別JPEGファイル保存
- 録画分割機能 (ファイルサイズ or 時間)
- 自動削除機能 (古いファイル)
- 再生機能 (built-in player)

---

## 🎯 機能要件

### FR1: 録画開始/停止

**優先度**: 必須

**説明:**
GUIに「録画開始」「録画停止」ボタンを追加し、ユーザーが任意のタイミングで
録画を開始・停止できるようにする。

**詳細仕様:**

| 項目 | 仕様 |
|------|------|
| ボタン配置 | ウィンドウ下部のコントロールパネル |
| 初期状態 | 停止状態 (録画開始ボタンが有効) |
| 録画開始時 | ファイル作成、フレーム書き込み開始 |
| 録画停止時 | ファイルクローズ、停止状態に戻る |
| ボタン表示 | 🔴 録画開始 / ⏹ 録画停止 |

**ユーザーストーリー:**
```
As a ユーザー
I want 録画ボタンをクリックする
So that 必要な時だけ録画できる
```

**受け入れ基準:**
- [x] 録画開始ボタンをクリック → ファイルが作成される
- [x] 録画停止ボタンをクリック → ファイルがクローズされる
- [x] 録画中にボタン状態が変わる (開始→停止)
- [x] 録画中は録画開始ボタンが無効化される

---

### FR2: ファイル形式 - MJPEGストリーム

**優先度**: 必須

**説明:**
受信したJPEGフレームを連結してMJPEGストリームファイルとして保存する。

**詳細仕様:**

| 項目 | 仕様 |
|------|------|
| ファイル拡張子 | `.mjpeg` |
| データ形式 | JPEG frames の単純連結 |
| ヘッダー | 不要 (bare JPEG concatenation) |
| フレーム区切り | なし (JPEG SOI/EOIマーカーで自動判別) |

**ファイル構造:**
```
[MJPEG Stream File]
├─ JPEG Frame 1 (SOI 0xFF 0xD8 ... EOI 0xFF 0xD9)
├─ JPEG Frame 2 (SOI 0xFF 0xD8 ... EOI 0xFF 0xD9)
├─ JPEG Frame 3 (SOI 0xFF 0xD8 ... EOI 0xFF 0xD9)
...
└─ JPEG Frame N (SOI 0xFF 0xD8 ... EOI 0xFF 0xD9)
```

**再生互換性:**
- ✅ `ffplay <filename>.mjpeg`
- ✅ `vlc <filename>.mjpeg`
- ✅ `ffmpeg -i <filename>.mjpeg -vcodec copy output.mp4`

**受け入れ基準:**
- [x] 録画したファイルが `ffplay` で再生できる
- [x] 録画したファイルが `vlc` で再生できる
- [x] フレームドロップなく全フレームが保存される
- [x] ファイルサイズが期待通り (JPEG合計 + 若干のオーバーヘッド)

---

### FR3: タイムスタンプ付きファイル名

**優先度**: 必須

**説明:**
録画開始時刻をファイル名に含め、ファイルを識別しやすくする。

**詳細仕様:**

**ファイル名形式:**
```
recording_YYYYMMDD_HHMMSS.mjpeg
```

**例:**
- `recording_20260101_143052.mjpeg` (2026年1月1日 14時30分52秒)
- `recording_20260101_151223.mjpeg` (2026年1月1日 15時12分23秒)

**タイムゾーン:**
- システムローカル時刻を使用
- UTC変換は不要 (ローカル運用想定)

**実装:**
```rust
use chrono::Local;

fn generate_filename() -> String {
    format!("recording_{}.mjpeg", Local::now().format("%Y%m%d_%H%M%S"))
}
```

**受け入れ基準:**
- [x] ファイル名に日時が含まれる
- [x] ファイル名の日時が録画開始時刻と一致 (±1秒)
- [x] ファイル名に無効な文字が含まれない
- [x] 同一秒に複数録画を開始しても重複しない (後述)

---

### FR4: 録画先ディレクトリ

**優先度**: 必須

**説明:**
録画ファイルの保存先ディレクトリを指定できるようにする。

**詳細仕様:**

| 項目 | 仕様 |
|------|------|
| デフォルトディレクトリ | `./recordings/` (実行ディレクトリ配下) |
| ディレクトリ作成 | 自動作成 (存在しない場合) |
| 権限エラー | エラーダイアログ表示 |

**設定方法 (Phase 3.0):**
- ハードコード (将来的には設定ファイルまたはGUI設定)

**パス解決:**
```rust
use std::path::PathBuf;
use std::fs;

fn ensure_recording_dir() -> io::Result<PathBuf> {
    let dir = PathBuf::from("./recordings");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
```

**受け入れ基準:**
- [x] `./recordings/` ディレクトリが自動作成される
- [x] ディレクトリ作成失敗時にエラーメッセージが表示される
- [x] 録画ファイルが指定ディレクトリに保存される

---

### FR5: 容量管理

**優先度**: 必須 (Phase 3.0では簡易版)

**説明:**
ディスク容量を浪費しないよう、基本的な容量制限を設ける。

**Phase 3.0の仕様 (簡易版):**

| 項目 | 仕様 |
|------|------|
| 最大ファイルサイズ | 1 GB (ハードコード) |
| 超過時の動作 | 録画自動停止、警告メッセージ表示 |
| ディスク空き容量チェック | なし (Phase 3.1以降) |

**サイズチェック実装:**
```rust
const MAX_FILE_SIZE: u64 = 1_000_000_000;  // 1 GB

if file.metadata()?.len() >= MAX_FILE_SIZE {
    // 録画停止
    warn!("Recording stopped: File size limit reached (1 GB)");
    stop_recording();
}
```

**Phase 3.1以降の拡張:**
- ディスク空き容量チェック (最低10GB確保)
- 自動ファイル分割 (1GB毎)
- 古いファイル自動削除

**受け入れ基準:**
- [x] 1GB超過時に録画が自動停止する
- [x] 超過時に警告メッセージが表示される
- [x] ファイルサイズが1GBを超えない

---

## 🏗️ アーキテクチャ設計

### 状態管理

**RecordingState enum:**
```rust
#[derive(Debug, Clone, PartialEq)]
enum RecordingState {
    Idle,
    Recording {
        file: Arc<Mutex<File>>,
        start_time: Instant,
        frame_count: u32,
        total_bytes: u64,
    },
}
```

**状態遷移:**
```
    [Idle] ──(録画開始)──> [Recording]
      ↑                         |
      └────(録画停止)────────────┘
```

### データフロー

```
[Capture Thread]
    ↓ (JPEG frame)
[録画状態チェック]
    ↓ (if Recording)
[File::write_all(jpeg_data)]
    ↓
[バイト数カウント]
    ↓ (if > 1GB)
[自動停止]
```

### UI設計

**コントロールパネルレイアウト:**
```
+------------------------------------------+
|  [🎥 カメラビュー]                        |
|                                          |
+------------------------------------------+
| 📊 FPS: 11.0 | 📷 Cam: 11.0 | 📦 Q: 1   |
+------------------------------------------+
| 🔴 録画開始  | ⏹ 録画停止 (無効)          |
| 📁 ./recordings/recording_20260101...   |
| 📊 0 frames, 0.0 MB, 00:00              |
+------------------------------------------+
```

**録画中のUI:**
```
+------------------------------------------+
| 📊 FPS: 11.0 | 📷 Cam: 11.0 | 📦 Q: 1   |
+------------------------------------------+
| 🔴 録画開始 (無効) | ⏹ 録画停止            |
| 📁 ./recordings/recording_20260101...   |
| 📊 1,234 frames, 67.8 MB, 01:52 🔴 REC  |
+------------------------------------------+
```

---

## 💻 実装詳細

### 新規ファイル

**なし** (既存ファイルの拡張のみ)

### 変更ファイル

#### 1. `src/gui_main.rs`

**追加フィールド:**
```rust
struct CameraApp {
    // 既存フィールド...

    // 録画関連
    recording_state: RecordingState,
    recording_dir: PathBuf,
}
```

**追加メソッド:**
```rust
impl CameraApp {
    fn start_recording(&mut self) -> Result<(), io::Error> {
        // ファイル名生成
        let filename = format!("recording_{}.mjpeg",
                              chrono::Local::now().format("%Y%m%d_%H%M%S"));
        let filepath = self.recording_dir.join(filename);

        // ファイル作成
        let file = File::create(&filepath)?;

        // 状態更新
        self.recording_state = RecordingState::Recording {
            file: Arc::new(Mutex::new(file)),
            start_time: Instant::now(),
            frame_count: 0,
            total_bytes: 0,
        };

        info!("Recording started: {:?}", filepath);
        Ok(())
    }

    fn stop_recording(&mut self) {
        if let RecordingState::Recording { frame_count, total_bytes, .. } = &self.recording_state {
            info!("Recording stopped: {} frames, {} bytes", frame_count, total_bytes);
        }
        self.recording_state = RecordingState::Idle;
    }

    fn write_frame(&mut self, jpeg_data: &[u8]) -> Result<(), io::Error> {
        if let RecordingState::Recording { file, frame_count, total_bytes, .. } = &mut self.recording_state {
            let mut f = file.lock().unwrap();
            f.write_all(jpeg_data)?;
            *frame_count += 1;
            *total_bytes += jpeg_data.len() as u64;

            // サイズチェック
            if *total_bytes >= MAX_FILE_SIZE {
                warn!("File size limit reached, stopping recording");
                drop(f);  // unlock before calling stop_recording
                self.stop_recording();
            }
        }
        Ok(())
    }
}
```

**UIコード追加:**
```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // 既存のUI...

    // 録画コントロールパネル
    egui::TopBottomPanel::bottom("recording_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            // 録画開始ボタン
            if ui.add_enabled(
                matches!(self.recording_state, RecordingState::Idle),
                egui::Button::new("🔴 録画開始")
            ).clicked() {
                if let Err(e) = self.start_recording() {
                    error!("Failed to start recording: {}", e);
                }
            }

            // 録画停止ボタン
            if ui.add_enabled(
                !matches!(self.recording_state, RecordingState::Idle),
                egui::Button::new("⏹ 録画停止")
            ).clicked() {
                self.stop_recording();
            }

            // 録画情報表示
            if let RecordingState::Recording { start_time, frame_count, total_bytes, .. } = &self.recording_state {
                let duration = start_time.elapsed();
                ui.label(format!("🔴 REC | {} frames | {:.1} MB | {:02}:{:02}",
                                frame_count,
                                *total_bytes as f64 / 1_048_576.0,
                                duration.as_secs() / 60,
                                duration.as_secs() % 60));
            } else {
                ui.label("待機中");
            }
        });
    });
}
```

#### 2. Capture Thread修正

**フレーム受信時に録画:**
```rust
// gui_main.rs の capture_thread 内
Packet::Mjpeg(mjpeg_packet) => {
    // 既存の処理...

    // 録画中なら書き込み
    if let Err(e) = app.write_frame(&mjpeg_packet.jpeg_data) {
        error!("Failed to write frame: {}", e);
    }
}
```

---

## 🧪 テスト計画

### Unit Tests

**テスト項目:**
1. `generate_filename()` - ファイル名形式の確認
2. `start_recording()` - ファイル作成の確認
3. `stop_recording()` - ファイルクローズの確認
4. `write_frame()` - データ書き込みの確認
5. サイズ制限チェック - 1GB超過時の自動停止

**実装例:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_filename() {
        let filename = generate_filename();
        assert!(filename.starts_with("recording_"));
        assert!(filename.ends_with(".mjpeg"));
    }

    #[test]
    fn test_recording_lifecycle() {
        let mut app = CameraApp::new();

        // 初期状態
        assert!(matches!(app.recording_state, RecordingState::Idle));

        // 録画開始
        app.start_recording().unwrap();
        assert!(!matches!(app.recording_state, RecordingState::Idle));

        // 録画停止
        app.stop_recording();
        assert!(matches!(app.recording_state, RecordingState::Idle));
    }
}
```

### Integration Tests

**テストシナリオ1: 短時間録画**
1. アプリ起動
2. 録画開始ボタンをクリック
3. 10秒待機
4. 録画停止ボタンをクリック
5. ファイルが `./recordings/` に作成されていることを確認
6. `ffplay` でファイルを再生し、正常に再生できることを確認

**期待結果:**
- ファイルサイズ: 約5-10 MB (11 fps × 10秒 × 50 KB/frame)
- フレーム数: 約110フレーム
- 再生可能: ✅

**テストシナリオ2: 長時間録画 (30分)**
1. 録画開始
2. 30分待機
3. 録画停止
4. ファイルサイズ確認
5. ランダムシークで再生確認

**期待結果:**
- ファイルサイズ: 約900 MB - 1 GB
- フレーム数: 約19,800フレーム (11 fps × 1800秒)
- 再生可能: ✅
- メモリリークなし: ✅

**テストシナリオ3: サイズ制限**
1. 録画開始
2. 1GB超過まで待機 (約18分)
3. 自動停止を確認
4. 警告メッセージ表示を確認

**期待結果:**
- ファイルサイズ: ≤ 1 GB
- 自動停止: ✅
- 警告表示: ✅

---

## 📊 パフォーマンス要件

### 録画オーバーヘッド

| 指標 | 目標値 | 測定方法 |
|------|--------|---------|
| FPS影響 | < 5% | 録画中/非録画中のFPS比較 |
| CPU使用率増加 | < 10% | `top` コマンドで測定 |
| メモリ使用量増加 | < 50 MB | `ps` コマンドで測定 |
| ディスクI/O遅延 | < 10 ms/frame | write()時間測定 |

### スケーラビリティ

**1時間録画:**
- ファイルサイズ: 約2 GB (11 fps × 50 KB/frame)
- フレーム数: 約39,600フレーム
- 期待動作: ファイル分割 (Phase 3.1で対応)

**24時間録画 (Phase 3.0 Step 4):**
- ファイルサイズ: 約48 GB
- フレーム数: 約950,400フレーム
- 期待動作: 要ファイル分割機能 (Phase 3.1で対応)

---

## ✅ 完了条件

### Phase 3.0 Step 3完了基準

- [ ] 録画開始/停止ボタンがGUIに実装されている
- [ ] 録画開始時にMJPEGファイルが作成される
- [ ] 録画停止時にファイルが正しくクローズされる
- [ ] ファイル名にタイムスタンプが含まれる
- [ ] `ffplay` でファイルが再生できる
- [ ] `vlc` でファイルが再生できる
- [ ] 1GB超過時に自動停止する
- [ ] 録画中のFPS低下が5%未満
- [ ] メモリリークがない (30分録画テスト)

### オプション条件

- [ ] 録画中のUI表示が充実している (フレーム数、サイズ、時間)
- [ ] エラーハンドリングが適切 (ディスク満杯、権限エラー)
- [ ] 録画ファイルのリスト表示機能
- [ ] 録画履歴の管理

---

## 🔄 Phase 3.1以降の拡張

### 自動ファイル分割

**仕様:**
- 1GB到達時に新しいファイルを作成
- ファイル名に連番を付与: `recording_20260101_143052_001.mjpeg`
- 録画は継続 (ユーザーは停止ボタンを押すまで)

### 個別JPEGファイル保存

**仕様:**
- MJPEGストリームと並行して個別JPEGも保存
- ディレクトリ: `./recordings/recording_20260101_143052/`
- ファイル名: `frame_000001.jpg`, `frame_000002.jpg`, ...

### 再生機能

**仕様:**
- GUI内でMJPEG再生
- 一時停止、シーク、コマ送り
- 再生速度調整 (0.5x, 1x, 2x)

### 自動削除機能

**仕様:**
- 古いファイルを自動削除 (7日以上経過)
- ディスク空き容量が10GB未満で警告
- 空き容量が5GB未満で古いファイルを自動削除

---

## 📚 参考資料

### MJPEG形式

- [RFC 2046 - Multipurpose Internet Mail Extensions (MIME)](https://tools.ietf.org/html/rfc2046)
- [MJPEG on Wikipedia](https://en.wikipedia.org/wiki/Motion_JPEG)

### Rustライブラリ

- `std::fs::File` - ファイルI/O
- `chrono` - 日時処理
- `log` - ロギング

### 再生ツール

- `ffplay` - FFmpegに含まれる軽量プレイヤー
- `vlc` - VLC media player
- `ffmpeg` - 変換ツール (MJPEG → MP4など)

---

**作成者**: Claude Code (Sonnet 4.5)
**レビュー**: 実装前にユーザー確認
**承認**: Phase 3.0 Step 3実装開始前
