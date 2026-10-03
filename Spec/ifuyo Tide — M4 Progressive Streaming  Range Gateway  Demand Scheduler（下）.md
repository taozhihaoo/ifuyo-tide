# ifuyo Tide — M4 Progressive Streaming / Range Gateway / Demand Scheduler（下）

> **拆分说明（必读）**：本文件是 M4 完整开发实施提示词的**下半部分（第 98~215 节：错误模型、安全、协议正确性、并发与取消、测试矩阵、Critical Acceptance Test、性能与最终验收）**，承接上半部分《ifuyo Tide — M4 Progressive Streaming  Range Gateway  Demand Scheduler（上）.md》（第 0~97 节：架构与核心实现）。上半部分的阶段定位、执行纪律、架构原则与安全边界对本部分**全部生效**；执行本部分前必须先阅读并完成上半部分。
# 98. 错误类型

M4 至少扩展：

```text
PlaybackError
GatewayError
ResourceError
DemandError
SchedulerError
RangeError
TimeoutError
```

底层错误不能直接：

```text
unwrap()
```

暴露给前端。

---

# 99. Playback Error UI

UI 至少能显示：

```text
Unable to play resource
Resource unavailable
Network timeout
Unsupported media
Playback session expired
```

不要直接显示：

```text
Rust panic
stack trace
Windows filesystem exception
rqbit internal debug string
```

---

# 100. 安全：任意文件读取 P0

实现完成后必须专门验证：

```text
../../secret
..\..\secret
%2e%2e
%252e%252e
C:\
C:/
\\server\share
\\?\C:\
file://
```

不能通过 Gateway 访问任意文件。

---

# 101. 安全：Token P0

测试：

```text
guessable token
empty token
reused token
token from another session
expired token
token after session close
```

结果必须：

```text
access denied
```

---

# 102. 安全：Task 删除

如果：

```text
Task deleted
```

但：

```text
PlaybackSession
```

还活着：

必须有明确行为。

推荐：

```text
existing playback may finish current session
```

或者：

```text
session invalidated
```

二选一并写进架构文档。

禁止出现：

```text
dangling path
```

---

# 103. 安全：资源隔离

必须明确：

```text
DownloadRoot
AppDataRoot
TempRoot
```

分别是什么。

Gateway 只允许访问：

```text
authorized Resource
```

不允许：

```text
arbitrary filesystem
```

---

# 104. Tauri Capability

继续遵守 M0：

```text
minimal capability
strict CSP
minimal filesystem permissions
minimal shell permissions
```

不要为了：

```text
player
gateway
```

而把：

```text
filesystem:all
shell:all
process:all
```

开放给 frontend。

---

# 105. Gateway 不等于 Tauri IPC

不要让前端：

```text
invoke(read_bytes, ...)
```

不断向 Rust 拉媒体数据。

正确方向是：

```text
WebView2 <video>
    ↓
HTTP Range
    ↓
Local Gateway
```

这样能减少：

```text
IPC overhead
JS memory copy
Rust→JS serialization
```

---

# 106. 为什么必须使用 Local HTTP Gateway

M4 必须验证：

```text
WebView2 media element
```

是否能够稳定通过：

```text
127.0.0.1
```

读取：

```text
Range
```

并且：

```text
seek
```

能够正常工作。

---

# 107. HTTP Status 正确性

严格测试：

### 无 Range

```text
200
```

### 有效 Range

```text
206
```

### 超范围

```text
416
```

### 无效 Range

```text
416 / controlled error
```

### Resource unavailable

```text
503 / controlled wait-failure
```

具体采用哪个状态必须记录设计决策。

---

# 108. Content-Length

对于：

```text
known-size Resource
```

必须提供：

```text
Content-Length
```

对于：

```text
unknown-size
```

不得伪造。

---

# 109. Content-Range

206：

```text
Content-Range: bytes start-last/total
```

必须与实际 body 完全一致。

例如：

```text
range = [100, 200)
```

那么：

```text
Content-Range = bytes 100-199/...
Content-Length = 100
```

三者必须一致：

```text
200 - 100 = 100
```

---

# 110. Gateway 的 Range Parser

不要自己写脆弱的：

```text
split('-')
```

解析器。

必须有：

```text
unit
start
end
suffix
```

明确模型。

并覆盖：

```text
overflow
whitespace
invalid token
duplicate hyphen
missing end
invalid suffix
```

---

# 111. 读写并发

必须考虑：

```text
Downloader writes range
Gateway reads same resource
```

确保：

```text
reader
不会看到半写入的无效数据
```

至少要保证：

