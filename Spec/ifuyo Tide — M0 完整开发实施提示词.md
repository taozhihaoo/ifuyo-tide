# ifuyo Tide — M0 开发实施提示词

你现在负责实施项目 **ifuyo Tide** 的 **M0：工程骨架与安全地基阶段**。

这是一个长期项目，不是一次性 Demo。

你必须把 M0 做成后续 M0.5～M12 都能稳定建立在其上的工程基础，但**严禁在 M0 提前实现后续阶段功能**。

---

# 一、项目背景

## 产品

**ifuyo Tide**

定位：

> Windows 本地优先的下载器 + 渐进式媒体播放器。

最终支持：

- HTTP / HTTPS 下载
- BitTorrent / Magnet / `.torrent`
- 断点续传
- 多任务
- Scheduler
- Progressive Streaming
- 边下边播
- Seek 驱动的数据调度
- 本地媒体库
- Windows 集成

但这些都不是 M0 的实现内容。

---

# 二、总体产品边界

Tide 的最终核心链路：

```text
User-provided Source
        ↓
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
HTTP / Torrent Engine
        ↓
Storage
        ↓
Progressive Stream Gateway
        ↓
Player
        ↓
Library
```

核心技术资产最终是：

```text
Resource / Availability
+
Scheduler
+
Progressive Stream Gateway
```

M0 必须为这三个核心资产留下清晰、稳定的架构边界。

---

# 三、合规产品边界

Tide 是一个通用本地工具。

Tide：

- 不提供盗版内容
- 不搜索盗版资源
- 不建立 Torrent 索引
- 不推荐资源
- 不提供资源排行榜
- 不提供第三方盗版站导航
- 不提供 Magnet 搜索
- 不绕过 DRM
- 不绕过 Paywall
- 不提供所谓“匿名下载”
- 不提供云端代下载
- 不把用户下载内容上传到 ifuyo
- 不建立用户内容云服务

用户主动向 Tide 提供：

```text
HTTP URL
HTTPS URL
Magnet
Torrent File
Local Media
```

Tide 负责本地处理。

M0 不需要实现上述下载能力，但架构和文档中必须保留这个边界。

---

# 四、隐私原则

默认 Local-first。

Tide 不应把以下信息发送给 ifuyo：

```text
Download URL
Magnet
Info Hash
File Name
File Path
Downloaded Content
Playback History
Download History
Peer IP
User Activity
```

M0 不实现 analytics。

M0 不实现 telemetry。

M0 不实现账号。

M0 不实现云端 API。

M0 不创建任何业务后端依赖。

---

# 五、BitTorrent 隐私与产品边界

未来 Tide 会支持 BitTorrent。

必须明确：

> Tide 不宣传匿名下载，也不承诺隐藏用户 IP。

BitTorrent 网络通信天然需要网络连接信息。

未来会提供：

```text
Download Only
Balanced
Seed
```

等共享策略，但 M0 不实现这些功能。

M0 只需要为未来的 `SharingPolicy` 保留合理的 domain model 扩展位置，不要提前实现 Torrent 行为。

---

# 六、M0 的唯一目标

M0 完成后必须得到一个：

> **可以运行、可以关闭、可以重启恢复状态、前后端边界明确、具备基础安全防护、能够继续进行 M0.5 技术验证的 Tide 空壳应用。**

用户此时还不能下载文件。

用户此时还不能使用 Torrent。

用户此时还不能播放媒体。

这是正确结果。

---

# 七、技术栈

目标：

```text
Windows 11 x64
Tauri 2
Rust
React
TypeScript
Vite
SQLite
```

Rust 负责：

```text
Domain
Persistence
Business Logic
Future Scheduler
Future Download Engine
Security-sensitive Operations
```

Frontend 负责：

```text
UI
Presentation
Interaction
View State
```

不要在前端复制业务逻辑。

---

# 八、第一原则：先检查环境，不要盲目初始化

开始编码之前：

