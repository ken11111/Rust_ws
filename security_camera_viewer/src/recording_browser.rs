//! 録画ファイル ブラウザ (X-5c)
//!
//! Q11 (アプリ内再生 UI) に対応する egui ベースのファイル一覧パネル。
//! 当面は **外部プレーヤー起動** + メタデータ表示で実用性を確保し、
//! 将来 ffmpeg-next を導入した内部デコード再生に拡張する余地を残す。
//!
//! 関連:
//! - docs/security_camera/02_specifications/quality/PENDING_NFR_WORK.md X-5c
//! - docs/security_camera/01_requirements/FUNCTIONAL_REQUIREMENTS.md Q11
//! - 録画パス: mp4_recorder::RecordingPolicy::directory (default `./recordings`)
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use eframe::egui;

use crate::ui_tokens;

/// 録画ファイル 1 件の情報。
#[derive(Debug, Clone)]
pub struct RecordingEntry {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified: SystemTime,
}

impl RecordingEntry {
    pub fn filename(&self) -> String {
        self.path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("(unknown)")
            .to_string()
    }

    pub fn size_human(&self) -> String {
        let bytes = self.size_bytes;
        if bytes >= 1_000_000_000 {
            format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
        } else if bytes >= 1_000_000 {
            format!("{:.1} MB", bytes as f64 / 1_000_000.0)
        } else if bytes >= 1_000 {
            format!("{:.1} KB", bytes as f64 / 1_000.0)
        } else {
            format!("{} B", bytes)
        }
    }

    pub fn modified_local(&self) -> String {
        let datetime: chrono::DateTime<chrono::Local> = self.modified.into();
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

/// 録画ファイル ブラウザの UI 状態。
pub struct RecordingBrowser {
    pub directory: PathBuf,
    entries: Vec<RecordingEntry>,
    selected: Option<usize>,
    last_scan_error: Option<String>,
    last_action_result: Option<String>,
}

impl RecordingBrowser {
    pub fn new(directory: PathBuf) -> Self {
        let mut s = Self {
            directory,
            entries: Vec::new(),
            selected: None,
            last_scan_error: None,
            last_action_result: None,
        };
        s.refresh();
        s
    }

    /// ディレクトリを再スキャン (新規ファイルが増えた / 削除されたとき)。
    pub fn refresh(&mut self) {
        self.entries.clear();
        self.selected = None;
        self.last_scan_error = None;

        let dir = match fs::read_dir(&self.directory) {
            Ok(d) => d,
            Err(e) => {
                self.last_scan_error = Some(format!(
                    "ディレクトリ読込失敗: {} ({})",
                    self.directory.display(),
                    e
                ));
                return;
            }
        };

        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("mp4") {
                continue;
            }
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            self.entries.push(RecordingEntry {
                path,
                size_bytes: meta.len(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            });
        }

        // 新しい順 (mtime desc)
        self.entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    }

    /// 選択中ファイルを OS の既定アプリで開く (`xdg-open` / `open` / `start`)。
    pub fn launch_selected(&mut self) {
        let path = match self.selected.and_then(|i| self.entries.get(i)) {
            Some(e) => e.path.clone(),
            None => return,
        };
        match launch_external(&path) {
            Ok(_) => {
                self.last_action_result = Some(format!(
                    "外部プレーヤーで開きました: {}",
                    path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("(unknown)")
                ));
            }
            Err(e) => {
                self.last_action_result = Some(format!("起動失敗: {}", e));
            }
        }
    }

    /// 選択中ファイルを削除。
    pub fn delete_selected(&mut self) {
        let path = match self.selected.and_then(|i| self.entries.get(i)) {
            Some(e) => e.path.clone(),
            None => return,
        };
        match fs::remove_file(&path) {
            Ok(_) => {
                self.last_action_result = Some(format!(
                    "削除しました: {}",
                    path.file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("(unknown)")
                ));
                self.refresh();
            }
            Err(e) => {
                self.last_action_result = Some(format!("削除失敗: {}", e));
            }
        }
    }

    pub fn total_size_bytes(&self) -> u64 {
        self.entries.iter().map(|e| e.size_bytes).sum()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// 録画ファイル一覧を SidePanel として描画。
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("ファイル · FILES");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(format!("{} files", self.count()));
            ui.label("/");
            let total = self.total_size_bytes();
            let total_human = if total >= 1_000_000_000 {
                format!("{:.2} GB", total as f64 / 1_000_000_000.0)
            } else if total >= 1_000_000 {
                format!("{:.1} MB", total as f64 / 1_000_000.0)
            } else {
                format!("{} B", total)
            };
            ui.label(total_human);
            if ui.small_button("再読込").clicked() {
                self.refresh();
            }
        });

        if let Some(err) = &self.last_scan_error {
            ui.colored_label(ui_tokens::STATUS_FAULT, err);
            return;
        }

        ui.separator();

        // テーブル本体
        let row_count = self.entries.len();
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                for i in 0..row_count {
                    let entry = &self.entries[i];
                    let selected = self.selected == Some(i);
                    let label = format!(
                        "{}  ·  {}  ·  {}",
                        entry.filename(),
                        entry.size_human(),
                        entry.modified_local()
                    );
                    if ui.selectable_label(selected, label).clicked() {
                        self.selected = Some(i);
                    }
                }
                if row_count == 0 {
                    ui.label("(録画ファイルなし)");
                }
            });

        ui.separator();

        ui.horizontal(|ui| {
            let has_selection = self.selected.is_some();
            if ui
                .add_enabled(has_selection, egui::Button::new("外部で開く"))
                .clicked()
            {
                self.launch_selected();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("削除"))
                .on_hover_text("ファイルを完全に削除します")
                .clicked()
            {
                self.delete_selected();
            }
        });

        if let Some(msg) = &self.last_action_result {
            ui.add_space(ui_tokens::SPACE_1);
            ui.small(msg);
        }
    }
}

/// OS 既定アプリでパスを開く。
///
/// プラットフォーム別:
/// - Linux/WSL: `xdg-open`
/// - macOS: `open`
/// - Windows: `cmd /C start ""`
fn launch_external(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", path.to_str().unwrap_or("")])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn empty_directory_lists_no_entries() {
        let dir = tempdir().unwrap();
        let browser = RecordingBrowser::new(dir.path().to_path_buf());
        assert_eq!(browser.count(), 0);
        assert_eq!(browser.total_size_bytes(), 0);
    }

    #[test]
    fn lists_mp4_files_only() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.mp4"), b"hello").unwrap();
        fs::write(dir.path().join("b.txt"), b"ignored").unwrap();
        fs::write(dir.path().join("c.mp4"), b"world!!").unwrap();

        let browser = RecordingBrowser::new(dir.path().to_path_buf());
        assert_eq!(browser.count(), 2);
        assert_eq!(browser.total_size_bytes(), 5 + 7);
    }

    #[test]
    fn missing_directory_records_error() {
        let browser = RecordingBrowser::new(PathBuf::from("/nonexistent_dir_for_test"));
        assert_eq!(browser.count(), 0);
        assert!(browser.last_scan_error.is_some());
    }
}
