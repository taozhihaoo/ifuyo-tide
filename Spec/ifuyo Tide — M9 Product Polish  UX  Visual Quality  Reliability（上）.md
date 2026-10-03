# ifuyo Tide — M9 Product Polish / UX / Visual Quality / Reliability（上）

> **拆分说明（必读）**：本文件是 M9 完整开发实施提示词的**上半部分（第 0~90 节：UX 与视觉打磨 —— UX Audit、视觉设计体系、下载/播放器/Library/Settings 各界面打磨、错误与文案、i18n、状态渲染与 Panic Safety）**。回归测试体系、性能、启动与关机体验、报告文档与最终验收位于下半部分《ifuyo Tide — M9 Product Polish  UX  Visual Quality  Reliability（下）.md》。两部分必须配合使用：先完成上半部分，再执行下半部分；只完成上半部分不构成 M9 完成。

## 完整开发实施提示词

> **阶段：M9**
>
> **阶段名称：Product Polish / UX / Visual Quality / Reliability**
>
> **前置阶段：M0 → M8**
>
> **后续阶段：M10 Security & Release Hardening**
>
> **本阶段性质：产品化打磨阶段，不新增核心能力**
>
> **核心目标：让 Tide 从“功能已经完整的工程产品”进入“真正可长期使用、操作自然、反馈清晰、视觉统一、异常不吓人、性能观感稳定”的产品状态。**

---

# 0. 最高优先级原则

本阶段不是重新设计 Tide。

也不是借着“打磨 UX”继续增加功能。

M9 的任务是：

```text
已有功能
    ↓
交互一致性
    ↓
视觉一致性
    ↓
状态表达清晰
    ↓
反馈及时
    ↓
异常体验可理解
    ↓
操作路径缩短
    ↓
性能观感稳定
    ↓
长期使用舒适
```

必须严格遵守：

> **M9 = Polish，不是 Feature Expansion。**

除非某个问题已经严重影响核心使用路径，否则不得借 M9 引入新的业务系统、协议、后端服务或大型第三方框架。

---

# 1. 开始之前：必须先审计，不要直接修改

在任何代码修改前，完整检查当前仓库。

必须检查：

* M0 当前状态
* M1 当前状态
* M2 当前状态
* M3 当前状态
* M4 当前状态
* M5 当前状态
* M6 当前状态
* M7 当前状态
* M8 当前状态
* 当前 frontend
* 当前 Rust backend
* 当前 IPC
* 当前数据库 schema
* 当前 migrations
* 当前错误体系
* 当前日志体系
* 当前主题/样式体系
* 当前 Windows integration
* 当前下载任务 UI
* 当前 Torrent UI
* 当前 Player UI
* 当前 Library UI
* 当前 Settings UI
* 当前系统托盘/窗口行为
* 当前快捷键
* 当前通知
* 当前 loading / empty / error / success 状态
* 当前性能与渲染策略
* 当前测试

尤其不要假设之前的里程碑完全按设计实现。

必须以当前代码为准。

首先输出：

```text
FACTS
HYPOTHESES
CURRENT M0-M8 STATE
KNOWN IMPLEMENTATION GAPS
CURRENT UX STATE
CURRENT VISUAL STATE
CURRENT PERFORMANCE STATE
CURRENT RELIABILITY STATE
M9 SCOPE
M9 NON-GOALS
M9 IMPLEMENTATION PLAN
RISKS
```

其中：

### FACTS

只写从代码、配置、测试、实际运行结果中确认的事实。

### HYPOTHESES

当前尚未验证的问题。

### IMPLEMENTATION PLAN

必须区分：

```text
P0
P1
P2
```

但真正的质量阻塞问题只使用 P0 / P1。

P2 只做非常轻量的可选 polish，不允许扩大范围。

---

# 2. M9 核心目标

M9 完成以后，用户应该感觉：

> “Tide 已经不像一个开发中的工具，而像一个完成度很高的 Windows 原生桌面应用。”

重点改善：

```text
视觉
交互
反馈
状态
错误
动画
过渡
空状态
加载状态
列表
播放器
下载任务
Library
Settings
窗口体验
键盘操作
鼠标操作
高 DPI
主题
性能观感
长期使用体验
```

但必须保持：

```text
核心架构不重写
资源模型不重写
Scheduler 不重写
Download Engine 不重写
Torrent Engine 不重写
Playback Engine 不重写
SQLite 不重构
IPC 协议不无意义重构
```

---

# 3. M9 明确禁止事项

除非当前代码存在无法运行或安全阻塞问题，否则禁止因为 M9 而新增：