1. 检查当前工作目录。
2. 检查是否已经存在项目。
3. 检查 git 状态。
4. 检查 Node / npm / pnpm。
5. 检查 Rust / Cargo。
6. 检查 Tauri CLI。
7. 检查 Windows 环境。
8. 检查当前已有文件。
9. 检查 README / package.json / Cargo.toml 等已有配置。
10. 判断项目是否已经存在半成品。

执行类似：

```powershell
git status --short
node --version
npm --version
rustc --version
cargo --version
cargo metadata --no-deps
```

如果项目已经存在：

> **禁止删除现有实现后重新生成脚手架。**

应该先审计，再在现有结构基础上实施。

---

# 九、M0 不允许做什么

严格禁止在 M0 实现：

```text
HTTP 下载
HTTPS 下载
Magnet
Torrent
librqbit
DHT
Tracker
Peer
Piece Download
Media Player
WebView video streaming
libmpv
FFmpeg
Media Probe
Scheduler 实际算法
Progressive Streaming
Range Gateway
Library
字幕
缩略图
NAS
UPnP
mDNS
VPN
Cloud API
账号
Telemetry
Steam
```

尤其不要因为“提前做好架构”而引入：

```text
librqbit
FFmpeg
mpv
大规模媒体库
复杂网络库
```

M0 应保持轻量。

---

# 十、M0 项目结构目标

建议结构：

```text
ifuyo-tide/
│
├── src/
│   ├── app/
│   ├── components/
│   ├── features/
│   ├── lib/
│   ├── routes/
│   ├── styles/
│   └── main.tsx
│
├── src-tauri/
│   ├── src/
│   │   ├── app/
│   │   ├── commands/
│   │   ├── core/
│   │   │   ├── task/
│   │   │   ├── source/
│   │   │   ├── resource/
│   │   │   ├── scheduler/
│   │   │   ├── playback/
│   │   │   ├── storage/
│   │   │   └── library/
│   │   ├── persistence/
│   │   ├── security/
│   │   ├── diagnostics/
│   │   └── main.rs
│   │
│   ├── capabilities/
│   ├── migrations/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── tests/
│   ├── fixtures/
│   └── integration/
│
├── docs/
│   ├── architecture/
│   ├── security/
│   └── development/
│
├── README.md
├── SECURITY.md
├── PRIVACY.md
├── CONTENT_POLICY.md
├── LICENSE
├── THIRD_PARTY_NOTICES.md
├── .gitignore
├── package.json
└── ...
```

但是：

> 上述目录是目标方向，不要求机械创建所有空目录。

没有实际用途的空目录不要创建。

---

# 十一、Rust 架构原则

M0 必须首先建立 domain boundary。

至少准备以下 domain：

```text
Task
Source
Resource
PlaybackSession
LibraryItem
```

但只建立：

```text
Type
State
Identity
Persistence boundary
```

不要提前实现复杂行为。

---

# 十二、Task Domain

建立：

```rust
Task
```

至少具有：

```text
task_id
source_id
name
state
created_at
updated_at
```

状态至少设计为：

```text
Created
Ready
Paused
Running
Completed
Failed
Cancelled
```

建议实现为正式 enum。

不要使用：

```text
is_running
is_paused
is_completed
has_error
```

之类大量布尔值组合。

---

# 十三、Task State Machine

M0 建立基本状态转换规则。

合法示例：

```text
Created
  ↓
Ready
  ↓
Running
  ↓
Paused
  ↓
Running
```

以及：

```text
Running
  ↓
Completed
```

错误：

```text
Running
  ↓
Failed
```

取消：

```text
Any Active State
  ↓
Cancelled
```

不允许任意状态随意跳转。

必须有：

```text
transition()
```

或等价机制。

---

# 十四、Source Domain

M0 只定义来源类型：

```text
Http
Torrent
Magnet
LocalFile
```

不实现协议。

例如：

```rust
enum SourceType {
    Http,
    Torrent,
    Magnet,
    LocalFile,
}
```

可以建立：

```text
Source
source_id
source_type
display_name
```

