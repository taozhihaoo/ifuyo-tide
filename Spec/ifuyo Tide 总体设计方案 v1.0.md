# ifuyo Tide 总体设计方案 v1.0

**项目名称**：ifuyo Tide  
**英文产品名**：ifuyo Tide  
**定位**：Windows 本地下载器 + 渐进式媒体播放器 + 本地媒体库  
**设计日期**：2026-10-03  
**状态**：总体架构基线 / 可进入 Vibe Coding 实施  
**目标平台**：Windows 11 x64  
**核心技术栈**：Tauri 2 + Rust + TypeScript + WebView2  
**核心 Torrent 引擎**：librqbit / rqbit  
**数据层**：SQLite  
**网络层**：HTTP/HTTPS + BitTorrent  
**运行模式**：Local-first / No Cloud Backend  
**核心资产**：Scheduler、Range/Piece Abstraction、Progressive Streaming Gateway、Task State Machine

---

# 1. 产品定义

## 1.1 产品一句话

> **Tide 是一个干净、快速、可控的 Windows 本地下载器，让用户可以把自己拥有或有权获取的 HTTP/HTTPS、`.torrent`、Magnet 来源交给 Tide，并在下载尚未完成时直接播放支持的媒体内容。**

Tide 不负责回答：

> “去哪里找资源？”

Tide 只负责：

> “你已经有这个来源了，我帮你稳定地下载、管理、播放它。”

因此产品核心链路为：

```text
用户提供来源
      ↓
Source
      ↓
Download Engine
      ↓
Availability / Range / Piece Map
      ↓
Scheduler
      ↓
Storage
      ↓
Progressive Stream Gateway
      ↓
Player
      ↓
Library
```

---

# 2. 产品目标

Tide 的第一目标不是“做一个功能最多的下载器”，而是解决三个实际问题：

### 2.1 下载与播放分离的问题

传统下载器：

```text
下载
↓
等待
↓
完整文件
↓
打开播放器
```

Tide：

```text
开始下载
↓
可播放数据逐渐形成
↓
播放器立即开始
↓
边下载边播放
↓
后台继续完成
```

---

### 2.2 HTTP 与 BitTorrent 体验割裂的问题

用户不应该关心：

```text
这是 HTTP？
这是 Magnet？
这是 torrent？
```

在 Tide 内部，它们最终都抽象成：

```text
Resource
    ↓
Byte Availability
    ↓
Demand
    ↓
Scheduler
```

这样 HTTP、Torrent 可以共享大量上层逻辑。

---

### 2.3 “下载器”通常很重、很乱的问题

Tide 的产品哲学：

```text
少打扰
少弹窗
少广告
少云服务
少账号
少追踪
少内容推荐

但是：

快
稳
清晰
可控
好看
好用
```

---

# 3. 非目标：哪些事情 Tide 永远不做

这些边界应该从架构层面就固定下来，而不是等以后再限制。

| 非目标 | 处理原则 |
|---|---|
| 资源搜索引擎 | 不做 |
| 盗版资源站索引 | 不做 |
| 磁力资源搜索 | 不做 |
| 资源排行榜 | 不做 |
| “热门资源”推荐 | 不做 |
| 用户资源目录聚合 | 不做 |
| 第三方盗版站导航 | 不做 |
| 内置 Tracker 推荐 | 不做 |
| 通过文件名识别盗版内容并推荐 | 不做 |
| DRM 绕过 | 不做 |
| Paywall 绕过 | 不做 |
| 内容破解 | 不做 |
| “匿名下载”宣传 | 不做 |
| 云端中转下载 | 不做 |
| ifuyo 服务器代下载 | 不做 |
| 用户下载内容上传到 ifuyo | 不做 |

Tide 可以下载什么，取决于用户提供的来源和用户自己的授权，而不是 Tide 自己建立一个“内容生态”。

---

# 4. 合规总体边界

## 4.1 BitTorrent 本身不是盗版技术

BitTorrent 是文件分发协议，本身具有合法用途。其协议设计本来就是让多个节点互相传输文件。

因此：

```text
使用 BitTorrent
≠
自动违法
```

真正的风险在于：

```text
内容是否合法
+
用户是否有权获取/复制/传播
+
软件本身是否主动参与侵权生态
```

---

## 4.2 Tide 采取“工具中立 + 用户提供来源”模型

产品设计固定为：

```text
用户
 ↓
自己找到合法来源
 ↓
复制 URL / Magnet / Torrent
 ↓
交给 Tide
 ↓
Tide 下载/播放
```

而不是：

```text
用户
 ↓
Tide 搜索
 ↓
Tide 推荐盗版资源
 ↓
Tide 聚合资源站
 ↓
Tide 引导下载
```

这一区别非常重要。

中国现行著作权法明确保护复制权、发行权、信息网络传播权等权利；最高人民法院关于信息网络传播权的司法解释也明确讨论了“利用文件分享软件”等方式向公众提供作品的情形。

因此 Tide 的设计应避免自己成为：

> “内容提供者 / 资源聚合者 / 侵权链接平台”。

---

# 5. 必须正视 BitTorrent 的特殊问题：上传

这是 Tide 与普通 HTTP 下载器最大的合规区别。

BitTorrent 是对等网络，协议本身允许节点之间互相传输已经获得的数据。

欧盟法院在 C-597/19 案中认定，在特定案件背景下，BitTorrent 下载时自动上传已经取得的文件片段，可以构成向公众提供作品；美国版权局也明确指出，未经授权通过 P2P 上传或下载受版权保护的作品可能侵犯权利。

因此 Tide 不能简单地把自己设计成：

> “我只是下载，所以所有网络行为都和我无关。”

更合理的是把共享行为显式产品化。

---

# 6. BitTorrent Sharing Policy

Tide 设置：

### Download Only

```text
不主动上传文件数据
下载完成后也不继续做种
```

并明确说明：

> 禁止上传文件片段可能影响 BitTorrent 下载速度与可用 Peer 数量。

---

### Balanced

```text
下载过程中允许有限上传
下载完成后停止做种
```

---

### Seed

```text
下载完成后继续做种
```

必须是用户主动开启。

默认建议：

```text
Download Only
```

并允许用户在第一次 Torrent 下载时看到一次明确说明。

注意：

> “Download Only”不是“内容合法保证”，只是网络行为控制。

不能在 UI 中写：

> “开启后你就安全了。”

应该写：

> “该模式会尽量阻止文件数据上传，但不会判断内容是否具有合法授权。”

---

# 7. IP 地址与隐私设计

## 7.1 不做“隐藏 IP”

这是一个非常重要的产品边界。

Tide 不应该宣传：

> 匿名下载  
> IP 保护  
> 不可追踪  
> 不留痕  
> 安全躲避版权追踪

BitTorrent 的网络模型本来就需要节点交换网络地址信息；rqbit 也具备 Peer 连接、DHT、IPv6 等能力。

因此正确产品语言是：

> **Tide 不向 ifuyo 上传你的下载数据，也不提供匿名网络服务。BitTorrent 网络连接仍然会按照协议与其他 Peer 直接建立通信。**

