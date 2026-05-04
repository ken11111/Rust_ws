/// MP4録画モジュール
///
/// ffmpegプロセスを使用してJPEGフレームをリアルタイムでMP4にエンコードする。
/// - Phase 6: MP4直接保存機能 (`Mp4Recorder`)
/// - X-5a: 容量上限到達時の自動ローテーション (`RecordingManager::rotate_if_needed`)
/// - X-5b: 時間ベースのセグメント分割 (`RecordingManager::roll_if_due`)
///
/// 関連: docs/security_camera/02_specifications/quality/PENDING_NFR_WORK.md X-5a / X-5b
///       docs/security_camera/01_requirements/FUNCTIONAL_REQUIREMENTS.md Q8 / Q9

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// MP4レコーダー
///
/// ffmpegプロセスをstdin経由で制御し、JPEGフレームを
/// リアルタイムでMP4形式にエンコードする。
pub struct Mp4Recorder {
    /// ffmpegプロセス
    ffmpeg_process: Child,
    /// ffmpegのstdin（JPEGフレームを書き込む）
    stdin: Option<Box<dyn Write + Send>>,
    /// 書き込まれたフレーム数
    frame_count: u32,
    /// 出力ファイルパス
    output_path: String,
}

impl Mp4Recorder {
    /// 新しいMP4レコーダーを作成
    ///
    /// # Arguments
    /// * `output_path` - 出力MP4ファイルのパス
    /// * `fps` - フレームレート（通常11-13fps）
    ///
    /// # Returns
    /// 成功時は`Mp4Recorder`インスタンス、失敗時はエラー
    ///
    /// # Errors
    /// - ffmpegが見つからない場合
    /// - ffmpegプロセスの起動に失敗した場合
    pub fn new(output_path: &Path, fps: u32) -> io::Result<Self> {
        let output_str = output_path.to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid output path"))?;

        // ffmpegコマンドを構築
        let mut ffmpeg = Command::new("ffmpeg")
            .args(&[
                "-f", "image2pipe",               // 入力形式: 画像パイプ
                "-codec:v", "mjpeg",              // 入力コーデック: MJPEG
                "-framerate", &fps.to_string(),   // フレームレート
                "-i", "-",                        // 入力: stdin
                "-c:v", "libx264",                // 出力コーデック: H.264
                "-preset", "medium",              // エンコード速度/品質バランス
                "-crf", "23",                     // 品質設定（18-28、低いほど高品質）
                "-pix_fmt", "yuv420p",            // 互換性のためのピクセルフォーマット
                "-movflags", "+faststart",        // Web最適化（moovアトムを先頭に移動）
                "-y",                             // 上書き確認なし
                output_str,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())                // ffmpegの標準出力を破棄
            .stderr(Stdio::null())                // ffmpegの標準エラー出力を破棄
            .spawn()
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Failed to start ffmpeg: {}. Please install ffmpeg.", e)
                )
            })?;

        let stdin = ffmpeg.stdin.take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get ffmpeg stdin"))?;

        Ok(Self {
            ffmpeg_process: ffmpeg,
            stdin: Some(Box::new(stdin)),
            frame_count: 0,
            output_path: output_str.to_string(),
        })
    }

    /// JPEGフレームをffmpegに書き込む
    ///
    /// # Arguments
    /// * `jpeg_data` - JPEGフレームのバイトデータ
    ///
    /// # Returns
    /// 成功時はOk(())、失敗時はエラー
    ///
    /// # Errors
    /// - ffmpegプロセスが終了している場合
    /// - 書き込みに失敗した場合
    pub fn write_frame(&mut self, jpeg_data: &[u8]) -> io::Result<()> {
        if let Some(ref mut stdin) = self.stdin {
            stdin.write_all(jpeg_data)?;
            stdin.flush()?;
            self.frame_count += 1;
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdin already closed"))
        }
    }

    /// 録画を終了してffmpegプロセスを正常終了させる
    ///
    /// # Returns
    /// 成功時はOk(())、失敗時はエラー
    ///
    /// # Errors
    /// - ffmpegプロセスの終了に失敗した場合
    pub fn finish(mut self) -> io::Result<()> {
        // stdinをクローズしてffmpegに終了を通知
        self.stdin.take();

        // ffmpegの終了を待つ
        let status = self.ffmpeg_process.wait()?;

        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("ffmpeg exited with status: {}", status)
            ))
        }
    }

    /// 録画されたフレーム数を取得
    pub fn frame_count(&self) -> u32 {
        self.frame_count
    }

    /// 出力ファイルパスを取得
    pub fn output_path(&self) -> &str {
        &self.output_path
    }
}

