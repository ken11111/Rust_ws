/*
 * Spresense JPEG 圧縮診断コード
 * 
 * 追加場所: /home/ken/Spr_ws/GH_wk_test/apps/examples/security_camera/camera_app_main.c
 * camera_app_main() 関数内、JPEG圧縮処理の周辺
 */

// ==================================================
// 1. JPEG圧縮前: RAWデータの検証
// ==================================================

// V4L2バッファからデータ取得後
v4l2_buffer_t *v4l2_buf = &camera_buffer;

printf("[DIAG] Frame %u: RAW data ready, size=%u bytes\n",
       frame_count, v4l2_buf->bytesused);

// バッファの先頭バイトを確認（デバッグ用）
if (v4l2_buf->bytesused > 16) {
    uint8_t *raw = (uint8_t *)v4l2_buf->m.userptr;
    printf("[DIAG] RAW header: %02X %02X %02X %02X %02X %02X %02X %02X\n",
           raw[0], raw[1], raw[2], raw[3],
           raw[4], raw[5], raw[6], raw[7]);
}

// ==================================================
// 2. JPEG圧縮実行と結果検証
// ==================================================

// JPEG圧縮実行 (既存コード)
uint32_t jpeg_size = 0;
uint8_t *jpeg_data = packet_buffer + MJPEG_HEADER_SIZE;
const uint32_t jpeg_max_size = MJPEG_MAX_PACKET_SIZE - MJPEG_HEADER_SIZE - 2;

// 圧縮前にバッファをクリア（問題検出のため）
memset(jpeg_data, 0x00, 100);  // 先頭100バイトのみクリア

int compress_ret = /* JPEG圧縮関数呼び出し */;

printf("[DIAG] JPEG compress return: %d, size=%u bytes\n",
       compress_ret, jpeg_size);

// ==================================================
// 3. JPEG形式の検証（重要！）
// ==================================================

bool jpeg_valid = false;

if (jpeg_size >= 4) {
    // SOI マーカー確認 (0xFF 0xD8)
    bool has_soi = (jpeg_data[0] == 0xFF && jpeg_data[1] == 0xD8);
    
    // EOI マーカー確認 (0xFF 0xD9)
    bool has_eoi = (jpeg_data[jpeg_size-2] == 0xFF && 
                    jpeg_data[jpeg_size-1] == 0xD9);
    
    jpeg_valid = (has_soi && has_eoi);
    
    printf("[DIAG] JPEG markers: SOI=%s, EOI=%s, Valid=%s\n",
           has_soi ? "OK" : "NG",
           has_eoi ? "OK" : "NG",
           jpeg_valid ? "YES" : "NO");
    
    // マーカーの実際の値を表示
    printf("[DIAG] JPEG bytes: [0-3]=%02X %02X %02X %02X, [end-4 to end]=%02X %02X %02X %02X\n",
           jpeg_data[0], jpeg_data[1], jpeg_data[2], jpeg_data[3],
           jpeg_data[jpeg_size-4], jpeg_data[jpeg_size-3],
           jpeg_data[jpeg_size-2], jpeg_data[jpeg_size-1]);
} else {
    printf("[DIAG] ERROR: JPEG size too small: %u bytes\n", jpeg_size);
}

// ==================================================
// 4. エラー時の対応
// ==================================================

if (!jpeg_valid) {
    printf("[ERROR] Frame %u: Invalid JPEG detected!\n", frame_count);
    
    // オプション1: このフレームをスキップ
    // continue; // ループの次のイテレーションへ
    
    // オプション2: エラーカウントを増やして継続
    error_count++;
    
    // オプション3: 前のフレームを再送（実装次第）
    // ...
}

// ==================================================
// 5. サイズ異常の検出
// ==================================================

// 過去のJPEGサイズの移動平均を保持
static uint32_t jpeg_size_history[10] = {0};
static int history_index = 0;

// 現在のサイズを記録
jpeg_size_history[history_index] = jpeg_size;
history_index = (history_index + 1) % 10;

// 移動平均を計算
uint32_t jpeg_size_avg = 0;
int valid_samples = 0;
for (int i = 0; i < 10; i++) {
    if (jpeg_size_history[i] > 0) {
        jpeg_size_avg += jpeg_size_history[i];
        valid_samples++;
    }
}
if (valid_samples > 0) {
    jpeg_size_avg /= valid_samples;
}

// サイズが平均から大きく外れている場合に警告
if (valid_samples >= 5) {
    int32_t size_diff = (int32_t)jpeg_size - (int32_t)jpeg_size_avg;
    float size_ratio = (float)size_diff / (float)jpeg_size_avg;
    
    if (fabsf(size_ratio) > 0.5) {  // 50%以上の変動
        printf("[WARN] Frame %u: Unusual JPEG size=%u (avg=%u, diff=%.1f%%)\n",
               frame_count, jpeg_size, jpeg_size_avg, size_ratio * 100.0f);
    }
}

// ==================================================
// 6. メトリクス出力（デバッグ用）
// ==================================================

// 100フレームごとに統計を出力
if (frame_count % 100 == 0) {
    printf("[METRICS] Frame %u: avg_size=%u, error_count=%u\n",
           frame_count, jpeg_size_avg, error_count);
}

