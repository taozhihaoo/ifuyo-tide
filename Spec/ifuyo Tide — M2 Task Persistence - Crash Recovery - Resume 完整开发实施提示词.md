# ifuyo Tide — M2 Task Persistence / Crash Recovery / Resume 完整开发实施提示词

> **Milestone：M2**
>
> **目标：**
> 将 M1 的 HTTP/HTTPS Download Core 从“当前进程中能够下载”升级为：
>
> **“任务、状态、进度、下载目标、恢复信息可以持久化；程序关闭、崩溃、系统重启后能够恢复任务，并在不破坏已有数据的前提下继续下载。”**
>
> 本阶段仍然只服务于 HTTP/HTTPS 下载。
>
> **M2 不实现 Torrent、Progressive Streaming、Scheduler、Player、Library 产品功能。**
>
> 本阶段最重要的资产不是 UI，而是：
>
> `Task Persistence + Recovery Model + Crash Consistency + Resume Semantics`

---

# 1. 首先检查现状，不要直接改代码

开始实施前，完整审计当前项目，并先输出：

```text
FACTS
HYPOTHESES
CURRENT M0 STATE
CURRENT M1 STATE
M1 IMPLEMENTATION GAPS
M2 IMPLEMENTATION PLAN
RISKS
```

必须实际检查：

```text
项目目录结构
Cargo.toml / Cargo.lock
Tauri 2 配置
Rust module structure
SQLite implementation
database location
migration mechanism
repository implementation
Task domain model
TaskState
DownloadEngine
DownloadRequest
DownloadResult
DownloadProgress
DownloadError
temporary .part file handling
finalization logic
HTTP resume implementation
ETag / Last-Modified handling
checksum handling
IPC commands
frontend task store/state
event system
logging
tests
```

不要假设 M1 已经严格按照旧提示词实现。

如果 M1 中存在：

- persistence 缺失
- task identity 不稳定
- state machine 不完整
- `.part` 文件语义混乱
- resume metadata 不足
- progress 与磁盘实际状态不一致
- 下载状态直接依赖内存对象
- 数据库与文件系统之间存在明显一致性风险

先记录为：

```text
M1 IMPLEMENTATION GAP
```

然后在 M2 中修复。

不要因为“原计划是这样”而保护错误设计。

---

# 2. M2 的唯一核心问题

M2 必须回答：

> **Tide 关闭之后，下次启动时，能不能知道之前到底下载到了哪里，并安全地继续？**

必须覆盖至少这些场景：

```text
正常关闭
用户主动退出
任务暂停
程序崩溃
Windows 强制结束进程
系统重启
下载途中断电（能覆盖模拟场景即可）
网络中断
HTTP 连接中断
数据库写入中断
文件写入中断
程序在 checkpoint 前崩溃
程序在 checkpoint 后崩溃
任务已经完成但数据库尚未完全更新
数据库已经记录进度但 .part 文件没有对应数据
.part 文件比数据库记录更大
.part 文件比数据库记录更小
.remote resource 发生变化
旧任务重新恢复
重复启动恢复
```

核心目标：

```text
Persisted Task
      ↓
Application Restart
      ↓
Recovery Scan
      ↓
Filesystem Reality Check
      ↓
Reconcile DB ↔ .part
      ↓
Reconstruct Runtime DownloadTask
      ↓
Resume HTTP Request
      ↓
Continue Download
```

---

# 3. M2 的范围

## 必须实现

### Task Persistence

任务能够持久化保存：

```text
Task ID
Task State
Source
Target
Destination
Filename
Expected Size
Downloaded Size
Content Type
ETag
Last-Modified
Checksum expectation
Checksum progress / verification status
Created At
Updated At
Started At
Completed At
Last Error
Retry Count
Resume Information
```

具体字段根据 M1 实际设计调整。

---

## 必须实现

### Task Restore

程序启动后：

```text
SQLite
   ↓
load persisted tasks
   ↓
validate records
   ↓
inspect filesystem
   ↓
reconcile
   ↓
restore recoverable tasks
```

UI 不能依赖：

```text
“程序启动后重新创建任务”
```

而必须是真正：

```text
数据库 → Task → Runtime
```

---

## 必须实现

### Crash Recovery

程序异常退出之后：

```text
Tide restarted
      ↓
find tasks in transient states
      ↓
inspect .part file
      ↓
verify metadata
      ↓
repair/reconcile
      ↓
mark task recoverable
```

不能简单：

```text
Running → Running
```

也不能：

```text
Running → Failed
```

然后要求用户手动重新创建。

需要定义明确的恢复语义。

---

## 必须实现

### Resume

已经存在：

```text
file.part
```

时，根据真实文件大小和 validator 信息决定：

```text
resume
restart
repair
complete
fail
```

不能只读取 SQLite 的 `downloaded_bytes`。

---

# 4. M2 明确不做什么

以下全部禁止进入 M2：

```text
Torrent
Magnet
DHT
Tracker
Peer
Piece
librqbit
Progressive Streaming
Streaming Gateway
Player
WebView media playback
libmpv
FFmpeg
Subtitle
Thumbnail
Scheduler
Demand Model
Adaptive streaming
Library product features
NAS
UPnP
mDNS
Casting
Cloud Sync
Account
Telemetry
Steam
Workshop
```

同时不要因为“以后需要”而提前实现复杂抽象。

---

# 5. M2 最核心设计：数据库和文件系统不是同一种状态

明确区分：

## Logical State

SQLite 记录：

```text
Task
TaskState
Source
Resource metadata
resume validators
errors
timestamps
```

---

## Physical State

文件系统真实状态：

```text
.part exists
.part size
final file exists
final file size
filesystem timestamp
```

---

## Runtime State

当前进程实际状态：

```text
DownloadEngine running
HTTP connection
current byte position
current throughput
cancel token
pause signal
retry state
```

三者不能混为一谈。

必须建立明确模型：

```text
Persistent State
Physical State
Runtime State
```

---

# 6. Source of Truth 原则

对于“已经写到磁盘多少字节”，优先相信：

```text
真实 .part 文件长度
```

而不是：

```text
SQLite.downloaded_bytes
```

原因：

如果：

```text
SQLite:
downloaded_bytes = 100 MB

.part:
actual size = 96 MB
```

继续从：

```text
100 MB
```

恢复会跳过 4 MB，导致文件损坏。

反之：

```text
SQLite = 96 MB
.part = 100 MB
```

也不能简单宣称已经下载 100 MB。

因此 M2 必须实现：

```text
reconciliation
```

---

# 7. Task 持久化模型

在当前 M0/M1 Task 模型基础上扩展。

推荐至少具有：

```text
Task
├── id
├── source
├── resource
├── destination
├── filename
├── state
├── created_at
├── updated_at
├── started_at
├── completed_at
├── paused_at
├── downloaded_bytes
├── total_bytes
├── content_type
├── etag
├── last_modified
├── checksum_algorithm
├── checksum_expected
├── checksum_verified
├── retry_count
├── last_error_code
├── last_error_message
├── last_checkpoint_at
├── recovery_status
└── version
```

