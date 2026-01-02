/// 動き検知モジュール
///
/// フレーム間差分法により、映像内の動きを検出する。
/// グレースケール変換後、前フレームとの差分を計算し、
/// 閾値を超えたピクセル数で動きを判定する。

use image::{GrayImage, Luma, RgbaImage};

/// 動き検知設定
#[derive(Debug, Clone)]
pub struct MotionDetectionConfig {
    /// 動き検知ON/OFF
    pub enabled: bool,
    /// 感度 (0.0-1.0, デフォルト0.5)
    /// 値が小さいほど感度が高い（小さな動きも検知）
    pub sensitivity: f32,
    /// 最小動き領域 (%, デフォルト1.0%)
    /// 画面全体に対する動きの最小割合
    pub min_motion_area: f32,
    /// プリ録画秒数（10秒）
    pub pre_record_seconds: u32,
    /// ポスト録画秒数（30秒）
    pub post_record_seconds: u32,
}

impl Default for MotionDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sensitivity: 0.5,
            min_motion_area: 1.0,
            pre_record_seconds: 10,
            post_record_seconds: 30,
        }
    }
}

/// フレーム差分による動き検知器
pub struct MotionDetector {
    /// 前フレーム（グレースケール）
    previous_frame: Option<GrayImage>,
    /// 設定
    config: MotionDetectionConfig,
    /// 統計: 総フレーム数
    total_frames: u64,
    /// 統計: 動き検知回数
    motion_detected_count: u64,
}

impl MotionDetector {
    /// 新しい動き検知器を作成
    pub fn new(config: MotionDetectionConfig) -> Self {
        Self {
            previous_frame: None,
            config,
            total_frames: 0,
            motion_detected_count: 0,
        }
    }

    /// デフォルト設定で作成
    pub fn default() -> Self {
        Self::new(MotionDetectionConfig::default())
    }

    /// 動き検知を実行
    ///
    /// # Arguments
    /// * `current_frame` - 現在のフレーム（RGBA）
    ///
    /// # Returns
    /// 動きが検知された場合true
    pub fn detect(&mut self, current_frame: &RgbaImage) -> bool {
        self.total_frames += 1;

        if !self.config.enabled {
            return false;
        }

        // 1. グレースケール変換
        let gray = Self::rgba_to_gray(current_frame);

        if let Some(prev) = &self.previous_frame {
            // 2. フレーム差分計算
            let diff = Self::compute_difference(prev, &gray);

            // 3. 閾値処理
            let threshold = self.compute_threshold();
            let changed_pixels = Self::count_changed_pixels(&diff, threshold);

            // 4. 動き判定
            let total_pixels = gray.width() * gray.height();
            let motion_ratio = (changed_pixels as f32) / (total_pixels as f32) * 100.0;

            self.previous_frame = Some(gray);

            let motion_detected = motion_ratio >= self.config.min_motion_area;

            if motion_detected {
                self.motion_detected_count += 1;
            }

            motion_detected
        } else {
            // 初回フレーム（比較対象なし）
            self.previous_frame = Some(gray);
            false
        }
    }

    /// 設定を更新
    pub fn update_config(&mut self, config: MotionDetectionConfig) {
        self.config = config;
    }

    /// 現在の設定を取得
    pub fn config(&self) -> &MotionDetectionConfig {
        &self.config
    }

    /// 統計情報を取得
    pub fn stats(&self) -> MotionDetectorStats {
        MotionDetectorStats {
            total_frames: self.total_frames,
            motion_detected_count: self.motion_detected_count,
            detection_rate: if self.total_frames > 0 {
                (self.motion_detected_count as f32) / (self.total_frames as f32) * 100.0
            } else {
                0.0
            },
        }
    }

    /// 統計をリセット
    pub fn reset_stats(&mut self) {
        self.total_frames = 0;
        self.motion_detected_count = 0;
    }

    /// 前フレームをクリア（状態リセット）
    pub fn reset(&mut self) {
        self.previous_frame = None;
        self.reset_stats();
    }

