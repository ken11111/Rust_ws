# Phase 4.2 テスト結果 - バグ報告書

**日付**: 2025-12-31
**テスト実施者**: ユーザー
**分析者**: Claude Code (Sonnet 4.5)
**重要度**: 🔴 Critical

---

## 📊 テスト結果サマリ

### テスト環境
- **実施日時**: 2025-12-31 10:23:35 - 10:38:29 (約15分)
- **PC 側実装**: Phase 4.1 (メトリクス機能拡張版)
- **Spresense 側**: Phase 1.5 (VGA 30fps パイプライン版)
- **接続**: USB シリアル (115200 bps)

### 性能測定結果

| 項目 | 測定値 | 期待値 (Phase 3.0) | 判定 |
|------|--------|-------------------|------|
| **PC FPS** | 14.87 fps | 19-20 fps | ❌ **26% 低下** |
| **Serial 時間** | 68.27 ms | 48 ms | ❌ **42% 悪化** |
| **JPEG サイズ** | 56.18 KB | 53 KB | ⚠️ 6% 増加 |
| **累積エラー** | 114 回 | 0 回 | ❌ **重大** |
| **デコード失敗率** | 43.5% | 0% | ❌ **重大** |

### 重大な問題

1. **JPEG デコードエラーの多発**
   ```
   [ERROR] Failed to decode JPEG: The image format could not be determined
   ```
   - 10:33:08 から連続発生
   - デコード時間 0.00 ms が 43.5% の行で発生
   - GUI が時間経過で更新されなくなる

2. **性能劣化**
   - PC FPS が 26% 低下 (20 fps → 14.87 fps)
   - シリアル読み込み時間が 42% 悪化 (48ms → 68ms)

3. **動作の不安定性**
   - FPS 変動範囲: 0.09 ~ 21.20 fps (変動幅 21 fps!)
   - 累積エラー数: 114 回

---

## 🔍 原因分析

### 根本原因: Sync Word 同期ずれ

**問題のあるコード** (`src/serial.rs:110-146`):

```rust
pub fn read_packet(&mut self) -> io::Result<MjpegPacket> {
    // Read header first (12 bytes)
    let mut header_buf = [0u8; MJPEG_HEADER_SIZE];
    self.read_exact(&mut header_buf)?;  // ← 問題箇所①

    let header = MjpegHeader::parse(&header_buf)?;  // ← 問題箇所②

    // Read JPEG data + CRC
    let remaining_size = header.jpeg_size as usize + 2;
    self.read_exact(&mut packet_buf[MJPEG_HEADER_SIZE..total_size])?;

    MjpegPacket::parse(&packet_buf)  // ← 問題箇所③
}
```

**問題の流れ**:

```
Step 1: 正常なパケット受信
  [0xBE 0xBA 0xFE 0xCA] [seq] [size] [JPEG data...] [CRC]
   ↑ Sync word 検出成功

Step 2: ノイズや一時的な通信エラーで1バイトずれる
  [0xXX] [0xBE 0xBA 0xFE 0xCA] [seq] [size] [JPEG data...] [CRC]
   ↑ 余分な1バイト

Step 3: 次の read_exact() で12バイト読む
  [0xXX 0xBE 0xBA 0xFE 0xCA seq] [...]
   ↑ これをヘッダーとして解釈

Step 4: Sync word チェック失敗 (0xCA FE BA BE XX != 0xCA FE BA BE)
  MjpegHeader::parse() がエラー → io::Error 返却

Step 5: エラーハンドリング (gui_main.rs:356-370)
  error_count++
  continue;  // ← 同期復帰しないまま次のループ

Step 6: 再度 read_exact() で12バイト読む（ずれたまま）
  永遠に同期が取れない → 以降すべてのパケットが破損
```

### 副次的な問題

#### 1. エラー回復処理の不備

**現在のエラーハンドリング** (`src/gui_main.rs:356-370`):

```rust
let read_result = serial.read_packet();
match read_result {
    Ok(packet) => {
        // 正常処理
    }
    Err(e) => {
        error_count += 1;
        error!("Packet read error: {}", e);
        continue;  // ← 同期復帰しない!
    }
}
```

**問題点**:
- エラー発生後、sync word を探索しない
- 次のループで再度同じ位置から読み始める
- 同期ずれが永続化

#### 2. JPEG バリデーション不足

**現在の実装**:
```rust
// protocol.rs に is_valid_jpeg() は実装されているが、
// serial.rs の read_packet() では使用されていない
```

