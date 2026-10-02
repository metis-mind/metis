# Journal/Replay（全程记录与可重放）领域调研

> 调研日期：2026-10-02
> 调研目的：为 Metis 的 journal/replay 设计提供工业参照。需求（已定全做）：**a** 审计回看 / **b** 崩溃恢复 / **c** 确定性重放（LLM 原始响应必须录制，模型不可重调）/ **d** 从历史点 fork 分支。设计约束见 [ADR-0011](../decisions/0011-actor-execution-model.md)（actor 模型、host 星型路由）、[ADR-0014](../decisions/0014-dispatch-semantics.md)、[ADR-0015](../decisions/0015-payload-value-model.md)、[fiber 设计](../design/fiber.md)（保守重启哲学）。
> 核实约定：✅ = 本次经 fetch 工具核对一手来源；🧭 = 作者推断（含类比推理）；❓ = 未联网核实，仅按既有知识陈述。本文是 point-in-time 报告：更正走 addendum，不改结论。

---

## 第〇部分：结论速览

1. **"记边界输入、重算内部推导"是 durable execution 的工业共识形态**（✅ Temporal / Restate / DBOS 三家一致）。分歧只在"边界"画在哪：Temporal 画在 Activity/Timer/Signal，DBOS 画在 step，Restate 画在 journal 化的每个副作用调用。我们的初步判断①成立，但"边界"必须精确枚举：入邮箱消息、LLM 响应、定时器触发、随机/时间读取——少一类 replay 就发散。
2. **确定性的定义是行为性的而非代码性的**：Temporal 表述为"相同输入下产生相同的 SDK API 调用序列"（✅）。这给我们一个可执行的判定标准：replay 时插件向 host 发出的调用（派发/服务调用/effect 注册）序列必须与录制期一致，不一致 = nondeterminism error，响亮失败而非静默采信（对初步判断①的重要修正：重算结果必须与录制比对，不能盲信）。
3. **LLM 调用录制有 20 年前的直接先例**：Fowler 2005 年 Event Sourcing 原文的"LoggedPricingGateway"模式（外部查询经 gateway 录制响应、replay 时重放）（✅）；Temporal 官方文档更把 "LLM/AI invocations" 列为必须放进 Activity 的首例（✅）。需求 c 在理论上毫无新意，工程上照做即可。
4. **从历史点 fork 有三个工业先例**：Temporal 的 workflow reset（`fork_event_version` + `base_run_id`/`new_run_id`，新 run 复制历史前缀）（✅）；Claude Code 的 `--resume --fork-session`（沿用 transcript、新 session ID）（✅）；git 本身（Aider 把 git 当账本，`/undo` = revert）（✅）。需求 d 的实现可以极简：fork = 引用或复制 journal 前缀 + 新 session 从该 seq 继续追加（物理组织留作开放问题，见第七部分 §4）。
5. **代码版本化是 replay 的第一难题，工业答案是"pinning 优于 patching"**：Temporal 的 GetVersion/patch API 被自家文档标注为次选，Worker Versioning（按部署版本 pin worker）才是推荐路径（✅）；Orleans 的 LogStorage provider 则演示了另一极——事件全量可反序列化时，代码可以"激进重构"（✅）。映射到我们：录制 plugin/preset revision，replay 默认 pin 录制期代码。
6. **快照的前提是状态可序列化；我们的 fiber 哲学把这个问题消掉了**。Akka/Orleans 的快照都假设 actor 状态可落盘（✅）；Metis 插件状态 = `(config, 输入序列)` 的纯函数（保守重启、无状态迁移，见 [fiber.md](../design/fiber.md) §5），无法也无须序列化 lua_State——快照只需覆盖 host 侧路由表，插件侧老老实实重放（🧭）。
7. **agent 领域的事实标准已经收敛成"append-only JSONL transcript + 显式 session id + resume/fork CLI"**（✅ Claude Code；🧭 对 Aider 等同类的归纳）。观测面的事实是标准 OTel GenAI semconv（✅），其中 `gen_ai.prompt.name/version`、`gen_ai.conversation.compacted`、内容捕获三档（不录/录属性/外部存储+引用）可直接抄。
8. **落笔脱敏是 VCR 系工具的标准特性**（✅ vcrpy：`filter_headers`、`before_record_request/response` 钩子、可调用替换值），OTel 更激进：消息内容默认不录、opt-in 才录（✅）。初步判断③成立，且有现成钩子形态可借鉴。
9. **单点路由器的全序红利超出预期**：Temporal 的因果链靠事件上的反向 ID 引用（`scheduled_event_id`、`workflow_task_completed_event_id`）（✅）；我们的 host 作为唯一定序点，可以给每条目发全局 seq，因果链 = 一个 `cause: seq` 字段，vector clock 确认不需要（🧭 由 ✅ 事实类比推出）。
10. **不可信插件参与 replay 的信任边界只能画在内核**：所有工业系统的确定性保证都由框架层强制（Temporal 比对 command 序列、Akka 单写者、FDB 全虚拟化），无一依赖业务代码自觉（✅）。模型写的插件同理：replay 正确性由 seam 注入 + 邮箱串行 + VM 边界纯数据 + 发散检测四件事结构性保证（🧭）。

---

## 第一部分：Durable execution 框架

### 1. Temporal：Event History 的确切结构（✅）

Temporal 的 Event History 是 append-only 事件序列，事件由 Temporal Service 响应外部发生与 workflow 产出的 Command 而创建。全部条目类型（官方 events reference 枚举，约 50 种）可按用途分六族：

