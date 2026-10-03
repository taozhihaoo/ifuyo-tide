# ifuyo Tide — M8 Windows Integration 完整开发实施提示词

> 项目：ifuyo Tide
> 里程碑：M8
> 主题：Windows Integration / Windows Desktop Native Integration
> 平台基线：Windows 11 x64
> 技术基线：Tauri 2 + Rust + TypeScript + React + Vite + SQLite
> 前置里程碑：M0 / M0.5 / M1 / M2 / M3 / M4 / M5 / M6 / M7
> 状态：实施任务
> 核心要求：**先审计、后设计、再实现；禁止跨里程碑扩张。**

---

# 一、M8 的目标

M8 的目标不是继续增加 Tide 的下载协议或播放器能力，而是完成：

> **“Tide 在 Windows 上像一个真正的桌面应用一样工作。”**

M8 完成后，用户应该能够自然地：

```text
启动 Tide
   ↓
只有一个实例
   ↓
最小化 / 隐藏到系统托盘
   ↓
后台继续执行下载 / Torrent / Scheduler
   ↓
下载完成或失败时获得可控的 Windows 通知
   ↓
双击 .torrent / 使用 Tide 自定义协议打开 Tide
   ↓
第二次启动请求不会创建第二个实例，而是转交给已有实例
   ↓
打开下载文件 / 打开所在文件夹
   ↓
应用关闭后再次启动
   ↓
恢复合理的窗口状态
   ↓
可选地随 Windows 启动
```

最终形成：

```text
Tide Core
    ↓
M7 Scheduler / Task / Playback / Library
    ↓
Windows Integration Layer
    ├── Single Instance
    ├── Launch Request Routing
    ├── System Tray
    ├── Window State
    ├── Autostart
    ├── Deep Link / File Association
    ├── Native Open / Show in Explorer
    ├── Windows Notification
    └── Windows Installer Integration
```

M8 的核心原则：

> **Windows 是宿主环境，不是新的业务层。**

不要把 Windows API 逻辑散落到 Task、DownloadEngine、TorrentEngine、Scheduler、Player、Library 中。

---

# 二、严格边界

## 2.1 M8 必须实现

M8 至少实现以下能力：

### A. Single Instance

确保正常情况下：

```text
同一用户环境
+
同一 Tide 安装
+
同一实例标识
=
只能存在一个 Tide 主实例
```

第二次启动时：

```text
启动请求
    ↓
发现已有实例
    ↓
把启动参数 / 文件 / URL 转交给已有实例
    ↓
已有实例激活
    ↓
处理请求
    ↓
第二进程退出
```

不得出现：

```text
Tide #1
Tide #2
Tide #3
```

同时运行并分别修改同一 SQLite / 下载任务。

---

## 2.2 Launch Request 统一入口

所有“从 Windows 打开 Tide”的行为必须先归一化为统一启动请求。

建议抽象：

```text
LaunchRequest
├── OpenHttpUrl
├── OpenMagnet
├── OpenTorrentFile
├── OpenMediaFile
└── Focus
```

实际类型必须根据现有项目结构调整。

核心思想：

```text
CLI
Deep Link
File Association
Second Instance Forwarding
Explorer
Browser
Installer-created association
        ↓
Launch Request Parser
        ↓
LaunchRequest
        ↓
Application Layer
        ↓
真正业务 Command
```

不要让：

```text
deep-link callback
single-instance callback
CLI parser
tray callback
```

直接调用 DownloadEngine / TorrentEngine。

必须经过 Application Boundary。

---

# 三、M8 禁止事项

本阶段明确禁止：

```text
❌ 新增 HTTP 下载协议能力
❌ 新增 Torrent 协议能力
❌ 修改 librqbit 核心逻辑
❌ 重写 Scheduler
❌ 重写 Resource / Availability
❌ 重写 Progressive Streaming
❌ 新增媒体协议
❌ 新增播放器后端
❌ FFmpeg
❌ libmpv
❌ VLC
❌ NAS
❌ UPnP
❌ DLNA
❌ mDNS
❌ 云同步
❌ 账号系统
❌ 云端服务
❌ 遥测
❌ 在线资源搜索
❌ 资源推荐
❌ 第三方资源站导航
❌ Tracker 推荐
❌ 匿名下载
❌ VPN
❌ 代理系统
❌ DRM 绕过
❌ 付费内容绕过
❌ 破解下载限制
❌ 广告系统
❌ Steam Integration
❌ 自动更新完整体系
❌ 完整发布签名流程
❌ 大规模 Windows 原生 UI 重写
```

尤其禁止：

> 为了实现某个 Windows Integration 功能，而把业务逻辑反向塞进平台层。

---

# 四、第一阶段：审计现有项目

开始修改代码之前，必须完整检查当前仓库。

不要假定 M0～M7 一定按照设计完成。

首先检查：

```text
README
docs/
src/
src-tauri/
Cargo.toml
Cargo.lock
package.json
tauri.conf.json
capabilities/
migrations/
数据库 schema
现有 IPC
Task
Download
Torrent
Scheduler
Playback
Library
Resource
Persistence
Error
Event
```

同时检查当前版本：

```text
Tauri
Rust
Node
Package manager
React
TypeScript
Vite
WebView2
Windows SDK / MSVC
SQLite
已安装 Tauri plugins
```

必须实际读取：

```text
Cargo.toml
package.json
tauri.conf.json
capabilities/*
lib.rs
main.rs
现有 commands
现有 application layer
现有 event layer
现有 persistence layer
```

---

# 五、审计输出格式

实现前先输出：

## FACTS

只写仓库中已经确认存在的事实。

例如：

```text
FACT:
Tauri version = ...
FACT:
Single instance not implemented.
FACT:
Task state persistence already exists.
FACT:
Scheduler is exposed through ApplicationService.
FACT:
Current app already has tray icon.
```

---

## HYPOTHESES

无法确认但合理推测的内容必须单独写：

```text
HYPOTHESIS:
Current app lifecycle may already have a central shutdown hook.
```

---

## M8 GAP ANALYSIS

明确：

```text
Feature
Current State
Required State
Gap
Risk
```

至少覆盖：

```text
Single Instance
Launch Routing
System Tray
Window State
Autostart
Deep Link
Torrent File Association
Media File Association
Native Open
Notification
Installer
Capability Permissions
Logging
Shutdown
Background Runtime
```

---

## IMPLEMENTATION PLAN

给出：

```text
P0
P1
Implementation Order
Dependencies
Tests
Manual Validation
```

