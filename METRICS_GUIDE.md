# メトリクス測定ガイド

**最終更新**: 2025-12-31
**対象**: Phase 4 - VGA 統合動作テスト
**バージョン**: 2.0 (Spresense メトリクスプロトコル対応版)

---

## 📊 概要

Phase 4 で追加された性能測定機能について説明します。

### 機能

1. **PC 側メトリクスの測定**
   - PC で受信・表示している実効 FPS
   - JPEG デコード時間、シリアル読み込み時間などの詳細計測
   - GUI 画面にリアルタイム表示

2. **CSV 形式でのメトリクス記録**
   - 全性能データを CSV ファイルに自動保存
   - 24 時間テストなど長時間動作の評価に最適

3. **Spresense 側メトリクスの送信** (実装予定)
   - Spresense から実測メトリクスを専用パケットで送信
   - Spresense 側の FPS、キュー深度、エラー数などを記録
   - 詳細は `SPRESENSE_METRICS_PROTOCOL.md` 参照

---

## 🎯 測定項目

### GUI 表示（リアルタイム）

```
📊 PC: 19.9 fps | 🎬 Frames: 1250 | ❌ Errors: 0 |
⏱ Decode: 2.3ms | 📨 Serial: 48ms | 🖼 Texture: 0ms | 📦 JPEG: 53.2KB
```

| アイコン | 項目 | 説明 |
|---------|------|------|
| 📊 PC | PC 側 FPS | PC で受信・表示している実効 FPS |
| 🎬 Frames | フレーム数 | 受信した総フレーム数 |
| ❌ Errors | エラー数 | 通信エラー回数 |
| ⏱ Decode | デコード時間 | JPEG → RGBA 変換時間 |
| 📨 Serial | シリアル読み込み時間 | USB からの読み込み時間 |
| 🖼 Texture | テクスチャ時間 | GPU アップロード時間（未測定） |
| 📦 JPEG | JPEG サイズ | 受信した JPEG データサイズ |

**注意**: Spresense 側 FPS は GUI に表示されません。Spresense 側の実測メトリクスは CSV に記録されます（メトリクスプロトコル実装後）。

---

## 📁 CSV 出力形式

### ファイル保存場所

```bash
metrics/
└── metrics_20251231_143022.csv  # タイムスタンプ付きファイル名
```

### CSV ヘッダー (Phase 4.1 現在)

```csv
timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb
```

### カラム定義 (Phase 4.1 現在)

| カラム | 型 | 単位 | 説明 |
|--------|-----|------|------|
| `timestamp` | float | 秒 | Unix タイムスタンプ（秒.ミリ秒） |
| `pc_fps` | float | fps | PC 側 FPS |
| `frame_count` | integer | フレーム | 累積フレーム数 |
| `error_count` | integer | 回 | エラー発生回数 |
| `decode_time_ms` | float | ms | JPEG デコード時間 |
| `serial_read_time_ms` | float | ms | シリアル読み込み時間 |
| `texture_upload_time_ms` | float | ms | テクスチャアップロード時間 |
| `jpeg_size_kb` | float | KB | JPEG データサイズ |

### 将来の拡張 (メトリクスプロトコル実装後)

Spresense メトリクスプロトコル実装後は、以下のカラムが追加されます:

```csv
timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb,spresense_timestamp_ms,spresense_camera_frames,spresense_camera_fps,spresense_usb_packets,spresense_action_q_depth,spresense_avg_packet_size,spresense_errors
```

追加カラムの詳細は `SPRESENSE_METRICS_PROTOCOL.md` を参照してください。

### サンプルデータ (Phase 4.1 現在)

```csv
timestamp,pc_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb
1735650622.145,19.8,120,0,2.3,48.2,0.0,53.1
1735650623.147,19.9,140,0,2.2,47.8,0.0,52.9
1735650624.149,20.1,160,0,2.4,48.5,0.0,53.4
```

**更新頻度**: 1 秒ごと（統計更新と同期）

---

## 🔧 使用方法

### 1. アプリケーション起動

```bash
# Linux（ネイティブ）
cd /home/ken/Rust_ws/security_camera_viewer
cargo run --release --features gui

# Windows（クロスコンパイル）
cargo build --release --target x86_64-pc-windows-gnu --features gui
# Windows で target/x86_64-pc-windows-gnu/release/security_camera_gui.exe を実行
```

### 2. メトリクス自動記録

アプリケーション起動時に自動で CSV ファイルが作成されます。

**ログ出力例**:
```
[INFO] Metrics logging to: "metrics/metrics_20251231_143022.csv"
```

### 3. データ確認

```bash
# ヘッダー確認
head -1 metrics/metrics_20251231_143022.csv

# 最新 10 行表示
tail -10 metrics/metrics_20251231_143022.csv

# 行数カウント（測定時間の推定）
wc -l metrics/metrics_20251231_143022.csv
# 例: 3600 行 → 約 1 時間（1 秒/行）
```

---

## 📈 データ分析例

### 1. 平均 FPS の計算

```bash
# PC 側 FPS 平均（2 列目）
awk -F',' 'NR>1 {sum+=$2; count++} END {print "Average PC FPS:", sum/count}' \
  metrics/metrics_20251231_143022.csv
```

### 2. エラー率の計算

```bash
# 総フレーム数とエラー数
awk -F',' 'NR>1 {frames=$3; errors=$4} END {print "Total frames:", frames, "| Errors:", errors, "| Error rate:", (errors/frames)*100 "%"}' \
  metrics/metrics_20251231_143022.csv
```

### 3. パフォーマンスサマリ

