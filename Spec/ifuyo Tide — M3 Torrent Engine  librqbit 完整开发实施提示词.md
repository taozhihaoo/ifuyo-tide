# ifuyo Tide — M3 Torrent Engine / librqbit 完整开发实施提示词

> **阶段：M3**
>
> **目标：将 Tide 的下载后端从“HTTP/HTTPS Download Core”扩展为真正可用的 BitTorrent Download Core。**
>
> **核心原则：不要把 Tide 变成 rqbit 的 UI，也不要自己重写 BitTorrent。Tide 负责产品级 Task / Source / Resource / Persistence / Security / IPC / UX 边界；librqbit 负责 BitTorrent 协议、Peer、Piece、DHT、Tracker、校验、存储和 Torrent 生命周期等底层能力。**
>
> **M3 结束后，Tide 应能够稳定完成：**
>
> `用户提供的 Magnet / .torrent → Torrent Metadata → 文件选择 → librqbit ManagedTorrent → Piece 下载 → 持久化恢复 → 完成校验 → 文件落盘 → Tide Task 状态同步`
>
> **M3 不实现正式 Progressive Streaming / Stream Gateway / Player / Scheduler。**
>
> M4 才正式把 Torrent 的 Availability 接到 `Resource.read(range)` / Demand / Scheduler / Progressive Streaming。

---

# 0. M3 的定位

M1 已经解决：

`HTTP/HTTPS → 下载 → 暂停/继续 → Range Resume → Checkpoint → 崩溃恢复 → Finalize`

M2 已经解决：

`Task → SQLite → .part / 物理文件 → Recovery → Resume`

M3 现在加入第二种真正的下载来源：

`Torrent Source → librqbit → Torrent Storage → Tide Task`

但必须承认：

**Torrent 的物理存储模型与 M1 的 HTTP `.part` 模型并不完全相同。**

因此：

* 不要为了“架构统一”强行把 Torrent 套进 HTTP `.part`。
* 不要自行保存 Torrent piece bitfield。
* 不要自己实现 Piece scheduler。
* 不要自己实现 DHT。
* 不要自己实现 Tracker。
* 不要自己实现 Peer protocol。
* 不要自己实现 Piece hash verification。
* 不要复制 librqbit 内部实现。
* Tide SQLite 保存的是 **产品层任务状态**。
* librqbit 保存/管理的是 **Torrent engine state / fast resume / piece state / storage state**。
* 两者通过稳定的 Bridge / Adapter 做 reconciliation。

M3 的真正资产不是“接上 librqbit”，而是：

> **把第三方 Torrent Engine 驯化成 Tide 自己的 Resource / Task / Persistence / Security 模型。**

---

# 1. 开始工作前：必须先审计，不允许直接改代码

先完整检查当前仓库，不要假设 M0/M1/M2 已经完全正确。

优先检查：

* repository structure
* Cargo.toml
* Cargo.lock
* Tauri 2 配置
* capabilities
* CSP
* current Task model
* current TaskState
* Source / Resource model
* SQLite schema
* migrations
* TaskRepository
* DownloadEngine
* HttpTransport
* FileSink
* M2 Recovery
* app startup / shutdown
* IPC commands
* frontend task store
* event bridge
* tests
* docs
* current dependency versions
* 当前是否已经存在任何 torrent / rqbit / bittorrent 代码

先输出：

```text
FACTS
HYPOTHESES
CURRENT M0 STATE
CURRENT M1 STATE
CURRENT M2 STATE
CURRENT DEPENDENCY STATE
CURRENT RQBIT INTEGRATION STATE
M3 IMPLEMENTATION PLAN
M3 RISKS
M3 OUT-OF-SCOPE
```

然后再开始修改。

禁止：

* 因为“看起来应该这样”就假设 librqbit API
* 使用旧版本网上示例直接复制
* 自动升级所有依赖
* 重构整个项目
* 顺便实现 M4
* 顺便实现完整播放器

---

# 2. 先冻结 librqbit 版本策略

这是 M3 的第一道技术闸门。

检查：

1. 当前 Cargo.lock 中是否已有 librqbit
2. 当前实际版本
3. 精确 git revision / crates.io version
4. edition / MSRV
5. 当前 Rust 版本
6. feature flags
7. transitive dependencies
8. license
9. security advisories
10. Windows 编译情况
11. 当前 API 与官方 release 是否一致

不要直接复制网上任何以下类型的 API：

```rust
Session::new(...)
Torrent::new(...)
ManagedTorrent::...
```

除非已经确认与你当前锁定版本一致。

### 版本决策要求

根据当前项目情况，在以下两个方案中明确选择：

```text
A. 保留当前已验证版本
B. 升级到经过审计的稳定版本
```

如果决定升级：

* 必须说明原因
* 必须记录 breaking changes
* 必须更新 Cargo.lock
* 必须重新跑完整测试
* 必须重新验证 Windows build

如果当前项目 M0.5 使用的是 beta API，而现在存在更合适的 stable release：

**必须认真评估一次，而不是因为“之前已经能跑”就永久冻结 beta。**

但不要为了追新版本而进行无必要升级。

---

# 3. 使用 librqbit Library，不要启动外部 rqbit Server

优先采用：

```text
Tide
  ↓
librqbit
  ↓
BitTorrent engine
```

而不是：

```text
Tide
  ↓
spawn rqbit.exe
  ↓
rqbit HTTP API
  ↓
BitTorrent engine
```

默认禁止：

* 启动外部 rqbit binary
* Tide → rqbit HTTP API
* 暴露 rqbit Web UI
* 暴露 rqbit API 给公网
* 把 rqbit Web UI 当 Tide UI

原因：

* 多一个进程
* 多一个 HTTP server
* 多一个安全边界
* 生命周期难统一
* 日志难统一
* Task ID 映射复杂
* M2 recovery 更复杂
* M4 Progressive Gateway 容易出现两层 HTTP

只有当当前 librqbit API 无法满足某项核心能力时，才允许重新评估其他方案。

官方项目本身明确支持 library 形态，因此优先验证直接集成。

---

# 4. M3 领域架构

推荐形成：

```text
Tide Application
    │
    ├── TaskRepository
    │
    ├── TorrentRepository / TorrentMetadataStore
    │
    ├── TorrentEngine
    │      │
    │      └── LibrqbitAdapter
    │
    ├── RecoveryCoordinator
    │
    └── IPC
```

其中：

```text
TorrentEngine
    ↓
librqbit
```

必须存在独立抽象边界。

不要让整个 Tide 到处出现：

```rust
librqbit::xxx
```

理想结构类似：

```text
commands/
core/
    task/
    source/
    resource/
    torrent/
        mod.rs
        engine.rs
        adapter.rs
        source.rs
        metadata.rs
        file_selection.rs
        availability.rs
        stats.rs
        state.rs
        error.rs
        recovery.rs
    download/
    storage/
persistence/
security/
```

