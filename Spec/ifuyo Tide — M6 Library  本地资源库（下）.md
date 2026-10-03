# ifuyo Tide — M6 Library / 本地资源库（下）

> **拆分说明（必读）**：本文件是 M6 完整开发实施提示词的**下半部分（第九十二~一百六十节：测试体系、前端实际验收、跨阶段协同、工程纪律与最终验收）**，承接上半部分《ifuyo Tide — M6 Library  本地资源库（上）.md》（第一~九十一节：领域模型与实现）。上半部分的阶段定位、执行纪律、架构原则与安全边界对本部分**全部生效**；执行本部分前必须先阅读并完成上半部分。
# 九十二、测试：必须大量测试

M6 不能靠：

```text
“我扫了一个文件夹，看起来可以”
```

结束。

至少需要以下测试层级：

```text
Unit Tests
Integration Tests
Filesystem Tests
SQLite Tests
IPC Tests
Playback Integration Tests
Performance Tests
Security Tests
Failure Injection
```

---

# 九十三、Unit Test

至少覆盖：

### Path

```text
canonicalization
containment
separator normalization
case handling
reparse handling
```

### Classification

```text
mp4
mkv
webm
mp3
flac
wav
jpg
png
unknown
```

具体扩展名以实际产品支持矩阵为准。

### State

覆盖：

```text
Available → Missing
Missing → Available
Available → Unsupported
```

等合法/非法状态。

---

# 九十四、Scanner Tests

测试：

```text
empty folder
single file
nested folders
large tree
10k files
duplicate names
unsupported files
.part files
temporary files
permission denied
deleted file during scan
file modified during scan
folder removed during scan
```

---

# 九十五、Database Tests

测试：

```text
insert root
duplicate root
insert item
duplicate item path
update item
remove item
missing state
foreign key
migration
rollback / failed migration
```

---

# 九十六、Critical Test：Duplicate Path

例如：

```text
same root
same canonical path
```

重复扫描：

```text
must remain exactly one LibraryItem
```

---

# 九十七、Critical Test：File Missing

流程：

```text
scan
→ Available

delete via Explorer

refresh
→ Missing

restore file

refresh

→ Available
```

整个过程必须保持数据库一致。

---

# 九十八、Critical Test：Partial Scan Failure

模拟：

```text
Root
 ├── A       readable
 ├── B       readable
 └── C       access denied
```

确保：

```text
C
```

不会导致：

```text
整个 Root 中所有旧 Item
→ Missing
```

---

# 九十九、Critical Test：Download Completion

模拟：

```text
M1/M3 Task
→ Completed
→ Final File
```

验证：

```text
LibraryItem created once
```

重复通知：

```text
Completed
Completed
Completed
```

依旧：

```text
one LibraryItem
```

---

# 一百、Critical Test：Task Deletion

验证：

```text
Task deleted
```

之后：

```text
LibraryItem
```

不会意外删除。

---

# 一百零一、Critical Test：Library Remove

验证：

```text
Remove from Library
```

之后：

```text
actual file still exists
```

---

# 一百零二、Critical Test：Delete File

验证：

```text
Delete File
```

必须：

```text
explicit confirmation
↓
backend validation
↓
delete actual file
↓
remove/update LibraryItem
```

不能仅仅：

```text
DELETE /path
```

---

# 一百零三、Critical Test：Path Escape

模拟：

```text
LibraryRoot = D:\Media
Item path = D:\Media\..\Secret\a.mp4
```

必须拒绝。

模拟：

```text
junction
symbolic link
reparse point
```

导致：

```text
D:\Media → C:\Private
```

必须拒绝跨出 root。

---

# 一百零四、Critical Test：Playback Security

frontend 只能提交：

```text
item_id
```

不能提交：

```text
arbitrary file path
```

测试：

```text
fake item_id
missing item
foreign root
tampered DB path
path escape
```

全部必须安全失败。

---

# 一百零五、Critical Test：Large Files

至少测试：

```text
> 2 GiB
> 4 GiB
```

确保：

```text
size_bytes
file offset
range
SQLite integer
Rust u64
```

