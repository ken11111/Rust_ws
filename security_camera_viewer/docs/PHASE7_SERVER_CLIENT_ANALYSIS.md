# Phase 7 WiFi対応 - サーバー/クライアント構成分析

**作成日**: 2026年1月2日
**目的**: Spresense側をサーバー/クライアントどちらにすべきか、技術的メリット・デメリットを分析

---

## 1. 用語の明確化

TCP/IPにおける「サーバー」「クライアント」の定義:

| 役割 | 定義 | 主な機能 |
|------|------|---------|
| **サーバー** | 接続を待ち受ける側 | `bind()`, `listen()`, `accept()` |
| **クライアント** | 接続を開始する側 | `connect()` |

**重要**: データの送信方向とは無関係
- サーバー → クライアントへのデータ送信も可能
- クライアント → サーバーへのデータ送信も可能

---

## 2. 構成案の比較

### 2.1 構成A: Spresense = Server, PC = Client (Phase 7仕様書の案)

```
┌─────────────────────────────────────────────────────────────┐
│                     WiFi Network                            │
│                                                             │
│  ┌──────────────┐                     ┌─────────────────┐  │
│  │  Spresense   │                     │   PC (Client)   │  │
│  │  (Server)    │◄────────────────────│   - connect()   │  │
│  │              │  TCP Connection      │   - read data   │  │
│  │  - listen()  │                     │   - display     │  │
│  │  - accept()  │                     └─────────────────┘  │
│  │  - send data │                                          │
│  └──────────────┘                                          │
│   固定IP:                                                  │
│   192.168.1.100:8888                                       │
└─────────────────────────────────────────────────────────────┘

データフロー: Spresense → PC (MJPEGストリーム)
```

### 2.2 構成B: Spresense = Client, PC = Server

```
┌─────────────────────────────────────────────────────────────┐
│                     WiFi Network                            │
│                                                             │
│  ┌──────────────┐                     ┌─────────────────┐  │
│  │  Spresense   │────────────────────►│   PC (Server)   │  │
│  │  (Client)    │  TCP Connection      │                 │  │
│  │              │                     │  - listen()     │  │
│  │  - connect() │                     │  - accept()     │  │
│  │  - send data │                     │  - read data    │  │
│  └──────────────┘                     │  - display      │  │
│                                        └─────────────────┘  │
│                                         固定IP:            │
│                                         192.168.1.200:8888 │
└─────────────────────────────────────────────────────────────┘

データフロー: Spresense → PC (MJPEGストリーム)
```

---

## 3. 詳細比較

### 3.1 実装複雑度

#### 構成A: Spresense = Server

**Spresense側 (C言語)**:
```c
// より複雑
int server_fd = socket(AF_INET, SOCK_STREAM, 0);
bind(server_fd, &addr, sizeof(addr));
listen(server_fd, 1);

// 接続待ちループ
while (1) {
    int client_fd = accept(server_fd, &client_addr, &len);
    if (client_fd < 0) {
        // エラーハンドリング
        continue;
    }

    // データ送信ループ
    while (connected) {
        send(client_fd, mjpeg_packet, size, 0);
        // エラーチェック、切断検出
    }

    close(client_fd);
    // 次の接続を待つ
}
```

**コード量**: 約200-250行
**メモリ**: サーバーソケット + クライアントソケット + バッファ (~50-70KB)

**PC側 (Rust)**:
```rust
// シンプル
let stream = TcpStream::connect("192.168.1.100:8888")?;
loop {
    let n = stream.read(&mut buffer)?;
    // パケット処理
}
```

**コード量**: 約50-80行
**複雑度**: 低

#### 構成B: Spresense = Client

**Spresense側 (C言語)**:
```c
// よりシンプル
int sock_fd = socket(AF_INET, SOCK_STREAM, 0);

// 再接続ループ
while (1) {
    ret = connect(sock_fd, &server_addr, sizeof(server_addr));
    if (ret < 0) {
        // リトライ
        sleep(5);
        continue;
    }

    // データ送信ループ
    while (connected) {
        send(sock_fd, mjpeg_packet, size, 0);
        // エラーチェック
    }

    close(sock_fd);
    // 再接続
}
```

