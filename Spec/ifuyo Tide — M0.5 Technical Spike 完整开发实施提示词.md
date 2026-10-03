# ifuyo Tide — M0.5 Technical Spike 完整开发实施提示词

## 0. 任务定位

你现在进入 **ifuyo Tide M0.5：Technical Spike / 技术可行性闸门**。

M0 已经建立了：

- Tauri 2 + Rust + TypeScript/React 基础工程
- 基础 Domain / Repository / IPC 边界
- SQLite 持久化
- Task / Source / Resource / PlaybackSession 等基础模型
- 基础 Security Baseline
- 基础测试与工程规范

**M0.5 的目标不是继续堆产品功能。**

M0.5 的唯一核心任务是：

> **用最小、真实、可重复的实验，证明 Tide 的核心链路：**
>
> `真实 Torrent → librqbit → 部分数据可用 → 本地 HTTP/Range Stream → WebView2 <video> → 播放 / 暂停 / Seek`
>
> 并同时验证：
>
> `HTTP/HTTPS → 本地下载 → Progressive Read → Stream → WebView2`
>
> 最终给出一个有证据支撑的架构决策：
>
> **WebView2 是否足够作为 Tide 主播放器，以及后续 M3/M4/M5 应该如何落地。**

M0.5 是**技术决策节点**。

不要为了“看起来已经能播放”而把 M1/M3/M4/M5 大量代码提前塞进来。

---

# 1. 首要原则

严格遵守以下原则：

1. **先审计现有 M0 实现，再修改代码。**
2. 先输出：
   - FACTS
   - HYPOTHESES
   - SPIKE PLAN
   - RISKS
3. 所有实验优先使用：
   - 公版 / Public Domain 内容
   - 开源项目合法提供的测试资源
   - 自己生成的测试媒体
4. 禁止为了测试而寻找、下载、索引或导航明显未经授权的影视/盗版资源。
5. 不实现任何资源搜索、资源推荐、资源站导航、Tracker 推荐等功能。
6. 不做匿名下载、隐匿 IP 或绕过版权保护。
7. 所有结论必须区分：
   - 已实测
   - 依赖官方文档
   - 尚未验证
8. 如果某项测试失败，不允许为了让测试“通过”而偷偷引入大量后续架构。
9. M0.5 允许写 Spike / Prototype 代码，但必须与正式生产代码有明确边界。
10. 不要在 M0.5 引入没有必要的新第三方框架。

---

# 2. M0.5 的核心问题

本阶段必须明确回答下面这些问题。

## Q1：librqbit 当前锁定版本能否满足 Tide 的真实需求？

验证：

- Magnet 是否可用
- Torrent metadata 获取是否可用
- Torrent file 是否可用
- 文件选择是否可控
- piece 下载是否可观察
- 顺序 / 流式优先下载是否可用
- 文件读取是否可以阻塞等待尚未完成的数据
- Range / Seek 是否可行
- 下载与读取能否并发进行
- 暂停 / 恢复是否影响读取
- 多文件 Torrent 如何定位具体媒体文件
- 大文件是否能稳定工作

**不要凭旧 rqbit 文档或网上代码猜 API。**

必须以当前 `Cargo.lock` / 实际 dependency version / 当前源码和编译结果为准。

---

# 3. M0.5 第一阶段：环境和依赖审计

先检查：

```text
Cargo.toml
Cargo.lock
package.json
src/
src-tauri/
capabilities/
migrations/
```

确认：

- Tauri 版本
- Rust edition
- Node / package manager
- Windows target
- WebView2 runtime
- 当前已锁定 dependency
- 是否已经存在任何播放器相关依赖
- 是否已经存在任何 HTTP server / Range 组件
- 是否已经存在任何 Torrent 代码

输出：

```text
M0 FACTS
- Tauri:
- Rust:
- Frontend:
- WebView2:
- librqbit:
- SQLite:
- Existing streaming code:
- Existing player code:
```

然后检查当前 `librqbit` 实际版本。

重点：

- 不要升级依赖只是为了 Spike 方便。
- 如确需升级，必须先说明原因。
- 默认保持 M0 锁定版本不变。

---

