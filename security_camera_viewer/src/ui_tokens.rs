//! Design System Tokens — Spresense Security Camera Operator Console
//!
//! 出典: `examples/Spresense Security Camera Design System.zip` の
//!      `colors_and_type.css` ダークテーマ既定値
//!
//! 参照: `docs/security_camera/02_specifications/quality/FUNCTIONAL_SPEC_AUDIT.md`
//! の派生 (X-5d OSD 重畳の補強)
//!
//! Hi-fi 忠実度: 配色 / タイポ / 余白 / 角丸を定数化
#![allow(dead_code)]

use eframe::egui::{self, Color32, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle, Visuals};
use std::collections::BTreeMap;

// =============================================================================
// Surfaces (背景階層)
// =============================================================================
pub const BG_0: Color32 = Color32::from_rgb(0x0a, 0x0c, 0x10); // 深い墨
pub const BG_1: Color32 = Color32::from_rgb(0x11, 0x14, 0x1a);
pub const BG_2: Color32 = Color32::from_rgb(0x18, 0x1c, 0x24);
pub const BG_3: Color32 = Color32::from_rgb(0x20, 0x25, 0x2f);
pub const BG_4: Color32 = Color32::from_rgb(0x2a, 0x31, 0x40);

// =============================================================================
// Foreground (前景・テキスト)
// =============================================================================
pub const FG_1: Color32 = Color32::from_rgb(0xe8, 0xec, 0xf4);
pub const FG_2: Color32 = Color32::from_rgb(0xb0, 0xb8, 0xc7);
pub const FG_3: Color32 = Color32::from_rgb(0x6c, 0x76, 0x89);
pub const FG_4: Color32 = Color32::from_rgb(0x42, 0x4b, 0x5c);

// =============================================================================
// Borders (罫線・分割)
// =============================================================================
pub const BORDER_1: Color32 = Color32::from_rgb(0x23, 0x29, 0x36);
pub const BORDER_2: Color32 = Color32::from_rgb(0x2f, 0x36, 0x45);

// =============================================================================
// Accent (warm amber — 主要アクション)
// =============================================================================
pub const ACCENT: Color32 = Color32::from_rgb(0xd9, 0x9a, 0x3d);
pub const ACCENT_HI: Color32 = Color32::from_rgb(0xe8, 0xb2, 0x5c);
pub const ACCENT_LO: Color32 = Color32::from_rgb(0xb0, 0x7d, 0x2c);

// =============================================================================
// Status (ステータスマシン: OFFLINE → LINK → LIVE / FAULT)
// =============================================================================
pub const STATUS_LIVE: Color32 = Color32::from_rgb(0xe8, 0x4a, 0x4a);
pub const STATUS_READY: Color32 = Color32::from_rgb(0x4a, 0xd2, 0x7a);
pub const STATUS_LINK: Color32 = ACCENT;
pub const STATUS_OFFLINE: Color32 = FG_3;
pub const STATUS_FAULT: Color32 = Color32::from_rgb(0xff, 0x5a, 0x5a);

// =============================================================================
// Spacing scale (4px ベース: 4, 8, 12, 16, 20, 24, 32, 40, 48, 56, 64)
// =============================================================================
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 20.0;
pub const SPACE_6: f32 = 24.0;
pub const SPACE_7: f32 = 32.0;
pub const SPACE_8: f32 = 40.0;

// =============================================================================
// Radius (角丸)
// =============================================================================
pub const RADIUS_INPUT: f32 = 2.0;
pub const RADIUS_CARD: f32 = 4.0;
pub const RADIUS_MODAL: f32 = 8.0;

// =============================================================================
// Type scale (px: 11, 12, 13, 14, 16, 18, 22, 28, 36, 48, 64)
// =============================================================================
pub const FONT_TINY: f32 = 11.0;
pub const FONT_CAPTION: f32 = 12.0;
pub const FONT_BODY_SM: f32 = 13.0;
pub const FONT_BODY: f32 = 14.0;
pub const FONT_LABEL: f32 = 16.0;
pub const FONT_SUBHEAD: f32 = 18.0;
pub const FONT_HEAD_SM: f32 = 22.0;
pub const FONT_HEAD: f32 = 28.0;

// =============================================================================
// Status / Connection state
// =============================================================================

/// 接続状態マシン — Design System §振る舞い に準拠
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    Offline,
    Link,
    Live,
    Fault,
}

impl ConnState {
    /// ステータスドット色
    pub fn dot_color(self) -> Color32 {
        match self {
            ConnState::Offline => STATUS_OFFLINE,
            ConnState::Link => STATUS_LINK,
            ConnState::Live => STATUS_LIVE,
            ConnState::Fault => STATUS_FAULT,
        }
    }

