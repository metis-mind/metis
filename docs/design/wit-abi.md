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
| W7  | 契约与类型来源（① 平台封闭面 = WIT 富类型原生；A7 半消融 = E5 拍板：瘦身形状 DSL 继承 Value 形状面）                              | A7                          | 未开始                     |
| W8  | 事件静态声明                                                                                                                      | A8                          | 未开始                     |
| W9  | deps/provide 与能力门（进口对账机械化）                                                                                           | A9                          | 未开始                     |
| W10 | 工具形态与 ABI 版本纪律（WIT package version 承接 abi_rev）                                                                       | A10                         | 未开始                     |

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

### W1.2 D117 WIT 包、world 与物理布局

**D117 定案：package = `metis:plugin@<abi-version>`，world = `plugin`，WIT SSOT = `crates/metis-plugin/wit/`。**

- **名字**：package `metis:plugin`，world `plugin`，v1 版本号 `0.1.0`。版本号语义、比较规则与兼容判定 = W10 章；本章只定名字与版本字符串的住处。
- **物理位置**：WIT 目录住 SDK crate（`crates/metis-plugin/wit/`）——SDK 是作者实际依赖、将来要按 crate 发布的单元，Cargo 只打包 crate 目录内的文件，WIT 放在里面才能随 SDK 一起走。宿主 `metis-component` 以源码路径引用同一目录生成绑定（Zed 式两视角：guest 只对当前版本生成，宿主对全部支持版本生成）；两组绑定必须来自同一目录树，禁止拷贝第二份。这是工作区内源码级路径引用，不产生 Cargo 依赖（`metis-component` 不依赖 guest 侧 SDK）；若宿主 crate 将来需要独立发布，再评估中立 WIT crate 或构建期复制。
- **文件按章拆**：规划中 `plugin.wit`（world + 生命周期导出）；后续 `error.wit`（W5）、`ctx.wit`（W3）、`events.wit`（W3 import 面 + W8 声明面）、`blob.wit` / `streams.wit` / `payload.wit`（W6）加入同一 package 目录。WIT 允许多文件同包。
- **固定 world**：单一 world，import 词汇表在一个 package version 内封闭（[plugin-sdk](plugin-sdk.md) S6 已定）；增补 import / export = 新 package version（W10）。宿主与插件不生成按能力裁剪的 world（编译期组合 world = 扩展位，前提 = W2 落地）。
- **版本快照**：v1 只有一个 ABI 版本；多版本共存形态（每个支持版本一份 WIT 目录、宿主全量生成）归 W10。旧组件与新宿主的兼容判定 = manifest 声明的 package version ≤ 宿主支持版本（比较语义 W10）。

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
    // on-event 随 W8 增补（W3 D146 已定形状：listener-id + name + payload；payload = W6 D160）
}
```

- **W3 增补（2026-10-19）**：`on-call` / `on-timer` 是非生命周期导出，随 W3 D147 / D145 加入 v1 world；`on-event` 随 W8。`0.1.0` 尚未发布，本节草图与 W3 同批补全 = 首个发布快照的编制（W10 冻结）；发布后的增补才适用「新 package version」纪律。**W6 增补（2026-10-19）**：`on-call` 的 args / 返回改 `payload`（D160）；config 仍是 TLV。

- **`setup(config)`**：拿启用期合成并冻结的完整配置（config-format D36 管线产物：校验 → 秘密解析 → 默认值合成），成功返回 = 实例可以进入 Active。返回 `error` = 加载失败（W1.5）。
- **`on-config(config)`**：配置热更新通道（config-format D42）。`Ok(true)` = live-apply 成交；`Ok(false)` / `Err` / trap = 重挂载。默认实现 = `Ok(false)`（SDK 补全），所以「没实现」与「拒绝吸收」是同一行为。
- **`dispose()`**：唯一的手动清理通道（A1.2 平移）。在飞任务结算后由宿主调用（W1.6）；可 await ctx 效应调用；错误 / trap 记日志、不中断后续清理。
- **无 ctx 参数**：ctx = 宿主 import 接口集（W3）；SDK 在作者面合成 `Ctx`，作者签名形如 `async fn setup(&mut self, ctx: &Ctx, config: Value) -> Result<(), Error>`。ABI 不重复传递宿主能力表。
- **world 导出集在单一 package version 内封闭**：组件必须导出生命周期三者与 W3 的 `on-call` / `on-timer`；缺任一 = 安装期 world 不匹配错误。作者面的「可选钩子」= trait 默认方法，不是缺失导出。`on-event` 导出（W8）在同一 world 上 additive 增补；发布后的增补 = 新 package version（W10）。
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
- 兼容判定：无 manifest 即无 package version 声明面；零仪式组件只在结构匹配宿主当前 world 时被接受（world 不匹配 = 安装错误）。ABI 版本演进后，旧零仪式包须加 manifest 声明版本（W2/W10）。
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
| W7（T13-m）    | 随包契约 `lib/`（包内容区的一部分，D35）；W1 只定 `manifest.yml` + 组件文件 + `lib/` 的包布局边界                                                                                                                |
| W8（T13-n）    | `on-event` 导出 additive 增补到同一 world（D118）                                                                                                                                                                |
| W10（T13-p）   | package version 的数值与比较、多版本快照、ABI 版本纪律（D117）                                                                                                                                                   |
| T10.10         | 实例化 / Store 限额 / fuel / epoch；加载预算与 dispose 预算的数值；`.cwasm` 缓存                                                                                                                                 |
| T10.12         | 安装期 start / world 校验；包内容复制与未知文件处置；卸载                                                                                                                                                        |
| T10.14         | SDK 默认实现（`setup` = `Ok(())`、`on-config` = `Ok(false)`、`dispose` = `Ok(())`）；作者面签名；`Ctx` 合成                                                                                                      |
| T10.15         | 可部署包布局与组件文件名（D116）；预检 start 拒绝（D122）；零仪式源码合成 manifest                                                                                                                               |
| config-format  | D42 `on_config` 的 ABI 签名已兑现（D118）；D29 `plugin.wasm` 占位已兑现（D116）；D35 零仪式 ABI 面（D123）                                                                                                       |
| build-pipeline | D106/D108 的组件文件名 / 包布局指针已兑现（D116）；D108 start 政策已确认（D122）                                                                                                                                 |
| instance-model | D63 零仪式无 mount 资格（D123 原位）                                                                                                                                                                             |
| plugin-sdk     | S2 WIT 目录确认（D117）；S1 生命周期钩子形态（D118）；`on_config` 归 W1                                                                                                                                          |
| ADR-0005       | Consequences 成本账局部取代（[ADR-0037](../decisions/0037-component-instance-cost-ledger.md)，D124）                                                                                                             |
| 实现期配置     | 加载总预算（W1.5）；`dispose` 墙钟预算（W1.6）                                                                                                                                                                   |
| 扩展位         | 编译期组合 world（S6 登记，W2 已落地后维持不启用）；多组件包（W1/W2 包布局面）；多 ABI 版本共存（W10）                                                                                                           |

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
- 不立 manifest 自身的版本号；字段集演进随 WIT package version（W10）。
- 读取时序：安装期解析 / 未知字段与类型校验 / 登记部署索引；语义对账（config 校验、秘密解析、inject / provides / events 对账、能力门）= 启用期（对账时机 = 启用期，W9 章承接；config-format §0.2 同判；W2.9 D133）。

### W2.2 D126 顶层字段全集与身份

**D126 定案：顶层字段 = 封闭集；`name` 不立；身份恒 = 目录名。**

| 字段           | 缺省          | 一句                                             |
| -------------- | ------------- | ------------------------------------------------ |
| `description`  | 无            | 审批 / 展示用文本                                |
| `version`      | 无            | 插件分发版本（semver 约束归任务 9）              |
| `abi`          | 无            | 组件构建所对的 WIT package version（比较归 W10） |
| `component`    | `plugin.wasm` | 包内组件文件（形状 = W1.1 D116）                 |
| `config`       | 无            | config schema（W2.4 D128）                       |
| `capabilities` | 无            | 能力声明（W2.5 D129）                            |
| `provides`     | 无            | 服务面（W2.7 D131）                              |
| `inject`       | 无            | 依赖面（W2.7 D131）                              |
| `events`       | 无            | 事件声明面（W2.7 D131）                          |

- **`name` 不立**：身份恒 = 目录名（config-format D31；W1.1 D116）；crate 名只决定构建输出目录 / 安装目录名。
- 不在表内的顶层字段 = 未知字段响亮；嵌套条目同理。新增字段 = WIT package version additive（W10），不另立 manifest 版本机制。
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
- **mount**：`true` = 允许调用 `ctx.mount` 创建子树实例（激活新代码的元权限；[instance-model](instance-model.md) §S3.3 D63；[ADR-0028](../decisions/0028-instance-model.md) 批次 #1）；`false` / 缺省 = 拒绝。它是编排面能力门的唯一例外。
- 声明是作者意图；运行期门 fail-closed；能力词汇 additive 增补 = WIT package version（W10）。审批面逐条展示；global 最高档。

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
    memory: {} # contract 缺省 = 服务名
    search:
        contract: search@2 # 生态契约带 @N（W7）
inject:
    required: [llm, memory]
    optional: [cache]
events:
    publish: [some/event]
    listen: [another/event]
```

