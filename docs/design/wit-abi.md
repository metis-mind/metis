# Metis — WIT ABI 设计（living doc）

> 状态：living design doc——2026-10-15 开章（[ADR-0034](../decisions/0034-wasm-first-carrier.md) wasm-first 转向同批）。W 系列章节随设计批逐章收官，章末联动登记汇总惯例同前。各章状态见目录表。
> 前提：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；接缝能力集 = [seam-capabilities](seam-capabilities.md)；遏制四约束 = [ADR-0005](../decisions/0005-vm-topology.md)；边界拷贝语义四层 = [wasm-research](../research/wasm-research.md) 补遗 5 §5.4。
> 拍板锚（2026-10-16 两轮）：接口数据三分法 / E4 fs（现两档：private 隐式 / global 声明）——SSOT = 台账 T13 状态行，议题背景 = [wasm-first-design-review](../research/wasm-first-design-review.md) §6/§7；W 各章收官时吸收进正文，此行随吸收删除（E3 并发投递 已吸收 = §W4，2026-10-17；三分法①/TLV 边界已吸收 = §W1.3，2026-10-18；E4 fs 已吸收 = §W2.5/W2.6 + ADR-0039（private 隐式 / global 声明，命名根撤回，2026-10-19））。
> Last reviewed: 2026-10

---

## 0. 术语锚点

本文档新增（共享术语见 [seam-capabilities](seam-capabilities.md) §0）：

- **任务（task）**：组件实例内一条在飞执行流 = 一次 export 调用及其 guest 内派生；同一瞬间至多一条任务在 CPU 上，切换仅在挂起点（W4.1）。
- **挂起点**：guest 任务让出执行权的点 = 任意 await 处；宿主只见 import 边界的挂起与唤醒（W4.1）。
- **在飞任务**：已接纳（邮箱投递或激活管线直调）未完结的任务；背压预算面 = 每实例在飞任务数封顶（W4.3；`setup` 直调也记入在飞预算，W1.4）。
- **重入（reentrancy）**：前一条 export 调用挂起期间，后一条进入同一实例起跑新任务（W4.1；实证 = S2 spike，[wasm-research](../research/wasm-research.md) 补遗 7）。
- **邮箱 / worker**：actor 基线件（[ADR-0011](../decisions/0011-actor-execution-model.md)），host 侧每插件实例各一份；投递语义 = W4.3。
- **world（固定）**：组件模型的接口契约单元，声明 import（宿主能力面）与 export（宿主调用插件的入口）；本项目单一固定 world，export 集在单一 package version 内封闭（W1.2/W1.3）。
- **生命周期导出集**：固定 world 中宿主在实例生命周期上调用的导出 = `setup` / `on-config` / `dispose`（W1.3）。
- **实例即弃**：热换/卸载时整个组件实例（Store+Instance）随弃，新实例干净启动（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §1；卸载交接 = W4.5）。

## 章节目录（W 系列，随设计批逐章开章）

| 章  | 题                                                                                                                                | 前身（A 系列 = 语义平移源） | 状态                       |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------- | -------------------------- |
| W1  | 插件包形态与入口契约（包布局 / world 定义 / 生命周期导出）                                                                        | A1                          | ✅ 2026-10-18（D116–D124） |
| W2  | manifest 格式                                                                                                                     | A2                          | ✅ 2026-10-19（D125–D134） |
| W3  | ctx 形态：WIT import 接口集与 SDK 映射（作者不见 WASI/WIT = ADR-0034 §4）                                                         | A3                          | ✅ 2026-10-19（D142–D154） |
| W4  | async 与交错语义（组件模型 async 映射；S1/S2 spike 双 PASS 托底 = 补遗 6/7）                                                      | A4                          | ✅ 2026-10-17              |
| W5  | 错误与 result 习语（`result<T,E>` 原生）                                                                                          | A5                          | ✅ 2026-10-19（D136–D141） |
| W6  | 边界转换：canonical ABI / 递归 Value 线编码（TLV 主线化 = E1 选项① 拍板）/ resource 句柄族（blob / byte-stream / 双车道 payload） | A6                          | ✅ 2026-10-19（D155–D162） |
| W7  | 契约与类型来源（① 平台封闭面 = WIT 富类型原生；A7 半消融 = E5 拍板：瘦身形状 DSL 继承 Value 形状面）                              | A7                          | ✅ 2026-10-19（D163–D172） |
| W8  | 事件静态声明（扁平名 + `internal/` 保留；声明 = 审批面；`on-event` 导出）                                                         | A8                          | ✅ 2026-10-19（D173–D179） |
| W9  | deps/provide 与能力门（进口对账机械化）                                                                                           | A9                          | ✅ 2026-10-19（D180–D185） |
| W10 | 工具形态与 ABI 版本纪律（WIT package version 承接 abi_rev）                                                                       | A10                         | ✅ 2026-10-19（D186–D193） |

---

## W1 插件包形态与入口契约

### W1.0 边界与前提

本章定：可部署包的物理布局与组件文件名；WIT 包身份与固定 world；生命周期导出集（`setup` / `on-config` / `dispose`）的 ABI 签名与调用语义；入口执行时序与激活门；`setup` 失败与加载预算；`dispose` 调用位；`start` 函数政策；零仪式插件的 ABI 面。不归本章：ctx import 接口集与 SDK 映射 = W3；错误类型的 WIT 定义 = W5；事件声明 = W8；契约与类型来源 = W7；manifest 字段形状 = W2；ABI 版本比较语义 = W10；组件运行时（Store/Instance、限额、fuel/epoch）= T10.10。

前提（已定不议）：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §1）；作者不见 WASI/WIT（ADR-0034 §4）；插件形态 = 目录单形态（[config-format](config-format.md) D35）；插件身份 = 目录名（config-format §0.2，字段面归 W2）；接口数据三分法（E1①/E2①/E5①，2026-10-16 拍板）；实例内执行模型 = W4（D93–D97）；配置值管线 = config-format D36/D42。

| 节    | 议题                                | 状态               |
| ----- | ----------------------------------- | ------------------ |
| W1.1  | 可部署包布局与组件文件名            | ✅ D116 2026-10-18 |
| W1.2  | WIT 包、world 与物理布局            | ✅ D117 2026-10-18 |
| W1.3  | 生命周期导出集与入口契约            | ✅ D118 2026-10-18 |
| W1.4  | 入口执行时序与激活门                | ✅ D119 2026-10-18 |
| W1.5  | `setup` 失败、加载预算与半成品处置  | ✅ D120 2026-10-18 |
| W1.6  | `dispose` 调用位与预算              | ✅ D121 2026-10-18 |
| W1.7  | `start` 函数与组件形态政策          | ✅ D122 2026-10-18 |
| W1.8  | 零仪式插件的 ABI 面                 | ✅ D123 2026-10-18 |
| W1.9  | f2/D2 兑现：ADR-0005 成本账局部取代 | ✅ D124 2026-10-18 |
| W1.10 | A1 机制消亡清单（考古索引）         | ✅ 2026-10-18      |
| W1.11 | 联动登记汇总（章末）                | ✅ 2026-10-18      |

### W1.1 D116 可部署包布局与组件文件名

**D116 定案：可部署包 = `manifest.yml`（可选）+ 一个组件文件（缺省 `plugin.wasm`，可由 manifest 指定）+ `lib/`（可选，随包契约，格式归 W7）。**

- 部署根插件目录 `plugins/<name>/` 分三区（[config-format](config-format.md) D29/D35）：包内容区 = `manifest.yml` + 组件文件 + `lib/`（随包契约）；部署区 = `config.yml`；运行期区 = `data/`。部署级契约包 `lib/` 另住部署根，与插件内随包 `lib/` 同名不同层。
- 组件文件名缺省 `plugin.wasm`；manifest 可给包内相对路径指定其他文件名（字段名与形状 = W2 D127）。指定值必须是包根下的单个相对路径分量（不进入 `lib/` / `data/`，不含 `..`、绝对路径、符号链接与大小写折叠碰撞），且目标必须是 wasm 组件（`.wasm` 扩展或组件 magic 校验；非组件 = 安装错误）。一个包只有一个插件组件：包内容区（不含 `lib/` 与 `data/`）内除被指名的组件外若还有 `.wasm` = 安装期响亮错误；未指定时 `plugin.wasm` 必须存在，其他 `.wasm` 同理。
- 组件文件以外的未知文件：保留、不解释、不加载（作者笔记、README 等）；安装期校验包内容的大小与文件数上限（SSOT = T10.12，构建预检复用同一常量）；升级时随包内容整体替换（D35）；卸载按目录整体处置（config-format C7）。
- 构建产物 `dist/<name>/` 与部署根可安装子集同构（[build-pipeline](build-pipeline.md) D108）：`manifest.yml` + 组件文件 + `lib/`（可选）；可选 `build-info.json` = 诊断 sidecar，不安装。`metis build` 对无 manifest 源码合成最小 manifest（D108/D35）；部署根仍接受手工放置的无 manifest 零仪式包（W1.8）。
- 插件身份 = 目录名；manifest 不承载身份（config-format D31；W2）。
- 组件内 `metis.build` custom section 只作诊断与溯源，不是信任锚，也不是元数据 SSOT（build-pipeline D112）。
- **W9 注（2026-10-19）**：本节「安装期响亮错误」按 W9.1 D180 读 = 安装期检出并记录；休眠 / 未装配件不拖垮启动，被启用时以记录原因响亮（W9.1 的两分政策）。

### W1.2 D117 WIT 包、world 与物理布局

**D117 定案：package = `metis:plugin@<abi-version>`，world = `plugin`，WIT SSOT = `crates/metis-plugin/wit/`。**

- **名字**：package `metis:plugin`，world `plugin`，v1 版本号 `0.1.0`。版本号语义、比较规则与兼容判定 = W10 D187；本章只定名字与版本字符串的住处。
- **物理位置**：WIT 目录住 SDK crate（`crates/metis-plugin/wit/`）——SDK 是作者实际依赖、将来要按 crate 发布的单元，Cargo 只打包 crate 目录内的文件，WIT 放在里面才能随 SDK 一起走。宿主 `metis-component` 以源码路径引用同一目录生成绑定（Zed 式两视角：guest 只对当前版本生成，宿主对全部支持版本生成）；两组绑定必须来自同一目录树，禁止拷贝第二份。这是工作区内源码级路径引用，不产生 Cargo 依赖（`metis-component` 不依赖 guest 侧 SDK）；若宿主 crate 将来需要独立发布，再评估中立 WIT crate 或构建期复制。
- **文件按章拆**：规划中 `plugin.wit`（world + 生命周期导出）；后续 `error.wit`（W5）、`ctx.wit`（W3）、`events.wit`（W3 import 面 + W8 声明面）、`blob.wit` / `streams.wit` / `payload.wit`（W6）加入同一 package 目录。WIT 允许多文件同包。
- **固定 world**：单一 world，import 词汇表在一个 package version 内封闭（[plugin-sdk](plugin-sdk.md) S6 已定）；增补 import / export = 新 package version（W10 D188）。宿主与插件不生成按能力裁剪的 world（编译期组合 world = 扩展位，前提 = W2 落地）。
- **版本快照**：W10 D189 定 = `crates/metis-plugin/wit/<version>/` 每支持版本一个子目录，guest 只对当前快照生成、宿主对全部支持快照生成；D190 定首个冻结快照。旧组件与新宿主的兼容判定 = W10 D187 三合一（声明 / 结构 / 词汇表；本条原「≤ 宿主支持版本」按「∈ 集合」读，下界 = `0.1.0`、上界 = `current`）。

### W1.3 D118 生命周期导出集与入口契约

**D118 定案：固定 world 的生命周期导出 = `setup` / `on-config` / `dispose`，全 async；config 以 TLV 过边界；服务调用与事件 payload 走 W6 的 `payload` 双车道。**

签名示意（非可编译 WIT：`list<u8>` = TLV 编码的 Value；`payload` = W6 D160 的 value / blob 变体；`error` = W5 D136 的共享错误类型）：

```text
package metis:plugin@0.1.0;

world plugin {
    export setup: async func(config: list<u8>) -> result<_, error>;
    export on-config: async func(config: list<u8>) -> result<bool, error>;
    export dispose: async func() -> result<_, error>;
    export on-call: async func(service: string, method: string, args: payload) -> result<payload, error>;  // W3 D147 + W6 D160
    export on-timer: async func(id: u64, coalesced: u32);                                                     // W3 D145
    export on-event: async func(listener-id: u64, name: string, payload: payload) -> result<option<payload>, error>; // W8 D177
}
```

- **W3 增补（2026-10-19）**：`on-call` / `on-timer` 是非生命周期导出，随 W3 D147 / D145 加入 v1 world；`on-event` = W8 D177。`0.1.0` 尚未发布，本节草图与 W3 同批补全 = 首个发布快照的编制（W10 D190）；发布后的增补才适用「新 package version」纪律。**W6 增补（2026-10-19）**：`on-call` 的 args / 返回改 `payload`（D160）；config 仍是 TLV。**W8 增补（2026-10-19）**：`on-event` 定形并进入 required 导出集（D177）；SDK 默认 `Ok(None)`。

- **`setup(config)`**：拿启用期合成并冻结的完整配置（config-format D36 管线产物：校验 → 秘密解析 → 默认值合成），成功返回 = 实例可以进入 Active。返回 `error` = 加载失败（W1.5）。
- **`on-config(config)`**：配置热更新通道（config-format D42）。`Ok(true)` = live-apply 成交；`Ok(false)` / `Err` / trap = 重挂载。默认实现 = `Ok(false)`（SDK 补全），所以「没实现」与「拒绝吸收」是同一行为。
- **`dispose()`**：唯一的手动清理通道（A1.2 平移）。在飞任务结算后由宿主调用（W1.6）；可 await ctx 效应调用；错误 / trap 记日志、不中断后续清理。
- **无 ctx 参数**：ctx = 宿主 import 接口集（W3）；SDK 在作者面合成 `Ctx`，作者签名形如 `async fn setup(&mut self, ctx: &Ctx, config: Value) -> Result<(), Error>`。ABI 不重复传递宿主能力表。
- **world 导出集在单一 package version 内封闭**：组件必须导出生命周期三者与 W3 的 `on-call` / `on-timer` / `on-event`（W8 D177）；缺任一 = 安装期 world 不匹配错误。作者面的「可选钩子」= trait 默认方法，不是缺失导出。发布后的增补 = 新 package version（W10 D188）。
- **元数据唯一去处 = manifest**：组件 custom section 不作声明面（A1.1「table 字段集封闭」的载体平移）。

### W1.4 D119 入口执行时序与激活门

**D119 定案：`setup` 由激活管线直调；Loading 期间投递闸门关闭，Active 后开门。**

本节只记入口契约对 ABI 调用的约束；投递与背压语义的 SSOT = W4.3，配置变更语义 = config-format D42。

- 顺序：实例化（imports 已注册）→ 宿主直调 `setup` → 成功：fiber 进 Active、闸门开门 → 邮箱积压消息按既有投递政策消费（W4.3）。失败：W1.5。
- `setup` 不走邮箱、不过投递闸门，但按在飞预算记账（第一个任务）。Loading 期间事件 / 服务调用进入有界邮箱积压（不投递、不丢弃）；溢出走既有背压链（[context-events](context-events.md) 政策；W4.3）。定时器 tick 按 W3 D145 只置 pending / 合并计数，Active 后投递。
- `setup` 期间无第二条 export 进入同一实例：同实例重入（W4.1）从 Active 起成立。`setup` 可以 await import（宿主调用 / 服务调用 / 定时器）；死锁红线沿 A1.4 平移——只能等「完成不依赖自身邮箱」的对象，等自己的 tick / 事件 = 死锁。
- `on-config` 撞 Loading = 合并取最新，Active 后投递；热更撞 Loading = 惯性原则：进行中的加载跑完落定再响应（语义 SSOT = config-format D42）。
- boot、热更、mount 走同一条激活管线（[instance-model](instance-model.md) D63；[ADR-0023](../decisions/0023-core-restart-semantics.md)）。

### W1.5 D120 `setup` 失败、加载预算与半成品处置

**D120 定案：三类失败同归 Failed；实例即弃 + 宿主账本 drain；不调 `dispose`。**

- **失败三类**：返回 `Err`；trap / panic（wasm abort）；加载总预算耗尽。
- **处置**：fiber → Failed（原因进 host tracing 与 `fiber_state_changed` 族）；实例即弃；宿主账本 drain `setup` 期间注册的半成品（监听 / 定时器 / 服务 / 句柄）；不调用 `dispose`——半初始化态的清理语义不可靠，且 A1.2 的 disposer 只在 `setup` 成功返回后才成立。
- **Loading 期积压处置**：`setup` 失败时，加载期间积压在邮箱的事件 / 服务调用 / timer tick 一并丢弃——事件按投递失败正常跳过，服务调用按 D11.3 即错回 `Err(Unavailable)`，timer tick 丢弃（D11.4 同族）。
- **配置校验失败不算 `setup` 失败**：boot / 启用期发生在 `setup` 之前（D36：插件代码一行未跑），走启用期响亮失败；reload 期走 D42 的拒绝路径（运行中实例持旧配置）。
- **加载总预算** = 实例化 + `setup` 的墙钟总预算；数值 = 实现期配置（登记 T10.10/T10.14）。store 资源上限（内存 / 表 / 组件大小）归 T10.10；fuel/epoch 逐调用生效（W4.4）。
- **不自动重试**：恢复语义 = 人工修复 / 显式 reload（[ADR-0018](../decisions/0018-service-registry.md) S4）。

### W1.6 D121 `dispose` 调用位与预算

**D121 定案：guest `dispose` 调用插入卸载时序的「优雅窗口」与「实例即弃」之间（完整五段 = W4.5 D97）；正常路径必调一次，强制路径跳过。**

- `dispose` 由宿主直调（不走邮箱、不过投递闸门），有独立墙钟预算（数值 = 实现期配置）。正常路径 = 在飞任务全部结算后调用一次。
- 调用时宿主账本仍在（注册尚未摘除），因此可以 await 效应调用（flush / 持久化 / 告别消息）；新注册「比我长寿」的账仍拒绝（W4.5 D11.2），新句柄允许但 drain 强 close 兜底。
- 错误 / trap：记日志，不中断后续 drain（A1.2「逐个吞掉、不中断其他清理」的平移）。宿主保证至多调用一次（fiber 状态机 + 实例即弃）。
- **跳过条件**：仅当优雅窗口耗尽且在飞任务仍未结算时跳过 `dispose`（串行不重叠纪律优先），直接实例即弃，journal 记强杀点（D11.5 既有面）；窗口内任务已全部结算 = 正常路径，必调 `dispose`。
- **`dispose` 预算耗尽**：实例即弃，journal 记强杀点，宿主不再等待；后续 drain 照常（宿主账本清理不依赖 guest 返回）。
- `setup` 失败不调用（W1.5）；被强杀的实例不调用。
- 五段时序的 SSOT = W4.5 D97；本节只定 `dispose` 调用面的契约。

### W1.7 D122 `start` 函数与组件形态政策

**D122 定案：只接受 reactor 组件；`start` 与命令类导出一律拒绝。**

- 合法组件 = 精确导出固定 world 的 reactor：无 component / module start 函数、无 `wasi:cli/run` / `_start` 命令导出。
- 理由：start 在实例化时执行，早于激活门、加载预算与宿主监督；import 级能力门虽已在链接期登记，但 start 可执行任意代码、产生副作用，且其失败不在受管生命周期内——等于给作者一条绕过 `setup` 的执行通道。
- 检查点：`metis check` / `metis build` 预检（[build-pipeline](build-pipeline.md) D108）与安装期结构校验（T10.12）共用同一判定；违反 = 响亮错误。
- 额外 export（调试函数等）忽略（宿主只调用 world 导出）；命令类导出按上条拒绝。
- WASI 0.3 基线导入（clocks / random / filesystem async + std 拖带集）不受影响（它们是 import，不是 start；[plugin-sdk](plugin-sdk.md) S3 既定）。

### W1.8 D123 零仪式插件的 ABI 面

**D123 定案：零仪式 = 无 manifest 的目录 + 同一固定 world。**

- 物理形态：`plugins/<name>/plugin.wasm`（缺省名；零仪式无 manifest，故不可改名），无 `manifest.yml`。经 `metis build` 产出的零仪式源码包带合成的最小 manifest（[build-pipeline](build-pipeline.md) D108），不改变本节 ABI 面。
- ABI 面不变：三个生命周期导出恒在场（SDK 默认补全）；config = 空 map；无 config schema、无能力声明面；private fs 结构性授予读写（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）；global / http / process / mount 等效应能力 fail-closed；imports 仍按固定 world 对账；无 provide/inject；不可 mount（config-format D35；[instance-model](instance-model.md) D63）。
- 兼容判定：无 manifest 即无 package version 声明面；零仪式组件只在结构匹配宿主当前 world 时被接受（world 不匹配 = 安装错误）。ABI 版本演进后，旧零仪式包须加 manifest 声明版本（W2/W10 D187：零仪式仅当前快照）。
- 手写组件的导出集必须匹配 world；缺导出 = 安装期错误，不因零仪式放宽。
- 成长路径：加 `manifest.yml`，组件文件不动；能力面按 manifest 声明启用。

### W1.9 D124 f2/D2 兑现：ADR-0005 成本账局部取代

**D124 定案：ADR-0005 的决策本体（每插件独立运行时实例）不变；Consequences 的载体成本账由新 ADR 局部取代。**

- 产出 = [ADR-0037](../decisions/0037-component-instance-cost-ledger.md)：VA = 槽 ×（预留 + guard）、RSS ≤ 槽 × max；实测 pooling 实例化 1.7 µs、限额稳态 0.999×、drop 即归还。
- ADR-0005 Status 行记录 partial supersede；正文冻结。
- 其余 Consequences 不变（事件 payload 可序列化；跨插件数据共享不可能）。

### W1.10 A1 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史与归档设计文档。

| 机制                                                           | 死因                                                                                                                                       |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `plugin()` 糖 / 入口返回 table / 裸函数报错（A1.1）            | 入口形状 = 固定 world 的导出集；作者面糖归 SDK 宏（S1），ABI 无入口返回物                                                                  |
| `setup` 返回 disposer 函数（A1.2）                             | 跨组件边界不能传闭包；手动清理独立为 `dispose` 导出（D118/D121）                                                                           |
| table 字段集封闭 / `on_config` 作可选字段（A1.1）              | world 导出集封闭；`on-config` 是恒在导出，可选性 = SDK 默认实现；元数据唯一去处 = manifest                                                 |
| 单文件形态与成长路径搬家（A1.3）                               | 目录单形态已由 D35 定；成长路径 = 加 manifest，组件文件不动（D116/D123）                                                                   |
| require 体系 / `.luaurc` / 入口模块缓存 / 循环 require（A1.5） | guest 是编译产物，模块解析在编译期完成；机制无对应物（已落地 = [ADR-0038](../decisions/0038-module-system-require-supersede.md)，W2 D134） |
| 加载期「编译入口 chunk」步骤与字节码缓存（A1.5）               | 组件编译 / 实例化由 wasmtime 承担（`.cwasm` 缓存归 T10.10）                                                                                |
| `setup(ctx, config)` 的 ABI ctx 参数（A1.1）                   | imports 即 ctx（W3）；ABI 不传能力表（D118）；作者面 `ctx` 句柄保留（SDK 合成）                                                            |
| `ctx.on_cleanup(fn)` 注册式多清理扩展位（A1.2）                | guest 语言设施（Drop / 组合清理）自管；宿主效应账本仍在，只是不承载 guest 清理回调                                                         |

**语义平移保留**：A1.4 的「setup 里只能等完成不依赖自身邮箱的调用」红线不是消亡项，只是表述换载体（W1.4 D119）。

### W1.11 联动登记汇总（章末）

| 去向           | 内容                                                                                                                                                                                                             |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W2（T13-i）    | **已兑现（2026-10-19）**：manifest 字段全集与形状 = D125–D133；模块系统消亡 = D134/ADR-0038；fs 两档 = D129/D130/ADR-0039                                                                                        |
| W3（T13-j）    | **已兑现（2026-10-19）**：import 集 = W3 D142–D154（W6 增补 `blobs` / `streams` / `payloads`）；作者面 `Ctx` / 句柄 / RAII；`on-call` / `on-timer` 导出；fs 两档 / timer / 投递纪律 / 能力门；world 导出集补两款 |
| W6（T13-l）    | **已兑现（2026-10-19）= W6 D155–D162**：`on-call` args / 返回 = `payload`（value / blob）；world import 集 += `blobs` / `streams` / `payloads`；`setup` / `on-config` config 保持 TLV                            |
| W5（T13-k）    | **已兑现（2026-10-19）**：`error` 类型 = W5 D136–D141；生命周期返回的 `result` 形状确认（D118）                                                                                                                  |
| W7（T13-m）    | **已兑现（2026-10-19）= W7 D163–D172**：随包契约 `lib/`（包内容区的一部分，D35）；W1 只定 `manifest.yml` + 组件文件 + `lib/` 的包布局边界                                                                        |
| W8（T13-n）    | **已兑现（2026-10-19）= W8 D173–D179**：`on-event` 定形并进入 required 导出集；事件名 / `internal/` / 声明语义 / emit 校验 / 形状生命周期由 W8 承接（D118 的 additive 增补完成）                                 |
| W9（T13-o）    | **已兑现（2026-10-19）= W9 D180–D185**：安装期结构事实（导出集 / import 词汇表 / 组件文件）与启用期五格分界；W1.1 / W1.3 的「安装错误」按「检出并记录」读（W9.1）                                                |
| W10（T13-p）   | **已兑现（2026-10-19）= W10 D186–D193**：ABI 面清单；版本载体与三合一接受；bump 纪律；快照布局与多版本共存；首个冻结快照；版本面独立；钉版纪律；工具面判定落点                                                   |
| T10.10         | 实例化 / Store 限额 / fuel / epoch；加载预算与 dispose 预算的数值；`.cwasm` 缓存                                                                                                                                 |
| T10.12         | 安装期 start / world 校验；包内容复制与未知文件处置；卸载                                                                                                                                                        |
| T10.14         | SDK 默认实现（`setup` = `Ok(())`、`on-config` = `Ok(false)`、`dispose` = `Ok(())`、`on-event` = `Ok(None)`）；作者面签名；`Ctx` 合成                                                                             |
| T10.15         | 可部署包布局与组件文件名（D116）；预检 start 拒绝（D122）；零仪式源码合成 manifest                                                                                                                               |
| config-format  | D42 `on_config` 的 ABI 签名已兑现（D118）；D29 `plugin.wasm` 占位已兑现（D116）；D35 零仪式 ABI 面（D123）                                                                                                       |
| build-pipeline | D106/D108 的组件文件名 / 包布局指针已兑现（D116）；D108 start 政策已确认（D122）                                                                                                                                 |
| instance-model | D63 零仪式无 mount 资格（D123 原位）                                                                                                                                                                             |
| plugin-sdk     | S2 WIT 目录确认（D117）；S1 生命周期钩子形态（D118）；`on_config` 归 W1                                                                                                                                          |
| ADR-0005       | Consequences 成本账局部取代（[ADR-0037](../decisions/0037-component-instance-cost-ledger.md)，D124）                                                                                                             |
| 实现期配置     | 加载总预算（W1.5）；`dispose` 墙钟预算（W1.6）                                                                                                                                                                   |
| 扩展位         | 编译期组合 world（S6 登记，W2 已落地后维持不启用）；多组件包（W1/W2 包布局面）；多 ABI 版本共存（W10 D189 已兑现）                                                                                               |

---

## W2 manifest 格式

### W2.0 边界与前提

本章定 manifest 的物理形态、顶层字段全集与字段形状、校验模型、A2 机制消亡清单。不归本章：组件文件路径校验细则 = W1.1；config 值的落盘与管线 = config-format D36；瘦身形状 DSL 的语法与词汇 = W7；契约包格式与命名空间 = W7；事件静态声明语义 = W8；inject / provides / events 对账与能力门 = W9；ABI 版本比较 = W10；安装事务 = T10.12；工具面 = T10.15。

前提（已定不议）：插件唯一载体 = wasm 组件；身份 = 目录名（W1.1 D116；config-format D31）；manifest 可选（W1.8 D123）；config 值住 `config.yml`（D36）；接口数据三分法（E1①/E2①/E5①）；fs = private 隐式 + global 声明（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)，W2.6 D130）。

| 节    | 议题                               | 状态               |
| ----- | ---------------------------------- | ------------------ |
| W2.1  | manifest 形态与边界                | ✅ D125 2026-10-19 |
| W2.2  | 顶层字段全集与身份                 | ✅ D126 2026-10-19 |
| W2.3  | `component` 字段                   | ✅ D127 2026-10-19 |
| W2.4  | `config` schema                    | ✅ D128 2026-10-19 |
| W2.5  | `capabilities`                     | ✅ D129 2026-10-19 |
| W2.6  | 命名根撤回                         | ✅ D130 2026-10-19 |
| W2.7  | `provides` / `inject` / `events`   | ✅ D131 2026-10-19 |
| W2.8  | `abi` / `version` / `description`  | ✅ D132 2026-10-19 |
| W2.9  | 校验模型与「声明 ≠ 观察」          | ✅ D133 2026-10-19 |
| W2.10 | 模块系统消亡（ADR-0003 supersede） | ✅ D134 2026-10-19 |
| W2.11 | A2 机制消亡清单                    | ✅ 2026-10-19      |
| W2.12 | 联动登记汇总（章末）               | ✅ 2026-10-19      |

### W2.1 D125 manifest 形态与边界

**D125 定案：`manifest.yml` 住包内容根；受限 YAML 子集；可选；作者意图面，不是配置值仓库。**

- 文件 = 包内容根 `manifest.yml`，UTF-8，[ADR-0007](../decisions/0007-yaml-config-subset.md) 受限 YAML 子集；一包一份；无 manifest = 零仪式插件（W1.8 D123）。
- 边界：manifest = 插件的自述（作者意图 + 元数据）；配置值住 `config.yml`（[config-format](config-format.md) D36）；装配单住 `metis.yml`（D31/D32）；身份 = 目录名（W1.1 D116）。
- 未知字段（顶层与嵌套同政策）= 响亮报错，带字段路径 +「core 版本过旧？」提示；重复键 = parser 响亮；空 manifest = 合法（全部缺省）。
- manifest 永不由机器重写；构建合成最小 manifest 是唯一例外（W2.8 D132）。组件 custom section（`metis.build`）只作诊断 / 溯源，不是元数据 SSOT（W1.1；[build-pipeline](build-pipeline.md) D112）。
- 不立 manifest 自身的版本号；字段集演进随 WIT package version（W10 D188）。
- 读取时序：安装期解析 / 未知字段与类型校验 / 结构事实（world、import 词汇表）/ 登记部署索引；语义对账（config 校验、秘密解析、inject / provides / events 对账）= 启用期；能力门 = 运行期逐调用（对账时机两分 = W9 D180；config-format §0.2 同判；W2.9 D133）。

### W2.2 D126 顶层字段全集与身份

**D126 定案：顶层字段 = 封闭集；`name` 不立；身份恒 = 目录名。**

| 字段           | 缺省          | 一句                                                  |
| -------------- | ------------- | ----------------------------------------------------- |
| `description`  | 无            | 审批 / 展示用文本                                     |
| `version`      | 无            | 插件分发版本（semver 约束归任务 9）                   |
| `abi`          | 无            | 组件构建所对的 WIT package version（比较归 W10 D187） |
| `component`    | `plugin.wasm` | 包内组件文件（形状 = W1.1 D116）                      |
| `config`       | 无            | config schema（W2.4 D128）                            |
| `capabilities` | 无            | 能力声明（W2.5 D129）                                 |
| `provides`     | 无            | 服务面（W2.7 D131）                                   |
| `inject`       | 无            | 依赖面（W2.7 D131）                                   |
| `events`       | 无            | 事件声明面（W2.7 D131）                               |

- **`name` 不立**：身份恒 = 目录名（config-format D31；W1.1 D116）；crate 名只决定构建输出目录 / 安装目录名。
- 不在表内的顶层字段 = 未知字段响亮；嵌套条目同理。新增字段 = WIT package version additive（W10 D188），不另立 manifest 版本机制。
- 零仪式 = 无 manifest，ABI 面同 W1.8 D123；空 manifest 全缺省合法；成长路径 = 加 `manifest.yml`，组件文件不动。
- 有意不收：marketplace 元数据（authors / license / repository）归任务 9；`runtime` / `min_core` / `schedule` 不立（W2.11）；自由 `build.command` = build-pipeline D108 扩展位，不启用。

### W2.3 D127 `component` 字段

**D127 定案：`component` = 可选字符串，缺省 `plugin.wasm`；路径与组件形态校验 = W1.1 原位。**

- 取值必须是包根下的单个相对路径分量；不得进入 `lib/` / `data/`，不含 `..`、绝对路径、符号链接与大小写折叠碰撞；目标必须是 wasm 组件（`.wasm` 扩展或组件 magic）。包内容区（不含 `lib/` / `data/`）内除被指名的组件外若还有 `.wasm` = 安装期响亮错误。完整规则 SSOT = W1.1 D116，本节只定字段名与缺省。
- 构建产物按 manifest 声明的文件名落盘；合成 manifest 使用缺省名（W1.1 D116；W1.8 D123）。
- 不立 `entry` / 模块入口字段；组件内部模块由 guest 编译期解决（W2.10 D134）。

### W2.4 D128 `config` schema

**D128 定案：`config` = 键 → `{ type, required, default, secret }`；`type` 必填；配置值以 TLV 过边界。**

```yaml
config:
    api_key:
        type: string
        required: true
        secret: true
    refresh_interval:
        type: integer
        default: 60
```

- 条目字段 = 封闭集；未知字段响亮。
- `type` **必填**，取值 = 瘦身形状 DSL 表达式（W7 定语法与词汇；要任意值就写 `any`）。
- `required` 缺省 `false`；`default` 为字面量 Value（`{ $value: ... }` 写在 `default` = 响亮）；`secret` 缺省 `false`。`volatile` 已退役（config-format D36/D37）。
- 冲突组合响亮：`required: true` + `default`；`secret: true` + `default`；`secret: true` 且 `type` 非 `string` / `any`（secrets store 值仅字符串，[config-format](config-format.md) C6.1 D46）。
- 共享变量引用 = `{ $value: "name" }`，覆盖 secret 与非 secret（config-format C6.4 D135；[ADR-0040](../decisions/0040-config-shared-variables.md)）。
  - `secret: true` 键：键存在时必须是 `{ $value: "name" }` 且解析自 `secrets.yml`；写明文 / 解析自 `values.yml` = 响亮。`required: false` 时键可缺席（缺席 = 无值，不做引用检查）；`required: true` 时缺席 = 响亮。
  - 非 secret 键：字面量合法；写 `{ $value: "name" }` 时必须解析自 `values.yml`；引用只存在于 `secrets.yml` 的名字 = 响亮。
  - 引用查无 = 响亮；两 store 同名 = 响亮。
- 管线：读 `config.yml` → schema 识别引用形状与 `secret` 标记（缺 required / 类型错 / 多给键 = 响亮）→ 共享变量解析 → 最终类型校验 → 默认值合成 → 冻结 → TLV → `setup`；插件代码一行未跑（config-format D36/D135；W1.3 D118）。
- `config` 缺省 ≡ 空 schema；此时 `config.yml` 出现任何键 = 多给键响亮。
- 一等 `schedule` / cron 字段维持不立（config-format C8.2 D51）；周期意图 = config 键约定。
- 嵌套字段级 `secret` 标记 = 扩展位（v1 只标顶层键）。

### W2.5 D129 `capabilities`

**D129 定案：`capabilities` = `{ fs, http, process, mount }`；private fs 隐式读写；`fs` 标量 = global 声明；sqlite 能力撤回。**

```yaml
capabilities:
    fs: read
    http:
        - api.openai.com
        - "*.anthropic.com"
    process: true
    mount: true
```

