# ifuyo Tide — M5 Player 完整开发实施提示词

你现在进入项目 **ifuyo Tide 的 M5：Player**。

请把下面内容作为本阶段的正式实施规范。

---

# 一、项目上下文

Tide 是一个 Windows 桌面应用，定位为：

> **用户自己提供来源的本地下载器 + 渐进式媒体播放器。**

当前总体技术路线：

* Windows 11 x64 为主要目标平台
* Tauri 2
* Rust
* TypeScript
* React
* Vite
* SQLite
* WebView2 / Chromium media stack
* HTTP/HTTPS 下载
* BitTorrent / `librqbit`
* Local Progressive Stream Gateway
* Resource / Availability / Demand / Scheduler 架构

当前里程碑：

* M0：Foundation
* M0.5：Technical Spike / Media Compatibility
* M1：HTTP Download Core
* M2：Task Persistence / Crash Recovery / Resume
* M3：Torrent Engine
* M4：Progressive Streaming / Range Gateway / Scheduler / Seek
* **M5：Player ← 当前阶段**
* M6：Library
* M7：Smart Scheduler
* M8：Windows Integration
* M9：Polish
* M10：Security & Release Hardening
* M11：Release Candidate
* M12：1.0

本阶段必须严格建立在已经完成的 M0～M4 基础之上。

---

# 二、M5 的核心目标

M5 的唯一核心目标：

> **把 Tide 已有的 `Resource + Availability + Progressive Stream Gateway` 能力，变成一个真正可使用、可恢复、可观察、可处理异常的桌面播放器。**

最终形成：

```text
User
  ↓
Player UI
  ↓
PlaybackController
  ↓
PlaybackSession
  ↓
Resource
  ↓
Progressive Stream Gateway
  ↓
Resource.read(range)
  ↓
Availability / Scheduler / Download Engine
```

播放器本身不负责：

* HTTP 下载
* Torrent 下载
* Piece 管理
* DHT
* Tracker
* Peer 管理
* 文件完整性计算
* 下载任务持久化
* Scheduler 核心决策
* 任意文件系统读取

播放器只负责：

> **把一个已经获得授权的 `PlaybackSession/Resource` 播放出来，并正确把用户的播放行为转化成媒体控制行为。**

---

# 三、最重要的架构原则

## 3.1 Player ≠ Downloader

严禁出现：

```text
Player → HTTP Client
Player → librqbit
Player → SQLite
Player → fs::read()
Player → direct torrent file access
```

正确关系：

```text
Player
  ↓
PlaybackSession
  ↓
Resource
  ↓
Gateway
  ↓
Underlying Resource implementation
```

播放器不应该知道：

* Resource 是 HTTP 还是 Torrent
* 是否正在下载
* Peer 数量
* Piece 数量
* Tracker 状态
* DHT 状态
* 实际文件路径
* `.part` 文件在哪里
* SQLite 具体表结构

这些全部属于 backend/domain infrastructure。

---

# 四、实施前必须先审计

在修改代码之前，不要直接开始写 Player。

先完整检查：

1. M0 architecture
2. M0.5 technical spike
3. M0.5 media compatibility matrix
4. M0.5 player decision
5. M1 HTTP implementation
6. M2 persistence/recovery
7. M3 torrent implementation
8. M4 Resource
9. M4 Availability
10. M4 Range Gateway
11. M4 Scheduler / Demand
12. 现有 IPC
13. Tauri capabilities
14. CSP
15. 现有 frontend architecture
16. 当前 tests

优先检查这些实际代码：

```text
src-tauri/src/core/resource/
src-tauri/src/core/playback/
src-tauri/src/core/scheduler/
src-tauri/src/core/download/
src-tauri/src/core/task/
src-tauri/src/core/...
src-tauri/src/commands/
src/features/
src/components/
src/lib/
src-tauri/capabilities/
src-tauri/tauri.conf.json
```

不要假设目录一定完全符合上面结构。

---

# 五、先输出审计结论

修改代码之前先输出：

```text
FACTS
HYPOTHESES
CURRENT M0 STATE
CURRENT M0.5 STATE
CURRENT M1 STATE
CURRENT M2 STATE
CURRENT M3 STATE
CURRENT M4 STATE

PLAYER-READY COMPONENTS

1. Resource
2. Availability
3. PlaybackSession
4. Stream Gateway
5. Range support
6. Scheduler integration
7. IPC
8. Frontend media architecture

KNOWN GAPS

PLAYER IMPLEMENTATION PLAN

RISKS

P0
P1
```

其中：

* FACT = 可以从代码、测试、配置、文档直接确认
* HYPOTHESIS = 尚未完全验证
* INFERENCE = 根据事实推导出的结论

不要把推测写成事实。

---

# 六、M5 范围

M5 必须完成：

## A. Player Core

* 播放
* 暂停
* 停止
* seek
* volume
* mute
* playback rate
* duration
* current position
* progress
* buffering
* loading
* ended
* error
* source switching

## B. Playback Session

* 创建 PlaybackSession
* 启动播放
* 结束播放
* session 生命周期
* session 与 Resource 绑定
* session 与 Gateway token 绑定
* session cleanup

## C. WebView2 HTML5 Media

以：

```html
<video>
```

以及必要时：

```html
<audio>
```

作为播放器基础。

默认不要引入：

* libmpv
* VLC
* FFmpeg
* GStreamer
* 自己实现 decoder
* 自己实现 demuxer

除非 M0.5 明确证明 WebView2 路线无法满足既定产品目标。

如果 M0.5 已经明确要求 fallback，则严格按照 M0.5 的决策执行，不自行扩大技术路线。

---

# 七、格式支持策略

绝对不要自行声称：

> “Tide 支持所有视频格式。”

格式能力必须来自：

```text
M0.5 Compatibility Matrix
+
实际 WebView2 当前环境
+
真实媒体测试
```

优先保障已经验证的：

* MP4 / H.264 / AAC
* WebM / VP8 / VP9 / Opus
* AV1（若 M0.5 实机验证通过）
* 常见音频格式

对于：