* 新下载协议
* 新 Torrent 协议
* 新资源发现系统
* 搜索引擎
* Tracker 搜索
* 资源索引
* 推荐系统
* 社区
* 用户账号
* 云服务
* 云同步
* 在线媒体库
* P2P 社交功能
* UPnP
* NAS 新协议
* Chromecast
* DLNA
* VPN
* 代理平台
* 匿名下载
* 远程中继
* 服务器加速
* 会员体系
* 广告
* telemetry
* 强制 analytics
* DRM 绕过
* paywall bypass
* 字幕云服务
* AI 推荐
* AI 播放摘要
* AI 内容生成
* Steam 功能扩展
* 大型插件框架
* 大型 UI framework 替换
* 重写整个前端
* 重写整个数据库
* 重写核心 Scheduler
* 重写 librqbit integration
* 重写 Player architecture
* 提前实现 M10/M11/M12 内容

尤其禁止：

> 为了“看起来更完整”，偷偷加入新业务能力。

---

# 4. M9 产品质量模型

整个阶段统一按照以下模型审视：

```text
Discoverability
    ↓
Understandability
    ↓
Feedback
    ↓
Recoverability
    ↓
Consistency
    ↓
Efficiency
    ↓
Delight
```

分别意味着：

### Discoverability

用户是否知道某个功能存在？

### Understandability

用户是否知道当前发生了什么？

### Feedback

点击之后是否马上看到反馈？

### Recoverability

出错之后是否知道怎么恢复？

### Consistency

相同概念是否始终使用相同方式表达？

### Efficiency

常用操作是否足够快？

### Delight

视觉、动效、细节是否有品质感？

---

# 5. 建立 M9 UX Audit

首先对 Tide 的主要使用路径逐条走查。

至少包括：

```text
首次启动
打开主窗口
创建 HTTP 下载
暂停
恢复
取消
失败
重试
删除
打开文件
开始 Torrent
Torrent 下载
Torrent 暂停
Torrent 恢复
Torrent 文件选择
开始播放
暂停播放
拖动进度
Seek
播放失败
切换任务
打开 Library
删除/移除 Library 条目
打开 Settings
修改设置
改变下载目录
退出程序
重新启动
后台运行
窗口最小化
系统托盘操作
多任务同时执行
磁盘不足
网络失败
无网络
资源失效
文件缺失
播放器不支持
```

对每条路径记录：

```text
User Intent
User Action
System Response
Visual Feedback
Error Feedback
Recovery Path
Potential Confusion
Interaction Cost
```

---

# 6. 首次启动体验

M9 必须优化首次启动。

但：

> **不做强制账号注册。**
>
> **不做复杂 onboarding。**
>
> **不做产品宣传页。**

首次启动必须让用户在极短路径内理解：

```text
Tide 是什么
我可以做什么
下载文件在哪里
任务在哪里
怎么开始
```

建议：

```text
首次启动
    ↓
简洁欢迎页
    ↓
默认下载目录确认
    ↓
进入主界面
```

如果已有设置，则不要重复出现。

必须避免：

* 大面积宣传
* 强制教程
* 连续弹窗
* 要求登录
* 请求无关权限
* 请求 analytics
* 启动即联网
* 复杂配置向导

---

# 7. 主界面信息架构

检查当前 UI 是否存在：

```text
Downloads
Library
Settings
```

或者当前实际设计对应的 equivalent。

不要为了理论上的 IA 重做整个产品。

重点检查：

* 导航层级
* 当前页面指示
* 页面标题
* secondary information
* 操作按钮
* context menu
* task status
* progress
* empty states
* error states

必须做到：

> 用户不需要猜“现在自己在哪”。

---

# 8. 下载任务 UI 打磨

这是 Tide 最核心的 UI 之一。

每个 Task 必须能清晰表达：

```text
Name
Type
State
Progress
Downloaded / Total
Speed
ETA
Source
Destination
Relevant Actions
```

但不要让所有字段同时争夺注意力。

建立信息优先级：

```text
Primary:
Name
State
Progress

Secondary:
Downloaded / Total
Speed
ETA

Tertiary:
Source
Destination
Metadata
```

---

# 9. Task State 的视觉表达必须统一

统一定义：

```text
Created
Ready
Running
Paused
Completed
Failed
Cancelled
```

不要让不同页面使用不同词语表达同一状态。

例如：

```text
Running
Downloading
Active
In Progress
```

如果实际上是同一状态，不要同时存在四种用户-facing terminology。

状态必须拥有明确视觉层级。

尤其：

### Paused

用户必须知道：

> 是我暂停了。

而不是：

> 好像坏了。

### Failed

用户必须知道：

> 这是失败，不是暂停。

并且必须能够快速找到：

```text
Retry
Open Details
```

### Completed

明确表达：

> 已完成。

并提供合理的后续操作：