---

## 7.2 Tide 不收集

默认不发送：

```text
下载 URL
Magnet
Info Hash
文件名
文件路径
媒体内容
Peer IP
下载历史
播放历史
用户行为分析
```

到 ifuyo。

---

## 7.3 本地调试数据与生产日志隔离

普通生产日志只记录：

```text
task_id
source_type
state
error_code
duration
bytes
speed
```

不要记录：

```text
完整 Magnet
完整 URL
完整文件路径
Peer IP
用户文件名
```

开发模式下 Tide Lab 可以临时打开更详细数据，但：

```text
DEV ONLY
```

且不得默认写入长期生产日志。

---

# 8. 网络架构

总体：

```text
┌───────────────────────────────────────────┐
│                 Tide UI                   │
│          Tauri + React + TypeScript       │
└───────────────────┬───────────────────────┘
                    │ IPC
                    ▼
┌───────────────────────────────────────────┐
│               Tide Core                   │
│                 Rust                      │
│                                           │
│ Task Manager                              │
│ Scheduler                                 │
│ Download Engine                           │
│ Playback Session                          │
│ Library                                   │
│ Storage                                   │
│ Security                                  │
└────────────┬───────────────┬──────────────┘
             │               │
             ▼               ▼
      ┌──────────────┐ ┌───────────────┐
      │ HTTP Engine  │ │ librqbit      │
      │ reqwest      │ │ BitTorrent    │
      └──────┬───────┘ └───────┬───────┘
             │                 │
             └────────┬────────┘
                      ▼
              ┌───────────────┐
              │ Availability  │
              │ Range / Piece │
              └───────┬───────┘
                      ▼
                ┌──────────┐
                │Scheduler │
                └────┬─────┘
                     ▼
                ┌──────────┐
                │ Storage  │
                └────┬─────┘
                     ▼
             Progressive Stream
                     ▼
                  Player
```

---

# 9. 为什么选择 Tauri + Rust

Tide 与 rqbit 的关系决定了 Rust 非常合适。

rqbit 当前项目本身就是 Rust BitTorrent client，并同时提供库形式；其项目还已经有 Tauri 桌面应用。当前文档显示 `librqbit` 可直接作为 Torrent library 使用，并提供 Session API。

因此不应该做：

```text
Tauri
 +
Node torrent process
 +
Rust bridge
 +
Python helper
 +
独立 HTTP server
```

而应该尽可能做成：

```text
Tauri
  +
Rust Core
  +
librqbit
```

只有确实必要的播放器/探测组件才采用独立进程。

---

# 10. 前端技术栈

建议：

```text
Tauri 2
React
TypeScript
Vite
```

不建议第一版再塞：

```text
大型 UI Framework
大量状态管理插件
微前端
GraphQL
Electron
```

前端只负责：

```text
UI
状态展示
用户交互
播放器控制
动画
设置
```

核心业务逻辑全部在 Rust。

---

# 11. Rust Core 分层

推荐目录：

```text
src-tauri/
├── src/
│   ├── app/
│   ├── commands/
│   ├── core/
│   │   ├── task/
│   │   ├── source/
│   │   ├── scheduler/
│   │   ├── availability/
│   │   ├── playback/
│   │   ├── storage/
│   │   ├── library/
│   │   └── network/
│   ├── engines/
│   │   ├── http/
│   │   └── torrent/
│   ├── gateway/
│   ├── media/
│   ├── security/
│   ├── persistence/
│   ├── diagnostics/
│   └── windows/
└── capabilities/
```

核心原则：

> **Engine 是“怎么拿数据”，Scheduler 是“现在应该拿什么”。**

---

# 12. Source 抽象

统一：

```rust
enum Source {
    Http(HttpSource),
    TorrentFile(PathBuf),
    Magnet(String),
}
```

以后可以扩展：

```text
File
LAN
Custom Protocol
```

但是第一版不要预留几十种协议。

---

# 13. Resource 抽象

这是 Tide 最核心的数据模型之一。

不要让上层直接理解：

```text
HTTP Chunk
Torrent Piece
```

而统一抽象成：

```text
Resource
 ├── total_size
 ├── availability
 ├── read(range)
 ├── wait_until_available(range)
 └── metadata
```

例如：

```text
HTTP Resource
0 ─────────────────────────── 1000 MB
███████████████░░░░░░░░░░░░░

Torrent Resource
0 ─────────────────────────── 1000 MB
███░████░░███░░████░░░░███░░
```

上层 Scheduler 只关心：

> 哪些 byte range 可用？下一步应该获取哪些 range？

这样 Tide 的 HTTP / Torrent 能真正共享播放调度。

---

# 14. HTTP 下载引擎

支持：

```text
HTTP
HTTPS
Redirect
Range
Resume
ETag
Last-Modified
Content-Length
Chunked Transfer
```

任务状态：

```text
Pending
↓
Resolving
↓
Downloading
↓
Paused
↓
Completed
↓
Failed
```

支持断点续传：

```text
.part
+
metadata
```

完成后：

```text
fsync
↓
完整性确认
↓
atomic rename
↓
Completed
```

---

# 15. HTTP 下载安全

用户输入 URL 后：

### 默认允许

```text
http://
https://
```

### 默认拒绝

```text
file://
ftp://
smb://
gopher://
data://
javascript:
```

URL 解析、重定向、DNS 和 IP 分类要统一经过安全层。

OWASP 对 SSRF 的建议包括限制协议、避免直接信任任意 URL、验证目标地址、处理重定向链，并阻止 localhost/private/link-local 等敏感地址。

因此 Tide 建议：

```text
External HTTP Download
        ↓
URL Parse
        ↓
Scheme Check
        ↓
DNS Resolve
        ↓
IP Classification
        ↓
Redirect Check
        ↓
Request
```

---

# 16. LAN 下载

这是一个很有实际价值的后期功能。

例如：

```text
NAS
家庭服务器
局域网 Web Server
本地开发服务器
```

但是不能默认把：

```text
192.168.x.x
10.x.x.x
172.16.x.x
localhost
```

全部禁止而不给用户选择。

采用：

```text
Internet Source
LAN Source
```

两个安全模式。

默认：

```text
Internet Source
```

需要访问局域网：

```text
允许局域网来源
```

由用户显式开启。

---

# 17. Torrent Engine

使用：

```text
librqbit
```

作为核心 Torrent Engine。

当前 rqbit 已具备：

```text
Magnet
DHT
Trackers
PEX
Private Torrent
IPv6
Sequential downloading
Resume
Selective file
Streaming
Seeking
Upload/download rate limiting
```

以及通过 HTTP Range 进行流式播放的能力。

这意味着 Tide 不应该重新实现 BitTorrent 协议。

---

# 18. Tide 与 librqbit 的职责边界

非常重要。

### librqbit 负责

```text
Tracker
DHT
Peer
Piece
Hash
BitTorrent protocol
Connection management
Piece verification
Torrent storage mechanics
```

### Tide 负责

```text
Task
UI
Library
Scheduling policy
Playback demand
User priority
Download policy
Upload policy
Security
Persistence
Diagnostics
```

