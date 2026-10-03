# ifuyo Tide — M1 HTTP Download Core 完整开发实施提示词

> **阶段：M1**
>
> **目标：建立 Tide 第一版真正可用、可测试、可恢复扩展的 HTTP/HTTPS 下载内核。**
>
> **本阶段只解决 HTTP/HTTPS 下载。**
>
> Torrent、librqbit、Progressive Streaming、Range Gateway、Player、Library、Scheduler 智能调度、Windows Shell Integration 等都不在本阶段实现。
>
> M1 的目标不是堆 UI，而是建立一个可靠的 `DownloadEngine`，让后续 M2 持久化、M3 Torrent、M4 Progressive Streaming 可以直接复用。

---

# 0. 最高优先级规则

开始编码之前，**必须先检查当前仓库实际状态**。

必须检查：

- M0 是否已经完成；
- 当前 Rust / Tauri / TypeScript / React / Vite 版本；
- Cargo.lock；
- 当前目录结构；
- M0 已存在的 `Task / Source / Resource / Storage / Security / AppState / IPC`；
- 当前错误体系；
- 当前 SQLite migration/repository；
- 当前 capability / CSP；
- M0 已留下的 M0.5 技术 Spike 是否已经修改过任何公共接口；
- 当前测试状态；
- 当前是否存在未提交修改。

**不要假设项目是干净仓库。**

先输出：

```text
FACTS
HYPOTHESES
CURRENT M0 STATE
M1 IMPLEMENTATION PLAN
RISKS
```

然后再开始修改。

不能因为发现 M0 某处“不够漂亮”就顺手重构整个项目。

---

# 1. M1 的核心目标

M1 完成后，Tide 应具备一个真实工作的 HTTP/HTTPS 下载内核：

```text
HTTP/HTTPS URL
      ↓
URL Validation
      ↓
HTTP Client
      ↓
Response Metadata
      ↓
DownloadEngine
      ↓
Temporary File
      ↓
Progress / Throughput / Cancellation
      ↓
Completed File
```

需要支持：

```text
HTTP GET
HTTPS GET
Content-Length
Chunked Transfer
Redirect
Download Progress
Speed Calculation
ETA
Pause
Resume
Cancel
Retry
Connection Failure Recovery
Partial Download
HTTP Range Resume
ETag / Last-Modified Validator
Checksum
Atomic Finalization
Filename Resolution
Content-Type Detection
Disk Space Checks
```

但：

**M1 不实现数据库级下载任务恢复。**

这属于 M2。

M1 应提供：

```text
DownloadEngine
DownloadRequest
DownloadState
DownloadProgress
DownloadResult
DownloadError
DownloadHandle
```

供 M2 接入持久化。

---

# 2. M1 与后续架构边界

必须严格保持下面边界：

```text
M0
Foundation
   ↓
M1
HTTP Download Engine
   ↓
M2
Persistent Task / Crash Recovery
   ↓
M3
Torrent Engine
   ↓
M4
Progressive Stream / Range Gateway / Scheduler
   ↓
M5
Player
```

不要在 M1 中提前实现：

```text
Torrent
Magnet
DHT
Tracker
Peer
Piece
librqbit
Streaming Gateway
Player
HTML <video>
libmpv
FFmpeg
VLC
Subtitle
Thumbnail
Library
NAS
UPnP
mDNS
Cloud
Account
Telemetry
Smart Scheduler
```

---

# 3. 最重要的架构原则

## 3.1 DownloadEngine 必须与 UI 解耦

不要写成：

```text
React Button
    ↓
download()
    ↓
reqwest
    ↓
write file
```

应形成：

```text
Command / Application Layer
            ↓
        DownloadEngine
            ↓
      HttpTransport
            ↓
        FileSink
```

其中：

```text
DownloadEngine
```

负责：

- 下载流程；
- 状态；
- resume；
- retry；
- progress；
- cancellation；
- validator；
- 完成处理。

HTTP transport 负责：

- HTTP request；
- headers；
- response；
- redirect；
- network errors。

文件层负责：

- 创建临时文件；
- seek；
- append / ranged write；
- flush；
- finalize。

UI 只消费 DTO / events。

---

# 4. 推荐目录

根据现有 M0 结构进行最小修改。

优先考虑：

```text
src-tauri/src/core/task/
src-tauri/src/core/source/
src-tauri/src/core/resource/
src-tauri/src/core/storage/
```

增加类似：

```text
src-tauri/src/core/download/
    mod.rs
    engine.rs
    request.rs
    response.rs
    progress.rs
    state.rs
    retry.rs
    validator.rs
    filename.rs
    checksum.rs
    error.rs
    transport.rs
    file_sink.rs
```

如果现有结构有更合理位置，以实际 M0 架构为准。

