# Security Camera Viewer - Current Status

**日付**: 2025-12-31
**ブランチ**: master (Phase 4.1 baseline)
**フェーズ**: Phase 4.1 (メトリクス & CSV ログ機能のみ)

---

## ✅ 実装されている機能

### Phase 4.1: メトリクス & CSV ログ機能

**CSV 出力機能**:
- メトリクスファイル: `metrics/metrics_YYYYMMDD_HHMMSS.csv`
- 記録データ:
  - タイムスタンプ
  - PC FPS
  - フレーム数
  - エラー数
  - JPEG デコード時間
  - シリアル読み込み時間
  - JPEG サイズ

**GUI 表示機能**:
- ステータスバーにリアルタイムメトリクス表示
- FPS、フレーム数、エラー数等の可視化

**実装ファイル**:
- `src/gui_main.rs`: MetricsLogger と統計収集
- `src/main.rs`: CLI メトリクス出力
- `src/metrics.rs`: メトリクス構造体
- `Cargo.toml`: csv クレート依存関係

**ドキュメント**:
- `METRICS_GUIDE.md`: メトリクス使用ガイド (v2.0)
- `SPRESENSE_METRICS_PROTOCOL.md`: 将来のメトリクスプロトコル仕様
- `PHASE4_TEST_GUIDE.md`: テスト手順 (v2.0)

---

## 🔀 ブランチ構成

### master (現在のブランチ)
- **状態**: Phase 4.1 baseline
- **機能**: メトリクス & CSV ログのみ
- **エラー回復**: なし (シンプルな実装)

### phase4.2-full-implementation
- **状態**: Phase 4.2 完全実装
- **機能**: Phase 4.1 + Sync Word エラー回復
- **GitHub**: https://github.com/ken11111/Rust_ws/tree/phase4.2-full-implementation

**Phase 4.2 の実装内容** (別ブランチに保存済み):
- `find_sync_word()`: Sync word 探索機能
- `read_packet_after_sync()`: Sync word 消費後のパケット読み込み
- `read_packet_with_recovery()`: 自動エラー回復 (最大3回リトライ)
- 詳細な診断ログ (ERROR レベル)

---

## 🎯 Phase 4.1 の使用方法

### ビルド

```bash
# Linux 版
cargo build --release --features gui

# Windows 版
cargo build --release --target x86_64-pc-windows-gnu --features gui
```

### 実行

```bash
# Linux
RUST_LOG=info ./target/release/security_camera_gui

# Windows
.\security_camera_gui.exe
```

### メトリクス確認

```bash
# CSV ファイルの確認
ls -lh metrics/

# 最新のメトリクスを表示
tail -20 metrics/metrics_*.csv

# 平均 FPS を計算
awk -F',' 'NR>1 {sum+=$2; count++} END {print "Avg PC FPS:", sum/count}' metrics/metrics_*.csv
```

---

## 📊 期待される動作

### 正常動作時

```bash
# コンソール出力
[INFO] Stats: PC FPS=19.9, Frames=20
[INFO] Stats: PC FPS=19.8, Frames=40
```

### エラー発生時 (Phase 4.1 では自動回復なし)

```bash
# エラーが発生すると停止する可能性がある
[ERROR] Packet read error: Invalid sync word: 0x12345678
[ERROR] Too many consecutive errors (10), stopping capture thread
```

**注意**: Phase 4.1 にはエラー回復機能がないため、一度同期がずれると復帰できません。Phase 4.2 (別ブランチ) にエラー回復機能があります。

---

## 🔄 Phase 4.2 への切り替え

エラー回復機能が必要な場合は、Phase 4.2 ブランチに切り替えることができます:

```bash
# Phase 4.2 ブランチに切り替え
git checkout phase4.2-full-implementation

# ビルド
cargo build --release --features gui

# 元に戻す
git checkout master
```

---

## 📁 ファイル構成

```
security_camera_viewer/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── gui_main.rs          # GUI with metrics (Phase 4.1)
│   ├── serial.rs            # Serial comm (basic, no recovery)
│   ├── protocol.rs          # MJPEG packet protocol
│   └── metrics.rs           # Metrics structures
├── docs/
│   ├── METRICS_GUIDE.md
│   ├── PHASE4_TEST_GUIDE.md
│   └── SPRESENSE_METRICS_PROTOCOL.md
├── Cargo.toml
└── README.md
```

---

## 🚀 次のステップ

### Spresense 側の対応

現在、Spresense から送信されるデータに問題があるため、PC 側でどのような実装を使用しても正常動作しません:

**問題**:
- 全パケットで `jpeg_size=65536` (不自然に一定)
- JPEG データに SOI/EOI マーカーなし
- デコード不可能

**必要な作業**:
1. Spresense のパケット送信コードを確認
2. JPEG 圧縮処理を確認
3. エンディアン (little-endian) を確認
4. JPEG マーカーの検証ログを追加

### PC 側の選択肢

Spresense 側が修正されたら:

**Option A: Phase 4.1 を使用** (現在の master)
- シンプルな実装
- エラー回復機能なし
- 安定したデータストリームが必要

**Option B: Phase 4.2 を使用** (別ブランチ)
- エラー回復機能あり
- 一時的なノイズに対応
- USB 接続が不安定な環境に適している

---

## 📂 Git コマンド

```bash
# 現在のブランチ確認
git branch -a

# ブランチ切り替え
git checkout phase4.2-full-implementation  # Phase 4.2
git checkout master                        # Phase 4.1

# 変更の確認
git status
git diff

# コミット
git add <file>
git commit -m "message"
git push
```

---

## 📞 ドキュメント

- **メトリクス使用方法**: `METRICS_GUIDE.md`
- **テスト手順**: `PHASE4_TEST_GUIDE.md`
- **将来のプロトコル**: `SPRESENSE_METRICS_PROTOCOL.md`

---

**作成者**: Claude Code (Sonnet 4.5)
**最終更新**: 2025-12-31
**ブランチ**: master (Phase 4.1 baseline)