- `provides` = map 服务名 → 条目；条目封闭字段 = `{ contract }`（可选字符串，缺省 = 服务名；`@N` 规则归 W7）；`methods` 字段消亡（方法集 = 契约数据）；`null` ≡ `{}`。
- `inject` = map，封闭字段 `{ required, optional }`，值为服务名列表；缺省空；required 缺失 = 启用期 Pending / 响亮（W9）；判环 = config-format C9.1 D52。
- `events` = map，封闭字段 `{ publish, listen }`，值为事件名列表；形状不写在这里，住契约 `types:` / `events:`（W7/W8）；`internal/` 规则归 W8。
- **作者自定义 schema 的住处 = 契约包**：服务方法集、业务 data 形状、事件形状由契约定义；三来源 = 平台内嵌 / 部署 `lib/` / 插件随包（包内容区 `lib/`，W1.1 D116）；格式与命名空间合并 = W7，事件语义 = W8。manifest 只引用契约名与事件名，不重复声明形状。
- 无契约也可提供私有服务 / 发事件，但失去形状对账（W9）；同名不同 schema = 启用期响亮；同名同 schema = 可替换实现（seam 三角）。
- 服务名扁平；`plugin-name/service` 前缀仅约定不强制；撞名判定 = W9。契约治理三层与契约包版本规则 = W7 原位，本章不转抄。