* HEVC/H.265
* 特殊 MPEG
* XAVC
* 非常规封装
* 冷门 codec

必须根据实际兼容性矩阵处理。

Edge 当前文档明确指出 H.264、VP8/VP9、AV1 有支持路径，而 HEVC 是有条件支持，实际硬件/系统环境会影响结果。

因此播放器必须提供：

```text
Unsupported / Decoder unavailable / Media error
```

之类的可理解状态，而不是：

```text
播放失败
```

然后假装播放器存在问题。

---

# 八、统一 PlaybackSession

建立或复用统一：

```text
PlaybackSession
```

建议至少包含：

```text
session_id
resource_id
source_kind
resource_name
content_type
size
duration
state
gateway_url
created_at
started_at
last_position
volume
muted
playback_rate
error
```

具体字段以现有架构为准。

不要为了 Player 重复定义 Resource。

---

# 九、PlaybackSession 状态机

建议 Player 层使用：

```text
Idle
Loading
Ready
Playing
Paused
Buffering
Seeking
Ended
Error
Disposed
```

注意：

这些是 **Playback UI / Runtime State**。

不能把这些强行塞进：

```text
TaskState
```

也不能让：

```text
DownloadState
```

和：

```text
PlaybackState
```

互相污染。

---

# 十、状态转换必须明确

合法示例：

```text
Idle → Loading
Loading → Ready
Loading → Error

Ready → Playing
Ready → Paused

Playing → Paused
Playing → Buffering
Playing → Seeking
Playing → Ended
Playing → Error

Buffering → Playing
Buffering → Error

Paused → Playing
Paused → Seeking

Seeking → Buffering
Seeking → Playing
Seeking → Error

Ended → Playing
Ended → Disposed

任何状态 → Disposed
```

必须测试非法转换。

不要允许：

```text
Disposed → Playing
Disposed → Loading
```

---

# 十一、Player Controller

建立明确的：

```text
PlaybackController
```

Frontend 不应该到处直接操作：

```typescript
video.play()
video.pause()
video.currentTime = ...
video.volume = ...
```

建议统一：

```text
PlaybackController.play()
PlaybackController.pause()
PlaybackController.stop()
PlaybackController.seek()
PlaybackController.setVolume()
PlaybackController.setMuted()
PlaybackController.setPlaybackRate()
PlaybackController.load()
PlaybackController.unload()
```

然后由 Controller 操作实际 HTMLMediaElement。

这样以后换 native player backend 时，不需要重写整个 UI。

---

# 十二、HTMLMediaElement 事件模型

完整监听至少：

```text
loadstart
loadedmetadata
loadeddata
canplay
play
playing
pause
timeupdate
progress
durationchange
seeking
seeked
waiting
stalled
suspend
ended
error
volumechange
ratechange
emptied
```

这些事件应该经过一层：

```text
MediaEventAdapter
```

或者等价抽象。

不要让 React component 自己维护所有 media event。

HTMLMediaElement 本身已经提供上述媒体生命周期和 buffering/seek 相关事件，例如 `waiting` 表示因暂时数据不足而暂停播放，`stalled` 表示数据获取异常停滞，`seekable` 可用于判断当前可 seek 区间。

---

# 十三、不要把 HTML5 事件直接当最终业务状态

例如：

```text
waiting
```

不一定代表：

```text
network error
```

可能只是：

```text
Torrent 数据还没有到
```

因此必须建立：

```text
Media State
+
Backend Playback State
```

两个观察层。

例如：

```text
mediaState = waiting
backendState = WaitingForAvailability
```

最终 UI 显示：

```text
正在等待数据…
```

而不是：

```text
网络错误
```

---

# 十四、Streaming 播放流程

统一：

```text
Player.open(resourceId)
        ↓
Backend create PlaybackSession
        ↓
Resource validated
        ↓
Gateway capability URL created
        ↓
Frontend receives session
        ↓
video.src = playbackUrl
        ↓
loadedmetadata
        ↓
canplay
        ↓
play()
```

不要允许 frontend 自己拼：

```text
http://127.0.0.1:xxxx/stream?path=...
```

也不要允许：

```typescript
video.src = arbitraryUserPath
```

播放器只能使用 backend 创建的：

```text
PlaybackSession
```

---

# 十五、Gateway URL 必须继续沿用 M4 安全模型

M5 不得创建第二套播放服务器。

必须复用 M4 的：

```text
Local Stream Gateway
```

安全原则：

* loopback only
* random port
* high entropy capability token
* session → resource mapping
* Host validation
* Range validation
* no arbitrary filesystem path
* no arbitrary URL proxy
* no remote web navigation
* session expiration
* session cleanup
* request limits
* concurrency limits
* timeout

前端永远不能指定：

```text
filesystem path
torrent path
arbitrary resource ID
```

来绕过 backend authorization。

---

# 十六、所有播放来源统一 Resource

Player 不应该区分：

```text
HTTP Player
Torrent Player
Local File Player
```

而是：

```text
Resource Player
```

例如：

```text
HTTP Resource
Torrent Resource
Completed File Resource
```

最终都表现为：

```text
Readable Resource
```

播放器只知道：

```text
Resource ID
Content Type
Size
Playback URL
Availability
```

---

# 十七、HTTP 下载中的播放

支持：

```text
正在下载
+
已经下载了一部分
+
播放器开始播放
```

如果 M4 已经完成 progressive HTTP，则播放器直接消费对应 Resource。

禁止重新实现一个：

```text
PlayerHTTPDownloader
```

---

# 十八、Torrent 中的播放

支持：

```text
Torrent
  ↓
selected file
  ↓
Resource
  ↓
Availability
  ↓
Gateway
  ↓
Player
```

对于多文件 Torrent：

```text
Torrent Resource
    ├─ file A
    ├─ file B
    └─ file C
```

播放器必须只获得被用户选择的目标文件。

绝对不能把整个 torrent root 作为媒体来源暴露给 WebView。

---

# 十九、Seek 是 M5 最关键的场景之一

用户拖动：

```text
00:03:00 → 00:47:00
```

Player 应执行：

```text
HTMLMediaElement.currentTime = target
```