```text
Open
Show in Folder
Play
```

具体 action 取决于当前资源类型。

---

# 10. Progress Bar 不能欺骗用户

M9 禁止：

```text
假的百分比
假的速度
假的 ETA
假的 buffering
```

所有视觉数字都必须来自真实状态。

尤其：

```text
Torrent Availability
Download Progress
Playback Buffer
```

不要混成一个数字。

例如：

```text
Downloaded: 42%
Available for playback: 78%
Playback position: 31%
```

实际产品不一定需要全部展示，但内部概念必须区分。

---

# 11. Speed Display

统一速度显示规则。

考虑：

```text
B/s
KB/s
MB/s
GB/s
```

并统一：

* 小数位
* rounding
* 采样时间
* smoothing
* zero-state
* startup state

不要出现：

```text
0.000 MB/s
```

这种没有产品价值的显示。

应该根据当前状态合理表现：

```text
— 
Waiting
Connecting
128 KB/s
3.4 MB/s
```

具体文案以实际设计体系为准。

---

# 12. ETA Display

ETA 只在有足够可靠数据时显示。

禁止：

```text
NaN
Infinity
-12 sec
0 sec
99999 hours
```

必须处理：

* unknown total
* zero speed
* unstable speed
* paused
* waiting
* retrying
* nearly completed

例如：

```text
Calculating…
—
Paused
Waiting…
```

---

# 13. Task Actions

检查：

```text
Pause
Resume
Cancel
Retry
Delete
Open
Show in Folder
Play
```

必须遵循状态约束。

例如：

```text
Running:
Pause / Cancel

Paused:
Resume / Cancel

Failed:
Retry / Delete

Completed:
Open / Show in Folder / Play / Delete

Cancelled:
Delete / Retry
```

具体能力根据现有产品实际实现。

绝对禁止出现：

> UI 显示了一个当前实际上不能执行的 Action。

前端只是 UI。

最终状态约束必须仍然由 backend enforcement。

---

# 14. 二次确认策略

不要对所有操作都弹确认框。

确认仅用于：

```text
高破坏性
高不可逆性
用户意图高度不明确
```

例如删除：

区分：

```text
Remove task
Delete downloaded file
```

这两个概念不能混淆。

推荐明确表达：

```text
Remove from Tide
Delete downloaded file
```

如果当前功能支持两者，必须在 UI 上明确区分。

---

# 15. 错误体验重构

M9 非常重要的一项：

> **技术错误不应该直接等于用户错误。**

例如 backend：

```text
HttpStatus(416)
ConnectionReset
PermissionDenied
DiskFull
FileNotFound
ChecksumMismatch
TorrentUnavailable
PlaybackUnsupported
```

不能直接把 Rust/debug 错误文本扔到用户面前。

建立：

```text
Technical Error
        ↓
Domain Error
        ↓
User-facing Explanation
        ↓
Recovery Action
```

例如：

```text
Download failed

The server stopped accepting the requested range.

[Retry]
[Start over]
```

而不是：

```text
Reqwest error:
builder error ...
```

---

# 16. 错误信息原则

用户可见错误至少回答：

```text
发生了什么？
为什么可能发生？
我现在能做什么？
```

但不要写毫无证据的原因。

禁止：

> “服务器拒绝你，因为你不是会员。”

如果系统实际上并没有这个证据。

应该：

> “服务器返回了 403 Forbidden。”

然后提供：

```text
Retry
Open details
```

---

# 17. Technical Details

高级技术信息可以保留，但应折叠。

例如：

```text
Something went wrong.

[Retry]

▸ Technical details
    HTTP 403
    Request ID: ...
```

这样既方便普通用户，也保留工程诊断能力。

---

# 18. Empty State

为每个主要页面设计真正有意义的 empty state。

至少检查：

```text
Downloads empty
Library empty
No selected task
No playable media
Search/filter no result（若已有搜索/过滤）
```

Empty state 必须：

```text
Explain
Orient
Act
```

不要出现：

```text
No data.
```

这种毫无帮助的信息。

例如：

```text
No downloads yet

Paste a link to start your first download.
```

具体文案以 Tide 产品语言体系为准。

---

# 19. Loading State

所有可能出现延迟的操作必须检查：

```text
Startup
Database loading
Task restoration
Torrent metadata
File scanning
Player initialization
Resource opening
Settings loading
```

不能：

```text
白屏
卡死
按钮无响应
```

需要区分：

```text
Loading
Waiting
Connecting
Recovering
Initializing
```

避免所有状态都使用一个无限 spinner。

---

# 20. Skeleton / Placeholder

只在真正有价值的位置使用 skeleton。

禁止：

> 到处塞 skeleton 只是为了看起来像成熟互联网产品。

