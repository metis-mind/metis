# Metis — WIT ABI 设计（living doc）

> 状态：living design doc——2026-10-15 开章（[ADR-0034](../decisions/0034-wasm-first-carrier.md) wasm-first 转向同批）。语义地基平移自 [luau-abi](luau-abi.md)（已归档，仅供考古）；W 系列章节随设计批逐章收官，章末联动登记汇总惯例同 A 系列。各章状态见目录表。
> 前提：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；接缝能力集 = [seam-capabilities](seam-capabilities.md)；遏制四约束 = [ADR-0005](../decisions/0005-vm-topology.md)；边界拷贝语义四层 = [wasm-research](../research/wasm-research.md) 补遗 5 §5.4。
> 拍板锚（2026-10-16 两轮）：接口数据三分法 / E4 fs 三层——SSOT = 台账 T13 状态行，议题背景 = [wasm-first-design-review](../research/wasm-first-design-review.md) §6/§7；W 各章收官时吸收进正文，此行随吸收删除（E3 并发投递 已吸收 = §W4，2026-10-17）。
> Last reviewed: 2026-10

---

## 0. 术语锚点

本文档新增（共享术语见 [seam-capabilities](seam-capabilities.md) §0）：

- **任务（task）**：组件实例内一条在飞执行流 = 一次 export 调用及其 guest 内派生；同一瞬间至多一条任务在 CPU 上，切换仅在挂起点（W4.1）。
- **挂起点**：guest 任务让出执行权的点 = 任意 await 处；宿主只见 import 边界的挂起与唤醒（W4.1）。
- **在飞任务**：已投递未完结的任务；背压预算面 = 每实例在飞任务数封顶（W4.3）。
- **重入（reentrancy）**：前一条 export 调用挂起期间，后一条进入同一实例起跑新任务（W4.1；实证 = S2 spike，[wasm-research](../research/wasm-research.md) 补遗 7）。
- **邮箱 / worker**：actor 基线件（[ADR-0011](../decisions/0011-actor-execution-model.md)），host 侧每插件实例各一份；投递语义 = W4.3。
- **实例即弃**：热换/卸载时整个组件实例（Store+Instance）随弃，新实例干净启动（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §1；卸载交接 = W4.5）。

## 章节目录（W 系列，随设计批逐章开章）

| 章  | 题                                                                                                   | 前身（A 系列 = 语义平移源） | 状态          |
| --- | ---------------------------------------------------------------------------------------------------- | --------------------------- | ------------- |
| W1  | 插件包形态与入口契约（world 定义 / export setup·dispose）                                            | A1                          | 未开始        |
| W2  | manifest 格式                                                                                        | A2                          | 未开始        |
| W3  | ctx 形态：WIT import 接口集与 SDK 映射（作者不见 WASI/WIT = ADR-0034 §4）                            | A3                          | 未开始        |
| W4  | async 与交错语义（组件模型 async 映射；S1/S2 spike 双 PASS 托底 = 补遗 6/7）                         | A4                          | ✅ 2026-10-17 |
| W5  | 错误与 result 习语（`result<T,E>` 原生）                                                             | A5                          | 未开始        |
| W6  | 边界转换：canonical ABI / 递归 Value 线编码（TLV 主线化 = E1 选项① 拍板）/ resource 句柄族           | A6                          | 未开始        |
| W7  | 契约与类型来源（① 平台封闭面 = WIT 富类型原生；A7 半消融 = E5 拍板：瘦身形状 DSL 继承 Value 形状面） | A7                          | 未开始        |
| W8  | 事件静态声明                                                                                         | A8                          | 未开始        |
| W9  | deps/provide 与能力门（进口对账机械化）                                                              | A9                          | 未开始        |
| W10 | 工具形态与 ABI 版本纪律（WIT package version 承接 abi_rev）                                          | A10                         | 未开始        |

---

## W4 async 与交错语义（2026-10-17 收官）

### W4.0 边界与前提

