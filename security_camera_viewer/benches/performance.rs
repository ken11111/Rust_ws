use criterion::{black_box, criterion_group, criterion_main, Criterion};
// use security_camera_viewer::protocol; // プロジェクトの実際のモジュール名に合わせる

/// CRC-16-CCITT計算のベンチマーク
fn benchmark_crc16(c: &mut Criterion) {
    // サンプルデータ（QVGA JPEG相当: 22KB）
    let data_22kb: Vec<u8> = vec![0xFF; 22_000];

    // サンプルデータ（VGA JPEG相当: 64KB）
    let data_64kb: Vec<u8> = vec![0xFF; 64_000];

    c.bench_function("crc16_22kb", |b| {
        b.iter(|| {
            // protocol::calculate_crc16_ccitt(black_box(&data_22kb))
            // 実際の実装に合わせて修正
            black_box(&data_22kb);
        })
    });

    c.bench_function("crc16_64kb", |b| {
        b.iter(|| {
            // protocol::calculate_crc16_ccitt(black_box(&data_64kb))
            black_box(&data_64kb);
        })
    });
}

/// JPEGデコードのベンチマーク（モックデータ）
fn benchmark_jpeg_decode(c: &mut Criterion) {
    // 実際のJPEGデータを使用する場合は、適切なサンプルデータを用意
    c.bench_function("jpeg_decode_mock", |b| {
        b.iter(|| {
            // デコード処理のベンチマーク
            // 実際の実装に合わせて修正
            black_box(());
        })
    });
}

/// シリアル読み込みのベンチマーク（モックデータ）
fn benchmark_serial_read(c: &mut Criterion) {
    let buffer: Vec<u8> = vec![0; 64_000];

    c.bench_function("serial_read_64kb", |b| {
        b.iter(|| {
            // シリアル読み込み処理のベンチマーク
            black_box(&buffer);
        })
    });
}

criterion_group!(
    benches,
    benchmark_crc16,
    benchmark_jpeg_decode,
    benchmark_serial_read
);
criterion_main!(benches);
