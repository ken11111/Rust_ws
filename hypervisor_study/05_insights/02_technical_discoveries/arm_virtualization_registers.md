# ARM仮想化レジスタの詳細解析

## 🔧 **HCR_EL2（Hypervisor Configuration Register）詳細**

### **MiniVisorで使用されるHCR_EL2ビット**

```rust
// src/registers.rs での定義
pub const HCR_EL2_API: u64 = 1 << 41;   // Bit 41: API
pub const HCR_EL2_RW: u64 = 1 << 31;    // Bit 31: RW
pub const HCR_EL2_AMO: u64 = 1 << 5;    // Bit 5:  AMO
pub const HCR_EL2_IMO: u64 = 1 << 4;    // Bit 4:  IMO
pub const HCR_EL2_FMO: u64 = 1 << 3;    // Bit 3:  FMO
pub const HCR_EL2_VM: u64 = 1 << 0;     // Bit 0:  VM
```

### **各ビットの詳細機能**

#### **HCR_EL2.VM [Bit 0] - Virtualization Enable**
```
0: 仮想化無効（EL1はNon-secure EL1として動作）
1: 仮想化有効（EL1はVirtual EL1として動作）
```
- **最重要ビット**：このビットで仮想化の有効/無効を制御
- **Stage-2有効化**：1に設定するとStage-2ページ変換が有効

#### **HCR_EL2.FMO [Bit 3] - Fast Interrupt Override**
```
0: FIQはEL1に配送
1: FIQはEL2に配送（ハイパーバイザーが処理）
```

#### **HCR_EL2.IMO [Bit 4] - IRQ Mask Override**
```
0: IRQはEL1に配送
1: IRQはEL2に配送（ハイパーバイザーが処理）
```

#### **HCR_EL2.AMO [Bit 5] - SError Mask Override**
```
0: SErrorはEL1に配送
1: SErrorはEL2に配送（ハイパーバイザーが処理）
```

#### **HCR_EL2.RW [Bit 31] - Register Width**
```
0: EL1はAArch32で実行
1: EL1はAArch64で実行
```

#### **HCR_EL2.API [Bit 41] - Address Pointer Authentication**
```
0: EL1でのPointer Authentication無効
1: EL1でのPointer Authentication有効
```

## 🔧 **SPSR_EL2（Saved Program Status Register）**

### **MiniVisorでの設定**
```rust
pub const SPSR_EL2_M_EL1H: u64 = 0b0101;  // EL1h mode
```

### **SPSR_EL2.M[3:0] - Exception Level**
```
0b0000 (0): EL0t (User mode, SP_EL0)
0b0001 (1): EL1t (Kernel mode, SP_EL0)
0b0100 (4): EL0h (User mode, SP_ELx)
0b0101 (5): EL1h (Kernel mode, SP_EL1)  ← MiniVisorの設定
```

**意味**：
- **EL1h**: ゲストOSをEL1（カーネルレベル）で実行
- **SP_EL1**: EL1専用スタックポインタ使用
- **Normal execution**: ゲストOSは通常のカーネルとして動作

## 🔧 **Stage-2ページング関連レジスタ**

### **VTTBR_EL2（Virtual Translation Table Base Register）**
```rust
pub const VTTBR_BADDR: u64 = ((1 << 47) - 1) & !1;
```

**構造**：
```
[63:56] VMID     : Virtual Machine ID
[55:48] Reserved
[47:1]  BADDR    : Stage-2変換テーブルのベースアドレス
[0]     CnP      : Common not Private
```

### **VTCR_EL2（Virtual Translation Control Register）**
```rust
pub const VTCR_EL2_RES1: u64 = 1 << 31;           // Reserved bit (must be 1)
pub const VTCR_EL2_PS_BITS_OFFSET: u64 = 16;      // Physical Address Size
pub const VTCR_EL2_TG0_BITS_OFFSET: u64 = 14;     // Translation Granule
pub const VTCR_EL2_SH0_BITS_OFFSET: u64 = 12;     // Shareability
pub const VTCR_EL2_ORGN0_BITS_OFFSET: u64 = 10;   // Outer Cacheability
pub const VTCR_EL2_IRGN0_BITS_OFFSET: u64 = 8;    // Inner Cacheability
pub const VTCR_EL2_SL0_BITS_OFFSET: u64 = 6;      // Starting Level
pub const VTCR_EL2_T0SZ_BITS_OFFSET: u64 = 0;     // Size offset
```

### **Stage-2ページングの設定例**
```rust
// 典型的なVTCR_EL2設定
let vtcr_el2 = VTCR_EL2_RES1 |                    // Reserved bit
               (0b011 << VTCR_EL2_PS_BITS_OFFSET) |   // 40-bit PA space
               (0b01 << VTCR_EL2_TG0_BITS_OFFSET) |   // 64KB granule
               (0b11 << VTCR_EL2_SH0_BITS_OFFSET) |   // Inner shareable
               (0b01 << VTCR_EL2_ORGN0_BITS_OFFSET) | // Write-back cacheable
               (0b01 << VTCR_EL2_IRGN0_BITS_OFFSET) | // Write-back cacheable
               (0b00 << VTCR_EL2_SL0_BITS_OFFSET) |   // Starting at level 2
               (24 << VTCR_EL2_T0SZ_BITS_OFFSET);     // 2^40 = 1TB address space
```

## 🔍 **実装での使用例検索**

### **HCR_EL2設定の確認**
```bash
cd MiniVisor
grep -r "HCR_EL2" src/
# → どのファイルでHCR_EL2が設定されているか確認

grep -r "set_hcr_el2" src/
# → HCR_EL2設定関数の呼び出し箇所確認
```

### **Stage-2ページング設定の確認**
```bash
grep -r "VTTBR" src/
grep -r "VTCR" src/
# → Stage-2ページングの設定箇所確認
```

## 💡 **学習のポイント**

### **ARM仮想化の階層構造**
```
EL2 (Hypervisor)
├─ HCR_EL2: 全体の仮想化制御
├─ VTTBR_EL2/VTCR_EL2: Stage-2ページング
└─ SPSR_EL2/ELR_EL2: ゲスト実行制御

EL1 (Guest OS)
├─ SCTLR_EL1: ゲスト内システム制御
├─ TTBR0_EL1/TTBR1_EL1: Stage-1ページング
└─ TCR_EL1: ゲスト内変換制御
```

### **2段階アドレス変換**
```
Guest Virtual Address
        ↓ Stage-1 (ゲスト管理)
Guest Physical Address
        ↓ Stage-2 (ハイパーバイザー管理)
Host Physical Address
```

## 🎯 **理解度チェック**

### **レジスタ理解**
- [x] HCR_EL2の主要ビットの役割を説明できる
- [x] SPSR_EL2でのEL1h設定の意味を理解している
- [x] VTTBR_EL2/VTCR_EL2のStage-2制御を理解している

### **仮想化メカニズム理解**
- [x] VM BitによるVirtual EL1への切り替えを理解
- [x] 割り込みルーティング（FMO/IMO/AMO）の制御を理解
- [x] Stage-2ページングによるメモリ仮想化を理解

## 📚 **次のステップ**

Day 5-6では：
1. **実際のStage-2ページテーブル構築**
2. **メモリ分離メカニズムの実装確認**
3. **ゲストVMの物理メモリ配置**

このレジスタ知識により、MiniVisorの実装をより深く理解できるようになります！