- `capabilities` = map，封闭键 `fs` / `http` / `process` / `mount`；未知键响亮；节缺省 = 无该能力；fail-closed 不变。
- `fs` = 标量 `read|write`（write ⊃ read），语义 = **global 档**；不写 `global` 子键，也不写 `private`。`fs` 缺省 = 无 global。**private 隐式**：`plugins/<name>/data/fs/` 读写不声明、不可收窄（W2.6 D130；[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）。
- `http` = host 列表：精确 host + `*.` 任意深度子域（不含 apex，apex 须单列）、不表达端口、https 缺省、明文须 `http://` 显式；SSRF / 跨域重定向 / 最终解析结果判定 = [seam-capabilities](seam-capabilities.md) §5.2 原位。空列表 = 无 host。
- `process` / `mount` 值为布尔：`true` = 启用；`false` / 缺省 = 无能力（拒绝）。timer 无门；纯算件无需声明。
- **sqlite 不立**：`sqlite` 键 = 未知字段响亮。宿主不代持 DB 句柄、不提供 SQL 方法族 / `sql.*` 错误族 / `sql_call` / `sql_result` journal kind 的新设计面。插件需要数据库时自带 guest crate（SQLite / redb / sled 等），库文件住 `data/fs/` 内，由插件自己管理 schema、迁移与生命周期；private fs 隐式读写使这条路径无需能力声明。
- **mount**：`true` = 允许调用 `ctx.mount` 创建子树实例（激活新代码的元权限；[instance-model](instance-model.md) §S3.3 D63；[ADR-0028](../decisions/0028-instance-model.md) 批次 #1）；`false` / 缺省 = 拒绝。它是编排面能力门的第一个例外（当前唯一）。
- 声明是作者意图；运行期门 fail-closed；能力词汇 additive 增补 = WIT package version（W10 D188）。审批面逐条展示；global 最高档。

### W2.6 D130 命名根撤回

**D130 定案：命名根不立；manifest 不设 `roots` 字段，部署根不设 `fs_roots` 绑定节。**

- 原命名根 = ADR-0036 的「manifest 声明逻辑名 + 部署者绑定路径」中间档。撤回理由：插件可能同时服务多个 project，目标目录也可能随运行期变化；静态绑定一次配不齐，中间档的成本（声明 + 绑定 + 校验 + 审批）超过收益（ADR-0039）。
- fs 收窄为两档：private 隐式读写 + `fs: read|write` 标量（语义 = global；W2.5 D129）。需要用户世界访问的插件写 `fs`；动态 project 目录由插件自己的 config 值承载（config-format D36/D45 同族）。
- 代价明记：没有中间档；global 是高信任档，部署者以审批与 OS 级沙箱兜底。workspace / session 语义仍归插件层服务投影，核心不做一一映射。
- 局部 supersede ADR-0036 Decision 第 2 条 = [ADR-0039](../decisions/0039-fs-capability-two-tier.md)；config-format C5、seam-capabilities §5.2、W1.8 D123 措辞同步。

### W2.7 D131 `provides` / `inject` / `events`

**D131 定案：provides / inject / events 只承载声明与契约引用；形状住契约包，不写进 manifest。**

```yaml
provides:
    memory:
        contract: memory # 显式引用平台内嵌契约；不写 = contractless（无形状对账）
    search:
        contract: search@2 # 生态契约带 @N（W7）
inject:
    required: [llm, memory]
    optional: [cache]
events:
    publish: [some/event] # 名字须在契约 events: 中有定义（W8 D174）
    listen: [another/event] # listen 允许无定义名（开放词汇）
```

- `provides` = map 服务名 → 条目；条目封闭字段 = `{ contract }`（可选字符串；**缺省 = 无契约（contractless）**，不默认解析为服务名；显式引用查无 = 启用期响亮；`@N` 规则归 W7）；`methods` 字段消亡（方法集 = 契约数据）；`null` ≡ `{}`。（2026-10-19 W7 修订：缺省语义由「服务名」改为 contractless——否则无契约私有服务无法表达。）
- `inject` = map，封闭字段 `{ required, optional }`，值为服务名列表；缺省空；required 缺失 = 启用期 Pending / 响亮（W9）；判环 = config-format C9.1 D52。
- `events` = map，封闭字段 `{ publish, listen }`，值为事件名列表；形状不写在这里，住契约 `types:` / `events:`（W7）；声明语义、`internal/` 规则与运行期强制程度 = W8 D173–D179。**W8 兑现（2026-10-19）**：声明 = 审批面，运行期不强制；`publish` 名须有形状出处，`listen` 任意名合法。
- **作者自定义 schema 的住处 = 契约包**：服务方法集、业务 data 形状、事件形状由契约定义；三来源 = 平台内嵌 / 部署 `lib/` / 插件随包（包内容区 `lib/`，W1.1 D116）；格式与命名空间合并 = W7，事件语义 = W8。manifest 只引用契约名与事件名，不重复声明形状。
- 无契约也可提供私有服务 / 发事件，但失去形状对账（W9）；同名不同 schema = 启用期响亮；同名同 schema = 可替换实现（seam 三角）。
- 服务名扁平；`plugin-name/service` 前缀仅约定不强制；撞名判定 = W9。契约治理三层与契约包版本规则 = W7 原位，本章不转抄。

### W2.8 D132 `abi` / `version` / `description`

**D132 定案：`abi` = 可选 WIT package version 声明；`version` = 插件分发版本；`abi_rev` 消亡。**

- `abi` = 可选字符串，组件构建所对的 `metis:plugin` WIT package version（如 `"0.1.0"`）；比较 / 接受条件 = W10 D187 三合一（声明 / 结构 / 词汇表）。缺省 = 无版本声明；接受条件 = 结构匹配宿主当前 world（与零仪式同规则）。
- `version` = 插件自身分发版本，可选；v1 不强制 semver（任务 9）；不是身份，与 `abi` 无关。
- `description` = 可选自由文本。
- 构建合成最小 manifest = `version: 0.1.0` + `abi: <构建时的 WIT package version>`；不含 `name`；安装目录名取 crate 名 / `dist/<name>`（[build-pipeline](build-pipeline.md) D108 同步）。
- `abi_rev` 字段消亡；ADR-0028 的「`abi_rev` +1」只作历史措辞，演进纪律由 W10 承接。
- marketplace 元数据 v1 不收；`min_core` 不立。

### W2.9 D133 校验模型与「声明 ≠ 观察」

**D133 定案：manifest 校验 = 纯数据单一实现、三面暴露；声明是意图，组件结构是事实。**

- 三面暴露：编辑器 JSON Schema 工件（yaml-language-server）→ `metis check` / `metis build` → loader 安装 / 启用期；三面措辞逐字一致（校验器单一实现、多面暴露原则；config-format C3.1 同 parser 同校验器）。
- 时机两分：安装期 = 解析 + 未知字段 / 类型校验 + 结构事实（world / import 词汇表）+ 登记部署索引；启用期 = config 校验、秘密解析、inject / provides / events 对账；能力门 = 运行期逐调用（W9 D180）。插件代码在任何对账前均未运行。
- 声明 ≠ 观察：manifest 承载作者意图（capabilities、required / default / secret、inject / provides / events）；组件 import 清单 = 结构事实；对账 = import 清单 ⊆ 宿主词汇表（安装期结构事实，W9 D180–D182），**不做**能力自动推断，也不以扫描结果反写 manifest。
- 元数据唯一去处 = manifest；组件 custom section 只作诊断 / 溯源（W1.1；build-pipeline D112）。
- 空 manifest 合法、全缺省；错误带字段路径 + typo 提示；YAML 受限子集违规响亮。
- `metis check` 部署模式另含 config-format C2.4 D34 的未装配件 / 休眠件提示，不属 manifest 校验本体。

### W2.10 D134 模块系统消亡（ADR-0003 supersede）

**D134 定案：ADR-0003 整篇 supersede = [ADR-0038](../decisions/0038-module-system-require-supersede.md)；运行期模块系统无承接物。**

- 死亡：require 包装器、`(importer, specifier)` 依赖边账本、反向传递闭包失效、插件内相对路径 import 规则、跨插件文件 require 的路径逃逸判定、共享 `lib/` 纯代码库、`@lib` 别名、`.luaurc`、模块缓存与循环 require 报错。
- 存活：跨插件运行期组合只走 inject / events；同一插件内部由 guest 编译期模块系统承担；契约变更级联 = 数据驱动反向索引（config-format D43 / W7）。
- `lib/` = 契约包（`contract.yml` + 文档），扫描进契约命名空间，不被 import 为代码（W1.1 D116；config-format D35；W7 D164 修正载体措辞）。
- [ADR-0019](../decisions/0019-crate-layout.md) C3 的 `@lib` 别名与 lib 两层机制同属死亡，正式 supersede 归 W7（T13-m）。
- W1.10 的 A1.5 考古行更新为「已落地 = ADR-0038」；`luau-abi` 归档正文不动。

### W2.11 A2 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与上述 ADR。

| 机制                                   | 死因                                                                                                                                      |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `name` 字段                            | 身份恒 = 目录名（config-format D31；W1.1 D116）；manifest 不承载身份                                                                      |
| `entry` 字段                           | 组件文件名由 `component` 承载（W1.1 D116；W2.3 D127）；无运行期模块入口                                                                   |
| `boxes` 字段与双向对账                 | 盒概念消解进插件本体（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §2）                                                            |
| `abi_rev` 字段                         | WIT package version 承接（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §3）；字段 = `abi`（W2.8 D132）                             |
| `provides.methods` 列表                | 方法集住契约（W7）；固定 world 经 `on-call` 分发（W3）                                                                                    |
| 旧 `fs` scope map（两档 → 三层命名根） | 现形态 = private 隐式 + `fs: read/write` 标量（global）；命名根撤回（W2.5/W2.6；[ADR-0039](../decisions/0039-fs-capability-two-tier.md)） |
| `sqlite` 能力 / `data/db.sqlite`       | 宿主代持 DB 无独立价值；插件自带 guest DB crate，库文件住 private fs，schema / 迁移自管（W2.5 D129；ADR-0039）                            |
| `volatile` 词汇                        | config-format D36/D37 退役；热应用 = `on-config` 返回值                                                                                   |
| `runtime` 字段                         | 从未立；wasm-first 后无组成可声明（ADR-0034）                                                                                             |
| `min_core` 字段                        | 不立；`abi` + 未知字段「core 版本过旧？」提示兜底（W2.8 D132）                                                                            |
| 一等 `schedule` / cron 字段            | 维持不立（config-format D51）；周期意图 = config 键约定                                                                                   |
| require / `.luaurc` / `@lib` / 边账本  | ADR-0003 整篇 supersede = [ADR-0038](../decisions/0038-module-system-require-supersede.md)                                                |

**语义平移保留**：插件的自述与意图（description / version / config schema / capabilities / provides / inject / events）仍唯一住 manifest；配置值仍唯一住 `config.yml`；身份仍 = 目录名；声明 ≠ 观察仍成立（W2.9 D133）。

### W2.12 联动登记汇总（章末）

| 去向            | 内容                                                                                                                                                                                                                                 |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W1（T13-h）     | `component` 缺省 `plugin.wasm`（D116）；零仪式 ABI 面（D123）措辞按 ADR-0039 更新：private fs 隐式，效应能力仍 fail-closed                                                                                                           |
| W3（T13-j）     | ctx.fs = private（隐式读写）/ global（`fs: read/write` 声明）；无命名根；global 能力门在运行期强制；DB = guest crate + private fs（无宿主 sqlite）                                                                                   |
| W5（T13-k）     | **已兑现（2026-10-19）**：生命周期返回的 `error` 类型 = W5 D136；manifest 校验错误是宿主侧错误，不经 guest 错误通道（D133）                                                                                                          |
| W7（T13-m）     | **已兑现（2026-10-19）= W7 D163–D172**：`type:` 表达式语法与词汇（D166）；契约包格式 / 三来源合并（D164/D165）；`provides.contract` 与 `@N`（D165）；作者自定义服务 / 事件 schema 的住处（D167）；作者类型由 `types!()` 生成（D170） |
| W8（T13-n）     | **已兑现（2026-10-19）= W8 D173–D179**：事件名语义 / `internal/` 规则 / 声明强制程度 / 形状总表 / emit 校验 / `on-event`；manifest 只列名字，形状住契约                                                                              |
| W9（T13-o）     | **已兑现（2026-10-19）= W9 D180–D185**：inject / provides / events 对账形态；能力门（含 `mount` 例外）；import 清单 ⊆ 宿主词汇表；方法集对账消亡；失败聚合；`metis check` 范围                                                       |
| W10（T13-p）    | **已兑现（2026-10-19）= W10 D186–D193**：`abi` 的精确声明与三合一接受（D187）；manifest 字段集 / 能力词汇演进 = D188；manifest 版本机制仍不立                                                                                        |
| config-format   | D36 配置管线按 `config` schema 消费；共享变量 = `{ $value: "name" }`（含 secret；C6.4 D135 / [ADR-0040](../decisions/0040-config-shared-variables.md)）；fs 两档（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）          |
| build-pipeline  | D108 合成最小 manifest = `version` + `abi`（无 `name`）；脚手架按 W2 字段集生成；组件文件名 = D116                                                                                                                                   |
| plugin-sdk      | S6 能力白名单 = manifest `capabilities` 声明源；编译期组合 world 维持扩展位（W2 已落地后仍不启用）                                                                                                                                   |
| instance-model  | `mount: true` 能力键与 `ctx.mount` 方法族（ADR-0028 批次 #1）                                                                                                                                                                        |
| ADR-0003        | 整篇 supersede = [ADR-0038](../decisions/0038-module-system-require-supersede.md)（D134）                                                                                                                                            |
| ADR-0036        | Decision 第 2 条「命名根」局部 supersede = [ADR-0039](../decisions/0039-fs-capability-two-tier.md)（D130）                                                                                                                           |
| ADR-0025 / 0027 | Status 行记：ADR-0025 §5 fs 通道条两档 → ADR-0036 三层 → ADR-0039 两档（private 隐式 / `fs` 标量 global），sqlite 项撤回；ADR-0027 D44 与 C1.1 `data/db.sqlite` 行由 ADR-0039 定型/失效                                              |
| ADR-0040        | 配置共享变量（`values.yml` + `secrets.yml`，`{ $value }` 含 secret）= C6.4 D135；W2 D128 引用语法已同步                                                                                                                              |
| T10.12          | 安装期 manifest 解析 / 未知字段 / 类型校验；world 与 start 校验；包内容复制                                                                                                                                                          |
| T10.14          | 作者面 trait / 宏；`Value` 手动面；能力声明消费                                                                                                                                                                                      |
| T10.15          | 脚手架 manifest 骨架；合成最小 manifest；构建期 manifest 校验                                                                                                                                                                        |
| 实现期清单      | manifest 校验器（三面同一实现）；JSON Schema 工件生成；部署索引解析登记；共享变量解析随 T10.8（C6.4 D135）                                                                                                                           |
| 扩展位          | 嵌套字段 `secret` / 嵌套 `$value` 引用；编译期组合 world；自由 `build.command`；marketplace 元数据                                                                                                                                   |

---

## W3 ctx 形态：WIT import 接口集与 SDK 映射

### W3.0 边界与前提

本章定：固定 world 的 import 接口集（分族、名字、归属）；每个接口的 WIT 形状（async / sync、富类型 vs TLV、错误面）；作者面 `Ctx` 与宿主能力的映射（访问器、句柄、RAII）；能力门在 import 实现侧的落点；投递纪律（背压 / 合并计数 / 卸载可见）；A3 机制消亡清单。不归本章：事件名语义与 `on-event` 导出 = W8；契约形状与方法集 = W7；TLV 线编码与 resource / stream 细节 = W6；错误类型定义 = W5；provide / inject / events 对账与能力门判定 = W9；package version 比较 = W10；宿主 import 实现 = T10.10 / T10.11；SDK 实现 = T10.14。

前提（已定不议）：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；作者不见 WASI / WIT（ADR-0034 §4）；**WASI 目标 = 0.3**（2026-10-19 维护者确认；async filesystem 是 D144 的地基）；固定 world（W1.2 D117）；接口数据三分法（E1① / E2① / E5①，2026-10-16 拍板）；执行模型 = W4（D93–D97）；错误面 = W5（D136–D141）；fs 两档 = [ADR-0039](../decisions/0039-fs-capability-two-tier.md)；sqlite 撤回；命名根撤回；**crypto 撤回 = [ADR-0041](../decisions/0041-crypto-capability-withdrawal.md)**（2026-10-19 本批）。

| 节    | 议题                                             | 状态               |
| ----- | ------------------------------------------------ | ------------------ |
| W3.1  | 能力面判据：只立 guest 不能自做的 import         | ✅ D142 2026-10-19 |
| W3.2  | 导入接口集与文件归属                             | ✅ D143 2026-10-19 |
| W3.3  | fs 两档：`fs` / `plugin_fs()`，统一 async 作者面 | ✅ D144 2026-10-19 |
| W3.4  | timer：sleep / after / every 与回调投递          | ✅ D145 2026-10-19 |
| W3.5  | events：宿主登记与四派发                         | ✅ D146 2026-10-19 |
| W3.6  | services：call / provide / deps 与 on-call       | ✅ D147 2026-10-19 |
| W3.7  | http：富类型请求 / 响应                          | ✅ D148 2026-10-19 |
| W3.8  | process：富类型与 resource 句柄                  | ✅ D149 2026-10-19 |
| W3.9  | instance：mount / unmount                        | ✅ D150 2026-10-19 |
| W3.10 | async / sync 分类                                | ✅ D151 2026-10-19 |
| W3.11 | 投递纪律：背压、合并计数、卸载可见               | ✅ D152 2026-10-19 |
| W3.12 | 作者面 `Ctx` 与句柄                              | ✅ D153 2026-10-19 |
| W3.13 | 能力门载体与 world 终态                          | ✅ D154 2026-10-19 |
| W3.14 | A3 机制消亡清单（考古索引）                      | ✅ 2026-10-19      |
| W3.15 | 联动登记汇总（章末）                             | ✅ 2026-10-19      |

### W3.1 D142 能力面判据：只立 guest 不能自做的 import

**D142 定案：import 判据 = 宿主中介必要性；纯算件归 guest 包生态，不立 ABI。**

- **判据**：只有 guest 不能自做的事进 metis import 面——OS 访问（fs）、网络（http）、时钟唤醒（timer）、子进程（process）、跨实例编排（events / services / instance）。纯算（json / yaml / regex / diff / uuid / hex / base64 / timefmt / crypto / DB）一律由 guest 自选 crate；核心不提供纯算内建件。
- **crypto 撤回**（2026-10-19 维护者拍板）：crypto 是 Lua 时代对解释器慢的补偿；wasm 下 guest 是编译语言，基础计算性能可信。sha256 / sha512 / hmac / AES-GCM 归 guest crate；CSPRNG 走 `wasi:random`（WASI 标准件，作者用 `getrandom` / `rand`，SDK 便利 re-export = feature 扩展位，v1 不做）。正式记录 = [ADR-0041](../decisions/0041-crypto-capability-withdrawal.md)；W5 的 crypto 错误面随撤回消失（参数违约不再有宿主侧入口）。
- **已消解项**：json / yaml / regex / diff / uuid / hex / base64 / timefmt = design-review B2；sqlite = [ADR-0039](../decisions/0039-fs-capability-two-tier.md)（DB = guest crate + private fs）；crypto = ADR-0041。
- **WASI 0.3 标准件**：clocks / random / filesystem（async）直接复用，不进 metis 词汇表；SDK 可便利 re-export，作者不见 WASI 名。`wasi:random` 是 CSPRNG 的唯一来源；private fs 的 async 面由 WASI 0.3 提供。
- **成本明记**：wasm 无 AES-NI / SHA-NI 直通，guest crypto 比原生慢约 3–10×；高吞吐 crypto 走外部服务 / T2 进程；宿主不再提供单一审计实现，crypto 供应链接管归插件作者。复审触发器 = 批量插件因 crypto 性能 / 审计问题成串（ADR-0041）。
- **timer 不是纯算件**：`sleep` 是宿主到时唤醒当前任务；`after` / `every` 的回调要活过一次 export 调用，宿主只能通过一次新的 export 调用送 tick。调度面必须宿主中介（W3.4）；这与「减少纯算跨界」不矛盾。

### W3.2 D143 导入接口集与文件归属

**D143 定案：metis 功能 import = 9 个按族接口（W6 D159 增补 `blobs` / `streams`）；private fs = WASI 0.3 `wasi:filesystem` 标准件（SDK 映射，不入本包）；类型面 = `errors` + `fs-types` + `payloads`；`events` 的 import 形状归 W3，W8 只加声明语义与 `on-event`。**

| 接口（功能面） | 内容                                                                 | WIT 文件      | 语义归属                             |
| -------------- | -------------------------------------------------------------------- | ------------- | ------------------------------------ |
| `events`       | `emit` / `serial` / `parallel` / `waterfall` / `listen` / `unlisten` | `events.wit`  | W3 形状；W8 名称 / 声明 / `on-event` |
| `services`     | `call` / `provide` / `deps`                                          | `ctx.wit`     | W3；对账 W9                          |
| `timer`        | `sleep` / `after` / `every` / `cancel`                               | `ctx.wit`     | W3                                   |
| `http`         | `request` / `download` + `stream-response`                           | `ctx.wit`     | W3；大负载 W6                        |
| `fs`           | global scope 的 11 方法 + `open-read` / `open-write` + 路径接口      | `ctx.wit`     | W3；大负载 W6                        |
| `process`      | `exec` / `spawn` + `process-handle`                                  | `ctx.wit`     | W3                                   |
| `instance`     | `mount` + `mount-handle`                                             | `ctx.wit`     | W3                                   |
| `blobs`        | `blob` resource + `from-bytes`                                       | `blob.wit`    | W6（大负载 / fanout）                |
| `streams`      | `byte-reader` / `byte-writer` resources                              | `streams.wit` | W6（大负载流）                       |

- **private fs 不在 metis 接口表**：`plugins/<name>/data/fs/` 走 WASI 0.3 `wasi:filesystem`（preopen；SDK 映射为 `ctx.plugin_fs()`）；计入 WASI 标准件，不计入 metis 功能 import。

- 类型面：`errors`（W5 D136）、`fs-types`（`entry-kind` / `dir-entry` / `file-metadata`，global `fs` 接口用）与 `payloads`（W6 `payload` 变体）住 `error.wit` / `ctx.wit` / `payload.wit`；只承载类型，不产生功能 import。private 侧用 WASI 0.3 的富类型，SDK 映射成同一 Rust `Fs` 面。
- **`fs` = global；private fs = WASI 0.3 `wasi:filesystem`（作者面 `ctx.plugin_fs()`）**（2026-10-19 维护者拍板）；`instance` = 动态子树挂载面（[instance-model](instance-model.md) D63 / D64），不是能力族。命名沿用 `instance`：将来 `list` / `describe` 可 additive 增补；`mount` 只是其中一个操作。
- **`events` 归属调整**：W1.2 原规划 `events.wit` 归 W8；但固定 world 的 import 词汇表必须在 W3 收口，`Ctx::emit` / `on` 也需要 ABI 锚。W3 写 import 形状，W8 写事件名语义、`internal/` 规则、契约形状与 `on-event` 导出。
- **不立聚合 `ctx` 接口**：W9 的进口对账粒度 = 接口（design-review B1），按族切才能让 import 清单可读、演进可局部；聚合接口会把所有能力焊在一个版本面上。
- **文件按章拆**（W1.2 D117 原位）：`plugin.wit`（world + 生命周期导出）/ `error.wit`（W5）/ `ctx.wit`（W3）/ `events.wit`（W3 import 面 + W8 声明面）/ `blob.wit`（W6）/ `streams.wit`（W6）/ `payload.wit`（W6 类型）。WIT 允许多文件同包。
- **world 导入全集**（示意，非可编译）：

```text
world plugin {
    import errors;     // 类型面（W5）
    import fs-types;   // 类型面（W3 fs）
    import payloads;   // 类型面（W6 value / blob）
    import events;
    import services;
    import timer;
    import http;
    import fs;          // global scope（host-mediated）
    import process;
    import instance;
    import blobs;       // W6：不可变大对象 / fanout
    import streams;     // W6：byte-reader / byte-writer
    // WASI 0.3：clocks / random / filesystem（async）由 guest std 与 SDK 引入；宿主统一链接，不在此列
}
```

- **固定 world 不裁剪**：所有功能接口恒在 import 清单里；能力门 = 运行期 fail-closed（W3.13），不是链接期裁剪。编译期组合 world 维持扩展位（plugin-sdk S6）。

### W3.3 D144 fs 两档：`fs` / `plugin_fs()`，统一 async 作者面

**D144 定案：`ctx.fs()` = global（声明门 + 绝对路径，host-mediated async）；`ctx.plugin_fs()` = private（隐式 + WASI 0.3 `wasi:filesystem` async over preopen）；两者同一 `Fs` 方法族、全部 async；private 不套宿主中介层，也不是 metis WIT 接口。**

- **private 机制**：`plugins/<name>/data/fs/` 以 WASI preopen 授予（guest 路径 `/`）；**作者面 = SDK 直接包 WASI 0.3 `wasi:filesystem` 的 async 文件面**（`ctx.plugin_fs()`），不另造 host-mediated 层——WASI 0.3 的 filesystem 本身 async，宿主中介是舍近求远。SQLite / redb 等 guest crate 用常规 `std::fs`（std 适配 WASI 0.3）；`ctx.plugin_fs()` 与 `std::fs` 同一 preopen，安全边界由 WASI 结构限界提供，不立第二套 resolver。
- **global 机制**：[ADR-0039](../decisions/0039-fs-capability-two-tier.md) 的 host-mediated 自研逃逸舱；`capabilities.fs: read / write` 门；绝对路径；两段式解析 + TOCTOU 硬化原位（[seam-capabilities](seam-capabilities.md) §5.2）——preopen 表达不了减法，这一档必须宿主中介。
- **版本前提**：目标 = WASI 0.3（2026-10-19 维护者确认）；`wasm32-wasip3` target / std p3 迁移的工具链落地归 T10.10 / T10.15（ADR-0034 W7 风险面）。S1/S2 spike 的 wasip2 证据只用于 async 语义，不改变 filesystem 的 0.3 async 面。
- **不提供同步 / 异步两套作者接口**：作者面统一 async；guest crate 的 `std::fs` 是标准库同步 API，底层由 std 适配到 WASI 0.3。
- **为什么两个访问器而不是一个 `fs`**：两档的安全模型不同——private 是隐式 preopen（结构限界），global 是声明门 + 路径政策（必须宿主中介）；`Fs` 方法族同形，但入口分开让「插件自己的数据目录」与「用户文件系统」在代码里一眼可辨，也让 private 走零中介快路径。
- **性能澄清**：两档都是一次 host 调用；差别在 global 多了宿主中介的路径解析、能力门与预算记账（µs 级开销），private 由 WASI 直接落盘、零中介。async 不降低单次往返延迟，但并发 IO 可以重叠、不占住调用方任务；把 private 也统一走 global 会把私有目录 IO 拖上中介与能力门，得不偿失。
- **方法族**（两 scope 同形，Rust-like）：

| 方法                             | 语义                                                                                                              |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `read` / `read_text`             | ABI 层 bytes-only（`read`）；`read_text` 是 SDK 便利方法（UTF-8 校验失败 = `fs-invalid-utf8`，SDK 产生）          |
| `write` / `append`               | 原子写（tmp + rename）/ 原子追加；自动建父目录                                                                    |
| `exists` / `metadata`            | `metadata` = `file-metadata { kind, len, modified-ms }`                                                           |
| `read_dir`                       | 返回 `list<dir-entry>`（`{ name, kind, len }`），带条目预算（超限 = `fs-too-large`）；glob / walk = guest 侧组合  |
| `open_read` / `open_write`       | W6 D159：返回 `byte-reader` / `byte-writer`；大文件分块读写，不整段物化；`open_write` close = tmp+rename 原子提交 |
| `create_dir_all`                 | 递归建目录                                                                                                        |
| `remove_file` / `remove_dir_all` | 删除；目录删除显式                                                                                                |
| `rename` / `copy`                | 同 scope 内；跨 scope = read + write 组合                                                                         |
| 路径接口                         | `home_dir` / `config_dir` / `data_dir` / `cache_dir` / `temp_dir`（`fs` 专属，绝对路径）                          |
| `guest_root`                     | `ctx.plugin_fs()` 专属，返回 `/`（preopen guest 路径，给 crate）                                                  |

- **路径规则**：`fs` 只接受绝对路径；相对路径 = `fs-invalid-path` Err。`ctx.plugin_fs()` 接受相对路径或 preopen-绝对路径（`/...`），两者都在私域根内解析。路径接口返回的绝对路径不授予访问权——访问仍过 `fs` 门。
- **范围与预算**：整文件读 / 写带预算（数值 = 实现期配置，登记 T10.11）；超限 = `fs-too-large`。大文件 / 流式 = W6 D159：`open_read` / `open_write` 返回 `byte-reader` / `byte-writer`，分块读写、不整段物化；`read` / `write` 一次性方法继续服务小文件。
- **错误映射**：两 scope 共用 W5 `fs` 族；`read_text` 的 UTF-8 失败 = `fs-invalid-utf8`（SDK 产生；ABI bytes-only，宿主不读文本）；private 侧 WASI 错误码 → fs kind、global 侧宿主错误 → fs kind，映射实现归 T10.14（not-found / permission-denied / invalid-type / too-large / io）。
- **ADR-0039 不变**：private 隐式、preopen 保留、global 标量声明；作者面 = SDK 直接包 WASI 0.3 async filesystem，不新增 host-mediated 私有层。

### W3.4 D145 timer：sleep / after / every 与回调投递

**D145 定案：`sleep` = await import；`after` / `every` = 宿主登记 + `on-timer(id, coalesced)` 导出回调；回调体在 wasm 内执行；RAII 管早停，宿主账本管卸载兜底。**

```wit
interface timer {
  sleep: async func(ms: u64);
  after: func(ms: u64) -> u64;
  every: func(ms: u64) -> u64;
  cancel: func(id: u64);
}
```

- **两条唤醒路径**（A3.6 载体平移）：`sleep` = 当前任务挂起，宿主到时直接恢复（不排队、不走邮箱）；`after` / `every` = 宿主时钟到期后发起一次新的 export 调用 `on-timer(id, coalesced)`，SDK 在 wasm 内查回调表并执行作者闭包。**回调体在 wasm 内**；宿主只持有 timer 条目（id / 间隔 / 状态），不持有 guest 闭包或指针。
- **为什么仍须宿主**：guest 是被动的；稳定 canonical ABI 没有 guest 自发后台任务。`after` / `every` 的回调要活过一次 export 调用，宿主只能通过一次新的 export 调用来送 tick——这是调度面，不是纯算件。
- **tick 投递**：tick 走 W4.3 的在飞预算闸门；每 timer 一个 pending 标记 + **非重叠**策略——上一个 tick 的 handler 未完结时，后续 tick 合并计数；下一次投递带 `coalesced`（0 = 无合并）。不排无界队列、不静默丢弃；作者显式看到合并次数。
- **RAII**：`TimerHandle` 包裹 id；`Drop` → `cancel`；`detach()` 把生命周期交给宿主账本（fire-and-forget / 不想持有句柄时）；`cancel()` 保留为提前停。**dispose 不需要手动清理 timer**：卸载五段（W4.5）关闸后不再投递，宿主 fiber 账本 LIFO 结清全部登记；RAII 只是早停层。
- **id 作用域**：timer id 是每实例作用域的 opaque token；宿主校验归属，外来 id = trap；自己的已取消 id = 幂等 no-op。`metis_plugin::raw` 直接调用也过同一校验。
- **回调生命周期**：SDK 要求 `'static`（必要时 `+ Send`）；借用数据必须 `move` / `Arc` / clone——语言层保证。回调表住实例线性内存，实例丢弃即消失；宿主保证不向已死实例投递，在飞 tick 按 W4.5 drain；trap = 实例死亡，timer 随账本取消。不存在跨边界悬垂。
- **错误面**：timer 无 `Err` 面（W5 D137）；`after` / `every` 间隔非法（低于最小间隔等）= trap（W5 D139）；回调返回 `Err` = SDK 记日志（fail-open），回调 panic = trap。
- **作者面**：`ctx.timer().after(ms, handler)` / `every(ms, handler)` 注册；handler = `impl AsyncFnMut(u32) -> Result<(), Error> + 'static`（参数 = 本次投递合并的 tick 数，0 = 无合并）；`Err` = SDK 记日志（fail-open），panic = trap。`sleep(ms).await` 直接可用；handler 可捕获零尺寸 `Ctx`。
- **Loading 期**：`setup` 内可注册 timer；tick 在 Loading 期只置 pending / 合并计数，Active 后投递（W1.4 既有「积压不丢弃」）；`setup` 失败 = 账本 drain、timer 取消（W1.5）。
- **卸载期**：Unloading 新注册 timer = trap（W4.5 D11.2）；已登记 timer 随账本取消；在飞 tick 按 W4.5 drain。

### W3.5 D146 events：宿主登记与四派发

**D146 定案：监听登记住宿主（`listen` / `unlisten`），四派发 = import；`emit` 也 async（只 await 入队）；`Listener` RAII。**

```wit
interface events {
  use errors.{error};
  use payloads.{payload};
  emit: async func(name: string, payload: payload) -> result<_, error>;
  serial: async func(name: string, payload: payload) -> result<option<payload>, error>;
  parallel: async func(name: string, payload: payload) -> result<list<result<option<payload>, error>>, error>;
  waterfall: async func(name: string, payload: payload) -> result<option<payload>, error>;
  listen: func(name: string) -> result<u64, error>;
  unlisten: func(id: u64);
}
```

- **为什么登记在宿主**：ADR-0014 的派发快照、注册序、逐项活性检查、卸载摘除都是宿主侧机制；闭包不能跨界，宿主必须知道「哪个实例听哪个事件」才能只投递给登记者。guest 侧只存 `name → 引用计数`（宿主 listener 表持有 id 与注册序）。
- **`emit` 为什么 async**：`await` 只等**入队接纳**，不等监听者完成——fire-and-forget 语义不变；邮箱满时把背压传导给发射方。**容量等待进 wait-for 图**：若多个在飞任务互相等待同一邮箱空位形成圈，宿主 fail-fast 返回 `core-deadlock` Err（W4.4 层三扩展）；`Ok` = 已入队。不静默丢弃。
- **监听者返回约定**（A3.4 平移）：`Ok(Some(payload))` = 有值；`Ok(None)` = 无值。`serial` 首个 `Some` 短路；`waterfall` `Some` 继续链 / `None` 否决；`parallel` 逐项收集；`emit` 忽略返回值。监听者 `Err` = 事件错误（serial / waterfall fail-open 记日志，parallel 逐项收，emit 记日志）；非 business 的 listener 错误 = 宿主判 `core-contract-violation`（W5 D138）。
- **作者面**：`Ctx::on(name) -> Result<Listener, Error>` 只做登记；事件处理走 S1 的 catch-all `Plugin::on_event(ctx, name, payload) -> Result<Option<Payload>, Error>`。**同名去重 + 引用计数**：一个实例一个宿主 listener 条目（按首次登记序）；`on(name)` 首次 = 宿主 `listen` 建条目（refcount = 1，attached），再次只增引用；`Listener` Drop / `off()` 减引用，**归零且条目未 host-owned 时** `unlisten` 并删条目；`detach()` 减引用并把条目标为 **host-owned**（此后归零也不再 unlisten，留到卸载）；host-owned 条目被后续 `on(name)` 复用时只增引用，不恢复自动摘除。这样一次 emit 每实例只触发一次 catch-all（作者按 `name` 分支；逐 handler 区分 = S7 强类型演进位）。
- **id 作用域**：listener id 每实例作用域；宿主校验归属，外来 id = trap；自己的已摘除 id = 幂等 no-op。
- **条目预算**：每实例 listener 条目数上限 = 实现期配置（T10.10），超限 = `listen` 返回 `core-contract-violation` Err；host-owned / `detach()` 条目同计（W8 D177）。
- **`on-event` 导出**：**已定形 = W8 D177**：`on-event(listener-id: u64, name: string, payload: payload) -> result<option<payload>, error>`，async（W6 D160）。带 listener-id 才能逐监听者保序 / 短路；事件名语义、`internal/` 规则与声明面 = W8。
- **动态名**：`listen` / `emit` 接受字符串；声明语义（manifest `events.listen/publish`）= W8 D173–D179；机械化对账 = W9。**W8 定案**：未声明的名字恒合法（`listen` 不因此失败），`listen` 的 `result` 只承载空名 / 超长名等值错误与宿主侧失败。
- **投递**：事件派发 = 宿主向监听者实例发起 export 调用，走 W4.3 闸门；邮箱满 = 背压，不丢弃（W3.11）。

### W3.6 D147 services：call / provide / deps 与 on-call

**D147 定案：服务调用 = `call` import + `on-call` export；`provide(key)` 保留激活期对账；`deps` 目录住宿主；方法集归契约，不上 ABI。**

```wit
interface services {
  use errors.{error};
  use payloads.{payload};
  record dep { name: string, required: bool, available: bool }
  call: async func(service: string, method: string, args: payload, timeout-ms: option<u32>) -> result<payload, error>;
  provide: func(service: string) -> result<_, error>;
  deps: func() -> list<dep>;
}
```

- **调用**：`call` async；args / 返回 = 单个 `payload`（W6 D160：`value` = TLV `Value`，`blob` = 不可变大对象）；`timeout-ms` 覆盖全局默认（service-registry S5「全局默认 + per-call 覆盖」的作者面落点）；缺席 = 默认；0 / 非法 = `core-contract-violation` Err。调用方可见错误集合 = service 四态 + `core-deadlock` / `core-contract-violation`（W5 D137）。
- **provide 观测**：`ctx.provide(key)` 在 setup 内调用（宿主登记）；宿主在 setup 成功后对账「manifest provides 全履行 / provide 了没声明 = 响亮」（S3 的「漂移无法被发布」保留）。Active 后 provide = trap（S2 激活点原子动作）；Unloading 新注册 = trap（W4.5 D11.2）。方法集不在此走线——住契约（W7）；无契约服务的 missing method = guest catch-all 返回 `business`（message = method missing），宿主只在契约已知时短路 `service-method-missing`（契约less 私有服务 = W2 D131 已明记的「失去形状对账」退化）。
- **deps 目录**：`deps()` 同步返回 manifest `inject` 声明的服务集（required / optional / available），宿主按 parent 链可达集解析；SDK 用它合成 `Deps`。未声明服务的调用 = trap（S3 响亮 × W5 D139）；已声明但运行期缺席 = `service-unavailable` Err；可选依赖静态缺席 = `ctx.service(name)` 返回 `None`。
- **on-call 导出**：`on-call(service: string, method: string, args: payload) -> result<payload, error>` async（W6 D160）；宿主按注册表路由到提供方实例；SDK 分发到 S1 的 catch-all `on_call`。提供方错误只许 `business`（W5 D138）；非 business = 宿主判 `core-contract-violation`。
- **对账**：provide / inject / events 的启用期判定、判环、可见集 = W9；本章只定 ABI 与作者面。服务调用路径的 7 步（context-events §1）不变，只是「打包 CallService」换成了 import 调用。
- **不立方法集 ABI**：方法名是契约数据（W2 D131 `provides.methods` 已消亡）；WIT 面只有 service / method 字符串 + `payload`。强类型服务方法 = W7 / T10.14 演进位（S7）。

### W3.7 D148 http：富类型请求 / 响应

**D148 定案：`request` 单方法 + 富类型 record；4xx / 5xx = `Ok` 载荷；v1 一次性响应进内存带预算，大下载走 `download` + `byte-reader`（W6 D159）。**

```wit
interface http {
  use errors.{error};
  use streams.{byte-reader};
  record request-params {
    method: string,
    url: string,
    headers: list<tuple<string, string>>,
    body: option<list<u8>>,
    timeout-ms: option<u32>,
  }
  record response {
    status: u16,
    headers: list<tuple<string, string>>,
    body: list<u8>,
  }
  record stream-response {
    status: u16,
    headers: list<tuple<string, string>>,
    body: own<byte-reader>,
  }
  request: async func(req: request-params) -> result<response, error>;
  download: async func(req: request-params) -> result<stream-response, error>;
}
```

- 平台封闭接口走 WIT 富类型（三分法①）；opts table 的 Luau 形态消亡。
- `method` 缺省 GET 由 SDK 层给；URL 畸形 / 方法值非法 = `http-invalid` Err（运行期数据）；策略拒绝（域名白名单 / SSRF / 跨域重定向）= `http-policy` Err；超时 = `http-timeout`；连接 = `http-connect`；响应超预算 = `http-too-large`；兜底 = `http-error`（W5 D137）。
- 4xx / 5xx = `Ok(response.status)`，不是错误（W5 判据①）；body = bytes。
- 强制超时：`timeout-ms` 可覆盖，但宿主设上限；缺省 = 宿主配置（[seam-capabilities](seam-capabilities.md) §5.2）。透明解压、域名白名单、SSRF、跨域重定向重新对账原位（seam §5.2）。
- v1 一次性响应全量进内存带预算；大下载 = `download` + `byte-reader`（W6 D159：分块读、不整段物化、EOF = `Ok(none)`）；流式上传维持扩展位。

### W3.8 D149 process：富类型与 resource 句柄

**D149 定案：`exec` / `spawn` 富类型；`process-handle` = WIT resource；argv / command XOR；输出截断显式；chunk v1 = `list<u8>`。**

```wit
interface process {
  use errors.{error};
  record exec-request {
    argv: option<list<string>>,
    command: option<string>,
    cwd: option<string>,
    env: option<list<tuple<string, string>>>,
    timeout-ms: option<u32>,
  }
  record exec-result {
    code: s32,
    stdout: list<u8>,
    stderr: list<u8>,
    stdout-truncated: bool,
    stderr-truncated: bool,
  }
  record exit-status {
    code: option<s32>,
    signal: option<string>,
  }
  resource process-handle {
    read-stdout: async func() -> result<option<list<u8>>, error>;
    read-stderr: async func() -> result<option<list<u8>>, error>;
    write-stdin: async func(data: list<u8>) -> result<_, error>;
    signal: func(sig: string) -> result<_, error>;
    wait: async func() -> result<exit-status, error>;
    kill: func() -> result<_, error>;
  }
  exec: async func(req: exec-request) -> result<exec-result, error>;
  spawn: async func(req: exec-request) -> result<process-handle, error>;
}
```

- `argv` XOR `command`：给且只给一个；违约 = `core-contract-violation` Err（运行期值形状；W5 D139）。**shell 边界**：`command` 经 `sh -c`（有 shell，注入面 = 作者责任）；`argv` 直跑不过 shell。`cwd` 缺省 = 用户家目录（config-format D45 原位）；相对 cwd 同基解析；env = 核心 env + 覆盖（secrets 不在 env）；命令级审批 / 策略归插件层（[seam-capabilities](seam-capabilities.md) §5.2 支配关系）。
- 强制超时；超时杀进程组；输出带预算；截断由 `stdout-truncated` / `stderr-truncated` 显式表达（不静默）。
- `read-stdout` / `read-stderr` 返回 `option<list<u8>>`：`Some` = 一块，`None` = EOF（A5 D14.3 的 EOF 语义载体平移）；两路分流不合并。chunk v1 = `list<u8>`；W6 D159 已定案不采用 `stream<u8>`（扩展位），process 句柄保持现状。
- `wait` → `exit-status`：正常退出 = `code = Some`，信号终止 = `signal = Some`；两者互斥。
- 句柄 = 账本条目；卸载 drain 杀进程组（无孤儿）；`signal` / `kill` 同步、`read` / `write` / `wait` async；Drop 句柄不杀进程（显式 `kill` / `wait`，宿主账本卸载兜底）。
- 能力门 `process: true`（W2 D129）；未声明 = trap（W5 D139）。journal `process_spawn` / `process_exit` 不变（[seam-capabilities](seam-capabilities.md) §5.3）。

### W3.9 D150 instance：mount / unmount

**D150 定案：`mount(declaration)` 过 TLV，返回 `mount-handle` resource；声明校验失败 = `core-contract-violation`；子实例启用失败 = `core-mount-failed`；`mount: true` 门；Drop = best-effort 异步 unmount。**

```wit
interface instance {
  use errors.{error};
  resource mount-handle {
    unmount: async func();
    detach: func();
  }
  mount: async func(declaration: list<u8>) -> result<mount-handle, error>;
}
```

- `instance` = 动态子树挂载面（[instance-model](instance-model.md) D63 / D64）；不是能力族，是编排面。`declaration` = TLV 编码的 `Value`（声明形状 = D61，含 `use` + `config:` overlay）；mount 执行 = 标准启用管线（部署索引解析 → config 合成校验 + 共享变量 / secret 解析 → inject 解析 → setup；D63 原位）。
- 声明形状 / 校验失败 = `core-contract-violation` Err（运行期值形状；宿主侧校验，插件代码未跑）；`capabilities.mount: true` 缺省 = trap（W2 D129 / W5 D139）。零仪式插件无 mount 资格（W1.8 D123）。
- `unmount` 走句柄（async，无错误面，**幂等**：已卸载 / 子实例已死 = 正常返回）；宿主失败 = host log + 强制 drop。父实例卸载 drain 子实例（D64）。**Drop 句柄 = 宿主 best-effort 异步 unmount**（入队 + host log（D72 人读面））；`detach()` 可放弃 Drop 语义（子实例留到父卸载）。这样 `let _ = mount(...)` 不会泄漏子实例；要确定性完成用显式 `unmount().await`。
- **错误映射**：声明形状 / 校验失败 = `core-contract-violation`；子实例启用失败（setup Err / trap、依赖缺失、预算 / 并发上限） = `core-mount-failed`（W5 本批增补）；未声明 mount = trap。
- **并发上限**：每父实例同时存活的子实例数 = 实现期配置（登记 T10.12）；超限 = `core-mount-failed`。
- 返回句柄 = WIT resource（wasm 原生所有权）；不立 `unmount(id)` 自由函数（丢父子账本的所有权表达）。mount handle 服务面出口 / 跨实例输入 = 既有扩展位（[instance-model](instance-model.md) D68 / D73），本章不启用。**W6 注（2026-10-19）**：只读 `blob` 已开启跨实例重物化（W6 D159）；`mount-handle` 仍按本行关闭。

### W3.10 D151 async / sync 分类

**D151 定案：metis 效应 import 全 async；只有纯登记 / 查询 / 立即本地工作 sync；private = WASI 0.3 async filesystem，作者面同为 async，不套同步逃生舱。**

| 类别                                 | 接口 / 方法                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| async（等待宿主 / 他人、需背压接纳） | `events.emit` / `serial` / `parallel` / `waterfall`；`services.call`；`timer.sleep`；`http.request` / `download`；`fs.*`（global）含 `open-read` / `open-write`；`ctx.plugin_fs()`（private，WASI 0.3 async filesystem）；`process.exec` / `spawn` 与 `read-*` / `write-stdin` / `wait`；`instance.mount` / `mount-handle.unmount`；`byte-reader.read` / `collect`；`byte-writer.write` / `close` |
| sync（登记 / 查询 / 立即返回）       | `events.listen` / `unlisten`；`services.provide` / `deps`；`timer.after` / `every` / `cancel`；`process-handle.signal` / `kill`；`fs` 的路径接口（`home-dir` 等）；`blob.duplicate` / `len` / `read`；`blobs.from-bytes`；`byte-reader.close`                                                                                                                                                     |

- papercut 7 只拒绝同步 **export**；同步 import 合法（S1 spike 的 `tick` 同步 import 已实证）。本表仍把效应调用做成 async，是为了让等待让出执行权、让背压有挂载点。
- `emit` 是 async 但只 await 入队——不等监听者完成；`serial` / `parallel` / `waterfall` await 派发结果。
- `timer.after` / `every` 是登记（sync），tick 回调是 async export（`on-timer`）；`sleep` 是 async import。
- private 走 WASI 0.3 async filesystem，与 global 同为 async；guest crate 的 `std::fs` 是标准库同步 API 的适配层，不属于 metis 作者面。

### W3.11 D152 投递纪律：背压、合并计数、卸载可见

**D152 定案：投递路径不静默丢弃；满时只有两种可见结果——背压等待，或显式合并计数；卸载期残留进 journal。**

| 通道                                           | 满时行为                                                                         | 丢弃可见性                                               |
| ---------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------- |
| events（emit / serial / parallel / waterfall） | 邮箱满 = 发射方 await 背压；容量等待成圈 = `core-deadlock` Err；派发调用等待闸门 | 不丢；等待或类型化错误                                   |
| services.call                                  | 在飞预算满 = 留在邮箱；调用方 deadline 到 = `service-timeout`                    | 不丢；`service-timeout` Err                              |
| timer tick                                     | 每 timer 一个 pending + 合并计数；handler 未完结不并发重入                       | 不丢；下次 tick 带 `coalesced`                           |
| 卸载（W4.5）                                   | 关闸后残留丢弃                                                                   | journal `unload_gate_closed` / `unload_killed`（既有面） |

- **blob payload 的投递**：邮箱 / 在飞预算按消息条数计，不按 blob 字节计；fanout 时宿主对每个接收方 `duplicate` 句柄（0 内容拷贝），发送方 own 句柄入队后释放；接收方不读 = 不产生内容拷贝。blob 内存预算 = 租户级（W6 D159）。

- **邮箱满不丢**：W4.3 的背压链原样——预算满留邮箱，邮箱满背压投递方；`emit` 的 await 是接纳点，不是监听者完成点。
- **timer 不排无界队列**：每 timer 最多一个 pending；handler 未完结时的 tick 合并进计数，下一次投递 `coalesced` 告诉作者合并了多少。作者要每 tick 必达 = handler 保持短 / 显式并发；宿主不静默丢。
- **卸载丢弃可见**：实例已死，作者不可观测；宿主 journal 记录关闸与强杀点（W4.5 D11.4 / D11.5），审计可查「哪条没处理完」。
- **不立「丢弃回调」**：没有静默丢弃，就不需要 guest 侧丢弃通知；`coalesced` 是唯一合并面。
- **`emit` 自投递的容量圈 = 显式错误**：容量等待进 W4.4 wait-for 图；成圈时 `emit` 返回 `core-deadlock` Err，不静默等待。

### W3.12 D153 作者面 `Ctx` 与句柄

**D153 定案：零尺寸 `Ctx`；编排面扁平、效应面访问器；所有钩子拿 `&Ctx`；id 一律包成句柄；`metis_plugin::raw` 是唯一 escape hatch。**

- **Ctx 形态**：零尺寸 handle（可存进作者 struct）；不携带 FiberId——实例身份由宿主持有，guest 侧无需重复。宿主入口（生命周期 `setup` / `on_config` / `dispose`、`on_call`、SDK 的 `on_event` / `on_timer` 分发器）都拿 `&Ctx`，SDK 合成；作者面事件处理 = `on_event` catch-all，timer 回调 = `ctx.timer().after/every(ms, handler)` 注册闭包（无 `on_timer` 作者钩子）。
- **组织**（A3.2 平移）：编排面扁平——`emit` / `serial` / `parallel` / `waterfall` / `on` / `provide` / `service` / `deps`；效应面访问器——`fs()` / `plugin_fs()` / `http()` / `timer()` / `process()` / `instance()`。命名空间在 `Ctx` 上恒定存在（纯代码插件同形）；未声明能力 = 调用时 trap 指路 manifest。
- **句柄**（作者面不出现裸 `u64`）：`Service`（名字 + `call`）、`Listener`（`off()` / `detach()`）、`TimerHandle`（`cancel()` / `detach()`）、`MountHandle` / `ProcessHandle`（WIT resource，显式 unmount / kill；`MountHandle.detach()` 放弃 Drop 语义）、`Blob` / `ByteReader` / `ByteWriter`（W6：`duplicate` / `read` / `write` / `close`）。`Listener` / `TimerHandle` 的 `Drop` 调 sync 的 unlisten / cancel；`detach()` 把生命周期交给宿主账本。
- **Deps**：`ctx.service(name) -> Option<Service>`——未声明 = trap；可选依赖静态缺席 = `None`；required 由激活门保证。`Deps` 目录由 `services.deps()` 合成，形状 = `{ name, required, available }`。
- **Value / TLV / Payload**：服务 args / 返回、事件 payload = `metis_plugin::Payload`（`Value` / `Blob` 两车道，W6 D160）；config / mount 声明 / business data = `metis_plugin::Value`，SDK 编解码 TLV（T10.13）；平台封闭接口（fs / http / process / timer）用生成的富类型，作者不接触 WIT 名。
- **原始绑定 escape hatch**：`metis_plugin::raw` 公开 re-export 生成绑定，文档标注「正常不需使用」；不进 prelude（plugin-sdk S6 原则③）。WASI 名不进作者面：`std::fs` / `std::time` 可直接用；CSPRNG 用 guest crate（`getrandom` / `rand`，底层 `wasi:random`）；SDK 便利 re-export = feature 扩展位（v1 不做）。
- **trait 形状**（示意）：`async fn setup(&mut self, ctx: &Ctx, config: Value) -> Result<(), Error>`；`on_call(&mut self, ctx: &Ctx, service: &str, method: &str, args: Payload) -> Result<Payload, Error>`；`on_event(&mut self, ctx: &Ctx, name: &str, payload: Payload) -> Result<Option<Payload>, Error>`。timer 回调是闭包注册，不是 trait 钩子。具体宏展开归 T10.14。

### W3.13 D154 能力门载体与 world 终态

**D154 定案：能力门 = 宿主 import 实现内逐调用 fail-closed；未声明 = trap 指路 manifest；WASI 0.3 基线是基础设施白名单；v1 world 导出集 = 生命周期三导出 + `on-call` / `on-timer` / `on-event`（W8 D177 补全）。**

- **门载体**：固定 world 下所有 import 恒链接，链接期无法裁剪；门在宿主 import 实现内逐调用判定（S6 先例：Spin / Zed 均运行期门）。未声明 = trap（W5 D139），message 指路 manifest 字段；依赖方收 `service-unavailable`（D11.3）。
- **门表**：

| 面                                   | 门                                         | 缺省                                                           |
| ------------------------------------ | ------------------------------------------ | -------------------------------------------------------------- |
| `fs`（global）                       | `capabilities.fs: read / write`（W2 D129） | 无 global；相对路径 = `fs-invalid-path` Err；未声明调用 = trap |
| private fs（WASI `wasi:filesystem`） | 无 metis 门（preopen）                     | 恒可用                                                         |
| `http`                               | `capabilities.http` host 列表              | 无 host；调用 = trap                                           |
| `process`                            | `capabilities.process: true`               | 拒绝；调用 = trap                                              |
| `instance.mount`                     | `capabilities.mount: true`                 | 拒绝；调用 = trap                                              |
| `timer`                              | 无                                         | 恒可用；`every` 最小间隔 = 政策                                |
| `events` / `services`                | 无能力门（编排面）                         | 声明对账 = W9                                                  |
| `blobs`                              | 无能力门（只读不可变大对象）               | 恒可用；blob 内存预算 = 租户级                                 |
| `streams`                            | 随属主（fs / http）；无独立门              | 无属主调用 = trap；reader / writer 不跨实例                    |

- **WASI 0.3 基线白名单**：`wasi:clocks` / `wasi:random` / `wasi:filesystem`（async；preopen 与 `std::fs`）/ 以及 guest std 拖带的 WASI 0.3 导入，是宿主基础设施；宿主统一注册 wasmtime-wasi 0.3。W9 的进口对账把「std 拖带集 + 上述标准件」视为基线白名单，不计入作者能力面；作者手写其他 WASI import = 越权（词汇表外）。具体清单 SSOT = W9。wasip2 时代的 14 导入 papercut（补遗 6）是历史证据，0.3 基线由 T10.10 核实。
- **world 终态**：v1 导出集 = `setup` / `on-config` / `dispose` / `on-call` / `on-timer` / `on-event`（全 async；`on-event` = W8 D177）。`0.1.0` 尚未发布；本批补全 = 首个发布快照的编制，不是版本演进；首个冻结快照由 W10 D190 记录（本章收官时 `0.1.0` = 当前快照、未冻结）。W1.3 的签名草图同批更新。
- **不立编译期能力检查**：SDK 面不按 manifest 裁剪类型；作者面形状恒定（A3.2「出现规则」平移），越权调用 = 运行期 trap。编译期组合 world 维持扩展位（plugin-sdk S6）。

### W3.14 A3 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 ADR-0026（已 superseded）。

| 机制                                              | 死因                                                                         |
| ------------------------------------------------- | ---------------------------------------------------------------------------- |
| ctx = 宿主构建的 frozen table / 递归只读 / 无元表 | wasm import 集 + Rust 类型；容器不再由宿主装配                               |
| userdata 淘汰 / 烘焙方法集                        | WIT resource（process / mount）+ 生成绑定；无运行期方法表                    |
| `ctx.on` 返回 Luau disposer 函数                  | `Listener` RAII + 宿主 listener 表                                           |
| `deps` 第三参数                                   | `services.deps` 目录 + `Ctx::service`                                        |
| `ctx.provide(key, 方法表)`                        | `provide(key)` + `on-call` catch-all；方法集归契约                           |
| `ctx.exclusive` / `ctx.fanout`                    | guest async / SDK async mutex（W4.2）                                        |
| `ctx.box` / `box(name)`                           | 盒概念消解（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §2）         |
| `ctx.sql` / 私有 db                               | sqlite 撤回（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）       |
| `ctx.workspace` / 三 scope                        | fs 两档（ADR-0039）                                                          |
| `ctx.fs` / `ctx.global` 的 Luau 同步 / 两跳形态   | 统一 async `fs` / `ctx.plugin_fs()`（W3.3）                                  |
| `ctx.http.request` opts table                     | WIT request record（W3.7）                                                   |
| timer after / every 的宿主调 Luau 闭包            | `on-timer` export + `TimerHandle`（W3.4）                                    |
| process 冻结表句柄                                | `process-handle` resource（W3.8）                                            |
| `ctx.mount` 表句柄                                | `mount-handle` resource（W3.9）                                              |
| int64 / buffer / UTF-8 裂缝                       | WIT s64 / `list<u8>` / `string`                                              |
| require / `.luaurc` / `@lib`                      | [ADR-0038](../decisions/0038-module-system-require-supersede.md)             |
| crypto 内建                                       | [ADR-0041](../decisions/0041-crypto-capability-withdrawal.md)（guest crate） |
| 「方法名写错 = 字段不存在当场炸」                 | Rust 编译器 + 运行期 trap                                                    |
| optional dependency 字段缺席                      | `Option<Service>`                                                            |
| `ctx.once`                                        | 维持扩展位（组合写法）                                                       |
| `plugin()` 糖 / 入口 table                        | W1（固定 world + SDK 宏）                                                    |
| `ctx.on_config` 可选字段                          | W1 `on-config` 导出 + trait 默认实现                                         |

**语义平移保留**：编排面扁平 / 效应面命名空间化（A3.2）；接口形状恒定、失败指路 manifest（A3.2 出现规则）；provide setup-only 与激活点原子注册（S2）；`Result` + kind 分支（W5）；timer 两条唤醒路径（sleep 直唤醒 / tick 投递，A3.6）；fs 两档（ADR-0039）；http 政策（seam §5.2）；process 账本与进程组杀（A3.10）；mount 启用管线（D63）；监听快照 / 注册序 / 逐项活性检查（ADR-0014）。

### W3.15 联动登记汇总（章末）

| 去向              | 内容                                                                                                                                                                                                                                                                                                                                |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1（T13-h）       | world 导出集补 `on-call` / `on-timer`（W1.3 签名草图同批更新；W8 D177 补 `on-event`）；生命周期三导出不变；`on-call` args / 返回 = `payload`（W6 D160）                                                                                                                                                                             |
| W2（T13-i）       | 消费 `capabilities.fs/http/process/mount`、`provides/inject/events`、`abi`；无 manifest 改动                                                                                                                                                                                                                                        |
| W4（T13-c）       | tick 投递走 W4.3 闸门；`emit` await 入队；投递不静默丢弃（W3.11）；W4.3「tick = 普通邮箱消息」增补为「每 timer pending + 合并计数」                                                                                                                                                                                                 |
| W5（T13-k）       | **已兑现（2026-10-19）**：所有 fallible import = `result<T, error>`；timer 无 Err 面；`fs-invalid-utf8` 保留（ABI bytes-only；SDK `read_text` 产生）；服务调用四态 + 服务路径 core 两格；mount 路径 `core-mount-failed`（mount 启用失败）；未声明能力 = trap；crypto 无错误面（ADR-0041）                                           |
| W6（T13-l）       | **已兑现（2026-10-19）= W6 D155–D162**：TLV canonical 线格式与拷贝预算；大负载双车道 = `blob`（不可变 / 可跨实例只读）+ `byte-reader` / `byte-writer`（per-instance）；`fs` += `open-read` / `open-write`，`http` += `download`；process chunk 保持 `list<u8>`（不改写）；`payload` 变体（value / blob）；resource 生命周期与失败面 |
| W7（T13-m）       | **已兑现（2026-10-19）= W7 D163–D172**：服务方法集 = 契约数据（D167）；强类型生成 = `types!()`（D170）；无契约服务的 method-missing 退化保留（D167 / D169）                                                                                                                                                                         |
| W8（T13-n）       | **已兑现（2026-10-19）= W8 D173–D179**：`on-event(listener-id, name, payload: payload)` 定形并进入 required 导出集；事件名语义 / `internal/` 规则 / 声明面 / emit 校验 / 形状生命周期；`emit` / `on-event` 的 payload = W6 双车道变体                                                                                               |
| W9（T13-o）       | **已兑现（2026-10-19）= W9 D180–D185**：import 清单 ⊆ 词汇表（含 `blobs` / `streams` / `payloads`）；provide / inject / events 对账；能力门判定（含 mount 例外）；blob 无门 / streams 随属主；WASI 0.3 基线 = clocks + random + filesystem（async）+ std 拖带集（逐条以宿主注册集为准）                                             |
| W10（T13-p）      | **已兑现（2026-10-19）= W10 D186–D193**：package version 与首个冻结快照（D187 / D190）；import / export 增补纪律（D188）；多版本共存与逐快照词汇表（D189）                                                                                                                                                                          |
| T10.10            | 宿主 import 注册；wasmtime-wasi 0.3；preopen `/`；`on-call` / `on-timer` / `on-event` 导出调用；blob / reader / writer 资源表与跨 Store 重物化                                                                                                                                                                                      |
| T10.11            | fs（global host-mediated / private WASI 0.3 async filesystem）/ http / process / timer 实现；crypto 撤回（ADR-0041）；预算 / TOCTOU 硬化；random = wasi:random；`open-read` / `open-write` / `download`；blob 创建 / collect                                                                                                        |
| T10.12            | 安装期 world 导出校验补 `on-call` / `on-timer` / `on-event`；preopen 授予                                                                                                                                                                                                                                                           |
| T10.13            | TLV 编解码（payload value 车道、config、mount 声明、business data）；blob / stream 不进 TLV；`ValueRef` / `encoded_len` + `encode_into`；canonical 编码与版本字段                                                                                                                                                                   |
| T10.14            | `Ctx` / 句柄 / RAII / `metis_plugin::raw`；默认钩子（含 `on_event = Ok(None)`）；`Payload` / `Blob` / `ByteReader` / `ByteWriter` / `Value::Bytes`；WASI 0.3 错误码 → fs kind 映射；CSPRNG = guest crate（`getrandom` / `rand`）；crypto 便利 re-export = feature 扩展位（v1 不做）                                                 |
| T10.15            | 脚手架不含 crypto；预检 world 导出集与 `wasm32-wasip3` target；`metis check` 的 import 提示                                                                                                                                                                                                                                         |
| seam-capabilities | §2.1 纯算件判据、§4.2 crypto 撤回、§5.2 fs 两档作者面、§5.3 能力映射、§6.1 W3 行兑现                                                                                                                                                                                                                                                |
| service-registry  | per-call deadline 作者面 = `call(..., timeout-ms)`；provide 观测保留 S3 对账；deps 目录 = inject 声明集                                                                                                                                                                                                                             |
| context-events    | 监听登记住宿主 + `Listener` RAII；`emit` await 入队；投递不丢弃                                                                                                                                                                                                                                                                     |
| instance-model    | `mount` / `unmount` 句柄形态 = `mount-handle` resource；Drop = best-effort 异步 unmount（host log）；`detach()` 放弃 Drop 语义；`core-mount-failed` 映射                                                                                                                                                                            |
| plugin-sdk        | S6 三原则落地（生命周期实体 = resource、无状态 = ctx 方法、raw escape hatch）；`Ctx` / 句柄映射；crypto = 作者自加 crate                                                                                                                                                                                                            |
| journal           | 不新增 kind；卸载丢弃可见 = `unload_gate_closed` / `unload_killed`；`process_spawn` / `process_exit` 不变                                                                                                                                                                                                                           |
| ADR-0041          | crypto 能力撤回（局部 supersede ADR-0025 §4 crypto 项 / ADR-0024 §4 crypto 行结论面）                                                                                                                                                                                                                                               |
| wasm-research     | 补遗 8：crypto 内建件撤回（ADR-0041）；补遗 9：目标 WASI 0.3（private fs async 面）；技术事实保留                                                                                                                                                                                                                                   |
| design-review     | 补遗 1：crypto 撤回（ADR-0041）                                                                                                                                                                                                                                                                                                     |
| 实现期配置        | timer `every` 最小间隔；http 超时 / 预算；fs 读写预算；process 输出预算；message 长度上限；在飞任务预算；private fs 读预算；TLV 条目 / 深度 / 字节；blob 单大小 / 总量；reader / writer 块上限                                                                                                                                      |
| 扩展位            | 强类型服务方法 / 事件方法（S7）；http 流式上传；fs 流式与 walk / glob（guest 组合）；`ctx.once`；WIT resource 化 listener / timer；instance list / describe；crypto 便利 re-export（feature）；WASI `stream<T>` 采用面；blob 写入型 / 可变跨实例；per-read timeout                                                                  |

---

## W4 async 与交错语义

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

**挂起点重定义**：挂起点 = guest 任务的任意 await 处。旧 ABI 的「挂起点 = ctx 方法调用处」是同一语义的解释器时代表述——那时一切挂起必经宿主；现在语言设施自管。宿主只见 import 边界的挂起与唤醒（await import 完成 = 宿主可控唤醒源）；guest 内部 await 宿主不可见、无需见。

**实证地基**（S2 = [wasm-research](../research/wasm-research.md) 补遗 7，wasmtime 49.0.3 / wit-bindgen 0.63 / wasm32-wasip2——取证环境；目标 WASI 0.3）：

- **R1 同实例重入**：单宿主线程 `run_concurrent` 内并发两调同一实例 async export（500 ms 与 100 ms 挂起），墙钟 ≈ 500 ms（非串行 ≈ 600 ms），交错日志序 = start 1 → start 2 → end 2 → end 1——第二条调用在第一条挂起期间进入同一实例并先完成。
- **R2 共享堆与原子性**：两条在飞任务共享同一线性内存堆；无 await 的读改写两次 = 2（挂起点间原子）；跨挂起点 read-modify-write 并发两调 = 丢失更新（终值 1 非 2）——危险形状实证，即 async mutex 的全部动机。

**概念地基（平移，教学条，Creator 文档必带）**：并发等待 ≠ 并行计算——插件耗时大头在**等**（LLM/HTTP/磁盘/子进程），等待 = 任务挂起、执行权让给别人；「一个执行器」限制的是「算」，不限制「等」。分工一句：**等的事重叠 = guest async 原生**（`join!`/`select!` 一族）；**算的事并行 = 同码多实例并跑或外部服务**（薄编排反转 = [wasm-first-design-review](../research/wasm-first-design-review.md) B8：插件内 CPU 密集合法化）。协作式免费性质平移：不跨挂起点的代码区间天然原子 → 需要显式互斥的形状只有一种 = 跨挂起点 read-modify-write。

**作者心智**：普通 async Rust（SDK 藏 ABI = ADR-0034 §4）。A4 作者契约四条平移：协作式（两挂起点之间原子）/ 挂起窗口内什么都可能变（挂起期间其他消息先跑，自家状态可能被改）/ 同步全显式（宿主零隐式串行，W4.2）/ 在飞即成本（每条挂起任务持 guest 栈与捕获态，W4.3）。

**淘汰项存档**（E3 议题批已决）：实例级串行化（run-to-completion，挂起即不投递新消息）——回退到 A4.1 枪毙过的头阻问题（一个 session 的 30 秒 LLM 调用卡住全插件邮箱）；声明式重入许可（按 handler/事件名标注）——灵活但引入新声明面，而真正需要同步的位置很少且可枚举（单一形状，见概念地基）。

### W4.2 D94 宿主零同步原语（2026-10-17 定案，E3 拍板吸收）

**D94 定案：宿主不提供任何同步原语。** fan-out = guest 异步设施原生（`futures::join!` 一族；SDK 呈现形态归 T13-d/T10.14），A4 的 `ctx.fanout` ABI 原语消亡——宿主无观测点（journal `fanout_cancelled` 挂载点随之消解，处置登记 = W4.6/W4.7）。互斥 = SDK async mutex 助手（同名互斥、含挂起期间占用的语义在 guest 内兑现），A4 的 `ctx.exclusive` ABI 原语消亡。原子变量无场景 = E3 澄清（单线程执行器，await 间天然原子）。

- **D1.2 三规则重估面归 T10.14**（E3 拍板登记原位）：不嵌套 / 自家人 / 强放——可由 SDK 类型系统表达的部分归类型，余者归 SDK 文档纪律；宿主侧名字注册表与名字条目回收机器整体消亡。
- **全局互斥不立 = A4 三理由原样平移**：占用中一次普通服务调用即可成圈（死锁从罕见变常态）；manifest 看不见的隐式跨插件契约（与 [ADR-0018](../decisions/0018-service-registry.md) 的 manifest-first 纪律冲突：跨插件关系以 manifest 声明为准）；宿主机制厚度。跨插件协调三归宿不变：属主插件 + 服务调用（manifest 可见可审批）/ 宿主侧配置化策略 / 分布式现实（seam 既定立场：两插件写同一文件靠协议不靠互斥）；原子方法族 = seam 能力面原位（首件 = fs `append`，形态归 W3/W9 章与 T10.11 能力站）。
- **结构化纪律平移**：handler 完结时其 fan-out 子任务须已完结——旧 ABI 靠宿主强制取消兑现，wasm 世界 = guest 语言设施职责（`join!` 天然结构化；detach spawn = SDK 纪律面）。宿主不补机制。
- **扩展位登记**：exclusive 可选上报宿主做 wait-for 可见性——上报是仪表不是机制（E3 议题批原话）；触发器 = 首个死锁调试痛点。

### W4.3 D95 投递政策与背压

**D95 定案：投递 = 向实例发起新 export 调用；邮箱 + 每实例在飞任务预算闸门；完成永不挡。**

- **worker 语义**：worker 空闲取邮箱消息，经闸门接纳后投递，任务立即起跑；import 完成 = 唤醒源直接唤醒对应 guest 任务，不经邮箱、不经闸门。宿主侧没有「就绪恢复」调度对象——A4 单可调度队列 FIFO（D8.1）的问题形（恢复 vs 新消息的优先级之争、tie-break、两级队列）随载体整体消解；原被否项「完成优先」的饿死分析（流式长读插队致热更等待无界）随之失去对象。热更/卸载不进投递队列（关闸 = 停接纳，W4.5），等待硬上界结论由卸载时序兑现，不再由队列规范兑现。
- **预算闸门（A4 D8.2 平移）**：预算只挡「邮箱消息 → 投递」的接纳——每实例在飞 async 任务数封顶（数值 = 实现期配置，登记 = W4.7）；预算满 = 新消息留邮箱，背压链 = 邮箱满 → 投递方背压/超时（[context-events](context-events.md) 既有政策）→ 服务调用方 deadline 收场（service-registry S5）。完成永不挡 → 在飞必然收敛 → 预算必然释放；超限政策 = 纯回压不立响亮档（A4 D10.2 拍板原位：回压把瞬态尖峰变成延迟，响亮把它变成用户可见错误——尖峰是常态，错误化 = 误伤）。
- **ADR-0014 兼容注**：单派发者保序承诺（同一监听者收件序 = 派发入邮箱序）= 邮箱层性质，不受重入影响——承诺的是收件序，非处理完成序（A4 既有注平移）。
- **timer 处置平移（A4 D1.4）**：tick 走同一闸门，但投递形态按 W3 D145 细化——每 timer 一个 pending 标记 + 合并计数（非重叠：上一个 handler 未完结时后续 tick 合并），`on-timer(id, coalesced)` 把合并次数交给作者；**邮箱满不丢、合并可见**。积压面 = 在飞任务数（稳态界 ≈ 到达率 × 平均延迟；每条在飞任务持 guest 栈与捕获态）；兜底 = `every` 最小间隔政策（seam §5.2）+ 写码期教具（W4.4 层二）。
- **在飞即成本**：高频事件源回调写短；怕自撞 = SDK async mutex（A4「在飞协程」成本条的载体词平移）。

### W4.4 D96 跨插件死锁与保险丝

**D96 定案：跨插件死锁四层处置平移；插件内等待面宿主不立 deadline；fail-fast 清单按宿主可见面重订。**

- **D1.7 四层平移**：① 启用期 manifest 依赖图禁环（不动；WIT 形态归 W9 章）；② 写码期教具 = 形状教材归 SDK lints 与 Creator 文档（AST lint 管线随载体消亡 = [wasm-first-design-review](../research/wasm-first-design-review.md) A8；三形状载体中立，于 Rust 同样成立：跨 await 读改写 / 持锁等跨插件调用 / `every` 回调挂起未自锁）；③ 运行期 wait-for 图（宿主全见跨插件等待——服务调用、事件链与邮箱容量等待（`emit` 入队，W3 D146）；收窄写实：插件内名字等待随宿主零同步原语退出图的节点集，除非 W4.2 扩展位上报激活）；④ 超时 = 最后保险丝（[ADR-0011](../decisions/0011-actor-execution-model.md) 三层保险既有面）。
- **插件内等待面盘点（A4 D10.1 平移）**：fan-out join 与 exclusive 排队 = guest 内部面，宿主不见不立 deadline（guest 侧结构化纪律 = SDK）；流 `read()` 挂起 = 唯一真无界源的性质不变（作者自开自流、可见可控），处置 = W6 资源面登记（per-read timeout 扩展位随章平移）。不立的理由平移：能定出原则性数值的形状都已有 deadline，剩下的是合法长等，任何统一数值都是拍脑袋；兜底三件套 = 在飞预算记账 + 卸载强杀（W4.5）+ 成环检测（层三）。
- **fail-fast 清单（宿主可见面重订）**：wait-for 圈闭合（含邮箱容量等待；当场报错 + 整圈路径 + fail-open 解圈 = [ADR-0014](../decisions/0014-dispatch-semantics.md)）；卸载期注册新账（W4.5）；未声明能力调用 = 两层：import 清单 ⊆ 宿主词汇表 = 安装期结构对账（[wit-abi](wit-abi.md) W9 D180–D182）；manifest 能力声明 = 运行期门 fail-closed，越权 = trap（W5 D139）；不可转换值跨界（W6 面）。A4 清单的插件内四项消亡或归 SDK：exclusive 嵌套 / 自家人进名（SDK 类型与纪律面）；被登记线程裸 yield（无对应物——宿主无「无登记挂起」可检，观测点缺失即消亡理由）；fan-out 子任务未随 handler 完结（guest 结构化纪律，宿主无观测点）。
- **兜底保险丝层平移**：单步执行硬截断 = fuel/epoch 计量（wasmtime 原生件，实测 1.022× = [ADR-0024](../decisions/0024-carrier-layering.md) §8；[ADR-0011](../decisions/0011-actor-execution-model.md)「防堵 executor」语义不动，机制自 interrupt 回调换血——Status 注同批落地）；服务调用 deadline（service-registry S5）/ bounded 邮箱 + 在飞预算回压（W4.3）/ 加载总预算（W1 章面）/ drain 总预算（[fiber](fiber.md) §10）。

### W4.5 D97 卸载与在飞任务交接（2026-10-18 五段化）

**D97 定案：卸载与在飞任务交接 = 严格串行五段（关闸 → 优雅窗口 → `dispose` → 实例即弃 → drain），A4 D11.1 时序平移，强杀机制换血。**

1. **关闸**：进 Unloading 即停收新投递（邮箱关闭 = 既有「投递失败正常跳过」纪律）、在飞闸门停接纳；
2. **优雅窗口**：在飞任务自然完结（import 完成照常唤醒；数值 = 实现期配置，热更场景默认短）——挂在静默流上的合法长等不等它；
3. **`dispose` 调用**：在飞任务全部结算后，宿主直调 guest `dispose` 导出（独立预算；语义与错误处置 = W1.6 D121）；
4. **实例即弃**：drop 整个组件实例——interrupt 逐线程杀机制消亡。S2 R4 实证（补遗 7）：在飞 guest 任务随并发域遗弃（`run_concurrent` future drop 实测），无 trap、无诊断输出；实例即弃 = 同一丢弃语义的外推（实例 drop 未单列实测）。诚实边界 = 静默是载体事实，作者可见性由 journal 关闸/强杀点记录兑现（D11.5 既有面平移）。在飞任务挂起的 import 随实例一同消失，宿主侧迟到的完成 = 丢弃（既有纪律平移）。**强制路径** = 优雅窗口耗尽而在飞任务仍未结算，或 `dispose` 预算耗尽：跳过（或中断）`dispose`，直接进入本段并记强杀点；
5. **drain**：既有路径（fiber LIFO、disposer 可 await、超时强制）。**串行不重叠**平移：disposer 起跑时 guest 任务已全灭（实例已弃），无并发干扰。

淘汰项存档（A4.5 平移）：无限等完结（静默流挂起 = 热更卡死）；立即杀（丢掉片刻后就写完的盘，且违 [ADR-0018](../decisions/0018-service-registry.md) S4「优雅执行/超时强制」既有精神）。

- **D11.2 卸载期拒新账平移**：Unloading 全程拒绝「比我长寿」的新账（新注册监听 / provide / timer = 响亮错误——语义上想活得比插件久的注册，在卸载期无意义）；效应调用放行（在飞任务收尾要用）；新句柄放行但 drain 强 close 兜底（流式句柄族铁律，W6 章平移面）。
- **D11.3 对手方即错平移**：A 调 B、B 进 Unloading——B 的在飞任务窗口内跑完则结果照常路由回 A；被杀则宿主立即给挂起的 A 注入 `Err(Unavailable)`，不傻等 deadline（[ADR-0018](../decisions/0018-service-registry.md) S4「故障检测结构性」的延伸）；Unloading 期间新调用 B = 即 `Err(Unavailable)`（邮箱已关）。
- **D11.4 邮箱残留 = 丢弃平移**：关闸后未投递消息直接丢（事件 = 投递失败正常跳过；服务调用投递 = 按 D11.3 即错回调用方）。A4 的「可调度队列残留」一格随 D8.1 消解无对应物（wasm 下投递即起跑，无中间队列态）。
- **D11.5 保证分级平移**：服务调用 = 结果或显式 `Err`，绝无静默；事件/消息处理 = 尽力（窗口期完结）+ 可见（关闸丢弃清单、强杀点进 journal，事后可查「哪条没处理完」）；消息重投 = 否决（被杀 handler 可能已发出过回复——重投 = 重复效应风险；exactly-once 物理不可达）。业务连续性归宿不动：跨热更要连续的业务 = 插件经 seam 持久化 + 新版本 setup 读回接续；核心的责任止于给显式结果、给可查记录、给持久化能力。

> **2026-10-18 五段化**：guest `dispose` 调用位 = W1.6 D121；`setup` 失败不调用 `dispose`（W1.5 D120）。

### W4.6 A4 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史与归档设计文档。

| 机制                                                                        | 死因                                                                                                                                                                  |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| coroutine shim / SENTINEL / 挂起冒泡链 / outstanding 防假注入（§A4.3 D9.2） | guest async 运行时原生挂起恢复，无需宿主令牌机器（S1/S2 实证）                                                                                                        |
| 线程/协程术语分层（§A4.3 D9.1）                                             | 分层对象为旧解释器协程；Rust guest 的 async task 无需再包装                                                                                                           |
| 被登记线程裸 yield 响亮杀（§A4.3 D9.3）                                     | 无对应物——宿主无「无登记挂起」可检（guest 内部挂起宿主不可见 = W4.1）                                                                                                 |
| 调度规则纯函数化冻结进 ABI（§A4.1 D1.6）                                    | replay 不立（[journal](journal.md) D75）+ guest 内调度器宿主不可冻结；D27.1 ABI 确定性面随之缩没                                                                      |
| 单可调度队列 FIFO 规范（§A4.2 D8.1）                                        | 问题形消解（W4.3）；tokio/Temporal/FDB 确定性调度教学条随之退役                                                                                                       |
| `ctx.fanout` / `ctx.exclusive` ABI 原语（§A4.1 D1.2/D1.3）                  | 降级为 guest 异步设施与 SDK async mutex 助手（W4.2）                                                                                                                  |
| lint 三形状（§A4.1 D1.5）                                                   | AST lint 管线随载体消亡（design-review A8）；形状教材归 SDK/Creator（W4.4 层二）                                                                                      |
| journal `fanout_cancelled` 族（[journal](journal.md) D78 错误现场族）       | fan-out 归 guest 内部件，宿主无观测点；**已兑现 = [ADR-0035](../decisions/0035-journal-fanout-cancelled-supersede.md)**（2026-10-18：D78 该格 supersede，无替代条目） |

### W4.7 联动登记汇总（章末）

| 去向                 | 内容                                                                                                                                                                                                                                                       |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T10.2（fiber-core）  | 实现期配置原位存活（载体词已换）：每实例在飞 async 任务预算（W4.3）+ 优雅窗口与 drain 总预算数值（W4.5）；wait-for 图存活 = W4.4 层三（节点集收窄在案）                                                                                                    |
| T10.4（事件派发）    | worker 投递/完成模型与闸门位置 = W4.3；ADR-0014 保序兼容注原位；D6 事件吞吐件不变；裸 yield 检测 D9.3 消亡在案（T10 方向变更注）                                                                                                                           |
| T10.14（插件 SDK）   | exclusive = async mutex 助手 + D1.2 三规则重估面（W4.2）；fan-out 呈现形态（W4.2）；写码期教具三形状（W4.4 层二）；S2 papercuts 7–11 输入（[wasm-research](../research/wasm-research.md) 补遗 7），含 `exports: { default: async }` 绑定约束（papercut 7） |
| W1 章（入口契约）    | `dispose` 调用位回填（W1.6 D121）；卸载五段 = 关闸 → 优雅窗口 → dispose → 实例即弃 → 宿主 drain                                                                                                                                                            |
| W3 章（ctx 形态）    | 挂起点的作者面呈现（W4.1）；SDK async 映射输入（W4.1/W4.2）；补遗 6/7 papercuts 分流在案                                                                                                                                                                   |
| W5 章（错误）        | **已兑现（2026-10-19）**：fail-fast 清单的错误面 = W5 D137/D139——wait-for 圈 = `core-deadlock`；卸载期新注册长寿账 = trap；`service-unavailable` 承接 D11.3                                                                                                |
| W6 章（边界转换）    | **已兑现（2026-10-19）= W6 D155–D162**：拷贝下限与源快照必须先于挂起（W4.1）；流 `read()` 无界源 = `byte-reader`；不可转换值跨界 = TLV 边界错误；流式 / blob drain 强 close 接 W4.5 D11.2；per-read timeout 维持扩展位                                     |
| journal.md           | `fanout_cancelled` 处置 = **已兑现 = [ADR-0035](../decisions/0035-journal-fanout-cancelled-supersede.md)**（2026-10-18）；`unload_gate_closed`/`unload_killed` 两族存活（W4.5 语义原位，条目措辞随实现期载体词）                                           |
| 实现期配置           | 每实例在飞 async 任务预算（W4.3，超限 = 纯回压）；优雅窗口 + drain 总预算（W4.5）；host 侧事件在途投递界（A4 D10.2 原位存活）                                                                                                                              |
| 扩展位               | exclusive 上报 wait-for 可见性（W4.2，触发器 = 首个死锁调试痛点）；原子方法族其余成员（A4 D1.8 平移，触发器 = 首个真实消费者）；per-read timeout 参数（W4.4 → W6 章面，触发器 = 真实需求）                                                                 |
| seam-capabilities    | 旧 FAQ（引 `ctx.fanout`/`ctx.exclusive`）与旧「挂起点 = ctx 方法调用处」行 = 已消亡/重定义——**已清写 = T13-f**（2026-10-18）                                                                                                                               |
| ADR-0011             | 「独立解释器状态」「interrupt 时间盒」两格载体面 partial supersede——**同批落地 = Status 行注**（2026-10-17）                                                                                                                                               |
| context-events §0/§1 | worker 伪码、三层保险行与服务调用路径行清写——**已清写 = T13-c/T13-f**（2026-10-18）                                                                                                                                                                        |
| Creator 文档         | 作者契约四条平移（协作式 / 挂起窗口 / 同步全显式 / 在飞即成本，W4.1）+「等的事重叠 = guest async 原生；算的事并行 = 多实例/外部服务」分工条                                                                                                                |

---

## W5 错误与 result 习语

### W5.0 边界与前提

本章定：共享错误类型的 WIT 形状与封闭 kind 枚举；错误族清单；宿主↔guest 的方向纪律；「响亮报错」与 `result` 的载体分界；business 数据与 message 纪律；作者面投影与 A5 习语消亡。不归本章：ctx import 接口集与各方法签名 = W3；递归 Value 的 TLV 线编码与 resource 句柄族 = W6；契约形状来源与运行期校验细则 = W7/W9；事件声明与 emit 校验形态 = W8；能力门机械对账 = W9；WIT package version 比较与演进 = W10；trap 的运行时处置实现 = T10.10；journal kind 表 = journal D78。

前提（已定不议）：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；错误封套语义 = A5（`{kind, message, data?}`、kind 定控制流、三条分界判据、服务四态）；B5 要求呈现层类型化（[wasm-first-design-review](../research/wasm-first-design-review.md) §3）；接口数据三分法①（平台封闭面 = WIT 富类型；E1①/E2①/E5①，2026-10-16 拍板）；作者面错误封套 = [plugin-sdk](plugin-sdk.md) S5 D102；实例内执行模型 = W4；sqlite 撤回（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）；盒概念消解（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §2）。

| 节   | 议题                          | 状态               |
| ---- | ----------------------------- | ------------------ |
| W5.1 | 错误类型形状                  | ✅ D136 2026-10-19 |
| W5.2 | 错误族清单                    | ✅ D137 2026-10-19 |
| W5.3 | 方向性：宿主↔guest            | ✅ D138 2026-10-19 |
| W5.4 | 响亮 vs Result 的 wasm 载体   | ✅ D139 2026-10-19 |
| W5.5 | business data 与 message 纪律 | ✅ D140 2026-10-19 |
| W5.6 | 作者面投影与 A5 习语消亡      | ✅ D141 2026-10-19 |
| W5.7 | A5 机制消亡清单（考古索引）   | ✅ 2026-10-19      |
| W5.8 | 联动登记汇总（章末）          | ✅ 2026-10-19      |

### W5.1 D136 错误类型形状

**D136 定案：共享 `record error { kind, message, data? }` + 封闭 `enum error-kind`；`kind` 定控制流，`message` 只进日志，`data` 仅 business 携 TLV Value。**

```wit
package metis:plugin@0.1.0;

interface errors {
  record error {
    kind: error-kind,
    message: string,
    data: option<list<u8>>,
  }

  enum error-kind {
    // fs
    fs-not-found,
    fs-permission-denied,
    fs-invalid-path,
    fs-invalid-utf8,
    fs-invalid-type,
    fs-too-large,
    fs-io,
    // http
    http-timeout,
    http-connect,
    http-policy,
    http-too-large,
    http-invalid,
    http-error,
    // process
    process-spawn-failed,
    process-timeout,
    process-gone,
    // service（ADR-0018 S5 四态；business 共享）
    service-unavailable,
    service-method-missing,
    service-timeout,
    // 宿主框架族
    core-deadlock,
    core-contract-violation,
    core-mount-failed,
    // provider-defined
    business,
  }
}
```

- **world 引用**：W1.3 D118 的 world 以 `use errors.{error}` 引用本类型；生命周期导出的签名不变（`setup` / `on-config` / `dispose`）。

- **1:1 平移**：A5 的封套三字段与 [plugin-sdk](plugin-sdk.md) S5 D102 的三格封套逐字段对应。`kind` = WIT enum，控制流分支由编译器穷尽检查（A5 的 analyze #10「kind 串必须在枚举内」由编译器白拿）；`message` = 人读摘要；`data` = `option<list<u8>>`，内容 = TLV 编码的 `Value`（E1①，线编码归 W6/T10.13）。
- **`data` 纪律**：仅 `business` 使用；其余 kind 恒 `none`，宿主不读，SDK 不对非 business 暴露 data 语义。封闭族不需要结构化 data——message 足够，完整诊断进 journal。
- **族分组是命名约定**：kind 名的族前缀（`fs-` / `http-` / `process-` / `service-` / `core-`）只作分组与审计检索，不是类型分层；封闭 enum 已提供穷尽匹配。
- **选扁平 record 而非嵌套 variant**：A5 的三字段分工是设计本体，D102 作者面已冻结为同一封套；ABI 与作者面同形，避免第二套 SSOT 与 SDK 扁平整形层。代价明记 = 非 business 的 `data` 组合在结构上合法但被纪律禁止（宿主不读、SDK 不暴露）。
- **类型住处**：`crates/metis-plugin/wit/<version>/error.wit` 的 `interface errors`（W1.2 D117 的文件按章拆；版本子目录 = W10 D189）；world 与各 import interface 以 `use errors.{error}` 引用，W3 落地 import 接口时同源引用。不立独立错误版本号；kind 增补随 WIT package version（W10 D188）。
- **message**：必填字符串；宿主生成时给出非空摘要；guest business 错误可给空串，SDK 补默认文案。错误消息永不参与控制流（分支只对 `error-kind`）。

### W5.2 D137 错误族清单

**D137 定案：族 = fs / http / process / service / core / business；`sql.*` 与 `box.trap` 死亡；timer 无 Err 面。**

| 族       | kind                      | 触发                                                                                                    | 备注                                                                                          |
| -------- | ------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| fs       | `fs-not-found`            | read / list / delete 目标不存在                                                                         | A5 原位                                                                                       |
|          | `fs-permission-denied`    | OS 层拒绝（EACCES / EPERM / EROFS / ENOTCAPABLE，动作相同 = 放弃）                                      | private preopen 逃逸同格                                                                      |
|          | `fs-invalid-path`         | global 档两段式解析拒绝全族：相对路径（无根可相对）/ 逃逸 / 豁免命中 / 循环 / 超长；TOCTOU 竞态窗口同格 | message 通用化，不点名豁免命中；完整原因进 journal（D140）                                    |
|          | `fs-invalid-utf8`         | 文本读取的 UTF-8 校验失败                                                                               | **已兑现（W3 D144）**：ABI bytes-only；SDK `read_text` 产生；保留                             |
|          | `fs-invalid-type`         | read 目录 / list 文件                                                                                   | A5 原位                                                                                       |
|          | `fs-too-large`            | read 预算超限                                                                                           | 预算数值归 T10.11                                                                             |
|          | `fs-io`                   | 兜底（message = OS 原文，按 D89 脱敏）                                                                  | 开放空间诚实兜底格                                                                            |
| http     | `http-timeout`            | 强制超时命中（与 `service-timeout` 不同族：彼 = 服务调用 deadline，此 = 外部请求超时）                  | A5 原位                                                                                       |
|          | `http-connect`            | DNS / TCP / TLS / 连接中断一格（动作相同）                                                              | A5 原位                                                                                       |
|          | `http-policy`             | 域名白名单 / SSRF / 跨域重定向拒绝（部署者数据 → Err）                                                  | 与未声明能力的 loud 两界不混（W5.4）                                                          |
|          | `http-too-large`          | 响应预算超限                                                                                            | A5 原位                                                                                       |
|          | `http-invalid`            | URL 畸形等值层参数错误                                                                                  | 参数形状错仍走响亮（WIT 类型面静态排除）                                                      |
|          | `http-error`              | 兜底：响应畸形 / 解压失败 / 重定向异常                                                                  | 开放空间诚实兜底格                                                                            |
| process  | `process-spawn-failed`    | 命令不存在 / 权限 / fork 失败                                                                           | A5 原位                                                                                       |
|          | `process-timeout`         | exec 强制超时杀进程组                                                                                   | A5 原位                                                                                       |
|          | `process-gone`            | write / signal 已退出进程（高频竞态）                                                                   | read 在进程死后 = 正常 EOF，不算错误                                                          |
| service  | `service-unavailable`     | 提供方缺席 / 崩溃 / 卸载窗口；可选依赖常态答案                                                          | ADR-0018 S5；D11.3 即错即回                                                                   |
|          | `service-method-missing`  | 方法名不存在（契约已知时宿主查注册表拦下，不走消息）                                                    | 无契约服务 = guest catch-all 返回 `business`（W3 D147 退化格）                                |
|          | `service-timeout`         | 服务调用 deadline / 提供方被 fuel / epoch 截断                                                          | ADR-0018 S5                                                                                   |
| core     | `core-deadlock`           | wait-for 圈闭合（W4.4 层三）；message 带整圈路径（按实现期上限截断）                                    | 宿主检测 + fail-open 解圈；不占服务族第五格                                                   |
|          | `core-contract-violation` | 运行期契约 / 方向违约：服务参数或返回值形状不符、emit payload 不符、guest 导出返回非 business kind      | journal `contract_violation`；message 人读，结构化细节住 journal                              |
|          | `core-mount-failed`       | mount 的启用管线失败：声明有效但子实例未激活（setup Err / trap、依赖缺失、预算 / 并发上限）             | W3 D150；message 带子实例名与原因；unmount 幂等                                               |
| business | `business`                | guest 起源的业务错误（服务 handler 的业务失败 / 生命周期钩子与事件 listener 的唯一合法导出 kind）       | `data` = TLV Value；服务路径形状 SSOT = 服务契约（W7/W9）；非服务路径由 SDK 约定（通常 none） |

- **W6 增补（2026-10-19）**：TLV / blob 的条目 / 深度 / 字节预算超限 = `core-contract-violation` Err（边界预算格；message 带边界与预算）；不新增 kind。
- **三条分界判据平移**（A5 D14.1）：① `Err` vs `Ok` 内状态字段看载荷是否完整成立——HTTP 4xx/5xx、进程非零 exit = `Ok` 载荷分支，状态码 / exit code 不进 kind；② `Err` vs 响亮报错看静态可否查——契约违反 = loud（W5.4），运行期数据决定 = `Err`；③ kind 细分粒度看调用方动作是否不同，开放空间（OS / 网络 / 进程）必设诚实兜底格。
- **死亡项**：`sql.*` 族随 sqlite 能力撤回永久死亡（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)；DB = guest crate + private fs）；`box.trap` 随盒概念消解（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §2）——插件 trap = 实例死亡 + journal `vm_trap`，在飞调用方收 `service-unavailable`（D11.3），不立 guest 可见的 trap kind；timer 无 Err 面，参数非法走响亮（W5.4）。
- **service 四态不立第五格**：`service-unavailable` / `service-method-missing` / `service-timeout` + 共享 `business` 即 ADR-0018 S5 四态；服务调用路径的调用方可见错误集合 = 这四格 + `core-deadlock` / `core-contract-violation`（宿主框架族）。`core-*` 不改变服务注册表语义，也不占服务族第五格；ADR-0018 正文与 Status 不动。
- **封闭枚举演进**：新增 kind = 新 WIT package version（W10 D188）；不做开放字符串，也不在 guest 侧枚举未知 kind 的透传槽——跨版本共存时宿主按 `abi` 使用对应版本绑定（W5.3 / W10 D189），旧版绑定不接收新版 kind。