不会截断。

---

# 一百零六、性能测试

至少测试：

```text
1,000
10,000
50,000
100,000
```

文件。

记录：

```text
scan time
DB time
peak memory
CPU
startup impact
search latency
filter latency
sort latency
```

不要只说：

```text
“performance good”
```

必须给实际数据。

---

# 一百零七、Library 搜索性能

例如测：

```text
100k items
search “movie”
```

记录：

```text
p50
p95
```

如果无法得到正式 percentile，至少记录：

```text
cold
warm
```

并说明测试条件。

---

# 一百零八、Filesystem Race

必须考虑：

```text
scan sees file
↓
file deleted
↓
metadata read
```

以及：

```text
metadata read
↓
file modified
↓
play
```

这些情况下：

> 不能 crash，也不能把旧 metadata 当成永久事实。

---

# 一百零九、Failure Injection

至少加入 test-only injection：

```text
AfterDiscover
AfterMetadataRead
BeforeDBUpsert
AfterDBUpsert
BeforeMarkMissing
AfterScan
BeforePlaybackSession
AfterPlaybackSession
```

用于测试失败恢复。

---

# 一百一十、Property / Fuzz Style Tests

对于：

```text
path normalization
relative path
classification
query parameters
sort/filter
state transitions
```

尽可能使用：

```text
property-based testing
```

尤其：

> 任意用户提供的 path 字符串都不能绕过 root containment。

---

# 一百一十一、前端实际验收

不能只测 backend。

必须启动真实 Tide UI。

执行：

## 场景 1

```text
Add Library Folder
↓
Scan
↓
Library 出现真实文件
```

## 场景 2

```text
Search
↓
真实过滤
```

## 场景 3

```text
Sort
↓
真实排序
```

## 场景 4

```text
Play
↓
M5 Player
↓
真实播放
```

## 场景 5

```text
Explorer 删除文件
↓
Rescan
↓
Missing
```

## 场景 6

```text
Remove from Library
↓
文件依然存在
```

## 场景 7

```text
Download complete
↓
自动进入 Library
```

---

# 一百一十二、禁止 Fake UI

严禁：

```text
fake library items
fake scan count
fake file size
fake play state
fake availability
fake duration
fake metadata
```

任何展示数据必须来自：

```text
SQLite
Filesystem
Download Task
Resource
PlaybackSession
```

之一。

---

# 一百一十三、不要提前做海报墙

即使 UI 看起来比较简陋，也不要为了视觉效果引入：

```text
cover downloading
poster API
TMDB
AI tagging
```

M6 第一目标：

> **“我能可靠找到我的文件，并可靠打开它。”**

不是：

> “看起来像 Netflix。”

---

# 一百一十四、可以做的视觉增强

在不引入网络内容的前提下，可以做：

```text
Video icon
Audio icon
Image icon
file type badge
file size
duration（仅在已有 M5 能力可可靠获取时）
```

不要增加复杂视觉资产流水线。

---

# 一百一十五、Open Folder

Windows 下：

```text
Open Containing Folder
```

应该只允许 backend 打开：

```text
validated LibraryItem path
```

而不是 frontend 传：

```text
arbitrary path
```

---

# 一百一十六、文件冲突

如果出现：

```text
两项 LibraryItem
→ 指向同一 canonical path
```

必须有确定的处理。

不能让：

```text
两个 ID
```

同时控制：

```text
一个文件
```

数据库约束应优先阻止此类问题。

---

# 一百一十七、数据库与文件的一致性

Library 无法像事务数据库一样同时提交：

```text
filesystem + SQLite
```

因此必须承认：

```text
SQLite index
Filesystem
```

之间可能短暂不一致。

M6 的设计目标不是假装不存在这个问题，而是：

> **能够检测、解释并修复这种不一致。**

---

# 一百一十八、Reconciliation

建议实现：

```text
LibraryReconciler
```

或等价机制。

负责判断：

```text
DB says Available
Filesystem exists
→ Available

DB says Available
Filesystem missing
→ Missing

DB says Missing
Filesystem now exists
→ Available

DB metadata stale
Filesystem metadata changed
→ Refresh
```