但是不要真正解析 Magnet。

不要真正发送 HTTP。

不要真正读取 Torrent。

---

# 十五、Resource Domain

这是为 M4/M5 预留的核心抽象。

建立最小：

```text
Resource
```

概念。

至少允许未来表达：

```text
resource_id
total_size
availability
```

但 M0 不需要真正计算 availability。

推荐设计为：

```text
ResourceId
ByteRange
```

这样的基础值对象。

---

# 十六、ByteRange

实现一个干净的：

```text
ByteRange
```

至少考虑：

```text
start
end
length
contains(position)
overlaps(other)
```

M0 可以实现纯数学逻辑。

必须有单元测试。

例如：

```text
[0, 100)
[100, 200)
```

两个 range 不应该 overlap。

明确采用：

> half-open interval

即：

```text
[start, end)
```

不要在项目中混用：

```text
inclusive end
exclusive end
```

---

# 十七、未来 Scheduler 的边界

M0 不实现 Scheduler 算法。

但是建立：

```text
scheduler/
```

domain boundary。

未来：

```text
PlaybackDemand
DownloadDemand
Availability
TaskPriority
PiecePriority
```

都应该进入 Scheduler。

M0 可以建立：

```text
Priority
```

枚举或基础类型。

不要提前实现：

```text
priority queue
```

不要提前优化。

---

# 十八、PlaybackSession Domain

M0 建立：

```text
PlaybackSession
```

最小字段：

```text
session_id
task_id/resource_id
created_at
```

可以保留：

```text
position_ms
```

但不要真正播放。

作用只是确定：

> “播放会话不是下载任务本身。”

未来必须支持：

```text
DownloadTask
    +
PlaybackSession
```

独立生命周期。

---

# 十九、LibraryItem Domain

M0 只建立概念：

```text
LibraryItem
```

例如：

```text
library_item_id
resource_id
path
created_at
```

不要做：

```text
TMDB
IMDB
metadata scraping
poster download
subtitle search
```

---

# 二十、SQLite

M0 必须把 SQLite 打通。

建议：

```text
SQLite
+
migration
+
repository
```

不要让 domain 直接操作 SQL。

采用：

```text
Domain
 ↓
Repository
 ↓
SQLite
```

而不是：

```text
React
 ↓
SQL
```

---

# 二十一、数据库最小 schema

M0 至少建立：

```text
tasks
sources
```

可以建立：

```text
playback_sessions
library_items
```

如果没有实际使用价值，可以先不创建。

原则：

> 不为了“未来可能需要”制造几十张表。

---

# 二十二、数据库字段原则

使用明确类型：

```text
UUID / TEXT
INTEGER
BOOLEAN
TIMESTAMP
```

不要把整个对象：

```text
JSON blob
```

塞进一个字段。

尤其：

```text
Task
```

不要做：

```text
task.data = "{...整个 task...}"
```

后续很难迁移。

---

# 二十三、数据库 Migration

建立 migration 机制。

例如：

```text
0001_initial
```

启动时：

```text
open DB
 ↓
check migration
 ↓
apply pending migration
 ↓
ready
```

Migration 必须可重复确认。

不要在应用启动代码里大量执行：

```text
CREATE TABLE IF NOT EXISTS ...
```

来代替真正 migration。

---

# 二十四、Persistence Repository

至少：

```text
TaskRepository
SourceRepository
```

提供：

```text
create
get
list
update
delete
```

M0 不需要复杂查询。

---

# 二十五、App State

建立统一 application state。

例如：

```text
AppState
 ├── database
 ├── repositories
 ├── runtime state
 └── diagnostics
```

避免：

```text
static mut
global singleton everywhere
```

Rust 中必须尽量使用明确依赖注入。

---

# 二十六、Tauri Commands

建立最小 IPC。

至少：

```text
get_app_info
list_tasks
create_test_task
delete_task
```

`create_test_task` 可以用于 M0 验证。

也可以设计：

```text
health_check
```

但不要暴露：

```text
任意 shell
任意 filesystem
任意 URL request
```

