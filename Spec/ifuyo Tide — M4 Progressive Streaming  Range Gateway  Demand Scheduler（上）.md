# ifuyo Tide — M4 Progressive Streaming / Range Gateway / Demand Scheduler（上）

> **拆分说明（必读）**：本文件是 M4 完整开发实施提示词的**上半部分（第 0~97 节：架构与核心实现 —— Resource / Availability / Demand 抽象、HTTP 与 Torrent Resource、Stream Gateway、PlaybackSession、Scheduler 联动、持久化与稳定性）**。错误模型、安全、测试矩阵、性能与最终 DoD 位于下半部分《ifuyo Tide — M4 Progressive Streaming  Range Gateway  Demand Scheduler（下）.md》。两部分必须配合使用：先完成上半部分，再执行下半部分；只完成上半部分不构成 M4 完成。

**阶段：M4**
**项目：ifuyo Tide**
**目标平台：Windows 11 x64**
**技术基线：Tauri 2 + Rust + TypeScript/React + SQLite + WebView2**
**前置阶段：M0 / M0.5 / M1 / M2 / M3**

---

# 0. 本阶段定位

M4 不是简单增加一个 `<video>` 播放器，也不是给 rqbit 包一层 HTTP URL。

M4 的核心任务是把 Tide 前面已经建立的：

```text
Source
↓
Resource
↓
Availability
↓
Download Engine / Torrent Engine
```

真正连接到：

```text
Demand
↓
Scheduler
↓
Readable Resource
↓
Progressive Stream Gateway
↓
WebView2 Player
```

形成 Tide 第一条真正完整的：

```text
用户播放意图
→ Demand
→ Scheduler
→ Resource
→ Availability
→ 等待/获取所需字节
→ Local HTTP Range Gateway
→ WebView2
→ 播放 / Pause / Seek
```

最终目标不是“播放器能播”，而是：

> **播放器不需要知道底层资源来自 HTTP 还是 BitTorrent。播放器只看到一个本地、受保护、支持 Range 的媒体 URL；系统内部负责判断哪些字节现在可用、哪些字节需要等待、哪些字节应该优先获取。**

---

# 1. 最重要的架构原则

必须把 M4 做成 Tide 的核心架构阶段之一。

以后 Tide 的业务核心不能建立在：

```text
rqbit streaming URL
```

或者：

```text
某个 HTTP 下载器的临时文件路径
```

之上。

最终必须抽象为：

```text
Playback
    ↓
PlaybackSession
    ↓
Demand
    ↓
Scheduler
    ↓
ReadableResource
    ├── HttpResource
    └── TorrentResource
    ↓
Availability + Read
    ↓
Storage / Torrent Engine
```

外部播放器只能看到：

```text
http://127.0.0.1:<random-port>/stream/<opaque-token>
```

而不能直接知道：

* 本地真实路径
* Torrent 文件路径
* Torrent ID
* InfoHash
* Peer
* Magnet
* 数据库 ID
* 任意文件系统路径

---

# 2. 执行规则：必须先审计，不允许盲改

开始实现之前，必须先完整检查当前仓库实际状态。

至少检查：

```text
M0 architecture
M1 HTTP implementation
M2 persistence/recovery
M3 torrent/librqbit integration
Cargo.toml
Cargo.lock
Tauri capabilities
IPC commands
SQLite migrations
current tests
current frontend architecture
```

尤其确认：

```text
Resource
Availability
Task
PlaybackSession
Scheduler
Storage
DownloadEngine
TorrentEngine
```

目前到底已经实现到什么程度。

---

# 3. 开始修改前必须输出以下报告

先不要修改代码。

输出：

```text
FACTS
HYPOTHESES
CURRENT M0 STATE
CURRENT M1 STATE
CURRENT M2 STATE
CURRENT M3 STATE
CURRENT RESOURCE MODEL
CURRENT TORRENT MODEL
CURRENT STORAGE MODEL
CURRENT IPC MODEL
CURRENT FRONTEND PLAYER STATE
M4 IMPLEMENTATION GAPS
M4 IMPLEMENTATION PLAN
M4 RISKS
```

明确区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

不要把猜测写成事实。

---

# 4. M4 的最终目标

M4 完成以后，至少应实现下面这条完整链：

