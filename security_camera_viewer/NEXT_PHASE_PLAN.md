# Phase 4.1.1以降の実装計画

**作成日**: 2025-12-31
**現在のステータス**: Phase 4.1.1完了（静止シーンのみ検証済み）

---

## 📊 現状の課題整理

### ✅ 解決済み
1. JPEG validation機能の実装と検証（静止シーンで100%成功）
2. Error counter separation（パケットエラー vs JPEGエラー）
3. 長期連続稼働の実証（32.5分間、20,179フレーム）
4. USB転送ボトルネックの特定（95ms/frame, 91%占有率）

### ⚠️ 未解決の課題
1. **動的シーンでのJPEG圧縮エラー**: Phase 4.1で0.45%のエラー率を確認
2. **ISX012ハードウェア制約**: 30fps時、動的シーンでJPEG圧縮失敗の可能性
3. **低FPS**: 現在10.4 fps（Phase 1.5 pipelining未適用）

---

## 🎯 次フェーズの目標と優先順位

### Phase 2: Pipelining統合 + 動的シーン検証

**目標**:
1. Phase 1.5のpipelining技術をPhase 4.1.1と統合
2. 動的シーンでの長時間テスト実施
3. ISX012ハードウェア制約の詳細分析

**期待される成果**:
- FPS向上: 10.4 → 35+ fps (+237%)
- JPEG validation機能維持（エラー検出率100%）
- 動的シーンでのエラー率測定と分析

**想定リスク**:
- 動的シーン時、30fps動作でJPEG圧縮エラー発生（Phase 4.1で0.45%確認）
- Pipelining適用によりFPS向上 → エラー率が増加する可能性

---

## 📋 Phase 2 実装計画（詳細）

### Step 1: Phase 1.5 Pipelining コードのレビューと準備

**目的**: Phase 1.5の実装を確認し、Phase 4.1.1と統合可能か検証

**実施項目**:
1. Phase 1.5のコード確認
   - `/home/ken/Spr_ws/GH_wk_test/spresense/examples/multi_webcamera/` を参照
   - Camera thread + USB threadのアーキテクチャ確認
   - Frame queueの実装パターン確認

2. Phase 4.1.1のJPEG validation機能との互換性確認
   - `mjpeg_protocol.c`のJPEG validation機能
   - Pipelining環境でも正常動作するか検証

3. 統合ポイントの特定
   - Camera thread内でのJPEG validation実施箇所
   - Error handling機能の配置箇所

**成果物**:
- 統合設計書（アーキテクチャ図、データフロー図）
- 互換性確認レポート

**所要時間**: 2-3時間

---

### Step 2: Pipelining Infrastructure実装

**目的**: Camera thread + USB threadの基盤を構築

**実施項目**:
1. **新規ファイル作成**:
   - `frame_queue.h/c`: Frame queueの実装
   - `camera_threads.h/c`: Thread関数とsynchronization

2. **Frame Queue設計**:
   ```c
   typedef struct frame_buffer {
       void *data;              // JPEG packet buffer
       uint32_t length;         // Buffer capacity
       uint32_t used;           // Actual JPEG size
       int id;                  // Buffer ID
       struct frame_buffer *next;
   } frame_buffer_t;
   ```
   - Queue depth: 3 buffers（Phase 1.5と同様）
   - Buffer size: MJPEG_MAX_PACKET_SIZE (~98KB)

3. **Synchronization機構**:
   - 1つのmutex（`queue_mutex`）で両方のqueueを保護
   - 1つのcondition variable（`queue_cond`）で双方向シグナリング
   - Priority inheritance有効化（`PTHREAD_PRIO_INHERIT`）

4. **Thread優先度設定**:
   - Camera thread: Priority 110（高優先度）
   - USB thread: Priority 100（低優先度）

**成果物**:
- `frame_queue.h/c`
- `camera_threads.h/c`
- Makefileの更新

**検証方法**:
- コンパイル成功確認
- Thread作成・終了の正常動作確認

**所要時間**: 3-4時間

---

### Step 3: Camera Thread実装

**目的**: カメラキャプチャとJPEG validationをCamera threadに移行