**不要为了 M1 发明新的顶层架构。**

---

# 5. 核心领域对象

## 5.1 DownloadRequest

至少需要：

```text
url
destination
optional filename
overwrite policy
optional expected size
optional expected checksum
checksum algorithm
retry policy
```

其中：

```text
url
```

必须经过 Rust 侧安全验证。

---

# 6. URL 安全边界

M1 只接受：

```text
http://
https://
```

明确拒绝：

```text
file://
ftp://
javascript:
data:
blob:
ws:
wss:
custom protocol
```

不得通过字符串拼接绕过 scheme 校验。

---

# 7. Redirect 安全

必须处理 HTTP Redirect。

至少测试：

```text
301
302
303
307
308
```

需要确认：

```text
HTTP → HTTPS
HTTPS → HTTPS
HTTPS → HTTP
HTTP → HTTP
```

具体策略不要凭经验猜测，依据现有 HTTP 客户端实际行为并在文档中记录。

必须防止：

```text
redirect loop
excessive redirects
unsupported scheme redirect
```

建议：

```text
最大 redirect 次数有限制
```

例如：

```text
10
```

具体数值可以根据实际实现选择。

---

# 8. SSRF / 本地地址安全

M1 的下载器虽然是桌面应用，但仍然需要建立安全基线。

必须检查：

- localhost；
- loopback；
- private IPv4；
- link-local；
- IPv6 loopback；
- IPv6 link-local；
- 内部解析地址；
- redirect 后地址变化。

不要简单只检查：

```text
hostname == localhost
```

也不要只检查 URL 字符串。

因为：

```text
domain → private IP
redirect → private IP
DNS rebinding
```

都可能绕过简单字符串检查。

M1 默认策略：

```text
公网 HTTP/HTTPS 下载允许
本机 / 私有网络地址默认拒绝
```

除非项目已有明确的 LAN 下载策略。

注意：

**不要在 M1 为 NAS/局域网下载建立完整产品功能。**

---

# 9. 文件系统安全

下载路径绝不能直接：

```text
root + user_input
```

必须：

```text
canonical / normalized destination
containment check
```

至少防止：

```text
..
absolute path escape
invalid Windows filename
reserved device names
trailing dot
trailing space
colon abuse
UNC path abuse
reparse-point / junction escape
```

Windows 下尤其注意：

```text
CON
PRN
AUX
NUL
COM1...
LPT1...
```

以及路径规范化。

下载文件必须位于用户明确选择的下载目录中。

---

# 10. 文件名解析

下载文件名优先级建议：

```text
1. 用户显式指定 filename
2. Content-Disposition filename*
3. Content-Disposition filename
4. URL path 最后一级
5. fallback filename
```

必须处理：

```text
UTF-8
percent encoding
quoted filename
filename*
空文件名
非法 Windows 字符
路径分隔符
超长文件名
```

服务器提供的 filename：

**只能作为文件名，不能成为路径。**

例如服务器不能通过：

```text
filename="../../secret.exe"
```

逃逸下载目录。

---

# 11. 临时文件策略

不要直接：

```text
download -> final file
```

推荐：

```text
target.ext
target.ext.part
```

或者 UUID 临时文件方案。

下载过程中写入：

```text
.part
```

完成后：

```text
flush
sync
close
atomic rename
```

最终：

```text
.part → final
```

尽量做到崩溃时不会产生一个看起来“完整”的损坏正式文件。

---

# 12. Partial Download

这是 M1 最重要的功能之一。

假设：

```text
target.ext.part
```

已经存在：

```text
size = 500 MB
```

而服务器资源：

```text
size = 2 GB
```

如果服务器支持 Range：

```http
Range: bytes=524288000-
```

继续下载剩余部分。

HTTP Range 的正式语义以 RFC 9110 为准：

```text
206 Partial Content
Content-Range
Accept-Ranges
416 Range Not Satisfiable
```

不要根据自己猜测的协议行为实现。

---

# 13. Resume 必须有 Validator

不能看到：

```text
.part > 0
```

就无条件：

```text
Range: bytes=<local_size>-
```

必须考虑远端资源是否已经发生变化。

优先保存：

```text
ETag
Last-Modified
Content-Length
```

恢复时优先使用：

```text
Range
If-Range
```

或其它合理的条件请求策略。

RFC 9110 明确规定了 `If-Range` 用于让客户端在资源未变化时恢复 Range，否则获取新的完整表示。

必须测试：

```text
same ETag
changed ETag
missing ETag
same Last-Modified
changed Last-Modified
missing validator
```

---

# 14. 服务器不支持 Range

必须处理：

```text
Accept-Ranges: none
```

或者：

```text
服务器直接忽略 Range
返回 200
```

客户端不能假设：

```text
Accept-Ranges: bytes
```

就永远会得到：

