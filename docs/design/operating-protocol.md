# Metis — 1+N 运行协议

> 状态：✅ 冻结面 = [ADR-0032](../decisions/0032-operating-protocol.md)（2026-10-14）；本文档为 living doc（教义面，read-mostly）。演进纪律：additive 增补（实践澄清、检查单细化、新扩展位登记）落本文档修订注；破坏式变更（原则改写、流程废除、台账制度替换）走新 ADR supersede。
> 运行状态不住本文档——活跃任务、阻塞链、火花一律住 [docs/backlog.md](../backlog.md)（状态面 SSOT，git 追踪）。

## 1. 定位与原则

Metis 的开发模式 = 1+N：一个维护者 + N 个 agent，agent 承担绝大多数代码、测试与文档写作，维护者负责拍板、抽查与纠偏。本文档是该模式的运行协议：§2 常态循环、§3 并发窗口、§4 变更注入、§5 评审面纪律。核心矛盾 = 维护者注意力是唯一不可扩展的资源（评审经济学五条教义 = [ADR-0031](../decisions/0031-testing-strategy.md)），本协议全部机制从该矛盾推出。

四条明文原则：

1. **教义/状态两面分离**：本协议 = 教义面，read-mostly——写操作只在拍板点与收官点发生，且经直推 main 串行；运行状态不住协议 doc（住 backlog）。
2. **git 追踪原则**（维护者原话）：「我们不能依赖任何非 git 追踪的文件。不然会非常容易出现悬垂指针，导致文档腐烂。」——运行状态 SSOT 一律 git 追踪；gitignored 会话交接物（HANDOFF）因此退役（[ADR-0032](../decisions/0032-operating-protocol.md) 条款 2）。
3. **外部服务只作对外接口，不作状态 SSOT**：GitHub Issues = 对外接口本位——不建 label、不把队列迁 issue；外部事项被采纳时才以「链接条目」进 backlog（单写者、无双向同步）。
4. **写冲突结构性规避**：backlog 按「一任务一块」切分，并行窗口内每 agent 只写自己的任务块——同一时刻一个任务块只有一个写者；文档（含 backlog 自身）的结构性改动视同共享契约，串行进行（接 §3 闸门第 2 条）。

## 2. 常态工作循环（1+1）

同一时刻一个 agent 会话；跨会话衔接靠 git 追踪面（backlog + 本协议），不靠会话外记忆。

**循环**（每任务 / 每议题批）：

1. **议题出示**——每条议题须过 **actionable 入场测试**：要求维护者一个决策或一个行动；纯通报降级为台账记录，不占拍板批。出示格式 = 先背景再取舍分析；术语首次出现就地一句话解释；小决策点合并成批一次拍板。
2. **批量拍板**——维护者没异议的部分不重述，只讲异议。
3. **固化**——living doc 修订 + 冻结 ADR（决策点确认即晋升）。
4. **`just ci` 全绿**——唯一质量门禁入口，不许拼临时管线绕过。
5. **self-review**——作者机械 sweep + 全新子代理判断 pass（作者盲区是结构性的）。
6. **commit + push**——无 PR 流程直推 main，每条 commit message 必须可落地。

**会话启动序列**：读 AGENTS.md 文档地图 → [docs/backlog.md](../backlog.md) 活跃面（任务块 + 阻塞链 + 火花区）→ 任务涉及的 living doc 状态头。

**会话收尾序列**：更新 backlog 任务块状态行——状态词 + 已完成范围 + 下一步 + 未拍板项 + 评审面三数（拍板决策点数 / self-review findings 数 / 评审耗时量级；诊断位，不设门禁）——随当次 commit 入库。

**台账摘要义务**：议题出示时附活跃任务数 / 阻塞链 / 待拍板项，让维护者零成本对账；摘要是呈现层，SSOT 仍是 backlog 本体。

## 3. 弹性并发窗口

并发不是常备形态而是按需开启的窗口：常态 1+1，发现可并发批或工期紧迫时开窗，本批合并完毕即关窗回落 1+1。设计动因 = 火-5（维护者观察：并发需求是长期的，但大量并发需求是偶然且短期的——逐字见 backlog 火花区）。升级阶梯（逐级扩容并常驻）废弃。

