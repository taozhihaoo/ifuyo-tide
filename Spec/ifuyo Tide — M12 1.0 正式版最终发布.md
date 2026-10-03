# ifuyo Tide — M12 1.0 正式版最终发布

## Final Release / Product Freeze / Production Baseline

> **阶段：M12**
>
> **目标：Tide 1.0 正式版**
>
> 本阶段是在 **M11 Release Candidate（RC）** 基础上的最终发布收口。
>
> M12 不以新增功能为目标，而以：
>
> **功能冻结 → 最终验收 → 发布构建 → 安装/升级验证 → 数据迁移验证 → 安全/合规最终检查 → 文档冻结 → Release Artifact → 1.0.0 基线**
>
> 为核心。
>
> **原则：**
>
> > M1 解决“下载正确性”
> > M2 解决“生命周期正确性”
> > M3 解决“Torrent 能力”
> > M4 解决“资源可用性 / 渐进式读取 / Scheduler 基础”
> > M5 解决“播放体验”
> > M6 解决“Library”
> > M7 解决“Scheduler”
> > M8 解决“Windows Integration”
> > M9 解决“Polish”
> > M10 解决“Security & Release Hardening”
> > M11 解决“Release Candidate”
> > **M12 只解决：能不能作为真正的 1.0 产品交付。**
>
> 不要把 M12 变成 M13。

---

# 0. 最重要的执行纪律

开始修改代码之前，必须先完整检查当前仓库实际状态。

**禁止根据历史提示词假设 M11 已完整实现。**

先检查：

* Git 当前分支
* 工作区状态
* 当前版本号
* Rust / Tauri / Node / pnpm/npm/yarn 版本
* `Cargo.lock`
* `package-lock.json` / `pnpm-lock.yaml` / `yarn.lock`
* M0~M11 实际代码
* 数据库 migration
* Task / Resource / Availability / Scheduler / Playback / Library
* HTTP Download
* Torrent / librqbit
* Progressive Streaming
* Range Gateway
* Player
* Windows Integration
* Security Hardening
* Error Handling
* Logging / Diagnostics
* Tests
* CI
* Packaging / Installer
* Release Configuration
* Documentation
* License / Third-party notices
* README
* CHANGELOG
* Version metadata
* Application identifier
* Product name
* Icon / installer assets
* 开发环境残留配置
* debug-only code
* test-only code
* feature flags
* mock / fake implementation
* TODO / FIXME / HACK
* hard-coded localhost/debug ports
* development filesystem paths
* temporary test data
* sensitive logs
* secrets / API keys / tokens / certificates
* unnecessary permissions / Tauri capabilities
* production build configuration

然后输出：

```text
FACTS
HYPOTHESES
CURRENT_VERSION
CURRENT_M11_STATE
RELEASE_BLOCKERS
NON_BLOCKING_ISSUES
M12_PLAN
RELEASE_RISKS
```

其中：

### FACTS

只写通过代码、构建、测试、配置实际确认的事实。

### HYPOTHESES

明确标记仍需要实验或环境验证的内容。

### RELEASE_BLOCKERS

只列阻止 1.0 发布的问题。

### NON_BLOCKING_ISSUES

允许留给 1.1+ 的问题。

### M12_PLAN

只规划为了 1.0 必须完成的事情。

---

# 1. M12 产品目标

M12 完成后，Tide 应当能够被理解为：

> 一个 Windows 本地下载 + Torrent + 渐进式媒体读取 + 本地播放 + Library + 智能调度工具。

用户提供来源：

```text
HTTP / HTTPS
Magnet
Torrent File
Local File
```

系统执行：

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
Download Engine / Torrent Engine
  ↓
Storage
  ↓
Progressive Stream Gateway
  ↓
Player
  ↓
