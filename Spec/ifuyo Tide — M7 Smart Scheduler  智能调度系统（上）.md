# ifuyo Tide — M7 Smart Scheduler / 智能调度系统（上）

> **拆分说明（必读）**：本文件是 M7 完整开发实施提示词的**上半部分（第 0~96 节：模型与算法 —— Demand / PriorityClass / Buffer 模型、调度循环与 Tick、Fairness 与 Aging、带宽分配、Playback 状态策略、运行时与持久化边界、可观测性以及六个完整调度示例）**。合规边界、完整测试体系、性能、恢复与 Prefetch、实施顺序与最终验收位于下半部分《ifuyo Tide — M7 Smart Scheduler  智能调度系统（下）.md》。两部分必须配合使用：先完成上半部分，再执行下半部分；只完成上半部分不构成 M7 完成。

## 完整开发实施提示词

你现在负责实现 **ifuyo Tide M7：Smart Scheduler / 智能调度系统**。

Tide 是一个 Windows 本地桌面应用：

> **用户自己提供来源的本地下载器 + 渐进式媒体播放器**

M7 是 Tide 的核心架构阶段之一。

本阶段不是简单增加“优先级数字”，也不是给下载任务做一个排序列表。

本阶段要建立 Tide 真正的：

> **Demand → Scheduling Policy → Resource Allocation → Engine Execution**

体系，使 Tide 能够同时处理：

* 普通 HTTP/HTTPS 下载
* BitTorrent 下载
* 正在播放的 Torrent/HTTP 资源
* 用户主动操作的任务
* 后台任务
* 多任务并发
* 播放中的动态 seek
* 缓冲不足
* 网络波动
* 用户带宽上限
* 任务公平性
* 防止后台任务永久饿死
* 应用重启后的调度重建

最终目标：

> **Tide 不再只是“多个下载任务同时跑”，而是能够理解当前用户真正需要哪些字节，并把有限的网络、磁盘和引擎能力优先分配给最有价值的 Demand。**

---

# 0. 极其重要：先检查，不要直接修改

开始任何代码修改之前，必须完整检查当前仓库。

首先确认：

* 当前 Git branch
* 工作区状态
* 当前 commit
* Rust / Cargo 版本
* Tauri 版本
* frontend package manager
* Node.js / pnpm/npm/yarn 版本
* SQLite 实际实现
* M0 当前架构
* M1 当前实现
* M2 当前实现
* M3 当前实现
* M4 当前实现
* M5 当前实现
* M6 当前实现
* 当前 `Task`
* 当前 `Source`
* 当前 `Resource`
* 当前 `Availability`
* 当前 `PlaybackSession`
* 当前 `DownloadEngine`
* 当前 `librqbit` 依赖版本
* rqbit 当前实际暴露的 public API
* 当前 Range / Streaming Gateway
* 当前 Player
* 当前 Task persistence / recovery
* 当前 IPC
* 当前事件系统
* 当前错误系统
* 当前日志系统
* 当前配置系统

尤其禁止根据本提示词假定 M3/M4/M5/M6 已经按照某种具体方式完成。

---

# 1. 第一阶段：审计输出，禁止一上来写代码

开始后先输出以下内容：

## FACTS

只写从代码、配置、Cargo manifest、数据库 schema、测试和文档中实际确认的事实。

## HYPOTHESES

列出无法直接确认、需要通过代码阅读或实验验证的假设。

## CURRENT M0–M6 STATE

分别描述：

* 已实现
* 部分实现
* 未实现
* 与本提示词预期不一致的地方

## M7 DEPENDENCY MAP

说明 M7 当前依赖哪些：

* Task
* Source
* Resource
* Availability
* Download Engine
* HTTP
* Torrent
* Playback
* Stream Gateway
* Persistence
* IPC

## M7 IMPLEMENTATION PLAN

先给出实际准备修改的模块和顺序。

## RISKS

使用：

* P0
* P1

禁止使用 P2/P3。

如果 M3/M4/M5/M6 当前存在结构问题，先识别问题。

不要为了 M7 强行绕过结构问题。

---

# 2. M7 的核心目标

M7 要实现：

```text
User Intent
    ↓
Demand
    ↓
Demand Aggregation
    ↓
Scheduling Policy
    ↓
Resource Allocation
    ↓
Engine-specific Commands
    ↓
HTTP Engine / librqbit
    ↓
Network / Disk
```

同时：

```text
PlaybackSession
      ↓
PlaybackDemand
      ↓
byte-range / availability requirement
      ↓
Scheduler
      ↓
priority / allocation
      ↓
Torrent / HTTP
```

Scheduler 必须成为**独立的策略层**。

---

# 3. M7 最重要的架构边界

严格建立：

```text
Scheduler
    ↓
Scheduling Decision
    ↓
Engine Adapter
    ↓
Download Engine
```

而不是：