**実施項目**:
1. **Camera threadの処理フロー**:
   ```c
   while (!shutdown_requested) {
       // 1. カメラからフレーム取得（mutex外）
       ret = camera_get_frame(&frame);

       // 2. JPEG validation（mutex外）
       ret = mjpeg_validate_jpeg_data(frame.data, frame.size, &actual_size);
       if (ret < 0) {
           // エラーカウント、ログ出力
           jpeg_validation_error_count++;
           continue;  // 不正フレームをスキップ
       }

       // 3. MJPEG packet作成（mutex外）
       packet_size = mjpeg_pack_frame(frame.data, actual_size, ...);

       // 4. Action queueにenqueue（mutex内）
       pthread_mutex_lock(&queue_mutex);
       while (queue_depth >= MAX_DEPTH && !shutdown) {
           pthread_cond_wait(&queue_cond, &queue_mutex);
       }
       push_action_queue(packet);
       pthread_cond_signal(&queue_cond);  // USB threadを起動
       pthread_mutex_unlock(&queue_mutex);

       // 5. Empty queueからbufferを取得してQBUF（mutex内外）
       recycle_empty_buffers();
   }
   ```

2. **エラーハンドリング統合**:
   - JPEG validation エラー → `jpeg_validation_error_count++`
   - Camera タイムアウト → 3回連続失敗で`shutdown_requested = true`
   - ログ出力: エラー詳細情報

3. **統計情報収集**:
   - 処理時間測定（camera capture, JPEG validation, pack）
   - Queue depth監視
   - 30フレームごとに統計ログ出力

**成果物**:
- `camera_threads.c`のcamera_thread_func()実装
- `camera_app_main.c`の修正（camera threadへの移行）

**検証方法**:
- Camera threadが正常にフレームをenqueueすること
- JPEG validationエラーが正しくカウントされること
- Queue depthが適切に制御されること

**所要時間**: 3-4時間

---

### Step 4: USB Thread実装

**目的**: USB転送をUSB threadに移行し、完全なpipelineを構築

**実施項目**:
1. **USB threadの処理フロー**:
   ```c
   while (!shutdown_requested) {
       // 1. Action queueから packet取得（mutex内）
       pthread_mutex_lock(&queue_mutex);
       while (action_queue_empty() && !shutdown) {
           pthread_cond_wait(&queue_cond, &queue_mutex);
       }
       packet = pull_action_queue();
       pthread_mutex_unlock(&queue_mutex);

       // 2. USB転送（mutex外）
       ret = usb_transport_send_bytes(packet->data, packet->size);

       // 3. エラーハンドリングとbuffer recycling（mutex内）
       pthread_mutex_lock(&queue_mutex);
       if (ret < 0) {
           usb_error_count++;
           if (usb_error_count >= 10) {
               shutdown_requested = true;  // USB接続切断
           }
       }
       push_empty_queue(packet);  // Buffer再利用
       pthread_cond_signal(&queue_cond);  // Camera threadを起動
       pthread_mutex_unlock(&queue_mutex);
   }
   ```

2. **エラーハンドリング**:
   - USB書き込みエラー → `usb_error_count++`
   - 10回連続エラー → `shutdown_requested = true`
   - ログ出力: USB転送エラー詳細

3. **Main threadの役割変更**:
   - Thread作成・終了管理のみ
   - SIGINT handlerでshutdown通知
   - Statisticsログ出力（periodic）

**成果物**:
- `camera_threads.c`のusb_thread_func()実装
- `camera_app_main.c`のmain thread簡素化

**検証方法**:
- USB threadが正常にパケットを送信すること
- Queue depthが適切にバランスすること
- FPSが向上すること（目標: ≥12.0 fps）

**所要時間**: 2-3時間

---

### Step 5: Error Handling強化

**目的**: 異常系の処理を完全にする

**実施項目**:
1. **SIGINT handler実装**:
   ```c
   void sigint_handler(int sig) {
       shutdown_requested = true;
       pthread_cond_broadcast(&queue_cond);  // 全threadを起動
   }
   ```

2. **Clean shutdown処理**:
   - Thread終了待ち（`pthread_join`、タイムアウト2秒）
   - Queue内のbufferを全て解放
   - Mutex/condition variable破棄
   - カメラデバイスクローズ