### W2.8 D132 `abi` / `version` / `description`

**D132 定案：`abi` = 可选 WIT package version 声明；`version` = 插件分发版本；`abi_rev` 消亡。**

- `abi` = 可选字符串，组件构建所对的 `metis:plugin` WIT package version（如 `"0.1.0"`）；比较 / 接受条件 = W10。缺省 = 无版本声明；接受条件 = 结构匹配宿主当前 world（与零仪式同规则）。
- `version` = 插件自身分发版本，可选；v1 不强制 semver（任务 9）；不是身份，与 `abi` 无关。
- `description` = 可选自由文本。
- 构建合成最小 manifest = `version: 0.1.0` + `abi: <构建时的 WIT package version>`；不含 `name`；安装目录名取 crate 名 / `dist/<name>`（[build-pipeline](build-pipeline.md) D108 同步）。
- `abi_rev` 字段消亡；ADR-0028 的「`abi_rev` +1」只作历史措辞，演进纪律由 W10 承接。
- marketplace 元数据 v1 不收；`min_core` 不立。

### W2.9 D133 校验模型与「声明 ≠ 观察」

**D133 定案：manifest 校验 = 纯数据单一实现、三面暴露；声明是意图，组件结构是事实。**

- 三面暴露：编辑器 JSON Schema 工件（yaml-language-server）→ `metis check` / `metis build` → loader 安装 / 启用期；三面措辞逐字一致（校验器单一实现、多面暴露原则；config-format C3.1 同 parser 同校验器）。
- 时机两分：安装期 = 解析 + 未知字段 / 类型校验 + 登记部署索引；启用期 = config 校验、秘密解析、inject / provides / events 对账、能力门判定。插件代码在任何校验前均未运行。
- 声明 ≠ 观察：manifest 承载作者意图（capabilities、required / default / secret、inject / provides / events）；组件 import 清单 = 结构事实；对账 = import 清单 ⊆ SDK 封闭词汇表（design-review B1），**不做**能力自动推断，也不以扫描结果反写 manifest。
- 元数据唯一去处 = manifest；组件 custom section 只作诊断 / 溯源（W1.1；build-pipeline D112）。
- 空 manifest 合法、全缺省；错误带字段路径 + typo 提示；YAML 受限子集违规响亮。
- `metis check` 部署模式另含 config-format C2.4 D34 的未装配件 / 休眠件提示，不属 manifest 校验本体。