不要自己把：

```text
3min
→
byte offset
```

硬编码在 Player。

字节级 Range 推导属于：

```text
Browser
/
Media Stack
/
Gateway
/
Resource
/
Availability
```

链路。

播放器只提供时间维度。

---

# 二十、Seek 过程

完整链路必须可以观察：

```text
User Seek
 ↓
video.currentTime
 ↓
Browser issues media request
 ↓
Gateway receives Range
 ↓
Resource maps request
 ↓
Availability check
 ↓
Scheduler Demand
 ↓
required data becomes available
 ↓
Gateway returns bytes
 ↓
WebView2 resumes playback
```

这一链必须真实测试。

不能用假数据模拟。

---

# 二十一、不可用数据时的行为

例如：

```text
用户跳到尚未下载的位置
```

不要：

```text
立即报错
```

而应该让系统进入：

```text
Seeking
/
Buffering
/
WaitingForAvailability
```

直到：

```text
required bytes available
```

或者：

```text
timeout
/
fatal error
/
task cancelled
/
resource unavailable
```

---

# 二十二、Buffer UI

至少显示：

```text
played
buffered
seekable
loading
```

建议在 timeline 上显示：

```text
━━━━━━━━━━━━━━━━━━━━━━━━
███ played
████████ buffered
━━━━━━━━━━━━ seekable
```

但是：

> `buffered`、`seekable`、Resource Availability 是三个不同概念。

绝对不要把：

```text
downloaded %
```

直接画成：

```text
player buffered %
```

---

# 二十三、下载进度与播放缓冲必须分离

例如：

```text
文件下载：
37%

播放器：
当前位置 12:31
已缓冲至 13:10
```

两套数据必须独立。

尤其 Torrent：

```text
downloadedBytes
availabilityRanges
bufferedTime
currentTime
```

不能混为一个：

```text
progress
```

---

# 二十四、Player Timeline

Timeline 至少支持：

* 点击跳转
* 拖动跳转
* keyboard 操作
* current time
* duration
* buffered visualization
* seeking 状态
* buffering 状态

需要防止：

* 高频 seek 导致请求风暴
* 鼠标移动产生大量 IPC
* React 重渲染拖慢播放

建议：

```text
pointer move
```

只更新临时 UI。

真正：

```text
seek()
```

在：

* pointerup
* Enter
* debounce
* keyboard step

时执行。

---

# 二十五、时间显示

统一：

```text
HH:MM:SS
```

短视频可：

```text
MM:SS
```

不要使用：

```text
NaN
Infinity
undefined
```

duration 不可用时显示：

```text
--
```

不要伪造 duration。

---

# 二十六、播放速度

支持：

```text
0.5
0.75
1
1.25
1.5
2
```

至少保证：

```text
1x
1.25x
1.5x
2x
```

播放速度必须直接映射：

```text
HTMLMediaElement.playbackRate
```

并同步 UI。

---

# 二十七、音量

支持：

```text
0~100%
mute
unmute
```

必须处理：

```text
volumechange
```

并避免：

```text
mute
→ volume = 0
→ unmute
→ volume = 0
```

推荐保持：

```text
lastNonZeroVolume
```

用于恢复。

---

# 二十八、自动播放策略

不要依赖浏览器自动播放策略。

推荐：

```text
用户明确点击播放
```

之后调用：

```text
video.play()
```

不要偷偷：

```text
app startup → autoplay
```

特别是：

```text
有声音的视频
```

。

如果播放被浏览器策略拒绝：

```text
play() rejected
```

必须捕获并映射成可理解的 UI 状态。

不要产生未处理 Promise rejection。

---

# 二十九、暂停

暂停必须真正执行：

```text
video.pause()
```

但是：

```text
Pause Player
```

不等于：

```text
Pause Download Task
```

例如：

```text
Player paused
Download continues
```

是合法状态。

不要把播放暂停错误传给 M1/M2 download state machine。

---

# 三十、停止

Stop 至少意味着：

```text
pause
+
reset position
+
release media source
+
dispose/release playback session
```

具体 Resource 生命周期必须服从 M4 设计。

不要删除：

```text
下载文件
.part 文件
Torrent data
```

除非用户明确执行 Delete Task。

---

# 三十一、切换媒体

例如：

```text
A.mp4
→
B.mp4
```

必须：

```text
dispose A session
create B session
set new source
load
play only when requested
```

避免：

```text
A Gateway session
```

泄漏。

验证：

```text
旧 token 失效
旧 session 被释放
旧 Resource 不再被 player 引用
新 session 正常工作
```

---

# 三十二、Player Error Model

至少分类：

```text
UnsupportedMedia
DecodeError
NetworkError
ResourceUnavailable
PlaybackTimeout
SessionExpired
GatewayError
ResourceClosed
UnknownMediaError
```

前端显示的是用户可理解的信息。

例如：

```text
此媒体格式当前环境无法播放
```

而不是：

```text
MEDIA_ERR_DECODE: 3
```

详细错误可写 diagnostic log。

---

# 三十三、媒体错误必须保留诊断上下文

错误对象可包含：

```text
error_code
media_error_code
resource_id
session_id
content_type
player_state
timestamp
recoverable
```

不要记录：

* 完整磁力链接
* 完整用户 URL
* peer IP
* auth token
* 完整本机敏感路径

---

# 三十四、失败恢复

可以自动恢复的情况：

```text
temporary buffering
temporary gateway timeout
temporary network interruption
```

尝试：

```text
wait
retry playback request
reload source
```

但必须有：

```text
bounded retry
```

不能无限：

```text
retry forever
```

不可恢复：

```text
unsupported codec
corrupt media
invalid session
resource deleted
```

应明确显示错误。

---

# 三十五、重新加载策略

例如：

```text
video error
```

不要无条件：

```text
video.load()
video.play()
video.load()
video.play()
...
```

必须有：

```text
maximum retry
backoff
state guard
```

避免播放器进入重试死循环。

---

# 三十六、Playback Position

M5 可以维护：

```text
current playback position
```

但不要提前实现完整：