```text
Scheduler
    ↓
HTTP socket
    ↓
Torrent peer
    ↓
piece request
```

Scheduler 禁止直接：

* 创建 HTTP socket
* 管理 HTTP connection
* 管理 Torrent peer
* 管理 tracker
* 管理 DHT
* 请求 peer piece
* 实现 BitTorrent protocol
* 自己解析 Torrent protocol
* 自己完成文件下载
* 自己写文件
* 绕过 DownloadEngine
* 绕过 librqbit

Scheduler 是：

> **Control Plane**

HTTP/Torrent Engine 是：

> **Data Plane**

---

# 4. 一个最重要的原则：Demand First

不要让 Scheduler 从：

```text
Task list
→ priority sort
```

开始设计。

必须从：

```text
Demand
```

开始。

因为：

> “用户正在看电影第 42 分钟”

与：

> “用户下载一个 10GB 文件”

虽然都属于 Download Task，

但它们的调度需求完全不同。

---

# 5. Demand 模型

建立统一的 `Demand` 抽象。

建议至少具备：

```text
DemandId
TaskId
ResourceId
DemandKind
PriorityClass
ByteRange / RangeSet
CreatedAt
Deadline
Urgency
Weight
MinimumThroughput
TargetThroughput
State
Reason
```

实际字段必须根据当前架构调整。

---

# 6. DemandKind

至少支持：

```text
PlaybackStartup
PlaybackContinue
PlaybackSeek
PlaybackRebuffer
InteractiveDownload
BackgroundDownload
ExplicitUserPriority
Recovery
Verification
```

可以根据现有系统合并。

但必须能够区分：

### Playback Startup

播放器刚刚打开。

需求：

```text
尽快拿到最前面的可播放数据
```

### Playback Continue

正常播放。

需求：

```text
保持 playhead 前方足够安全的 buffer
```

### Playback Seek

用户突然拖到远处。

需求：

```text
旧位置 Demand 快速降权
新位置 Demand 快速升权
```

### Playback Rebuffer

即将卡顿。

需求：

```text
临时进入紧急调度状态
```

### Interactive Download

用户明确点击开始下载。

### Background Download

低紧迫度后台任务。

### Recovery

程序重启后恢复任务。

### Verification

校验、最终确认等后台工作。

---

# 7. PriorityClass

不要仅仅使用：

```text
priority = 1..100
```

推荐建立语义化优先级：

```text
Emergency
Interactive
Playback
Normal
Background
Maintenance
```

再映射到内部权重。

但是：

> **PriorityClass 不是最终 Scheduler Decision。**

Scheduler 必须结合：

* urgency
* deadline
* buffer health
* task age
* user intent
* current bandwidth
* estimated completion
* network condition
* starvation
* task state
* resource availability

进行计算。

---

# 8. Playback Buffer 模型

M7 必须真正理解播放缓冲。

至少建立：

```text
playhead
availableAhead
availableBehind
bufferTarget
bufferLowWatermark
bufferCriticalWatermark
bufferHighWatermark
```

例如：

```text
buffer < critical
    → Emergency

critical <= buffer < low
    → Rebuffer Risk

low <= buffer < target
    → Playback Priority

buffer >= target
    → Playback Demand 降低

buffer >> target
    → 可以把资源让给其他任务
```

具体阈值不要拍脑袋。

通过：

* M4 实测
* M5 实际播放
* 网络实验

确定合理初始值。

将参数集中管理：

```text
SchedulerConfig
PlaybackSchedulingConfig
```

不要散落 magic number。

---

# 9. Demand 必须是 Range-aware

不要把需求表示成：

```text
TaskId = X
Priority = 100
```

而应该允许：

```text
Task A
需要：

[10MB, 30MB)
```

或者：

```text
[100MB, 110MB)
[200MB, 220MB)
```

建立：

```text
ByteRange
RangeSet
DemandRange
```

或者使用项目已有抽象。

必须明确区分：

```text
Demand
Availability
Downloaded
Buffered
```

这四者不是同一个概念。

---

# 10. Demand 与 Availability

Scheduler 不应该假设：

```text
requested range == available range
```

应该计算：

```text
Demand
    ↓
Availability
    ↓
Missing Range
    ↓
Scheduling Requirement
```

例如：

```text
Demand:
[100MB, 150MB)

Availability:
[0, 120MB)

Missing:
[120MB, 150MB)
```

只有缺失部分需要被调度。

---

# 11. Torrent 场景

Torrent 的 Scheduler 不应该替代 librqbit。

当前 librqbit 本身已经支持流式下载中的 piece prioritization、seek 和 HTTP Range streaming，因此 Tide 上层只需要表达播放/下载需求，再把适配后的意图交给 Torrent Engine。

建立：

```text
TorrentSchedulerAdapter
```

负责：

```text
Scheduler Decision
        ↓
Torrent Engine command
```

例如概念上：

```text
Playback range
    ↓
file priority / streaming priority
    ↓
librqbit
```