| 族 | 条目类型（节选） | 对我们的意义 |
| -- | --------------- | ------------ |
| 生命周期 | `WorkflowExecutionStarted/Completed/Failed/TimedOut/Canceled/Signaled/Terminated/ContinuedAsNew/OptionsUpdated` | session/fiber 生命周期事件族 |
| 任务机械 | `WorkflowTaskScheduled/Started/Completed/TimedOut/Failed` | replay 不消费、audit 消费——证明"journal 可以包含 replay 不用的条目" |
| 边界副作用（Activity） | `ActivityTaskScheduled/Started/Completed/Failed/TimedOut/CancelRequested/Canceled` | **LLM 调用的对应物**：`ActivityTaskCompleted` 携带 `result`（录制点） |
| 定时器 | `TimerStarted/Fired/Canceled` | 定时器是边界输入：`TimerFired` 入史，replay 按史注入而非按墙钟 |
| 标记 | `MarkerRecorded`（对 server 透明，SDK 用于 local activity 与 side effect） | "内核哑存储 + SDK 策略"的直接证据——初步判断②的先例 |
| 跨执行 | 子 workflow 6 种 / 外部 signal·cancel 4 种 / Update 2 种 / Nexus 7 种 | 跨 session 交互的条目形态 |

两个结构性细节值得抄：

- **因果链靠反向 ID 引用**：几乎每个条目带 `scheduled_event_id` / `started_event_id` / `workflow_task_completed_event_id`，指回触发它的条目。没有 vector clock，因为历史本身是单序列。
- **失败也是条目**：`ActivityTaskFailed` 携带 `failure` 与 `retry_state`；`WorkflowTaskFailed` 的注释明言"通常意味着 workflow 代码非确定"——发散被记录为事实，不是隐藏状态。

### 2. Replay 语义与确定性约束（✅）

- **Replay = 重跑 workflow 代码，把生成的 Command 序列与既有 Event History 逐一比对**；匹配则推进，不匹配则 nondeterminism error。workflow 代码每次重跑都从空状态开始，靠 Event History 中已录制的 Activity 结果"注入"推进——activity 结果不重算、timer 不真等、signal 按史重放。
- **确定性的官方表述**："any time your Workflow code is executed it makes the same Workflow API calls in the same sequence, given the same input"。约束对象是**调用序列**，不是内部变量——内部随便算，出口必须一致。
- **intrinsic non-determinism**（内禀非确定，如内联 `local_clock()` 分支）的处理：SDK 提供 replay-safe 的时间/随机 API，其结果**存入 Event History**，重跑时取录制值。这正是"时间/随机注入"在框架层的同构物——我们的构造注入纪律（AGENTS.md Style）是同一思想的前置静态版。
- **Awaitable 白名单**：workflow 只能阻塞在 SDK 提供的 Awaitable 上（activity 结果、timer、子 workflow、signal 确认）——副作用边界用类型/API 白名单画，不靠自觉。

### 3. 版本化：patching vs Worker Versioning（✅）

workflow 代码变了怎么办，Temporal 给三条路，优先级有官方排序：

1. **Worker Versioning（官方推荐）**：worker 打上部署版本标签，旧 worker 跑旧代码、新 worker 跑新代码，运行中的执行被 pin 在旧版本上。错误率更低（官方原话 "users see improved error rates"）。
2. **Patching（`GetVersion(changeId, min, max)`）**：代码内打版本分支，首次执行时在历史中写 `MarkerRecorded`，之后该执行永远走录制版本的分支；旧版本全部退出 retention 后才能删分支。长期运行系统会累积大量分支，"challenging to manage"（官方原话）。
3. **Cutover**：整个 workflow 改名（`PizzaWorkflowV2`），新旧并存注册。简单但只管新执行。

补丁机制对我们的启示不是抄 API，而是它的**分类学**：改动分为"安全改动"（不动 command 序列：改 activity 参数、超时、timer 时长）与"破坏改动"（增删重排任何产 Command 的调用）。DBOS 的表述一模一样："breaking change = any change in what steps run or the order in which steps run"（✅）。**这给了我们插件版本兼容性判定标准**（见第六部分）。

### 4. Reset：从历史点 fork 的工业先例（✅）

Temporal 的 workflow reset 在事件结构里留下直接证据：`WorkflowTaskFailed` 条目带 `base_run_id` / `new_run_id` / `fork_event_version`（"Identifies the Event version that was forked off to the reset Workflow"）。语义 = 从历史的某个事件版本切开，新 run 继承前缀、从该点重跑。这就是需求 d 在 durable execution 世界的标准答案：**fork 不是修改历史，是引用历史前缀开新分支**。

### 5. Restate 与 DBOS：同一模型的两个变体（✅）

**Restate**：server 作为反向代理坐在服务前面，**journal 记录每个副作用操作及其结果**；崩溃重放时跳过已完成步骤、从断点续跑。两个独特点：

- **内嵌 KV store 与 journal 一体化**：状态更新与执行步骤记入同一 journal，"state is never out of sync with the execution"，单写者 per Virtual Object。这是"派生状态与事实日志必须同事务"的工业确认。
- **幂等键去重**：入口请求可带 idempotency key，重复请求返回原结果——崩溃重试不产生重复调用。

**DBOS**：无独立服务器，**checkpoint 进 Postgres**：每 step 一次写（录制输出）+ 每 workflow 两次写（输入、结果）。恢复三步：找 PENDING workflow → 用录制的输入重跑 → 每个 step 先查 checkpoint，有则直接返回录制值，第一个无 checkpoint 的 step 就是断点。要求：**workflow 确定性 + step 幂等**——"once a step completes and is checkpointed, it is never re-executed"。单库 >40K step/s（官方基准，✅）。

