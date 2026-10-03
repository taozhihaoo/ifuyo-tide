# ifuyo Tide — M11 Release Candidate / RC 完整开发实施提示词

## 0. 任务定位

你现在正在实现：

> **ifuyo Tide — M11 Release Candidate（RC）**

M11 不是一个“继续开发功能”的 Milestone。

M11 的任务是：

> **把 M0–M10 已经完成的 Tide，冻结为一个真实可安装、可运行、可验证、可发布、可升级、可卸载、可回归测试的 Release Candidate。**

M11 的核心目标不是增加功能数量，而是验证：

```text
Source
  ↓
Task / Resource
  ↓
Download Engine / Torrent Engine
  ↓
Persistence / Recovery
  ↓
Availability / Scheduler
  ↓
Progressive Streaming
  ↓
Player
  ↓
Library
  ↓
Windows Integration
  ↓
Security Hardening
  ↓
Installer / Package
  ↓
Installed Application
  ↓
Real User Workflow
  ↓
Release Candidate
```

M11 必须回答：

> **“如果今天把这个 RC 版本交给一台没有开发环境的干净 Windows 机器，它是否能够像一个真正的软件一样被安装、启动、使用、关闭、升级、卸载，并且不会破坏用户下载数据和应用数据？”**

---

# 1. 极其重要的执行纪律

## 1.1 第一原则：先审计，再修改

不要直接修改代码。

首先完整检查：

* 当前 repository 状态
* 当前 branch
* 当前 commit
* M0–M10 已完成内容
* 当前 Cargo / Node / Tauri / Rust / WebView2 环境
* 当前版本号
* 当前构建配置
* 当前 installer 配置
* 当前 updater 状态
* 当前 capabilities
* 当前 CSP
* 当前 SQLite migrations
* 当前 app-data 路径
* 当前 download-data 路径
* 当前日志系统
* 当前 crash/recovery 机制
* 当前 HTTP/Torrent/Streaming/Player/Library/Windows Integration 实现
* 当前测试状态
* 当前 CI/CD
* 当前 release scripts
* 当前签名配置
* 当前第三方依赖与许可证清单
* 当前 README / Privacy / Security / Content Policy / Third Party Notices
* 当前所有已知 P0/P1 问题

禁止凭空假设 M0–M10 已经完成。

---

# 2. 开始前必须输出审计结果

在任何代码修改前，先输出：

```text
FACTS
HYPOTHESES
CURRENT_STATE
M0–M10 COMPLETION AUDIT
M11 GAPS
RELEASE RISKS
IMPLEMENTATION PLAN
```

必须明确区分：

```text
FACT
= 代码 / 构建 / 测试 / 实测可以证明

HYPOTHESIS
= 尚未实测，只能合理推测

INFERENCE
= 根据事实推导出的结论
```

不要把：

```text
“理论上能安装”
```

写成：

```text
“安装已验证”
```

不要把：

```text
“CI 可以构建”
```

写成：

```text
“真实用户机器已验证”
```

---

# 3. M11 核心范围

M11 只处理：

```text
Release Candidate
Build
Package
Installer
Signing
Install
Upgrade
Uninstall
Data Preservation
Migration
Release Validation
Clean-Machine Validation
Regression
Documentation
Release Artifact Integrity
```

核心验证对象：

```text
开发环境
    ↓
Release Build
    ↓
Installer / Package
    ↓
干净 Windows 环境
    ↓
安装
    ↓
启动
    ↓
真实工作流
    ↓
关闭 / 重启
    ↓
升级
    ↓
再次启动
    ↓
数据保持
    ↓
卸载
    ↓
残留数据策略验证
    ↓
重新安装
    ↓
最终状态验证
```

---

# 4. 明确禁止在 M11 新增的大功能

除非当前 M0–M10 已经存在且只是 RC 验证需要，否则禁止新增：

* 新的下载协议
* 新 Torrent 协议能力
* 新播放器后端
* 新媒体格式架构
* NAS
* UPnP
* mDNS
* Casting
* VPN
* 云同步
* 账号系统
* 登录
* 社交
* 在线资源发现
* 搜索引擎
* 资源推荐
* 资源排行榜
* Tracker 推荐
* 匿名下载模式
* DRM bypass
* paywall bypass
* 盗版资源导航
* 第三方资源站索引
* ifuyo 云服务
* 默认 telemetry
* 新的复杂插件系统
* Steam 集成
* Mod Workshop
* 大规模 UI redesign
* 新的核心调度算法

M11 原则：

> **Fix > Validate > Package > Release**

而不是：

> **Add > Expand > Refactor > Release**

---

# 5. M11 Release Candidate 的定义

RC 不是普通 Beta。

必须定义一个明确的：

```text
RC Version
RC Build ID
RC Commit
RC Build Timestamp
Target Platform
Toolchain Version
Dependency Lock State
```

建议统一输出：

```text
Tide version
Tauri version
Rust version
Cargo version
Node version
Package manager version
WebView2 prerequisite / runtime assumption
Windows target
Architecture = x64
Commit SHA
Build profile = release
```

所有版本号来源必须尽可能单一。

禁止：

```text
package.json = 1.0.0
tauri.conf.json = 1.0.0
Cargo.toml = 0.9.8
UI = 1.0
```

这种情况必须修复成单一可信版本来源或自动同步机制。

---

# 6. Release Candidate 构建矩阵

确认当前项目实际选择的 Windows 发布格式。

Tauri 2 的 Windows 发布通常包含：

```text
NSIS installer
MSI installer
```

是否同时发布由项目实际需求决定。

不要为了“看起来专业”机械地同时维护多个安装器。

首先判断：

```text
当前 Tide 最终面向普通 Windows 用户的主要安装方式是什么？
```

然后确定：

```text
Primary Installer
Secondary Installer（如有）
Portable Artifact（如确有需求）
```

对于每一个发布产物，必须记录：

```text
filename
version
architecture
size
SHA-256
signature status
build source commit
```

---

# 7. Release Build 基线

Release Build 必须使用：

```text
locked dependencies
release profile
clean build verification
```