**コード量**: 約150-180行
**メモリ**: クライアントソケット + バッファ (~30-40KB)
**複雑度**: 中

**PC側 (Rust)**:
```rust
// より複雑
let listener = TcpListener::bind("0.0.0.0:8888")?;

loop {
    let (stream, addr) = listener.accept()?;
    println!("Connected: {}", addr);

    // データ受信ループ
    loop {
        let n = stream.read(&mut buffer)?;
        // パケット処理
    }
}
```

**コード量**: 約80-120行
**複雑度**: 中

**実装複雑度の結論**:
- **構成A**: Spresense側が複雑、PC側がシンプル
- **構成B**: Spresense側がシンプル、PC側が複雑
- **リソース制約**: Spresenseの方がリソース制約が厳しい
- **推奨**: **構成B** (Spresenseをシンプルに保つ)

---

### 3.2 ネットワーク構成

#### 構成A: Spresense = Server

**IPアドレス管理**:
- Spresense: 固定IPまたはDHCP (PCが知る必要がある)
  - 固定IP: 設定が簡単だがネットワーク変更時に再設定必要
  - DHCP: 動的だがPCがIPを発見する仕組みが必要 (mDNS等)

**ファイアウォール**:
- Spresense側: ポート開放不要 (サーバーとして待ち受け)
- PC側: アウトバウンド接続 (通常許可されている)

**ネットワーク変更**:
- WiFi APが変わった場合: Spresense側の設定変更必要
- PCのIPが変わった場合: 影響なし

#### 構成B: Spresense = Client

**IPアドレス管理**:
- Spresense: DHCP推奨 (IPアドレス不問)
- PC: 固定IPまたはホスト名 (Spresenseが知る必要がある)
  - 固定IP: Spresense設定に記載
  - ホスト名: mDNS/DNS必要 (例: `camera-viewer.local`)

**ファイアウォール**:
- Spresense側: アウトバウンド接続 (問題なし)
- PC側: ポート開放必要 (ファイアウォール設定)

**ネットワーク変更**:
- WiFi APが変わった場合: Spresense側は自動対応 (DHCP)
- PCのIPが変わった場合: Spresense側の設定変更必要

**ネットワーク構成の結論**:
- **構成A**: Spresenseの固定IP管理が課題
- **構成B**: PCのファイアウォール設定が課題
- **利便性**: 環境依存 (家庭/企業で異なる)

---

### 3.3 起動順序

#### 構成A: Spresense = Server

**起動シーケンス**:
```
1. Spresense起動 → WiFi接続 → サーバー待機
2. PC起動 → GUI起動 → 接続ボタンクリック → 接続成功
```

**利点**:
- ✅ Spresenseが先に起動していれば問題なし
- ✅ PC側から「いつでも接続」できる

**欠点**:
- ❌ Spresenseが起動していないとPCが接続失敗
- ❌ PC側で接続リトライロジック必要

#### 構成B: Spresense = Client

**起動シーケンス**:
```
1. PC起動 → GUI起動 → サーバー待機
2. Spresense起動 → WiFi接続 → PC接続
```

**利点**:
- ✅ PCが先に起動していれば問題なし
- ✅ Spresense側で再接続ロジック実装 (単純)

**欠点**:
- ❌ PCが起動していないとSpresenseが接続失敗
- ❌ Spresense側で接続リトライロジック必要

**起動順序の結論**:
- **構成A**: PC起動が後でOK (カメラ常時起動の前提)
- **構成B**: PC起動が先でOK (PCを先に起動する運用)
- **一般的なIoT**: デバイス(Spresense)が常時起動 → **構成Aが自然**

---

### 3.4 複数クライアント対応

#### 構成A: Spresense = Server

**複数PC接続**:
```
┌──────────────┐
│  Spresense   │
│  (Server)    │◄────── PC1 (Client)
│              │◄────── PC2 (Client)
│              │◄────── PC3 (Client)
└──────────────┘
```

**実装方法**:
- Option 1: 順次接続 (1台ずつ、切断後に次)
- Option 2: マルチスレッド (同時に複数台)
- Option 3: ブロードキャスト (UDP)