三家对照给我们的形态确认（🧭）：journal 的写入量级 = 边界事件数，不是内部消息数。我们的内部流量（事件派发）全量进 audit 是可行奢侈，因为 replay 只依赖边界子集——两种消费可以共享一条流，用条目类型过滤。

---

## 第二部分：Actor 持久化

### 1. Akka Persistence：event-sourced actor 的完备形态（✅）

`EventSourcedBehavior` 四件套：`persistenceId`（稳定 ID）、`emptyState`、`commandHandler (state, cmd) -> Effect`、`eventHandler (state, evt) -> newState`。要点：

- **command 不持久化，event 才持久化**。command 先经校验，产出 event 落盘后才改状态；replay 只重放 event——"events cannot fail when being replayed"（校验已在 command 时做过）。恢复与正常运行复用同一个 `eventHandler`。
- **Effect 代数**：`persist`（多事件原子写：全存或全不存）、`none`、`unhandled`、`stash`/`thenUnstashAll`、`reply`；`thenRun` 挂持久化成功后的副作用。
- **副作用语义诚实**：`thenRun` 是 **at-most-once**（持久化失败不执行），且**重启/replay 时不重跑**；文档明言若在 `RecoveryCompleted` 里补做未确认的副作用，可能执行超过一次——即框架层给的是 at-most-once + 业务层补票 = effectively-once，不存在免费的 exactly-once。
- **单写者原则**：同一 `persistenceId` 同时只允许一个活跃实例（Cluster Sharding 强制）；多写者交错写入会破坏 replay，为此提供 replay filter（按 `writerUuid` 甄别，模式：`repair-by-discard-old` / `fail` / `warn` / `off`）。**我们的 host 星型拓扑在结构上就是单写者**（🧭 由 ✅ 类比：一个 host = 全系统唯一 sequencer，比 Akka 的按实体单写者更强）。
- **恢复期的邮箱语义**：恢复期间到达的新消息被 stash，恢复完成后才投递；persist 进行中也 stash。对应我们的邮箱串行 + Unloading 拒绝新账（[fiber.md](../design/fiber.md) §2）——恢复是一种特殊的串行区间。
- **快照**：`snapshotWhen` 谓词或 `RetentionCriteria.snapshotEvery(N, keepN)`；快照加载失败默认停 actor（可配成忽略快照全量重放）；**事件删除被明确劝阻**——"deleting events you will lose the history… one of the main reasons for using Event Sourcing in the first place"。
- **schema 演化的官方指路**：序列化选型时即警告"must be possible to read old events when the application has evolved"，指向 schema evolution 文档。

### 2. Orleans JournaledGrain：两种存储哲学（✅）

- `JournaledGrain<TGrainState, TEventBase>`：`State` 只读、`Version` == 已确认事件总数；`RaiseEvent` 不等待、`await ConfirmEvents()` 等落盘确认；`RaiseEvents(多个)` 原子写。
- **Transition 方法约束**："no side effects other than modifying the state object and should be deterministic"——与 Temporal 的 workflow 约束同构。
- **log-consistency provider 分野是本次调研对"版本 pinning"最有启发的对照**：
  - `LogStorage`：每次加载**全量重放事件序列**重建状态——只要旧事件还能反序列化，"you can radically modify the GrainState class and the transition methods"。代码自由，代价是恢复成本与历史长度成正比。
  - `StateStorage`：只持久化最新 `GrainState`——恢复 O(1)，但状态类必须永远可反序列化，schema 演化压力转移到状态本身。
  
  我们的 fiber 哲学（插件无持久状态、重启即重算）天然是 LogStorage 一派，且事件序列有界（session 级）（🧭）。

---

## 第三部分：Event sourcing 经典

### 1. Fowler 2005：仍然是最锋利的概念框架（✅）

- **两个 system of record 选择**：event log 为正本（状态全为派生）vs 当前状态为正本（log 只做审计）。我们的答案几乎是强制的：journal 为正本——因为需求 b/c 都要求从 journal 重建，audit-only 形态不够。
- **快照的标准用法**：隔夜快照 + 日内内存态 + 崩溃后从快照重放增量事件。快照是**性能优化**，永远可以用"从头重放"替代——"reverting to a past snapshot and replaying the event stream"。
- **外部系统的 gateway 纪律**（直接命中需求 c）：
  - **外部更新**（对外发消息）：replay 期间 gateway 必须能静默——"the domain logic should never care about the context of the running of the events"，是否真发由 gateway 判断。映射：replay 模式下 host 的对外通道（LLM、网络、文件写入工具）由 host 统一掐断或接管，插件无感。
  - **外部查询**（读外部数据影响处理结果）：`LoggedPricingGateway`——查询经 gateway，**请求与响应都作为事件落 log**，replay 时查旧记录直接返回。这是"LLM 原始响应必须录制"在 2005 年的原型。
- **代码变更三分类**（与 Temporal 版本化对照看）：
  1. **新功能**：随时加，重放旧事件得新结果（gateway 关闭状态下）；
  2. **bug fix**：修好直接重放，状态被"修正"——event sourcing 的重放红利；
  3. **temporal logic**（"11 月 18 日前收 $10 之后收 $15"这类规则本身随时间变）：必须进领域模型（`chargingRules.get(aDate)`），不能靠代码分支——**这是最接近我们"热更插件 replay 用哪版代码"问题的经典论述**：规则的时间维度数据化（策略对象按时间索引），而不是代码版本的时间旅行。