至少验证：

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked
```

以及项目实际使用的：

```text
npm / pnpm / yarn / bun
```

对应：

```text
lint
typecheck
test
build
tauri build
```

实际命令以项目现有配置为准。

禁止为了通过构建而：

```text
删除测试
关闭 warning
降低 lint
移除安全检查
跳过 migration
绕过 signing
修改测试预期
硬编码环境路径
```

---

# 8. Release 构建必须可复现

检查：

```text
Cargo.lock
package-lock / pnpm-lock / yarn.lock / bun.lock
```

是否存在并被正确追踪。

验证：

```text
同一个 commit
+ 同一依赖锁定状态
+ 同一构建环境
≈ 同一 release artifact
```

不要求二进制严格 bit-for-bit identical，除非当前 pipeline 已支持。

但至少必须保证：

```text
版本明确
commit 明确
依赖明确
构建流程明确
release artifact 来源可追踪
```

---

# 9. Build Artifact Hygiene

检查 release 输出目录。

禁止把以下内容误带入最终 release：

```text
debug binary
tests
source map（除非明确需要）
development server
temporary files
.editorconfig
.git
.env
private credentials
certificate private key
API key
developer logs
local database
test downloads
magnet samples
personal paths
```

尤其检查：

```text
*.pfx
*.key
*.pem
.env
credentials
signing keys
private updater keys
```

任何私钥进入 repository 或 release artifact 都视为：

```text
P0 Security Issue
```

---

# 10. Windows Installer 验证

必须实际生成 installer，而不是只执行应用 build。

至少验证：

```text
Installer launches
Installer completes
Application installed
Application appears correctly
Application starts
Application exits
Uninstaller exists
Uninstall works
```

同时检查：

```text
安装目录
开始菜单
桌面快捷方式（如果配置）
Uninstall registration
Application identity
Publisher metadata
Version metadata
Executable metadata
```

不要假设 installer UI 正确。

必须在真实 Windows 环境验证。

---

# 11. 安装路径测试

至少测试：

```text
默认安装路径
包含空格的路径
非默认磁盘路径（如果 installer 支持）
```

如果项目明确限制安装目录，则写入文档并测试。

重点验证：

```text
Installer Path
≠
App Data Path
≠
Download Root
```

三者必须保持清晰边界。

---

# 12. 用户数据必须与应用安装目录分离

必须确认：

```text
安装程序升级 / 卸载
```

不会无意删除：

```text
用户下载文件
任务数据库
Library 数据
用户设置
恢复信息
```

但这里必须区分：

```text
Application Data
User Download Data
```

是否保留应用数据由产品策略决定。

不要简单地：

```text
uninstall => delete everything
```

必须设计明确的：

```text
Code
App Data
User Data
Cache
Temp Data
```

生命周期。

---

# 13. App Data / Download Data 生命周期审计

明确记录每一类文件：

| 类型                 | 示例               | 安装               | 升级     | 卸载   | 重装    |
| ------------------ | ---------------- | ---------------- | ------ | ---- | ----- |
| Application Binary | Tide.exe         | 创建               | 替换     | 删除   | 重建    |
| SQLite DB          | tasks.db         | 创建               | 保留/迁移  | 按策略  | 按策略恢复 |
| User Downloads     | *.mp4 / *.mkv    | 不应受 installer 控制 | 保留     | 通常保留 | 保留    |
| Cache              | temp/cache       | 创建               | 可清理    | 清理   | 重建    |
| `.part`            | partial download | 创建               | 保留     | 按策略  | 恢复/保留 |
| Logs               | log files        | 创建               | 可保留/轮转 | 按策略  | 重建    |

表格必须根据真实实现填写。

---

# 14. 首次安装验证

测试：

```text
完全干净机器
    ↓
安装 RC
    ↓
首次启动
    ↓
初始化 App Data
    ↓
初始化 SQLite
    ↓
初始化必要目录
    ↓
进入主界面
```

验证：

```text
无 panic
无崩溃
无 migration 错误
无权限异常
无路径异常
无缺失资源
无 dev server 依赖
无本机开发环境依赖
```

特别注意：

> 应用不能依赖开发机上某个已经存在的目录、环境变量、缓存、证书、Node、Rust、FFmpeg、VLC、Python 等开发环境组件。

---

# 15. 干净 Windows 机器测试

这是 M11 最重要的工作之一。

不要只在开发机验证。

至少准备：

```text
Clean Windows x64 Environment
```

尽可能接近普通用户环境。

验证：

```text
Install
Launch
Download
Pause
Resume
Torrent
Streaming
Seek
Library
Restart
Recovery
Settings
Uninstall
Reinstall
```

实际范围以 M0–M10 最终功能为准。

所有“真实用户路径”至少走一遍。

---

# 16. Administrator / Non-Administrator 测试

默认情况下优先验证普通用户权限。

测试：

```text
普通用户启动
普通用户下载
普通用户写入 Download Root
普通用户安装（依据 installer 权限策略）
普通用户升级
普通用户卸载
```

如果某个操作必须管理员权限：

必须明确：

```text
为什么需要
什么时候需要
Windows 如何表现
失败后如何提示
```

禁止通过：

```text
Run as Administrator
```

粗暴掩盖权限架构问题。

---

# 17. 下载数据真实验证

至少执行完整真实任务：

```text
HTTP Download
```

验证：

```text
Create Task
Start
Progress
Pause
Resume
Completion
File Exists
File Size
Checksum（如果配置）
Open File
```

然后：

```text
Restart Application
```

再次验证。

---

# 18. Torrent 真实验证

使用：

```text
legal / public-domain / self-generated / authorized test content
```

禁止在测试中寻找或提供盗版资源站。

测试：

```text
Torrent Metadata
File Selection
Start
Download
Availability
Partial State
Pause
Resume
Restart
Recovery
Completion
```

多文件 Torrent 必须再次验证：

```text
目标文件边界
路径安全
任务恢复
文件完整性
```

---

# 19. Progressive Streaming RC 验证

这是 Tide 的核心产品能力之一。

必须验证：

```text
Resource
    ↓
Availability
    ↓
Demand
    ↓
