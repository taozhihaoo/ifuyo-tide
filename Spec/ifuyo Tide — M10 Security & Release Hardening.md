# ifuyo Tide — M10 Security & Release Hardening

## 完整、详细、专业的开发实施提示词

> **阶段定位：M10**
>
> **阶段名称：Security & Release Hardening**
>
> **目标：**
> 将当前 Tide 从“功能已经比较完整的可运行开发版”推进到“安全边界明确、发布配置收口、供应链经过审计、生产构建可验证、可进入 M11 Release Candidate”的状态。
>
> **本阶段不是功能开发阶段。**
>
> M10 的核心工作不是继续增加播放器能力、下载能力、Torrent 能力、Library 能力或 UI 功能，而是：
>
> **Attack Surface Audit → Threat Model → Security Hardening → Dependency/Supply-chain Audit → Production Configuration → Installer/Update Preparation → Release Validation → Security Regression**
>
> ---
>
> ## 一、开始前必须遵守的工作纪律
>
> 在修改任何代码之前，必须先完整检查当前仓库，而不是假设 M0～M9 的实现一定与设计文档完全一致。
>
> 必须先检查：
>
> 1. 当前 Git 状态
> 2. 当前分支
> 3. 当前 commit
> 4. Tauri 版本
> 5. Rust toolchain
> 6. Node/package manager/前端依赖版本
> 7. `Cargo.lock`
> 8. `package-lock.json` / 其他 lockfile
> 9. M0～M9 实际代码结构
> 10. Tauri capabilities
> 11. Tauri permissions
> 12. CSP
> 13. IPC commands
> 14. HTTP downloader
> 15. Torrent/librqbit 集成
> 16. Resource / Availability
> 17. Scheduler
> 18. Stream Gateway
> 19. Player
> 20. Library
> 21. SQLite / migration
> 22. 日志系统
> 23. 错误系统
> 24. 文件系统访问
> 25. 外部进程调用
> 26. 安装包配置
> 27. CI/CD
> 28. release 配置
> 29. updater 配置（如果已经存在）
> 30. development-only/debug/test-only 代码
>
> 开始阶段先输出：
>
> ```text
> FACTS
> HYPOTHESES
> CURRENT M0-M9 STATE
> ATTACK SURFACE INVENTORY
> THREAT MODEL
> EXISTING SECURITY CONTROLS
> SECURITY GAPS
> RELEASE GAPS
> SUPPLY-CHAIN GAPS
> M10 IMPLEMENTATION PLAN
> RISKS
> ```
>
> **禁止根据历史方案脑补当前代码。**
>
> 代码、配置、依赖、测试结果优先于历史文档。
>
> 如果历史设计与当前代码冲突：
>
> ```text
> FACT = 当前代码真实行为
> DOCUMENTED INTENT = 设计文档中的预期行为
> GAP = 二者之间的差异
> ```
>
> 不允许直接把“设计如此”当成“已经如此”。
>
> ---
>
> # 二、M10 的核心目标
>
> M10 必须最终建立下面这条完整安全闭环：
>
> ```text
> Untrusted Input
>     ↓
> Parse
>     ↓
> Validate
>     ↓
> Authorize
>     ↓
> Execute
>     ↓
> Constrain Side Effects
>     ↓
> Audit / Log Safely
>     ↓
> Recover / Fail Closed
>     ↓
> Security Regression Test
> ```
>
> 对 Tide 而言，需要重点覆盖：
>
> ```text
> URL
> Redirect
> DNS / IP
> HTTP Response
> Torrent / Magnet
> Torrent Metadata
> Peer Data
> File Name
> File Path
> Local File
> Range Request
> Stream Session
> HTTP Gateway Request
> IPC Arguments
> WebView Navigation
> WebView Messages
> Media File
> SQLite Data
> Config Data
> Release Artifact
> Third-party Dependency
> Update Artifact
> CLI / Environment / Debug Flags
> ```
>
> ---
>
> # 三、严格的 M10 Scope
>
> ## 必须做
>
> ### A. 全面安全审计
>
> ### B. Tauri 权限 / Capability 收口
>
> ### C. CSP 收口
>
> ### D. IPC 安全审计
>
> ### E. HTTP/HTTPS SSRF / Redirect 安全审计
>
> ### F. 文件系统安全审计
>
> ### G. Torrent / librqbit 安全边界审计
>
> ### H. Local Stream Gateway 安全审计
>
> ### I. WebView2 安全审计
>
> ### J. SQLite / Persistence 安全审计
>
> ### K. 日志与隐私审计
>
> ### L. 依赖与供应链审计
>
> ### M. Production Build Hardening
>
> ### N. Windows Installer / Uninstaller / Upgrade 验证
>
> ### O. Code Signing / Update Signing 发布准备
>
> ### P. Release Artifact 完整性验证
>
> ### Q. 安全回归测试
>
> ### R. 安全与发布文档
>
> ---
>
> # 四、M10 明确禁止范围
>
> 本阶段禁止借机加入：
>
> * 新的资源搜索
> * 资源索引
> * 推荐系统
> * 用户账号
> * 云端服务
> * 用户上传
> * 在线媒体库
> * 社区
> * 社交功能
> * 第三方资源站导航
> * Tracker 推荐
> * 匿名下载
> * VPN
> * Proxy 产品化
> * DRM 绕过
> * Paywall 绕过
> * 新媒体格式支持
> * 新播放器后端
> * 新的视频解码器
> * NAS 产品化
> * UPnP Media Server
> * Chromecast
> * AirPlay
> * Steamworks
> * Steam Workshop
> * 成就系统
> * 广告
> * 默认 telemetry
> * 用户行为分析
> * 云端 crash reporting
> * 新的下载协议
> * 新的 Torrent 协议实现
> * 大规模 UI redesign
>
> 如果安全审计发现上述模块存在漏洞，只做必要的安全修复，不顺便扩张功能。
>
> ---
>
> # 五、威胁模型 Threat Model
>
> 必须建立正式 Threat Model，而不是只列漏洞名称。
>
> 至少覆盖以下攻击者模型。
>
> ## Threat Actor A：恶意输入提供者
>
> 攻击者可能提供：
>
> * 恶意 URL
> * 恶意 redirect
> * 恶意 hostname
> * 恶意 IP
> * 恶意 Magnet
> * 恶意 `.torrent`
> * 恶意文件名
> * 恶意路径
> * 恶意 Range
> * 恶意 HTTP Header
> * 恶意媒体文件
>
> 目标可能包括：
>
> * 任意文件覆盖
> * 任意文件读取
> * SSRF
> * 资源耗尽
> * 崩溃
> * 路径逃逸
> * 代码执行链
> * 本地服务访问
> * 敏感信息泄露
>
> ---
>
> ## Threat Actor B：本机其他进程
>
> 必须考虑：
>
> * 本机其他程序扫描 localhost
> * 尝试访问 Stream Gateway
> * 构造恶意 HTTP 请求
> * 调用不应开放的 IPC
> * 读取配置或日志
> * 读取临时文件
> * 竞争修改下载文件
> * 劫持/猜测临时端口
> * 观察命令行 / 日志 / 临时目录信息
>
> ---
>
> ## Threat Actor C：恶意网络 Peer
>
> Torrent 环境中的 peer、tracker、DHT 网络消息都不能视为可信输入。
>
> 必须检查：
>
> * parser 边界
> * 输入长度
> * connection limits
> * timeout
> * malformed metadata
> * malformed peer data
> * resource exhaustion
> * unexpected protocol errors
>
> 不要求重新实现 librqbit 的协议安全机制，但必须审计 Tide 自己暴露出的控制边界以及依赖版本。
>
> ---
>
> ## Threat Actor D：恶意媒体文件
>
> Media 文件是 untrusted input。
>
> 即使文件扩展名是：
>
> ```text
> .mp4
> .mkv
> .webm
> .mov
> ```
>
> 也不能因此认为内容可信。
>
> 必须审计：
>
> ```text
> file → probe → gateway → WebView2/player
> ```
>
> 中每一级的信任边界。
>
> 不允许因为“播放器能播放”就视为安全。
>
> ---
>
> # 六、Tauri Capability / Permission Security
>
> Tauri capability 是 M10 的 P0/P1 审计项目。
>
> 必须逐项列出：
>
> ```text
> Window/WebView
> Capability
> Permission
> Command
> Plugin Permission
> Filesystem Scope
> Shell Scope
> Process Scope
> Dialog Scope
> HTTP Scope
> Notification Scope
> Other
> ```
>
> 建立：
>
> ```text
> Frontend Feature
>     ↓
> IPC Command
>     ↓
> Backend Handler
>     ↓
> Required Permission
> ```
>
> 对每个 capability 回答：
>
> 1. 为什么需要？
> 2. 哪个窗口需要？
> 3. 哪个 command 需要？
> 4. 是否可以进一步缩小 scope？
> 5. 是否仅开发环境需要？
> 6. 是否能从 production 删除？
>
> 特别检查：
>
> ```text
> filesystem
> shell
> process
> HTTP
> dialog
> clipboard
> notification
> updater
> opener
> path
> window
> ```
>
> **禁止保留“为了方便开发而给 frontend 全权限”的配置。**
>
> 如果已经存在过宽权限：
>
> ```text
> REDUCE
> TEST
> DOCUMENT
> ```
>
> 而不是保留并只写注释。
>
> 同时审计：
>
> ```text
> invoke_handler
> command registration
> command exposure
> capability scopes
> window scopes
> ```
>
> 确保没有一个“不需要被前端调用”的 command 暴露在 production capability 中。
>
> 注意：
>
> **Capability 是第一层安全边界，不是最终安全边界。**
>
> 所有 command handler 仍必须在 Rust backend 内自行验证参数、权限、路径、任务 ID、资源 ID 和状态。
>
> ---
>
> # 七、CSP Security
>
> 必须审计当前 CSP。
>
> 目标：
>
> ```text
> default-src
> script-src
> style-src
> img-src
> media-src
> connect-src
> font-src
> object-src
> frame-src
> base-uri
> form-action
> ```
>
> 原则：
>
> * 尽可能 self-contained
> * 禁止无必要的 remote script
> * 禁止无必要 CDN
> * 不允许因为某个功能不工作就简单扩大到 `*`
> * 不允许为修复开发问题永久加入危险的 `unsafe-*`
> * development CSP 与 production CSP 分离
>
> 对以下内容逐项验证：
>
> ```text
> http
> https
> localhost
> 127.0.0.1
> asset:
> ipc:
> blob:
> data:
> custom protocol
> ```
>
> 尤其审计：
>
> **Tide 的 Local Stream Gateway 使用动态 localhost 端口时，不能为了放行播放器请求而把 CSP 简化成过宽的 `http://*`、`connect-src *` 等配置。**
>
> 应根据当前实际 WebView2/Tauri 行为找到最小可行范围，并用自动化测试验证。
>
> ---
>
> # 八、IPC Security Audit
>
> 所有 IPC command 都必须进行输入安全审计。
>
> 每个 command 建立：
>
> ```text
> Command
> Input DTO
> Validation
> Authorization
> Side Effects
> Error Handling
> Sensitive Output
> Log Risk
> Test Coverage
> ```
>
> 特别禁止：
>
> ```rust
> arbitrary_path_from_frontend
> arbitrary_shell_command
> arbitrary_process_spawn
> arbitrary_url_to_system
> arbitrary_command_name
> raw_sql_from_frontend
> ```
>
> 所有 ID：
>
> ```text
> TaskId
> ResourceId
> SessionId
> LibraryItemId
> ```
>
> 必须：
>
> * 严格格式验证
> * 不存在则返回结构化错误
> * 不允许通过字符串拼接构造 SQL
> * 不允许通过 frontend 自己决定安全状态
>
> Rust backend 必须是最终权限与合法性判断者。
>
> ---
>
> # 九、HTTP / HTTPS / SSRF Security
>
> 对 M1 下载器和后续所有 HTTP 请求入口重新审计。
>
> ## 仅允许明确支持的 scheme
>
> 默认只允许：
>
> ```text
> http://
> https://
> ```
>
> 明确拒绝：
>
> ```text
> file://
> ftp://
> ws://
> wss://
> javascript:
> data:
> blob:
> custom:
> ```
>
> ---
>
> ## URL Parser 安全
>
> 禁止用脆弱的字符串匹配判断：
>
> ```text
> hostname
> scheme
> port
> userinfo
> redirect target
> ```
>
> 必须使用成熟 URL parser。
>
> 必须测试：
>
> * userinfo
> * encoded characters
> * IPv4
> * IPv6
> * unusual numeric IP
> * localhost
> * loopback
> * link-local
> * private address
> * redirect
> * malformed host
> * port abuse
> * parser disagreement
>
> ---
>
> ## Redirect Security
>
> 每一次 redirect 都重新进行：
>
> ```text
> Parse
> Scheme validation
> Host validation
> IP validation
> Port validation
> Policy validation
> ```
>
> 不能：
>
> ```text
> initial URL safe
> → follow redirect
> → destination unvalidated
> ```
>
> 必须测试：
>
> ```text
> public → public
> public → localhost
> public → 127.0.0.1
> public → private IP
> public → link-local
> https → http
> http → https
> redirect chain overflow
> redirect loop
> malformed redirect
> ```
>
> 是否允许跨 scheme/cross-host redirect 必须以当前产品设计为准，但任何 redirect target 都必须再次经过安全策略。
>
> ---
>
> ## DNS / IP Security
>
> 审计：
>
> ```text
> hostname resolution
> IPv4
> IPv6
> redirect resolution
> proxy usage
> connection reuse
> DNS rebinding assumptions
> ```
>
> 如果当前实现不能可靠保证某种高级 SSRF 防护，不要伪称已经完全解决。
>
> 必须将：
>
> ```text
> VERIFIED
> PARTIALLY VERIFIED
> NOT GUARANTEED
> ```
>
> 明确写入报告。
>
> ---
>
> # 十、Filesystem Security
>
> 这是 Tide 的最高优先级安全面之一。
>
> 因为 Tide 会：
>
> ```text
> create
> write
> append
> rename
> delete
> read
> stream
> persist
> ```
>
> 文件。
>
> ## 所有用户可控路径必须重新验证
>
> 包括：
>
> * 下载目录
> * 文件名
> * torrent 内部文件名
> * `.part`
> * final file
> * imported local file
> * media path
> * library path
> * stream source
>
> ---
>
> ## 路径逃逸测试
>
> 必须覆盖：
>
> ```text
> ..
> ../
> ..\
> absolute path
> UNC path
> device path
> reserved device name
> trailing dot
> trailing space
> ADS-like syntax
> mixed separators
> UTF-8
> Unicode normalization
> long path
> empty filename
> repeated separators
> encoded traversal
> ```
>
> Windows 特别检查：
>
> ```text
> junction
> symlink
> reparse point
> UNC
> device namespace
> volume path
> alternate data stream
> ```
>
> ---
>
> ## Containment
>
> 所有下载产物必须确保：
>
> ```text
> resolved target
> MUST REMAIN INSIDE
> configured download root
> ```
>
> 不能靠：
>
> ```text
> starts_with(root)
> ```
>
> 这种不充分的字符串判断。
>
> 必须根据 Windows 实际路径语义进行测试。
>
> ---
>
> ## TOCTOU / Race
>
> 对以下操作审计：
>
> ```text
> validate path
> ↓
> create/open
> ↓
> write
> ```
>
> 是否存在攻击者可以在中间替换路径对象的窗口。
>
> 对 reparse point / junction 场景特别测试。
>
> ---
>
> # 十一、Torrent / librqbit Security Audit
>
> 必须重新审计：
>
> ```text
> Magnet
> .torrent
> metainfo
> file list
> file path
> piece data
> peer connection
> tracker response
> DHT data
> HTTP torrent source
> ```
>
> Tide 不重新实现 BitTorrent 协议，但不能把：
>
> ```text
> librqbit 是第三方库
> ```
>
> 当成安全审计完成。
>
> 必须明确：
>
> ```text
> Tide-controlled boundary
> librqbit-controlled boundary
> OS-controlled boundary
> ```
>
> ---
>
> ## 必查
>
> 1. 使用的确切 librqbit/librqbit-core 版本
> 2. Cargo.lock 中实际解析版本
> 3. 是否启用了不必要 feature
> 4. 是否启用了 UPnP / 自动端口映射
> 5. peer listener 绑定地址
> 6. 是否可能监听 `0.0.0.0`
> 7. 是否会产生 Windows firewall 暴露
> 8. inbound peer policy
> 9. connection limits
> 10. timeout
> 11. malformed torrent handling
> 12. malformed metadata handling
> 13. torrent 内部路径处理
> 14. large metadata/resource exhaustion
> 15. 错误是否可能 panic
>
> 默认原则：
>
> **不要为了追求 Torrent 性能而无声地把本地服务暴露到 LAN。**
>
> 如果 inbound peer listener 的确需要非 loopback：
>
> * 必须确认是 Torrent 工作所必需
> * 明确记录实际绑定地址
> * 明确记录端口策略
> * 明确记录用户可感知行为
> * 禁止自动启用 UPnP/NAT port mapping，除非产品明确决定支持
>
> ---
>
> # 十二、Local Progressive Stream Gateway Security
>
> Stream Gateway 是 Tide 最重要的本地安全边界之一。
>
> 重新审计整个：
>
> ```text
> Player
> ↓
> Local HTTP Request
> ↓
> Stream Gateway
> ↓
> PlaybackSession
> ↓
> Resource
> ↓
> Availability
> ↓
> Storage / Torrent
> ```
>
> ---
>
> ## Listener
>
> 默认必须：
>
> ```text
> loopback only
> ```
>
> 优先使用明确可验证的：
>
> ```text
> 127.0.0.1
> ```
>
> 如果实现同时支持 IPv6 loopback，则必须单独测试 `[::1]`。
>
> 禁止：
>
> ```text
> 0.0.0.0
> public interface
> LAN interface
> ```
>
> 除非经过明确产品决策并另行设计。
>
> ---
>
> ## Authentication
>
> Gateway 必须具备：
>
> ```text
> high-entropy session capability
> ```
>
> 不允许：
>
> ```text
> sequential ID
> timestamp token
> predictable hash
> task ID as secret
> resource ID as secret
> ```
>
> 最好采用：
>
> ```text
> random session identifier
> + high entropy capability token
> + short session lifetime
> ```
>
> 因为普通 `<video src>` 很难方便地添加自定义 Authorization header，因此如果 token 必须出现在 URL 中：
>
> * token 必须高熵
> * 生命周期尽可能短
> * 只能绑定当前 PlaybackSession
> * 不记录完整 token
> * 不进入普通诊断日志
> * 设置合适的 Referrer Policy
> * 播放结束后立即失效
> * Session 销毁后立即失效
>
> ---
>
> ## Resource Isolation
>
> Gateway 请求不能直接接受：
>
> ```text
> ?path=C:\...
> ```
>
> 或：
>
> ```text
> ?file=../../...
> ```
>
> Gateway 请求只允许：
>
> ```text
> Session → Resource → Authorized file/range
> ```
>
> 即：
>
> ```text
> Request
> ↓
> Session lookup
> ↓
> Resource lookup
> ↓
> Authorized source
> ↓
> Range validation
> ↓
> Read
> ```
>
> 永远不能：
>
> ```text
> HTTP request → arbitrary filesystem path
> ```
>
> ---
>
> ## Range Security
>
> 必须测试：
>
> ```text
> bytes=0-0
> bytes=0-
> bytes=-1
> bytes=large-large
> bytes=500-100
> multiple ranges
> duplicate ranges
> malformed ranges
> overflow
> u64 boundary
> empty range
> huge range
> ```
>
> 必须确保：
>
> ```text
> no integer overflow
> no panic
> no OOM
> no arbitrary file read
> no range escape
> ```
>
> ---
>
> ## Request Limits
>
> Gateway 至少审计：
>
> ```text
> concurrent requests
> request header size
> range count
> range length
> request timeout
> idle timeout
> session count
> stream count
> cleanup behavior
> ```
>
> 必须有资源耗尽保护。
>
> ---
>
> ## Host Header / Origin
>
> 对 localhost Gateway 请求：
>
> * 校验 Host
> * 拒绝明显异常 Host
> * 不因为 Host header 而访问任意目标
> * 不允许 Gateway 变成 proxy
>
> Gateway 绝对不能支持：
>
> ```text
> /proxy?url=...
> ```
>
> 或任何等价功能。
>
> ---
>
> # 十三、WebView2 Security Audit
>
> 必须把 WebView2 看成：
>
> ```text
> potentially navigable/untrusted content surface
> ```
>
> 而不是天然可信的“应用 UI”。
>
> 审计：
>
> * initial navigation
> * navigation interception
> * external navigation
> * message channel
> * script injection
> * devtools
> * custom protocol
> * local gateway origin
> * downloaded file rendering
> * media loading
> * external hyperlinks
>
> ---
>
> ## Navigation
>
> 必须限制：
>
> ```text
> app UI origin
> expected local stream origin
> ```
>
> 其余导航：
>
> ```text
> block
> ```
>
> 或根据明确产品策略交给系统浏览器。
>
> 不允许在生产环境随意打开任意网站作为 WebView page。
>
> ---
>
> ## Web Messages
>
> 所有：
>
> ```text
> postMessage
> PostWebMessage...
> window.chrome.webview
> ```
>
> 都必须：
>
> * 验证 origin
> * 验证 message schema
> * 验证类型
> * 验证长度
> * 验证枚举
> * 验证 ID
> * 拒绝未知字段（如有必要）
> * 不直接执行未经验证的内容
>
> ---
>
> ## Script Injection
>
> 审计：
>
> ```text
> ExecuteScript
> AddScriptToExecuteOnDocumentCreated
> ```
>
> 任何注入脚本都必须明确其：
>
> ```text
> target origin
> lifecycle
> permission
> data exposure
> ```
>
> 不允许为了方便而注入拥有过大访问能力的全局脚本。
>
> ---
>
> ## Production WebView2
>
> 清理：
>
> * dev-only flags
> * remote debugging
> * test switches
> * debug endpoints
> * test pages
> * development hooks
>
> 不允许把仅用于开发诊断的 WebView2 browser flags 带进 production build。
>
> ---
>
> # 十四、Media Security Boundary
>
> 由于 Tide 会播放用户下载的媒体文件，必须明确：
>
> ```text
> media content = untrusted
> ```
>
> 审计：
>
> ```text
> local file
> HTTP download
> Torrent download
> progressive stream
> probe
> metadata
> poster
> subtitle
> WebView2
> native player
> ```
>
> 如果当前 M5/M9 使用 WebView2 原生 media：
>
> * 不要自行声称 codec 本身安全
> * 不要读取不需要的 metadata
> * 不要把媒体 metadata 解释成应用命令
> * 不允许 media path 越权
>
> 如果当前版本存在 native player / FFmpeg / libmpv：
>
> 必须额外审计：
>
> ```text
> subprocess
> DLL
> codec library
> executable path
> arguments
> environment
> temp files
> process lifetime
> crash behavior
> ```
>
> 如果并不存在，则不要为了“完整审计”新增 native player。
>
> ---
>
> # 十五、SQLite / Persistence Security
>
> 重新审计：
>
> ```text
> SQL
> migration
> IDs
> file paths
> URLs
> error messages
> retry state
> playback sessions
> library
> settings
> ```
>
> 必须保证：
>
> * 所有 SQL 参数化
> * 无 raw SQL 拼接用户输入
> * migration 可重复验证
> * migration 在失败时保持一致性
> * 数据库文件位于 app data 区域
> * 不把大媒体文件存入 SQLite
> * 不将秘密 token 持久化到不必要位置
>
> 数据库中的：
>
> ```text
> URL
> magnet
> info_hash
> peer IP
> local path
> ```
>
> 是否必须存储，要重新评估。
>
> 能不存就不存。
>
> ---
>
> # 十六、Secrets / Sensitive Data
>
> M10 必须进行一次全项目 secret scan。
>
> 搜索：
>
> ```text
> password
> token
> secret
> key
> private_key
> api_key
> auth
> bearer
> credential
> certificate
> signer
> updater
> ```
>
> 检查：
>
> * Git history
> * source
> * config
> * CI
> * logs
> * screenshots/test fixtures
> * test data
>
> 禁止把：
>
> * signing private key
> * updater private key
> * CI secret
> * API credential
> * test production secret
>
> 提交进 Git。
>
> ---
>
> # 十七、Logging / Privacy Security
>
> Tide 默认不上传用户下载内容，也不应默认将用户下载行为发送给 ifuyo。
>
> M10 必须重新检查日志。
>
> production logs 禁止默认出现：
>
> ```text
> full URL
> full magnet
> full info hash（若无必要）
> peer IP
> full filesystem path
> session token
> auth token
> cookies
> request headers
> media metadata
> personal data
> ```
>
> 推荐建立：
>
> ```text
> Safe Diagnostic Representation
> ```
>
> 例如：
>
> ```text
> task_id
> error_category
> http_status
> byte_count
> duration
> retry_count
> resource_id
> session_id
> sanitized filename
> ```
>
> 而不是：
>
> ```text
> original URL
> Authorization header
> absolute path
> token
> ```
>
> 如果必须记录敏感值：
>
> ```text
> redact
> hash
> truncate
> classify
> ```
>
> 同时检查：
>
> ```text
> tracing
> panic message
> error Display
> anyhow chain
> debug formatting
> serde serialization
> frontend console
> ```
>
> 避免敏感内容通过“错误信息”绕过日志脱敏。
>
> ---
>
> # 十八、Privacy Baseline
>
> 检查：
>
> ```text
> telemetry
> analytics
> update checks
> crash reporting
> external requests
> DNS traffic
> CDN
> external scripts
> fonts
> images
> remote configuration
> ```
>
> 必须明确回答：
>
> ```text
> Tide 启动后默认会连接哪些外部服务？
> Tide 是否存在默认 telemetry？
> Tide 是否存在默认 analytics？
> Tide 是否上传用户文件？
> Tide 是否上传 URL？
> Tide 是否上传磁力链接？
> Tide 是否上传日志？
> Tide 是否上传 peer IP？
> ```
>
> 如果答案为“不存在”，必须通过代码与网络测试验证，而不是只写 Privacy 文档。
>
> ---
>
> # 十九、Content / Compliance 文案审计
>
> Tide 的产品文案、README、About、网站介绍、设置页、下载提示、GitHub 页面等必须检查是否存在不必要的风险表述。
>
> 产品应描述为：
>
> ```text
> user-provided source
> local downloader
> local progressive media player
> user-controlled storage
> ```
>
> 不应宣传：
>
> ```text
> anonymous downloading
> pirate resources
> copyright-free torrents
> bypass
> unlock
> evade monitoring
> unlimited piracy
> ```
>
> 同时不要写出过度法律化的绝对承诺，例如：
>
> ```text
> “本软件保证合法”
> “使用本软件绝对不会产生法律风险”
> ```
>
> 正确做法是：
>
> * 软件本身是通用下载/播放工具
> * 用户应确保拥有相应下载、传播和使用权限
> * BitTorrent 某些工作方式可能同时产生上传行为
> * Tide 不提供资源搜索或盗版资源目录
>
> 文案应是事实性说明，而不是法律意见。
>
> ---
>
> # 二十、Dependency / Supply-chain Audit
>
> 对 Rust + frontend dependency 做一次完整供应链审计。
>
> ## Rust
>
> 至少检查：
>
> ```text
> Cargo.lock
> cargo tree
> cargo audit
> RustSec advisories
> dependency source
> git dependencies
> path dependencies
> optional features
> build scripts
> proc macros
> native dependencies
> ```
>
> 使用当前可用的 RustSec tooling。
>
> 如果环境安装不了 `cargo-audit`：
>
> 不得假装完成审计。
>
> 应明确记录：
>
> ```text
> TOOL NOT AVAILABLE
> MANUAL CHECK PERFORMED
> LIMITATION
> ```
>
> ---
>
> ## cargo-deny / license / duplicate dependency
>
> 如果项目规模适合，应考虑建立：
>
> ```text
> cargo deny check
> ```
>
> 至少检查：
>
> ```text
> advisories
> licenses
> bans
> sources
> ```
>
> 不允许因为工具给出“存在多个版本”就机械升级全部依赖。
>
> 应区分：
>
> ```text
> vulnerability
> compliance issue
> maintenance issue
> duplicate-version noise
> ```
>
> ---
>
> ## Frontend
>
> 审计：
>
> ```text
> package-lock
> npm audit
> dependency tree
> direct dependencies
> transitive dependencies
> remote package loading
> postinstall scripts
> build scripts
> bundled assets
> ```
>
> 如果发现漏洞：
>
> 分类：
>
> ```text
> reachable / unreachable
> development-only / production
> direct / transitive
> exploitable / theoretical
> fixed / no fixed version
> ```
>
> 不要追求“扫描器数字必须为 0”而进行无意义升级。
>
> ---
>
> # 二十一、第三方组件 License Audit
>
> 建立：
>
> ```text
> THIRD_PARTY_NOTICES
> ```
>
> 或维护已有：
>
> ```text
> THIRD_PARTY_NOTICES.md
> ```
>
> 至少覆盖：
>
> * Tauri
> * Rust crates
> * librqbit
> * WebView2
> * frontend libraries
> * icon libraries
> * media/player components
> * bundled DLL/executable
>
> 输出：
>
> ```text
> Package
> Version
> License
> Direct/Transitive
> Used In
> Notice Required?
> Source
> ```
>
> 特别确认当前实际使用的 `librqbit` / `librqbit-core` 版本和 license。
>
> 不要只根据 GitHub README 推断，应检查实际 dependency metadata、lockfile 与上游 license 文件。
>
> ---
>
> # 二十二、rqbit / BitTorrent 发布声誉风险
>
> 对当前版本 `librqbit/rqbit` 做一次额外 release-risk 检查：
>
> * upstream issues
> * security advisories
> * dependency advisories
> * Windows Defender / SmartScreen / AV reputation
> * false-positive reports
> * unusual installer detections
>
> 如果发现历史上存在“基于 librqbit 的软件发布后被安全产品误报”的上游报告：
>
> **必须作为 release risk 记录，而不是直接得出“rqbit 恶意”的结论。**
>
> 要做的事情是：
>
> ```text
> VERIFY
> REPRODUCE
> DOCUMENT
> MITIGATE
> MONITOR
> ```
>
> 不能用未经验证的论坛、社交媒体说法直接替代安全结论。
>
> ---
>
> # 二十三、Production Build Hardening
>
> 建立明确：
>
> ```text
> Development
> Staging/Test
> Production
> ```
>
> 三种配置边界。
>
> Production 必须清理：
>
> * debug UI
> * fake data
> * mock download
> * test command
> * test gateway
> * local dev server
> * remote debugger
> * development browser flags
> * verbose sensitive logging
> * test credentials
> * test URLs
> * sample torrent URLs
> * hidden debug hotkeys
>
> 确认：
>
> ```text
> debug assertions
> test-only feature flags
> dev server dependency
> source map policy
> panic behavior
> logging level
> bundle features
> ```
>
> ---
>
> # 二十四、Release Configuration
>
> 重新审计：
>
> ```text
> tauri.conf.json
> version
> productName
> identifier
> bundle
> resources
> icons
> Windows installer
> updater
> capabilities
> CSP
> externalBin
> plugins
> permissions
> ```
>
> 重点检查：
>
> ```text
> identifier 是否稳定
> version 是否唯一来源
> installer 是否包含测试文件
> resource 是否包含 secret
> bundle 是否包含不必要 binary
> ```
>
> 不允许把：
>
> ```text
> .env
> private key
> debug fixture
> test database
> local download
> ```
>
> 打进 installer。
>
> ---
>
> # 二十五、Windows Installer Security
>
> 至少验证：
>
> ### Clean Install
>
> ```text
> clean machine
> → install
> → first launch
> → create task
> → download
> → playback
> ```
>
> ### Upgrade
>
> ```text
> old version
> → partial task
> → library
> → settings
> → new installer
> → upgrade
> → verify preserved
> ```
>
> ### Uninstall
>
> 验证：
>
> ```text
> application binary
> app data
> database
> downloads
> logs
> user files
> webview data
> ```
>
> 的清理策略。
>
> **绝不能默认删除用户下载文件，除非产品明确设计如此。**
>
> 用户下载数据与 app data 必须明确分离。
>
> ---
>
> # 二十六、Code Signing
>
> M10 必须把 Windows code signing 的发布路径设计清楚。
>
> 如果当前还没有正式 code-signing certificate：
>
> **不要为了完成 M10 去购买证书。**
>
> 应做到：
>
> ```text
> signing strategy documented
> unsigned local build works
> signed build command path defined
> CI secret path defined
> certificate never stored in repository
> release artifact verification procedure documented
> ```
>
> 如果当前已经有签名配置：
>
> 验证：
>
> * signer
> * certificate
> * timestamp
> * SHA-256
> * signature validity
> * installer signature
> * executable signature
> * CI secret handling
>
> 不得声称：
>
> ```text
> “已签名 = 一定不会触发 SmartScreen”
> ```
>
> SmartScreen 声誉仍可能与文件、证书及历史分发建立过程相关。
>
> ---
>
> # 二十七、Updater Security
>
> 如果当前项目已经存在 updater：
>
> 必须检查：
>
> ```text
> update endpoint
> signature verification
> public key
> private key
> manifest
> artifact hash
> downgrade behavior
> version comparison
> transport
> failure recovery
> rollback behavior
> ```
>
> **更新签名私钥绝不进入仓库。**
>
> 如果 updater 尚未进入当前产品范围：
>
> M10 只做：
>
> ```text
> architecture decision
> security requirements
> signing-key management design
> release compatibility review
> ```
>
> 不要因为“发布安全”就擅自加入完整自动更新系统。
>
> ---
>
> # 二十八、Release Artifact Integrity
>
> 最终 release 至少建立：
>
> ```text
> artifact
> SHA-256
> version
> build timestamp
> git commit
> Rust toolchain
> frontend lockfile
> Cargo.lock
> target platform
> signing status
> ```
>
> 形成：
>
> ```text
> Release Manifest
> ```
>
> 但不要把本地用户名、绝对路径、token 等敏感 build metadata 写进公开 release manifest。
>
> ---
>
> # 二十九、Crash / Panic Security Audit
>
> 全项目搜索：
>
> ```text
> unwrap()
> expect()
> panic!()
> unreachable!()
> debug_assert!()
> indexing without bounds check
> integer arithmetic
> unchecked cast
> path conversion
> UTF-8 assumptions
> ```
>
> 注意：
>
> **不是要求机械消灭所有 unwrap。**
>
> 而是分类：
>
> ```text
> impossible invariant
> startup invariant
> external input
> filesystem
> network
> IPC
> database
> media
> torrent
> ```
>
> 任何由：
>
> ```text
> user input
> network
> malformed file
> malformed torrent
> malformed HTTP
> database corruption
> ```
>
> 直接触发的 panic 都必须优先审计。
>
> ---
>
> # 三十、Resource Exhaustion Audit
>
> 安全不仅是“能不能越权”，还包括：
>
> ```text
> memory
> CPU
> threads
> sockets
> disk
> DB writes
> logs
> connections
> HTTP requests
> stream sessions
> torrent peers
> ```
>
> 必须检查：
>
> * unlimited allocation
> * unbounded queues
> * unbounded retry
> * unbounded redirects
> * unlimited range count
> * unlimited task concurrency
> * uncontrolled peer count
> * huge metadata
> * huge headers
> * huge filenames
> * huge JSON
> * huge error messages
> * log flooding
>
> 尽可能建立明确上限。
>
> ---
>
> # 三十一、Security Test Suite
>
> M10 不允许只做静态审计。
>
> 必须增加/完善真正的测试。
>
> ## URL/SSRF
>
> ```text
> valid HTTP
> valid HTTPS
> localhost
> loopback
> private IPv4
> private IPv6
> link-local
> encoded host
> malformed host
> redirect-to-private
> redirect loop
> unsupported scheme
> excessive redirects
> ```
>
> ## Filesystem
>
> ```text
> ..
> ../
> ..\
> absolute path
> UNC
> device path
> reserved name
> long path
> unicode filename
> reparse point
> symlink/junction
> final file collision
> .part collision
> ```
>
> ## Stream Gateway
>
> ```text
> valid session
> invalid token
> expired token
> wrong resource
> wrong task
> wrong session
> invalid Host
> malformed Range
> overflow Range
> out-of-bound Range
> repeated requests
> parallel requests
> destroyed session
> missing resource
> missing file
> file replaced
> file deleted
> ```
>
> ## IPC
>
> ```text
> invalid task id
> invalid resource id
> invalid path
> invalid URL
> invalid enum
> oversize string
> oversize JSON
> unauthorized command
> stale task
> stale session
> deleted resource
> ```
>
> ## Torrent
>
> ```text
> invalid magnet
> malformed torrent
> huge metadata
> invalid internal path
> invalid file name
> cancelled torrent
> destroyed task
> unavailable piece
> peer failure
> tracker failure
> repeated peer failure
> ```
>
> ## Persistence
>
> ```text
> corrupted DB
> missing DB
> invalid migration
> stale task
> missing .part
> stale checkpoint
> final file exists
> .part + final both exist
> invalid resource path
> ```
>
> ---
>
> # 三十二、Fuzz / Property-style Security Testing
>
> 至少考虑对这些边界做 fuzz/property test：
>
> ```text
> URL parser wrapper
> Range parser
> Content-Range parser
> filename sanitizer
> path containment
> TaskState transition
> RecoveryDecision
> PlaybackSession validation
> JSON DTO
> Torrent filename mapping
> ```
>
> 目标：
>
> ```text
> no panic
> no overflow
> no invalid state
> no path escape
> no malformed success
> ```
>
> ---
>
> # 三十三、Security Regression Corpus
>
> 不要只依赖单元测试。
>
> 建立：
>
> ```text
> tests/security/
> ```
>
> 保存：
>
> ```text
> malicious URL cases
> path traversal cases
> malformed range cases
> malformed torrent cases
> malformed IPC cases
> gateway attack cases
> ```
>
> 每修一个真实漏洞或高风险 bug：
>
> ```text
> reproduction test
> fix
> regression test
> documentation
> ```
>
> 必须成为永久回归用例。
>
> ---
>
> # 三十四、Network Surface Inventory
>
> M10 必须生成：
>
> ```text
> docs/M10-network-surface.md
> ```
>
> 列出 Tide 所有可能的网络通信：
>
> | Surface         | Protocol         | Initiator | Default? | Destination    | Purpose   | User Data? | Security |
> | --------------- | ---------------- | --------- | -------- | -------------- | --------- | ---------- | -------- |
> | HTTP Downloader | HTTPS/HTTP       | Tide      |          | user URL       | download  | possible   |          |
> | Torrent         | BitTorrent       | Tide      |          | peers          | torrent   |            |          |
> | Tracker         | HTTP/HTTPS/UDP 等 | library   |          | tracker        | torrent   |            |          |
> | DHT             | UDP              | library   |          | DHT            | discovery |            |          |
> | Stream Gateway  | localhost HTTP   | WebView   |          | local only     | playback  |            |          |
> | Update          | HTTPS            | Tide      |          | release source | update    |            |          |
>
> 以实际代码为准填写。
>
> ---
>
> # 三十五、No-Cloud / No-Telemetry Verification
>
> 如果产品原则仍然是：
>
> ```text
> no cloud backend
> no account
> no telemetry by default
> ```
>
> 必须做真正的运行时验证。
>
> 使用：
>
> ```text
> local firewall
> packet capture
> proxy / test endpoint
> network monitor
> application logs
> ```
>
> 检查启动、空闲、下载、Torrent、播放、退出、更新检查等典型流程。
>
> 输出：
>
> ```text
> Expected network connections
> Observed network connections
> Unexpected connections
> Explanation
> ```
>
> 不允许简单根据源码 grep 得出“没有网络请求”。
>
> ---
>
> # 三十六、Release Environment / CI Security
>
> 审计 CI：
>
> ```text
> GitHub Actions / other CI
> ```
>
> 至少检查：
>
> * workflow permissions
> * token scope
> * secrets
> * signing secrets
> * dependency installation
> * untrusted PR handling
> * artifact upload
> * release publishing
> * cache poisoning considerations
> * dependency pinning
>
> 不要给 CI：
>
> ```text
> write-all
> ```
>
> 这类不必要的权限。
>
> 发布 workflow 与普通 CI workflow 分离。
>
> 普通 PR 不应该拥有 release signing 权限。
>
> ---
>
> # 三十七、Build Reproducibility / Build Traceability
>
> 不要求追求完全 bit-for-bit reproducible build，除非当前项目已经准备这么做。
>
> 但必须做到：
>
> ```text
> build source identifiable
> dependency versions identifiable
> lockfiles fixed
> toolchain identifiable
> commit identifiable
> artifact hash identifiable
> ```
>
> 建议 release build 固定：
>
> ```text
> Rust toolchain
> Node version
> package manager
> Cargo.lock
> package-lock
> Tauri CLI
> ```
>
> 避免：
>
> ```text
> “今天构建出的 Tide 和下周构建出的 Tide 依赖完全不同”
> ```
>
> ---
>
> # 三十八、Versioning Policy
>
> 必须明确：
>
> ```text
> MAJOR
> MINOR
> PATCH
> ```
>
> 或项目实际采用的版本策略。
>
> 至少保证：
>
> ```text
> application version
> package version
> Cargo package version
> installer version
> updater version
> ```
>
> 不出现互相矛盾。
>
> 以后从：
>
> ```text
> M11 RC
> →
> M12 1.0
> ```
>
> 必须可以稳定管理。
>
> ---
>
> # 三十九、安全错误处理
>
> 所有 security-sensitive failure 必须：
>
> ```text
> fail closed
> ```
>
> 例如：
>
> ```text
> invalid token
> → reject
>
> invalid range
> → reject
>
> unsafe path
> → reject
>
> invalid redirect
> → reject
>
> unknown command input
> → reject
>
> malformed torrent
> → reject / structured error
>
> missing security metadata
> → no unsafe fallback
> ```
>
> 不允许：
>
> ```text
> validation failed
> → fallback to unrestricted mode
> ```
>
> ---
>
> # 四十、Security Logging
>
> 安全事件要有结构化日志，但不泄露敏感信息。
>
> 推荐：
>
> ```text
> security.event
> security.category
> resource_id
> task_id
> result
> reason
> timestamp
> ```
>
> 示例：
>
> ```text
> security.path_rejected
> security.gateway_token_invalid
> security.redirect_blocked
> security.invalid_range
> security.command_validation_failed
> ```
>
> 不记录：
>
> ```text
> full token
> full URL
> full local path
> peer IP
> Authorization
> ```
>
> ---
>
> # 四十一、Documentation
>
> M10 至少维护：
>
> ```text
> docs/M10-security-audit.md
> docs/M10-threat-model.md
> docs/M10-network-surface.md
> docs/M10-release-hardening.md
> docs/M10-supply-chain.md
> docs/M10-test-matrix.md
> docs/M10-windows-release.md
> docs/M10-report.md
> ```
>
> 如果项目已有相同用途文档，应优先更新，而不是制造大量重复文档。
>
> 同时检查并更新：
>
> ```text
> README.md
> SECURITY.md
> PRIVACY.md
> CONTENT_POLICY.md
> THIRD_PARTY_NOTICES.md
> ```
>
> ---
>
> # 四十二、Security Severity
>
> 只使用：
>
> ```text
> P0
> P1
> P2
> P3
> ```
>
> 但最终汇报阶段重点只强调：
>
> ```text
> P0
> P1
> ```
>
> ## P0
>
> 例如：
>
> * arbitrary code execution
> * arbitrary file read
> * arbitrary file write
> * arbitrary path escape with realistic exploit
> * Stream Gateway arbitrary filesystem read
> * critical secret exposure
> * unrestricted remote/native command execution
> * production build exposes debug remote control
>
> ## P1
>
> 例如：
>
> * realistic SSRF
> * meaningful local security-boundary bypass
> * unsafe WebView navigation
> * overbroad capability with reachable dangerous command
> * significant resource exhaustion
> * unsafe update verification
> * release secret exposure risk
>
> 不要把普通技术债务都升级成 P1。
>
> ---
>
> # 四十三、Security Decision Table
>
> 必须最终输出一张：
>
> | Area             | Current | Risk | Evidence | Status | Action |
> | ---------------- | ------- | ---- | -------- | ------ | ------ |
> | Tauri Capability |         |      |          |        |        |
> | CSP              |         |      |          |        |        |
> | IPC              |         |      |          |        |        |
> | URL/SSRF         |         |      |          |        |        |
> | Filesystem       |         |      |          |        |        |
> | Torrent          |         |      |          |        |        |
> | Stream Gateway   |         |      |          |        |        |
> | WebView2         |         |      |          |        |        |
> | SQLite           |         |      |          |        |        |
> | Logging          |         |      |          |        |        |
> | Privacy          |         |      |          |        |        |
> | Dependencies     |         |      |          |        |        |
> | License          |         |      |          |        |        |
> | Installer        |         |      |          |        |        |
> | Signing          |         |      |          |        |        |
> | Updater          |         |      |          |        |        |
> | CI/CD            |         |      |          |        |        |
>
> ````
>
> Status 只能使用：
>
> ```text
> RESOLVED
> CONFIRMED
> OPEN
> PARTIALLY VERIFIED
> WORSENED
> NOT APPLICABLE
> ````
>
> ---
>
> # 四十四、M10 实施顺序
>
> 不要随机修改。
>
> 按下面顺序：
>
> ```text
> Phase 1  Audit
> ↓
> Phase 2  Threat Model
> ↓
> Phase 3  Critical Security Fixes
> ↓
> Phase 4  Tauri/IPC/WebView Hardening
> ↓
> Phase 5  Network/SSRF Hardening
> ↓
> Phase 6  Filesystem/Gateway Hardening
> ↓
> Phase 7  Torrent/librqbit Hardening
> ↓
> Phase 8  Logging/Privacy Hardening
> ↓
> Phase 9  Dependency/Supply-chain Audit
> ↓
> Phase 10 Production Build Hardening
> ↓
> Phase 11 Installer/Signing/Updater Preparation
> ↓
> Phase 12 Security Regression
> ↓
> Phase 13 Release Validation
> ↓
> Phase 14 Final Audit Report
> ```
>
> ---
>
> # 四十五、修改策略
>
> 对每一个安全问题：
>
> ```text
> FIND
> ↓
> CLASSIFY
> ↓
> PROVE
> ↓
> FIX
> ↓
> TEST
> ↓
> DOCUMENT
> ```
>
> 不允许：
>
> ```text
> 猜测漏洞
> ↓
> 大规模重构
> ```
>
> 不允许为了“看起来更安全”大规模重写已经稳定的：
>
> * DownloadEngine
> * librqbit bridge
> * Resource
> * Scheduler
> * Playback
> * SQLite
>
> 除非确实存在 P0/P1 安全问题。
>
> ---
>
> # 四十六、避免安全过度工程
>
> Tide 是本地 Windows 应用，不是：
>
> ```text
> multi-tenant server
> banking application
> cloud infrastructure
> hostile public API
> ```
>
> 因此：
>
> 不要为了“理论上更安全”引入：
>
> * 自建认证系统
> * JWT
> * OAuth
> * cloud KMS
> * complex RBAC
> * remote access server
> * zero-trust platform
> * microservices
> * service mesh
> * HSM
> * Kubernetes
>
> 安全方案必须与 Tide 的真实攻击面匹配。
>
> ---
>
> # 四十七、重点验证真实安全边界
>
> M10 必须回答以下问题，而且回答必须基于代码/测试：
>
> ### Q1
>
> 一个恶意 URL 能否让 Tide 访问本机 localhost 服务？
>
> ### Q2
>
> redirect 能否绕过 URL 安全策略？
>
> ### Q3
>
> 恶意文件名能否跳出 download root？
>
> ### Q4
>
> junction/reparse point 能否让文件写到 root 外？
>
> ### Q5
>
> 恶意 Torrent 能否控制输出路径？
>
> ### Q6
>
> 恶意 peer 能否让 Tide 无限建立连接？
>
> ### Q7
>
> 本机另一个程序能否直接读取 Stream Gateway 中任意媒体？
>
> ### Q8
>
> 本机另一个程序能否猜测 PlaybackSession？
>
> ### Q9
>
> Stream Gateway 是否可能被变成任意文件读取服务？
>
> ### Q10
>
> frontend 能否调用不应该开放的 Rust command？
>
> ### Q11
>
> WebView 是否能导航到任意网页？
>
> ### Q12
>
> WebView message 是否能伪造 backend action？
>
> ### Q13
>
> 日志是否会泄露用户 URL / path / token / peer 信息？
>
> ### Q14
>
> production build 是否仍存在 debug endpoint？
>
> ### Q15
>
> production build 是否仍启用了不必要的 Tauri permission？
>
> ### Q16
>
> 是否有依赖存在已知安全漏洞？
>
> ### Q17
>
> 是否存在 license/compliance 问题？
>
> ### Q18
>
> installer 是否会意外删除用户数据？
>
> ### Q19
>
> updater 是否能够验证更新完整性？
>
> ### Q20
>
> signing private key 是否可能进入 Git / artifact / CI log？
>
> ### Q21
>
> 是否存在 default telemetry / cloud upload？
>
> ### Q22
>
> Tide 默认是否向 LAN 暴露 Local Gateway？
>
> ### Q23
>
> Torrent peer listener 是否暴露到 LAN？这是否是当前设计有意为之？
>
> ### Q24
>
> 一个恶意媒体文件能否进入超出预期的 native/backend execution path？
>
> ---
>
> # 四十八、Release Smoke Test
>
> 必须创建接近真实用户环境的 clean-machine test。
>
> 流程：
>
> ```text
> Install
> ↓
> Launch
> ↓
> Settings
> ↓
> HTTP Download
> ↓
> Pause
> ↓
> Restart
> ↓
> Resume
> ↓
> Complete
> ↓
> Playback
> ↓
> Seek
> ↓
> Torrent
> ↓
> Pause
> ↓
> Restart
> ↓
> Resume
> ↓
> Playback
> ↓
> Close
> ↓
> Reopen
> ↓
> Uninstall
> ```
>
> 每一步记录：
>
> ```text
> result
> duration
> logs
> errors
> network
> filesystem
> database
> security anomalies
> ```
>
> ---
>
> # 四十九、Performance Regression
>
> 安全修复不能明显破坏正常使用体验。
>
> 至少测：
>
> ```text
> HTTP throughput
> Torrent throughput
> Stream throughput
> seek latency
> CPU idle
> memory idle
> memory downloading
> memory playing
> DB write frequency
> Gateway overhead
> startup time
> startup recovery
> installer startup
> ```
>
> 不要求 M10 做性能优化。
>
> 只要求：
>
> ```text
> detect regression
> quantify regression
> explain regression
> decide accept/reject
> ```
>
> ---
>
> # 五十、验证命令
>
> 根据项目实际情况运行并记录：
>
> ```bash
> cargo fmt --all -- --check
> cargo check --locked
> cargo test --locked
> cargo clippy --all-targets --all-features
> ```
>
> 如果 workspace 存在：
>
> ```bash
> cargo test --workspace --locked
> cargo clippy --workspace --all-targets --all-features
> ```
>
> 但不要机械因为预存在警告就声称 M10 失败。
>
> 应区分：
>
> ```text
> PRE-EXISTING
> INTRODUCED BY M10
> FIXED
> ACCEPTED
> ```
>
> 依赖安全工具根据环境实际执行：
>
> ```bash
> cargo audit
> cargo deny check
> ```
>
> frontend 根据项目实际执行：
>
> ```bash
> npm ci
> npm audit
> ```
>
> 如果使用其他 package manager，则使用对应 lockfile-safe 的 CI 命令。
>
> 必须保证 release build 使用 lockfile。
>
> ---
>
> # 五十一、不得虚报测试
>
> 严禁写：
>
> ```text
> security verified
> ```
>
> 但实际上只是：
>
> ```text
> code reviewed
> ```
>
> 严禁写：
>
> ```text
> no vulnerabilities
> ```
>
> 如果没有完成完整扫描。
>
> 应准确写：
>
> ```text
> SOURCE REVIEWED
> UNIT TESTED
> INTEGRATION TESTED
> MANUALLY VERIFIED
> TOOL VERIFIED
> NOT VERIFIED
> ```
>
> ---
>
> # 五十二、M10 Definition of Done
>
> M10 完成必须同时满足：
>
> ### Security
>
> * [ ] Threat Model 完成
> * [ ] Attack Surface Inventory 完成
> * [ ] Tauri capability 审计完成
> * [ ] Tauri permission 审计完成
> * [ ] CSP 审计完成
> * [ ] IPC 审计完成
> * [ ] SSRF/Redirect 审计完成
> * [ ] Filesystem 审计完成
> * [ ] Torrent/librqbit 审计完成
> * [ ] Stream Gateway 审计完成
> * [ ] WebView2 审计完成
> * [ ] SQLite 审计完成
> * [ ] Logging/Privacy 审计完成
>
> ### Supply Chain
>
> * [ ] Cargo.lock 已审计
> * [ ] RustSec audit 已执行或明确记录限制
> * [ ] frontend dependency audit 已执行
> * [ ] licenses 已审计
> * [ ] THIRD_PARTY_NOTICES 已更新
> * [ ] 关键第三方版本可追溯
>
> ### Release
>
> * [ ] Production configuration 收口
> * [ ] Debug surface 删除
> * [ ] Installer 验证
> * [ ] Upgrade 验证
> * [ ] Uninstall 验证
> * [ ] User download data preservation 已验证
> * [ ] Signing path 已定义
> * [ ] Signing secrets 不进入 repository
> * [ ] Updater（若存在）验证签名机制
> * [ ] Release artifact SHA-256 可生成
> * [ ] Release metadata 可追溯
>
> ### Testing
>
> * [ ] Security unit tests
> * [ ] Security integration tests
> * [ ] Path traversal tests
> * [ ] SSRF tests
> * [ ] Gateway tests
> * [ ] IPC tests
> * [ ] WebView tests
> * [ ] Torrent malformed-input tests
> * [ ] Resource exhaustion tests
> * [ ] Release smoke test
> * [ ] Clean install test
> * [ ] Upgrade test
> * [ ] Uninstall test
>
> ### Quality
>
> * [ ] `cargo check --locked`
> * [ ] `cargo test --locked`
> * [ ] format/lint validated
> * [ ] release build validated
> * [ ] 无 M10 新增 P0
> * [ ] 无未解释 P1
>
> ---
>
> # 五十三、最终报告必须使用下面结构
>
> 文件：
>
> ```text
> docs/M10-report.md
> ```
>
> 格式：
>
> ```markdown
> # Tide M10 Security & Release Hardening Report
>
> ## 1. Executive Summary
>
> ## 2. FACTS
>
> ## 3. HYPOTHESES
>
> ## 4. INFERENCES
>
> ## 5. Current M0-M9 State
>
> ## 6. Attack Surface Inventory
>
> ## 7. Threat Model
>
> ## 8. Tauri Capability Audit
>
> ## 9. CSP Audit
>
> ## 10. IPC Security
>
> ## 11. HTTP/SSRF Security
>
> ## 12. Filesystem Security
>
> ## 13. Torrent / librqbit Security
>
> ## 14. Stream Gateway Security
>
> ## 15. WebView2 Security
>
> ## 16. Media Security
>
> ## 17. SQLite / Persistence Security
>
> ## 18. Logging / Privacy
>
> ## 19. Dependency Audit
>
> ## 20. License Audit
>
> ## 21. Build Security
>
> ## 22. Installer Security
>
> ## 23. Signing
>
> ## 24. Updater
>
> ## 25. CI/CD Security
>
> ## 26. Security Tests
>
> ## 27. Release Smoke Test
>
> ## 28. Performance Regression
>
> ## 29. RESOLVED
>
> ## 30. CONFIRMED
>
> ## 31. OPEN
>
> ## 32. PARTIALLY VERIFIED
>
> ## 33. WORSENED
>
> ## 34. P0
>
> ## 35. P1
>
> ## 36. Remaining Limitations
>
> ## 37. Changed Files
>
> ## 38. M11 Readiness
>
> ## 39. DoD Checklist
> ```
>
> ---
>
> # 五十四、最终报告的关键原则
>
> 所有安全结论都必须标注证据类型：
>
> ```text
> FACT
> TEST RESULT
> STATIC ANALYSIS
> TOOL RESULT
> MANUAL REVIEW
> ASSUMPTION
> ```
>
> 例如：
>
> ```text
> FACT:
> Gateway binds to 127.0.0.1.
>
> TEST RESULT:
> connection from another machine failed.
>
> INFERENCE:
> current gateway is not remotely reachable under tested configuration.
> ```
>
> 而不是直接写：
>
> ```text
> Gateway is secure.
> ```
>
> ---
>
> # 五十五、M10 与后续 M11/M12 的边界
>
> M10 结束后：
>
> ```text
> M0   Foundation
> M0.5 Technical Spike
> M1   HTTP Download Core
> M2   Persistence / Recovery
> M3   Torrent Engine
> M4   Progressive Streaming
> M5   Player
> M6   Library
> M7   Smart Scheduler
> M8   Windows Integration
> M9   Product Polish
> M10  Security & Release Hardening
> M11  Release Candidate
> M12  1.0
> ```
>
> M10 不应完成 M11/M12 的所有工作。
>
> M10 的成功标准是：
>
> ```text
> Security boundaries are real.
> Production configuration is hardened.
> Dependencies are audited.
> Release process is traceable.
> Installer behavior is verified.
> P0/P1 risks are closed or explicitly documented.
> ```
>
> ---
>
> # 五十六、M11 Readiness Gate
>
> 最终必须明确判断：
>
> ```text
> READY FOR M11
> ```
>
> 或：
>
> ```text
> NOT READY FOR M11
> ```
>
> 如果 NOT READY：
>
> 只列真正阻塞 M11 的：
>
> ```text
> P0
> P1
> critical release defects
> unresolved signing/update integrity problems
> installer/data-loss problems
> security boundary failures
> ```
>
> 不要因为普通 UI polish、低优先级重构、可选优化而阻塞 M11。
>
> ---
>
> # 五十七、M10 的核心验收链
>
> 最终必须能够证明：
>
> ```text
> User Input
> ↓
> Strict Parsing
> ↓
> Backend Validation
> ↓
> Security Policy
> ↓
> Authorized Operation
> ↓
> Constrained Network/File/Process Access
> ↓
> Safe Persistence
> ↓
> Safe Logging
> ↓
> Safe Recovery
> ```
>
> 同时：
>
> ```text
> Development Build
>     ≠
> Production Build
> ```
>
> 并且：
>
> ```text
> Production Artifact
> ↓
> Version
> ↓
> Dependency Lock
> ↓
> Build Identity
> ↓
> SHA-256
> ↓
> Signature / Signing Readiness
> ↓
> Installer Validation
> ```
>
> ---
>
> # 五十八、最终执行要求
>
> 现在开始 M10。
>
> **第一步只做审计，不要直接大改。**
>
> 先完整检查当前仓库并输出：
>
> ```text
> FACTS
> HYPOTHESES
> CURRENT M0-M9 STATE
> ATTACK SURFACE
> THREAT MODEL
> EXISTING CONTROLS
> SECURITY GAPS
> RELEASE GAPS
> P0/P1 CANDIDATES
> M10 PLAN
> ```
>
> 然后再实施。
>
> 实施过程中：
>
> * 每一个安全修复必须有对应测试
> * 每一个 release 改动必须有验证
> * 不做无关大重构
> * 不扩大产品 scope
> * 不引入不必要的新依赖
> * 不引入 cloud backend
> * 不引入 telemetry
> * 不引入账户系统
> * 不把安全结论写得超过证据
> * 不允许测试失败后用“理论上应该没问题”代替
>
> 最终：
>
> ```text
> cargo check --locked
> cargo test --locked
> cargo fmt --all -- --check
> cargo clippy ...
> security tests
> release build
> installer smoke test
> upgrade test
> uninstall test
> dependency audit
> final report
> ```
>
> **完成 M10 后立即停止。**
>
> 不要继续实现 M11 Release Candidate，也不要提前进入 M12 1.0。
>
> ---
>
> ## M10 最终成功条件
>
> > **Tide 已经不仅“能工作”，而且明确知道自己暴露了什么、为什么暴露、谁能够调用、谁不能调用、哪些输入不可信、哪些数据不会离开本机、哪些文件不能被访问、哪些网络请求被禁止、生产构建与开发构建有什么区别，以及最终发布产物如何被验证。**
>
> 这才算 M10 完成。
