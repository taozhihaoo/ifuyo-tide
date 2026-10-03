# ifuyo Tide — M7 Smart Scheduler / 智能调度系统（下）

> **拆分说明（必读）**：本文件是 M7 完整开发实施提示词的**下半部分（第 97~172 节：边界与合规、完整测试体系、性能、Failure Recovery 与 Prefetch、文档、实施顺序与最终验收）**，承接上半部分《ifuyo Tide — M7 Smart Scheduler  智能调度系统（上）.md》（第 0~96 节：模型与算法）。上半部分的阶段定位、执行纪律、架构原则与安全边界对本部分**全部生效**；执行本部分前必须先阅读并完成上半部分。
# 97. M7 不实现资源发现

禁止：

* 搜索 Torrent
* 搜索磁力
* 推荐资源
* Tracker 推荐
* Pirate site integration
* 第三方资源站点
* 搜索 API
* 资源排行

Scheduler 只处理：

> 用户已经提供给 Tide 的来源。

---

# 98. M7 不实现匿名能力

禁止添加：

* VPN
* Proxy
* Tor
* “匿名下载”
* Peer masking
* IP hiding

Tide 不应该以这些作为产品卖点。

---

# 99. M7 不实现云服务

禁止：

* 云端 Scheduler
* 云速度优化
* 中转服务器
* Relay
* Proxy backend
* CDN
* ifuyo server

Scheduler 必须：

> Local-first / local-only。

---

# 100. M7 不实现商业会员速度模型

不要加入：

```text
Free speed
Premium speed
Turbo server
Cloud acceleration
```

M7 所有速度差异必须来源于：

```text
local scheduling policy
network reality
user configuration
source capability
```

而不是虚假的云端“加速”。

---

# 101. Legal / Compliance

Tide 是通用下载器。

不要在 UI 或文档中声称：

```text
任何 Torrent 下载都是合法
```

也不要声称：

```text
Tide 保证用户不承担版权风险
```

对于 BitTorrent：

> 用户应仅使用其有权访问、下载、传播的内容。

不要设计：

* 破解 DRM
* 绕过付费墙
* 规避版权保护
* 匿名传播

---

# 102. Tests：第一原则

M7 必须大量测试 Scheduler。

不要只写：

```text
scheduler starts
scheduler stops
```

必须测试：

```text
decision correctness
fairness
playback priority
seek
buffer emergency
bandwidth
starvation
range
availability holes
engine capability
restart
```

---

# 103. Unit Test：Priority

测试：

```text
Emergency > Playback > Interactive > Normal > Background
```

但不要只测试 class。

还要测试：

```text
same class fairness
aging
caps
```

---

# 104. Unit Test：Demand

测试：

```text
create
merge
split
cancel
expire
replace
```

特别是：

```text
seek → old demand cleanup
```

---

# 105. Unit Test：Range

测试：

```text
[0,100)
[100,200)
overlap
adjacent
nested
disjoint
empty
invalid
```

以及：

```text
merge
subtract
intersection
difference
```

---

# 106. Unit Test：Availability

例如：

```text
Availability:
[0,100)
[200,300)

Demand:
[50,250)
```

Expected missing：

```text
[100,200)
```

必须精确。

---

# 107. Unit Test：Buffer

输入：

```text
playhead
availability
consumption_rate
```

测试：

```text
Healthy
Low
Critical
Emergency
```

边界必须覆盖：

```text
0
negative impossible values
unknown total
large offsets
hole ranges
```

---

# 108. Unit Test：Fairness

例如：

```text
A = Normal
B = Normal
C = Normal
```

经过足够 tick：

不得：

```text
A forever
B never
C never
```

必须验证没有 starvation。

---

# 109. Unit Test：Aging

测试：

```text
background waits
```

其 effective priority：

```text
monotonic increase
```

直到：

```text
aging cap
```

---

# 110. Unit Test：Aging 与 Playback

确认：

```text
Background aging
```

永远不能永久压过：

```text
PlaybackEmergency
```

---

# 111. Unit Test：Bandwidth

测试：

```text
global = 10 MB/s
```

所有 allocation 总和：

```text
<= 10 MB/s
```

包括：

* HTTP
* Torrent
* Playback
* Background

---

# 112. Unit Test：Task Limit

测试：

```text
max active = 3
```

永远不会同时存在：

```text
4 active transfers
```