```text
Library history
Watch history
cross-device sync
cloud resume
account sync
```

本阶段最多做：

```text
PlaybackSession.last_position
```

用于当前 session / 明确必要的轻量恢复。

完整观看历史归入 M6 Library。

---

# 三十七、窗口刷新

Frontend reload 后：

```text
backend PlaybackSession
```

不能突然丢失。

至少应该能够：

```text
frontend reconnect
→
query active playback session
→
restore player projection
```

如果 M4 已有 session registry，应复用。

不要让：

```text
React state
```

成为唯一真相源。

---

# 三十八、Backend Source of Truth

原则：

```text
Backend
    ↓
PlaybackSession state
    ↓
Frontend projection
```

而不是：

```text
React state
    ↓
guess backend state
```

---

# 三十九、IPC

根据实际架构实现最小 IPC。

可能需要：

```text
create_playback_session
get_playback_session
close_playback_session
get_playback_status
```

如果 M4 已经提供对应 command，则优先复用。

Player 控制命令可以考虑：

```text
playback_open
playback_close
playback_get_session
```

但不要把：

```text
play
pause
seek
volume
```

全部做成 Rust IPC。

这些属于 HTMLMediaElement 本地操作。

只有真正需要 backend 的操作才进入 IPC。

---

# 四十、事件模型

建议：

```text
playback.session_created
playback.state_changed
playback.error
playback.resource_waiting
playback.resource_ready
playback.session_closed
```

Frontend 自己从：

```text
HTMLMediaElement events
```

获取：

```text
timeupdate
waiting
playing
seeking
seeked
ended
volumechange
```

不要把浏览器每一个 `timeupdate` 都通过 Rust IPC 再转一遍。

---

# 四十一、事件频率

绝对禁止：

```text
每一帧
→ Rust IPC
→ React state
→ render
```

也不要：

```text
every 1ms
```

同步 currentTime。

推荐：

```text
UI progress update ≈ 100~250ms
```

实际以性能测试调整。

播放 timeline 拖动期间可使用：

```text
requestAnimationFrame
```

处理纯 frontend visual preview。

---

# 四十二、Player UI

建立真正的 Player 页面 / View。

建议结构：

```text
PlayerPage

 ├─ VideoSurface
 │
 ├─ PlayerOverlay
 │
 ├─ PlayerControls
 │   ├─ PlayPause
 │   ├─ CurrentTime
 │   ├─ Timeline
 │   ├─ Duration
 │   ├─ Volume
 │   ├─ PlaybackRate
 │   └─ Fullscreen
 │
 ├─ BufferIndicator
 │
 ├─ LoadingState
 │
 ├─ ErrorState
 │
 └─ ResourceInfo
```

---

# 四十三、视觉要求

M5 不追求最终品牌级视觉，但必须已经像一个真正播放器。

至少做到：

* dark media surface
* 大面积视频区域
* controls overlay
* controls auto-hide
* 清楚的播放状态
* 清楚的 buffering 状态
* 清楚的错误状态
* timeline 易操作
* mouse hover
* keyboard 操作
* window resize 不破坏布局

禁止：

* 大量花哨动画
* 复杂玻璃拟态
* 3D 特效
* 音频可视化
* AI 功能
* 推荐系统

这些都不是 M5 核心。

---

# 四十四、Video Surface

`<video>` 必须做到：

```text
width: 100%
height: 100%
object-fit: contain
```

默认避免裁剪。

用户应能完整看到视频。

可提供：

```text
contain
```

作为默认播放模式。

---

# 四十五、controls 自动隐藏

正常播放时：

```text
mouse idle
→ controls fade
```

鼠标移动：

```text
controls appear
```

暂停时：

```text
controls remain visible
```

实现必须避免频繁 timer 创建。

正确处理：

```text
unmount
source switch
page destroy
```

时的 timer cleanup。

---

# 四十六、键盘快捷键

至少支持：

```text
Space      play/pause
←          -5s
→          +5s
↑          volume +
↓          volume -
M          mute
F          fullscreen
Esc        exit fullscreen
```

必须避免与：

* input
* textarea
* select
* slider

产生冲突。

---

# 四十七、Fullscreen

优先使用当前平台已经存在的：

```text
Tauri window / WebView fullscreen capability
```

根据实际 M8 架构决定是否全部交给 M8。

但 M5 至少必须验证：

```text
进入全屏
退出全屏
resize
ESC
source switch
```

不能做出一个 UI 按钮，但是实际没有效果。

---

# 四十八、不要提前加入 Windows Media Integration

M5 不实现：

* global media keys
* Media Session native bridge
* taskbar thumbnail buttons
* taskbar progress
* system tray
* startup
* deep links
* file associations

这些属于 M8。

如果浏览器层面的 Media Session API 不需要额外 native integration，可以预留接口，但不要扩大范围。

---

# 四十九、字幕原则

M5 核心播放器暂不做：

* 自动字幕匹配
* 在线字幕搜索
* 字幕下载
* 字幕站点
* OCR
* AI 字幕
* 自动翻译

如 M0.5/M4 已经证明实现成本极低，可以考虑：

```text
用户明确指定的本地 WebVTT
```

但只能作为：

> 可选增强项。

不能以字幕功能阻塞 M5 主线。

不要自行加入复杂 subtitle engine。

---

# 五十、缩略图 / Poster

支持：

```text
poster
```

即可。

不要实现：

* FFmpeg thumbnail extraction
* 视频扫描
* 自动关键帧分析

如果 Resource metadata 已存在 poster，就使用。

不存在则：

```text
neutral placeholder
```

不要伪造图片。

---

# 五十一、Content-Type

Player 必须优先使用：

```text
Resource.content_type
```

而不是：

```text
filename extension
```

但 MIME 不可信时仍需依赖实际媒体解码结果。

不要因为：

```text
foo.mp4
```

就强制：

```text
video/mp4
```

破坏正确 metadata。

---

# 五十二、安全：禁止任意 media URL

Player API 不应该允许：

```typescript
play("C:\\secret\\password.mp4")
play("file:///...")
play("http://arbitrary-host/...")
```

默认模型必须是：