### W5.3 D138 方向性：宿主↔guest

**D138 定案：共享类型 + 方向纪律；guest 导出只许 `business`；guest 返回非 business kind = 宿主判 `core-contract-violation`。**

- **import（宿主→guest）**：宿主可产生 fs / http / process / service / core 族；`business` 仅由 guest 提供方产生、宿主在服务调用路径上透传，宿主自身不产生 `business`（宿主级失败用 core 族）。
- **export（guest→host）**：生命周期钩子、服务 handler、事件 listener 的 `Err` 只许 `business`；`data` = TLV Value（服务路径 = 契约形状；生命周期 / 事件路径 = SDK 约定，通常 none）。SDK 在作者面只提供 `Error::business(data)` 作为导出错误构造入口（具体 API 归 T10.14），文档同条。
- **非 business 的导出错误 = 方向违约**：宿主不静默降级为 business，而是判 `core-contract-violation` 并按出口处置——服务 handler → 调用方收该格 + journal `contract_violation`；`setup` → 加载失败（W1.5），日志记违约；`on-config` → 重挂载（W1.3），日志记违约；`dispose` → 记日志、继续 drain（W1.6）；事件 listener → emit / serial / waterfall = `listener_error` + `contract_violation`，fail-open；parallel = 该项结果计 `core-contract-violation`（A5 D15.2 / ADR-0014）。
- **不立两个 WIT 类型**（`host-error` / `guest-error`）：一个封套是 A5 的设计本体；方向纪律在宿主一处归一即可；两个类型不带来安全收益（business data 本就是任意 TLV），却让 W1.3 签名与 SDK 映射双份。
- **跨版本 kind 面**：`error-kind` 是封闭 enum，新增 case = 源码面破坏式（W10 D188）；跨版本共存（W10 D189）时宿主按组件声明的 `abi` 使用对应版本的 WIT 绑定，不得把新版 kind 送进旧版绑定；宿主侧新失败按该快照的投影表落到既有 kind（W10 D188）。