```text
206
```

HTTP 规范明确指出服务器可以忽略 Range，而且 `Accept-Ranges` 本身并不保证后续请求一定返回 partial response。

策略应明确：

```text
有 partial + server 返回 206
    → resume

有 partial + server 返回 200
    → 判断是否安全
    → 不安全则不要把完整 body 追加到 partial
    → 重新建立完整下载

有 partial + 416
    → 根据 Content-Range / remote size 判断
    → 如果本地已经完整，则进入 finalize
    → 否则视为 resume 不可用并重新开始
```

绝对不能出现：

```text
partial file
+
200 full response
=
append
```

这种最容易产生“文件大小看似正确、实际内容损坏”的 bug。

---

# 15. Content-Length

需要处理：

```text
Content-Length 存在
Content-Length 不存在
Content-Length 与实际 body 不一致
```

进度允许：

```text
known total:
downloaded / total

unknown total:
downloaded / unknown
```

不要为了显示百分比而伪造 total。

UI 不允许出现：

```text
57%
```

但后台实际上不知道总大小。

---

# 16. Chunked Transfer

必须测试：

```text
Transfer-Encoding: chunked
```

如果没有：

```text
Content-Length
```

下载仍然可以成功。

此时：

```text
progress.total = unknown
percentage = null
eta = null
```

但：

```text
downloaded bytes
speed
elapsed
```

仍然应该正常统计。

---

# 17. Content-Encoding

这是一个容易产生隐性 bug 的地方。

下载器最终保存的是：

```text
用户希望得到的原始文件
```

而不是 HTTP 内容编码解压后的错误字节范围。

必须确认 HTTP client 的自动 compression / decompression 行为。

尤其检查：

```text
gzip
br
deflate
Range request
```

不要出现：

```text
服务器返回压缩 representation
客户端自动解压
本地按解压后的 bytes resume
```

造成 Range / Content-Length / 本地文件长度语义冲突。

优先采用：

```text
Accept-Encoding: identity
```

或显式禁用会破坏 byte-for-byte 下载语义的自动解压。

最终行为必须通过真实测试确认。

---

# 18. HTTP 状态码策略

至少明确：

```text
2xx → success / body processing

3xx → redirect handling

401 → authentication required / failure
403 → forbidden
404 → not found
408 → retry candidate
409 → server-specific failure
429 → retry-after
5xx → retry candidate

4xx other than transient cases
    → generally non-retryable
```

不要简单：

```text
if status != 200 => fail
```

Range 情况必须接受：

```text
206
```

并正确处理：

```text
200
206
416
```

---

# 19. Retry

M1 要有基础 retry 机制。

但不要实现复杂“智能调度器”。

只解决：

```text
network timeout
connection reset
temporary connection failure
temporary 5xx
429 with Retry-After
```

建议：

```text
exponential backoff
jitter
maximum attempts
maximum delay
```

不要对明显不可重试错误无限 retry：

```text
404
401
403
invalid URL
invalid destination
checksum mismatch caused by stable remote content
```

---

# 20. Retry 与 Resume 联动

例如：

```text
开始
  ↓
下载 100 MB
  ↓
connection reset
  ↓
保存 partial
  ↓
重新建立 request
  ↓
Range resume
```

而不是：

```text
connection reset
↓
删除 partial
↓
从 0 开始
```

必须测试：

```text
retry from zero
retry from partial
multiple interruptions
```

---

# 21. Pause / Cancel / Abort

M1 需要真正的取消能力。

设计：

```text
DownloadHandle
    ├── cancel()
    └── pause()
```

但需要区分：

```text
pause
cancel
```

## Pause

意味着：

```text
保留 partial
释放 HTTP connection
进入 Paused
```

未来：

```text
resume
→ Range
```

## Cancel

需要定义产品语义：

建议：

```text
取消下载
→ 停止 network
→ 删除或者保留 partial
```

具体策略必须在 M1 文档中拍板。

推荐：

```text
Cancel = 删除临时下载数据
Pause = 保留临时下载数据
```

如果实际产品希望支持“取消但保留数据”，应另设状态，不要让语义混乱。

---

# 22. Cancellation Safety

取消不能导致：

```text
panic
deadlock
zombie task
HTTP connection 泄漏
file handle 泄漏
temporary file lock
```

必须测试：

```text
download startup cancel
cancel while receiving body
cancel while retry waiting
cancel while writing
cancel near completion
```

尤其：

```text
cancel during rename
```

---

# 23. DownloadState

M1 的状态至少建议：

```text
Created
Preparing
Downloading
Paused
Retrying
Completed
Failed
Cancelled
```

如果 M0 已定义 TaskState：

```text
Created
Ready
Running
Paused
Completed
Failed
Cancelled
```

不要重新发明第二套平行任务状态。