未经审计，不得先大规模修改项目。

---

# 六、M8 的总体架构

推荐形成以下逻辑：

```text
src-tauri/src/platform/windows/
├── mod.rs
├── launch.rs
├── tray.rs
├── window.rs
├── autostart.rs
├── notification.rs
├── associations.rs
├── integration.rs
└── error.rs
```

如果现有项目已经有：

```text
platform/
app/
integration/
desktop/
```

则必须优先复用现有结构。

不要为了“看起来整齐”而创建第二套平台抽象。

---

# 七、Windows Platform Boundary

建立明确平台边界：

```text
Application Layer
        ↓
Platform Integration Trait / Service
        ↓
Windows Implementation
```

但不要为了只有一个 Windows 实现而制造十几个无意义 trait。

只有在存在实际调用边界时抽象。

推荐概念：

```text
WindowsIntegrationService
```

负责：

```text
activate_main_window()
show_main_window()
hide_main_window()
open_path()
show_in_explorer()
send_notification()
install_autostart()
remove_autostart()
```

但：

```text
create_download_task()
pause_download()
resume_download()
```

绝对不能属于 WindowsIntegrationService。

---

# 八、Single Instance

使用当前项目对应版本的官方 Tauri Single Instance 机制。

不要自己手写 named mutex，除非当前 Tauri 版本/需求确认官方能力不足。

注意：

> Single Instance Plugin 在 Tauri 官方文档中明确支持 Windows，并且插件注册顺序存在要求，必须根据当前实际版本文档正确初始化。

## 8.1 必须实现

启动：

```text
First Process
    ↓
Acquire Instance Ownership
    ↓
Continue Startup
```

第二进程：

```text
Second Process
    ↓
Detect Existing Instance
    ↓
Forward Args / Launch Request
    ↓
Activate Existing Window
    ↓
Exit
```

---

## 8.2 必须处理的参数

至少考虑：

```text
无参数
```

```text
tide://...
```

```text
磁力链接
```

```text
C:\Downloads\example.torrent
```

```text
C:\Videos\movie.mp4
```

非法参数：

```text
random.exe.argument
```

不能直接进入业务层。

---

## 8.3 第二实例行为

第二实例转发请求以后：

```text
已有实例：
    restore / show
    bring_to_front
    focus
    process LaunchRequest
```

不得：

```text
创建第二窗口
```

不得：

```text
创建第二 Scheduler
```

不得：

```text
启动第二 DownloadEngine
```

不得：

```text
打开第二 SQLite connection manager
```

---

# 九、LaunchRequest 安全模型

所有外部输入均视为：

> **Untrusted Input**

包括：

```text
CLI arguments
Deep Link
File Association
`.torrent` path
URL
magnet URI
```

必须重新验证。

---

## 9.1 HTTP / HTTPS

必须继续使用 M1 的 URL Validator。

不要在 Windows Integration 中重新实现一份 URL Validation。

必须复用：

```text
HttpUrlValidator
```

或者当前项目对应组件。

---

## 9.2 Magnet

必须：

```text
trim
parse
validate scheme
validate structure
normalize
```

不要简单：

```text
starts_with("magnet:")
```

就认为合法。

---

## 9.3 Torrent File

外部传入：

```text
C:\foo\bar.torrent
```

必须：

```text
validate path
canonicalize
verify file exists
verify regular file
check extension
check reasonable file size
check access
```

禁止：

```text
path traversal
directory-as-file
device path
unexpected UNC path
reparse-point trick
```

并继续遵守 M1/M2 的 filesystem security boundary。

---

## 9.4 Media File

M8 不负责媒体解析。

这里只负责：

```text
用户从 Windows 打开媒体文件
        ↓
解析路径
        ↓
交给现有 Library / Playback Application Command
```

如果当前 M5/M6 尚未定义此入口，则只建立：

```text
OpenMediaFile
```

Application Command。

不要趁机重写 Player。

---

# 十、Deep Link

使用当前版本 Tauri Deep Link 官方能力。

Tauri 当前官方文档显示，Windows 桌面端 Deep Link 可以通过命令行参数进入应用，并可与 Single Instance 集成；因此 M8 必须把两者作为一个统一的启动路由问题处理。

---

## 10.1 建议协议

建议至少设计：

```text
tide://add
```

例如：

```text
tide://add?url=https%3A%2F%2Fexample.com%2Ffile.zip
```

或者：

```text
tide://add?magnet=...
```

具体格式必须在项目中正式定义。

---

## 10.2 协议处理原则

Deep Link：

```text
不直接下载
```

而是：

```text
Deep Link
 ↓
Parse
 ↓
Validate
 ↓
LaunchRequest
 ↓
Application
 ↓
Create Task
```

---

## 10.3 防止伪造

不要认为：

```text
Tauri Deep Link Plugin
```

等于：

```text
trusted input
```

官方文档也明确提醒，用户可以手工构造带有 URL 的命令行参数，因此应用仍然必须检查 URL 的格式与预期 scheme。

必须验证：

```text
scheme
host
path
query
parameter count
parameter size
encoding
```

拒绝：

```text
unexpected scheme
oversized query
malformed URL
unsupported command
```

---

# 十一、Torrent 文件关联

M8 重点之一：

```text
double-click *.torrent
        ↓
Windows
        ↓
Tide
        ↓
OpenTorrentFile
        ↓
用户确认 / 任务创建
```

注意：

> “双击 .torrent”不是“自动下载”。

推荐默认行为：

```text
打开 .torrent
    ↓
读取 metadata
    ↓
显示确认界面
    ↓
用户确认
    ↓
创建 Torrent Task
```

不要因为 Windows file association 而绕过正常用户确认。

---

# 十二、磁力链接处理

对 `magnet:` 必须非常克制。

M8 可以提供：

```text
Paste / browser handoff / system integration
```

但不要因为系统集成而加入：

```text
资源发现
```

```text
磁力搜索
```

```text
Tracker 搜索
```

```text
第三方资源站
```

```text
自动推荐
```

---

# 十三、System Tray

这是 M8 的核心桌面体验功能。

建议：

```text
Tide Main Window
       ↕
System Tray
```

---

## 13.1 Tray 菜单至少包含

```text
显示 Tide
暂停全部下载
恢复全部下载
正在下载：N
正在播放：N
打开下载目录
设置
退出 Tide
```

但这些菜单必须基于真实状态。

禁止：

```text
fake task count
fake speed
fake progress
```

---

## 13.2 Tray 单击行为

