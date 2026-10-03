# ifuyo Tide — M9 Product Polish / UX / Visual Quality / Reliability（下）

> **拆分说明（必读）**：本文件是 M9 完整开发实施提示词的**下半部分（第 91~146 节：回归测试体系、性能、启动与关机体验、报告文档、实施优先级与最终验收）**，承接上半部分《ifuyo Tide — M9 Product Polish  UX  Visual Quality  Reliability（上）.md》（第 0~90 节：UX 与视觉打磨）。上半部分的阶段定位、执行纪律、架构原则与安全边界对本部分**全部生效**；执行本部分前必须先阅读并完成上半部分。
# 91. State Machine Regression

M9 不得改变已经确认的 state machine 语义。

重新跑：

```text
Created → Ready
Ready → Running
Running → Paused
Paused → Running
Running → Completed
Running → Failed
Running → Cancelled
Failed → Retry/Ready
Cancelled → Retry/Ready if explicitly supported
```

以及所有非法 transition。

---

# 92. Persistence Regression

M9 不得破坏 M2。

完整验证：

```text
create
pause
close
restart
resume
crash
recover
complete
delete
```

尤其：

```text
UI polish refactor
```

不能导致 persistence behavior 改变。

---

# 93. Torrent Regression

M9 不得破坏 M3。

至少验证：

```text
metadata
file selection
start
pause
resume
availability
completion
error
```

---

# 94. Streaming Regression

M9 不得破坏 M4/M5。

至少验证：

```text
HTTP resource
Torrent resource
local resource if supported
play
pause
seek
retry/wait
completion
```

尤其：

```text
seek while unavailable
```

---

# 95. Library Regression

M9 不得破坏 M6。

验证：

```text
task complete
library entry
play
open
remove
restart
```

---

# 96. Scheduler Regression

M9 不得改变 M7 的实际 scheduling semantics。

测试：

```text
multiple tasks
priority
playback demand
seek demand
download demand
pause
resume
resource availability
```

只是验证。

不要重新设计 Scheduler。

---

# 97. Windows Integration Regression

M9 不得破坏 M8。

验证：

```text
tray
window
notifications if present
startup behavior
default app interactions
file opening
folder opening
shortcuts
DPI
multi-monitor
```

---

# 98. Visual Regression Test

建立合理的页面截图基准。

至少考虑：

```text
Downloads empty
Downloads active
Downloads paused
Downloads failed
Downloads completed
Torrent task
Library empty
Library populated
Player playing
Player loading
Player error
Settings
Dialogs
Dark
Light if supported
100% DPI
150% DPI
```

不要求引入重量级视觉测试框架。

可以使用现有工具或轻量 snapshot/manual process。

目标是：

> 能发现明显 UI regression。

---

# 99. Interaction Regression Matrix

建立：

| Area      | Action       | Expected               | Failure           |
| --------- | ------------ | ---------------------- | ----------------- |
| Downloads | Pause        | task paused            | no transition     |
| Downloads | Resume       | task running           | stuck             |
| Downloads | Cancel       | cancelled              | still downloading |
| Torrent   | Select files | selection persisted    | mismatch          |
| Player    | Seek         | position changes       | freeze            |
| Library   | Play         | player opens           | no response       |
| Settings  | Change       | actual setting changed | UI-only           |
| Tray      | Open         | window restores        | no response       |

扩展至当前真实功能。

---

# 100. Automated Tests

M9 必须增加或补齐 UX 相关的可自动验证部分。

至少：

```text
state rendering helpers
formatting
speed formatting
ETA formatting
size formatting
time formatting
status mapping
error mapping
action availability
task sorting
filtering
i18n keys
```

不要把所有 UX 测试都变成 screenshot test。

---

# 101. Property Tests

对于纯函数继续使用：

```text
size formatter
speed formatter
duration formatter
ETA formatter
percentage formatter
task action availability
sorting
```

重点保证：

```text
no NaN
no Infinity
no negative nonsense
no overflow
no panic
```