**問題点**:
- CRC チェックは通過するが、JPEG マーカー (0xFF 0xD8, 0xFF 0xD9) が壊れているケースを検出できない
- `image::load_from_memory()` でエラーになって初めて気づく

#### 3. タイムアウト設定の妥当性

**現在の設定**:
```rust
.timeout(Duration::from_millis(1000))  // 1秒
```

**問題点**:
- Spresense が 30fps で送信 → 33ms 間隔
- 1秒タイムアウトは妥当だが、エラー後の復帰が遅い

---

## 🛠️ 修正内容

### 修正 1: Sync Word 探索機能の追加

**新規関数**: `SerialConnection::find_sync_word()`

**実装場所**: `src/serial.rs`

```rust
/// Find sync word (0xCAFEBABE) in the byte stream
///
/// Reads bytes one at a time until the sync word is found.
/// This is used to recover from sync errors.
pub fn find_sync_word(&mut self) -> io::Result<()> {
    let mut buf = [0u8; 4];
    let sync_word = crate::protocol::SYNC_WORD;

    info!("Searching for sync word 0x{:08X}...", sync_word);

    // Initialize buffer with first 4 bytes
    self.read_exact(&mut buf)?;

    let mut bytes_read = 4;
    const MAX_SEARCH_BYTES: usize = 100_000; // 100KB safety limit

    loop {
        // Check if current 4 bytes match sync word
        let current_word = u32::from_le_bytes(buf);
        if current_word == sync_word {
            info!("Sync word found after reading {} bytes", bytes_read);
            return Ok(());
        }

        // Safety check: prevent infinite loop
        if bytes_read >= MAX_SEARCH_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("Sync word not found after {} bytes", MAX_SEARCH_BYTES),
            ));
        }

        // Shift buffer left by 1 byte and read new byte
        buf[0] = buf[1];
        buf[1] = buf[2];
        buf[2] = buf[3];

        let mut new_byte = [0u8; 1];
        self.port.read_exact(&mut new_byte)?;
        buf[3] = new_byte[0];

        bytes_read += 1;
    }
}
```

**特徴**:
- 1バイトずつシフトして sync word を探索
- 最大 100KB まで探索 (安全装置)
- ログ出力で診断可能

### 修正 2: エラー回復処理の実装

**新規関数**: `SerialConnection::read_packet_with_recovery()`

**実装場所**: `src/serial.rs`

```rust
/// Read MJPEG packet with automatic error recovery
///
/// This function wraps read_packet() and adds:
/// - Sync word search on parse errors
/// - JPEG marker validation
/// - Automatic retry on recoverable errors
pub fn read_packet_with_recovery(&mut self) -> io::Result<MjpegPacket> {
    const MAX_RETRIES: usize = 3;
    let mut retry_count = 0;

    loop {
        match self.read_packet() {
            Ok(packet) => {
                // Validate JPEG markers (SOI and EOI)
                if packet.is_valid_jpeg() {
                    return Ok(packet);
                } else {
                    warn!("Invalid JPEG markers detected (no SOI/EOI)");
                    warn!("  First 4 bytes: {:02X?}", &packet.jpeg_data[..4.min(packet.jpeg_data.len())]);
                    warn!("  Last 4 bytes: {:02X?}", &packet.jpeg_data[packet.jpeg_data.len().saturating_sub(4)..]);

                    // Search for sync word and retry
                    warn!("Attempting to resync...");
                    self.find_sync_word()?;
                    retry_count += 1;

                    if retry_count >= MAX_RETRIES {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Too many invalid JPEG packets",
                        ));
                    }
                    continue;
                }
            }
            Err(e) => {
                match e.kind() {
                    // Recoverable errors - try to resync
                    io::ErrorKind::InvalidData => {
                        warn!("Packet parse error: {}", e);
                        warn!("Attempting to resync...");
                        self.find_sync_word()?;
                        retry_count += 1;

                        if retry_count >= MAX_RETRIES {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Failed to recover after {} retries", MAX_RETRIES),
                            ));
                        }
                        continue;
                    }
                    // Non-recoverable errors - propagate
                    _ => return Err(e),
                }
            }
        }
    }
}
```

**特徴**:
- パケット解析エラー時に自動的に sync word を探索
- JPEG マーカー (SOI/EOI) をチェック
- 最大3回までリトライ
- 詳細なログ出力

### 修正 3: GUI 側の変更

**変更箇所**: `src/gui_main.rs:355`

**変更前**:
```rust
let read_result = serial.read_packet();
```

**変更後**:
```rust
let read_result = serial.read_packet_with_recovery();
```