**开窗闸门** = 附录 A 检查单全过，要点四条：

1. 本批任务过切分三属性——independent（各自拥有文件集）/ bounded（一个 PR 的量；本仓无 PR 流 = 一个收官 commit 量级）/ verifiable（可测试的完成条件）；
2. 无共享契约交叉——schema、公共类型、文档结构、门禁面属共享契约，此类改动一律串行：先单独跑、合并、再从新 base fan out；
3. 维护者评审预算有余量——评审带宽是唯一天花板，N 由每会话可认真评审的批次数决定，不由机器闲忙决定；
4. worktree / bootstrap 就绪——每 agent 独立 worktree，依赖、env、端口一键就绪；bootstrap 自动化 = 实现期清单（[ADR-0032](../decisions/0032-operating-protocol.md) Consequences 登记，届时再建）。

**窗口形态**：N = 2–3；每 agent 独立 worktree；开窗即写明关闭条件（默认 = 本批全部合并完毕）；待评审堆积即停止派新活（WIP limit——未评审产出是库存不是进度）。

**窗口遥测**：哪类任务开了窗、评审面三数、合并冲突实录，沉淀进对应任务块——回答「哪类窗口开得值」，为后续开窗决策供证据。

## 4. 变更注入 SOP

开发中途维护者注入变更（新需求 / 方向修正 / 叫停）的处置纪律。干预杠杆按代价升序：

1. **queue**（默认）——排队到边界处投递，保留在途工作；新需求一律走 queue。
2. **interrupt**——方向错误时停当前步，会话存活。
3. **rewind**——污染已落地时回滚 checkpoint；**改向前先 git commit**（rewind 不跟踪 shell 副作用）。

纪律三条：**低频、保序、局部**——只 replan 受影响的 agent；真实代价是重新协调与重新获得维护者的监督上下文。同一问题两次纠偏不生效即作废重开。

两个工具层陷阱（必须知道）：

- queue 并非严格 FIFO：排队消息在下一个 LLM pause 即注入，可能打断在途工作。
- agent → agent 的「留便条」消息会静默取消接收方的在途 turn（作废的 token 无任何提示）——跨 agent 的 steer / interrupt 必须显式，默认非破坏。

## 5. 评审面运行纪律

[ADR-0031](../decisions/0031-testing-strategy.md) 教义五条的运行面落实（依据回溯见该 ADR）：

1. **不可伪造锚**：验证声明优先挂 agent 不可伪造的独立证据（CI 运行 URL / commit SHA）；自述不算证据（AGENTS.md Reporting 同条）。
2. **actionable 入场测试**：见 §2 循环第 1 步。
3. **AI 摘要置信闸门**：摘要 / 导览类产出在质量不足时明示「无法可靠摘要」，不给流畅的低质摘要（explain-diff skill 同条）。
4. **非零下限抽查**：每实现站维护者全文抽查 ≥ 1 个 commit 的原始 diff（ISA 330 非零下限的 1+1 版；初值 = 一站一件，可按缺陷逃逸率上调）。
5. **校准配额**：立原则不钉参数——每实现站维护者亲手实现 / 调试至少一个片段（批准式工作是退化条件；配额占用产出时间是明示代价）。抽样框独立化 = 扩展位（§6）。

## 6. 扩展位登记

- **台账工具化**（火-4）：backlog.md 是临时方案；长期需要一个工具承担任务状态管理，而非并发 agent 直接读写该文件。消费点 = 任务系统工具化立项。
- **dashboard 正式形态**：目标级语义摘要 dashboard（RQ7 前提 #12：显示工程决定监督容量上限）；现状 = backlog 活跃面 + 台账摘要义务。
- **抽样框独立化**：抽样框与议题生成权和被审者分离（[ADR-0031](../decisions/0031-testing-strategy.md) 教义 3）；触发器 = 实现期代码上量后按缺陷逃逸率先验分层，再立项。

## 附录 A 开窗检查单（RQ7 十二前提转写）