```text
HTTP Resource
    ↓
Download / Range Fetch
    ↓
Availability
    ↓
ReadableResource
    ↓
Playback Demand
    ↓
Scheduler
    ↓
Local Range Gateway
    ↓
WebView2 <video>
```

以及：

```text
Torrent Resource
    ↓
librqbit
    ↓
Availability
    ↓
ReadableResource
    ↓
Playback Demand
    ↓
Scheduler
    ↓
Local Range Gateway
    ↓
WebView2 <video>
```

要求：

### HTTP

能够：

* 边下载边播放
* 已下载部分立即播放
* Playback 对尚未到达的范围等待
* seek 到其他位置后产生新的 Demand
* Scheduler 提升所需范围优先级
* 下载失败可以恢复
* 播放 session 不破坏 Download Task 生命周期

### Torrent

能够：

* Torrent 还没有完成时开始播放
* 当前缺少的 piece 可以等待
* 播放范围被优先请求
* seek 到其他位置以后产生新的 Demand
* 不要求整个 Torrent 完成以后才能播放
* 不把 rqbit 私有 API 泄露给前端

---

# 5. M4 明确不做的事情

以下内容本阶段禁止做：

```text
字幕系统
音轨切换
倍速系统
播放器皮肤大规模重做
缩略图
章节系统
媒体库
NAS
UPnP
DLNA
Chromecast
AirPlay
局域网投送
mDNS
VPN
代理
匿名下载
云服务
账号
云同步
遥测
Steam
Workshop
推荐系统
资源搜索
资源索引
资源站导航
Tracker 推荐
Peer 推荐
DRM 绕过
付费内容绕过
```

以下也不属于 M4：

```text
高级智能调度
机器学习调度
自适应码率
视频转码
FFmpeg 全套媒体管线
libmpv 默认集成
硬件转码
视频编辑
```

M4 只解决：

> **可靠的渐进式字节读取 + Demand + 基础调度 + 安全本地 Range Gateway + WebView2 播放链。**

---

# 6. 最核心的统一抽象

必须确认并统一：

```text
Resource
Availability
Demand
ReadableResource
PlaybackSession
```

---

# 7. Resource

Resource 表示一个可读取的逻辑数据资源。

建议模型：

```text
ResourceId
Source
Name
Size
ContentType
Identity
```

其中：

```text
Size = Some(u64)
```

或者：

```text
Size = None
```

不要假设所有资源都已知最终长度。

---

# 8. ReadableResource

建立统一读取接口。

概念上至少需要：

```text
read(range) -> stream/read result
availability(range) -> availability result
size() -> Option<u64>
content_type() -> Option<String>
identity() -> ResourceIdentity
```

可以根据现有代码选择：

```text
trait ReadableResource
```

或者：

```text
ResourceReader
```

但必须保证：

> HTTP 与 Torrent 在播放器看来是同一个读取模型。

---

# 9. ByteRange

必须继续使用：

```text
[start, end)
```

即左闭右开。

例如：

```text
0..1024
```

表示：

```text
bytes 0 through 1023
```

所有系统内部范围统一使用这一模型：

```text
HTTP
Availability
Demand
Storage
Scheduler
Gateway
Playback
```

不要在内部混用：

```text
[start, end)
[start, end]
```

---

# 10. Availability

M4 必须正式把 Availability 从“概念”提升为实际核心能力。

至少支持：

```text
Available
PartiallyAvailable
Unavailable
Complete
```

更推荐直接用连续区间：

```text
AvailabilitySet
```

例如：

```text
[0, 4MB)
[8MB, 16MB)
[32MB, 48MB)
```

而不是只保存：

```text
downloaded_bytes = 16MB
```

---

# 11. 为什么必须是区间 Availability

Torrent 天然不是只有：

```text
0 → X
```

可能是：

```text
0–4MB available
4–8MB unavailable
8–16MB available
```

HTTP 在 M4 以后也可能出现：

```text
0–20MB available
20–40MB requested
60–80MB available
```

因此不要设计成：

```text
downloaded_bytes: u64
```

然后假设：

```text
0..downloaded_bytes
```

永远是唯一可用区间。

---

# 12. Availability API

至少设计：

```text
is_available(range)
available_subranges(range)
missing_subranges(range)
contiguous_available_from(offset)
```

例如：

```text
requested = [10MB, 30MB)
available =
    [10MB, 20MB)
    [25MB, 30MB)
```

那么：

