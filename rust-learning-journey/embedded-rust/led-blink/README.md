# LED点滅プログラム (Embedded Rust)

## 🎯 目的
組込みRustの基礎を学ぶため、最もシンプルなLED点滅プログラムを実装

## 🔧 ターゲットハードウェア

### Option 1: STM32 (推奨)
- ボード: STM32F103C8T6 (Blue Pill)
- 理由: 安価、入手容易、ドキュメント豊富

### Option 2: ESP32
- ボード: ESP32-DevKitC
- 理由: WiFi/Bluetooth内蔵、Rust対応進んでいる

### Option 3: Raspberry Pi Pico (RP2040)
- ボード: Raspberry Pi Pico
- 理由: 安価、USB接続簡単、Rust対応良好

## 📋 実装計画

### Phase 1: 環境構築 ⏳未着手
- [ ] ターゲットボード入手
- [ ] Rust embedded toolchain インストール
- [ ] デバッガ設定 (ST-Link / JTAG / USB)

### Phase 2: 基本実装 ⏳未着手
- [ ] `no_std` プロジェクト作成
- [ ] HAL (Hardware Abstraction Layer) 設定
- [ ] GPIO制御実装
- [ ] LED点滅 (1秒間隔)

### Phase 3: 応用 ⏳未着手
- [ ] タイマー割り込み実装
- [ ] PWM制御 (LED明るさ調整)
- [ ] ボタン入力 + LED制御

## 🛠️ 環境構築手順

### 1. Rust embedded toolchain インストール
```bash
# ARM Cortex-M向けターゲット追加
rustup target add thumbv7m-none-eabi     # Cortex-M3
rustup target add thumbv7em-none-eabi    # Cortex-M4/M7 (FPUなし)
rustup target add thumbv7em-none-eabihf  # Cortex-M4F/M7F (FPUあり)

# cargo-embed (フラッシュツール)
cargo install cargo-embed

# cargo-binutils (バイナリ解析ツール)
cargo install cargo-binutils
rustup component add llvm-tools-preview
```

### 2. プロジェクト作成
```bash
cargo new led-blink --bin
cd led-blink
```

### 3. Cargo.toml設定
```toml
[dependencies]
cortex-m = "0.7"
cortex-m-rt = "0.7"
panic-halt = "0.2"

# STM32の場合
stm32f1xx-hal = { version = "0.10", features = ["stm32f103", "rt"] }

[profile.release]
opt-level = "z"     # サイズ最適化
lto = true          # Link Time Optimization
codegen-units = 1   # 最適化優先
```

## 📝 実装コード (予定)

```rust
#![no_std]
#![no_main]

use panic_halt as _;
use cortex_m_rt::entry;
use stm32f1xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    // ペリフェラル取得
    let dp = pac::Peripherals::take().unwrap();

    // クロック設定
    let mut rcc = dp.RCC.constrain();
    let mut flash = dp.FLASH.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // GPIO設定
    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    // 遅延設定
    let mut delay = cortex_m::delay::Delay::new(
        core.SYST,
        clocks.sysclk().0
    );

    loop {
        led.set_high();
        delay.delay_ms(1000_u32);

        led.set_low();
        delay.delay_ms(1000_u32);
    }
}
```

## 🎓 学習ポイント

- [ ] `no_std` 環境の理解
- [ ] `#![no_main]` の意味
- [ ] `entry` マクロの役割
- [ ] HAL (Hardware Abstraction Layer) の使い方
- [ ] GPIO制御の基礎
- [ ] 遅延処理の実装

## 📚 参考リソース

- [Embedded Rust Book](https://docs.rust-embedded.org/book/)
- [STM32F1xx HAL Documentation](https://docs.rs/stm32f1xx-hal/)
- [Awesome Embedded Rust](https://github.com/rust-embedded/awesome-embedded-rust)

---

**作成日**: 2026-03-15
**開始予定**: -
**完了予定**: -