可以：

```text
TaskState
+
DownloadPhase
```

例如：

```text
TaskState = Running
DownloadPhase = Retrying
```

但优先复用 M0 的状态模型。

---

# 24. Progress 数据

不要只发送：

```text
percent
```

建议：

```text
DownloadProgress {
    downloaded_bytes
    total_bytes
    percentage
    instant_speed
    smoothed_speed
    elapsed
    eta
    state
}
```

其中：

```text
total_bytes = null
```

是合法状态。

---

# 25. Speed 统计

不要简单：

```text
total_downloaded / total_elapsed
```

作为 UI 当前速度。

需要区分：

```text
instant speed
smoothed speed
```

建议使用滑动窗口或指数平滑。

例如：

```text
instant = recent_bytes / recent_duration
smoothed = EMA(...)
```

目标：

```text
UI 不抖动
测速不会因为单个 chunk 突然跳变
```

---

# 26. ETA

只有：

```text
total known
speed > 0
```

时计算：

```text
remaining / speed
```

否则：

```text
null
```

绝对不要显示：

```text
ETA: 00:00
```

作为未知值。

---

# 27. Progress Event 频率

不要每收到一个网络 chunk 就：

```text
Rust → IPC → React
```

否则高速下载会导致 IPC 洪泛。

应设置事件节流，例如：

```text
100–250 ms
```

范围内可调。

UI 更新应与底层写盘频率解耦。

---

# 28. Disk Space

开始下载前需要进行基本磁盘检查。

如果：

```text
Content-Length known
```

尝试检查：

```text
available disk space
```

并在明显不足时提前失败。

同时注意：

```text
Content-Length unknown
```

不能提前判定最终大小。

下载过程中仍需处理：

```text
disk full
write failure
filesystem error
```

错误必须转为结构化：

```text
DiskFull
WriteFailed
PermissionDenied
PathInvalid
```

而不是把底层字符串直接扔给 UI。

---

# 29. 文件写入

必须支持：

```text
large file
> 4GB
```

任何计数、seek、range 坐标都必须使用：

```text
u64
```

不要使用：

```text
u32
usize
```

作为跨层的文件大小协议字段。

尤其 Windows 下必须实际测试：

```text
2GB+
4GB+
```

如果本机没有超大测试文件，可以使用：

```text
稀疏文件
本地测试服务器
程序生成测试 payload
```

而不要下载大型版权内容进行测试。

---

# 30. Checksum

M1 增加基础 checksum 能力。

至少支持：

```text
SHA-256
```

结构：

```text
optional expected checksum
↓
download
↓
final checksum
↓
compare
```

结果：

```text
match
mismatch
not configured
```

checksum mismatch：

```text
不能进入 Completed
```

而应该：

```text
Failed / IntegrityError
```

必要时保留 `.part` 供诊断。

---

# 31. MIME / Content-Type

下载器记录：

```text
Content-Type
```

但：

**不要因为 MIME 就擅自决定一定是什么文件。**

例如：

```text
application/octet-stream
video/mp4
application/zip
```

都可以作为普通下载。

M1 不做：

```text
媒体探测
FFmpeg probe
缩略图
播放器识别
```

只保存 metadata。

---

# 32. HTTP HEAD

可以使用：

```text
HEAD
```

进行 metadata probing。

但不能假设所有服务器：

```text
HEAD == GET
```

必须支持：

```text
HEAD 不可用
HEAD 返回异常
HEAD Content-Length 与 GET 不一致
```

因此最终真实下载仍必须以：

```text
GET response
```

为准。

不要为了“提前显示大小”让 HEAD 成为强依赖。

---

# 33. HTTP Range 下载抽象

M1 虽然主要是完整下载，但底层应提前建立：

```text
read_range(range)
```

或等价的 transport abstraction。

这是为了 M4：

```text
Player
 ↓
Range Gateway
 ↓
Resource
 ↓
HTTP Range / Torrent Piece
```

M1 不实现 M4 的 gateway。

但不要把 HTTP 下载实现成：

```text
reqwest_response → directly write until EOF
```

以后完全无法复用。

---

# 34. Resource 抽象

继续沿用 M0 中：

```text
Resource
```

概念。

HTTP Resource 至少能表达：

```text
identity
size
content_type
source
availability
```

M1 可以增加：

```text
HttpResource
```

或实现：

```text
ResourceProvider
```

但必须避免现在就制造：

```text
HttpResource
TorrentResource
MagnetResource
LocalResource
RemoteResource
...
```

一大堆没有真实需求的层级。

目标只是：

```text
HTTP 是第一个真实 Resource Provider
```

---

# 35. Availability

M4 需要：

```text
availability(range)
```

因此 M1 应能表达：

```text
HTTP Downloaded File

[0, downloaded_bytes)
    AVAILABLE
```