### W5.4 D139 响亮 vs Result 的 wasm 载体

**D139 定案：调用契约违反（静态可查的 API 误用，含 timer 参数越界 / 卸载期注册）= trap；运行期交互 / 形状失败 = `core-*` Err；运行期数据失败 = 族 Err。**

| 情形                                                                                         | 载体                                                                                                                                             |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 未声明能力调用（manifest 门：fs global / http / process / mount 缺省）                       | trap——宿主 import 拒绝，实例 Failed，journal `vm_trap`；依赖方收 `service-unavailable`                                                           |
| guest 四派发 emit / serial / parallel / waterfall 冒用 `internal/*`（W8 D176）               | trap——命名空间边界冒用；宿主拒绝调用，实例 Failed，journal `vm_trap`；依赖方收 `service-unavailable`；运行期构造的名字同判，转发型插件须自行校验 |
| Unloading 期新注册「比我长寿」的账（监听 / provide / timer）                                 | trap——实例本就在卸载，无额外损失（W4.5 D11.2）                                                                                                   |
| timer 参数非法（`every` 低于最小间隔等）                                                     | trap——参数非法 = 契约违反，A5 timer「无 Err 面」平移                                                                                             |
| import 参数形状错（WIT 类型面已静态排除；服务 TLV 参数运行期违约、process argv/command XOR） | `core-contract-violation` Err；fs 路径值错误归 `fs-invalid-path`（W3 D144）；违约为参数侧还是返回侧由 message + journal 指示                     |
| 服务返回形状违约 / emit payload 违约                                                         | `core-contract-violation` Err + journal `contract_violation`（D19.1 / D22.4 的运行期校验点存活）                                                 |
| wait-for 圈闭合                                                                              | `core-deadlock` Err 给调用方（W4.4 fail-open 解圈；message 带整圈路径）                                                                          |
| 运行期数据失败（文件不存在 / 网络失败 / 进程退出等）                                         | 对应族 Err                                                                                                                                       |
| 事件 listener 失败                                                                           | emit / serial / waterfall = fail-open：journal `listener_error`，不进控制流；parallel = 逐项 `result` 收集（A5 D15.2 / ADR-0014）                |
| guest trap（panic / abort / fuel / epoch 截断自身）                                          | 实例死亡；journal `vm_trap`；在飞调用方按 D11.3 收 `service-unavailable`                                                                         |
| `setup` 返回 Err / trap                                                                      | 加载失败，实例即弃（W1.5 D120）                                                                                                                  |
| `on-config` 返回 Err / trap                                                                  | 重挂载（W1.3 D118 / config-format D42）                                                                                                          |
| `dispose` 返回 Err / trap                                                                    | 记日志，不中断后续清理（W1.6 D121）                                                                                                              |
| manifest / config / 安装校验失败                                                             | 宿主侧响亮（miette / 启用期拒），不经 guest 错误通道（W2 D133 / W1.5）                                                                           |

- **为何「响亮」= trap**：wasm 没有可恢复的 guest 异常；guest panic = abort = trap = 实例死亡。这是载体的诚实事实，也是 A5「契约违反不进 Result」的忠实兑现——trap 不可被 `Result` 分支吞掉。代价明记 = 可用性：一个未声明能力调用会杀死插件，依赖方收 `service-unavailable`；作者应在开发期用 `metis check` 与 SDK 类型面排除。
- **为何 deadlock 与运行期形状违约不 trap**：它们是运行期交互结果，不是静态作者错；W4.4 明确「当场报错 + 整圈路径 + fail-open 解圈」，返回 `core-deadlock` 让调用方解圈，比杀实例更符合 fail-open；形状违约的调用方 / 提供方可能因升级竞态而持有旧契约，返回 `core-contract-violation` 让调用方统一失败 / 告警，具体违约侧由 message + journal 指示。
- **未声明能力 vs 部署策略拒绝两界不混**：manifest 缺省 = loud（trap）；manifest 声明但部署数据拒绝（http 白名单 / SSRF / 跨域重定向）= `http-policy` Err（A5 D14.5 平移）。
- **不立 `retryable` 位**：A5 D14.6 三理由原样成立（错误性质 × 操作幂等 × 业务语义的乘积超出宿主知识范围；单 bit 诱导裸重试；信息不明）。kind 分格已含动作信息；重试决策归作者，business 的重试语义归契约。

### W5.5 D140 business data 与 message 纪律

**D140 定案：business `data` = TLV 编码的 Value，宿主不解释；`message` = 人读摘要，不泄 secrets / 受保护路径细节；错误不做 message 匹配。**

- **business data**：`data = option<list<u8>>`，内容 = TLV 编码的 `Value`（E1①；codec = T10.13）；宿主不解释业务语义，也不对 business payload 做契约形状校验（A5 A7.4：`errors:` 节不进运行期校验）；成功返回值的形状仍按 D19.1 校验。形状 SSOT = 服务契约（W7/W9）+ SDK 生成类型。接收方 SDK 解码失败 = 本地作者面错误（不回流宿主），因为宿主对 business payload 无形状知识。
- **host 族 data**：恒 `none`；宿主不把结构化诊断塞进 `data`——有专门 journal kind 的族（`fs-invalid-path` → `fs_invalid_path_detail`）进 journal，其余族的完整原因 = message（按 D89 脱敏）。
- **message**：人读摘要；宿主按族政策生成——`fs-invalid-path` 通用化（不点名受保护路径 / 豁免清单），完整原因进 journal `fs_invalid_path_detail`；其余族可带 OS / 库原文，但按 D89 脱敏（secrets / 受保护路径）。guest business message 由提供方负责，宿主不二次加工。
- **无 message 匹配**：控制流只对 `error-kind`；SDK 不提供 message 匹配 API，`Error` 的 `Display` 只服务日志。
- **错误通道自身必须无敌**：宿主产生封套失败（编码 / 分配）不在设计面内；guest business data 是不可解释字节，最坏情况是接收方解码失败，不会二次杀死宿主。A5 的「不可转换错误值落字符串摘要」carve-out 因此消亡（B5）。
- **预算**：business data 的字节预算沿用 W6 / T10.13 TLV 编解码预算；`message` 是 WIT `string`，不适用 TLV 预算——宿主侧设长度上限（实现期配置，登记 T10.10 / T10.11 / T10.14），超限截断并 journal 注记；`core-deadlock` 的整圈路径同受该上限约束。W5 不冻结数值。

### W5.6 D141 作者面投影与 A5 习语消亡

**D141 定案：SDK 作者面 = D102 三格封套不变；`ErrorKind` 与 ABI `error-kind` 1:1；A5 的呈现层习语消亡或换载体。**

- **类型投影**：`metis_plugin::Error { kind: ErrorKind, message: String, data: Option<Value> }`；`ErrorKind` 与 `error-kind` 逐格对应（kebab-case → Rust variant）；`Error` 实现 `std::error::Error` + `Display`；`Result<T, Error>` 贯穿效应调用与钩子返回。
- **构造面**：`Error::business(data)` 为 guest 导出的唯一合法构造入口；宿主族 kind 的构造器只服务测试 / 宿主侧映射，SDK 文档明确其不可用于导出返回（宿主兜底 = W5.3）。具体构造器命名与可见性归 T10.14。
- **作者契约**（A5 D15.3 的 Rust 版）：① 错误必须处置——`?` 传播或 match，裸丢弃受 `#[must_use]` 警告（lint 面随载体消亡，纪律仍在）；② kind 定控制流，message 只进日志；③ 重试自由、幂等自负，无 retryable 位；④ guest 导出只返回 business，data 形状按契约；⑤ 响亮错误（trap）不可恢复，不要设计成可捕获。
- **习语映射**：`assert` 升级 → `?`；`error(err)` 透传 → `?`；fail-fast 姿态 → `expect` / `panic`（注意 = trap = 实例死亡，只用于真正的不可恢复分支）；`res, err` 两槽 → `Result`；`Ok` 载荷不为 nil → WIT 类型无 nil，`option<T>` 显式表达可选。
- **SDK 不做 message 匹配**：`ErrorKind` 是唯一分支面；`Error::message()` 只给日志 / 展示。

### W5.7 A5 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 [ADR-0026](../decisions/0026-luau-abi.md)（已 superseded）。

| 机制                                      | 死因                                                                                                           |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 多返回值 `res, err` 两槽（A5 D12.1）      | WIT `result<T, error>` 原生；Rust `Result` + `?`                                                               |
| `assert` 升级习语                         | `?` / `expect`；panic = trap = 实例死亡，只保留 fail-fast 姿态                                                 |
| `nil, nil` = EOF 位                       | 流式 read 的 EOF = `Ok(none)`（W6 D162 已兑现）；非错误                                                        |
| 「Ok 载荷永不为 nil」纪律                 | WIT 类型无 nil；可选值用 `option<T>` 显式表达                                                                  |
| err 纯数据表 / frozen 待遇                | WIT record 值语义；无 frozen 概念                                                                              |
| kind 两级字符串 / 封闭枚举串              | WIT `error-kind` enum；编译器穷尽检查（analyze #10 白拿）                                                      |
| `sql.*` 错误族                            | sqlite 撤回（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）；DB = guest crate + private fs          |
| `box.trap`                                | 盒概念消解（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §2）；插件 trap = 实例死亡 + journal `vm_trap` |
| `error(v)` 深快照 / 不可转换 carve-out    | `data` = TLV Value（E1①）；不存在任意错误值，也不存在「错误通道自身失败」                                      |
| uncaught 独立格 / fanout 子任务错误收集   | guest trap = 实例死亡，宿主无收集面（W4.2）；handler Err = business                                            |
| `return nil, err` 非法升级通道            | `?` 唯一升级通道；无两槽                                                                                       |
| `retryable` 布尔标识                      | A5 D14.6 从未立；kind 分格 + 契约语义                                                                          |
| analyze #9 err 槽裸丢弃 / #10 kind 串枚举 | Rust `#[must_use]` 警告 + 穷尽 match；AST lint 管线随载体消亡（design-review A8）                              |
| V3 两槽关联 narrowing                     | 无两槽；`Result` 类型原生收窄                                                                                  |

**语义平移保留**：三条分界判据（D14.1）；kind 定控制流、message 只进日志、data 按族查表；服务四态（ADR-0018 S5）；事件 fail-open（ADR-0014）；契约违反响亮、运行期数据失败 `Err`；不立 retryable。

### W5.8 联动登记汇总（章末）

| 去向              | 内容                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1（T13-h）       | **已兑现（2026-10-19）**：`error` 类型 = D136；W1.3 生命周期签名确认；`setup` Err / trap = 加载失败（W1.5）；`dispose` Err / trap = 记日志继续（W1.6）                                                                                                                                                                                                                                                                                                                 |
| W2（T13-i）       | **已兑现（2026-10-19）**：manifest 校验错误 = 宿主侧响亮，不经 guest 错误通道（D133）；生命周期 `error` 类型 = D136                                                                                                                                                                                                                                                                                                                                                    |
| W3（T13-j）       | **已兑现（2026-10-19）= W3 D142–D154**：所有 fallible import 返回 `result<T, error>`；能力门未声明 = trap（D139）；fs / http / process 错误映射；timer 无 Err 面（tick 合并计数可见）；`mount` 未声明 = trap；服务调用返回 service / business / core 族；`fs-invalid-utf8` **保留**（ABI bytes-only，SDK `read_text` 产生）；`core-mount-failed` 增补（mount 启用失败）；crypto 撤回 = [ADR-0041](../decisions/0041-crypto-capability-withdrawal.md)，无 crypto 错误面 |
| W4（T13-c）       | **已兑现（2026-10-19）**：W4.4 fail-fast 错误面 = D137 / D139——wait-for = `core-deadlock`；卸载期新账 = trap；D11.3 = `service-unavailable`                                                                                                                                                                                                                                                                                                                            |
| W6（T13-l）       | **已兑现（2026-10-19）= W6 D155–D162**：流式 read EOF = `Ok(none)`（显式，非错误）；reader / writer / blob 错误复用本类型；TLV / blob 预算 / 版本失败 = `core-contract-violation`；business data = TLV；无新增 kind                                                                                                                                                                                                                                                    |
| W7（T13-m）       | **已兑现（2026-10-19）= W7 D163–D172**：business data 形状 SSOT = 契约 `errors:`（D167）；运行期形状违约 = `core-contract-violation`（D169）；`data` 承载 = TLV（D166）                                                                                                                                                                                                                                                                                                |
| W8（T13-n）       | **已兑现（2026-10-19）= W8 D173–D179**：listener 错误封套 = 本类型；emit 校验违约 = `core-contract-violation` Err + journal（D139 运行期形状失败格）；`internal/*` 冒用 = trap（`vm_trap`）；不新增错误族                                                                                                                                                                                                                                                              |
| W9（T13-o）       | **已兑现（2026-10-19）= W9 D180–D185**：能力对账 + 运行期门；未声明能力 = trap；契约运行期校验点 = `core-contract-violation`；manifest 缺省 vs 部署拒绝两界不混                                                                                                                                                                                                                                                                                                        |
| W10（T13-p）      | **已兑现（2026-10-19）= W10 D186–D193**：`error-kind` 增补 = 新快照（D188）；跨版本按声明绑定、不把新 kind 送入旧版（D189）；无独立错误版本号                                                                                                                                                                                                                                                                                                                          |
| journal           | 不新增 kind；`listener_error` / `vm_trap` / `contract_violation` / `fs_invalid_path_detail` 原位承接；错误 kind 落盘用 kebab-case 规范名                                                                                                                                                                                                                                                                                                                               |
| seam-capabilities | §6.1 W5 行兑现：能力失败 `result` 形态 + service 四态；`fs-invalid-path` message 通用化政策                                                                                                                                                                                                                                                                                                                                                                            |
| plugin-sdk        | **已兑现（2026-10-19）**：D102 三格封套不变；`ErrorKind` 1:1；guest 导出只许 business；`?` 习语                                                                                                                                                                                                                                                                                                                                                                        |
| service-registry  | ADR-0018 S5 四态 = `service-unavailable` / `service-method-missing` / `service-timeout` + 共享 `business`；core 族不占第五格；调用方可见集合 = 四态 + core 两格（服务调用路径）                                                                                                                                                                                                                                                                                        |
| T10.10            | trap → 实例 Failed + journal `vm_trap`；在飞调用方 `service-unavailable`                                                                                                                                                                                                                                                                                                                                                                                               |
| T10.11            | 能力错误映射（fs / http / process）；未声明能力 trap；timer 参数 trap                                                                                                                                                                                                                                                                                                                                                                                                  |
| T10.13            | business data = TLV Value（`metis-codec`）                                                                                                                                                                                                                                                                                                                                                                                                                             |
| T10.14            | `Error` / `ErrorKind` 类型与构造器；SDK 文档（服务四态 + 服务路径 core 两格、business-only 导出、message 不分支）                                                                                                                                                                                                                                                                                                                                                      |
| Creator 文档      | 作者契约五条（W5.6）：`?` / kind 分支 / business-only 导出 / business data 契约形状 / trap 不可捕获；「族 → 建议动作」表按 kind 清单重写                                                                                                                                                                                                                                                                                                                               |
| 实现期配置        | message 长度上限（宿主侧截断 + journal 注记；登记 T10.10 / T10.11 / T10.14）；core-deadlock 路径同受该上限；business data 预算沿用 W6 / T10.13                                                                                                                                                                                                                                                                                                                         |
| 扩展位            | 非 business 族的结构化 `data` 不立（将来需要走 package version）；`retryable` 维持不立；per-family data = 扩展位                                                                                                                                                                                                                                                                                                                                                       |
| ADR               | 不新增；A5 语义继承（ADR-0026 已 superseded）；ADR-0018 S5 四态不动（调用方可见集合 = 四态 + core 两格（服务调用路径），见 W5.2）                                                                                                                                                                                                                                                                                                                                      |