本章定插件实例内的执行模型（async 与交错语义）、宿主投递政策与背压、跨插件死锁与保险丝、卸载与在飞任务交接。**前提（已定不议）**：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；actor 基线 = 邮箱 + 星型拓扑 + 三层保险（[ADR-0011](../decisions/0011-actor-execution-model.md)，保留资产语义不动）；S1 spike PASS（guest async 工效学，[wasm-research](../research/wasm-research.md) 补遗 6）；S2 spike PASS（同实例重入，补遗 7——本章的实证地基）；E3 拍板与澄清（2026-10-16，SSOT = 台账 T13 状态行：最大化并发投递；宿主零同步原语，同步归 guest 语言设施；实例内各任务共享同一堆，同步件 = await 间天然原子 + async mutex，原子变量无场景）。不归本章：ctx import 接口集与 SDK 映射 = W3；错误与 result 习语 = W5；边界转换与 resource 句柄族 = W6；插件包形态与入口契约 = W1；能力面 = W3/W9 章（ctx import 集 / 能力门）与 T10.11 站。

| 节   | 议题                                       | 状态              |
| ---- | ------------------------------------------ | ----------------- |
| W4.1 | 执行模型 = 同实例重入 + 自由交错           | ✅ D93 2026-10-17 |
| W4.2 | 宿主零同步原语（fan-out / exclusive 归宿） | ✅ D94 2026-10-17 |
| W4.3 | 投递政策与背压（邮箱 + 在飞任务预算闸门）  | ✅ D95 2026-10-17 |
| W4.4 | 跨插件死锁与保险丝                         | ✅ D96 2026-10-17 |
| W4.5 | 卸载与在飞任务交接                         | ✅ D97 2026-10-17 |
| W4.6 | A4 机制消亡清单（考古索引）                | ✅ 2026-10-17     |
| W4.7 | 联动登记汇总（章末）                       | ✅ 2026-10-17     |

### W4.1 D93 执行模型 = 同实例重入 + 自由交错（2026-10-17 定案，E3 拍板吸收）

**D93 定案：插件实例内执行模型 = 协作式多任务自由交错。** 宿主向同一实例并发投递多条 export 调用（最大化并发投递 = E3 拍板）：前一条 handler 挂起期间，后一条消息立即进入同一实例起跑新任务，无实例级串行门。同一瞬间至多一条任务在 CPU 上——协作式语义结构性成立：guest 单线程执行器（组件模型 async 的任务 = 协作式切换）+ shared-memory 配置禁用（[ADR-0034](../decisions/0034-wasm-first-carrier.md) W6 风险行）双层保证；切换仅发生在挂起点，两挂起点之间原子——无并行、无数竞。

**挂起点重定义**：挂起点 = guest 任务的任意 await 处。A4 的「挂起点 = ctx 方法调用处」是 Luau 世界的等价物——彼时一切挂起必经宿主，此刻语言设施自管。宿主只见 import 边界的挂起与唤醒（await import 完成 = 宿主可控唤醒源）；guest 内部 await 宿主不可见、无需见。

**实证地基**（S2 = [wasm-research](../research/wasm-research.md) 补遗 7，wasmtime 49.0.3 / wit-bindgen 0.63 / wasm32-wasip2）：

- **R1 同实例重入**：单宿主线程 `run_concurrent` 内并发两调同一实例 async export（500 ms 与 100 ms 挂起），墙钟 ≈ 500 ms（非串行 ≈ 600 ms），交错日志序 = start 1 → start 2 → end 2 → end 1——第二条调用在第一条挂起期间进入同一实例并先完成。
- **R2 共享堆与原子性**：两条在飞任务共享同一线性内存堆；无 await 的读改写两次 = 2（挂起点间原子）；跨挂起点 read-modify-write 并发两调 = 丢失更新（终值 1 非 2）——危险形状实证，即 async mutex 的全部动机。

**概念地基（平移，教学条，Creator 文档必带）**：并发等待 ≠ 并行计算——插件耗时大头在**等**（LLM/HTTP/磁盘/子进程），等待 = 任务挂起、执行权让给别人；「一个执行器」限制的是「算」，不限制「等」。分工一句：**等的事重叠 = guest async 原生**（`join!`/`select!` 一族）；**算的事并行 = 同码多实例并跑或外部服务**（薄编排反转 = [wasm-first-design-review](../research/wasm-first-design-review.md) B8：插件内 CPU 密集合法化）。协作式免费性质平移：不跨挂起点的代码区间天然原子 → 需要显式互斥的形状只有一种 = 跨挂起点 read-modify-write。