不要将全部 reconciliation 逻辑塞进 Scanner。

---

# 一百一十九、不要做自动“智能修复路径”

禁止：

```text
MovieA missing
↓
全盘搜索 MovieA
↓
发现另一个
↓
自动改路径
```

因为这可能导致：

```text
同名文件误绑定
```

M6 采取：

> **确定性优先，而不是智能猜测。**

---

# 一百二十、启动顺序

推荐：

```text
Application start
      ↓
DB migration
      ↓
Load Library Roots
      ↓
Load Library Items
      ↓
Initialize services
      ↓
Frontend ready
      ↓
Background verification/scan
```

不要：

```text
scan 100k files
↓
everything ready
↓
UI appears
```

---

# 一百二十一、与 M2 Recovery 协同

如果 M2 恢复任务：

```text
Task resumes
```

Library 不应该在：

```text
.part
```

阶段显示正常文件。

只有：

```text
atomic finalization complete
```

后：

```text
LibraryItem
```

才进入正常状态。

---

# 一百二十二、与 M3 Torrent 协同

一个 Torrent 可能：

```text
1 Task
N Files
```

所以 M3 → M6 的关系必须是：

```text
Torrent Task
       ↓
Completed Torrent Files
       ↓
N LibraryItems
```

不要建立：

```text
1 Torrent = 1 LibraryItem
```

---

# 一百二十三、与 M4 Resource 协同

M6 应该验证：

```text
LibraryItem → Resource
```

然后测试：

```text
Resource.read([0, N))
Resource.availability([0, N))
```

对于完整本地文件：

```text
availability = complete
```

---

# 一百二十四、与 M5 Player 协同

至少完成：

```text
Library → Play → PlaybackSession → Stream Gateway → Player
```

验证：

```text
play
pause
seek
resume
stop
```

但不要重新实现 Player。

---

# 一百二十五、如果 M5 尚未完整

如果发现 M5 尚未达到：

```text
PlaybackSession
+
Stream Gateway
+
Player
```

不要偷偷把 M6 改写成“Player 2.0”。

应该：

```text
FACTS
→ M5 missing pieces
→ minimal compatibility bridge
→ Library implementation
```

只补完成 M6 必须依赖的最小缺口。

---

# 一百二十六、架构原则

最终应该保持：

```text
                    ┌──────────────┐
                    │    Source    │
                    └──────┬───────┘
                           ↓
                    ┌──────────────┐
                    │    Task      │
                    └──────┬───────┘
                           ↓
                    ┌──────────────┐
                    │   Resource   │
                    └──────┬───────┘
                           │
               ┌───────────┴───────────┐
               ↓                       ↓
        ┌──────────────┐        ┌──────────────┐
        │ Availability │        │   Library    │
        └──────────────┘        └──────┬───────┘
                                      ↓
                              ┌──────────────┐
                              │  Playback    │
                              └──────┬───────┘
                                     ↓
                              ┌──────────────┐
                              │    Player    │
                              └──────────────┘
```

这里最重要的是：

```text
Task
Resource
LibraryItem
PlaybackSession
```

保持清晰边界。

---

# 一百二十七、不要创建新的万能 Manager

禁止为了 M6 出现：

```text
MediaManager
ContentManager
UniversalManager
AppDataManager
MegaLibraryManager
```

如果已有：

```text
Service
Repository
Scanner
Resource
Playback
```

就继续使用它们。

---

# 一百二十八、不要把所有逻辑塞进 LibraryService

尤其禁止：

```text
LibraryService
    ├── scan
    ├── filesystem
    ├── SQLite
    ├── playback
    ├── download
    ├── player
    ├── hashing
    ├── path security
    ├── search
    └── UI state
```

应该保持：

```text
Scanner
Path Validator
Classifier
Repository
Service
PlaybackService
```

职责清晰。

---

# 一百二十九、代码质量要求

重点关注：

```text
SRP
clear ownership
dependency direction
error handling
testability
async cancellation
bounded concurrency
no global mutable state
no hidden singleton
no duplicated path logic
no duplicated classification logic
```