- **bi-temporal 警告**："按 8 月 1 日的规则反转、按 10 月 1 日的规则重放"这条路"clearly can get very messy, don't go down this path unless you really need to"。

### 2. Schema 演化：upcasting（🧭 综合 ✅ 事实推出）

各来源的共识形态：事件落盘后不可变；读取侧负责把旧版本事件升级到当前 schema（upcast）；升级发生在反序列化边界，不写回（写回 = 篡改历史）。Akka 指向 schema evolution 文档、Orleans LogStorage 的"只要旧事件能反序列化"是同一约束的框架表述。与我们"消息形态 schema 化"的联动：payload 已经是统一 `Value`（[ADR-0015](../decisions/0015-payload-value-model.md)），upcast 可以实现为 `Value -> Value` 的纯函数链，挂版本号在条目头部——纯函数核心可进 fiber-core 式的可验证 crate（[fiber.md](../design/fiber.md) §9 待定项的同族）。

❓ 未展开核实：Axon Framework 的 upcaster 链文档是此模式最系统的工程表述，本次未取一手来源。

---

## 第四部分：确定性重放/仿真测试工程

### 1. 一个光谱：录制派 — 混合派 — 仿真派（🧭 组织轴，成员均 ✅）

| 位置 | 系统 | 非确定性的处置 |
| ---- | ---- | -------------- |
| 纯录制 | rr | 录制内核给进程的一切输入（syscall 返回值 + CPU 非确定指令），重放时逐字节注入 |
| 混合（编排骨架重算 + 边界录制） | Temporal / Restate / DBOS | 编排代码确定性重跑，边界结果（activity/step/timer）录制注入 |
| 纯仿真 | FoundationDB Simulation / TigerBeetle DST / Antithesis | 不录制——一切 IO/网络/时钟/随机都虚拟化并由仿真器驱动，同样的 seed 跑同样的世界 |

Metis 的 replay 是混合派（LLM 响应必须录制）；Metis 的测试策略（[testing-strategy](../design/testing-strategy.md) L2 故障注入）可以吸收仿真派思想。两派共享同一个前提：**所有非确定性必须从可替换的 seam 进入**——我们的构造注入纪律（AGENTS.md Style 节）被四个系统独立验证（🧭）。

### 2. rr：录制派的极限（✅）

- 录制整组 Linux 用户态进程，捕获"all inputs to those processes from the kernel, plus any nondeterministic CPU effects"；重放保证指令级控制流、内存、寄存器逐次一致——**连堆地址都一致**。
- 代价模型：**仿真单核**（并行程序被单核化）、不许与录制树外进程共享内存。Firefox 测试套件录制开销 ≤1.2x。
- 反向执行 = checkpoint + 正向重放——与 Fowler 的"reversal 永远可以用快照+重放替代"同一招。
- 对我们的相关性：rr 证明"录制一切输入"在 syscall 粒度可行但代价是执行模型阉割；我们的边界粒度粗得多（邮箱消息、LLM 响应、定时器），录制开销可忽略（🧭）。

### 3. FoundationDB Simulation：仿真派的宗师（✅）

- 单线程进程内**确定性仿真整个集群**；约 10:1 的真实:仿真时间比；每晚数万场仿真、累计等效约一万亿 CPU 小时；"seems unlikely that we would have been able to build FoundationDB without this technology"。
- **仿真一切物理组件**：机器数与型号、磁盘性能与写满、网络包投递；故障注入覆盖网络/机器/机房级，团队内部竞赛冠军故障模式 "swizzle-clogging"（随机子集连接逐个 clog 再随机序恢复）。
- 同一套 workload 代码同时用于仿真与 Circus 性能测试——**测试代码与生产代码共享 seam**。
- 对我们的相关性：故障注入测试（testing-strategy §3-F，安装事务中途 kill 等）的场景库可以直接借鉴其故障模式分类（🧭）。

### 4. TigerBeetle：DST + 防御纵深（✅）

- DST 作为内核：确定性仿真测试在单机上完美复现分布式故障。
- **接口设计原则**：网络与存储接口被刻意设计成 "promise nothing"——与底层故障模型一样烂（消息可丢、写可撕裂），stub 与真实实现可互换。这是 seam 设计的最高标准：**接口签名本身就承认故障**。
- **Vörtex**（2025-02，✅）：刻意**非确定性**的补足 harness——因为 DST 覆盖不到原生客户端绑定、真实网络、真实存储；用 TCP 代理注入延迟/丢包/损坏，kill/pause 副本进程；四个月抓到两个真 bug。教训：**确定性仿真与非确定性实景不是竞争关系，是纵深关系**。Antithesis 把同一思路产品化："hostile than prod" 的确定性仿真环境 + 完美复现 + Multiverse Debugger（✅），并明确定位到 "AI writing code" 时代的验证工具。

---

## 第五部分：LLM agent 领域现状

### 1. Claude Code：transcript 即 session 的参考实现（✅）

- **session 持久化为 `.jsonl` transcript**：`claude --resume` 接受 session ID、名字，**或直接接受 transcript 文件的绝对路径**——文件即会话，恢复 = 读文件重建。`--continue` 续最近会话。
- **fork 的一等公民形态**：`--fork-session` ——"When resuming, create a new session ID instead of reusing the original"。需求 d 在 agent 工具里的现状：fork = 沿用 transcript + 新 ID，成本一次 flag。
- **系统提示词的版本 pinning**：默认行为是——首次请求时构建 system prompt 并**录制进 session**，compaction 之前后续所有请求都用录制值，即使后续启动传了不同的 prompt flag。另有 `--system-prompt-snapshot off` 显式解除 pinning。**这就是"录代码/配置 revision"在 agent 领域的既有实践：影响行为的文本本身被钉进会话历史**。
- 子 agent 的 transcript 通过 `parent_tool_use_id` 串联（`--forward-subagent-text` 可重建每个 subagent 的对话树）——层级会话的因果结构 = 父指针，不是时钟。
- `claude respawn`：以对话完整保留的方式重启 session——崩溃恢复（需求 b）在 agent 工具里的形态。