    /// 閾値を計算
    ///
    /// sensitivity (0.0-1.0) を 8bit閾値 (0-255) に変換
    /// sensitivity=0.0 → threshold=5 (超高感度)
    /// sensitivity=0.5 → threshold=30 (中感度)
    /// sensitivity=1.0 → threshold=100 (低感度)
    fn compute_threshold(&self) -> u8 {
        let min_threshold = 5;
        let max_threshold = 100;
        let range = max_threshold - min_threshold;

        (min_threshold + (self.config.sensitivity * range as f32) as u8).min(max_threshold)
    }

    /// RGBA → グレースケール変換
    ///
    /// 輝度計算: Y = 0.299*R + 0.587*G + 0.114*B (ITU-R BT.601)
    fn rgba_to_gray(rgba: &RgbaImage) -> GrayImage {
        GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
            let pixel = rgba.get_pixel(x, y);
            let gray = (0.299 * pixel[0] as f32 +
                       0.587 * pixel[1] as f32 +
                       0.114 * pixel[2] as f32) as u8;
            Luma([gray])
        })
    }

    /// フレーム差分計算
    ///
    /// 各ピクセルの輝度差の絶対値を計算
    fn compute_difference(prev: &GrayImage, current: &GrayImage) -> GrayImage {
        GrayImage::from_fn(prev.width(), prev.height(), |x, y| {
            let prev_val = prev.get_pixel(x, y)[0] as i16;
            let curr_val = current.get_pixel(x, y)[0] as i16;
            let diff = (prev_val - curr_val).abs() as u8;
            Luma([diff])
        })
    }

    /// 閾値を超えたピクセル数をカウント
    fn count_changed_pixels(diff: &GrayImage, threshold: u8) -> usize {
        diff.pixels()
            .filter(|p| p[0] > threshold)
            .count()
    }
}

/// 動き検知統計情報
#[derive(Debug, Clone)]
pub struct MotionDetectorStats {
    /// 総フレーム数
    pub total_frames: u64,
    /// 動き検知回数
    pub motion_detected_count: u64,
    /// 検知率（%）
    pub detection_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn create_solid_color_image(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
        RgbaImage::from_fn(width, height, |_, _| color)
    }