---

# 二十七、IPC 设计原则

前端：

```text
invoke("list_tasks")
```

后端：

```text
validate
 ↓
domain
 ↓
repository
 ↓
return DTO
```

禁止：

```text
Frontend
 ↓
直接暴露数据库对象
```

DTO 与 domain model 分离。

---

# 二十八、DTO

例如：

```text
TaskDto
```

用于：

```text
Rust
 ↓
IPC
 ↓
TypeScript
```

不要直接依赖 Rust 内部 domain struct 的全部字段。

这样以后 domain 可以演进，而 UI API 保持稳定。

---

# 二十九、统一 Error

建立：

```text
TideError
```

分类至少：

```text
InvalidInput
NotFound
Persistence
StateTransition
Internal
```

未来可扩展：

```text
Network
Torrent
Storage
Scheduler
Playback
Security
```

但是 M0 不需要真正实现这些模块。

---

# 三十、错误返回

IPC 不允许：

```text
panic
unwrap
expect
```

进入正常用户输入路径。

错误必须转换成：

```text
error_code
message
```

例如：

```json
{
  "code": "TASK_NOT_FOUND",
  "message": "Task not found"
}
```

不要直接把 Rust debug stack 泄露到前端。

---

# 三十一、Frontend Architecture

React 前端至少分：

```text
app
features
components
lib
```

推荐：

```text
features/
├── tasks/
├── library/
├── player/
└── settings/
```

即使 `player` 此时只是 placeholder。

---

# 三十二、Frontend State

M0 不需要引入大型状态库。

优先：

```text
React state
+
context where necessary
+
small API layer
```

不要因为以后可能复杂，就提前安装：

```text
Redux
Zustand
MobX
RxJS
```

除非当前代码真的需要。

---

# 三十三、UI

M0 需要有真实 UI。

不是：

```text
Hello World
```

而是一个能体现未来 Tide 的骨架。

建议：

```text
┌────────────────────────────────────────┐
│ Tide                              +    │
├────────────────────────────────────────┤
│                                        │
│ Downloads                              │
│                                        │
│  No downloads yet                      │
│                                        │
│                                        │
│ Library                                │
│                                        │
│  No media yet                          │
│                                        │
└────────────────────────────────────────┘
```

保留：

```text
Downloads
Library
Settings
```

三个主要入口。

但功能可以是 placeholder。

---

# 三十四、M0 UI 允许的真实功能

至少实现：

```text
启动
查看空任务列表
创建一个 Test Task
看到 Task
删除 Task
关闭
重新启动
Task 仍存在
```

这样 M0 才算真正可运行。

---

# 三十五、不要做假功能

禁止：

```text
假下载进度 0% → 100%
假速度
假 Peer
假 Buffer
假播放
假 Scheduler
```

M0 的 UI 可以显示：

```text
Not implemented in M0
```

但不能用随机数伪造以后功能。

---

# 三十六、Settings

M0 可以建立 Settings 页面。

只实现基础：

```text
Download directory
Language
Start on boot = placeholder/disabled
```

但真正可用的系统启动集成功能放到 M8。

对于 Download Directory：

M0 可以允许用户选择并持久化。

但还没有实际下载，因此只验证配置保存。

---

# 三十七、安全基线：Tauri Capabilities

M0 必须建立最小 capability。

遵守：

> 默认不给前端不必要权限。

不要：

```text
allow-all
filesystem-all
shell-all
process-all
```

如果某权限 M0 没用：

> 不授予。

Tauri 官方当前安全模型将 permissions / capabilities / scopes / runtime authority 作为主要安全边界。

---

# 三十八、安全基线：CSP

M0 必须配置明确 CSP。

默认禁止：

```text
任意远程 script
任意未知 iframe
任意第三方 CDN script
```

尽量只允许：

```text
self
Tauri IPC
应用自身资源
```

媒体 Gateway 在 M4 才加入，因此：

> M0 不要为了未来而给 CSP 开一个大的 `http:*` / `https:*`。