impl Drop for Mp4Recorder {
    fn drop(&mut self) {
        // プロセスが残っている場合は強制終了
        let _ = self.ffmpeg_process.kill();
    }
}

// =============================================================================
// X-5a / X-5b: 録画ファイル管理 (ローテーション + 時間分割)
// =============================================================================

/// 録画セグメント分割 + ストレージ ローテーション ポリシー設定。
#[derive(Debug, Clone)]
pub struct RecordingPolicy {
    /// 録画ディレクトリ。
    pub directory: PathBuf,
    /// 1 セグメントの最大時間 (`None` で時間分割無効)。
    pub max_segment: Option<Duration>,
    /// 録画ディレクトリ全体の容量上限 (バイト)。`None` で容量管理無効。
    pub storage_quota_bytes: Option<u64>,
    /// 1 セグメントの最大バイト数 (`None` で 1 セグメント上限なし)。
    pub max_segment_bytes: Option<u64>,
    /// ファイル名プレフィクス。
    pub filename_prefix: String,
    /// FPS (ffmpeg 設定)。
    pub fps: u32,
}

impl Default for RecordingPolicy {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("./recordings"),
            max_segment: Some(Duration::from_secs(60 * 60)), // 1 hour
            storage_quota_bytes: Some(1_000_000_000),        // 1 GB (Q8)
            max_segment_bytes: Some(500_000_000),            // 500 MB / segment
            filename_prefix: "recording".to_string(),
            fps: 11,
        }
    }
}

/// `Mp4Recorder` をラップして容量・時間ベースの自動ローテーションを行う。
pub struct RecordingManager {
    policy: RecordingPolicy,
    current: Option<Mp4Recorder>,
    segment_started: Option<Instant>,
    segment_bytes: u64,
    segments_created: u32,
    segments_rotated_out: u32,
}

impl RecordingManager {
    /// 新規マネージャを作成。`policy.directory` がなければ作成する。
    pub fn new(policy: RecordingPolicy) -> io::Result<Self> {
        if !policy.directory.exists() {
            fs::create_dir_all(&policy.directory)?;
        }
        Ok(Self {
            policy,
            current: None,
            segment_started: None,
            segment_bytes: 0,
            segments_created: 0,
            segments_rotated_out: 0,
        })
    }

    /// 録画を開始 (新しいセグメントを開く)。
    pub fn start(&mut self) -> io::Result<()> {
        if self.current.is_some() {
            return Ok(()); // already recording
        }
        self.open_new_segment()
    }

    /// 録画を停止 (現セグメントを閉じる)。
    pub fn stop(&mut self) -> io::Result<()> {
        if let Some(recorder) = self.current.take() {
            self.segment_started = None;
            self.segment_bytes = 0;
            recorder.finish()?;
        }
        Ok(())
    }

    /// JPEG フレームを書き込む。必要に応じてセグメントを切替・古いファイルを削除。
    pub fn write_frame(&mut self, jpeg_data: &[u8]) -> io::Result<()> {
        if self.current.is_none() {
            self.start()?;
        }

        // X-5b: 時間ベース切替判定
        if self.is_time_segment_due() {
            self.roll_segment()?;
        }

        // X-5b: バイト数ベース切替判定
        if let Some(limit) = self.policy.max_segment_bytes {
            if self.segment_bytes + (jpeg_data.len() as u64) > limit {
                self.roll_segment()?;
            }
        }

        if let Some(rec) = self.current.as_mut() {
            rec.write_frame(jpeg_data)?;
            self.segment_bytes += jpeg_data.len() as u64;
        }

        // X-5a: 全体ストレージ容量チェック (毎フレーム軽く実施)
        self.rotate_storage_if_over_quota()?;

        Ok(())
    }