除非特定 engine capability 明确允许。

---

# 113. Unit Test：Engine Capability

HTTP 不支持某种 scheduling feature：

必须：

```text
degrade
```

不能：

```text
panic
```

Torrent 支持：

```text
priority / streaming
```

则 adapter 应正确映射。

---

# 114. Property Testing

至少考虑：

```text
range operations
demand merging
allocation sum <= capacity
fairness invariants
priority monotonicity
state cleanup
```

随机生成：

```text
N tasks
M demands
K resources
```

运行 Scheduler：

必须满足：

```text
No invalid range
No negative allocation
No capacity overflow
No orphan decision
No duplicate demand
No starvation invariant violation
```

---

# 115. Deterministic Simulation

M7 强烈建议建立：

```text
Headless Scheduler Simulation
```

输入：

```text
tick
bandwidth
tasks
demands
availability
playhead
buffer
```

输出：

```text
allocations
states
```

这样无需真实网络即可验证算法。

---

# 116. Simulation Scenario 1

```text
Global = 20 MB/s

Playback = 0 buffer
Task A = normal
Task B = background
```

预期：

```text
Playback receives emergency allocation
```

---

# 117. Simulation Scenario 2

```text
Playback buffer = 120 sec
```

预期：

```text
Playback allocation decreases
```

并释放容量。

---

# 118. Simulation Scenario 3

```text
Playback seek
```

预期：

```text
old range allocation quickly disappears
new range allocation rises
```

---

# 119. Simulation Scenario 4

```text
3 background tasks
```

长期运行：

必须验证：

```text
fairness
```

---

# 120. Simulation Scenario 5

```text
bandwidth:
20 → 10 → 2 → 30 MB/s
```

验证：

```text
scheduler adapts
```

不得产生：

```text
allocation > global capacity
```

---

# 121. Simulation Scenario 6

```text
availability:
[0,100)
[200,400)
```

playhead：

```text
150
```

Scheduler 必须：

```text
identify hole
```

---

# 122. Integration Test：HTTP

真实：

```text
HTTP Resource
+
HTTP Engine
+
Scheduler
```

验证：

```text
background
+
playback demand
```

能够动态调整。

---

# 123. Integration Test：Torrent

使用：

```text
合法测试 Torrent
```

例如：

* self-generated content
* public-domain content
* open-source distribution

验证：

```text
Torrent
→ librqbit
→ Availability
→ Scheduler
→ Playback
```

---

# 124. Integration Test：HTTP + Torrent

同时：

```text
HTTP Task
Torrent Task
Playback Session
```

检查：

```text
global bandwidth limit
fairness
playback priority
```

---

# 125. Integration Test：Seek

必须做真实：

```text
Start playback
→ buffer
→ seek
→ observe new demand
→ verify allocation changes
→ verify player continues
```

---

# 126. Integration Test：Rebuffer

模拟：

```text
slow server
slow torrent
high latency
intermittent failure
```

验证：

```text
buffer decreases
scheduler escalates
buffer recovers
scheduler relaxes
```

---

# 127. Integration Test：Paused Player

验证：

```text
pause playback
```

后：

```text
Playback Demand removed
```

后台任务得到资源。

---

# 128. Integration Test：Close Player

验证：

```text
player closes
```

以后：

```text
no playback demand
no zombie allocation
no leaked scheduler state
```

---

# 129. Integration Test：Application Restart

测试：

```text
Task persisted
App exits
App restarts
Scheduler reconstructed
```

验证：

```text
task state correct
scheduler state reconstructed
```

而不是恢复旧 runtime allocation。

---

# 130. Crash Test

至少在这些点注入故障：

```text
during scheduler update
after decision generated
before engine apply
after engine apply
during persistence
during playback session destruction
```

不能产生：

```text
deadlock
orphan task
zombie demand
invalid state
negative allocation
```

---

# 131. Race Tests

重点：

```text
Seek
+
Pause
+
Resume
+
Task Cancel
```

并发执行。

例如：

```text
seek
↓
pause
↓
scheduler tick
↓
cancel
↓
engine completion
```

最终状态必须确定。

---

# 132. Idempotency

Scheduler 重新收到：

```text
playback.seek
```

不能产生：

```text
2 identical active demands
```

Task pause/resume/cancel：

必须幂等。

---

# 133. Event Ordering

检查：

```text
Task Cancel
```

与：

