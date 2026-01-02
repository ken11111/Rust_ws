/// MP4録画モジュール
///
/// ffmpegプロセスを使用してJPEGフレームをリアルタイムでMP4にエンコードする。
/// Phase 6: MP4直接保存機能

use std::io::{self, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

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