**作者心智**：普通 async Rust（SDK 藏 ABI = ADR-0034 §4）。A4 作者契约四条平移：协作式（两挂起点之间原子）/ 挂起窗口内什么都可能变（挂起期间其他消息先跑，自家状态可能被改）/ 同步全显式（宿主零隐式串行，W4.2）/ 在飞即成本（每条挂起任务持 guest 栈与捕获态，W4.3）。

**淘汰项存档**（E3 议题批已决）：实例级串行化（run-to-completion，挂起即不投递新消息）——回退到 A4.1 枪毙过的头阻问题（一个 session 的 30 秒 LLM 调用卡住全插件邮箱）；声明式重入许可（按 handler/事件名标注）——灵活但引入新声明面，而真正需要同步的位置很少且可枚举（单一形状，见概念地基）。

### W4.2 D94 宿主零同步原语（2026-10-17 定案，E3 拍板吸收）

**D94 定案：宿主不提供任何同步原语。** fan-out = guest 异步设施原生（`futures::join!` 一族；SDK 呈现形态归 T13-d/T10.14），A4 的 `ctx.fanout` ABI 原语消亡——宿主无观测点（journal `fanout_cancelled` 挂载点随之消解，处置登记 = W4.6/W4.7）。互斥 = SDK async mutex 助手（同名互斥、含挂起期间占用的语义在 guest 内兑现），A4 的 `ctx.exclusive` ABI 原语消亡。原子变量无场景 = E3 澄清（单线程执行器，await 间天然原子）。

- **D1.2 三规则重估面归 T10.14**（E3 拍板登记原位）：不嵌套 / 自家人 / 强放——可由 SDK 类型系统表达的部分归类型，余者归 SDK 文档纪律；宿主侧名字注册表与名字条目回收机器整体消亡。
- **全局互斥不立 = A4 三理由原样平移**：占用中一次普通服务调用即可成圈（死锁从罕见变常态）；manifest 看不见的隐式跨插件契约（与 [ADR-0018](../decisions/0018-service-registry.md) 的 manifest-first 纪律冲突：跨插件关系以 manifest 声明为准）；宿主机制厚度。跨插件协调三归宿不变：属主插件 + 服务调用（manifest 可见可审批）/ 宿主侧配置化策略 / 分布式现实（seam 既定立场：两插件写同一文件靠协议不靠互斥）；原子方法族 = seam 能力面原位（首件 = fs `append`，形态归 W3/W9 章与 T10.11 能力站）。
- **结构化纪律平移**：handler 完结时其 fan-out 子任务须已完结——Luau 世界靠宿主强制取消兑现，wasm 世界 = guest 语言设施职责（`join!` 天然结构化；detach spawn = SDK 纪律面）。宿主不补机制。
- **扩展位登记**：exclusive 可选上报宿主做 wait-for 可见性——上报是仪表不是机制（E3 议题批原话）；触发器 = 首个死锁调试痛点。

### W4.3 D95 投递政策与背压（2026-10-17 定案）

**D95 定案：投递 = 向实例发起新 export 调用；邮箱 + 每实例在飞任务预算闸门；完成永不挡。**

- **worker 语义**：worker 空闲取邮箱消息，经闸门接纳后投递，任务立即起跑；import 完成 = 唤醒源直接唤醒对应 guest 任务，不经邮箱、不经闸门。宿主侧没有「就绪恢复」调度对象——A4 单可调度队列 FIFO（D8.1）的问题形（恢复 vs 新消息的优先级之争、tie-break、两级队列）随载体整体消解；原被否项「完成优先」的饿死分析（流式长读插队致热更等待无界）随之失去对象。热更/卸载不进投递队列（关闸 = 停接纳，W4.5），等待硬上界结论由卸载时序兑现，不再由队列规范兑现。
- **预算闸门（A4 D8.2 平移）**：预算只挡「邮箱消息 → 投递」的接纳——每实例在飞 async 任务数封顶（数值 = 实现期配置，登记 = W4.7）；预算满 = 新消息留邮箱，背压链 = 邮箱满 → 投递方背压/超时（[context-events](context-events.md) 既有政策）→ 服务调用方 deadline 收场（service-registry S5）。完成永不挡 → 在飞必然收敛 → 预算必然释放；超限政策 = 纯回压不立响亮档（A4 D10.2 拍板原位：回压把瞬态尖峰变成延迟，响亮把它变成用户可见错误——尖峰是常态，错误化 = 误伤）。
- **ADR-0014 兼容注**：单派发者保序承诺（同一监听者收件序 = 派发入邮箱序）= 邮箱层性质，不受重入影响——承诺的是收件序，非处理完成序（A4 既有注平移）。
- **timer 处置平移（A4 D1.4）**：tick = 普通邮箱消息，与其他消息同等走闸门，不立特殊政策。积压面 = 在飞任务数（稳态界 ≈ 到达率 × 平均延迟；每条在飞任务持 guest 栈与捕获态）；兜底 = `every` 最小间隔政策（seam §5.2）+ 写码期教具（W4.4 层二）。
- **在飞即成本**：高频事件源回调写短；怕自撞 = SDK async mutex（A4「在飞协程」成本条的载体词平移）。