不要照抄字段。

必须根据实际 M1 结构判断哪些字段已经存在。

---

# 8. Persistent Task State

继续复用 M0 定义的：

```text
Created
Ready
Running
Paused
Completed
Failed
Cancelled
```

但必须重新检查这些状态对“持久化”和“恢复”的意义。

重点解决：

```text
Running
```

不是一个可靠的“重启后状态”。

因为：

> 进程死亡后，不存在“仍然 Running 的 Runtime”。

因此建议区分：

```text
Persisted Task State
```

和：

```text
Runtime Activity
```

例如：

```text
TaskState = Ready
Runtime = inactive
```

而不是把：

```text
Running
```

永久写成一种运行事实。

可以增加：

```text
RecoveryRequired
```

或者使用：

```text
last_runtime_state
```

但必须避免无意义地扩大状态机。

优先采用最简单且可证明正确的模型。

---

# 9. 启动恢复策略

应用启动时必须明确执行：

```text
1. Open database
2. Run migrations
3. Load persisted tasks
4. Detect interrupted/transient tasks
5. Inspect corresponding filesystem artifacts
6. Reconcile each task
7. Reconstruct recoverable tasks
8. Persist corrected state
9. Expose restored tasks to frontend
```

不能：

```text
打开 UI
↓
用户点一下任务
↓
才尝试发现旧任务
```

---

# 10. Recovery Classification

每一个旧任务恢复时都必须分类。

建议至少：

```text
RECOVERABLE
COMPLETED
RESTART_REQUIRED
CORRUPTED_METADATA
MISSING_FILE
MISSING_SOURCE
INVALID_DESTINATION
NEEDS_USER_ACTION
```

但不要无必要地创建大量状态。

最好使用：

```text
RecoveryDecision
```

作为内部结果。

例如：

```text
Resume
Restart
Finalize
Fail
Ignore
```

让 Recovery Logic 成为明确、可测试的纯逻辑。

---

# 11. Recovery Algorithm

建立明确算法。

伪代码类似：

```text
load task

if final file exists:
    inspect final file

if .part file exists:
    inspect .part size

compare:
    database metadata
    .part physical size
    final file

validate:
    destination
    source
    validator
    expected size if known
```

然后：

```text
if valid final artifact:
    verify/finalize task

else if valid .part:
    reconcile downloaded_bytes
    task becomes resumable

else:
    task becomes restartable or failed
```

禁止模糊逻辑：

```text
if something exists => resume
```

---

# 12. `.part` 文件规范

M1 已经使用：

```text
filename.part
```

M2 必须正式定义其生命周期。

例如：

```text
download.mp4.part
        ↓
download complete
        ↓
checksum verify
        ↓
flush / sync
        ↓
atomic finalization
        ↓
download.mp4
```

在任务未完成之前：

```text
final target
```

不应该伪装成完整文件。

---

# 13. `.part` 文件命名

不要只依赖最终文件名。

例如多个任务可能：

```text
movie.mp4
movie.mp4
```

必须有稳定 task identity。

可以：

```text
task_id
```

参与内部路径映射。

例如：

```text
<task-id>.part
```

或者：

```text
temporary/<task-id>/<filename>.part
```

最终位置再映射到用户指定目标。

不要让用户文件名变化导致恢复任务无法定位。

---

# 14. Task ID

必须使用稳定、唯一的：

```text
UUID
```

或当前 M0 已选定的等价 ID。

Task ID：

```text
never changes
```

即：

```text
暂停
恢复
重启
重试
完成
```

都不能创建新 Task ID。

禁止：

```text
resume => new task
retry => new task
restart app => new task
```

---

# 15. SQLite Schema

M2 必须建立正式 migration。

例如：

```text
0001_initial
0002_task_persistence
0003_download_resume
```

实际名称根据现有 migrations 调整。

不要删除旧 migration 重写历史。

---

# 16. SQLite Schema 原则

Task 表必须能表达：

```text
task identity
source
destination
state
progress
validator
timestamps
error
recovery metadata
```

必须考虑：

```text
NOT NULL
DEFAULT
CHECK
UNIQUE
INDEX
foreign key
```

但不要过度数据库化。

真正的业务状态转换仍然由 domain/application logic 控制。

---

# 17. SQLite Migration 要求

必须测试：

```text
empty database
existing database
upgrade from M0
upgrade from M1
reopen after migration
migration repeated
migration interruption
invalid schema
```

Migration 必须是：

```text
deterministic
idempotent
versioned
transactional where appropriate
```

禁止：

```text
程序启动时偷偷 CREATE TABLE IF NOT EXISTS 到处修数据库
```

业务代码不应该散落 migration。

---

# 18. Repository Boundary

继续坚持：

```text
Application
    ↓
TaskRepository
    ↓
SQLite
```

不要让：

```text
DownloadEngine
```

直接操作 SQL。

例如禁止：

```rust
download_engine.rs

sqlx::query(...)
```

或对应数据库 API。

应当：

```text
DownloadEngine
    ↓
TaskPersistence / Repository abstraction
```

---

# 19. Repository 最小接口

根据现有架构设计，例如：

```text
create_task
get_task
list_tasks
update_task
update_progress
update_state
update_resume_metadata
update_error
delete_task
```

但不要机械拆成十几个接口。

重点是：

> repository 负责 persistence，不负责业务决策。

例如：

```text
should_resume?
```

不应该由 SQLite repository 判断。

应该：

```text
RecoveryService
```

决定。

---

# 20. Task Write Policy

M2 最重要的问题之一：

> **多久写一次数据库？**

绝对禁止：

```text
every downloaded chunk => SQLite UPDATE
```

这会导致：

```text
大量数据库写入
WAL 膨胀
IO 抖动
下载性能下降
数据库锁竞争
```

应使用 checkpoint 策略。

例如：

```text
time-based
+
byte-based
+
state transition
```

例如：

```text
每 250ms～1000ms
或每 N MB
取较先者
```

具体阈值必须通过实际测试决定。

---

# 21. Checkpoint 内容

每次 checkpoint 至少包含：

```text
task state
downloaded bytes
total bytes
updated_at
validator metadata
retry information if changed
```

不要保存：

```text
每个 packet
每个 chunk 的细节
每个 TCP 状态
```

因为这些都是 Runtime State。

---

# 22. Checkpoint 一致性

这是 M2 的核心。

必须思考：

```text
先写文件
还是先写数据库？
```

正确目标不是追求“两个动作绝对原子”，因为：

```text
Filesystem + SQLite
```

通常不是一个原子事务。

目标应该是：

> **即使崩溃，也最多回退一小段可接受的 checkpoint，而绝不能继续到数据库声明之外的数据。**

因此设计必须满足：

