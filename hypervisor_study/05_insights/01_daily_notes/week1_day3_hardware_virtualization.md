# Week 1 Day 3: ARM仮想化拡張の実装確認

## 📅 日付: 2026-01-26

## 🎯 学習目標
- [x] ARM Virtualization Extensionsの理解
- [x] MiniVisorでの実装確認
- [x] ハイパーバイザー制御レジスタの詳細理解

## 🔧 **ARM仮想化拡張の核心レジスタ**

### **発見：MiniVisorのARM仮想化実装**

#### **1. HCR_EL2（Hypervisor Configuration Register）**
```rust
pub unsafe fn set_hcr_el2(hcr_el2: u64) {
    unsafe { asm!("msr hcr_el2, {}", in(reg) hcr_el2) };
}
```

**HCR_EL2の重要ビット**：
- **VM bit [0]**: 仮想化有効/無効
- **SWIO bit [1]**: SCTLR_EL1, TCR_EL1のトラップ制御
- **PTW bit [2]**: Page Table Walk のトラップ
- **FMO/IMO/AMO**: F/I/A割り込みのルーティング制御
- **TSC bit [19]**: SMC命令のトラップ制御
- **TGE bit [27]**: Guest実行制御

#### **2. ELR_EL2（Exception Link Register）**
```rust
pub unsafe fn set_elr_el2(elr_el2: u64) {
    unsafe { asm!("msr elr_el2, {}", in(reg) elr_el2) };
}
```

**用途**：
- **ゲストOS復帰アドレス**：VMEntryでの復帰先
- **例外処理復帰**：EL2での例外処理後の復帰
- **コンテキストスイッチ**：vCPU切り替え時の状態保存

#### **3. SPSR_EL2（Saved Program Status Register）**
```rust
pub unsafe fn set_spsr_el2(spsr_el2: u64) {
    unsafe { asm!("msr spsr_el2, {}", in(reg) spsr_el2) };
}
```

**保存内容**：
- **PSTATE**: プロセッサ状態（NZCV flags, DAIF, etc.）
- **Exception Level**: 復帰先のEL（通常EL1 = ゲストカーネル）
- **実行状態**: AArch64/AArch32の選択

#### **4. ERET（Exception Return）**
```rust
pub unsafe fn eret(x0: u64, x1: u64, x2: u64, x3: u64) -> ! {
    unsafe {
        asm!("eret",
             in("x0") x0,    // ゲストへの引数
             in("x1") x1,    // ゲストへの引数
             in("x2") x2,    // ゲストへの引数
             in("x3") x3,    // ゲストへの引数
        );
    }
}
```

**動作**：
1. **SPSR_EL2 → PSTATE**: 状態復元
2. **ELR_EL2 → PC**: 実行アドレス復元
3. **EL2 → EL1**: 特権レベル変更（ハイパーバイザー → ゲスト）

## 🔍 **VMEntry/VMExitメカニズムの実証**

### **VMEntry（ハイパーバイザー → ゲスト）**
```rust
// vm.rs での実装例
pub fn boot_vm(entry_point: usize, argument: usize) -> ! {
    unsafe {
        // ゲストの実行状態を設定
        asm::set_spsr_el2(SPSR_EL2_M_EL1H);  // EL1での実行
        asm::set_elr_el2(entry_point as u64);  // エントリポイント設定

        // ゲストに制御を移行（VMEntry）
        asm::eret(argument as u64, 0, 0, 0);  // EL2 → EL1
    }
}
```

### **VMExit（ゲスト → ハイパーバイザー）**
```rust
// exception.rs での実装例
#[naked]
pub unsafe extern "C" fn vectors_el2() -> ! {
    naked_asm!(
        ".balign 0x800",  // ベクターテーブルのアライメント

        // EL1からEL2への例外（VMExit）
        "bl handle_exception",  // ハイパーバイザーの例外ハンドラー呼び出し

        // 復帰時はERETでVMEntry
    );
}
```

## 💡 **Intel VT-x / AMD-Vとの比較**

### **ARM vs x86 仮想化拡張**

| 項目 | ARM (MiniVisor) | Intel VT-x | AMD-V |
|------|-----------------|-------------|-------|
| **制御構造** | HCR_EL2 | VMCS | VMCB |
| **VMEntry** | ERET命令 | VMRESUME/VMLAUNCH | VMRUN命令 |
| **VMExit** | Exception to EL2 | VM Exit | #VMEXIT |
| **ゲスト状態保存** | SPSR_EL2, ELR_EL2 | VMCS Guest State | VMCB Guest Save |
| **特権レベル** | EL2 ↔ EL1 | VMX Root ↔ Non-root | Host ↔ Guest |

### **共通の概念**
1. **ハードウェア支援**: CPU が仮想化を直接サポート
2. **特権分離**: ハイパーバイザーとゲストの明確な分離
3. **透明性**: ゲストOSは仮想化を意識しない
4. **トラップ**: 特権操作を自動的にハイパーバイザーに転送

## 🧪 **実践的理解の確認**

### **実験：HCR_EL2設定の確認**

MiniVisorでHCR_EL2がどのように設定されているか確認：

```bash
# MiniVisorのコードでHCR_EL2設定を探す
grep -r "hcr_el2" src/
grep -r "HCR" src/
```

### **学習課題：仮想化拡張の有効化**

1. **HCR_EL2.VM = 1**: 仮想化モード有効
2. **Stage-2ページング設定**: VTTBR_EL2設定
3. **割り込みルーティング**: HCR_EL2.{F,I,A}MO設定

## 🎯 **理解度チェック**

### **基本理解**
- [x] ARM EL2の役割を理解している
- [x] HCR_EL2、SPSR_EL2、ELR_EL2の用途を説明できる
- [x] ERETによるVMEntryの仕組みを理解している

### **実装理解**
- [x] MiniVisorでの仮想化拡張使用を確認した
- [x] VMEntry/VMExitのコード実装を理解した
- [x] x86仮想化との類似点・相違点を把握した

## 📚 **次の学習ステップ**

Day 5-6では：
1. **Stage-2ページング**: メモリ仮想化の詳細
2. **VTTBR_EL2, VTCR_EL2**: ページテーブル制御
3. **メモリ分離メカニズム**: ゲスト間のメモリ保護

## 🌟 **重要な発見**

MiniVisorは**教育用でありながら本格的**な仮想化拡張を使用：
- ARM仕様に完全準拠
- 商用ハイパーバイザーと同等のメカニズム
- シンプルながら本質的な実装

この実装により、Type-1ハイパーバイザーの核心技術を実体験できる！