Tauri 官方明确建议 CSP 尽量严格，并避免加载远程内容/脚本。

---

# 三十九、安全基线：Frontend 不可信假设

把前端视为：

```text
Semi-trusted
```

也就是说：

```text
Frontend validation
```

只是 UX。

真正安全检查必须位于：

```text
Rust backend
```

例如以后：

```text
path
URL
task_id
resource_id
```

都不能因为前端检查过就直接信任。

---

# 四十、File System 权限

M0：

```text
不开放任意文件系统权限。
```

只有用户主动选择目录时才获取对应 scope。

如果当前 Tauri 版本/模板需要额外配置：

> 按当前官方文档实现最小 scope。

不要为了方便：

```text
允许整个 C:\
```

---

# 四十一、安全文档

创建：

```text
SECURITY.md
```

至少说明：

```text
Threat Model
Supported Reporting
Security Boundaries
Local Gateway planned design
Filesystem security principles
Dependency security
```

M0 不需要把未来漏洞全部预测出来。

但必须记录当前边界。

---

# 四十二、Privacy 文档

创建：

```text
PRIVACY.md
```

核心原则：

```text
Local-first
No account
No analytics by default
No cloud download service
No upload of user content to ifuyo
```

不要承诺：

```text
绝对匿名
绝对无法追踪
绝对不会产生网络连接记录
```

---

# 四十三、CONTENT_POLICY.md

创建：

```text
CONTENT_POLICY.md
```

内容：

```text
Tide is a general-purpose downloader and media player.

Tide does not provide content search,
torrent indexing, pirate-resource discovery,
or DRM bypass.

Users are responsible for ensuring that they
have the necessary rights to obtain, copy,
store, play, or distribute the content they use.
```

中文版也可以提供。

措辞必须中性，不要做无法保证的法律承诺。

---

# 四十四、README

README 至少：

```text
Project
Features planned
Architecture overview
Development setup
Build
Test
Security
Privacy
Content policy
License
```

README 的当前功能必须明确写：

```text
M0 — Project Skeleton
```

不要声称：

```text
supports Torrent
supports streaming
supports media playback
```

因为 M0 尚未实现。

---

# 四十五、LICENSE

如果项目已经确定采用某个主许可证：

> 保留现有许可证。

如果尚未确定：

> 不要擅自替用户选择并声称整个项目已经正式授权。

可以建立 placeholder/documentation，但最终许可证策略必须在首次公开发布前确定。

---

# 四十六、THIRD_PARTY_NOTICES

M0 建立：

```text
THIRD_PARTY_NOTICES.md
```

至少为后续依赖审计建立位置。

记录：

```text
Dependency
Version
License
Source
Notes
```

当前不要为了“完整”手工抄几十个传递依赖。

可以说明：

```text
Automated dependency license inventory will be
generated during release hardening.
```

---

# 四十七、Dependency 管理

原则：

> 少依赖。

每加入一个依赖，先判断：

```text
是否真的需要？
标准库能否解决？
当前框架是否已经提供？
会不会增加许可证负担？
会不会增加攻击面？
```

---

# 四十八、Cargo.lock

桌面应用必须保持可复现依赖状态。

M0 建立：

```text
Cargo.lock
```

并纳入版本控制。

CI/验证环境优先使用：

```text
cargo check --locked
cargo test --locked
```

Cargo 官方文档明确说明 `Cargo.lock` 用于锁定成功构建时的具体依赖状态，并可配合 `--locked` 保证不重新解析依赖。

---

# 四十九、代码质量原则

整个 Tide 项目从 M0 开始遵守：

```text
No God Object
No speculative abstraction
No duplicate business logic
No hidden global state
No unnecessary singleton
No unsafe unless justified
No unwrap/expect in runtime user paths
No filesystem path trust
No frontend-only security validation
No hard-coded secrets
No fake implementations
No dead abstraction
```

---

# 五十、Rust 代码风格

尽量：

```text
small modules
small functions
explicit types
Result<T, TideError>
```