```text
database progress
≤
physically durable file progress
```

或者采用更严格的等价模型。

不能出现：

```text
DB says 100 MB
physical durable file only 80 MB
```

这种恢复时可能跳过数据的状态。

---

# 23. 推荐的 Checkpoint Ordering

具体实现根据 Rust 文件 IO 模型调整，但必须保证类似：

```text
1. write bytes to .part
2. flush buffered writer
3. ensure required durability boundary
4. record actual file position
5. update SQLite checkpoint
```

不要：

```text
1. SQLite downloaded_bytes += chunk
2. write chunk
```

然后进程崩溃。

否则：

```text
DB > file
```

会产生错误恢复位置。

---

# 24. Checkpoint 不等于完整事务

必须明确记录：

```text
checkpoint is recovery hint
```

而不是：

```text
checkpoint = proof that task is complete
```

最终完成必须独立经过：

```text
all bytes received
→ file flushed
→ validator/checksum verified
→ atomic finalize
→ DB state = Completed
```

---

# 25. 崩溃一致性模型

至少模拟以下：

### Crash A

```text
HTTP received
↓
write .part
↓
process crashes
↓
DB remains old checkpoint
```

恢复时：

```text
DB <= .part
```

必须能够安全回退到：

```text
actual .part size
```

或者截断到已确认 checkpoint，取决于最终设计。

---

### Crash B

```text
write .part
↓
flush
↓
DB checkpoint
↓
crash
```

应该恢复到新的 checkpoint。

---

### Crash C

```text
all bytes written
↓
process crashes before Completed
```

启动时不能从头下载。

应该发现：

```text
.part contains complete resource
```

然后：

```text
verify
→ finalize
→ Completed
```

---

### Crash D

```text
DB says completed
but final file missing/corrupt
```

不能盲目：

```text
Completed
```

必须重新校验 filesystem reality。

---

# 26. Resume 必须重新验证

应用重启后：

```text
不要相信旧 Runtime State。
```

必须重新建立 HTTP 请求。

例如：

```text
GET /resource
Range: bytes=<actual_resume_position>-
If-Range: <ETag or Last-Modified>
```

如果 M1 已实现的 validator 机制可复用，必须直接复用。

禁止复制一套新的 resume 逻辑。

---

# 27. Remote Resource Changed

重点测试：

```text
old .part
+
old ETag
+
server resource changed
```

不能继续：

```text
append new bytes
```

导致：

```text
old prefix + new suffix
```

形成伪完整文件。

必须定义明确行为：

```text
resume safely
restart safely
or require user action
```

推荐：

```text
validator mismatch
→ discard/restart partial content
```

但必须结合 M1 实际能力决定。

---

# 28. Range Resume 和 Persistence 的边界

M2 不重新设计 Range。

复用 M1：

```text
Range
Content-Range
If-Range
ETag
Last-Modified
206
200
416
```

M2 做的是：

```text
让这些信息在重启后依然存在
```

即：

```text
M1:
process lifetime resume

M2:
process lifetime + application restart resume
```

---

# 29. Pause 语义

M2 必须明确：

```text
Pause
```

到底是什么意思。

推荐：

```text
Pause
→ stop network activity
→ preserve .part
→ persist task state
→ keep resume metadata
```

必须做到：

```text
Pause ≠ Cancel
Pause ≠ Delete
```

---

# 30. Cancel 语义

推荐：

```text
Cancel
→ stop download
→ persist Cancelled
→ cleanup temporary artifact according to documented policy
```

但必须统一。

不要出现：

```text
UI says Cancelled
.part still silently remains
next launch automatically resumes it
```

除非这是明确设计。

如果保留临时文件用于“重新开始”，必须有明确生命周期。

---

# 31. Application Exit

处理：

```text
user closes application
```

必须尽可能执行：

```text
stop active downloads
checkpoint
persist state
flush
shutdown runtime
```

但绝不能只依赖正常退出。

因为：

```text
crash
taskkill
power loss
```

都不会执行 graceful shutdown。

因此：

> graceful shutdown 是优化，不是恢复正确性的前提。

---

# 32. Startup Recovery 必须幂等

假设：

```text
Tide 启动
→ recovery
→ 第二次 recovery
```

结果必须相同。

例如：

```text
Task A already repaired
Task B already finalized
Task C still resumable
```

再次扫描不能：

```text
重复 finalize
重复创建 task
重复下载
重复写 metadata
```

必须具有：

```text
idempotent recovery
```

---

# 33. Finalization Recovery

特别处理：

```text
.part
final file
```

同时存在。

例如：

```text
movie.mp4.part
movie.mp4
```

必须定义策略：

```text
Which one is authoritative?
Are they from same task?
Is final file already complete?
Is .part stale?
```

绝对不能简单：

```text
if final exists => Completed
```

因为：

```text
final file may be older unrelated file
```

---

# 34. Target Collision

考虑：

```text
user downloads:
A/file.mp4

existing:
A/file.mp4
```

必须定义：

```text
overwrite
skip
rename
replace
```

M2 至少建立稳定策略。

不要让：

```text
DownloadEngine
```

随机覆盖用户文件。

推荐先支持：

```text
explicit overwrite policy
```

并持久化这个策略。

---

# 35. Path Persistence

不要只保存：

```text
filename
```

也不要只保存：

```text
absolute path
```

必须明确保存：

```text
destination root
+
resolved filename
```

并在恢复时重新验证：

```text
path exists
path accessible
path still inside allowed destination
```

特别考虑 Windows：

```text
drive removed
folder deleted
permission changed
network drive unavailable
junction/reparse point
UNC path
invalid path
```

M2 不需要做 NAS 功能，但必须避免恢复逻辑对路径安全视而不见。

---

# 36. Task Recovery 与 Security

恢复时不能绕过 M1 安全检查。

例如数据库里保存：

```text
http://127.0.0.1/...
```

或者：

```text
file://...
```

不能因为“这是以前创建的任务”就直接允许。

恢复请求必须重新经过：

```text
URL validation
redirect validation
filesystem validation
```

数据库不是安全边界。

---

# 37. URL 与 Source Identity

任务必须能够明确知道：

```text
source identity
```

至少保存：

```text
scheme
normalized URL
```

如果 M1 已定义 canonicalization，继续复用。

注意：

```text
display URL
```

和：

```text
request URL
```

不一定完全相同。

不要为了日志/展示而丢失真正恢复所需的信息。

---

# 38. Sensitive Data

数据库中可能包含：

```text
URL
filename
download history
```

因此：

```text
logs != database
```

日志中不要直接输出：

```text
full URL
query secrets
authorization token
signed URL
magnet
peer information
```

尤其：

```text
Authorization
Cookie
token
signature
AWS-like query credentials
```

不得写入普通日志。

---

# 39. Database Corruption

至少考虑：

```text
database unavailable
database locked
migration failed
record malformed
record references missing data
```

要求：

```text
no panic
structured error
clear recovery behavior
```