建议：

```text
左键：
    Show / Activate

双击：
    Show / Activate
```

右键：

```text
Context Menu
```

实际行为根据当前 Tauri tray API 和 Windows UX 测试确定。

---

## 13.3 Minimize to Tray

提供设置：

```text
关闭窗口时：
    □ 退出 Tide
    □ 最小化到系统托盘
```

以及：

```text
最小化到托盘
```

的明确行为。

尤其处理：

```text
点击 X
```

和：

```text
点击任务栏最小化
```

不要混淆。

---

# 十四、后台运行语义

M8 必须正式定义：

```text
Window Hidden
```

与：

```text
Application Stopped
```

完全不同。

例如：

```text
Window hidden
      ≠
Scheduler stopped
```

```text
Tray-only
      ≠
Process exited
```

---

## 14.1 Window 隐藏时

应继续：

```text
Download
Torrent
Scheduler
Playback backend if policy allows
Persistence
Recovery checkpoints
```

但不应继续高频刷新前端。

---

## 14.2 UI 不可见时降低前端负载

不要：

```text
后台每 16ms
↓
React rerender
↓
500 tasks
```

应使用：

```text
Backend continues real work
        ↓
Event / state updates
        ↓
Frontend only when visible / required
```

---

# 十五、Windows Notification

使用官方 Notification Plugin 或当前版本等价机制。

Tauri 当前官方通知插件支持 Windows，但 Windows 上生产通知行为依赖已安装应用环境，因此开发环境与安装后的行为必须分别验证。

---

## 15.1 通知类型

至少设计：

```text
DownloadCompleted
DownloadFailed
TorrentCompleted
TaskNeedsAttention
```

---

## 15.2 不允许

不得添加：

```text
营销通知
```

```text
资源推荐通知
```

```text
诱导回流通知
```

```text
强制通知
```

---

## 15.3 用户设置

建议：

```text
Notifications
    [ ] 下载完成
    [ ] 下载失败
    [ ] Torrent 完成
```

默认策略必须根据产品整体 UX 决定，但必须：

```text
可关闭
```

并且：

```text
通知权限被拒绝
```

不能导致业务失败。

---

# 十六、Autostart

使用当前 Tauri Autostart 官方插件能力。

Tauri 官方插件目前支持 Windows 自动启动。

---

## 16.1 默认状态

建议：

```text
默认关闭
```

用户主动打开：

```text
Settings
→ General
→ Start Tide with Windows
```

---

## 16.2 启动行为

如果：

```text
Autostart = ON
```

建议：

```text
Windows boot
    ↓
Tide process
    ↓
Restore persistent state
    ↓
Start required backend services
    ↓
Optional tray-only mode
```

不要默认弹出巨大主窗口打扰用户。

---

## 16.3 与 Scheduler 的关系

Autostart 只是：

```text
Process Startup Policy
```

不是：

```text
Scheduler Policy
```

Scheduler 仍然根据 M7 的规则工作。

不要在 M8 重新设计 Scheduler。

---

# 十七、Window State

使用当前 Tauri Window State 能力，保存：

```text
window position
window size
maximize state
```

Tauri 官方 Window State 插件支持在重新启动时恢复窗口位置和尺寸，并且可以在窗口创建时隐藏窗口以减少恢复过程中的闪烁。

---

## 17.1 必须处理

用户：

```text
窗口拖到第二块显示器
```

然后：

```text
拔掉第二块显示器
```

重新启动 Tide。

不得出现：

```text
窗口跑到屏幕外
```

必须进行：

```text
Monitor Bounds Validation
```

---

## 17.2 最小尺寸

必须定义合理：

```text
min width
min height
```

并防止：

```text
恢复非法尺寸
```

---

# 十八、Native Open

使用当前 Tauri Opener 能力，而不是随意调用 shell。

官方 Tauri Opener 插件支持：

```text
open file
open URL
show file in system file manager
```

并提供权限控制。

---

## 18.1 必须支持

例如：

```text
打开文件
```

```text
在资源管理器中显示
```

```text
打开下载目录
```

---

## 18.2 禁止

不要：

```text
Command::new("cmd")
```

然后拼接：

```text
用户输入路径
```

不要：

```text
cmd /c ...
```

不要：

```text
powershell ...
```

不要：

```text
shell execute arbitrary
```

---

# 十九、Tauri Capability / Permission

M8 新增插件以后，重新审计：

```text
capabilities/default.json
```

任何新增 plugin：

```text
single-instance
autostart
deep-link
notification
opener
window-state
```

都必须：

```text
只开启真正使用的 permission
```

不要简单：

```text
plugin:default
```

全部打开。

---

## 19.1 安全原则

坚持：

```text
Minimum Capability
```

尤其不要为了：

```text
Open Explorer
```

而给予：

```text
Shell execute
```

不要为了：

```text
Notification
```

而增加 filesystem 权限。

---

# 二十、Installer Integration

M8 开始正式验证：

```text
开发模式
```

和：

```text
安装后模式
```

的区别。

Tauri 当前 Windows 安装器支持：

```text
MSI
NSIS
```

并且 WebView2 安装策略可以配置。官方文档还说明 Windows 11 默认已经提供 WebView2，但安装包策略仍必须明确。

---

## 20.1 M8 不负责

不要在 M8 做完整：

```text
代码签名
发布证书
自动更新
Release Channel
Microsoft Store
正式发行流水线
```

这些属于后续 Release Hardening / RC 阶段。

---

## 20.2 M8 必须验证

至少能生成：

```text
Windows installer
```

然后在干净环境/测试环境验证：

```text
Install
Launch
Single Instance
Tray
Window State
Notification
Autostart
Deep Link
Torrent Association
Open File
Uninstall
```

---

# 二十一、Installer 与 File Association

最终必须明确：

```text
.torrent → Tide
```

以及：

```text
tide:// → Tide
```

的注册、测试、卸载行为。

如果决定支持其它文件类型：

```text
.mp4
.mkv
.webm
...
```

必须非常谨慎。

不要为了“看起来像媒体播放器”而在 M8 注册大量文件类型。

Tide 的核心身份依然是：

> Downloader + Progressive Media Player

而不是：

> Windows 全能媒体默认播放器。

---

# 二十二、退出语义

M8 必须重新梳理：

```text
Hide
Minimize
Close Window
Exit App
System Shutdown
Windows Logout
Crash
```

这些状态不能混在一起。

---

## 22.1 Exit App

显式：

```text
退出 Tide
```

必须：