Scheduler
    ↓
Read Range
    ↓
Local Gateway
    ↓
WebView2 Player
```

至少进行：

```text
Play while downloading
Pause
Resume
Seek forward
Seek backward
Seek into available region
Seek into unavailable region
Wait for availability
Continue playback
```

必须确认：

> 播放器没有因为“文件尚未 100% 下载”而错误地认为文件损坏。

---

# 20. Progressive Stream Gateway 回归

重点验证：

```text
loopback only
session token
Host validation
Range validation
resource mapping
request limits
timeout
cleanup
```

禁止：

```text
http://localhost:<port>/C:/Users/...
```

之类任意文件读取路径。

验证：

```text
正常资源
无效 token
过期 token
错误 Host
非法 Range
超大 Range
负值
整数溢出
不存在 Resource
已删除 Resource
已关闭 Session
```

所有安全测试必须通过。

---

# 21. Playback Security Regression

媒体文件属于：

```text
Untrusted Input
```

再次检查：

```text
异常媒体文件
损坏媒体文件
超大媒体
错误扩展名
错误 MIME
空文件
特殊文件名
Unicode 文件名
超长文件名
```

验证：

```text
不会导致应用崩溃
不会执行任意文件
不会访问任意路径
不会突破 Gateway 权限边界
```

---

# 22. SQLite Migration Release Test

这是 RC 阶段的 P0/P1 重点。

必须模拟：

```text
Old Version DB
    ↓
Install RC
    ↓
Launch
    ↓
Migration
    ↓
Data Preserved
```

至少准备：

```text
empty DB
M0-compatible state
M1-compatible state
M2-compatible state
...
latest pre-RC DB
```

实际以项目 migration 历史为准。

验证：

```text
migration successful
migration idempotent
no data loss
no duplicate migration
no corrupt database
```

---

# 23. 不允许修改既有 migration 历史

禁止：

```text
修改已经提交的 migration
删除旧 migration
重新编号
偷偷重写历史 migration
```

正确方式：

```text
新增 migration
```

除非项目当前还没有任何真实 schema history。

---

# 24. Upgrade Test

必须测试：

```text
Previous Release
    ↓
Install RC / Upgrade RC
    ↓
Launch
    ↓
Existing Tasks
    ↓
Existing Settings
    ↓
Existing Library
    ↓
Existing Downloads
```

必须验证：

```text
已完成任务仍然完成
暂停任务仍然暂停
失败任务仍然保持失败语义
取消任务不会莫名重新开始
`.part` 不丢失
已下载文件不丢失
Library 不丢失
设置不丢失
```

---

# 25. In-Progress Download Upgrade Test

这是关键测试。

构造：

```text
Task Running
↓
Partial .part
↓
Application Version A
```

然后：

```text
关闭 A
↓
安装 RC
↓
启动 RC
```

必须验证：

```text
Task restored correctly
.part recognized
safe resume point correct
validator correct
Range resume correct
no duplicate bytes
no corruption
```

尤其测试：

```text
DB downloaded_bytes
<
>
=
physical .part size
```

三种情况。

---

# 26. Crash Recovery Regression

再次验证 M2 的 crash consistency。

故意在不同边界制造退出：

```text
AfterWrite
AfterFlush
BeforeCheckpoint
AfterCheckpoint
BeforeFinalize
AfterFinalizeBeforeDBUpdate
```

启动 RC。

必须保证：

```text
不会从超过安全持久化位置继续下载
不会重复破坏文件
不会跳过缺失字节
不会把 incomplete 标成 complete
```

---

# 27. Finalization Regression

验证：

```text
.part
    ↓
flush
    ↓
checksum
    ↓
atomic finalization
    ↓
final file
    ↓
