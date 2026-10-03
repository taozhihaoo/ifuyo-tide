# ifuyo Tide — M6 Library / 本地资源库（上）

> **拆分说明（必读）**：本文件是 M6 完整开发实施提示词的**上半部分（第一~九十一节：领域模型与实现 —— LibraryItem、Canonical Path 与路径安全、Scanner、数据库与 Repository、查询/过滤/分页、Playback 链路、UI 与前端状态、与 M7/M8 的边界）**。测试体系、前端实际验收、跨阶段协同、工程纪律与最终验收位于下半部分《ifuyo Tide — M6 Library  本地资源库（下）.md》。两部分必须配合使用：先完成上半部分，再执行下半部分；只完成上半部分不构成 M6 完成。

## 完整、详细、专业的开发实施提示词

你现在负责实现 **ifuyo Tide M6：Library（本地资源库）**。

Tide 的产品定位已经确定：

> **Tide 是“用户自己提供来源的本地下载器 + 渐进式媒体播放器”。**
>
> 它不是资源发现平台、盗版资源平台、媒体内容搜索引擎，也不是云盘、NAS 管理器或 Plex/Jellyfin 类媒体中心。

M6 的目标，是在已经存在的：

* M0 Foundation
* M0.5 Technical Spike
* M1 HTTP/HTTPS Download Core
* M2 Task Persistence / Crash Recovery
* M3 Torrent Engine
* M4 Progressive Streaming / Resource / Availability / Scheduler foundation
* M5 Player

之上，建立一个真正可用的 **本地 Library 资源索引层**。

---

# 一、最高优先级：先审计，后实现

**绝对不要直接开始写 Library。**

第一步必须完整检查当前仓库、当前分支和现有代码，尤其确认 M0～M5 当前真实落地情况。

检查至少包括：

* 当前 Git branch
* 工作区是否有未提交修改
* Cargo.toml / Cargo.lock
* package.json / lockfile
* Tauri configuration
* capabilities
* CSP
* SQLite migrations
* repository 层
* Task domain
* Source domain
* Resource domain
* Availability
* Scheduler
* Download Engine
* HTTP Transport
* Torrent/rqbit 集成
* PlaybackSession
* Stream Gateway
* Player
* IPC commands
* frontend state management
* existing Library placeholder
* existing Settings
* existing download directory configuration
* existing error system
* existing logging / tracing
* existing tests
* existing documentation

同时确认：

1. M1 的下载任务是否已经能够产生完整文件。
2. M2 的任务恢复是否可靠。
3. M3 的 Torrent 文件是否能够形成稳定的本地文件。
4. M4 的 Resource / Availability / Range / Stream Gateway 是否已经存在。
5. M5 是否已经形成正式 PlaybackSession。
6. 当前播放器播放本地资源究竟走什么链路。
7. 当前数据库 schema 与 migration 真实状态。
8. 当前 AppState / Repository / IPC 架构真实状态。

---

# 二、首先输出审计结果

在任何代码修改之前，先输出：

```text
FACTS
HYPOTHESES
CURRENT M0 STATE
CURRENT M1 STATE
CURRENT M2 STATE
CURRENT M3 STATE
CURRENT M4 STATE
CURRENT M5 STATE
EXISTING LIBRARY STATE
ARCHITECTURAL GAPS
M6 IMPLEMENTATION PLAN
SECURITY RISKS
P0
P1
```

其中：

* `FACTS`：已经从代码、配置、测试、数据库 migration 中确认的事实。
* `HYPOTHESES`：尚未验证的推断。
* `P0`：必须阻止继续发布/实现的问题。
* `P1`：当前 M6 必须处理，否则会造成架构或数据一致性问题的问题。

**不要把推测写成事实。**

如果 M0～M5 实际实现与此前设计不同，以当前代码为准。

但如果当前实现与既定架构存在明显冲突，不要默默接受，要报告：

```text
ARCHITECTURAL CONFLICT
WHY IT MATTERS
MINIMAL CORRECTION
```

然后采用最小必要修改。

---

# 三、M6 的核心目标

M6 不负责重新发明 Download。

M6 的核心职责是：

```text
Local Filesystem
        ↓
Library Scanner
        ↓
File Validation
        ↓
Library Item
        ↓
SQLite Persistence
        ↓
Search / Filter / Sort
        ↓
PlaybackSession
        ↓
M4 Stream Gateway
        ↓
M5 Player
```

同时建立下载任务到 Library 的正式关系：

```text
HTTP Download / Torrent Download
                ↓
          Completed File
                ↓
          Library Item
```

以及现有本地文件进入 Library：

```text
User-selected local folder
          ↓
       Scanner
          ↓
       Library Item
```

因此 M6 最终应该建立一个清晰的边界：

```text
Task       = “一次下载/操作任务”
Resource   = “可读取的字节资源”
LibraryItem= “用户希望长期管理/访问的本地资源”
Playback   = “一次播放会话”
```

**不要把这四个概念合并。**

---

# 四、M6 明确不做什么