```text
停止运行时任务
保存必要状态
关闭后台服务
关闭 tray
释放资源
关闭数据库
退出进程
```

---

## 22.2 Window Close

如果设置：

```text
Close → Tray
```

则：

```text
Close window
≠
Exit application
```

---

# 二十三、Windows Shutdown / Logout

M8 至少验证：

```text
Windows shutdown
Windows restart
user logout
```

应用不能由于：

```text
shutdown hook
```

而死锁。

不能把：

```text
完整 shutdown
```

做成几十秒同步等待。

应继续使用 M2 已定义的：

```text
durable persistence
```

保证崩溃/强制退出仍然可恢复。

---

# 二十四、与 M2 Crash Recovery 的关系

必须保持：

```text
Graceful Shutdown
    ≠
Recovery Requirement
```

也就是说：

即使 Windows：

```text
kill process
power loss
crash
```

M2 仍然必须能恢复。

M8 只优化：

```text
graceful lifecycle
```

不能把：

```text
正确恢复
```

建立在：

```text
Windows 正常通知退出
```

之上。

---

# 二十五、与 M7 Scheduler 的关系

M8 必须使用 M7 已存在的 Scheduler。

例如：

```text
Windows startup
   ↓
Application starts
   ↓
Task persistence restored
   ↓
Scheduler restored
   ↓
Scheduler evaluates policy
   ↓
Downloads continue
```

不要新增：

```text
WindowsScheduler
```

除非已有架构明确要求。

---

# 二十六、系统托盘 + Scheduler 的真实状态

Tray 上可以显示：

```text
正在下载 3 个任务
```

但必须来自：

```text
Scheduler / Task Runtime
```

不能自行维护：

```text
trayDownloadCount
```

否则：

```text
Scheduler = 3
Tray = 2
```

最终一定产生状态漂移。

推荐：

```text
Backend State
      ↓
Derived Application Snapshot
      ↓
Tray Projection
      ↓
Frontend Projection
```

---

# 二十七、统一 Application Snapshot

如当前架构合适，可以增加：

```text
AppRuntimeSnapshot
```

例如：

```text
active_tasks
active_downloads
active_torrents
completed_tasks
failed_tasks
playback_active
scheduler_running
window_visibility
```

Tray 只读取 snapshot。

不要让 tray 自己查询十个 service。

---

# 二十八、Background Mode

定义：

```text
Foreground
Background
TrayOnly
Exiting
```

这些可以作为：

```text
Runtime Lifecycle
```

不要塞进：

```text
TaskState
```

不要把：

```text
Running
Paused
Completed
```

和：

```text
TrayOnly
Visible
Hidden
```

混成一套 state machine。

---

# 二十九、Windows Integration Error Model

所有 Windows Integration 错误统一进入现有：

```text
TideError
```

建议：

```text
PlatformError
├── SingleInstance
├── LaunchRouting
├── Tray
├── WindowState
├── Autostart
├── Notification
├── FileAssociation
├── Opener
└── Installer
```

但不要过度细化。

---

# 三十、日志要求

M8 新增：

```text
single instance
launch request
tray
autostart
deep link
notification
association
```

日志。

但是生产日志不得包含：

```text
完整磁力链接
完整 URL
tracker URL
peer IP
完整用户路径
token
数据库敏感内容
```

例如不要：

```text
Opening magnet:magnet:?xt=urn:btih:FULL_HASH...
```

建议：

```text
launch_request type=magnet
```

必要时仅保留：

```text
redacted identifier
```

---

# 三十一、Telemetry

M8 继续保持：

```text
NO TELEMETRY BY DEFAULT
```

不得为了：

```text
统计多少次打开
```

```text
统计通知点击
```

```text
统计 tray 使用
```

```text
统计默认浏览器来源
```

而偷偷加入网络请求。

---

# 三十二、通知与隐私

通知正文不要泄漏过多本地信息。

避免：

```text
C:\Users\Someone\SecretProject\PrivateVideo.mkv 下载完成
```

更推荐：

```text
下载完成
PrivateVideo.mkv
```

但仍应允许用户关闭。

如果文件名本身可能非常敏感，不应写入日志。

---

# 三十三、M8 测试环境

必须准备至少两个环境：

```text
Development
Installed Windows Build
```

因为：

```text
Deep Link
Notification
File Association
Autostart
Installer
```

很多行为与：

```text
npm run tauri dev
```

不同。

---

# 三十四、Single Instance Test Matrix

测试：

```text
T1:
启动 Tide
再次启动 Tide
```

结果：

```text
只有一个实例
```

---

```text
T2:
Tide 已运行
启动 tide://...
```

结果：

```text
原实例收到请求
窗口激活
第二实例退出
```

---

```text
T3:
Tide 未运行
启动 tide://...
```

结果：

```text
Tide 启动
处理 deep link
```

---

```text
T4:
Tide 正在 Tray
第二次启动
```

结果：

```text
窗口恢复
正确处理请求
```

---

# 三十五、Torrent File Association Test Matrix

至少：

```text
valid .torrent
invalid .torrent
huge .torrent
missing .torrent
restricted file
UNC path
reparse point
duplicate open
same file opened repeatedly
```

验证：

```text
不会崩溃
不会创建重复任务
不会绕过验证
不会访问任意文件
```

---

# 三十六、Window State Test Matrix

至少：

```text
normal window
maximized
minimized
multi-monitor
monitor unplugged
display resolution changed
DPI changed
window moved partially off-screen
```

必须确保：

```text
Tide always recoverable
```

---

# 三十七、Tray Test Matrix

至少：

```text
left click
right click
double click
show
hide
close
exit
tray while downloading
tray while torrenting
tray with zero tasks
tray with 100+ tasks
scheduler paused
scheduler running
```

---

# 三十八、Autostart Test Matrix

至少：

```text
Autostart OFF
Autostart ON
toggle ON
toggle OFF
restart Windows
login
logout
remove/reinstall app
upgrade app later
```

验证：

```text
不会重复启动
不会产生僵尸实例
不会绕过 Single Instance
```

---

# 三十九、Notification Test Matrix

至少：

```text
permission granted
permission denied
permission unavailable
app foreground
app hidden
app tray-only
download completed
download failed
torrent completed
```

必须确保：

```text
Notification failure
≠
Download failure
```

---

# 四十、Launch Request Fuzz / Negative Testing

对：

```text
CLI args
Deep Link
URLs
Magnet
Torrent paths
```

做 malformed input 测试。

例如：