---

# 102. Fuzz / Edge Input

至少针对：

```text
filename
URL display
task name
error message
long strings
unicode
emoji
CJK
RTL text if relevant
zero values
u64 max-ish values
```

测试 UI formatter 和 DTO serialization 不崩溃。

---

# 103. Performance Test

建立至少：

```text
1 task
10 tasks
100 tasks
500 tasks
1000 tasks
```

测试：

```text
startup
task list
progress updates
scrolling
filtering
database query
IPC
memory
CPU
```

如果发现问题：

优先解决结构性问题，而不是用：

```text
setTimeout
```

之类的表层补丁掩盖。

---

# 104. No Fake Benchmarking

不得写：

```text
Performance improved 80%
```

除非有：

```text
before
after
same scenario
same machine
measurement methodology
```

否则只写：

```text
Observed improvement
```

或：

```text
No measurable regression detected under tested conditions
```

---

# 105. Memory Leak Investigation

长时间运行时重点检查：

```text
Task subscriptions
React listeners
Event listeners
Timers
Intervals
Player sessions
Gateway sessions
Rust channels
Tokio tasks
Torrent handles
HTTP connections
```

任何创建必须对应：

```text
lifecycle
cleanup
```

---

# 106. Cleanup Audit

尤其检查：

```text
window close
player close
task delete
task cancel
app shutdown
startup recovery
error path
```

这些路径是否正确释放：

```text
listener
timer
session
file handle
network resource
memory
```

---

# 107. Shutdown Experience

退出时不要出现：

```text
卡死
白屏
“Application not responding”
```

要区分：

```text
Close UI
Terminate app
Stop active tasks
Background behavior
```

严格遵循 M8 已确定的产品语义。

不要在 M9 擅自改变：

> 用户关闭窗口时下载是否继续。

---

# 108. Background Behavior

如果当前产品定义：

```text
minimize to tray
close to tray
background download
```

必须统一。

用户应该知道：

```text
窗口关闭
≠
应用退出
```

如果两者行为不同，UI 文案必须明确。

---

# 109. Startup Performance

重点优化启动路径：

```text
process start
↓
database open
↓
restore
↓
frontend init
↓
first meaningful paint
```

不要为了追求“启动瞬间”而破坏：

```text
M2 recovery correctness
```

正确优先。

---

# 110. First Meaningful UI

启动时优先展示：

```text
可信 UI
```

而不是：

```text
假的任务
假的 progress
```

用户宁愿看到：

```text
Loading downloads…
```

也不要看到：

```text
0% → 37% → 100%
```

这种错误状态跳变。

---

# 111. Product Polish Checklist

逐页面检查：

```text
Alignment
Spacing
Typography
Contrast
Icon
Hover
Focus
Loading
Empty
Error
Success
Disabled
Long content
Localization
DPI
Keyboard
Mouse
Performance
```

每个页面必须至少完整走一遍。

---

# 112. Visual Consistency Checklist

全项目检查：

```text
same color → same meaning
same icon → same action
same radius → same hierarchy
same spacing → same rhythm
same status → same semantics
same error → same presentation
same button → same interaction
```

---

# 113. Do Not Over-polish

M9 最容易出现的风险：

> 做到最后变成“重新设计软件”。

必须控制。

如果一个问题：

```text
不影响理解
不影响操作
不影响一致性
不影响稳定性
```

且需要大规模架构改变：

则：

```text
记录为 future improvement
```

不要 M9 实现。

---

# 114. Third-party Dependency Discipline

如果为了 UI 引入第三方库：

必须先回答：

```text
为什么现有技术栈做不到？
它解决的是不是实际问题？
维护成本是什么？
bundle 增量是多少？
license 是什么？
权限是什么？
```

优先：

```text
已有依赖
原生 Web APIs
现有 design system
轻量实现
```

不要为了一个：

```text
Toast
Modal
Tooltip
Animation
```

引入巨型 UI library。

---

# 115. Bundle / Build Audit

M9 检查：