具体 API 必须以当前锁定版本实际 public API 为准。

---

# 12. 禁止直接调用 librqbit 私有实现

禁止：

* 修改 dependency source
* 依赖 private module
* 使用未经支持的内部结构
* 复制 rqbit 核心 piece selector
* 自己 fork 一个 scheduler

除非审计确认当前 public API 不足。

如果 public API 无法表达关键需求：

必须记录：

```text
P1:
librqbit public API limitation
```

然后选择：

```text
A. 使用已有 public API
B. 建立最小 adapter workaround
C. 留待后续版本
```

不能为了 M7 偷偷建立长期 fork。

---

# 13. HTTP 场景

HTTP 下载不能假设只有：

```text
从头下载到尾
```

Scheduler 必须支持未来：

```text
Resource → read(range)
```

这样的抽象。

HTTP Engine Adapter 应该接收类似：

```text
ScheduleIntent {
    task_id
    resource_id
    desired_ranges
    concurrency
    bandwidth_budget
}
```

实际模型根据当前 M1/M4 调整。

---

# 14. HTTP 与 Torrent 必须统一到同一 Scheduler

目标：

```text
                    Scheduler
                   /          \
                  /            \
        HTTP Adapter        Torrent Adapter
             ↓                    ↓
        HTTP Engine          librqbit
```

Scheduler 不应该知道：

```text
TCP
HTTP connection
Peer
Tracker
DHT
Piece hash
```

Scheduler 只应该知道：

```text
Resource
Demand
Availability
Capacity
Allocation
```

---

# 15. Scheduling Decision

建立明确的数据结构。

例如：

```text
SchedulingSnapshot
SchedulingDecision
ResourceAllocation
SchedulingIntent
```

至少需要描述：

```text
Task
Resource
Priority
Demand
Desired Range
Allocated Concurrency
Allocated Bandwidth
Deadline
Reason
```

实际字段以项目现有结构为准。

核心要求：

> Scheduler 的输出必须是可解释的。

例如：

```text
Task T1:
Priority = PlaybackEmergency
Allocation = 8 MB/s
Reason = buffer 4.2s < critical 6s

Task T2:
Priority = Background
Allocation = 1 MB/s
Reason = playback protection
```

---

# 16. 不要让 Scheduler 成为黑箱

必须提供：

```text
DecisionReason
```

至少能够解释：

```text
为什么这个任务现在优先？
为什么这个任务被降速？
为什么这个任务暂停？
为什么播放任务抢占后台下载？
为什么 seek 后旧 Demand 被取消？
```

这对于调试和未来 UI 非常重要。

---

# 17. 调度循环

建立统一 Scheduler Loop。

例如：

```text
Timer / Trigger
     ↓
Collect Snapshot
     ↓
Update Demand
     ↓
Read Availability
     ↓
Calculate Urgency
     ↓
Apply Fairness
     ↓
Calculate Allocation
     ↓
Apply Limits
     ↓
Generate Decisions
     ↓
Diff Current Allocation
     ↓
Apply Changes
```

不要每个 tick 都强制重启任务。

必须做：

```text
Decision diff
```

只有发生实际变化才发送 engine command。

---

# 18. Scheduler Tick

初始可以使用：

```text
100ms ~ 1000ms
```

范围内的低频 tick。

不要默认：

```text
16ms
60FPS
```

Scheduler 不是渲染循环。

优先：

```text
event-driven + periodic reconciliation
```

例如：

```text
Playback position changed
Buffer changed
Seek
Task created
Task paused
Task resumed
Network changed
Availability changed significantly
Timer tick
```

触发重新计算。

---

# 19. 避免高频抖动

禁止：

```text
8 MB/s
→ 3 MB/s
→ 8 MB/s
→ 4 MB/s
→ 7 MB/s
```

每几十毫秒不停变化。

应该加入：

```text
hysteresis
minimum allocation duration
change threshold
cooldown
```

例如：

只有当：

```text
priority change significant
OR
buffer state changed
OR
allocation difference > threshold
OR
new urgent Demand
```

才更新。

---

# 20. Fairness

必须解决：

> Playback 不应该导致普通任务永久饿死。

同时：

> 普通任务也不应该因为公平而导致播放卡顿。

建立明确策略。

例如：

```text
Emergency Playback
    > Playback
    > Interactive
    > Normal
    > Background
```

但同一级别内部需要公平。

可以使用：

* Weighted Fair Queuing
* Deficit Round Robin
* Aging
* Virtual Finish Time

任选合适方案。

不要为了“高级算法”而复杂化。

关键目标：

```text
No starvation
No unnecessary thrashing
Predictable allocation
```

---

# 21. Aging

例如：

一个 Background task 已经等待很久。

它的有效优先级应该逐渐增加：

```text
effective_priority =
    base_priority
    + aging_bonus
```

但必须有：

```text
aging cap
```

