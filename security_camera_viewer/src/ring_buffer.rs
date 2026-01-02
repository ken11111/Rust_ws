/// リングバッファ（プリバッファ用）
///
/// 常に最新N秒分のJPEGフレームをメモリに保持し、
/// 動き検知時にファイルに書き込むことで「10秒前から録画」を実現する。

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Write};
use std::time::Instant;

/// JPEGフレーム
#[derive(Clone)]
pub struct JpegFrame {
    /// JPEG画像データ
    pub jpeg_data: Vec<u8>,
    /// 受信時刻
    pub timestamp: Instant,
}

/// リングバッファ
pub struct RingBuffer {
    /// フレームキュー（古い順）
    frames: VecDeque<JpegFrame>,
    /// 最大フレーム数
    capacity: usize,
    /// 現在のバッファ内総バイト数
    total_bytes: usize,
}

impl RingBuffer {
    /// 新しいリングバッファを作成
    ///
    /// # Arguments
    /// * `capacity` - 最大フレーム数（例: 110 = 10秒@11fps）
    pub fn new(capacity: usize) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
            total_bytes: 0,
        }
    }

    /// フレーム数から容量を計算
    ///
    /// # Arguments
    /// * `seconds` - プリバッファ秒数
    /// * `fps` - フレームレート
    pub fn from_seconds(seconds: u32, fps: u32) -> Self {
        let capacity = (seconds * fps) as usize;
        Self::new(capacity)
    }

    /// 新しいフレームを追加
    ///
    /// 容量を超える場合、最も古いフレームを自動削除する。
    pub fn push(&mut self, frame: JpegFrame) {
        // 容量チェック
        if self.frames.len() >= self.capacity {
            if let Some(old_frame) = self.frames.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(old_frame.jpeg_data.len());
            }
        }

        // 新しいフレームを追加
        self.total_bytes += frame.jpeg_data.len();
        self.frames.push_back(frame);
    }

    /// バッファ内の全フレームをファイルに書き込み
    ///
    /// # Arguments
    /// * `file` - 書き込み先ファイル
    ///
    /// # Returns
    /// 書き込まれたフレーム数と総バイト数
    pub fn flush_to_file(&self, file: &mut File) -> io::Result<(usize, usize)> {
        let frame_count = self.frames.len();
        let mut bytes_written = 0;

        for frame in &self.frames {
            file.write_all(&frame.jpeg_data)?;
            bytes_written += frame.jpeg_data.len();
        }

        file.flush()?;

        Ok((frame_count, bytes_written))
    }

    /// バッファクリア
    pub fn clear(&mut self) {
        self.frames.clear();
        self.total_bytes = 0;
    }

    /// 現在のフレーム数
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// バッファが空かどうか
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// 現在の総バイト数
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// 容量（最大フレーム数）
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// バッファの使用率（0.0-1.0）
    pub fn usage_ratio(&self) -> f32 {
        if self.capacity == 0 {
            0.0
        } else {
            self.frames.len() as f32 / self.capacity as f32
        }
    }

    /// 最も古いフレームの経過時間（秒）
    pub fn oldest_frame_age_secs(&self) -> Option<f32> {
        self.frames.front().map(|frame| {
            frame.timestamp.elapsed().as_secs_f32()
        })
    }

    /// 最も新しいフレームの経過時間（秒）
    pub fn newest_frame_age_secs(&self) -> Option<f32> {
        self.frames.back().map(|frame| {
            frame.timestamp.elapsed().as_secs_f32()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer = RingBuffer::new(3);

        // 空のバッファ
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.total_bytes(), 0);

        // フレーム追加
        buffer.push(JpegFrame {
            jpeg_data: vec![1, 2, 3],
            timestamp: Instant::now(),
        });

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.total_bytes(), 3);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer = RingBuffer::new(2);

        // 2フレーム追加（容量いっぱい）
        buffer.push(JpegFrame {
            jpeg_data: vec![1, 2, 3],
            timestamp: Instant::now(),
        });
        buffer.push(JpegFrame {
            jpeg_data: vec![4, 5, 6, 7],
            timestamp: Instant::now(),
        });

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.total_bytes(), 7); // 3 + 4

        // 3フレーム目追加（古いフレームが削除される）
        buffer.push(JpegFrame {
            jpeg_data: vec![8, 9],
            timestamp: Instant::now(),
        });

        assert_eq!(buffer.len(), 2); // 容量は2のまま
        assert_eq!(buffer.total_bytes(), 6); // 4 + 2（最初のフレーム3bytesは削除）
    }

    #[test]
    fn test_flush_to_file() {
        let mut buffer = RingBuffer::new(3);

        buffer.push(JpegFrame {
            jpeg_data: vec![0xFF, 0xD8, 0xFF, 0xD9], // 最小JPEG
            timestamp: Instant::now(),
        });
        buffer.push(JpegFrame {
            jpeg_data: vec![0xFF, 0xD8, 0x00, 0xFF, 0xD9],
            timestamp: Instant::now(),
        });

        // テンポラリファイルに書き込み
        let mut file = tempfile::NamedTempFile::new().unwrap();

        let (frame_count, bytes_written) = buffer.flush_to_file(file.as_file_mut()).unwrap();

        assert_eq!(frame_count, 2);
        assert_eq!(bytes_written, 9); // 4 + 5
    }

    #[test]
    fn test_clear() {
        let mut buffer = RingBuffer::new(3);

        buffer.push(JpegFrame {
            jpeg_data: vec![1, 2, 3],
            timestamp: Instant::now(),
        });

        assert_eq!(buffer.len(), 1);

        buffer.clear();

        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.total_bytes(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_from_seconds() {
        // 10秒@11fps = 110フレーム
        let buffer = RingBuffer::from_seconds(10, 11);
        assert_eq!(buffer.capacity(), 110);

        // 5秒@30fps = 150フレーム
        let buffer = RingBuffer::from_seconds(5, 30);
        assert_eq!(buffer.capacity(), 150);
    }

    #[test]
    fn test_usage_ratio() {
        let mut buffer = RingBuffer::new(10);

        assert_eq!(buffer.usage_ratio(), 0.0);

        for i in 0..5 {
            buffer.push(JpegFrame {
                jpeg_data: vec![i],
                timestamp: Instant::now(),
            });
        }

        assert_eq!(buffer.usage_ratio(), 0.5); // 5/10

        for i in 0..5 {
            buffer.push(JpegFrame {
                jpeg_data: vec![i],
                timestamp: Instant::now(),
            });
        }

        assert_eq!(buffer.usage_ratio(), 1.0); // 10/10
    }
}
