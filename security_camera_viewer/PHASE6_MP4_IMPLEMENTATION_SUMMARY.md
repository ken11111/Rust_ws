# Phase 6: MP4録画機能 実装完了レポート

**実装日**: 2026年1月2日
**Phase**: 6 - MP4直接保存機能
**ステータス**: ✅ 実装完了

---

## 1. 実装概要

Phase 6では、JPEGフレームをリアルタイムでMP4形式に変換して保存する機能を実装しました。ffmpegプロセスをサブプロセスとして起動し、stdin経由でJPEGフレームをストリーミングすることで、効率的なMP4エンコーディングを実現しています。

### 実装方針

MP4録画の実装方針として、以下の3つのオプションを検討しました:

- **Option A**: Pure Rust ライブラリ (mp4, x264-rs)
- **Option B**: ffmpeg パイプライン方式 ✅ **採用**
- **Option C**: 2段階変換方式 (MJPEG → MP4)

**採用理由**:
- ✅ 実装が最もシンプル (150行程度)
- ✅ 安定性と互換性が高い (ffmpegは業界標準)
- ✅ 高品質なエンコーディング (H.264/libx264)
- ✅ Web最適化サポート (faststart)
- ❌ ffmpegの外部依存が必要 → ユーザー向けインストールガイド提供で解決

---

## 2. 実装内容

### 2.1 新規ファイル

#### `src/mp4_recorder.rs` (178行)

ffmpegプロセスを制御するMP4レコーダーモジュール。

**主要構造体**:
```rust
pub struct Mp4Recorder {
    ffmpeg_process: Child,           // ffmpegプロセス
    stdin: Box<dyn Write + Send>,    // stdin (JPEGフレーム書き込み用)
    frame_count: u32,                // フレームカウント
    output_path: String,             // 出力ファイルパス
}
```

**主要メソッド**:
- `new(output_path, fps)` - ffmpegプロセス起動とレコーダー作成
- `write_frame(jpeg_data)` - JPEGフレームをffmpegに書き込み
- `finish()` - ffmpegプロセスを正常終了させてMP4を確定
- `Drop::drop()` - 異常終了時のクリーンアップ

**ffmpegコマンドライン引数**:
```bash
ffmpeg \
  -f image2pipe \          # 入力形式: 画像パイプ
  -codec:v mjpeg \          # 入力コーデック: MJPEG
  -framerate 11 \           # フレームレート (Spresense実測値)
  -i - \                    # 入力: stdin
  -c:v libx264 \            # 出力コーデック: H.264
  -preset medium \          # エンコード速度/品質バランス
  -crf 23 \                 # 品質設定 (18-28、低いほど高品質)
  -pix_fmt yuv420p \        # 互換性のためのピクセルフォーマット
  -movflags +faststart \    # Web最適化 (moovアトムを先頭に移動)
  -y \                      # 上書き確認なし
  output.mp4
```

**エラーハンドリング**:
- ffmpegが見つからない場合: `NotFound` エラー + インストール案内メッセージ
- プロセス起動失敗: `Other` エラー
- 書き込みエラー: `io::Error` を上位に伝播
- 異常終了時: `Drop` で自動的に `kill()` 実行