---

## W6 边界转换：canonical ABI / TLV / resource 句柄族

### W6.0 边界与前提

本章定：canonical ABI 的拷贝纪律与热路径预算；递归 `Value` 的 TLV 线编码（语义合同）；`Value` 与 WIT 类型的映射及严格制；`Value::Bytes` 提正；大负载的 blob / byte-stream 车道；`payload` 双车道形状；resource 句柄生命周期与跨实例边界；边界失败面、预算与 EOF。不归本章：运行时调度与投递 = W4；契约形状来源与运行期校验 = W7/W9；事件名语义与 `on-event` 导出的最终形状 = W8；能力对账 = W9；package version 与首个发布快照 = W10；宿主实现 = T10.10 / T10.11；SDK 实现 = T10.14。

前提（起点基线；本章可在其上修订）：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；接口数据三分法（E1① / E2① / E5①，2026-10-16 拍板）；递归 `Value` 走 TLV 主线（E1①）；`list<u8>` 入 / `result<list<u8>, list<u8>>` 出的琐碎签名（A6.6 平移；W6.6 把服务 / 事件 payload 改为 `payload` 变体，其余 TLV 面不变）；错误面 = W5 D136–D141；执行模型 = W4（D93–D97，自由交错使 guest 内存在 await 期间可变）；边界拷贝四层语义 = [wasm-research](../research/wasm-research.md) 补遗 5 §5.4；S3 blob 跨实例 spike **PASS** = [wasm-research](../research/wasm-research.md) 补遗 10；星形拓扑 = [ADR-0011](../decisions/0011-actor-execution-model.md)（不改为直连/共享内存）。

| 节    | 议题                                           | 状态               |
| ----- | ---------------------------------------------- | ------------------ |
| W6.1  | 边界拷贝纪律与热路径预算                       | ✅ D155 2026-10-19 |
| W6.2  | canonical binary Value 格式（TLV）             | ✅ D156 2026-10-19 |
| W6.3  | Value 类型映射与严格制                         | ✅ D157 2026-10-19 |
| W6.4  | `Value::Bytes` 提正（B6）                      | ✅ D158 2026-10-19 |
| W6.5  | 大负载双车道：blob / byte-reader / byte-writer | ✅ D159 2026-10-19 |
| W6.6  | `payload` 形状：`value` / `blob` 变体          | ✅ D160 2026-10-19 |
| W6.7  | resource 生命周期与跨实例边界                  | ✅ D161 2026-10-19 |
| W6.8  | 失败面、预算与 EOF                             | ✅ D162 2026-10-19 |
| W6.9  | f2 兑现：ADR-0015/0022 论据换血                | ✅ 2026-10-19      |
| W6.10 | A6 机制消亡清单（考古索引）                    | ✅ 2026-10-19      |
| W6.11 | 联动登记汇总（章末）                           | ✅ 2026-10-19      |

### W6.1 D155 边界拷贝纪律与热路径预算

**D155 定案：跨界只走 canonical ABI；不立共享内存 / 宿主内存别名 / 直连旁路；在 ABI 内冻结一组必须实现的拷贝与分配优化。**

- **拷贝下限是结构性的**：

| 路径                            | 拷贝次数 | 原因                                                                                    |
| ------------------------------- | -------- | --------------------------------------------------------------------------------------- |
| 源 guest → 宿主快照             | 1        | W4.1 自由交错：guest 内存在 await 期间可被同实例其他任务改写，宿主不能跨挂起持有借用    |
| 宿主 → 每个目标 guest           | 1        | 各自独立线性内存，canonical ABI 只能写入目标内存                                        |
| 宿主内部转发 / 只读服务         | 0        | 同一份 `Arc<[u8]>` / `Arc<Vec<u8>>` 借用                                                |
| resource 句柄传递               | 0        | 只传句柄，数据留在宿主                                                                  |
| blob 转发（生产者→宿主→消费者） | 中间 0   | 宿主重物化句柄（S3 补遗 10）；每个接收方 `read` 时按块 1 次                             |
| 共享内存                        | 0        | 与实例隔离 / 限额 / 池化冲突，[ADR-0024](../decisions/0024-carrier-layering.md) C2 已禁 |

- **必须实现的优化**（W6 冻结要求，落地归 T10.10 / T10.13 / T10.14）：
  1. **encode once**：TLV 编码结果以 `Arc<[u8]>` 共享；N 路 fanout = 1 次编码 + N 次目标写入，不重复编码。
  2. **wire-first**：宿主边界表示以编码字节为准，`Value` 树按需构建；转发 / 路由默认复用编码字节，不重建树。journal 不在此列——它按 [journal](journal.md) D76/D89 的 `Value` 语义与脱敏管线处理（需要时解码；`Bytes` 落盘 = base64 包裹，W6.4）。
  3. **borrowed decode**：`ValueRef<'a>` 从字节缓冲借用字符串 / 字节 / 容器，零分配；需要持有时才转 `Value`。
  4. **typed decode**：SDK 生成的契约类型直接从字节解码（schema-directed），不经过中间 `Value` 树；宿主形状校验也走字节走查。
  5. **direct-to-guest encode**：codec 提供 `encoded_len` + `encode_into`；宿主经 guest `realloc` 直接写入目标内存，省掉「宿主 Vec + 一次 memcpy」。
  6. **大负载 resource 化**：见 W6.5；不把大块数据塞进 TLV。
  7. **热路径无全局锁**：路由表 / 监听者表 / 服务注册表读多写少，用快照或分片；journal 不逐消息写（v1 骨架已避开）。
- **成本量级**（估算，非实测；精确值归 T10.4 的 D6 吞吐件 + T10.13 codec 基准）：memcpy 约 15.6 GiB/s，1 KB ≈ 64 ns；1 MB ≈ 64 µs。小消息高频时，每消息固定开销（canonical ABI / TLV / 分配 / wakeup / 路由）通常大于拷贝；大载荷或高 fanout 时拷贝才成为主瓶颈。

| 场景               | 每消息拷贝 | 100k msg/s | 1M msg/s | 判断                               |
| ------------------ | ---------- | ---------- | -------- | ---------------------------------- |
| 1 KB 单播          | 2 KB       | 0.2 GB/s   | 2 GB/s   | 拷贝不是瓶颈；CPU 调度 / 编解码是  |
| 1 KB 10 路 fanout  | 11 KB      | 1.1 GB/s   | 11 GB/s  | 1M/s 时内存带宽开始显著            |
| 1 KB 100 路 fanout | 101 KB     | 10.1 GB/s  | 101 GB/s | 高频 + 大 fanout = 带宽瓶颈        |
| 64 KB 单播         | 128 KB     | 12.8 GB/s  | 128 GB/s | 100k/s 已逼近单机带宽              |
| 1 MB 单播          | 2 MB       | 200 GB/s   | ——       | 必须走 blob / reader，不能整段 TLV |

- **淘汰**：共享内存 / 宿主内存别名（破坏隔离与限额）；直连通道（绕开能力门、fiber 账本、journal 拦截与故障遏制）；惰性代理表（无共享内存时无对象可代理）。
- **架构建议（Creator / SDK 文档）**：高频小消息优先批量化（契约 batch 方法）；极高频成对通信应合并进一个插件或改用粗粒度服务；大载荷永远走 blob / reader；事件 fanout 用事件，不用服务调用模拟广播。

### W6.2 D156 canonical binary Value 格式（TLV）

**D156 定案：递归 `Value` 的线格式 = metis 自有 canonical binary Value 格式（TLV 家族）；W6 冻结语义合同，字节位分配归 T10.13。**

- **为什么不是「Lua 妥协」**：WIT 拒绝一切递归类型（[wasm-research](../research/wasm-research.md) 补遗 4，五种形态全拒）；服务参数 / 事件 payload / config 是递归 `Value`，因此只能走 `list<u8>` + 自描述编码。Lua 时代真正妥协的是 A6.6 的具体编码选择（位分配、NaN-box 参照、frozen table 心智），不是 TLV 这件事本身。
- **语义合同**（W6 冻结）：
  - 类型全集 = `Null / Bool / Float(f64) / Int(i64) / String / Bytes / Array / Map`（W6.3/W6.4）；自描述，无外部 schema 即可解码；
  - compact：varint 长度 / 计数，小整数内联，字符串 / 字节 / 容器 = tag + 长度 + 载荷；
  - canonical：Map 键按 UTF-8 字节序排序、最短 varint、禁重复键、`f64` 固定编码——**同一 `Value` 树 → 同一字节序列**；
  - 每载荷带格式版本字段；不识版本 = **解码前**失败（config = 启用期响亮失败；服务 / emit / mount = `core-contract-violation` Err，W6.8）；格式版本与 WIT package version 独立（前者管字节布局，后者管接口集；演进纪律分别归 T10.13 / W10 D191）；
  - 预算为形状：条目数 / 嵌套深度 / 总字节；数值 = 实现期配置（T10.13）；超限 = 边界错误（W6.8），不做部分解码后静默截断；
  - `Map` 键只允许 UTF-8 字符串；`Array` / `Map` 严格二分（混合 / 稀疏 / 非字符串键 = 边界错误）。
- **两种解码模式，同一线格式**：动态 `Value` / `ValueRef<'a>`；契约已知时 typed decode（W7 `types!()` 生成 `T::decode`）。不为 typed 面另立紧凑格式——标签成本可忽略，双格式会分裂版本与测试面。
- **字节布局**：tag 位分配、版本号数值、varint 细节 = T10.13 实现期定稿；冻结时点 = 首插件发布前（W10 D190 记录首个快照）。W6 只冻结上述语义与解码顺序。
- **淘汰**：CBOR deterministic 子集（标准但与既有 `list<u8>` 琐碎签名 / 自有位分配纪律冲突，且引入第三方 codec 依赖）；typed-first WIT 服务接口（重开 E2①；宿主动态路由 + 静态 world 生成问题）；WIT 扁平节点表（ABI 形状膨胀、拷贝面更大）。

### W6.3 D157 Value 类型映射与严格制

**D157 定案：`Value` 变体与 WIT 类型一一映射；`Int` / `Float` 由 TLV tag 严格区分，不做隐式 coercion；WIT 面不出现递归与 `any`。**

| `Value`      | TLV 语义              | WIT / 作者面                          |
| ------------ | --------------------- | ------------------------------------- |
| `Null`       | 空值                  | `option<T>` 的 `none` / `Value::Null` |
| `Bool`       | 布尔                  | `bool`                                |
| `Float(f64)` | IEEE-754 双精度       | `f64`                                 |
| `Int(i64)`   | 64 位有符号整数       | `s64` / `i64`                         |
| `String`     | UTF-8 文本            | `string`（跨边界强制 UTF-8）          |
| `Bytes`      | 任意字节              | `list<u8>` / `Vec<u8>` / `Arc<[u8]>`  |
| `Array`      | 有序值列表            | `list<Value>`                         |
| `Map`        | 字符串键 → 值，键排序 | `BTreeMap<String, Value>`             |

- **严格制**：`Int` 槽位只收 Int tag，`Float` 槽位只收 Float tag；整值浮点 / 超精度往返一律响亮失败（A6.1 严格制在 `Value` 层存活，载体从 Luau tag 换成 TLV tag）。无契约覆盖的自由态 payload 不做形状检查，`Value` 原样透传。
- **平台封闭面不走 TLV**：fs / http / process / timer / instance / services 控制面用 WIT 富类型（三分法①）；开放数据面（config、服务 args/returns、事件 payload、business data、mount 声明）用 `list<u8>` + TLV。任何新 import 必须归入三格之一（WIT 富类型 / TLV / blob·reader）；不立第四种形态。
- **WIT 面约束**：不出现递归类型；不出现 `any`；`string` = UTF-8；二进制线面 = `list<u8>`（`Value::Bytes` / SDK `Bytes` 映射到它）；缺席用 `option<T>`，失败用 `result<T, error>`（W5）。
- **config**：`setup` / `on-config` 的 config 仍是 TLV `Value`；YAML 没有二进制类型，`Bytes` 只从程序 / 事件 / 服务路径产生，不从 config 解析产生。

### W6.4 D158 `Value::Bytes` 提正（B6）

**D158 定案：`Value` 增加 `Bytes(Arc<[u8]>)`；正式记录 = [ADR-0042](../decisions/0042-value-bytes.md)；局部取代 ADR-0015 的 Bytes 扩展位格与 ADR-0022 的「Bytes 维持扩展位」行。**

- **理由**：ADR-0015 的触发器和成本结构按 Luau 设定（buffer 映射 + 类型词 + 编码纪律）；wasm 下 `list<u8>` 原生，跨插件二进制与文本同权（design-review B6）。`Value::String` 强 UTF-8，当前跨插件二进制没有合法载体。
- **形态**：`Bytes(Arc<[u8]>)`；`Send + Sync` 不变；解码时优先借用（`ValueRef::Bytes(&[u8])`），需要持有时才 `Arc` 化。
- **范围纪律**：`Bytes` = **数据模型内的二进制**（事件 / 服务参数 / business data），受 TLV 条目 / 深度 / 总字节预算约束；**大二进制不塞 `Bytes`**，走 W6.5 的 blob / byte-reader。
- **journal**：JSONL 落盘表示 = base64 包裹对象（与 D77 Int 包裹同族）；字节预算沿用 TLV 预算；实现归 T10.5。
- **联动**：`context-events` §3 方案 C 行改为「Bytes 已提正 = ADR-0042」；T10.1 站表 / 遗留问题登记删除扩展位行；T10.13 TLV 增 bytes tag；T10.14 SDK 增 `Bytes` 类型。

### W6.5 D159 大负载双车道：blob / byte-reader / byte-writer

**D159 定案：大负载走三条 resource 车道——`blob`（不可变、可共享、可跨实例只读）、`byte-reader` / `byte-writer`（一次性流，宿主↔guest）；TLV 只承载小 / 中数据。**

```wit
interface blobs {
  use errors.{error};

  /// 不可变、宿主持有的字节对象。句柄 0 拷贝；内容按 read 分块拷贝。
  resource blob {
    duplicate: func() -> own<blob>;          // 引用计数 +1，0 拷贝
    len: func() -> u64;
    read: func(offset: u64, max: u32) -> list<u8>; // offset >= len = 空列表；无错误面
  }

  /// guest 字节 → 宿主 blob（一次写拷贝）；超预算 = core-contract-violation。
  from-bytes: func(data: list<u8>) -> result<own<blob>, error>;
}

interface streams {
  use errors.{error};
  use blobs.{blob};

  resource byte-reader {
    read: async func(max: u32) -> result<option<list<u8>>, error>; // Ok(none) = EOF
    collect: async func(max: u64) -> result<own<blob>, error>;     // 读尽并转 blob；超 max = core-contract-violation
    close: func();                                                // 幂等；close 后 read = Ok(none)
  }

  resource byte-writer {
    write: async func(data: list<u8>) -> result<_, error>;
    close: async func() -> result<_, error>;                       // 幂等；fs 写 = tmp+rename 原子提交
  }
}
```

- **采用面（v1）**：
  - `fs.open-read(path) -> result<own<byte-reader>, error>` / `fs.open-write(path) -> result<own<byte-writer>, error>`（global 档）；private 档由 WASI 0.3 `wasi:filesystem` 原生资源承接；SDK 用 `ByteReader` / `ByteWriter` 统一包装 metis 与 WASI 资源（enum / newtype 形态归 T10.14），作者面保持同一 `Fs`。
  - `http.download(req) -> result<stream-response, error>`，`stream-response { status, headers, body: own<byte-reader> }`；`http.request` 的 v1 全量响应带预算不变；http 流式上传维持扩展位（W3 D148 原位）；
  - `process` 保持 W3 句柄与 `result<option<list<u8>>, error>` 分块读，不改写；
  - `instance.mount` 声明保持 TLV（小、结构化）。
- **blob 语义**：不可变；宿主引用计数；跨实例传递 = 宿主在目标实例 `ResourceTable` 重物化句柄（S3 补遗 10：同一 `Arc`，0 数据拷贝）；生产者卸载不影响其他持有者；内容默认不进 journal（v1 无 blob kind；审计层扩展位若开启才评估元数据 / 引用，见 W6.8）；按租户计内存预算——guest 可触发路径（`from-bytes` / `collect`）超限 = `core-contract-violation` Err；host 侧创建超限 = 拒绝 + host tracing 记原因。
- **blob 跨实例边界**：只允许 `blob` 这类**只读不可变**资源跨实例；`byte-reader` / `byte-writer` / `process-handle` / `mount-handle` 等可变 / 有副作用资源维持 per-instance（[instance-model](instance-model.md) D68/D73 的可变面继续关闭）。这把零拷贝大载荷限制在无副作用数据面。
- **reader / writer 语义**：`read` 每块 1 次拷贝、不整段物化；EOF = `Ok(none)`；中途错误 = 宿主主族（fs / http）Err；`close` 幂等；Drop = best-effort close（writer 未 close = 弃 tmp，W3 fs 原子写纪律）；卸载五段 drain 强 close（W4.5 D11.2）。
- **拷贝账**：blob 句柄 0；`from-bytes` 1 次写；fanout 0（每接收方句柄）；每个接收方 `read` 按其读取量 1 次；reader/writer 每块 1 次。此表只计**边界拷贝**；guest 内 TLV 编码 / 解码是 CPU pass，另计（W6.1 的 encode-once / typed decode 压这部分）。
- **淘汰**：WASI 0.3 `stream<T>` 直接采用——中途错误需包 `stream<result<...>>` 或配独立 future；`stream<list<u8>>` 可避免逐字节项，但引入第二套流 API 与双轨错误面；v1 的 reader / writer 已覆盖宿主↔guest 大流，跨实例 fanout 由 blob 解决。`stream<T>` 采用面登记扩展位。

### W6.6 D160 `payload` 形状：`value` / `blob` 变体

**D160 定案：事件与服务的新增签名统一用 `variant payload { value(list<u8>), blob(own<blob>) }`；config / mount 声明 / business error data 保持 TLV。**

```wit
interface payloads {
  use blobs.{blob};
  variant payload {
    value(list<u8>),   // TLV 编码的 Value
    blob(own<blob>),   // 大负载 / fanout
  }
}

interface events {
  use errors.{error};
  use payloads.{payload};
  emit: async func(name: string, payload: payload) -> result<_, error>;
  serial: async func(name: string, payload: payload) -> result<option<payload>, error>;
  parallel: async func(name: string, payload: payload) -> result<list<result<option<payload>, error>>, error>;
  waterfall: async func(name: string, payload: payload) -> result<option<payload>, error>;
  // listen / unlisten 不变
}

interface services {
  use errors.{error};
  use payloads.{payload};
  call: async func(service: string, method: string, args: payload, timeout-ms: option<u32>) -> result<payload, error>;
  // provide / deps 不变
}

// on-call / on-event 已定形（on-event = W8 D177）：
// on-call: async func(service: string, method: string, args: payload) -> result<payload, error>;
// on-event: async func(listener-id: u64, name: string, payload: payload) -> result<option<payload>, error>;
```

- **fanout 语义**：宿主对每个接收方 `duplicate` 一个 blob 句柄（引用计数，0 内容拷贝）；发送方的 own 句柄在入队后释放。接收方不读 = 0 内容拷贝；读多少拷多少。
- **服务调用**：args / 返回 = 单个 payload；`blob` 所有权随调用转移，发送方需要保留时自行 `duplicate`；提供方返回 `blob` 时调用方获得 own 句柄。
- **不变面**：`setup` / `on-config` 的 config = `list<u8>` TLV；`instance.mount` 声明 = `list<u8>` TLV；W5 `error.data` = `option<list<u8>>` TLV（business 错误不放 blob）；`on-timer` 无 payload。
- **作者面**：SDK 映射 `Payload::Value(Value)` / `Payload::Blob(Blob)`；`Value::Bytes` 是 value 车道内的小二进制，blob 是独立车道。
- **兼容性**：`0.1.0` 未发布，本批改签名 = 首个发布快照的编制；冻结后的增补 = 新 package version（W10 D188；冻结触发 = D190）。

### W6.7 D161 resource 生命周期与跨实例边界

**D161 定案：resource 分三类——不可变 blob（可克隆、可跨实例只读）/ 一次性流（per-instance）/ 可变生命周期对象（per-instance）；析构同步 best-effort，显式方法给确定性完成。**

| 资源             | 可变性   | 跨实例       | Drop 语义                               | 确定性完成                     |
| ---------------- | -------- | ------------ | --------------------------------------- | ------------------------------ |
| `blob`           | 不可变   | 允许（只读） | 引用计数 −1；宿主释放最后一份           | 无（内容不可变）               |
| `byte-reader`    | 流状态   | 不允许       | best-effort close                       | `close()` / EOF                |
| `byte-writer`    | 流状态   | 不允许       | best-effort 异步 close；未 close 弃 tmp | `close().await`                |
| `process-handle` | 有副作用 | 不允许       | 不杀进程，只关账本                      | `wait()` / `kill()`            |
| `mount-handle`   | 有副作用 | 不允许       | best-effort 异步 unmount                | `unmount().await` / `detach()` |

- **跨 Store 机制**：wasmtime 的 resource handle 绑定 Store 的资源表；跨实例传递 = 宿主持有底层不可变数据（`Arc`），在目标实例表里重物化一个新句柄（S3 补遗 10）。不存在「同一个 handle 跨 Store 直接使用」。
- **析构**：canonical ABI 的资源析构是同步函数，不能 await；因此 Drop 只做非阻塞记账 / 入队。需要确定性完成时用显式异步方法。
- **卸载**：W4.5 五段 drain 强 close 所有 per-instance 资源；blob 只释放本实例持有的句柄，其他实例的句柄继续有效；宿主按租户内存预算回收。
- **句柄校验**：作者面不出现裸 id；外来 / 已释放 / 错误类型的 handle = trap（宿主资源表校验）；自己的已关闭 reader / writer = 幂等 / 类型化错误（W6.8）。
- **预算**：每实例句柄数、blob 总字节 / 单 blob 大小、reader/writer 缓冲 = 实现期配置（T10.10 / T10.11 / T10.14）。策略预算超限 = `core-contract-violation` Err（`from-bytes` / `collect` 等 guest 可触发路径）或宿主拒绝创建（host 侧创建路径，host tracing 记原因）；宿主资源表 / 分配耗尽 = trap（基础设施边界，按实现期配置登记）。
- **扩展位**：listener / timer resource 化（W3 维持 `u64` + RAII）；`byte-reader` 跨实例（当前不允许）；blob 写入型资源（当前只读）；`stream<T>` 采用面。

### W6.8 D162 失败面、预算与 EOF

**D162 定案：数据相关失败 = 类型化 Err；静态作者错 = trap；TLV / blob 预算与版本失败按边界分格；EOF 显式；不新增错误 kind。**

| 情形                                                                                      | 载体                                                                                                       |
| ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 服务参数 / 返回、emit payload、mount 声明的 TLV 解码失败 / 形状违约 / 预算超限 / 版本不识 | `core-contract-violation` Err + journal `contract_violation`（服务 / emit / mount 路径；message 指明边界） |
| config 的 TLV 畸形 / 版本不识                                                             | 启用期响亮失败（guest 未跑，W1.5 / W2 D133 原位）；reload = config 拒绝（D42）                             |
| business data 解码失败                                                                    | 接收方 SDK 本地作者错；宿主不解释、不校验                                                                  |
| `blob.from-bytes` 超预算                                                                  | `core-contract-violation` Err（message 带预算数）                                                          |
| `blob.read` 越界 / `max` 超预算                                                           | 不报错：越界 = 空列表；`max` 由宿主按块上限截断（`max` 是提示）                                            |
| `byte-reader.read` 中途失败                                                               | 属主族 Err（fs / http）；EOF = `Ok(none)`，不是错误                                                        |
| `byte-reader.read(max = 0)`                                                               | 空列表（非 EOF）；短块也非 EOF——只有 `none` 是 EOF                                                         |
| `byte-reader.collect` 超 max / 超租户 blob 预算 / close 后调用                            | `core-contract-violation` Err（message 指路 `max` / 预算 / close 状态）                                    |
| `byte-writer.write` 在 close 后 / `byte-reader.read` 在 close 后                          | `core-contract-violation` Err / `Ok(none)`（read 幂等）                                                    |
| 未声明能力                                                                                | trap（W5 D139 原位）                                                                                       |
| 外来 / 已释放 / 错误类型的 resource handle                                                | trap（canonical ABI 资源表校验，W6.7）                                                                     |
| 宿主资源表 / 分配耗尽                                                                     | trap（基础设施边界；不是业务错误）                                                                         |

- **严格制**：`Int` / `Float` 由 TLV tag 区分，不做 coercion；契约形状校验按 tag 精确匹配；混合 `Array` / `Map`、非字符串 Map 键 = 边界错误。
- **预算形状**：TLV 条目数 / 深度 / 总字节；blob 单大小 / 总量；reader / writer 块上限；每实例句柄数。数值 = 实现期配置（T10.10 / T10.11 / T10.13 / T10.14）；W6 不冻结数值。
- **EOF**：流式 read 统一 `result<option<list<u8>>, error>`；`Ok(none)` = EOF；`Err` = 中途错误；无 `nil, nil` 双槽。
- **journal**：不新增 kind；blob / stream 内容默认不录；`Bytes` 落盘 = base64 包裹（W6.4）；审计层扩展位若开启，blob 只记元数据 / 引用而不记内容（需新 kind，归扩展位）。
- **per-read timeout**：维持扩展位（W4.7 登记），触发器 = 真实长读取消需求。

### W6.9 f2 兑现：ADR-0015/0022 论据换血

**定案：ADR-0015 / ADR-0022 的决策本体（自定义 `Value`、`Int(i64)`）不变；Luau 对齐论据由本章换血为 WIT canonical + TLV 边界论据。**

- 两份 ADR 的 Status 行追加批次注记（2026-10-19），正文冻结；`Bytes` 扩展位格的局部取代由 ADR-0042 单独记录。
- 换血内容：`Value` 的约束不再是「与 Lua table 一一对应」，而是「与 YAML 1.2 core schema + WIT canonical 类型 + TLV tag 三落点对齐」；`Int` 的约束不再是「Luau 原生 integer」，而是「WIT `s64` + TLV Int tag」；二进制不再依赖 Luau buffer，`Bytes` / blob 见 W6.4/W6.5。
- 不再为「论据换血」单独开 ADR：决策未变，章内记录 + Status 注记足够。

### W6.10 A6 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 [ADR-0026](../decisions/0026-luau-abi.md)（已 superseded）。

| 机制                                                    | 死因                                                                       |
| ------------------------------------------------------- | -------------------------------------------------------------------------- |
| Luau VM 值 / mlua `Value` / 注册表句柄三层辨析          | 组件实例独立线性内存；跨界 = canonical ABI + TLV 字节，无 VM 句柄          |
| table 深快照 / 元表拒绝 / Array-Map 严格二分            | `Value` 纯数据树 + TLV tag；无 Lua table / 元表                            |
| `push_int64_exact` / tag=integer 严格匹配 / `123i` 写法 | WIT `s64` + TLV Int tag；严格制保留，载体换                                |
| `buffer` / `readbytes` / `writebytes` / UTF-8 校验裂缝  | `list<u8>` / `Bytes` 原生；`read_text` UTF-8 校验归 SDK（fs-invalid-utf8） |
| frozen table 烘焙句柄 / userdata 淘汰                   | WIT resource（process / mount / blob / reader / writer）                   |
| A6.6 盒线编码 / 盒流式扩展位                            | TLV 主线化（E1①）；大负载 = blob / reader（W6.5）                          |
| `Value::Bytes` 扩展位                                   | 提正 = ADR-0042（W6.4）                                                    |
| NaN-box 参照 / 位分配草案 / 32bit 偏移假设              | T10.13 实现期定稿；语义合同在 W6.2                                         |
| 流式句柄三态 `chunk, err` 两槽                          | `result<option<list<u8>>, error>` 单返回值（W6.8）                         |
| 盒 trap / 业务错误封套                                  | W5 `error` 类型 + W6.8 失败面                                              |
| journal 记录格式草案 §A6.7                              | [journal](journal.md) D75–D92（已冻结）                                    |

**语义平移保留**：`Value` 纯数据树 + `Send + Sync`；严格 Int / Float 制；调用点快照（wasm 版 = 源 guest 快照 1 次）；`Arc` 共享 + 1 + N 成本结构；拉式流式读 + EOF 显式；大负载 resource；错误复用属主族；Map 确定性迭代序（`BTreeMap`）。

### W6.11 联动登记汇总（章末）

| 去向              | 内容                                                                                                                                                                                                                                                                                                               |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W1（T13-h）       | 生命周期签名不变（config = TLV）；`on-call` / 未来 `on-event` 的参数 / 返回改 `payload`（W6.6）；world import 集 += `blobs` / `streams` / `payloads`                                                                                                                                                               |
| W2（T13-i）       | manifest 不变；`capabilities` 不变；`events` 声明面不变（W8 已消费 payload 变体）                                                                                                                                                                                                                                  |
| W3（T13-j）       | import 集 = 9 个功能接口（`events` / `services` / `timer` / `http` / `fs` / `process` / `instance` / `blobs` / `streams`）+ 类型面 `errors` / `fs-types` / `payloads`；`fs` += `open-read` / `open-write`；`http` += `download`；`Ctx` += `Payload` / `Blob` / `ByteReader` / `ByteWriter`；W3.15 的 W6 行本批兑现 |
| W4（T13-c）       | 拷贝下限利用 W4.1 自由交错（源快照必须先于挂起）；resource 卸载 drain 接 W4.5；不改变调度                                                                                                                                                                                                                          |
| W5（T13-k）       | 错误封套不变；`error.data` 保持 TLV；流式 EOF / 错误复用属主族；不新增 kind；`core-contract-violation` 承接 TLV / blob 预算失败                                                                                                                                                                                    |
| W7（T13-m）       | **已兑现（2026-10-19）= W7 D163–D172**：契约形状 = typed decode 的 schema 来源（D167/D170）；`ValueRef<'a>` / `T::decode` 消费同一线格式；blob payload 不做形状校验（D169）；契约包 = `contract.yml` + 文档（D164）                                                                                                |
| W8（T13-n）       | **已兑现（2026-10-19）= W8 D173–D179**：`on-event` / emit 的 payload = `payload` 变体；listener 返回 `option<payload>`；blob 车道不参与形状校验；链值所有权按 W6 D161；事件名 / `internal/` / 声明语义不变                                                                                                         |
| W9（T13-o）       | **已兑现（2026-10-19）= W9 D180–D185**：进口对账词汇表 += `blobs` / `streams` / `payloads`；blob 无能力门；reader/writer 走 fs/http 门；跨实例只读 blob = 结构允许，可变资源仍拒                                                                                                                                   |
| W10（T13-p）      | **已兑现（2026-10-19）= W10 D186–D193**：本批 = 首个发布快照编制（`0.1.0` 未发布，D190）；新增接口 / payload 变体在同一 package version 内落定（D188）；TLV 版本面独立（D191）                                                                                                                                     |
| T10.10            | 组件运行时：每实例 `ResourceTable`；blob 注册表（`Arc` + 引用计数）；跨 Store 句柄重物化（S3 补遗 10）；blob / reader / writer 析构；预算                                                                                                                                                                          |
| T10.11            | fs `open-read` / `open-write`（global host-mediated；private 走 WASI 0.3）；http `download`；blob 创建 / collect；预算 / 卸载强 close                                                                                                                                                                              |
| T10.12            | 安装期 world 导出校验补 `payload`；preopen 授予不变；mount 声明仍 TLV                                                                                                                                                                                                                                              |
| T10.13            | TLV codec：位分配 / 版本字段 / varint / canonical 编码 / `ValueRef` / `encoded_len` + `encode_into`；blob / stream 不进 TLV；round-trip 基线                                                                                                                                                                       |
| T10.14            | SDK：`Payload` / `Blob` / `ByteReader` / `ByteWriter` / `Value::Bytes`；typed decode；raw escape hatch；两车道文档                                                                                                                                                                                                 |
| T10.15            | 脚手架 / `metis check`：world import 集含 `blobs` / `streams` / `payloads`；不生成 crypto                                                                                                                                                                                                                          |
| journal           | 不新增 kind；blob / stream 内容不录；`Bytes` JSONL = base64 包裹（T10.5）                                                                                                                                                                                                                                          |
| seam-capabilities | §6.1 W6 行兑现：TLV 线编码 / 大负载 resource / 拷贝预算；能力门不变                                                                                                                                                                                                                                                |
| instance-model    | mount 声明保持 TLV；blob payload 不改变父子实例所有权；可变资源仍 per-instance                                                                                                                                                                                                                                     |
| config-format     | config 仍 TLV `Value`；YAML 不产生 `Bytes`                                                                                                                                                                                                                                                                         |
| plugin-sdk        | 作者面 `Payload` / `Blob` / `ByteReader` / `ByteWriter`；大负载 = blob / reader；`Bytes` = 数据模型内二进制                                                                                                                                                                                                        |
| ADR-0042          | `Value::Bytes` 提正；局部取代 ADR-0015 / 0022 的扩展位格                                                                                                                                                                                                                                                           |
| ADR-0043          | 只读 blob 跨实例车道与双车道 payload；安全边界 = 可变资源不跨实例                                                                                                                                                                                                                                                  |
| wasm-research     | 补遗 10：S3 blob 跨实例 spike **PASS**；补遗 4/5 技术事实原位                                                                                                                                                                                                                                                      |
| 实现期配置        | TLV 条目 / 深度 / 字节；blob 单大小 / 总量；reader / writer 块上限；每实例句柄数；per-read timeout（扩展位）                                                                                                                                                                                                       |
| 扩展位            | WASI `stream<T>` 采用面；blob 写 / 可变跨实例；listener / timer resource 化；per-read timeout；`http` 流式上传；schema-aware compact 编码（不立）                                                                                                                                                                  |
| Creator 文档      | 两车道选择（小 = Value / 大 = blob）；拷贝预算与批量化 / 合并插件建议；blob 只读 / 不可变；`Bytes` 预算纪律                                                                                                                                                                                                        |

---

## W7 契约与类型来源

### W7.0 边界与前提

本章定：类型来源三分与 `Value` 的定位；契约包物理形态、身份、三来源与版本；瘦身形状 DSL 的语法与词汇；契约文件节结构；规范化比较与引用解析；运行期形状校验；作者类型投影（`types!()`）；契约生命周期与级联；f2 兑现（ADR-0019 C3 supersede）。不归本章：事件名语义 / `internal/` 规则 / `on-event` 导出最终形状 = W8；provides / inject / events 对账与能力门 = W9；`abi` 比较与 WIT package version 纪律 = W10；manifest 字段形状 = W2；平台 import 接口集 = W3；TLV 字节布局 = T10.13；契约解析器 / 规范化比较器 / 运行期 walk 实现 = T10.3；SDK 宏实现 = T10.14；部署索引与级联执行 = T10.9 / T10.7 / T10.12。

前提（已定不议）：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；接口数据三分法（E1① / E2① / E5①，2026-10-16 拍板）；开放数据面线格式 = TLV `Value`（W6 D156）；`Value` 变体全集 = Null / Bool / Float / Int / String / Bytes / Array / Map（W6 D157 / D158）；business data = TLV Value（W5 D140）；服务调用 = `call(service, method, payload)` + `on-call`（W3 D147）；事件 = 动态名 + `payload`（W3 D146）；配置 = TLV Value（W6 D157）；作者不见 WASI/WIT（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §4）。