```text
frontend bundle size
dependency count
unused dependencies
duplicate dependencies
Rust binary size
debug-only dependencies
production assets
```

如果明显存在：

```text
unused
duplicate
accidental heavyweight dependency
```

可以清理。

---

# 116. Production Build

至少验证：

```bash
cargo check --locked
cargo test --locked
npm/pnpm/yarn build
Tauri production build
```

具体命令必须以当前项目实际 package manager 为准。

不要假设 package manager。

---

# 117. Clean Machine Validation

尽可能测试：

```text
fresh install
fresh launch
upgrade from previous build
existing data
existing tasks
existing downloads
```

重点：

> M9 UI refactor 不得破坏真实用户已有数据。

---

# 118. Upgrade Compatibility

检查：

```text
old DB
old settings
old task
old library
old incomplete downloads
```

确认当前 migrations/recovery 仍能正确工作。

M9 不允许随意删除字段或历史 migration。

---

# 119. Data Migration Safety

如果确实必须调整 schema：

必须：

```text
new migration
transaction where appropriate
rollback consideration
backup/compatibility consideration
old data test
```

禁止：

```text
直接改旧 migration
删除旧 migration history
```

---

# 120. Documentation Update

M9 至少更新：

```text
docs/M9-ux-audit.md
docs/M9-design-system.md
docs/M9-error-ux.md
docs/M9-performance.md
docs/M9-regression-matrix.md
docs/M9-report.md
```

如果实际项目已有类似文档，则沿用现有命名。

---

# 121. UX Audit Report

`M9-ux-audit.md` 至少记录：

```text
Screen
Issue
Severity
Evidence
Root Cause
Fix
Verification
```

不要写：

```text
这个页面不够高级。
```

这种主观结论。

应写：

```text
Primary action is visually equivalent to secondary actions.
Observed during task creation flow.
Recommended hierarchy correction.
```

---

# 122. Design System Report

记录实际项目中的：

```text
colors
spacing
typography
radii
buttons
dialogs
status styles
icons
motion
```

如果已经存在 design tokens：

做收敛，不要另起炉灶。

---

# 123. Performance Report

记录：

```text
Environment
Build
Task Count
Duration
CPU
Memory
Startup
UI responsiveness
Player performance
Known hotspots
```

必须说明测试方法。

---

# 124. Regression Report

记录：

```text
M1 HTTP
M2 Persistence
M3 Torrent
M4 Streaming
M5 Player
M6 Library
M7 Scheduler
M8 Windows
```

每项：

```text
PASS
FAIL
NOT TESTED
KNOWN LIMITATION
```

不要把：

```text
NOT TESTED
```

写成：

```text
PASS
```

---

# 125. Security Regression

虽然 M10 才是最终安全加固，但 M9 必须做基本回归：

```text
URL validation still enforced
Path validation still enforced
Gateway token still required
Host restriction still enforced
Capabilities unchanged unless justified
No sensitive logging introduced
Frontend cannot bypass backend validation
```

---

# 126. No Telemetry Creep

特别检查：

```text
new analytics
new crash reporting
new update checker
new remote request
```

如果 M9 某组件为了 UX 自动联网：

必须明确：

```text
why
where
what data
when
whether opt-in
```

不能偷偷加入。

---

# 127. No Cloud Dependency

Tide 的核心功能必须仍然能够：

```text
without ifuyo server
without account
without cloud
```

运行。

M9 不能让 UI polish 依赖远端服务。

---

# 128. Accessibility + Product Quality

至少完成：

```text
keyboard-only basic flow
high DPI
dark/light if supported
reduced motion
long filename
long error
CJK
English
```

核心流程必须可操作。

---

# 129. Manual Acceptance Flow

必须实际执行完整用户流程：

## Flow A — HTTP

```text
Launch
→ Paste URL
→ Start
→ Progress
→ Pause
→ Resume
→ Complete
→ Open
```

---

## Flow B — HTTP Crash Recovery