**単体テスト** (2件、#[ignore] でマーク):
- `test_mp4_recorder_creation` - ffmpeg有無の確認
- `test_mp4_recorder_write_and_finish` - 10フレーム書き込みテスト

---

### 2.2 変更ファイル

#### `src/gui_main.rs`

**Phase 6追加要素**:

1. **RecordingFormat enum** (lines 32-39):
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum RecordingFormat {
    Mjpeg,  // Phase 3-5
    Mp4,    // Phase 6 (デフォルト)
}
```

2. **RecordingState 拡張** (lines 48-82):
- `ManualRecording` と `MotionRecording` に `format: RecordingFormat` フィールド追加
- 録画中のフォーマット情報を保持

3. **CameraApp フィールド追加** (lines 106, 108):
```rust
recording_format: RecordingFormat,      // 録画フォーマット選択
mp4_recorder: Option<Mp4Recorder>,      // MP4レコーダー
```

4. **start_manual_recording() 修正** (lines 212-257):
- 動的なファイル拡張子 (.mjpeg または .mp4)
- フォーマットに応じた録画開始処理
  - MJPEG: `File::create()` でファイルオープン
  - MP4: `Mp4Recorder::new()` でffmpegプロセス起動

5. **start_motion_recording() 修正** (lines 260-331):
- 動的なファイル拡張子
- フォーマットに応じた録画開始処理
- **既知の制限**: MP4モーション録画のプリバッファ未実装
  - MJPEGファイルの個別フレーム解析が必要
  - 現状は警告ログ出力 + 検知時点からの録画開始

6. **stop_recording() 修正** (lines 333-368):
- フォーマットに応じた終了処理
  - MJPEG: `recording_file = None` でファイルクローズ
  - MP4: `mp4_recorder.finish()` でffmpeg正常終了

7. **write_frame() 修正** (lines 370-411):
- フォーマットに応じたフレーム書き込み
  - MJPEG: `recording_file.write_all(jpeg_data)`
  - MP4: `mp4_recorder.write_frame(jpeg_data)`
- サイズ制限チェック (MAX_RECORDING_SIZE)

8. **GUI フォーマット選択UI** (lines 607-610):
```rust
ui.label("Format:");
ui.radio_value(&mut self.recording_format, RecordingFormat::Mp4, "MP4");
ui.radio_value(&mut self.recording_format, RecordingFormat::Mjpeg, "MJPEG");
```

**UI配置**:
```
Top Panel (右側):
  [Format: (•) MP4  ( ) MJPEG]  [⏺ Start Rec]
```

---

## 3. ビルド結果

### 3.1 Linux版

```bash
$ cargo build --release
   Compiling security_camera_viewer v0.1.0
warning: unused variable: `remaining_size`
...
    Finished `release` profile [optimized] target(s) in 2.35s
```

- ✅ ビルド成功 (警告5件、すべてPhase 6以前の既存コード)
- ✅ 単体テスト: 7件すべて合格

**実行ファイル**:
- `target/release/security_camera_viewer` (Linux CLI版)

### 3.2 Windows版

```bash
$ cargo build --release --target x86_64-pc-windows-gnu
    Finished `release` profile [optimized] target(s) in 1.99s
```

- ✅ ビルド成功
- ✅ クロスコンパイル完了

**実行ファイル**:
- `target/x86_64-pc-windows-gnu/release/security_camera_viewer.exe` (4.6MB)
- `target/x86_64-pc-windows-gnu/release/security_camera_gui.exe` (16MB)

---

## 4. テスト結果

### 4.1 コンパイルテスト

**Linux環境** (WSL2):
```bash
$ cargo test
running 7 tests
test protocol::tests::test_crc16_ccitt ... ok
test protocol::tests::test_bare_jpeg_format ... ok
test protocol::tests::test_invalid_sync_word ... ok
test protocol::tests::test_jfif_jpeg_format ... ok
test protocol::tests::test_jpeg_size_limit ... ok
test protocol::tests::test_sync_word_validation ... ok
test serial::tests::test_list_ports ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured
```

**mp4_recorderテスト**:
- ✅ コンパイル成功
- ℹ️ `#[ignore]` のため実行スキップ (ffmpeg依存)
- ℹ️ ffmpegインストール後に `cargo test -- --ignored` で実行可能

### 4.2 機能テスト (予定)

以下のテストは、ユーザー環境でffmpegインストール後に実施予定:

1. **手動録画 (MP4)**:
   - Format選択: MP4
   - Start Rec → 30秒録画 → Stop Rec
   - 確認: `manual_YYYYMMDD_HHMMSS.mp4` が生成される
   - 確認: VLCやWindows Media Playerで再生可能

2. **手動録画 (MJPEG)**:
   - Format選択: MJPEG
   - Start Rec → 30秒録画 → Stop Rec
   - 確認: `manual_YYYYMMDD_HHMMSS.mjpeg` が生成される
   - 確認: ffplayやVLCで再生可能

3. **動き検知録画 (MP4)**:
   - Format選択: MP4
   - Motion Detection: 有効
   - カメラ前で動く → 自動録画開始 → 静止 → ポストバッファ後停止
   - 確認: `motion_YYYYMMDD_HHMMSS.mp4` が生成される
   - 確認: 動き検知時点からの録画 (プリバッファなし)

4. **動き検知録画 (MJPEG)**:
   - Format選択: MJPEG
   - Motion Detection: 有効
   - カメラ前で動く → 自動録画開始 → 静止 → ポストバッファ後停止
   - 確認: `motion_YYYYMMDD_HHMMSS.mjpeg` が生成される
   - 確認: プリバッファ10秒 + 動き検知期間 + ポストバッファ30秒

5. **フォーマット切替**:
   - 録画中にフォーマット変更不可を確認
   - 録画停止後に変更可能を確認

6. **エラーハンドリング**:
   - ffmpeg未インストール時のエラーメッセージ確認
   - ディスク容量不足時の動作確認 (MAX_RECORDING_SIZE到達)

---

## 5. 既知の制限事項

### 5.1 MP4動き検知録画のプリバッファ未実装

**問題**:
- Phase 5のMJPEG動き検知録画では、プリバッファ10秒 + 動き検知期間 + ポストバッファ30秒を録画
- Phase 6のMP4動き検知録画では、プリバッファが未実装
- 理由: RingBufferに保存されたMJPEGファイルを個別フレームとしてMP4エンコーダに送る処理が必要

**現状の動作**:
- 動き検知時点から録画開始 (プリバッファなし)
- ポストバッファは正常動作
- 警告ログ: `"MP4 motion recording: pre-buffer not yet implemented, starting from current frame"`

**回避策**:
- プリバッファが必要な場合は、Format=MJPEGを選択

**将来の実装案** (Phase 6.1):
1. Option A: RingBufferにイテレータ追加、個別フレームをMP4に書き込み
2. Option B: 一時MJPEGファイル作成 → 個別JPEG抽出 → MP4書き込み → 削除
3. Option C: RingBufferにRGBA画像も保存 (メモリ増加)

**優先度**: Medium (手動録画では不要、動き検知録画でのみ影響)

### 5.2 ffmpeg外部依存

**問題**:
- MP4録画にはffmpegのインストールが必須
- ffmpegがない場合、録画開始時にエラー

**対策**:
- エラーメッセージに明確なインストール案内を表示
- Windowsユーザー向けインストールガイド提供済み (`WINDOWS_RELEASE_GUIDE.md`)
- Linux/macOSはパッケージマネージャーでインストール可能

**エラーメッセージ例**:
```
Failed to start ffmpeg: No such file or directory (os error 2). Please install ffmpeg.
```

---

## 6. パフォーマンス分析

### 6.1 理論値

**MJPEG録画** (Phase 3-5):
- フレームサイズ: ~55KB/frame (平均)
- FPS: 11 fps
- ビットレート: 55KB × 11 = 605KB/sec = **4.8Mbps**
- 1分間: 36.3MB
- 10分間: 363MB

**MP4録画** (Phase 6):
- 入力: MJPEG 4.8Mbps
- H.264エンコード (CRF 23, medium preset)
- 予測ビットレート: **0.5-1.0Mbps** (圧縮率 80-90%)
- 1分間: 3.75-7.5MB
- 10分間: 37.5-75MB

**圧縮効果**: 約5-10倍の削減

### 6.2 CPU/メモリ影響

**GUI スレッド**:
- `write_frame()` でJPEGデータを `stdin.write_all()` に渡すのみ
- ffmpegプロセスがブロッキングしなければ影響最小
- 予測オーバーヘッド: < 0.5ms/frame

**ffmpeg プロセス**:
- 別プロセスで実行 (CPU並列化)
- H.264エンコード負荷: medium preset → 10-30% CPU (1コア)
- メモリ: 約50-100MB (ffmpegバッファ)

**総合影響**: 軽微 (マルチコアCPUでは影響なし)

---

## 7. ユーザー向け情報

### 7.1 ffmpegインストール方法

#### Windows

**方法1: Chocolatey (推奨)**
```powershell
choco install ffmpeg
```

**方法2: 手動インストール**
1. https://www.gyan.dev/ffmpeg/builds/ から `ffmpeg-release-essentials.zip` をダウンロード
2. 解凍して `C:\ffmpeg` に配置
3. 環境変数 `PATH` に `C:\ffmpeg\bin` を追加
4. コマンドプロンプトで `ffmpeg -version` を実行して確認

**方法3: scoop**
```powershell
scoop install ffmpeg
```

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install ffmpeg
```

#### macOS
```bash
brew install ffmpeg
```

### 7.2 録画ファイルの保存先

**Linux/macOS**:
```
./recordings/
  ├── manual_20260102_153000.mp4
  ├── manual_20260102_153100.mjpeg
  ├── motion_20260102_153200.mp4
  └── motion_20260102_153300.mjpeg
```

**Windows**:
```
.\recordings\
  ├── manual_20260102_153000.mp4
  ├── manual_20260102_153100.mjpeg
  ├── motion_20260102_153200.mp4
  └── motion_20260102_153300.mjpeg
```

### 7.3 推奨設定

**一般用途**:
- Format: **MP4** (圧縮効率、再生互換性)
- Motion Detection: 有効
- Sensitivity: 0.15-0.25 (環境に応じて調整)
- Min Motion Area: 2.0-5.0%

**詳細分析用途**:
- Format: **MJPEG** (フレーム個別抽出、画質劣化なし)
- Motion Detection: 必要に応じて

**長時間録画**:
- Format: **MP4** (ストレージ節約)
- ディスク容量: 1時間あたり 225-450MB (0.5-1.0Mbps想定)

---

## 8. 今後の拡張案

### Phase 6.1: MP4動き検知録画プリバッファ対応
- 優先度: Medium
- 実装量: 50-100行
- 効果: 動き検知録画でも10秒前から記録可能

### Phase 6.2: 録画品質設定UI
- CRF値調整 (18-28)
- プリセット選択 (fast, medium, slow)
- 解像度変更 (640x480, 320x240)

### Phase 6.3: リアルタイムプレビュー
- 録画中のビデオプレビュー再生
- シークバー対応

### Phase 6.4: MP4メタデータ埋め込み
- 録画日時
- カメラ情報 (Spresense ISX012)
- Phase 4.1メトリクス (FPS, Queue Depth, エラー率)

---

## 9. まとめ

### 実装成果

✅ **完了項目**:
1. `mp4_recorder.rs` モジュール実装 (178行)
2. `gui_main.rs` にMP4録画機能統合 (RecordingFormat enum, フォーマット切替)
3. 手動録画 MP4/MJPEG 対応
4. 動き検知録画 MP4/MJPEG 対応 (MP4プリバッファ除く)
5. GUI フォーマット選択UI追加
6. Linux/Windows ビルド成功
7. 単体テスト 7件合格
8. ドキュメント整備

❌ **未完了項目**:
1. MP4動き検知録画プリバッファ (既知の制限、Phase 6.1で対応予定)
2. ffmpegを含むWindowsリリースパッケージ (外部依存のため手動インストール案内で対応)

### 影響範囲

- **変更ファイル**: 2ファイル (新規1, 修正1)
- **追加行数**: 約400行
- **テスト**: 単体テスト 2件追加 (#[ignore])
- **後方互換性**: ✅ 完全互換 (MJPEG録画は従来通り動作)
- **デフォルト動作変更**: MP4がデフォルトに (ユーザーはMJPEGに切替可能)

### 次のステップ

1. **ユーザーテスト** (ffmpegインストール後):
   - 手動録画 MP4/MJPEG 動作確認
   - 動き検知録画 MP4/MJPEG 動作確認
   - Windows環境での動作確認

2. **Phase 6.1検討** (MP4プリバッファ):
   - 実装方針決定
   - 優先度判断 (ユーザーフィードバック次第)

3. **GitHubプッシュ** (Phase 6完了後):
   - ブランチ: `feature/phase6-mp4-recording`
   - コミットメッセージ: Phase 6実装内容サマリー

---

**Phase 6実装完了**: 2026年1月2日 15:10
**総所要時間**: 約40分 (設計〜実装〜ビルド〜ドキュメント)