```text
Download Completed
```

竞态。

最终状态应根据合法状态转换确定。

Scheduler 不得篡改 M0/M2 state machine。

---

# 134. Performance Benchmark

至少 benchmark：

```text
10 tasks
100 tasks
500 tasks
1000 tasks
5000 demands
```

记录：

```text
scheduler tick latency
CPU usage
memory
allocation computation
event generation
startup reconstruction
```

---

# 135. 不要过早优化

第一版优先：

```text
Correctness
Determinism
Explainability
Testability
```

之后才：

```text
Allocation performance
Memory
Lock contention
```

如果 profiling 显示瓶颈，再优化。

---

# 136. Concurrency Architecture

尽量避免：

```text
global Mutex<Scheduler>
```

包住所有工作。

推荐：

```text
Scheduler owns its state
messages/events enter scheduler
decisions leave scheduler
```

保持单一写者模型更容易推理。

---

# 137. Tokio / async

如果项目已经使用 Tokio：

Scheduler loop 可以基于 Tokio。

但不要：

```text
spawn 1000 tasks every tick
```

也不要：

```text
blocking DB
```

塞进 async runtime。

---

# 138. Locking

避免：

```text
Scheduler lock
→ Engine lock
→ DB lock
```

造成锁层级：

```text
Scheduler
  ↓
Engine
  ↓
Persistence
```

推荐：

```text
snapshot
→ release locks
→ calculate
→ apply through narrow interfaces
```

---

# 139. Frontend 不参与核心算法

禁止：

```text
React computes priority
```

禁止：

```text
frontend calculates bandwidth allocation
```

禁止：

```text
frontend decides which torrent piece
```

Frontend 只负责：

```text
display
user input
```

---

# 140. Scheduler Decision 可观测性

开发模式下每个 decision 可以产生：

```text
decision_id
task_id
resource_id
reason
allocation
timestamp
```

但不要无限保存历史。

只保留：

```text
ring buffer
```

例如最近：

```text
100–1000 decisions
```

具体根据性能决定。

---

# 141. Metrics

至少记录：

```text
scheduler_tick_ms
active_tasks
active_demands
playback_demands
allocation_changes
starvation_events
rebuffer_events
seek_events
```

以及：

```text
allocated_bandwidth
actual_bandwidth
```

---

# 142. Scheduler Health

可以增加：

```text
healthy
degraded
error
```

但不要让普通策略问题变成 App Fatal Error。

例如：

```text
Engine unavailable
```

应该：

```text
degrade
```

而不是：

```text
Application crash
```

---

# 143. Failure Recovery

Scheduler 异常：

应该：

```text
log
rebuild snapshot
retry decision
```

尽量不能：

```text
panic
```

正常业务异常禁止：

```text
unwrap()
expect()
panic!()
```

---

# 144. Resource / Task 消失

如果：

```text
Task deleted
```

Scheduler 必须：

```text
cancel demands
remove allocations
notify engine if needed
```

不能保留 dangling reference。

---

# 145. Resource Completion

如果：

```text
Resource completed
```

相关：

```text
Download Demand
```

应自动结束。

---

# 146. Playback End

如果：

```text
media ended
```

Scheduler：

```text
PlaybackDemand → complete
```

不要继续下载大量无意义数据。

---

# 147. 预下载策略

M7 可以实现有限：

```text
forward prefetch
```

例如：

```text
current playhead
+
target buffer
```

但禁止：

```text
提前下载整个剩余文件
```

除非用户明确选择“完整下载”。

---

# 148. Prefetch 与 Download Task

必须区分：

```text
Playback Prefetch
```

与：

```text
Full Download
```

Prefetch：

```text
Demand expires / changes
```

Full Download：

```text
persistent Task
```

---

# 149. 完整下载按钮

用户点击：

```text
Download
```

Scheduler 应生成：

```text
InteractiveDownload Demand
```

而不是假装：

```text
Playback Demand
```

---

# 150. 用户双重需求

用户：

```text
边播
+
同时下载完整文件
```

此时至少有：

```text
PlaybackDemand
FullDownloadDemand
```

Scheduler 应合并两者。

例如：

```text
playback range = 100–300 MB
full download = entire file
```

共同请求应避免重复资源竞争。

---

# 151. Demand Merge

最终应形成：

```text
effective demand:
entire file
```

但：

```text
priority map:
playback range = high
remaining range = normal
```