避免：

```text
500-line function
1000-line manager
```

尤其不要创建：

```text
TideManager
```

然后什么都往里面放。

---

# 五十一、Frontend 代码风格

避免：

```text
App.tsx = 1500 lines
```

应该：

```text
App
 ├── Sidebar
 ├── DownloadView
 ├── LibraryView
 └── SettingsView
```

组件职责单一。

---

# 五十二、Logging

使用结构化日志。

例如：

```text
app.started
app.shutdown
db.ready
task.created
task.deleted
ipc.error
```

生产日志不要包含：

```text
完整用户 URL
完整 Magnet
完整本地文件路径
Peer IP
```

M0 可以建立 diagnostics 模块。

---

# 五十三、Log Levels

至少：

```text
error
warn
info
debug
```

默认：

```text
info
```

开发：

```text
debug
```

Tide Lab 未来再允许：

```text
trace
```

---

# 五十四、应用目录

Windows 下不要把数据库直接放在项目目录。

使用应用数据目录。

建议最终：

```text
%LOCALAPPDATA%\ifuyo\Tide\
```

内部未来：

```text
config/
database/
logs/
cache/
sessions/
temp/
```

M0 至少真正创建：

```text
database
logs
```

其余按实际需要。

---

# 五十五、用户下载目录与应用数据目录必须分离

这是架构原则。

```text
Application Data
≠
User Download Data
```

以后：

```text
App Update
```

不能碰：

```text
Downloaded Files
```

---

# 五十六、ID

建议：

```text
UUID
```

或者当前生态中可靠的随机 ID。

不要使用：

```text
increment integer
```

作为长期暴露给 frontend / IPC / local gateway 的资源标识。

特别是未来 Stream Gateway：

> 不应该直接使用可枚举 ID。

---

# 五十七、时间

统一：

```text
UTC internally
```

UI：

```text
local timezone
```

不要：

```text
数据库混入各种本地时间字符串
```

---

# 五十八、M0 测试

必须建立自动化测试。

至少包括：

## Rust unit tests

```text
ByteRange
Task state transition
Task validation
Repository
```

## Integration

```text
Create Task
Persist
Reload
Delete
```

## Frontend

至少：

```text
Render main page
Task list empty
Task appears after creation
```

---

# 五十九、关键 Task 状态测试

必须覆盖：

```text
Created → Ready
Ready → Running
Running → Paused
Paused → Running
Running → Completed
Running → Failed
Running → Cancelled
```

并测试非法转换。

例如：

```text
Completed → Running
Cancelled → Running
```

应该失败。

---

# 六十、数据库持久化验收

必须真实运行：

```text
启动
 ↓
创建 Task
 ↓
关闭应用
 ↓
再次启动
 ↓
读取 Task
```

Task 必须还存在。

这不能用 fake memory state。

---

# 六十一、Crash-like 验收

至少模拟：

```text
启动
创建 Task
立即终止进程
再次启动
```

数据库不能因此损坏。

如果发现 SQLite transaction 问题：

> 修复，不要通过“异常情况下忽略数据库”绕过去。

---

# 六十二、M0 的 Git 原则

每完成一个稳定增量：

```text
git diff
git status
```

确认：

```text
没有误修改
没有 build output
没有临时密钥
没有本机路径
没有日志
```

`.gitignore` 必须正确。

---

# 六十三、不要提交

禁止提交：

```text
target/
node_modules/
dist/
build artifacts
local database
local logs
.env
credentials
private keys
personal paths
IDE junk
```

---

# 六十四、M0 不需要 CI，但推荐至少准备验证命令

项目应该可以使用：

```text
npm run build
cargo check --locked
cargo test --locked
```

以及 Tauri 的 build/check 命令。

如果配置 CI：

> 只做最基本的 build/test，不要在 M0 搭建复杂 release pipeline。

---

# 六十五、开发文档

创建：

```text
docs/development/GETTING_STARTED.md
```

说明：

```text
Prerequisites
Install
Dev Run
Build
Test
Directory structure
Troubleshooting
```