### W2.10 D134 模块系统消亡（ADR-0003 supersede）

**D134 定案：ADR-0003 整篇 supersede = [ADR-0038](../decisions/0038-module-system-require-supersede.md)；运行期模块系统无承接物。**

- 死亡：require 包装器、`(importer, specifier)` 依赖边账本、反向传递闭包失效、插件内相对路径 import 规则、跨插件文件 require 的路径逃逸判定、共享 `lib/` 纯代码库、`@lib` 别名、`.luaurc`、模块缓存与循环 require 报错。
- 存活：跨插件运行期组合只走 inject / events；同一插件内部由 guest 编译期模块系统承担；契约变更级联 = 数据驱动反向索引（config-format D43 / W7）。
- `lib/` = 契约包（WIT / 数据），扫描进契约命名空间，不被 import 为代码（W1.1 D116；config-format D35）。
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

| 去向            | 内容                                                                                                                                                                                                                        |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1（T13-h）     | `component` 缺省 `plugin.wasm`（D116）；零仪式 ABI 面（D123）措辞按 ADR-0039 更新：private fs 隐式，效应能力仍 fail-closed                                                                                                  |
| W3（T13-j）     | ctx.fs = private（隐式读写）/ global（`fs: read/write` 声明）；无命名根；global 能力门在运行期强制；DB = guest crate + private fs（无宿主 sqlite）                                                                          |
| W5（T13-k）     | **已兑现（2026-10-19）**：生命周期返回的 `error` 类型 = W5 D136；manifest 校验错误是宿主侧错误，不经 guest 错误通道（D133）                                                                                                 |
| W7（T13-m）     | `type:` 表达式语法与词汇；契约包格式 / 三来源合并；`provides.contract` 与 `@N`；作者自定义服务 / 事件 schema 的住处                                                                                                         |
| W8（T13-n）     | 事件名语义、`internal/` 规则、事件形状在契约中的写法；manifest 只列名字                                                                                                                                                     |
| W9（T13-o）     | inject / provides / events 对账；能力门（含 `mount` 例外）；import 清单 ⊆ SDK 词汇表                                                                                                                                        |
| W10（T13-p）    | `abi` 的数值与比较；manifest 字段集 / 能力词汇 additive 演进纪律                                                                                                                                                            |
| config-format   | D36 配置管线按 `config` schema 消费；共享变量 = `{ $value: "name" }`（含 secret；C6.4 D135 / [ADR-0040](../decisions/0040-config-shared-variables.md)）；fs 两档（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)） |
| build-pipeline  | D108 合成最小 manifest = `version` + `abi`（无 `name`）；脚手架按 W2 字段集生成；组件文件名 = D116                                                                                                                          |
| plugin-sdk      | S6 能力白名单 = manifest `capabilities` 声明源；编译期组合 world 维持扩展位（W2 已落地后仍不启用）                                                                                                                          |
| instance-model  | `mount: true` 能力键与 `ctx.mount` 方法族（ADR-0028 批次 #1）                                                                                                                                                               |
| ADR-0003        | 整篇 supersede = [ADR-0038](../decisions/0038-module-system-require-supersede.md)（D134）                                                                                                                                   |
| ADR-0036        | Decision 第 2 条「命名根」局部 supersede = [ADR-0039](../decisions/0039-fs-capability-two-tier.md)（D130）                                                                                                                  |
| ADR-0025 / 0027 | Status 行记：ADR-0025 §5 fs 通道条两档 → ADR-0036 三层 → ADR-0039 两档（private 隐式 / `fs` 标量 global），sqlite 项撤回；ADR-0027 D44 与 C1.1 `data/db.sqlite` 行由 ADR-0039 定型/失效                                     |
| ADR-0040        | 配置共享变量（`values.yml` + `secrets.yml`，`{ $value }` 含 secret）= C6.4 D135；W2 D128 引用语法已同步                                                                                                                     |
| T10.12          | 安装期 manifest 解析 / 未知字段 / 类型校验；world 与 start 校验；包内容复制                                                                                                                                                 |
| T10.14          | 作者面 trait / 宏；`Value` 手动面；能力声明消费                                                                                                                                                                             |
| T10.15          | 脚手架 manifest 骨架；合成最小 manifest；构建期 manifest 校验                                                                                                                                                               |
| 实现期清单      | manifest 校验器（三面同一实现）；JSON Schema 工件生成；部署索引解析登记；共享变量解析随 T10.8（C6.4 D135）                                                                                                                  |
| 扩展位          | 嵌套字段 `secret` / 嵌套 `$value` 引用；编译期组合 world；自由 `build.command`；marketplace 元数据                                                                                                                          |

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
- **`on-event` 导出**：形状登记给 W8：`on-event(listener-id: u64, name: string, payload: payload) -> result<option<payload>, error>`，async（W6 D160）。带 listener-id 才能逐监听者保序 / 短路；W8 定事件名语义、`internal/` 规则与声明面。
- **动态名**：`listen` / `emit` 接受字符串；声明对账（manifest `events.listen/publish`）归 W8 / W9；`listen` 返回 `result` 让 W9 决定未声明名的失败载体。
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