Completed
```

模拟：

```text
final file exists
.part exists
both exist
neither exists
DB says complete but final file missing
DB says incomplete but final file complete
```

系统必须有确定性的 recovery decision。

---

# 28. Restart / Shutdown Test

分别测试：

```text
Graceful Exit
Window Close
Tray Close（如果存在）
OS Shutdown
OS Restart
Process Kill
Force Termination
```

验证：

```text
Task state
Partial file
DB checkpoint
Recovery
```

不要把：

```text
graceful shutdown
```

当成唯一可靠性来源。

正确目标是：

> Crash 也应该安全恢复；graceful exit 只是优化体验。

---

# 29. Offline / Network Failure Regression

测试：

```text
No network
Network disappears
Network returns
DNS failure
Connection reset
Timeout
Server 5xx
Server 429
Range unavailable
Torrent peer temporarily unavailable
```

必须验证：

```text
Task state stable
retry bounded
no busy loop
partial data preserved
resume safe
UI error understandable
```

---

# 30. Disk Failure Regression

测试：

```text
Insufficient disk space
Write failure
Permission denied
Drive unavailable
Download directory deleted
Destination moved
```

验证：

```text
Task enters correct state
error is structured
partial data policy is correct
database remains consistent
app does not crash
```

---

# 31. 路径安全 Release Regression

再次验证 Windows 特殊路径：

```text
..
.
CON
PRN
AUX
NUL
COM1
LPT1
UNC path
absolute path
relative path
junction
reparse point
symlink
trailing dot
trailing space
reserved names
very long paths
Unicode paths
emoji paths
Chinese paths
```

实际支持范围必须根据 M10 的安全设计确认。

不能因为 RC 阶段“正常用户不会这么做”就跳过。

---

# 32. HTTP / Torrent Source Security Regression

重新验证：

```text
URL validation
redirect validation
scheme validation
IP resolution validation
SSRF protection
path containment
torrent file path sanitization
magnet parsing
```

特别测试：

```text
合法 URL
恶意 URL
localhost
127.0.0.1
private IPv4
IPv6 loopback
IPv6 private/link-local
redirect → private IP
redirect → unsupported scheme
```

数据库中保存的 source 不能绕过新的安全策略。

---

# 33. Security Configuration Release Audit

检查 Tauri capabilities。

原则：

> **只给 RC 必须的权限。**

重新审计：

```text
filesystem
shell
process
http
dialog
window
notification
clipboard
path
updater
```

任何未使用权限：

```text
remove
```

而不是留下：

```text
future use
```

---

# 34. Frontend / Rust Trust Boundary Audit

再次确认：

```text
Frontend
≠
Security Authority
```

Frontend validation 只能改善 UX。

真正验证必须在 Rust backend。

重点检查：

```text
task ID
path
URL
range
resource ID
session token
file selection
download destination
source type
```

不能因为：

```text
React 已经验证
```

就省略：

```text
Rust validation
```

---

# 35. Gateway / IPC Release Audit

检查：

```text
Tauri IPC
Local Gateway
```

是否存在：

```text
arbitrary invocation
missing authorization
missing input validation
unbounded request
unbounded memory
unbounded concurrency
panic
leaked internal error
sensitive logs
```

所有对前端暴露的 API 必须稳定。

---

# 36. Logging Release Audit

Release 日志必须避免暴露：

```text
完整 URL
完整 Magnet
Info Hash（除非产品明确需要）
Peer IP
完整本地路径
Session Token
Authentication credential
Private key
```

日志应该优先使用：

```text
task_id
resource_id
short error category
operation
state
timestamp
```

必要时使用脱敏值。

---

# 37. Error Message Release Audit

用户看到的错误不能直接泄露：

```text
Rust stack trace
filesystem internals
network internals
private path
security tokens
raw HTTP internals
```

后台日志可以保留必要诊断信息，但必须按 Release logging policy 执行。

用户错误应该类似：

```text
下载失败
服务器暂时不可用
无法写入下载目录
文件已发生变化
恢复失败
媒体暂时无法播放
```

而不是把内部错误对象原样 dump 给用户。

---

# 38. UX Freeze

M11 允许：

```text
bug fix
misleading state fix
crash fix
accessibility fix
installer-related UI fix
release-critical UI fix
```

禁止：

```text
large redesign
new navigation architecture
new product concept
new feature discovery system
new customization ecosystem
```

原则：

> RC 阶段产品行为必须趋于冻结。

---

# 39. UI 状态必须是真实状态

重新检查：

```text
Downloading
Paused
Recovering
Waiting
Streaming
Buffering
Completed
Failed
Cancelled
```

不能存在假数据：

```text
fake speed
fake progress
fake buffer
fake ETA
fake completion
```

任何显示数字必须来自真实 backend state。

---

# 40. RC 视觉回归

检查：

```text
主窗口
下载列表
任务详情
播放器
Library
Settings
错误状态
空状态
首次启动
长文件名
长任务名
中文
英文
高 DPI
125%
150%
200%
```

至少验证：

```text
没有明显布局破坏
没有文字溢出
没有按钮失效
没有滚动锁死
没有窗口尺寸异常
```

---

# 41. Windows 高 DPI / 多显示器

至少测试：

```text
100%
125%
150%
200%
双显示器
不同 DPI 显示器
移动窗口
最大化
还原
重新启动
```

如果当前应用支持：

```text
always-on-top
transparent
tray
minimize-to-tray
```

也必须回归。

---

# 42. Window Lifecycle

验证：

```text
Create
Show
Hide
Minimize
Restore
Maximize
Resize
Close
Reopen
```

如果有：

```text
single-instance
system tray
deep link
file association
```

则一起验证。

不存在的能力不要为了 M11 新增。

---

# 43. Application Data Lock / Multi-instance

确认：

```text
两个 Tide 实例同时启动
```

会发生什么。

必须避免：

```text
双重 Scheduler
双重 Torrent Engine
DB corruption
duplicate Gateway
same task concurrent execution
```

如果项目采用：

```text
single-instance
```

则必须验证。

如果项目允许多实例，则必须明确：

```text
哪些资源可共享
哪些资源必须互斥
```

---

# 44. Update Strategy

先确认当前产品实际是否已经实现 updater。

分两种情况。

## A. Updater 已实现

必须验证：

```text
update check
signature verification
download
verification
install
restart
database preservation
download preservation
rollback / failure behavior
```

特别确认：

```text
update artifact signature
public key
private key handling
version ordering
downgrade behavior
interrupted update
```

Tauri updater 要求更新包签名校验，私钥必须严格保密；不要为了测试而关闭签名校验。

## B. Updater 尚未实现

不要为了 M11 强行增加完整 updater。

此时实现：

```text
manual upgrade validation
```

并在：

```text
docs/M11-release.md
```

中明确记录：

```text
Updater Status = Not Included in RC
```

以及：

```text
为什么
后续如何实现
当前升级方式是什么
用户数据如何保持
```

---

# 45. Code Signing

如果当前拥有正式代码签名证书：

执行真实 release signing。

至少验证：

```text
installer signature
main executable signature
relevant signed binaries
```

并记录：

```text
Signer
Certificate
Timestamp
Digest
Verification Result
```

Windows 官方 SignTool 支持签名、验证和时间戳；对于新签名建议使用 SHA-256，并可使用 RFC 3161 时间戳。

禁止：

```text
commit private certificate
commit password
print password in CI logs
store private key in repo
```

---

# 46. 没有正式证书时

不要生成一个“伪正式”方案冒充生产签名。

应明确：

```text
Production Signing = Pending
```

然后仍然完成：

```text
Unsigned RC Build
Artifact Verification
Install Validation
Upgrade Validation
Clean-machine Validation
```

并把正式签名作为：

```text
Release Gate before 1.0
```

记录下来。

---

# 47. Artifact Integrity

为每一个最终发布文件生成：

```text
SHA-256
```

例如：

```text
Tide-0.x.x-x64-setup.exe
Tide-0.x.x-x64.msi
```

对应：

```text
SHA256SUMS.txt
```

或项目已有的等价机制。

必须验证：

```text
hash generated
hash recorded
artifact not modified
verification succeeds
```

---

# 48. Installer Upgrade / Uninstall Matrix

建立真实测试矩阵：

| Scenario                 | Expected          |
| ------------------------ | ----------------- |
| Clean Install            | Pass              |
| Reinstall Same Version   | Defined           |
| Upgrade Older Version    | Pass              |
| Upgrade With Active Task | Defined           |
| Upgrade With `.part`     | Pass              |
| Upgrade With Library     | Preserve          |
| Uninstall                | Defined           |
| Reinstall                | Defined           |
| DB Migration             | Pass              |
| Download Preservation    | Pass              |
| Corrupt App Data         | Recoverable/Error |
| Interrupted Install      | Defined           |

每个 scenario 必须有：

```text
actual result
evidence
logs / screenshots if useful
PASS/FAIL
```

---

# 49. Release Candidate Smoke Test

建立一个可以在每台测试机器执行的：

```text
Tide RC Smoke Test
```

推荐流程：

```text
1. Install
2. Launch
3. Create HTTP task
4. Start download
5. Pause
6. Resume
7. Complete
8. Open resulting file
9. Create torrent task
10. Start
11. Partial download
12. Stream
13. Seek
14. Pause
15. Restart application
16. Verify recovery
17. Verify Library
18. Close
19. Reopen
20. Uninstall / Upgrade according to scenario
```

整个流程不能依赖开发工具。

---

# 50. Automated Release Smoke Test

尽可能建立：

```text
release-smoke-test
```

至少自动检查：

```text
version
launch
IPC health
database open
migration status
required directory creation
basic task creation
basic task persistence
```

真实媒体播放 / Windows UI 行为不一定全部自动化。

不能因为自动化困难就假装已覆盖。

---

# 51. Release Test Categories

最终测试必须划分：

```text
Unit
Integration
E2E
Installer
Upgrade
Recovery
Security
Performance
Manual
Clean-machine
```

不要只输出：

```text
cargo test = PASS
```

就认为：

```text
RC = PASS
```

---

# 52. 回归测试重点

必须特别回归以下 M0–M10 核心链：

```text
Task State Machine
Persistence
Recovery
HTTP Download
HTTP Resume
Torrent
Availability
Resource
Scheduler
Progressive Streaming
Range Gateway
Player
Library
Windows Integration
Security
```

每一层记录：

```text
PASS
FAIL
PARTIAL
NOT TESTED
NOT APPLICABLE
```

禁止使用：

```text
“应该没问题”
```

作为测试结论。

---

# 53. Performance Release Validation

M11 不要求全面性能优化。

但必须建立真实基线。

至少记录：

```text
startup time
cold startup
warm startup
task list load time
database open time
migration time
HTTP throughput
Torrent throughput
stream startup latency
seek response
memory idle
memory downloading
memory streaming
CPU idle
CPU downloading
CPU streaming
```

测试规模建议覆盖：

```text
1 task
10 tasks
100 tasks
500 tasks
1000 tasks
```

是否实际跑到全部规模取决于当前项目能力。

重点不是追求漂亮数字，而是：

> **确认 RC 没有明显异常。**

---

# 54. Idle Resource Budget

Tide 是桌面常驻型工具，因此特别检查：

```text
idle CPU
idle memory
background timer
polling frequency
torrent engine idle cost
player inactive cost
gateway idle state
scheduler idle state
database polling
```

确认：

```text
关闭所有任务后
没有高频 busy loop
没有无意义网络请求
没有不断写 SQLite
没有空转 Scheduler
没有残留 Player/Gateway
```

---

# 55. Resource Leak Audit

RC 重点检查：

```text
HTTP connection
Torrent session
Gateway
Playback session
File handle
DB connection
background task
event subscription
timer
channel
thread
process
```

生命周期必须明确：

```text
Create
Use
Stop
Dispose
```

特别测试：

```text
Task Cancel
Window Close
Application Exit
Application Restart
Crash
```

之后是否仍然存在：

```text
zombie process
zombie port
locked file
locked DB
background network
```

---

# 56. Port / Gateway Cleanup

启动：

```text
Gateway
```

然后关闭：

```text
Playback
```

必须验证：

```text
session closed
port released or session invalidated
token invalid
resources released
```

应用退出后：

```text
no zombie gateway
```

重新启动时也不能继承错误状态。

---

# 57. Task Concurrency Regression

验证当前实际支持的并发任务数。

至少：

```text
1
2
4
8
```

根据 M7 Scheduler 的实际上限继续。

检查：

```text
scheduler fairness
disk contention
UI updates
database checkpoint pressure
stream priority
torrent priority
HTTP priority
```

M11 不应该重新设计 Scheduler。

如果发现严重调度问题：

```text
修复 P0/P1 bug
```

而不是：

```text
引入新调度架构
```

---

# 58. Backward Compatibility

检查：

```text
Database schema
Task data
Settings
Library
Download paths
Resource IDs
```

确认升级后不会因为：

```text
版本变化
字段变化
enum 变化
默认值变化
```

导致任务无法读取。

特别注意：

```text
Rust enum serialization
SQLite enum representation
JSON DTO
frontend state
```

---

# 59. Configuration Validation

检查：

```text
Settings
```

特别是：

```text
download directory
concurrency
bandwidth
torrent sharing policy
player behavior
startup behavior
language
theme
```

验证：

```text
invalid config
missing config
old config
new config
partially written config
```

应用仍能安全启动。

---

# 60. Default Policy Verification

Release Candidate 必须明确所有默认策略。

例如：

```text
Download Only / Balanced / Seed
```

必须与实际实现一致。

不能 UI 显示一种：

```text
Download Only
```

而后台实际使用另一种行为。

同时不要宣传：

```text
Download Only = legal
```

或：

```text
Download Only = anonymous
```

Tide 的定位仍然是：

> 用户自己提供来源的本地下载器与渐进式媒体播放器。

---

# 61. Privacy Regression

M11 再次确认：

```text
No account
No cloud
No mandatory telemetry
No upload of user content
No upload of task history
No upload of downloaded files
```

如果当前存在：

```text
update check
license verification
crash reporting
```

必须明确：

```text
what is sent
when
why
whether opt-in
```

不能用：

```text
“不会收集任何数据”
```

这种无法证明的绝对表述。

---

# 62. Third-Party License Audit

重新扫描：

```text
Cargo dependencies
npm dependencies
Tauri plugins
native libraries
bundled binaries
codec-related dependencies
```

输出：

```text
dependency
version
license
source
notice requirement
distribution requirement
```

确保：

```text
THIRD_PARTY_NOTICES.md
```

与最终 release 一致。

如果项目内存在：

```text
FFmpeg
libmpv
VLC
GStreamer
other native runtime
```

必须检查其实际分发许可，而不是只看 Rust/npm 包的 license。

---

# 63. SBOM / Dependency Inventory

如果项目当前已有 SBOM 能力：

验证 release 是否生成。

如果没有：

至少生成：

```text
release dependency inventory
```

不要为了 M11 引入复杂供应链平台。

---

# 64. CI / Release Pipeline

检查：

```text
CI build
test
lint
typecheck
release build
artifact upload
checksum
signing
```

必须区分：

```text
PR Build
Nightly
Release Candidate
Production Release
```

不要让：

```text
debug CI
```

伪装成：

```text
production release pipeline
```

---

# 65. CI Secrets

重新审计：

```text
signing key
updater private key
certificate password
GitHub token
release token
cloud credentials
```

全部通过：

```text
CI Secret
```

或等价安全机制管理。

禁止：

```text
hardcode
git history
console output
artifact
```

泄露。

---

# 66. Git Release Discipline

M11 结束时：

工作树应明确。

记录：

```text
RC commit
branch
tag candidate
changed files
```

建议创建：

```text
vX.Y.Z-rc.1
```

但具体命名遵守当前项目版本规范。

不要直接把：

```text
main
```

上的随机工作状态称作 RC。

---

# 67. Git Diff 审计

最终提交前运行：

```text
git status
git diff
git diff --stat
```

检查：

```text
unexpected file
secret
temporary artifact
large binary
debug code
TODO accidentally committed
test fixture
personal data
```

确认所有修改都与 M11 有关。

---

# 68. Documentation Freeze

确保至少存在并更新：

```text
README.md
SECURITY.md
PRIVACY.md
CONTENT_POLICY.md
THIRD_PARTY_NOTICES.md
```

以及：

```text
docs/M11-release.md
docs/M11-test-matrix.md
docs/M11-install-upgrade.md
docs/M11-recovery-validation.md
docs/M11-security-regression.md
docs/M11-report.md
```

实际目录结构以项目为准。

---

# 69. README Release 信息

README 不应该再像一个：

```text
development repository
```

而应该能让用户理解：

```text
Tide 是什么
Tide 不是什么
支持什么
如何安装
如何使用
数据保存在哪里
如何升级
如何卸载
Torrent 使用边界
隐私策略
已知限制
```

不要营销性夸大：

```text
Fastest
Anonymous
Untraceable
Unlimited
No copyright risk
```

这些内容都禁止。

---

# 70. CONTENT_POLICY 最终审计

必须继续明确：

```text
Tide does not provide resource discovery.
Tide does not recommend infringement sources.
Tide does not bypass DRM.
Tide does not bypass paywalls.
Tide does not provide anonymity.
```

产品文案应保持：

```text
User-provided source
Local-first
Privacy-conscious
No cloud relay by default
```

并避免把协议能力本身描述为违法或免责结论。

---

# 71. Release Notes

生成 RC Release Notes：

```text
Version
Highlights
Supported Features
Security
Compatibility
Known Limitations
Upgrade Notes
Data Preservation
Known Issues
Testing Coverage
```

尤其说明：

```text
什么已经验证
什么还没有
```

不要把未验证功能写成 fully supported。

---

# 72. Known Issues Policy

建立：

```text
KNOWN_ISSUES.md
```

每个问题至少包含：

```text
ID
Severity
Area
Reproduction
Expected
Actual
Workaround
Release Impact
Status
```

严重级别只使用：

```text
P0
P1
P2
P3
```

M11 阶段重点清零：

```text
P0
```

以及会阻塞正常发布的：

```text
P1
```

---

# 73. Release Gate

M11 RC 不允许在以下状态标记：

```text
RELEASE CANDIDATE
```

如果存在：

```text
P0 open
critical data-loss bug
installer unusable
upgrade corruption
database migration corruption
download corruption
security boundary bypass
release artifact unverifiable
app crashes on normal startup
```

这些必须视为：

```text
RC BLOCKER
```

---

# 74. P0 / P1 判定标准

## P0

包括但不限于：

```text
data loss
file corruption
security boundary bypass
arbitrary local file access
credential/private key leakage
installer unusable
application cannot start
migration corrupts data
upgrade destroys user downloads
critical crash on normal workflow
```

## P1

包括但不限于：

```text
core workflow broken on common Windows configuration
streaming unusable for supported format
resume repeatedly corrupts
library loses state
installer works only in developer environment
major UI workflow blocked
serious resource leak
```

---

# 75. 不允许用“Known Issue”掩盖 P0

禁止：

```text
P0 → Known Issue → Release anyway
```

必须：

```text
P0 = Release Blocker
```

---

# 76. Final Acceptance Matrix

最终必须输出：

| Area                | Result            | Evidence           |
| ------------------- | ----------------- | ------------------ |
| Build               | PASS/FAIL         | command + artifact |
| Unit Tests          | PASS/FAIL         | test result        |
| Integration         | PASS/FAIL         | result             |
| E2E                 | PASS/FAIL         | scenario           |
| HTTP Download       | PASS/FAIL         | scenario           |
| Torrent             | PASS/FAIL         | scenario           |
| Persistence         | PASS/FAIL         | scenario           |
| Recovery            | PASS/FAIL         | scenario           |
| Streaming           | PASS/FAIL         | scenario           |
| Player              | PASS/FAIL         | scenario           |
| Library             | PASS/FAIL         | scenario           |
| Windows Integration | PASS/FAIL         | scenario           |
| Installer           | PASS/FAIL         | artifact           |
| Upgrade             | PASS/FAIL         | scenario           |
| Uninstall           | PASS/FAIL         | scenario           |
| Security            | PASS/FAIL         | audit              |
| Signing             | PASS/FAIL/PENDING | verification       |
| License             | PASS/FAIL         | notice             |
| Performance         | PASS/FAIL         | measurements       |
| Clean Machine       | PASS/FAIL         | test record        |
| P0                  | count             | issue IDs          |
| P1                  | count             | issue IDs          |

---

# 77. 真实 RC 安装测试

至少完成：

```text
Machine A
Clean install
```

以及：

```text
Machine B
Older version
→ Upgrade RC
```

如果环境资源允许：

```text
Machine C
Different Windows configuration
```

重点不是机器数量，而是：

> **不能只在开发机自己运行一下，然后宣布 RC。**

---

# 78. 最终用户工作流

执行完整用户路径：

```text
启动 Tide
↓
创建 HTTP 任务
↓
下载
↓
暂停
↓
恢复
↓
完成
↓
播放