```text
empty
whitespace
very long string
invalid percent encoding
invalid UTF-8 boundary if applicable
unsupported scheme
nested URI
duplicated query
control characters
quotes
shell metacharacters
UNC paths
device paths
relative paths
path traversal
```

要求：

```text
no panic
no process spawn
no arbitrary command execution
no arbitrary filesystem access
no corrupted state
```

---

# 四十一、不要通过 Shell 解决问题

禁止：

```text
cmd.exe
powershell.exe
start.exe
explorer.exe
```

作为普通业务逻辑的通用执行后端。

例如：

不要：

```rust
Command::new("explorer")
    .arg(user_path)
```

除非：

```text
明确经过安全封装
```

并且当前官方 Tauri 能力不足。

优先：

```text
Tauri Opener
```

并通过 capability 限制范围。

---

# 四十二、文件打开安全

用户点击：

```text
Open File
```

必须确保：

```text
path 是已有文件
```

并且：

```text
属于合法 Library / Download artifact
```

不要出现：

```text
任意路径可由前端传给 opener
```

推荐：

```text
frontend:
    artifact_id

backend:
    artifact_id
    ↓
resolve actual path
    ↓
verify containment
    ↓
open
```

而不是：

```text
frontend:
    "C:\whatever\anything.exe"
```

---

# 四十三、目录打开安全

同理：

```text
Open Download Folder
```

不要前端直接传：

```text
C:\...
```

而应该：

```text
download_root_id
```

或者：

```text
well-known application location
```

由 backend resolve。

---

# 四十四、M8 前端

M8 前端不需要重新设计整个 UI。

重点是：

```text
Settings
```

增加必要选项：

```text
General
    Start with Windows
    Close to tray

Notifications
    Download completed
    Download failed
    Torrent completed
```

以及：

```text
Tray
```

相关行为。

---

# 四十五、来源接入 UI

当：

```text
.torrent
tide://
magnet
```

进入应用时：

统一进入：

```text
Add Source Flow
```

而不是为每种来源创建完全不同的页面。

例如：

```text
LaunchRequest
      ↓
Source Intake
      ↓
Validate
      ↓
Preview
      ↓
Confirm
      ↓
Create Task
```

这样未来：

```text
HTTP
Magnet
Torrent File
```

可以共用入口。

---

# 四十六、Launch Request 不等于 Task

非常重要：

```text
LaunchRequest
```

是：

> “Windows 要求 Tide 做什么”

而：

```text
Task
```

是：

> “Tide 持久化了什么工作”

二者不能混淆。

例如：

```text
用户双击 torrent
```

只是：

```text
LaunchRequest::OpenTorrentFile
```

用户最终确认以后才：

```text
Create Task
```

---

# 四十七、Idempotency

需要防：

```text
double click .torrent
```

导致：

```text
duplicate task
```

可以使用已有 M2/M3 的：

```text
source identity
resource identity
info hash
URL identity
```

进行幂等判断。

不要新建第二套重复检测算法。

---

# 四十八、Installer Uninstall 行为

卸载 Tide：

必须清理：

```text
application install files
startup registration
protocol registration
file associations
shortcut
```

但：

```text
用户下载文件
```

默认：

> **不得因为卸载应用而删除。**

数据库数据的处理方式必须明确记录。

建议：

```text
download data = user-owned
```

与：

```text
app data = app-owned
```

严格分离。

---

# 四十九、App Data / User Data

M8 继续保持：

```text
%LOCALAPPDATA%\ifuyo\Tide\
```

用于：

```text
SQLite
config
runtime
logs
window state
```

而：

```text
Downloads/
Media/
Torrent data/
```

保持用户数据边界。

卸载时不要误删用户内容。

---

# 五十、Installer Strategy

开始实施前明确：

```text
MSI
or
NSIS
```

为什么。

不允许：

```text
两个都做
```

然后没有任何明确用途。

M8 至少需要一个真实 Windows 安装器作为集成测试载体。

正式 Release 的 Installer Strategy / Code Signing 在 M10/M11 再冻结。

---

# 五十一、WebView2

M8 必须验证：

```text
开发模式
安装版本
首次安装
已有 WebView2
WebView2 状态异常
```

因为 Tide 的 UI / Player 基于 Windows WebView2。

不要在 M8 擅自切换：

```text
fixed runtime
offline installer
```

除非 M0.5 的结果已经明确要求。

---

# 五十二、性能要求

M8 目标不是“Windows API 越多越好”。

必须验证：

```text
Tray-only CPU
Hidden window CPU
Idle CPU
Idle memory
100 tasks
500 tasks
1000 tasks
```

重点确认：

```text
Tray timer
notification polling
window events
single instance IPC
frontend event listeners
```

不会产生：

```text
高频 polling
高 CPU
高内存
event storm
```

---

# 五十三、禁止过度轮询

禁止：

```text
setInterval(..., 100ms)
```

持续扫描：

```text
tasks
downloads
filesystem
scheduler
```

系统托盘必须使用：

```text
event-driven
```

或者合理频率的聚合 snapshot。

---

# 五十四、Windows Native API 使用原则

只有在：

```text
Tauri API
+
官方 Plugin
```

确实不足时，才考虑：

```text
windows-rs
Win32 API
Windows Registry
COM
Shell API
```

并且：

```text
使用最小 API 面
```

不要建立：

```text
Win32Utility.rs
```

几千行“万能 Windows 工具库”。

---

# 五十五、Windows Registry

如果必须直接操作 Registry：

只能用于真正需要的：

```text
file association
protocol association
startup
```

并且：

```text
HKCU
vs
HKLM
```

必须明确。

默认优先：

```text
per-user
```

避免强制管理员权限。

---

# 五十六、权限 / Admin

M8 目标：

> **普通用户安装、普通用户运行、普通用户可以使用完整核心功能。**

不要因为：

```text
file association
```

或：

```text
autostart
```

而让 Tide 默认要求：

```text
Administrator
```

---

# 五十七、第二实例传参安全

不能简单相信：

```text
secondInstanceArgs
```

必须：

```text
parse
validate
normalize
classify
```

例如：

```text
arg1 = "malicious input"
```

不能直接：

```text
execute
```

不能直接：

```text
open path
```

---

# 五十八、URI 大小限制

所有：

```text
tide://
magnet:
http://
https://
```

进入 Application Layer 前必须有：

```text
maximum input length
```

防止极端：

```text
multi-megabyte query string
```

占用大量内存。

---

# 五十九、Path 大小与深度

文件关联输入必须考虑：