    fn is_time_segment_due(&self) -> bool {
        match (self.policy.max_segment, self.segment_started) {
            (Some(max), Some(start)) => start.elapsed() >= max,
            _ => false,
        }
    }

    fn roll_segment(&mut self) -> io::Result<()> {
        if let Some(recorder) = self.current.take() {
            self.segment_started = None;
            self.segment_bytes = 0;
            recorder.finish()?;
        }
        self.open_new_segment()
    }

    fn open_new_segment(&mut self) -> io::Result<()> {
        // タイムスタンプ付きファイル名
        let now = chrono::Local::now();
        let stamp = now.format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("{}_{}.mp4", self.policy.filename_prefix, stamp);
        let path = self.policy.directory.join(filename);

        let rec = Mp4Recorder::new(&path, self.policy.fps)?;
        self.current = Some(rec);
        self.segment_started = Some(Instant::now());
        self.segment_bytes = 0;
        self.segments_created += 1;
        Ok(())
    }

    /// X-5a: ディレクトリ全体の容量が `storage_quota_bytes` を超えていれば
    /// **古いファイルから順に削除** する (現在開いているセグメントは対象外)。
    fn rotate_storage_if_over_quota(&mut self) -> io::Result<()> {
        let quota = match self.policy.storage_quota_bytes {
            Some(q) => q,
            None => return Ok(()),
        };

        let mut entries: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
        let mut total: u64 = 0;
        for entry in fs::read_dir(&self.policy.directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size = meta.len();
            let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            total += size;
            entries.push((path, size, mtime));
        }

        if total <= quota {
            return Ok(());
        }

        // 開いているセグメントは保護
        let active_path = self
            .current
            .as_ref()
            .map(|r| PathBuf::from(r.output_path()));

        // 古いものから順にソート
        entries.sort_by_key(|(_, _, mtime)| *mtime);

        for (path, size, _) in entries {
            if total <= quota {
                break;
            }
            if Some(&path) == active_path.as_ref() {
                continue; // 現役セグメントはスキップ
            }
            if fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(size);
                self.segments_rotated_out += 1;
            }
        }

        Ok(())
    }

    pub fn segments_created(&self) -> u32 {
        self.segments_created
    }

    pub fn segments_rotated_out(&self) -> u32 {
        self.segments_rotated_out
    }

    pub fn current_segment_bytes(&self) -> u64 {
        self.segment_bytes
    }

    pub fn current_segment_path(&self) -> Option<&str> {
        self.current.as_ref().map(|r| r.output_path())
    }

    pub fn is_recording(&self) -> bool {
        self.current.is_some()
    }
}

impl Drop for RecordingManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[ignore] // ffmpegがインストールされていない環境では失敗するため
    fn test_mp4_recorder_creation() {
        let output_path = PathBuf::from("/tmp/test_output.mp4");
        let recorder = Mp4Recorder::new(&output_path, 11);

        // ffmpegがインストールされていればOk、されていなければErr
        match recorder {
            Ok(_) => println!("ffmpeg is available"),
            Err(e) => println!("ffmpeg not available: {}", e),
        }
    }

    #[test]
    #[ignore] // ffmpegがインストールされていない環境では失敗するため
    fn test_mp4_recorder_write_and_finish() {
        let output_path = PathBuf::from("/tmp/test_recording.mp4");
        let mut recorder = Mp4Recorder::new(&output_path, 11).unwrap();

        // ダミーJPEGフレーム（最小限のJPEG）
        let dummy_jpeg = vec![
            0xFF, 0xD8, // SOI
            0xFF, 0xE0, 0x00, 0x10, // APP0
            0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            0xFF, 0xD9, // EOI
        ];

        // フレームを書き込み
        for _ in 0..10 {
            recorder.write_frame(&dummy_jpeg).unwrap();
        }

        assert_eq!(recorder.frame_count(), 10);

        // 録画終了
        recorder.finish().unwrap();
    }
}