目标是：

> 新开一个终端后，按照 README 可以启动项目。

---

# 六十六、Architecture 文档

创建：

```text
docs/architecture/ARCHITECTURE.md
```

画出至少：

```text
Frontend
   ↓ IPC
Application Core
   ↓
Domain
   ↓
Repository
   ↓
SQLite
```

以及未来：

```text
HTTP Engine
Torrent Engine
   ↓
Resource
   ↓
Availability
   ↓
Scheduler
   ↓
Gateway
   ↓
Player
```

明确：

> 后半段在 M0 尚未实现。

---

# 六十七、Future Boundary 文档

可以在 Architecture 中记录：

```text
M0      Foundation
M0.5    Technical Spike
M1      HTTP
M2      Persistence
M3      Torrent
M4      Streaming
M5      Player
M6      Library
M7      Smart Scheduler
M8      Windows Integration
M9      Polish
M10     Security / Release Hardening
M11     Release Candidate
M12     1.0
```

不要提前实现。

---

# 六十八、M0 不要引入共享 ifuyo-core

即使其他 ifuyo 项目存在类似：

```text
cache
range
storage
```

M0 也不要抽共享 crate。

暂时：

```text
Tide-specific
```

保持独立。

未来真正出现稳定重复代码再抽取。

---

# 六十九、命名

项目：

```text
ifuyo Tide
```

package：

```text
ifuyo-tide
```

Rust crate：

```text
ifuyo-tide
```

建议 application identifier：

```text
com.ifuyo.tide
```

如果项目中已有正式命名规则：

> 以现有规则为准。

---

# 七十、应用视觉

M0 不需要精雕 UI。

但应该有一个非常简单、干净的 Tide 视觉基线：

```text
dark/light friendly
minimal
no gradients overload
no noisy dashboard
```

产品气质：

```text
calm
technical
clean
fast
```

避免传统下载器常见的：

```text
大量表格
几十个按钮
工具栏堆满
状态灯泛滥
```

---

# 七十一、M0 UI 需要表现的未来信息

即便功能未实现，也可以留下合理的信息层：

```text
Downloads
    Active
    Completed

Library

Settings
```

未来 Task card 可以承载：

```text
Title
Status
Progress
Speed
ETA
Playing
```

M0 不显示 fake 数值。

---

# 七十二、运行方式

必须能：

```text
npm install
npm run tauri dev
```

或者根据实际 package manager：

```text
pnpm install
pnpm tauri dev
```

具体以最终项目 package.json 为准。

---

# 七十三、构建验证

M0 完成前必须运行：

```text
frontend typecheck
frontend build
cargo check --locked
cargo test --locked
tauri build
```

如果某条命令由于本机环境缺失无法执行：

> 明确记录原因。

禁止写：

```text
Build OK
```

而实际上没有运行。

---

# 七十四、最终手工验收

必须实际启动 Windows App。

验证：

### Case 1

```text
启动 Tide
```

结果：

```text
主界面正常
```

### Case 2

```text
Create Test Task
```

结果：

```text
Task 出现在列表
```

### Case 3

```text
关闭 Tide
重新启动
```

结果：

```text
Task 仍然存在
```

### Case 4

```text
删除 Task
```

结果：

```text
Task 消失
```

### Case 5

```text
非法状态转换
```

结果：

```text
被 Rust Core 拒绝
```

### Case 6

```text
随机输入/错误 task ID
```

结果：

```text
返回结构化错误
```

而不是：

```text
panic
```

---

# 七十五、M0 Definition of Done

只有全部满足以下条件，M0 才算完成：