```text
missing =
    [20MB, 25MB)
```

必须能够正确表达。

---

# 13. Read(range) 语义

`read(range)` 不允许返回错误的字节。

它必须保证：

```text
requested range
=
returned bytes
```

或者明确返回：

```text
waiting
error
cancelled
```

绝不能：

```text
请求 10MB
只返回 7MB
然后假装成功
```

---

# 14. Read 的等待机制

当请求：

```text
read([10MB, 20MB))
```

而其中：

```text
10–15MB available
15–20MB unavailable
```

必须允许：

```text
read
→ 返回 10–15MB
→ 等待 15MB+
→ 继续输出
```

或者采用内部缓冲：

```text
wait until required contiguous range
```

具体方式可以按当前 WebView2 行为和性能测试决定。

但必须记录清楚：

```text
WHAT
WHY
TRADEOFF
```

---

# 15. Demand

M4 正式建立 Demand 模型。

Demand 表示：

> 当前哪个消费者，需要哪些字节，并且需要到什么程度。

至少包含：

```text
DemandId
ResourceId
Range
Priority
Reason
CreatedAt
Deadline / urgency
PlaybackSessionId
State
```

Reason 至少支持：

```text
PlaybackInitial
PlaybackBuffer
PlaybackSeek
BackgroundDownload
Recovery
```

---

# 16. Demand Priority

第一版不要搞几十个优先级。

建议简单分层：

```text
Critical
High
Normal
Low
```

默认：

```text
PlaybackSeek      = Critical
PlaybackInitial   = Critical
PlaybackBuffer    = High
ForegroundDownload = Normal
Background        = Low
```

具体值根据实际测试调整。

不要过早设计复杂评分系统。

---

# 17. Scheduler 的定位

M4 实现的是：

> **最小可用、真实有效的 Demand Scheduler。**

不是 M7 的 Smart Scheduler。

M4 Scheduler 只解决：

```text
谁现在最需要带宽？
哪个 Resource 正在被播放？
哪个 Range 是播放器马上要用的？
哪些后台任务可以让路？
```

---

# 18. M4 Scheduler 必须具备的能力

至少支持：

```text
submit_demand()
cancel_demand()
reprioritize()
complete_demand()
get_pending_demands()
```

并且能够：

```text
Playback > Foreground Download > Background
```

---

# 19. Scheduler 不应该直接处理协议细节

禁止：

```text
Scheduler → reqwest
Scheduler → rqbit internal API
```

应该：

```text
Scheduler
    ↓
Resource Provider
```

即：

```text
Scheduler
    ↓
Resource
    ↓
HttpResource / TorrentResource
```

这样以后添加：

```text
LocalFileResource
NASResource
```

不会污染 Scheduler。

---

# 20. HTTP Resource

M4 必须把 M1 HTTP Downloader 演进为：

```text
HttpResource
```

它不仅是：

```text
DownloadTask
```

还必须支持：

```text
read(range)
availability(range)
```

---

# 21. HTTP Resource 的基本策略

M4 不允许重新写一个完全独立的 HTTP 下载器。

应最大程度复用 M1：

```text
HttpTransport
Retry
Validator
Range parsing
FileSink
Checksum
Persistence
```

新增：

```text
RangeDemand
HttpRangeFetcher
AvailabilityStore
```

形成：

```text
M1 Download Engine
+
M4 Range Fetching
```

---

# 22. HTTP Range 获取

对于：

```text
missing = [20MB, 30MB)
```

可以发：

```http
GET /resource
Range: bytes=20971520-31457279
```

服务器如果返回：

```text
206 Partial Content
Content-Range: bytes 20971520-31457279/...
```

才可以安全写入对应范围。

---

# 23. HTTP 服务器返回 200 的处理

这是 M4 的 P0 级正确性要求之一。

例如请求：

```http
Range: bytes=20MB-
```

服务端却返回：

```http
200 OK
```

绝不能：

```text
把整个 body 写入 20MB 位置
```

更不能覆盖已有内容。

正确行为必须是：

```text
detect Range ignored
→ 不可安全作为该 Range 响应
→ 根据当前 Resource 策略决定：
   fallback sequential download
   / restart
   / retry
   / fail
```

必须有明确测试。

---

# 24. HTTP 416 的处理

如果请求：

```text
Range: bytes=start-end
```

返回：