Tide 是 desktop utility。

应优先：

```text
稳定布局
轻量 loading
明确状态
```

而不是复杂动画。

---

# 21. Motion Design

M9 可以加入轻量 motion。

但是 Tide 的产品性格应该是：

```text
Fast
Quiet
Precise
Calm
```

而不是：

```text
Flashy
Bouncy
Game-like
Distracting
```

推荐：

* 页面切换淡入
* task state transition
* progress transition
* player panel transition
* dialog appearance
* hover feedback
* toast transition
* sidebar transition

必须：

```text
短
轻
可关闭
不阻塞
```

---

# 22. Respect Reduced Motion

如果用户系统启用了：

```text
prefers-reduced-motion
```

或 Windows 对应 accessibility preference，则降低或关闭非必要动画。

不要把动画作为信息唯一载体。

---

# 23. Visual Design System

建立或整理当前已有的设计变量。

至少统一：

```text
Colors
Typography
Spacing
Radius
Border
Shadow
Icon size
Button height
Input height
Card density
Transition duration
```

原则：

> 不要出现 17 种灰色、11 种圆角、9 种按钮高度。

优先通过 tokens / CSS variables / theme system 统一。

例如逻辑上：

```text
--color-bg
--color-surface
--color-text-primary
--color-text-secondary
--color-text-muted
--color-border
--color-accent
--color-danger

--space-1
--space-2
...

--radius-sm
--radius-md
--radius-lg
```

实际命名适应现有项目。

---

# 24. Theme

若当前已有 Light / Dark：

M9 必须检查：

```text
Downloads
Library
Player
Settings
Dialogs
Toasts
Menus
Progress
Error
Disabled
Selected
Hover
Focus
```

确保没有：

* 某组件在 dark mode 下看不见
* 某文字对比度过低
* hover 状态丢失
* disabled 状态与 normal 无区别
* player overlay 不可读

如果当前没有完整主题系统，不要为了 M9 强行增加复杂主题引擎。

至少确保当前主题完整、一致。

---

# 25. Accessibility

M9 必须进行基础 accessibility audit。

检查：

```text
Keyboard navigation
Focus visibility
Tab order
Button semantics
Input labels
Tooltip alternatives
Contrast
Disabled state
Screen reader-friendly semantics（能力范围内）
Reduced motion
Window scaling
```

禁止：

> 用颜色作为唯一状态表达。

例如：

```text
红色 = Failed
```

必须同时有：

```text
icon + text
```

---

# 26. Keyboard UX

至少检查当前已有功能是否支持合理键盘操作。

重点：

```text
Tab
Shift+Tab
Enter
Escape
Space
Arrow keys
Delete
Ctrl/Cmd-compatible abstractions where appropriate
```

Windows 下优先考虑合理的：

```text
Enter = activate
Escape = close/cancel
Space = play/pause where appropriate
Delete = remove where appropriate
```

具体快捷键必须与当前产品语义一致。

不要为了“快捷键很多”制造冲突。

---

# 27. Context Menu

Task / Library / Player 等支持 context menu 的位置必须统一。

检查：

```text
Open
Play
Pause
Resume
Retry
Show in Folder
Remove
Delete
Copy source information
```

只显示当前状态合法的操作。

---

# 28. Toast / Notification

如果已有 Toast 系统，M9 需要统一：

```text
success
info
warning
error
```

Toast 必须：

* 不遮挡核心操作
* 不无限积累
* 生命周期合理
* 能表达关键结果
* 不代替永久状态

例如：

```text
Download completed
```

适合 Toast。

但：

```text
Task is Failed
```

不能只通过 Toast 表达，Task 本身必须持续显示 Failed。

---

# 29. Window Behavior

结合 M8 已有 Windows integration，M9 进行产品体验打磨。

检查：

```text
Open
Close
Minimize
Maximize
Restore
Resize
Startup position
Remember size
Remember position
Multiple monitor
DPI scaling
```

原则：

> 窗口行为必须可预测。

避免：

* 启动后随机出现在奇怪位置
* 记住了已经不存在的显示器
* DPI 切换后 UI 变形
* resize 后布局溢出
* 最小尺寸过小导致不可用

---

# 30. High DPI

重点测试：

```text
100%
125%
150%
175%
200%
```

以及：

```text
1920×1080
2560×1440
3840×2160
```

检查：

* 字体
* icons
* buttons
* progress bar
* sidebar
* player controls
* dialogs
* context menus

不得出现：

```text
文字裁切
图标模糊
按钮重叠
布局溢出
```

---

# 31. 多窗口/对话框一致性

所有 dialog 检查：

```text
Title
Description
Primary button
Secondary button
Close behavior
Escape behavior
Focus
Loading
Error
```

统一。