```text
Resource ID
→ Backend authorize
→ PlaybackSession
→ Capability URL
```

---

# 五十三、安全：禁止通过 Player 绕过文件权限

测试：

```text
session A
```

不能读取：

```text
resource B
```

不能访问：

```text
arbitrary path
```

不能通过：

```text
../../
```

绕过 Gateway。

不能通过改变：

```text
resource_id
file_id
token
Range
```

访问未授权资源。

---

# 五十四、安全：Tauri Capability

M5 不得为了播放器方便而添加：

```text
fs:default
shell:default
process:default
broad filesystem permissions
```

之类的广泛权限。

继续保持最小权限。

Tauri 官方当前能力模型就是通过 capability 对窗口/WebView 的权限做细粒度控制，因此 Player 不应为了本地媒体访问而扩大整个主窗口的权限边界。

---

# 五十五、安全：CSP

继续保持严格 CSP。

Player 如果需要：

```text
http://127.0.0.1:<dynamic-port>
```

必须根据 M4 Gateway 的真实实现调整：

```text
connect-src
media-src
```

相关策略。

不要简单：

```text
default-src *
```

不要：

```text
unsafe-eval
```

作为逃避配置问题的手段。

Tauri 官方文档明确建议 CSP 尽可能只允许可信来源，并警告远程/不受信任内容会扩大攻击面。

---

# 五十六、网络错误与播放错误必须区分

例如：

```text
Resource not available
```

不是：

```text
Unsupported Codec
```

也不是：

```text
Corrupt file
```

必须建立：

```text
Resource layer error
Playback layer error
Media decoder error
Gateway error
```

之间的映射。

---

# 五十七、缓冲 UX

至少显示：

```text
Loading…
Buffering…
Waiting for download…
Playing
Paused
Ended
Playback error
```

Torrent progressive playback 时尤其要避免用户误以为：

```text
程序卡死
```

例如：

```text
正在等待网络数据…
```

或：

```text
正在下载播放所需的数据…
```

都应该来自真实 backend state。

禁止假装：

```text
buffer 78%
```

除非确有真实数据。

---

# 五十八、Playback telemetry

不实现：

```text
analytics
```

不上传：

```text
watch history
media filename
URL
torrent info hash
peer IP
```

本阶段仅保留本地 diagnostic metrics，例如：

```text
startup_ms
time_to_metadata
time_to_first_frame
buffer_count
stall_duration
seek_count
seek_wait_duration
playback_error_count
```

这些只进入本地开发/diagnostic 层。

默认不发送到服务器。

---

# 五十九、性能目标

至少观察：

```text
CPU idle
CPU playback
memory
frontend render frequency
IPC frequency
gateway overhead
startup to first frame
```

目标：

> 播放器应该尽量让 WebView2/系统媒体栈做真正的媒体工作，而不是让 React / Rust 持续搬运媒体数据。

绝对禁止：

```text
video bytes
→ Rust
→ IPC
→ JS
→ Blob
→ video
```

作为默认媒体路径。

必须优先：

```text
video
→ local HTTP Gateway
```

---

# 六十、不要把整个文件 Blob 化

禁止默认：

```typescript
const blob = await readEntireFile(...)
video.src = URL.createObjectURL(blob)
```

特别是大文件。

这样会：

* 增加内存
* 破坏 Range
* 破坏渐进播放
* 破坏 Torrent seek
* 失去统一 Gateway

正确：

```text
video.src = playbackUrl
```

---

# 六十一、不要自己实现 MSE，除非必要

默认不要为了“高级播放器”加入：

```text
MediaSource
SourceBuffer
custom segmenter
custom transmuxer
```

M4 已经有：

```text
HTTP Range Gateway
```

优先让 WebView2 原生 media pipeline 完成播放。

只有 M0.5 明确证明必须使用 MSE 时，才允许引入。

---

# 六十二、测试媒体集

建立真实测试资源：

```text
fixtures/media/
```

至少包含：

```text
small_h264_aac.mp4
small_vp9_opus.webm
small_av1_opus.webm    // 若环境支持
audio_sample
long_sample
```

并记录：

```text
container
video codec
audio codec
resolution
duration
size
expected support
```

测试资源应该：

* public domain
* open licensed
* self-generated
* synthetic

不要下载来源不明的版权内容。

---

# 六十三、播放器测试矩阵

至少验证：

| 场景                      | 必须验证                       |
| ----------------------- | -------------------------- |
| MP4/H264/AAC            | play/pause/seek            |
| WebM/VP9/Opus           | play/pause/seek            |
| unsupported codec       | error UI                   |
| audio                   | play/pause/seek            |
| zero/unknown duration   | graceful                   |
| corrupt media           | error                      |
| progressive HTTP        | play before completion     |
| progressive Torrent     | play before completion     |
| seek unavailable region | buffering                  |
| slow resource           | buffering                  |
| network interruption    | recover/error              |
| resource closed         | graceful error             |
| session expired         | graceful error             |
| source switch           | old session released       |
| app UI reload           | session recover/projection |
| fullscreen              | enter/exit                 |
| keyboard                | all required keys          |
| volume                  | mute/unmute                |
| playback rate           | all rates                  |

---

# 六十四、关键集成测试

至少实现：

## Test 1

```text
completed local resource
→ PlaybackSession
→ Gateway
→ WebView2
→ play
```

## Test 2

```text
HTTP downloading resource
→ partial availability
→ player
→ play
```

## Test 3

```text
Torrent resource
→ partial availability
→ player
→ play
```

## Test 4

```text
Torrent resource
→ seek into unavailable region
→ Demand
→ pieces arrive
→ resume playback
```

## Test 5

```text
player pause
→ download continues
```

## Test 6

```text
player stop
→ resource remains intact
```

## Test 7

```text
source switch
→ old session released
→ new session works
```

## Test 8

```text
gateway token invalid
→ playback rejected
```

## Test 9

```text
frontend reload
→ active session still exists
→ frontend can query state
```

## Test 10

```text
resource deleted / cancelled
→ player receives error
→ no crash
```

---

# 六十五、Critical Seek Test