形成：

```text
Tide Policy
      ↓
librqbit Capability
```

而不是：

```text
Tide 改写 rqbit 内部 BitTorrent 算法
```

---

# 19. librqbit Adapter

不要直接让整个项目到处调用 librqbit API。

统一：

```text
TorrentEngine
```

接口：

```text
add()
pause()
resume()
remove()
get_metadata()
get_files()
get_progress()
get_availability()
read()
set_upload_limit()
set_download_limit()
```

内部：

```text
TorrentEngine
      ↓
RqbitAdapter
      ↓
librqbit
```

这样以后 librqbit API 大改，不会污染整个项目。

特别是当前生态版本仍在快速变化，因此要：

```text
Cargo.lock 固化
+
Adapter 隔离
+
版本升级单独验证
```

当前 docs.rs 已列出 `librqbit 9.0.1`，而 GitHub README 中部分 API 示例仍显示 beta 版本号，因此 Tide 发布版本时应以实际锁定的 crate/revision 和 Cargo.lock 为准，而不是仅凭 README 版本文本判断。

---

# 20. Scheduler：Tide 的核心技术资产

这是整个项目真正值得长期维护的部分。

目标不是：

> “下载速度最大。”

而是：

> **在有限带宽、磁盘和 Peer 条件下，让用户当前最需要的数据最先可用。**

---

# 21. Scheduler 的输入

Scheduler 接收：

```text
Playback Demand
Download Demand
User Priority
Available Data
Network State
Disk State
Peer State
Task State
```

---

# 22. Priority 模型

推荐：

| Priority | 含义 |
|---|---|
| P0 | 当前正在播放的字节范围 |
| P1 | 播放器启动所需初始化数据 |
| P2 | 当前播放位置附近 buffer |
| P3 | 用户刚刚 Seek 到的位置 |
| P4 | 当前文件顺序下载 |
| P5 | 用户选中的其他文件 |
| P6 | 普通后台任务 |
| P7 | 完成/校验/低优先级后台工作 |

---

# 23. Progressive Playback Priority

不能只做：

```text
文件开头优先
+
文件结尾优先
```

这种策略太粗糙。

更合理：

```text
Playback Demand
      ↓
Media Structure
      ↓
Required Range
      ↓
Current Range
      ↓
Forward Buffer
      ↓
Background Range
```

也就是说：

> **优先级应该由“播放需要什么”决定，而不是简单由“文件位置”决定。**

---

# 24. Format-aware Priority

第一阶段至少支持识别：

```text
MP4
MKV
WebM
```

并记录：

```text
Container
Video Codec
Audio Codec
Duration
Track
Index / Seek information
Initialization region
```

然后形成：

```text
PlaybackRequirements
```

例如：

```text
MP4
 ├── initialization
 ├── metadata
 ├── sample table
 ├── current media range
 └── forward buffer
```

最终转换成：

```text
ByteRangeDemand
```

再转换：

```text
HTTP byte range
        或
Torrent piece range
```

---

# 25. Scheduler 不替代 Torrent Piece Picker

这一点必须锁死。

Tide Scheduler：

```text
决定“宏观需求”
```

librqbit：

```text
决定 Peer / Piece / Block 的实际获取
```

即：

```text
Player
 ↓
Tide Scheduler
 ↓
Demand
 ↓
librqbit
 ↓
Piece picker
 ↓
Peer
```

不要让 Tide 重新造一个完整 Torrent piece picker。

---

# 26. Playback Demand

播放器不是只告诉 Tide：

```text
我想播放 video.mp4
```

而应该告诉：

```text
current_position
current_byte_range
buffer_target
seek_target
startup_required
```

例如：

```text
Current:
820 MB

Need:
820–840 MB

Forward:
840–900 MB

Background:
900–1200 MB
```

---

# 27. Buffer 策略

建议动态 buffer：

### 启动

```text
至少可连续读取 X 秒
```

### 网络优秀

```text
扩大 buffer
```

### 网络差

```text
降低 buffer target
+
提高当前播放段优先级
```

### 用户 Seek

```text
立即取消旧 demand
+
提升新位置
+
短暂降低后台任务
```

---

# 28. Seek

必须支持：

```text
拖动进度条
±10 秒
±30 秒
任意位置
```

流程：

```text
Seek
 ↓
Cancel previous demand
 ↓
Map time → byte range
 ↓
Check availability
 ↓
Unavailable?
   ↓
Scheduler priority ↑
 ↓
Wait
 ↓
Resume playback
```

---

# 29. Progressive Stream Gateway

播放器不要直接操作：

```text
torrent file
```

而通过：

```text
Tide Local Media Gateway
```

访问：

```text
http://127.0.0.1:<random-port>/stream/<opaque-session>
```

Gateway 负责：

```text
HTTP Range
Content-Length
Content-Type
Content-Range
Accept-Ranges
Availability
Blocking
Retry
Seek
```

---

# 30. 为什么使用 Loopback Gateway

因为 WebView2 的 `<video>` 对 HTTP Range 是天然友好的。

相比：

```text
Rust
 ↓
IPC
 ↓
JS
 ↓
Blob
```

更适合：

```text
video
 ↓
HTTP Range
 ↓
Rust gateway
 ↓
Scheduler
```

同时 WebView2 官方提供 local-content/custom-scheme 等能力；Tauri 本身也提供 capability、permission、CSP 等安全机制。

---

# 31. Stream Gateway 安全模型

这是 Tide 最值得认真做安全设计的地方之一。

必须：

```text
Bind = 127.0.0.1 / ::1
```

不要默认：

```text
0.0.0.0
```

不要开放：

```text
LAN
Internet
```

---

## 31.1 Random Port

不要固定：

```text
3030
8080
3000
```

应该：

```text
OS assigned random port
```

---

## 31.2 Capability Token

每一个 Playback Session：

```text
session_id
+
unguessable capability token
```

例如：

```text
/stream/<opaque-token>
```

Token：

```text
高熵
随机
短生命周期
播放结束立即失效
```

---

## 31.3 Host Check

只接受：

```text
127.0.0.1
localhost
[::1]
```

拒绝其他 Host。

---

## 31.4 禁止路径型文件访问

绝对不要：

```text
/stream?path=C:\Users\...
```

也不要：

```text
/file/C:/Users/...
```

否则容易演变成：

```text
任意文件读取
```

正确做法：

```text
session token
 ↓
server-side session
 ↓
resource_id
 ↓
validated file/resource
```

---

# 32. Range 请求安全

播放器可能发送：

```text
Range: bytes=...
```

必须验证：

```text
start >= 0
end >= start
range <= max allowed
```

错误情况：

```text
416 Range Not Satisfiable
```

播放数据还未下载：

```text
等待
```

不能：

```text
返回完整文件
```

也不能：

```text
无限占用连接
```

应该有：

```text
stream timeout
request cancellation
max concurrent readers
```

---

# 33. Player Architecture

采用：

```text
PlayerManager
```

下面有：

```text
WebViewPlayer
NativePlayer
```

统一 API：