具体目录可根据 M0-M2 现实结构调整。

---

# 5. TorrentEngine 抽象

建立 Tide 自己的 TorrentEngine 边界。

不要让上层依赖 librqbit 类型。

概念上至少支持：

```text
add_source()
inspect_metadata()
get_files()
start()
pause()
resume()
remove()
forget()
select_files()
get_status()
get_stats()
get_file_status()
get_availability()
shutdown()
```

具体 API 名称按照真实代码和 librqbit 当前版本设计。

### 重要原则

Tide 的 API 不应该暴露：

```text
ManagedTorrent
Session
TorrentState
librqbit internal type
```

而应该返回自己的 DTO / domain model：

```text
TorrentInfo
TorrentFile
TorrentStatus
TorrentStats
TorrentProgress
TorrentAvailabilitySnapshot
TorrentError
```

---

# 6. Torrent Source

M3 至少实现：

```text
Magnet
LocalTorrentFile
```

建议内部统一：

```text
TorrentSource
├── Magnet
└── TorrentFile
```

其中：

### Magnet

需要支持：

* `magnet:?`
* xt / info hash
* display name
* trackers
* relevant magnet parameters

但：

**不要自己实现 magnet parser。**

优先使用经过验证的库 / librqbit 自身 parser。

需要做：

* scheme validation
* malformed magnet rejection
* size / parameter limit
* invalid info hash rejection
* structured error

### .torrent

支持：

* 用户选择本地 `.torrent`
* 导入到 Tide

建议不要永久依赖用户原路径。

例如：

```text
%LOCALAPPDATA%\ifuyo\Tide\
    torrent-metadata\
        <task-id>.torrent
```

这样：

```text
用户导入 .torrent
      ↓
Tide 保存稳定副本
      ↓
SQLite 保存 metadata reference
      ↓
重启后仍可恢复
```

原始文件移动 / 删除后，不应破坏 Tide 已经建立的任务。

---

# 7. Magnet Metadata Persistence

这是 M3 的重要恢复问题。

Magnet 初始状态可能：

```text
Magnet
  ↓
等待 metadata
  ↓
Metadata resolved
  ↓
Torrent known
  ↓
Downloading
```

因此不能假设：

```text
Magnet == 完整 Torrent Metadata
```

需要明确设计：

```text
MetadataPending
MetadataResolved
MetadataFailed
```

具体是否进入正式 TaskState，要根据 M0/M2 状态机设计决定。

推荐不要把：

```text
WaitingMetadata
ResolvingMetadata
```

疯狂加入永久 TaskState。

可以使用：

```text
TaskState + TorrentPhase
```

例如：

```text
TaskState = Running

TorrentPhase =
    ResolvingMetadata
    Downloading
    Checking
    Completed
    Error
```

否则 M0 的 TaskState 会逐渐变成 Torrent 专用状态机。

---

# 8. Torrent Metadata Model

metadata 至少需要能够表达：

```text
info_hash
name
total_size
piece_length
file_count
files[]
```

每个 TorrentFile：

```text
file_index
relative_path
size
offset
selected
downloaded_bytes
completed
```

注意：

### 不要把 Piece 与 File 强行视为同一单位

一个 Piece 可以跨越多个文件。

因此绝对不要实现成：

```text
file 0 → piece 0
file 1 → piece 1
```

必须保留：

```text
Torrent logical byte space
        ↓
Piece space
        ↓
File space
```

这会直接影响 M4 的：

```text
Availability
Range
Seek
Scheduler Demand
```

---

# 9. Multi-file Torrent 是 M3 的核心测试项

必须完整验证：

```text
Torrent
├── folder/
│   ├── movie.mkv
│   ├── poster.jpg
│   ├── subtitle.srt
│   └── bonus/
│       └── extra.mp4
```

以及：

* nested folders
* Unicode filename
* spaces
* very long path
* zero-byte file
* duplicate filename in different directories
* piece spanning file boundaries
* selected / unselected files

---

# 10. File Selection

M3 正式支持：

```text
全选
选择部分文件
取消部分文件
```

但不要做复杂的“智能媒体筛选”。

先只提供：

```text
Select All
Select None
Select Individual
```

内部使用 librqbit 对 file selection 的能力。

如果当前版本支持类似：

```text
only_files
update_only_files
```

可以映射。

但：

**不要凭旧 API 猜名称。**

先检查真实版本。

---

# 11. Selection Persistence

SQLite 至少保存：

```text
selected files
```

或者等价的稳定表示。

推荐：

```text
task_id
file_index
selected
```

但要考虑 Torrent metadata 改变 / infohash 改变。

因此真正 identity 应基于：

```text
task_id + info_hash + file_index
```

而不是文件名。

---

# 12. Download Directory / Filesystem 安全

Torrent 是外部不可信输入。

绝不能直接：

```rust
output_root.join(torrent_file.path)
```

然后放心写入。

必须验证：

```text
Torrent path
    ↓
normalize
    ↓
reject absolute path
    ↓
reject ..
    ↓
reject invalid components
    ↓
sanitize Windows names
    ↓
canonicalize / containment check
    ↓
final path
```

重点测试：

```text
../foo
..\foo
C:\foo
C:/foo
\\server\share
\\?\...
CON
PRN
AUX
NUL
COM1
LPT1
trailing dot
trailing space
reserved characters
very long component
very deep directory
Unicode normalization edge cases
```

同时考虑：

* symlink
* junction
* reparse point
* Windows device path
* UNC path

不要因为“rqbit 已经做了 path traversal check”就省略 Tide 自己的验证。

**Tide 的安全边界不能依赖第三方实现细节。**

---

# 13. Torrent Root Directory 设计

推荐：

```text
User Download Root
    └── <Torrent Root>
          ├── file A
          ├── file B
          └── ...
```

需要明确：

### 单文件 torrent

例如：

```text
movie.mkv
```

最终：

```text
Downloads/
    movie.mkv
```

还是：

```text
Downloads/
    TorrentName/
        movie.mkv
```

必须根据产品体验拍板并记录。

建议：

* single-file：允许直接放入目标目录
* multi-file：以 torrent root folder 隔离

但最终以 M3 实测 + Windows UX 为准。

---

# 14. Tide Persistence 与 librqbit Persistence 必须明确分层

这是 M3 最重要的架构决策之一。

不要：

```text
SQLite = Torrent Piece State
```

也不要：

```text
librqbit = Tide Task State
```

正确方向：

```text
SQLite
──────
Tide Task
Tide Source
Tide User Intent
Tide Destination
Tide Selection
Tide Error
Tide Lifecycle

librqbit
────────
Torrent Session
Piece State
Fast Resume
Peer State
Torrent Storage State
Protocol State
```