禁止某个 Dialog：

```text
Enter = Cancel
```

而其他 Dialog：

```text
Enter = Confirm
```

除非语义明确要求不同。

---

# 32. Player UX Polish

M5 已经完成 Player 的功能后，M9 不重新实现播放器。

重点打磨：

```text
Play
Pause
Seek
Volume
Fullscreen
Progress
Loading
Buffering
Playback error
Unsupported format
Ended
```

尤其检查：

### Loading

用户知道：

```text
播放器正在准备
```

还是：

```text
卡住了
```

### Buffering

必须能区分：

```text
播放中
缓冲中
资源不可用
网络等待
```

### Seek

用户拖动后要有明确反馈。

不要让：

```text
seek → UI 卡住 → 无反馈 → 数秒后跳转
```

---

# 33. Player Error UX

播放器失败必须显示可理解信息。

例如：

```text
This file cannot be played.

The current WebView2 media stack does not support this format.

[Open in default app]
[Show details]
```

是否提供外部播放器取决于当前 M8/M5 已实现能力。

绝不在 M9 偷偷加入完整 native player。

---

# 34. Progressive Playback UX

已有 progressive playback 的情况下，视觉上必须区分：

```text
Downloaded
Available
Buffered
Played
```

但不要求全部显示给用户。

重点：

> 用户进行 Seek 时，系统必须让“等待数据”和“播放器卡顿”在体验上尽可能可理解。

不要伪装成完整本地文件已经存在。

---

# 35. Torrent UX Polish

M9 不新增 Torrent 核心能力。

但可以整理现有 UX：

```text
Metadata loading
Connecting
Downloading
Paused
Seeding
Completed
Error
```

文件选择界面必须清晰表达：

```text
file
size
selected
download priority / relevance（如果已有）
```

避免：

> 一张表塞几十个技术字段。

---

# 36. Availability 可视化

如果 M4/M7 已经存在 Availability 数据：

不要默认暴露所有技术细节。

用户真正关心的可能是：

```text
Can play now
Partially available
Waiting for data
Completed
```

高级界面再显示更详细信息。

---

# 37. Library UX Polish

Library 必须回答：

```text
我有哪些内容？
它在哪里？
能不能播放？
什么时候添加？
是什么类型？
```

检查：

```text
Card
List
Thumbnail
Title
Subtitle
Duration
Status
Action
```

如果当前已存在缩略图/媒体 metadata，则进行一致性优化。

不要因为 M9 开始开发全新的 metadata scraper。

---

# 38. Library 文件失效

必须考虑：

```text
file deleted externally
file moved externally
drive disconnected
path unavailable
```

UI 不应直接：

```text
Broken
```

而应该表达：

```text
File unavailable
```

并在可能情况下提供：

```text
Locate file
Remove entry
```

具体能力只允许使用已有架构能够安全支持的实现。

---

# 39. Settings UX

Settings 必须避免“工程师配置面板”。

检查：

```text
Download location
Concurrency
Bandwidth
Theme
Startup behavior
Playback
Torrent behavior
Storage
```

按照实际已有设置组织。

每个 setting 必须明确：

```text
名称
当前值
作用
限制
立即生效/重启生效
```

---

# 40. Destructive Settings

对于会产生明显副作用的设置：

例如：

```text
clear data
reset settings
delete history
change storage
```

必须明确影响范围。

绝对避免模糊：

```text
Delete
```

这种按钮。

应该让用户知道：

```text
仅删除 Tide 数据
```

还是：

```text
删除真实下载文件
```

---

# 41. Search / Filter

如果 M6/M7 已经存在搜索或过滤：

M9 优化：

```text
clear
empty
no result
keyboard
performance
debounce
```

不要在 M9 增加新的复杂查询语言。

---

# 42. Large Dataset UX

测试：

```text
100 tasks
500 tasks
1000 tasks
```

观察：

* scrolling
* filtering
* rendering
* task updates
* memory
* CPU
* IPC event pressure

不能因为任务数量增加而：

```text
整个列表频繁重渲染
每秒数百次 IPC
UI 明显卡顿
```

---

# 43. Event / IPC Polish

检查 frontend 是否：

```text
subscribe
receive
transform
render
```

造成过度事件流。

尤其：

```text
progress
speed
availability
buffer
torrent state
```

不要直接：

```text
每个 byte / piece → IPC → React render
```

建立合理的 throttling / coalescing。

具体阈值必须基于当前实现和实际性能测试。

---

# 44. State Rendering Architecture

重点避免：

```text
Backend state
    ↓
IPC DTO
    ↓
UI-local duplicate state
    ↓
another derived state
    ↓
manual patch
```

最终出现状态不一致。

M9 应清理明显的：