```text
long path
long filename
deep path
UNC
```

不能因为：

```text
MAX_PATH
```

而出现异常。

同时不能为了“支持所有路径”而放弃：

```text
canonicalization
containment
```

---

# 六十、前端与后端职责

Frontend 负责：

```text
展示
用户交互
settings
```

Rust backend 负责：

```text
system integration
validation
filesystem
launch requests
platform API
```

Frontend 不能决定：

```text
是否允许 open path
是否允许 autostart
是否允许 association
```

---

# 六十一、事件模型

建议至少统一：

```text
app.launch_request
app.focus_requested
app.visibility_changed
app.tray_action
app.notification_requested
```

但不必机械创建大量事件。

优先：

```text
Command
Application Service
Event
```

三者职责清晰。

---

# 六十二、Tray Action 不直接调用底层 Engine

例如不要：

```text
tray.rs
   ↓
DownloadEngine.pause_all()
```

推荐：

```text
tray action
   ↓
Application Command
   ↓
PauseAll
   ↓
Scheduler / Task Service
```

这样：

```text
UI
Tray
CLI
Deep Link
```

可以共用业务入口。

---

# 六十三、M8 文档

新增：

```text
docs/M8-windows-integration.md
docs/M8-launch-routing.md
docs/M8-tray.md
docs/M8-file-association.md
docs/M8-lifecycle.md
docs/M8-security.md
docs/M8-test-matrix.md
docs/M8-report.md
```

如果仓库已有类似文档体系，应合并到现有结构，而不是制造重复文档。

---

# 六十四、必须写清楚的设计决策

文档必须明确：

```text
Single Instance Strategy
Launch Request Model
Deep Link Scheme
Torrent Association Strategy
Magnet Association Strategy
Media File Association Policy
Tray Close Semantics
Autostart Default
Autostart Hidden/Visible Behavior
Notification Default
Window State Strategy
Installer Type
Per-user / Per-machine
Uninstall Policy
App Data Policy
User Data Policy
```

---

# 六十五、Magnet Association 特别审慎

关于：

```text
magnet:
```

必须先做实际验证。

如果当前 Tauri Deep Link 能力适合：

```text
tide://
```

不等于：

```text
Windows magnet association
```

必须把二者区分。

如果决定支持：

```text
magnet:
```

需要单独记录：

```text
FACT
Windows registry behavior
Installer behavior
Existing default handler
Uninstall behavior
Per-user behavior
```

不能因为“技术上可以写 Registry”就直接实现。

---

# 六十六、当前 Tauri Plugin 的选择原则

优先使用当前项目实际锁定版本的官方能力：

```text
Single Instance
Autostart
Deep Link
Window State
Opener
Notification
```

不要：

```text
看到网上旧教程
```

然后使用：

```text
Tauri 1 API
```

或者旧版本 plugin API。

必须以：

```text
当前 Cargo.lock
当前 package.json
当前官方文档
```

为准。

---

# 六十七、依赖管理

不要为了 M8：

```text
升级整个 Tauri
```

除非：

```text
当前版本无法完成 M8
```

如果需要升级：

必须先输出：

```text
Current Version
Target Version
Reason
Breaking Changes
Affected APIs
Test Impact
Migration Plan
```

否则：

> **保持版本不动。**

---

# 六十八、Cargo.lock

继续要求：

```text
cargo check --locked
cargo test --locked
```

前端：

```text
npm/pnpm/yarn lockfile
```

必须保持一致。

---

# 六十九、Unit Tests

至少覆盖：

```text
LaunchRequest parsing
URL classification
Magnet classification
Torrent path classification
Media path classification
Invalid argument rejection
Deep Link parsing
Single Instance routing
Settings validation
Notification policy
Autostart configuration model
Window state normalization
Monitor bounds validation
```

---

# 七十、Integration Tests

至少包含：

```text
First launch
Second launch
Second launch with URL
Second launch with magnet
Second launch with torrent
Deep link while app closed
Deep link while app running
Torrent association
Tray
Hide
Show
Exit
Autostart
Window restore
Open folder
Open file
Notification
Uninstall cleanup
```

---

# 七十一、Crash / Failure Tests

必须模拟：

```text
tray initialization failure
notification permission failure
window state corrupted
autostart unavailable
association registration failure
deep link malformed
second instance forwarding failure
existing instance unavailable
```

要求：

```text
单个 Windows Integration 功能失败
≠
整个 Tide 启动失败
```

例如：

```text
Notification 初始化失败
```

不应该：

```text
Tide 无法启动
```

---

# 七十二、Degraded Mode

如果：

```text
Notification
```

不可用：

```text
Tide 仍然正常下载
```

如果：

```text
Autostart
```

不可用：

```text
Tide 仍然正常运行
```

如果：

```text
Window State
```

损坏：

```text
恢复默认窗口
```

如果：

```text
Tray
```

失败：

```text
Main Window 仍可运行
```

---

# 七十三、启动顺序

必须形成清晰的 startup lifecycle：

```text
Process Start
    ↓
Single Instance
    ↓
Parse Initial Launch Args
    ↓
Initialize Core App State
    ↓
Open Persistence
    ↓
M2 Recovery
    ↓
Restore M7 Scheduler
    ↓
Initialize Windows Integration
    ↓
Create / Restore Main Window
    ↓
Register Tray
    ↓
Register Deep Link
    ↓
Prepare Notification
    ↓
Frontend Ready
    ↓
Dispatch Pending LaunchRequest
```

实际顺序可调整，但必须明确：

> **不能让 Deep Link / Second Instance Request 在 Application 尚未准备好时丢失。**

---

# 七十四、Pending Launch Request

如果：

```text
Windows 在 Tide 尚未完全启动时
```

收到：

```text
Deep Link
```

不能丢失。

应有：

```text
pending_launch_requests
```

或者等价机制。

例如：

```text
Process started
    ↓
capture launch request
    ↓
store in memory queue
    ↓
core ready
    ↓
dispatch
```

---

# 七十五、如果用户同时传多个请求

例如异常情况下：

```text
tide://add?... 
file1.torrent
file2.torrent
```

必须定义：

```text
是否支持多个
还是只支持一个
```

不要出现：

```text
第一个被处理
第二个静默丢弃
```

如果当前业务不需要多请求：

明确拒绝并记录可诊断错误。

---

# 七十六、Focus 行为

当：

```text
Second Instance
```

或：

```text
Deep Link
```

触发时：

必须让用户看到：

```text
Tide
```

处理：