3. **エラーログの充実**:
   - JPEG validation error: シーケンス番号、サイズ、マーカー情報
   - USB error: errno、転送サイズ
   - Thread error: スレッド名、エラーコード

**成果物**:
- 完全なエラーハンドリング実装
- Clean shutdownの動作確認

**検証方法**:
- Ctrl+C押下 → 2秒以内にクリーン終了
- USB切断 → エラーログ出力後、クリーン終了
- メモリリーク無し確認

**所要時間**: 2-3時間

---

### Step 6: 動的シーンテスト実施

**目的**: 動的シーンでのJPEG圧縮エラー率を測定し、ハードウェア制約を分析

**テスト項目**:

#### Test 1: 静止シーン（ベースライン）
- **期間**: 5分間
- **シーン**: カメラ前に静止物体のみ
- **期待結果**: エラー率 0.00%
- **測定項目**: FPS, JPEG validation error count, USB error count

#### Test 2: 低動的シーン
- **期間**: 5分間
- **シーン**: 手をゆっくり動かす（動き少ない）
- **期待結果**: エラー率 < 0.1%
- **測定項目**: JPEG size変動、FPS変動、error rate

#### Test 3: 中動的シーン
- **期間**: 5分間
- **シーン**: 手を素早く動かす、物体を移動
- **期待結果**: エラー率 0.1-0.5%
- **測定項目**: JPEG size変動、FPS変動、error rate、consecutive errors

#### Test 4: 高動的シーン（ストレステスト）
- **期間**: 10分間
- **シーン**: カメラを動かす、複数物体を高速移動
- **期待結果**: エラー率測定（Phase 4.1で0.45%を参考）
- **測定項目**: 全項目 + hardware encoder status

#### Test 5: 長時間動的シーン
- **期間**: 30分間
- **シーン**: 中〜高動的シーンを継続
- **期待結果**: エラー率の時間推移を分析
- **測定項目**: 全項目 + 時系列データ

**分析項目**:
1. JPEG圧縮エラー率とシーン動的度の相関
2. FPS向上によるエラー率への影響
3. 30fps制約の詳細（どのFPSからエラーが増加するか）
4. ISX012ハードウェアエンコーダーの処理時間限界

**成果物**:
- 動的シーンテスト結果レポート
- JPEG圧縮エラー率の分析
- ISX012ハードウェア制約の詳細分析

**所要時間**: 3-4時間（テスト実施 + 分析）

---

### Step 7: パフォーマンス最適化とチューニング

**目的**: FPSを最大化し、Queue depthを最適化

**実施項目**:
1. **Priority調整**:
   - Camera thread: 110 → 調整
   - USB thread: 100 → 調整
   - 最適な優先度差を実験的に決定

2. **Queue depth調整**:
   - 現在: 3 buffers
   - テスト: 2, 3, 4 buffersで比較
   - 最適なdepthを決定（FPS vs メモリ使用量）

3. **統計情報の充実**:
   - Frame interval統計
   - Queue depth分布
   - 各処理段階の時間測定

4. **ログ出力の最適化**:
   - 通常動作時: 30フレームごとに統計ログ
   - エラー発生時: 詳細ログ
   - デバッグビルド: 全フレームログ（オプション）

**成果物**:
- 最適化されたパラメータ
- パフォーマンス測定レポート

**検証方法**:
- 目標FPS達成: ≥12.0 fps（Phase 3達成）、理想35+ fps（Phase 1.5レベル）
- Queue depth variance分析
- Frame interval標準偏差測定

**所要時間**: 2-3時間

---

## 📊 Phase 2 完了基準

### 必須条件（Phase 3移行の最低基準）
- ✅ FPS ≥ 12.0 fps（Phase 1.5未満でも可、ただし現状10.4より向上）
- ✅ JPEG validation機能が正常動作（静止シーンで100%成功）
- ✅ 動的シーンテスト実施完了（エラー率測定完了）
- ✅ Zero dropped frames（sequence番号連続）
- ✅ Clean shutdown動作確認（Ctrl+C, USB切断）