不要：

```text
unwrap()
expect()
panic()
```

处理正常业务异常。

---

# 40. Task Versioning

建议为 Task Persistence 增加：

```text
schema version
```

或者：

```text
record version
```

用于未来 M3/M4 演进。

例如：

```text
TaskPersistenceV1
```

但不要现在就设计：

```text
10 代未来兼容系统
```

只保证：

```text
当前 schema 可以可靠升级
```

---

# 41. 不要把 Runtime Object 直接序列化进 SQLite

禁止类似：

```text
serialize DownloadEngine
serialize HttpClient
serialize CancellationToken
serialize TCP state
serialize async task
```

数据库只保存：

```text
persistent intent
persistent metadata
recovery information
```

Runtime 必须重新构建。

---

# 42. Runtime Reconstruction

应用启动后：

```text
Persistent Task
        ↓
Task DTO / Domain
        ↓
Recovery Decision
        ↓
DownloadRuntime
        ↓
DownloadEngine
```

而不是：

```text
deserialize old runtime object
```

---

# 43. IPC 设计

M2 扩展 M1 IPC。

至少具备：

```text
list_tasks
get_task
create_download_task
pause_task
resume_task
cancel_task
retry_task
delete_task
recover_tasks
```

实际名称按项目风格调整。

注意：

```text
pause_task
resume_task
cancel_task
```

必须由 Rust backend 验证状态转换。

前端不能自己决定：

```text
Running → Completed
```

---

# 44. Event Model

后台下载状态必须通过：

```text
Tauri Events
```

或者当前已经建立的 equivalent event channel 通知 frontend。

例如：

```text
task.created
task.updated
task.progress
task.state_changed
task.error
task.completed
```

但不要设计过多事件类型。

---

# 45. UI 不持有真相

Frontend store 只是：

```text
projection / cache
```

SQLite + Rust domain/application 才是 authoritative state。

因此：

```text
UI reload
```

之后：

```text
IPC list_tasks
```

应恢复正确状态。

浏览器刷新式重新载入不能让任务消失。

---

# 46. 启动 UI 行为

Tide 启动后，UI 应该能够直接看到：

```text
之前存在的任务
```

例如：

```text
movie.mp4
Paused
1.8 GB / 5.2 GB
Resume available
```

而不是：

```text
No tasks
```

直到用户重新添加。

---

# 47. 恢复中的 Task UI

可以显示：

```text
Recovering...
```

但这个状态最好属于：

```text
transient runtime presentation
```

而不是强行进入永久 TaskState。

例如：

```text
Persisted:
Paused / Ready

Runtime:
Recovering
```

这样状态机不会膨胀。

---

# 48. Progress Persistence

必须区分：

```text
persistent progress
```

和：

```text
live progress
```

Live progress 可以：

```text
100ms / 250ms
```

更新。

Persistent checkpoint 可以：

```text
250ms / 500ms / 1s
```

或 byte threshold。

二者不能绑定为：

```text
every UI event = database write
```

---

# 49. Speed 不需要持久化

默认不要保存：

```text
current_speed
instant_speed
current_eta
```

因为应用重启后没有意义。

可以保存：

```text
historical metrics
```

但 M2 没必要。

启动后：

```text
speed = recalculated
ETA = recalculated
```

没有真实数据时：

```text
ETA = unknown
```

禁止伪造。

---

# 50. Retry State

需要持久化：

```text
retry_count
last_error
```

但不要持久化：

```text
sleep timer
exponential backoff runtime timer
```

重启后应该重新建立 runtime retry policy。

例如：

```text
retry_count = 3
```

有意义。

而：

```text
sleep_until = 12:01:38.321
```

通常没有意义。

---

# 51. Failed Task

失败任务必须保留：

```text
Task ID
source
destination
progress
last error
```

用户可以：

```text
Retry
Delete
```

而不是：

```text
failed → automatically disappear
```

---

# 52. Retry Semantics

必须明确：

```text
Retry
```

不是：

```text
create new task
```

而是：

```text
same Task ID
+
new Runtime
+
resume if safe
```

除非明确判断需要：

```text
restart from zero
```

也仍然应该：

```text
same Task ID
```

---

# 53. Delete Semantics

删除 Task 时必须明确：

```text
DB record
.part file
final file
```

之间分别发生什么。

特别是：

```text
Delete Task
```

是否删除：

```text
completed final file?
```

推荐：

```text
Delete Task
≠
Delete downloaded user file
```

即：

```text
Task metadata deleted
final user file preserved
```

而 `.part` 根据 task lifecycle 清理。

必须形成明确文档。

---

# 54. Crash Recovery Test Harness

M2 最重要的测试不是普通：

```text
download succeeds
```

而是：

```text
download crashes at controlled points
```

实现 test-only failure injection。

例如：

```text
AfterPartWrite
AfterFlush
AfterCheckpoint
BeforeCheckpoint
BeforeFinalize
AfterFinalizeBeforeDBUpdate
```

这些可以：

```text
debug/test builds only
```

禁止 production 默认开启。

---

# 55. Crash Injection

模拟：

```text
panic
process abort
forced task termination
```

至少覆盖：

```text
10%
25%
50%
75%
90%
100%
```

附近的恢复点。

不要仅测试：

```text
下载 50% 时崩溃
```

而应该测试：

```text
关键一致性边界
```

---

# 56. Recovery Invariants

建立明确 invariants。

至少：

```text
Invariant 1:
persisted downloaded_bytes must never cause resume beyond physically valid data.

Invariant 2:
Completed task must have a validated final artifact.

Invariant 3:
Cancelled task must not silently auto-resume.

Invariant 4:
Retry must preserve Task ID.

Invariant 5:
Recovery must be idempotent.

Invariant 6:
No task may silently lose its source/destination.

Invariant 7:
Restart must not corrupt existing partial bytes.

Invariant 8:
Resource validator must survive restart when needed for safe resume.

Invariant 9:
UI state must be reconstructable from backend persistence.

Invariant 10:
Normal crash must not require recreating the task.
```

把这些 invariants 写入：

```text
tests
+
docs
```

---

# 57. Filesystem Reconciliation

建议独立模块，例如：

```text
core/recovery/
    mod.rs
    service.rs
    decision.rs
    filesystem.rs
    reconcile.rs
```

具体位置根据现有 M0 架构调整。

Recovery 逻辑尽量做到：

```text
input:
TaskMetadata
+
FilesystemMetadata
+
ResourceMetadata

output:
RecoveryDecision
```

使它尽可能成为：

```text
pure / deterministic logic
```

这样非常适合测试。

---

# 58. Reconciliation Example

例如：

```text
DB:
downloaded = 40 MB

.part:
42 MB
```

不能简单把：

```text
downloaded = 42 MB
```

然后继续。

必须判断：

```text
42 MB 是否实际可恢复？
42 MB 是否在正确 resource 上？
validator 是否一致？
```