```text
load()
play()
pause()
seek()
stop()
set_volume()
set_speed()
get_position()
get_duration()
get_tracks()
```

---

# 34. 第一播放器：WebView2 `<video>`

优点：

```text
开发快
UI 简单
Tauri 原生集成
硬件加速
浏览器生态成熟
```

Windows/WebView2 对 H.264、VP8、VP9、AV1 等有较明确支持，但 HEVC 是条件支持；Windows 11 24H2 起 AC-3 不再默认包含，设备厂商可能另行提供，因此不能把“Windows 能播放”理解成“所有机器都能播放”。

---

# 35. M0.5 必须进行真实播放器矩阵测试

至少准备：

```text
MP4 H.264 + AAC
MP4 HEVC + AAC
MKV HEVC + AAC
MKV HEVC + AC3
MKV HEVC + FLAC
WebM VP9 + Opus
AV1 + Opus
大文件
长视频
多音轨
SRT
ASS
```

分别测试：

```text
本地完整文件
HTTP Range
HTTP Progressive
Torrent Progressive
Seek
快速 Seek
暂停/恢复
弱网
断网恢复
```

---

# 36. 第二播放器：libmpv

当 WebView2 无法覆盖目标格式时，再增加：

```text
libmpv backend
```

mpv 当前源码说明：

```text
默认：GPLv2+
-Dgpl=false：LGPLv2.1+
```

但 mpv 自身明确警告：`-Dgpl=false` 并不自动等价于获得一个完整的 LGPL 授权，最终链接的依赖也可能影响整体许可证，因此必须审计实际构建产物及依赖。

所以 Tide 的工程规则是：

> **播放器功能已经验证通过 ≠ 可以直接发布。**

必须经过：

```text
源码许可证
+
构建参数
+
依赖许可证
+
链接方式
+
分发文件
+
Notice
+
Source Offer
```

完整审查。

---

# 37. 不建议第一版自己写 FFmpeg Player

不要做：

```text
FFmpeg
+
自己解码
+
自己同步音视频
+
自己渲染
+
自己处理字幕
```

这会把 Tide 从：

```text
Downloader
```

直接拖成：

```text
Media Player Engine Project
```

完全不符合 Vibe Coding 的收益/复杂度比。

---

# 38. 媒体 Probe

统一：

```text
MediaProbe
```

负责：

```text
container
duration
resolution
fps
video codec
audio codec
tracks
subtitle
bitrate
```

第一版可以：

```text
轻量探测
+
播放器回报
```

以后才考虑：

```text
独立 ffprobe/process
```

---

# 39. 不可信媒体隔离

媒体文件不能默认视作可信输入。

特别是：

```text
MKV
MP4
WebM
ASS
SRT
封面图
字体
缩略图
```

都属于外部输入。

媒体分析如果最终使用第三方 probe：

```text
Media File
 ↓
Probe Worker
 ↓
Timeout
 ↓
Resource Limit
 ↓
JSON Result
```

而不是：

```text
主进程
直接解析任何恶意媒体
```

---

# 40. Windows Process Isolation

后期可以给 Probe / Player Worker 配合：

```text
Windows Job Object
CPU limit
Memory limit
Timeout
Process kill
No inherited secrets
Minimal environment
```

避免第三方媒体库崩溃直接影响 Tide 主进程。

---

# 41. Storage Architecture

推荐：

```text
%LOCALAPPDATA%\ifuyo\Tide\
```

结构：

```text
Tide/
├── config/
├── database/
│   └── tide.db
├── logs/
├── cache/
├── sessions/
└── temp/
```

用户下载文件：

```text
用户自选目录
```

默认：

```text
Downloads/Tide
```

---

# 42. 下载文件状态

HTTP：

```text
movie.mp4.part
movie.mp4.part.meta
```

完成：

```text
movie.mp4
```

Torrent：

```text
按照 torrent storage 管理
```

Tide 数据库只记录：

```text
task
resource
location
state
user preferences
library metadata
```

不要让 SQLite 成为大文件存储。

---

# 43. Disk Safety

必须做：

```text
free space check
```

例如：

```text
可用空间 < safety threshold
```

则：

```text
Paused: Insufficient Disk Space
```

不要等：

```text
write()
→ disk full
→ database corruption
→ task state lost
```

还需要：

```text
max download size
per-task reservation
temp cleanup
orphan cleanup
```

---

# 44. SQLite 数据模型

核心表：

```text
tasks
sources
resources
task_files
download_sessions
playback_sessions
library_items
settings
events
```

推荐：

```text
Task
 ├── Source
 ├── Resource
 ├── Files
 ├── Download Session
 └── Playback Sessions
```

---

# 45. Task State Machine

必须使用正式状态机。

```text
CREATED
  ↓
RESOLVING
  ↓
READY
  ↓
RUNNING
  ├── PAUSED
  ├── WAITING_NETWORK
  ├── WAITING_DISK
  └── ERROR
  ↓
COMPLETED
```

取消：

```text
ANY STATE
 ↓
CANCELLING
 ↓
CANCELLED
```

不要使用到处散落的：

```text
isDownloading
isPaused
isError
isCompleted
```

组合布尔值最终必然出现非法状态。

---

# 46. Download Task 与 Playback Session 分离

一个任务：

```text
Task A
```

可以：

```text
下载
+
播放
```

但播放不是 Task 生命周期的一部分。

因此：

```text
DownloadTask
```

与：

```text
PlaybackSession
```

必须分离。

这样以后可以：

```text
播放多个任务
一个任务多个播放器
一个任务播放 + 后台下载
关闭播放器但继续下载
```

---

# 47. Media Library

Tide 不是“影音刮削器”。

第一版 Library 只需要：

```text
文件
大小
路径
类型
时长
最后播放位置
收藏
```

可以使用：

```text
嵌入式 metadata
```

但不要默认根据文件名去：

```text
搜索 TMDB
搜索字幕站
搜索资源站
搜索磁力站
```

这样会逐渐把 Tide 推向“内容服务”。

---

# 48. Library UI

首页建议：

```text
Downloads
Watching
Library
```

Task：

```text
Downloading
Paused
Completed
Error
```

播放：

```text
Continue Watching
Recently Added
```

不要第一版做：

```text
影视评分
影视推荐
社区
评论
热门
排行榜
```

---

# 49. UI 总体布局

建议非常克制：

```text
┌──────────────────────────────────────┐
│ Tide                         + Add   │
├──────────────────────────────────────┤
│                                      │
│ Active                               │
│                                      │
│ ┌──────────────────────────────────┐ │
│ │ Movie.mkv       38%   32 MB/s    │ │
│ │ ████████████░░░░░░░              │ │
│ │ Downloading · Playing            │ │
│ └──────────────────────────────────┘ │
│                                      │
│ Library                              │
│                                      │
└──────────────────────────────────────┘
```

---

# 50. Add Flow

点击：

```text
+
```

弹出：

```text
Add Download
```

支持：

```text
Paste URL
Paste Magnet
Choose Torrent
Drag & Drop
```

不要：