Library
```

并能够可靠完成：

```text
创建任务
↓
持久化
↓
下载
↓
暂停
↓
恢复
↓
退出
↓
重新启动
↓
恢复任务
↓
继续下载
↓
边下边播
↓
Seek
↓
完成
↓
校验
↓
归档到 Library
```

1.0 的核心不是“功能数量”。

而是：

> **整个核心链路能够长期稳定运行，而且用户不会因为一次退出、重启、网络波动、seek、异常、版本更新而轻易破坏任务或文件。**

---

# 2. M12 明确禁止事项

除非发现属于 1.0 发布阻塞问题，否则禁止新增：

* 新下载协议
* 新资源发现系统
* 资源搜索
* 资源站
* 资源索引
* 推荐系统
* 排名系统
* Tracker 推荐
* Pirate Resource Finder
* 第三方资源站导航
* DRM 绕过
* Paywall 绕过
* 匿名下载
* VPN
* 代理服务
* 云下载
* 云中转
* 用户账号系统
* 用户内容上传
* 云端 Library
* 云端同步服务
* 默认 telemetry
* 广告系统
* 社交系统
* 评论系统
* AI 推荐系统
* NAS 大规模功能扩展
* UPnP 大规模功能扩展
* Casting
* Steam Workshop
* Steam Overlay
* 积分
* VIP
* 会员
* “加速套餐”
* 内容运营
* 资源聚合
* 全新的播放器体系
* 全新的 UI 框架
* 全新的状态机
* 全新的 Resource 抽象
* 全新的 Scheduler 架构
* 大规模重构

尤其禁止因为“还有一点不够优雅”而重写整个架构。

---

# 3. M12 的核心任务

M12 必须完成以下八件事情：

```text
A. Product Freeze
B. Final Integration Verification
C. Release Build
D. Install / Upgrade / Uninstall Validation
E. Data Migration & Recovery Validation
F. Security / Privacy / Compliance Final Audit
G. Documentation Freeze
H. 1.0.0 Release Artifact
```

---

# 4. A — Product Freeze

在 M12 第一阶段，建立正式的：

```text
Tide 1.0 Feature Freeze
```

检查 M11 当前功能。

把功能分成：

```text
RELEASED
RELEASE-BLOCKER
NON-BLOCKING
DEFERRED
REMOVED
```

不要再使用：

```text
TODO someday
temporary implementation
mock implementation
fake implementation
prototype UI
debug shortcut
```

作为 1.0 正式功能。

如果代码仍存在：

```rust
TODO
FIXME
HACK
unimplemented!
todo!()
panic!("not implemented")
```

必须逐项分类：

```text
P0: Release Blocker
P1: Must Fix Before 1.0
P2: 1.1+
```

但 M12 原则仍然：

> **只处理真正阻止 1.0 的问题。**

---

# 5. 产品功能最终矩阵

建立最终功能矩阵：

| 功能                   | 状态 | 自动化测试 | 手工测试 | Release |
| -------------------- | -- | ----: | ---: | ------: |
| HTTP Download        |    |       |      |         |
| HTTPS Download       |    |       |      |         |
| Redirect             |    |       |      |         |
| Range Resume         |    |       |      |         |
| ETag / Last-Modified |    |       |      |         |
| Retry                |    |       |      |         |
| Pause                |    |       |      |         |
| Cancel               |    |       |      |         |
| Crash Recovery       |    |       |      |         |
| SQLite Persistence   |    |       |      |         |
| Torrent              |    |       |      |         |
| Magnet               |    |       |      |         |
| Torrent Resume       |    |       |      |         |
| Availability         |    |       |      |         |
| Progressive Read     |    |       |      |         |
| Range Gateway        |    |       |      |         |
| Streaming            |    |       |      |         |
| Seek                 |    |       |      |         |
| Multi-file Torrent   |    |       |      |         |
| Playback             |    |       |      |         |
| Library              |    |       |      |         |
| Scheduler            |    |       |      |         |
| Windows Integration  |    |       |      |         |
| Settings             |    |       |      |         |
| Error Reporting      |    |       |      |         |
| Security Controls    |    |       |      |         |

必须避免出现：

```text
UI 看起来支持
但 backend 实际不支持
```

或者：

```text
backend 支持
但 IPC 没接
```

或者：

```text
正常流程支持
但重启/异常流程失效
```

---

# 6. B — Final Integration Verification

必须进行一次真正的：

# End-to-End Acceptance

不能只运行 unit tests。

至少验证：

## Scenario 1 — HTTP Download

```text
URL
↓
Create Task
↓
HTTP GET
↓
Progress
↓
Pause
↓
Resume
↓
Complete
↓
Checksum
↓
Final File
```

确认：

* 文件正确
* 文件大小正确
* 状态正确
* 数据库正确
* UI 正确
* 最终文件存在
* `.part` 被正确处理

---

# 7. Scenario 2 — HTTP Crash Recovery

模拟：

```text
Download
↓
Partial .part
↓
Force Process Termination
↓
Restart Application
↓
Recovery
↓
Range Resume
↓
Complete
```

确认：

* 没有重复追加
* 没有文件损坏
* 没有丢失有效 prefix
* 不会从错误 offset 开始
* DB 状态正确
* UI 状态正确

---

# 8. Scenario 3 — Torrent

使用：

* public-domain
* open-source
* 自生成测试资源
* 明确拥有/获授权资源

进行：

```text
Torrent / Magnet
↓
Metadata
↓
File Selection
↓
Start
↓
Availability
↓
Progress
↓
Pause
↓
Resume
↓
Complete
```

不得依赖盗版资源进行测试。

---

# 9. Scenario 4 — Torrent Progressive Playback

验证真正核心能力：

```text
Torrent
↓
Partial Pieces
↓
Availability
↓
Resource Read
↓
Scheduler Demand
↓
Stream Gateway
↓
WebView2
↓
Player
```

必须验证：

* 尚未完全下载时能够开始播放
* 缓冲区域正确
* seek 能触发新的 demand
* Scheduler 能优先满足播放需求
* 资源尚未可用时不会读出错误数据
* 取消 playback 不会破坏 download task
* 播放 session 与 download task 生命周期独立

---

# 10. Scenario 5 — Seek

测试：

```text
Play
↓
Seek forward
↓
Range Request
↓
Availability
↓
Demand
↓
Scheduler
↓
Piece acquisition
↓
Continue playback
```

测试：

* 已缓存区域 seek
* 未缓存区域 seek
* 大幅 seek
* 多次连续 seek
* 快速来回 seek
* seek 后暂停
* seek 后退出
* seek 后恢复

不得出现：

```text
seek → 读取错误数据
seek → 下载任务崩溃
seek → player 卡死
seek → scheduler 饥饿
seek → gateway 泄漏
```

---

# 11. Scenario 6 — Multi-file Torrent

验证：

```text
Torrent
├── video.mkv
├── audio.mp3
├── subtitle.srt
└── metadata.txt
```

播放器只能访问目标资源。

必须确保：

```text
video request
```

不会越过 file boundary 读取：

```text
audio
subtitle
metadata
```

必须验证：

* 文件映射
* offset 映射
* Range translation
* EOF
* 416
* file boundary
* gateway authorization

---

# 12. Scenario 7 — Application Restart

建立至少：

```text
Downloading Task
Paused Task
Failed Task
Completed Task
Cancelled Task
Playing Resource
Library Item
```

然后：

```text
Close Application
↓
Start Application
```

验证所有任务状态。

特别检查：

```text
Running
```

不能在重启后被错误当成：

```text
仍然正在运行
```

必须经过 recovery。

---

# 13. Scenario 8 — Dirty Shutdown

模拟：

```text
kill process
```

而不是正常关闭。

测试至少：

```text
download write 中
checkpoint 前
checkpoint 后
flush 前
flush 后
finalize 前
rename 后 DB update 前
```

确认：

> 应用即使没有执行 graceful shutdown，也不能因为状态机或 checkpoint 不一致而把用户数据推进到错误位置。

---

# 14. C — Release Build

建立唯一正式 Release Profile。

例如：

```text
release
```

确认：

* debug=false
* test-only feature disabled
* development server disabled
* mock backend disabled
* fake progress disabled
* fake player disabled
* fake scheduler disabled
* development logging disabled或降级
* debug IPC 不可用
* test endpoints 不可用
* localhost development route 不可用
* dev-only filesystem access 不可用
* dev-only command 不可用

检查：

```text
Cargo.toml
Cargo.lock
tauri.conf.json
capabilities/*
package.json
vite.config.*
environment files
build scripts
installer configuration
```

---

# 15. 版本号统一

建立唯一版本来源。

至少检查：

```text
Tauri version
package version
Rust package version
Windows executable version
Installer version
Product version
```

最终统一为：

```text
1.0.0
```

禁止出现：

```text
1.0
1.0.0-dev
1.0.0-beta
1.0.0-rc
0.9.9
```

之类不一致情况。

除非项目现有版本策略有明确原因。

如果存在多个版本来源，必须明确：

```text
SOURCE OF TRUTH
DERIVED VALUES
```

避免未来版本号漂移。

---

# 16. Application Identity

最终检查：

```text
Product Name
Application ID
Executable Name
Install Directory
Data Directory
Protocol Association
File Association
Uninstaller Identity
Windows Registry Entries（如果使用）
Start Menu Entries
Desktop Shortcut
```

不能残留：

```text
Tauri App
Example App
Test App
ifuyo-test
dev-build
```

等开发阶段名称。

---

# 17. Release Artifact

正式生成至少：

```text
Tide-1.0.0-setup.exe
```

或者项目实际采用的正式 Windows 安装包格式。

如果同时提供 portable：

```text
Tide-1.0.0-portable.zip
```

则必须分别测试。

不得为了“多一个下载格式”而临时增加复杂 packaging system。

---

# 18. Installer 测试

使用干净 Windows 环境验证：

```text
Fresh Install
```

检查：

* 安装成功
* application 启动
* shortcut 正常
* uninstall 正常
* data directory 创建正确
* download directory 不被错误覆盖
* SQLite 正常创建
* application data 权限正常
* firewall/network 行为符合设计
* local gateway 正常
* player 正常

---

# 19. Upgrade 测试

至少测试：

```text
Previous Version
↓
Install Tide 1.0.0
```

重点验证：

### Database Migration

```text
old DB
↓
migration
↓
new DB
```

确认：

* task 不丢失
* state 不丢失
* source 不丢失
* destination 不丢失
* completed file 不丢失
* Library 不丢失
* settings 不异常
* migration 可重复检查
* migration 失败时不会产生半完成状态

绝对禁止：

> 为了 1.0 方便而删除旧 migration 历史重新生成数据库。

---

# 20. Upgrade + Interrupted Task

这是必须测试的高级场景：

```text
Tide 0.x
↓
Partial Download
↓
Upgrade
↓
Tide 1.0
↓
Recovery
↓
Resume
↓
Complete
```

确认：

```text
partial file
DB record
validators
task ID
destination
resource identity
```

仍然一致。

---

# 21. Reinstall 测试

明确产品行为：

### Uninstall

默认是否：

```text
卸载程序
≠
删除用户下载文件
≠
删除 Library
≠
删除用户数据库
```

必须与实际设计一致。

如果卸载器提供：

```text
Remove user data
```

则必须明确提示。

不得静默删除用户媒体文件。

---

# 22. Filesystem Safety Final Audit

最终检查：

```text
Download Root
App Data Root
Library Root
Temporary Root
Gateway
```

确保没有：

```text
../
..\ 
UNC traversal
device path
junction escape
symlink escape
reparse-point escape
arbitrary absolute path injection
```

尤其验证 Windows：

```text
C:\...
\\server\share
\\?\...
CON
PRN
AUX
NUL
COM1
LPT1
```

等特殊情况。

---

# 23. Gateway Final Security Audit

Progressive Stream Gateway 是 Tide 的高风险边界之一。

最终必须确认：

```text
Loopback Only
+
Random Port
+
High Entropy Capability Token
+
Session → Resource Binding
+
Host Validation
+
Range Validation
+
Request Limits
+
Concurrency Limits
+
Timeouts
+
Lifecycle Cleanup
```

确认不存在：

```text
http://127.0.0.1:<port>/../../secret
```

或者类似：

```text
arbitrary local file read
```

能力。

绝不能存在：

```text
?path=C:\Users\...
```

这种直接把 filesystem path 暴露给浏览器的设计。

---

# 24. SSRF Final Audit

HTTP 下载最终检查：

```text
URL validation
↓
redirect validation
↓
DNS resolution
↓
IP classification
↓
request
```

检查是否可能访问：

```text
localhost
127.0.0.1
::1
private network
link-local
loopback
metadata endpoint
特殊 IPv4/IPv6 表示
```

并确认：

> redirect 不能绕过首次 URL 验证。

如果项目明确支持 LAN/local download，则必须把它定义成：

```text
explicit user opt-in
```

而不是默认开放。

---

# 25. Tauri Capability Final Audit

最终检查：

```text
capabilities/*.json
permissions
commands
window permissions
filesystem permissions
shell permissions
process permissions
dialog permissions
http permissions
```

原则：

> **只给必要权限。**

禁止为了开发方便而留下：

```text
fs:*
shell:*
process:*
http:*
window:*
```

等过宽权限。

尤其检查：

```text
frontend
```

是否能调用本不该暴露给 UI 的 command。

---

# 26. Frontend / Backend Trust Boundary

最终确认：

```text
Frontend = untrusted input / presentation
Backend = authority
```

前端不能成为安全验证唯一来源。

Rust 必须再次验证：

* Task ID
* Resource ID
* Source
* URL
* filesystem destination
* Range
* playback resource
* gateway session
* settings
* user input

---

# 27. Secrets Audit

进行全项目搜索：

```text
API_KEY
TOKEN
SECRET
PASSWORD
PRIVATE_KEY
ACCESS_TOKEN
CLIENT_SECRET
```

检查：

* Rust
* TS
* JSON
* YAML
* TOML
* `.env`
* test data
* CI
* logs
* docs
* screenshots
* installer config

不得把任何实际 secret 打进：

```text
binary
frontend bundle
repository
release artifact
logs
```

---

# 28. Logging Finalization

Production log 必须避免：

```text
完整 URL
Magnet URI
InfoHash + 完整上下文
Peer IP
完整本地文件路径
Gateway token
Session token
用户历史
敏感 header
Authorization
Cookie
```

可以使用：

```text
Task ID
Resource ID
错误分类
HTTP status
阶段
duration
downloaded bytes
```

等经过控制的数据。

---

# 29. Privacy Final Audit

再次确认：

```text
Default
=
Local First
=
No Cloud
=
No Account
=
No Upload to ifuyo
=
No Telemetry by Default
```

检查：

* DNS/HTTP 请求是否只为了用户请求的资源
* 是否存在隐藏 analytics
* crash report 是否默认启用
* update check 是否存在
* update check 是否需要网络
* 是否向 ifuyo 服务发送信息
* 是否上传任务历史
* 是否上传媒体元数据
* 是否上传 magnet/hash

如果未来存在网络检查：

必须在：

```text
PRIVACY.md
```

明确说明。

---

# 30. BT 行为与用户控制最终检查

Tide 使用 BitTorrent 时必须明确：

```text
Download Only
Balanced
Seed
```

或项目已经确定的实际模式。

默认策略必须与产品设计一致。

UI 中不要宣传：

```text
100% anonymous
untraceable
完全安全
不会上传
不会被发现
```

因为 BitTorrent 本身涉及 peer-to-peer 网络通信。

如果某模式会上传 piece：

必须真实反映其行为。

不要把：

```text
Download Only
```

描述成：

```text
Legal Mode
Safe Mode
Copyright Safe
```

因为这不是法律结论。

---

# 31. CONTENT_POLICY 最终确认

检查：

```text
CONTENT_POLICY.md
```

必须明确 Tide：

### 支持

用户有权使用或拥有的：

* 自有文件
* 开源资源
* Public Domain
* 授权资源
* 合法分发内容

### 不提供

* 盗版资源发现
* 侵权资源索引
* 资源站导航
* 盗版内容推荐
* DRM 绕过
* Paywall 绕过

措辞必须是：

```text
工具能力描述
```

而不是法律保证。

---

# 32. Third-party License Final Audit

完整检查：

```text
Cargo dependencies
npm dependencies
Tauri plugins
librqbit
WebView2 related components
player libraries
icons
fonts
images
audio/video samples
test assets
```

生成或更新：

```text
THIRD_PARTY_NOTICES.md
LICENSES/
```

根据实际依赖许可证记录：

```text
Package
Version
License
Source
Required Notice
```

禁止凭印象填写许可证。

必须从：

```text
Cargo.lock
Cargo metadata
package lock
actual dependency metadata
```

确认。

---

# 33. 测试资产 License Audit

尤其检查媒体测试样本。

所有正式仓库中的：

```text
mp4
webm
mkv
mp3
jpg
png
subtitle
torrent
```

测试资源都要明确来源和许可。

避免把未知来源媒体打进 Release。

---

# 34. WebView2 / Player Final Compatibility

利用 M0.5 已建立的兼容性矩阵。

不要在 M12 重新发明播放器。

建立最终支持矩阵：

| Format | Container | Codec     | Play | Seek | Progressive | Status |
| ------ | --------- | --------- | ---: | ---: | ----------: | ------ |
| MP4    | MP4       | H.264/AAC |      |      |             |        |
| WebM   | WebM      | VP9/Opus  |      |      |             |        |
| WebM   | WebM      | AV1/Opus  |      |      |             |        |
| MKV    | MKV       | HEVC/etc. |      |      |             |        |

必须区分：

```text
SUPPORTED
CONDITIONALLY SUPPORTED
NOT SUPPORTED
```

不要把：

```text
理论可播放
```

写成：

```text
支持
```

---

# 35. Error Handling Final Audit

建立最终错误矩阵：

```text
Validation
Network
HTTP
Filesystem
Persistence
Torrent
Resource
Availability
Scheduler
Playback
Gateway
Checksum
Migration
Recovery
Cancelled
Internal
```

每一类确认：

```text
Backend Error
↓
TideError
↓
IPC DTO
↓
Frontend Error Message
```

用户看到的必须是可理解信息。

不要向用户暴露：

```text
Rust stack trace
SQL error internals
filesystem internals
panic messages
secret values
```

---

# 36. State Machine Final Audit

最终审计：

```text
Created
Ready
Running
Paused
Completed
Failed
Cancelled
```

检查：

* 所有合法 transition
* 所有非法 transition
* restart recovery
* retry semantics
* cancel semantics
* pause semantics
* complete semantics

重点：

```text
Cancelled
```

不能在下一次启动时突然自动变成：

```text
Running
```

除非产品有明确设计。

---

# 37. Download Task / Playback Session 解耦验证

必须再次确认：

```text
DownloadTask
```

和：

```text
PlaybackSession
```

是两个生命周期。

例如：

```text
关闭播放器
```

不应该：

```text
删除 DownloadTask
```

同时：

```text
取消 DownloadTask
```

不应该导致：

```text
Player 崩溃
```

而应该得到清晰的：

```text
resource unavailable
```

状态。

---

# 38. Scheduler Final Audit

M7 的 Scheduler 必须最终验证：

```text
User Demand
↓
Priority
↓
Scheduling Decision
↓
Engine
```

确认：

* 普通下载不会永久饿死
* playback demand 可以获得合理优先级
* seek demand 可以临时提高优先级
* cancel 后 demand 会消失
* playback session 结束后 demand 被清理
* 不会出现 stale demand
* 不会出现无限 priority escalation
* 不会出现 scheduler loop
* 不会因为一个资源失败导致其他任务停滞

---

# 39. Concurrency Final Audit

必须检查：

```text
multiple downloads
+
multiple playback sessions
+
multiple gateway requests
+
scheduler
+
database
```

是否存在：

```text
deadlock
race
double finalize
double cancel
use-after-close
stale state
duplicate event
duplicate task
```

至少进行：

```text
1 task
5 tasks
10 tasks
20 tasks
```

实际压力测试。

不需要为了 1.0 构造大型 benchmark farm。

重点是找出：

> 正常用户多任务使用下是否存在明显稳定性问题。

---

# 40. Long-running Stability Test

进行真实长时间运行测试。

例如：

```text
multiple HTTP tasks
+
Torrent tasks
+
at least one playback session
```

持续运行。

观察：

```text
Memory
CPU
Disk IO
Thread count
Task count
Gateway connections
DB size
Log growth
```

确认：

* 内存没有明显持续增长
* gateway session 正常清理
* task handle 正常释放
* cancellation token 正常释放
* DB connection 正常管理
* 临时文件不会无限增加
* log 不会无限爆炸

---

# 41. Offline / Network Failure

测试：

```text
network disconnect
↓
network reconnect
```

包括：

* 下载中断
* torrent peers 暂时消失
* HTTP timeout
* DNS failure
* server unavailable
* connection reset
* slow response
* partial response

确认：

```text
retry
resume
failed
paused
```

语义清晰。

---

# 42. Disk Failure

至少测试：

```text
disk full
directory inaccessible
file locked
destination disappeared
permission denied
```

确认不会：

```text
panic
corrupt state
mark Completed incorrectly
```

并且错误能够真正到达 UI。

---

# 43. Database Failure

模拟：

```text
SQLite unavailable
migration failure
DB locked
corrupted DB
```

确认：

* 不会静默启动成空白新应用
* 不会把原任务全部“丢失”
* 不会偷偷覆盖数据库
* 不会在用户不知情情况下重建数据

如果恢复机制有限：

必须明确记录到文档。

---

# 44. Corruption / Consistency Test

最终至少覆盖：

### Case A

```text
DB downloaded_bytes
>
physical .part size
```

必须回退到安全位置。

### Case B

```text
physical .part size
>
DB downloaded_bytes
```

不能未经验证就直接全部视作安全数据。

### Case C

```text
.final exists
.part exists
```

必须根据状态和内容判断，而不是简单覆盖。

### Case D

```text
Completed in DB
but final file missing
```

不能继续保持虚假的 Completed。

### Case E

```text
final file exists
but DB says Running
```

必须进行 reconciliation。

---

# 45. Final Integrity Test

对于测试资源：

```text
source
↓
Tide download
↓
SHA-256
↓
expected SHA-256
```

必须一致。

对 progressive playback：

不能只验证“能播放”。

还必须验证：

```text
random range
```

返回的数据与原始资源完全一致。

---

# 46. Migrations Final Audit

建立：

```text
Migration Matrix
```

例如：

```text
DB v1
DB v2
DB v3
...
Current DB
```

逐版本测试升级。

确认：

* migration 顺序正确
* 不可逆操作有说明
* 数据字段保真
* 默认值合理
* nullable 逻辑正确
* index 存在
* constraint 存在
* migration transaction 正确

---

# 47. Fresh DB Test

同时验证：

```text
Fresh Install
↓
Empty Database
↓
First Launch
```

不会要求：

```text
manual SQL
```

或者开发者干预。

---

# 48. Backup / Recovery Documentation

即使 1.0 暂时不提供自动 backup，也必须记录：

```text
App Data Location
Database Location
Download Root
Library Location
```

以及：

```text
哪些数据可以手工备份
哪些数据可以恢复
恢复时有哪些要求
```

注意不要在产品 UI 中加入一整套 Backup System，除非 M11 已经实现并且属于既定范围。

---

# 49. README 最终版

重新整理：

```text
What is Tide
Features
Architecture
Requirements
Installation
Usage
Supported Sources
Supported Media
Torrent Notes
Security
Privacy
Content Policy
Data Locations
Troubleshooting
License
Third-party Notices
```

首页必须明确 Tide：

> 是本地工具，而不是资源搜索/发现平台。

---

# 50. CHANGELOG

正式生成：

```text
CHANGELOG.md
```

至少包含：

```text
1.0.0
```

分为：

```text
Added
Improved
Fixed
Security
Known Limitations
```

不要写：

```text
marketing fluff
```

不要夸张：

```text
industry-leading
best downloader
fastest
100% secure
```

只描述实际能力。

---

# 51. Release Notes

生成：

```text
docs/release-1.0.0.md
```

内容包括：

```text
Release Overview
Major Features
Supported Sources
Playback
Streaming
Persistence
Scheduler
Windows Integration
Security / Privacy
Known Limitations
Upgrade Notes
```

重点写：

> 1.0 实际支持什么，以及明确不支持什么。

---

# 52. Known Limitations

必须诚实列出：

```text
codec limitations
container limitations
network limitations
torrent limitations
filesystem limitations
Windows limitations
player limitations
scheduler limitations
```

尤其：

```text
部分媒体格式依赖系统 WebView2 codec 能力
```

这种平台差异必须真实体现。

---

# 53. 不允许隐藏已知问题

所有 P1 / P2 必须分类：

```text
P0 = Must Fix
P1 = Strong Release Concern
P2 = Post-1.0
```

M12 最终：

```text
P0 = 0
```

P1：

> 必须逐项解释为什么不会阻止 1.0，或者修掉。

P2：

> 记录进入 1.1 backlog。

---

# 54. CI 最终检查

确认 CI 至少覆盖：

```text
cargo check
cargo test
frontend test
frontend build
tauri build
```

如已有：

```text
clippy
fmt
typecheck
lint
```

则执行。

推荐使用：

```text
cargo check --locked
cargo test --locked
cargo fmt --check
```

以及项目实际对应 frontend commands。

不能通过：

```text
修改 lockfile
```

偷偷规避依赖问题。

---

# 55. Dependency Freeze

1.0 发布前建立：

```text
Dependency Freeze
```

原则：

> 除非是明确的 release blocker，否则 M12 不主动升级依赖。

特别是：

```text
Tauri
Rust crates
librqbit
reqwest
SQLite layer
player/webview dependencies
```

避免在最后阶段引入新的行为变化。

如果必须升级：

必须记录：

```text
Why
Version
Breaking Changes
Affected Areas
Tests
Rollback
```

---

# 56. Build Reproducibility

尽量验证：

```text
Clean Checkout
↓
Install Dependencies
↓
Locked Build
↓
Release Artifact
```

可以成功完成。

不得依赖：

```text
developer local path
developer local cache
hidden DLL
manual environment variable
```

才能启动。

---

# 57. Windows Runtime Dependency Audit

最终检查正式用户机器是否必须安装：

```text
WebView2
VC Runtime
其他 runtime
```

如果需要：

必须明确：

```text
Required
Bundled
Installer Checks
Manual Requirement
```

不能假设开发机器存在。

---

# 58. Installer Security

检查正式安装包：

* 来源明确
* 不包含 debug payload
* 不包含测试媒体
* 不包含 secrets
* 不包含开发数据库
* 不包含开发日志
* 不包含 source maps（除非明确需要）
* 不包含无关 binaries
* 文件路径正确
* 权限合理

如有代码签名能力：

验证签名。

如果没有：

不要伪造“已签名”。

应在报告中明确：

```text
Code Signing: NOT AVAILABLE / NOT CONFIGURED
```

或者真实记录签名状态。

---

# 59. Artifact Hash

正式构建完成后：

为最终发布文件生成：

```text
SHA-256
```

例如：

```text
Tide-1.0.0-setup.exe
SHA-256: ...
```

生成：

```text
SHA256SUMS.txt
```

如项目有其他正式 artifact，也一并记录。

---

# 60. Release Artifact Self-Test

安装包生成后：

**禁止直接宣布完成。**

必须把 Release Artifact 当成第三方用户拿到的文件测试。

流程：

```text
Fresh Windows Environment
↓
Download Release Artifact
↓
Verify SHA-256
↓
Install
↓
Launch
↓
Create Task
↓
Download
↓
Playback
↓
Restart
↓
Recovery
↓
Uninstall
```

不要直接运行开发目录中的 executable 代替 installer 验收。

---

# 61. Clean Machine Test Matrix

至少进行：

| 场景           | Fresh Install | Upgrade | Restart | Uninstall |
| ------------ | ------------: | ------: | ------: | --------: |
| Empty App    |               |         |         |           |
| HTTP Task    |               |         |         |           |
| Paused Task  |               |         |         |           |
| Partial Task |               |         |         |           |
| Torrent Task |               |         |         |           |
| Playback     |               |         |         |           |
| Library      |               |         |         |           |

---

# 62. UI Final Acceptance

1.0 UI 不要求继续新增功能。

只确认：

* 所有核心按钮工作
* Loading 正常
* Error 正常
* Empty state 正常
* Disabled state 正常
* Progress 真实
* Speed 真实
* ETA 真实或明确 unavailable
* 状态一致
* Restart 后一致
* 异常后可理解
* 不能点击的东西不能看起来能点击

禁止出现：

```text
fake progress
fake speed
fake availability
fake buffering
fake completion
```

---

# 63. UI 与 Backend State 一致性

重点检查：

```text
Backend State
vs
Frontend State
```

包括：

```text
Running
Paused
Failed
Completed
Cancelled
Recovering
Buffering
Playing
```

如果 frontend reload：

```text
list_tasks
```

必须恢复真实状态。

不能依赖：

```text
React local state
```

作为持久事实。

---

# 64. Accessibility / UX Final Pass

只做低风险最终修正：

* keyboard navigation
* button labels
* tooltip
* focus
* disabled states
* error readability
* truncation
* long filename
* long resource name
* large number formatting
* empty state

不要因此引入新的设计系统。

---

# 65. Long Filename / Unicode Final Test

至少测试：

```text
中文
日文
韩文
emoji
空格
括号
特殊字符
超长文件名
超长路径
```

确认：

* DB 正常
* filesystem 正常
* UI 正常
* player 正常
* Library 正常
* gateway 正常

---

# 66. Localization Final Check

如果 Tide 有：

```text
zh-CN
en-US
```

等语言支持。

进行：

```text
missing key
fallback
placeholder
plural
number
time
byte size
error message
```

检查。

不能出现：

```text
undefined
missing translation key
```

或者直接把内部 enum：

```text
PlaybackResourceNotAvailable
```

暴露给用户。

---

# 67. Telemetry Final Check

默认：

```text
OFF
```

必须通过代码搜索确认没有隐藏：

```text
analytics
telemetry
tracking
beacon
user behavior collection
```

如果未来提供 opt-in：

必须明确：

```text
what
why
when
where
how to disable
```

---

# 68. Network Boundary Final Check

最终输出一份：

```text
Network Behavior Matrix
```

例如：

| Feature          | Network             | Destination                          | Default        |
| ---------------- | ------------------- | ------------------------------------ | -------------- |
| HTTP Download    | Yes                 | user URL                             | User initiated |
| Torrent          | Yes                 | peers / trackers / DHT as configured | User initiated |
| Playback Gateway | No external network | loopback                             | Internal       |
| Telemetry        | No                  | —                                    | Disabled       |
| Cloud Upload     | No                  | —                                    | Unsupported    |

目的：

> 让 1.0 用户和维护者能够知道 Tide 到底会和哪些网络实体通信。

---

# 69. No ifuyo Backend Dependency

最终验证：

```text
Tide
```

在：

```text
没有 ifuyo API
没有 ifuyo server
没有用户账号
没有 ifuyo database
```

情况下仍能够独立运行。

如果：

```text
ifuyo.com
```

完全不可用：

Tide 核心功能仍应正常。

---

# 70. App Data Isolation

确认：

```text
Application Data
```

与：

```text
User Download Data
```

严格区分。

例如：

```text
%LOCALAPPDATA%\ifuyo\Tide\
```

用于：

```text
DB
logs
settings
runtime metadata
```

而：

```text
User selected Download Root
```

用于：

```text
.part
completed files
```

不要把所有用户媒体都塞进 AppData。

---

# 71. Temp File Cleanup

最终验证：

```text
failed task
cancelled task
completed task
crashed task
recovered task
```

不会留下无意义的：

```text
.part
.tmp
.lock
session
gateway
```

垃圾。

但必须注意：

> 不得因为 cleanup 过度而删除用户仍然需要恢复的 `.part`。

---

# 72. Crash Recovery Final Gate

建立最终：

```text
Recovery Decision Matrix
```

至少包括：

| DB State  | .part   | Final  | Remote  | Decision                     |
| --------- | ------- | ------ | ------- | ---------------------------- |
| Running   | Exists  | No     | Same    | Resume                       |
| Running   | Exists  | No     | Changed | Restart                      |
| Paused    | Exists  | No     | Same    | Remain Paused                |
| Failed    | Exists  | No     | Same    | Remain Failed                |
| Cancelled | Exists  | No     | Same    | Cleanup / Preserve by policy |
| Completed | No      | Exists | —       | Completed                    |
| Completed | Exists  | Exists | —       | Reconcile                    |
| Running   | Missing | No     | —       | Recovery failure / restart   |

实际行为以已经实现的 recovery engine 为准，但必须：

> **确定性、可解释、可测试。**

---

# 73. Resource / Availability Final Audit

确认 M4 的抽象已经稳定：

```text
Resource
Availability
Range
Readable
```

至少能表达：

```text
resource exists
resource size known/unknown
range available
range unavailable
read range
```

不要在 M12 再创造新的：

```text
MediaResource2
StreamResource
TorrentResourceV2
```

除非这是明确的 release blocker。

---

# 74. Scheduler / Resource / Player Boundary Final Audit

确认：

```text
Player
```

不直接控制：

```text
Torrent Piece logic
```

```text
Scheduler
```

不直接控制：

```text
WebView2
```

```text
Gateway
```

不直接操作：

```text
raw filesystem arbitrary path
```

应该保持：

```text
Player
 ↓
Playback Session / Demand
 ↓
Resource
 ↓
Availability
 ↓
Scheduler
 ↓
Engine
```

---

# 75. Avoid Final-stage Architecture Rewrite

出现架构问题时，先分类：

```text
P0
P1
P2
```

只有：

```text
P0 / true P1
```

才允许进入 M12。

例如：

### 可以修

```text
race condition
data corruption
crash
security issue
broken migration
broken installer
broken core flow
```

### 不应该现在修

```text
class name 不够优雅
folder 可以重新组织
某个 trait 可以更漂亮
可以换状态机框架
可以引入新 DI framework
可以重做整个 UI
可以换数据库 ORM
可以换 player
```

---

# 76. Code Quality Final Gate

重点不是代码“漂亮”。

重点检查：

```text
unsafe
unwrap
expect
panic
dead code
feature flags
duplicate logic
global mutable state
thread lifecycle
async task leaks
channel leaks
resource leaks
blocking IO on UI path
blocking mutex
SQL in UI/application layer
raw path operations
raw URL handling
```

对真正危险的问题进行修复。

不要为了“代码审美”大规模重构。

---

# 77. Release Build Static Search

最终进行全项目搜索：

```text
TODO
FIXME
HACK
DEBUG
TEST_ONLY
MOCK
FAKE
localhost
127.0.0.1
example.com
password
secret
token
api_key
unwrap()
expect(
panic!(
```

然后逐项分类。

不能简单要求：

```text
所有 TODO 必须消灭
```

而应该：

```text
每一项都必须有明确 release classification。
```

---

# 78. Documentation Freeze

M12 结束时正式冻结：

```text
README.md
CHANGELOG.md
LICENSE
THIRD_PARTY_NOTICES.md
SECURITY.md
PRIVACY.md
CONTENT_POLICY.md
docs/
```

文档必须描述：

> 实际 1.0 能力，而不是未来计划。

---

# 79. Architecture Documentation

最终更新：

```text
docs/architecture/
```

至少确认：

```text
overview
domain model
download architecture
torrent architecture
resource / availability
scheduler
stream gateway
playback
persistence
recovery
security boundary
IPC
Windows integration
```

形成一份真正能够让未来维护者理解 Tide 的架构地图。

---

# 80. Release Decision

最终建立：

```text
docs/M12-release-decision.md
```

只允许：

```text
GO
NO-GO
CONDITIONAL GO
```

但：

```text
CONDITIONAL GO
```

必须列明精确条件。

不能用模糊语言：

```text
应该没问题
看起来可以
基本稳定
我感觉可以
```

必须有证据：

```text
test
build
manual validation
artifact validation
```

---

# 81. Final Test Pyramid

执行：

## Unit

覆盖：

```text
State
Range
Recovery
Validation
Serialization
Filename
Path
Checksum
Scheduler decisions
Availability
```

## Integration

覆盖：

```text
HTTP
Torrent
Persistence
Gateway
Playback
Library
Scheduler
```

## End-to-End

覆盖：

```text
Install
Launch
Download
Pause
Resume
Crash
Recover
Playback
Seek
Complete
Library
Restart
Uninstall
```

---

# 82. Final Acceptance Test

必须完成以下完整链：

```text
Fresh Install
    ↓
Launch
    ↓
Create HTTP Task
    ↓
Download Partial
    ↓
Pause
    ↓
Resume
    ↓
Crash
    ↓
Restart
    ↓
Recovery
    ↓
Range Resume
    ↓
Complete
    ↓
Checksum
    ↓
Library
    ↓
Open Media
    ↓
Playback
    ↓
Seek
    ↓
Close Player
    ↓
Download Another Resource
    ↓
Torrent
    ↓
Partial Availability
    ↓
Progressive Playback
    ↓
Seek into Unavailable Region
    ↓
Scheduler Demand
    ↓
Continue Playback
```

该链条不是要求一个自动化测试覆盖全部步骤。

可以拆成多个 acceptance tests。

但必须全部真实验证。

---

# 83. Release Artifact Validation

正式生成：

```text
Tide 1.0.0
```

后：

```text
hash artifact
install artifact
run artifact
test artifact
uninstall artifact
```

最终产物必须不是：

```text
developer build
```

而是真正的：

```text
release build
```

---

# 84. Git Release Discipline

检查：

```text
git status
```

确保：

* 无无关生成文件
* 无 secrets
* 无个人路径
* 无临时文件
* 无测试数据库
* 无 debug dump

确认：

```text
Cargo.lock
lockfiles
migrations
release config
docs
```

应该提交的内容都已提交。

---

# 85. Commit Strategy

M12 不需要产生大量无意义 commit。

建议按照实际工作拆成：

```text
release: freeze Tide 1.0 scope
fix: resolve release blockers
test: finalize release acceptance coverage
docs: finalize 1.0 documentation
build: prepare Tide 1.0.0 release artifacts
release: tag 1.0.0
```

实际 commit 数量按项目情况调整。

---

# 86. Git Tag

只有在：

```text
All release checks pass
```

之后：

```text
v1.0.0
```

创建正式 tag。

不要在测试失败时先打 tag。

---

# 87. Release Checklist

最终必须输出：

```text
[ ] Feature freeze complete
[ ] M0-M11 integrated
[ ] No P0
[ ] P1 reviewed
[ ] Core download validated
[ ] Crash recovery validated
[ ] Torrent validated
[ ] Progressive streaming validated
[ ] Seek validated
[ ] Scheduler validated
[ ] Library validated
[ ] Windows integration validated
[ ] Security audit complete
[ ] Privacy audit complete
[ ] Content policy complete
[ ] Third-party licenses complete
[ ] Database migration verified
[ ] Fresh install verified
[ ] Upgrade verified
[ ] Uninstall verified
[ ] Clean machine verified
[ ] Release build verified
[ ] Installer verified
[ ] Artifact hash generated
[ ] README finalized
[ ] CHANGELOG finalized
[ ] Release notes finalized
[ ] Known limitations documented
[ ] Git clean
[ ] v1.0.0 tag ready
```

---

# 88. 最终报告

生成：

```text
docs/M12-report.md
```

必须严格按照以下结构：

```markdown
# Tide M12 Final Release Report

## 1. Executive Summary

## 2. FACTS

## 3. CURRENT_M11_STATE

## 4. M12 OBJECTIVES

## 5. PRODUCT FREEZE

## 6. RELEASE BLOCKERS

## 7. NON-BLOCKING ISSUES

## 8. FINAL ARCHITECTURE

## 9. DOWNLOAD VALIDATION

## 10. TORRENT VALIDATION

## 11. RESOURCE / AVAILABILITY VALIDATION

## 12. PROGRESSIVE STREAMING VALIDATION

## 13. PLAYBACK VALIDATION

## 14. SCHEDULER VALIDATION

## 15. PERSISTENCE / RECOVERY VALIDATION

## 16. DATABASE MIGRATION

## 17. WINDOWS INTEGRATION

## 18. SECURITY AUDIT

## 19. PRIVACY AUDIT

## 20. CONTENT POLICY

## 21. THIRD-PARTY LICENSE AUDIT

## 22. INSTALL / UPGRADE / UNINSTALL TEST

## 23. CLEAN MACHINE TEST

## 24. LONG-RUN TEST

## 25. PERFORMANCE

## 26. ARTIFACTS

## 27. SHA-256

## 28. KNOWN LIMITATIONS

## 29. P0 / P1 / P2

## 30. RESOLVED

## 31. CONFIRMED

## 32. OPEN

## 33. DEFERRED TO 1.1+

## 34. CHANGED FILES

## 35. RELEASE DECISION

## 36. DEFINITION OF DONE
```

---

# 89. FACT / HYPOTHESIS / INFERENCE 要求

所有重要结论都必须区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

### FACT

```text
Release build completed successfully.
```

### FACT

```text
Crash recovery integration test passed.
```

### HYPOTHESIS

```text
WebView2 codec availability may vary by Windows environment.
```

### INFERENCE

```text
Therefore MKV should remain documented as conditionally supported rather than universally supported.
```

不得把推测写成事实。

---

# 90. RESOLVED / CONFIRMED / OPEN / WORSENED

最终问题管理必须使用：

```text
RESOLVED
CONFIRMED
OPEN
WORSENED
```

例如：

```text
[P1][RESOLVED] Resume could append after HTTP 200.
[P1][CONFIRMED] Gateway rejects arbitrary filesystem access.
[P2][OPEN] Certain HEVC combinations remain environment-dependent.
```

---

# 91. M12 的真正 Definition of Done

只有满足以下条件，才能宣布：

# Tide 1.0.0 Ready

### Product

```text
Core user journeys work.
```

### Reliability

```text
Crash recovery works.
```

### Data Integrity

```text
No known core path causes silent corruption.
```

### Security

```text
No known P0 security issue.
```

### Persistence

```text
Restart preserves expected state.
```

### Torrent

```text
Torrent workflow is functional and documented.
```

### Streaming

```text
Progressive playback path is functional for supported media.
```

### Player

```text
Play / pause / seek work for supported formats.
```

### Scheduler

```text
Download and playback demands interact predictably.
```

### Windows

```text
Install / launch / uninstall work.
```

### Release

```text
Release artifact has been independently validated.
```

### Documentation

```text
Documentation matches real 1.0 behavior.
```

### Legal / Safety

```text
Product does not market itself as an anonymity or copyright-bypass tool.
```

---

# 92. 最终禁止“假完成”

以下任何一种情况都不得宣布：

```text
1.0 Ready
```

如果：

```text
测试没跑
```

或者：

```text
只测试开发环境
```

或者：

```text
installer 没测
```

或者：

```text
upgrade 没测
```

或者：

```text
crash recovery 没测
```

或者：

```text
release artifact 没测
```

或者：

```text
存在未解释 P0
```

或者：

```text
文档与实际行为不一致
```

---

# 93. M12 结束时最终输出

最后必须向我报告：

```text
1. Tide 当前版本
2. 是否达到 1.0.0
3. Release Decision
4. 已修复的 Release Blocker
5. 仍存在的 P1
6. 仍存在的 P2
7. 最终测试数量
8. 自动化测试结果
9. E2E 结果
10. Crash Recovery 结果
11. Torrent 结果
12. Progressive Streaming 结果
13. Playback / Seek 结果
14. Scheduler 结果
15. Migration 结果
16. Installer 结果
17. Upgrade 结果
18. Uninstall 结果
19. Security Audit 结果
20. Privacy Audit 结果
21. License Audit 结果
22. Artifact 路径
23. Artifact SHA-256
24. Git commit
25. Git tag
26. Known Limitations
27. 1.1 Backlog
```

最终明确写：

```text
RELEASE DECISION: GO
```

或者：

```text
RELEASE DECISION: NO-GO
```

不要为了满足提示词而强行写 GO。

---

# 94. 最终执行原则

再次强调：

> **M12 不是“大版本最后再做一轮开发”。**

M12 应当是：

```text
M11 RC
   ↓
Freeze
   ↓
Audit
   ↓
Fix Release Blockers
   ↓
Validate
   ↓
Package
   ↓
Install
   ↓
Upgrade
   ↓
Crash Recovery
   ↓
E2E
   ↓
Security
   ↓
Documentation Freeze
   ↓
Artifact Hash
   ↓
v1.0.0
```

不要在 M12 中加入：

```text
“顺便做一个很酷的功能”
```

不要因为：

```text
“既然都做到这里了”
```

而扩大范围。

如果发现一个问题无法在不破坏稳定性的情况下安全修复：

```text
document
classify
defer
```

优先保证：

> **1.0 的可靠性 > 1.0 的功能数量。**

---

# M12 FINAL ACCEPTANCE PRINCIPLE

最终验收只问一个问题：

> **一个完全不知道 Tide 内部实现的普通 Windows 用户，拿到 Tide 1.0.0 安装包后，是否能够在没有开发环境、没有手工改数据库、没有手工修改配置的情况下，完成正常下载、Torrent、暂停/恢复、重启恢复、渐进式播放、Seek、Library 使用，并且在异常情况下不会轻易损坏任务或用户文件？**

如果答案成立：

```text
Tide 1.0.0
```

才真正成立。

否则：

```text
NO-GO
```

继续修复真正的 Release Blocker，而不是继续添加功能。