例如：

```text
total = 1000 MB
downloaded = 300 MB
```

则：

```text
[0,300MB)
available

[300MB,1000MB)
unavailable
```

这为以后统一：

```text
HTTP
Torrent
Local File
```

的 Resource 语义做准备。

但：

**不要在 M1 实现通用 Scheduler。**

---

# 36. Local Test Server

必须建立一个本地测试 HTTP server。

推荐测试场景：

```text
/static/small.bin
/static/medium.bin
/static/large.bin
/range/supported.bin
/range/unsupported.bin
/chunked/test.bin
/redirect/...
/slow/...
/unstable/...
/etag/...
/last-modified/...
/status/...
```

不要依赖公网站点作为自动化测试基础。

---

# 37. Test Payload

测试 payload 应：

```text
可重复生成
可验证
无版权风险
```

例如：

```text
deterministic bytes
```

生成：

```text
sha256
```

然后所有测试最后验证最终文件 checksum。

---

# 38. 必须测试的 HTTP 场景

至少覆盖：

```text
200 full download
206 resume
416 invalid range
404
403
500
502
503
429
redirect
timeout
connection reset
chunked
unknown content length
Content-Disposition
UTF-8 filename
filename*
ETag
Last-Modified
Range unsupported
partial file
zero-byte file
large file
```

---

# 39. Range Resume 测试

至少做以下矩阵：

```text
Case A
local = 0
remote = known
server supports Range
→ full download

Case B
local partial
ETag same
→ Range resume

Case C
local partial
ETag changed
→ don't append stale representation

Case D
local partial
server ignores Range
→ don't corrupt file

Case E
local == remote size
→ verify/finalize, not blind append

Case F
local > remote size
→ reject/restart safely

Case G
server returns 416
→ recover safely

Case H
network interruption during resume
→ retain valid partial and retry
```

---

# 40. Atomic Completion

只有以下全部成功后：

```text
HTTP body complete
size valid when known
checksum valid when configured
file flush successful
```

才允许：

```text
.part → final
```

否则：

```text
Completed
```

不能发生。

---

# 41. Existing File Policy

M1 必须明确：

如果：

```text
destination/final.ext
```

已经存在怎么办？

至少设计：

```text
Fail
Overwrite
AutoRename
```

不要默认直接覆盖用户文件。

如果产品目前只支持一种策略：

可以先实现：

```text
Fail
```

但接口必须预留扩展。

---

# 42. Concurrent Download

M1 不做完整 Scheduler。

但是：

`DownloadEngine` 不能假设系统永远只有一个任务。

应至少确保：

```text
Task A
Task B
Task C
```

并发调用不会：

```text
共享错误状态
覆盖临时文件
污染 progress
串错 cancellation token
```

M1 可以设置简单：

```text
max concurrent downloads
```

例如：

```text
1–3
```

但不要在此阶段实现复杂优先级算法。

复杂调度属于 M7。

---

# 43. Download ID

每次 download：

```text
download_id
```

必须唯一。

不要使用：

```text
URL
filename
timestamp only
```

作为唯一 ID。

复用 M0 的 UUID / ID 策略。

---

# 44. Cancellation Token

Rust 内部推荐使用结构化 cancellation mechanism，例如：

```text
CancellationToken
```

或与现有架构等价的方式。

要求：

```text
cancel()
```

必须能中断：

```text
request
body stream
retry delay
file loop
```

不能只在 chunk 处理完才检查。

---

# 45. Error Model

继续使用 M0 的：

```text
TideError
```

不要重新创造另一套：

```text
DownloadError
```

如果确有领域错误：

可以：

```text
TideError::Download(...)
```

例如：

```text
InvalidUrl
UnsupportedScheme
RedirectLimitExceeded
HttpStatus
Network
Timeout
Cancelled
DiskFull
PermissionDenied
FileWrite
ResumeNotSupported
RemoteChanged
ChecksumMismatch
InvalidContentRange
```

错误 DTO 给前端时：

```text
code
message
recoverable
```

不要把：

```text
Rust backtrace
full reqwest error chain
full URL
Authorization headers
```

直接返回前端。

---

# 46. Sensitive Logging

生产日志禁止默认记录：

```text
完整 URL
完整 query string
Authorization
Cookie
Magnet
InfoHash
Peer IP
用户下载历史
完整本地隐私路径
```

下载日志更适合：

```text
download_id
host
status
bytes
state
error_code
elapsed
```

例如：

```text
download=abc123 host=example.com state=retrying
```

而不是：

```text
https://example.com/private/file?token=...
```

---

# 47. IPC

M1 需要增加最小真实下载 IPC。

例如：

```text
start_http_download
pause_download
resume_download
cancel_download
get_download_progress
```