```text
Start
→ download partial
→ force terminate
→ restart
→ recovery
→ resume
→ complete
```

---

## Flow C — Torrent

```text
Paste magnet/torrent
→ metadata
→ select
→ start
→ progress
→ pause
→ resume
→ complete
```

---

## Flow D — Progressive Playback

```text
Resource available
→ Play
→ Buffer
→ Seek
→ Wait for availability
→ Continue
```

---

## Flow E — Error

至少模拟：

```text
network failure
HTTP failure
disk failure
missing file
invalid source
unsupported media
remote resource changed
```

---

## Flow F — Long Running

```text
Start several tasks
→ leave application running
→ interact with UI
→ open/close player
→ switch pages
→ leave idle
→ inspect resource usage
```

---

# 130. M9 Core Quality Scenarios

至少验证：

```text
No task
1 task
10 tasks
100 tasks
Failed task
Paused task
Completed task
Torrent task
Playing task
Unavailable file
Very long filename
Very long path
No network
Slow network
High DPI
Dark mode
English
Chinese
Restart
Crash recovery
```

---

# 131. P0 / P1 Definition

## P0

M9 完成前必须解决：

* 核心流程无法正常使用
* UI 显示与真实状态严重不一致
* 删除/取消等操作可能破坏用户数据
* persistence regression
* torrent regression
* streaming regression
* player regression
* backend security regression
* crash / panic in normal flow
* severe resource leak
* startup unusable
* high DPI 导致核心流程不可操作

---

## P1

应尽量解决：

* 明显 UX confusion
* 一致性问题
* error recovery 不清晰
* keyboard navigation 缺失
* visual regression
* long-text layout
* excessive IPC
* unnecessary rendering
* notification spam
* poor loading / empty state
* obvious memory / CPU inefficiency

---

# 132. M9 不追求的东西

不要为了完成度数字而追求：

```text
100% test coverage
100% pixel perfection
100% accessibility
100% browser compatibility
100% codec support
100% OS version coverage
```

重点是：

> 核心路径可靠 + UX 一致 + 产品质感提升 + 没有新的严重问题。

---

# 133. 代码质量要求

M9 依然必须：

* 保持职责边界
* 不新增 God Object
* 不新增全局 mutable singleton
* 不把 UI 逻辑塞入 DownloadEngine
* 不把数据库逻辑塞进 React component
* 不把 IPC DTO 当 domain model
* 不让 frontend 成为 backend state source of truth
* 不复制一套 state machine
* 不绕过 repository
* 不绕过 security boundary
* 不绕过 Scheduler
* 不绕过 Resource abstraction

---

# 134. Refactor 原则

允许：

```text
小型重构
职责收敛
重复代码消除
UI component extraction
formatter extraction
state selector extraction
style token extraction
event lifecycle cleanup
```

不允许：

```text
为了“整洁”
而整体重写
```

任何大型 refactor 必须先说明：

```text
为什么
影响什么
不做会怎样
如何验证没有 regression
```

---

# 135. Git / Change Discipline

每完成一个较大 M9 子任务：

```text
build
test
manual verification
```

确保提交粒度清楚。

不要：

```text
一次改 200 个文件
最后才发现无法启动
```

如果当前仓库已有提交规范，则严格遵循。

---

# 136. M9 实现优先级建议

建议优先顺序：

```text
Phase A
UX audit

Phase B
State / error / loading / empty consistency

Phase C
Task / Player / Library polish

Phase D
Design system consolidation

Phase E
Accessibility / keyboard / DPI

Phase F
performance / event / rendering cleanup

Phase G
visual regression + manual regression

Phase H
documentation + final audit
```

---

# 137. M9 实际实施策略

不要一次性大改。

建议按照：

```text
Audit
↓
Fix P0/P1
↓
Build/Test
↓
Manual Run
↓
Next Area
```

逐步推进。

推荐每次修改控制在一个逻辑域：

```text
Downloads
Player
Library
Settings
Shared UI
Windows
Performance
```