    /// ローカライズされた表示文字列 (`配信中 · LIVE` 等)
    pub fn label(self) -> &'static str {
        match self {
            ConnState::Offline => "切断 · OFFLINE",
            ConnState::Link => "接続中 · LINK",
            ConnState::Live => "配信中 · LIVE",
            ConnState::Fault => "障害 · FAULT",
        }
    }
}

// =============================================================================
// Visuals / Style 適用
// =============================================================================

/// Design System のダークテーマを `egui::Visuals` に焼き付ける。
///
/// `eframe::App::new()` または `cc.egui_ctx.set_visuals(...)` 経由で適用。
pub fn apply_visuals(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.window_fill = BG_1;
    visuals.panel_fill = BG_1;
    visuals.faint_bg_color = BG_2;
    visuals.extreme_bg_color = BG_0;
    visuals.code_bg_color = BG_3;

    visuals.override_text_color = Some(FG_1);
    visuals.hyperlink_color = ACCENT_HI;

    visuals.widgets.noninteractive.bg_fill = BG_2;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_1);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, FG_2);
    visuals.widgets.noninteractive.rounding = Rounding::same(RADIUS_CARD);

    visuals.widgets.inactive.bg_fill = BG_3;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_1);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, FG_2);
    visuals.widgets.inactive.rounding = Rounding::same(RADIUS_INPUT);

    visuals.widgets.hovered.bg_fill = BG_4;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_2);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, FG_1);
    visuals.widgets.hovered.rounding = Rounding::same(RADIUS_INPUT);

    visuals.widgets.active.bg_fill = ACCENT_LO;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, FG_1);
    visuals.widgets.active.rounding = Rounding::same(RADIUS_INPUT);

    visuals.selection.bg_fill = ACCENT_LO;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_HI);

    visuals.window_rounding = Rounding::same(RADIUS_CARD);
    visuals.menu_rounding = Rounding::same(RADIUS_CARD);

    ctx.set_visuals(visuals);

    // テキストスタイル (body サイズを 14px に統一)
    let mut style = (*ctx.style()).clone();
    let mut text_styles: BTreeMap<TextStyle, FontId> = BTreeMap::new();
    text_styles.insert(
        TextStyle::Heading,
        FontId::new(FONT_HEAD_SM, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Body,
        FontId::new(FONT_BODY, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Monospace,
        FontId::new(FONT_BODY_SM, FontFamily::Monospace),
    );
    text_styles.insert(
        TextStyle::Button,
        FontId::new(FONT_BODY, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Small,
        FontId::new(FONT_CAPTION, FontFamily::Proportional),
    );
    style.text_styles = text_styles;
    style.spacing.item_spacing = egui::vec2(SPACE_2, SPACE_2);
    style.spacing.button_padding = egui::vec2(SPACE_3, SPACE_2);
    style.spacing.window_margin = Margin::same(SPACE_4);
    ctx.set_style(style);
}

// =============================================================================
// HUD / OSD ヘルパ (X-5d: タイムスタンプ・フレーム#・REC オーバーレイ)
// =============================================================================

/// HUD を Viewport 領域に描画する。
///
/// `rect` は描画対象領域 (画像が貼られる egui::Rect)。
/// `frame_no` が `Some(_)` ならフレーム番号を左下に表示。
/// `recording_secs` が `Some(_)` なら REC ドット + 録画時間を右下に表示 (1Hz 点滅)。
/// 右上には常に現在時刻を `YYYY-MM-DD HH:MM:SS.sss` 形式で表示。
pub fn paint_hud(
    painter: &egui::Painter,
    rect: egui::Rect,
    state: ConnState,
    frame_no: Option<u64>,
    recording_secs: Option<u64>,
    timestamp: Option<&str>,
) {
    use egui::{Align2, FontId, Pos2, Rect};

    let pad = SPACE_3;
    let mono_med = FontId::new(FONT_BODY, FontFamily::Monospace);
    let mono_sm = FontId::new(FONT_CAPTION, FontFamily::Monospace);

    // 暗いテロップ背景を描く小ヘルパ
    let label = |painter: &egui::Painter, anchor: Align2, pos: Pos2, text: &str, font: FontId, fg: Color32| {
        let galley = painter.layout_no_wrap(text.to_string(), font.clone(), fg);
        let size = galley.size() + egui::vec2(pad, SPACE_1);
        let bg_rect = match anchor {
            Align2::LEFT_TOP => Rect::from_min_size(pos, size),
            Align2::RIGHT_TOP => Rect::from_min_size(pos - egui::vec2(size.x, 0.0), size),
            Align2::LEFT_BOTTOM => Rect::from_min_size(pos - egui::vec2(0.0, size.y), size),
            Align2::RIGHT_BOTTOM => Rect::from_min_size(pos - egui::vec2(size.x, size.y), size),
            _ => Rect::from_min_size(pos, size),
        };
        painter.rect_filled(
            bg_rect,
            Rounding::same(RADIUS_INPUT),
            Color32::from_rgba_premultiplied(0, 0, 0, 160),
        );
        painter.text(
            bg_rect.center(),
            Align2::CENTER_CENTER,
            text,
            font,
            fg,
        );
    };

    // 右上: タイムスタンプ
    if let Some(ts) = timestamp {
        label(
            painter,
            Align2::RIGHT_TOP,
            rect.right_top() - egui::vec2(pad, -pad),
            ts,
            mono_med.clone(),
            FG_1,
        );
    }

    // 左下: フレーム番号 F#000123 (LIVE 中のみ)
    if state == ConnState::Live {
        if let Some(fno) = frame_no {
            let txt = format!("F#{:06}", fno);
            label(
                painter,
                Align2::LEFT_BOTTOM,
                rect.left_bottom() + egui::vec2(pad, -pad),
                &txt,
                mono_sm.clone(),
                FG_2,
            );
        }
    }

    // 右下: REC + 録画時間 (録画中のみ、1Hz 点滅)
    if let Some(secs) = recording_secs {
        let blink_on = (secs % 2) == 0;
        let dot_color = if blink_on { STATUS_LIVE } else { STATUS_LIVE.gamma_multiply(0.35) };
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        let txt = format!("● 録画 · {:02}:{:02}:{:02}", h, m, s);

        // 描画位置 + 色を分けてドットを赤くする
        let galley = painter.layout_no_wrap(txt.clone(), mono_med.clone(), FG_1);
        let size = galley.size() + egui::vec2(pad, SPACE_1);
        let bg_rect = egui::Rect::from_min_size(
            rect.right_bottom() - egui::vec2(size.x + pad, size.y + pad),
            size,
        );
        painter.rect_filled(
            bg_rect,
            Rounding::same(RADIUS_INPUT),
            Color32::from_rgba_premultiplied(0, 0, 0, 160),
        );
        // ドット
        let dot_pos = bg_rect.left_center() + egui::vec2(SPACE_2, 0.0);
        painter.circle_filled(dot_pos, 5.0, dot_color);
        // 文字本体 (ドット部分は別描画なので空白を冠する)
        let text_pos = bg_rect.center() + egui::vec2(SPACE_2, 0.0);
        painter.text(
            text_pos,
            egui::Align2::CENTER_CENTER,
            &format!("録画 · {:02}:{:02}:{:02}", h, m, s),
            mono_med,
            FG_1,
        );
    }

    // ステータス枠 (LIVE/FAULT で色付き 1px)
    let stroke_color = match state {
        ConnState::Live => STATUS_LIVE,
        ConnState::Fault => STATUS_FAULT,
        ConnState::Link => STATUS_LINK,
        ConnState::Offline => BORDER_1,
    };
    painter.rect_stroke(
        rect,
        Rounding::same(RADIUS_CARD),
        Stroke::new(1.0, stroke_color),
    );
}

// =============================================================================
// Tests (X-8 Stage 1 派生: ROI 高い ui_tokens のカバレッジを上げる)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conn_state_dot_color_distinct_per_state() {
        // 各状態で異なる色になることを確認 (Live と Fault は赤系で似ているが
        // RGB 値で区別可能)
        let colors = [
            ConnState::Offline.dot_color(),
            ConnState::Link.dot_color(),
            ConnState::Live.dot_color(),
            ConnState::Fault.dot_color(),
        ];
        // ペアワイズで一意 (LINK = ACCENT を共有してもよいが他とは別)
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "states {} and {} share color",
                    i, j
                );
            }
        }
    }

    #[test]
    fn conn_state_link_uses_accent() {
        // Design System §振る舞い: LINK は ACCENT (warm amber) と同色
        assert_eq!(ConnState::Link.dot_color(), ACCENT);
        assert_eq!(STATUS_LINK, ACCENT);
    }

    #[test]
    fn conn_state_label_format_jp_en() {
        // Design System §文言: "<日本語> · <英大文字>" 形式を維持
        for state in [
            ConnState::Offline,
            ConnState::Link,
            ConnState::Live,
            ConnState::Fault,
        ] {
            let label = state.label();
            assert!(label.contains(" · "), "label '{}' missing separator", label);
            // セパレータ前後に何かしら文字がある
            let parts: Vec<&str> = label.split(" · ").collect();
            assert_eq!(parts.len(), 2, "label '{}' is not 2-part", label);
            assert!(!parts[0].is_empty());
            assert!(!parts[1].is_empty());
            // 後半は ASCII 大文字
            assert!(
                parts[1].chars().all(|c| c.is_ascii_uppercase()),
                "second half of '{}' should be uppercase ASCII",
                label
            );
        }
    }

    #[test]
    fn spacing_scale_is_4px_based() {
        // Design System §余白: 4px scale 4, 8, 12, 16, 20, 24, 32, 40
        let scale = [
            SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5, SPACE_6, SPACE_7, SPACE_8,
        ];
        for v in scale {
            assert_eq!((v as i32) % 4, 0, "{} is not 4px-aligned", v);
        }
        // 単調増加
        for w in scale.windows(2) {
            assert!(w[0] < w[1]);
        }
    }

    #[test]
    fn font_scale_increasing() {
        // Design System §タイポグラフィ: 11/12/13/14/16/18/22/28
        let scale = [
            FONT_TINY,
            FONT_CAPTION,
            FONT_BODY_SM,
            FONT_BODY,
            FONT_LABEL,
            FONT_SUBHEAD,
            FONT_HEAD_SM,
            FONT_HEAD,
        ];
        for w in scale.windows(2) {
            assert!(w[0] < w[1]);
        }
        // 最小値の妥当性 (極端に小さいフォントは UI 不能)
        assert!(FONT_TINY >= 10.0);
        assert!(FONT_HEAD <= 64.0);
    }

    #[test]
    fn radius_hierarchy() {
        // 入力欄 < カード < モーダル
        assert!(RADIUS_INPUT < RADIUS_CARD);
        assert!(RADIUS_CARD < RADIUS_MODAL);
    }

    #[test]
    fn surface_colors_progressively_lighter() {
        // BG_0 が最も暗い、BG_4 に向けて段階的に明るくなる
        let surfaces = [BG_0, BG_1, BG_2, BG_3, BG_4];
        for w in surfaces.windows(2) {
            // ほぼ単調増加 (各チャネルの平均で比較)
            let lum_a = (w[0].r() as u32 + w[0].g() as u32 + w[0].b() as u32) / 3;
            let lum_b = (w[1].r() as u32 + w[1].g() as u32 + w[1].b() as u32) / 3;
            assert!(lum_a < lum_b, "{:?} should be darker than {:?}", w[0], w[1]);
        }
    }

    #[test]
    fn foreground_colors_progressively_darker() {
        // FG_1 が最も明るい (主テキスト色), FG_4 に向けて沈む
        let fgs = [FG_1, FG_2, FG_3, FG_4];
        for w in fgs.windows(2) {
            let lum_a = (w[0].r() as u32 + w[0].g() as u32 + w[0].b() as u32) / 3;
            let lum_b = (w[1].r() as u32 + w[1].g() as u32 + w[1].b() as u32) / 3;
            assert!(lum_a > lum_b, "{:?} should be lighter than {:?}", w[0], w[1]);
        }
    }

    #[test]
    fn accent_palette_consistency() {
        // ACCENT_LO < ACCENT < ACCENT_HI (明度で)
        let lum = |c: Color32| (c.r() as u32 + c.g() as u32 + c.b() as u32) / 3;
        assert!(lum(ACCENT_LO) < lum(ACCENT));
        assert!(lum(ACCENT) < lum(ACCENT_HI));
    }

    #[test]
    fn status_colors_distinct() {
        // 5 状態色がペアワイズで異なる
        let statuses = [
            STATUS_LIVE,
            STATUS_READY,
            STATUS_LINK,
            STATUS_OFFLINE,
            STATUS_FAULT,
        ];
        for i in 0..statuses.len() {
            for j in (i + 1)..statuses.len() {
                assert_ne!(statuses[i], statuses[j]);
            }
        }
    }

    #[test]
    fn live_and_fault_are_red_family() {
        // Design System: LIVE = warm red (録画ドット), FAULT = bright red (障害)
        // 両方とも R チャネルが他チャネルより高いことを期待
        assert!(STATUS_LIVE.r() > STATUS_LIVE.g());
        assert!(STATUS_LIVE.r() > STATUS_LIVE.b());
        assert!(STATUS_FAULT.r() > STATUS_FAULT.g());
        assert!(STATUS_FAULT.r() > STATUS_FAULT.b());
    }
}