# 4. M0.5 实验边界

本阶段只建立一个：

```text
experimental / spike
```

边界。

推荐：

```text
src-tauri/src/spike/
```

或者：

```text
src-tauri/src/experimental/
```

前端可以：

```text
src/features/spike/
```

所有 Spike 代码必须有清晰标识：

```rust
// M0.5 SPIKE
```

不能让实验代码未经验证直接污染核心架构。

---

# 5. 实验 A：WebView2 本地媒体播放基线

首先完全绕开 Torrent。

建立最小播放器页面。

测试：

```html
<video controls />
```

至少测试以下组合。

## A1：MP4 + H.264 + AAC

这是第一优先级。

测试：

- load
- play
- pause
- seek
- duration
- currentTime
- ended
- error
- stalled
- waiting
- progress

记录结果。

---

## A2：WebM + VP9 + Opus

测试：

- load
- play
- pause
- seek
- duration
- error

---

## A3：WebM + AV1 + Opus

测试：

- load
- play
- pause
- seek
- error

---

## A4：MKV

测试：

- 是否能直接通过 `<video>` 播放
- 容器不支持时错误表现是什么
- 是否存在“浏览器支持 codec，但容器仍不能播放”的情况

---

## A5：HEVC / H.265

必须实际测试。

不要写成：

> “Windows 支持 HEVC，所以 WebView2 肯定支持。”

必须记录：

- 当前测试机器
- Windows 版本
- WebView2 Runtime 版本
- 是否安装额外 codec
- 播放结果

---

# 6. 建立 Media Compatibility Matrix

创建：

```text
docs/M0.5-media-compatibility.md
```

表格至少包括：

| Container | Video | Audio | WebView2 | Seek | Notes |
|---|---|---|---|---|---|
| MP4 | H.264 | AAC | PASS/FAIL | PASS/FAIL | |
| WebM | VP9 | Opus | PASS/FAIL | PASS/FAIL | |
| WebM | AV1 | Opus | PASS/FAIL | PASS/FAIL | |
| MKV | H.264 | AAC | PASS/FAIL | PASS/FAIL | |
| MP4 | HEVC | AAC | PASS/FAIL | PASS/FAIL | |

增加字段：

```text
Windows version
WebView2 version
Codec pack
File size
Duration
Local file / HTTP
```

最终区分：

```text
SUPPORTED
CONDITIONALLY_SUPPORTED
UNSUPPORTED
NOT_TESTED
```

绝对不要把“理论支持”写成“实际支持”。

Microsoft 当前文档本身也明确提示某些 codec 的支持存在设备/可选 codec 条件，因此最终结论必须以 Tide 实测环境为准。

---

# 7. 实验 B：HTTP Progressive Playback

建立一个最简单的本地 HTTP 数据源。

不要马上使用真实下载器。

准备一个本地媒体文件。

实现一个最小测试 server：

```text
GET /media/test.mp4
```

支持：

```text
200
206
Accept-Ranges: bytes
Content-Length
Content-Range
Content-Type
```

验证 WebView2：

```text
HTTP → video
```

测试：

1. 首次加载
2. 播放
3. 暂停
4. 向前 Seek
5. 向后 Seek
6. 拖动进度条
7. 快速连续 Seek
8. Range 请求
9. 文件较大时行为
10. server 边发送边读取

记录浏览器实际发出的请求：

```text
Request headers
Range
Response status
Content-Range
Content-Length
```

建立：

```text
docs/M0.5-http-range.md
```

---

# 8. 实验 C：Torrent 基础能力

现在才接入真实 `librqbit`。

使用合法测试 Torrent。

推荐优先：

- Linux ISO
- Public Domain 文件
- 自己制作的 Torrent
- 其他明确允许 P2P 测试的开源资源

不要使用未经授权的商业影视资源。

验证：

```text
Torrent metadata
↓
Torrent session
↓
file list
↓
selected file
↓
piece availability
↓
download
```

最低要求：

- 能添加 Torrent
- 能获得 Torrent metadata
- 能列出文件
- 能选择目标媒体文件
- 能启动下载
- 能读取下载状态
- 能检测已完成数据

---