创建 Torrent 任务
↓
部分下载
↓
播放
↓
Seek
↓
继续下载
↓
完成

关闭 Tide
↓
重新打开
↓
恢复状态
↓
继续使用
```

然后执行：

```text
升级
↓
重新启动
↓
数据保持
```

最后：

```text
卸载
↓
确认数据策略
↓
重新安装
↓
确认策略结果
```

---

# 79. RC Artifact Manifest

生成：

```text
release/
    Tide-X.Y.Z-rc.1-x64-setup.exe
    Tide-X.Y.Z-rc.1-x64.msi        (if applicable)
    SHA256SUMS.txt
    RELEASE-MANIFEST.json
    RELEASE-NOTES.md
```

Manifest 至少包含：

```json
{
  "product": "ifuyo Tide",
  "version": "...",
  "channel": "rc",
  "commit": "...",
  "platform": "windows",
  "arch": "x64",
  "build_profile": "release",
  "artifacts": [],
  "checksums": [],
  "signing": {},
  "toolchain": {},
  "dependencies_locked": true
}
```

字段根据项目实际情况调整。

---

# 80. Release Manifest 必须可追溯

给定：

```text
RC artifact
```

应该能够回答：

```text
它由哪个 commit 构建？
用什么版本的 Rust？
什么 Tauri？
什么 Node？
什么依赖锁？
是否签名？
SHA-256 是什么？
```

---

# 81. 不要污染正式用户数据测试

所有测试：

```text
Download Root
App Data
DB
Cache
Logs
```

尽量使用独立测试环境。

不要拿真实用户数据做 migration / uninstall / recovery 测试。

---

# 82. Release Test Evidence

测试不是只写：

```text
PASS
```

尽可能保存：

```text
command output
test report
installer filename
SHA256
signature verification
screenshots where useful
migration result
clean-machine result
upgrade result
performance result
```

证据必须能够支持结论。

---

# 83. M11 架构冻结要求

M11 最终必须识别：

```text
哪些是核心架构
哪些是稳定 API
哪些是内部实现
哪些是未来扩展点
```

但：

> **不要因为“架构冻结”而进行大规模重构。**

M11 的架构动作只允许：

```text
release-blocking fixes
stability fixes
security fixes
testability fixes
dependency/build fixes
```

---

# 84. 不要为了 RC 制造“完美架构”

明确禁止：

```text
把所有模块重新重构一遍
重新设计 Resource
重新设计 Scheduler
重新设计 Task
重新设计 Player
重新设计 frontend
重新拆 crate
```

除非存在：

```text
P0/P1 release blocker
```

---

# 85. M11 的最终目标

最终代码应该达到：

```text
Development Project
        ↓