```text
stale state
race condition
double update
optimistic update mismatch
```

尤其是：

```text
Pause
Resume
Cancel
Retry
Delete
```

这些操作后的 UI 状态。

---

# 45. Optimistic UI

只在操作确定安全且容易回滚的场景使用。

如果 backend 操作可能失败：

不要立即把 UI 标记成最终状态然后永不纠正。

原则：

```text
User intent
    ↓
Backend command
    ↓
Backend state transition
    ↓
Event / response
    ↓
UI projection
```

优先真实状态而不是假状态。

---

# 46. Race Condition Audit

重点检查：

```text
Pause → Resume
Pause → Cancel
Resume → Cancel
Retry → Delete
Delete → progress event
Completion → Delete
Player open → task delete
Task recovery → frontend initialization
Window close → background task
```

必须确保旧事件不会覆盖新状态。

例如：

```text
Task #A
running
↓
cancelled
↓
late progress event
↓
不能再次显示 running
```

---

# 47. Long-running Stability

M9 必须进行长时间运行测试。

至少设计：

```text
1 hour
4 hours
8 hours
overnight
```

根据现实条件完成尽可能长的测试。

观察：

```text
RAM
CPU
handles
threads
IPC event count
logs
DB growth
render performance
download stability
player stability
```

重点寻找：

* memory leak
* task leak
* session leak
* timer leak
* event subscription leak
* abandoned gateway/session（若已有）
* runaway logs
* runaway polling

---

# 48. Performance Budget

不要只说“感觉不卡”。

建立实际指标。

至少观察：

```text
Idle CPU
Idle RAM
UI active CPU
UI active RAM
Task update CPU
Large task list memory
Player CPU
Database query latency
Startup time
Resume time
```

对于 Desktop utility：

> Idle 状态应该尽量安静。

尤其不要为了 UI 动画/刷新：

```text
持续 60 FPS
```

如果没有必要。

---

# 49. Idle Rendering

如果当前 UI 没有变化：

应尽可能：

```text
停止无意义动画
降低 polling
减少 render
减少 IPC
减少 timer
```

原则：

```text
Active when needed
Quiet when idle
```

Tide 是工具，而不是游戏。

---

# 50. Resource Usage

M9 不要过早优化到极端程度。

但必须消除明显错误：

```text
高频 polling
busy loop
不必要 JSON serialization
全列表频繁 re-render
重复读取数据库
大文件进入内存
无界缓存
无界日志
```

---

# 51. Crash UX

如果程序上一次崩溃后重新启动：

不要给用户暴露：

```text
Rust panic
stack trace
thread 'tokio-runtime-worker' panicked
```

应使用已有 M2 recovery 体系：

```text
Recovering downloads…
```

如果存在无法恢复的任务：

```text
Some tasks could not be resumed.

[Review]
```

技术详情进入高级区域。

---

# 52. Restart / Recovery UX

结合 M2：

启动：

```text
Database restore
    ↓
Task reconciliation
    ↓
Recovery decisions
    ↓
UI ready
```

期间不要：

```text
UI 显示全部任务为 Completed
```

然后几秒后突然变回 Running。

应该尽量做到：

```text
Loading / Restoring
```

然后一次性进入可信状态。

---

# 53. Error Persistence

失败信息要避免无限增长。

例如：

```text
last_error
error_code
error_category
error_at
```

详细日志不应该无限写数据库。

M9 检查是否已经出现：

```text
error text duplicated into every task update
```

这种问题。

---

# 54. Logging Polish

日志分级：

```text
trace
debug
info
warn
error
```

生产环境默认不要过度 verbose。

尤其不要记录：

```text
完整 URL
Magnet
InfoHash
Peer IP
完整路径
token
secret
用户历史
```

即使只是 debug，也要考虑脱敏。

M9 不等于 M10，但不得为了调试 UI 而破坏 M10 的安全边界。

---

# 55. Error Copy / Product Language

建立统一的产品语言风格。

建议：

```text
短
明确
中性
可操作
不吓人
不推测
```

避免：

```text
Oops!!!
Something magical went wrong!!!
```

也避免：

```text
Network Error.
```

这种过于技术化但没有帮助的信息。

---

# 56. Terminology Audit

全项目扫描术语。

例如统一：

```text
Download
Task
Resource
File
Library
Playback
Source
Torrent
Completed
Paused
Failed
Cancelled
```

确保：

frontend / backend / logs / IPC / docs

尽量使用一致概念。

内部类型名和用户-facing 文案可以不同，但不能造成语义冲突。

---

# 57. i18n Audit

如果当前已经支持：

```text
zh-CN
en-US
```

M9 必须检查：

* 所有主要页面
* 按钮
* 状态
* 错误
* dialog
* tooltip
* settings
* player

