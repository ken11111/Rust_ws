# Chapter 4: 所有権システム (Ownership)

**学習日**: -
**進捗**: ⏳未着手

## 📚 学習内容

### 4.1 所有権とは

#### 所有権の3つのルール
1. Rustの各値は、**所有者 (owner)** と呼ばれる変数を持つ
2. 値の所有者は常に**1つだけ**
3. 所有者がスコープから外れると、値は**破棄 (drop)** される

#### コード例

```rust
{
    let s = String::from("hello");  // sが所有者
    // sを使う処理
}   // ここでsがスコープを抜ける → メモリ解放
```

### 4.2 参照と借用 (References and Borrowing)

#### 不変参照 (Immutable Reference)
```rust
fn calculate_length(s: &String) -> usize {
    s.len()  // 所有権を奪わず、参照だけ
}

let s1 = String::from("hello");
let len = calculate_length(&s1);  // s1を借用
println!("{} の長さは {}", s1, len);  // s1はまだ有効
```

#### 可変参照 (Mutable Reference)
```rust
fn change(s: &mut String) {
    s.push_str(", world");
}

let mut s = String::from("hello");
change(&mut s);
println!("{}", s);  // "hello, world"
```

#### 参照のルール
- **不変参照**: 同時に複数持てる
- **可変参照**: 同時に1つだけ
- **不変と可変の混在**: 不可

### 4.3 スライス型 (Slices)

```rust
let s = String::from("hello world");

let hello = &s[0..5];   // "hello"
let world = &s[6..11];  // "world"

// 文字列スライス型: &str
fn first_word(s: &str) -> &str {
    let bytes = s.as_bytes();

    for (i, &item) in bytes.iter().enumerate() {
        if item == b' ' {
            return &s[0..i];
        }
    }

    &s[..]
}
```

## 🎯 重要ポイント

### 組込み開発での応用
- **ゼロコスト抽象化**: 所有権システムはコンパイル時にチェック、実行時オーバーヘッドなし
- **メモリ安全**: バッファオーバーフローやダングリングポインタを防ぐ
- **並行性**: データ競合をコンパイル時に防止

### よくある間違い

#### 1. ダングリングポインタ（コンパイルエラー）
```rust
fn dangle() -> &String {  // エラー！
    let s = String::from("hello");
    &s  // sはここで破棄されるのに参照を返そうとしている
}   // sがスコープを抜ける
```

#### 2. 可変参照と不変参照の混在（コンパイルエラー）
```rust
let mut s = String::from("hello");

let r1 = &s;      // 不変参照
let r2 = &s;      // 不変参照
let r3 = &mut s;  // エラー！可変参照と不変参照を同時に持てない

println!("{}, {}, {}", r1, r2, r3);
```

## 💡 理解度チェック

- [ ] 所有権の3つのルールを説明できる
- [ ] moveとcopyの違いを理解している
- [ ] 参照と借用の違いを説明できる
- [ ] 可変参照と不変参照のルールを理解している
- [ ] スライスの用途を説明できる
- [ ] ダングリングポインタを防ぐ仕組みを理解している

## 🔗 関連リンク

- [The Rust Programming Language - Chapter 4](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rust by Example - Ownership](https://doc.rust-lang.org/rust-by-example/scope/move.html)

## 📝 学習メモ

（学習中に気づいたこと、疑問点などを記録）

---

**次の学習**: [Chapter 5: 構造体](./ch05-structs.md)