这是 M5 的核心验收测试之一。

构造：

```text
media = 500 MB
downloaded = first 20 MB
```

播放器：

```text
start
```

然后：

```text
seek → 15 MB
```

应当：

```text
playback starts
```

然后：

```text
seek → 350 MB
```

如果：

```text
350 MB unavailable
```

必须：

```text
Seeking
→ Buffering
→ Scheduler Demand
→ availability increases
→ playback resumes
```

不能：

```text
crash
```

不能：

```text
fake playback
```

不能：

```text
return random bytes
```

不能：

```text
immediately mark completed
```

---

# 六十六、Critical Range Test

模拟 WebView2：

```text
Range: bytes=350000000-
```

验证：

```text
Gateway
→ Resource
→ Availability
```

实际只返回合法数据。

验证：

```text
206
Content-Range
Content-Length
Accept-Ranges
Content-Type
```

全部正确。

同时测试：

```text
200
206
416
```

场景。

---

# 六十七、Critical Multi-file Torrent Test

构造：

```text
torrent
 ├─ video.mp4
 ├─ subtitle.txt
 └─ image.jpg
```

播放：

```text
video.mp4
```

验证：

```text
Gateway cannot access subtitle.txt
Gateway cannot access image.jpg
```

除非 backend 创建了对应 PlaybackSession。

---

# 六十八、Memory Test

播放：

```text
1 GB+
```

甚至更大的测试文件。

监控：

```text
RSS
WebView memory
Rust memory
JS heap
```

确认没有：

```text
entire file buffered into JS
```

---

# 六十九、UI Reload Test

运行：

```text
Torrent progressive playback
```

播放中刷新 frontend。

然后：

```text
frontend reconnect
```

必须：

```text
backend session remains coherent
```

不允许：

```text
gateway zombie
session duplicate
second player session leak
```

---

# 七十、Session Cleanup Test

重点测试：

```text
play
pause
stop
switch source
close page
close window
frontend crash
backend error
gateway error
```

最终都应释放：

```text
session
timer
event listener
gateway capability
resource lease
```

不能出现：

```text
zombie session
zombie timer
zombie gateway token
zombie event listener
```

---

# 七十一、Frontend Architecture

不要把 Player 写成：

```text
OneHugePlayer.tsx
```

推荐分层：

```text
features/player/
    PlayerPage
    PlayerShell
    VideoSurface
    PlayerControls
    Timeline
    VolumeControl
    PlaybackRateControl
    PlayerStatus
    PlayerError
    hooks/
    state/
    controller/
    types/
```

具体结构根据现有 frontend architecture 调整。

核心目标：

> UI component 不掌握底层 Resource 逻辑。

---

# 七十二、状态管理

不要制造一个巨大的全局：

```text
AppStore
```

把播放器状态集中在：

```text
PlayerState
```

大致：

```text
session
media
controls
timeline
error
```

例如：

```text
session:
  id
  resourceId
  state

media:
  currentTime
  duration
  buffered
  seekable

controls:
  playing
  volume
  muted
  playbackRate

error:
  code
  message
```

实际可按照现有项目状态管理方案实现。

---

# 七十三、避免 React 高频更新

禁止：

```text
timeupdate
→ global store
→ whole App rerender
```

播放器应局部更新。

必要时将：

```text
currentTime
buffered
```

放在 player-local state / refs。

---

# 七十四、真实数据原则

播放器 UI 所有关键数据显示：

```text
real
```

不得：

```text
fake progress
fake buffering
fake download
fake duration
fake speed
fake availability
```

特别是：

```text
Buffering 63%
```

如果没有真实依据，不允许显示。

---

# 七十五、Player / Download 完全解耦

验证：

```text
Pause Player
```

不会：

```text
pause Download Task
```

验证：

```text
Download Task Pause
```

会真实导致：

```text
Player eventually waiting
```

而不是：

```text
Player magically continues downloading
```

---

# 七十六、Player / Scheduler 解耦

Player 可以通过：

```text
PlaybackSession
```

产生：

```text
Demand
```

但 Player 不应知道：

```text
priority algorithm
torrent piece priority
queue
download weight
```

Scheduler 负责：

```text
what to download
```

Player 负责：

```text
what the user is watching
```

---

# 七十七、Demand 来源

理想模型：

```text
PlaybackSession
    ↓
current playback need
    ↓
Demand
    ↓
Scheduler
```

例如：

```text
currentTime
+
lookahead window
+
seek target
```

最终形成 backend demand。

但是：

> 如果 M4 已经完成 Demand 模型，M5 只负责接入，不重新设计 Scheduler。

---

# 七十八、播放状态与 Availability

不要直接：

```text
availability > 0 → Playing
```

真正播放器能否播放由：

```text
WebView2 media pipeline
```

决定。

后台 Availability 主要帮助解释：

```text
为什么 waiting
```

而不是取代浏览器媒体状态。

---

# 七十九、Media Capability Discovery

可以增加轻量：

```text
canPlayType()
```

用于 UI 提示。

例如：

```javascript
video.canPlayType("video/mp4; codecs=\"avc1...\"")
```

但它只是前置提示。

不能把：

```text
canPlayType()
```

当成真正播放成功保证。

最终结果必须通过真实播放测试。

---

# 八十、播放器首次播放体验

目标：

```text
Open Resource
→ Loading
→ metadata
→ first frame
→ Playing
```

记录：

```text
time_to_metadata
time_to_first_frame
```

如果需要：

```text
buffering
```

应该明确显示。

---

# 八十一、Time-to-First-Frame

重点优化：

```text
TTFF
```

不要：

```text
open player
→ wait entire file
→ play
```

对于 progressive resource：

```text
只等待实际需要的数据
```

即可。

---

# 八十二、Player Metrics

本地记录：

```text
open_started
metadata_loaded
first_frame
play_started
buffering_started
buffering_ended
seek_started
seek_ended
playback_ended
playback_error
```

可以计算：

```text
startup latency
seek latency
stall duration
```

开发模式中可以展示 diagnostic panel。

生产版本默认不要暴露复杂诊断信息。