```text
Availability only becomes AVAILABLE
after corresponding bytes are durably/safely visible
```

具体 durability 级别必须与 M2 的设计一致。

---

# 112. Availability 更新顺序

推荐：

```text
fetch bytes
↓
write bytes
↓
flush / ensure visibility
↓
update availability
↓
wake waiting readers
```

不能：

```text
availability = available
↓
实际上 bytes 还没有写完
```

---

# 113. Demand 更新顺序

推荐：

```text
submit demand
↓
Scheduler assigns
↓
Resource fetch starts
↓
bytes arrive
↓
Availability updated
↓
reader wakes
```

状态必须有严格生命周期。

---

# 114. In-flight Demand Dedup

如果同时收到：

```text
[0, 4MB)
[0, 4MB)
[2MB, 6MB)
```

不要启动：

```text
3 independent full fetches
```

应尽可能合并/共享：

```text
shared in-flight demand
```

---

# 115. Reader Wakeup

实现：

```text
Condition variable
Notify
broadcast
async channel
watch
```

均可。

但是必须避免：

```text
busy loop
```

禁止：

```rust
loop {
    if available() { break; }
    sleep(10ms);
}
```

除非只是测试代码。

---

# 116. Cancellation

每个：

```text
PlaybackSession
Demand
RangeFetch
```

都必须能取消。

例如：

```text
user seek
```

旧 Demand 可以取消。

例如：

```text
user closes player
```

所有 session-specific Demand 必须释放。

---

# 117. Shutdown

应用退出：

```text
Gateway
↓
PlaybackSession
↓
Demand
↓
Resource waits
↓
Network fetches
```

必须按顺序停止。

不能：

```text
Gateway gone
but tasks continue waiting forever
```

---

# 118. Windows 特别测试

必须在真实 Windows 环境验证：

```text
127.0.0.1 binding
ephemeral port
file locking
random access write
sparse file if used
rename semantics
shutdown
WebView2
```

尤其是：

```text
打开正在播放的文件时
是否可以继续下载/写入
```

---

# 119. 性能测试

M4 不只是功能测试。

至少测：

```text
CPU
RAM
Disk IO
Network
Startup latency
Playback startup latency
Seek latency
Gateway throughput
Range request rate
Scheduler overhead
```

---

# 120. 至少建立以下测试媒体

使用：

```text
public domain
open source
self-generated
legally owned
```

不要在测试文档里放：

```text
盗版资源链接
非法资源站链接
```

---

# 121. 测试素材

至少有：

```text
small MP4
medium MP4
large MP4
WebM if supported
unsupported format
multi-file torrent
single-file torrent
slow HTTP resource
range-enabled HTTP resource
range-disabled HTTP resource
```

---

# 122. HTTP Range 测试矩阵

至少：

```text
GET full
GET range
GET suffix range
GET open-ended range
416
Range ignored
ETag changed
Last-Modified changed
Connection reset
slow response
timeout
```

---

# 123. Torrent Streaming 测试矩阵

至少：

```text
metadata acquired
file selected
partial download
start playback
pause
seek
resume
peer slowdown
no immediate piece available
piece arrives
piece verification
torrent restart
application restart
```

---

# 124. Player 测试矩阵

至少：

```text
load
play
pause
seek forward
seek backward
rapid seek
waiting
resume from waiting
ended
session stop
app shutdown
```

---

# 125. Gateway Security Test Matrix

至少：

```text
valid token
invalid token
expired token
wrong resource token
path traversal
encoded traversal
arbitrary path
invalid range
overflow range
huge suffix
multi-range
wrong HTTP method
wrong Host
session deleted
task deleted
app shutdown
```

---

# 126. Critical Acceptance Test #1

### HTTP Progressive Playback

必须实现：

```text
create HTTP task
↓
start download
↓
file only partially downloaded
↓
create playback session
↓
WebView2 opens local gateway URL
↓
video starts playing
↓
download continues
↓
player reaches frontier
↓
gateway waits
↓
download extends
↓
playback continues
```

这是 M4 P0。

---

# 127. Critical Acceptance Test #2

### HTTP Seek

必须实现：

```text
download partial
↓
play
↓
seek into not-yet-downloaded region
↓
Demand created
↓
Scheduler prioritizes Demand
↓
HTTP Range fetched
↓
Availability updated
↓
Gateway returns bytes
↓
player resumes
```

这是 M4 P0。

---

# 128. Critical Acceptance Test #3

### Torrent Progressive Playback

必须实现：

```text
add legal Torrent/Magnet test resource
↓
metadata ready
↓
file partially available
↓
create playback session
↓
play
↓
missing piece requested/prioritized
↓
piece arrives
↓
playback continues
```