具体命令名称以 M0 现有命名风格为准。

不要暴露：

```text
reqwest
DownloadEngine internals
filesystem handles
raw HTTP response
```

IPC 使用 DTO。

---

# 48. Event Model

下载进度建议通过：

```text
Tauri event
```

而不是 React 轮询高频请求。

例如：

```text
download://progress
download://state
download://completed
download://failed
```

但事件数量要克制。

原则：

```text
domain state → event DTO → frontend
```

不能：

```text
Rust internal struct
直接 serialize 全部
```

---

# 49. Frontend UI

M1 UI 只需要一个**真实下载验证界面**。

不需要追求最终视觉。

至少支持：

```text
URL input
Destination selection
Start
Pause
Resume
Cancel
```

显示：

```text
filename
state
downloaded
total
speed
ETA
error
```

禁止：

```text
fake progress
fake speed
fake buffer
```

所有数据必须来自真实 `DownloadEngine`。

---

# 50. M1 UI 不做

不要提前实现：

```text
Download queue designer
Drag & drop sorting
Smart priority
Speed graph
Category system
Media poster
Library
Search
Filters
Torrent UI
Player
Streaming
```

---

# 51. Resume Demo

最终必须能够演示：

```text
下载一个测试文件
        ↓
运行到 30%
        ↓
Pause
        ↓
关闭/停止 download
        ↓
再次 Resume
        ↓
HTTP Range
        ↓
继续下载
        ↓
完成
```

M1 的 resume 可以通过：

```text
内存 download session
+
现有 partial 文件
```

实现。

真正的：

```text
应用崩溃
应用重启
自动恢复 task
```

放到 M2。

---

# 52. Failure Injection

自动化测试必须支持人为制造：

```text
connection reset
slow response
timeout
5xx
partial transfer
disk failure where practical
```

目标不是测试公网。

而是验证：

```text
DownloadEngine
```

在坏环境下的状态转换。

---

# 53. 单元测试

至少：

```text
URL validation
filename sanitization
filename parsing
redirect policy
HTTP status classification
retry decision
Range parsing
Content-Range parsing
resume decision
validator comparison
progress calculation
speed calculation
ETA calculation
destination policy
checksum
state transition
```

---

# 54. 集成测试

至少：

```text
full HTTP download
HTTP resume
Range unsupported fallback
redirect
chunked
unknown length
retry
cancel
pause/resume
checksum success
checksum mismatch
invalid server response
```

---

# 55. Property / Fuzz 思路

对于这些函数特别适合做 property-style testing：

```text
Range parser
Content-Range parser
Filename sanitizer
Path containment
Progress calculation
```

重点验证：

```text
任何非法输入不会 panic
```

例如：

```text
bytes=-1
bytes=abc
bytes=100-50
bytes=18446744073709551615-
```

以及：

```text
Content-Range malformed
```

---

# 56. 不能只测试“200 OK”

非常重要：

Agent 不允许以：

```text
curl 某个 URL 成功
```

作为 M1 完成证明。

必须有：

```text
server controlled test
+
unit tests
+
integration tests
+
manual test
```

---

# 57. M1 文档

至少新增：

```text
docs/M1-http-download.md
docs/M1-resume-and-range.md
docs/M1-error-model.md
docs/M1-security.md
docs/M1-test-matrix.md
```

---

# 58. M1-http-download.md

记录：

```text
DownloadEngine architecture
HTTP transport
file sink
state model
progress events
retry
cancellation
checksum
filename resolution
completion
```

---

# 59. M1-resume-and-range.md

明确写清：

```text
Range semantics
206
200 fallback
416
ETag
Last-Modified
If-Range
partial file
remote changed
server without Range
```

尤其记录：

```text
when resume is allowed
when resume must restart
```

不要留下“实现自行判断”的模糊描述。

---

# 60. M1-error-model.md

建立真实错误矩阵：

```text
error code
source
recoverable?
retry?
user action?
```

例如：

```text
NETWORK_TIMEOUT
recoverable=yes
retry=yes

HTTP_404
recoverable=no
retry=no

DISK_FULL
recoverable=yes
retry=after user action

CHECKSUM_MISMATCH
recoverable=yes
retry=policy-dependent
```

---

# 61. M1-security.md

记录：

```text
URL validation
redirect policy
SSRF policy
path containment
filename sanitization
temporary file handling
sensitive logging
resource limits
```

---

# 62. Resource Limits

桌面软件也不能无限制吃资源。

至少设置合理边界：

```text
max redirect
max retries
request timeout
connect timeout
idle timeout where applicable
max filename length
max header-related assumptions
```

M1 不需要复杂防御，但必须避免：

```text
无限 redirect
无限 retry
无限等待
无限 IPC event
```

---

# 63. HTTP Client 生命周期