### W4.4 D96 跨插件死锁与保险丝（2026-10-17 定案）

**D96 定案：跨插件死锁四层处置平移；插件内等待面宿主不立 deadline；fail-fast 清单按宿主可见面重订。**

- **D1.7 四层平移**：① 启用期 manifest 依赖图禁环（不动；WIT 形态归 W9 章）；② 写码期教具 = 形状教材归 SDK lints 与 Creator 文档（Luau AST lint 管线消亡 = [wasm-first-design-review](../research/wasm-first-design-review.md) A8；三形状载体中立，于 Rust 同样成立：跨 await 读改写 / 持锁等跨插件调用 / `every` 回调挂起未自锁）；③ 运行期 wait-for 图（宿主全见跨插件等待——服务调用与事件链；收窄写实：插件内名字等待随宿主零同步原语退出图的节点集，除非 W4.2 扩展位上报激活）；④ 超时 = 最后保险丝（[ADR-0011](../decisions/0011-actor-execution-model.md) 三层保险既有面）。
- **插件内等待面盘点（A4 D10.1 平移）**：fan-out join 与 exclusive 排队 = guest 内部面，宿主不见不立 deadline（guest 侧结构化纪律 = SDK）；流 `read()` 挂起 = 唯一真无界源的性质不变（作者自开自流、可见可控），处置 = W6 资源面登记（per-read timeout 扩展位随章平移）。不立的理由平移：能定出原则性数值的形状都已有 deadline，剩下的是合法长等，任何统一数值都是拍脑袋；兜底三件套 = 在飞预算记账 + 卸载强杀（W4.5）+ 成环检测（层三）。
- **fail-fast 清单（宿主可见面重订）**：wait-for 圈闭合（当场报错 + 整圈路径 + fail-open 解圈 = [ADR-0014](../decisions/0014-dispatch-semantics.md)）；卸载期注册新账（W4.5）；未声明能力调用 = 进口对账编译期结构事实（[wasm-first-design-review](../research/wasm-first-design-review.md) B1——运行期出现规则兜底退役为结构保证）；不可转换值跨界（W6 面）。A4 清单的插件内四项消亡或归 SDK：exclusive 嵌套 / 自家人进名（SDK 类型与纪律面）；被登记线程裸 yield（无对应物——宿主无「无登记挂起」可检，观测点缺失即消亡理由）；fan-out 子任务未随 handler 完结（guest 结构化纪律，宿主无观测点）。
- **兜底保险丝层平移**：单步执行硬截断 = fuel/epoch 计量（wasmtime 原生件，实测 1.022× = [ADR-0024](../decisions/0024-carrier-layering.md) §8；[ADR-0011](../decisions/0011-actor-execution-model.md)「防堵 executor」语义不动，机制自 interrupt 回调换血——Status 注同批落地）；服务调用 deadline（service-registry S5）/ bounded 邮箱 + 在飞预算回压（W4.3）/ 加载总预算（W1 章面）/ drain 总预算（[fiber](fiber.md) §10）。

### W4.5 D97 卸载与在飞任务交接（2026-10-17 定案）

**D97 定案：卸载与在飞任务交接 = 严格串行四段（关闸 → 优雅窗口 → 强杀 → drain），A4 D11.1 时序平移，强杀机制换血。**