---

# 129. Critical Acceptance Test #4

### Torrent Seek

必须实现：

```text
play
↓
seek to distant position
↓
Demand changes
↓
rqbit receives appropriate read pressure
↓
required pieces become prioritized
↓
Gateway eventually serves range
↓
playback resumes
```

---

# 130. Critical Acceptance Test #5

### Obsolete Seek

必须实现：

```text
seek A
↓
seek B
↓
seek C
```

确认：

```text
C = highest priority
A/B = cancelled or deprioritized
```

不能出现：

```text
A/B/C simultaneously consume all bandwidth
```

---

# 131. Critical Acceptance Test #6

### Security

确认：

```text
GET /stream/<valid-token>
```

只能读：

```text
mapped Resource
```

无法读：

```text
C:\
Users\
Windows\
AppData\
arbitrary .part
```

---

# 132. Critical Acceptance Test #7

### Restart

```text
download partial
↓
playback active
↓
close app
↓
relaunch
↓
task recovered
↓
resource availability reconstructed
↓
new playback session created
↓
playback works
```

---

# 133. Critical Acceptance Test #8

### Resource Abstraction

同一个播放器：

```text
Player
```

分别播放：

```text
HttpResource
TorrentResource
```

前端不应出现：

```text
if source.type == torrent
if source.type == http
```

之类的播放分支。

---

# 134. Critical Acceptance Test #9

### Byte Preservation

对一份已知测试文件：

```text
original
```

通过：

```text
Resource.read
→ Gateway
```

读取多个随机 Range：

```text
[0,1MB)
[1MB,2MB)
[17MB,19MB)
[EOF-1MB,EOF)
```

与：

```text
original[offset..end]
```

逐字节比较。

必须完全一致。

---

# 135. Property Tests

至少做：

```text
Range parser
Content-Range parser
range normalization
range merge
range overlap
availability merge
missing range calculation
demand merge
demand dedup
priority ordering
```

尽可能加入：

```text
property-based testing
```

---

# 136. Fuzz / Abuse Testing

至少针对：

```text
HTTP Range header
session token
gateway path
Content-Range
query strings
```

进行 malformed input 测试。

重点是：

```text
never panic
never access arbitrary path
never overflow
never deadlock
```

---

# 137. Deadlock / Cancellation Test

重点验证：

```text
read waiting for unavailable range
↓
seek
↓
cancel old demand
↓
new demand starts
```

确保旧 reader：

```text
及时退出
```

不能：

```text
一直等待旧资源
```

---

# 138. No Busy Loop

审计所有等待逻辑：

```text
Gateway
Scheduler
Resource
Availability
Torrent adapter
```

禁止高频轮询。

如果发现：

```text
10ms / 50ms
```

这类固定轮询，优先改成：

```text
event-driven
notify
async channel
watch
condition
```

---

# 139. Scheduler Telemetry

不做外部遥测。

内部可以有：

```text
debug metrics
```

例如：

```text
pending_demands
active_demands
playback_demands
cancelled_demands
average_wait
```

但：

```text
production logs
```

不得包含敏感信息。

---

# 140. 日志安全

禁止默认记录：

```text
完整 Magnet
完整 URL
完整本地路径
Peer IP
InfoHash
Gateway token
```

必要时：

```text
hash/redact
```

---

# 141. 结构化日志

建议：

```text
playback_session_created
demand_submitted
demand_promoted
range_wait_started
range_available
gateway_request
gateway_range_response
playback_stopped
```

但要：

```text
low cardinality
safe fields
```

---

# 142. 错误恢复

如果：

```text
HTTP Range fetch failed
```

不能把：

```text
PlaybackSession
```

永久标记失败，除非资源真的不可恢复。

允许：

```text
retry
wait
fallback
```

---

# 143. 网络错误

至少区分：

```text
timeout
connection reset
connection refused
DNS failure
HTTP 4xx
HTTP 5xx
```

不要全部：

```text
NetworkError
```

---

# 144. Torrent 无数据可得

如果：

```text
Torrent range unavailable
```

但：

```text
torrent still alive
```

应该：

```text
wait/retry
```

而不是立即：

```text
PlaybackError
```

---

# 145. HTTP Range 不支持

如果：

```text
origin ignores Range
```

M4 必须定义明确 fallback。

可接受：

```text
restart sequential download
```

或者：

```text
disable random seek for this resource
```

但必须：

```text
detect
record
explain
```

不能偷偷错误处理。

---

# 146. Resource Capability

考虑为 Resource 暴露：

```text
supports_range
supports_random_access
supports_progressive
known_size
```

