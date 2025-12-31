# Phase 4 仕様書: メトリクス & エラーハンドリング

**作成日**: 2025-12-31  
**バージョン**: 1.1  
**ステータス**: Phase 4.1.1 実装中

---

## 概要

Phase 4では、PC側アプリケーションにメトリクス収集・CSV出力機能とロバストなエラーハンドリングを追加し、長時間の連続稼働に対応します。

---

## Phase 4.1: メトリクス & CSV ログ機能

### 目的
- PC側のパフォーマンスメトリクスを記録
- CSV形式でログ出力
- Spresense側FPSの推定

### 実装済み機能

#### 1. メトリクス収集
- **PC FPS**: 1秒ごとに計算
- **Spresense FPS**: シーケンス番号から推定
- **フレーム数**: 累積カウント
- **JPEG デコード時間**: 平均値 (ms)
- **シリアル読み込み時間**: 平均値 (ms)
- **JPEG サイズ**: 平均値 (KB)

#### 2. CSV出力
**ファイル**: `metrics/metrics_YYYYMMDD_HHMMSS.csv`

**フォーマット**:
```csv
timestamp,pc_fps,spresense_fps,frame_count,error_count,decode_time_ms,serial_read_time_ms,texture_upload_time_ms,jpeg_size_kb
1767190113.960,13.5,13.5,1000,2,2.3,70.0,0.0,62.5
```

#### 3. GUI表示
ステータスバーにリアルタイムメトリクスを表示:
```
📊 PC: 13.5 fps | 🎬 Frames: 1000 | ❌ Errors: 2 | ⏱ Decode: 2.3ms | ...
```

### 実装ファイル
- `src/gui_main.rs`: MetricsLogger統合、統計収集
- `src/main.rs`: CLIメトリクス出力
- `src/metrics.rs`: メトリクス構造体、CSVライター
- `Cargo.toml`: csv クレート依存関係

---

## Phase 4.1.1: エラーハンドリング強化 (NEW)

### 目的
**連続稼働時の安定性確保**:
- JPEGデコードエラーを適切に処理
- エラーの種類を区別（パケットエラー vs JPEGエラー）
- 連続エラーの検出と警告
- エラー統計の詳細化

### 背景

#### 問題点（Phase 4.1）
1. **エラーカウントの混在**
   - パケット読み込みエラーとJPEGデコードエラーが同じカウンタ
   - JPEGエラーの連続発生を検出できない

2. **エラー時の動作**
   - JPEGデコード失敗時、新しいフレームを送信しない
   - 古いフレームが画面に残り続ける

3. **長時間稼働のリスク**
   - 散発的なエラーが蓄積
   - 連続でなければ停止しないため問題を見逃す

#### テスト結果（8分間）
- **総フレーム数**: 6,516
- **JPEGデコードエラー**: 29回（0.45%）
- **発生パターン**: 5秒間に集中発生
- **原因**: Spresense側のJPEG圧縮エラー（動的シーン）

### 仕様

#### 1. エラーカウンタの分離

**変数定義**:
```rust
let mut packet_error_count = 0u32;        // パケット読み込みエラー
let mut jpeg_decode_error_count = 0u32;   // JPEGデコードエラー
let mut consecutive_jpeg_errors = 0u32;   // 連続JPEGエラー
```

**目的**:
- エラーの種類を明確に区別
- 各エラータイプの統計を個別に収集

#### 2. JPEGデコードエラー処理

**成功時の動作**:
```rust
match image::load_from_memory(&packet.jpeg_data) {
    Ok(img) => {
        // 1. 連続エラーカウントをリセット
        consecutive_jpeg_errors = 0;
        
        // 2. 通常のデコード処理
        let decode_time_ms = decode_start.elapsed().as_secs_f32() * 1000.0;
        total_decode_time_ms += decode_time_ms;
        
        // 3. フレームを送信
        tx.send(AppMessage::DecodedFrame { ... }).ok();
    }
    // ...
}
```

**エラー時の動作**:
```rust
Err(e) => {
    // 1. エラーログ
    error!("Failed to decode JPEG: {}", e);
    
    // 2. エラーカウント更新
    jpeg_decode_error_count += 1;
    consecutive_jpeg_errors += 1;
    
    // 3. 連続エラー警告
    if consecutive_jpeg_errors == 5 {
        warn!("5 consecutive JPEG decode errors detected");
    } else if consecutive_jpeg_errors >= 10 {
        error!("10+ consecutive JPEG errors - possible Spresense issue");
    }
    
    // 4. エラーフレームをスキップ（フレーム送信しない）
    // → 古いフレームが表示され続ける
    // → 次のフレーム読み込みを継続
}
```