```text
minimized
hidden
tray-only
covered window
```

都应能：

```text
restore
activate
focus
```

---

# 七十七、DPI / 多显示器

Windows 11 环境必须至少人工验证：

```text
100%
125%
150%
200%
```

以及：

```text
single monitor
dual monitor
mixed DPI
```

重点不是做视觉重构，而是：

```text
window restore
tray position
dialog behavior
```

不异常。

---

# 七十八、Tray Menu 与本地化

Tray 文本必须进入现有：

```text
i18n
```

不要：

```text
tray.rs 里硬编码所有中文
```

至少支持当前产品已有语言。

---

# 七十九、Notification 文案与本地化

同理：

```text
Download completed
Download failed
Torrent completed
```

必须通过现有 localization 系统。

不要散落英文常量。

---

# 八十、Settings Persistence

Windows Integration 设置：

```text
start_with_windows
close_to_tray
notification_download_complete
notification_download_failed
notification_torrent_complete
```

必须使用现有持久化机制。

如果当前项目已经有：

```text
SettingsRepository
```

继续复用。

不要因为“这些只是 UI 设置”再单独制造新的配置存储体系。

---

# 八十一、不要滥用 Tauri Store

如果项目当前：

```text
SQLite already handles application persistence
```

那么不要把：

```text
Task
Scheduler
Library
```

放进 Store。

Tauri Store 适合轻量持久化配置，但不能替代已有 SQLite 领域持久化。

---

# 八十二、安装/运行路径分离

必须确认：

```text
Tide executable
```

与：

```text
Download Root
```

完全无关。

用户可以：

```text
Tide installed on C:
downloads on D:
```

甚至：

```text
downloads on another volume
```

Windows Integration 不得假定：

```text
download folder = install folder
```

---

# 八十三、Installer Smoke Test

生成安装包后验证：

```text
Install
   ↓
Launch
   ↓
Create test task
   ↓
Restart
   ↓
Tray
   ↓
Deep Link
   ↓
Torrent association
   ↓
Autostart
   ↓
Uninstall
```

至少完成一轮端到端测试。

---

# 八十四、Uninstall 安全

尤其测试：

```text
Install path
AppData
Download path
Torrent data
Completed files
Library metadata
```

验证：

> 卸载 Tide 不会误删除用户下载内容。

---

# 八十五、Windows Event / Logging

需要检查：

```text
Windows Event Viewer
```

当发生：

```text
startup failure
crash
integration failure
```

是否能够定位。

但不要为此引入完整远程 crash-reporting。

---

# 八十六、Release Hardening 的边界

M8 只需要：

```text
可安装
可验证
可使用
```

不要提前完成：

```text
代码签名
证书轮换
自动更新
正式发布服务器
Release CDN
```

这些保留给：

```text
M10 Security & Release Hardening
M11 Release Candidate
```

---

# 八十七、静态检查

必须运行：

```text
cargo fmt --check
cargo clippy --locked --all-targets --all-features
cargo check --locked
cargo test --locked
```

前端：

```text
typecheck
lint
build
```

使用项目实际存在的命令。

---

# 八十八、最终人工验证

必须真人操作：

## Scenario A

```text
启动 Tide
↓
开始真实下载
↓
最小化到 Tray
↓
等待
↓
下载继续
```

---

## Scenario B

```text
Tide 在 Tray
↓
双击 torrent
↓
Tide 被激活
↓
出现 torrent 处理流程
```

---

## Scenario C

```text
Tide 已运行
↓
从系统触发 tide://
↓
已有实例收到请求
↓
没有第二实例
```

---

## Scenario D

```text
Tide 已运行
↓
关闭窗口
↓
进入 Tray
↓
下载继续
↓
再次点击 Tray
↓
窗口恢复
```

---

## Scenario E

```text
Autostart ON
↓
重启 Windows
↓
Tide 自动启动
↓
进入 Tray
↓
Scheduler 正常工作
```

---

## Scenario F

```text
双屏
↓
将窗口移动到第二屏
↓
退出
↓
拔掉第二屏
↓
再次启动
↓
窗口仍然可见
```

---

## Scenario G

```text
下载完成
↓
应用处于后台
↓
收到 Windows notification
↓
点击通知
↓
Tide 激活
```

---

# 八十九、M8 安全验收

必须完成：

```text
S1:
Deep Link 不允许任意协议

S2:
CLI 不允许任意命令执行

S3:
File Association 不允许任意路径访问

S4:
Opener 不允许任意 shell command

S5:
Second Instance 不允许重复后台 runtime

S6:
Notification failure 不影响核心业务

S7:
Autostart failure 不影响核心业务

S8:
Corrupt Window State 不导致启动失败

S9:
Uninstall 不删除 user download data

S10:
Logs 不泄漏 URL / magnet / peer IP / sensitive path
```

---

# 九十、M8 性能验收

至少测：

```text
Idle Tray
Idle Main Window
100 tasks
500 tasks
1000 tasks
3 concurrent HTTP tasks
multiple Torrent tasks
Scheduler running
```

观察：

```text
CPU
Memory
IPC frequency
Event frequency
Tray update frequency
Startup time
Second-instance handoff latency
```

不能为了 Windows Integration 引入明显：

```text
CPU spin
polling
event storm
memory leak
```

---

# 九十一、文档必须给出架构图

至少：

```text
Windows
 ├── CLI / File Association
 ├── Deep Link
 ├── Tray
 ├── Autostart
 ├── Notification
 └── Window Manager
           ↓
   Windows Integration
           ↓
     Application Layer
           ↓
   ┌───────┼────────┐
   │       │        │
 Task   Scheduler  Playback
   │       │        │
   └───────┼────────┘
           ↓
       Persistence
```

---

# 九十二、M8 Definition of Done

只有同时满足以下条件，才算 M8 完成：

## Core

```text
[ ] Single Instance
[ ] Second Instance Forwarding
[ ] Unified LaunchRequest
[ ] Deep Link
[ ] Torrent File Association
[ ] Tray
[ ] Show / Hide / Focus
[ ] Close-to-Tray
[ ] Window State
[ ] Multi-monitor Recovery
[ ] Autostart
[ ] Notification
[ ] Native Open
[ ] Show in Explorer
[ ] Installer Integration
```

## Security

```text
[ ] External launch input validation
[ ] Path validation
[ ] URI validation
[ ] No arbitrary shell execution
[ ] Minimal capabilities
[ ] No sensitive logging
[ ] No user-data deletion on uninstall
```