# 9. 实验 D：Torrent Progressive Read

这是整个 M0.5 最重要的实验。

目标：

> **文件尚未完全下载时，能否从已经获得或正在获得的数据开始读取。**

测试流程：

```text
Add Torrent
↓
Select video file
↓
Start download
↓
Request beginning of video
↓
等待必要 pieces
↓
Read bytes
↓
送入 HTTP stream
↓
WebView2 video
```

重点观察：

- 首帧等待时间
- 哪些 pieces 被优先请求
- 数据是否真的按播放需求优先获取
- 未就绪数据是否会正确等待
- 下载是否继续进行
- 播放是否会把下载任务饿死
- 播放读取是否影响其他 piece 的下载

不要只测试：

> “Torrent 下载完以后能播放。”

这种结果对 Tide 的关键架构没有证明意义。

必须明确验证：

> **未完成 Torrent 是否能够直接支撑播放。**

rqbit 当前官方项目描述中明确包含 Torrent streaming、对播放所需 pieces 进行优先级处理以及支持 Range/seek，因此这是 M0.5 最值得做真实性验证的链路。

---

# 10. 实验 E：Torrent Seek

测试：

```text
播放开始
↓
Seek +30s
↓
Seek +5min
↓
Seek +20min
↓
Seek backward
```

每次记录：

```text
seek target
↓
HTTP Range
↓
required data
↓
piece availability
↓
wait time
↓
playback resumed
```

特别关注：

> Seek 到尚未下载的位置时，rqbit 是否能够正确优先请求对应数据并等待，而不是必须从文件头顺序下载到那里。

---

# 11. 实验 F：多文件 Torrent

测试一个包含：

```text
video.mp4
subtitle.srt
cover.jpg
other.bin
```

的 Torrent。

验证：

- 文件列表
- file index
- file path
- file size
- media file selection
- HTTP stream 指向正确文件
- 不会越界读取其他文件
- 不会因为错误 file id 访问任意磁盘文件

这一步非常重要，因为未来 Tide 的：

```text
Resource
Availability
PlaybackSession
Gateway
```

都不能假设：

> 一个 Torrent = 一个文件。

---

# 12. 实验 G：Tide Progressive Stream Gateway Spike

这里只实现 **最小安全 Spike**。

不要直接写成 M4 正式 Gateway。

建立：

```text
GET /stream/{session}/{resource}
```

最小能力：

```text
GET
HEAD
Range
206
416
Content-Length
Content-Range
Content-Type
Accept-Ranges
```

只允许：

```text
loopback
```

例如：

```text
127.0.0.1
```

禁止：

```text
0.0.0.0
LAN
WAN
mDNS
UPnP
```

---

# 13. Gateway 安全实验

即使是 Spike，也必须验证下面这些边界。

## 13.1 不允许任意文件路径

错误：

```text
/stream/C:/Users/...
```

错误：

```text
../
```

错误：

```text
..\..\...
```

Gateway 不允许客户端自己指定文件系统路径。

必须是：

```text
session_id
resource_id
```

映射到内部对象。

---

## 13.2 Session Token

不要使用：

```text
/stream/123/456
```

这种可枚举 ID。

Spike 至少使用：

```text
random high entropy session token
```

并绑定：

```text
session → resource
```

---

## 13.3 Host 验证

测试：

```text
Host: 127.0.0.1
Host: localhost
Host: malicious-host
```

确保不会因为错误 Host 导致异常行为。

---

## 13.4 Range 验证

至少测试：

```text
bytes=0-1023
bytes=1024-
bytes=-1024
bytes=1000-1999
bytes=9999-10000
bytes=10000-9999
multiple ranges
malformed range
```

验证：

```text
206
416
```

行为符合预期。

---

# 14. 实验 H：WebView2 → Tide Gateway → Torrent

这是 M0.5 的最终 Demo。

最终链路必须真实运行：

```text
WebView2
    ↓
<video src="http://127.0.0.1:random-port/...">
    ↓
Tide Local Gateway
    ↓
Resource
    ↓
librqbit
    ↓
Torrent pieces
    ↓
Disk / piece storage
```

要求：

### H1 首播