```text
Search Web
Search Torrents
Find Movies
```

---

# 51. Drag & Drop

支持：

```text
.torrent
.magnet
URL text
media file
```

媒体文件可以：

```text
直接加入 Library
```

Torrent：

```text
添加 Task
```

URL：

```text
解析 Source
```

---

# 52. Desktop Integration

后期：

```text
System Tray
Global hotkey
Default magnet handler
Default torrent handler
Auto start
Notifications
```

但第一版不需要。

---

# 53. Notifications

只做必要状态：

```text
Download Complete
Download Error
Disk Space Low
```

不要做：

```text
“你已经一天没打开 Tide 了”
“发现 5 个热门资源”
```

Tide 的体验应该是：

> 安静地工作。

---

# 54. Tide Lab

这是开发阶段最重要的内部工具之一。

入口：

```text
Developer → Tide Lab
```

或者启动参数：

```text
--lab
```

---

# 55. Tide Lab 显示内容

```text
Torrent
 ├── Piece map
 ├── Have
 ├── Requested
 ├── Missing
 ├── Peer count
 └── Throughput

Scheduler
 ├── Current demand
 ├── Priority
 ├── Buffer target
 ├── Seek demand
 └── Queue

Player
 ├── Current byte
 ├── Range requests
 ├── Buffer
 └── Stall

Storage
 ├── Read
 ├── Write
 ├── Queue
 └── Free space
```

---

# 56. Tide Lab 强制测试模式

开发模式支持模拟：

```text
Slow Peer
Random Peer Drop
Connection Reset
HTTP 404
HTTP 429
HTTP 500
Range Failure
Disk Full
Slow Disk
Corrupt Piece
Metadata Delay
Magnet Slow Resolve
Random Seek
Rapid Seek
Pause/Resume
Network Offline
```

这样 Scheduler 才不会只是“看起来能工作”。

---

# 57. Deterministic Test Swarm

必须建立一个合法、可控、离线测试 Torrent 环境。

例如：

```text
Test Seeder A
Test Seeder B
Test Downloader
```

测试数据：

```text
公开领域文件
自制测试视频
随机二进制文件
开源素材
```

不使用：

```text
商业电影
商业电视剧
未经授权音乐
盗版游戏
```

---

# 58. 合规测试数据原则

仓库：

```text
tests/fixtures/
```

只能放：

```text
自制
Public Domain
明确开放许可证
```

不要因为：

> “只是测试”

而把版权内容直接放进 Git 仓库。

---

# 59. Security Threat Model

Tide 至少考虑以下攻击面：

| 攻击面 | 主要风险 |
|---|---|
| URL | SSRF / 重定向 |
| Torrent metadata | 恶意输入 |
| Media file | Parser exploit |
| Local Gateway | 未授权本地读取 |
| Path | Directory Traversal |
| WebView | XSS / IPC abuse |
| Tauri commands | 权限提升 |
| SQLite | 状态损坏 |
| Disk | Disk Exhaustion |
| Logs | 隐私泄露 |
| Auto update | Supply Chain |
| npm dependencies | Supply Chain |
| Rust dependencies | Supply Chain |

---

# 60. Tauri Security

Tauri 的 capability 系统可以针对不同窗口/WebView 精细授予权限；官方也强调，capability 只能减少前端被攻破后的影响，不能替代 Rust 代码本身的输入检查。

因此 Tide 采用：

```text
Frontend
 ↓
Minimal Capability
 ↓
Explicit Command
 ↓
Rust Validation
 ↓
Business Logic
```

而不是：

```text
Frontend
 ↓
万能 filesystem command
```

---

# 61. Tauri Capability 原则

主窗口只获得：

```text
window
event
dialog
selected filesystem scope
task commands
player commands
```

不要给予：

```text
任意路径读写
任意 shell
任意 process execute
任意 URL navigation
```

尤其不要：

```text
shell:allow-all
fs:allow-all
```

---

# 62. CSP

CSP 必须显式配置。

原则：

```text
self
Tauri IPC
Tide Media Gateway
必要 blob/media
```

默认拒绝：

```text
任意远程 script
任意 iframe
任意 CDN
```

Tauri 官方也建议配置严格 CSP，并避免加载远程脚本，因为不受信任内容会扩大 WebView 攻击面。

---

# 63. 文件系统安全

用户选择：

```text
Download Directory
```

以后所有路径操作都必须通过：

```text
Path Normalizer
```

检查：

```text
canonical path
parent containment
symlink/reparse point
reserved Windows names
illegal filename
path length
extension
```

永远不要：

```text
拼接用户字符串
直接 fs::write(path)
```

---

# 64. 文件名 Sanitization

Torrent 可以出现恶意文件名，例如：

```text
..\..\secret.txt
CON
NUL
AUX
C:\...
```

Tide 必须：

```text
sanitize
normalize
contain
```

最终只能写入：

```text
configured output root
```

---

# 65. Symlink / Reparse Point

Windows 上不要只做：

```text
path.starts_with(download_dir)
```

需要实际检查：

```text
canonical path
reparse point
link target
```

防止：

```text
下载目录
 ↓
恶意 junction
 ↓
C:\Users\...
```

---

# 66. Media Gateway 与文件系统完全解耦

Gateway 不应该收到：

```text
C:\foo\bar.mkv
```

而应该收到：

```text
PlaybackSessionId
```

然后：

```text
PlaybackSession
 ↓
ResourceId
 ↓
Authorized Storage Handle
```

这是安全设计的关键。

---

# 67. 网络安全

Tide 不提供 VPN，但允许用户选择网络接口时，可以设计：

```text
Network Interface
 ├── Auto
 ├── Wi-Fi
 ├── Ethernet
 └── VPN Adapter
```

这里的意义是：

> 网络路径选择。

不是：

> IP anonymity。

UI 不出现：

```text
匿名
隐身
不可追踪
```

---

# 68. Private Torrent

检测到：

```text
private torrent
```

必须尊重其 private flag。

不要：

```text
强制 DHT
强制 PEX
主动发现公共 Peer
```

不要让 UI 提供破坏 private torrent 语义的“一键增强”功能。

---

# 69. UPnP / mDNS

rqbit 本身支持 UPnP Media Server 和 mDNS。

但 Tide 第一版：

```text
关闭
```

后期如加入：

```text
LAN Streaming
```

必须：

```text
Explicit opt-in
```

并在设置页明确：

> 允许局域网设备访问 Tide 媒体。

因为一旦打开，就改变了最初：

```text
Localhost Only
```

的安全边界。

---

# 70. No Cloud Architecture

Tide 不需要服务器。

完整下载链路：

```text
Tide
 ↓
Internet
 ↓
Source
```

或：

```text
Tide
 ↓
BitTorrent
 ↓
Peers
```

不存在：

```text
Tide
 ↓
ifuyo Server
 ↓
Source
```

所以：

```text
无需账号
无需数据库服务
无需对象存储
无需 CDN
无需后端下载节点
无需云端任务队列
```

---

# 71. 更新服务不等于业务云服务

以后可以有：

```text
Update Check
```

但它不承载：

