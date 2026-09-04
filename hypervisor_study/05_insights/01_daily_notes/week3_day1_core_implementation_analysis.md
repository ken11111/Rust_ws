# Week 3 Day 1: Type-1コア機能の実装解析と動作確認

## 📅 日付: 2026-01-27

## 🎯 学習目標
- [ ] MiniVisorのベアメタルブートシーケンス詳細解析
- [ ] ARM EL2仮想化実装の動作確認
- [ ] 物理メモリ管理機構の実装理解
- [ ] 基本性能測定基盤の設計・実装

## 🚀 **ベアメタルブートシーケンスの詳細解析**

### **1. ブートプロセスの全体フロー確認**

まずMiniVisorのブートプロセスを実際のコードで確認しましょう。

### **2. MiniVisorのエントリポイント解析**

<details>
<summary>MiniVisor main.rs の詳細確認</summary>

```bash
# MiniVisorのメイン関数の実装を確認
cd /home/ken/Rust_ws/hypervisor_study/MiniVisor
cat src/main.rs
```

重要なポイント：
- `#![no_std]`, `#![no_main]` の意味
- カスタムエントリポイントの実装
- EL2での直接実行確認
- ハードウェア初期化シーケンス

</details>

### **3. ファームウェアからの制御移行**

#### **QEMUでのブート確認**
```bash
# MiniVisorの実際のブートプロセスを観察
cd /home/ken/Rust_ws/hypervisor_study/MiniVisor

# デバッグ情報付きでビルド
cargo build --release

# QEMUで実行してブートログを確認
cargo run --release 2>&1 | tee boot_log.txt
```

#### **期待される出力の解析**
```
Expected Boot Sequence:
1. QEMU BIOS/UEFI initialization
2. MiniVisor ELF loading
3. EL2 entry confirmation
4. Memory setup
5. Exception vector setup
6. GIC initialization
7. Guest VM boot
```

## 🔧 **ARM EL2仮想化実装の動作確認**

### **HCR_EL2設定の実装確認**

<details>
<summary>仮想化レジスタ設定の動作確認</summary>

MiniVisorでのHCR_EL2設定を実際に確認：

```rust
// src/asm.rs または registers.rs での実装確認
pub unsafe fn setup_virtualization() {
    // HCR_EL2の設定値確認
    let hcr_val = HCR_EL2_VM |      // 仮想化有効
                  HCR_EL2_RW |      // AArch64ゲスト
                  HCR_EL2_IMO |     // IRQ routing to EL2
                  HCR_EL2_FMO |     // FIQ routing to EL2
                  HCR_EL2_AMO;      // SError routing to EL2

    set_hcr_el2(hcr_val);
}
```

</details>

### **VMEntry/VMExit メカニズムの動作確認**

#### **ERET命令によるVMEntry確認**
```rust
// vm.rs でのゲスト起動確認
pub fn boot_vm(entry_point: usize, argument: usize) -> ! {
    unsafe {
        // ゲスト実行状態の設定
        asm::set_spsr_el2(SPSR_EL2_M_EL1H);  // EL1h mode
        asm::set_elr_el2(entry_point as u64); // エントリポイント

        // VMEntry実行
        asm::eret(argument as u64, 0, 0, 0);  // EL2 → EL1
    }
}
```

### **実際の動作テスト**

#### **仮想化動作の確認手順**
```bash
# 1. MiniVisorの動作確認
cd /home/ken/Rust_ws/hypervisor_study/MiniVisor
cargo run --release -- --guest-image /path/to/guest.bin

# 2. ログから仮想化動作を確認
# - Current EL: 2 (EL2での実行確認)
# - HCR_EL2 setup (仮想化設定確認)
# - VM boot (VMEntry成功確認)
# - Exception handling (VMExit処理確認)
```

## 🧠 **物理メモリ管理実装の詳細解析**

### **メモリレイアウト設定の確認**

<details>
<summary>MiniVisorのメモリ管理実装</summary>

```bash
# メモリ管理関連のソースコード確認
find /home/ken/Rust_ws/hypervisor_study/MiniVisor/src -name "*.rs" | xargs grep -l "memory\|Memory"

# 期待されるファイル:
# - memory.rs または mm.rs: メモリアロケータ
# - paging.rs: ページテーブル管理
# - vm.rs: VM用メモリ設定
```

</details>

### **Stage-2ページングの実装確認**