## Reliability

```text
[ ] No duplicate runtime
[ ] No lost LaunchRequest
[ ] Tray failure graceful
[ ] Notification failure graceful
[ ] Autostart failure graceful
[ ] Corrupt window state graceful
[ ] Windows shutdown graceful
```

## Persistence

```text
[ ] Existing M2 recovery remains correct
[ ] Existing M7 Scheduler remains correct
[ ] Windows lifecycle does not break recovery
```

## Quality

```text
[ ] Unit Tests
[ ] Integration Tests
[ ] Negative Tests
[ ] Manual Windows Tests
[ ] Installer Tests
[ ] Multi-monitor Tests
[ ] Autostart Tests
[ ] Deep Link Tests
[ ] Tray Tests
```

---

# 九十三、P0 / P1 标准

## P0

只有以下问题可以判定 P0：

```text
Tide 无法启动
Single Instance 失效并导致数据竞争
第二实例导致重复 Scheduler / DownloadEngine
Deep Link 导致任意命令执行
File Association 导致任意文件访问
Uninstall 删除用户下载数据
Windows Integration 破坏 M2 recovery
后台模式导致下载任务错误终止
```

---

## P1

例如：

```text
Tray 偶发不刷新
Notification 不稳定
窗口恢复位置异常
Autostart 部分机器失败
installer 某配置问题
secondary launch 偶发延迟
少量 DPI edge case
```

---

# 九十四、最终报告格式

完成实现后必须输出：

# M8 Report

## 1. Executive Summary

回答：

```text
M8 是否完成
完成了哪些 Windows Integration
还有哪些限制
```

---

## 2. FACTS

---

## 3. HYPOTHESES

---

## 4. INFERENCES

---

## 5. Current Architecture

说明：

```text
Windows Integration
Application
Core
Persistence
Scheduler
Frontend
```

之间的关系。

---

## 6. Single Instance

说明：

```text
mechanism
startup
handoff
failure handling
```

---

## 7. LaunchRequest

给出完整模型。

---

## 8. Deep Link

包括：

```text
scheme
examples
validation
startup behavior
running behavior
```

---

## 9. File Association

说明：

```text
.torrent
other associations
installer
uninstaller
```

---

## 10. Tray

---

## 11. Autostart

---

## 12. Window State

---

## 13. Notification

---

## 14. Native Open

---

## 15. Installer

---

## 16. Security Audit

使用：

```text
Threat
Attack Surface
Mitigation
Test
Result
```

---

## 17. Test Matrix

必须给出：

```text
Test
Expected
Actual
Pass/Fail
```

---

## 18. Performance

---

## 19. Files Changed

---

## 20. Dependencies Added

---

## 21. RESOLVED

---

## 22. CONFIRMED

---

## 23. OPEN

---

## 24. WORSENED

---

## 25. P0

---

## 26. P1

---

## 27. M9 Readiness

说明：

```text
是否具备进入 Polish
```

以及：

```text
哪些问题应该留给 M9
```

---

## 28. M10/M11 Deferred Items

明确哪些功能：

```text
不要在 M8 做
```

---

# 九十五、最终禁止越界

M8 完成后不要继续做：

```text
M9 Polish
M10 Security & Release Hardening
M11 Release Candidate
M12 1.0
```

特别是：

```text
不要顺手加入自动更新
不要顺手加入 Steam
不要顺手加入云同步
不要顺手加入账号
不要顺手加入遥测
不要顺手加入主题商城
不要顺手加入资源搜索
不要顺手加入资源站
```

---

# 九十六、最终验收链

M8 必须最终证明：

```text
Windows
   ↓
Launch Request
   ↓
Single Instance
   ↓
Application Layer
   ↓
Task / Scheduler / Playback
```

同时：

```text
Windows
   ├── Tray
   ├── Autostart
   ├── Notification
   ├── Deep Link
   ├── File Association
   ├── Native Open
   └── Window State
```

都只是：

> **对现有 Tide 核心能力的 Windows 原生入口与宿主集成。**

不能形成第二套业务架构。

---

# 九十七、M8 最终成功标准

最终应该达到这样的体验：

```text
用户启动 Tide
        ↓
像普通 Windows 软件一样启动
        ↓
只有一个实例
        ↓
任务开始运行
        ↓
关闭窗口
        ↓
Tide 留在 Tray
        ↓
任务继续
        ↓
完成时可选通知
        ↓
双击 .torrent
        ↓
Tide 自动接管
        ↓
已有实例被激活
        ↓
用户确认后创建任务
        ↓
重启 Windows
        ↓
Tide 可选自动启动
        ↓
M2 恢复 + M7 Scheduler 继续工作
```

这才是 M8 的完成状态。

---

# 九十八、实施纪律

必须始终遵守：

```text
1. Inspect first
2. FACT / HYPOTHESIS / INFERENCE 分离
3. 不假定 M0-M7 已实现
4. 复用现有 Application Layer
5. 不重复实现业务逻辑
6. 外部输入全部视为 untrusted
7. Windows API 最小化
8. 官方 Tauri plugin 优先
9. Capability 最小权限
10. 不用 Shell 绕过安全边界
11. 不修改 M2 Recovery semantics
12. 不修改 M7 Scheduler semantics
13. 不制造第二套 Task state machine
14. 不制造第二套 persistence
15. 不引入 telemetry
16. 不引入 cloud
17. 不进入 M9+
18. 所有真实功能必须有测试
19. 所有 Windows-specific 行为必须至少做一次安装后人工验证
20. 完成 M8 后停止并输出报告
```

---

# 九十九、最后的核心原则

请始终记住：

> **M8 不是“给 Tide 加一堆 Windows 小功能”。**

而是：

> **建立一个干净的 Windows Host Integration Boundary。**

最终结构应稳定在：

```text
                ┌──────────────────────────┐
                │       Windows OS         │
                │                          │
                │ Tray / Startup / URI     │
                │ File Association / etc.  │
                └────────────┬─────────────┘
                             │
                             ▼
                ┌──────────────────────────┐
                │ WindowsIntegration Layer │
                └────────────┬─────────────┘
                             │
                             ▼
                ┌──────────────────────────┐
                │    Application Layer     │
                └────────────┬─────────────┘
                             │
             ┌───────────────┼────────────────┐
             ▼               ▼                ▼
          Task/Core       Scheduler        Playback
             │               │                │
             └───────────────┼────────────────┘
                             ▼
                      Persistence/Core
```

**实现完成后停止，不要自行进入 M9。**