| 节    | 议题                           | 状态               |
| ----- | ------------------------------ | ------------------ |
| W7.1  | 类型来源三分与 `Value` 的定位  | ✅ D163 2026-10-19 |
| W7.2  | 契约包形态与身份               | ✅ D164 2026-10-19 |
| W7.3  | 三来源、版本与 `@N`            | ✅ D165 2026-10-19 |
| W7.4  | 瘦身形状 DSL：语法与词汇       | ✅ D166 2026-10-19 |
| W7.5  | 契约文件节结构                 | ✅ D167 2026-10-19 |
| W7.6  | 规范化比较、同名与引用解析     | ✅ D168 2026-10-19 |
| W7.7  | 运行期形状校验                 | ✅ D169 2026-10-19 |
| W7.8  | 作者类型投影（`types!()`）     | ✅ D170 2026-10-19 |
| W7.9  | 契约生命周期与级联             | ✅ D171 2026-10-19 |
| W7.10 | f2 兑现：ADR-0019 C3 supersede | ✅ D172 2026-10-19 |
| W7.11 | A7 机制消亡清单（考古索引）    | ✅ 2026-10-19      |
| W7.12 | 联动登记汇总（章末）           | ✅ 2026-10-19      |

### W7.1 D163 类型来源三分与 `Value` 的定位

**D163 定案：三分法不变；`Value` 从「作者面通用数据模型」收窄为「动态数据面 + 宿主交换格式」；作者面契约类型 = `types!()` 生成物，`any` 是唯一回落。**

- 三分法（W6 D157 已定，本章确认其类型来源面）：
  - **平台封闭面** = WIT 富类型原生（fs / http / process / timer / instance / services 控制面；W3 D142–D154）；
  - **开放数据面** = TLV `Value` + 瘦身形状 DSL + SDK 生成类型（config / 服务 args / returns / 事件 payload / business data / mount 声明）；
  - **大负载** = blob / byte-reader / byte-writer（W6 D159）；不做形状描述，内容 opaque。
- `Value` 的承重面 = 宿主在不知道 schema 时仍需能路由 / 校验 / 记录 / 脱敏：config 通用树（overlay / diff / 结构相等 / 错误路径）、动态服务路由与 contractless 服务、开放事件与 contractless 事件、business data、journal payload、`any` 槽。WIT 是静态、非递归、无 `map` / `any`，不能替代这一面。
- 作者面：契约已知的类型由 `types!()` 生成（D170）；作者只在 `any` 槽与 contractless 动态路径接触 `Value`。平台 import 的 WIT 绑定由 wit-bindgen 生成并由 SDK 隐藏（[plugin-sdk](plugin-sdk.md) S2 / S6）。
- **不立第四种形态**：任何新类型面必须归入三格之一；新增 = 新 ABI package version（W10 D188）。
- **递归写实**：契约类型非递归——类型引用成环 = 启用期响亮（A7 D21.4 语义原位）；运行期 `Value` 可任意嵌套，但契约只描述有限形状；任意深度动态树 = `any`（不做形状校验）。

### W7.2 D164 契约包形态与身份

**D164 定案：契约包 = `lib/<contract-name>/contract.yml`（+ 可选 `README.md`）；纯数据、无 WIT 面；身份 = 目录名。**

- 物理形态：部署根 `lib/<contract-name>/` 与插件包 `plugins/<plugin>/lib/<contract-name>/` 同形；一个插件可带多份随包契约。契约包不含 `.wit` 文件——WIT 只承载平台固定 world（W3 D142–D154）。
- `contract.yml` = 唯一机器数据，YAML 受限子集（[ADR-0007](../decisions/0007-yaml-config-subset.md)）；`README.md` = 人读文档（business data 语义、重试语义等），可选、不设机器闸。
- 身份 = 目录名；不设 `name:` 字段（与 manifest W2 D126 同纪律；出现 = 未知字段响亮）。契约名可被引用；插件名不是契约名。
- **编译期期望的载体 = 随包契约**：使用 `types!()` 的插件必须把所用契约放入自己包的 `lib/`（与部署根 `lib/` 同形）；该副本既是作者类型来源，也是运行期「二进制按哪个形状编译」的声明。宿主合并三来源时同名不同形状（两个具体形状不等；`any` 相容规则见 D168）= 启用期响亮并指名双方；破坏性契约变更因此先让陈旧消费者启用失败，而不是静默跑错形状。重启用不能重建二进制——消费者必须重新编译（D171）。动态消费者（不使用 `types!()`）不必随包契约。
- 平台内嵌契约 = 编入核心二进制的同形数据；源码位置归实现期（T10.3），不下发到部署根。作者开发期使用平台契约时，从同一源码复制到自己的 `lib/`；导出工具 = 扩展位（W7.12）。
- 契约包不被 import 为代码；跨插件代码共享 = 编译期依赖（[ADR-0038](../decisions/0038-module-system-require-supersede.md) 原位）。
- **修正前向引用**：[service-registry](service-registry.md) §7 / [config-format](config-format.md) D29 的「契约包 = WIT 包」措辞按本节点修正；「WIT 包加载」保留 = 平台固定 world 的 WIT 加载（T10.3）。
- 校验管线：`metis check` / `metis build` / loader 安装期共用同一契约校验器（W2 D133 三面暴露纪律；config-format C3.1 同 parser 同校验器）；语义对账在启用期（D168）。

### W7.3 D165 三来源、版本与 `@N`

**D165 定案：三来源逻辑合并；版本 = 单整数、破坏性变更才 bump；裸名 = 内嵌 / 随包，`name@N` = 部署级生态契约。**

| 来源           | 位置          | `version:`         | 引用写法                     |
| -------------- | ------------- | ------------------ | ---------------------------- |
| 平台内嵌       | 核心二进制    | 禁写（≡ 核心版本） | 裸名                         |
| 部署级生态契约 | 部署根 `lib/` | 必填               | `name@N`，`N` 与版本精确相符 |
| 插件随包契约   | 插件包 `lib/` | 禁写（部署内对账） | 裸名                         |

- 版本 = 单个整数；加方法 / 加类型 = 兼容不 bump；已有名字的形状变更 / 删除条目（类型 / 方法 / 事件名）= 破坏 bump（A7 D18.2 原位；D171）。
- 版本整数 = 声明与 `@N` 门；兼容性判定 = 规范化形状比较（D168）——版本未 bump 但形状变了 = 同名不等响亮；版本 bump 但形状未变 = 合法但无意义的声明（不报错）。
- 引用来源类别与写法不符 = 启用期响亮：生态契约漏 `@N`、内嵌 / 随包契约带 `@N`、`@N` 与 `version:` 不符三格同判。
- 同名规则：规范化后结构相等（`any` 相容规则见 D168）则共存（契约竞标）；两个具体形状不等 = 启用期响亮并指名对手方与差异位置；引用永远按契约名，不按插件名（Seam 三角，[ADR-0018](../decisions/0018-service-registry.md) S6）。
- 三来源逻辑合并 = 不搬文件、不进模块系统；安装期登记部署索引，判定在启用期（D168；W9 承接 provides / inject / events 面）。
- 契约版本与 WIT package version 独立：前者管契约数据，后者管 ABI 接口集（W10 D191）；两者不互相推导。

### W7.4 D166 瘦身形状 DSL：语法与词汇

**D166 定案：YAML 管结构 + 引号内表达式串管形状；词汇 = `Value` 变体全集；删除 ctx 面词族；`bytes` 新增；契约形状非递归。**

- 载体：与 manifest / config 同 YAML 受限子集（ADR-0007）；表达式串必须加引号；串内语法由同一 parser 校验，报错定位 = 串内偏移。
- 产生式（冻结）：基元词 / 字符串字面量 / `array<T>` / `map<T>` / record `{ field: T, ... }` / union `A | B` / optional `T?`（= `T | nil`）/ 类型引用 `Contract.Type`（本契约内可裸写 `Type`）。签名产生式 `(<shape>) -> <shape>` 仅用于 `methods:`，参数恒单个匿名形状（服务面单 args）；A7 的命名参数列表形态死亡。

| 词              | 对应                                                            |
| --------------- | --------------------------------------------------------------- |
| `nil`           | Null                                                            |
| `boolean`       | Bool                                                            |
| `string`        | String（UTF-8）                                                 |
| `integer`       | Int(i64)                                                        |
| `number`        | Float(f64)                                                      |
| `bytes`         | Bytes（**新增**；[ADR-0042](../decisions/0042-value-bytes.md)） |
| `array<T>`      | Array                                                           |
| `map<T>`        | Map（键恒 string，`BTreeMap`）                                  |
| record `{...}`  | Map 的封闭子形                                                  |
| `A \| B`        | 动态 union                                                      |
| `T?`            | `T \| nil`                                                      |
| `any`           | 任意 Value                                                      |
| `Contract.Type` | 引用展开                                                        |

- 删除：`buffer` / `function`（ctx 面已由 WIT 富类型承接；`Value` 不含这二者）；A7 的 ctx 方法签名词汇。
- `nil` 恢复为合法值；A7「服务返回不得为 nil / 不得含 nil」的禁令死亡（两槽习语随载体消亡，W5.7 已列死）。
- 字符串字面量 = 精确值类型（`"a" | "b"`）；运行期校验 = 字符串全等；`types!()` 投影为枚举（成员名按字面量派生，冲突 = 编译错误；定形归 T10.14）。
- record 字段的归一化类型含 `nil`（`T?` 或 `T | nil` 同义）= optional：键缺席或键存在但值为 Null 均合法，生成类型同归 `None`；归一化不含 `nil` 且不是 `any` = 必须存在且非 Null。
- **`any` 吸收**：任何含 `any` 的 union 归一化为 `any`（`nil` 不被吸收——`any | nil` 保持 optional，可缺席或值为 Null）；`any` 字段键必须存在，值可为任意 Value（含 Null）。
- `bytes` 只允许出现在 `types:` / `methods:` / `errors:` / `events:`；config `type:` 直接出现或经类型引用展开后出现 = 响亮；config 可达的 `types:` 不得含 `bytes`（YAML 无法产生 Bytes，W6 D157）。
- 严格制：`integer` ≠ `number`，无隐式 coercion；`array` / `map` 严格二分；record 字段封闭；`map` 键只允许字符串。
- 类型引用展开 + 环检测；成环 / 未知 = 启用期响亮；引用不带版本钉（版本钉 = `provides.contract` 的 `@N`，D165）。
- 词汇 / 产生式演进 = 新 ABI package version（W10 D188）；parser 实现期细则（空白、转义、字面量）归 T10.3。
- `any` 兼容一切（递归子位置）；`any` 不参与形状校验；同名多声明合并时具体形状胜出（A8 D22.2 原位，D168）。

### W7.5 D167 契约文件节结构

**D167 定案：`contract.yml` 节 = `version` / `types` / `methods` / `errors` / `events`；方法集住契约；business data 形状不进运行期校验。**

```yaml
# lib/memory/contract.yml
version: 2 # 仅部署级生态契约；内嵌 / 随包禁写
types:
    Job: "{ id: string, cron: string }"
methods:
    recall: "({ key: string }) -> any"
errors:
    recall: "{ reason: string, retry_after_ms: number? }"
events:
    schedule/due: "{ job: Job, at: integer }"
```

- 节全可选；未知节 / 字段 = 响亮；空契约合法。
- `version:` 规则 = D165。
- `types:` = 命名形状，供四节复用；类型名 = 标识符。
- `methods:` = 方法名 → `(<args>) -> <returns>`；方法集 = 契约数据（manifest `provides.methods` 已消亡，W2 D131）；args / returns = Value 形状。
- `errors:` = 方法名 → business data 形状；只服务类型生成 / 文档 / 静态对账，不进运行期校验（W5 D140 / A7 D19.1 原位）；business data 承载 = TLV Value。
- `events:` = 事件名 → payload 形状；事件名语义、`internal/` 规则、`on-event` 最终形状归 W8；本章只定形状出处。
- `provides.contract` 缺省 = **无契约（contractless）**，不默认解析为服务名；显式 `contract:` 引用查无 = 启用期响亮（D168）。这使「无契约私有服务」与「显式契约引用」两分清晰（W2 D131 2026-10-19 修订）。
- 无契约也可提供私有服务 / 发事件；代价 = 失去形状对账与运行期形状校验（W2 D131 原位）。
- **E5 澄清**：死的是 ABI 级「每方法一 WIT 导出」，不是契约里的 Value 形状；`on-call(service, method, payload)` 是唯一 ABI 面（W3 D147）。
- 校验管线：YAML 受限子集 → 节 / 字段封闭 → DSL 语法 → 类型引用 / 成环 → 规范化；与 manifest / config 同一实现（W2 D133）。

### W7.6 D168 规范化比较、同名与引用解析

**D168 定案：规范化后结构相等才相容；`any` 递归相容、具体胜出；缺失两分；判定在启用期。**

- 规范化四条（A7 D18.3 原位：record 字段序无关 / union 成员序无关 / `T?` → `T | nil` / 空白无关）+ 类型引用展开（A8 D22.2 原位）+ **`any` 吸收**（含 `any` 的 union = `any`，`nil` 除外；`any | nil` 保持 optional）。
- `any`：与一切相容（递归子位置）；同名多声明合并时具体形状胜出，`any` 永不覆盖具体定义。
- 同名规则：三来源撞名规范化相等 = 共存（竞标）；两个具体形状不等 = 启用期响亮；`any` 与具体形状相容（D165）。
- 缺失两分（A8 D22.3 原位）：部署索引查无 = 启用期响亮，指路「用现成词汇 / 自己随包定义」；索引中有但来源插件未启用 = Pending 等待（无超时，来源启用后反向索引唤醒）。
- 引用两分：显式 `contract:` 查无 = 响亮；`provides` 条目缺省 `contract:` = contractless，不做引用解析（D167）。
- 判定时机：安装期 = 解析 + 登记部署索引；启用期 = 撞名 / 引用解析 / 形状对账 / 出处检查（W9 承接 provides / inject / events 面）；`metis check` = 同一实现的静态预演。
- 解析冻结：各插件在自己启用事务内解析并冻结；两次启用之间恒定（D171 级联送达变更）。
- 规范化比较器 / 引用展开 / 记忆化预算 = 实现期（T10.3）。

### W7.7 D169 运行期形状校验

**D169 定案：契约已知时校验服务 args / returns 与事件 payload；违约 = `core-contract-violation` Err + journal；blob 与 contractless 面不校验。**

- 服务调用：宿主在两跳中点（Value 树转换后、投递前）按契约 walk args；提供方返回后按契约 walk 返回；违约 = 调用方 `core-contract-violation` Err + journal `contract_violation`（W5 D139 / D140；[journal](journal.md) D78）。
- 事件 emit：按事件形状总表 walk payload；违约 = `core-contract-violation` Err + journal（**已兑现（2026-10-19）= W8 D176**：四派发入队前同判；`internal/*` 冒用 = trap；空名 / 超长名 = `core-contract-violation` Err）。
- business data：不做运行期校验（W5 D140）；`errors:` 只服务类型生成 / 文档 / 静态对账。
- `payload.blob`：opaque，不参与形状校验；契约形状只描述 `value` 车道。车道选择不受契约约束——方法 / 事件可以收到 `blob`，此时形状校验跳过；value-typed 生成解码器遇 blob = 解码失败（作者面错误）。per-method / per-event blob 声明 = 扩展位。大负载语义归 W6 D159。
- walk 语义：record 字段归一化类型含 `nil`（`T?` / `T | nil` / `any | nil`）时键缺席或值为 Null 同义；归一化为 `any`（含含 `any` 的 union，`nil` 已排除）时键必须存在、值任意（含 Null）；其余类型键必须存在且非 Null；字符串字面量按全等校验。
- 无契约：不做形状校验，仅 TLV 合式性；contractless 服务 / 事件照常工作。
- 方法存在性：契约方法集 = 宿主 `service-method-missing` 短路依据；不立实现侧注册清单（`on-call` catch-all 决定运行期行为；W3 D147）。
- 配置：`type:` 在启用期按同一 DSL 校验（config-format D36）；`bytes` 直接出现或经引用展开后出现 = 响亮（D166）。
- 成本：一次字节 / 树走查（typed walk 直接走编码字节），不新增拷贝；v1 不做投机优化。
- 事件形状退化：唯一形状定义随卸载离场 = emit 回退无校验 + journal `event_shape_degraded`（D171；journal D78）。

### W7.8 D170 作者类型投影（`types!()`）

**D170 定案：YAML 是唯一来源；`types!()` 在编译期生成作者 Rust 类型；生成覆盖契约四节 + manifest config schema；反向生成不立。**

- 形态：`metis_plugin::types!()`（[plugin-sdk](plugin-sdk.md) S7 D104）编译期读本地契约包 + 本 crate manifest；生成 struct / enum / 类型别名；不 check-in 生成物（SSOT 单写）。
- 覆盖：
  - `types:` → 命名类型；
  - `methods:` → 每方法 Args / Returns 类型（服务调用助手用）；
  - `errors:` → business data 类型；
  - `events:` → payload 类型；
  - manifest `config:` schema → config struct（`required / default / secret` 不影响字段的 Value 形状；`required: false` 且无默认 = `Option<T>`，其余 = `T`；`secret` 不改变 Rust 类型；细则归 T10.14）。
- 映射：record → struct / union → enum / `array<T>` → `Vec<T>` / `map<T>` → `BTreeMap<String,T>` / `integer` → `i64` / `number` → `f64` / `bytes` → `metis_plugin::Bytes` / `any` → `metis_plugin::Value` / `nil` 单独 → `()`；含 `any` 的 union 归一化为 `any`（`nil` 除外）→ `Value`；归一化含 `nil` 的 union：非 nil 成员单成员 → `Option<T>`，多成员 → `Option<enum>`（`any | nil` → `Option<Value>`）；record 字段同规则。字符串字面量（含单成员）→ 枚举（成员名按字面量派生）。
- typed decode：直接从 TLV 字节走 schema（W6 D155 typed decode），不经中间 Value 树；`any` 槽回落 Value。
- 来源解析：**只读** `<crate>/lib/<contract-name>/contract.yml`（包内容区，随构建产物同包）；不立外部路径 / registry / 网络。使用 `types!()` 的契约必须随包——该副本同时是运行期编译期期望声明（D164 / D171）。消费他插件的随包契约 = 从提供方源码 / 包复制到自己的 `lib/`（开发期动作），或由部署者提升为部署 `lib/` 生态契约；v1 无自动分发（任务 9）。构建 / `metis check` 验证 `types!()` 引用的契约都在包内 `lib/`；缺失 = 构建错误。
- 服务名 → 契约的绑定由作者选择的 `types!()` 契约决定；宿主按提供方 `provides.contract` 校验。用错契约 = 运行期形状违约 / 解码失败，不是静默成功。
- 契约作者 / 消费者同一机制：提供方与消费方都用同一契约包生成类型；契约独立于提供方（Seam 三角）。
- **反向（Rust 类型 → schema）不立**：ADR-0006 要求宿主在跑插件代码前读到 schema；契约必须独立于提供方源码。
- **WIT 投影**：契约 → WIT 生成物 = 扩展位（触发器 = 外部工具链互操作 / 标准 IDL 需求），不启用；平台 import 的 WIT 绑定仍由 wit-bindgen 生成并由 SDK 隐藏（S2 / S6）。
- 死亡：`.d.luau` / `export type` 生成、schema → WIT 生成、文档骨架生成（工具扩展位）、`init.luau`。

### W7.9 D171 契约生命周期与级联

**D171 定案：引用冻结 + 新增吸收 + 破坏级联；部署 `lib/` 变更走 boot / reload；随包契约随插件更新；卸载降级写实。**

- 解析冻结：各插件启用事务内解析并冻结；两次启用之间恒定。
- 变更分级：新增条目（类型 / 方法 / 事件名）= 兼容，命名空间直接吸收；删除条目 / 已有名字形状变更（规范化后不等）= 破坏 → 契约版本 bump。
- **两个兼容面**：
  - **运行期契约面**：动态消费者（不使用 `types!()`）走反向索引级联重启用，按新形状继续；动态提供方同样按新契约运行。
  - **编译期二进制面**：使用 `types!()` 的插件随包携带编译时契约副本（D164）；破坏性变更使旧副本与新契约同名不同形状（两个具体形状不等；`any` 相容见 D168）→ 启用期响亮，消费者 / 提供方必须重新编译；重启用不能修复编译期类型。审批 / 检查面显示「N 个动态消费者将重启 / M 个编译期消费者需重建」。
- 触发：部署 `lib/` 增删改 = boot / 显式 reload（config-format D43；检测半 T10.9、执行半 T10.7 / T10.12）；插件随包契约变更 = 插件更新审批事务（同批）。
- 卸载：提供方卸载只移走它自己的契约副本；使用 `types!()` 的消费者持有自己的随包副本，名字仍被定义、引用不悬空，已编译类型照常工作。没有提供方时调用 = `service-unavailable`；替代提供方形状不同 = 启用期同名冲突（消费者须重编译）。事件形状定义只在**最后一份副本**离场时消失 → emit 回退无校验 + journal `event_shape_degraded`。动态消费者显式引用该契约且无其他来源 = 下次启用响亮；仅按服务名调用 = `service-unavailable`。
- **冻结形状共享**：冻结形状图按部署共享（`Arc` / interned ID），不按消费者复制；展开结果按规范化形状记忆化（版本不是键；T10.3）。
- blocking-uninstall（尚有引用时拒绝卸载）= 扩展位（真实摩擦 → 新 ADR / package version）。
- 级联 = 数据驱动反向索引（契约 → 引用方）；实现期 T10.3 / T10.9。

### W7.10 D172 f2 兑现：ADR-0019 C3 supersede

**D172 定案：ADR-0019 C3 整篇 supersede = [ADR-0044](../decisions/0044-contract-package-type-sources.md)；ADR-0018 S6 局部 supersede；Seam 三角与演化规则存活。**

- **ADR-0019 C3 死亡**：`@lib` alias / `.luaurc` / require 包装器 / lib 两层代码库 / 作者 lib 来源（A / B / C）/ lib 代码护栏（纯代码、每 VM 副本、少而精的代码面）。存活：`lib/` 契约数据职能（D164）、契约源三来源（D165）、契约变更级联（D171）。
- ADR-0019 C1 / C2 不由本 ADR 处置（crate 布局 / fiber-core 纯度继续有效；具体 crate 清单随 T13-b 修订，C2 的载体名以 [ADR-0034](../decisions/0034-wasm-first-carrier.md) 为准）。
- **ADR-0018 S6 局部 supersede**：manifest 方法集机器可查 + 插件服务「文档约定起步」两格由契约数据承接（D167 / D170）；Seam 三角（契约独立于提供方）、框架级契约必须有包、加方法兼容 / 删改破坏规则存活。
- ADR-0006 / ADR-0007 不动（schema Rust 可读 / YAML 受限子集）。
- ADR 正文冻结；Status 行记录 partial supersede（ADR-0019 / ADR-0018）；新决策 = ADR-0044。

### W7.11 A7 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 [ADR-0026](../decisions/0026-luau-abi.md)（已 superseded）。

| 机制                                          | 死因                                                                                               |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `buffer` 词                                   | ctx 面归 WIT 富类型；`Value` 无 buffer（W6）                                                       |
| `function` 词                                 | 回调归 guest 语言设施 / WIT resource；`Value` 不含 function                                        |
| ctx 方法签名表达式                            | 平台 import = WIT 富类型原生（W3）                                                                 |
| 命名参数列表签名                              | 服务面恒单 args 形状；ctx 面归 WIT                                                                 |
| 服务返回禁 `nil`                              | 两槽习语随载体消亡；`nil` = 合法 Value（D166）                                                     |
| `bytes` 不立词                                | [ADR-0042](../decisions/0042-value-bytes.md) 提正；`bytes` 进词汇表（D166）                        |
| 契约包 `init.luau`                            | 代码不是契约数据；模块系统死亡（[ADR-0038](../decisions/0038-module-system-require-supersede.md)） |
| 插件根单份 `contract.yml` / `name` ≡ 插件名   | 契约包住 `lib/<name>/`，身份 = 目录名（W1.1 D116；D164）                                           |
| `name:` 字段                                  | 身份 = 目录名；manifest 同纪律（W2 D126；D164）                                                    |
| `version:` 平台内嵌禁写                       | 存活（D165）                                                                                       |
| WIT 生成（盒面）                              | 盒概念死亡（design-review A9）；平台 WIT 手写（W3）                                                |
| 隐式 contract 缺省 = 服务名                   | contractless 两分（D167；W2 D131 2026-10-19 修订）                                                 |
| 外部路径 / registry 契约来源                  | 编译期期望必须随包（D164 / D170）；跨部署分发 = 任务 9                                             |
| `.d.luau` / `export type` 生成                | Luau 工具面消亡；作者类型 = `types!()`（D170）                                                     |
| 文档骨架生成                                  | 工具扩展位；不属 ABI 面                                                                            |
| 递归类型引用                                  | A7 已禁（成环 = 响亮）；原位存活（D163 / D166）                                                    |
| `types:` / `events:` / `methods:` / `errors:` | 存活（D167）；`methods:` / `errors:` 继续服务 Value 形状                                           |
| `map` / `any`                                 | 存活（Value 形状面；WIT 无对应）                                                                   |

**语义平移保留**：类型引用展开 + 规范化比较（D168）；`any` = 诚实宽形状；运行期形状校验（D169）；契约三来源（D165）；按契约名引用、永不按插件名（Seam 三角）；加方法兼容 / 删改破坏（D171）。

### W7.12 联动登记汇总（章末）

| 去向             | 内容                                                                                                                                                                                                                                                    |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1（T13-h）      | `lib/` 包布局边界确认（D116）；W1.11 的 W7 行本批兑现                                                                                                                                                                                                   |
| W2（T13-i）      | `config.type` = 瘦身 DSL（D166）；`provides.contract` 的 `@N` 规则 = D165；`provides.contract` 缺省 = contractless（W2 D131 2026-10-19 修订）；`methods` 字段消亡后的方法集住处 = 契约（D167）；「契约包 = WIT 包」措辞修正（D164）                     |
| W3（T13-j）      | `on-call` 方法集 = 契约数据；服务 args / returns 形状 = 契约（D167）；事件 payload 形状出处 = 契约 `events:`（W8 定名字语义）                                                                                                                           |
| W5（T13-k）      | business data 形状 SSOT = 契约 `errors:`；运行期形状违约 = `core-contract-violation`（D169）                                                                                                                                                            |
| W6（T13-l）      | typed decode 的 schema 来源 = 契约（D170）；TLV 线格式不变；blob payload 不做形状校验（D169）                                                                                                                                                           |
| W8（T13-n）      | **已兑现（2026-10-19）= W8 D173–D179**：事件名语义 / `internal/` 规则 / 声明面 / `on-event` 最终形状；事件形状总表 = 全部契约 `events:` 派生（D165 / D167）；emit 校验违约 = `core-contract-violation` Err + journal（D169）；形状生命周期与降级 = D179 |
| W9（T13-o）      | **已兑现（2026-10-19）= W9 D180–D185**：provides / inject / events 对账与能力门；契约引用缺失两分、同名判定、`@N` 校验的判定面（D168）；import 清单 ⊆ 宿主词汇表；方法集 = 契约数据（D183）                                                             |
| W10（T13-p）     | **已兑现（2026-10-19）= W10 D186–D193**：DSL 词汇 / 产生式演进 = 新快照（D188）；契约版本与 WIT package version 独立（D191）                                                                                                                            |
| config-format    | D29 `lib/` 行按 D164 修正；D36 配置校验消费 `type:` DSL（D166 / D169）；D43 级联机器复用（D171）                                                                                                                                                        |
| service-registry | §7「契约包 = WIT 包」按 D164 修正；S6 局部 supersede（D172）；§7 生命周期措辞按 D171 修正（动态消费者级联 / 编译期消费者重编译）；Seam 三角 / 加方法兼容规则存活                                                                                        |
| plugin-sdk       | S7 `types!()` 覆盖契约四节 + config schema（D170）；S1 / S7 的强类型演进位兑现                                                                                                                                                                          |
| journal          | `contract_violation` / `event_shape_degraded` 原位承接（D169 / D171）；不新增 kind                                                                                                                                                                      |
| ADR-0019         | C3 整篇 supersede = [ADR-0044](../decisions/0044-contract-package-type-sources.md)；Status 行记 partial supersede（D172）                                                                                                                               |
| ADR-0018         | S6 局部 supersede = ADR-0044；Seam 三角与演化规则存活（D172）                                                                                                                                                                                           |
| ADR-0044         | 契约包与类型来源的新决策记录（D163–D172）                                                                                                                                                                                                               |
| T10.3            | 契约包解析 / DSL parser / 规范化比较器 / 运行期 walk / 命名空间合并 / 引用展开与环检测 / 记忆化预算；平台 WIT 包加载（W1 D117 / W2 D132）                                                                                                               |
| T10.9            | 部署索引登记契约包（含休眠件）；`lib/` 变更检测半与反向索引（D168 / D171）                                                                                                                                                                              |
| T10.14           | `types!()` 实现：契约四节 + config schema 投影；`any` → `Value`；typed decode（D170）                                                                                                                                                                   |
| T10.15 / T10.14  | 平台 / 生态契约副本获取路径：v1 = metis 检出或手动复制到作者 `lib/`；`metis sdk export` = 工具扩展位；构建 / `metis check` 验证 `types!()` 引用的契约都在包内 `lib/`（D170）                                                                            |
| T10.7 / T10.12   | provides / inject / events 对账执行；级联重启用执行；安装期契约校验（D168 / D171）                                                                                                                                                                      |
| 实现期配置       | 引用展开 / 结构相等的记忆化或节点预算；冻结形状图按部署共享（`Arc` / interned ID）；契约字段 / 嵌套深度预算（T10.3）                                                                                                                                    |
| 扩展位           | 契约 → WIT 投影；per-method / per-event blob 车道声明；blocking-uninstall；嵌套 config 字段 `secret`；README 缺失 lint；构建期服务名 ↔ 契约绑定检查；跨部署契约发现性（任务 9）                                                                         |

---

## W8 事件静态声明

### W8.0 边界与前提

本章定：事件名语义与 `internal/` 命名空间；manifest `events` 声明的语义与强制程度；事件形状总表与名字绑定；四派发 import 的 emit 校验与违约载体；`on-event` 导出的最终形状与监听者契约；动态名与通配政策；事件面的生命周期、冻结与降级；A8 机制消亡清单。不归本章：`events` import 的方法形状 = W3 D146（本章只定 `on-event` 导出与声明语义）；事件 payload 的线格式 = W6 D160；错误类型 = W5 D136–D141；契约包格式 / 形状 DSL / 规范化 / 引用解析 = W7 D164–D168；provides / inject / events 的机械化对账与能力门 = W9；package version 演进 = W10；宿主 import 实现与派发器 = T10.10 / T10.11；SDK catch-all 与强类型演进 = T10.14。

前提（已定不议）：事件四派发语义 = [ADR-0014](../decisions/0014-dispatch-semantics.md)；`internal/` 保留与 v1 两钩子 = [ADR-0016](../decisions/0016-internal-hooks.md)；实例内事件直上租户总线、无实例作用域 = [instance-model](instance-model.md) D71；listen ≠ 依赖 = [config-format](config-format.md) C2.8；`payload` 双车道 = W6 D160；契约 `events:` 节 = W7 D167；AST `analyze` 管线消亡 = [wasm-first-design-review](../research/wasm-first-design-review.md) A8；事件对账四项（#4–#7）的预期推翻 = 同文档补遗 4。

| 节   | 议题                                | 状态               |
| ---- | ----------------------------------- | ------------------ |
| W8.1 | 事件名语义与命名纪律                | ✅ D173 2026-10-19 |
| W8.2 | 声明面语义与强制程度                | ✅ D174 2026-10-19 |
| W8.3 | 事件形状总表与名字绑定              | ✅ D175 2026-10-19 |
| W8.4 | emit 校验与违约载体                 | ✅ D176 2026-10-19 |
| W8.5 | `on-event` 导出与监听者契约         | ✅ D177 2026-10-19 |
| W8.6 | `internal/*` 命名空间与平台内嵌事件 | ✅ D178 2026-10-19 |
| W8.7 | 生命周期、冻结与降级                | ✅ D179 2026-10-19 |
| W8.8 | A8 机制消亡清单（考古索引）         | ✅ 2026-10-19      |
| W8.9 | 联动登记汇总（章末）                | ✅ 2026-10-19      |

### W8.1 D173 事件名语义与命名纪律

**D173 定案：事件名 = 扁平、精确匹配的 UTF-8 字符串；只有 `internal/` 前缀保留；不立字符集文法、通配与实例作用域。**

- **键语义**：事件名是租户事件总线内的全局键，不是插件命名空间；比较 = UTF-8 全等，大小写敏感，不做 Unicode 归一化。名字非空；空名 = `core-contract-violation` Err（emit / listen 同判）；长度上限 = 实现期配置（T10.10），超限 = 同格 Err。
- **保留前缀**：只有 ASCII `internal/`（精确小写 + 斜杠）保留给框架；`Internal/`、`internal`（无斜杠）不保留。`internal/` 的发射规则 = W8.6。
- **命名约定**：文档建议 `plugin-name/event`，不强制；名字不因插件身份自动前缀化（manifest 无 `name`，W2 D126）。
- **不立通配**：`*` / `#` 等没有特殊语义；声明面全静态事件名（A8 D23.1 原位）。
- **动态名**：允许构造，但名字是精确键——拼出的名字只有恰好等于总表定义时才获得形状校验。引导写法 = 静态名 + 动态参数进 payload（`session/input` + `session_id` 字段，instance-model D71）。
- **无实例作用域**：事件直上租户事件总线；同租户跨实例互相可见，按 payload 字段过滤是监听者责任（instance-model D71）。
- **载体事实**：WIT 面只有 `string name`，事件名无法进 world 静态面（E5 拍板）；声明面 = manifest，形状面 = 契约。
- **人读面转义**：事件名不立字符集文法，但进 journal / 审批 diff 等展示面时按展示层规则转义控制字符与非 ASCII 混淆字符；键空间本身不收紧（不引入第二套名字语法）。
- **值错误载体**：空名 / 超长名复用 `core-contract-violation`（封闭枚举里最接近的格）；若将来值错误族成串，走新 package version（W10 D188）的新 kind 议题，不在此另立。

### W8.2 D174 声明面语义与强制程度

**D174 定案：manifest `events` = 意图 + 审批面，运行期不强制；`publish` 名须有形状出处，`listen` 任意名合法；不立源码漂移对账。**

- **字段**：`events: { publish, listen }`，两项皆为扁平事件名列表（W2 D131）；名单按集合语义，重复项 = 安装期 manifest 校验错误；名单项名值规则 = D173（非空 / 长度上限），违规 = 安装期 manifest 校验错误；未知字段 = W2 D125 / D133 原位。
- **listen = 弱耦合被动面**：未定义、无人 publish、未来词汇都合法；不因未声明而拒绝 `listen`；`listen` 的 `result` 只承载值错误（空名 / 超长名 / 条目预算超限）与宿主侧失败，不承载「名字未声明」（config-format C2.8「listen ≠ 依赖」的 ABI 面兑现）。**审批面读法**：`events.listen` 是流入意图的可见面，不是运行期白名单——未声明的 `listen`（含 `internal/*`）仍可发生；审批覆盖 = 可见性，不是闸门。
- **publish = 主动面 + 形状出处门**：每个 publish 名必须在事件形状总表中有定义（三来源任一，W8.3）；缺失两分 = 部署契约索引中存在但来源插件未启用 → Pending（等启用，无超时，warn 政策 = W9 / A8 D22.3）；索引查无 → 启用期响亮，报错写清缺哪个名字、可由谁提供。
- **`internal/` 声明**：`events.publish` 含 `internal/*` = 启用期响亮（插件发不出该名字）；`events.listen` 含 `internal/*` = 合法（观测一等用法）。
- **不声明也可 emit / listen**：运行期不强制；未声明 = 不参与审批面，但仍受 emit 形状校验（按名字跟总表走，W8.4）与 `internal/` 拦截。W2 D131「无契约也可发事件」由此保持：未声明的 contractless 名字可发，代价 = 无形状校验、无审批可见性。
- **审批覆盖面**：`events.publish` 只覆盖有形状定义的主动面；contractless 发射结构性不可声明（D174 的取舍）——它们在审批 diff 中不可见，且无形状校验。需要可见性 = 扩展位（允许无定义 publish 并在审批面标 `unvalidated`，走 manifest 演进 / 新 package version）。
- **淘汰源码漂移机器**：A10 D28.3 的四项事件检查（#4 emit↔publish / #5 on↔listen / #6 动态名拼接 / #7 跨插件同名无契约；§7 曾预期随 E5 存活）在 wasm 期无源码扫描面，D174 明确淘汰（[补遗 4](../research/wasm-first-design-review.md)）；不立运行期对账（不新增 journal kind）。`metis check` 只做静态预演：manifest 结构、publish 出处（部署上下文内）、DSL 语法；不扫代码。
- **声明价值**：审批 diff 展示「想发什么 / 想听什么」，部署者裁决；声明不是运行时白名单。
- **零仪式插件**：无 manifest → 无声明面；emit 形状校验不因形态豁免（W1.8）。

### W8.3 D175 事件形状总表与名字绑定

**D175 定案：事件形状总表 = 三来源契约 `events:` 的逻辑并集；名字即绑定；listen 不写形状；publish 名必须解析；作者 `internal/*` 定义仅允许与平台定义规范化相等。**