**D154 定案：能力门 = 宿主 import 实现内逐调用 fail-closed；未声明 = trap 指路 manifest；WASI 0.3 基线是基础设施白名单；v1 world 导出集 = 生命周期三导出 + `on-call` / `on-timer`（W8 再补 `on-event`）。**

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
- **world 终态**：v1 导出集 = `setup` / `on-config` / `dispose` / `on-call` / `on-timer`（全 async），W8 再补 `on-event`。`0.1.0` 尚未发布；本批补全 = 首个发布快照的编制，不是版本演进；首个冻结快照由 W10 记录。W1.3 的签名草图同批更新。
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
| W1（T13-h）       | world 导出集补 `on-call` / `on-timer`（W1.3 签名草图同批更新；W8 再补 `on-event`）；生命周期三导出不变；`on-call` args / 返回 = `payload`（W6 D160）                                                                                                                                                                                |
| W2（T13-i）       | 消费 `capabilities.fs/http/process/mount`、`provides/inject/events`、`abi`；无 manifest 改动                                                                                                                                                                                                                                        |
| W4（T13-c）       | tick 投递走 W4.3 闸门；`emit` await 入队；投递不静默丢弃（W3.11）；W4.3「tick = 普通邮箱消息」增补为「每 timer pending + 合并计数」                                                                                                                                                                                                 |
| W5（T13-k）       | **已兑现（2026-10-19）**：所有 fallible import = `result<T, error>`；timer 无 Err 面；`fs-invalid-utf8` 保留（ABI bytes-only；SDK `read_text` 产生）；服务调用四态 + 服务路径 core 两格；mount 路径 `core-mount-failed`（mount 启用失败）；未声明能力 = trap；crypto 无错误面（ADR-0041）                                           |
| W6（T13-l）       | **已兑现（2026-10-19）= W6 D155–D162**：TLV canonical 线格式与拷贝预算；大负载双车道 = `blob`（不可变 / 可跨实例只读）+ `byte-reader` / `byte-writer`（per-instance）；`fs` += `open-read` / `open-write`，`http` += `download`；process chunk 保持 `list<u8>`（不改写）；`payload` 变体（value / blob）；resource 生命周期与失败面 |
| W7（T13-m）       | 服务方法集与强类型生成（`types!()`）；无契约服务的 method-missing 退化                                                                                                                                                                                                                                                              |
| W8（T13-n）       | `on-event(listener-id, name, payload: payload)` 导出形状；事件名语义 / `internal/` 规则 / manifest `events` 对账；`emit` / `on-event` 的 payload = W6 双车道变体                                                                                                                                                                    |
| W9（T13-o）       | import 清单 ⊆ 词汇表（含 `blobs` / `streams` / `payloads`）；provide / inject / events 对账；能力门判定（含 mount 例外）；blob 无门 / streams 随属主；WASI 0.3 基线 = clocks + random + filesystem（async）+ std 拖带集                                                                                                             |
| W10（T13-p）      | package version 与首个发布快照；import / export 增补纪律；多版本共存                                                                                                                                                                                                                                                                |
| T10.10            | 宿主 import 注册；wasmtime-wasi 0.3；preopen `/`；`on-call` / `on-timer` 导出调用；blob / reader / writer 资源表与跨 Store 重物化                                                                                                                                                                                                   |
| T10.11            | fs（global host-mediated / private WASI 0.3 async filesystem）/ http / process / timer 实现；crypto 撤回（ADR-0041）；预算 / TOCTOU 硬化；random = wasi:random；`open-read` / `open-write` / `download`；blob 创建 / collect                                                                                                        |
| T10.12            | 安装期 world 导出校验补 `on-call` / `on-timer`（W8 后补 `on-event`）；preopen 授予                                                                                                                                                                                                                                                  |
| T10.13            | TLV 编解码（payload value 车道、config、mount 声明、business data）；blob / stream 不进 TLV；`ValueRef` / `encoded_len` + `encode_into`；canonical 编码与版本字段                                                                                                                                                                   |
| T10.14            | `Ctx` / 句柄 / RAII / `metis_plugin::raw`；默认钩子；`Payload` / `Blob` / `ByteReader` / `ByteWriter` / `Value::Bytes`；WASI 0.3 错误码 → fs kind 映射；CSPRNG = guest crate（`getrandom` / `rand`）；crypto 便利 re-export = feature 扩展位（v1 不做）                                                                             |
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
- **fail-fast 清单（宿主可见面重订）**：wait-for 圈闭合（含邮箱容量等待；当场报错 + 整圈路径 + fail-open 解圈 = [ADR-0014](../decisions/0014-dispatch-semantics.md)）；卸载期注册新账（W4.5）；未声明能力调用 = 两层：import 清单 ⊆ SDK 封闭词汇表 = 编译期结构事实（[wasm-first-design-review](../research/wasm-first-design-review.md) B1）；manifest 能力声明 = 运行期门 fail-closed，越权 = trap（W5 D139）；不可转换值跨界（W6 面）。A4 清单的插件内四项消亡或归 SDK：exclusive 嵌套 / 自家人进名（SDK 类型与纪律面）；被登记线程裸 yield（无对应物——宿主无「无登记挂起」可检，观测点缺失即消亡理由）；fan-out 子任务未随 handler 完结（guest 结构化纪律，宿主无观测点）。
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
- **类型住处**：`crates/metis-plugin/wit/error.wit` 的 `interface errors`（W1.2 D117 的文件按章拆）；world 与各 import interface 以 `use errors.{error}` 引用，W3 落地 import 接口时同源引用。不立独立错误版本号；kind 增补随 WIT package version（W10）。
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
- **封闭枚举演进**：新增 kind = 新 WIT package version（W10）；不做开放字符串，也不在 guest 侧枚举未知 kind 的透传槽——跨版本共存时宿主按 `abi` 使用对应版本绑定（W5.3 / W10），旧版绑定不接收新版 kind。