以下功能禁止在 M6 中偷偷加入：

## 1. 不做资源发现

禁止：

* 网页资源搜索
* Torrent 搜索
* Tracker 推荐
* Magnet 推荐
* 第三方资源站导航
* 内容排行榜
* 推荐算法
* “热门资源”
* “猜你喜欢”

Tide 永远保持：

```text
用户提供 Source
```

而不是：

```text
Tide 帮用户寻找 Source
```

---

## 2. 不做云端 Library

禁止：

* 云同步
* 云数据库
* 用户账号
* ifuyo 服务器
* 云端媒体索引
* 用户内容上传
* 远程控制服务

M6 的 Library 必须是：

> **Local-first / Offline-first**

---

## 3. 不做 NAS / SMB / UPnP

本阶段只处理本机文件。

暂不加入：

* SMB
* NFS
* UPnP
* DLNA
* WebDAV
* FTP
* NAS 自动发现
* mDNS 网络扫描

即使未来支持，也不能在 M6 混进来。

---

## 4. 不做完整媒体中心

禁止把 Tide 变成：

* Plex
* Jellyfin
* Kodi
* Emby

尤其不要在 M6 加入：

* 在线 TMDB/IMDb 元数据抓取
* 海报数据库
* 演员数据库
* 影视剧自动识别
* 网络字幕下载
* 在线封面下载
* 在线观看源
* 内容推荐

---

## 5. 不做大规模 Media Probe

M1 已经明确：

> 不把 FFmpeg / libmpv / VLC / GStreamer 等引入到下载核心。

M6 也不要为了 Library 列表而突然引入重量级媒体分析链。

第一版 Library 只需要可靠识别：

* file extension
* MIME / media category
* size
* modified time
* path
* availability
* origin
* task association

例如：

```text
Video
Audio
Image
Subtitle
Archive
Document
Other
```

具体能力以当前代码和 M5 实际播放器支持矩阵为准。

---

# 五、Library 的定位

M6 的 Library 不是“另一个任务列表”。

它回答的是：

> **“我电脑上现在有哪些 Tide 可以管理和播放的本地资源？”**

而 Download 页面回答的是：

> **“我现在有哪些下载任务？”**

因此 UI 和 Domain 必须明确分开。

例如：

```text
Downloads
------------------------------------------------
Task A    Downloading    1.2 GB / 4.0 GB
Task B    Completed      800 MB
Task C    Paused         2.4 GB / 6.0 GB
```

Library：

```text
Library
------------------------------------------------
Movie A.mp4      Video     4.0 GB    Ready
Movie B.mkv      Video     8.2 GB    Ready
Album.flac       Audio     1.1 GB    Ready
```

完成任务可以进入 Library。

但：

> **“Task Completed” ≠ “LibraryItem”**

二者必须保持独立。

---

# 六、核心领域模型

根据当前工程进行适配，但原则上至少需要：

## LibraryRoot

表示用户授权 Tide 扫描的本地目录。

建议包含：

```text
LibraryRoot
- id
- path
- display_name
- enabled
- created_at
- updated_at
- last_scan_at
- scan_status
```

不要把目录扫描结果全部写成“Root 的子对象”。

Root 与 Item 必须独立持久化。

---

# 七、LibraryItem

建议至少包含：

```text
LibraryItem
- id
- root_id
- canonical_path
- display_name
- relative_path
- extension
- media_category
- mime_type
- size_bytes
- modified_at
- created_at
- updated_at
- availability_status
- origin_type
- task_id nullable
- resource_id nullable
- checksum nullable
- fingerprint nullable
- last_verified_at nullable
```

实际 schema 根据现有数据库设计调整。

---

# 八、LibraryItem 的 origin

必须明确资源来源，但不要把它与 Source 混成一层。

例如：

```text
OriginType

Download
ImportedLocal
Unknown
```

其中：

```text
Download
```

意味着这个文件来自 Tide 的 Download Task。

并且：

```text
task_id
```

可以关联到对应任务。

但不要强制：

```text
每个 LibraryItem 都必须有 Task
```

因为用户可以把现有本地文件夹加入 Library。

---

# 九、Resource 与 LibraryItem 的关系

这是 M6 的重要架构边界。

不要定义：

```text
Resource = LibraryItem
```

也不要定义：

```text
LibraryItem = Resource
```

建议：

```text
Resource
    ↓
可读取字节的抽象
```

而：

```text
LibraryItem
    ↓
用户希望长期看到、管理、打开的本地实体
```

一个 LibraryItem 可以产生：

```text
PlaybackSession
```

而 PlaybackSession 最终通过：

```text
Resource / ReadRange
```

进入：

```text
M4 Stream Gateway
```

形成：

```text
LibraryItem
    ↓
CreatePlaybackSession
    ↓
Resource
    ↓
Range Gateway
    ↓
Player
```

**不要让 frontend 直接拿本地绝对路径去播放。**

---

# 十、Filesystem 是事实来源之一

Library 数据库不能成为“文件真实存在性”的唯一来源。

必须明确：