两者之间：

```text
TorrentTaskRecord
        ↕
TorrentEngineAdapter
        ↕
librqbit
```

---

# 15. M2 Recovery 如何扩展到 Torrent

M2 已经规定：

> SQLite 状态 + 物理文件状态 + runtime state 三者不能互相打架。

M3 新增：

```text
SQLite
+
Filesystem
+
librqbit persistent state
```

因此启动流程扩展为：

```text
App Start
   ↓
Open SQLite
   ↓
Load Tasks
   ↓
Initialize librqbit Session
   ↓
Restore Torrent Engine State
   ↓
Match Task ↔ info_hash
   ↓
Inspect engine state
   ↓
Inspect filesystem
   ↓
Reconcile
   ↓
Persist corrected Tide state
   ↓
Resume according to user intent
```

---

# 16. Torrent Recovery Decision

为 Torrent 单独设计：

```text
TorrentRecoveryDecision
```

至少考虑：

```text
Resume
Pause
Restart Check
Rebuild Engine State
Completed
Failed
Missing Metadata
Missing Storage
Missing Torrent Metadata
Inconsistent
Ignore
```

恢复必须幂等。

例如：

第一次启动：

```text
Resume
```

第二次再次启动：

```text
Resume
```

不能变成：

```text
DuplicateTorrent
DuplicateDownload
DuplicateSession
```

---

# 17. Duplicate Torrent Invariant

M3 不要一开始就支持：

```text
same info_hash
same files
different tasks
```

多个逻辑 Task 同时映射一个 Torrent Engine 实例会显著增加复杂度。

建议第一版硬约束：

```text
同一个 info_hash + 同一个 storage root
只能有一个 active TorrentTask
```

发现重复：

```text
reuse existing task
```

或者：

```text
structured duplicate error
```

不要启动第二份下载。

未来真有需要，再做 shared-resource architecture。

---

# 18. Torrent State Mapping

建立 Tide → Torrent Engine 的有限映射。

例如：

```text
Tide Created
    ↓
Torrent Metadata Pending

Tide Ready
    ↓
Torrent Added / Ready

Tide Running
    ↓
Torrent Downloading

Tide Paused
    ↓
Torrent Paused

Tide Completed
    ↓
Torrent Completed

Tide Failed
    ↓
Torrent Error

Tide Cancelled
    ↓
Torrent Removed
```

但是注意：

librqbit 的内部状态不要机械复制进 Tide。

例如：

```text
Checking
Live
Paused
Metadata fetching
Initializing
```

可以是：

```text
TorrentPhase
```

而不是全部变成全局 TaskState。

---

# 19. Pause / Resume

必须验证：

```text
Running
  ↓ Pause
Paused
  ↓ Resume
Running
```

期间：

* engine 真正停止/暂停
* 不继续建立无意义的新下载
* SQLite 更新
* 前端收到 state event

禁止：

```text
UI 显示 Paused
实际上 engine 仍在大量下载
```

测试实际：

* bytes 不再增长
* disk write activity 降低/停止
* peer activity 收敛
* 恢复后继续增长

---

# 20. Cancel / Remove / Forget

明确区分至少两种语义：

```text
Cancel Task
Delete Task
Delete Task + Data
```

不要把：

```text
Delete
```

含糊处理成：

```text
database delete
+
engine remove
+
filesystem delete
```

建议：

### Remove from Tide

```text
停止 engine
从 Tide Task 列表移除
保留数据
```

### Remove + Delete Data

```text
停止 engine
移除 torrent
删除对应下载数据
删除 Tide task
```

整个过程必须有安全边界和失败处理。

---

# 21. Torrent Completed 定义

不能仅仅：

```text
downloaded_bytes >= total_bytes
```

就认为完成。

必须基于 engine 确认：

```text
all required pieces complete
verification passed
storage state valid
```

然后：

```text
Torrent Completed
        ↓
Tide Task Completed
```

不要自己再重新实现 SHA-1/SHA-256 piece verification。

利用 librqbit。

---

# 22. Torrent Hash Verification

M3 必须验证：

```text
正常下载
↓
完整完成
↓
文件内容正确
```

并且测试：

```text
人为破坏部分已下载数据
↓
重新启动
↓
engine 检测
↓
坏 piece 被重新下载
```

必须形成真实集成测试。

这也是判断：

```text
M2 recovery
+
librqbit fast resume
```

是否真的可靠的重要证据。

---

# 23. Magnet 流程

完整验证：

```text
Magnet Input
   ↓
Validate
   ↓
Create Task
   ↓
Resolve Metadata
   ↓
Metadata Available
   ↓
Show File List
   ↓
User Select Files
   ↓
Start Download
```

不要：

```text
用户粘贴 magnet
↓
立刻开始下载
↓
UI 猛猜文件列表
```

正确顺序应该是真实 metadata ready 后再展示准确文件列表。

---

# 24. Torrent URL

M3 可评估：

```text
https://example.com/file.torrent
```

是否作为：

```text
TorrentSource::TorrentUrl
```

但不是 M3 的核心阻塞项。

如果实现成本低，并且能够干净复用 M1 已经验证的 HTTP transport：

可以实现。

否则：

```text
P1 / M3.1
```

明确记录。

绝对不要因为支持 `.torrent URL` 就把 M1 HTTP downloader 重构一遍。

---

# 25. DHT / Tracker / Peer

M3 必须使用 librqbit 的真实 BitTorrent能力。

不要自己做：

* DHT
* tracker
* peer handshake
* peer wire protocol
* piece picker
* choking
* request pipeline
* connection management

Tide 只关心：

```text
Engine
    ↓
Status
    ↓
Availability
    ↓
Task
```

---

# 26. Networking 安全默认值

M3 必须明确：

```text
No UPnP port forwarding by default
No public HTTP API
No mDNS API advertisement
No Prometheus exporter
No proxy
No anonymity claims
```

不要为了“自动加速”偷偷打开：

```text
UPnP
NAT traversal
Port forwarding
```

如果 librqbit 当前版本具有这些能力：

**明确关闭，除非用户未来显式开启。**

不要做“自动帮用户开端口”这种默认行为。

---

# 27. uTP / Experimental Features

如果当前 librqbit release 存在：

```text
experimental uTP
```

不要默认开启。

先：

```text
inspect
build
test
benchmark
```

然后决定是否进入以后阶段。

M3 的目标是：

```text
稳定 Torrent Download Core
```

而不是：

```text
把所有 rqbit feature 一次性打开
```

---

# 28. Upload / Sharing Policy

BitTorrent 与普通 HTTP 最大的产品区别之一：

**下载 Torrent 的过程中可能同时上传数据。**

因此 Tide 必须从架构上预留：

```text
SharingPolicy
```

例如：