```text
下载任务
文件
用户数据
Torrent
```

只是：

```text
版本检查
```

---

# 72. Analytics

默认：

```text
No analytics
```

如果以后真的要：

```text
crash reporting
```

也应：

```text
opt-in
或
最小匿名化
```

并明确说明发送哪些信息。

---

# 73. 许可证总体策略

Tide 应建立：

```text
THIRD_PARTY_NOTICES
LICENSE_AUDIT.md
SBOM
```

每个版本做：

```text
cargo tree
npm dependency audit
license scan
binary dependency scan
```

---

# 74. librqbit 许可

当前 rqbit 仓库 LICENSE 文件标明 Apache License 2.0。

因此：

```text
librqbit
```

在产品里属于相对清晰的开源依赖。

但是仍然要：

```text
保留 LICENSE
保留版权声明
检查传递依赖
生成 Third Party Notices
```

---

# 75. mpv 许可

不能简单写：

> “mpv 是 LGPL。”

更准确：

```text
默认 GPLv2+
-Dgpl=false → LGPLv2.1+
```

但实际构建中仍可能涉及其他许可证，官方源码也特别指出链接库的许可证可能影响最终分发。

因此：

```text
MPV = 第二阶段许可证审计项目
```

而不是 M0 就拍脑袋集成。

---

# 76. GitHub / Steam 发布合规

以后如果发布：

```text
GitHub
Steam
官网
```

都应该随安装包提供：

```text
LICENSE
THIRD_PARTY_NOTICES
OPEN_SOURCE_LICENSES/
```

同时 README 包含：

```text
Legal Use
Privacy
Security
Source
License
```

---

# 77. 产品法律声明

建议使用这样的中性表述：

> Tide 不提供、搜索、索引或推荐未经授权的版权内容。Tide 仅处理用户主动提供的来源，并不判断用户是否有权下载、复制或传播特定内容。用户应仅使用自己拥有或被授权获取、复制或分发的内容。不同司法辖区对于版权及网络传播行为的规定可能不同。

比下面这种写法更合适：

> “我们保证 Tide 下载任何资源都合法。”

后者无法成立。

---

# 78. README 合规声明

建议：

```text
Tide is a local downloader and media player.

Tide does not provide:
- content search
- torrent indexing
- pirate resource discovery
- copyrighted-content catalogs
- DRM bypass

Users are responsible for ensuring that they have
the necessary rights to download, copy, store,
play, or distribute the content they use with Tide.
```

---

# 79. Abuse / Copyright 联系方式

尽管 Tide 不托管用户内容，也不建立内容服务，但正式产品仍建议准备：

```text
legal@ifuyo.com
security@ifuyo.com
```

以及：

```text
Copyright / Legal
Security Vulnerability
```

两个独立入口。

目的不是承认：

> “我们托管用户内容。”

而是建立成熟的软件项目责任边界。

---

# 80. 数据库恢复

Tide 必须能够：

```text
Crash
Kill Process
Power Loss
```

之后恢复：

```text
Task
Progress
Pause State
Download Path
Playback Position
```

启动流程：

```text
Load DB
 ↓
Validate Tasks
 ↓
Inspect Storage
 ↓
Reconcile
 ↓
Recover
 ↓
Resume Eligible Tasks
```

---

# 81. Crash Recovery

例如：

```text
Task says 70%
Disk actually 68%
```

不能盲信 SQLite。

应该：

```text
Engine truth
+
Storage truth
+
DB metadata
```

三者 reconcile。

Torrent 则优先依赖：

```text
librqbit verified state
```

而不是 Tide 自己猜 Piece 状态。

---

# 82. Download Speed Model

Tide 不需要人为做：

```text
Free = 1 MB/s
Premium = 50 MB/s
```

也不需要服务器。

下载速度主要受：

```text
Source
Peer count
Peer capacity
Network
Disk
Scheduler
```

影响。

所以产品定位不是：

> “付费购买下载带宽。”

而是：

> “让本机的下载资源得到更好的调度。”

---

# 83. Speed Optimization

第一阶段：

```text
Connection reuse
HTTP Range
Concurrent requests
Torrent parallelism
Buffering
Scheduler
```

第二阶段：

```text
Adaptive concurrency
Bandwidth allocation
Per-task priority
Disk QoS
```

第三阶段：

```text
Network quality estimation
Peer quality scoring
Dynamic buffer
```

---

# 84. 不做“奇技淫巧”式加速

不采用：

```text
所谓破解 VIP
盗用第三方加速服务
绕过服务端限制
隐藏真实来源
```

Tide 的性能资产必须来自：

```text
合理调度
本地并发
缓存
Range
磁盘优化
```

---

# 85. Task Priority

用户可以设置：

```text
High
Normal
Low
```

Scheduler 转换成：

```text
Task Weight
```

例如：

```text
正在观看：100
用户手动指定：80
普通：50
后台：20
```

最终再结合：

```text
Playback Demand
```

形成实际优先级。

---

# 86. 多任务调度

例如：

```text
Task A
Task B
Task C
```

用户播放：

```text
Task B
```

则：

```text
B Playback = P0
A = Normal
C = Low
```

播放停止后：

```text
B Playback demand 消失
```

恢复普通下载调度。

---

# 87. 网络带宽控制

设置：

```text
Global Download Limit
Global Upload Limit
Per Task Limit
```

支持：

```text
Unlimited
10 MB/s
5 MB/s
1 MB/s
Custom
```

---

# 88. 定时任务

Tide 很适合做真正实用的 Scheduler：

```text
Start at 02:00
Pause at 08:00
Download only on weekdays
Limit bandwidth during daytime
```

但第一版只做：

```text
Manual
Global limit
Per task limit
```

定时调度放后面。

---

# 89. Playback UI

播放时显示：

```text
Title
Progress
Buffer
Speed
Resolution
Audio Track
Subtitle
```

隐藏复杂技术信息。

只有：

```text
Tide Lab
```

才显示：

```text
Piece
Peer
Range
Scheduler
```

---

# 90. Player UX

必须支持：

```text
Space = Play/Pause
← / →
Seek
↑ / ↓
Volume
M
Mute
F
Fullscreen
```

双击：

```text
Fullscreen
```

并支持：

```text
Picture-in-Picture
```

作为后期功能。

---

# 91. Subtitle

第一版：

```text
Embedded Subtitle
SRT
```

ASS：

```text
M0.5
```

实际验证。

复杂 ASS 渲染依赖播放器后端，不应该自己实现字幕渲染器。

---

# 92. Resume Playback

Library 保存：

```text
position_ms
```

再次打开：

```text
Continue from 01:24:31?
```

但只保留本地：

```text
SQLite
```

不上传服务器。

---

# 93. Thumbnail

第一版可以：

```text
没有缩略图
```

第二阶段：

```text
first-frame thumbnail
```

第三阶段：

```text
smart thumbnail
```

不应该为了漂亮 UI 在 M0 就引入一整套媒体转码系统。

---

# 94. Vibe Coding 核心原则

Tide 非常适合 AI 辅助开发，但必须控制 AI 生成的复杂度。