例如：

```text
HTTP:
progressive = yes
random_access = depends

Torrent:
progressive = yes
random_access = yes
```

如果 M0.5 结果显示 WebView2 对某类资源有限制，明确记录。

---

# 147. Player Capability 与 Resource Capability 分离

不要设计：

```text
if resource.can_seek && player.supports_seek...
```

这种散乱判断。

建议统一：

```text
PlaybackCapability
```

在 session 创建阶段形成。

---

# 148. M4 UI

UI 不需要漂亮。

至少实现：

```text
Resource Name
Player
Play/Pause
Seek bar
Current Time / Duration
Buffer state
Playback state
Error
Download state
```

额外显示：

```text
downloading
available
waiting
```

可选。

---

# 149. Buffer UI

不要画：

```text
假的蓝色 buffer
```

必须使用实际：

```text
WebView2 buffered ranges
```

或者 backend 实际 availability。

明确二者区别：

```text
Browser Buffered
vs
Backend Available
```

不要混成一个指标。

---

# 150. Playback Diagnostics

开发模式下可提供：

```text
Session ID
Resource ID
Current Range
Available Range
Pending Demand
Scheduler Priority
Gateway Request Range
```

但：

```text
release build
```

不要默认显示。

---

# 151. Architecture Documentation

至少新增：

```text
docs/M4-resource.md
docs/M4-availability.md
docs/M4-demand.md
docs/M4-scheduler.md
docs/M4-stream-gateway.md
docs/M4-playback.md
docs/M4-http-progressive.md
docs/M4-torrent-progressive.md
docs/M4-security.md
docs/M4-test-matrix.md
docs/M4-report.md
```

如果已有文档体系，则合并到现有结构。

---

# 152. Architecture Decision Records

M4 必须明确记录至少这些决策：

```text
ADR-1 Resource.read(range) semantics
ADR-2 Availability representation
ADR-3 HTTP random range storage strategy
ADR-4 Torrent adapter strategy
ADR-5 Gateway binding/security
ADR-6 session token strategy
ADR-7 scheduler priority model
ADR-8 browser/player integration model
ADR-9 single-range vs multi-range support
ADR-10 cancellation semantics
```

---

# 153. 不要创建 premature abstraction

禁止为了“未来支持一切”创建：

```text
UniversalMediaEngine
UniversalProtocolEngine
UniversalResourceFramework
UniversalStreamingRuntime
```

如果抽象只服务当前：

```text
HTTP
Torrent
Playback
```

即可。

---

# 154. 目录结构建议

必须以现有项目为准。

可以演进为：

```text
src-tauri/src/
  core/
    resource/
      mod.rs
      model.rs
      readable.rs
      availability.rs
      http.rs
      torrent.rs

    playback/
      mod.rs
      session.rs
      demand.rs
      scheduler.rs
      capabilities.rs

    gateway/
      mod.rs
      server.rs
      route.rs
      range.rs
      session.rs
      security.rs
```

不是强制照搬。

---

# 155. Application Layer

推荐：

```text
Application
    ↓
PlaybackService
    ↓
Scheduler
    ↓
ResourceProvider
```

Frontend：

```text
IPC → PlaybackService
```

Gateway：

```text
HTTP → GatewayService
```

两者最终都进入：

```text
core
```

---

# 156. 不允许 Gateway 直接访问 SQLite

禁止：

```text
HTTP Request
→ SQL
```

应该：

```text
Gateway
→ PlaybackSession
→ Resource
```

Repository 只负责：

```text
persistent metadata
```

---

# 157. 不允许 Player 直接访问 Repository

禁止：

```text
React
→ SQL concept
```

前端只通过：

```text
IPC DTO
```

---

# 158. DTO

Backend internal model 与 frontend DTO 分离。

例如：

```text
PlaybackSession
```

不要直接 Serialize 所有内部字段。

创建：

```text
PlaybackSessionDto
```

只暴露真正需要的信息。

---

# 159. Gateway Capability DTO

前端得到：

```text
PlaybackUrl
```

即可。

不要返回：

```text
storage_path
resource_internal_id
torrent_internal_id
```

除非诊断模式明确需要。

---

# 160. M4 Persistence

M4 不需要重新发明任务持久化。

沿用 M2。

新增数据只增加：

```text
minimal persistent metadata
```

例如：

```text
resource range strategy
availability version
```

但要避免：

```text
每个 Range Request
=
一次 SQLite transaction
```

---

# 161. Availability Storage 性能

如果采用 SQLite：

必须 benchmark：

```text
10 ranges
100 ranges
1000 ranges
10000 ranges
```