#### **VTTBR_EL2/VTCR_EL2設定確認**
```rust
// Stage-2ページテーブルの設定
unsafe fn setup_stage2_paging() {
    // VTCR_EL2の設定
    let vtcr_val = VTCR_EL2_RES1 |              // Reserved bits
                   (0b011 << 16) |               // 40-bit PA space
                   (0b01 << 14) |                // 64KB granule
                   (0b11 << 12) |                // Inner shareable
                   (0b01 << 10) |                // Write-back cacheable
                   (0b01 << 8) |                 // Write-back cacheable
                   (24 << 0);                    // T0SZ = 24 (40bit)

    set_vtcr_el2(vtcr_val);

    // VTTBR_EL2の設定
    let page_table_addr = allocate_page_table();
    let vttbr_val = (page_table_addr & VTTBR_BADDR) |
                    (0u64 << 48);  // VMID = 0

    set_vttbr_el2(vttbr_val);
}
```

### **メモリ分離の実装確認**

#### **ゲスト用メモリ領域設定**
```rust
// ゲストメモリの分離実装
struct GuestMemoryLayout {
    guest_physical_base: usize,  // ゲスト物理アドレス空間
    host_physical_base: usize,   // 実際の物理アドレス
    size: usize,                 // メモリサイズ
    permissions: MemoryPermissions,
}

impl GuestMemoryLayout {
    fn setup_memory_isolation(&self) {
        // Stage-2ページテーブルでのマッピング
        for page in 0..(self.size / PAGE_SIZE) {
            let guest_pa = self.guest_physical_base + (page * PAGE_SIZE);
            let host_pa = self.host_physical_base + (page * PAGE_SIZE);

            map_stage2_page(guest_pa, host_pa, self.permissions);
        }
    }
}
```

## 📊 **基本性能測定基盤の実装**

### **性能カウンター設計**

#### **MiniVisor用性能メトリクス定義**
```rust
// 基本的な性能測定構造体
pub struct MiniVisorPerformanceCounters {
    // VMExit/VMEntry測定
    vmexit_count: AtomicU64,
    vmexit_total_time: AtomicU64,    // nanoseconds
    vmentry_count: AtomicU64,

    // メモリ操作測定
    page_fault_count: AtomicU64,
    page_fault_total_time: AtomicU64,

    // 割り込み処理測定
    interrupt_count: AtomicU64,
    interrupt_total_time: AtomicU64,

    // 例外処理測定
    exception_counts: [AtomicU64; 16], // 例外タイプ別

    // タイムスタンプ取得用
    start_time: Instant,
}

impl MiniVisorPerformanceCounters {
    pub fn new() -> Self {
        Self {
            vmexit_count: AtomicU64::new(0),
            vmexit_total_time: AtomicU64::new(0),
            vmentry_count: AtomicU64::new(0),
            page_fault_count: AtomicU64::new(0),
            page_fault_total_time: AtomicU64::new(0),
            interrupt_count: AtomicU64::new(0),
            interrupt_total_time: AtomicU64::new(0),
            exception_counts: [const { AtomicU64::new(0) }; 16],
            start_time: Instant::now(),
        }
    }

    pub fn record_vmexit(&self, start_time: Instant) {
        let duration = start_time.elapsed().as_nanos() as u64;
        self.vmexit_count.fetch_add(1, Ordering::Relaxed);
        self.vmexit_total_time.fetch_add(duration, Ordering::Relaxed);
    }

    pub fn get_average_vmexit_time(&self) -> f64 {
        let total_time = self.vmexit_total_time.load(Ordering::Relaxed);
        let count = self.vmexit_count.load(Ordering::Relaxed);

        if count > 0 {
            total_time as f64 / count as f64
        } else {
            0.0
        }
    }
}
```

### **タイムスタンプ取得の実装**

#### **高精度時刻測定**
```rust
// ARM Generic Timer の使用
pub fn get_timestamp() -> u64 {
    let mut count: u64;
    unsafe {
        asm!("mrs {}, cntpct_el0", out(reg) count);
    }
    count
}

pub fn get_timer_frequency() -> u64 {
    let mut freq: u64;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) freq);
    }
    freq
}

pub fn timestamp_to_nanoseconds(timestamp: u64) -> u64 {
    let freq = get_timer_frequency();
    (timestamp * 1_000_000_000) / freq
}
```

### **実際の測定実装例**