每次 AI 实施：

```text
一个垂直可运行增量
```

而不是：

```text
“帮我把 Scheduler 全部实现。”
```

---

# 95. AI 开发边界

每个 Milestone 都必须：

```text
Build
Run
Test
Verify
```

不能接受：

```text
代码写完
编译成功
所以完成
```

---

# 96. AI 生成代码的架构规则

任何 AI agent 都必须遵守：

```text
No speculative abstraction
No duplicate protocol implementation
No global mutable state unless justified
No unwrap/expect in runtime paths
No arbitrary filesystem access
No arbitrary shell execution
No giant god object
No frontend business logic duplicating Rust state
```

---

# 97. Error Handling

Rust runtime：

```text
Result<T, TideError>
```

统一错误：

```text
SourceError
NetworkError
StorageError
TorrentError
PlaybackError
SchedulerError
SecurityError
PersistenceError
```

前端：

```text
error_code
message
recoverability
```

而不是直接把：

```text
Rust panic
```

显示给用户。

---

# 98. Observability

统一：

```text
tracing
```

事件：

```text
task.created
task.started
task.paused
task.failed
task.completed

playback.started
playback.stalled
playback.seeked

scheduler.demand_added
scheduler.demand_cancelled

torrent.peer_connected
torrent.piece_completed
```

Production 默认：

```text
info
```

Lab：

```text
debug / trace
```

---

# 99. 性能目标

目标不是跑分，而是：

### Idle

```text
低 CPU
低磁盘
低网络
```

### Download

```text
尽可能接近可用带宽
```

### Playback

```text
稳定
低 Stall
Seek 快
```

### Memory

```text
避免按文件大小增长
```

媒体文件：

```text
10 GB
100 GB
```

都不应导致：

```text
10 GB RAM
100 GB RAM
```

---

# 100. 大文件原则

全项目禁止：

```text
read_to_end()
```

读取巨大媒体文件。

必须：

```text
stream
range
mmap only when justified
bounded buffer
```

---

# 101. Cache Strategy

Tide 不需要为每个组件各自造缓存。

统一考虑：

```text
Storage
 ↓
OS Cache
 ↓
Tide Buffer
```

不要做：

```text
HTTP Cache
+
Torrent Cache
+
Player Cache
+
Scheduler Cache
+
Library Cache
```

最后五套缓存互相打架。

---

# 102. Aura 与 Tide 是否共享 Cache

第一版：

> **不共享。**

Aura 的 Chunk/Range/LRU cache 与 Tide Torrent/Piece/Playback storage 虽然概念类似，但底层语义并不一样。

Tide：

```text
piece hash
multi-file mapping
sparse file
torrent verification
```

Aura：

```text
HTTP range
cache eviction
```

现在强行抽象：

```text
ifuyo-core
```

很容易制造过度工程。

只有以后真的出现稳定重复逻辑，再抽：

```text
ifuyo-core
```

例如：

```text
logging
config
diagnostics
updater
```

---

# 103. M0 — Skeleton

目标：

```text
Tauri
+
React
+
Rust
+
SQLite
```

完成：

```text
Main Window
Settings
Task model
IPC
SQLite
Error handling
```

验收：

```text
启动
创建 task
保存 task
关闭
重启
task 仍存在
```

---

# 104. M0.5 — Technology Spike

这是整个项目最重要的技术闸门。

不能跳过。

建立：

```text
Tide Lab
```

验证：

```text
HTTP Range
Torrent
Magnet
librqbit
Progressive Stream
WebView2 video
Seek
```

播放器矩阵全部测试。

同时测试：

```text
MP4
MKV
WebM
HEVC
AC3
AV1
字幕
大文件
```

最终拍板：

```text
WebView2 only
```

或：

```text
WebView2 + libmpv fallback
```

---

# 105. M1 — HTTP Download Core

实现：

```text
URL
Resolve
Download
Pause
Resume
Cancel
Range
Retry
Error
```

验收：

```text
100MB
1GB
大文件
网络断开
恢复
服务器支持/不支持 Range
```

---

# 106. M2 — Task Persistence

完成：

```text
SQLite
State Machine
Crash Recovery
Resume
Download Directory
Disk Check
```

验收：

```text
杀进程
重启电脑
磁盘断满
任务暂停
任务恢复
```

---

# 107. M3 — Torrent Engine

完成：

```text
Magnet
Torrent
Metadata
librqbit Adapter
Piece Progress
Private Torrent
DHT
Pause
Resume
```

验收使用：

```text
合法测试 Torrent
```

---

# 108. M4 — Progressive Streaming

完成：

```text
Media Gateway
Range
Playback Session
Availability
Scheduler
Buffer
Seek
```

目标：

```text
Torrent 30% 下载
→ 可以播放
```

以及：

```text
下载 20%
→ Seek
→ Scheduler 获取目标位置
→ 继续播放
```

---

# 109. M5 — Player

完成：

```text
Video
Audio
Seek
Subtitle
Volume
Fullscreen
Resume
Playback Position
```

如果 M0.5 决定使用：

```text
libmpv
```

此时才引入。

---

# 110. M6 — Library

完成：

```text
Library
History
Continue Watching
Favorites
Folders
Metadata
```

仍然：

```text
Local only
```

---

# 111. M7 — Smart Scheduler

加入：

```text
Adaptive Buffer
Task Priority
Bandwidth Allocation
Playback Demand
Format-aware Priority
Disk QoS
```

这是真正体现 Tide 技术特色的版本。

---

# 112. M8 — Windows Integration

加入：

```text
Tray
Magnet association
Torrent association
Auto start
Notifications
Drag & Drop
System media integration
```

---

# 113. M9 — Polish

重点：

```text
UI
Animations
Keyboard shortcuts
Settings
Error UX
Accessibility
Performance
```

---

# 114. M10 — Security & Release Hardening

完成：

```text
Threat Model
Security Test
Fuzzing
Path Test
Gateway Test
Media Probe isolation
Dependency Audit
License Audit
CSP
Capability Audit
Crash Recovery
```

---

# 115. M11 — Release Candidate

完成：

```text
installer
updater
uninstall
portable build
logs
diagnostics
third-party notices
license
privacy
legal use
security contact
```

---

# 116. M12 — 1.0

1.0 的定义不是：

> “所有想到的功能都有。”

而是：

```text
HTTP 下载稳定
Torrent 稳定
Progressive Playback 稳定
Seek 稳定
Task Recovery 稳定
安全边界稳定
许可证清晰
UI 稳定
```

---

# 117. 必须拥有的自动化测试

## Core

```text
Task State Machine
Scheduler
Range Mapping
Availability
Persistence
```

## HTTP

```text
Range
Redirect
Timeout
Retry
429
5xx
Resume
```

## Torrent

```text
Piece
Integrity
Resume
Private Torrent
Peer Drop
```

## Gateway

```text
Range
416
Auth
Host
Path Traversal
Concurrency
Timeout
```

## Security

```text
Malicious path
Malformed URL
Redirect
Private IP
IPv6
Symlink
Huge Range
Huge Header
Malformed media metadata
```