避免后台任务随着时间最终压过 Playback。

---

# 22. 权重系统

建立：

```text
base_weight
urgency_weight
aging_weight
playback_weight
user_weight
```

但不要让它变成不可维护的 30 项公式。

最终应该能回答：

```text
为什么 Task A > Task B？
```

而不是：

```text
因为 score = 0.73
```

如果使用 score：

必须允许拆解：

```text
base = ...
urgency = ...
aging = ...
playback = ...
fairness = ...
final = ...
```

便于测试和 debug。

---

# 23. 用户显式优先级

用户应该可以表达：

```text
Highest
High
Normal
Low
Background
```

但：

> 用户优先级不是绝对命令。

例如：

```text
PlaybackEmergency
```

仍然可以暂时超过用户标记为：

```text
High
```

的普通下载。

同时必须在 decision reason 中说明原因。

---

# 24. 用户带宽限制

实现全局配置：

```text
Download Bandwidth Limit
```

支持：

```text
Unlimited
N KB/s
N MB/s
```

未来预留：

```text
Playback reserved bandwidth
```

但 M7 不要做复杂的时间段限速。

不要提前实现：

* 每小时限速
* 日历
* 夜间策略
* 网络类型识别
* VPN 自动检测

这些留给后续需求。

---

# 25. Per-task bandwidth

允许：

```text
global limit
+
task weight / cap
```

例如：

```text
Global = 20 MB/s

Task A = 60%
Task B = 30%
Task C = 10%
```

但如果：

```text
A is PlaybackEmergency
```

可以突破正常 share，在全局 cap 内重新分配。

---

# 26. Min / Target / Max allocation

建议每个 Demand 区分：

```text
minimum
target
maximum
```

例如：

```text
Playback:
minimum = 保证不继续恶化
target = 正常 buffer 恢复
maximum = 避免过量抢占

Background:
minimum = 可能为 0
target = 正常吞吐
maximum = 全局剩余资源
```

最终算法必须避免：

> 为了给 Playback 补 10 秒 buffer，错误地把整个 Torrent 下载资源全部长期吞掉。

---

# 27. Playback Scheduling 策略

至少实现以下状态：

```text
STARTUP
PLAYING
LOW_BUFFER
CRITICAL_BUFFER
SEEKING
PAUSED
ENDED
ERROR
```

---

# 28. STARTUP

播放器刚开始：

目标：

```text
快速形成最小可播放 buffer
```

策略：

```text
temporarily elevate playback demand
```

达到：

```text
startup target
```

后逐步释放抢占资源。

---

# 29. PLAYING

正常播放：

Scheduler 不需要疯狂下载。

目标：

```text
维持安全 buffer
```

例如：

```text
playhead + target buffer
```

之后优先满足。

---

# 30. LOW_BUFFER

buffer 较低：

提高：

```text
urgency
```

降低：

```text
background allocation
```

但不要立刻暂停所有后台任务。

---

# 31. CRITICAL_BUFFER

buffer 快耗尽：

进入：

```text
Emergency
```

可以：

```text
暂停/极低配额后台任务
最大化 Playback Demand
```

直到恢复到：

```text
recovery threshold
```

而不是只恢复 1 秒就退出 Emergency。

---

# 32. SEEK

Seek 是 M7 的核心测试。

例如当前：

```text
playhead = 10 min
```

用户突然：

```text
seek → 70 min
```

必须：

```text
旧 Demand
    ↓
快速 invalidate / downgrade

新 Demand
    ↓
立即 elevate
```

绝对不能继续大量下载：

```text
10 min → 11 min → 12 min ...
```

然后用户已经到了 70 分钟。

---

# 33. Seek hysteresis

不能因为播放器发送大量 position events 就不停调整。

要：

```text
coalesce
debounce
```

最终只对稳定 seek 位置产生 Demand。

---

# 34. Buffer 不是简单“下载字节数”

必须从：

```text
Availability
```

与：

```text
Playback Position
```

计算。

例如：

```text
available:
[0, 200MB)

playhead:
100MB

buffer ahead:
100MB
```

如果：

```text
availability:
[0, 110MB)
+
[150MB, 200MB)
```

那么：

```text
buffer ahead ≠ 100MB
```

因为中间有 hole。

Scheduler 必须尊重：

> Availability holes。

---

# 35. Range Demand 与 Torrent Piece

播放需要：

```text
byte range
```

Torrent 实际工作是：

```text
piece
```

必须通过：

```text
Range → Piece mapping
```

或者交给 rqbit。

不要让 Scheduler 自己重建完整 Piece Scheduler。

---

# 36. 文件选择

多文件 Torrent 必须支持：

```text
Resource / File / Range
```

用户播放其中一个文件时：

优先：

```text
selected file
```

而不是整个 Torrent。

例如：

```text
Torrent:
A.mp4
B.mp4
C.srt
D.iso
```

播放：

```text
A.mp4
```