确认不会因为 Range 碎片导致：

```text
查询/合并过慢
```

必要时使用：

```text
interval compaction
```

---

# 162. Range Fragmentation

假设：

```text
[0,1MB)
[2MB,3MB)
[4MB,5MB)
...
```

可能出现大量碎片。

必须有：

```text
merge adjacent
merge overlap
compact
```

策略。

---

# 163. Availability 与 piece alignment

Torrent：

```text
piece aligned
```

HTTP：

```text
arbitrary byte range
```

内部统一：

```text
byte range
```

底层可以：

```text
round to piece
round to request block
```

但不能把底层限制泄露给 Player。

---

# 164. Piece Alignment

Torrent adapter 允许：

```text
Demand byte range
→ required pieces
```

例如：

```text
Demand [10, 100]
```

实际可能需要：

```text
piece 0
```

和：

```text
piece 1
```

但返回给 Gateway 的数据仍必须严格：

```text
[10,100)
```

---

# 165. Scheduler 与 piece alignment

Scheduler 不必理解：

```text
PieceIndex
```

除非底层 adapter 必需。

推荐：

```text
Scheduler = byte-range intent
TorrentAdapter = piece mapping
```

---

# 166. Playback Byte Mapping

M4 第一版允许：

```text
approximate seek
```

但必须建立：

```text
time → target byte
```

的最小模型。

不要创建完整：

```text
demuxer
codec parser
```

---

# 167. 容器索引限制

必须在 M4 文档中记录：

```text
WebView2 seek 的实际行为
```

以及：

```text
MP4/WebM/MKV 在当前环境下的实测差异
```

如果某种格式需要完整尾部 metadata 才能 seek：

记录为：

```text
known limitation
```

而不是偷偷增加：

```text
FFmpeg
```

---

# 168. Native Player 决策

除非 M0.5 已明确证明：

```text
WebView2 cannot satisfy required media workflow
```

M4 不默认加入：

```text
libmpv
FFmpeg
VLC
GStreamer
```

如果需要 fallback：

必须先输出：

```text
FACTS
failure cases
compatibility matrix
engineering cost
binary size
license implications
security implications
```

然后再实施。

---

# 169. M4 的性能目标

M4 不要求“迅雷级”。

目标是：

```text
可靠
低额外开销
可恢复
可取消
可 seek
无明显内存爆炸
```

建议至少验证：

```text
Gateway throughput ≥ local storage read throughput 的合理比例
```

具体阈值根据硬件与实现测量。

---

# 170. Startup Latency

重点测：

```text
create playback session
→ player load
→ first frame
```

记录：

```text
p50
p95
```

或者至少：

```text
cold start
warm start
```

---

# 171. Seek Latency

测试：

```text
small seek
near seek
far seek
backward seek
rapid repeated seek
```

记录：

```text
seek issued
→ demand submitted
→ first required range available
→ browser resumes
```

---

# 172. Buffer Stall

至少测：

```text
fast network
medium network
slow network
intermittent network
```

记录：

```text
startup buffering
rebuffer frequency
rebuffer duration
```

---

# 173. CPU

重点观察：

```text
Gateway
Scheduler
Availability
Rust serialization
Tauri IPC
```

不要因为：

```text
Range events
```

导致 CPU 持续高占用。

---

# 174. IPC Flood

禁止：

```text
每 1KB
→ IPC progress event
```

必须：

```text
throttle/coalesce
```

---

# 175. UI Refresh

前端推荐：

```text
100–500ms
```

级别更新。

不要：

```text
每个 chunk setState
```

如果实际测试证明其他频率更合适，以测试结果为准。

---

# 176. Event Model

建议：

```text
playback.created
playback.state_changed
playback.progress
playback.buffer_changed
playback.error
playback.ended
```

Backend：

```text
demand.created
demand.updated
demand.completed
```

不要把每个内部 Range/peer/piece 事件全部暴露给前端。

---

# 177. Testing Architecture

建议：

```text
unit
integration
property
end-to-end
manual
```

五层测试。

---

# 178. Unit Test

至少：

```text
ByteRange
RangeParser
ContentRangeParser
AvailabilitySet
Demand
PriorityQueue
Dedup
Merge
PlaybackState
SessionToken validation
Gateway routing
```

---

# 179. Integration Test

至少：

```text
HttpResource + Storage
TorrentResource + rqbit
Resource + Availability
Demand + Scheduler
Gateway + Resource
Gateway + Scheduler
PlaybackSession + Gateway
```

---

# 180. End-to-End Test

真实：