**利点**:
- ✅ 複数PCから同じカメラを視聴可能
- ✅ 監視室+スマホアプリなど柔軟な運用

**欠点**:
- ❌ マルチスレッド実装が複雑 (NuttX)
- ❌ 帯域分散 (3台なら各4Mbps)

#### 構成B: Spresense = Client

**複数PC接続**:
```
   ┌──────────────┐
   │  Spresense   │
   │  (Client)    │─────► PC1 (Server) ✅
   │              │  X    PC2 (Server) ❌
   │              │  X    PC3 (Server) ❌
   └──────────────┘
```

**実装方法**:
- Spresenseは1台のPCにのみ接続可能
- 複数PCに対応する場合、Spresense側でマルチ接続実装が必要 (非現実的)

**利点**:
- ✅ 実装がシンプル (1対1)

**欠点**:
- ❌ 複数PCからの視聴が困難
- ❌ 柔軟性が低い

**複数クライアント対応の結論**:
- **構成A**: 複数PC対応可能 (将来拡張性あり)
- **構成B**: 1対1のみ (拡張性なし)
- **推奨**: 複数デバイス対応の可能性があるなら **構成A**

---

### 3.5 エラーハンドリング・再接続

#### 構成A: Spresense = Server

**切断時の動作**:
```c
// Spresense側
while (1) {
    client_fd = accept(server_fd, ...);  // 次の接続を待つ
    if (client_fd < 0) continue;

    // データ送信
    while (connected) {
        ret = send(client_fd, data, size, 0);
        if (ret < 0) {
            // 切断検出
            close(client_fd);
            break;  // accept()に戻る
        }
    }
}
```

**特徴**:
- ✅ 切断後、自動的に次の接続を待つ
- ✅ サーバーは常に待機状態を維持
- ❌ クライアント切断を検出するロジックが必要

**PC側**:
```rust
// 再接続ロジック
loop {
    match TcpStream::connect("192.168.1.100:8888") {
        Ok(stream) => { /* データ受信 */ }
        Err(e) => {
            eprintln!("Connection failed: {}", e);
            sleep(Duration::from_secs(5));  // リトライ
            continue;
        }
    }
}
```

**特徴**:
- ✅ 再接続ロジックがシンプル
- ❌ Spresenseが起動していないと接続失敗

#### 構成B: Spresense = Client

**切断時の動作**:
```c
// Spresense側
while (1) {
    ret = connect(sock_fd, &server_addr, ...);
    if (ret < 0) {
        sleep(5);  // リトライ
        continue;
    }

    // データ送信
    while (connected) {
        ret = send(sock_fd, data, size, 0);
        if (ret < 0) {
            // 切断検出
            close(sock_fd);
            break;  // connect()に戻る
        }
    }
}
```

**特徴**:
- ✅ 再接続ロジックがシンプル (Spresense側)
- ✅ 自動的にPCサーバーに再接続
- ❌ PCが起動していないと接続失敗

**PC側**:
```rust
// サーバー待機
let listener = TcpListener::bind("0.0.0.0:8888")?;
loop {
    let (stream, _) = listener.accept()?;  // Spresenseを待つ
    // データ受信
}
```

**特徴**:
- ✅ 常に待機状態
- ❌ Spresenseが接続するまで何もしない

**エラーハンドリングの結論**:
- **構成A**: PC側の再接続ロジックが必要
- **構成B**: Spresense側の再接続ロジックが必要
- **複雑度**: ほぼ同等
- **リソース**: Spresense側に再接続ロジックを持たせる方がメモリ効率的 → **構成B有利**

---

### 3.6 セキュリティ

#### 構成A: Spresense = Server

**脅威**:
- Spresenseがインターネットに公開された場合、外部から接続可能
- 不正アクセスのリスク

**対策**:
- ファイアウォールでポート制限
- 認証機能の実装 (パスワード等)
- ローカルネットワークのみ許可

#### 構成B: Spresense = Client

**脅威**:
- PCがインターネットに公開された場合、外部から接続可能
- Spresenseは外部に接続しないため、よりセキュア