Scheduler 不应该因为整个 Torrent 是一个 Task，就公平地把带宽分给：

```text
B
C
D
```

---

# 37. 字幕相关

M7 不负责完整字幕系统。

但 Scheduler 数据模型应该允许未来：

```text
main media Demand
+
subtitle Demand
```

不要把字幕逻辑硬编码到 Scheduler。

---

# 38. 预读方向

正常播放时：

```text
forward buffer
```

权重大于：

```text
behind buffer
```

但是不要完全忽视 behind。

保留少量：

```text
seek-back buffer
```

参数化。

---

# 39. 未来 seek prediction

不要在 M7 实现复杂 AI 预测。

可以预留：

```text
PlaybackPattern
```

但第一版只实现：

```text
forward priority
```

以及：

```text
recently visited range
```

不要做：

* ML
* 用户画像
* 云端推荐
* 行为上传

---

# 40. 网络状态

建立简单：

```text
NetworkSnapshot
```

例如：

```text
bandwidth estimate
latency estimate
recent throughput
error rate
connection availability
```

可以用于 Scheduler。

不要在 M7 实现：

* VPN 检测
* ISP 识别
* 地理定位
* 网络指纹
* 第三方测速服务

---

# 41. Bandwidth estimation

下载速度应该来自真实 engine telemetry。

不要使用：

```text
random
fake progress
estimated fake speed
```

至少区分：

```text
instant throughput
smoothed throughput
allocated throughput
```

推荐 EMA：

```text
smoothed =
alpha * current
+
(1-alpha) * previous
```

alpha 必须集中配置。

---

# 42. Scheduler Capacity

Scheduler 需要知道当前可分配资源：

```text
network capacity
disk capacity
engine concurrency
task concurrency
```

建立：

```text
SchedulerCapacity
```

不要把：

```text
network bandwidth
HTTP connection count
torrent peer count
```

混在同一个 integer。

不同资源有不同含义。

---

# 43. 多资源约束

最终分配应该受到多个 constraint 限制：

```text
Global bandwidth
Task bandwidth
Engine concurrency
Disk write capability
Playback urgency
```

最终：

```text
allocation <= all applicable limits
```

---

# 44. Scheduler 不负责磁盘调度

Scheduler 可以读取：

```text
disk availability
write pressure
```

但不能直接决定：

```text
file write syscall
```

仍由 DownloadEngine / Storage 完成。

---

# 45. Task 生命周期

复用 M0/M2 TaskState。

不要因为 M7 新建：

```text
SchedulerTaskState
```

然后与：

```text
TaskState
DownloadState
PlaybackState
```

互相冲突。

正确关系：

```text
Task lifecycle
    ↓
scheduler observes

Playback lifecycle
    ↓
scheduler generates Demand

Engine lifecycle
    ↓
scheduler observes
```

---

# 46. Runtime Scheduler State

以下内容属于 runtime：

```text
current allocation
current tick
active demand
recent throughput
fairness counters
cooldowns
buffer snapshot
```

不要全部写 SQLite。

应用重启后：

```text
load persistent tasks
+
load playback sessions if appropriate
+
rebuild runtime scheduler state
```

---

# 47. M2 Persistence 边界必须保留

数据库持久化：

```text
task
task configuration
user priority
download state
source
destination
```

Scheduler 的瞬时：

```text
score
allocation
tick
fairness counter
buffer estimate
```

一般不持久化。

---

# 48. Restart

程序重启：

```text
SQLite
    ↓
Task Restore
    ↓
Engine Restore
    ↓
Availability
    ↓
Scheduler Rebuild
```

不要：

```text
serialize Scheduler object
deserialize Scheduler object
```

Scheduler 必须可重建。

---

# 49. Event-driven Scheduler

可以监听：

```text
task.created
task.state_changed
task.progress
availability.changed
playback.started
playback.position_changed
playback.seeked
playback.buffer_changed
network.changed
```

但不要把高频数据无脑广播给 UI。

Tauri 官方区分了 command 与 event，events 是异步 JSON 消息机制，并不适合作为高吞吐实时流。

因此：

```text
Engine
    ↓
Runtime
    ↓
Scheduler
```

内部可以高频采样。

而：

```text
Scheduler
    ↓
Frontend
```

必须：

```text
coalesce
throttle
```

---

# 50. Scheduler Snapshot

建立可测试的 immutable snapshot。

例如：

```text
SchedulerSnapshot {

    now

    network

    global_limits

    tasks[]

    resources[]

    demands[]

    playback_sessions[]
}
```

Scheduler 输入：

```text
Snapshot
```

Scheduler 输出：

```text
Decision
```

尽量让核心算法：

```text
pure / deterministic
```

---

# 51. 强烈要求 Deterministic Scheduler

对于完全相同的：

```text
SchedulerSnapshot
+
SchedulerConfig
```

必须产生一致的：

```text
SchedulingDecision
```

