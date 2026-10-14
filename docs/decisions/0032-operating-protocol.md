# 0032. 1+N 运行协议与 HANDOFF 退役：backlog 台账与弹性并发窗口

- Status: accepted
- Date: 2026-10-14

## Context

任务 8（[ADR-0031](0031-testing-strategy.md)）拍板时维护者立「1+N 评审经济学」元问题，调研回流（[review-economics-research](../research/review-economics-research.md) RQ7 编排 + RQ8 防膨胀 + 映射建议）确认两点：例外管理通道未制度化是并行扩容的首要判据（缺口 4）；HANDOFF 是无 TTL、只增不减的串行衔接装置（缺口 5），须 transient 化并向结构化状态协议演进。任务 11（1+N 运行协议 + HANDOFF 退役）收官开发流程面：2026-10-14 第三十二会话一批出示 Q1–Q10，维护者对 Q2/Q4/Q5 提异议，改稿四点后全部拍板，并附一条定调（火-4：backlog = 临时方案，长期需要一个工具承担任务状态管理）。任务 10（实现）开工门槛要求「审计流程 ✅ + 开发流程完备 + HANDOFF 迁移完毕」——本 ADR 即该门槛的开发流程与迁移两件。定案十二条与执行包原文当时住 HANDOFF，逐字归档 = [handoff-era-archive](../research/handoff-era-archive.md)（其 §2「任务 11 执行包」）。

## Decision

### 1. 运行协议三件套：教义面 / 状态面 / 入口

- **教义面** = [operating-protocol](../design/operating-protocol.md)（中文 living，read-mostly）：常态循环、弹性并发窗口、变更注入 SOP、评审面纪律；写操作只在拍板点与收官点发生，经直推 main 串行。
- **状态面** = [docs/backlog.md](../backlog.md)（中文，git 追踪，agent 全权维护）：任务队列 + 遗留登记 + 火花区 + 依赖图四位一体；按「一任务一块」切分，同一时刻一个任务块只有一个写者（并行窗口内每 agent 只写自己的任务块），写冲突结构性规避；文档结构性改动视同共享契约串行。
- **入口** = AGENTS.md 完整文档地图（decisions 编号即索引不逐件列；design / research / backlog 逐件一行）+ xtask `agent-check` 孤儿检查（地图未收录的 docs 文档 = CI 失败）。
- **git 追踪原则**（维护者原话）：「我们不能依赖任何非 git 追踪的文件。不然会非常容易出现悬垂指针，导致文档腐烂。」——运行状态 SSOT 一律 git 追踪。
- **外部服务只作对外接口，不作状态 SSOT**：GitHub Issues 回归对外接口本位——不建 label、不把队列迁 issue；外部事项被采纳时才以「链接条目」进 backlog（单写者、无双向同步）。首批出示的 Issues 方案废弃。
- **backlog.md = 临时方案**（维护者定调 = 火-4）：长期需要一个工具承担任务状态管理，而非并发 agent 直接读写该文件——扩展位登记（Consequences）。

### 2. HANDOFF 制度退役

gitignored 会话交接物制度（[ADR-0012](0012-docs-governance.md) 立「易失交接物」定位）撤销：HANDOFF.md 删除，最终版全文逐字归档 = [handoff-era-archive](../research/handoff-era-archive.md)（point-in-time 声明 + 归档正文永不改写）；.gitignore 条目与全部机制触点同批清零（AGENTS.md 四处 / CONTRIBUTING / adding-dependencies skill / explain-diff skill / testing-strategy §6 写实 / 本批 supersede 三条）。会话衔接 = operating-protocol §2 启动 / 收尾序列 + backlog 任务块状态行。

### 3. 协议冻结面原则（细节 SSOT = operating-protocol 对应章）

