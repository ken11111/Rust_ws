# Phase 7.3.3: エラーハンドリングモード実装仕様

## 概要

Phase 7.3.3では、PC側で処理が止まることでSpresense側も止まってしまう問題を解決し、商品性担保のため連続運転を可能にするエラーハンドリングモードを実装しました。

## 背景

### 問題

**Phase 7.1cで発見された問題**:
- 7個のバッファプールが満杯になるとSpresense側のキューが飽和
- PC側でエラーが発生すると10回の連続エラーで処理停止
- PC側が停止するとSpresense側も停止（連続運転不可）
- Metricsパケットが送信できなくなり、統計情報が欠落

**影響**:
- 商品性が低下（長時間運転が不可能）
- デバッグが困難（問題が発生しても早期に停止）

### 要件

1. **連続運転の実現**: エラーがあっても処理を継続
2. **デバッグモードの維持**: 開発時は現在の動作（エラーで停止）を維持
3. **モード切り替え**: GUI/CLIで簡単に切り替え可能
4. **統計記録**: エラー発生状況を可視化

## 設計

### 1. エラーハンドリングモード

#### モードの定義

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum ErrorHandlingMode {
    /// 本番モード: エラーがあっても処理継続（連続運転優先）
    Production,
    /// デバッグモード: 重大なエラーで停止（問題早期発見）
    Debug,
}
```

#### モードの特性

| 項目 | Production Mode | Debug Mode |
|------|-----------------|------------|
| **目的** | 連続運転の実現 | 問題の早期発見 |
| **Packet read error (10回)** | ログ記録 + 継続 | 停止 |
| **JPEG decode error** | ログ記録 + 継続 | ログ記録 + 継続 |
| **Metrics欠落** | 警告 + 継続 | 警告 + 継続 |
| **デフォルト** | Production | - |
| **用途** | 本番運用、長時間運転 | 開発、デバッグ |

### 2. 実装箇所

#### 2.1 CameraApp構造体

```rust
struct CameraApp {
    // ...existing fields...

