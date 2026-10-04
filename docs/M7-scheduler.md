# M7 Scheduler Architecture

Answers §155 (the twelve questions every scheduler doc must answer)
and §156 (ADR table). The full milestone results live in M7-report.md.

## Why a scheduler at all?

"用户正在看电影第 42 分钟" and "用户在下载 10GB" are both downloads,
but their urgency is worlds apart. Before M7 the engines raced for
bandwidth uncoordinated. The scheduler is Tide's resource-allocation
BRAIN (§171): it answers "此刻最应该先拿哪些数据、给哪个来源多少资源"
— and NOTHING else (§157: protocol/peers/files/codec stay in the
engines, storage, gateway and player).

## The twelve answers (§155)

| Question | Answer |
|---|---|
| 为什么需要 Scheduler | Playback-aware allocation: 边播边下时播放优先、健康时让出、seek 时转向（§171） |
| Demand 是什么 | 一段"谁需要哪个字节范围、多急"的意图（§5）；运行时另有 M4 的 per-chunk DemandScheduler 做登记/取消 |
| Availability 是什么 | 已持久化可读的字节集合；Demand − Availability = Missing = 需要调度的部分（§10；RangeSet::missing_from） |
| Scheduler 如何计算 | 纯函数 `compute(snapshot, config)` 四阶段分配（§55），确定性（§51） |
| Playback 如何优先 | 水位状态机（§27–31）+ Emergency 最低配额（Phase B）+ target 补足 + CRITICAL 压力递升至 4× target（§31/§92），迟滞退出（§31 recovery） |
| Fairness 如何实现 | 剩余容量按有效权重比例分配（§20）；同类等权 → 等份额；§95 健康时 background 保底份额 |
| Aging 如何实现 | 等待越久权重越高，上限 +0.5（§21 cap）— 永远追不上 Emergency（§110 测试） |
| Bandwidth 如何限制 | 全局 cap（用户可设，§24）在分配阶段被硬约束（sum ≤ cap，§111/§114 property）；HTTP per-task token bucket + Torrent 会话级 rqbit 限速真实执行（§25） |
| HTTP 如何适配 | HttpAdapter（§81）：per-task 带宽 = yes；range 意图由 M4 的 read-through 天然服务 |
| Torrent 如何适配 | TorrentAdapter（§82/§12）：会话级限速（rqbit 公开 ratelimits API）；piece 选择仍归 rqbit（§35） |
| Seek 如何处理 | 位置上报节流合并（§33）；SEEKING → Emergency 提权 + M4 notify_seek 取消旧 demand（§32/§80）；旧范围没有"残留权重"可与新范围竞争 |
| Restart 如何恢复 | 运行时状态不持久化（§46/§47）：重启后由 provider 重新收集 snapshot、runtime 从零重建分配（§48/§129 测试） |

## ADR table (§156)

| Decision | FACT / DECISION / RATIONALE / TRADE-OFF |
|---|---|
| 单线程 owner | DECISION: runtime 持有全部状态，tick 单写者（§136 单写者模型）。RATIONALE: 可推理、无锁层级。TRADE-OFF: tick 串行 — 500ms 预算内绰绰有余（实测 1000 任务 < 1ms） |
| Tick 频率 | DECISION: 500ms + 事件快速通道（§18）。RATIONALE: 调度不是渲染循环。TRADE-OFF: 状态变化最多 500ms 后才反映 |
| Event trigger | DECISION: AtomicBool trigger，tick 顶部消耗（§49）。TRADE-OFF: 事件合并而非逐个响应 |
| Priority 模型 | DECISION: 语义类 + 可分解权重（base × user + aging，§7/§22）。TRADE-OFF: 不是单一 score，输出 reason 分解（§16） |
| Fairness 算法 | DECISION: 剩余容量按权重比例（WFQ 简化形，§20）。TRADE-OFF: 不做 DRR/virtual-finish — 等权场景等价，实现可测 |
| Aging | DECISION: 线性增长、0.5 封顶、仅 Normal/Background（§21）。TRADE-OFF: 长饿背景只能涨到 Normal 一半 |
| Bandwidth allocation | DECISION: 全局 cap 硬约束 + min/target 阶梯 + CRITICAL 4× 压力上限（§26/§31）。TRADE-OFF: 超额压力配额可能剩余不足（cap 内重分配） |
| Playback buffer thresholds | FACT(初始): 4/12/30/10 秒 — INITIAL DEFAULTS 待真网调优（§74）。TRADE-OFF: 假设流式消费速率 ≈ 码率 |
| HTTP adapter capability | DECISION: per-task 带宽 yes / range yes（read-through 已有）。FACT: token bucket 实测生效 |
| Torrent adapter capability | FACT: rqbit 公开 API 仅会话级限速。DECISION: 会话级 + piece 选择归 rqbit。TRADE-OFF: per-task 精度不可得（§12 禁 fork） |
| 持久化 runtime | DECISION: 否 — 仅用户优先级与带宽上限入 settings KV（§46/§47/§72）。RATIONALE: 运行时状态可重建 |

## 职责边界（§157）

Scheduler 只做 Demand / Priority / Allocation / Coordination。HTTP
协议、Range、retry、resume → HTTP Engine；peer/piece/tracker/DHT →
librqbit；文件写入与原子改名 → Storage；HTTP Range 与安全 → Stream
Gateway；codec 与播放 → Player。