避免跨几十个模块同时变化。

---

# 138. 重要：所有数据必须是真实数据

M9 禁止：

```text
fake speed
fake progress
fake status
fake buffer
fake file
fake metadata
fake completion
fake availability
```

所有 UI 都应继续来自：

```text
actual backend state
```

或明确标记为：

```text
unknown
loading
not available
```

---

# 139. Final M9 Acceptance Criteria

M9 完成必须满足：

### 产品

```text
Tide 主界面视觉统一
核心操作路径清晰
下载 Task 易理解
Player 操作自然
Library 易理解
Settings 易使用
Error 可恢复
Loading 清晰
Empty State 完整
```

### 交互

```text
Pause/Resume/Cancel/Retry/Delete 正确
不会出现双提交
不会出现 stale state
不会出现旧事件覆盖新状态
```

### UI

```text
Typography consistent
Spacing consistent
Button hierarchy consistent
Status visuals consistent
Dark/Light correct if supported
Hover/focus/disabled correct
```

### Windows

```text
窗口行为稳定
DPI 正常
多显示器合理
托盘行为稳定（若已有）
```

### 性能

```text
Idle 安静
Active UI 流畅
Large task list 可接受
长期运行无明显 leak
```

### 安全

```text
backend validation intact
filesystem boundary intact
gateway security intact
capabilities unchanged or justified
no sensitive logging
no telemetry creep
```

### 回归

```text
M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
M6 PASS
M7 PASS
M8 PASS
```

或明确记录：

```text
KNOWN LIMITATION
```

---

# 140. M9 Definition of Done

只有满足以下全部条件，才可以声明：

```text
M9 COMPLETE
```

### A. UX

* 核心用户流程已经实际走查
* P0/P1 UX 问题已经处理
* Loading / Empty / Error / Success 完整
* Task actions 与真实 state 一致
* Player UX 清晰
* Library UX 清晰
* Settings UX 清晰

### B. Visual

* Design tokens 已收敛
* typography 一致
* spacing 一致
* buttons 一致
* icons 一致
* dialogs 一致
* status visuals 一致

### C. Accessibility

* keyboard 基础流程可用
* focus 可见
* high DPI 可用
* reduced motion 已考虑
* 长文本不会破坏布局

### D. Reliability

* state race audit 完成
* frontend/backend state 一致
* shutdown/recovery regression 通过
* normal business path 无 panic
* 长时间运行无明显 leak

### E. Performance

* idle CPU/RAM 已观察
* task update 性能已观察
* large task list 已观察
* player 性能已观察
* 无明显 busy loop / render storm / IPC flood

### F. Regression

* M1–M8 核心流程已回归
* SQLite/migration 没有破坏
* `.part` / recovery 没有破坏
* Torrent 没有破坏
* progressive playback 没有破坏
* Player 没有破坏
* Library 没有破坏
* Scheduler 没有破坏
* Windows integration 没有破坏

### G. Build

至少：

```bash
cargo check --locked
cargo test --locked
```

以及当前前端和 Tauri 的生产构建。

所有命令必须使用当前项目实际工具链，不得假设 package manager。

### H. Documentation

完成：

```text
M9 UX Audit
M9 Design System
M9 Error UX
M9 Performance
M9 Regression Matrix
M9 Final Report
```

---

# 141. 最终报告格式

完成 M9 后必须生成：

```text
## 1. Executive Summary

## 2. FACTS

## 3. CURRENT M0-M8 STATE

## 4. M9 OBJECTIVES

## 5. UX AUDIT

## 6. DESIGN SYSTEM CHANGES

## 7. DOWNLOAD UX

## 8. TORRENT UX

## 9. PLAYER UX

## 10. LIBRARY UX

## 11. SETTINGS UX

## 12. ERROR / LOADING / EMPTY STATES

## 13. ACCESSIBILITY

## 14. WINDOWS / DPI

## 15. PERFORMANCE

## 16. LONG-RUN STABILITY

## 17. RACE CONDITION / STATE AUDIT

## 18. SECURITY REGRESSION

## 19. TEST RESULTS

## 20. MANUAL ACCEPTANCE

## 21. REGRESSION MATRIX

## 22. CHANGED FILES

## 23. DEPENDENCY CHANGES

## 24. FACT / HYPOTHESIS / INFERENCE

## 25. RESOLVED

## 26. CONFIRMED

## 27. OPEN

## 28. WORSENED

## 29. P0

## 30. P1

## 31. KNOWN LIMITATIONS

## 32. M10 READINESS

## 33. M9 DEFINITION OF DONE
```