---

# 118. Scheduler 单元测试

例如：

```text
Input:
Playback = 500MB
Buffer = 500-550MB
Background = 550-2000MB

Expected:
500-550MB > background
```

Seek：

```text
Old = 500MB
New = 1400MB

Expected:
1400MB priority ↑
old demand cancelled
```

多任务：

```text
Task A = background
Task B = playback
Task C = user high priority

Expected:
B > C > A
```

---

# 119. Fuzz Testing

推荐对：

```text
Torrent metadata
Bencode
URL parser
Range parser
Path sanitizer
Media metadata
```

做 fuzz。

尤其：

```text
Range
```

和：

```text
Path
```

绝对值得测试。

---

# 120. Security Acceptance Gate

任何版本不能进入 Release Candidate，除非满足：

```text
[ ] Gateway localhost-only
[ ] Random port
[ ] Session capability
[ ] Host validation
[ ] No arbitrary file path
[ ] Safe URL parser
[ ] Redirect validation
[ ] Private IP handling
[ ] Path traversal tests
[ ] Reparse/symlink tests
[ ] Disk exhaustion protection
[ ] Media process isolation
[ ] CSP
[ ] Tauri capability audit
[ ] Production logs redacted
[ ] No telemetry by default
```

---

# 121. Compliance Acceptance Gate

```text
[ ] No resource search
[ ] No pirate index
[ ] No content catalog
[ ] No piracy recommendation
[ ] No DRM bypass
[ ] No paywall bypass
[ ] User-provided source model
[ ] Legal-use documentation
[ ] Torrent sharing policy
[ ] Download-only option
[ ] No anonymous-download marketing
[ ] Third-party licenses documented
```

---

# 122. Release 文案基调

不要：

> 最强磁力下载器

不要：

> 极速看片神器

不要：

> 海量资源免费下载

不要：

> 免费解锁所有影视资源

也不要：

> 匿名安全下载

---

更合适：

> **Tide — Download. Watch. Flow.**

或者：

> **A calm, local-first downloader and media player.**

中文：

> **Tide —— 下载，也可以马上播放。**

或者：

> **你提供来源，Tide 负责把它变成一个流畅的下载与播放体验。**

---

# 123. Tide 的真正技术卖点

不是：

```text
“我也能下载”
```

而是：

```text
Download
    +
Scheduler
    +
Progressive Playback
    +
Range Seeking
    +
Local-first
```

最终体验：

```text
Paste Magnet
       ↓
Metadata ready
       ↓
Start downloading
       ↓
Playable
       ↓
Play
       ↓
User seeks
       ↓
Scheduler reprioritizes
       ↓
Continue
       ↓
Background completion
```

这是 Tide 与普通“下载器 GUI”的差异。

---

# 124. Tide 的核心架构资产

如果项目最后只有三块值得长期保留，我认为应该是：

### ① Unified Resource / Availability Model

```text
HTTP Range
Torrent Piece
        ↓
Byte Availability
```

---

### ② Scheduler

```text
Demand
 ↓
Priority
 ↓
Allocation
 ↓
Playback
```

---

### ③ Progressive Media Gateway

```text
Player
 ↓
Range
 ↓
Resource
 ↓
Scheduler
 ↓
Network
```

这三部分形成闭环：

```text
Player
  ↑
  │
Gateway
  ↑
  │
Scheduler
  ↑
  │
Availability
  ↑
  │
HTTP / Torrent
```

这就是 Tide 最核心的技术设计。

---

# 125. 最终项目边界

最终 Tide 应该是：

```text
                 IFUYO TIDE
                      │
        ┌─────────────┴─────────────┐
        │                           │
     DOWNLOAD                    PLAYBACK
        │                           │
   ┌────┴────┐                ┌─────┴─────┐
   │         │                │           │
  HTTP      BT             WebView2     mpv*
   │         │                │           │
   └────┬────┘                └─────┬─────┘
        │                           │
        └──────────┬────────────────┘
                   ▼
               SCHEDULER
                   │
                   ▼
              AVAILABILITY
                   │
                   ▼
                STORAGE
```

---

# 126. 最终产品原则

### 原则一：用户带来源，Tide 不找内容

这是合规边界，也是产品边界。

### 原则二：Torrent 是协议，不是内容生态

不因为支持 BT，就去做资源搜索、索引、推荐。

### 原则三：不承诺匿名

可以做隐私保护，但不伪装成匿名网络工具。

### 原则四：下载和播放必须统一调度

否则“边下边播”永远只是一个 Demo。

### 原则五：Scheduler 是 Tide 的核心

播放器、HTTP、BT 都围绕它服务。

### 原则六：播放器后端可替换

WebView2 先行，格式覆盖不足再引入 libmpv；许可证必须与技术选择一起审查。

### 原则七：本地优先

无需账号、无需服务器、无需云端文件中转。

### 原则八：安全边界从第一天建立

尤其是：

```text
URL
Filesystem
Media
Gateway
IPC
```

### 原则九：不为了“功能全面”破坏产品边界

Tide 不需要变成：

```text
浏览器
影视资源站
NAS
媒体服务器
VPN
下载加速平台
云盘
```

---

# 127. 总体结论

Tide 的最佳形态不是“重新造一个 qBittorrent”。

也不是：

> “做一个带播放器的下载器。”

而是：

> **围绕“数据需求调度”重新设计下载与播放之间的关系。**

最终用户看到的只是：

```text
Paste
→ Download
→ Play
→ Seek
→ Continue
→ Done
```

但底层真正运行的是：

```text
Source
 ↓
Resource
 ↓
Availability
 ↓
Demand
 ↓
Scheduler
 ↓
Range/Piece
 ↓
Storage
 ↓
Gateway
 ↓
Player
```

这套结构同时满足：

```text
Vibe Coding
+
Rust 工程能力
+
Tauri 桌面应用
+
BitTorrent
+
HTTP
+
边下边播
+
高安全边界
+
隐私优先
+
无云架构
+
可持续扩展
```

**推荐将本方案作为 Tide 的总体基线。M0 之前不要继续堆功能，真正应该先完成的是 M0 → M0.5，尤其是 M0.5 的 Torrent → Availability → Scheduler → Range → WebView2 实际播放链路。**

在合规层面，目标也不是声称“任何用户怎样使用 Tide 都绝对不会产生版权风险”，而是让 **Tide 本身不建立侵权内容供应链、不提供盗版发现与分发服务、不绕过 DRM，并把用户授权责任与工具本身清晰分离**。不同司法辖区的具体法律责任仍需按实际发布地区和产品运营方式单独评估。中国现行著作权法及最高法相关司法解释、欧盟 C-597/19 判决、美国版权局 FAQ 都表明，未经授权的复制/传播以及 P2P 中的自动上传行为可能涉及版权责任，因此 Tide 的 Download Only / Sharing Policy 和“用户提供来源”边界不应省略。

另外，Windows 11 应作为正式目标平台也比较合理：Windows 10 已于 2025 年 10 月 14 日结束常规支持，微软目前建议迁移到 Windows 11。