```text
SQLite = Library metadata / index
Filesystem = actual file existence / actual size / actual readability
```

因此：

数据库认为：

```text
Movie.mp4 exists
```

但文件已经被 Windows Explorer 删除：

```text
Filesystem
不存在
```

Library 必须能够识别：

```text
Missing
```

而不是继续伪造：

```text
Available
```

---

# 十一、Canonical Path

Library 中路径必须标准化。

必须处理：

* 相对路径
* `.` / `..`
* Windows drive letter
* 大小写问题
* path separator
* trailing separator
* UNC
* junction
* symlink/reparse point
* 不存在路径
* 权限不足

核心原则：

> Library 保存的是经过验证的本机路径，不允许来自 frontend 的任意字符串直接成为数据库可信路径。

所有读取、播放、打开、删除操作都必须再次验证。

---

# 十二、Windows 路径安全

这是 M6 的 P0/P1 安全重点之一。

必须测试：

```text
..\secret.mp4
..\..\Windows\...
C:\Windows\...
\\server\share\...
\\?\...
CON
NUL
PRN
AUX
COM1
LPT1
```

以及：

* junction
* symbolic link
* directory reparse point
* file reparse point

不能因为：

```text
path.starts_with(root)
```

就认为安全。

必须使用可靠的 canonicalization / containment 方案。

尤其注意：

```text
C:\Library
C:\Library-Evil
```

不能被错误判定为：

```text
C:\Library 的子目录
```

---

# 十三、Library Root 的规则

M6 只允许显式加入的本地目录成为 LibraryRoot。

UI：

```text
Settings
   ↓
Library Folders
```

可以：

```text
Add Folder
Remove Folder
Enable / Disable
Rescan
Open Folder
```

不要自动扫描整个：

```text
C:\
D:\
```

更不能扫描：

```text
所有磁盘
```

除非用户明确选择。

---

# 十四、默认 Download Root

如果当前设置已经存在：

```text
download_root
```

可以允许它成为默认 Library Root，或提供：

```text
Add Download Folder to Library
```

但不要强制二者始终相同。

这是两个概念：

```text
Download Destination
```

和：

```text
Library Root
```

用户可能：

```text
下载目录 = D:\Downloads
Library = D:\Media
```

也可能：

```text
下载目录 = D:\Media
Library = D:\Media
```

都必须支持。

---

# 十五、Scanner

实现正式的：

```text
LibraryScanner
```

建议职责：

```text
Root
 ↓
Directory traversal
 ↓
Candidate files
 ↓
Path validation
 ↓
File metadata
 ↓
Supported media classification
 ↓
LibraryRepository upsert
```

Scanner 不应该负责：

* 播放
* 下载
* 删除任务
* 网络
* HTTP
* Torrent
* Gateway

---

# 十六、扫描文件规则

默认排除：

```text
.part
.tmp
.crdownload
.download
```

以及 Tide 自己产生的未完成临时文件。

不要把：

```text
Movie.mp4.part
```

识别成：

```text
Movie.mp4
```

---

# 十七、支持的媒体分类

根据 M0.5 的实际 Media Compatibility Matrix + M5 当前播放器支持矩阵生成。

建议建立统一分类：

```text
Video
Audio
Image
Subtitle
Archive
Document
Other
```

但 Library 默认只展示：

```text
Video
Audio
Image
```

其余类型可以保留索引能力，但不作为主要播放内容。

不要假设：

```text
.mp4 = 必然可以播放
.mkv = 必然可以播放
```

扩展名只是分类依据之一。

真实播放能力属于 M5 Player。

---

# 十八、不要把 Extension 当作唯一真相

必须避免：

```text
if extension == ".mp4"
    => playable
```

更合理的是：

```text
Extension
+
MIME / known type
+
Player capability
```

最终 UI 可以显示：

```text
Playable
Unsupported
Unknown
Missing
```

但这个状态必须来自真实系统能力，而不是“看起来可能支持”。

---

# 十九、扫描时不要粗暴读取整个文件

首次扫描不能因为：

```text
10,000 个媒体文件
```

就对每个文件执行：

```text
SHA-256 entire file
```

否则 Library 首次扫描可能变成 IO 灾难。

默认索引应该优先使用：

```text
path
size
modified_at
extension
filesystem metadata
```

只有在确有需求时：

```text
checksum / fingerprint
```

再进行。

---

# 二十、Checksum / Fingerprint

M1 已经存在的下载 checksum 应尽可能复用。

例如：

```text
Download completed
↓
M1 checksum verified
↓
LibraryItem.checksum = known value
```

而不是：

```text
Download completed
↓
Library 又重新 SHA-256 一遍
```

对于用户自行加入的文件：

可以提供：

```text
Fingerprint
```

而不是默认全文件 hash。

一个合理的增量指纹策略可以是：

```text
size
+
mtime
+
small sample hash
```

但只有在实际实现确实有价值时再加入。

**不要为了“未来可能需要去重”而把 M6 变成 Hash 工程。**

---

# 二十一、扫描幂等性

