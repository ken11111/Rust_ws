# Phase 2 実装クイックガイド

**現状**: Phase 4.1.1完了（静止シーンのみ、FPS 10.4）
**目標**: Pipelining統合 + 動的シーン検証（FPS 35+、エラー率測定）

---

## 📋 実装ステップ（7ステップ）

### Step 1: Phase 1.5レビュー（2-3h）
```bash
# 参照ファイル
/home/ken/Spr_ws/GH_wk_test/spresense/examples/multi_webcamera/multiwebcam_threads.c
/home/ken/Spr_ws/GH_wk_test/spresense/examples/multi_webcamera/multiwebcam_util.c
```
- Camera thread + USB threadアーキテクチャ確認
- JPEG validation機能との互換性確認
- 統合設計書作成

---

### Step 2: Infrastructure実装（3-4h）
**新規ファイル**:
- `frame_queue.h/c`: Queue実装
- `camera_threads.h/c`: Thread関数

**Queue設計**:
- Depth: 3 buffers
- Size: ~98KB/buffer
- Mutex: 1個（両queueを保護）
- Condition variable: 1個（双方向シグナリング）

**Thread優先度**:
- Camera: 110
- USB: 100

---

### Step 3: Camera Thread実装（3-4h）
**処理フロー**:
```c
while (!shutdown) {
    camera_get_frame(&frame);              // mutex外
    mjpeg_validate_jpeg_data(...);         // mutex外 ← JPEG validation
    mjpeg_pack_frame(...);                 // mutex外

    pthread_mutex_lock(&queue_mutex);
    push_action_queue(packet);
    pthread_cond_signal(&queue_cond);
    pthread_mutex_unlock(&queue_mutex);

    recycle_empty_buffers();
}
```

**エラーハンドリング**:
- JPEG validation error → カウント、スキップ
- Camera timeout → 3回連続で終了

---

### Step 4: USB Thread実装（2-3h）
**処理フロー**:
```c
while (!shutdown) {
    pthread_mutex_lock(&queue_mutex);
    while (action_queue_empty() && !shutdown) {
        pthread_cond_wait(&queue_cond, &queue_mutex);
    }
    packet = pull_action_queue();
    pthread_mutex_unlock(&queue_mutex);

    usb_transport_send_bytes(...);         // mutex外

    pthread_mutex_lock(&queue_mutex);
    if (ret < 0) usb_error_count++;
    push_empty_queue(packet);
    pthread_cond_signal(&queue_cond);
    pthread_mutex_unlock(&queue_mutex);
}
```

**検証基準**:
- FPS ≥ 12.0（必須）、理想35+
- Queue depth: 0-3で安定

---

### Step 5: Error Handling強化（2-3h）
**実装項目**:
- SIGINT handler → `shutdown_requested = true`
- Clean shutdown: thread join、queue解放、mutex破棄
- エラーログ充実

**検証**:
- Ctrl+C → 2秒以内に終了
- USB切断 → エラーログ出力後終了

---

### Step 6: 動的シーンテスト（3-4h）★重要★
**テストケース**:

| Test | シーン | 期間 | 期待エラー率 |
|------|--------|------|-------------|
| 1 | 静止 | 5分 | 0.00% |
| 2 | 低動的 | 5分 | < 0.1% |
| 3 | 中動的 | 5分 | 0.1-0.5% |
| 4 | 高動的 | 10分 | 測定（Phase 4.1で0.45%） |
| 5 | 長時間動的 | 30分 | 時系列分析 |

**分析項目**:
- ✅ JPEG圧縮エラー率 vs シーン動的度
- ✅ FPS vs エラー率の相関
- ✅ 30fps制約の詳細（何fpsからエラー増加？）
- ✅ ISX012処理時間限界の特定

---

### Step 7: パフォーマンス最適化（2-3h）
**調整項目**:
- Thread優先度（110/100 → 調整）
- Queue depth（2, 3, 4で比較）
- ログ出力頻度

**目標**:
- FPS: 35+ fps
- Queue depth variance: 安定
- Frame interval標準偏差: < 5ms

---

## ✅ Phase 2 完了基準

### 必須条件
- [ ] FPS ≥ 12.0 fps
- [ ] JPEG validation正常動作
- [ ] 動的シーンテスト完了（エラー率測定）
- [ ] Zero dropped frames
- [ ] Clean shutdown動作確認

### 理想条件
- [ ] FPS ≥ 35 fps
- [ ] 動的シーンエラー率 < 0.5%
- [ ] Queue depth安定（0-3）

---

## 🔧 動的シーンエラー対策（Phase 2後）

### エラー率 < 0.5%
→ **Phase 3へ進む**

### エラー率 0.5-1.0%
→ **Option A**: JPEG品質調整（80 → 70 or 60）

### エラー率 > 1.0%
→ **Option A + Option B**: JPEG品質調整 + FPS制限（30 → 25 or 20）

---

## 📅 スケジュール

| Step | 所要時間 | 累計 |
|------|---------|------|
| 1. レビュー | 2-3h | 2-3h |
| 2. Infrastructure | 3-4h | 5-7h |
| 3. Camera thread | 3-4h | 8-11h |
| 4. USB thread | 2-3h | 10-14h |
| 5. Error handling | 2-3h | 12-17h |
| 6. 動的シーンテスト | 3-4h | 15-21h |
| 7. 最適化 | 2-3h | 17-24h |

**総所要時間**: 17-24時間（2-3日）

---

## 🎯 ISX012ハードウェア制約の検証

### 現在の理解
- 30fps時: ~33ms/frame処理時間予算
- 動的シーン: 圧縮効率低下 → 処理時間増加 → 予算超過 → エラー

### Phase 2で明らかにする項目
1. どのFPSからエラー発生？（20, 25, 30fps）
2. JPEG size vs エラー率の相関
3. シーン動的度 vs エラー率の相関
4. 連続エラーパターン（burst or random）

### 最終目標
- 静止シーン: 35+ fps, 0% error
- 低〜中動的: 30+ fps, < 0.5% error
- 高動的: 20-25 fps（制限）, < 0.5% error

---

**詳細**: NEXT_PHASE_PLAN.md参照
**Plan from**: ~/.claude/plans/iterative-beaming-marble.md