禁止：

```text
硬编码中文
硬编码英文
```

散落于组件内部。

尤其注意：

```text
pluralization
数字
文件大小
时间
日期
速度
百分比
```

具体 i18n 框架不必更换。

---

# 58. Long Text / Localization Layout

测试英文往往比中文更长的情况。

例如：

```text
The downloaded file is currently unavailable
```

不得导致：

```text
overflow
button clipping
dialog overflow
```

同时测试极长：

```text
filename
resource name
error message
path
URL display
```

必须允许：

```text
truncate
wrap
tooltip
details
```

而不是破坏整体布局。

---

# 59. Filename / Path Display

Windows 下长路径尤其需要测试。

界面中：

```text
C:\very\very\very\long\path\...
```

不得占满整个 Task row。

采用合理：

```text
ellipsis
tooltip
details panel
```

但复制原始信息的地方仍应保证数据准确。

---

# 60. Date / Time / Number Formatting

统一：

```text
file size
speed
duration
ETA
date
time
```

内部保持稳定数据类型。

前端只负责 localized presentation。

避免：

```text
backend String → frontend String → 再 parse
```

这种脆弱设计。

---

# 61. Desktop-native Feel

检查 Tide 是否像：

```text
桌面软件
```

而不是：

```text
网页塞进窗口
```

主要检查：

* context menu
* keyboard behavior
* window behavior
* dialogs
* file paths
* system integration
* drag/drop（若已有）
* tray
* clipboard
* open folder
* default application integration

只打磨已经存在的能力。

---

# 62. Drag & Drop

如果 M8/M1 已有 URL/file drag-and-drop 能力：

M9 进行：

```text
hover state
invalid item
multiple items
feedback
loading
duplicate
```

完善。

如果当前完全没有：

不要因为 M9 强制添加复杂 drag-and-drop subsystem。

---

# 63. Duplicate Task UX

若当前系统能够识别重复任务：

确保提示清晰。

例如：

```text
This resource is already downloading.

[Open existing task]
[Create another task]
```

但不得在 M9 自行设计复杂的去重算法。

这里只改善已有逻辑的表达。

---

# 64. Notifications

如果已有 Windows notification：

检查：

```text
download completed
download failed
important recovery result
```

但：

> 默认不要变成高打扰产品。

尤其不要：

```text
每次状态变化通知
每次 piece 完成通知
每次 retry 通知
```

---

# 65. Notification Preferences

如当前已有 notification setting：

至少允许用户控制：

```text
completion
failure
```

不要在 M9 增加十几个通知开关。

---

# 66. Sound

默认不要加入音效。

Tide 核心气质应该保持安静。

如果当前已有声音功能，只做基础一致性检查。

---

# 67. Onboarding / Discoverability

对于用户第一次使用：

确保：

```text
Paste source
```

是非常容易找到的。

核心路径最好接近：

```text
Open Tide
    ↓
Paste URL / Magnet
    ↓
Start
```

不要让用户先穿过：

```text
Library
Settings
Advanced
Protocols
```

---

# 68. Clipboard

如果当前支持 clipboard：

检查：

```text
URL
Magnet
multiple lines
invalid content
```

不要默认偷偷读取或上传 clipboard 内容。

如产品已有自动识别 clipboard：

必须明确：

```text
local-only
```

并且默认不要进行网络行为。

---

# 69. Privacy UX

保持 Tide 的核心原则：

```text
No account
No cloud
No default telemetry
No upload to ifuyo
```

M9 可以优化 Privacy UI / Settings 文案，但不要增加隐私声明弹窗轰炸。

---

# 70. Security Boundary 不能被 UX 破坏

这是 M9 的硬约束。

任何 UX 优化不得绕过：

```text
backend validation
filesystem containment
URL validation
stream gateway authorization
capability restrictions
state machine
```

禁止：

```text
“为了更方便”把 Rust 安全检查移到前端
```

或：

```text
“为了减少弹窗”默认放开危险操作
```

---

# 71. Delete / Remove UX

必须明确区分：

```text
Remove task
Delete local file
Delete task and file
```

如果产品当前只支持其中之一，就不要伪装存在其他能力。

必须测试：

```text
completed
paused
running
failed
cancelled
library item
```

不同状态下的删除行为。

---

# 72. Settings Reset

如已有 reset 功能：

必须支持：

```text
Reset settings
```

与：

```text
Delete all app data
```

彻底区分。

不能让用户一次点击就误删用户下载内容。

---

# 73. Data / User Files Boundary

继续保持：

```text
AppData
≠
Download Folder
```

任何 UI：

```text
Clear cache
Reset
Delete app data
```

都必须不能误删用户真实媒体文件，除非用户明确选择删除。