同一个 Root 连续扫描十次：

结果必须稳定。

例如：

```text
Scan
Scan
Scan
```

不应该创建：

```text
LibraryItem #1
LibraryItem #2
LibraryItem #3
```

必须有稳定唯一约束，例如基于：

```text
root_id + canonical_path
```

具体实现按照 SQLite 实际能力设计。

---

# 二十二、扫描更新规则

对于已经存在的 LibraryItem：

如果：

```text
path same
size same
mtime same
```

不要重复执行昂贵操作。

如果：

```text
size changed
mtime changed
```

重新验证 metadata。

如果文件消失：

```text
Available → Missing
```

如果再次出现：

```text
Missing → Available
```

---

# 二十三、Orphan / Missing 处理

扫描后必须能够识别：

```text
Database item exists
but filesystem item doesn't
```

不要立即删除数据库记录。

默认状态：

```text
Missing
```

这样用户把文件移动回来后仍可以恢复。

---

# 二十四、删除 / 移动语义

必须区分：

```text
Remove from Library
```

和：

```text
Delete File
```

不能绑定成一个动作。

例如：

```text
Remove from Library
```

只删除索引。

而：

```text
Delete from Disk
```

才删除真实文件。

两者必须有不同 IPC command。

---

# 二十五、物理删除必须极其谨慎

`delete_file` 只能作用于：

```text
LibraryItem
```

并且 backend 再次验证：

```text
canonical path
belongs to configured LibraryRoot
not reparse escape
not directory
not protected/system path
```

不能允许 frontend 直接调用：

```text
delete_path("C:\\xxx")
```

必须：

```text
delete_library_item(item_id)
```

backend 根据 item_id 找出数据库中的 canonical path，再重新验证。

---

# 二十六、下载任务 → Library 自动入库

这是 M6 的关键产品链之一。

当：

```text
Task
↓
Completed
```

并且最终文件：

```text
存在
可访问
已经完成原子 finalization
```

则可以：

```text
Create / Update LibraryItem
```

不要在：

```text
Running
```

阶段自动把未完成文件显示成正常 Library 内容。

除非明确标记：

```text
Incomplete
```

但第一版可以直接：

> **未完成资源不进入主 Library。**

这比把 `.part` 暴露给用户简单可靠。

---

# 二十七、Torrent 多文件任务

M3 Torrent 可能产生：

```text
Torrent
 ├── Movie.mp4
 ├── Subtitle.srt
 ├── Poster.jpg
 └── Sample/
```

M6 必须明确决定：

### 模式 A

一个 Torrent Task：

```text
→ 多个 LibraryItem
```

这是推荐结构。

不要：

```text
Torrent Task = 一个 LibraryItem
```

因为一个 Torrent 可能包含多个实际可管理文件。

---

# 二十八、Directory 与 LibraryItem

M6 默认不要把“目录”当作媒体 LibraryItem。

例如：

```text
D:\Movies
  MovieA.mp4
  MovieB.mp4
```

LibraryItem：

```text
MovieA.mp4
MovieB.mp4
```

而：

```text
Movies
```

属于：

```text
LibraryRoot / folder hierarchy
```

而不是一个可播放 Item。

未来可以增加 Folder view，但不要扭曲 Item 模型。

---

# 二十九、数据库设计

新建 migration，例如：

```text
migrations/
    xxxx_m6_library.sql
```

具体编号根据当前项目 migration 状态决定。

建议至少：

```text
library_roots
library_items
```

如实际需要：

```text
library_scan_runs
```

但不要默认创建一堆审计表。

---

# 三十、推荐 LibraryItem 数据结构

逻辑上至少保证：

```text
id                  UUID
root_id             UUID
canonical_path      TEXT
relative_path       TEXT
display_name        TEXT

extension            TEXT nullable
media_category       TEXT
mime_type            TEXT nullable

size_bytes           INTEGER
modified_at          TIMESTAMP

availability_status  TEXT

origin_type          TEXT

task_id              UUID nullable
resource_id          UUID nullable

checksum             TEXT nullable
fingerprint          TEXT nullable

created_at           TIMESTAMP
updated_at           TIMESTAMP
last_verified_at     TIMESTAMP
```

字段具体名称可以适配现有 schema。

不要为了“完整”加入大量没有实际使用者的字段。

---

# 三十一、SQLite 约束

数据库中必须建立合理的：

```text
PRIMARY KEY
FOREIGN KEY
UNIQUE
INDEX
CHECK
```

尤其：

```text
root_id + canonical_path
```

必须避免重复。

并且考虑：

```text
task_id
```

可能为空。

不要把：

```text
task_id NOT NULL
```

作为 Library 的强约束。

---

# 三十二、Repository 层

严格维持：

```text
Application
    ↓
LibraryService
    ↓
LibraryRepository
    ↓
SQLite
```

Scanner 不应该：

```text
直接执行 SQL
```

IPC command 也不应该：

```text
直接操作 SQLite
```

统一通过 Service / Repository。

---

# 三十三、推荐服务边界