如果 `.part` 的额外数据没有可靠持久化证明，可以：

```text
truncate to safe checkpoint
```

而不是盲信文件长度。

具体取舍必须在报告中写明：

```text
FACT
TRADE-OFF
DECISION
```

---

# 59. 强烈要求：设计 Safe Resume Point

不要把：

```text
file_size
```

直接等价成：

```text
safe_resume_offset
```

最好明确：

```text
SafeResumePoint
```

概念。

它表示：

> 从这个位置继续请求，不会跳过尚未可靠持久化的数据。

可以来自：

```text
durably flushed file offset
```

或者：

```text
verified checkpoint
```

最终模型以实现为准。

---

# 60. Complete File Detection

当：

```text
.part size == Content-Length
```

不能立即：

```text
Completed
```

必须经过：

```text
complete-size check
→ content integrity checks if configured
→ flush
→ checksum if required
→ finalize
→ DB state transition
```

---

# 61. Zero-Length Resource

测试：

```text
Content-Length: 0
```

必须能正确：

```text
download
→ finalize
→ Completed
```

不要进入：

```text
divide by zero
NaN %
infinite ETA
```

---

# 62. Unknown-Length Resource

当：

```text
Content-Length = unknown
```

SQLite 必须允许：

```text
total_bytes = NULL
```

不能写：

```text
0
```

冒充真实大小。

恢复时：

```text
downloaded_bytes = actual known bytes
total_bytes = unknown
```

最终响应结束时才知道完成。

---

# 63. Percent Persistence

数据库可以存：

```text
downloaded_bytes
total_bytes
```

而不要保存：

```text
percent = 37.24
```

百分比始终由：

```text
downloaded / total
```

计算。

这样不会出现：

```text
percent stale
```

---

# 64. Time Persistence

统一：

```text
UTC
```

存数据库。

Frontend：

```text
local timezone
```

展示。

至少保证：

```text
created_at
updated_at
started_at
completed_at
paused_at
```

语义一致。

---

# 65. Schema/Data Integrity

对关键字段增加合理约束，例如：

```text
downloaded_bytes >= 0
total_bytes >= 0 when present
created_at <= updated_at
```

不要为了“数据库严格”加入难以维护的大量 SQL CHECK。

Domain 层仍然必须负责复杂状态合法性。

---

# 66. State Transition Persistence

例如：

```text
Ready → Running
Running → Paused
Paused → Running
Running → Failed
Failed → Ready
Ready → Cancelled
...
```

必须有统一入口。

不要让：

```text
UI
DownloadEngine
RecoveryService
RetryService
```

各自直接：

```text
task.state = ...
```

建议：

```text
Task::transition(...)
```

或已有 M0 状态机入口。

---

# 67. Recovery 不应该绕过 State Machine

例如恢复：

```text
Running(persisted)
```

不能直接：

```text
Completed
```

而应该执行合法恢复路径。

例如：

```text
Running(interrupted)
→ Recovery
→ Ready
→ Running
```

或者：

```text
Running(interrupted)
→ Recovery
→ Finalizing
```

但如果 M0 状态机没有 Finalizing，不要为了这一点就疯狂扩展状态。

应使用：

```text
RecoveryDecision
```

等 transient application logic。

---

# 68. Startup Recovery 与 User Intent

非常重要：

```text
用户主动 Pause
```

不能因为：

```text
应用启动
```

自动变成：

```text
Running
```

例如：

```text
Before shutdown:
Paused
```

启动后：

```text
Paused
```

而不是：

```text
Auto Resume
```

同理：

```text
Failed
```

默认：

```text
Failed
```

不能无限自动 retry。

---

# 69. Auto Resume Policy

M2 可以支持：

```text
recover task
```

但建议区分：

```text
recoverable
```

和：

```text
auto-resume
```

至少默认：

```text
Paused → remain Paused
Interrupted Running → recover to Ready / resumable
```

是否：

```text
startup immediately resume
```

暂时不要在 M2 做复杂设置系统。

---

# 70. App Crash vs Download Failure

必须区分：

```text
network failure
```

和：

```text
process crash
```

例如：

```text
HTTP 503
```

属于：

```text
Download Failure
```

而：

```text
Tide process terminated
```

属于：

```text
Interrupted Runtime
```

两者恢复路径可以不同。

---

# 71. Error Model

M2 增加结构化 error classification：

```text
Network
Http
Filesystem
Persistence
Validation
Recovery
Checksum
Cancelled
Internal
```

例如：

```text
RecoveryError::DatabaseUnavailable
RecoveryError::MissingArtifact
RecoveryError::ValidatorMismatch
```

不要把错误压成：

```text
"Something went wrong"
```

---

# 72. Error Persistence

数据库可以存：

```text
error_code
error_summary
```

但不建议无限保存：

```text
full Rust debug trace
```

尤其不要把：

```text
credentials
URLs with secrets
paths
network internals
```

全部写进去。

---

# 73. Download Metadata Snapshot

启动恢复时不要依赖：

```text
remote server still returns same metadata
```

先利用数据库保存：

```text
expected size
content type
ETag
Last-Modified
source URL
```

再决定是否需要 HTTP 验证。

---

# 74. HTTP Client Lifecycle

M2 不需要持久化：

```text
reqwest::Client
TCP connection
HTTP response object
stream state
```

它们启动后重新创建。

---

# 75. Persistence Dependency Injection

继续 M0 原则：

```text
AppState
 ├─ TaskRepository
 ├─ DownloadService
 └─ RecoveryService
```

不要：

```text
static mut DATABASE
global connection
lazy global repository
```

除非当前 Tauri runtime 的确有明确且合理的单例 ownership。

核心原则：

> 生命周期由 AppState 管理，而不是到处偷偷获取数据库连接。

---

# 76. SQLite Connection / WAL

如果当前项目使用 SQLite，认真检查：

```text
journal mode
WAL mode
busy timeout
foreign key enforcement
transaction behavior
```

选择适合 Tide desktop application 的配置。

不要为了“听起来专业”开启一堆 pragma。

每个设置必须知道：

```text
why
benefit
trade-off
```

并记录到：

```text
docs/M2-persistence.md
```

---

# 77. 数据库备份

M2 不实现：

```text
cloud backup
database sync
```

但至少考虑：

```text
database corruption / recovery
```

是否需要：

```text
safe backup copy
```

可以作为低优先级分析项。

不要把数据库备份系统扩展成新产品。

---

# 78. Atomic Task Updates

在涉及多个字段变化时，尽可能使用：

```text
single SQL transaction
```

例如：

```text
state = Paused
downloaded_bytes = 1024MB
updated_at = ...
```

不要：

```text
UPDATE state
UPDATE progress
UPDATE timestamp
```

然后中间崩溃。

---

# 79. Transaction Boundary

数据库事务只能保证：

```text
SQLite 内部原子性
```

不能假装：

```text
SQLite + filesystem
```