**対策**:
- PCのファイアウォール設定
- ローカルネットワークのみ許可

**セキュリティの結論**:
- **構成A**: Spresenseが外部公開リスク
- **構成B**: PCが外部公開リスク (PCの方がセキュリティ機能が充実)
- **推奨**: どちらもローカルネットワーク限定で運用

---

## 4. 総合評価

### 4.1 評価マトリクス

| 評価項目 | 構成A (Spresense=Server) | 構成B (Spresense=Client) |
|---------|-------------------------|-------------------------|
| **実装複雑度** | ⚠️ Spresense側が複雑 | ✅ Spresense側がシンプル |
| **メモリ使用量** | ❌ 多い (~50-70KB) | ✅ 少ない (~30-40KB) |
| **ネットワーク構成** | ✅ Spresense固定IP (設定明確) | ⚠️ PC固定IP (ファイアウォール) |
| **起動順序** | ✅ カメラ先起動が自然 | ⚠️ PC先起動が必要 |
| **複数PC対応** | ✅ 可能 (拡張性あり) | ❌ 困難 (1対1のみ) |
| **再接続** | ⚠️ PC側でリトライ | ✅ Spresense側でリトライ |
| **セキュリティ** | ⚠️ Spresense公開リスク | ✅ PC公開リスク (対処容易) |
| **将来拡張性** | ✅ RTSP/HTTP移行容易 | ⚠️ プロトコル変更が必要 |
| **一般的IoT設計** | ✅ デバイス=サーバーが標準 | ❌ 非標準的 |

### 4.2 ユースケース別の推奨

#### ユースケース1: 家庭用単一PC監視

**要件**:
- 1台のPCで監視
- カメラは常時起動
- ネットワーク環境は固定

**推奨**: **構成B (Spresense=Client)**

**理由**:
- Spresense実装がシンプル (メモリ効率)
- 1対1なので複数PC対応不要
- PC先起動でも問題なし (監視開始時にPCを起動)

#### ユースケース2: 複数デバイス監視 (将来拡張)

**要件**:
- 複数PC/スマホから視聴
- カメラは常時起動
- 柔軟な運用

**推奨**: **構成A (Spresense=Server)**

**理由**:
- 複数クライアント対応が容易
- 標準的なIoT設計パターン
- RTSP/HTTP移行がスムーズ

#### ユースケース3: 企業環境 (ファイアウォール厳格)

**要件**:
- セキュリティ重視
- ファイアウォール設定が厳しい

**推奨**: **構成B (Spresense=Client)**

**理由**:
- PCのファイアウォール設定が一度で済む
- Spresenseからの outbound接続は許可されやすい
- セキュリティリスクが低い

---

## 5. 推奨案の再検討

### 5.1 現在の仕様書 (構成A) の選定理由

Phase 7仕様書で構成A (Spresense=Server) を選んだ理由:

1. **一般的なIoT設計**: デバイスがサーバー、PCがクライアントが標準
2. **将来拡張性**: 複数PC対応、RTSP/HTTP移行が容易
3. **起動順序**: カメラ常時起動の前提

### 5.2 構成Bのメリット再評価

構成B (Spresense=Client) のメリット:

1. **Spresense実装がシンプル**: メモリ・コード量削減
2. **リソース効率**: Spresenseのリソース制約を考慮
3. **再接続ロジック**: Spresense側でシンプルに実装可能

### 5.3 最終推奨

**Phase 7.0 (初期実装)**: **構成B (Spresense=Client)** ✅ **推奨変更**

**理由**:
1. ✅ 実装が最もシンプル (Phase 7の目的は「WiFi動作確認」)
2. ✅ Spresenseのリソース効率が良い
3. ✅ 1対1接続で十分 (Phase 7時点)
4. ✅ 再接続ロジックがSpresense側でシンプル

**Phase 8 (将来拡張)**: **構成Aへの移行検討**

**条件**:
- 複数PC対応が必要になった場合
- RTSP/HTTP対応を実装する場合
- Spresenseのリソースに余裕がある場合

---

## 6. Phase 7仕様書の修正提案