根据当前项目结构，可以增加类似：

```text
core/library/
    mod.rs
    model.rs
    repository.rs
    service.rs
    scanner.rs
    classifier.rs
    path.rs
    metadata.rs
    error.rs
```

不要机械照搬。

如果现有 Core 结构已经有：

```text
TaskService
ResourceService
StorageService
```

应该与现有风格保持一致。

---

# 三十四、Library Service

LibraryService 至少负责：

```text
add_root
remove_root
list_roots
scan_root
scan_all
list_items
get_item
refresh_item
remove_item
delete_item
create_playback_session
```

实际 command 命名以现有项目为准。

---

# 三十五、Scanner 与 Service 分离

不要：

```text
LibraryService = Scanner
```

应该：

```text
LibraryService
    ↓
LibraryScanner
```

原因：

Scanner 以后可能需要：

* startup scan
* manual scan
* background scan
* watcher
* rescan single folder

如果全部写进 Service，后期很容易失控。

---

# 三十六、M6 不强制实时 File Watcher

第一版至少必须有：

```text
Manual Scan
Startup Verification
Download Completion Update
```

不要求为了“看起来高级”立刻实现完整：

```text
ReadDirectoryChangesW
```

或者复杂 cross-platform watcher。

如果当前代码已经有可靠 watcher，则可以复用。

如果没有：

> **优先保证 Scanner 正确，再考虑实时监听。**

不要让 watcher 反过来决定 Library 数据模型。

---

# 三十七、扫描并发

大目录扫描需要考虑：

```text
CPU
Disk IO
SQLite writes
UI responsiveness
```

不要：

```text
for every file:
    INSERT
    COMMIT
```

建议：

```text
enumerate
↓
batch
↓
transaction
```

具体 batch size 通过 benchmark 决定。

不要主观选择一个巨大的数字。

---

# 三十八、不要阻塞 UI

Library 扫描不能在：

```text
UI thread
```

同步执行整个目录遍历。

必须：

```text
background task
```

并提供：

```text
scan_started
scan_progress
scan_completed
scan_failed
```

类似事件。

---

# 三十九、扫描进度

建议事件包含：

```text
root_id
files_seen
items_added
items_updated
items_missing
errors
```

如果无法可靠获得总文件数：

不要伪造：

```text
37%
```

可以显示：

```text
Scanning...
1,284 files
```

或者：

```text
1,284 files scanned
```

只有真实知道 total 时才显示 percentage。

---

# 四十、事件节流

不要：

```text
每扫描一个文件
→ IPC event
→ frontend render
```

否则大目录会产生 event flood。

应采用：

```text
batch / throttled update
```

例如约：

```text
100–500ms
```

或按：

```text
N items
```

发送。

具体阈值以实际 benchmark 调整。

---

# 四十一、Library 查询

前端必须支持基本：

```text
list
search
filter
sort
```

至少支持：

### Search

搜索：

```text
display_name
relative_path
```

不要默认：

```text
全文搜索二进制内容
```

---

# 四十二、Filter

至少支持：

```text
All
Video
Audio
Image
Missing
Playable
```

实际 Player Matrix 若不支持某类别，则不要提供虚假的 Playable。

也可以支持：

```text
Origin:
  All
  Downloaded
  Imported
```

---

# 四十三、Sort

至少：

```text
Name
Size
Modified Time
Added Time
```

排序必须在 backend/query 层完成，而不是：

```text
frontend 拉全量
↓
JS sort
```

尤其当 Library 达到：

```text
10,000+
```

项时。

---

# 四十四、分页 / 虚拟化

如果当前 frontend 已经有成熟列表组件，可以使用。

否则至少避免：

```text
一次渲染 50,000 DOM nodes
```

Library frontend 应该支持：

```text
pagination
```

或者：

```text
virtualized list
```

二者选一。

不要为了复杂而同时实现。

---

# 四十五、Library 不应该一次加载所有文件

错误设计：

```text
SELECT * FROM library_items
```

然后：

```text
frontend render everything
```

正确方向：

```text
query
limit
offset / cursor
```

例如：

```text
page=1
page_size=50
```

最终根据实际 performance 调整。

---

# 四十六、Playback 链路

用户点击 Library Item：

```text
frontend
  ↓
play(item_id)
  ↓
IPC
  ↓
LibraryService
  ↓
validate item
  ↓
create PlaybackSession
  ↓
Resource
  ↓
M4 Stream Gateway
  ↓
stream URL
  ↓
M5 Player
```

禁止：

```text
frontend:
  video.src = "file:///C:/..."
```

也禁止：

```text
frontend:
  send absolute_path to backend
```

---

# 四十七、PlaybackSession

重新使用 M5 已有：

```text
PlaybackSession
```

不要因为 Library 又造一个：

```text
LibraryPlaybackSession
```

或：

```text
MediaPlayerSession
```

除非现有 M5 架构确实缺失且需要最小修正。

---

# 四十八、播放前验证

必须在创建 PlaybackSession 前验证：