除非明确存在：

```text
time-dependent aging
```

并且时间必须作为 Snapshot 输入。

禁止：

```text
SystemTime::now()
```

深藏在核心算法内部。

---

# 52. Scheduler Core

建议类似：

```text
Scheduler
├── demand.rs
├── demand_aggregator.rs
├── snapshot.rs
├── policy.rs
├── priority.rs
├── fairness.rs
├── allocation.rs
├── playback.rs
├── bandwidth.rs
├── decisions.rs
├── adapters/
│   ├── http.rs
│   └── torrent.rs
└── scheduler_loop.rs
```

但必须以现有 M0–M6 架构为准。

不要机械创建这些文件。

---

# 53. Demand Aggregation

多个组件可能生成 Demand：

```text
Playback
Task
User
Recovery
```

必须先：

```text
collect
```

再：

```text
merge
```

例如：

```text
Playback:
[100, 120)

User:
[100, 200)

Background:
full file
```

不能简单生成三个完全独立下载动作。

应该聚合为统一需求。

---

# 54. Demand Conflict

例如：

```text
Playback wants 100–120
Download wants entire file
```

Download 是：

```text
general demand
```

Playback 是：

```text
specific urgent demand
```

Scheduler 应该让：

```text
intersection:
100–120
```

具有更高 urgency。

---

# 55. Allocation Algorithm

至少解决：

```text
which task
which resource
which range
how much bandwidth
how much concurrency
```

推荐基本框架：

### Phase A

计算所有 Demand 的：

```text
urgency
weight
deadline
availability gap
```

### Phase B

确定：

```text
mandatory allocation
```

例如：

```text
PlaybackEmergency
```

### Phase C

剩余容量进行：

```text
fair allocation
```

### Phase D

剩余资源分配给：

```text
background
```

---

# 56. Deadline

Playback Demand 可以使用：

```text
deadline
```

例如：

```text
buffer end time
```

如果：

```text
playhead + buffer
```

即将达到：

```text
playback deadline
```

urgency 提升。

这通常比静态 priority 更准确。

---

# 57. 基于 Buffer 的 Urgency

可以使用类似：

```text
urgency = f(
    buffer_seconds,
    throughput,
    estimated_download_time,
    deadline
)
```

核心目的：

```text
预计下载速度 < 消耗速度
```

时提前提升优先级。

但不要追求复杂数学模型。

---

# 58. Rebuffer Protection

Scheduler 不应该等：

```text
buffer == 0
```

才行动。

应该根据：

```text
estimated throughput
+
download latency
+
current buffer
```

提前进入：

```text
risk state
```

---

# 59. Background Download

后台任务目标：

```text
use spare capacity
```

而不是：

```text
always maximize throughput
```

例如：

```text
Playback buffer healthy
→ background rises

Playback buffer declining
→ background falls

Playback critical
→ background near zero
```

---

# 60. 用户暂停播放

当：

```text
PlaybackSession = Paused
```

不要维持：

```text
PlaybackEmergency
```

Demand 应该：

```text
remove
```

或：

```text
downgrade
```

具体以产品策略为准。

建议默认：

```text
pause playback
→ playback demand removed
→ existing download can use capacity
```

---

# 61. 用户关闭播放器

必须：

```text
PlaybackSession destroyed
```

然后：

```text
Playback Demand destroyed
```

不能留下 zombie demand。

实现：

```text
Demand lifecycle cleanup
```

---

# 62. 垃圾 Demand 防护

Scheduler 必须定期检查：

```text
Task exists?
Resource exists?
PlaybackSession exists?
Demand owner exists?
```

发现孤立 Demand：

```text
remove
```

---

# 63. Engine Backpressure

Scheduler 下发 allocation 后，Engine 可能无法满足。

例如：

```text
requested = 10 MB/s
actual = 3 MB/s
```

这不是 Scheduler 错误。

必须区分：

```text
allocated
```

与：

```text
achieved
```

然后下一个周期重新计算。

---

# 64. Torrent 特殊情况

Torrent 可能存在：

```text
no peers
few peers
slow peers
availability holes
piece unavailable
```

Scheduler 不应该因为：

```text
allocated = 10 MB/s
```

就认为：

```text
10 MB/s will happen
```

必须使用真实：

```text
availability
throughput
peer/engine telemetry
```

进行反馈。

Scheduler 也不要直接操作 peer。

---

# 65. HTTP 特殊情况

HTTP 可能：

```text
server supports Range
server doesn't support Range
connection unstable
slow server
redirect
429
5xx
```

Scheduler 不应该实现 HTTP retry。

HTTP Engine 已在 M1 负责：

```text
retry
resume
validator
Range
```

Scheduler 只观察：

```text
capacity
state
availability
```

---

# 66. Retry 与 Scheduler 的边界

正确：

```text
Scheduler:
Task priority = High

HTTP Engine:
Retry after transient failure
```