**追加の改善**:
```rust
match read_result {
    Ok(packet) => {
        // 正常処理
    }
    Err(e) => {
        error_count += 1;
        error!("Packet read error (after recovery attempts): {}", e);

        // エラー後、少し待機してから再試行
        std::thread::sleep(Duration::from_millis(10));
        continue;
    }
}
```

---

## 📈 期待される改善効果

### 1. デコードエラーの大幅削減

**Before**:
- デコード失敗率: 43.5%
- 累積エラー数: 114 回 (15分間)

**After (予測)**:
- デコード失敗率: < 1%
- 累積エラー数: < 5 回 (15分間)
- 一時的なノイズによるエラーから自動回復

### 2. 性能の回復

**Before**:
- PC FPS: 14.87 fps (26% 低下)
- Serial 時間: 68.27 ms (42% 悪化)

**After (予測)**:
- PC FPS: 19-20 fps (Phase 3.0 レベルに回復)
- Serial 時間: 48-50 ms (正常値)

### 3. 安定性の向上

**Before**:
- FPS 変動: 0.09 ~ 21.20 fps (不安定)
- GUI が時間経過で停止

**After (予測)**:
- FPS 変動: 18 ~ 21 fps (安定)
- 長時間動作でも安定した画面更新

---

## 🧪 テスト計画

### テスト 1: 基本動作確認 (5分)

**手順**:
1. 修正版をビルド
2. アプリケーション起動
3. 5分間動作させる

**合格基準**:
- ✅ デコードエラーが 5 回以下
- ✅ PC FPS が 19 fps 以上
- ✅ GUI が正常に更新され続ける

### テスト 2: USB ノイズ耐性テスト (10分)

**手順**:
1. アプリケーション起動
2. USB ケーブルを軽く揺らす (意図的にノイズを発生)
3. 10分間動作させる

**合格基準**:
- ✅ 一時的なエラーから自動回復
- ✅ ログに "Sync word found after reading X bytes" が出力される
- ✅ 回復後、正常動作が継続

### テスト 3: 長時間安定性テスト (30分)

**手順**:
1. 修正版で Phase 4.2 テストを再実行
2. 30分間動作させる

**合格基準**:
- ✅ PC FPS: 19.0-20.5 fps
- ✅ エラー数: < 10 回
- ✅ デコード失敗率: < 1%
- ✅ CSV データに異常なし

---

## 📝 次のステップ

### 即時対応 (本日中)

1. **修正実装** (30分)
   - [x] 報告書作成
   - [ ] `serial.rs` に修正を適用
   - [ ] `gui_main.rs` に修正を適用
   - [ ] ビルド確認

2. **基本動作確認** (5分)
   - [ ] テスト 1 実施
   - [ ] デコードエラー発生率確認

3. **USB ノイズ耐性テスト** (10分)
   - [ ] テスト 2 実施
   - [ ] エラー回復ログ確認

### 短期対応 (明日以降)

4. **Phase 4.2 再テスト** (30分)
   - [ ] テスト 3 実施
   - [ ] CSV データ分析
   - [ ] Phase 3.0 との性能比較

5. **Phase 4.3 エラー回復テスト** (15分)
   - [ ] USB 抜き差しテスト
   - [ ] リセットテスト

6. **Phase 4.4 性能プロファイリング** (15分)
   - [ ] 1000 フレーム詳細分析

7. **Phase 4.5 完了報告書作成**

---

## 🔗 関連ドキュメント

- **Phase 4 テストガイド**: `PHASE4_TEST_GUIDE.md`
- **メトリクス測定ガイド**: `METRICS_GUIDE.md`
- **Spresense メトリクスプロトコル**: `SPRESENSE_METRICS_PROTOCOL.md`

---

## 📌 重要な教訓

### 1. パケット同期の重要性

シリアル通信では、**sync word の同期管理**が最重要課題です。

- ❌ 悪い実装: エラー時に何もせず次のループへ
- ✅ 良い実装: エラー時に sync word を探索して同期復帰

### 2. 多層防御の必要性

単一のエラーチェックでは不十分:

1. **CRC チェック** (データ整合性)
2. **JPEG マーカーチェック** (フォーマット検証)
3. **デコード成功確認** (最終検証)

### 3. エラーログの価値

詳細なログ出力により、問題の早期発見が可能:

- デコードエラーの頻発 → 同期ずれの可能性
- デコード時間 0.00 ms の多発 → デコード失敗の証拠

---

**作成者**: Claude Code (Sonnet 4.5)
**作成日**: 2025-12-31
**優先度**: 🔴 Critical
**対応状況**: 🟡 修正実装中