是一个事务。

因此必须设计：

```text
filesystem durability
+
database checkpoint
+
reconciliation
```

三者共同保证恢复正确性。

---

# 80. Duplicate Task

考虑：

```text
same URL
same destination
same filename
```

是否允许重复创建？

M2 不需要强制做“去重系统”。

但必须明确：

```text
Task ID uniqueness
```

和：

```text
resource deduplication
```

是两个问题。

默认可以：

```text
允许用户创建多个 Task
```

只要不会覆盖/污染彼此文件。

---

# 81. Concurrent Downloads

M2 可以支持多个 HTTP Task 并行，但：

```text
M2 不实现全局 Scheduler
```

可以使用简单：

```text
runtime task manager
```

管理：

```text
task_id → running handle
```

但不要演化成：

```text
smart scheduler
priority queue
bandwidth allocation
demand model
```

这些留给 M7。

---

# 82. Active Task Registry

运行时可以存在：

```text
HashMap<TaskId, DownloadHandle>
```

但必须：

```text
AppState-owned
thread-safe
bounded
```

并且：

```text
running handles ≠ persistence
```

数据库仍然是 persistent record。

---

# 83. DownloadHandle

M1 已经有的 handle 可以继续。

M2 应明确：

```text
DownloadHandle
```

只保存：

```text
runtime control
```

例如：

```text
pause signal
cancel signal
join handle
```

不要放：

```text
persistent task metadata
```

---

# 84. Startup Sequence

推荐最终流程：

```text
Tide start
   ↓
load config
   ↓
open SQLite
   ↓
migrate
   ↓
initialize repositories
   ↓
scan persisted tasks
   ↓
recover/interrogate interrupted tasks
   ↓
reconcile filesystem
   ↓
persist repaired metadata
   ↓
initialize runtime registry
   ↓
frontend ready
```

frontend 不应成为 recovery 的触发器。

---

# 85. UI 最小范围

M2 UI 不需要漂亮。

但必须真实展示：

```text
Task list

Task name
State
Downloaded / Total
Resume availability
Last error
Updated time
```

操作：

```text
Resume
Pause
Cancel
Retry
Delete
```

启动之后应真实加载数据库里的任务。

---

# 86. Recovery Information

对恢复过的任务，可以展示：

```text
Resumable
Interrupted
Needs retry
Source changed
Destination unavailable
```

不要显示过于技术化的信息：

```text
SQLite reconciliation phase 2
```

这些留给 debug log。

---

# 87. 不允许 Fake State

绝对禁止：

```text
UI 模拟任务
fake progress
fake download speed
fake recovery
setTimeout pretending download
```

必须是：

```text
SQLite
+
real DownloadEngine
+
real filesystem
```

---

# 88. Test HTTP Server

继承 M1 Test Server。

M2 增加能够：

```text
track range requests
simulate disconnect
change ETag
change Last-Modified
return 200 unexpectedly
return 206
return 416
delay response
truncate response
```

这样才能真正测试 restart/resume。

---

# 89. Integration Test Matrix

至少覆盖：

## Basic persistence

```text
create task
persist
close
reopen
list task
```

---

## Pause persistence

```text
start
pause
persist
restart app
task remains paused
resume manually
```

---

## Interrupted download

```text
start
download partial
force process termination
restart
recover
resume
complete
```

---

## Crash before checkpoint

```text
partial write
crash
restart
reconcile
resume safely
```

---

## Crash after checkpoint

```text
write
flush
checkpoint
crash
restart
resume from safe position
```

---

## Complete-before-state-update

```text
write complete file
crash before DB Completed
restart
detect complete artifact
verify
finalize
```

---

## Database progress ahead of file

人工构造：

```text
DB = 100 MB
.part = 90 MB
```

要求：

```text
must not resume from 100 MB
```

---

## File ahead of database

人工构造：

```text
DB = 90 MB
.part = 100 MB
```

要求：

```text
must follow defined SafeResumePoint policy
```

不得随便处理。

---

## Remote changed

```text
download partial
change server content
restart
resume attempt
validator mismatch
safe restart/fail
```

---

## Missing .part

```text
DB says resumable
.part missing
restart
```

必须得到明确：

```text
restart
or
needs user action
```

而不是 panic。

---

## Destination unavailable

```text
folder removed
drive unavailable
permission revoked
```

要求：

```text
structured recovery result
```

---

## Duplicate startup recovery

```text
recover
recover again
```

最终状态必须稳定。

---

# 90. Property-Based / Fuzz-Style Tests

优先测试：

```text
Task state transitions
RecoveryDecision
filesystem reconciliation
byte counts
progress reconstruction
path validation
task serialization
database row mapping
```

尤其：

```text
downloaded_bytes
total_bytes
file_size
```

边界：

```text
0
1
MAX
>2GB
>4GB
u64 boundary
```

---

# 91. Large File Test

必须确保：

```text
u64
```

处理文件大小。

不要：

```text
i32
u32
f32
```

作为核心 byte counter。

百分比可以：

```text
f64
```

但：

```text
byte offset = integer
```

---

# 92. Database Restart Test

至少：

```text
open DB
insert task
commit
close connection
reopen
read task
```

并测试：

```text
process terminated after transaction boundary
```

---

# 93. Migration Regression

所有 M0/M1 测试必须继续通过。

至少：

```bash
cargo check --locked
cargo test --locked
```

并执行 frontend tests/build。

如果已有 lint/format：

```bash
cargo fmt --check
cargo clippy ...
```

根据现有项目标准执行。

---

# 94. Observability

增加结构化 recovery log：

例如：

```text
task_recovery_started
task_recovery_decision
task_recovery_reconciled
task_recovery_resumed
task_recovery_failed
```

每条至少：

```text
task_id
decision
reason_code
```

不要包含：

```text
full URL
auth token
peer IP
full filesystem path
```

---

# 95. Debug-only Diagnostics

开发环境允许：

```text
actual_part_size
db_downloaded_bytes
safe_resume_point
validator
recovery decision
```

生产日志尽可能减少敏感信息。

---

# 96. Recovery Reason Code

不要只存：

```text
"resume failed"
```

至少可区分：

```text
InterruptedRuntime
ArtifactMissing
ArtifactShorterThanCheckpoint
ArtifactLargerThanCheckpoint
ValidatorChanged
DestinationUnavailable
DatabaseRecordInvalid
FinalArtifactDetected
SafeResumeAvailable
RestartRequired
```

名称按照实际代码风格调整。

---

# 97. Documentation

M2 至少新增：

```text
docs/M2-persistence.md
docs/M2-recovery.md
docs/M2-consistency.md
docs/M2-test-matrix.md
docs/M2-report.md
```

如果项目已有 architecture 文档，可以合并，而不是机械增加文件。

---

# 98. persistence.md 必须回答

```text
SQLite 保存什么？
什么不保存？
Task schema 是什么？
migration 怎么运行？
Repository 边界是什么？
checkpoint 频率如何决定？
为什么这样设计？
```