错误：

```text
Scheduler:
if 503 then wait 1.7 sec
```

Retry policy 属于 Transport/Download Engine。

---

# 67. Concurrency

M7 可以控制：

```text
max active tasks
```

以及：

```text
per-engine concurrency
```

但不要做：

```text
per-peer connection tuning
```

除非底层库已经有明确 public API。

---

# 68. 最大并发

建立配置：

```text
global_max_active_tasks
http_max_active_tasks
torrent_max_active_tasks
```

但注意：

> active ≠ downloading at full speed。

Paused / waiting / metadata 状态应该有不同处理。

---

# 69. Scheduler Queue

不要创建一个：

```text
Vec<Task>
```

然后每次排序。

内部可能使用：

```text
priority queue
+
demand index
+
task/resource index
```

但首先保证正确性。

性能优化必须基于 profiling。

---

# 70. Complexity

默认目标：

```text
N tasks
M demands
```

Scheduler tick 尽量：

```text
O(N log N + M log M)
```

或更好。

不要每个：

```text
Task × Resource × Demand
```

暴力交叉。

---

# 71. No SQLite in scheduler tick

绝对禁止：

```text
scheduler tick
→ SELECT *
→ calculate
→ UPDATE task
→ SELECT
→ ...
```

Scheduler 必须依赖内存 snapshot。

SQLite 只负责持久化配置和生命周期状态。

SQLite 写冲突也不应成为高频调度路径；SQLite 官方文档明确说明并发写入时可能出现 `SQLITE_BUSY`，并建议通过合适的 busy timeout / transaction 策略处理，而不是让业务高频调度依赖数据库锁。

---

# 72. Scheduler 与 Persistence

需要持久化：

```text
user priority
bandwidth limits
scheduler preferences
```

不需要持续持久化：

```text
score
current allocation
buffer prediction
tick state
fairness counter
```

应用重新启动：

```text
reconstruct
```

而不是：

```text
restore memory graph
```

---

# 73. Config

建议：

```text
SchedulerConfig
```

集中管理：

```text
global bandwidth
max active tasks
playback target buffer
playback low watermark
playback critical watermark
aging rate
aging cap
fairness weights
allocation tick
change threshold
cooldown
```

所有参数：

* 有默认值
* 可测试
* 可序列化
* 有文档
* 不允许散落 magic number

---

# 74. 默认值必须经过实验

不要写：

```text
buffer_target = 30 sec
```

然后声称：

> 这是最佳值。

应记录：

```text
INITIAL DEFAULT
```

并根据 M4/M5 实测调整。

---

# 75. UI

M7 不需要大规模 UI redesign。

只增加能够验证 Scheduler 的必要 UI。

至少显示：

```text
Task Priority
Current State
Allocated bandwidth
Actual throughput
Scheduler reason
Playback buffer
```

播放 UI 至少能看到：

```text
Buffer
```

并用于验证：

```text
Play
Seek
Buffer low
Scheduler response
```

---

# 76. Scheduler Debug Panel

建议提供开发模式：

```text
Scheduler Debug
```

显示：

```text
Active demands
Task allocations
Priority class
Effective priority
Buffer health
Bandwidth limit
Decision reason
```

开发模式即可。

不要默认暴露复杂内部数据给普通用户。

---

# 77. Fake Data 禁止

所有：

```text
speed
progress
buffer
allocation
priority
```

必须来自真实系统。

禁止：

```text
Math.random()
fake timer
fake network
fake progress
```

测试环境可以使用 deterministic mock。

---

# 78. IPC

Command 用于：

```text
set_task_priority
set_global_bandwidth_limit
get_scheduler_snapshot
start_playback
stop_playback
```

Event 用于：

```text
scheduler.updated
task.updated
playback.buffer_changed
```

但是必须节流。

不要：

```text
scheduler tick every 100ms
→ frontend JSON event every 100ms
```

除非 profiling 证明有必要。

---

# 79. Scheduler Snapshot IPC

Frontend 如果请求：

```text
get_scheduler_snapshot
```

只返回 DTO。

不能把：

```text
Rust internal Scheduler
```

暴露给 frontend。

---

# 80. 错误模型

加入：

```text
SchedulingError
```

但不要制造大量细碎错误。

至少区分：

```text
InvalidDemand
InvalidResource
EngineUnavailable
AllocationRejected
SchedulerConflict
Internal
```

---

# 81. Engine Adapter Contract

建立明确 trait/interface。

例如概念：

```text
trait SchedulingEngine {
    fn capabilities(...)
    fn apply_decision(...)
    fn observe(...)
}
```

实际 API 必须根据当前代码调整。

至少需要：

```text
HTTP adapter
Torrent adapter
```

---

# 82. Adapter Capability

不要假设所有 Engine 能力一样。

例如：

```text
HTTP:
Range = yes
Concurrency = yes
PiecePriority = no

Torrent:
Range = through resource stream
PiecePriority = yes
Concurrency = engine-controlled
```