    // Phase 7.3.3: Error handling mode
    error_handling_mode: ErrorHandlingMode,
}
```

#### 2.2 GUI切り替えスイッチ

**Settings パネル**に配置:
```rust
// Phase 7.3.3: Error handling mode
ui.label("Error Handling:");
ui.horizontal(|ui| {
    ui.radio_value(&mut self.error_handling_mode, ErrorHandlingMode::Production, "🟢 Production");
    ui.radio_value(&mut self.error_handling_mode, ErrorHandlingMode::Debug, "🔴 Debug");
});
ui.label(format!("  {}", self.error_handling_mode.as_str()));
```

**表示例**:
- 🟢 Production: "Production (連続運転)"
- 🔴 Debug: "Debug (エラーで停止)"

#### 2.3 capture_threadエラー処理

**修正前（Phase 7.3.2まで）**:
```rust
if packet_error_count >= 10 {
    error!("Too many consecutive packet errors ({}), stopping capture thread", packet_error_count);
    tx.send(AppMessage::ConnectionStatus("Too many packet errors".to_string())).ok();
    break;  // 常に停止
}
```

**修正後（Phase 7.3.3）**:
```rust
if packet_error_count >= 10 {
    if error_handling_mode.is_debug() {
        // Debug mode: Stop on critical errors
        error!("Too many consecutive packet errors ({}), stopping capture thread (Debug mode)", packet_error_count);
        tx.send(AppMessage::ConnectionStatus("Too many packet errors".to_string())).ok();
        break;
    } else {
        // Production mode: Log and continue
        warn!("Too many consecutive packet errors ({}), continuing (Production mode)", packet_error_count);
        // Don't break - continue operation for continuous running
    }
}
```

### 3. モード別動作

#### Production Mode（本番モード）

**目的**: 連続運転の実現、商品性担保

**動作**:
1. Packet read error（10回連続）
   - 警告ログを出力
   - 処理を継続（停止しない）
   - エラーカウンターは継続してカウント

2. JPEG decode error
   - フレームをスキップ
   - エラー統計を記録
   - 処理を継続

3. Metrics packet欠落
   - 警告ログを出力
   - PC側で計算した統計を使用
   - 処理を継続

**用途**:
- 本番運用
- 長時間運転（24時間連続など）
- デモンストレーション

#### Debug Mode（デバッグモード）

**目的**: 問題の早期発見、デバッグ効率化

**動作**:
1. Packet read error（10回連続）
   - エラーログを出力
   - 処理を停止（従来の動作）
   - GUIに"Too many packet errors"を表示

2. JPEG decode error
   - フレームをスキップ
   - エラー統計を記録
   - 処理を継続（Production modeと同じ）

3. Metrics packet欠落
   - 警告ログを出力
   - 処理を継続（Production modeと同じ）

**用途**:
- 開発時のデバッグ
- 問題の再現テスト
- 新機能の動作確認

## 実装結果

### ファイル変更

**PC側（Rust）**:
- `src/gui_main.rs`:
  - ErrorHandlingMode enum定義（63-94行目）
  - CameraApp構造体にフィールド追加（203行目）
  - GUI切り替えスイッチ（763-770行目）
  - capture_thread シグネチャ修正（969行目）
  - エラー処理ロジック修正（1391-1403行目）

### ビルド

```bash
cd /home/ken/Rust_ws/security_camera_viewer
cargo build --release --features gui --bin security_camera_gui
```

**結果**: ✅ 成功（warning のみ）

## テスト計画

### テストケース

#### TC1: Production Mode - パケットエラー継続
1. Production Modeに設定
2. USB/TCP接続を不安定にする
3. 10回以上のパケットエラーが発生
4. **期待結果**: 処理が継続、警告ログが出力

#### TC2: Debug Mode - パケットエラー停止
1. Debug Modeに設定
2. USB/TCP接続を不安定にする
3. 10回連続パケットエラーが発生
4. **期待結果**: 処理が停止、"Too many packet errors"表示

#### TC3: Production Mode - 長時間運転
1. Production Modeに設定
2. 2時間以上連続運転
3. 途中でエラーが発生しても継続
4. **期待結果**: 2時間後も正常動作

#### TC4: モード切り替え
1. Debug Modeで開始
2. 実行中にProduction Modeに切り替え
3. **期待結果**: 次回起動時に反映（実行中は変更されない）

## 今後の拡張

### Phase 7.4候補: エラー統計の可視化

**目的**: エラー発生状況をGUI/CSVで可視化

**実装案**:
1. エラーカウンター追加:
   - `total_packet_errors`: 累積パケットエラー数
   - `total_jpeg_errors`: 累積JPEG decodeエラー数
   - `metrics_missing_count`: Metrics欠落回数

2. GUI表示:
   - 統計パネルにエラー数を表示
   - エラー率（%）を表示

3. CSV記録:
   - エラー統計をCSVに記録
   - 後から分析可能

### Phase 7.5候補: 自動リカバリー

**目的**: エラー発生時の自動回復

**実装案**:
1. 接続自動再確立:
   - TCP接続切断時に自動再接続
   - USB Serial切断時に再オープン

2. バッファフラッシュ:
   - エラー累積時にバッファをクリア
   - 同期ずれを自動修正

## まとめ

Phase 7.3.3では、エラーハンドリングモードを導入し、連続運転を実現しました。

**達成したこと**:
- ✅ Production/Debug モードの実装
- ✅ GUI切り替えスイッチ
- ✅ capture_threadエラー処理修正
- ✅ ビルド成功

**メリット**:
- 商品性向上（連続運転可能）
- デバッグ効率維持（Debug mode）
- 柔軟な運用（モード切り替え）

**次のステップ**:
- 実機テスト（TC1-TC4）
- エラー統計の可視化（Phase 7.4）
- 自動リカバリー（Phase 7.5）