```text
WebView2
+
Tauri
+
Rust backend
+
Gateway
+
HTTP test server
```

以及：

```text
WebView2
+
Tauri
+
Rust backend
+
Gateway
+
rqbit
+
legal Torrent
```

---

# 181. HTTP Test Server

复用/升级 M1 测试服务器。

至少支持：

```text
range on/off
slow
disconnect
ETag
Last-Modified
200 to Range
206
416
large file
random delay
```

---

# 182. Torrent Test Resource

不要依赖公网不稳定 Torrent。

优先建立：

```text
local private test swarm
```

或者：

```text
self-generated legal test torrent
```

例如：

```text
known binary/media generated locally
```

---

# 183. 不要使用盗版影视测试

测试媒体必须来自：

```text
self-generated
public-domain
open-source
properly licensed
```

不要在代码或文档中加入：

```text
pirated movies
copyright torrent magnets
```

---

# 184. App Data

沿用 M2：

```text
%LOCALAPPDATA%\ifuyo\Tide\
```

例如：

```text
db
logs
runtime metadata
```

而：

```text
DownloadRoot
```

保持用户资源目录。

---

# 185. Gateway 日志安全

Gateway log 不得默认记录：

```text
full URL
token
absolute file path
full query string
```

可以记录：

```text
session short id
status
requested range
content length
duration
```

但要确认不会泄露敏感信息。

---

# 186. DoD — Architecture

M4 完成必须满足：

```text
Resource abstraction stable
Availability real
Demand real
Scheduler real
Gateway real
PlaybackSession real
HTTP path real
Torrent path real
```

---

# 187. DoD — HTTP

必须：

```text
partial HTTP download
→ playback
→ Range
→ wait
→ continue
```

成功。

---

# 188. DoD — Torrent

必须：

```text
partial Torrent
→ playback
→ piece wait
→ continue
```

成功。

---

# 189. DoD — Seek

至少：

```text
forward seek
backward seek
far seek
rapid seek
```

其中受 M0.5 媒体格式能力限制的情况，必须明确记录。

---

# 190. DoD — Gateway

必须：

```text
loopback
ephemeral port
opaque token
GET
HEAD
Range
206
416
session isolation
path isolation
shutdown cleanup
```

---

# 191. DoD — Scheduler

必须：

```text
priority
dedup
merge
cancel
reprioritize
playback priority
```

---

# 192. DoD — Recovery

必须：

```text
app restart
→ HTTP resource recovery
→ Torrent resource recovery
→ playback can be recreated
```

---

# 193. DoD — Security

必须证明：

```text
no arbitrary filesystem read
no LAN exposure
no predictable playback token
no stale session access
no raw internal path exposure
no accidental broad Tauri capability
```

---

# 194. DoD — Stability

必须：

```text
cargo check --locked
cargo test --locked
```

并运行：

```text
clippy
rustfmt
frontend tests
frontend typecheck
build
```

所有新增代码不得引入：

```text
panic
unwrap
expect
```

于正常业务路径中。

---

# 195. DoD — Tests

必须：

```text
unit tests pass
integration tests pass
property tests pass
security tests pass
E2E tests pass
```

所有失败测试必须说明：

```text
failure
cause
decision
next step
```

---

# 196. DoD — Performance

至少得到真实测量：

```text
startup latency
first playback latency
seek latency
gateway throughput
CPU
RAM
disk IO
```

不能写：

```text
should be fast
```

而没有数据。

---

# 197. DoD — Documentation

至少：

```text
M4 resource docs
M4 availability docs
M4 scheduler docs
M4 gateway docs
M4 playback docs
M4 security docs
M4 test matrix
M4 report
```

---

# 198. 最终架构验收图

完成后必须能够明确画出：

```text
                 ┌───────────────────┐
                 │    WebView2       │
                 │   <video>         │
                 └─────────┬─────────┘
                           │
                    HTTP GET / Range
                           │
                 ┌─────────▼─────────┐
                 │ Progressive       │
                 │ Stream Gateway     │
                 └─────────┬─────────┘
                           │
                    PlaybackSession
                           │
                 ┌─────────▼─────────┐
                 │      Demand       │
                 └─────────┬─────────┘
                           │
                 ┌─────────▼─────────┐
                 │     Scheduler     │
                 └─────────┬─────────┘
                           │
                  Resource / Provider
                     ┌─────┴─────┐
                     │           │
               HttpResource  TorrentResource
                     │           │
                HTTP Range     librqbit
                     │           │
                     └─────┬─────┘
                           │
                      Availability
                           │
                         Storage
```

如果最终代码结构无法对应这一逻辑关系，需要说明原因。

---