```text
DownloadOnly
Balanced
Seed
```

### 默认：

```text
DownloadOnly
```

但这里有一个极其重要的工程要求：

**不要假装“DownloadOnly”存在。**

必须确认当前 librqbit 精确版本是否真正能够做到：

```text
no upload
```

如果能够：

* 通过正式 feature/config 开启
* 编译验证
* 集成测试验证

如果不能：

* 不得在 UI 显示“禁止上传”然后实际仍然上传
* 改成 capability / unavailable
* 或者暂停 M3 该项 DoD 并明确报告

产品文案也不能宣称：

```text
不会上传
绝对不共享
绝对匿名
```

除非有可验证的技术保证。

---

# 29. Peer 信息

M3 可以提供：

```text
peer count
connected peers
download speed
upload speed
```

但不需要做完整：

```text
Peer Table
IP
Country
ASN
Client Fingerprint
```

尤其默认不要在 Tide 日志中写：

```text
peer IP
```

也不要在 telemetry 中发送 peer 信息。

M3 的 UI 只做用户真正需要的：

```text
Downloading
Peers: 8
↓ 4.2 MB/s
↑ 0 MB/s
```

---

# 30. Stats Model

Tide 自己定义：

```text
TorrentStats
```

至少：

```text
downloaded_bytes
uploaded_bytes
download_speed
upload_speed
total_bytes
completed_bytes
progress
peer_count
seeder_count
availability
```

具体字段依据实际 librqbit API 能力。

不要把不存在的数据伪造为 0。

规则：

```text
Unknown = null
NotSupported = null / capability
Actual Zero = 0
```

避免：

```text
UI shows 0 peers
```

实际含义却是：

```text
API unavailable
```

---

# 31. Progress

不要让前端自己计算：

```text
downloaded / total
```

前端只消费后端 DTO。

至少：

```text
downloaded_bytes
total_bytes
progress_percent
speed
```

但：

```text
total_bytes
```

必须来自真实 Torrent metadata。

不要在 metadata 未解析时伪造：

```text
0%
```

更合理：

```text
Resolving metadata...
```

---

# 32. IPC

增加 Torrent 专用 IPC。

例如：

```text
inspect_torrent_source
create_torrent_task
list_torrent_files
set_torrent_file_selection
start_torrent
pause_torrent
resume_torrent
remove_torrent
delete_torrent_data
get_torrent_status
```

具体名称根据当前项目保持一致。

必须通过：

```text
DTO
```

跨 IPC。

禁止直接序列化 librqbit structs。

---

# 33. Event Model

Torrent stats 不应通过每个 Piece completion 都直接发送 IPC。

采用：

```text
Engine event
   ↓
Aggregator
   ↓
throttle
   ↓
Task event
   ↓
Frontend
```

建议：

```text
100–500ms
```

作为初始观察区间，根据实际性能调整。

核心事件：

```text
task.created
task.updated
task.state_changed
torrent.metadata_ready
torrent.progress
torrent.stats
torrent.completed
torrent.error
```

不要制造：

```text
piece.completed
piece.requested
peer.packet
peer.connected
```

这种 UI 无法消费的高频事件。

---

# 34. TorrentEngine 生命周期

Tide 进程内部原则：

```text
App
  ↓
TorrentEngineManager
  ↓
single librqbit Session
  ↓
multiple ManagedTorrent
```

避免：

```text
每个 task 一个 Session
```

除非实际 API/资源隔离强制要求。

Session 生命周期：

```text
App Start
   ↓
Initialize Session
   ↓
Restore Torrent Tasks
   ↓
Run
   ↓
Pause/Resume/Add/Remove
   ↓
App Shutdown
   ↓
Graceful Session Shutdown
```

---

# 35. App Shutdown

退出程序时：

```text
Frontend close
   ↓
Tauri shutdown request
   ↓
stop accepting new Torrent operations
   ↓
persist Tide state
   ↓
shutdown librqbit session
   ↓
flush required state
   ↓
process exit
```

但：

**正确性不能依赖 graceful shutdown。**

仍然必须通过 M2 的：

```text
crash recovery
```

验证异常退出。

---

# 36. Torrent Storage Location

不要将 librqbit 内部状态与用户下载文件混在一起。

优先区分：

```text
%LOCALAPPDATA%\ifuyo\Tide\
    app.db
    torrent-metadata\
    torrent-engine\
    logs\
```

和：

```text
D:\Downloads\
    user files
```

如果 librqbit 当前版本要求特定 state directory：

* 使用稳定 app-data 路径
* 不放到安装目录
* 不放到 Program Files
* 不依赖当前 working directory

---

# 37. SQLite Schema 扩展

适配当前 M2 schema。

可能需要增加：

```text
torrent_tasks
```

或者：

```text
tasks
sources
torrent_metadata
torrent_files
```

推荐避免所有内容塞进一张：

```text
tasks
```

至少保持职责清晰。

概念结构：

```text
tasks
sources
torrent_metadata
torrent_files
task_file_selection
```

不要过早做：

```text
torrent_pieces
torrent_peers
torrent_bitfield
torrent_piece_history
```

这些应该留给 librqbit。

---

# 38. Info Hash Identity

Torrent 的稳定 identity：

```text
info_hash
```

是重要概念。

但不要让：

```text
info_hash
```

直接替代：

```text
task_id
```

正确关系：

```text
task_id
    ↓
user task identity

info_hash
    ↓
torrent resource identity
```

未来 M4：

```text
Resource
    ↓
Torrent availability
```

会需要这两层。

---

# 39. Resource 边界

M3 需要为未来 M4 预留：

```text
Torrent Resource
```

概念：

```text
Resource
    id
    source
    size
    content_type
    availability
```

但：

**M3 不实现完整 `ReadableResource`。**

只验证：

```text
Torrent file
    ↓
known byte range
    ↓
piece availability information
```

M4 才正式实现：

```text
read(range)
availability(range)
```

---

# 40. Availability Snapshot

为了 M4 做准备，M3 建议提供有限的：

```text
AvailabilitySnapshot
```

至少能够表达：

```text
piece_size
piece_count
available_piece_count
available ranges / bitfield
```

但：

不要把它长期持久化进 SQLite。

不要自己维护 canonical bitfield。

正确：

```text
librqbit = source of truth
Tide = adapter snapshot
```

---

# 41. File Availability

进一步验证：

```text
File A
   ↓
which pieces are available?

File B
   ↓
which pieces are available?
```

特别测试：

```text
Piece 3:
File A tail
+
File B head
```

因为 M4 的：

```text
seek(file offset)
```

必须正确转换到：

```text
Torrent piece availability
```

---

# 42. M3 暂不实现 Progressive Streaming

即使 librqbit 已经支持：

```text
stream file
Range
seek
```

M3 仍然：