```text
item exists
path exists
path still within root
not directory
not forbidden/reparse escape
file readable
metadata reasonable
```

数据库记录不能被视为可信权限证明。

---

# 四十九、文件被外部修改

如果：

```text
LibraryItem:
size = 4 GB

actual file:
size = 2 GB
```

必须在播放之前检测。

可以：

```text
refresh_item
```

或者：

```text
create_playback_session
→ validate metadata
```

发现变化：

```text
mark stale / refresh
```

然后重新读取实际状态。

---

# 五十、文件移动

用户可能在 Explorer：

```text
Movie.mp4
↓
Move
D:\Movies\Movie.mp4
```

M6 不要求立刻自动猜出新路径。

默认：

```text
old item → Missing
```

以后可以通过：

```text
Rescan
```

发现新的文件。

不要做模糊路径猜测。

尤其不要：

```text
同名文件 → 自动绑定
```

否则可能发生错误归档。

---

# 五十一、重复文件

M6 至少处理：

### Path duplicate

数据库层禁止。

### Content duplicate

如果有：

```text
verified checksum
```

可以提示：

```text
Possible duplicate
```

但不要默认自动删除。

例如：

```text
Movie A.mp4
Movie Copy.mp4
```

即使 hash 一样：

也不能自动删掉其中一个。

---

# 五十二、Download Task 与 Library 生命周期

明确设计：

```text
Task Created
↓
Running
↓
Completed
↓
Final File
↓
LibraryItem Available
```

如果：

```text
LibraryItem 删除
```

不应反向：

```text
Task = Cancelled
```

因为：

> Library 生命周期和 Task 生命周期不同。

---

# 五十三、Task 删除与 Library

例如用户：

```text
删除 Download Task
```

但最终文件已经存在于 Library。

不能默认：

```text
Task delete
→ Delete library item
→ Delete file
```

这是危险的。

合理原则：

```text
Task deletion
≠
File deletion
```

最终文件是否删除必须有明确的独立语义。

---

# 五十四、UI 结构

M6 至少提供：

```text
Downloads
Library
Settings
```

Library 页面建议：

```text
┌───────────────────────────────────────┐
│ Library                     [Scan]    │
│ [ Search...                         ] │
│                                       │
│ All  Video  Audio  Image  Missing     │
│                                       │
│ Name            Type    Size   State  │
│ ------------------------------------- │
│ Movie A.mp4     Video   4.1G   Ready  │
│ Movie B.mkv     Video   8.3G   Ready  │
│ Album.flac      Audio   900M   Ready  │
└───────────────────────────────────────┘
```

不要在 M6 花大量时间做视觉设计。

---

# 五十五、Library Item UI 至少提供

每个 Item 可以：

```text
Play
Open File
Open Folder
Refresh
Remove from Library
Delete from Disk
```

其中：

```text
Delete from Disk
```

必须明显标记为 destructive action。

---

# 五十六、Context Menu

可以实现：

```text
Play
Open
Open Containing Folder
Refresh
Remove from Library
Delete File
```

不要提供：

```text
Download Again
Search Online
Find Source
Recommend Similar
```

后者违反 Tide 产品边界。

---

# 五十七、Library 与 Player 的关系

Library 是 Player 的主要入口之一。

但：

```text
Player 不负责 Library database
```

Player 只负责：

```text
PlaybackSession
play
pause
seek
volume
state
```

Library 负责：

```text
which resource
```

保持：

```text
Library
   ↓
PlaybackSession
   ↓
Player
```

而不是：

```text
Library = Player
```

---

# 五十八、M4 Gateway 安全边界必须继续保持

Library 不得破坏 M4 的 Stream Gateway 安全模型。

必须继续：

```text
loopback only
random port
high-entropy session token
session → resource mapping
Host validation
Range validation
bounded requests
timeout
concurrency limit
cleanup
```

Library 不应该获得：

```text
任意文件 HTTP server
```

特别禁止：

```text
GET /C:/Users/...
GET /../../secret
```

---

# 五十九、禁止把 Library Root 变成文件服务器

错误设计：

```text
/library?path=...
```

用户只要改变：

```text
path
```

就能读取任意文件。

正确设计：

```text
LibraryItem ID
↓
backend lookup
↓
validated canonical path
↓
PlaybackSession capability
↓
Range Gateway
```

---

# 六十、IPC

IPC 应该使用：

```text
typed request
typed response
stable DTO
```

不要：

```text
frontend → arbitrary JSON → backend
```

至少需要类似：

```text
library.list_roots
library.add_root
library.remove_root
library.scan
library.list_items
library.get_item
library.refresh_item
library.remove_item
library.delete_item
library.play
```

具体命名必须匹配当前项目风格。

---

# 六十一、DTO 与 Domain 分离

不要直接把：

```text
Rust LibraryItem
```

暴露给 frontend。

建立：

```text
LibraryItemDto
LibraryRootDto
LibraryScanProgressDto
```

必要时隐藏：

```text
canonical internal path
```

尤其不应该因为 UI 需要一个显示名称，就把所有 backend 内部字段直接暴露。