Torrent 尚未完整下载时：

```text
Add Torrent
↓
select video
↓
Play
↓
buffer
↓
video starts
```

---

### H2 播放期间继续下载

播放过程中检查：

```text
download progress increases
```

并确认：

```text
player read ≠ full file download first
```

---

### H3 Seek

播放过程中：

```text
Seek forward
```

Gateway 必须收到：

```text
Range
```

并请求正确的数据区域。

---

### H4 Seek 未下载区域

这是最关键的场景：

```text
Current playback = 1 min

Seek = 20 min

20 min corresponding data = not downloaded
```

期望：

```text
Range request
↓
required torrent pieces prioritized
↓
wait
↓
data available
↓
play resumes
```

如果失败，不允许强行包装成 PASS。

---

# 15. 必须测量的数据

M0.5 不是“肉眼看到能播”就结束。

至少记录：

```text
startup latency
time to first frame
download speed
playback position
buffer / stall events
seek latency
torrent piece availability
requested Range
bytes read
bytes downloaded
```

建议定义：

```text
T_first_frame
T_seek
T_resume
T_stall
```

不要求在此阶段做性能优化。

重点是获得真实数据。

---

# 16. 实验 I：Pause / Resume

测试：

```text
Torrent downloading
+
video playing
```

执行：

```text
pause
```

观察：

- player
- gateway
- torrent
- pending read
- task state

然后：

```text
resume
```

验证：

- session 是否恢复
- read 是否恢复
- player 是否恢复
- 不会死锁
- 不会创建重复 Torrent session

---

# 17. 实验 J：App Restart

这是为了验证 M0 的持久化边界是否正确。

流程：

```text
Torrent starts
↓
partial data
↓
close Tide
↓
reopen
↓
restore Torrent
↓
try playback
```

M0.5 不要求把完整 crash recovery 做完。

但必须明确：

```text
WHAT WORKS
WHAT DOES NOT WORK
WHAT M2/M3/M4 MUST IMPLEMENT
```

不要为了完成这个实验提前实现完整 M2。

---

# 18. 实验 K：Slow Network / Buffering

人为降低网络质量。

至少模拟：

```text
slow download
high latency
intermittent availability
```

观察：

```text
video waiting
download continues
required pieces prioritized
resume
```

核心问题：

> 当网络速度低于播放消耗速度时，Tide 的行为是否可预测？

记录：

```text
stall frequency
stall duration
resume time
```

---

# 19. 实验 L：HTTP 与 Torrent 的统一 Resource 抽象

M0.5 不要实现完整 Scheduler。

只验证一个接口是否成立：

```text
ReadableResource
```

概念上要求支持：

```text
read(range)
availability(range)
```

例如：

```text
Resource
 ├── HttpResource
 └── TorrentResource
```

目标不是完成生产代码。

目标是验证：

> Gateway / Player 是否可以不关心底层到底是 HTTP 还是 Torrent。

也就是说：

```text
Player
   ↓
Gateway
   ↓
Resource
   ↓
HTTP / Torrent
```

而不是：

```text
Player
 ↓
if torrent
else http
```

大量业务分支。

---

# 20. 实验 M：Seek 对 Resource 层的影响

验证：

```text
player seek
↓
HTTP Range
↓
resource.read(range)
↓
availability(range)
↓
scheduler / source
```

即未来的抽象方向：

```text
Demand
```

而不是简单：

```text
Download whole file
```

M0.5 不要求完整实现：

```text
DemandScheduler
```

但必须确认接口设计不会阻碍后续实现。

---

# 21. 明确禁止实现的内容

M0.5 绝对不要完成以下功能：

### 下载产品功能

- HTTP 下载完整 UI
- 多任务下载器
- 下载分类
- 下载历史
- 下载速度图表
- 自动重试策略
- 完整 Scheduler
- 下载限速 UI

这些属于 M1/M2/M7。

---

### Torrent 产品功能

- 完整 Torrent UI
- Tracker 管理
- Peer 管理
- DHT UI
- Seed UI
- Torrent 分类
- Torrent 搜索
- 资源站
- Tracker 推荐

属于后续阶段，并且 Tide 产品定位不包含资源发现。