- **派生**：总表从平台内嵌契约、部署 `lib/` 生态契约与插件随包契约的 `events:` 节逻辑合并（W7 D165 三来源）；键 = 精确事件名；不搬文件、不进模块系统。
- **同名**：规范化后结构相等 = 共存（契约竞标）；两个具体形状不等 = 启用期响亮并指名双方；`any` 与一切相容，具体形状胜出（W7 D168 原位）。
- **名字即绑定**：publish / listen 名单零重写；listen 永不写形状——监听者的类型知识由 `types!()` 从总表读取（W7 D170），重复抄写 = 分歧源。
- **publish 出处**：名字必须在总表中解析（D174）；定义写 `any` 合法（不表态 = 诚实宽形状）。
- **平台 `internal/*` 形状**：平台内嵌契约是唯一权威；v1 = `internal/plugin` / `internal/status`，具体字段随 T10.4 实现冻结（首个发布快照前）。本节只定机制，不冻结字段形状。
- **作者复制件例外**：使用 `types!()` 监听 internal 事件的插件，可把平台契约复制进自己包的 `lib/`（发布仅限宿主，插件不复制用于发布）；该副本可含 `internal/*` 名，**仅当该 `internal/*` 事件条目及其类型引用展开后与平台定义规范化相等**。这是对 `internal/*` 命名空间的显式收紧：W7 D168 的 `any` 相容规则在此不适用——复制件必须是同形副本，不是放宽版；其余非 internal 条目仍按 D168。新增或分歧的 `internal/*` 名 = 启用期响亮——谁都发不出的死定义不许进场；复制件是类型来源，不是第二权威。
- **无契约事件**：总表无定义 → emit 不校验（D176）；仍可 listen；不要求 manifest publish（D174）。W2 D131「无契约也可发事件」由此保持。
- **数据面**：总表是宿主数据（契约解析产物），不是 WIT；WIT 不承载动态名（E5 拍板）。解析 / 合并 / 规范化 = T10.3；消费 / 派发 = T10.4。

### W8.4 D176 emit 校验与违约载体

**D176 定案：四派发在入队前按活表校验 payload；违约 = `core-contract-violation` Err + journal；`internal/*` 冒用 = trap。**

- **检查点**：宿主 import 实现内，TLV 合式性 / 预算检查后、入邮箱（emit）或开始派发（serial / parallel / waterfall）前；一次树 / 字节走查，不新增拷贝（W7 D169 成本口径）。
- **查找**：按精确事件名查当前部署级总表（W8.7 活表）；无定义 / 归一化为 `any` = 跳过；具体形状 = walk。
- **车道**：`payload.blob` = opaque，不参与形状校验；事件名仍受空名 / 超长 / `internal/` 规则约束。value 车道越过契约形状 = 违约。
- **违约**：`core-contract-violation` Err 给发射方 + journal `contract_violation`，不派发；message 人读，结构化细节（事件名、违约路径、期望形状）进 journal。四派发同判。
- **空名 / 超长名**：同格 Err（D173）。
- **`internal/*` 冒用**：guest 四派发任一 = trap——宿主拒绝调用、实例 Failed、journal `vm_trap`；依赖方收 `service-unavailable`。理由 = 命名空间边界冒用，与 W5 D139「未声明能力调用 = trap」同族；不降级为 Err。宿主自身 emit `internal/*` 不走 guest import，不受此检查；宿主按平台契约自家定义构造。
- **转发危害明记**：名字是运行期字符串，宿主无法区分「字面量冒用」与「转发数据的名字恰好以 `internal/` 开头」；转发型插件必须在转发前自行校验名字（SDK 提供 `is_reserved` 类助手 = T10.14 扩展），否则外部数据可触发实例死亡。这是「命名空间边界 = 响亮」的明示代价。
- **无校验面**：contractless / 全 `any` / blob 车道；TLV 合式性仍检查（W6 D156）。
- **形状退化**：唯一形状定义离场 → 该名字回退为无校验 + journal `event_shape_degraded`（W8.7；journal D78）。
- **返回值不校验**：emit 忽略；serial / waterfall 链值不校验；parallel 逐项收集。事件返回形状声明 = 扩展位（A8 D22.5 原位）。
- **零监听者仍校验**：校验是无条件的（有定义即 walk），不以「当前无人监听」为跳过条件——形状违约是作者错，不因部署恰好无人订阅而静默；成本 = 一次 walk，v1 接受。
- **成本**：一次 walk；typed walk 直接走编码字节（W7 D169）；v1 不做投机优化。

### W8.5 D177 `on-event` 导出与监听者契约

**D177 定案：`on-event` 进 v1 world 导出集（required，SDK 默认 `Ok(None)`）；listener-id 每实例作用域；四派发返回语义按 ADR-0014；listener 错误 fail-open / parallel 逐项收。**

```wit
// W1.3 world 的 W8 增补（payload / error 引用同 W1.3）
export on-event: async func(listener-id: u64, name: string, payload: payload) -> result<option<payload>, error>;
```

- **required 导出**：world 导出集 = 生命周期三 + `on-call` + `on-timer` + `on-event`；缺任一 = 安装期 world 不匹配。SDK 提供默认 `Ok(None)`，零仪式插件不需手写（W1.8）。
- **调用**：宿主只向已登记 listener 投递；一个 (实例, 事件名) 一个宿主条目（同名去重 + 引用计数，W3 D146），因此每实例每次派发至多一次 `on-event`；`listener-id` 用于逐监听者保序 / 短路，宿主校验归属（外来 id = trap；自己的已摘除 id = 幂等 no-op，W3 D146 原位）。
- **参数**：`name` = 原始事件名；`payload` = 双车道（W6 D160），宿主不重编码；blob 句柄按 W6 D161 所有权规则转移。
- **返回**：`emit` = 忽略返回值；`serial` = 首个 `Ok(Some(payload))` 短路并带回，`Ok(None)` 继续；`waterfall` = `Some` 继续链，`None` 否决；`parallel` = 逐项收集 `result<option<payload>, error>`；`Err` 在 emit / serial / waterfall = 记日志后继续（fail-open，ADR-0014），在 parallel = 逐项返回。
- **错误方向**：listener `Err` 只许 `business`（W5 D138）；非 business = 宿主判 `core-contract-violation`——emit / serial / waterfall = journal `listener_error` + `contract_violation`，fail-open；parallel = 该项结果计 `core-contract-violation`。宿主不把非 business 静默降级为 business。
- **条目预算**：每实例 listener 条目数（含 host-owned / `detach()` 保留的条目）= 实现期配置（T10.10），必须设上限；超限 = `listen` 返回 `core-contract-violation` Err（message 带预算）。`detach()` 不释放条目，动态唯一名 + detach 可耗尽预算——作者应复用稳定名；这是 ADR-0014「监听者百级以内」规模纪律的 ABI 面兜底。
- **作者面**：SDK 分发到 catch-all `Plugin::on_event(ctx, name, payload) -> Result<Option<Payload>, Error>`（plugin-sdk S1）；按事件名生成强类型方法 = S7 演进位，不属 v1。
- **卸载 / 关闸**：Unloading 期投递按 W4.5；在飞 `on-event` 按五段化 drain；卸载丢弃可见 = `unload_gate_closed` / `unload_killed`。
- **`internal/*` 监听**：允许；v1 internal 事件由宿主 emit（只观测），返回值为 `None` 被忽略，`on-event` 仍被调用。
- **无收信复核**：listener 收到的 payload 已按 D176 校验（有定义时），收信复核无意义（A8 D22.5 原位）；listener 返回链值不校验。

### W8.6 D178 `internal/*` 命名空间与平台内嵌事件

**D178 定案：`internal/*` 只许宿主 emit；插件可 listen；平台内嵌契约是形状权威；插件目录名规则死亡。**

- **保留前缀**：`internal/`（精确小写 + 斜杠）；插件四派发冒用 = trap（D176）。
- **平台 v1 事件**：`internal/plugin` / `internal/status`，emit-only（只观测不介入，ADR-0016）；宿主在 fiber 生命周期点 emit；具体 payload 形状 = 平台内嵌契约，字段随 T10.4 实现冻结。
- **作者契约**：可复制平台契约（规范化相等，D175）；新增 / 分歧 `internal/*` 名 = 启用期响亮。部署 `lib/` 生态契约同判。
- **契约名不保留**：`lib/internal/` 作为目录名合法，但其中的 `internal/*` 事件仍受上述约束；不立第二套命名保留。
- **插件目录名规则死亡**：A8 的「插件目录取名 `internal` = 保留」在 wasm 期无兑现面——manifest 无 `name`（W2 D126），插件名不构成事件命名空间。
- **发送方身份与冒充**：`on-event` 不携带发送方身份，v1 不鉴别「谁发的」——任何插件可发任何非 `internal/` 名（事件本是开放词汇）。需要来源鉴别 = 服务调用（点对点 + 注册表，ADR-0018 语义原位）；事件来源鉴别 = 扩展位（`on-event` 增补 `from` 参数 = 新 package version）。宿主可见实际派发方，`listener_error` / `contract_violation` journal 记录派发方供事后取证。
- **internal 监听审批读法**：`events.listen` 中的 `internal/*` 是可见意图，不是运行期闸门；未声明的 internal 监听仍可发生（D174 的开放广播取舍），审批面只提供可见性。
- **不占能力门**：`internal/` 事件不进能力声明、不占 import；emit 拦截是宿主实现内的命名空间检查，不是能力声明。

### W8.7 D179 生命周期、冻结与降级

**D179 定案：事件形状总表 = 部署级活视图；声明过 publish 的发射方进反向索引并级联；最后一份定义离场 = 降级无校验 + journal。**

- **活表**：总表随契约来源变化重建（部署 `lib/` 增删改 = boot / 显式 reload，W7 D171 / config-format D43；插件随包契约 = 插件更新审批事务）；emit 校验读当前活表。替换必须原子——读者看到完整旧表或完整新表，不得看到半重建表（`Arc` / `ArcSwap` 形态，T10.4）。
- **声明冻结**：各插件在启用事务内解析并冻结自己的 publish 绑定 / 类型引用（W7 D171 原位）；两次启用之间，编译期期望恒定。
- **破坏性变更**：已有名字的形状变更 / 删除条目 = 契约版本 bump——仅部署级生态契约有 `version:`（W7 D165）；插件随包契约禁写版本，靠同部署内同名对账与级联送达。反向索引（契约 → publish 绑定者 / 类型引用者）级联重启用；动态消费者按新形状继续；编译期消费者（`types!()` 随包副本）必须重新编译——旧副本与新契约同名不同形状 = 启用期响亮。
- **未声明发射方**：不在反向索引；emit 时读活表，形状变化可能使其运行期失败（`core-contract-violation` Err），不是 trap。这是「校验跟名字走」的诚实代价（A8 D22.4 语义原位）。
- **最后一份定义离场**：该事件名 emit 回退为无校验 + journal `event_shape_degraded`（一次；journal D78）；监听者已冻结的类型知识不受影响；监听登记不清除。
- **监听者 / 发射方卸载**：监听条目随实例卸载摘除（W4.5）；在飞派发按五段化；卸载丢弃可见。
- **无事件持久化**：总表不是日志；journal 只记违约 / 降级 / 监听错误；无 replay。
- **实现期**：记忆化 / 节点预算 / 形状图共享 = T10.3 / T10.4（W7 D171 已登记）。

### W8.8 A8 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 [ADR-0026](../decisions/0026-luau-abi.md)（已 superseded）。

| 机制                                                                                       | 死因                                                                                                                                            |
| ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| A10 D28.3 事件四项（#4 emit↔publish / #5 on↔listen / #6 动态名拼接 / #7 跨插件同名无契约） | §7 曾预期随 E5 存活；wasm 无源码扫描面，D174 淘汰（[补遗 4](../research/wasm-first-design-review.md)）；声明面降为审批面 + publish 出处静态预演 |
| listen 期望形状声明 / 事件返回形状字段                                                     | v1 不立（A8 D22.5 原位；返回形状 = 扩展位）                                                                                                     |
| manifest 内联 `define:` 节                                                                 | 形状唯一住处 = 契约包（W7 D164 / D167）                                                                                                         |
| 通配语法                                                                                   | A8 从未立（D23.1 原位）                                                                                                                         |
| 插件目录名 `internal` 保留                                                                 | wasm 期插件名不构成事件命名空间（D178）                                                                                                         |
| 契约定义 `internal/*` = 一律响亮                                                           | 修正为「仅允许与平台定义规范化相等的复制件」（D175 / D178）                                                                                     |
| Luau `sdk types` 的 `.d.luau` / `export type` 事件类型生成                                 | `types!()`（W7 D170）                                                                                                                           |
| 事件 payload = Luau table 深快照 / 契约优先槽位                                            | TLV Value + 总表 walk（W6 / W7）                                                                                                                |
| 两槽 `res, err` 事件返回                                                                   | `result<option<payload>, error>`（W5 D136）                                                                                                     |
| `internal/dispatch` 钩子                                                                   | 替换为 tracing（ADR-0016；W8 不复活）                                                                                                           |
| 运行期全强制 / 能力门式事件白名单                                                          | 明确淘汰（A8 D20.2 理由原位：开放广播、观测类插件听一切）                                                                                       |
| journal 内部流量族（`message_enqueued` / `dispatch_completed` 等）                         | journal D84 v1 骨架化，事件面只剩违约 / 降级 / 监听错误（非 W8 新增死亡，注明）                                                                 |

**语义平移保留**：四派发语义（ADR-0014）；fail-open；`internal/` 保留；声明 = 审批面；名字即绑定；publish 出处两分；`any` 诚实宽形状；emit 按名字校验；事件词汇开放。

### W8.9 联动登记汇总（章末）

| 去向                     | 内容                                                                                                                                                                                                             |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1（T13-h）              | **已兑现（2026-10-19）**：`on-event` 进入 required 导出集并补全 world 草图（D177）；W1.11 的 W8 行本批兑现                                                                                                       |
| W2（T13-i）              | **已兑现（2026-10-19）**：`events` 声明语义（publish 出处 / listen 自由 / `internal/` 规则）；无字段改动                                                                                                         |
| W3（T13-j）              | **已兑现（2026-10-19）**：`on-event` 最终形状；`listen` 未声明名恒 `Ok`；四派发校验点 = D176；world 导出集补 `on-event`                                                                                          |
| W5（T13-k）              | **已兑现（2026-10-19）**：listener 错误封套 = D136–D141；非 business listener 错误 = `core-contract-violation`（D138）；emit 形状违约 = `core-contract-violation`（D139）；`internal/*` 冒用 = trap；不新增 kind |
| W6（T13-l）              | **已兑现（2026-10-19）**：`payload` 双车道；blob 不参与形状校验；`on-event` 返回 `option<payload>`；链值所有权按 D161                                                                                            |
| W7（T13-m）              | **已兑现（2026-10-19）**：总表 = 契约 `events:` 并集；D168 同名规则；D171 生命周期；D169 事件校验点由 W8 钉死                                                                                                    |
| W9（T13-o）              | **已兑现（2026-10-19）= W9 D180–D185**：机械化对账 = provides / inject / events（publish 出处缺失两分、`internal/` 规则、import 清单 ⊆ 词汇表）；W8 只定语义                                                     |
| W10（T13-p）             | **已兑现（2026-10-19）= W10 D186–D193**：`on-event` 导出 / WIT 面演进 = 新快照（D188）；事件名 / 声明面词汇 = manifest 字段集演进纪律（`internal/*` 词汇增补同）                                                 |
| T10.3                    | 契约解析 / 事件总表合并 / 规范化 / 引用展开 / 记忆化预算                                                                                                                                                         |
| T10.4                    | 派发器；总表消费；emit 校验；`on-event` 调用与 listener 表；`internal/plugin` / `internal/status` 的 payload 形状与发射点                                                                                        |
| T10.10 / T10.11          | 宿主 import 实现；`internal/*` 冒用拦截；事件名长度预算                                                                                                                                                          |
| T10.12                   | 安装期 world 导出校验补 `on-event`                                                                                                                                                                               |
| T10.14                   | SDK 默认 `on_event`；`Listener` RAII；名称常量 = 扩展位（v1 不立）                                                                                                                                               |
| T10.15                   | 脚手架 / `metis check` 预检 world 导出集补 `on-event`；不扫代码                                                                                                                                                  |
| context-events           | §7 W8 兑现注；事件总线 / 派发语义不变                                                                                                                                                                            |
| instance-model           | D71 事件跨界 / 静态名 + payload 参数化兑现；D72 = 事件面不新增 journal 字段                                                                                                                                      |
| config-format            | C2.8 listen ≠ 依赖兑现；C8.3 W8 行兑现                                                                                                                                                                           |
| plugin-sdk               | S1 catch-all / `on_event` 默认实现；W8 行兑现；强类型事件方法 = S7 扩展位                                                                                                                                        |
| service-registry         | §6 事件形状共享同一契约包兑现                                                                                                                                                                                    |
| journal                  | 不新增 kind；`listener_error` / `contract_violation` / `event_shape_degraded` 原位                                                                                                                               |
| 实现期配置               | 事件名长度上限；事件总表节点 / 引用展开预算；事件 payload 的 TLV 深度 / 字节预算（原位）；listener 每实例条目上限（必设，含 host-owned / detach 条目）                                                           |
| 扩展位                   | 事件返回形状声明；强类型事件方法（S7）；`types!()` 事件名常量；通配（维持不立）；事件审计族（维持不立）                                                                                                          |
| wasm-first-design-review | 补遗 4：A10 D28.3 #4–#7 事件对账的去向 = D174 淘汰（wasm 无源码扫描面）                                                                                                                                          |
| ADR                      | 不新增；A8 为归档设计文档（非 ADR）；ADR-0016 机制不变、正文冻结；兑现记录住本章联动行                                                                                                                           |

---

## W9 deps/provide 与能力门

### W9.0 边界与前提

本章定：WIT 化后的对账时机分界；组件 import 清单的词汇表闭包判定；WASI 0.3 基线白名单；inject / provides / events publish / 撞名 / 判环五格的启用期判定形态；方法集对账的处置；能力门执行语义；失败报告与 `metis check` 范围；A9 机制消亡清单。不归本章：manifest 字段形状 = W2；import / export 接口形状 = W1 / W3 / W6；契约包格式、规范化比较与生命周期 = W7；事件名语义与 emit 校验 = W8；错误类型 = W5；package version 比较与多版本共存 = W10；部署索引解析 = T10.9；启用管线 = T10.7 / T10.12；`metis check` 命令面 = [build-pipeline](build-pipeline.md) D106；宿主 import 实现 = T10.10 / T10.11。

前提（已定不议）：对账时机统一 = 启用期（A9 D24；W2 D125 同判；自完备结构事实按 D180 在安装期检出并记录）；五格语义冻结存活（A9 D25.1–D25.4 + W8 D174；D25.2 的方法集半格按 D183 死亡；A9 D25.5 的盒双向半死亡、进口半改形 = W9.2）；能力面 / 契约控制面 = WIT 富类型（三分法①，W7 D163）；能力门 = 运行期 fail-closed（A9 D26；W3 D154；W5 D139）；固定 world 不裁剪（W3 D143）；可见集 = 实例作用域 + parent 链（[instance-model](instance-model.md) D68）；部署索引 = 目录扫描全量，含休眠件与未装配件（[config-format](config-format.md) D34）；warn 与对账失败 journal 条目 = [journal](journal.md) J1.4 D78（`pending_warn` / `reconcile_failed`）；插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）。

**五格口径**：① inject 三分（W9.4）② provides 契约（W9.5）③ provides 撞名（W9.5）④ required 判环（W9.7）⑤ events publish 出处（W9.8）。A9 D25.5 的盒双向对账半死亡（由组件文件校验承接，W1.1），盒进口对账半改形为全插件进口词汇表（安装期结构事实，W9.2–W9.3）；进口词汇表与五格同批复核、不同时机。

| 节    | 议题                                            | 状态               |
| ----- | ----------------------------------------------- | ------------------ |
| W9.1  | D180 判定时机：安装期结构事实 / 启用期跨实体    | ✅ D180 2026-10-19 |
| W9.2  | D181 进口对账：词汇表闭包（B1 口径）            | ✅ D181 2026-10-19 |
| W9.3  | D182 WASI 0.3 基线白名单                        | ✅ D182 2026-10-19 |
| W9.4  | A9 D25.1 原位：inject 三分与可见集              | ✅ 2026-10-19      |
| W9.5  | A9 D25.2 / D25.3 原位：provides 契约与 key 撞名 | ✅ 2026-10-19      |
| W9.6  | D183 方法集对账消亡                             | ✅ D183 2026-10-19 |
| W9.7  | A9 D25.4 原位：required 依赖图判环              | ✅ 2026-10-19      |
| W9.8  | W8 D174 机械化：events publish 出处对账         | ✅ 2026-10-19      |
| W9.9  | A9 D26 原位：能力门执行语义（WIT 形态）         | ✅ 2026-10-19      |
| W9.10 | D184 启用期失败的聚合报告                       | ✅ D184 2026-10-19 |
| W9.11 | D185 `metis check` 的五格范围                   | ✅ D185 2026-10-19 |
| W9.12 | A9 机制消亡清单（考古索引）                     | ✅ 2026-10-19      |
| W9.13 | 联动登记汇总（章末）                            | ✅ 2026-10-19      |

### W9.1 D180 判定时机：安装期结构事实 / 启用期跨实体

**D180 定案：自完备结构事实在安装期解析并记录；跨实体判定在启用期。休眠件的结构缺陷 = 索引标记 + 一条 warn，启用时响亮。**

- **安装期（纯数据，不跑插件代码）**：manifest 解析、未知字段与类型校验（W2 D125 / D133）；组件类型解析（import 清单与 export 清单）；导出集与宿主 world 匹配（W1.3 D118；零仪式同判 W1.8 D123）；导入清单 ⊆ 宿主词汇表（W9.2 / W9.3）的判定结果记录；随包契约解析与部署契约索引登记（W7 D164 / D165）；`component` 文件存在性、唯一性与组件 magic（W1.1 D116）。
- **结构缺陷两分**（A9 D24 解析失败政策平移）：启用对象 = 记录「不可用」原因，启用时响亮；休眠件 / 未装配件 = 索引标记损坏 + host warn 一条，不拖垮启动（config-format D34）。将来被启用 = 以记录原因响亮。
- **启用期（跨实体判定）**：config 校验与秘密解析（config-format D36）、inject 三分（W9.4）、provides 契约对账（W9.5）、key 撞名（W9.5）、required 判环（W9.7）、events publish 出处（W9.8）、setup 成功后的 provide 履行对账（W9.5）。
- **运行期**：能力门逐调用 fail-closed（W9.9），不属对账时机。
- **W1.1 / W1.3 / A9 D24 措辞读法**：W1.1「非组件 / 多 `.wasm` = 安装错误」与 W1.3「缺导出 = 安装期 world 不匹配错误」均按「安装期检出并记录」读——不是「安装事务整体拒绝」。A9 D24.1 的「唯一留在安装期的判定」按「唯一的语义判定」读；自完备结构事实（组件类型、导出集、导入词汇表、契约解析、文件存在性）归 D180。W1.1 / W1.3 正文不改写，读法住本节。
- 对账时机只有两段；能力门是运行期逐调用，不在此两段内。mount 走标准启用管线（instance-model D63），判定原样复用。`metis check` 用同一实现做静态预演（W9.11）。

### W9.2 D181 进口对账：词汇表闭包（B1 口径）

**D181 定案：进口对账 = 组件 import 清单 ⊆ 宿主词汇表，按接口粒度；不做 import → capability 的静态推断。**

- **词汇表** = `metis:plugin@<宿主支持版本>` 的 import 接口集 + WASI 0.3 基线（W9.3）。metis 侧 = W3 D143 的 9 个功能接口（`events` / `services` / `timer` / `http` / `fs` / `process` / `instance` / `blobs` / `streams`）与 3 个类型接口（`errors` / `fs-types` / `payloads`）。
- **粒度 = 包 + 接口**（W3 D143）：接口身份即封闭，函数级不单独判定；聚合 `ctx` 接口不立（W3 D143 原位）。
- **判定**：安装期解析 import 清单并对照宿主词汇表；任一条目不属于上述集合 = 索引记录为不可用（原因写明包名 / 接口名与「词汇表外」），被启用时以记录原因响亮（W9.1 的两分政策）。`metis:plugin` 的版本比较 = W10 D187；多版本共存时每个支持版本各一份词汇表（W10 D189）。
- **B1 口径写实**：固定 world 不裁剪（W3 D143）——所有 metis 功能接口恒在 import 清单，宿主恒链接；因此 import 清单不能证明「代码实际用了哪个能力」。B1 的「集合包含」= 词汇表闭包，不是能力漂移检测。D28.3 #1–#3（能力漂移 / scope 键↔命名空间 / 盒对账）死亡的理由 = wasm 无源码扫描面（W8 D174 同批），不是 import 能证明能力使用。
- **能力授权唯一来源 = manifest `capabilities`**（W2 D129）；未声明调用 = trap（W5 D139），message 指路 manifest 字段。import 存在不等于授权；capability 声明存在也不因 import 缺失而失效（未使用的声明合法）。
- **收益**：手写 / 外来 / 跨版本组件引入词汇表外的包或接口 = 启用时可读错误，而不是实例化时的 linker 报错。SDK 产物的 import 面恒为固定 world，索引展示只在上述异常组件上有信息量。
- **不做**：import ↔ capability 对账（固定 world 下不可靠，误报源）；以 import 清单反写 manifest；扫描 guest 源码。

### W9.3 D182 WASI 0.3 基线白名单

**D182 定案：WASI 基线 = 具名三族 + 钉版工具链 std 拖带且宿主已注册的 WASI 0.3 接口；逐条清单以宿主注册集为准，文档不冻结枚举。**

- **具名三族**：`wasi:clocks/*` / `wasi:random/*` / `wasi:filesystem/*`（async；private fs 与 `std::fs`，W3 D144）。
- **std 拖带集**：被钉版 Rust toolchain 的 `std` 无条件拖带、且宿主 wasmtime-wasi 0.3 已注册的接口，视为基线；不计入作者能力面。逐条清单 = 宿主构建期的注册集记录（T10.10），`metis check` 与部署索引复用同一份记录。
- **未确认项**：WASI 0.3 std 的精确逐条拖带集受 toolchain pin 影响，本文档不枚举；确认方式 = T10.10 以宿主注册集实测并登记。wasip2 的 14 导入（[wasm-research](../research/wasm-research.md) 补遗 6 papercut 1）为历史证据，不作为 0.3 基线。
- **基线内接口 = 基础设施白名单，不是能力**；作者手写基线外的 WASI import = 词汇表外（W9.2），安装期记录 / 启用时响亮。
- **演进**：基线增补（工具链升级带来新拖带接口）不触发 `metis:plugin` package version——WASI 基线不是 metis ABI 面；宿主注册集随 T10.10 的 wasmtime / toolchain 升级变化。**W10 注（2026-10-19）**：每个 WIT 快照的 `vocabulary.yml` 记下冻结时的基线版本集（W10 D189）；宿主实际注册集 ⊇ 该集合 = 该快照可服务；宿主升级删旧 WASI 版本 = 对旧插件的事实性破坏（W10 D191）。
- **private fs 边界**：preopen 授予 = T10.12；基线白名单不等于任意路径可读（WASI preopen 只给 `plugins/<name>/data/fs/`，ADR-0039）。

### W9.4 A9 D25.1 原位：inject 三分与可见集

- **声明面** = manifest `inject.required` / `optional`（W2 D131）；作者面消费 = `services.deps()` 目录 + `Ctx::service`（W3 D147）。
- **可见集** = 沿 fiber parent 链可达集（instance-model D68）：实例作用域先查、逐级向上、租户注册表兜底；兄弟 / 子实例提供方不可见（按「查无」同族处理）。
- **四状态三分**（A9 D25.1 原位）：

| 提供方状态                               | 处置                       |
| ---------------------------------------- | -------------------------- |
| enabled 未就绪（Loading / Pending）      | Pending 等就绪；无 warn    |
| enabled 但 Failed                        | Pending 等恢复 + warn      |
| 已安装未启用（休眠件，`enabled: false`） | Pending 等启用 + warn      |
| 可达集查无 key                           | 启用期响亮，写清缺哪个 key |

- **未装配件**（目录在、树无 entry）= 行四同族响亮，措辞区分「目录存在但树无 entry」（config-format D34）；残迹目录 = 行四，可在措辞层提示「该 key 曾由残迹目录提供」（config-format C7.2 D49），不立机制。
- **warn**：进入 Pending 当场一条；等待中途提供方转 Failed 当场补一条；每进入一次一条，不周期重发；重启 / 重启用后再次进入 = 新发一条。`internal/plugin` / `internal/status` 事件照常携带状态；journal `pending_warn`（J D78）。指路两条 = 启用 / 修复提供方，或修正依赖声明。
- **无超时**：Pending 不自行失败；恢复 = 配置驱动（ADR-0018 S4）。
- **optional**：缺席不阻塞激活、不进 Pending；`Ctx::service(name)` = `None`；出现 / 消失 / 热更统一重启消费者（ADR-0018 S3 复审点未触发——`deps()` 使 setup 期探测合法，维持重启纪律）。
- **错误面**：未声明服务的调用 = trap（W5 D139）；已声明但运行期缺席 = `service-unavailable` Err（W3 D147）。

### W9.5 A9 D25.2 / D25.3 原位：provides 契约对账与 key 撞名

- **声明面** = manifest `provides` map，条目 `{ contract }`（W2 D131）；`contract` 缺省 = contractless。
- **契约引用两分**（A9 D25.2 / W7 D168 原位）：显式 `contract: name` 在部署契约索引中存在但来源插件未启用 → Pending 等启用（反向索引唤醒，warn 政策同 W9.4）；索引查无 → 启用期响亮，指路「用现成契约 / 自己随包定义」。`@N` 与来源类别规则 = W7 D165（生态契约必带 `@N`、内嵌 / 随包禁带、不符 = 响亮）。
- **形状对账**：同名契约跨来源规范化比较（W7 D168）；两个具体形状不等 = 启用期响亮并指名双方；`any` 相容规则原位；契约竞标（规范化相等）合法。
- **contractless**：允许，无形状对账；失去方法存在性短路与类型知识（W3 D147 / W7 D167）；审批面可见「无契约」。
- **provide 履行对账**（ADR-0018 S2 / service-registry §2 原位）：`services.provide(key)` 在 setup 内登记；setup 成功后宿主对账「manifest `provides` 的 key 全部履行」——缺 = 激活失败响亮；provide 了未声明 key = 响亮；Active 后 provide = trap（S2 原子注册）；Unloading 期新注册 = trap（W4.5）。
- **key 撞名**（A9 D25.3 / instance-model D68 原位）：判定作用面 = **同作用域内**已启用 ∪ Pending ∪ 本次启用者；跨作用域同 key = 遮蔽不是撞名；休眠件之间不判定。撞名 = 本次启用失败，报错指名对手方。运行时 host 复查保留（S2 双保险）。
- **判定材料** = 部署索引 + 同作用域注册表；不读插件源码。

### W9.6 D183 方法集对账消亡

**D183 定案：提供方不注册方法集；方法集 = 契约数据；A9 D25.2 的「注册方法集 ⊆ 契约」不复活。**

- **死亡**：`provides.methods`（W2 D131 已消亡）、`provide(key, methods)` 形态、启用期「注册方法集超出契约 = 响亮」。
- **存活**：方法集 = 契约 `methods:` 节（W7 D167）；作者类型 = `types!()`（W7 D170）；契约引用 / `@N` / 形状对账（W9.5）。
- **运行期**：`on-call` catch-all 决定行为（W3 D147）；契约已知时宿主按契约方法集短路 `service-method-missing`；contractless 服务的 missing method = guest 返回 `business`（message = method missing）。
- **理由**：wasm 化后没有提供方侧注册面承载方法集；再加一份清单 = 与契约重复、双份维护（W2 D131 已按此方向删字段）。
- **有契约服务**：宿主从契约载入方法集到注册表（ADR-0018 S1 的「方法集入注册表」原样兑现——存在性响亮 / schema 挂点 / 自省目录数据源；数据来源 = 契约，不是提供方运行时注册）。
- **contractless 服务**：无方法集，S1 的三项职能不适用——missing method 退化为 guest catch-all 返回 `business`（W2 D131 已明记的「失去形状对账」退化）。
- **不新增 ADR**：S1 的语义在本读法下完整存活；若将来收紧为「注册表不存方法集」，才需新 ADR 局部 supersede 该条款。

### W9.7 A9 D25.4 原位：required 依赖图判环

- **图** = required inject 边（optional 不参与，造不出激活死锁）；节点 = **同作用域内**已启用 ∪ Pending ∪ 本次启用者（instance-model D68）。
- **判定时机** = 插件进入启用流程（含转入 Pending）时；成环 = 本次启用者 Failed，报错列出环上全部插件；环上已 Pending 者继续 Pending，提供方转 Failed 后 warn 机器接管（W9.4）。
- **环不给 Pending**：环 = 结构性配置错误，不存在「等它自己好」的路径。
- **跨层边恒向上**（实例 → parent 链），跨层结构性无环（D68）。
- **运行期 wait-for 图** = W4.4 D96 层三原位，不属本章；本章只判 manifest required 图。
- **成本**：判环复用活图 / 从本次启用者 DFS，不每次全图重建；图维护与复杂度归 T10.7。
- **`metis check` 部署根预演** = config-format C9.1 D52（节点 = 树内 `enabled` 插件；休眠 / 未装配不入图；悬空边不参与判环）。
- **零仪式插件**：无 manifest = 无 inject 边，不入图（W1.8）。

### W9.8 W8 D174 机械化：events publish 出处对账

- **声明面** = manifest `events.publish` / `listen`（W2 D131）；语义 = W8 D173–D179，本章只机械化。
- **publish 出处**：每个名字在启用期解析到事件形状总表（三来源契约 `events:` 逻辑并集，W8 D175）：
  - 索引中存在但来源插件未启用 → Pending 等启用（warn 同 W9.4；W8 D174 / A8 D22.3 原位）；
  - 索引查无 → 启用期响亮，写清缺哪个名字、可由谁提供；
  - 定义写 `any` = 合法（宽形状）。
- **`internal/*`**：`events.publish` 含 `internal/*` = 启用期响亮（插件发不出该名字，W8 D174 / D178）。
- **`events.listen` 不判定**：任何名合法（含未定义、`internal/*`）；listen ≠ 依赖（config-format C2.8）。
- **不扫源码**：不做 emit↔publish / on↔listen 漂移对账（W8 D174；D28.3 #4–#7 死亡）。
- **零仪式插件**：无声明面；可 emit contractless 名、可 listen；publish 出处不判（无声明）。

### W9.9 A9 D26 原位：能力门执行语义（WIT 形态）

- **载体**：固定 world 下所有 import 恒链接（W3 D143）；门在宿主 import 实现内 **逐调用 fail-closed**（W3 D154；A9 D26.1 平移）。
- **判定**：manifest `capabilities` 是唯一授权声明（W2 D129）；未声明 / 越界调用 = trap（W5 D139）——宿主拒绝、实例 Failed、journal `vm_trap`；依赖方收 `service-unavailable`。
- **门表** = W3 D154 原位（`fs` global / `http` / `process` / `instance.mount` 有门；private fs 无 metis 门；timer / events / services / blobs 无门；streams 随属主）。本章不转抄。
- **`mount`** = 编排面第一个例外（当前唯一；instance-model D63）；`events` / `services` 无能力门（A9 D26.2：能力间接化 = 服务存在的意义）。
- **两界不混**（W5 D139）：manifest 缺省 = trap；manifest 声明但部署数据拒绝（http host 白名单 / SSRF / 跨域重定向）= `http-policy` Err。
- **审批面三栏**（A9 D26.2 原位）：capabilities（直接包络）+ inject（间接包络）+ events listen（信息流入面）并示；`events.publish` 出流另列。展示实现归 Creator / 审批文档，本章只定 ABI 侧数据来源。
- **无静态能力使用检查、无编译期 world 裁剪**（W3 D154）；`metis check` 只做 manifest / 词汇表预检，不推断能力使用（W9.2）。

### W9.10 D184 启用期失败的聚合报告

**D184 定案：启用事务内按固定顺序收集该插件全部响亮项，聚合为一组结构化 findings（CLI 边界 miette 呈现）；不做部分激活；Pending 项不阻塞其余格判定。**

- **判定顺序**（诊断顺序，不改变结果）：结构记录（W9.1）→ 契约引用 / 形状对账（W9.5）→ events publish 出处（W9.8）→ inject 三分（W9.4）→ provides key 撞名（W9.5）→ required 判环（W9.7）。setup 成功后的 provide 履行对账单独成步。
- **响亮项收集完毕后**：任一项存在 = 该插件 Failed，实例不启动；loader 产出结构化 findings（缺哪个 key / 哪个契约 / 哪个事件名、对手方、环路径），CLI 边界用 miette 三段式呈现，help 指路 manifest 字段或契约包。
- **journal**：`reconcile_failed` 一条携带完整清单（J1.4 D78 原位）；不新增 kind。
- **Pending 项**照常记录为待满足依赖；若同批另有响亮项，插件 Failed（响亮优先）。修复后重新启用时按当时的提供方状态重新判定，不依赖本次判定登记的保留；warn 按 W9.4 发。
- **原子性**：启用事务不产生「部分格通过、部分格失败」的半激活实例；`setup` 不运行。
- `metis check` 复用同一收集器与顺序（W9.11）。