---

# 六十二、错误模型

继续使用项目已有：

```text
TideError
```

或等价统一错误体系。

Library 至少能区分：

```text
Validation
NotFound
PermissionDenied
Filesystem
Database
Scan
Playback
Security
Unsupported
Busy
```

不要向 frontend 返回：

```text
Rust panic
stack trace
raw filesystem exception
```

---

# 六十三、不要 Panic

正常情况下：

```text
file disappeared
permission denied
folder removed
invalid item
database constraint
```

都应该成为可处理错误。

不能：

```text
panic!
unwrap()
expect()
```

成为正常运行路径。

---

# 六十四、Logging

日志不能泄露：

* 用户完整 URL
* Magnet
* Info Hash（除非确实属于必要诊断且经过脱敏）
* Peer IP
* 完整用户历史
* 任意文件绝对路径

例如不要：

```text
Playing C:\Users\Gray\Downloads\Secret\Movie.mp4
```

可以：

```text
library.play item_id=... category=video
```

---

# 六十五、隐私

M6 继续坚持：

```text
Local-first
No telemetry by default
No cloud indexing
No content upload
```

Library 数据完全存本机 SQLite。

Tide 不应该为了：

```text
“更智能的媒体库”
```

把：

```text
文件名
路径
哈希
```

发送到 ifuyo 服务器。

---

# 六十六、Migration

必须使用现有 SQLite migration 机制。

禁止：

```text
直接 ALTER production DB
```

而没有 migration。

禁止修改旧 migration：

```text
已经应用过的 migration
```

应该新增：

```text
M6 migration
```

必须测试：

```text
old DB
↓
migration
↓
new schema
```

---

# 六十七、数据库事务

以下操作尽可能保证事务：

```text
scan batch
upsert item
mark missing
remove item
```

尤其不要：

```text
扫描 1000 个文件
↓
999 个成功
↓
第 1000 个失败
↓
整个 DB 状态处于未知半完成状态
```

应该设计清楚：

```text
scan run
transaction / batch transaction
```

或者可恢复的增量策略。

---

# 六十八、推荐 Scan 算法

一个可靠方向：

```text
Start Scan
    ↓
enumerate root
    ↓
validate each candidate
    ↓
collect discovered paths
    ↓
upsert current items
    ↓
mark unseen previous items as Missing
    ↓
finish scan
```

但是不要机械采用。

重点是：

> **只有扫描真正成功覆盖了某个目录范围，才能把数据库中未出现的旧 Item 判定为 Missing。**

不能：

```text
扫描中途权限错误
→
把剩余所有旧文件都标记 Missing
```

---

# 六十九、Partial Scan

如果：

```text
D:\Media\A
```

可读，

但：

```text
D:\Media\B
```

权限被拒绝，

不要把整个 Root 标记成：

```text
success
```

应该记录：

```text
partial
```

并保留：

```text
scan_errors
```

这样不会因为权限问题制造大量假 Missing。

---

# 七十、Scan Run

可以考虑：

```text
LibraryScanRun
- id
- root_id
- started_at
- finished_at
- status
- files_seen
- added
- updated
- missing
- errors
```

但只有当前架构确实需要持久化扫描历史时才实现。

否则可以保持：

```text
运行时状态 + root.last_scan_at
```

避免不必要复杂度。

---

# 七十一、自动扫描时机

M6 至少支持：

### App Startup

对 enabled roots 做：

```text
verification
```

不一定每次完整深度扫描。

### Manual Rescan

用户主动触发：

```text
Full Scan
```

### Download Completion

下载成功后：

```text
Target file
↓
Library upsert
```

这三个入口足够形成第一版完整闭环。

---

# 七十二、启动性能

如果有：

```text
100,000 files
```

不能让：

```text
Tide launch
```

等待完整 Library Scan 才显示主界面。

优先：

```text
DB load
↓
UI ready
↓
background scan
```

或者：

```text
lightweight verification
```

---

# 七十三、Library Startup Recovery

结合 M2：

启动时：

```text
SQLite
↓
load LibraryItems
↓
validate stale / missing state
↓
restore runtime
```

但不要每次启动都对所有大文件做 hash。

只做：

```text
exists
basic metadata
```

必要时再深入验证。

---

# 七十四、索引性能目标

必须进行真实 benchmark。

至少测试：

```text
1,000 files
10,000 files
50,000 files
100,000 files
```

观察：

```text
scan duration
SQLite write time
UI responsiveness
memory usage
startup time
search latency
filter latency
sort latency
```

不要只测 100 个文件。

---

# 七十五、Search 性能

至少测试：

```text
10k
50k
100k
```

确保：

```text
search latency
```

不会随规模产生不可接受退化。

SQLite 可以根据实际 benchmark 决定是否建立：

```text
INDEX(display_name)
INDEX(media_category)
INDEX(modified_at)
INDEX(size_bytes)
```

不要无脑给所有列建索引。

---

# 七十六、如果实现全文搜索

默认：

```text
LIKE
```