**不实现正式 Player / Stream Gateway。**

可以做内部验证：

```text
Availability
```

但不把它做成：

```text
/video/<task>/<file>
```

也不要接：

```html
<video>
```

不要提前实现：

```text
WebView2
Range Gateway
Demand
Scheduler
```

这些归 M4。

---

# 43. Torrent → HTTP 的边界

M3 结束时应该存在：

```text
TorrentEngine
```

M4 再建立：

```text
TorrentEngine
    ↓
ReadableResource
    ↓
Progressive Stream Gateway
    ↓
WebView2
```

而不是：

```text
librqbit HTTP server
    ↓
Tide Player
```

Tide 应最终自己控制：

```text
Resource
Range
Availability
Demand
```

这样未来 HTTP 与 Torrent 才真正统一：

```text
HTTP Resource
Torrent Resource
Local File Resource
        ↓
ReadableResource
        ↓
Stream Gateway
```

---

# 44. Test Strategy：不要依赖公共 Torrent

M3 的 CI / 自动测试：

**不要依赖公共 DHT。**

不要：

```text
CI
 ↓
public magnet
 ↓
public swarm
```

因为不可重复、速度不可控、可能失效。

应建立：

```text
Test Seeder
      ↓
Local Torrent
      ↓
Local Peer
      ↓
Tide TorrentEngine
      ↓
Downloaded File
```

也就是：

```text
in-process / local-process deterministic BitTorrent fixture
```

尽量用：

```text
127.0.0.1
```

构造合法测试 swarm。

---

# 45. Test Fixture

创建小型 fixture，例如：

```text
torrent-fixture/
    video.bin
    text.txt
    nested/
        data.dat
```

大小至少覆盖：

```text
tiny
multiple pieces
cross-file piece
multi-file
unicode
zero-byte
```

测试数据由测试代码自行生成。

不要把大二进制测试文件直接塞进 git。

---

# 46. Torrent Integration Tests

至少：

### T01

```text
.torrent
→ metadata
→ files
```

### T02

```text
.torrent
→ full download
→ correct bytes
```

### T03

```text
multi-file
→ selected files only
```

### T04

```text
pause
→ bytes stop
→ resume
→ bytes continue
```

### T05

```text
process crash
→ restart
→ resume
```

### T06

```text
partial
→ corruption injected
→ restart
→ bad piece re-download
```

### T07

```text
duplicate info_hash
→ no duplicate engine download
```

### T08

```text
Unicode paths
→ correct filesystem output
```

### T09

```text
malicious torrent paths
→ rejected
```

### T10

```text
torrent completed
→ Task Completed
```

---

# 47. Magnet Tests

至少分开：

### Unit

```text
valid magnet
invalid magnet
missing xt
invalid hash
multiple trackers
unexpected parameters
```

### Integration

如果能稳定构造本地 metadata resolution：

```text
magnet
→ metadata resolved
→ files listed
```

否则把公共 magnet 测试明确标记为：

```text
manual / non-CI
```

不能把不稳定网络测试伪装成 CI 保证。

---

# 48. Recovery Tests

完整覆盖：

```text
SQLite says Running
engine says Running
filesystem valid
```

```text
SQLite says Running
engine missing
filesystem partially downloaded
```

```text
SQLite says Paused
engine missing
```

```text
SQLite says Completed
engine says Completed
```

```text
SQLite says Completed
filesystem damaged
```

```text
SQLite says Running
metadata file missing
```

```text
SQLite says Running
download directory missing
```

```text
same info_hash restored twice
```

---

# 49. Crash Injection

继续沿用 M2 的 crash injection 思路。

关键点增加：

```text
AfterTorrentAdded
AfterMetadataResolved
AfterFileSelectionPersisted
AfterEngineStatePersisted
AfterTideCheckpoint
BeforeTideCheckpoint
AfterCompleted
BeforeCompleted
AfterEngineShutdown
```

测试：

```text
panic / process kill
```

恢复后：

* 不重复下载
* 不丢任务
* 不错误完成
* 不生成第二个 torrent
* 不产生 orphan engine
* 不破坏文件

---

# 50. Network Fault Tests

尽可能模拟：

```text
no peers
peer timeout
peer disconnect
tracker unavailable
DHT unavailable
intermittent connectivity
network temporarily lost
network recovered
```

验证：

```text
Task remains Running / Waiting appropriately
```

而不是：

```text
瞬间 Failed
```

除非错误真的不可恢复。

---

# 51. Retry / Backoff

M1 已有 HTTP retry。

M3 不要简单复制：

```text
HTTP retry logic
```

BitTorrent 的恢复机制不同。

因此 TorrentEngine 主要依赖 librqbit 自身：

```text
peer reconnect
piece retry
tracker retry
DHT behavior
```

Tide 层只处理：

```text
engine-level failure
startup failure
metadata failure
storage failure
fatal configuration failure
```

不要在 Tide 外面再包一层：

```text
while(true) restart torrent
```

造成双重 retry。

---

# 52. Error Boundary

统一映射到：

```text
TorrentError
```

然后映射 Tide：

```text
TideError
```

例如：

```text
InvalidSource
MetadataUnavailable
MetadataTimeout
TorrentNotFound
StorageError
PermissionDenied
PathRejected
EngineInitializationFailed
EngineStateConflict
TorrentVerificationFailed
NetworkUnavailable
Cancelled
Internal
```

但不要把：

```text
完整 librqbit debug error
```

直接返回前端。

---

# 53. Sensitive Logging

默认日志禁止写：

```text
完整 magnet
完整 .torrent path
peer IP
tracker full URL
完整 info_hash（如果无必要）
完整用户下载目录
```

可以记录：

```text
task_id
event type
error category
sanitized source type
torrent short identifier
state
```

例如：

```text
torrent task failed
task_id=...
error=metadata_timeout
```

开发 debug 模式如确需详细网络日志，也必须明确是：

```text
development-only
```

不能默认进入生产日志。

---

# 54. Security / Dependency Audit

M3 加入：

```text
cargo tree
cargo metadata
cargo audit
```

如果项目已经有：

```text
cargo deny
```

继续使用。

必须检查：

* direct dependency license
* transitive dependencies
* known vulnerabilities
* Windows-specific crates
* native dependencies
* build scripts
* binary packaging

尤其是 librqbit 相关依赖。

不要复制 rqbit 源代码到 Tide。

优先：

```text
crate dependency
```

而不是：

```text
fork code into Tide
```

除非明确记录原因、维护责任和 license。

---

# 55. Security Product / Antivirus Observation

由于 BitTorrent client 天生会包含：

```text
network listeners
P2P traffic
DHT
peer communication
file writes
```

M3 应额外记录：

```text
Windows Defender / SmartScreen
installer behavior
unsigned development build behavior
release artifact scanning
network listener visibility
```