Stable Release Candidate
```

而不是：

```text
Development Project
        ↓
Another Development Milestone
```

---

# 86. 最终报告格式

M11 结束后，必须生成：

```text
docs/M11-report.md
```

结构固定为：

# 1. Executive Summary

# 2. Release Candidate Identity

* version
* commit
* branch
* build ID
* platform
* architecture

# 3. M0–M10 Completion Audit

# 4. M11 Changes

# 5. Build System

# 6. Installer

# 7. Signing

# 8. Upgrade

# 9. Uninstall

# 10. Data Preservation

# 11. Migration

# 12. Download Regression

# 13. Torrent Regression

# 14. Streaming Regression

# 15. Player Regression

# 16. Library Regression

# 17. Security Regression

# 18. Privacy

# 19. Third-party Licenses

# 20. Clean-machine Validation

# 21. Performance

# 22. Test Matrix

# 23. FACT / HYPOTHESIS / INFERENCE

# 24. RESOLVED / CONFIRMED / OPEN / WORSENED

# 25. P0 / P1 / P2 / P3

# 26. Known Issues

# 27. Release Artifacts

# 28. Checksums

# 29. Limitations

# 30. M12 Readiness

# 31. Definition of Done

---

# 87. 必须明确记录这些状态

最终报告中必须明确：

```text
RESOLVED
CONFIRMED
OPEN
WORSENED
```

并且：

```text
FACT
HYPOTHESIS
INFERENCE
```

不能混写。

---

# 88. M12 Readiness Assessment

M11 最后必须回答：

```text
Tide 是否已经具备 1.0 发布条件？
```

但不要简单写：

```text
Yes
```

必须根据证据拆成：

```text
Product
Engineering
Security
Installer
Upgrade
Reliability
Documentation
Legal/License
Performance
```

每一项：

```text
READY
NOT READY
CONDITIONAL
```

以及对应证据。

不要为了“完成 M11”强行宣布：

```text
READY
```

---

# 89. M11 Definition of Done

只有同时满足以下条件，M11 才算完成：

```text
[ ] M0–M10 当前实现已审计
[ ] 当前版本号一致
[ ] Release Build 成功
[ ] Locked dependencies 验证通过
[ ] Automated tests 通过
[ ] Release artifact 成功生成
[ ] Installer 成功
[ ] Clean-machine install 成功
[ ] Application launch 成功
[ ] HTTP workflow 通过
[ ] Torrent workflow 通过
[ ] Persistence workflow 通过
[ ] Recovery workflow 通过
[ ] Progressive Streaming workflow 通过
[ ] Player workflow 通过
[ ] Library workflow 通过
[ ] Restart workflow 通过
[ ] Upgrade workflow 通过
[ ] Uninstall workflow 已验证
[ ] User download data lifecycle 已验证
[ ] SQLite migration 已验证
[ ] Crash consistency 已回归
[ ] Security regression 已验证
[ ] Gateway security 已验证
[ ] Path security 已验证
[ ] URL/SSRF security 已验证
[ ] Capabilities 已审计
[ ] Logs 已审计
[ ] Privacy 已审计
[ ] Third-party licenses 已审计
[ ] SHA-256 已生成
[ ] Signing 已验证或明确标记 Pending
[ ] Release manifest 已生成
[ ] Release notes 已生成
[ ] Known issues 已生成
[ ] P0 = 0
[ ] Release-blocking P1 = 0
[ ] Clean Windows environment validated
[ ] M12 readiness documented
```

---

# 90. M11 最重要的验收标准

不要用：

```text
“代码看起来不错”
```

验收。

而是用：

```text
真实 installer
+
真实 Windows
+
真实用户数据生命周期
+
真实下载
+
真实 Torrent
+
真实播放
+
真实崩溃恢复
+
真实升级
+
真实卸载
```

来验收。

---

# 91. 最终 Release Gate

最终只允许出现以下三种结果之一：

## A — RC PASS

满足：

```text
P0 = 0
Release-blocking P1 = 0
Core workflows PASS
Installer PASS
Upgrade PASS
Security PASS
Clean-machine PASS
```

则：

```text
M11 = COMPLETE
Tide = M12 READY
```

## B — RC CONDITIONAL

存在非阻塞问题：

```text
P2/P3
known limitations
non-critical signing pending
```

但核心用户流程全部成立。

则：

```text
M11 = COMPLETE / CONDITIONAL
```

并明确阻塞 M12 的项目。

## C — RC BLOCKED

存在：

```text
P0
critical P1
data loss
corruption
security breach
installer failure
migration failure
core workflow failure
```

则：

```text
M11 = BLOCKED
```

必须修复后重新执行对应 Release Gate。

---

# 92. 最终停止条件

完成 M11 后：

> **停止继续开发 M12 功能。**

不要顺手实现：

```text
Steam
Achievements
Cloud Save
Workshop
advanced updater
new media engine
new protocol
new social feature
```

除非它们已经属于 M12 明确范围。

M11 的职责就是：

```text
Freeze
Validate
Package
Verify
Document
```

---

# 93. 最终输出格式

完成全部工作后，最后只输出：

```text
M11 RELEASE CANDIDATE REPORT