```text
[ ] Tauri 2 项目可运行
[ ] React + TypeScript + Vite 可运行
[ ] Rust Core 可运行
[ ] SQLite 可用
[ ] Migration 可用
[ ] Task domain 已建立
[ ] Task state machine 已建立
[ ] Source domain 已建立
[ ] Resource 基础抽象已建立
[ ] ByteRange 已建立并测试
[ ] PlaybackSession 基础类型已建立
[ ] LibraryItem 基础类型已建立
[ ] Repository boundary 已建立
[ ] Tauri IPC 可用
[ ] DTO boundary 已建立
[ ] Error boundary 已建立
[ ] Capability 最小权限
[ ] CSP 已建立
[ ] 无任意 shell 权限
[ ] 无任意 filesystem 权限
[ ] 无 telemetry
[ ] 无 cloud service
[ ] SECURITY.md
[ ] PRIVACY.md
[ ] CONTENT_POLICY.md
[ ] README
[ ] THIRD_PARTY_NOTICES.md
[ ] Architecture 文档
[ ] Development 文档
[ ] Cargo.lock 已固定
[ ] .gitignore 正确
[ ] Rust unit tests pass
[ ] integration tests pass
[ ] frontend build pass
[ ] cargo check --locked pass
[ ] cargo test --locked pass
[ ] Tauri development run pass
[ ] Windows 手工验收 pass
[ ] 无 fake download
[ ] 无 fake playback
[ ] 无 fake peer
[ ] 无 fake Scheduler
```

---

# 七十六、M0 最终交付物

完成后必须得到：

```text
1. 可运行 Tide Windows 开发版本
2. 可持久化 Task 的最小 UI
3. Rust Core 基础架构
4. SQLite migration
5. IPC API
6. Security baseline
7. Privacy / Content / Security 文档
8. Architecture 文档
9. Development 文档
10. 自动化测试
```

---

# 七十七、完成后输出报告

实施结束后，不要只说：

```text
Done
```

必须输出一份结构化报告：

## 1. IMPLEMENTED

列出真正完成的内容。

## 2. FILES CHANGED

列出核心新增/修改文件。

## 3. ARCHITECTURE

简述实际架构。

## 4. SECURITY

说明实际配置了哪些：

```text
Capability
CSP
Filesystem scope
IPC validation
Error handling
```

## 5. TEST RESULTS

逐项给出：

```text
command
result
```

例如：

```text
cargo test --locked
PASS
```

## 6. MANUAL VALIDATION

说明 Windows GUI 实测结果。

## 7. KNOWN LIMITATIONS

明确：

```text
HTTP download not implemented
Torrent not implemented
Player not implemented
Streaming not implemented
Scheduler algorithm not implemented
```

## 8. M0.5 READINESS

判断当前项目是否可以进入：

> M0.5 Technical Spike

如果不可以，必须明确阻塞项。

---

# 七十八、最重要的执行纪律

整个实施过程中必须遵守：

> **M0 是 Foundation，不是 MVP。**

不要因为：

```text
“顺手可以做”
“以后肯定需要”
“反正现在有时间”
“AI 很快”
```

而提前实现 M1/M3/M4/M5。

尤其禁止：

```text
“既然已经有 Task，就顺手做 HTTP 下载”
“既然有 Torrent domain，就顺手接 rqbit”
“既然有 Player 页面，就顺手接 video”
```

这些全部留到后续阶段。

---

# 七十九、优先级

实施顺序严格：

```text
P0
工程可运行
+
架构边界
+
SQLite
+
Task
+
IPC
+
Security baseline

P1
Source / Resource / PlaybackSession 基础 domain

P2
UI polish
+
documentation

P3
额外优化
```

如果时间有限：

> 宁可牺牲 UI polish，也不要牺牲 Core / Persistence / Security / Tests。

---

# 八十、开始执行

现在开始。

第一步不要马上写代码。

先：

```text
Inspect repository
Inspect environment
Inspect current dependencies
Inspect existing architecture
```

然后给出：

```text
FACTS
HYPOTHESES
IMPLEMENTATION PLAN
```

再开始修改。

如果发现已有架构与本提示词冲突：

> 优先保留已有正确实现，不要机械重构。

只有存在明确技术问题时才修改。

整个 M0 完成后必须运行实际测试，并根据测试结果修复问题。

**不要停留在“代码看起来正确”。必须实际运行。**