来源 = [review-economics-research](../research/review-economics-research.md) RQ7 章前提条件候选清单（12 条；依据回溯见原表）。开窗前逐条过；「仓内映射」列记录现状，「验收方式」列记录该条如何判过。

| #  | 前提                                        | 仓内映射（2026-10-14 现状）                                                                                    | 验收方式                                                  |
| -- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| 1  | 单 agent 工作流已稳定产出价值               | 任务 4–8 五站 1+1 收官（ADR-0027～0031）                                                                       | 既有事实；开窗时复核无回退                                |
| 2  | backlog 可分解性盘点通过                    | 开窗前对本批任务过切分三属性（independent / bounded / verifiable）                                             | 检查单逐项通过记录进任务块                                |
| 3  | 共享契约类任务有串行化通道                  | §1 原则 4 + §3 闸门第 2 条                                                                                     | 开窗记录确认本批无共享契约交叉                            |
| 4  | worktree bootstrap 自动化                   | 现状 = 手工；自动化 = 实现期清单（届时再建）                                                                   | 开窗前每 worktree 依赖 / env / 端口就绪（手工清单或脚本） |
| 5  | 任务卡片含明确边界与 effort 档              | backlog 任务块（状态词 / 依赖 / 边界 / 完成条件）                                                              | 本批任务块字段完整                                        |
| 6  | 评审容量先行（N 由评审带宽决定，初始 N=2）  | §3 窗口形态 N=2–3 + 评审面三数遥测                                                                             | 维护者确认评审预算余量                                    |
| 7  | WIP limit（待评审堆积即停派新活）           | §3 窗口形态                                                                                                    | 窗口关闭条件明写                                          |
| 8  | 验证栈分层建成                              | `just ci`（机器门禁）→ self-review 子代理判断 pass（对抗评审）→ 维护者拍板 / 抽查（风险分级签批）              | 各 agent 产出过同一套                                     |
| 9  | 变更注入 SOP 与 queue 时机实测              | §4 在册                                                                                                        | 章在册；首次窗口实测 queue 行为后记一笔                   |
| 10 | token 预算按约 15× 估，高价值任务才 fan-out | 开窗决策写明本批价值                                                                                           | 开窗理由记录进任务块（遥测「哪类窗口开得值」）            |
| 11 | 维护者角色 = 分解任务 + 写 spec + 验收 diff | §2 常态循环（维护者拍板 / 抽查 / 纠偏）                                                                        | 协议在册                                                  |
| 12 | 状态协议 / 异常通道制度化                   | [docs/backlog.md](../backlog.md)（结构化状态协议临时形态；工具化 = §6 扩展位）；dashboard 正式形态 = §6 扩展位 | backlog 活跃面可用、摘要义务执行                          |

## 附录 B 外部关键参考

- 调研实体（本地）：DeepSeek Harness = `/home/duskgrow/project/demo/deepseek-harness`（vendored cordis 源码 = `vendor/cordis`；cordis.js.org 文档站抓不到内容，实现细节以 vendored 源码为准）；兄弟项目 Cadmus = `/home/duskgrow/project/rust/Cadmus`；工程化模板 = `../rust-template`。
- 上游参考：Harness = `https://github.com/deepseek-ai/deepseek-harness`；Cordis 上游 = `https://github.com/cordiverse/cordis`；论文 PDF = `https://arxiv.org/pdf/2608.25512`（只能 PDF 提取，定理编号可能有 ±1 偏差）；Luau RFC 仓 = `https://github.com/luau-lang/rfcs`。
- 托管面：org `metis-mind` 双仓——`metis`（public）+ `marketplace`（空壳占位）；remote = `git@github.com:metis-mind/metis.git`；git 身份 = `duskgrow <git@duskgrow.dev>`（repo-local）；gh CLI 已认证。
- 吸收的外部资源：Fenng/Tech-Doc-Style-Chinese（MIT → chinese-doc-style skill）；dualface/ste-zh（MIT → AGENTS.md Reporting 节）；geoffreylitt explain-diff gist（无 license → explain-diff skill）。