不要为了减少文件数量，把复杂逻辑集中成超大文件。

---

# 一百三十、Cancellation

扫描必须支持：

```text
Cancel Scan
```

取消时：

```text
当前操作停止
DB 保持一致
UI 状态恢复
```

不能留下：

```text
Scanning forever
```

也不能因为 cancel 而：

```text
把未扫描区域全部 Missing
```

---

# 一百三十一、并发扫描

默认防止：

```text
Scan Root A
Scan Root A
Scan Root A
```

三个任务同时执行。

必须明确：

```text
same-root single-flight
```

例如：

```text
AlreadyScanning
```

不同 Root 是否允许并行，可以通过实际 benchmark 决定。

---

# 一百三十二、后台扫描生命周期

必须保证：

```text
App closes
```

时不会：

```text
background task
继续访问已释放 AppState
```

要有：

```text
CancellationToken
Graceful shutdown
Join / cleanup
```

不能：

```text
spawn(async move { loop {} })
```

然后应用退出时留下 zombie task。

---

# 一百三十三、Thread / Async Safety

检查：

```text
SQLite connection ownership
filesystem traversal
background task
frontend event emission
AppState
```

不能因为 scanner 加入：

```text
global mutex
```

从而把整个 Library 变成锁竞争热点。

---

# 一百三十四、Windows Permission

模拟：

```text
Access denied
File locked
Folder locked
Read-only
File disappearing
```

UI 应该显示：

```text
Unable to access
```

而不是：

```text
Application Error
```

---

# 一百三十五、文件锁

Windows 下播放时：

```text
player / gateway
```

可能持有文件。

删除操作必须正确处理：

```text
file locked
```

不要假设：

```text
Delete always succeeds
```

---

# 一百三十六、Delete 与 Player

如果：

```text
Movie playing
```

用户执行：

```text
Delete File
```

必须定义行为。

合理方案：

```text
delete request
→ if active playback
→ reject with FileInUse
```

而不是：

```text
后台偷偷删掉
```

具体行为以 M5 session model 为准。

---

# 一百三十七、Library Item ID

必须继续使用稳定随机 ID：

```text
UUID / equivalent random ID
```

不要使用：

```text
array index
```

作为 ID。

也不要使用：

```text
filename
```

直接作为主键。

---

# 一百三十八、时间

继续保持：

```text
UTC in persistence
Local time in UI
```

例如：

```text
created_at
modified_at
last_verified_at
```

数据库统一 UTC 表示。

---

# 一百三十九、国际化

如果项目已有 i18n：

不要把：

```text
Library
Scan
Missing
Available
Playable
```

硬编码在 frontend。

使用现有翻译系统。

至少：

```text
zh
en
```

保持一致。

---

# 一百四十、Accessibility

至少保证：

```text
buttons have labels
keyboard navigation
focus
context actions
error messages
```

但不要把 M6 变成完整 Accessibility 专项。

---

# 一百四十一、Telemetry

M6 不新增：

```text
analytics
usage tracking
file statistics upload
library telemetry
```

不要把：

```text
library size
file names
paths
hash
```

上传。

---

# 一百四十二、Security Review

完成后专门检查：

## Path

```text
Traversal
Reparse
Junction
Symlink
UNC
Device paths
```

## DB

```text
Tampered path
invalid ID
foreign root
stale item
```

## IPC

```text
arbitrary path injection
arbitrary file deletion
arbitrary playback
```

## Gateway

```text
arbitrary file read
token bypass
Host bypass
Range abuse
```

## Filesystem

```text
permission
race
file replacement
TOCTOU
```

---

# 一百四十三、TOCTOU

必须认识：

```text
validate path
↓
file changes
↓
open file
```

之间存在 race。

因此：

> 路径验证不是一次完成后永久可信。

播放、删除、打开文件等敏感动作必须尽可能缩短：

```text
validation → use
```

窗口，并在使用阶段再次确认对象属性。

---

# 一百四十四、测试数据库损坏

至少做：

```text
missing table
invalid enum
invalid path
duplicate path
foreign key mismatch
```