```text
416 Range Not Satisfiable
```

必须检查：

```text
Content-Range
```

与当前资源长度。

不能把：

```text
416
```

当成普通网络错误无限 retry。

---

# 25. HTTP Validator

M4 必须延续 M1 的：

```text
ETag
Last-Modified
If-Range
```

尤其是已经缓存了一部分内容后再次请求新 Range。

必须确认：

```text
same representation
```

否则禁止把：

```text
旧数据
+
新数据
```

组合成一个资源。

---

# 26. HTTP Resource 的 Availability

HTTP Resource 最终应该能够表达：

```text
Available ranges
Requested ranges
In-flight ranges
Failed ranges
```

建议逻辑上至少区分：

```text
available
pending
missing
```

不要把：

```text
requested
```

直接当成：

```text
available
```

---

# 27. Concurrent Range Fetch

M4 第一版不要做疯狂的多连接下载。

默认：

```text
1–2 个 playback-related range requests
```

或者根据实际测试设置有限并发。

目标不是：

```text
把 HTTP 下载速度拉到极限
```

目标是：

```text
保证当前播放所需 Range 尽快到达
```

M7 再研究智能调度和吞吐优化。

---

# 28. 防止 Range 请求爆炸

播放器可能不断发：

```text
Range A
Range B
Range C
Range D
Range E
```

必须：

```text
coalesce overlapping ranges
deduplicate in-flight demands
merge adjacent compatible ranges
cancel obsolete seeks
```

例如：

```text
Demand A = [0, 5MB)
Demand B = [4MB, 10MB)
```

可以合并为：

```text
[0, 10MB)
```

但必须确保：

```text
不会因为过度合并而下载明显不需要的巨大范围
```

---

# 29. Seek Demand

播放器发生：

```text
seek(00:30:00)
```

不能简单地认为：

```text
set currentTime
```

系统必须形成：

```text
Playback
→ byte position estimate
→ Demand
→ missing range
→ Scheduler
→ Resource
```

具体 byte ↔ time 转换可以在 M4 做最小可用版本。

禁止建设复杂媒体分析系统。

---

# 30. 视频 Seek 的现实限制

不要假设：

```text
time position
→ exact byte position
```

永远成立。

尤其对于：

```text
MP4
WebM
MKV
```

容器结构和索引可能影响 seek。

所以必须把：

```text
requested playback time
```

与：

```text
requested byte range
```

分开建模。

---

# 31. 最低要求的 Seek 行为

M4 至少实现：

```text
初始播放
→ 播放一段
→ 拖动进度条
→ 触发新的 Range Request
→ Scheduler 提高新的 Demand
→ 等待需要的数据
→ 继续播放
```

不要求做到专业播放器级精确 seek。

---

# 32. Torrent Resource

M3 已经接入 librqbit。

M4 不应该重新实现 BitTorrent 调度。

必须建立：

```text
TorrentResource
```

负责把：

```text
librqbit
```

暴露为：

```text
ReadableResource
Availability
```

---

# 33. Torrent Adapter

推荐：

```text
ReadableResource
    ↑
TorrentResource
    ↑
TorrentResourceAdapter
    ↑
TorrentEngine / librqbit
```

前端不能直接知道：

```text
rqbit torrent id
piece index
peer
DHT
tracker
```

---

# 34. Torrent Availability

M4 必须解决：

```text
piece availability
→ byte availability
```

例如：

```text
piece 100 available
piece 101 unavailable
piece 102 available
```

转换为对应：

```text
byte ranges
```

供：

```text
ReadableResource
```

消费。

---

# 35. Torrent Read

Torrent Resource 的：

```text
read(range)
```

应该由：

```text
librqbit
```

负责底层 piece 获取和验证。

Tide 负责：

```text
Demand
Priority
Playback intent
Cancellation
Gateway
Session lifecycle
```

不要复制 rqbit 的 Peer/Piece/Scheduler 核心逻辑。

---

# 36. 关于 rqbit 自带 Streaming API

如果当前锁定版本已经存在：

```text
/torrents/{id}/stream/{file_idx}
```

之类的 HTTP streaming 能力：

可以利用它做 M4 的底层能力。

但必须评估：

```text
是否需要 Tide 自己的 Gateway
是否能安全封装 token
是否会暴露内部 API
是否能实现统一 Resource 模型
是否能处理生命周期
是否能统一 HTTP/Torrent
```