---

### 播放器功能

不要实现：

- 字幕系统
- 字幕下载
- 缩略图系统
- 播放历史
- 播放列表
- 音轨选择
- 播放速度设置
- 全屏高级 UI
- 播放器主题系统
- Media Session
- Cast
- NAS
- UPnP

---

### 高级媒体后端

M0.5 不默认集成：

- FFmpeg
- libmpv
- VLC
- GStreamer

除非 **WebView2 Spike 已经明确证明当前目标媒体集合无法满足 Tide 的产品需求**。

即使需要备用播放器，也先给出证据，不要提前引入。

---

# 22. WebView2 播放器决策标准

M0.5 最终必须回答：

## Decision A

```text
WebView2 only
```

是否可作为 Tide 1.0 主播放器？

---

## Decision B

```text
WebView2 + native fallback
```

是否必要？

---

## Decision C

如果需要 native fallback：

选择哪个方向：

```text
libmpv
FFmpeg-based pipeline
external player
other
```

这里只进行技术评估，不要求立即集成。

---

# 23. 播放器架构决策文档

创建：

```text
docs/M0.5-player-decision.md
```

格式：

```text
# Player Decision

## Environment
Windows:
WebView2:

## Tested Formats

## Passed

## Conditional

## Failed

## Seek Results

## Progressive HTTP Results

## Progressive Torrent Results

## Risks

## Decision

## Reason

## Consequences for M4/M5

## Rejected Alternatives
```

最终 Decision 必须是明确的：

```text
DECISION:
WebView2-only
```

或者：

```text
DECISION:
WebView2 + native fallback
```

或者：

```text
DECISION:
WebView2 unsuitable for primary playback
```

但必须基于实测证据。

---

# 24. M0.5 应该产生的正式资产

至少产生：

```text
docs/M0.5-media-compatibility.md
docs/M0.5-http-range.md
docs/M0.5-player-decision.md
docs/M0.5-torrent-streaming.md
docs/M0.5-architecture-decisions.md
```

可以产生：

```text
src-tauri/src/spike/
src/features/spike/
```

但必须明确：

```text
SPIKE CODE
```

哪些代码可进入生产、哪些必须删除/重写，要在最终报告中明确。

---

# 25. M0.5 的测试工程

至少加入：

### Range

- normal range
- open-ended
- suffix range
- invalid range
- out-of-bound
- reversed range

### Gateway

- valid session
- invalid session
- expired session
- arbitrary path rejection
- invalid Host rejection
- invalid Range rejection

### Resource

- HTTP read
- Torrent read
- unavailable range
- available range

### Torrent

- metadata
- file selection
- partial availability
- read pending
- read resume

### Player

至少有一个真实集成测试或手工测试流程：

```text
Torrent partial download
→ Gateway
→ WebView2
→ Play
→ Seek
```

测试必须真正运行。

---

# 26. 日志要求

日志可以帮助 Spike 调试，但不要把敏感内容直接打出来。

禁止生产日志出现：

- 完整 Magnet URI
- 完整下载 URL
- Peer IP
- 用户完整文件系统路径
- 用户隐私历史

可以记录：

```text
task_id
resource_id
session_id
torrent lifecycle event
file index
range start/end
read latency
availability state
```

并对高频日志做采样或 debug-level 控制。

---

# 27. Gateway 生命周期实验

验证：

```text
App start
↓
Gateway bind random loopback port
↓
session created
↓
player connects
↓
session destroyed
↓
port/server cleanup
```

测试：

- app 正常退出
- player 页面刷新
- session 过期
- resource 删除
- task pause
- task cancel

不得留下：

```text
zombie server
```

不得监听：

```text
0.0.0.0
```

---

# 28. 失败处理

特别关注：

```text
Torrent has no peers
Torrent metadata unavailable
Requested piece unavailable
File not found
Torrent paused
Resource deleted
Gateway session expired
Player aborts request
Range invalid
WebView2 rejects codec
```

必须确保：

```text
Rust error
↓
TideError
↓
stable DTO
↓
frontend
```

而不是把：

```text
Rust panic / debug stack
```

泄露到前端。

---