先满足需求即可。

只有 benchmark 证明：

```text
10万+ 数据
```

普通搜索不足，再考虑 SQLite FTS。

不要一开始就做：

```text
FTS5
fuzzy search
semantic search
AI search
```

---

# 七十七、File Type Classification

必须集中到：

```text
LibraryClassifier
```

而不是到处：

```text
if path.ends_with(".mp4")
```

例如：

```text
classify(path, metadata)
    → MediaCategory
```

未来扩展格式时只修改一处。

---

# 七十八、MIME

不要完全相信文件扩展名。

但也不要因为 MIME 不确定就直接：

```text
Unsupported
```

可以：

```text
mime = Optional
category = known from extension
playability = delegated to Player
```

这样 Library 与 Player 可以独立演进。

---

# 七十九、M5 Player 能力查询

如果 M5 已经有：

```text
PlayerCapability
```

Library 可以使用：

```text
player.can_play(item)
```

但必须保持反向依赖边界：

不要让：

```text
LibraryScanner
```

依赖完整 Player UI。

推荐：

```text
Library
  ↓
PlaybackCapability / PlaybackService
```

---

# 八十、Local File Playback 与 Download Playback 统一

最终：

```text
HTTP completed file
Torrent completed file
Existing local file
```

全部进入：

```text
LibraryItem
```

之后：

```text
LibraryItem
↓
Resource
↓
PlaybackSession
↓
same Player
```

这是 M6 最重要的架构收益之一。

不要出现三套播放器：

```text
HTTP Player
Torrent Player
Local Player
```

---

# 八十一、Streaming 不应该依赖文件已完整读取到内存

Library 播放必须继续走：

```text
Range
```

而不是：

```text
read entire file
→ Vec<u8>
→ frontend
```

禁止大文件：

```text
entire file into RAM
```

---

# 八十二、Range

例如：

```text
0-1MB
```

应该：

```text
Resource.read(range)
```

读取。

不要因为 Library 只是本地文件，就绕过 Resource 抽象。

这会破坏 M4 的统一架构。

---

# 八十三、Availability

对于已经完成的 Library 文件：

```text
Availability([0,size))
```

应当视为：

```text
fully available
```

因此：

```text
Downloaded HTTP file
Downloaded Torrent file
Local imported file
```

最终都可以映射为：

```text
complete Resource
```

这是 M4 Resource abstraction 的重要落地点。

---

# 八十四、未来统一 Resource

M6 最终应该让：

```text
LibraryItem
   ↓
Resource
```

成为标准入口。

例如：

```text
LibraryItemResource
```

实现：

```text
read(range)
availability(range)
size()
content_type()
```

这样 M4/M5 不需要知道：

```text
这个文件究竟来源于 Download
Torrent
还是 Local Import
```

---

# 八十五、不要把 Resource 改造成万能对象

禁止为了 M6 而让：

```text
Resource
```

直接包含：

* UI state
* Library state
* Task state
* Player state
* filesystem UI metadata
* download progress

Resource 只回答：

> **“这个东西的字节现在在哪里、能否读取、如何读取。”**

---

# 八十六、UI 状态机

Library Item 至少区分：

```text
Available
Missing
Inaccessible
Unsupported
```

如果当前项目已有统一 enum，复用。

不要使用大量：

```text
boolean:
isMissing
isPlayable
isAvailable
isIndexed
isDeleted
...
```

否则很容易产生：

```text
isMissing = true
isAvailable = true
```

这种非法组合。

---

# 八十七、Library Root 状态

建议：

```text
Enabled
Disabled
Unavailable
Scanning
```

不要使用很多布尔值堆叠。

---

# 八十八、Frontend 状态管理

frontend 不应该成为 Library 真相来源。

正确：

```text
Rust backend
     ↓
IPC
     ↓
Frontend cache/view state
```

用户刷新页面：

```text
Library 不丢
```

关闭应用：

```text
Library 不丢
```

重新打开：

```text
SQLite → UI
```

---

# 八十九、IPC 更新事件

建议：

```text
library.root_added
library.root_removed
library.scan_started
library.scan_progress
library.scan_completed
library.item_added
library.item_updated
library.item_removed
library.item_missing
```

但事件数量必须根据实际 UI 需要压缩。

不要：

```text
每个 filesystem operation → event
```

---

# 九十、与 M7 Scheduler 的边界

M6 不负责智能调度。

尤其不要新增：

```text
LibraryScheduler
```

或：

```text
PlaybackDownloadPlanner
```

M7 才负责真正：

```text
Demand
→ Priority
→ Scheduler
→ Download / Torrent
```

M6 只负责：

```text
LibraryItem
→ PlaybackSession
```

---

# 九十一、与 M8 Windows Integration 的边界

M6 可以实现基本：

```text
Open File
Open Folder
```

但不要在 M6 实现：

* Explorer shell extensions
* Windows thumbnail handler
* file associations
* Jump List
* context-menu integration
* tray integration
* startup integration
* Windows notifications

这些属于后续 Windows Integration / Polish。

---