这里只做工程观察：

**不要宣称“不会被误报”。**

如果出现：

```text
antivirus alert
SmartScreen
network warning
```

必须记录：

```text
environment
artifact hash
exact warning
reproducibility
```

而不是简单关闭安全软件继续开发。

---

# 56. Windows Resource Management

注意：

```text
socket count
file handles
memory
disk handles
threads
```

测试：

```text
1 torrent
10 torrents
50 torrents
```

至少观察：

```text
CPU
RAM
file handles
network sockets
startup time
shutdown time
```

不要默认支持：

```text
500 / 1000 concurrent torrent
```

没有实测就不要声称。

---

# 57. Performance Baseline

记录：

```text
torrent metadata resolution latency
time to first piece
download throughput
CPU
RAM
disk write throughput
SQLite checkpoint cost
event emission overhead
startup restore time
```

重点比较：

```text
1 torrent
10 torrents
```

以及：

```text
small torrent
large torrent
multi-file torrent
```

不要只测“下载速度”。

M3 更需要证明：

```text
Engine + Tide integration overhead
```

是可接受的。

---

# 58. Frontend：只做真实 Torrent UI

M3 前端最少支持：

```text
Add Torrent
    ├── Magnet
    └── .torrent

Metadata loading

File selection

Download

Pause

Resume

Remove

Delete Data
```

列表显示：

```text
name
state
progress
downloaded / total
download speed
upload speed
peers
selected files
```

错误显示：

```text
真实错误
```

禁止：

```text
fake speed
fake progress
fake peers
fake availability
```

---

# 59. Metadata Preview

用户加入 Magnet 后：

```text
Resolving...
```

然后：

```text
Torrent Name
Total Size
File Tree
Selected
```

必须使用真实 metadata。

不允许：

```text
先显示假的文件名
```

---

# 60. UI 不要做成“高级 BT 客户端”

M3 不是做：

```text
qBittorrent clone
```

暂时不要：

* peer detail
* tracker editor
* DHT routing table
* speed graph
* protocol flags
* port mapping page
* advanced choking controls
* RSS
* search
* category automation
* scheduler rules

这些以后有真实用户需求再加。

---

# 61. M3 不允许加入的功能

以下全部禁止：

```text
资源搜索
资源索引
磁力搜索
资源站导航
Tracker 推荐
盗版内容推荐
匿名下载
VPN
代理
Tor
Cloud Download
Remote Download
账号
云同步
Telemetry
Steam
NAS
UPnP media server
DLNA
Casting
Player
Subtitle
Thumbnail
FFmpeg
libmpv
Progressive Streaming
Stream Gateway
Scheduler
Smart Download Priority
AI recommendation
```

尤其不要因为：

```text
rqbit 已经支持这些
```

就全部接进 Tide。

---

# 62. M3 与 M4 的边界检查

M3 结束时必须能回答：

```text
1. Torrent metadata 能否可靠获取？
2. Magnet 能否可靠进入 metadata？
3. 多文件 Torrent 是否正确？
4. 文件选择是否可靠？
5. Torrent 能否真正下载？
6. Pause/Resume 是否真实？
7. Restart 后是否真实恢复？
8. Piece verification 是否真实？
9. Tide SQLite 与 librqbit state 是否能够 reconciliation？
10. Torrent File Range 如何映射到 Piece？
11. Availability 是否能够被 Tide 读取？
12. 是否可以在 M4 实现 Resource.read(range)？
```

如果第 10/11 项还没有做到：

不要假装 M4 已经准备好了。

---

# 63. M4 接口预留

M3 只需预留：

```rust
TorrentResourceAdapter
```

概念接口：

```text
metadata()
size()
file_info()
availability()
```

暂时可以不实现：

```text
read()
```

M4 再实现：

```text
read(range)
```

最终：

```text
HTTP Resource
        \
         \
          → ReadableResource
         /
Torrent Resource
```

这是 Tide 后续最关键的统一抽象之一。

---

# 64. Source / Resource / Task 三层必须继续分离

不要把：

```text
Torrent
```

同时当：

```text
Source
Resource
Task
```

正确理解：

```text
Source
= Magnet / .torrent

Resource
= Torrent content

Task
= 当前用户的下载行为
```

未来：

```text
Resource
↓
PlaybackSession
```

而：

```text
Task
```

依然是：

```text
Download lifecycle
```

不要因为 M3 就混成一个 giant `TorrentTask` object。

---

# 65. Repository 边界

坚持：

```text
Application
    ↓
TaskRepository
    ↓
SQLite
```

Torrent Engine：

```text
Application
    ↓
TorrentEngine
    ↓
librqbit
```

绝对不要：

```text
librqbit callback
    ↓
SQL query
```

不要让第三方 engine 知道 SQLite。

---

# 66. Do Not Over-Abstraction

M3 允许新增：

```text
TorrentEngine
LibrqbitAdapter
TorrentSource
TorrentMetadata
TorrentFile
TorrentStats
```

但禁止提前建立：

```text
UniversalDownloadEngine
UniversalProtocolAdapterFactory
GenericTransportGraph
PluginArchitecture
ExternalEngineMarketplace
```

除非代码中已经出现实际重复。

保持：

```text
HTTP
Torrent
```

两个清晰实现即可。

---

# 67. Test Matrix

至少建立：

| 分类           | Case                  |
| ------------ | --------------------- |
| Source       | valid magnet          |
| Source       | invalid magnet        |
| Source       | local torrent         |
| Metadata     | resolve success       |
| Metadata     | resolve timeout       |
| Metadata     | malformed torrent     |
| Files        | single file           |
| Files        | multi file            |
| Files        | nested paths          |
| Files        | Unicode               |
| Files        | zero-byte             |
| Selection    | all                   |
| Selection    | partial               |
| Selection    | none                  |
| Download     | full                  |
| Download     | partial               |
| Download     | slow peers            |
| Download     | no peers              |
| Recovery     | process restart       |
| Recovery     | crash                 |
| Recovery     | engine state mismatch |
| Recovery     | missing metadata      |
| Recovery     | missing destination   |
| Verification | corrupted piece       |
| Duplicate    | duplicate info_hash   |
| Security     | traversal             |
| Security     | absolute path         |
| Security     | Windows reserved name |
| Security     | junction/reparse      |
| State        | pause                 |
| State        | resume                |
| State        | cancel                |
| State        | delete data           |
| Performance  | 1 torrent             |
| Performance  | 10 torrents           |

---

# 68. Property / Fuzz Tests

重点：

```text
Magnet parser boundary
Torrent file path sanitization
Torrent path normalization
File selection serialization
Info hash parsing
Task ↔ Torrent mapping
RecoveryDecision
Torrent status mapping
u64 size handling
```

尤其：

