# Type-1ハイパーバイザー基礎理論

## 🎯 ベアメタル実行の原理

### ハードウェア電源投入からハイパーバイザー起動までの流れ

```
Power On → Firmware (UEFI/BIOS) → Bootloader → Type-1 Hypervisor → Guest OS
     ↑              ↑                    ↑              ↑
   物理HW         初期化              ロード          最高特権
```

#### **詳細なブートシーケンス**

1. **電源投入（Power-On Reset）**
   ```
   CPU Reset Vector → 0xFFFFFFF0 (x86) / 0x0 (ARM)
   - 初期状態: Real Mode (x86) / Secure State (ARM)
   - メモリ: 初期化前状態
   - 割り込み: 無効
   ```

2. **ファームウェア実行（UEFI/BIOS）**
   ```
   UEFI Boot Services:
   - ハードウェア初期化
   - メモリマップ構築
   - デバイス検出・初期化
   - セキュアブート検証
   - ブートデバイス特定
   ```

3. **ハイパーバイザー直接起動**
   ```
   Type-1 Direct Boot:
   - UEFI → Hypervisor (ブートローダー不要)
   - または GRUB → Hypervisor
   - 最高特権レベルで起動 (Ring 0/EL2)
   - 物理メモリ全体制御権取得
   ```

### **MiniVisorでの実装確認**

MiniVisorがどのようにベアメタル実行を実現しているか確認してみましょう。