确认：

```text
application fails safely
```

不要因为 Library DB 坏了而：

```text
panic
```

---

# 一百四十五、Migration Regression

必须测试：

```text
M0/M1 DB
→ M2 migrations
→ M3 migrations
→ M4 migrations
→ M5 migrations
→ M6 migration
```

如果项目已有测试 fixtures：

必须全部通过。

---

# 一百四十六、构建与测试

完成后至少：

```bash
cargo check --locked
cargo test --locked
```

以及项目实际 frontend 命令：

```bash
npm run build
```

或对应 package manager 命令。

如果项目有：

```text
lint
typecheck
integration test
```

全部执行。

---

# 一百四十七、Git

不要擅自：

```text
reset --hard
clean -fd
```

不要删除用户现有工作。

严格保留：

```text
Cargo.lock
package lockfile
migrations
tests
docs
```

---

# 一百四十八、文档

M6 至少新增：

```text
docs/M6-library.md
docs/M6-library-architecture.md
docs/M6-library-security.md
docs/M6-library-scan.md
docs/M6-library-test-matrix.md
docs/M6-report.md
```

具体文件名可适配现有项目。

---

# 一百四十九、Library Architecture 文档

必须解释：

```text
Task
Resource
LibraryItem
PlaybackSession
```

之间的关系。

并明确：

```text
Library != Download
Library != Resource
Library != Player
```

---

# 一百五十、Security 文档

至少记录：

```text
Root authorization
Path canonicalization
Path containment
Reparse behavior
Delete security
Playback security
Gateway security
IPC validation
TOCTOU considerations
Logging privacy
```

---

# 一百五十一、Test Matrix

生成真实矩阵：

| Area        | Case               | Expected       |
| ----------- | ------------------ | -------------- |
| Scan        | empty folder       | success        |
| Scan        | nested tree        | indexed        |
| Scan        | duplicate scan     | idempotent     |
| Scan        | permission denied  | partial        |
| Scan        | file deleted       | missing        |
| Scan        | file restored      | available      |
| DB          | migration          | success        |
| DB          | duplicate path     | rejected       |
| Path        | traversal          | rejected       |
| Path        | junction escape    | rejected       |
| Playback    | valid item         | session        |
| Playback    | missing item       | error          |
| Playback    | invalid path       | rejected       |
| Delete      | valid library file | success        |
| Delete      | path escape        | rejected       |
| Task        | completed download | indexed        |
| Torrent     | multi-file         | multiple items |
| Performance | 100k files         | measured       |

根据实际结果扩展。

---

# 一百五十二、最终验收场景

必须真实完成以下闭环。

## Scenario A — Local Library

```text
Add D:\Media
↓
Scan
↓
Movie.mp4 appears
↓
Play
↓
M5 Player
```

---

## Scenario B — Download Integration

```text
Create HTTP Task
↓
Download
↓
Completed
↓
LibraryItem created
↓
Library displays file
↓
Play
```

---

## Scenario C — Torrent Integration

```text
Create legal/open Torrent
↓
Download multi-file torrent
↓
Completed
↓
multiple LibraryItems
↓
Play selected media
```

---

## Scenario D — External Deletion

```text
Library shows Movie.mp4

Windows Explorer deletes Movie.mp4

Rescan

→ Missing
```

---

## Scenario E — Restoration

```text
Movie.mp4 restored

Rescan

→ Available
```

---

## Scenario F — Remove from Library

```text
Remove from Library

→ item disappears

actual file still exists
```

---

## Scenario G — Delete

```text
Delete File

→ explicit confirmation
→ backend validation
→ file removed
→ library index updated
```

---

# 一百五十三、M6 最终系统结构

最终应尽量形成：

```text
                 ┌─────────────────┐
                 │      Source     │
                 └────────┬────────┘
                          ↓
                 ┌─────────────────┐
                 │      Task       │
                 └────────┬────────┘
                          ↓
                 ┌─────────────────┐
                 │     Resource    │
                 └────────┬────────┘
                          │
             ┌────────────┼─────────────┐
             ↓            ↓             ↓
       Availability    Library      Scheduler
                          │
                     LibraryItem
                          │
                          ↓
                   PlaybackSession
                          │
                          ↓
                     Stream Gateway
                          │
                          ↓
                        Player
```