### W9.11 D185 `metis check` 的五格范围

**D185 定案：单插件模式跑自完备项；部署根模式跑全五格（④ required 判环按 D52 的静态预演形态）；跨实体项在单插件模式明示跳过。**

- **单插件模式**（无部署根上下文）：manifest / 契约语法与未知字段（W2 D133 / W7 D164）；组件类型解析、导出集匹配、import 词汇表（W9.1–W9.3）；随包契约内部形状（`types!()` 引用可解析、`bytes` 禁入 config 等，W7）。跳过并明示「需部署根」：inject 提供方、publish 出处、provides 撞名、required 判环；provides 契约来源解析中，部署 `lib/` / 他人随包半格跳过（平台内嵌 / 自身随包仍判）。
- **部署根模式**：上述全部 + 五格（④ required 判环按 C9.1 D52 的静态预演形态）；节点 = 树内 `enabled` 插件；休眠 / 未装配不入图。
- **同一实现**：`metis check` / `metis build` / loader 安装期解析共用校验器（W2 D133 三面暴露）；语义判定代码与 loader 启用期同一实现（A9 D24「检查只在启用期发生」的静态预演口径）。
- **不扫 guest 源码**；不做能力漂移推断（W9.2）。

### W9.12 A9 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 [ADR-0026](../decisions/0026-luau-abi.md)（已 superseded）。

| 机制                                                             | 死因                                                                                                                                                            |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 盒作为独立范畴 / `boxes` 字段 / `ctx.box` 访问器                 | 盒概念消解进插件本体（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §2）；进口对账改形为全插件词汇表闭包（W9.2）                                          |
| 盒进口对账（A9 D25.5②）                                          | 改形 = 「盒进口 ⊆ manifest 能力声明」→「组件 import 清单 ⊆ 宿主词汇表」，时机改安装期结构事实（W9.2 / B1）                                                      |
| 盒双向对账（A9 D25.5①）                                          | 由组件文件校验承接（W1.1 D116：唯一 `.wasm`、目标存在、组件 magic）                                                                                             |
| 方法集注册与「注册方法集 ⊆ 契约」对账（A9 D25.2 半）             | wasm 化后无提供方侧方法集注册面；方法集 = 契约数据（W9.6 / W3 D147 / W7 D167）                                                                                  |
| analyze 静态能力漂移 / scope 键↔命名空间 / 盒对账（D28.3 #1–#3） | wasm 无源码扫描面；词汇表闭包 + 运行期门取代（W9.2 / W9.9）                                                                                                     |
| 运行期「出现规则」响亮指路 manifest（A3.2）                      | 未声明能力调用 = trap（W5 D139；W3 D154）                                                                                                                       |
| 纯代码插件无能力门资格 / 无 provide-inject（A1.3）               | 零仪式插件（W1.8 D123）：无 manifest = 无声明面，效应 fail-closed；不是「无资格」，是「无声明位」                                                               |
| 安装期 / 加载期对账措辞（A9 D24.2 平移清单七处）                 | 两段时机（W9.1 D180）；W1.3 的「安装期」按「检出并记录」读                                                                                                      |
| 事件面四项漂移对账（D28.3 #4–#7）                                | W8 D174 已淘汰；W9.8 只机械化 publish 出处两分                                                                                                                  |
| 「能力对账 = import → capability 推断」                          | 固定 world 恒链接，import 不能证明能力使用；词汇表闭包 only（W9.2）                                                                                             |
| `abi_rev` / 能力词汇 `fs` scope map / `sqlite` / 盒流式扩展位    | 非本章新增死亡：`abi` = W2 D132 / W10；fs 两档 = W2 D129 / [ADR-0039](../decisions/0039-fs-capability-two-tier.md)；sqlite 撤回 = ADR-0039；大负载 = W6（注明） |

**语义平移保留**：对账时机两段（安装期结构事实 / 启用期跨实体）；inject 三分与 warn 政策；provides 契约两分、`@N`、形状对账；撞名同作用域；required 判环；events publish 出处两分；能力门 fail-closed；编排面无门 + `mount` 例外；审批三栏；声明 ≠ 观察；零仪式插件无声明面。

### W9.13 联动登记汇总（章末）

| 去向              | 内容                                                                                                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W1（T13-h）       | **已兑现（2026-10-19）**：world 导出集匹配与结构记录 = W9.1；W1.1 / W1.3 的「安装错误」按「检出并记录」读（W9.1）；零仪式无声明面 = W9.4 / W9.5 / W9.7 / W9.8 的空集情形 |
| W2（T13-i）       | **已兑现（2026-10-19）**：`capabilities` = 唯一授权；inject / provides / events 对账形态；休眠件 / 未装配件政策；不新增字段                                              |
| W3（T13-j）       | **已兑现（2026-10-19）**：进口对账粒度 = 接口（D181）；能力门表原位（D154）；`services` / `events` 无门；`mount` 例外；WASI 基线三族（D182）                             |
| W5（T13-k）       | **已兑现（2026-10-19）**：未声明能力 = trap；形状违约 = `core-contract-violation`；manifest 缺省 vs 部署拒绝两界不混（D139）                                             |
| W6（T13-l）       | **已兑现（2026-10-19）**：词汇表 += `blobs` / `streams` / `payloads`；blob 无门；reader / writer 随属主（W3 D154）                                                       |
| W7（T13-m）       | **已兑现（2026-10-19）**：契约引用两分、`@N`、规范化形状、生命周期；方法集 = 契约数据（D183）                                                                            |
| W8（T13-n）       | **已兑现（2026-10-19）**：publish 出处两分、`internal/*` 规则、listen 不判；本章机械化（W9.8）                                                                           |
| W4（T13-c）       | **原位**：卸载期新注册 = trap（W4.5）；wait-for 图（W4.4 D96）不属本章；对账判定与投递时序解耦                                                                           |
| W10（T13-p）      | **已兑现（2026-10-19）= W10 D186–D193**：package version 比较（D187）；词汇表演进（D188）与多版本共存（D189）；WASI 基线不随 package version（D191）                     |
| T10.3             | 契约解析 / 规范化比较 / 引用展开 / 反向索引                                                                                                                              |
| T10.7             | 启用管线五格判定；Pending 唤醒；撞名；判环；provide 履行对账                                                                                                             |
| T10.9             | 部署索引；组件类型解析与 import / export 清单记录；D52 判环预演                                                                                                          |
| T10.10            | 宿主 import 注册；WASI 0.3 基线注册集（逐条清单 SSOT）；词汇表来源；trap 落点                                                                                            |
| T10.11            | 能力门实现；`http-policy` Err；未声明调用 trap                                                                                                                           |
| T10.12            | 安装事务；结构记录；休眠 / 未装配件；world 匹配                                                                                                                          |
| T10.15            | `metis check` 单插件 / 部署根模式；metis 侧词汇表（SDK 版本）；预检共用实现                                                                                              |
| config-format     | D34 休眠 / 未装配 / 残迹；D52 判环预演；C2.8 listen ≠ 依赖；C7.2 残迹指路                                                                                                |
| instance-model    | D63 mount 标准启用管线；D68 可见集 / 撞名 / 判环作用面                                                                                                                   |
| service-registry  | S2 原子注册与 provide 履行；S3 三分 / optional 重启；S4 恢复；S1 方法集按契约数据兑现                                                                                    |
| seam-capabilities | §6.1 W9 行兑现：进口对账机械化 + 运行期 fail-closed；编排面无门 + `mount` 例外                                                                                           |
| journal           | `pending_warn` / `reconcile_failed` / `vm_trap` 原位；不新增 kind（J1.4 D78）                                                                                            |
| plugin-sdk        | 能力声明消费；`deps()` / `Ctx::service`；无编译期 world 裁剪（S6 扩展位不动）                                                                                            |
| build-pipeline    | `metis check` 命令面（D106）；预检共用实现；WASI 0.3 target 预检（D112）                                                                                                 |
| 实现期配置        | 词汇表版本映射；WASI 0.3 基线逐条记录；对账失败清单条数上限                                                                                                              |
| 扩展位            | 编译期组合 world；能力使用的静态证明（需新声明面）；跨作用域可见性清单                                                                                                   |
| ADR               | 不新增；A9 为归档设计文档（非 ADR）；ADR-0018 S1「方法集入注册表」按「宿主从契约载入」原样兑现（D183），S2 / S3 机制不变；兑现记录住本章联动行                           |

## W10 工具形态与 ABI 版本纪律

### W10.0 边界与前提

本章定：ABI 面清单（什么算 ABI）；版本载体与接受条件；演进与 bump 纪律；快照布局与多版本共存；首个冻结快照；版本面各自独立；钉版纪律；工具面残留与判定落点；A10 机制消亡清单。不归本章：命令面形态 = [build-pipeline](build-pipeline.md) D105/D106/D112；WIT 接口形状 = W1/W3/W6；manifest 字段形状 = W2；按版本的路由与绑定生成实现 = T10.10；TLV 位分配与格式版本数值 = T10.13；journal 格式 = [journal](journal.md) J4.2；契约版本 `@N` = W7 D165；SDK 作者面 = [plugin-sdk](plugin-sdk.md)；marketplace 分发与 semver 约束 = 任务 9。

前提（已定不议）：WIT package version 承接 `abi_rev` 纪律（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §3）；package 名与住处 = W1.2 D117；manifest `abi` 可选声明 = W2.8 D132；声明 ≠ 观察（W2.9 D133）；安装期结构事实 / 启用期跨实体两分（W9.1 D180）；源码扫描面不存在（W9.2 B1 口径）；工具链钉版 = ADR-0034 风险 W3。A10（[luau-abi](luau-abi.md) §A10，归档）为语义平移源：D27 组（版本纪律）在本章重做，D28 组（工具面）大半已由 T13-d / T13-e 消费。

拍板（2026-10-19，维护者全按推荐）：八项形态决策 = D186–D193；ADR 处置 = 不新开 ADR（原则已由 ADR-0034 §3 冻结，本章 = 细化），只给 [ADR-0025](../decisions/0025-seam-capabilities.md) + 五篇既有 ADR（0022 / 0024 / 0027 / 0028 / 0029）加 Status 局部注。版本纪律的冻结原则面 = ADR-0034 §3；本章 = living 细化——若将来需要外部承诺的不可变面，另开 ADR。

| 节     | 议题                         | 状态               |
| ------ | ---------------------------- | ------------------ |
| W10.1  | ABI 面清单                   | ✅ D186 2026-10-19 |
| W10.2  | 版本载体与接受条件           | ✅ D187 2026-10-19 |
| W10.3  | 演进与 bump 纪律             | ✅ D188 2026-10-19 |
| W10.4  | 快照布局与多版本共存         | ✅ D189 2026-10-19 |
| W10.5  | 首个冻结快照                 | ✅ D190 2026-10-19 |
| W10.6  | 版本面各自独立               | ✅ D191 2026-10-19 |
| W10.7  | 钉版纪律                     | ✅ D192 2026-10-19 |
| W10.8  | 工具面残留与判定落点         | ✅ D193 2026-10-19 |
| W10.9  | A10 机制消亡清单（考古索引） | ✅ 2026-10-19      |
| W10.10 | 联动登记汇总（章末）         | ✅ 2026-10-19      |

### W10.1 D186 ABI 面清单

**D186 定案：ABI 判据 = 跨边界且写进文档承诺的语义；两面 = 宿主↔插件接口面、字节面。**

- **宿主↔插件接口面**：固定 world 的 import / export 接口集与签名（WIT 文本，W1/W3）；`error-kind` 封闭枚举（W5）；事件名语义（`internal/*` 保留与命名纪律，W8）；可观测投递语义（每监听者收件序、timer 合并计数、邮箱满不丢，W3 D152；这些是跨边界承诺，变更按 D188 走）；能力词汇表（W2 D129）；manifest 字段集与未知字段政策（W2）；契约形状 DSL 的词汇 / 产生式与契约节结构（W7）。
- **字节面**：TLV canonical 编码语义与格式版本（W6 D156）；tag 位分配、版本号数值与 varint 细节 = T10.13 实现期钉定，冻结时点见 W10.5。
- **不进 ABI**：确定性面（调度规则 / 在飞预算 = 实现期配置，W4.3；A10 D27.1 的确定性面已随 replay 不立缩没，W4.6）；journal 落盘格式（自带头行版本，J4.2 D90）；契约包数据版本 `@N`（W7 D165）；SDK crate semver（诊断面，plugin-sdk S3 / build-pipeline D112）；CLI 输出与 `--json` 事件；`metis.build` section 与快照 hash（诊断 / 溯源，非信任锚）；组件 custom section；WASI 0.3 基线版本（不是 metis ABI 版本，但是准入词汇表的一部分；版本面归宿主工具链，W9.3 / D191）；一切实现内部。
- 判据取 A10 D27.1 的语义，但确定性面不承接（W4.6：replay 不立 + guest 内调度宿主不可冻结）；「跨边界」的边界在 wasm 化后 = 固定 world + 组件文件边界，不再有源码 / 运行时两重边界。

### W10.2 D187 版本载体与接受条件

**D187 定案：唯一接口面 ABI 版本载体 = WIT package version（`metis:plugin@MAJOR.MINOR.PATCH`）；manifest `abi` = 精确版本声明；接受 = 声明 ∈ 宿主支持快照集 + 结构匹配该快照 + import 清单 ⊆ 该版本词汇表。字节面版本 = TLV 格式版本，独立判定（D191 / W6 D156）。**

- **载体**：package 名 `metis:plugin`，版本串住 WIT 包声明与快照目录名（D189）；manifest `abi` 是可选的精确版本字符串（W2.8 D132），不是最低版本、不是范围。对常规组件（import 名带版本）声明是冗余的显式意图（审批面 / 预检面）；对无 metis import 的极简组件，它是唯一路由信号。
- **接受条件三合一**（结构缺陷按 W9.1 两分：安装期检出并记录，被启用时以记录原因响亮；直接安装并启用时即响亮）：
  1. 声明版本 ∈ 宿主支持快照集（枚举集合，见下）；
  2. 组件结构匹配该快照：导出集齐全；import 名字中的 `metis:plugin/...@X.Y.Z` 后缀可读出时，读出的版本是该快照版本（world 顶层导出函数名不带版本后缀，导出集按该快照的 world 形状核对）；签名级匹配由组件模型在链接 / 实例化时校验（类型不符 = 安装 / 实例化错误，不靠字符串比较）；
  3. 组件 import 清单 ⊆ 该快照的词汇表清单（metis 接口 + 冻结时记下的 WASI 基线版本集；W9.2 D181；清单住处 = D189）。
- **声明 ≠ 观察**（D133）：`abi` 是作者意图；结构可读出契约版本时，读出的版本是事实——与声明不一致 = 响亮，不按任何一方静默胜出。组件不含 metis import、读不出契约版本时，声明是唯一路由信号（明记代价：这是「无可观察的版本标记」，不是静默放行；导出集与类型级校验仍然执行）；审批 / 诊断面必须区分「声明值」与「结构验证通过」，此类组件标「仅声明 / 未验证」。
- **支持集** = 已冻结快照 + 当前快照的**枚举集合**（v1 = `{0.1.0}`）；创建新快照时，被取代的旧快照必须补写 `wit.sha256`（D189），否则支持集出现空洞；`floor` / `current` 只作诊断描述，不是区间判定（`0.1.5` 不在集合内即拒绝，尽管它在数值区间内）。无 sunset（集合只增不减）。W1.2 的「声明版本 ≤ 宿主支持版本」按「∈ 集合」读。
- **有 manifest 但无 `abi`**：与零仪式同规则——结构匹配**当前**快照；要路由到旧快照必须写 `abi`（W2.8 D132）。
- **两个方向**：旧组件（**带 `abi` 声明**）+ 新宿主 = 接受（旧快照在支持集内）；无声明的零仪式包只匹配当前快照（W1.8）；新组件 + 旧宿主 = 拒绝，报错给「声明版本 / 支持集 / 升级核心或按 X 重建」。
- **严格格式**：只收 `MAJOR.MINOR.PATCH`；范围、caret、通配、预发布、build metadata 一律 manifest 校验响亮拒绝（三面措辞一致，D133）。
- **零仪式包**（W1.8 D123）：无声明面，只按结构匹配**当前**快照；旧版本必须加 manifest `abi` 才能被路由——声明面同时是审批面（部署者看得见「这个包要求哪版 ABI」）。
- **min 语义不复活**：A10 D27.2 的「最低 rev」是 Luau 宿主「源码插件按当前规则跑」的产物；wasm 组件按所对版本路由，声明必须精确，否则「按哪份绑定链接」无解。

### W10.3 D188 演进与 bump 纪律

**D188 定案：任何 ABI 可见改动 = 新 package version + 新快照；0.x 窗口内一律 MINOR bump，1.0.0 后 additive → MINOR、破坏式 → MAJOR；PATCH 位保留不使用——WIT 文本不动 = 不 bump。**

- **快照不可变**：已冻结快照（D190）不得原地修改；改动一律新版本目录。
- **分类**：
  - **additive**（旧插件二进制照跑；用新东西的插件需要新宿主）：新增 import 接口 / 新增导出 / 新增可选 manifest 字段 / 新增契约 DSL 词汇或产生式 / 新增能力词。
  - **破坏式**（旧插件源码不改不能重编译；旧二进制仍因快照保留而可跑）：删除或改名接口 / 导出 / manifest 字段；改签名或字段形状；删或增 `error-kind` case（封闭 enum 的穷尽匹配被打破 = 源码面破坏，W5.2）；改既有名字的语义（含事件名语义、契约形状语义）。
- **版本号规则**：0.x 窗口 = 任何 ABI 可见改动 bump MINOR（`0.(y+1).0`）；1.0.0 后 additive = MINOR、破坏式 = MAJOR。**PATCH 位保留但不使用**：WIT 文本不动 = 版本不动（不 bump）；一旦改动包含 package version 串即 = 新快照，走 MINOR / MAJOR。实现 / 文档 / 诊断改动不产生新版本。1.0.0 时点不在本章承诺（维护者拍板）。
- **二进制兼容 vs 源码兼容**：v1 无 sunset，旧快照恒保留，因此破坏式变更不打断已安装的旧二进制；它只意味着「按新快照重建源码要改」。这条把 semver 的「破坏」从运行期事故降为编译期迁移成本。
- **不 bump 的情形**：工具链 / 实现升级不改 WIT 文本与 TLV 布局（D192）；TLV 布局变更走 TLV 格式版本（D191），不自动 bump package version；契约 `@N` 与 SDK semver 各自独立（D191）。
- **新 `error-kind`**：属 ABI 可见且源码面破坏式（封闭枚举），按本条开新快照；宿主按组件声明版本使用对应绑定，不把新 kind 送进旧绑定（W5.3 D138）。
- **宿主侧错误投影**：旧快照的 `error-kind` 是封闭枚举；宿主为旧快照服务时，内部新失败必须投影到该快照已有的 kind——按失败族选最近的既有格（如 fs 归 `fs-io`、http 归 `http-error`、service 归 `service-unavailable`），无法归类才落 `core-contract-violation`；投影表随该快照的绑定实现。这不是跨版本协议翻译（组件之间不互译），是旧绑定的封闭枚举对宿主实现的约束；代价 = 旧插件可能拿到较粗的 kind，长期解法 = sunset。
- **SDK 投影选项**：若作者面把 `ErrorKind` 改成 `#[non_exhaustive]`，新增 kind 可重分类为 additive（源码面不再穷尽匹配）；这需要 SDK 面决策（plugin-sdk S5），本章按现 D102 封闭枚举分类为源码面破坏式。
- **sunset（提高 floor / 删旧快照）** = 扩展位，v1 不立（A10 扩展位平移）。

### W10.4 D189 快照布局与多版本共存

**D189 定案：WIT 快照 = `crates/metis-plugin/wit/<version>/`，当前版本 = 最新快照；guest 只对当前快照生成绑定，宿主对全部支持快照生成；不立跨版本翻译层。**

- **布局**：W1.2 的 `crates/metis-plugin/wit/` 为根；每个支持版本一个子目录 `wit/<MAJOR.MINOR.PATCH>/`，目录内 `package metis:plugin@<version>;`，目录名与包声明版本必须一致（测试校验），`CURRENT_ABI` 必须等于最新目录名（测试校验）。同目录另落 `vocabulary.yml` = 该快照的进口对账清单（metis 接口清单 + 冻结时的 WASI 基线版本集），**冻结前**由 T10.10 维护、冻结后只读并纳入 hash；它不是 WIT 文本，不参与绑定生成。`0.1.0` 为当前唯一快照；不另存「current 副本」。
- **生成面**：guest 侧 SDK 只对 `CURRENT_ABI` 生成（作者永远只见当前）；宿主 `metis-component` 对支持集内全部快照生成，按版本各一份绑定。两组绑定同出 `wit/` 树，禁止拷贝（W1.2 D117）。
- **路由**：按 D187 的结构版本 + 声明版本选快照；不立跨版本协议翻译 / 垫片（组件之间不互译）；宿主为旧快照服务时按 D188 的投影规则产生该快照内的 kind。每个快照各有一份 `vocabulary.yml`，进口对账按所选快照判（W9.2）。
- **零仪式**：只匹配当前快照（D187；W1.8）。
- **实现核对项（未验证）**：同一宿主 crate 内多版本同名 package 的 `bindgen!` 共存形态（`with` 映射 vs 每版本独立模块）归 T10.10；设计层只冻结「逐版本绑定 + 无协议翻译层」。
- **快照 hash**：冻结时在目录内写 `wit.sha256`（排序后的相对路径 + 内容的规范化摘要；**只排除 `wit.sha256` 自身**；`vocabulary.yml` 与 WIT 树一并纳入）。未冻结快照不写；被取代时补写（D187）。用途 = 冻结不可变性的测试锚 + 诊断身份（脚手架与 `metis.build.wit_sha256` 记录，D108 / D112），不是信任锚，也不参与 ABI 判定。

### W10.5 D190 首个冻结快照

**D190 定案：`0.1.0` 的冻结触发 = 首个外部分发（marketplace 收录或非 monorepo 作者发布 / 分发）之前；冻结物 = D186 的两面全集；冻结后目录不可变。**

- **触发与责任人**：执行者 = 维护者（发布前检查单）；触发观测 = marketplace 收录 / 外部分发流程（任务 9）；「外部分发」含 `metis-plugin` crate 发布（SDK 随 crate 携带 WIT 树，W1.2）。核心不自动检测外部事件，唯一状态面 = 台账登记。**冻结前置 = 冻结物全部在场**（WIT 树 / TLV 位布局 / manifest·能力·DSL·事件规则 / `vocabulary.yml`）；任一不在场 = 冻结不成立，外部分发阻塞。若外部分发已发生而未冻结 = 发布事故：立即补冻结，`0.1.0` 不得原地改，后续改动开新快照。
- **冻结物清单**（= D186 接口面 + 字节面全集）：
  - `wit/0.1.0/` 的 WIT 文本树（world + 各 interface）；
  - TLV 格式版本与位布局（T10.13 的交付；W6 D156 的语义面已冻结）；
  - `error-kind` 封闭枚举（W5 D137）；
  - manifest 字段集与未知字段政策（W2 D126）；
  - 能力词汇表（W2 D129）；
  - 契约形状 DSL 词汇 / 产生式与契约节结构（W7 D166 / D167）；
  - 事件名规则与 `internal/*` 保留（W8 D173）；
  - 可观测投递语义（每监听者收件序、timer 合并计数、邮箱满不丢，W3 D152）；
  - `vocabulary.yml` 的 WASI 基线版本集（D189）。
- **机制**：冻结事件 = 写入 `wit/0.1.0/wit.sha256`（排除自身；覆盖 WIT 树 + `vocabulary.yml`）+ 本章与台账各记一笔（日期 + hash + 冻结 commit）；此后目录只读，任何改动开新快照（D188）。非 WIT 冻结项（TLV 位布局 / manifest·能力·DSL·事件规则）不由该 hash 钉：TLV 由 T10.13 的格式版本与位布局常量钉，其余项由本章 D 号 + 同一冻结 commit 的评审记录钉（无法机器化，明记）。
- **校验**：hash 测试（重算当前目录与 `wit.sha256` 比对）归 T10.14 的实现义务，测试形态，不动门禁面。它防失误，不防同一 commit 里同时改目录与 hash 的蓄意改写；后者靠评审纪律与 git 史（`git diff <冻结 commit> -- wit/0.1.0`）。若需要机器强制冻结目录的 git 史 = 门禁面变更（ASK）登记为扩展位，v1 不做。
- **现状（2026-10-19 本章收官）**：`0.1.0` = 当前快照、**尚未冻结**；冻结为待触发项，在首个外部分发前执行。W1.3 / W3.13 / W6.2 / W6.6 的「首个发布快照由 W10 记录」以此为准。

### W10.6 D191 版本面各自独立

**D191 定案：六个版本面互不同步、各 bump 各的；不立统一版本号；只有 WIT package version 参与 ABI 接受判定。**

| 面               | 载体                           | 管什么         | 演进 / 归属                          |
| ---------------- | ------------------------------ | -------------- | ------------------------------------ |
| WIT package      | `metis:plugin@X.Y.Z`           | ABI 接口集     | 本章 D187 / D188                     |
| TLV 格式         | 载荷内版本字段 + T10.13 位分配 | Value 字节布局 | T10.13；与 WIT 版本独立（W6 D156）   |
| journal 格式     | 段文件头行 `{"v":1,...}`       | 落盘格式       | J4.2 D90；独立                       |
| 契约版本         | `@N`                           | 契约数据形状   | W7 D165；独立                        |
| SDK crate semver | `metis-plugin` 版本            | 作者面 API     | plugin-sdk S3；诊断面，不作 ABI 判定 |
| 构建管线版本     | `metis.build.pipeline_version` | 构建溯源       | build-pipeline D112；独立            |

- WASI 0.3 基线版本是宿主词汇表事实，不是 metis ABI 版本（W9.3）；每个快照的 `vocabulary.yml` 记录冻结时的基线版本集（D189），进口对账按它判（D187）。宿主升级若删除旧 WASI 版本 = 对旧插件的事实性破坏；D192 的升级流程必须在升级前对比全部快照的 `vocabulary.yml`，保留旧版本或显式走 sunset 决策（扩展位）。metis package version 的「无 sunset」承诺与 WASI 基线的保留是**两个条件**，同时满足才等于「旧插件照跑」；本章不把后者包装进前者。
- 不互相推导：契约 `@N` bump 不 bump ABI；ABI bump 不自动改契约版本；TLV 版本 bump 不改 WIT 文本。

### W10.7 D192 钉版纪律

**D192 定案：版本号只住消费它的清单；钉版面 = rustc / target / wit-bindgen / wasmtime / wasm-tools / wasmparser / wasm-component-ld；升级 = 一次 lockstep 改动，WIT 文本不变则不 bump ABI。**

- **SSOT**：工具链版本住 `rust-toolchain.toml`；crate 版本住 Cargo 清单（workspace 依赖）；设计文档不复制数值（AGENTS.md「NEVER hand-write version numbers」）。本章只冻结纪律，不记录版本号。
- **钉版面**：
  - rustc + `wasm32-wasip3` target：guest 构建与宿主构建同一工具链版本（WASI 0.3，ADR-0034 W7 风险原位）；
  - `wit-bindgen`：SDK 私有钉版（plugin-sdk S3）；作者不见；宿主侧不直接钉它——`wasmtime::component::bindgen!` 经 wasmtime 间接携带，版本由 wasmtime 钉定（构建期生成，产物为组件；两侧不需要同一 wit-bindgen 版本，ABI 由 canonical ABI + WIT 文本保证）；
  - `wasmtime`：宿主嵌入；
  - `wasm-tools`：只作 WIT 提取 / 调试，版本钉定，**不参与安装判定**（build-pipeline D108）；
  - `wasmparser`：校验 SSOT（固定 feature 集合，D108）；
  - `wasm-component-ld`：组件链接，预检在场（D112）。
- **升级流程**：一次 lockstep 改动（清单 + 工具链）→ `just ci` + `just deny`；WIT 文本与 TLV 布局不变 = 不 bump（D188）；工具链迫使 WIT 文本变化 = 按 D188 正常 bump。
- **诊断身份**：WIT hash（D189，脚手架记录）与 `metis.build`（D112，记录 `wit_package_version` / `wit_sha256`）只作诊断 / 溯源，不是信任锚；签名 / attestation 单独绑定产物。
- **组件模型 1.0**：复审触发器原位（ADR-0034 W3）；到来时评估升级路径，不在本章预裁。

### W10.8 D193 工具面残留与判定落点

**D193 定案：命令面 SSOT = build-pipeline D105/D106/D112；W10 只定 ABI 判定落点 = 单一实现、两层执行（声明层 / 结构层）。**

- **不保留 `metis sdk` 组**。A10 D28.1 的映射：

| A10 子命令    | 去向                                                               |
| ------------- | ------------------------------------------------------------------ |
| `sdk new`     | `metis new`（build-pipeline D106）；写 SDK 版本 + WIT hash（D108） |
| `sdk check`   | `metis check`（D106）；ABI 判定并入                                |
| `sdk analyze` | 消亡：wasm 无源码扫描面（W9.2 / W8 D174）                          |
| `sdk types`   | 编译期 `types!()`（plugin-sdk D104 / W7 D170）                     |
| `sdk box *`   | 随盒消亡（ADR-0034 §2）                                            |

- **判定落点 = 单一实现、两层执行**：
  - 声明层（manifest `abi` 格式 + ∈ 支持集）：`metis check`（纯数据）、`metis build` 预检（D112）、loader 安装期都跑；
  - 结构层（组件导出集 + import 版本后缀 + import 词汇表 + **组件类型与期望 world 的类型级比较**）：只在有组件产物的面跑——`metis build` 产物校验、安装期、部署根模式的 `metis check`；类型级比较不需要实例化（解析组件类型即可），实例化时的运行时类型检查 = 启用期最后一道。源码模式的 `metis check` 明示「需构建产物」跳过。
  - 两层同源实现、措辞逐字一致（D133 原则）；`metis check` / `metis build` / loader 共用校验器。
- **错误形态**：结构化 findings（声明版本 / 支持集 / 结构实际版本 / import 越界条目 / 行动提示），CLI 边界 miette 三段式（code + 说明 + help，如「升级核心或按 0.1.0 重建」）。
- **`metis check` 的单插件模式**：声明层自完备；结构层需组件产物（源码模式跳过并明示），与 W9.11 的部署级格区分。
- **A10 D28.3 十四项检查的存亡**：见 W10.9；不扫 guest 源码、不做能力漂移推断（W9.2 原位）。
- **实现位置**：判定住共享纯校验面（build-pipeline D108 的 `metis-validate` 或 `metis-component`，T10.15 开工确认）；loader 与构建管线同源消费；运行时 crate 不依赖构建管线。

### W10.9 A10 机制消亡清单（考古索引）

每条一句死因；完整考古见 git 史、归档 [luau-abi](luau-abi.md) 与 [ADR-0026](../decisions/0026-luau-abi.md)（已 superseded）。

| 机制                                                                    | 死因                                                                              |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `abi_rev` 单整数 / min 语义                                             | 载体换 WIT package version；声明必须精确（D187 / D188）                           |
| A10 的「四层防线」工具链                                                | 换 D133 三面暴露 + wasm 结构事实                                                  |
| `metis sdk` 组 / `sdk box *`                                            | 命令面上移（D106）；盒消亡（ADR-0034 §2）                                         |
| `sdk analyze` + D28.3 #1–#3 / #5–#7 / #9–#14                            | wasm 无源码扫描面；能力对账 = 词汇表闭包（W9.2）；事件面声明降为审批面（W8 D174） |
| D28.3 #4 emit↔publish 漂移                                              | 收窄 = publish 出处静态检查（W9.8）                                               |
| D28.3 #8 判环预检                                                       | 存活 = config-format D52 / W9.11                                                  |
| `.d.luau` / `export type` / definitions 挂接 / luau-lsp 钉版 / AST 选型 | 换 wit-bindgen + 编译期宏（plugin-sdk S2 / S7）                                   |
| `.luaurc` / `@lib` / 部署根布线                                         | ADR-0038（模块系统消亡）                                                          |
| `plugin()` 泛型 declare / 两槽 narrowing / integer 类型器验证           | Rust 类型系统 + `Result` 原生（plugin-sdk S4 / S5）                               |
| WIT 递归 variant 验证项                                                 | 已成设计输入（递归 Value 走 TLV，W6）                                             |
| ABI rev sunset / analyze 提示 rev 写低                                  | sunset 改扩展位（D188）；analyze 消亡                                             |
| Creator 文档的 `abi_rev` 作者指南                                       | 改 `abi` 精确版本指南（T10.14 作者文档，首个外部分发前完成）                      |

**语义平移保留**：命令面 `new` / `check`（D106）；判环预检（D52）；publish 出处检查（W9.8）；三面暴露（D133）；快照保留承诺（D188）；钉版纪律（D192）。

### W10.10 联动登记汇总（章末）

| 去向                                | 内容                                                                                                                                                                                                                                             |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W1（T13-h）                         | **已兑现（2026-10-19）**：package version 语义与比较 = D187；快照布局 = D189；冻结 = D190；`wit/` 根 + 版本子目录                                                                                                                                |
| W2（T13-i）                         | **已兑现（2026-10-19）**：`abi` 精确声明与三合一接受（D187）；字段集 / 能力词汇演进 = D188；manifest 版本机制仍不立                                                                                                                              |
| W3（T13-j）                         | **已兑现（2026-10-19）**：首个冻结快照 = D190；import 词汇表按快照（D189）；WASI 基线独立（D191）                                                                                                                                                |
| W5（T13-k）                         | **已兑现（2026-10-19）**：新 kind = 新快照（D188）；跨版本按声明绑定，无翻译层（D189）                                                                                                                                                           |
| W6（T13-l）                         | **已兑现（2026-10-19）**：TLV 版本面独立（D191）；位布局冻结时点 = D190；TLV 变更不自动 bump ABI                                                                                                                                                 |
| W7（T13-m）                         | **已兑现（2026-10-19）**：DSL 词汇 / 产生式演进 = D188；`@N` 独立（D191）                                                                                                                                                                        |
| W8（T13-n）                         | **已兑现（2026-10-19）**：事件名规则 / `internal/*` 词汇演进 = D188；`on-event` 面演进 = 新快照                                                                                                                                                  |
| W9（T13-o）                         | **已兑现（2026-10-19）**：支持集与多版本词汇表（D187 / D189）；WASI 基线不随 package version（D191）                                                                                                                                             |
| ADR-0025 + 0022/0024/0027/0028/0029 | Status 行局部注：`abi_rev` 载体的历史措辞换为 WIT package version（引 ADR-0034 §3 + 本章 D187–D188）；正文全部不动                                                                                                                               |
| config-format                       | C4.2 ABI 演进归本章；C5.3 / C8.2 / C9.2 的 W10 行兑现                                                                                                                                                                                            |
| instance-model                      | S3.3 / S5.2 的 ABI 演进承接兑现                                                                                                                                                                                                                  |
| journal                             | J6.1 的 ABI 演进通道兑现；journal 版本面独立（D191）                                                                                                                                                                                             |
| plugin-sdk                          | 钉版纪律（D192）；`types!()` / 宏归属（D193）；SDK semver 诊断面（D191）                                                                                                                                                                         |
| build-pipeline                      | D108 的版本比较语义 = D187；D112 预检消费同一实现；§14 W10 行兑现；命令面 SSOT（D193）                                                                                                                                                           |
| seam-capabilities                   | §6.1 W10 行兑现（能力词汇演进 = 新快照）                                                                                                                                                                                                         |
| service-registry                    | §7 契约包版本面 = W7 D165 / W10 D191；工具链钉版纪律 = W10 D192                                                                                                                                                                                  |
| T10.9                               | ABI 比较函数消费；部署索引记录结构版本；单插件 / 部署根 check 共用                                                                                                                                                                               |
| T10.10                              | 开工首件 = `crates/metis-plugin/wit/<version>/` 目录落盘（W1.2 的 SDK crate 位置，SDK 实现归 T10.14）；多版本绑定生成（逐快照）；路由；`CURRENT_ABI` / 支持集常量；`bindgen!` 共存形态（未验证）；冻结前维护 `vocabulary.yml`（WASI 基线版本集） |
| T10.13                              | TLV 格式版本与位布局；冻结前交付                                                                                                                                                                                                                 |
| T10.14                              | SDK 只对当前快照生成；脚手架写 SDK 版本 + WIT hash；首次冻结执行 + hash 校验；`abi` 作者指南（首个外部分发前）                                                                                                                                   |
| T10.15                              | `metis check` ABI 判定；预检共用实现；命令面                                                                                                                                                                                                     |
| 扩展位                              | snapshot sunset / floor raise；1.0.0 时点；多版本 `bindgen!` 共存形态（实现核对）；预发布版本（不立，重开 = 有预发布消费方）；冻结目录 git 史机器校验（需门禁面 ASK）；快照数增长复审触发器（阈值实现期配置）                                    |