# 199. 最终审计要求

完成后进行一次完整代码审计。

输出：

```text
FACTS
INFERENCES
CONFIRMED
RESOLVED
OPEN
WORSENED
```

风险等级只使用：

```text
P0
P1
```

---

# 200. P0 定义

M4 出现以下任一项，不能声称完成：

```text
Gateway 可访问任意本地文件
Gateway 暴露到 LAN
Range 返回错误字节
200 被当成 206 处理
Availability 与实际 bytes 不一致
seek 会导致数据损坏
旧 seek Demand 无法取消导致系统失控
PlaybackSession 与 DownloadTask 生命周期混乱
App restart 后资源不可恢复
播放器实际不能播放，但 UI 显示 Playing
正常路径 panic
数据竞争 / deadlock
严重内存无限增长
```

---

# 201. P1 定义

例如：

```text
某些浏览器 seek 行为兼容性不足
某些媒体格式有限制
multi-range 暂未支持
seek 精度不高
调度策略较简单
UI 仍较粗糙
性能还有优化空间
```

这些可以存在，但必须记录。

---

# 202. 禁止偷偷扩大范围

如果实现过程中发现：

```text
WebView2 某格式不兼容
HTTP Range 某服务器特殊
rqbit API 不满足某场景
Sparse file 有 Windows 限制
```

禁止直接开始：

```text
安装 FFmpeg
换播放器
重写 Torrent Engine
重写 Download Engine
加入云服务
```

必须：

```text
记录 FACT
分析影响
确认是否阻塞 M4
选择最小解决方案
```

---

# 203. 与 M5 的边界

M4 完成后：

```text
Player
```

已经能够：

```text
真实播放
pause
seek
waiting
resume
```

但 M5 才进行：

```text
正式播放器 UI
媒体信息
播放控制完善
错误 UX
播放队列基础
音量
全屏
快捷键
```

不能在 M4 把播放器做成最终产品。

---

# 204. 与 M6 的边界

M4 不做：

```text
Library
poster
thumbnail
history
favorites
collections
```

但 PlaybackSession 可以支持：

```text
ResourceId
```

以便 M6 接入。

---

# 205. 与 M7 的边界

M4 Scheduler：

```text
simple priority scheduler
```

M7 再做：

```text
smart scheduler
adaptive buffering
throughput estimation
fairness
background optimization
disk/network awareness
```

不要把 M7 功能提前塞入 M4。

---

# 206. 与 M8 的边界

M4 不做：

```text
Windows shell integration
file associations
system tray
startup
jump list
```

这些留给 M8。

---

# 207. 与 M9/M10 的边界

不在 M4 大规模做：

```text
polish
release signing
installer polish
security release hardening
crash reporting
distribution
```

但 M4 必须建立足够好的安全基础。

---

# 208. M4 实现顺序

推荐按以下顺序执行：

```text
Step 1
审计 M0–M3

Step 2
冻结 Resource / Availability / ByteRange semantics

Step 3
实现 AvailabilitySet

Step 4
实现 Demand

Step 5
实现基础 Scheduler

Step 6
把 M1 HTTP 升级为 HttpResource

Step 7
把 M3 Torrent 升级为 TorrentResource

Step 8
实现 PlaybackSession

Step 9
实现 local Stream Gateway

Step 10
实现 GET/HEAD/Range

Step 11
实现 unavailable → Demand → wait

Step 12
实现 WebView2 player spike integration

Step 13
打通 HTTP progressive playback

Step 14
打通 HTTP seek

Step 15
打通 Torrent progressive playback

Step 16
打通 Torrent seek

Step 17
实现 cancellation / obsolete seek

Step 18
完成 security tests

Step 19
完成 crash/restart tests

Step 20
完成 performance benchmark

Step 21
完成 docs

Step 22
完整审计
```

不要跳步骤。

---

# 209. 每完成一个阶段都验证

不要一次修改几千行以后才运行测试。

至少采用：

```text
small change
→ build
→ test
→ inspect
→ continue
```

尤其涉及：

```text
Gateway
Range
Availability
Scheduler
Storage
Concurrency
```

必须频繁验证。

---

# 210. 修改原则

优先：

```text
reuse existing code
reuse existing interfaces
extend existing repository
extend existing error model
extend existing test infrastructure
```

避免：

```text
parallel implementation
duplicate code
temporary compatibility layer that becomes permanent
```

---

# 211. 最终必须回答的问题

完成 M4 后报告必须明确回答：