M6 的核心贡献是：

```text
               Local Filesystem
                       ↓
                 Library Index
                       ↓
               Stable LibraryItem
                       ↓
             Unified Playback Entry
```

---

# 一百五十四、M6 Definition of Done

只有全部满足以下条件，M6 才算完成：

### Architecture

* [ ] Library 与 Task 分离
* [ ] Library 与 Resource 分离
* [ ] Library 与 Player 分离
* [ ] Playback 使用 PlaybackSession
* [ ] Resource.read(range) 架构继续成立
* [ ] 没有新增万能 Manager

### Persistence

* [ ] LibraryRoot 持久化
* [ ] LibraryItem 持久化
* [ ] Migration 完成
* [ ] Repository 完成
* [ ] Restart 后数据存在
* [ ] Duplicate path 不会产生重复项

### Scanner

* [ ] 本地目录扫描
* [ ] nested folder
* [ ] temporary file exclusion
* [ ] unsupported classification
* [ ] missing detection
* [ ] restored detection
* [ ] partial scan handling
* [ ] cancellation
* [ ] background execution

### Download Integration

* [ ] HTTP Completed → Library
* [ ] Torrent Completed → Library
* [ ] multi-file torrent → multiple LibraryItems
* [ ] Task 删除不会误删 Library
* [ ] Library 删除不会误取消 Task

### Playback

* [ ] Library → PlaybackSession
* [ ] Library → M4 Stream Gateway
* [ ] Library → M5 Player
* [ ] 不允许 frontend 任意 path 播放
* [ ] Missing file 安全失败

### Filesystem

* [ ] path canonicalization
* [ ] traversal blocked
* [ ] reparse/junction escape blocked
* [ ] UNC behavior defined
* [ ] permissions handled
* [ ] file disappearance handled

### Search / UI

* [ ] search
* [ ] filter
* [ ] sort
* [ ] pagination / virtualization
* [ ] scan progress
* [ ] real data only
* [ ] i18n

### Delete

* [ ] Remove from Library 与 Delete File 分离
* [ ] Delete File 需要显式确认
* [ ] backend validates target
* [ ] active playback behavior defined

### Tests

* [ ] unit tests
* [ ] integration tests
* [ ] filesystem tests
* [ ] database tests
* [ ] security tests
* [ ] playback tests
* [ ] failure injection
* [ ] large-file tests
* [ ] 1k/10k/50k/100k benchmark
* [ ] locked build/test passes

### Documentation

* [ ] Library architecture
* [ ] Scanner behavior
* [ ] Security
* [ ] Test matrix
* [ ] M6 report

---

# 一百五十五、最终报告格式

完成后不要只说：

```text
M6 done.
```

必须输出：

```text
# M6 Final Report

## 1. Executive Summary

## 2. FACTS

## 3. HYPOTHESES

## 4. Existing M0–M5 State

## 5. M6 Architecture

## 6. LibraryRoot Model

## 7. LibraryItem Model

## 8. SQLite Schema / Migration

## 9. Scanner Design

## 10. Filesystem Reconciliation

## 11. Task → Library Integration

## 12. Torrent → Library Integration

## 13. Resource → Library Integration

## 14. Playback Integration

## 15. Search / Filter / Sort

## 16. Delete / Remove Semantics

## 17. Security Review

## 18. Privacy Review

## 19. Test Matrix

## 20. Failure Injection

## 21. Performance Benchmark

## 22. Changed Files

## 23. RESOLVED

## 24. CONFIRMED

## 25. OPEN

## 26. WORSENED

## 27. P0

## 28. P1

## 29. Known Limitations

## 30. M7 Readiness

## 31. Final DoD Checklist
```

---

# 一百五十六、最终必须明确的状态

报告最后必须给出：

```text
RESOLVED
CONFIRMED
OPEN
WORSENED
```

并且：

```text
P0
P1
```

只列真正的问题。