即：

```text
rqbit streaming API
```

可以是：

```text
implementation detail
```

不能直接成为：

```text
Tide public architecture
```

---

# 37. 最终 Gateway

M4 必须建立：

```text
Progressive Stream Gateway
```

它是 WebView2 与 Tide Backend 的安全边界。

---

# 38. Gateway 只监听 Loopback

必须绑定：

```text
127.0.0.1
```

或：

```text
localhost
```

优先使用：

```text
127.0.0.1
```

禁止：

```text
0.0.0.0
```

禁止：

```text
LAN exposure
```

禁止：

```text
IPv6 wildcard
```

除非有极强的技术原因并明确记录。

---

# 39. Random Port

不要固定：

```text
3030
```

这种端口。

应：

```text
bind ephemeral port
```

启动后获得：

```text
127.0.0.1:<ephemeral-port>
```

---

# 40. Session Token

不要允许：

```text
/stream/task-123
```

成为公开访问接口。

应使用高熵 opaque capability。

例如概念：

```text
/stream/<random-capability>
```

token 必须：

```text
cryptographically random
high entropy
unpredictable
short-lived or session-bound
```

不要把：

```text
UUID
```

当作唯一安全措施。

---

# 41. Gateway URL

推荐概念：

```text
http://127.0.0.1:<port>/stream/<opaque-session-token>
```

可以额外加入：

```text
/resource/<opaque-id>
```

但不要暴露：

```text
real path
task UUID
torrent ID
infohash
database primary key
```

---

# 42. Gateway 的请求模型

至少支持：

```http
GET
HEAD
```

不需要：

```text
POST
PUT
PATCH
DELETE
```

---

# 43. Range 支持

至少处理：

```http
Range: bytes=start-end
Range: bytes=start-
Range: bytes=-suffix
```

内部统一转换为：

```text
ByteRange[start,end)
```

---

# 44. Gateway HTTP 状态

至少正确支持：

```text
200 OK
206 Partial Content
404 Not Found
416 Range Not Satisfiable
405 Method Not Allowed
503 Service Unavailable
```

根据实际需求增加：

```text
500
```

但禁止把所有异常都变成：

```text
500
```

---

# 45. Gateway Headers

需要正确处理：

```http
Content-Length
Content-Type
Accept-Ranges: bytes
Content-Range
Cache-Control
```

不要随意加入：

```text
Access-Control-Allow-Origin: *
```

WebView2 与本地 gateway 的访问模型应明确设计。

---

# 46. HEAD

`HEAD` 必须返回与实际 Resource 一致的基本元数据：

```text
Content-Length
Content-Type
Accept-Ranges
```

如果最终长度未知：

```text
Content-Length
```

不要伪造。

---

# 47. 资源不可用时 Gateway 的行为

这是 M4 最重要的部分之一。

例如播放器请求：

```text
[20MB, 22MB)
```

但是：

```text
20–22MB unavailable
```

Gateway 不能：

```text
返回空数据
```

也不能：

```text
返回随机已有区间
```

更不能：

```text
把整个文件阻塞死
```

应：

```text
创建 / 提升 Demand
→ Scheduler
→ Resource 获取
→ 等待可读
→ 输出数据
```

---

# 48. Gateway 的等待策略

设计：

```text
read request
↓
check availability
↓
if available:
    read
↓
if unavailable:
    submit Demand
    wait
↓
re-check availability
↓
read
```

必须支持：

```text
cancellation
client disconnect
timeout
shutdown
```

---

# 49. Client Disconnect

如果 WebView2：

```text
seek
```

导致旧 Range 请求断开：

Gateway 必须能够：

```text
detect disconnect
cancel obsolete demand
release waiting resources
```

不能留下大量：

```text
orphan requests
```

---

# 50. Seek Race

必须特别测试：

```text
seek A
↓
Demand A
↓
seek B
↓
Demand B
```

最终应：

```text
B priority > A
```

并在可能情况下：

```text
cancel / deprioritize A
```

不要让：

```text
A
```

继续占满所有网络资源。

---

# 51. PlaybackSession

M4 正式实现：

```text
PlaybackSession
```

至少包含：

```text
SessionId
ResourceId
GatewayToken
CreatedAt
LastAccessAt
CurrentRange
State
```

状态不要过度复杂。

可考虑：