```text
1. HTTP 是否真正支持边下边播？
2. HTTP seek 到未下载区域是否真正可用？
3. Torrent 是否真正支持未完成时播放？
4. Torrent seek 是否真正会产生新的 Demand？
5. Gateway 是否完全 loopback？
6. Gateway 是否无法访问任意文件？
7. Playback token 是否不可预测？
8. Resource 是否真正统一 HTTP/Torrent？
9. Availability 是否真正表示 byte ranges？
10. Scheduler 是否真正影响网络/读取优先级？
11. 旧 seek 是否会被取消/降级？
12. App restart 后 Resource 能否恢复？
13. 是否存在数据损坏风险？
14. 是否有 P0？
15. 哪些限制来自 WebView2？
16. 哪些限制来自 HTTP server？
17. 哪些限制来自 rqbit？
18. 哪些问题留到 M5？
19. 哪些问题留到 M7？
20. M4 是否真正满足进入 M5 的条件？
```

---

# 212. 最终报告格式

最终输出：

```text
# M4 Report

## 1. Executive Summary

## 2. FACTS

## 3. M0–M3 Baseline

## 4. Resource Architecture

## 5. Availability Architecture

## 6. Demand Architecture

## 7. Scheduler

## 8. HTTP Progressive Streaming

## 9. Torrent Progressive Streaming

## 10. Stream Gateway

## 11. PlaybackSession

## 12. WebView2 Integration

## 13. Seek Behavior

## 14. Cancellation

## 15. Persistence / Recovery

## 16. Security

## 17. Test Matrix

## 18. End-to-End Results

## 19. Performance

## 20. FACT / HYPOTHESIS / INFERENCE

## 21. RESOLVED / CONFIRMED / OPEN / WORSENED

## 22. P0 / P1

## 23. Known Limitations

## 24. Changed Files

## 25. M5 Readiness

## 26. M7 Readiness

## 27. Definition of Done
```

---

# 213. M4 最终验收标准

Tide 至少必须能够真实完成下面这条链：

```text
User chooses legal HTTP media
        ↓
HTTP Download starts
        ↓
Partial bytes become available
        ↓
PlaybackSession created
        ↓
Gateway URL created
        ↓
WebView2 requests Range
        ↓
Gateway checks Availability
        ↓
Missing range becomes Demand
        ↓
Scheduler prioritizes Demand
        ↓
HTTP Range fetch completes
        ↓
Availability updated
        ↓
Gateway streams bytes
        ↓
Video plays
        ↓
User seeks
        ↓
New Demand created
        ↓
Old Demand cancelled/deprioritized
        ↓
New range becomes available
        ↓
Video resumes
```

以及：

```text
User chooses legal Torrent
        ↓
librqbit metadata ready
        ↓
Partial piece availability exists
        ↓
PlaybackSession created
        ↓
Gateway requests Range
        ↓
TorrentResource maps byte range → required pieces
        ↓
Scheduler prioritizes playback demand
        ↓
rqbit obtains pieces
        ↓
Availability updates
        ↓
Gateway streams
        ↓
WebView2 plays
        ↓
User seeks
        ↓
new byte-range Demand
        ↓
required pieces prioritized
        ↓
playback resumes
```

---

# 214. 最后的工程纪律

你现在不是在“做一个下载器加一个播放器”。

你是在建立 Tide 最关键的一层：

```text
Resource
Availability
Demand
Scheduler
Gateway
PlaybackSession
```

因此必须优先保证：

```text
正确性
可恢复性
可取消性
安全性
一致性
可测试性
```

其次才是：

```text
性能
UI
便利功能
```

尤其不要因为 Demo 看起来“已经可以播放”就宣称 M4 完成。

必须证明：

```text
HTTP progressive playback
HTTP seek
Torrent progressive playback
Torrent seek
Range correctness
Availability correctness
Gateway security
Demand cancellation
Restart recovery
```

全部真实成立。

---

# 215. 最终停止条件

当以下条件全部满足：

```text
M4 DoD = PASS
P0 = 0
cargo check --locked = PASS
cargo test --locked = PASS
frontend typecheck = PASS
frontend build = PASS
E2E HTTP playback = PASS
E2E HTTP seek = PASS
E2E Torrent playback = PASS
E2E Torrent seek = PASS
Gateway security = PASS
restart recovery = PASS
performance baseline = RECORDED
documentation = COMPLETE
```

立即停止 M4。

不要自动继续实现：

```text
M5
M6
M7
```

并输出最终报告。

如果某项无法达到，必须明确写：

```text
BLOCKED
```

并说明：

```text
FACT
ROOT CAUSE
IMPACT
WORKAROUND
RECOMMENDED NEXT MILESTONE
```

不得为了得到 PASS 而降低测试标准。