### W5.3 D138 方向性：宿主↔guest

**D138 定案：共享类型 + 方向纪律；guest 导出只许 `business`；guest 返回非 business kind = 宿主判 `core-contract-violation`。**

- **import（宿主→guest）**：宿主可产生 fs / http / process / service / core 族；`business` 仅由 guest 提供方产生、宿主在服务调用路径上透传，宿主自身不产生 `business`（宿主级失败用 core 族）。
- **export（guest→host）**：生命周期钩子、服务 handler、事件 listener 的 `Err` 只许 `business`；`data` = TLV Value（服务路径 = 契约形状；生命周期 / 事件路径 = SDK 约定，通常 none）。SDK 在作者面只提供 `Error::business(data)` 作为导出错误构造入口（具体 API 归 T10.14），文档同条。
- **非 business 的导出错误 = 方向违约**：宿主不静默降级为 business，而是判 `core-contract-violation` 并按出口处置——服务 handler → 调用方收该格 + journal `contract_violation`；`setup` → 加载失败（W1.5），日志记违约；`on-config` → 重挂载（W1.3），日志记违约；`dispose` → 记日志、继续 drain（W1.6）；事件 listener → emit / serial / waterfall = `listener_error` + `contract_violation`，fail-open；parallel = 该项结果计 `core-contract-violation`（A5 D15.2 / ADR-0014）。
- **不立两个 WIT 类型**（`host-error` / `guest-error`）：一个封套是 A5 的设计本体；方向纪律在宿主一处归一即可；两个类型不带来安全收益（business data 本就是任意 TLV），却让 W1.3 签名与 SDK 映射双份。
- **跨版本 kind 面**：`error-kind` 是封闭 enum，新增 case = 破坏性变更；跨版本共存（W10）时宿主按组件声明的 `abi` 使用对应版本的 WIT 绑定，不得把新版 kind 送进旧版绑定。多版本快照 / 翻译责任 = W10 章（登记见 W5.8）。