    fn create_half_split_image(width: u32, height: u32, left_color: Rgba<u8>, right_color: Rgba<u8>) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, _| {
            if x < width / 2 {
                left_color
            } else {
                right_color
            }
        })
    }

    #[test]
    fn test_rgba_to_gray() {
        let rgba = create_solid_color_image(10, 10, Rgba([128, 128, 128, 255]));
        let gray = MotionDetector::rgba_to_gray(&rgba);

        // 128の輝度は約128になるはず
        assert_eq!(gray.width(), 10);
        assert_eq!(gray.height(), 10);
        assert_eq!(gray.get_pixel(0, 0)[0], 128);
    }

    #[test]
    fn test_motion_detector_no_motion() {
        let mut detector = MotionDetector::new(MotionDetectionConfig {
            enabled: true,
            sensitivity: 0.5,
            min_motion_area: 1.0,
            pre_record_seconds: 10,
            post_record_seconds: 30,
        });

        let frame1 = create_solid_color_image(100, 100, Rgba([100, 100, 100, 255]));
        let frame2 = create_solid_color_image(100, 100, Rgba([100, 100, 100, 255]));

        // 初回フレーム（常にfalse）
        assert_eq!(detector.detect(&frame1), false);

        // 同じフレーム（動きなし）
        assert_eq!(detector.detect(&frame2), false);
    }

    #[test]
    fn test_motion_detector_with_motion() {
        let mut detector = MotionDetector::new(MotionDetectionConfig {
            enabled: true,
            sensitivity: 0.5,
            min_motion_area: 1.0,
            pre_record_seconds: 10,
            post_record_seconds: 30,
        });

        let frame1 = create_solid_color_image(100, 100, Rgba([50, 50, 50, 255]));
        let frame2 = create_solid_color_image(100, 100, Rgba([200, 200, 200, 255]));

        // 初回フレーム
        assert_eq!(detector.detect(&frame1), false);

        // 大きな変化（動き検知）
        assert_eq!(detector.detect(&frame2), true);
    }

    #[test]
    fn test_motion_detector_partial_motion() {
        let mut detector = MotionDetector::new(MotionDetectionConfig {
            enabled: true,
            sensitivity: 0.3,
            min_motion_area: 10.0, // 10%以上の動きが必要
            pre_record_seconds: 10,
            post_record_seconds: 30,
        });

        let frame1 = create_solid_color_image(100, 100, Rgba([100, 100, 100, 255]));

        // 左半分だけ変化（50%の動き）
        let frame2 = create_half_split_image(100, 100,
            Rgba([200, 200, 200, 255]),
            Rgba([100, 100, 100, 255])
        );

        // 初回フレーム
        detector.detect(&frame1);

        // 50%の領域が変化 → 検知される（10%以上）
        assert_eq!(detector.detect(&frame2), true);
    }

    #[test]
    fn test_motion_detector_disabled() {
        let mut detector = MotionDetector::new(MotionDetectionConfig {
            enabled: false, // 無効化
            sensitivity: 0.5,
            min_motion_area: 1.0,
            pre_record_seconds: 10,
            post_record_seconds: 30,
        });

        let frame1 = create_solid_color_image(100, 100, Rgba([50, 50, 50, 255]));
        let frame2 = create_solid_color_image(100, 100, Rgba([200, 200, 200, 255]));

        detector.detect(&frame1);

        // 大きな変化があっても、無効なので検知されない
        assert_eq!(detector.detect(&frame2), false);
    }

    #[test]
    fn test_stats() {
        let mut detector = MotionDetector::new(MotionDetectionConfig {
            enabled: true,
            sensitivity: 0.5,
            min_motion_area: 1.0,
            pre_record_seconds: 10,
            post_record_seconds: 30,
        });

        let frame_still = create_solid_color_image(100, 100, Rgba([100, 100, 100, 255]));
        let frame_motion = create_solid_color_image(100, 100, Rgba([200, 200, 200, 255]));

        // 初回
        detector.detect(&frame_still);

        // 動き検知
        detector.detect(&frame_motion);

        // 静止
        detector.detect(&frame_still);

        // 動き検知
        detector.detect(&frame_motion);

        let stats = detector.stats();

        assert_eq!(stats.total_frames, 4);
        assert_eq!(stats.motion_detected_count, 3);  // 初回以外の3フレームすべてで大きな変化
        assert_eq!(stats.detection_rate, 75.0); // 3/4 = 75%
    }

    #[test]
    fn test_reset() {
        let mut detector = MotionDetector::new(MotionDetectionConfig::default());

        let frame = create_solid_color_image(100, 100, Rgba([100, 100, 100, 255]));
        detector.detect(&frame);
        detector.detect(&frame);

        assert_eq!(detector.stats().total_frames, 2);

        detector.reset();

        assert_eq!(detector.stats().total_frames, 0);
        assert_eq!(detector.stats().motion_detected_count, 0);
        assert!(detector.previous_frame.is_none());
    }

    #[test]
    fn test_compute_threshold() {
        let mut detector = MotionDetector::new(MotionDetectionConfig {
            enabled: true,
            sensitivity: 0.0, // 超高感度
            min_motion_area: 1.0,
            pre_record_seconds: 10,
            post_record_seconds: 30,
        });

        assert_eq!(detector.compute_threshold(), 5);

        detector.config.sensitivity = 0.5; // 中感度
        let threshold = detector.compute_threshold();
        assert!(threshold >= 30 && threshold <= 55);

        detector.config.sensitivity = 1.0; // 低感度
        assert_eq!(detector.compute_threshold(), 100);
    }
}