---

# 142. 最终报告必须特别回答的问题

1. Tide 当前是否已经具有稳定、一致的核心 UX？

2. 是否存在 UI 状态与 backend 真实状态不一致？

3. 是否存在明显的 race condition？

4. 是否存在 stale progress / stale event？

5. 是否存在长时间运行的 memory leak / task leak？

6. 是否存在大任务列表导致的 UI 卡顿？

7. 是否存在不必要的 polling / IPC / rendering？

8. HTTP 下载是否仍然稳定？

9. Torrent 是否仍然稳定？

10. Progressive streaming 是否仍然稳定？

11. Player 是否仍然稳定？

12. Library 是否仍然稳定？

13. Scheduler 是否仍然稳定？

14. Windows integration 是否仍然稳定？

15. High DPI 是否正常？

16. Dark/Light theme 是否正常？

17. Keyboard operation 是否可用？

18. 错误是否都有合理 recovery path？

19. 删除操作是否仍然不会误删用户数据？

20. 是否产生了新的安全风险？

21. 是否产生了新的网络请求？

22. 是否产生了新的 telemetry？

23. 是否引入了新的第三方依赖？

24. 是否存在可以推迟到未来版本的 polish？

25. Tide 是否已经进入 M10 的 Release Hardening 状态？

---

# 143. 最终禁止越界

完成上述工作后：

**不要继续实现 M10。**

尤其不要自动开始：

```text
security hardening
signing
installer hardening
update system
release infrastructure
code signing
certificate
MSIX
installer polish
crash reporting
production deployment
Steam release
```

这些属于后续阶段。

---

# 144. M9 的核心判断标准

最终不要用：

```text
“我们改了多少 UI 文件”
```

判断 M9。

而要用：

```text
用户是否更容易理解 Tide？
用户是否更少迷惑？
用户是否更少等待时产生误解？
错误是否更容易恢复？
核心操作是否更快？
UI 是否更一致？
长期运行是否更稳定？
旧功能是否完全没有回归？
```

最终把 Tide 从：

```text
Engineering Complete
```

推进到：

```text
Product Quality Ready
```

但不要把它误判成：

```text
Release Ready
```

因为：

```text
M9 = Product Polish
M10 = Security & Release Hardening
M11 = Release Candidate
M12 = 1.0
```

---

# 145. 最终工作循环

整个 M9 始终围绕：

```text
Audit
  ↓
Identify P0/P1
  ↓
Implement
  ↓
Build
  ↓
Automated Test
  ↓
Manual Test
  ↓
Regression
  ↓
Performance
  ↓
UX Re-check
  ↓
Document
  ↓
Next Area
```

而不是：

```text
Redesign everything
  ↓
hope nothing broke
```

---

# 146. 最终产品目标

M9 完成后的 Tide 应该呈现出这样的产品状态：

```text
Simple to start
        +
Easy to understand
        +
Honest about state
        +
Fast enough
        +
Quiet when idle
        +
Recoverable when things go wrong
        +
Consistent everywhere
        +
Native enough for Windows
        +
No unnecessary cloud dependency
        +
No unnecessary tracking
        +
No unnecessary complexity
```

核心原则：

> **Tide 不需要让用户感到“功能很多”。**
>
> **Tide 应该让用户感到“这个软件很顺手”。**
>
> **M9 的任务，就是把前八个里程碑积累下来的工程能力，转换成这种顺手感。**