### W5.4 D139 响亮 vs Result 的 wasm 载体

**D139 定案：调用契约违反（静态可查的 API 误用，含 timer 参数越界 / 卸载期注册）= trap；运行期交互 / 形状失败 = `core-*` Err；运行期数据失败 = 族 Err。**

| 情形                                                                                         | 载体                                                                                                                              |
| -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 未声明能力调用（manifest 门：fs global / http / process / mount 缺省）                       | trap——宿主 import 拒绝，实例 Failed，journal `vm_trap`；依赖方收 `service-unavailable`                                            |
| Unloading 期新注册「比我长寿」的账（监听 / provide / timer）                                 | trap——实例本就在卸载，无额外损失（W4.5 D11.2）                                                                                    |
| timer 参数非法（`every` 低于最小间隔等）                                                     | trap——参数非法 = 契约违反，A5 timer「无 Err 面」平移                                                                              |
| import 参数形状错（WIT 类型面已静态排除；服务 TLV 参数运行期违约、process argv/command XOR） | `core-contract-violation` Err；fs 路径值错误归 `fs-invalid-path`（W3 D144）；违约为参数侧还是返回侧由 message + journal 指示      |
| 服务返回形状违约 / emit payload 违约                                                         | `core-contract-violation` Err + journal `contract_violation`（D19.1 / D22.4 的运行期校验点存活）                                  |
| wait-for 圈闭合                                                                              | `core-deadlock` Err 给调用方（W4.4 fail-open 解圈；message 带整圈路径）                                                           |
| 运行期数据失败（文件不存在 / 网络失败 / 进程退出等）                                         | 对应族 Err                                                                                                                        |
| 事件 listener 失败                                                                           | emit / serial / waterfall = fail-open：journal `listener_error`，不进控制流；parallel = 逐项 `result` 收集（A5 D15.2 / ADR-0014） |
| guest trap（panic / abort / fuel / epoch 截断自身）                                          | 实例死亡；journal `vm_trap`；在飞调用方按 D11.3 收 `service-unavailable`                                                          |
| `setup` 返回 Err / trap                                                                      | 加载失败，实例即弃（W1.5 D120）                                                                                                   |
| `on-config` 返回 Err / trap                                                                  | 重挂载（W1.3 D118 / config-format D42）                                                                                           |
| `dispose` 返回 Err / trap                                                                    | 记日志，不中断后续清理（W1.6 D121）                                                                                               |
| manifest / config / 安装校验失败                                                             | 宿主侧响亮（miette / 启用期拒），不经 guest 错误通道（W2 D133 / W1.5）                                                            |

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
| W7（T13-m）       | business data 形状 SSOT = 服务契约；运行期形状违约 = `core-contract-violation`；`data` 承载 = TLV                                                                                                                                                                                                                                                                                                                                                                      |
| W8（T13-n）       | listener 错误封套 = 本类型；emit 校验违约 = `core-contract-violation` Err + journal（D139 运行期形状失败格）；不新增错误族                                                                                                                                                                                                                                                                                                                                             |
| W9（T13-o）       | 能力对账 + 运行期门；未声明能力 = trap；契约运行期校验点 = `core-contract-violation`                                                                                                                                                                                                                                                                                                                                                                                   |
| W10（T13-p）      | `error-kind` 增补 = 新 WIT package version（封闭 enum 的破坏性变更）；跨版本共存时宿主按 `abi` 使用对应版本绑定，不得把新 kind 送入旧版；无独立错误版本号                                                                                                                                                                                                                                                                                                              |
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
  - 每载荷带格式版本字段；不识版本 = **解码前**失败（config = 启用期响亮失败；服务 / emit / mount = `core-contract-violation` Err，W6.8）；格式版本与 WIT package version 独立（前者管字节布局，后者管接口集；演进纪律分别归 T10.13 / W10）；
  - 预算为形状：条目数 / 嵌套深度 / 总字节；数值 = 实现期配置（T10.13）；超限 = 边界错误（W6.8），不做部分解码后静默截断；
  - `Map` 键只允许 UTF-8 字符串；`Array` / `Map` 严格二分（混合 / 稀疏 / 非字符串键 = 边界错误）。