---

# 八十三、Diagnostic Panel

允许开发模式查看：

```text
session_id
resource_id
player_state
media_state
duration
current_time
buffered_ranges
seekable_ranges
backend_availability
gateway state
scheduler demand state
```

但不要显示：

```text
auth token
full magnet URI
peer IP
sensitive absolute path
```

---

# 八十四、测试策略

至少建立：

## Unit

测试：

```text
PlaybackState
PlaybackController
time formatting
buffer range mapping
state transition
error mapping
retry policy
session lifecycle
```

## Frontend

测试：

```text
play
pause
seek
volume
mute
rate
keyboard
error
source switch
```

## Integration

测试：

```text
Gateway → HTMLMediaElement
Resource → Player
Availability → Buffering
Seek → Demand
Torrent → Player
HTTP → Player
```

## E2E / manual

真实 WebView2：

```text
launch
open
play
pause
seek
buffer
switch
fullscreen
error
close
```

---

# 八十五、Property-style Testing

如果现有测试体系允许，考虑：

```text
timeline ranges
state transitions
session lifecycle
buffer range normalization
```

例如验证：

```text
start <= end
```

以及：

```text
merged ranges remain normalized
```

防止：

```text
buffered[0].end < buffered[0].start
```

之类异常。

---

# 八十六、并发测试

特别测试：

```text
play()
pause()
play()
seek()
seek()
stop()
```

快速连续操作。

测试：

```text
source switch while buffering
source switch while seeking
close while request pending
stop while play promise pending
```

目标：

> no race panic / no unhandled rejection / no leaked session.

---

# 八十七、Cancellation / Disposal

所有异步操作必须能够：

```text
cancel
```

并且：

```text
cleanup guaranteed
```

例如：

```text
AbortController
cleanup()
finally
```

Rust：

```text
CancellationToken
Drop
RAII
```

遵循项目现有模式。

---

# 八十八、错误不能让应用崩溃

以下情况均不应 crash：

```text
invalid resource
expired session
missing resource
gateway unavailable
range invalid
media unsupported
decoder error
network disconnect
Torrent stopped
download cancelled
frontend destroyed
```

正常业务错误必须进入：

```text
typed error
```

而不是：

```text
panic
unwrap()
expect()
```

---

# 八十九、日志

日志只记录必要 diagnostic information。

允许：

```text
session_id
resource_id
error_code
state
duration
latency
```

禁止 production log：

```text
full URL
magnet URI
token
peer IP
full local path
user history
```

可以在 development log 中使用经过脱敏的上下文。

---

# 九十、禁止 scope creep

M5 禁止开始：

### Torrent

* DHT UI
* Tracker UI
* Peer UI
* Piece map UI

### Download

* advanced downloader
* bandwidth scheduler redesign
* cloud acceleration
* multi-source downloader

### Library

* watched history
* smart folders
* tags
* rating
* collections
* artwork scraping

### Network

* proxy
* VPN
* Tor
* anonymous download
* remote relay

### Media

* transcoding
* codec conversion
* FFmpeg pipeline
* AI subtitle
* online subtitles
* online metadata
* automatic media discovery

### Platform

* tray
* startup
* global hotkey
* Media Keys
* taskbar integration

这些留给后续 milestone。

---

# 九十一、禁止重新设计 M4

特别重要。

如果 M4 已经存在：

```text
Resource
Availability
Demand
Scheduler
StreamGateway
PlaybackSession
```

M5 不要重写。

首先：

```text
reuse
```

其次：

```text
adapt
```

最后才：

```text
refactor
```

只有发现真实 P0/P1 问题时才允许修改 M4。

所有修改必须记录：

```text
Why
Evidence
Impact
Tests
```

---

# 九十二、发现 M4 问题时的处理

如果 Player 实施过程中发现：

```text
Gateway cannot correctly satisfy Range
```

不要在 Player 中打补丁。

应该：

```text
M4 Layer fix
```

然后继续 Player。

同理：

```text
Availability bug
Scheduler bug
Resource bug
```

都应该在对应层解决。

---

# 九十三、依赖纪律

原则：

> 尽可能使用现有依赖。

不要因为播放器加入：

```text
10 个 UI library
```

不要随便升级：

```text
Tauri
Rust
React
WebView2
```

如果必须新增依赖：

必须说明：

```text
Package
Version
Purpose
License
Security impact
Why existing dependencies insufficient
```

---

# 九十四、许可证

任何新增第三方依赖必须检查：

```text
license
transitive dependencies
```

并更新：

```text
THIRD_PARTY_NOTICES.md
```

禁止偷偷加入 GPL/LGPL/未知许可证媒体库。

特别是：

```text
FFmpeg
libmpv
VLC
GStreamer
```

目前全部不默认加入。

---

# 九十五、文档

至少新增 / 更新：

```text
docs/M5-player.md
docs/M5-playback-session.md
docs/M5-media-compatibility.md
docs/M5-error-model.md
docs/M5-test-matrix.md
docs/M5-report.md
```

如果已有同类文档，优先扩展，不重复建立多个冲突文档。

---

# 九十六、M5 Player Architecture 文档必须回答

明确写清：

```text
Player
↓
PlaybackController
↓
PlaybackSession
↓
Gateway
↓
Resource
↓
Availability
↓
Scheduler
```

以及：

```text
HTMLMediaElement
```

和：

```text
Rust backend
```

各自负责什么。

---

# 九十七、一定要画生命周期

至少记录：

```text
OPEN
 ↓
CREATE SESSION
 ↓
CREATE CAPABILITY URL
 ↓
LOAD VIDEO
 ↓
METADATA
 ↓
PLAY
 ↓
BUFFER / SEEK
 ↓
PLAYING
 ↓
END
 ↓
CLOSE SESSION
```

异常：

```text
OPEN
 ↓
ERROR
 ↓
CLOSE
```

以及：

```text
PLAY
 ↓
WAITING
 ↓
AVAILABILITY
 ↓
PLAYING
```

---

# 九十八、M5 的核心 Definition of Done

M5 只有同时满足下面条件才算完成。

## Player