非常关键。

---

# 152. 不允许重复下载

HTTP/Torrent engine 最终不能因为：

```text
PlaybackDemand
+
DownloadDemand
```

建立两套相同数据下载。

必须共享：

```text
Resource
Availability
Download Task
```

---

# 153. 这将成为 Tide 的核心链

最终架构应逐渐形成：

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
Engine
  ↓
Storage
  ↓
Progressive Stream
  ↓
Player
```

这是 M7 最重要的成果。

---

# 154. Documentation

新增至少：

```text
docs/M7-scheduler.md
docs/M7-demand-model.md
docs/M7-allocation-policy.md
docs/M7-playback-scheduling.md
docs/M7-fairness.md
docs/M7-engine-adapter.md
docs/M7-test-matrix.md
docs/M7-report.md
```

根据仓库现有文档体系调整。

---

# 155. Scheduler 文档必须解释

至少包含：

```text
为什么需要 Scheduler
Demand 是什么
Availability 是什么
Scheduler 如何计算
Playback 如何优先
Fairness 如何实现
Aging 如何实现
Bandwidth 如何限制
HTTP 如何适配
Torrent 如何适配
Seek 如何处理
Restart 如何恢复
```

---

# 156. Architecture Decision Record

对于以下问题必须形成明确结论：

```text
Scheduler 是否单线程 owner？
Tick 频率？
Event trigger？
Priority 模型？
Fairness 算法？
Aging？
Bandwidth allocation？
Playback buffer threshold？
HTTP adapter capability？
Torrent adapter capability？
是否持久化 scheduler runtime？
```

每一项标：

```text
FACT
DECISION
RATIONALE
TRADE-OFF
```

---

# 157. 禁止把 Scheduler 做成“万能系统”

M7 完成后，以下仍然属于其他层：

### HTTP Engine

负责：

```text
HTTP protocol
Range
retry
resume
```

### Torrent Engine

负责：

```text
BitTorrent
peer
piece
tracker
DHT
```

### Storage

负责：

```text
filesystem
writes
atomic finalize
```

### Stream Gateway

负责：

```text
HTTP Range
security
streaming
```

### Player

负责：

```text
codec
playback
seek
UI
```

### Scheduler

只负责：

```text
Demand
Priority
Allocation
Coordination
```

---

# 158. M7 禁止新增功能

除非当前 M0–M6 已经存在并且只是必须适配，否则禁止趁机增加：

* NAS
* UPnP
* mDNS
* Cast
* SMB
* WebDAV
* VPN
* Proxy
* Cloud
* Account
* Telemetry
* Steam
* Workshop
* Subtitle marketplace
* Media discovery
* Metadata scraping
* Recommendation
* AI recommendation
* Content indexing platform

---

# 159. M7 实施顺序

严格按照以下顺序：

## Step 1

Audit M0–M6。

## Step 2

确认：

```text
Task
Resource
Availability
PlaybackSession
Engine
```

接口。

## Step 3

定义：

```text
Demand
DemandKind
PriorityClass
```

## Step 4

定义：

```text
SchedulerSnapshot
```

## Step 5

建立：

```text
priority / urgency
```

## Step 6

建立：

```text
fairness / aging
```

## Step 7

建立：

```text
bandwidth allocation
```

## Step 8

建立：

```text
Scheduler core
```

## Step 9

建立：

```text
HTTP adapter
```

## Step 10

建立：

```text
Torrent adapter
```

## Step 11

连接：

```text
PlaybackSession
```

## Step 12

实现：

```text
Seek
```

## Step 13

实现：

```text
Buffer emergency
```

## Step 14

实现：

```text
UI configuration
```

## Step 15

加入：

```text
Headless simulation
```

## Step 16

完成：

```text
integration tests
```

## Step 17

完成：

```text
performance tests
```

## Step 18

完成：

```text
documentation
```

---

# 160. Git Discipline

开始之前：

```text
git status
```

确认工作区。

不要：

```text
reset --hard
clean -fd
```

不要破坏用户已有修改。

M7 只能修改：

> 实际需要修改的代码、测试、配置、文档。

---

# 161. Dependency Discipline

不要因为 Scheduler：

```text
随便增加 crate
```

先检查当前依赖。

如果需要新 crate：

必须说明：

```text
why
alternatives
license
maintenance
size/performance impact
```

除非必要，不新增依赖。

特别不要为了一个：

```text
priority queue
```

引入大型 framework。

---

# 162. API Compatibility

如果：

```text
M3/M4/M5/M6 API
```

已经被现有 frontend 使用：

不要直接破坏。

优先：

```text
adapter
migration
deprecation
```

而不是大范围 rename。

---

# 163. Database

M7 尽量不修改数据库。

只有以下内容可能需要：

```text
task priority
scheduler preferences
bandwidth config
```

如果已有配置系统：

优先复用。

如果需要 Migration：

必须：

```text
new migration
transactional
repeatable
tested
```

绝对不能修改已有 migration 历史。

---

# 164. Acceptance Demo

最终必须能现场演示：

### Demo A

创建：

```text
HTTP Task A
Torrent Task B
```

两者正常工作。

### Demo B

启动媒体播放。

播放 Demand 自动建立。

### Demo C

后台下载自动让出资源。

### Demo D

buffer 下降。

Scheduler 自动提高 Playback 优先级。

### Demo E

用户 Seek。

旧 range 降权。

新 range 迅速升权。

### Demo F

暂停播放器。

Playback Demand 消失。

后台下载恢复。

### Demo G

用户修改全局带宽限制。

所有任务遵守。

### Demo H

关闭并重新打开应用。

Tasks 恢复。

Scheduler runtime 重新构建。

### Demo I

同时：

```text
Playback
+
Full Download
```

验证共享 Resource/Availability，不出现双重下载。

---

# 165. 核心验收指标

M7 不以：

```text
UI 漂亮
```

作为主要完成标准。

主要验收：

```text
1. Demand 正确
2. Range 正确
3. Availability 正确
4. Playback 优先级正确
5. Seek 正确
6. Buffer protection 正确
7. Fairness 正确
8. Aging 正确
9. Bandwidth cap 正确
10. HTTP adapter 正确
11. Torrent adapter 正确
12. Restart 可重建
13. Decision 可解释
14. 无重复下载
15. 无 zombie demand
16. 无 starvation
17. 无 allocation overflow
18. 无明显 scheduler jitter
```

---

# 166. 必须验证的核心不变量

Scheduler 运行任意场景，都必须尽可能保证：

```text
allocation >= 0
```

```text
sum(allocation) <= global_capacity
```

```text
invalid resource cannot receive allocation
```

```text
deleted task cannot receive allocation
```

```text
ended playback cannot receive playback allocation
```

```text
cancelled task cannot auto-reappear
```

```text
old seek demand cannot dominate new seek demand
```

```text
no demand points outside resource bounds
```

```text
no duplicate equivalent demand
```

```text
aging remains bounded
```

```text
background does not permanently starve
```

```text
playback emergency can obtain required capacity
```

---

# 167. DoD — Definition of Done

M7 只有满足以下全部条件才能视为完成。

## Architecture

* Scheduler 独立于 Engine
* Demand 独立于 Task
* Playback Demand 独立于 Download Task
* HTTP/Torrent 都通过 Adapter 接入
* Scheduler 不直接访问 socket/peer/filesystem

## Functional

* 多任务调度
* Playback-aware scheduling
* Buffer-aware priority
* Seek-aware scheduling
* HTTP scheduling
* Torrent scheduling
* Bandwidth limit
* Fairness
* Aging
* Background allocation
* Priority classes
* Decision reason

## Reliability

* restart reconstruction
* no zombie demand
* no duplicate demand
* no capacity overflow
* no invalid allocation
* race-safe
* deterministic core

## Testing

* unit tests
* integration tests
* simulation tests
* property tests
* seek tests
* rebuffer tests
* fairness tests
* bandwidth tests
* restart tests
* race tests
* large task-count tests

## Security

* IPC validation
* ownership validation
* no arbitrary resource access
* no arbitrary filesystem access
* no sensitive logs

## Performance

记录：

```text
10 tasks
100 tasks
500 tasks
1000 tasks
```

以及：

```text
scheduler tick latency
CPU
memory
allocation calculation
```

## Documentation

完成 M7 architecture / demand / policy / testing / report 文档。

## Build

必须通过：

```text
cargo check --locked
cargo test --locked
```

以及当前 frontend 的：

```text
lint
typecheck
test
build
```

具体命令以项目实际 package scripts 为准。

---

# 168. Final Report

完成后必须生成：

```text
docs/M7-report.md
```

报告结构：

## Executive Summary

## FACTS

## CURRENT M0–M6 STATE

## M7 Architecture

## Demand Model

## Scheduling Policy

## Priority Model

## Fairness Model

## Aging Model

## Bandwidth Allocation

## Playback Scheduling

## Seek Handling

## HTTP Adapter

## Torrent Adapter

## Persistence Boundary

## IPC

## UI

## Test Matrix

## Simulation Results

## Integration Results

## Performance Results

## Security Review

## FACT / HYPOTHESIS / INFERENCE

## RESOLVED / CONFIRMED / OPEN / WORSENED

## P0 / P1

## Known Limitations

## Changed Files

## M8 Readiness

## DoD Checklist

---

# 169. 最终输出必须非常诚实

不要因为：

```text
所有代码编译通过
```

就宣布：

> Smart Scheduler 已经验证正确。

必须区分：

```text
Implemented
Tested
Experimentally validated
Not yet validated
```

尤其对：

```text
real-world network behavior
Torrent swarm behavior
WebView playback behavior
```

不要过度推断。

---

# 170. 最终架构目标

M7 完成后，Tide 应逐渐形成：

```text
                    ┌──────────────────┐
                    │   User Intent    │
                    └────────┬─────────┘
                             ↓
                    ┌──────────────────┐
                    │      Demand      │
                    └────────┬─────────┘
                             ↓
                    ┌──────────────────┐
                    │  Demand Combine   │
                    └────────┬─────────┘
                             ↓
                    ┌──────────────────┐
                    │     Scheduler    │
                    │                  │
                    │ Priority         │
                    │ Urgency          │
                    │ Fairness         │
                    │ Aging            │
                    │ Bandwidth        │
                    │ Playback         │
                    └────────┬─────────┘
                             ↓
                    ┌──────────────────┐
                    │ SchedulingIntent │
                    └───────┬─────┬────┘
                            ↓     ↓
                ┌─────────────┐ ┌──────────────┐
                │ HTTP Adapter│ │Torrent Adapter│
                └──────┬──────┘ └──────┬───────┘
                       ↓                 ↓
                ┌────────────┐    ┌─────────────┐
                │ HTTP Engine│    │  librqbit   │
                └──────┬─────┘    └──────┬──────┘
                       └────────┬─────────┘
                                ↓
                         ┌────────────┐
                         │  Resource  │
                         │ Availability│
                         └──────┬─────┘
                                ↓
                         ┌────────────┐
                         │  Storage   │
                         └──────┬─────┘
                                ↓
                      Progressive Stream
                                ↓
                             Player
