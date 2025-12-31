# Phase 4 テストガイド - クイックスタート

**作成日**: 2025-12-31
**目的**: CSV 出力と PC 側メトリクス測定の確認
**バージョン**: 2.0

---

## 🚀 テスト手順

### 1. アプリケーション起動

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# GUI アプリケーション起動（デバッグログ有効）
RUST_LOG=info cargo run --release --features gui
```

**重要**: `RUST_LOG=info` を指定すると、以下の情報がコンソールに表示されます：
- Metrics ファイルの保存先
- 統計情報の更新

### 2. 起動時の確認項目

コンソールに以下のログが表示されることを確認：

```
[INFO] 📊 Metrics logging to: "/home/ken/Rust_ws/security_camera_viewer/metrics/metrics_20251231_HHMMSS.csv"
[INFO] Stats: PC FPS=19.9, Frames=20
[INFO] Stats: PC FPS=19.8, Frames=40
```

### 3. GUI での確認

GUI ウィンドウの下部ステータスバーを確認：

```
📊 PC: 19.9 fps | 🎬 Frames: 120 | ❌ Errors: 0 | ⏱ Decode: 2.3ms | ...
```

**確認ポイント**:
- ✅ `📊 PC: 19.9 fps` が表示される
- ✅ フレーム数が増加していく
- ✅ エラー数が 0 のまま

### 4. CSV ファイルの確認

**場所**:
```bash
/home/ken/Rust_ws/security_camera_viewer/metrics/
```

**確認コマンド**:
```bash
# metrics ディレクトリの確認
ls -lh /home/ken/Rust_ws/security_camera_viewer/metrics/

# CSV ファイルの内容確認（最初の 5 行）
head -5 /home/ken/Rust_ws/security_camera_viewer/metrics/metrics_*.csv

# リアルタイムで追記を監視
tail -f /home/ken/Rust_ws/security_camera_viewer/metrics/metrics_*.csv
```

**期待される出力**:
```csv
timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb
1735650622.145,19.8,120,0,2.3,48.2,0.0,53.1
1735650623.147,19.9,140,0,2.2,47.8,0.0,52.9
1735650624.149,20.1,160,0,2.4,48.5,0.0,53.4
```

---

## 🔍 トラブルシューティング

### 問題 1: CSV ファイルが作成されない

**確認 1**: metrics ディレクトリの存在
```bash
ls -ld /home/ken/Rust_ws/security_camera_viewer/metrics/
```

**確認 2**: コンソールログで保存先を確認
```
[INFO] 📊 Metrics logging to: "..."
```
または
```
[ERROR] Failed to create metrics logger: ...
```

**解決策**: コンソールログのエラーメッセージを確認

---

**原因**: 書き込み権限不足

**解決策**:
```bash
chmod 755 /home/ken/Rust_ws/security_camera_viewer
mkdir -p /home/ken/Rust_ws/security_camera_viewer/metrics
chmod 755 /home/ken/Rust_ws/security_camera_viewer/metrics
```

---

### 問題 2: GUI に「Logging to: ...」が表示される

**これは正常です**:
- 接続ステータスに metrics ファイルのパスが表示されます
- 数秒後に「Connected」に変わります

---

## 📊 デバッグログの見方

### 正常な起動例

```
[INFO] Capture thread started
[INFO] 📊 Metrics logging to: "/home/ken/Rust_ws/security_camera_viewer/metrics/metrics_20251231_143022.csv"
[INFO] Stats: PC FPS=19.9, Frames=20
[INFO] Stats: PC FPS=19.8, Frames=40
```

### 異常な例

**エラーが継続的に発生**:
```
[INFO] Stats: PC FPS=19.9, Errors=5
[INFO] Stats: PC FPS=19.8, Errors=10
[INFO] Stats: PC FPS=19.7, Errors=15
```
→ シリアル通信の問題（USB ケーブル、ドライバ確認）

---

## ✅ Phase 4.2 テスト（30 分動作）

### 開始前の準備

```bash
cd /home/ken/Rust_ws/security_camera_viewer

# 既存の metrics ファイルを退避（オプション）
if [ -d metrics ]; then
    mv metrics metrics_backup_$(date +%Y%m%d_%H%M%S)
fi

# ログファイルを準備
RUST_LOG=info cargo run --release --features gui 2>&1 | tee phase4_test_$(date +%Y%m%d_%H%M%S).log
```

### 30 分後の確認

**1. CSV データ行数**:
```bash
# 期待: 約 1800 行（30分 × 60秒）
wc -l metrics/metrics_*.csv
```

**2. 平均 FPS**:
```bash
awk -F',' 'NR>1 {pc+=$2; count++} END {
  print "PC FPS:", pc/count
}' metrics/metrics_*.csv
```

**期待値**:
- PC FPS: 19.0-20.5

**3. エラー数**:
```bash
awk -F',' 'END {print "Errors:", $4}' metrics/metrics_*.csv
```

**期待値**: 0

---

## 🎯 次のステップ

Phase 4.2 が成功したら：
1. Phase 4.3: エラー回復テスト（USB 抜き差し）
2. Phase 4.4: 性能プロファイリング（1000 フレーム詳細分析）
3. Phase 4.5: 完了報告書作成

**Spresense 側メトリクス**:
- Spresense 側の実測 FPS、キュー深度などを取得する新プロトコルは `SPRESENSE_METRICS_PROTOCOL.md` 参照
- メトリクスプロトコル実装後、CSV に Spresense 側の詳細データが追加されます

---

**作成者**: Claude Code (Sonnet 4.5)
**バージョン**: 2.0 (Spresense FPS 推定削除版)