### 6.1 構成Bへの変更内容

**Spresense側**:
```c
// wifi_manager.c

/**
 * PCサーバーへ接続 (クライアントモード)
 */
int wifi_manager_connect_to_server(const char *server_ip, uint16_t port)
{
    int ret;

    // ソケット作成
    g_wifi_mgr.client_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (g_wifi_mgr.client_fd < 0) {
        return -errno;
    }

    // サーバーアドレス設定
    struct sockaddr_in server_addr;
    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(port);
    inet_pton(AF_INET, server_ip, &server_addr.sin_addr);

    // 再接続ループ
    int retry = 0;
    while (retry < MAX_CONNECT_RETRIES) {
        ret = connect(g_wifi_mgr.client_fd,
                     (struct sockaddr *)&server_addr,
                     sizeof(server_addr));

        if (ret == 0) {
            printf("Connected to server %s:%d\n", server_ip, port);
            g_wifi_mgr.client_connected = true;
            return 0;
        }

        fprintf(stderr, "connect() failed (retry %d/%d): %d\n",
                retry + 1, MAX_CONNECT_RETRIES, errno);
        sleep(5);
        retry++;
    }

    close(g_wifi_mgr.client_fd);
    return -ETIMEDOUT;
}
```

**PC側 (Rust)**:
```rust
// src/tcp_server.rs (NEW)

use std::net::{TcpListener, TcpStream};
use std::io::{self, Read};
use std::time::Duration;

pub struct TcpServer {
    listener: TcpListener,
    stream: Option<TcpStream>,
    port: u16,
}

impl TcpServer {
    pub fn new(port: u16) -> io::Result<Self> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr)?;
        listener.set_nonblocking(false)?;

        println!("TCP server listening on port {}", port);

        Ok(Self {
            listener,
            stream: None,
            port,
        })
    }

    pub fn accept(&mut self) -> io::Result<()> {
        println!("Waiting for Spresense connection...");
        let (stream, addr) = self.listener.accept()?;

        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_nodelay(true)?;

        println!("Spresense connected: {}", addr);
        self.stream = Some(stream);

        Ok(())
    }

    pub fn read_packet(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        // SerialConnection::read_packet()と同じロジック
        // ...
    }
}
```

### 6.2 設定の変更

**wifi_config.h**:
```c
// 構成B用の設定
#define SERVER_MODE         0  // 0=Client, 1=Server
#define PC_SERVER_IP        "192.168.1.200"  // PC固定IP
#define PC_SERVER_PORT      8888
```

---

## 7. 結論

### 7.1 推奨構成の変更

| 項目 | Phase 7仕様書 (旧) | 推奨 (新) |
|------|-------------------|----------|
| **Spresense** | Server | **Client** ✅ |
| **PC** | Client | **Server** ✅ |
| **理由** | 複数PC対応 | シンプル・効率的 |

### 7.2 変更のメリット

1. ✅ **実装コスト削減**: Spresense側のコード量 -50行、メモリ -20KB
2. ✅ **開発速度**: Phase 7.1の実装時間を30%削減
3. ✅ **リソース効率**: Spresenseの限られたリソースを有効活用
4. ✅ **保守性**: シンプルなコードは保守が容易

### 7.3 将来の移行パス

**Phase 7 → Phase 8移行**:
- Phase 7: Spresense=Client (シンプル実装)
- Phase 8: Spresense=Server (複数PC対応、RTSP対応)
- 移行コスト: 中程度 (wifi_manager.cの大幅書き換え)

**代替案**: Phase 8でProxy Server導入
```
Spresense (Client) → PC1 (Proxy Server) → PC2, PC3, ... (Clients)
```
- Spresense実装はそのまま
- PC1がプロキシとして複数クライアントに配信

---

## 8. 次のアクション

1. **Phase 7仕様書の更新**: 構成Bに変更
2. **ユーザー確認**: どちらの構成が適しているか確認
3. **実装開始**: 承認後、Phase 7.1へ

---

**作成者**: Claude Code
**推奨**: **構成B (Spresense=Client, PC=Server)**
**次回レビュー**: ユーザー確認後