1. VERSION
2. COMMIT
3. BUILD
4. INSTALLER
5. CLEAN-MACHINE RESULT
6. UPGRADE RESULT
7. UNINSTALL RESULT
8. DATA PRESERVATION RESULT
9. DOWNLOAD RESULT
10. TORRENT RESULT
11. STREAMING RESULT
12. PLAYER RESULT
13. RECOVERY RESULT
14. SECURITY RESULT
15. SIGNING RESULT
16. LICENSE RESULT
17. PERFORMANCE RESULT
18. TEST RESULT
19. P0
20. P1
21. KNOWN ISSUES
22. ARTIFACTS
23. FACT / HYPOTHESIS / INFERENCE
24. RESOLVED / CONFIRMED / OPEN / WORSENED
25. M12 READINESS
26. DEFINITION OF DONE
```

并明确：

```text
RC PASS
```

或：

```text
RC CONDITIONAL
```

或：

```text
RC BLOCKED
```

---

# 94. M11 核心闭环

整个 M11 最终必须证明：

```text
Source
  ↓
Task
  ↓
Persistence
  ↓
Download / Torrent
  ↓
Partial Data
  ↓
Recovery
  ↓
Availability
  ↓
Scheduler
  ↓
Progressive Stream
  ↓
Player
  ↓