### 2. Aider：git 作为账本（✅）

- 每次编辑即 git commit，`/undo` = 撤销上一个 aider 提交的 commit——**文件系统状态的可重放/可回退外包给 git**，会话工具只管对话历史。
- `/save` 把会话命令存成可重建当前会话的文件；Ctrl-C 中断时部分响应保留在对话里。
- 🧭 推断：Aider 的 chat history 落盘为 markdown 文件（`.aider.chat.history.md` 等，本次未逐一核实文件名），定位是"人可读的审计"而非"机器可重放的事实"——可重放性由 git 侧承担。对我们的启示：审计（需求 a）与重放（需求 b/c）可以用不同载体服务，不必强塞进一个格式。

### 3. 观测面事实标准：LangSmith threads 与 OTel GenAI semconv（✅）

- **LangSmith threads**：trace 通过 `session_id`/`thread_id` metadata（建议 UUIDv7）聚成 thread；子 run 必须传播同一 metadata 否则聚合漏算；UI 分 Trajectory/Turns/Details 三视图。**会话历史由应用自己持久化，LangSmith 只做观测**——观测与重放分离的又一分工实例。
- **OTel GenAI 语义约定**（已迁至独立仓库 `open-telemetry/semantic-conventions-genai`）——可直接抄的属性集：
  - 调用标识：`gen_ai.operation.name`（`chat`/`execute_tool`/`invoke_agent`/`embeddings`/`retrieval`/`fetch_response`…）、`gen_ai.provider.name`、`gen_ai.request.model`、`gen_ai.request.seed`、`gen_ai.response.id`；
  - 会话与上下文：`gen_ai.conversation.id`（明确禁止无中生有造 ID）、`gen_ai.conversation.compacted`（本次调用用的是压缩后上下文的标记）、`gen_ai.request.previous_response.id`；
  - **prompt 版本化**：`gen_ai.prompt.name` + `gen_ai.prompt.version`（prompt 模板的版本是一等观测属性——我们的 preset/plugin revision 记录的对应物）；
  - 用量：`gen_ai.usage.input_tokens`/`output_tokens`/`cache_read.input_tokens`/`reasoning.output_tokens` 等；
  - 工具：`execute_tool` span（INTERNAL kind），`gen_ai.tool.call.arguments/result`（opt-in）。
  - **内容捕获三档**（对我们脱敏设计直接可用）：① 默认不录消息内容（`SHOULD NOT capture by default`，`OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT` 类开关 opt-in）；② 录进属性（pre-prod 场景）；③ **内容传外部存储、span 上只留引用**（生产推荐）——且有 in-process hook 可在录制前改写内容（脱敏 seam 的官方形态）。
  - `fetch_response` 操作的存在值得注意：OpenAI Responses / Google Interactions API 支持按 response ID 拉取已生成响应、**不重新推理**——provider 侧也在变成"半个 journal"（🧭）。

### 4. VCR 系录制回放：脱敏与匹配的工程细节（✅）

vcrpy 模型：首次运行把全部 HTTP 交互录制为扁平 cassette 文件（YAML），再次运行拦截匹配请求直接回放——offline、deterministic、fast。对我们最有价值的是它的两个工程面：

- **匹配策略显式化**：`match_on` 可配（method/scheme/host/port/path/query/body/headers），可注册自定义 matcher。映射：LLM 响应回放时"什么算同一个请求"必须有显式判定（我们自然有：replay 按 journal 顺序注入，不需要内容匹配——但 fork/测试夹具场景需要）。
- **落笔脱敏全套钩子**：`filter_headers=['authorization']`、`filter_query_parameters`、`filter_post_data_parameters`，值可为静态替换/可调用/`None`（删键）；`before_record_request/response` 钩子可任意改写或返回 `None` 整条不录；解压先于脱敏（`decode_compressed_response`）。**初步判断③（落笔脱敏）在此有完整先例与 API 形态**。
- cassette 的"更新故事"是删除重录，不做语义升级——测试夹具与事实日志的保留策略差异在此（🧭）。

### 5. 小结：agent 领域的事实标准（🧭 归纳）

会话载体 = append-only JSONL transcript（Claude Code）；变更账本 = git（Aider）；观测 schema = OTel GenAI semconv；录制回放 = VCR cassette 模式；fork/resume = CLI 一等 flag。**没有一家尝试"重放时重新调模型"**——LLM 一律被视为不可重放的外部边界，这与需求 c 完全一致。

---

## 第六部分：Metis 特有问题（推理与权衡）

### 1. 热更与 replay 的版本 pinning

**问题**：replay 用旧代码还是新代码？录制什么 revision？

三个工业答案的对照（均见前文）：Temporal 说"pin worker 到部署版本"（推荐）与"代码内 patch 分支"（次选）；Orleans LogStorage 说"只要事件能读，代码随便换"；Fowler 说新功能和 bug fix 可以直接重放、temporal logic 必须数据化。

**我们的推理**（🧭）：

