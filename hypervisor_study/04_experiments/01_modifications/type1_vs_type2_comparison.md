# Type-1 vs Type-2 ハイパーバイザー実装比較

## 🔍 実装レベルでの根本的違い

### **Type-1ハイパーバイザー（MiniVisor）**

#### **1. ビルド設定の違い**
```toml
# MiniVisor/.cargo/config.toml
[build]
target = "aarch64-unknown-none-softfloat"  # OS無し環境
rustflags = ["-C", "link-arg=-Tscripts/qemu.ld"]  # 専用リンカ

[target.aarch64-unknown-none-softfloat]
runner="tools/run_qemu.sh"  # 直接QEMU実行
```

#### **2. メイン関数の違い**
```rust
// MiniVisor/src/main.rs
#![no_std]   // 標準ライブラリ不使用
#![no_main]  // 通常のmain不使用

// カスタムエントリポイント
fn main(argc: usize, argv: *const *const u8) -> usize {
    // 直接EL2で実行開始
    let current_el = asm::get_currentel() >> 2;
    assert_eq!(current_el, 2);  // EL2強制確認

    // 物理ハードウェア直接制御
    setup_memory(&dtb, dtb_address, elf_address, stack_pointer);
    exception::setup_exception();
    init_gic_distributor(&dtb);
}
```

#### **3. メモリ管理の違い**
```rust
// 物理メモリの直接管理
pub struct MemoryAllocator {
    free_memory_list: LinkedList<FreeMemory>,
    memory_total_size: usize,
    memory_allocated_size: usize,
}

// 物理アドレスの直接操作
fn setup_memory(dtb: &Dtb, dtb_address: usize,
                elf_address: usize, stack_pointer: usize) {
    // 物理メモリマップの直接設定
}
```

### **Type-2ハイパーバイザー（VirtualBox等）**

#### **1. ホストOS依存のビルド**
```toml
# Type-2の典型的設定
[dependencies]
libc = "0.2"          # OS依存ライブラリ
winapi = "0.3"        # Windows API（Windows上）
# OS標準のシステムコール使用
```

#### **2. 通常のアプリケーション形式**
```rust
// Type-2の典型的エントリポイント
use std::*;  // 標準ライブラリ使用

fn main() {
    // ホストOSのプロセスとして起動
    println!("Starting Type-2 Hypervisor...");

    // OSのデバイスドライバー経由でハードウェア制御
    let vm = create_virtual_machine();

    // OSのスケジューリング下で実行
    vm.run();
}
```

#### **3. OS抽象化レイヤー経由**
```rust
// ホストOSのAPIを使用
use std::fs::File;
use std::os::unix::io::AsRawFd;

fn allocate_memory(size: usize) -> *mut u8 {
    // OSの仮想メモリシステム使用
    unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8
    }
}
```

## 📊 **重要な差異まとめ**

| 項目 | Type-1 (MiniVisor) | Type-2 (VirtualBox) |
|------|--------------------|--------------------|
| **ビルドターゲット** | `aarch64-unknown-none` | `x86_64-unknown-linux-gnu` |
| **標準ライブラリ** | `#![no_std]` | `std` 使用 |
| **エントリポイント** | `#![no_main]` カスタム | 通常の `fn main()` |
| **実行特権** | EL2/Ring -1 | ユーザーランド |
| **ハードウェア制御** | 直接制御 | OSドライバー経由 |
| **メモリ管理** | 物理メモリ直接 | OS仮想メモリ経由 |
| **プロセス管理** | ハイパーバイザーが制御 | OSスケジューラー下 |

## 🧪 **実証実験**

### **実験1：特権レベル確認**

#### Type-1での確認
```bash
cd MiniVisor
cargo run --release
# 出力: CurrentEL: 2  ← EL2で実行
```

#### Type-2での確認
```bash
# VirtualBoxやQEMU（user mode）
./virtualbox-app
# プロセスリストで確認
ps aux | grep virtualbox  ← 通常プロセスとして表示
```

### **実験2：システムコール使用の違い**

#### Type-1（MiniVisor）
```rust
// システムコール不使用
// OSカーネル関数を直接呼び出さない
// 全て自前実装
```

#### Type-2
```rust
// OSのシステムコールを多用
open("/dev/kvm", O_RDWR);     // Linux KVM使用
ioctl(fd, KVM_CREATE_VM, 0);  // カーネル機能活用
```

### **実験3：起動プロセスの違い**

#### Type-1起動フロー
```
UEFI/BIOS → MiniVisor → Guest OS
```

#### Type-2起動フロー
```
UEFI/BIOS → Host OS → VirtualBox → Guest OS
```

## 💡 **学習のポイント**

### **なぜType-1が高性能か**
1. **オーバーヘッド最小**: OS層を経由しない
2. **直接制御**: ハードウェアリソースを直接管理
3. **最適化可能**: 中間層の制約がない

### **なぜType-1がセキュアか**
1. **TCB最小**: 信頼すべきコードが最小限
2. **分離徹底**: ゲスト間の完全分離
3. **特権管理**: 最高特権での制御

### **Type-2の存在意義**
1. **導入容易**: 既存OS上で動作
2. **開発効率**: OSの機能を活用可能
3. **デスクトップ用途**: 開発・テスト環境

## 🎯 **次の学習ステップ**

Day 3-4では、この基盤の上で：
- Intel VT-x、AMD-V、ARM仮想化拡張の詳細
- ハードウェア支援型仮想化の実装
- MiniVisorでの具体的な仮想化メカニズム確認