```text
Created
Preparing
Playing
Paused
Seeking
Stopped
Failed
Ended
```

但如果已有状态模型，优先复用而不是重新设计。

---

# 52. PlaybackSession 与 DownloadTask 必须分离

必须保持：

```text
DownloadTask
```

与：

```text
PlaybackSession
```

两个不同生命周期。

例如：

```text
Task = Downloading
Playback = Stopped
```

或：

```text
Task = Paused
Playback = NotActive
```

也可能：

```text
Task = Downloading
Playback = Playing
```

禁止：

```text
关闭播放器
=
取消下载
```

除非用户明确选择。

---

# 53. 播放结束不等于下载结束

播放到 EOF：

```text
PlaybackSession = Ended
```

但：

```text
DownloadTask
```

仍然可以：

```text
Running
```

或者：

```text
Completed
```

由 Task 本身决定。

---

# 54. Gateway 生命周期

Gateway 启动：

```text
App startup
```

或者：

```text
first playback
```

取决于架构。

重点是：

```text
只有本机
只有必要时间
session 可管理
shutdown 可清理
```

---

# 55. Gateway Session Cleanup

必须防止：

```text
播放器关闭
窗口关闭
任务删除
App shutdown
```

之后仍然保留：

```text
active session
dangling token
pending demand
open file handle
```

必须有清理机制。

---

# 56. Gateway 安全测试

必须测试至少：

```text
随机 token
错误 token
过期 token
空 token
超长 token
路径穿越
encoded traversal
任意文件访问
绝对路径
UNC path
localhost path
文件系统设备路径
```

并确认：

```text
无法通过 HTTP 参数获取任意本地文件。
```

---

# 57. Host Header

必须评估：

```text
Host
```

校验。

至少确保 Gateway 不因：

```text
Host: attacker-controlled-host
```

等输入而改变安全行为。

---

# 58. CORS

除非真实需要，否则：

```text
不要开放 *
```

不要为了“播放器能播放”而直接：

```text
Access-Control-Allow-Origin: *
```

先测试 WebView2 是否真正需要 CORS。

---

# 59. URL 构造

前端只能获得：

```text
playback URL
```

不能获得：

```text
filesystem path
```

不能让 React 自己拼：

```text
file:///...
```

也不要把：

```text
absolute Windows path
```

注入 DOM。

---

# 60. Local File Security

Gateway 的逻辑必须始终通过：

```text
ResourceId
→ trusted backend mapping
→ allowed storage
```

而不是：

```text
HTTP request
→ path string
→ open(path)
```

---

# 61. Gateway Range 验证

必须防御：

```text
negative
overflow
u64 overflow
start > end
end >= file size
huge suffix
multiple ranges
malformed syntax
```

第一版：

> **可以只支持 single byte range。**

如果浏览器发出 multi-range：

```text
multipart/byteranges
```

可以明确：

```text
416 / 501-like controlled unsupported response
```

但不能错误解析。

---

# 62. 多 Range 请求

第一版不要为了“兼容性看起来完整”而实现复杂的：

```text
multipart/byteranges
```

除非 M0.5 的实际测试证明 WebView2 播放需要它。

优先：

```text
single range
```

简单、可验证。

---

# 63. HTTP Media Baseline

M4 必须使用 M0.5 已验证的实际播放器兼容矩阵。

不要在 M4 再次假设：

```text
所有 MP4 都能播
所有 MKV 都能播
所有 H265 都能播
```

播放器格式能力必须来自：

```text
M0.5 facts
```

---

# 64. WebView2 播放器

M4 实现最小真实 Player。

至少：

```text
load
play
pause
stop
seek
currentTime
duration
buffered
waiting
error
```

UI 可以非常简单。

重点是：

```text
真实数据
真实 playback state
真实 network/buffer state
```

---

# 65. 禁止 Fake UI

绝对禁止：

```text
假的下载速度
假的 buffer
假的 duration
假的 progress
假的 seek
假的 playing
```

如果浏览器无法提供可靠值：

```text
显示 unavailable
```

不要编造。

---

# 66. Player Events

至少处理：

```text
loadedmetadata
canplay
play
pause
waiting
playing
timeupdate
progress
seeking
seeked
ended
error
stalled
```

但不要把每个事件全部 IPC 到 Rust。

---

# 67. Frontend / Backend 边界

建议：