```

---

# 171. 最重要的最终原则

请牢记：

> **Scheduler 不是下载器。**

它不是：

```text
HTTP Client
```

不是：

```text
Torrent Client
```

不是：

```text
Player
```

不是：

```text
Storage
```

而是：

> **Tide 的资源分配大脑。**

它解决的问题不是：

> “怎么把数据下载下来？”

而是：

> **“在当前这一刻，Tide 最应该先拿哪些数据，以及应该给哪个来源多少资源？”**

最终必须做到：

```text
User Intent
    ↓
Demand
    ↓
Availability
    ↓
Urgency
    ↓
Scheduling
    ↓
Allocation
    ↓
HTTP / Torrent Engine
    ↓
Resource
    ↓
Progressive Playback
```

并且：

```text
播放越需要
    → Scheduler 越懂得抢资源

播放已经安全
    → Scheduler 越懂得释放资源

用户 Seek
    → Scheduler 越懂得迅速改变目标

后台任务等待很久
    → Scheduler 越懂得公平

网络突然变差
    → Scheduler 越懂得保护播放

任务数量增加
    → Scheduler 仍然保持确定性与可解释性
```

---

# 172. M7 完成后必须停止

M7 完成后：

**不要继续实现 M8 Windows Integration。**

不要顺便加入：

* 系统托盘高级功能
* 开机自启
* 文件关联
* Windows 通知
* Windows Explorer integration
* Steam
* 安装包
* 自动更新
* NAS
* Cast
* 云服务

全部留给后续 milestone。

最终停止位置必须是：

```text
Task
+
Resource
+
Availability
+
Demand
+
Scheduler
+
HTTP
+
Torrent
+
Playback
```

已经形成一个：

> **可测试、可解释、可恢复、可扩展的本地统一调度核心。**