---

# 99. recovery.md 必须回答

```text
启动如何发现中断任务？
哪些状态可以恢复？
如何检查 .part？
如何决定 resume/restart/finalize/fail？
用户主动 Pause 如何处理？
Failed 如何处理？
Recovery 是否幂等？
```

---

# 100. consistency.md 必须回答

最重要：

```text
Filesystem
vs
SQLite
```

如何保证：

```text
DB 不会引导 resume 到不存在的数据之后
```

必须明确：

```text
write ordering
flush/durability
checkpoint ordering
reconciliation
finalization
```

---

# 101. M2 报告必须非常诚实

最终报告分：

```text
FACTS
CONFIRMED
HYPOTHESES
IMPLEMENTED
TESTED
FAILED
LIMITATIONS
RISKS
```

不要写：

```text
robust
production-ready
crash-safe
fully reliable
```

除非测试真正支持这些结论。

---

# 102. 给每个关键问题标记状态

采用项目统一格式：

```text
[RESOLVED]
[CONFIRMED]
[WORSENED]
[OPEN]
```

例如：

```text
[RESOLVED] DB progress can no longer exceed safe persisted file position
[CONFIRMED] Interrupted HTTP task can resume after app restart
[OPEN] Power-loss durability behavior depends on filesystem/storage semantics
```

---

# 103. 风险等级只使用 P0 / P1

## P0

例如：

```text
restart causes corruption
DB can point beyond safe file position
task disappears after restart
resume ignores validator
duplicate task corrupts same file
completed task points to missing artifact
```

---

## P1

例如：

```text
recovery UI slightly confusing
checkpoint frequency needs tuning
slow startup on very large task list
retry metadata incomplete
```

不要滥用 P0。

---

# 104. M2 Performance Measurement

至少观察：

```text
database write frequency
CPU overhead
disk IO overhead
startup recovery time
task list loading time
download throughput before/after persistence
```

尤其比较：

```text
M1 no/low persistence
vs
M2 checkpoint enabled
```

不要只测：

```text
“程序能运行”
```

---

# 105. Task Count Test

至少模拟：

```text
10 tasks
100 tasks
500 tasks
1000 tasks
```

测试：

```text
startup restore
list_tasks
database query time
UI rendering basic behavior
```

如果 1000 task 导致架构需要分页，也只记录：

```text
P1
```

除非成为实际阻塞问题。

不要在 M2 顺便实现复杂虚拟列表系统。

---

# 106. Persistence Serialization

如果 Rust Domain Object 与 SQLite Row / IPC DTO 不同：

明确分层：

```text
TaskDomain
TaskRecord
TaskDto
```

不要：

```text
SQLite row directly sent to frontend
```

也不要：

```text
frontend DTO directly become DB schema
```

---

# 107. DTO Boundary

Frontend 只看到稳定 DTO。

例如：

```text
TaskDto {
    id
    name
    state
    downloaded_bytes
    total_bytes
    resumable
    error
    created_at
    updated_at
}
```

不要暴露：

```text
internal retry timer
database row id
filesystem implementation details
Rust internal enum serialization
```

---

# 108. Commands 必须进行业务验证

例如：

```text
resume_task(id)
```

必须：

```text
load task
validate state
validate artifact
validate source
construct runtime
start
```

而不是：

```text
find handle
call resume
```

因为：

```text
application restarted
```

之后原来的 Runtime Handle 根本不存在。

---

# 109. M2 的真正运行时流程

最终应该接近：

```text
                     ┌──────────────┐
                     │    SQLite    │
                     └──────┬───────┘
                            │
                         Task
                            │
                    ┌───────▼────────┐
                    │    Recovery    │
                    └───────┬────────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
           DB state      .part file     validator
              │             │             │
              └─────────────┼─────────────┘
                            │
                   RecoveryDecision
                            │
                    ┌───────▼────────┐
                    │ DownloadEngine │
                    └───────┬────────┘
                            │
                       HTTP Resume
                            │
                    ┌───────▼────────┐
                    │    .part       │
                    └───────┬────────┘
                            │
                     checkpoint
                            │
                    ┌───────▼────────┐
                    │    SQLite      │
                    └────────────────┘
```

---

# 110. M2 与未来 M3/M4 的接口要求

M2 不实现 Torrent。

但是设计不能把 persistence 写死为：

```text
HTTP URL downloader only
```

未来 Task 可能来自：

```text
HTTP
Torrent
Magnet
```

因此 Task persistence 中应该尽量保存：

```text
Source
Resource identity
```

而不是：

```text
HttpOnlyDownloadState
```

但：

> 不要现在实现 Torrent 的字段垃圾场。

应保持：

```text
Source abstraction
+
HTTP-specific resume metadata
```

分层。

---

# 111. 未来 Resource 边界

M2 可以进一步明确：

```text
Task
  ↓
Source
  ↓
Resource
  ↓
Download state
```

未来 M4：

```text
Resource.read(range)
Resource.availability(range)
```

可以建立在这里。

但是 M2 **不要实现**：

```text
ReadableResource
ProgressiveStream
StreamGateway
```

只保证：

```text
downloaded byte range
```

已经可以被明确描述。

---

# 112. 不要 prematurely abstract

不要因为以后会支持：

```text
Torrent
HTTP
Player
Streaming
```

就现在创建：

```text
GenericUniversalPersistenceEngine
GenericResourceStore
GenericDistributedTaskFramework
```

M2 只抽象：

```text
Task persistence
Recovery
Filesystem reconciliation
HTTP resume metadata
```

真正出现稳定重复模式后再抽象。

---

# 113. M2 最小产品闭环

M2 完成后，用户至少应该能够：

```text
1. 输入 HTTP/HTTPS URL
2. 创建下载任务
3. 开始下载
4. 暂停
5. 关闭 Tide
6. 再次启动 Tide
7. 看到旧任务
8. 继续下载
9. 下载完成
10. 正常得到最终文件
```

以及：

```text
1. 开始下载
2. 强制结束 Tide
3. 再次启动
4. 系统识别中断任务
5. 检查 .part
6. 安全恢复
7. 最终完成
```

---

# 114. M2 验收演示

最终必须准备至少以下真实演示：

### Demo A — Pause / Resume

```text
download
→ pause
→ close app
→ reopen
→ resume
→ complete
```

### Demo B — Crash Recovery

```text
download
→ force terminate
→ reopen
→ recovery
→ resume
→ complete
```

### Demo C — Validator Safety

```text
download partial
→ change remote ETag/content
→ restart
→ resume
→ detect mismatch
→ do NOT corrupt file
```

### Demo D — DB/File Mismatch

人工制造：

```text
DB offset != .part size
```

启动：

```text
reconcile
→ safe decision
→ resume/restart
```

### Demo E — Complete Artifact Recovery