```text
Frontend
    ↓
Playback commands
    ↓
Rust
    ↓
PlaybackSession
    ↓
Gateway
```

前端负责：

```text
UI
player element
user gesture
display state
```

Rust 负责：

```text
security
resource
session
gateway
demand
scheduler
download/torrent
```

---

# 68. IPC

根据现有 M0/M1/M2/M3 命名适配。

至少需要：

```text
create_playback_session
get_playback_info
stop_playback_session
pause_playback_session
resume_playback_session
notify_seek
```

不一定所有功能都需要 IPC。

原则：

> 能由 WebView2 原生事件直接完成的，不要多造 IPC。

---

# 69. Playback URL 获取

推荐：

```text
create_playback_session(resource_id)
→
PlaybackSessionDTO {
    session_id,
    url,
    size,
    content_type,
}
```

前端：

```text
video.src = url
```

但：

```text
session URL
```

不能永久持久化到 SQLite。

---

# 70. Session Token 不持久化

禁止把：

```text
gateway token
```

存入数据库。

数据库可以保存：

```text
PlaybackSession metadata
```

但：

```text
capability token
```

必须运行时生成。

应用重启后重新生成。

---

# 71. Scheduler + Gateway 的联动

完整链路：

```text
WebView2
    ↓
GET Range
    ↓
Gateway
    ↓
Resource.available?
    ↓
NO
    ↓
Demand.submit
    ↓
Scheduler
    ↓
Provider
    ↓
bytes arrive
    ↓
Availability update
    ↓
Gateway wakes
    ↓
read
    ↓
HTTP 206
    ↓
WebView2
```

这条链必须实际跑通。

---

# 72. HTTP 与 Torrent 的统一性验证

必须做一组对比测试：

```text
HTTP Resource
Torrent Resource
```

然后验证二者都能暴露：

```text
size
content_type
availability
read(range)
```

并能通过：

```text
same Gateway
same PlaybackSession
same Player
```

播放。

---

# 73. 不允许复制两套播放器逻辑

禁止：

```text
HttpVideoPlayer
TorrentVideoPlayer
```

必须：

```text
VideoPlayer
    ↓
Playback URL
```

---

# 74. 不允许复制两套 Gateway

必须：

```text
one Tide Stream Gateway
```

资源差异在：

```text
ReadableResource
```

内部解决。

---

# 75. Scheduler 的最小算法

第一版可以采用：

```text
Critical
>
High
>
Normal
>
Low
```

同 priority 下：

```text
earliest deadline
>
oldest demand
```

同时支持：

```text
dedup
merge
cancel
```

不要现在实现：

```text
machine learning
EWMA throughput optimization
complex scoring
multi-factor predictive model
```

---

# 76. Playback Buffer Policy

M4 可以定义最小 Buffer Window。

例如：

```text
playback point
+
N MB / N seconds
```

但 N 不得拍脑袋定死。

必须通过测试观察：

```text
WebView2 behavior
startup latency
rebuffer frequency
network throughput
disk usage
```

然后决定。

如果暂时无法可靠估计“秒数”，可以先采用：

```text
byte window
```

---

# 77. Startup Buffer

初次播放必须满足：

```text
minimum readable window
```

例如：

```text
first 2–10MB
```

或者：

```text
codec/container 实际需要的 metadata + data
```

具体数值以实测为准。

不要单纯：

```text
下载 5 秒就开始播
```

因为速度可能变化。

---

# 78. Rebuffer

如果播放器进入：

```text
waiting
```

系统应判断：

```text
available data insufficient?
network bottleneck?
resource failure?
container limitation?
```

不要一看到 waiting 就立即 restart。

---

# 79. Seek 后 Buffer

seek 后优先：

```text
seek target
```

其次：

```text
forward buffer
```

最后：

```text
background download
```

这是 M4 Scheduler 最重要的实际行为。

---

# 80. Obsolete Demand

例如：

```text
User seek A
User seek B
User seek C
```

旧 Demand：

```text
A
B
```

如果明显不再需要：

```text
cancel
deprioritize
```

当前：

```text
C
```

必须获得最高 playback priority。

---

# 81. Background Download

如果用户正在播放：

```text
playback demand
```

优先于普通：

```text
background download
```

但不能让后台任务永久饿死。

第一版可以采用：

```text
Playback has strict priority while blocked.
Background resumes when playback buffer is healthy.
```