HTTP client 应尽可能：

```text
reuse
connection pooling
```

不要每个 chunk：

```text
new client
```

也不要每个下载：

```text
new global mutable singleton
```

应该由：

```text
AppState dependency
```

持有合理生命周期。

---

# 64. 不要建立全局万能 Downloader Singleton

不要写：

```rust
static GLOBAL_DOWNLOADER: ...
```

也不要让所有模块直接访问：

```text
GLOBAL_APP_STATE
```

继续使用 M0 的：

```text
AppState
Dependency Injection
Repository / Service boundaries
```

---

# 65. 与 SQLite 的边界

M1 不负责完整的 Task persistence。

但必须给 M2 留出明确接口：

```text
DownloadProgress
DownloadCheckpoint
DownloadMetadata
```

未来 M2 可以保存：

```text
url
destination
partial path
downloaded bytes
total
etag
last_modified
filename
checksum
state
```

M1 不要自己散落着写 SQL。

---

# 66. Checkpoint 接口

为了 M2，可以设计：

```text
DownloadCheckpoint
```

包含：

```text
local_bytes
remote_total
etag
last_modified
resource metadata
```

但：

**M1 可以只提供内存 callback / result，不要求完整数据库同步。**

---

# 67. HTTP Resource 与 DownloadEngine 的关系

必须避免错误架构：

```text
HttpResource = DownloadEngine
```

正确理解：

```text
Resource
    =
    某一份可寻址的数据

DownloadEngine
    =
    把 Resource 从远端搬到本地的执行器
```

例如：

```text
HTTP Resource
    ↓
DownloadEngine
    ↓
Local File
```

未来：

```text
Torrent Resource
    ↓
Torrent Engine
    ↓
Local File
```

而 M4：

```text
Resource
    ↓
read(range)
    ↓
stream
```

---

# 68. 为 M4 预留的关键能力

M1 完成后，至少应该验证这个概念：

```text
Resource
 ├── size()
 ├── content_type()
 ├── availability(range)
 └── read(range)
```

HTTP 下载过程中：

```text
downloaded:
[0,n)
```

就是 available。

这不是要求 M1 做 Streaming。

这是要求：

**M1 的数据模型不要把“下载”和“数据可读性”彻底绑死。**

---

# 69. M1 不要实现 Progressive Playback

即使 M0.5 已经证明：

```text
HTTP partial
→ Range
→ WebView2
```

M1 也不要直接把：

```text
Downloader
+
Player
```

绑死。

M1 只负责：

```text
下载
```

M4 才负责：

```text
Progressive Stream Gateway
```

M5 才负责：

```text
Player
```

---

# 70. 实际依赖原则

首先查看当前：

```text
Cargo.toml
Cargo.lock
```

如果 M0 已经有 HTTP client：

```text
直接复用
```

不要无理由升级。

如果尚未引入：

根据当前项目状态选择成熟 HTTP client。

例如 Rust 生态中 `reqwest` 当前文档已经提供标准 HTTP Range / Content-Range header 支持，但实际 API 必须以项目锁定版本为准。

不要让 Agent：

```text
cargo update
```

顺带升级整个依赖树。

---

# 71. Cargo Lock

必须保持：

```text
Cargo.lock
```

一致。

最终执行：

```bash
cargo check --locked
cargo test --locked
```

前端：

```bash
npm / pnpm / yarn
```

必须根据当前项目实际 package manager 执行对应 frozen/lockfile-safe 命令。

不要修改 package manager。

---

# 72. M1 性能验证

不需要做 benchmark 平台。

但至少观察：

```text
CPU idle
CPU during download
RAM
disk write behavior
IPC event rate
```

测试：

```text
small file
medium file
large file
```

特别注意：

```text
高速度下载时 React 不应明显卡顿
```

---

# 73. M1 Manual Demo

最终 Agent 必须自己完成一次真实演示。

推荐流程：

```text
1. 启动 Tide
2. 输入本地测试 HTTP URL
3. 指定下载目录
4. 开始下载
5. 查看实时速度/进度
6. Pause
7. 检查 .part
8. Resume
9. 确认 Range request
10. 完成
11. SHA-256 校验
12. 确认 final file
13. 删除测试文件
```

然后：

```text
故意中断网络
↓
恢复
↓
确认 retry/resume
```

---

# 74. M1 完成后的真实能力

必须最终达到：

```text
Tide
  ↓
输入 HTTP/HTTPS URL
  ↓
安全校验
  ↓
解析 filename
  ↓
启动 DownloadEngine
  ↓
真实下载
  ↓
显示真实速度 / 进度
  ↓
Pause
  ↓
Resume
  ↓
Range
  ↓
网络异常 Retry
  ↓
Checksum
  ↓
Atomic Finalize
  ↓
真实文件
```