Library
  ↓
Windows Application
  ↓
Installer
  ↓
Clean Machine
  ↓
Real User
  ↓
Upgrade
  ↓
User Data Preserved
  ↓
Uninstall
  ↓
Reinstall
  ↓
Verified Release Candidate
```

---

# 95. 核心原则

始终遵守：

> **M11 不再证明“我们能不能做出来”，而是证明“做出来的东西能不能真正交给用户”。**

> **Installer 是产品的一部分。**

> **Upgrade 是产品的一部分。**

> **Recovery 是产品的一部分。**

> **Data preservation 是产品的一部分。**

> **Security 是产品的一部分。**

> **Release artifact 本身也是产品的一部分。**

> **没有证据的“应该可以”不算通过。**

> **没有真实机器验证的“能运行”不算 RC。**

> **没有用户数据生命周期验证的“升级成功”不算 RC。**

---

## 执行要求

现在开始：

### Step 1

审计当前 repository 与 M0–M10 实际状态。

### Step 2

输出：

```text
FACTS
HYPOTHESES
CURRENT_STATE
M0–M10 COMPLETION AUDIT
M11 GAPS
RELEASE RISKS
IMPLEMENTATION PLAN
```

### Step 3

实施 M11。

### Step 4

运行完整 Release Candidate 验证矩阵。

### Step 5

生成实际 release artifacts。

### Step 6

执行 clean-machine / install / upgrade / uninstall / recovery / core workflow validation。

### Step 7

生成：

```text
docs/M11-release.md
docs/M11-test-matrix.md
docs/M11-install-upgrade.md
docs/M11-recovery-validation.md
docs/M11-security-regression.md
docs/M11-report.md
```

### Step 8

最终提交前执行：

```text
git status
git diff --stat
cargo check --locked
cargo test --locked
frontend tests
frontend build
tauri release build
```

以及项目实际的：

```text
lint
typecheck
CI checks
installer checks
signature verification
checksum verification
```

### Step 9

输出最终：

```text
RC PASS
```

或：

```text
RC CONDITIONAL
```

或：

```text
RC BLOCKED
```

并用实际证据解释原因。

### 最后一道硬性约束

**不要把 M11 继续扩张成 M12。**

M11 必须结束在：

```text
A real, installable, testable, traceable, secure Tide Release Candidate.
```

而不是：

```text
一个功能更多但永远没有真正发布验证过的 Tide 开发版本。
```