- **常态工作循环 1+1**：议题出示（actionable 入场测试——每条须要求一个决策或行动，纯通报降级台账不占拍板批）→ 批量拍板 → 固化（living doc + 冻结 ADR）→ `just ci` → self-review → commit+push；会话启动序列（AGENTS.md 地图 → backlog 活跃面 → 涉及 living doc 状态头）与收尾序列（backlog 任务块状态行随当次 commit 入库）；台账摘要义务（议题出示附活跃任务数 / 阻塞链 / 待拍板项）。
- **弹性并发窗口**（升级阶梯废弃）：常态 1+1；开窗闸门 = RQ7 十二前提转写检查单（operating-protocol 附录 A）；窗口形态 = N=2–3、每 agent 独立 worktree、开窗即写明关闭条件（默认 = 本批合并完毕）、待评审堆积即停派新活；关窗回落 1+1，不维持常备并发；窗口遥测回答「哪类窗口开得值」。worktree bootstrap 自动化 = 实现期清单（届时再建）。设计动因 = 火-5。
- **变更注入 SOP**：三杠杆按代价升序 queue（默认，边界处投递）→ interrupt（方向错误）→ rewind（污染落地，先 git commit）；低频、保序、局部（只 replan 受影响 agent）；两陷阱入章——queue 非严格 FIFO（排队消息下个 LLM pause 即注入，可打断在途）、agent→agent 便条静默取消接收方在途 turn（steer / interrupt 必须显式、默认非破坏）。
- **评审面运行纪律五条**：验证声明优先挂 agent 不可伪造锚（CI 运行 URL / commit SHA）；actionable 入场测试；AI 摘要置信闸门（质量不足明示「无法可靠摘要」，不给流畅低质摘要）；非零下限抽查（每实现站维护者全文抽查 ≥ 1 个 commit 原始 diff，初值一站一件）；校准配额立原则不钉参数（每实现站维护者亲手实现 / 调试至少一个片段；抽样框独立化 = 扩展位）。

### 4. 演进纪律

additive 增补落 [operating-protocol](../design/operating-protocol.md) 修订注（living 面）；破坏式变更（原则改写、流程废除、台账制度替换）走新 ADR supersede。backlog.md 内容面 = 活状态，任务块日常变动不走 ADR；其格式与读写纪律的结构性改动视同共享契约，串行进行。

### 对既有 ADR 的局部 supersede（同批吸收）

各旧 ADR 正文冻结不动、Status 行记录本条；以下逐条命名被取代的确切条款：

1. **[ADR-0012](0012-docs-governance.md)** Decision 分类学格「`HANDOFF.md` 在仓库根、gitignored、易失」半格——supersede：HANDOFF 制度退役（本 ADR 条款 2）；分类学新增 `docs/backlog.md` 类（任务队列 + 遗留登记 + 火花区 + 依赖图四位一体；git 追踪；agent 全权维护；临时方案——工具化 = 扩展位）；decisions / design / research 三类原位。
2. **[ADR-0012](0012-docs-governance.md)** Consequences 末句「HANDOFF 的交接纪律：新会话先读HANDOFF，工作结束时重写。」——supersede = 会话衔接纪律移交 [operating-protocol](../design/operating-protocol.md) §2（启动 = AGENTS.md 地图 → backlog 活跃面 → 涉及 living doc 状态头；收尾 = backlog 任务块状态行随当次 commit 入库）。
3. **[ADR-0030](0030-engineering-phase-2.md)** 条款 4 常态面三件套之「HANDOFF 收工记录」件——supersede = 「backlog 任务块状态行收尾」；其余两件（议题拍板制 + `self-review` skill）原位。

## Consequences

- **任务 10 开工门槛自此成立（规则入库）**：门槛组件 = 审计流程 ✅（[ADR-0031](0031-testing-strategy.md)）+ 开发流程 ✅（本 ADR + operating-protocol，随收官 commit 完备）+ HANDOFF 迁移 ✅（同收官 commit）；门槛全备 ≠ 即刻开工——队列面 T10 另挂 T12 顺序依赖（[docs/backlog.md](../backlog.md) 依赖图），T12 收官后 T10 方可开工。
- 扩展位三条：台账工具化（火-4；消费点 = 任务系统工具化立项）/ dashboard 正式形态（RQ7 前提 #12 目标级语义摘要；现状 = backlog 活跃面 + 台账摘要义务）/ 抽样框独立化（触发器 = 实现期代码上量后按缺陷逃逸率先验分层再立项）。
- 实现期清单登记：worktree bootstrap 自动化（开窗闸门第 4 条；届时再建）。
- **首创面如实登记**：弹性并发窗口模型（开窗闸门 + 关窗回落不维持常备并发 + 遥测回答「哪类窗口开得值」）= RQ7 容量阶梯与前提清单之上的自研运行形态，公开文献无具名先例（设计动因 = 火-5 维护者观察）；有效性以窗口遥测验证后再硬化。
- 同批更新：[ADR-0012](0012-docs-governance.md) / [ADR-0030](0030-engineering-phase-2.md) Status 行记录批次条目号；AGENTS.md（NEVER 删 HANDOFF 行 / spark 条改指 backlog 火花区 / Documentation 升级完整文档地图 / Reporting 加不可伪造锚与 actionable 入场测试两条）；CONTRIBUTING、.gitignore、adding-dependencies skill（roadmap / handoff 指针改 backlog）、explain-diff skill（置信闸门条）、[testing-strategy](../design/testing-strategy.md) §6 定稿记录写实；xtask `agent-check` 扩展孤儿检查；HANDOFF.md 删除并归档（本批唯一不可逆操作）。