不要实现非常复杂的 fair-share 算法。

---

# 82. 下载任务与播放任务的交互

至少测试：

### 情况 A

```text
download + playback
```

### 情况 B

```text
playback → pause
```

### 情况 C

```text
playback → seek
```

### 情况 D

```text
playback stop
```

### 情况 E

```text
download pause while playback
```

### 情况 F

```text
app shutdown while playback
```

都必须有明确语义。

---

# 83. HTTP 下载数据写入模型

如果 M1 仍然只支持：

```text
single sequential file append
```

M4 需要最小升级为能够支持：

```text
range writes
```

推荐考虑：

```text
sparse file / random-access writes
```

或者：

```text
range storage abstraction
```

但禁止为了理论上的所有场景实现一个巨大的 Storage Framework。

---

# 84. Range Writer

建议建立：

```text
RangeWriter
```

概念能力：

```text
write(range, bytes)
flush(range)
read(range)
```

并保证：

```text
write range != overwrite unrelated bytes
```

---

# 85. Sparse File

Windows 上如果采用 sparse file：

必须验证：

```text
Windows behavior
file allocation
actual size
logical size
disk usage
read holes
crash recovery
```

不能只在 Linux 环境测试。

---

# 86. 如果不采用 Sparse File

可以采用：

```text
sidecar range map
+
random access writes
```

或者已有 Storage 机制。

但必须明确：

```text
data layout
availability persistence
crash recovery
```

---

# 87. 与 M2 的一致性

M2 已经建立：

```text
SQLite
+
.part
+
Recovery
```

M4 不得破坏这个模型。

必须决定：

```text
Availability metadata
```

到底：

```text
stored in SQLite
stored in sidecar file
recomputed
```

三者择一或组合。

优先：

> **任何可恢复性关键数据都必须有明确持久化来源。**

---

# 88. Torrent Availability Persistence

不要重复保存：

```text
rqbit 自己已经持久化的 piece state
```

如果 rqbit 能够在启动时恢复：

```text
piece availability
```

Tide 应通过：

```text
adapter
```

重新读取。

Tide 保存：

```text
logical resource / selection / playback metadata
```

不要维护第二套与 rqbit 相冲突的 bitfield。

---

# 89. HTTP Availability Persistence

HTTP Range Resource 如果允许随机范围下载：

必须能够：

```text
app crash
→ restart
→ rebuild availability
→ know which ranges remain
→ safely continue
```

不能仅存：

```text
downloaded_bytes
```

---

# 90. Checksum

M4 不要求播放中的每次读取都做：

```text
full-file SHA-256
```

不要这样做。

底层数据完整性继续由：

```text
M1 HTTP semantics
M3 Torrent piece verification
```

负责。

---

# 91. Torrent Piece Verification

Tide 不要重新验证：

```text
SHA1 pieces
```

除非 rqbit API 要求。

应该：

```text
rqbit = source of truth for Torrent piece integrity
```

---

# 92. Streaming Data Integrity

Gateway 必须是：

```text
byte-preserving
```

即：

```text
Resource bytes
=
Gateway response bytes
=
Player input bytes
```

除非明确存在：

```text
container/player-level interpretation
```

但 Tide 本身不转码。

---

# 93. Gateway Buffering

禁止一次性：

```text
read entire file
```

必须采用：

```text
streaming chunks
```

例如：

```text
64KB
128KB
256KB
```

具体值通过 benchmark 决定。

---

# 94. Backpressure

必须处理：

```text
Player consumes slowly
```

不能持续：

```text
download forever into RAM
```

必须建立基本：

```text
read backpressure
```

即：

```text
consumer demand
limits producer output
```

---

# 95. Memory Limit

必须设置合理上限。

禁止：

```text
user opens huge media
→ entire missing range buffered in RAM
```

所有 buffering 必须是：

```text
bounded
```

---

# 96. Timeout

Gateway 至少应该区分：

```text
connect timeout
read timeout
resource wait timeout
session idle timeout
shutdown timeout
```

不要：

```text
one global timeout
```

覆盖所有情况。

---

# 97. Resource Wait Timeout

例如：

```text
Torrent piece 永远没有 peer
```

或者：

```text
HTTP upstream 永久不响应
```

Gateway 不能无限挂住。

需要：

```text
bounded wait
```

并返回结构化错误。

---