Scheduler 应该知道：

```text
capability
```

而不是假设所有 Engine 都支持同样的操作。

---

# 83. Unsupported Decision

例如 Scheduler 想：

```text
prioritize arbitrary Range
```

但某 Engine 不支持。

不要：

```text
panic
```

应该：

```text
degrade gracefully
```

并生成：

```text
DecisionReason
```

例如：

```text
Range prioritization unavailable for this source
```

---

# 84. Availability Refresh

Scheduler 不应该每个 tick 完整扫描整个文件。

只在：

```text
meaningful change
```

时刷新。

对于 Torrent：

```text
piece availability changed significantly
```

才更新。

---

# 85. Availability 与 Scheduler 的性能边界

禁止：

```text
Scheduler tick
→ read every piece
→ rebuild all availability
```

Availability 应由资源层/engine 提供：

```text
snapshot
delta
query
```

---

# 86. Logging

Scheduler 日志可以记录：

```text
task_id
resource_id
priority class
allocation
decision reason
```

禁止记录：

* 完整 URL
* magnet
* info hash（除非必要且已脱敏）
* peer IP
* 用户完整路径
* 用户媒体文件名（无必要时）
* Token

生产日志不能成为用户历史记录。

---

# 87. Security

Scheduler 本身虽然不是主要安全边界，但必须：

* 不信任 frontend priority
* 不信任 frontend range
* 不信任 frontend task ID
* 不信任 frontend resource ID
* 二次验证 Task/Resource 所属关系
* 禁止通过 scheduler IPC 访问任意 Resource
* 不允许 frontend 构造 arbitrary filesystem target
* 不允许 scheduler 绕过现有 Security boundary

---

# 88. Authorization / Ownership

例如 frontend 请求：

```text
set_task_priority(task_id)
```

backend 必须验证：

```text
task exists
task belongs to current app state
task is mutable
```

不能：

```text
frontend tells backend Task X
→ blindly update
```

---

# 89. Scheduler 不能创建隐式下载任务

如果 Player 请求：

```text
read resource range
```

Scheduler 可以创建：

```text
PlaybackDemand
```

但不能偷偷创建：

```text
persistent DownloadTask
```

除非业务层明确要求。

---

# 90. Playback 与 Download 关系

必须保持：

```text
DownloadTask
```

和：

```text
PlaybackSession
```

分离。

Scheduler 只是协调：

```text
DownloadTask
+
PlaybackSession
```

不要合并成：

```text
PlaybackDownloadTask
```

---

# 91. 一个完整例子：正常播放

假设：

```text
Movie = 20 GB
Playhead = 40 MB
Availability = 0–500 MB
```

播放器需要：

```text
40–120 MB
```

Scheduler：

```text
Demand = [40,120)
Buffer = 460 MB
State = Healthy
```

结果：

```text
Playback allocation = moderate
Background tasks = receive remaining capacity
```

---

# 92. 一个完整例子：缓冲下降

网络突然变慢：

```text
download = 2 MB/s
playback consumption = 4 MB/s
```

buffer：

```text
60 sec
→
30 sec
→
15 sec
→
8 sec
```

Scheduler 应提前：

```text
Playback urgency ↑
Background allocation ↓
```

而不是等：

```text
buffer = 0
```

---

# 93. 一个完整例子：Seek

用户：

```text
10 min
→
72 min
```

系统：

```text
old demand invalidated
new demand generated
availability queried
missing range calculated
scheduler elevates new demand
engine receives new scheduling intent
```

必须验证：

```text
70–72 min
```

迅速得到资源，而不是继续：

```text
10–11–12–13...
```

---

# 94. 一个完整例子：两个下载 + 一个播放

资源：

```text
Global bandwidth = 20 MB/s

Movie playback
Download A
Download B
```

正常：

```text
Playback = 8 MB/s
A = 7 MB/s
B = 5 MB/s
```

Playback buffer下降：

```text
Playback = 15 MB/s
A = 3 MB/s
B = 2 MB/s
```

Playback critical：

```text
Playback ≈ global capacity
A/B ≈ minimum or 0
```

Buffer恢复：

```text
逐渐释放资源
```

不要立即瞬间切换造成抖动。

---

# 95. 一个完整例子：后台任务饥饿

假设：

```text
Playback 持续存在
```

但其 buffer长期稳定：

```text
> target
```

不能因为 Playback 永远优先：

```text
Background = 0 forever
```

Scheduler 应自动释放：

```text
unused playback capacity
```

让 Background 获得剩余资源。

---

# 96. 一个完整例子：多文件 Torrent

Torrent：

```text
movie.mkv
trailer.mp4
subtitles.srt
sample.jpg
```

用户播放：

```text
movie.mkv
```

Scheduler 应：

```text
movie = playback demand
others = low/background
```

不能平均下载整个 Torrent。

---