- fiber 保守重启哲学（[ADR-0010](../decisions/0010-fiber-core.md)）给了独一份的简化：**插件没有前向状态迁移问题**（DSU 明确是 future work）。插件状态 = `(config, 输入消息序列)` 的纯函数，所以"replay 用哪版代码"只影响行为语义，不影响状态兼容性。
- 录什么：每条目引用当时生效的 `plugin_revision`（插件文件内容 hash）+ `preset_revision` + `config_hash` + host 版本。**影响行为的文本钉进历史**——Claude Code 录制 system prompt、OTel 录 `prompt.version` 是同一原则的两处先例（✅）。
- replay 默认 pin：用录制期代码重放，才能保证需求 c 的"确定性"语义（同一历史、同一行为）。这要求插件代码可寻址存储（内容 hash → 插件文件），与 Creator 事务的原子提交+备份（[ADR-0008](../decisions/0008-creator-mode-self-modification.md)）天然衔接。
- "用新代码重放旧历史"是**另一类操作**而非 replay 的失败：它等价于 Fowler 的 bug-fix 重放（修正派生状态），产出应作为**新分支**落盘（即需求 d 的 fork），原历史保持不动。Fowler 的 bi-temporal 警告适用：不要试图在同一棵状态树上同时讲"当时的规则"和"现在的规则"。
- 版本兼容性判定可借用 Temporal/DBOS 的分类（✅）：插件改动的"安全/破坏"分界 = 是否改变它向 host 发出的调用序列形态（派发、服务调用、effect 注册的种类与次序）。

### 2. 单点路由器的全序红利

**已确立的结构事实**：host 星型拓扑路由一切跨插件交互（[ADR-0011](../decisions/0011-actor-execution-model.md)），即全系统存在一个唯一的消息定序点。

红利（🧭，由 ✅ 结构事实 + ✅ Temporal 先例推出）：

- host 给每个路由动作发**全局单调 seq**（u64 计数器即可），它就是全系统的 Lamport 时钟。不需要 vector clock、不需要混合逻辑时钟——调研确认无此需求的前提成立。
- **因果链 = 一个字段**：`cause: seq`（本条目由哪条目触发）。Temporal 事件上的 `scheduled_event_id`/`workflow_task_completed_event_id` 反向引用证明这种链接对"审计回看 + replay 定位"够用（✅）。
- 崩溃恢复的"单写者"问题自动消解：Akka 需要 Cluster Sharding 强制按实体单写者、还要 replay filter 防多写者污染（✅）；我们只有 host 一个写者，污染在结构上不可能。代价是 host 是 journal 写入的单点——写入量级 = 边界事件数（第一部分三家对照），单点 append JSONL 远够（🧭）。
- fork 后分支间的顺序无需协调：分支各自从 fork seq 起单调编号，分支间不存在并发写同一条历史的问题（Temporal reset 的 `base_run_id`/`new_run_id` 同构）。

### 3. 不可信插件作为 replay 参与者

**问题**：模型写的插件（不可信，[ADR-0008](../decisions/0008-creator-mode-self-modification.md) 隔离靠结构）在 replay 中发疯怎么办？

调研给出的统一答案（🧭 归纳 ✅ 事实）：**没有任何工业系统把确定性押在业务代码自觉上**。Temporal 检测（command 序列比对，发散 = `WorkflowTaskFailed`）；Akka 检测（单写者 + replay filter）；FDB/TigerBeetle 结构上消除（一切虚拟化，业务代码拿不到真 IO）。

我们的结构性保证栈（逐条对应既有决策）：

1. 插件拿不到非确定源：seam 注入（时间/随机/IO 构造注入，AGENTS.md Style）；Luau 沙箱白名单（cordis-research 第五部分结论）。
2. 插件拿不到执行顺序控制权：邮箱串行（[ADR-0011](../decisions/0011-actor-execution-model.md)），插件内无锁顺序代码。
3. 插件出不了纯数据边界：payload 永远是纯数据 `Value`（[ADR-0015](../decisions/0015-payload-value-model.md)），journal 条目可直接序列化。
4. **发散检测作为最后闸门**：replay 时 host 比对插件发出的调用序列与录制期是否一致（种类/目标/次序，内容可比到 payload hash）；不一致 = 该插件对该段历史不可重放——响亮失败进 Failed 态（fiber 状态机已有此归宿），并在 journal 留下 `replay_diverged` 条目（Temporal `WorkflowTaskFailed` 的对应物）。**这是对初步判断①的必要修正：重算不是免费可信的，必须与录制比对。**

可信度边界的表述：replay 信任 host 内核（Rust、seam、邮箱、定序），不信任插件行为的历史复现性；插件被当作"每次 replay 都重新受审的嫌疑人"，审据 = 录制。

---

## 第七部分：对 Metis 的映射

### 1. 该怎么做（按调研点落锤）