```text
malicious / malformed torrent paths
```

必须做 fuzz/property-style testing。

---

# 69. Large File

继续使用：

```text
u64
```

禁止：

```text
i32
u32
usize
```

承担持久化媒体/文件大小语义，除非有明确理由。

至少验证：

```text
>2GB
>4GB
```

可以使用：

```text
sparse files
generated fixtures
```

避免测试仓库膨胀。

---

# 70. Database Migration

新增 schema 必须：

```text
new migration
```

禁止：

```text
修改已经发布 migration
```

要求：

```text
old DB
   ↓
migration
   ↓
new DB
```

可重复验证。

升级过程中：

```text
M0/M1/M2 existing task
```

不能被破坏。

---

# 71. Backward Compatibility

重点验证：

```text
只有 HTTP Task 的旧数据库
```

升级到 M3 后：

```text
HTTP Task 仍正常
```

然后：

```text
新增 Torrent Task
```

两者共存：

```text
HTTP
Torrent
```

不能因为 Torrent Engine 初始化失败导致：

```text
整个 Tide 无法启动
```

建议：

```text
HTTP core unavailable
Torrent feature unavailable
```

分别处理。

---

# 72. Engine Initialization Failure

如果：

```text
librqbit init failed
```

不能：

```text
panic
```

更不能：

```text
Tide 整体启动失败
```

应该：

```text
Engine capability unavailable
```

并给出 structured error。

如果这是不可恢复的核心安装问题，则明确记录 P0/P1。

---

# 73. App State

推荐继续使用依赖注入：

```text
AppState
    ├── TaskRepository
    ├── TorrentEngine
    ├── DownloadEngine
    └── RecoveryCoordinator
```

避免：

```text
static mut
lazy singleton
global TorrentManager
```

除非 Rust ownership / Tauri lifecycle 有充分理由。

---

# 74. Threading / Async

BitTorrent engine 属于长期运行异步系统。

需要明确：

```text
Tauri command
≠
long-running download task
```

Command 应：

```text
submit / inspect / control
```

而不是阻塞：

```text
await full torrent download
```

正确：

```text
Command
   ↓
TorrentEngine
   ↓
background async task
   ↓
events
```

---

# 75. Concurrency

至少验证：

```text
Add Torrent A
Add Torrent B
Pause A
Resume B
Remove A
```

并发情况下：

* task mapping 正确
* event 不串任务
* SQLite 不死锁
* engine state 不互相污染
* shutdown 不产生 orphan task

所有 public command 都应有：

```text
task ownership / state validation
```

---

# 76. Command Idempotency

例如：

```text
pause(task)
pause(task)
```

第二次：

```text
no-op
```

或者返回：

```text
AlreadyPaused
```

同理：

```text
resume(resuming)
remove(removed)
```

不要因为用户双击按钮就产生：

```text
duplicate engine operation
```

---

# 77. Race Conditions

重点测试：

```text
pause
+
completion
```

```text
remove
+
piece completed
```

```text
shutdown
+
torrent completed
```

```text
recovery
+
frontend startup
```

```text
task delete
+
engine event
```

必须有明确的：

```text
event ordering
stale-event rejection
task-id validation
```

---

# 78. Stale Event Protection

例如：

```text
Task A removed
↓
librqbit delayed callback for Task A arrives
```

不能：

```text
重新创建 Task A
```

必须：

```text
discard stale event
```

建议所有异步事件携带：

```text
task_id
engine_torrent_id
generation / session identity
```

必要时验证当前 mapping。

---

# 79. Session Generation

如果设计合理，增加：

```text
engine_session_generation
```

或等价概念。

避免：

```text
旧 session event
```

污染：

```text
新 session
```

这对 M2 restart recovery 非常重要。

---

# 80. Finalization

Torrent 不需要照搬 M1：

```text
.part → rename
```

除非经过实际验证 librqbit 需要。

优先：

```text
librqbit storage
    ↓
verified completed files
    ↓
Tide Completed
```

如果某种状态下存在：

```text
incomplete file
```

不要自行 rename 成最终完成状态。

---

# 81. File Completion Semantics

M3 应支持：

```text
whole torrent completed
```

并且能够识别：

```text
file A completed
file B partial
```

即：

```text
Torrent completion
≠
every individual file necessarily completed
```

尤其当：

```text
partial file selection
```

存在时必须定义清晰。

建议：

```text
Task Completed
=
all selected required data complete
```

而：

```text
Torrent has unselected files
```

不应该导致任务永远无法完成。

---

# 82. Selected / Unselected Semantics

例如：

```text
Movie.mkv ✅
Trailer.mp4 ❌
Poster.jpg ❌
```

则：

```text
Task = Completed
```

应该是合法状态。

同时 UI 应显示：

```text
Selected: 1 / 3 files
```

---

# 83. Torrent Naming / Metadata Sanitization

Torrent Name 是不可信字符串。

不能直接拿来：

```text
folder name
window title
SQL interpolation
log path
```

必须：

```text
display name
filesystem safe name
persistent canonical name
```

三者可以不同。

---

# 84. DB 不保存大 Blob，除非必要

Torrent metadata 可以保存到：

```text
torrent-metadata/<task-id>.torrent
```

SQLite 保存：

```text
metadata_path
metadata_hash
info_hash
```

不要轻易：

```text
完整 .torrent binary BLOB
```

塞入数据库。

除非实际测试证明这样更合理。

---

# 85. Torrent File Metadata Integrity

如果保存了：

```text
task-id.torrent
```

建议记录：

```text
metadata SHA-256
```

启动时可以验证：

```text
file exists
hash matches
```

防止：

```text
metadata file silently changed
```

---

# 86. M3 UI 不做“播放器预览”

即使 Torrent 已经下载了一部分：

**M3 不显示播放按钮。**

可以显示：

```text
Partially downloaded
Availability
Ready for streaming in M4
```

但不要接：

```html
<video>
```

---

# 87. Manual Verification

开发完成后，在 Windows 11 上真实运行：

### Case A

```text
加入合法公开测试 torrent
→ metadata
→ file list
→ download
```

### Case B

```text
pause
→ close app
→ reopen
→ resume
```

### Case C

```text
kill process
→ reopen
→ automatic recovery
```

### Case D

```text
partial selected file
→ reopen
→ continue
```

### Case E

```text
malicious filename torrent
→ reject
```

### Case F

```text
duplicate magnet
→ no duplicate download
```

---

# 88. 真实 Torrent 测试来源

正式验证时只使用：

* 自己生成的合法测试 Torrent
* 自己运行的 local seeder
* 明确授权分发的内容
* 公有领域内容
* 官方发布的测试资源

不要在开发提示词中加入：

```text
资源站
磁力搜索站
tracker lists
盗版资源
```

Tide 本身也不实现资源发现。