不要为了显得专业而制造大量低价值风险。

---

# 一百五十七、M7 Readiness

M6 完成后必须验证：

```text
LibraryItem
    ↓
Resource
    ↓
Availability
    ↓
PlaybackSession
```

链路稳定。

同时不要实现 M7。

M7 将继续解决：

```text
Demand
Priority
Scheduler
Bandwidth / Connection allocation
Playback-aware download priority
Torrent piece prioritization
```

M6 只需要给 M7 一个干净的基础：

```text
Library knows what user has
Resource knows what can be read
Playback knows what user requests
```

---

# 一百五十八、最重要的实现纪律

整个 M6 实现始终遵守：

> **不要因为 Library 看起来像“文件列表”，就把它实现成一个简单的 `Path[]`。**

Tide 的 Library 必须成为：

```text
Local Resource Index
```

而不是：

```text
Array<File>
```

但与此同时：

> **不要为了“未来可能成为媒体中心”而过度工程化。**

第一版最重要的闭环只有：

```text
本地目录
   ↓
可靠扫描
   ↓
SQLite Index
   ↓
LibraryItem
   ↓
搜索/筛选/排序
   ↓
PlaybackSession
   ↓
M4 Gateway
   ↓
M5 Player
```

并保证：

```text
Task
Resource
LibraryItem
PlaybackSession
```

四者边界清晰。

---

# 一百五十九、绝对禁止事项

实现过程中禁止：

```text
❌ 改写旧 migration
❌ reset 用户代码
❌ 引入云服务
❌ 上传 Library 信息
❌ 引入账号
❌ 引入 telemetry
❌ 引入资源搜索
❌ 引入盗版资源发现
❌ 引入 NAS
❌ 引入 UPnP
❌ frontend 任意文件 path
❌ 任意文件 HTTP server
❌ file:// 直接播放作为正式架构
❌ 全文件读入内存
❌ 每文件一次 IPC
❌ 每文件一次 SQLite commit
❌ 每个文件重新 SHA-256 全文件
❌ 自动猜测移动文件
❌ 自动删除重复文件
❌ Task deletion 自动删除文件
❌ Remove Library 自动删除文件
❌ 把 Torrent Task = LibraryItem
❌ 把 LibraryItem = Resource
❌ 把 Library = Player
❌ 重新实现 M4 Gateway
❌ 重新实现 M5 Player
❌ 提前实现 M7 Scheduler
❌ 创建 MegaManager
❌ fake progress
❌ fake metadata
❌ fake playability
```

---

# 一百六十、最终验收核心

最终 Tide 必须能够真实完成：

```text
             ┌───────────────┐
             │ Local Folder  │
             └───────┬───────┘
                     ↓
              Library Scanner
                     ↓
              SQLite Library
                     ↓
               LibraryItem
                     ↓
          ┌──────────┴──────────┐
          ↓                     ↓
       Search                 Play
                                ↓
                         PlaybackSession
                                ↓
                           Resource
                                ↓
                         Range Gateway
                                ↓
                             Player
```

以及：

```text
HTTP Download ─────┐
                    ↓
Torrent Download ──→ Completed File → LibraryItem
                    ↑
Local Import ──────┘
```

最终验收标准不是：

> “Library 页面看起来完成了。”

而是：

> **“Tide 已经拥有一个可靠、持久、可恢复、可检索、可播放的本地资源索引层，并且没有破坏 Task / Resource / Playback / Scheduler 的边界。”**

完成 M6 后停止。

**不要继续实现 M7。**
**不要继续实现额外媒体中心功能。**
**不要用未来需求作为理由扩大 M6。**
::

M6 和前面的路线衔接起来后，核心链条就比较完整了：

**M1 = 下载正确性 → M2 = 生命周期正确性 → M3 = Torrent → M4 = 可用性/渐进读取 → M5 = 播放器 → M6 = 本地资源库。**

这样到 **M7 Scheduler** 时，才真正有东西可以调度：用户正在看的资源、即将播放的资源、普通下载任务、Torrent piece demand，都可以统一落到 `Demand → Scheduler` 上。