| # | 动作 | 依据 |
| - | ---- | ---- |
| M1 | **一条 JSONL 流承载 audit + replay 双消费**，条目类型区分；replay 只消费边界子集（入邮箱外部消息、LLM 响应、定时器触发、随机/时间读取），audit 消费全量 | Temporal 六族条目同存一史（✅）；DBOS/Restate 写入量级对照（✅）；日志方向 JSONL 已定（[ADR-0007](../decisions/0007-yaml-config-subset.md) 边界） |
| M2 | **LLM 调用录制原始请求与原始响应**，作为 replay 边界条目；模型永不因 replay 重调 | Fowler LoggedPricingGateway（✅）；Temporal 把 LLM 列进 Activity 首例（✅）；agent 领域无人重调（🧭） |
| M3 | **确定性判定行为化**：replay 比对插件向 host 的调用序列，发散即响亮失败（Failed + 条目记录），不静默采信 | Temporal nondeterminism error（✅）；对初步判断①的修正 |
| M4 | **版本 pinning 三件套进条目**：`plugin_revision`（内容 hash）、`preset_revision`、`config_hash`；replay 默认用录制期代码；"新代码重放旧史"产出强制落新分支 | Temporal Worker Versioning 推荐序（✅）；Claude Code system prompt 录制（✅）；OTel `prompt.version`（✅）；Fowler bi-temporal 警告（✅） |
| M5 | **fork = 引用前缀 + 新 session 追加**：`fork_created {base_seq, new_session_id}`，前缀只读共享，分支独立追加 | Temporal reset（✅）；Claude Code `--fork-session`（✅） |
| M6 | **落笔脱敏管线**：内核在 append 前执行脱敏钩子（凭证字段替换/删除/可调用），原始值永不落盘；LLM 请求中的凭证走 secret 引用而非值；内容级录制（消息全文）做成显式开关 | vcrpy filter/before_record 全家（✅）；OTel 内容 opt-in 三档（✅）；初步判断③成立 |
| M7 | **内核哑捕获**：host 只负责定序、append、脱敏钩子、回放注入；采样/保留/压缩/导出策略做成上层可替换组件（插件层或独立服务） | `MarkerRecorded` 对 server 透明（✅）；初步判断②成立 |
| M8 | **快照只做 host 侧、只做性能**：路由表/邮箱水位/fiber 树的快照条目；插件侧永远重放；快照永远可被"从头重放"替代 | Fowler 快照定位（✅）；Akka 快照语义（✅）；fiber 无状态哲学（[ADR-0010](../decisions/0010-fiber-core.md)） |
| M9 | **replay 模式的对外网关纪律**：replay 中 host 掐断/接管一切对外通道（LLM、网络、文件写工具），插件无感知 | Fowler gateway 纪律（✅） |
| M10 | **审计回看（需求 a）消费面**：journal 即查询源（`cause: seq` 链 + kind + fiber + session 过滤）；人读视图（markdown 导出）可作为派生物，不进正本 | Temporal 反向 ID 链（✅）；Aider 双载体分工（✅） |

### 2. 该避免什么

- **不要 vector clock / 任何分布式时钟机制**——全序已由单路由器结构给出（第六部分 §2）。
- **不要事件删除/改写**（保留策略只删快照与派生物，不删 journal；如必须裁剪，走"快照+前段归档且不可重放"的显式降级）——Akka 的劝阻（✅）。
- **不要把确定性寄托于插件自觉**——发散检测必须存在（M3）；同时接受"某些插件天生不可重放"是正常结论而非系统缺陷。
- **不要发明 patching API**（Temporal GetVersion 式代码内分支）——我们无长生命周期运行中实例需要兼容（fiber 重启即新实例），Worker Versioning 式的 pin 已够；patching 是 Temporal 为了"运行数月的执行"付的复杂度税，我们没有那个税基（🧭）。
- **不要把 journal 写成只能机器读的格式**——需求 a 要求审计回看，JSONL + 统一 `Value` 已保证人可读；二进制编码是负优化（🧭）。
- **不要在 replay 时允许插件热更生效于当前分支**——fork 才是新代码的入口（M4）。

### 3. Journal schema 初步候选

**公共字段**（每条必备）：

| 字段 | 类型 | 说明 |
| ---- | ---- | ---- |
| `seq` | u64 | host 分配的全局单调序号（全系统 Lamport 时钟） |
| `wall_ms` | i64 | 墙钟毫秒（展示用，不参与 replay 逻辑；[ADR-0015](../decisions/0015-payload-value-model.md) 时间戳纪律） |
| `kind` | string | 条目类型（见下表） |
| `session` | string | session id |
| `fiber` | object \| null | `{slot, generation, plugin, plugin_revision}`；host 自身条目为 null |
| `cause` | u64 \| null | 触发本条目的条目 seq（因果链） |
| `payload` | Value | 脱敏后的条目体（[ADR-0015](../decisions/0015-payload-value-model.md) 统一数据模型） |
| `code_ref` | object | `{plugin_revision, preset_revision, config_hash, host_version}`（M4；session 级条目携带一次即可，变更时再记） |
| `redacted` | array \| null | 被脱敏的 payload 路径 + 策略版本（M6；审计可见"哪里被擦过"） |

**候选条目类型**（replay 消费标 ★）：

| 族 | kind | 内容要点 |
| -- | ---- | -------- |
| 生命周期 | `session_started` ★ / `session_ended` | 启动 config 快照、code_ref |
| 生命周期 | `fiber_state_changed` | fiber 状态机迁移（对应 `internal/plugin` 钩子，[ADR-0016](../decisions/0016-internal-hooks.md)） |
| 生命周期 | `config_committed` | Creator 事务落盘（diff hash + 审批引用，[ADR-0008](../decisions/0008-creator-mode-self-modification.md)） |
| 边界输入 | `external_input` ★ | 用户消息/外部系统进入 session 的第一落点（fork 与重放的事实起点） |
| 边界输入 | `message_enqueued` ★ | host 向邮箱投递（事件派发四模式 / 服务调用 / dispose / timer tick；含派发模式与 payload） |
| 边界输入 | `timer_scheduled` / `timer_fired` ★ | 定时器登记与触发（Temporal `TimerStarted/Fired` 对应物；replay 按史注入不按墙钟） |
| 边界输入 | `nondeterminism_read` ★ | seam 注入的时间/随机读取结果（Temporal replay-safe API 录制对应物） |
| LLM 边界 | `llm_request` ★ / `llm_response` ★ / `llm_error` ★ | 原始请求（model、参数、消息引用）与原始响应（含 `response.id`、usage、finish_reason——属性名可直接对齐 OTel GenAI semconv） |
| 内部流量（audit） | `dispatch_completed` | 四模式派发结果（parallel 各项 Result / serial 终止值 / waterfall 链） |
| 内部流量（audit） | `service_call` / `service_result` | 跨插件服务调用（replay 重算，不注入） |
| 内部流量（audit） | `listener_error` / `vm_trap` | fail-open 记日志的落点（[ADR-0014](../decisions/0014-dispatch-semantics.md)） |
| 控制 | `snapshot_written` | host 侧快照引用 + 覆盖到哪个 seq（M8） |
| 控制 | `fork_created` ★ | `{base_seq, new_session_id}`（M5） |
| 控制 | `replay_diverged` | 发散检测报告：期望 vs 实际的调用（M3） |
| 控制 | `redaction_applied` | 脱敏策略版本与命中计数（M6） |