# 29. 不允许 Panic 驱动业务

对于：

- invalid session
- invalid resource
- invalid range
- missing file
- torrent unavailable

必须返回结构化错误。

禁止：

```rust
unwrap()
expect()
panic!()
```

承担正常业务失败。

Spike 中可以存在极少数开发期 invariant 检查，但最终报告必须说明。

---

# 30. 性能目标

M0.5 不做系统性性能优化。

只进行基本观察。

至少记录：

```text
CPU idle
CPU playback
Memory idle
Memory playback
Memory torrent playback
```

分别观察：

```text
Tide idle
HTTP playback
Torrent playback
Seek
```

不要为了追求某一个 benchmark 提前做缓存架构。

---

# 31. 是否共享 ifuyo 其他项目代码

继续遵守：

> **不要因为都是 ifuyo 项目，就创建共享 core。**

不要把 Tide 的：

```text
Torrent piece storage
Range availability
Scheduler
Playback
```

抽到通用：

```text
ifuyo-core
```

除非实际发现存在已经稳定、明确、低耦合的公共基础组件。

M0.5 不允许因为“以后可能复用”而提前抽象。

---

# 32. Git 边界

M0.5 可以产生 Spike 代码。

但在提交前必须区分：

```text
Production
Spike
Docs
Tests
```

避免：

```text
大量实验死代码
```

永久进入正式核心目录。

推荐提交结构：

```text
feat: add M0.5 technical spike infrastructure
test: validate WebView2 media compatibility
test: validate HTTP progressive range playback
test: validate torrent progressive streaming
docs: record M0.5 player decision
```

不要把所有实验内容压成一个巨型 commit。

---

# 33. M0.5 Definition of Done

只有以下条件全部满足，M0.5 才算完成。

## Engineering

- [ ] 当前 M0 工程审计完成
- [ ] librqbit 实际版本确认
- [ ] WebView2 环境确认
- [ ] Spike 与 production boundary 清晰
- [ ] 依赖没有无意义升级

## Player

- [ ] WebView2 本地文件播放测试
- [ ] HTTP playback 测试
- [ ] Range 测试
- [ ] Seek 测试
- [ ] 至少 4 种容器/codec 组合测试
- [ ] Media Compatibility Matrix 完成

## Torrent

- [ ] 真实 Torrent 成功运行
- [ ] Metadata 成功
- [ ] File selection 成功
- [ ] Partial download 成功
- [ ] Progressive read 成功/失败被明确记录
- [ ] Seek 未下载区域测试
- [ ] Multi-file Torrent 测试

## Gateway

- [ ] loopback only
- [ ] random port
- [ ] session token
- [ ] resource mapping
- [ ] Range validation
- [ ] invalid session rejection
- [ ] arbitrary path rejection
- [ ] invalid Host rejection

## Full Chain

- [ ] Torrent partial data
- [ ] Gateway
- [ ] WebView2
- [ ] Play
- [ ] Seek
- [ ] Resume

至少真实跑通一次。

## Documentation

- [ ] Media compatibility
- [ ] HTTP Range
- [ ] Torrent streaming
- [ ] Player decision
- [ ] Architecture decisions

---

# 34. 最终必须做出的架构决策

M0.5 结束时，不允许只写：

```text
测试完成。
```

必须回答：

### 1. Player

```text
WebView2 only
```

还是：

```text
WebView2 + native fallback
```

---

### 2. Torrent Streaming

回答：

> librqbit 是否足以作为 Tide 的底层 Torrent engine + progressive read engine？

---

### 3. Resource

回答：

> HTTP / Torrent 是否可以统一在 Resource / Availability 抽象下？

---

### 4. Gateway

回答：

> Tide 自己的 Local Progressive Gateway 是否值得作为正式 M4 核心层？

---

### 5. Seek

回答：

> Player Seek → Range → Resource Availability → Scheduler Demand 这条链是否成立？

---

### 6. Playback

回答：

> PlaybackSession 是否应该独立于 DownloadTask？

如果 Spike 暴露出当前 M0 模型存在结构性问题，可以修改 M0 架构。

---

# 35. Final Technical Report

最终生成：