#### **VMExit遅延測定**
```rust
// 例外ハンドラーでの測定
#[no_mangle]
pub extern "C" fn handle_sync_exception() {
    let start_timestamp = get_timestamp();

    // 実際の例外処理
    handle_guest_exception();

    let end_timestamp = get_timestamp();
    let duration_ns = timestamp_to_nanoseconds(end_timestamp - start_timestamp);

    // 性能カウンターに記録
    PERFORMANCE_COUNTERS.record_vmexit_duration(duration_ns);
}
```

### **測定結果出力機能**

#### **性能レポート生成**
```rust
impl MiniVisorPerformanceCounters {
    pub fn generate_performance_report(&self) -> PerformanceReport {
        let uptime = self.start_time.elapsed();
        let vmexit_count = self.vmexit_count.load(Ordering::Relaxed);
        let avg_vmexit_time = self.get_average_vmexit_time();

        PerformanceReport {
            uptime_seconds: uptime.as_secs(),
            vmexit_frequency: vmexit_count as f64 / uptime.as_secs() as f64,
            average_vmexit_latency_ns: avg_vmexit_time,
            page_fault_rate: self.get_page_fault_rate(),
            interrupt_rate: self.get_interrupt_rate(),
            overall_efficiency: self.calculate_efficiency(),
        }
    }

    pub fn print_performance_summary(&self) {
        let report = self.generate_performance_report();

        println!("=== MiniVisor Performance Summary ===");
        println!("Uptime: {} seconds", report.uptime_seconds);
        println!("VMExit Frequency: {:.2} exits/sec", report.vmexit_frequency);
        println!("Average VMExit Latency: {:.0} ns", report.average_vmexit_latency_ns);
        println!("Page Fault Rate: {:.2} faults/sec", report.page_fault_rate);
        println!("Interrupt Rate: {:.2} interrupts/sec", report.interrupt_rate);
        println!("Overall Efficiency: {:.2}%", report.overall_efficiency * 100.0);
    }
}
```

## 🧪 **実践的な動作確認プラン**

### **Day 1: 基本動作確認**
```bash
# 1. MiniVisorのビルドと基本動作確認
cd /home/ken/Rust_ws/hypervisor_study/MiniVisor
cargo build --release

# 2. ブートプロセス詳細ログ取得
cargo run --release 2>&1 | tee detailed_boot_log.txt

# 3. 仮想化機能の動作確認
grep -E "(Current EL|HCR_EL2|VM boot)" detailed_boot_log.txt
```

### **Day 2: メモリ管理確認**
```bash
# 1. メモリ関連コードの詳細確認
find src -name "*.rs" -exec grep -l "memory\|Memory\|paging" {} \;

# 2. Stage-2ページング設定の確認
grep -A 10 -B 5 "VTCR\|VTTBR" src/*.rs

# 3. メモリレイアウトの理解
grep -A 20 "memory_layout\|MemoryLayout" src/*.rs
```

### **Day 3: 性能測定実装**
```bash
# 1. 性能カウンター実装の追加
# 新しいファイル: src/performance.rs を作成

# 2. 既存コードへの測定コード挿入
# exception.rs にVMExit測定追加
# vm.rs にVMEntry測定追加

# 3. 測定結果の確認
# 定期的な性能レポート出力の実装
```

## 🎯 **期待される学習成果**

### **技術的理解の深化**
- [x] ベアメタル実行の実際の動作確認
- [ ] ARM仮想化拡張の実装動作理解
- [ ] 物理メモリ管理の実装詳細把握
- [ ] 性能測定手法の実践的習得

### **実践的スキルの獲得**
- [ ] ハイパーバイザーコードの読解能力
- [ ] 性能測定基盤の設計・実装能力
- [ ] 仮想化性能の分析・評価能力
- [ ] 低レベル実装の最適化理解

### **商用製品理解への橋渡し**
- [ ] MiniVisorと商用製品の機能ギャップ理解
- [ ] エンタープライズ要件の技術的実現方法理解
- [ ] 性能最適化の具体的手法理解

## 📚 **次の学習ステップ**

### **Week 3 Day 2-3**
1. **実装コード詳細解析**: 各コンポーネントの実装理解
2. **性能測定実装**: 実際の測定コード追加
3. **動作確認とデバッグ**: 実装した機能の動作検証

### **Week 3 Day 4-5**
1. **性能ベンチマーク実施**: 各種ワークロードでの測定
2. **結果分析と最適化**: ボトルネック特定と改善
3. **商用製品との比較**: 性能差の定量的理解

Week 2での理論基盤 + Week 3での実装理解により、実践的なType-1ハイパーバイザー技術が習得できます！