**明确的非目标**：条目不含函数/协程/插件引用（payload 纯数据铁律的延伸）；不含纳秒时间戳；不含凭证值。

### 4. 开放问题（留待设计期）

1. 单流还是双流（audit 与 replay 分文件）？本报告倾向单流 + kind 过滤（M1），但写入量实测前不算定案。
2. LLM **流式**响应的录制粒度：整段 buffered 录制最简单；chunk 序列录制可支撑断流恢复（OTel 的 streaming chunks 章节本身也还是 TODO，✅）。v1 倾向 buffered。
3. fork 前缀的物理组织：共享只读前缀文件 + 分支文件，还是 copy-on-write 复制？（Temporal 复制进新 run；我们体量小，倾向逻辑引用。）
4. 发散比对的强度：只比调用序列（种类/目标/次序），还是连 payload 内容 hash 一起比？后者更严但误报面大（插件合法地依赖注入时间戳时会误伤——需要 `nondeterminism_read` 条目先把这类读取变确定）。
5. host 崩溃时 journal 的 fsync/写放大策略与邮箱水位的对账（崩溃窗口内已投递未落盘的条目如何界定——Akka "journal 失败即停、不可 resume" 的戒律适用，✅）。
6. 跨 session 服务调用（若 session 模型引入）是否算边界输入——当前按内部流量处理，待 [session-model](../design/session-model.md) 正式设计时复审。

---

## 附录：主要信息来源（含核实状态）

- Temporal 官方文档（✅ 本次 fetch）：Events reference（`docs.temporal.io/references/events`）、Workflow Execution overview（replay/commands）、Workflow Definition（determinism constraints、intrinsic non-determinism）、Go SDK Versioning（GetVersion/patching、Worker Versioning 推荐序、cutover）。
- Restate 官方文档 Key Concepts（✅）：journal 录制步骤与结果、状态随 journal 一致、单写者 Virtual Object、幂等键。
- DBOS 官方文档 Architecture（✅）：Postgres checkpoint、恢复三步、确定性+幂等要求、patching/versioning 两策略、>40K step/s。
- Akka Persistence 官方文档（✅）：Event Sourcing（typed）与 Snapshotting 两页（Effect 代数、at-most-once 副作用、单写者、replay filter、retention、事件删除劝阻）。
- Microsoft Orleans 官方文档（✅）：Event sourcing overview 与 JournaledGrain basics（Version 语义、ConfirmEvents、两种 log-consistency provider）。
- Martin Fowler, "Event Sourcing"（2005，✅）：快照定位、gateway 纪律、LoggedPricingGateway、代码变更三分类、bi-temporal 警告。
- rr 官网（✅）：录制范围、单核仿真、chaos mode、≤1.2x 开销、反向执行原理。
- FoundationDB 官方文档 Simulation and Testing（✅）：单线程确定性仿真、10:1 时间比、swizzle-clogging、一万亿 CPU 小时等效。
- TigerBeetle 博客（✅）："A Descent Into the Vörtex"（2025-02-13，DST 防御纵深、"promise nothing" 接口、Vörtex 非确定性 harness）；博客索引另见 "Protocol-Aware Deterministic Simulation Testing"（2026-08）、"Simulation Testing For Liveness"（2023-07）等篇目（标题核实，内容未展开）。
- Antithesis 官方文档 Welcome（✅）：确定性仿真环境、敌对故障、Multiverse Debugger / Causality Analysis。
- Claude Code 官方文档 CLI reference（✅）：`.jsonl` transcript 可被 `--resume` 直接引用、`--fork-session`、system prompt 录制进 session（`--system-prompt-snapshot`）、`claude respawn`、`parent_tool_use_id`。
- Aider 官方文档 Usage / In-chat commands（✅）：git 账本 + `/undo`、`/save`、中断保留部分响应。
- LangSmith 官方文档 Configure threads（✅）：`session_id`/`thread_id` metadata、子 run 传播、三视图。
- OpenTelemetry GenAI 语义约定（✅，新仓库 `open-telemetry/semantic-conventions-genai`）：gen-ai-spans 全文（operation/provider/model/seed/usage 属性族、conversation.id/compacted、prompt.name/version、内容捕获三档与上传钩子、fetch_response/stream_cursor、execute_tool span）。
- vcrpy 官方文档（✅）：README（cassette 模型）、Configuration（record_mode、match_on）、Advanced Features（filter_headers/query/post_data、before_record 双钩子、解压先于脱敏）。
- ❓ 未联网核实：Axon Framework upcaster 文档（schema 演化工程表述）；Helicone/LiteLLM 等 LLM 代理的缓存录制形态；Aider 历史文件名细节；FoundationDB "Testing at the speed of light" 演讲原文。