```bash
awk -F',' '
NR>1 {
    pc_fps+=$2;
    decode+=$5; serial+=$6; jpeg+=$8;
    count++
}
END {
    print "=== Performance Summary ==="
    print "PC FPS:        ", pc_fps/count
    print "Decode time:   ", decode/count, "ms"
    print "Serial time:   ", serial/count, "ms"
    print "JPEG size:     ", jpeg/count, "KB"
}' metrics/metrics_20251231_143022.csv
```

### 4. Excel / Python での分析

**Excel**:
1. CSV ファイルを開く
2. データ → テキストから列へ → カンマ区切り
3. グラフ作成（時系列 FPS など）

**Python + pandas**:
```python
import pandas as pd
import matplotlib.pyplot as plt

# データ読み込み
df = pd.read_csv('metrics/metrics_20251231_143022.csv')

# タイムスタンプを日時に変換
df['datetime'] = pd.to_datetime(df['timestamp'], unit='s')

# FPS 推移グラフ
plt.figure(figsize=(12, 6))
plt.plot(df['datetime'], df['pc_fps'], label='PC FPS')
plt.xlabel('Time')
plt.ylabel('FPS')
plt.title('PC FPS Over Time')
plt.legend()
plt.grid(True)
plt.savefig('fps_over_time.png')
plt.show()

# 統計サマリ
print(df.describe())
```

---

## 🎯 Phase 4 テスト用途

### 1. 統合動作テスト（30 分）

**目的**: PC と Spresense の連携確認

```bash
# 30 分動作
# 期待データ: 約 1800 行（30分 × 60秒）

# テスト後の確認
wc -l metrics/metrics_*.csv
awk -F',' 'NR>1 {sum+=$2; count++} END {print "Avg PC FPS:", sum/count}' metrics/metrics_*.csv
```

**合格基準**:
- PC FPS: 19.0-20.5 fps（±5%）
- エラー数: 0

### 2. エラー回復テスト（15 分）

**目的**: USB 抜き差しやリセット時の挙動確認

```bash
# CSV でエラー発生時刻を特定
awk -F',' 'NR>1 && $4>0 {print "Error at timestamp:", $1, "| Errors:", $4}' \
  metrics/metrics_*.csv
```

**確認項目**:
- エラー後の自動復帰（error_count が再度 0 に戻る）
- FPS の回復時間（< 5 秒）

### 3. 性能プロファイリング（15 分）

**目的**: 1000 フレーム分の詳細データ取得

```bash
# 1000 フレーム到達時点のデータ（約 50 秒）
awk -F',' 'NR>1 && $3>=1000 {print; exit}' metrics/metrics_*.csv
```

**評価項目**:
- デコード時間の安定性（標準偏差 < 0.5ms）
- JPEG サイズの変動（シーンによる変化）

### 4. 24 時間テスト

**目的**: 長時間安定性の検証

```bash
# 24 時間 = 86400 秒 = 約 86400 行

# 1 時間ごとの FPS 推移
awk -F',' '
NR>1 {
    hour=int(($1 - start_time) / 3600)
    pc_fps[hour]+=$2
    count[hour]++
    if (NR==2) start_time=$1
}
END {
    for (h in pc_fps) {
        print "Hour", h, ": PC =", pc_fps[h]/count[h], "fps"
    }
}' metrics/metrics_*.csv
```

**合格基準**:
- 全期間で FPS 安定（変動 < 5%）
- メモリリークなし（エラー増加なし）
- ディスク I/O エラーなし

---

## 🔍 トラブルシューティング

### 問題 1: CSV ファイルが作成されない

**原因**: `metrics/` ディレクトリの権限不足

**解決策**:
```bash
mkdir -p /home/ken/Rust_ws/security_camera_viewer/metrics
chmod 755 /home/ken/Rust_ws/security_camera_viewer/metrics
```

### 問題 2: CSV のタイムスタンプがおかしい

**原因**: システム時刻の不一致

**確認**:
```bash
date +%s  # 現在の Unix タイムスタンプ
# CSV の timestamp と比較
```

---

## 📚 参考実装

### メトリクスモジュール

**ファイル**: `/home/ken/Rust_ws/security_camera_viewer/src/metrics.rs`

**主要構造体**:
- `PerformanceMetrics`: 性能データ構造
- `MetricsLogger`: CSV 書き込み

**主要関数**:
```rust
// CSV ログ
fn log(&self, metrics: &PerformanceMetrics) -> io::Result<()>
```

### GUI 統合

**ファイル**: `/home/ken/Rust_ws/security_camera_viewer/src/gui_main.rs`

**変更点**:
- `AppMessage::Stats` にメトリクスデータ追加
- `capture_thread` で統計計算
- 1 秒ごとに CSV ログ出力

---

## ✅ Phase 4 チェックリスト

- [ ] `metrics/` ディレクトリに CSV ファイルが作成される
- [ ] CSV に 1 秒ごとにデータが記録される
- [ ] 30 分動作で約 1800 行のデータ取得
- [ ] エラー発生時の CSV 記録を確認
- [ ] データ分析で性能評価を実施

---

## 🔗 関連ドキュメント

- **Spresense メトリクスプロトコル仕様**: `SPRESENSE_METRICS_PROTOCOL.md`
  - Spresense 側の実測 FPS、キュー深度などを取得する新プロトコル
  - メトリクスパケット設計、実装仕様、CSV フォーマット拡張
- **Phase 4 テストガイド**: `PHASE4_TEST_GUIDE.md`
  - クイックスタートガイド、トラブルシューティング

---

**作成者**: Claude Code (Sonnet 4.5)
**作成日**: 2025-12-31
**バージョン**: 2.0 (Spresense FPS 推定削除、メトリクスプロトコル対応)
**対象フェーズ**: Phase 4 - VGA 統合動作テスト