---

# 89. Documentation

新增至少：

```text
docs/M3-torrent-architecture.md
docs/M3-librqbit-integration.md
docs/M3-persistence-recovery.md
docs/M3-file-selection.md
docs/M3-security.md
docs/M3-test-matrix.md
docs/M3-report.md
```

其中：

### M3-torrent-architecture.md

说明：

```text
Tide
↓
TorrentEngine
↓
LibrqbitAdapter
↓
librqbit
```

### M3-librqbit-integration.md

记录：

```text
exact version
features
API used
configuration
limitations
upgrade constraints
```

### M3-persistence-recovery.md

记录：

```text
SQLite
vs
librqbit state
vs
filesystem
```

### M3-security.md

记录：

```text
path traversal
network listeners
UPnP
upload policy
logs
dependency audit
```

---

# 90. THIRD_PARTY_NOTICES

如果项目当前已有：

```text
THIRD_PARTY_NOTICES.md
```

必须更新。

至少记录：

```text
librqbit
version
license
source
```

并根据最终依赖树确认 transitive obligations。

不要凭记忆填写 license。

**从实际依赖与上游 LICENSE 文件核实。**

---

# 91. Privacy

M3 不能破坏：

```text
No telemetry by default
```

不得新增：

```text
torrent analytics
peer analytics
tracker analytics
usage reporting
```

不得把：

```text
torrent metadata
magnet
info_hash
peer IP
```

上传到 ifuyo。

---

# 92. Content Policy

继续维持 Tide 的定位：

```text
User-provided source
```

产品不主动：

```text
search
index
recommend
rank
discover
```

UI 文案不要出现：

```text
Find resources
Search torrents
Recommended trackers
Popular torrents
```

---

# 93. Performance / Resource Limits

不要为了“强”默认无限制。

至少有：

```text
max concurrent torrents
max simultaneous metadata resolutions
event update throttle
```

如果暂时没有用户可配置：

至少代码层保持合理上限 / backpressure。

不要产生：

```text
100 torrent × high-frequency UI event
```

导致 WebView2 被刷爆。

---

# 94. M3 Definition of Done

只有同时满足以下条件，M3 才算完成：

### Source

```text
Magnet works
Local .torrent works
```

### Metadata

```text
Metadata resolved
Files listed
Multi-file correct
```

### Download

```text
Real BitTorrent download works
```

### Selection

```text
Selected file download works
```

### State

```text
Pause
Resume
Cancel
Remove
```

真实生效。

### Persistence

```text
SQLite
+
librqbit state
+
filesystem
```

能够恢复。

### Verification

```text
piece verification
corruption recovery
```

真实验证。

### Security

```text
torrent path traversal blocked
Windows path abuse blocked
UPnP disabled by default
HTTP API not exposed
sensitive logging controlled
```

### Architecture

```text
Tide
↕
TorrentEngine
↕
librqbit
```

边界清晰。

### Compatibility

```text
M1 HTTP task remains working
M2 recovery remains working
```

### Tests

```text
unit
integration
recovery
security
failure injection
performance
```

均通过。

### Build

必须真实执行：

```bash
cargo check --locked
cargo test --locked
cargo clippy --all-targets
```

前端 / Tauri TypeScript 也必须按项目已有规范验证。

绝对不要写：

```text
应该可以
理论上可以
未验证但通过
```

---

# 95. M3 核心验收链

最终必须证明：

```text
Magnet / .torrent
        ↓
Source Validation
        ↓
Metadata
        ↓
File Selection
        ↓
Torrent Task
        ↓
librqbit Session
        ↓
Managed Torrent
        ↓
Piece Download
        ↓
Real Filesystem
        ↓
Verification
        ↓
Tide Task Completed
```

以及：

```text
Running Torrent
      ↓
App Crash
      ↓
Restart
      ↓
librqbit Recovery
      ↓
Tide Reconciliation
      ↓
Continue Download
```

---

# 96. M3 最终报告格式

完成后不要只说：

```text
M3 finished.
```

必须输出：

```text
# M3 Completion Report

## 1. Executive Summary

## 2. FACTS

## 3. CURRENT M0/M1/M2 STATE

## 4. librqbit Version / Dependency Audit

## 5. Architecture Changes

## 6. TorrentEngine Design

## 7. Source Model

## 8. Metadata Model

## 9. File Selection

## 10. Storage Architecture

## 11. Persistence Architecture

## 12. Recovery Algorithm

## 13. Tide ↔ librqbit State Mapping

## 14. Upload / Sharing Policy

## 15. Network Configuration

## 16. Security Audit

## 17. Test Matrix

## 18. Crash / Failure Injection

## 19. Performance Results

## 20. Manual Validation

## 21. Files Changed

## 22. FACT / HYPOTHESIS / INFERENCE

## 23. RESOLVED / CONFIRMED / OPEN / WORSENED

## 24. P0 / P1 Risks

## 25. Known Limitations

## 26. M4 Readiness

## 27. M3 Definition-of-Done Checklist
```

---

# 97. M3 最终输出必须特别回答的 12 个问题

```text
1. 当前实际使用的 librqbit 精确版本是什么？
2. 为什么选择这个版本？
3. 是否直接使用 librqbit library？
4. Tide 与 librqbit 的 state ownership 是什么？
5. Magnet metadata 如何持久化？
6. Torrent 文件如何持久化？
7. Task crash recovery 如何与 engine fast resume 对齐？
8. File selection 如何持久化？
9. Multi-file + piece boundary 是否正确？
10. DownloadOnly 是否有真实技术保证？
11. Torrent path traversal / reparse point 是否有测试？
12. M4 的 Resource.read(range) / Availability 是否已经具备清晰数据基础？
```

---

# 98. 最终硬性边界

M3 完成后：

**必须停止。**

不要继续实现：

```text
M4 Progressive Streaming
M4 Stream Gateway
M4 Scheduler
M5 Player
M6 Library
```

即使：

```text
librqbit 已经可以 stream
```

也不要顺手做。

---

# M3 的最终目标

不是：

> “Tide 现在能下载磁力了。”

而是：

> **“Tide 现在拥有第二个真正可靠的 Download Backend，而且 Torrent 已经被干净地纳入 Task / Resource / Persistence / Security 架构，为 M4 的 Progressive Streaming 做好了真实基础。”**

最终系统边界应接近：

```text
                    ┌── HTTP/HTTPS DownloadEngine
User Source ────────┤
                    └── TorrentEngine
                           │
                           ↓
                    Resource / Availability
                           │
                           ↓
                    Persistence / Recovery
```

M4 才继续：

```text
Resource
    ↓
Demand
    ↓
Scheduler
    ↓
Readable Range
    ↓
Progressive Stream Gateway
    ↓
WebView2 Player
```

**M3 不越过这条线。**