- **两种解码模式，同一线格式**：动态 `Value` / `ValueRef<'a>`；契约已知时 typed decode（W7 `types!()` 生成 `T::decode`）。不为 typed 面另立紧凑格式——标签成本可忽略，双格式会分裂版本与测试面。
- **字节布局**：tag 位分配、版本号数值、varint 细节 = T10.13 实现期定稿；冻结时点 = 首插件发布前（W10 记录首个快照）。W6 只冻结上述语义与解码顺序。
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

// W1 的导出与 W8 的 on-event 同批改为 payload：
// on-call: async func(service: string, method: string, args: payload) -> result<payload, error>;
// on-event: async func(listener-id: u64, name: string, payload: payload) -> result<option<payload>, error>;
```

- **fanout 语义**：宿主对每个接收方 `duplicate` 一个 blob 句柄（引用计数，0 内容拷贝）；发送方的 own 句柄在入队后释放。接收方不读 = 0 内容拷贝；读多少拷多少。
- **服务调用**：args / 返回 = 单个 payload；`blob` 所有权随调用转移，发送方需要保留时自行 `duplicate`；提供方返回 `blob` 时调用方获得 own 句柄。
- **不变面**：`setup` / `on-config` 的 config = `list<u8>` TLV；`instance.mount` 声明 = `list<u8>` TLV；W5 `error.data` = `option<list<u8>>` TLV（business 错误不放 blob）；`on-timer` 无 payload。
- **作者面**：SDK 映射 `Payload::Value(Value)` / `Payload::Blob(Blob)`；`Value::Bytes` 是 value 车道内的小二进制，blob 是独立车道。
- **兼容性**：`0.1.0` 未发布，本批改签名 = 首个发布快照的编制；发布后增补 = 新 package version（W10）。

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
| W2（T13-i）       | manifest 不变；`capabilities` 不变；`events` 声明面不变（W8 消费 payload 变体）                                                                                                                                                                                                                                    |
| W3（T13-j）       | import 集 = 9 个功能接口（`events` / `services` / `timer` / `http` / `fs` / `process` / `instance` / `blobs` / `streams`）+ 类型面 `errors` / `fs-types` / `payloads`；`fs` += `open-read` / `open-write`；`http` += `download`；`Ctx` += `Payload` / `Blob` / `ByteReader` / `ByteWriter`；W3.15 的 W6 行本批兑现 |
| W4（T13-c）       | 拷贝下限利用 W4.1 自由交错（源快照必须先于挂起）；resource 卸载 drain 接 W4.5；不改变调度                                                                                                                                                                                                                          |
| W5（T13-k）       | 错误封套不变；`error.data` 保持 TLV；流式 EOF / 错误复用属主族；不新增 kind；`core-contract-violation` 承接 TLV / blob 预算失败                                                                                                                                                                                    |
| W7（T13-m）       | 契约形状 = typed decode 的 schema 来源；`ValueRef<'a>` / `T::decode` 消费同一线格式；blob payload 不做形状校验（宿主不解释）                                                                                                                                                                                       |
| W8（T13-n）       | `on-event` / emit 的 payload 改 `payload` 变体；listener 返回 `option<payload>`；事件名 / `internal/` / 声明对账不变                                                                                                                                                                                               |
| W9（T13-o）       | 进口对账词汇表 += `blobs` / `streams` / `payloads`；blob 无能力门；reader/writer 走 fs/http 门；跨实例只读 blob = 结构允许，可变资源仍拒                                                                                                                                                                           |
| W10（T13-p）      | 本批 = 首个发布快照编制（0.1.0 未发布）；新增接口 / payload 变体在同一 package version 内落定；发布后增补 = 新版本                                                                                                                                                                                                 |
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