#### 3. パケット読み込みエラー処理

**変更前（Phase 4.1）**:
```rust
Err(e) => {
    if e.kind() != std::io::ErrorKind::TimedOut {
        error_count += 1;  // ← 混在
        // ...
        if error_count >= 10 {
            // 停止
        }
    }
}
```

**変更後（Phase 4.1.1）**:
```rust
Err(e) => {
    if e.kind() != std::io::ErrorKind::TimedOut {
        packet_error_count += 1;  // ← 分離
        
        if packet_error_count >= 10 {
            error!("Too many packet errors ({}), stopping", packet_error_count);
            break;
        }
    } else {
        // タイムアウトはエラーとしてカウントしない
        continue;
    }
}
```

**成功時のリセット**:
```rust
Ok(packet) => {
    packet_error_count = 0;  // パケット読み込み成功でリセット
    // ...
}
```

#### 4. メトリクス統計の更新

**AppMessage::Stats の拡張**:
```rust
tx.send(AppMessage::Stats {
    fps,
    spresense_fps: avg_spresense_fps,
    frame_count,
    errors: jpeg_decode_error_count,  // ← JPEGエラーのみ表示
    decode_time_ms: avg_decode_time_ms,
    serial_read_time_ms: avg_serial_read_time_ms,
    texture_upload_time_ms: 0.0,
    jpeg_size_kb: avg_jpeg_size_kb,
}).ok();
```

**CSVフォーマット（変更なし）**:
- `error_count` カラムはJPEGデコードエラーを記録
- パケットエラーは別途ログに記録

#### 5. エラーログの明確化

**JPEGデコードエラー**:
```
[ERROR] Failed to decode JPEG: The image format could not be determined
[WARN] 5 consecutive JPEG decode errors detected
```

**パケット読み込みエラー**:
```
[ERROR] Packet read error: Invalid sync word: 0x12345678
[ERROR] Too many packet errors (10), stopping capture thread
```

### 動作フロー

```
1. パケット読み込み
   ├─ 成功 → packet_error_count = 0
   └─ 失敗 → packet_error_count++
              ├─ < 10回: 次のパケットを読み込み
              └─ >= 10回: スレッド停止

2. JPEGデコード（パケット読み込み成功時のみ）
   ├─ 成功 → consecutive_jpeg_errors = 0
   │         フレームを送信
   └─ 失敗 → jpeg_decode_error_count++
              consecutive_jpeg_errors++
              ├─ == 5回: 警告ログ
              ├─ >= 10回: エラーログ
              └─ フレームをスキップ、次のパケットへ
```

### 期待される効果

1. **連続稼働の安定性向上**
   - JPEGエラーが発生してもアプリケーションは継続動作
   - パケットエラーのみでスレッド停止を判断

2. **問題の早期発見**
   - 連続5回のJPEGエラーで警告
   - 連続10回以上でSpresense側の問題を示唆

3. **デバッグの容易化**
   - エラーの種類が明確
   - ログから問題箇所を特定しやすい

4. **ユーザー体験の向上**
   - エラー発生時も映像が途切れない（前フレーム表示）
   - エラー統計で問題の程度を把握可能

### 非機能要件

- **パフォーマンス**: エラーハンドリングによるオーバーヘッドは無視できるレベル（< 0.1ms）
- **メモリ**: 追加メモリ使用量は最小限（カウンタ変数のみ）
- **互換性**: 既存のCSVフォーマットを変更しない

---

## Phase 4.2: Sync Word エラー回復（別ブランチ）

**ステータス**: 実装完了、`phase4.2-full-implementation` ブランチに保存

### 機能
- Sync word同期ずれの自動検出
- バイトストリームからのSync word探索
- 最大3回の自動リトライ

### 使用判断
- **Phase 4.1.1で十分な場合**: エラー率 < 1%
- **Phase 4.2が必要な場合**: エラー率 > 1%、USB接続不安定

---

## 実装ステータス

| フェーズ | ステータス | ブランチ |
|----------|-----------|----------|
| Phase 4.1 | ✅ 完了 | master |
| Phase 4.1.1 | 🚧 実装中 | master |
| Phase 4.2 | ✅ 完了（保存済み） | phase4.2-full-implementation |

---

## 関連ドキュメント

- `METRICS_GUIDE.md`: メトリクス使用ガイド
- `PHASE4_TEST_GUIDE.md`: テスト手順
- `PHASE4_TEST_RESULTS.md`: 8分間テスト結果分析
- `CURRENT_STATUS.md`: プロジェクト全体のステータス

---

**更新履歴**:
- 2025-12-31: Phase 4.1.1仕様追加
- 2025-12-31: Phase 4.1初版作成