---

# 75. M1 不应该出现的代码

最终检查：

```text
librqbit
torrent
magnet
DHT
tracker
peer
piece
WebView player
video element
FFmpeg
libmpv
stream gateway
Library
NAS
UPnP
Steam
telemetry
account
cloud
```

如果出现：

```text
不要默认删除
```

而要判断是不是：

```text
M0.5 实验遗留
```

或者：

```text
误实现的后续功能
```

必要时清理。

---

# 76. Definition of Done

M1 只有满足下面条件才算完成：

## Architecture

```text
[ ] DownloadEngine 独立
[ ] UI 与下载逻辑解耦
[ ] HTTP transport 独立
[ ] filesystem sink 独立
[ ] TideError 统一
[ ] DTO 边界清晰
[ ] AppState / DI 延续 M0
```

## HTTP

```text
[ ] HTTP
[ ] HTTPS
[ ] 301/302/303/307/308
[ ] Content-Length
[ ] Chunked
[ ] Range
[ ] 206
[ ] 416
[ ] ETag
[ ] Last-Modified
[ ] If-Range / 等价安全恢复逻辑
[ ] Content-Disposition
```

## Download

```text
[ ] 正常下载
[ ] Pause
[ ] Resume
[ ] Cancel
[ ] Retry
[ ] Timeout
[ ] Network error
[ ] Disk error
[ ] SHA-256
[ ] Atomic finalize
```

## Security

```text
[ ] Scheme validation
[ ] Redirect validation
[ ] SSRF baseline
[ ] Path containment
[ ] Filename sanitization
[ ] Sensitive log filtering
[ ] Resource limits
```

## Testing

```text
[ ] Unit tests
[ ] Integration tests
[ ] Range tests
[ ] Resume tests
[ ] Retry tests
[ ] Cancellation tests
[ ] Filename tests
[ ] Security tests
[ ] Manual test
```

## Documentation

```text
[ ] M1-http-download.md
[ ] M1-resume-and-range.md
[ ] M1-error-model.md
[ ] M1-security.md
[ ] M1-test-matrix.md
```

---

# 77. M1 最终报告

完成后必须输出：

```text
# M1 Implementation Report

## 1. FACTS
实际完成了什么。

## 2. Architecture
实际架构与 M0 如何衔接。

## 3. HTTP Support Matrix
200 / 206 / 416 / redirect / chunked / validator 等。

## 4. Resume Matrix
哪些场景可以 resume，哪些必须 restart。

## 5. Security
做了哪些防护。

## 6. Testing
unit / integration / manual。

## 7. Performance
CPU / memory / IPC / disk observations。

## 8. Files Changed
精确到文件。

## 9. Known Limitations
不要隐藏问题。

## 10. M2 Readiness
M2 需要增加什么。

## 11. M3 Readiness
Torrent Resource 接入还缺什么。

## 12. M4 Readiness
Progressive Streaming 还缺什么。

## 13. Remaining Risks
P0 / P1 only。
```

---

# 78. 最终禁止伪造结果

任何以下表述都必须以真实测试为依据：

```text
PASS
Supported
Reliable
Safe
Crash-proof
Resume works
Large files supported
```

如果没有测试：

```text
NOT TESTED
```

如果只能在当前机器/环境下成立：

```text
ENVIRONMENT-SPECIFIC
```

如果行为取决于服务器：

```text
SERVER-DEPENDENT
```

---

# 79. 最终验收标准

M1 的关键验收路径只有一条：

```text
HTTP/HTTPS URL
    ↓
real request
    ↓
real file writing
    ↓
real progress
    ↓
pause
    ↓
partial file
    ↓
resume with Range when valid
    ↓
network failure + retry
    ↓
checksum
    ↓
atomic finalization
    ↓
verified final file
```

再加一条最重要的错误验收：

```text
partial file
+
server returns 200 instead of valid 206
+
resume
=
绝不能发生错误追加导致文件损坏
```

---

# 80. 给 Agent 的最后指令

**先检查项目，再改代码。**

不要因为“M1 下载器”这个标题而顺手实现：

```text
Torrent
Player
Streaming
Scheduler
Library
```

不要为了未来抽象而创造大量暂时没有实现意义的接口。

优先顺序：

```text
Correctness
>
Data integrity
>
Cancellation / recovery
>
Security
>
Testability
>
Architecture cleanliness
>
Performance
>
UI polish
```

M1 的真正成果不是“界面上出现一个下载按钮”。

真正成果是：

> **Tide 第一次拥有一个可靠的、可暂停、可恢复、可校验、不会轻易损坏文件，并且能够被 M2/M3/M4 继续复用的 HTTP 下载内核。**

完成后，不要自行扩展到 M2/M3/M4。

停止在 M1 边界，提交完整报告。