* [ ] 可以打开真实 Resource
* [ ] 可以播放真实媒体
* [ ] 可以暂停
* [ ] 可以 seek
* [ ] 可以调整音量
* [ ] 可以 mute
* [ ] 可以调整播放速度
* [ ] 可以结束
* [ ] 可以切换资源
* [ ] 可以 fullscreen
* [ ] 键盘控制正常

## Progressive

* [ ] HTTP progressive playback 可用
* [ ] Torrent progressive playback 可用
* [ ] 尚未下载的 seek 会等待 Availability
* [ ] 数据到达后可以继续播放
* [ ] 不需要等待完整文件

## Gateway

* [ ] 使用 M4 Gateway
* [ ] 不存在第二套媒体服务器
* [ ] token/session 安全
* [ ] 无任意路径访问
* [ ] 无越权 Resource 访问
* [ ] session 能 cleanup

## Architecture

* [ ] Player 不访问 SQLite
* [ ] Player 不直接访问文件系统
* [ ] Player 不调用 librqbit
* [ ] Player 不实现 HTTP downloader
* [ ] Scheduler 不被 Player UI 污染
* [ ] Resource abstraction 保持统一

## UX

* [ ] Loading
* [ ] Buffering
* [ ] Waiting for data
* [ ] Error
* [ ] Ended
* [ ] Playing
* [ ] Paused

全部清晰可见。

## Performance

* [ ] 不将整个媒体文件读取进 JS
* [ ] 不通过 IPC 搬运媒体 bytes
* [ ] React 没有高频全局重渲染
* [ ] 播放过程中 CPU / RAM 无异常增长

## Reliability

* [ ] rapid play/pause 不 crash
* [ ] rapid seek 不 crash
* [ ] source switch 不泄漏
* [ ] frontend reload 可以恢复 session projection
* [ ] gateway error 可以处理
* [ ] resource error 可以处理
* [ ] unsupported media 可以处理

## Security

* [ ] Capability 最小权限
* [ ] CSP 没有被粗暴放宽
* [ ] 不存在 arbitrary file playback
* [ ] 不存在 arbitrary URL proxy
* [ ] 不记录敏感信息

## Testing

* [ ] Unit tests
* [ ] Frontend tests
* [ ] Integration tests
* [ ] Real WebView2 playback validation
* [ ] HTTP progressive test
* [ ] Torrent progressive test
* [ ] Seek unavailable test
* [ ] Gateway security test
* [ ] cleanup test

---

# 九十九、M5 最终验收链

最终必须亲自验证：

```text
HTTP Resource
    ↓
PlaybackSession
    ↓
Local Stream Gateway
    ↓
WebView2
    ↓
HTMLMediaElement
    ↓
load metadata
    ↓
play
    ↓
buffer
    ↓
seek
    ↓
Range request
    ↓
Resource Availability
    ↓
Demand
    ↓
Scheduler
    ↓
data becomes available
    ↓
playback continues
```

然后：

```text
Torrent Resource
    ↓
PlaybackSession
    ↓
Gateway
    ↓
WebView2
    ↓
play
    ↓
seek
    ↓
unavailable range
    ↓
buffer
    ↓
Torrent pieces arrive
    ↓
playback resumes
```

这是 M5 最核心的产品证明。

---

# 一百、M5 完成后的最终报告

实施结束后不要只说：

```text
M5 complete.
```

必须输出：

## 1. Executive Summary

## 2. FACTS

## 3. HYPOTHESES

## 4. CURRENT M0～M4 STATE

## 5. M5 Architecture

## 6. PlaybackSession Design

## 7. Player State Machine

## 8. WebView2 Media Integration

## 9. Gateway Integration

## 10. Resource Integration

## 11. Availability / Demand Integration

## 12. UI

## 13. Error Model

## 14. Security

## 15. Performance

## 16. Test Matrix

## 17. Real WebView2 Results

## 18. Progressive HTTP Results

## 19. Progressive Torrent Results

## 20. Seek-into-Unavailable Results

## 21. Session Cleanup Results

## 22. Files Changed

## 23. Dependencies Added

## 24. Documentation Updated

## 25. RESOLVED

## 26. CONFIRMED

## 27. OPEN

## 28. WORSENED

## 29. P0

## 30. P1

## 31. M6 Readiness

## 32. Definition of Done Checklist

---

# 一百零一、M5 的成功标准

M5 不是：

> “做出了一个好看的视频播放器。”

而是：

> **Tide 第一次真正把“下载”和“播放”连接起来。**

最终结构应稳定成为：

```text
             ┌──────── HTTP Download ────────┐
             │                                │
Source → Resource → Availability → Scheduler │
             │                                │
             └──────── Torrent ───────────────┘
                              ↓
                       PlaybackSession
                              ↓
                         Stream Gateway
                              ↓
                           WebView2
                              ↓
                            Player
```

也就是说：

> **M5 不创造新的媒体数据通路，只把已有数据通路变成用户真正可以使用的播放体验。**

---

# 一百零二、实施纪律

整个 M5 实施期间严格遵守：

1. 先审计，后修改。
2. 不凭空假设 M4 API。
3. 不重复实现 Resource。
4. 不重复实现 Gateway。
5. 不把媒体 bytes 经过 IPC 传给前端。
6. 不为了 Player 扩大 Tauri permissions。
7. 不为了 codec 支持直接引入 FFmpeg/libmpv。
8. 不使用 fake buffering/progress。
9. 不把 Player state 塞进 DownloadTask state。
10. 不因为发现一个非阻塞问题而顺手重构整个项目。
11. 所有跨层修改必须说明原因。
12. 所有真实错误必须修复或明确记录。
13. 测试不通过时不要宣称完成。
14. 如果 M4 设计本身存在 P0/P1 问题，修复根因而不是在 Player 中绕过去。
15. M5 完成后停止，不继续实现 M6 Library / M7 Scheduler optimization / M8 Windows Integration。

最终必须同时保证：

```text
architecture correct
+
playback works
+
progressive works
+
seek works
+
errors handled
+
security preserved
+
tests pass
```

然后停止在 M5。