```text
docs/M0.5-report.md
```

严格按照：

```text
# M0.5 Technical Spike Report

## 1. Executive Summary

## 2. Environment

## 3. FACTS

## 4. Hypotheses

## 5. Experiments

### E1 WebView2 Local Playback
### E2 HTTP Range
### E3 Torrent Metadata
### E4 Partial Torrent Read
### E5 Torrent Seek
### E6 Multi-file Torrent
### E7 Tide Gateway
### E8 Full Playback Chain
### E9 Pause / Resume
### E10 Restart

## 6. Results

## 7. Media Compatibility Matrix

## 8. Performance Observations

## 9. Security Findings

## 10. Architecture Findings

## 11. Decisions

## 12. Rejected Alternatives

## 13. M1 Impact

## 14. M3 Impact

## 15. M4 Impact

## 16. M5 Impact

## 17. Known Limitations

## 18. Follow-up Risks
```

---

# 36. Final Agent 输出格式

最后不要只给一句总结。

必须输出：

```text
FACTS
HYPOTHESES
IMPLEMENTED
EXPERIMENTS
TEST RESULTS
MEDIA MATRIX
TORRENT RESULTS
GATEWAY RESULTS
SECURITY RESULTS
ARCHITECTURE DECISIONS
FILES CHANGED
SPIKE CODE
PRODUCTION-CANDIDATE CODE
FAILED EXPERIMENTS
KNOWN LIMITATIONS
M1 READINESS
M3 READINESS
M4 READINESS
M5 READINESS
```

特别注意：

> **任何没有实际运行过的实验，不得写 PASS。**

只能写：

```text
NOT TESTED
```

或者：

```text
UNVERIFIED
```

---

# 37. 最终退出条件

M0.5 最重要的不是新增多少代码，而是回答：

> **Tide 的“边下边播”到底是不是一个真实成立、可工程化、值得进入 M3/M4 的架构。**

最终至少要得到下面这个结论之一：

```text
A.
Torrent → Resource → Gateway → WebView2
可稳定工作
```

或者：

```text
B.
Torrent → Resource → Gateway 成立
但 WebView2 对目标媒体覆盖不足
因此需要 native playback fallback
```

或者：

```text
C.
当前 Torrent progressive streaming 链路存在结构性问题
必须调整 Resource / Availability / Playback 架构
```

**不要为了通过里程碑而强行得到 A。**

真实失败结果同样是有效的 M0.5 产出。

---

# 38. 下一阶段接口预留

M0.5 完成后，才允许进入：

```text
M1 HTTP Download Core
M2 Task Persistence / Recovery
M3 Torrent Engine
M4 Progressive Streaming / Scheduler / Gateway
M5 Player
```

M0.5 不提前实现这些完整功能。

尤其不要提前写完整：

```text
Scheduler
```

M0.5 只需要验证：

```text
Demand
Availability
Resource
Range
Playback
```

这些边界在技术上能够成立。

---

# 39. 执行纪律

整个执行过程中：

### 第一步

先审计。

### 第二步

输出：

```text
FACTS
HYPOTHESES
SPIKE PLAN
```

### 第三步

建立最小实验。

### 第四步

运行真实测试。

### 第五步

记录数据。

### 第六步

修改必要的模型/接口。

### 第七步

再次运行。

### 第八步

形成 Architecture Decision。

**禁止一开始就大规模重构。**

**禁止为了展示效果制造 fake progress / fake streaming / fake playback。**

**禁止用“理论上应该可以”替代真实实验。**

**禁止把实验成功表述成生产级稳定性。**

---

## M0.5 的核心成功标准

最终用户打开 Tide，输入一个合法测试 Torrent，选择其中的视频文件：

```text
Torrent 尚未完成
        ↓
Tide 开始获取所需数据
        ↓
Local Gateway
        ↓
WebView2 <video>
        ↓
开始播放
        ↓
继续后台下载
        ↓
用户 Seek
        ↓
请求新的 Range
        ↓
获取对应 Torrent pieces
        ↓
恢复播放
```

**只要这条链路被真实验证，M0.5 就完成了它最重要的任务。**

其余能力全部让位于这个目标。