### 理想条件（Phase 1.5レベル達成）
- ⭐ FPS ≥ 35 fps（Phase 1.5の37.3 fpsに近い性能）
- ⭐ 動的シーンエラー率 < 0.5%
- ⭐ Queue depth variance: 0-3の範囲で安定
- ⭐ Frame interval標準偏差 < 5ms

---

## 🔧 動的シーンエラー対策（Phase 2後に検討）

### Option A: JPEG品質パラメータ調整（優先度: 高）
**内容**: ISX012のJPEG品質を下げて圧縮時間を短縮
- Quality: 80 → 70 or 60
- 期待効果: 圧縮時間短縮 → エラー率低減
- リスク: 画質低下

**実施タイミング**: Phase 2動的シーンテスト後、エラー率が0.5%以上の場合

---

### Option B: FPS制限（優先度: 中）
**内容**: 動的シーン検出時、FPSを制限して圧縮時間を確保
- 動的シーン検出: JPEG size変動率で判定
- FPS制限: 30 fps → 25 fps or 20 fps
- 期待効果: 圧縮処理時間 +20-50% → エラー率低減

**実施タイミング**: Option A実施後も改善不十分な場合

---

### Option C: 適応型JPEG品質（優先度: 低）
**内容**: シーンの動的度に応じてJPEG品質を動的に調整
- 静止シーン: Quality 80（高品質）
- 動的シーン: Quality 60（低品質、高速）
- 判定: 前フレームとの差分、JPEG size変動

**実施タイミング**: Phase 3以降（高度な最適化）

---

### Option D: Phase 4.2 エラー回復機能統合（優先度: 低）
**内容**: Phase 4.2の再送リクエスト機能を統合
- PC側から再送リクエスト
- Spresense側がフレームを再キャプチャ

**実施タイミング**: エラー率が1%以上の場合のみ検討

---

## 📅 Phase 2 スケジュール（推定）

| Step | 実施項目 | 所要時間 | 累計時間 |
|------|---------|---------|---------|
| 1 | Phase 1.5レビューと準備 | 2-3h | 2-3h |
| 2 | Pipelining infrastructure実装 | 3-4h | 5-7h |
| 3 | Camera thread実装 | 3-4h | 8-11h |
| 4 | USB thread実装 | 2-3h | 10-14h |
| 5 | Error handling強化 | 2-3h | 12-17h |
| 6 | 動的シーンテスト実施 | 3-4h | 15-21h |
| 7 | パフォーマンス最適化 | 2-3h | 17-24h |

**総所要時間**: 17-24時間（2-3日の作業セッション）

---

## 🎯 Phase 2後の方針

### ケース1: 動的シーンエラー率 < 0.5%
→ **Phase 3へ進む**（パフォーマンス最適化、機能追加）

### ケース2: 動的シーンエラー率 0.5-1.0%
→ **Option A（JPEG品質調整）を実施** → 改善確認 → Phase 3へ

### ケース3: 動的シーンエラー率 > 1.0%
→ **Option A + Option B実施** → 改善確認 → 必要ならOption C検討

---

## 📝 補足: ISX012ハードウェア制約について

### 現在の理解
- ISX012ハードウェアJPEGエンコーダーは固定処理時間予算を持つ
- 30fps時: ~33ms/frame
- 動的シーン: 圧縮効率低下 → 処理時間増加 → 予算超過の可能性
- 予算超過時: 不正なJPEGデータ生成（SOI/EOIマーカー欠落）

### Phase 2での検証項目
1. どのFPSからエラーが発生し始めるか（20fps, 25fps, 30fps）
2. JPEG sizeとエラー率の相関（大きいJPEG = エラー多い？）
3. シーン動的度とエラー率の相関
4. 連続エラー発生パターン（burst errors or random）

### 最終的な目標
- 静止シーン: 35+ fps, エラー率 0%
- 低〜中動的シーン: 30+ fps, エラー率 < 0.5%
- 高動的シーン: 20-25 fps（FPS制限）, エラー率 < 0.5%

---

**Document Version**: 1.0
**Author**: Claude Code (Sonnet 4.5)
**Date**: 2025-12-31
**Status**: Phase 2実装待ち