1. **关闸**：进 Unloading 即停收新投递（邮箱关闭 = 既有「投递失败正常跳过」纪律）、在飞闸门停接纳；
2. **优雅窗口**：在飞任务自然完结（import 完成照常唤醒；数值 = 实现期配置，热更场景默认短）——挂在静默流上的合法长等不等它；
3. **强杀**：窗口尽，**实例即弃**（drop 整个组件实例）——interrupt 逐线程杀机制消亡。S2 R4 实证（补遗 7）：在飞 guest 任务随并发域遗弃（`run_concurrent` future drop 实测），无 trap、无诊断输出；实例即弃 = 同一丢弃语义的外推（实例 drop 未单列实测）。诚实边界 = 静默是载体事实，作者可见性由 journal 关闸/强杀点记录兑现（D11.5 既有面平移）。在飞任务挂起的 import 随实例一同消失，宿主侧迟到的完成 = 丢弃（既有纪律平移）；
4. **drain**：既有路径（fiber LIFO、disposer 可 await、超时强制）。**串行不重叠**平移：disposer 起跑时 handler 已全灭（实例已弃），无并发干扰。

淘汰项存档（A4.5 平移）：无限等完结（静默流挂起 = 热更卡死）；立即杀（丢掉片刻后就写完的盘，且违 [ADR-0018](../decisions/0018-service-registry.md) S4「优雅执行/超时强制」既有精神）。

- **D11.2 卸载期拒新账平移**：Unloading 全程拒绝「比我长寿」的新账（新注册监听 / provide / timer = 响亮错误——语义上想活得比插件久的注册，在卸载期无意义）；效应调用放行（在飞任务收尾要用）；新句柄放行但 drain 强 close 兜底（流式句柄族铁律，W6 章平移面）。
- **D11.3 对手方即错平移**：A 调 B、B 进 Unloading——B 的在飞任务窗口内跑完则结果照常路由回 A；被杀则宿主立即给挂起的 A 注入 `Err(Unavailable)`，不傻等 deadline（[ADR-0018](../decisions/0018-service-registry.md) S4「故障检测结构性」的延伸）；Unloading 期间新调用 B = 即 `Err(Unavailable)`（邮箱已关）。
- **D11.4 邮箱残留 = 丢弃平移**：关闸后未投递消息直接丢（事件 = 投递失败正常跳过；服务调用投递 = 按 D11.3 即错回调用方）。A4 的「可调度队列残留」一格随 D8.1 消解无对应物（wasm 下投递即起跑，无中间队列态）。
- **D11.5 保证分级平移**：服务调用 = 结果或显式 `Err`，绝无静默；事件/消息处理 = 尽力（窗口期完结）+ 可见（关闸丢弃清单、强杀点进 journal，事后可查「哪条没处理完」）；消息重投 = 否决（被杀 handler 可能已发出过回复——重投 = 重复效应风险；exactly-once 物理不可达）。业务连续性归宿不动：跨热更要连续的业务 = 插件经 seam 持久化 + 新版本 setup 读回接续；核心的责任止于给显式结果、给可查记录、给持久化能力。

### W4.6 A4 机制消亡清单（考古索引）

每条一句死因；考古读 [luau-abi](luau-abi.md) §A4（已归档）。

| 机制                                                                        | 死因                                                                                                                                                                                                                 |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| coroutine shim / SENTINEL / 挂起冒泡链 / outstanding 防假注入（§A4.3 D9.2） | guest async 运行时原生挂起恢复，无需宿主令牌机器（S1/S2 实证）                                                                                                                                                       |
| 线程/协程术语分层（§A4.3 D9.1）                                             | 分层对象为 Luau coroutine；Rust guest 的 async task 无需再包装                                                                                                                                                       |
| 被登记线程裸 yield 响亮杀（§A4.3 D9.3）                                     | 无对应物——宿主无「无登记挂起」可检（guest 内部挂起宿主不可见 = W4.1）                                                                                                                                                |
| 调度规则纯函数化冻结进 ABI（§A4.1 D1.6）                                    | replay 不立（[journal](journal.md) D75）+ guest 内调度器宿主不可冻结；D27.1 ABI 确定性面随之缩没                                                                                                                     |
| 单可调度队列 FIFO 规范（§A4.2 D8.1）                                        | 问题形消解（W4.3）；tokio/Temporal/FDB 确定性调度教学条随之退役                                                                                                                                                      |
| `ctx.fanout` / `ctx.exclusive` ABI 原语（§A4.1 D1.2/D1.3）                  | 降级为 guest 异步设施与 SDK async mutex 助手（W4.2）                                                                                                                                                                 |
| lint 三形状（§A4.1 D1.5）                                                   | Luau AST lint 管线消亡（design-review A8）；形状教材归 SDK/Creator（W4.4 层二）                                                                                                                                      |
| journal `fanout_cancelled` 族（[journal](journal.md) D78 错误现场族）       | fan-out 归 guest 内部件，宿主无观测点；D78 该格处置 = journal 演进纪律通道（ADR-0029：机制面破坏式 → 新 ADR supersede）——处置通道已拍板 = 随下一文档批（W5/W6 章批或 T13-f 兜底批）同批 supersede 小条（2026-10-17） |