---

# 74. Multi-task UX

至少测试：

```text
1 task
2 tasks
5 tasks
10 tasks
20 tasks
```

检查：

* sorting
* rendering
* progress updates
* controls
* CPU
* responsiveness

如果 Scheduler 已经支持任务优先级，则 UI 必须能准确表达当前实际行为。

不要在 M9 发明新的调度模型。

---

# 75. Sorting / Filtering Consistency

如果已有排序：

必须稳定。

不要出现：

```text
一秒前最新
下一秒变成速度排序
```

除非用户明确启用了动态排序。

避免 progress 更新导致列表位置不断抖动。

---

# 76. List Stability

核心原则：

> 数据更新不能让用户正在操作的对象突然跳走。

例如：

用户正在：

```text
hover Task A
```

然后 Task A progress 更新：

不能：

```text
Task A 因排序变化跳走
```

导致用户点击到 Task B。

---

# 77. Player + Task Interaction

检查：

```text
Task deleted while playing
Task paused while playing
Task fails while playing
File externally deleted while playing
Source becomes unavailable while playing
```

定义清晰行为。

例如：

```text
Task may continue to provide existing local data
```

还是：

```text
Playback stops
```

必须基于现有架构作出明确、可验证的行为，而不是随机。

---

# 78. Progressive Gateway UX Boundary

如果 M4 已经有 Local Stream Gateway：

M9 只处理：

```text
player startup
loading
buffering
seek feedback
error feedback
session cleanup
```

不得为了 UX：

```text
把 gateway 暴露到 LAN
去掉 token
允许 arbitrary path
```

这些属于安全红线。

---

# 79. Dialog Density

检查当前 dialog 是否过多。

原则：

```text
Simple action → inline / toast
Potentially destructive → confirm
Complex configuration → dialog/page
Technical diagnostics → expandable
```

避免：

```text
点击暂停
↓
“Are you sure?”
```

这种低质量交互。

---

# 80. Tooltip

Tooltip 只用于：

```text
icon-only button
truncated information
advanced technical option
```

不要为普通文字重复解释。

Tooltip 文案要短。

---

# 81. Icon Audit

检查：

```text
same semantic → same icon
same action → same icon
```

例如：

```text
Pause
Resume
Delete
Open
Settings
```

全局一致。

不要同一个 Delete 在不同页面使用三个图标。

---

# 82. Button Hierarchy

统一：

```text
Primary
Secondary
Tertiary
Danger
Ghost
Icon
```

不要整个页面上所有按钮都是：

```text
Primary
```

这样会失去视觉层级。

---

# 83. Focus / Hover / Disabled

每个 Interactive Component 必须考虑：

```text
default
hover
active
focus
disabled
loading
error
```

不能只有 default。

---

# 84. Disabled 不等于隐藏

只有在没有意义时才隐藏 Action。

例如：

```text
Completed task
```

Pause 没意义，可以不显示。

但如果某操作因为暂时不可用：

可以：

```text
disabled + tooltip
```

帮助用户理解原因。

实际行为以当前产品模型为准。

---

# 85. Error Recovery Path

每个主要错误至少定义：

```text
Error
Possible recovery
User action
```

例如：

```text
Disk full
→ choose another location / free space / retry
```

```text
Network unavailable
→ retry
```

```text
Remote changed
→ restart download
```

```text
Playback unsupported
→ show supported path / external app if available
```

不能只：

```text
Failed.
```

---

# 86. Retry UX

Retry 不应变成：

```text
疯狂点击
```

检查是否已有 backend retry lock / task state guard。

UI 至少处理：

```text
Retrying
Waiting before retry
Retry failed
Retry exhausted
```

---

# 87. Double-click / Double-submit

检查：

```text
Start
Pause
Resume
Retry
Delete
Settings Save
```

避免双击导致：

```text
two tasks
two requests
two DB rows
two playback sessions
```

后端仍需 idempotency / state validation。

---

# 88. Clipboard / Dialog / Command Safety

任何来自 UI 的：

```text
ID
URL
path
index
task ID
resource ID
```

都必须由 backend 再验证。

不能认为：

> 用户从自己的 UI 点击出来的数据一定可信。

---

# 89. Frontend Error Boundary

M9 至少要保证：

某个组件错误不会直接让整个 Tide 白屏。

检查当前 React / frontend error boundary。

出现局部异常时：

```text
保留主界面
显示 fallback
提供 reload/retry
```

具体架构根据当前代码处理。

---

# 90. Backend Panic Safety

检查正常业务错误：

```text
invalid task
missing file
network error
database error
player unavailable
gateway closed
```

不得导致：

```text
panic
unwrap
expect
```

尤其检查新增的 M9 UI support commands。

---

