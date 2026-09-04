# Week 1 Day 1: ベアメタル実行とType-1の本質

## 📅 日付: 2026-01-26

## 🎯 今日の学習目標
- [x] ベアメタル実行の原理理解
- [x] MiniVisorでの実装確認
- [x] Type-1の定義と特徴の把握

## 📚 学習内容

### **重要な発見：MiniVisorのベアメタル実行証拠**

#### **1. `#![no_std]` `#![no_main]`**
```rust
#![no_std]   // 標準ライブラリ不使用 = OS依存なし
#![no_main]  // 通常のmain関数なし = OSによる起動なし
```

**意味**：
- **OSに依存しない**：標準ライブラリ（OS提供）を使用しない
- **直接起動**：OSのプロセス管理外で実行
- **ベアメタル証拠**：これがType-1の決定的証拠

#### **2. EL2特権レベルでの起動確認**
```rust
let current_el = asm::get_currentel() >> 2;
println!("CurrentEL: {}", current_el);
assert_eq!(current_el, 2);  // EL2で実行を強制確認
```

**ARM特権レベル**：
- **EL0**: ユーザーランド（アプリケーション）
- **EL1**: カーネル（通常のOS）
- **EL2**: ハイパーバイザー（最高特権）
- **EL3**: セキュアモニター（ARM TrustZone）

**Type-1の証拠**：EL2で直接起動 = ホストOSが存在しない

#### **3. 物理ハードウェア直接制御**
```rust
setup_memory(&dtb, dtb_address, elf_address, stack_pointer);
exception::setup_exception();
let distributor = init_gic_distributor(&dtb);
```

- **メモリ管理**: 物理メモリを直接管理
- **例外処理**: 割り込み・例外を直接制御
- **GIC**: ARM Generic Interrupt Controller を直接操作

## 💡 重要な理解

### **Type-1 vs Type-2 の決定的違い**

#### **Type-1 (MiniVisor)**
```
Firmware → MiniVisor (EL2) → Guest OS (EL1)
```
- ホストOS無し
- 最高特権で直接実行
- ハードウェア直接制御

#### **Type-2 (VirtualBox等)**
```
Firmware → Host OS → Hypervisor App → Guest OS
```
- ホストOS上のアプリケーション
- OS制約下で実行
- OSドライバー経由でハードウェア制御

## 🔍 実践確認

### **EL2実行の確認方法**

ARMでの特権レベル確認コード：
```rust
// src/asm.rs での実装
pub fn get_currentel() -> u64 {
    let mut current_el: u64;
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) current_el);
    }
    current_el
}
```

**CurrentEL レジスタ**：
- Bit[3:2] が現在のException Level
- 値 2 = EL2 (ハイパーバイザー)

## 🚀 学習の発展

### **次に確認すべきポイント**
1. **ファームウェアからの起動**: どのようにUEFI/ファームウェアからMiniVisorに制御が移るか
2. **物理メモリ管理**: どのように物理アドレス空間を管理するか
3. **ゲストOS起動**: どのようにEL1でゲストを実行するか

### **理解度チェック**
- [x] ベアメタル実行を技術的に説明できる
- [x] MiniVisorがType-1である理由をコードで証明できる
- [x] EL2の意味と重要性を理解している

## 📊 学習効果
### 理解度 (1-5): 5/5
- 理論と実装が完全に結びついた
- MiniVisorの設計思想を理解できた

### 満足度 (1-5): 5/5
- 具体的なコードで理論を確認できた
- Type-1の本質を深く理解できた