### W4.7 联动登记汇总（章末）

| 去向                 | 内容                                                                                                                                                                                                                                                       |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T10.2（fiber-core）  | 实现期配置原位存活（载体词已换）：每实例在飞 async 任务预算（W4.3）+ 优雅窗口与 drain 总预算数值（W4.5）；wait-for 图存活 = W4.4 层三（节点集收窄在案）                                                                                                    |
| T10.4（事件派发）    | worker 投递/完成模型与闸门位置 = W4.3；ADR-0014 保序兼容注原位；D6 事件吞吐件不变；裸 yield 检测 D9.3 消亡在案（T10 方向变更注）                                                                                                                           |
| T10.14（插件 SDK）   | exclusive = async mutex 助手 + D1.2 三规则重估面（W4.2）；fan-out 呈现形态（W4.2）；写码期教具三形状（W4.4 层二）；S2 papercuts 7–11 输入（[wasm-research](../research/wasm-research.md) 补遗 7），含 `exports: { default: async }` 绑定约束（papercut 7） |
| W3 章（ctx 形态）    | 挂起点的作者面呈现（W4.1）；SDK async 映射输入（W4.1/W4.2）；补遗 6/7 papercuts 分流在案                                                                                                                                                                   |
| W5 章（错误）        | fail-fast 清单的错误面接口（W4.4）——wait-for 圈 / 卸载期新账的错误族归置                                                                                                                                                                                   |
| W6 章（边界转换）    | 流 `read()` 无界源登记（W4.4）；不可转换值跨界 fail-fast 格（W4.4）；流式句柄族 drain 强 close 铁律平移面（W4.5 D11.2）                                                                                                                                    |
| journal.md           | `fanout_cancelled` 处置登记（W4.6；通道已拍板 = 随下一文档批同批 supersede 小条，2026-10-17）；`unload_gate_closed`/`unload_killed` 两族存活（W4.5 语义原位，条目措辞随实现期载体词）                                                                      |
| 实现期配置           | 每实例在飞 async 任务预算（W4.3，超限 = 纯回压）；优雅窗口 + drain 总预算（W4.5）；host 侧事件在途投递界（A4 D10.2 原位存活）                                                                                                                              |
| 扩展位               | exclusive 上报 wait-for 可见性（W4.2，触发器 = 首个死锁调试痛点）；原子方法族其余成员（A4 D1.8 平移，触发器 = 首个真实消费者）；per-read timeout 参数（W4.4 → W6 章面，触发器 = 真实需求）                                                                 |
| seam-capabilities    | §2.8 FAQ（引 `ctx.fanout`/`ctx.exclusive`）与 §6.1「挂起点 = ctx 方法调用处」行 = 本批消亡/重定义件——清写归 T13-f（f1），本章登记交接                                                                                                                      |
| ADR-0011             | 「独立 `lua_State`」「interrupt 时间盒」两格载体面 partial supersede——**同批落地 = Status 行注**（2026-10-17）                                                                                                                                             |
| context-events §0/§1 | worker 伪码、三层保险行与服务调用路径行清写——**同批落地**（T13-f 主通道分配项：ADR-0011 随 T13-c）                                                                                                                                                         |
| Creator 文档         | 作者契约四条平移（协作式 / 挂起窗口 / 同步全显式 / 在飞即成本，W4.1）+「等的事重叠 = guest async 原生；算的事并行 = 多实例/外部服务」分工条                                                                                                                |