```text
all bytes written
→ crash before Completed state
→ restart
→ detect complete artifact
→ verify
→ finalize
```

---

# 115. M2 Definition of Done

只有满足以下条件才算 M2 完成：

## Persistence

```text
[ ] Task metadata persisted
[ ] Task state persisted
[ ] Download progress persisted
[ ] Validator metadata persisted
[ ] Errors persisted
[ ] Task timestamps persisted
[ ] schema migration exists
[ ] repository boundary stable
```

## Recovery

```text
[ ] interrupted tasks detected
[ ] filesystem reconciled
[ ] .part correctly handled
[ ] startup recovery implemented
[ ] recovery is idempotent
[ ] paused tasks remain paused
[ ] failed tasks remain failed unless user retries
[ ] cancelled tasks do not silently resume
```

## Resume

```text
[ ] HTTP Range resume survives application restart
[ ] validator survives restart
[ ] 200/206/416 handling remains correct
[ ] remote-change protection works
[ ] no append corruption
```

## Crash Consistency

```text
[ ] checkpoint ordering defined
[ ] file durability strategy defined
[ ] DB/file mismatch handled
[ ] forced termination tested
[ ] recovery after incomplete finalization tested
```

## UI

```text
[ ] persisted tasks load on launch
[ ] real state shown
[ ] pause/resume/retry/cancel work
[ ] no fake progress
[ ] no fake recovery
```

## Security

```text
[ ] restored URL revalidated
[ ] destination revalidated
[ ] path containment enforced
[ ] sensitive information excluded from logs
```

## Testing

```text
[ ] unit tests pass
[ ] integration tests pass
[ ] migration tests pass
[ ] crash-injection tests pass
[ ] recovery tests pass
[ ] DB/file mismatch tests pass
[ ] large-file tests pass
```

## Quality

```text
[ ] cargo check --locked passes
[ ] cargo test --locked passes
[ ] existing M0/M1 tests still pass
[ ] no unexplained P0 issues
[ ] no scope creep into M3/M4
```

---

# 116. M2 Final Report 格式

最终严格按照以下结构输出：

```markdown
# M2 Implementation Report

## 1. Executive Summary

## 2. FACTS

## 3. CURRENT M1 STATE

## 4. M2 Changes

## 5. Persistence Architecture

## 6. SQLite Schema / Migrations

## 7. Checkpoint Strategy

## 8. Crash Consistency Model

## 9. Recovery Algorithm

## 10. Resume Semantics

## 11. Filesystem Reconciliation

## 12. State Machine Changes

## 13. IPC / Frontend Changes

## 14. Test Matrix

## 15. Crash Injection Results

## 16. Performance Results

## 17. Security Review

## 18. FACT / HYPOTHESIS / INFERENCE

## 19. RESOLVED / CONFIRMED / OPEN / WORSENED

## 20. P0 / P1 Risks

## 21. Known Limitations

## 22. Changed Files

## 23. M3 Readiness

## 24. M4 Readiness

## 25. Final Definition-of-Done Checklist
```

---

# 117. M2 最终判断标准

最后必须回答下面四个问题：

```text
1. Tide 在正常退出后，能否恢复所有应该恢复的 HTTP 下载任务？

2. Tide 在强制结束后，能否安全识别并恢复 partial download？

3. SQLite 与 .part 文件不一致时，是否有确定、可测试、不会导致数据损坏的策略？

4. 未来 M3 Torrent 与 M4 Progressive Streaming 是否可以复用 Task / Resource / Persistence / Recovery 边界，而无需推翻 M2？
```

---

# 118. 最关键的禁止事项

实施过程中始终遵守：

```text
不要实现 Torrent
不要引入 librqbit
不要实现 Player
不要实现 Streaming Gateway
不要实现 Progressive Streaming
不要实现 Scheduler
不要实现 Smart Scheduler
不要实现 Library
不要实现云同步
不要实现账户
不要实现 Telemetry
不要实现 Steam
不要为了未来需求建立巨大抽象
不要把 Runtime Object 序列化
不要让 UI 成为任务真相
不要让 SQLite 进度超过安全的物理文件进度
不要相信 stale validator
不要把 cancel 当 pause
不要把 retry 当 new task
不要让用户主动暂停的任务自动启动
不要用 fake progress 骗 UI
```

---

# 119. M2 的最终技术目标

完成 M2 后，Tide 的核心应该从：

```text
M1

URL
 ↓
HTTP
 ↓
DownloadEngine
 ↓
.part
 ↓
final file
```

升级为：

```text
                    ┌───────────────┐
                    │    SQLite     │
                    │ Persistent    │
                    │ Task State    │
                    └───────┬───────┘
                            │
                            ▼
                    ┌───────────────┐
                    │   Recovery    │
                    │ Reconciliation│
                    └───────┬───────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
      ┌───────────────┐           ┌───────────────┐
      │ Download Task │           │ .part Artifact│
      └───────┬───────┘           └───────┬───────┘
              │                           │
              └─────────────┬─────────────┘
                            ▼
                    ┌───────────────┐
                    │ Safe Resume   │
                    │ HTTP Range    │
                    └───────┬───────┘
                            ▼
                    ┌───────────────┐
                    │ DownloadEngine│
                    └───────┬───────┘
                            ▼
                    ┌───────────────┐
                    │  Final File   │
                    └───────────────┘
```

这意味着：

> **M1 解决“能下载”。**
>
> **M2 解决“下载任务不会因为程序生命周期而失去连续性”。**
>
> **M3 才解决 Torrent 来源。**
>
> **M4 才解决“边下边播”和 Demand / Range / Availability / Scheduler 的真正闭环。**

---

# 120. 最终执行指令

现在开始实施 M2。

严格遵守：

```text
先审计
→ 输出 FACTS / HYPOTHESES
→ 检查 M1 实际实现
→ 确定最小改动面
→ 实施 Persistence
→ 实施 Recovery
→ 实施 Reconciliation
→ 实施 Restart Resume
→ 实施 Crash Injection
→ 完善测试
→ 完善文档
→ 验证 M2 DoD
→ 输出最终报告
```

不要先写代码再解释设计。

不要因为发现 M1 缺陷而擅自扩展到 M3/M4。

如果为了 M2 必须修复 M1，允许修复，但必须在报告中明确标记：

```text
M1 FIX REQUIRED FOR M2
```

任何超出 M2 的想法只记录为：

```text
Future Work
```

不要实现。

最终必须保证这个闭环真实成立：

```text
HTTP/HTTPS
    ↓
Download Task
    ↓
SQLite Persistence
    ↓
.part Artifact
    ↓
Checkpoint
    ↓
Application Exit / Crash
    ↓
Startup Recovery
    ↓
Filesystem Reconciliation
    ↓
Safe Resume Point
    ↓
HTTP Range Resume
    ↓
Checksum / Finalization
    ↓
Completed Task
```

**只有当这个闭环通过真实测试，M2 才算完成。**