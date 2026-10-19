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

| 章  | 题                                                                                                   | 前身（A 系列 = 语义平移源） | 状态                       |
| --- | ---------------------------------------------------------------------------------------------------- | --------------------------- | -------------------------- |
| W1  | 插件包形态与入口契约（包布局 / world 定义 / 生命周期导出）                                           | A1                          | ✅ 2026-10-18（D116–D124） |
| W2  | manifest 格式                                                                                        | A2                          | ✅ 2026-10-19（D125–D134） |
| W3  | ctx 形态：WIT import 接口集与 SDK 映射（作者不见 WASI/WIT = ADR-0034 §4）                            | A3                          | 未开始                     |
| W4  | async 与交错语义（组件模型 async 映射；S1/S2 spike 双 PASS 托底 = 补遗 6/7）                         | A4                          | ✅ 2026-10-17              |
| W5  | 错误与 result 习语（`result<T,E>` 原生）                                                             | A5                          | ✅ 2026-10-19（D136–D141） |
| W6  | 边界转换：canonical ABI / 递归 Value 线编码（TLV 主线化 = E1 选项① 拍板）/ resource 句柄族           | A6                          | 未开始                     |
| W7  | 契约与类型来源（① 平台封闭面 = WIT 富类型原生；A7 半消融 = E5 拍板：瘦身形状 DSL 继承 Value 形状面） | A7                          | 未开始                     |
| W8  | 事件静态声明                                                                                         | A8                          | 未开始                     |
| W9  | deps/provide 与能力门（进口对账机械化）                                                              | A9                          | 未开始                     |
| W10 | 工具形态与 ABI 版本纪律（WIT package version 承接 abi_rev）                                          | A10                         | 未开始                     |

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
- **文件按章拆**：规划中 `plugin.wit`（world + 生命周期导出）；后续 `error.wit`（W5）、`ctx.wit`（W3）、`events.wit`（W8）加入同一 package 目录。WIT 允许多文件同包。
- **固定 world**：单一 world，import 词汇表在一个 package version 内封闭（[plugin-sdk](plugin-sdk.md) S6 已定）；增补 import / export = 新 package version（W10）。宿主与插件不生成按能力裁剪的 world（编译期组合 world = 扩展位，前提 = W2 落地）。
- **版本快照**：v1 只有一个 ABI 版本；多版本共存形态（每个支持版本一份 WIT 目录、宿主全量生成）归 W10。旧组件与新宿主的兼容判定 = manifest 声明的 package version ≤ 宿主支持版本（比较语义 W10）。

### W1.3 D118 生命周期导出集与入口契约

**D118 定案：固定 world 的生命周期导出 = `setup` / `on-config` / `dispose`，全 async；开放数据以 TLV 过边界。**

签名示意（非可编译 WIT：`list<u8>` = TLV 编码的 Value；`error` = W5 D136 的共享错误类型）：

```text
package metis:plugin@0.1.0;

world plugin {
    export setup: async func(config: list<u8>) -> result<_, error>;
    export on-config: async func(config: list<u8>) -> result<bool, error>;
    export dispose: async func() -> result<_, error>;
}
```

- **`setup(config)`**：拿启用期合成并冻结的完整配置（config-format D36 管线产物：校验 → 秘密解析 → 默认值合成），成功返回 = 实例可以进入 Active。返回 `error` = 加载失败（W1.5）。
- **`on-config(config)`**：配置热更新通道（config-format D42）。`Ok(true)` = live-apply 成交；`Ok(false)` / `Err` / trap = 重挂载。默认实现 = `Ok(false)`（SDK 补全），所以「没实现」与「拒绝吸收」是同一行为。
- **`dispose()`**：唯一的手动清理通道（A1.2 平移）。在飞任务结算后由宿主调用（W1.6）；可 await ctx 效应调用；错误 / trap 记日志、不中断后续清理。
- **无 ctx 参数**：ctx = 宿主 import 接口集（W3）；SDK 在作者面合成 `Ctx`，作者签名形如 `async fn setup(&mut self, ctx: &Ctx, config: Value) -> Result<(), Error>`。ABI 不重复传递宿主能力表。
- **world 导出集在单一 package version 内封闭**：组件必须导出三者；缺任一 = 安装期 world 不匹配错误。作者面的「可选钩子」= trait 默认方法，不是缺失导出。`on-call` 服务导出（W3）与 `on-event` 导出（W8）在同一个 world 上 additive 增补，增补 = 新 package version（W10）。
- **元数据唯一去处 = manifest**：组件 custom section 不作声明面（A1.1「table 字段集封闭」的载体平移）。

### W1.4 D119 入口执行时序与激活门

**D119 定案：`setup` 由激活管线直调；Loading 期间投递闸门关闭，Active 后开门。**

本节只记入口契约对 ABI 调用的约束；投递与背压语义的 SSOT = W4.3，配置变更语义 = config-format D42。

- 顺序：实例化（imports 已注册）→ 宿主直调 `setup` → 成功：fiber 进 Active、闸门开门 → 邮箱积压消息按既有投递政策消费（W4.3）。失败：W1.5。
- `setup` 不走邮箱、不过投递闸门，但按在飞预算记账（第一个任务）。Loading 期间事件 / 服务调用 / 定时器 tick 只进有界邮箱积压（不投递、不丢弃）；溢出走既有背压链（[context-events](context-events.md) 政策；W4.3）。
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
- wasip2 std 拖带的 14 个 WASI 0.2 导入不受影响（它们是 import，不是 start；[plugin-sdk](plugin-sdk.md) S3 既定）。

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

| 去向           | 内容                                                                                                                      |
| -------------- | ------------------------------------------------------------------------------------------------------------------------- |
| W2（T13-i）    | **已兑现（2026-10-19）**：manifest 字段全集与形状 = D125–D133；模块系统消亡 = D134/ADR-0038；fs 两档 = D129/D130/ADR-0039 |
| W3（T13-j）    | ctx = imports（D118）；作者面签名合成；`on-call` 服务调用导出；生命周期导出与 ctx import 接口集的边界                     |
| W5（T13-k）    | **已兑现（2026-10-19）**：`error` 类型 = W5 D136–D141；生命周期返回的 `result` 形状确认（D118）                           |
| W7（T13-m）    | 随包契约 `lib/`（包内容区的一部分，D35）；W1 只定 `manifest.yml` + 组件文件 + `lib/` 的包布局边界                         |
| W8（T13-n）    | `on-event` 导出 additive 增补到同一 world（D118）                                                                         |
| W10（T13-p）   | package version 的数值与比较、多版本快照、ABI 版本纪律（D117）                                                            |
| T10.10         | 实例化 / Store 限额 / fuel / epoch；加载预算与 dispose 预算的数值；`.cwasm` 缓存                                          |
| T10.12         | 安装期 start / world 校验；包内容复制与未知文件处置；卸载                                                                 |
| T10.14         | SDK 默认实现（`setup` = `Ok(())`、`on-config` = `Ok(false)`、`dispose` = `Ok(())`）；作者面签名；`Ctx` 合成               |
| T10.15         | 可部署包布局与组件文件名（D116）；预检 start 拒绝（D122）；零仪式源码合成 manifest                                        |
| config-format  | D42 `on_config` 的 ABI 签名已兑现（D118）；D29 `plugin.wasm` 占位已兑现（D116）；D35 零仪式 ABI 面（D123）                |
| build-pipeline | D106/D108 的组件文件名 / 包布局指针已兑现（D116）；D108 start 政策已确认（D122）                                          |
| instance-model | D63 零仪式无 mount 资格（D123 原位）                                                                                      |
| plugin-sdk     | S2 WIT 目录确认（D117）；S1 生命周期钩子形态（D118）；`on_config` 归 W1                                                   |
| ADR-0005       | Consequences 成本账局部取代（[ADR-0037](../decisions/0037-component-instance-cost-ledger.md)，D124）                      |
| 实现期配置     | 加载总预算（W1.5）；`dispose` 墙钟预算（W1.6）                                                                            |
| 扩展位         | 编译期组合 world（S6 登记，W2 已落地后维持不启用）；多组件包（W1/W2 包布局面）；多 ABI 版本共存（W10）                    |

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
- **结构化纪律平移**：handler 完结时其 fan-out 子任务须已完结——旧 ABI 靠宿主强制取消兑现，wasm 世界 = guest 语言设施职责（`join!` 天然结构化；detach spawn = SDK 纪律面）。宿主不补机制。
- **扩展位登记**：exclusive 可选上报宿主做 wait-for 可见性——上报是仪表不是机制（E3 议题批原话）；触发器 = 首个死锁调试痛点。

### W4.3 D95 投递政策与背压

**D95 定案：投递 = 向实例发起新 export 调用；邮箱 + 每实例在飞任务预算闸门；完成永不挡。**

- **worker 语义**：worker 空闲取邮箱消息，经闸门接纳后投递，任务立即起跑；import 完成 = 唤醒源直接唤醒对应 guest 任务，不经邮箱、不经闸门。宿主侧没有「就绪恢复」调度对象——A4 单可调度队列 FIFO（D8.1）的问题形（恢复 vs 新消息的优先级之争、tie-break、两级队列）随载体整体消解；原被否项「完成优先」的饿死分析（流式长读插队致热更等待无界）随之失去对象。热更/卸载不进投递队列（关闸 = 停接纳，W4.5），等待硬上界结论由卸载时序兑现，不再由队列规范兑现。
- **预算闸门（A4 D8.2 平移）**：预算只挡「邮箱消息 → 投递」的接纳——每实例在飞 async 任务数封顶（数值 = 实现期配置，登记 = W4.7）；预算满 = 新消息留邮箱，背压链 = 邮箱满 → 投递方背压/超时（[context-events](context-events.md) 既有政策）→ 服务调用方 deadline 收场（service-registry S5）。完成永不挡 → 在飞必然收敛 → 预算必然释放；超限政策 = 纯回压不立响亮档（A4 D10.2 拍板原位：回压把瞬态尖峰变成延迟，响亮把它变成用户可见错误——尖峰是常态，错误化 = 误伤）。
- **ADR-0014 兼容注**：单派发者保序承诺（同一监听者收件序 = 派发入邮箱序）= 邮箱层性质，不受重入影响——承诺的是收件序，非处理完成序（A4 既有注平移）。
- **timer 处置平移（A4 D1.4）**：tick = 普通邮箱消息，与其他消息同等走闸门，不立特殊政策。积压面 = 在飞任务数（稳态界 ≈ 到达率 × 平均延迟；每条在飞任务持 guest 栈与捕获态）；兜底 = `every` 最小间隔政策（seam §5.2）+ 写码期教具（W4.4 层二）。
- **在飞即成本**：高频事件源回调写短；怕自撞 = SDK async mutex（A4「在飞协程」成本条的载体词平移）。

### W4.4 D96 跨插件死锁与保险丝

**D96 定案：跨插件死锁四层处置平移；插件内等待面宿主不立 deadline；fail-fast 清单按宿主可见面重订。**

- **D1.7 四层平移**：① 启用期 manifest 依赖图禁环（不动；WIT 形态归 W9 章）；② 写码期教具 = 形状教材归 SDK lints 与 Creator 文档（AST lint 管线随载体消亡 = [wasm-first-design-review](../research/wasm-first-design-review.md) A8；三形状载体中立，于 Rust 同样成立：跨 await 读改写 / 持锁等跨插件调用 / `every` 回调挂起未自锁）；③ 运行期 wait-for 图（宿主全见跨插件等待——服务调用与事件链；收窄写实：插件内名字等待随宿主零同步原语退出图的节点集，除非 W4.2 扩展位上报激活）；④ 超时 = 最后保险丝（[ADR-0011](../decisions/0011-actor-execution-model.md) 三层保险既有面）。
- **插件内等待面盘点（A4 D10.1 平移）**：fan-out join 与 exclusive 排队 = guest 内部面，宿主不见不立 deadline（guest 侧结构化纪律 = SDK）；流 `read()` 挂起 = 唯一真无界源的性质不变（作者自开自流、可见可控），处置 = W6 资源面登记（per-read timeout 扩展位随章平移）。不立的理由平移：能定出原则性数值的形状都已有 deadline，剩下的是合法长等，任何统一数值都是拍脑袋；兜底三件套 = 在飞预算记账 + 卸载强杀（W4.5）+ 成环检测（层三）。
- **fail-fast 清单（宿主可见面重订）**：wait-for 圈闭合（当场报错 + 整圈路径 + fail-open 解圈 = [ADR-0014](../decisions/0014-dispatch-semantics.md)）；卸载期注册新账（W4.5）；未声明能力调用 = 两层：import 清单 ⊆ SDK 封闭词汇表 = 编译期结构事实（[wasm-first-design-review](../research/wasm-first-design-review.md) B1）；manifest 能力声明 = 运行期门 fail-closed，越权 = trap（W5 D139）；不可转换值跨界（W6 面）。A4 清单的插件内四项消亡或归 SDK：exclusive 嵌套 / 自家人进名（SDK 类型与纪律面）；被登记线程裸 yield（无对应物——宿主无「无登记挂起」可检，观测点缺失即消亡理由）；fan-out 子任务未随 handler 完结（guest 结构化纪律，宿主无观测点）。
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
| W6 章（边界转换）    | 流 `read()` 无界源登记（W4.4）；不可转换值跨界 fail-fast 格（W4.4）；流式句柄族 drain 强 close 铁律平移面（W4.5 D11.2）                                                                                                                                    |
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

| 族       | kind                      | 触发                                                                                               | 备注                                                                                          |
| -------- | ------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| fs       | `fs-not-found`            | read / list / delete 目标不存在                                                                    | A5 原位                                                                                       |
|          | `fs-permission-denied`    | OS 层拒绝（EACCES / EPERM / EROFS / ENOTCAPABLE，动作相同 = 放弃）                                 | private preopen 逃逸同格                                                                      |
|          | `fs-invalid-path`         | global 档两段式解析拒绝全族：逃逸 / 豁免命中 / 循环 / 超长；TOCTOU 竞态窗口同格                    | message 通用化，不点名豁免命中；完整原因进 journal（D140）                                    |
|          | `fs-invalid-utf8`         | 文本读取的 UTF-8 校验失败                                                                          | W3 若选 bytes-only 读取，本格在 WIT 包冻结前删除                                              |
|          | `fs-invalid-type`         | read 目录 / list 文件                                                                              | A5 原位                                                                                       |
|          | `fs-too-large`            | read 预算超限                                                                                      | 预算数值归 T10.11                                                                             |
|          | `fs-io`                   | 兜底（message = OS 原文，按 D89 脱敏）                                                             | 开放空间诚实兜底格                                                                            |
| http     | `http-timeout`            | 强制超时命中（与 `service-timeout` 不同族：彼 = 服务调用 deadline，此 = 外部请求超时）             | A5 原位                                                                                       |
|          | `http-connect`            | DNS / TCP / TLS / 连接中断一格（动作相同）                                                         | A5 原位                                                                                       |
|          | `http-policy`             | 域名白名单 / SSRF / 跨域重定向拒绝（部署者数据 → Err）                                             | 与未声明能力的 loud 两界不混（W5.4）                                                          |
|          | `http-too-large`          | 响应预算超限                                                                                       | A5 原位                                                                                       |
|          | `http-invalid`            | URL 畸形等值层参数错误                                                                             | 参数形状错仍走响亮（WIT 类型面静态排除）                                                      |
|          | `http-error`              | 兜底：响应畸形 / 解压失败 / 重定向异常                                                             | 开放空间诚实兜底格                                                                            |
| process  | `process-spawn-failed`    | 命令不存在 / 权限 / fork 失败                                                                      | A5 原位                                                                                       |
|          | `process-timeout`         | exec 强制超时杀进程组                                                                              | A5 原位                                                                                       |
|          | `process-gone`            | write / signal 已退出进程（高频竞态）                                                              | read 在进程死后 = 正常 EOF，不算错误                                                          |
| service  | `service-unavailable`     | 提供方缺席 / 崩溃 / 卸载窗口；可选依赖常态答案                                                     | ADR-0018 S5；D11.3 即错即回                                                                   |
|          | `service-method-missing`  | 方法名不存在（宿主查注册表拦下，不走消息）                                                         | ADR-0018 S5                                                                                   |
|          | `service-timeout`         | 服务调用 deadline / 提供方被 fuel / epoch 截断                                                     | ADR-0018 S5                                                                                   |
| core     | `core-deadlock`           | wait-for 圈闭合（W4.4 层三）；message 带整圈路径（按实现期上限截断）                               | 宿主检测 + fail-open 解圈；不占服务族第五格                                                   |
|          | `core-contract-violation` | 运行期契约 / 方向违约：服务参数或返回值形状不符、emit payload 不符、guest 导出返回非 business kind | journal `contract_violation`；message 人读，结构化细节住 journal                              |
| business | `business`                | guest 起源的业务错误（服务 handler 的业务失败 / 生命周期钩子与事件 listener 的唯一合法导出 kind）  | `data` = TLV Value；服务路径形状 SSOT = 服务契约（W7/W9）；非服务路径由 SDK 约定（通常 none） |

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

| 情形                                                                   | 载体                                                                                                                              |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 未声明能力调用（manifest 门：fs global / http / process / mount 缺省） | trap——宿主 import 拒绝，实例 Failed，journal `vm_trap`；依赖方收 `service-unavailable`                                            |
| Unloading 期新注册「比我长寿」的账（监听 / provide / timer）           | trap——实例本就在卸载，无额外损失（W4.5 D11.2）                                                                                    |
| timer 参数非法（`every` 低于最小间隔等）                               | trap——参数非法 = 契约违反，A5 timer「无 Err 面」平移                                                                              |
| import 参数形状错（WIT 类型面已静态排除；服务 TLV 参数运行期违约）     | `core-contract-violation` Err——调用方统一失败 / 告警；违约为参数侧还是返回侧由 message + journal 指示（v1 不提供机器可分支细节）  |
| 服务返回形状违约 / emit payload 违约                                   | `core-contract-violation` Err + journal `contract_violation`（D19.1 / D22.4 的运行期校验点存活）                                  |
| wait-for 圈闭合                                                        | `core-deadlock` Err 给调用方（W4.4 fail-open 解圈；message 带整圈路径）                                                           |
| 运行期数据失败（文件不存在 / 网络失败 / 进程退出等）                   | 对应族 Err                                                                                                                        |
| 事件 listener 失败                                                     | emit / serial / waterfall = fail-open：journal `listener_error`，不进控制流；parallel = 逐项 `result` 收集（A5 D15.2 / ADR-0014） |
| guest trap（panic / abort / fuel / epoch 截断自身）                    | 实例死亡；journal `vm_trap`；在飞调用方按 D11.3 收 `service-unavailable`                                                          |
| `setup` 返回 Err / trap                                                | 加载失败，实例即弃（W1.5 D120）                                                                                                   |
| `on-config` 返回 Err / trap                                            | 重挂载（W1.3 D118 / config-format D42）                                                                                           |
| `dispose` 返回 Err / trap                                              | 记日志，不中断后续清理（W1.6 D121）                                                                                               |
| manifest / config / 安装校验失败                                       | 宿主侧响亮（miette / 启用期拒），不经 guest 错误通道（W2 D133 / W1.5）                                                            |

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
| `nil, nil` = EOF 位                       | 流式 read 的 EOF 表示归 W6（显式形态）；非错误                                                                 |
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

| 去向              | 内容                                                                                                                                                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W1（T13-h）       | **已兑现（2026-10-19）**：`error` 类型 = D136；W1.3 生命周期签名确认；`setup` Err / trap = 加载失败（W1.5）；`dispose` Err / trap = 记日志继续（W1.6）                                                                                           |
| W2（T13-i）       | **已兑现（2026-10-19）**：manifest 校验错误 = 宿主侧响亮，不经 guest 错误通道（D133）；生命周期 `error` 类型 = D136                                                                                                                              |
| W3（T13-j）       | 所有 fallible import 返回 `result<T, error>`；能力门未声明 = trap（D139）；fs / http / process 错误映射；timer 无 Err 面；`mount` 未声明 = trap；服务调用返回 service / business / core 族；冻结前确认 `fs-invalid-utf8` 去留（bytes-only 则删） |
| W4（T13-c）       | **已兑现（2026-10-19）**：W4.4 fail-fast 错误面 = D137 / D139——wait-for = `core-deadlock`；卸载期新账 = trap；D11.3 = `service-unavailable`                                                                                                      |
| W6（T13-l）       | 流式 read 的 EOF 表示与错误复用属主族（A5 D14.3 语义）；resource 句柄错误复用本类型；business data = TLV（与 W6 线编码同源）                                                                                                                     |
| W7（T13-m）       | business data 形状 SSOT = 服务契约；运行期形状违约 = `core-contract-violation`；`data` 承载 = TLV                                                                                                                                                |
| W8（T13-n）       | listener 错误封套 = 本类型；emit 校验违约 = `core-contract-violation` Err + journal（D139 运行期形状失败格）；不新增错误族                                                                                                                       |
| W9（T13-o）       | 能力对账 + 运行期门；未声明能力 = trap；契约运行期校验点 = `core-contract-violation`                                                                                                                                                             |
| W10（T13-p）      | `error-kind` 增补 = 新 WIT package version（封闭 enum 的破坏性变更）；跨版本共存时宿主按 `abi` 使用对应版本绑定，不得把新 kind 送入旧版；无独立错误版本号                                                                                        |
| journal           | 不新增 kind；`listener_error` / `vm_trap` / `contract_violation` / `fs_invalid_path_detail` 原位承接；错误 kind 落盘用 kebab-case 规范名                                                                                                         |
| seam-capabilities | §6.1 W5 行兑现：能力失败 `result` 形态 + service 四态；`fs-invalid-path` message 通用化政策                                                                                                                                                      |
| plugin-sdk        | **已兑现（2026-10-19）**：D102 三格封套不变；`ErrorKind` 1:1；guest 导出只许 business；`?` 习语                                                                                                                                                  |
| service-registry  | ADR-0018 S5 四态 = `service-unavailable` / `service-method-missing` / `service-timeout` + 共享 `business`；core 族不占第五格；调用方可见集合 = 四态 + core 两格                                                                                  |
| T10.10            | trap → 实例 Failed + journal `vm_trap`；在飞调用方 `service-unavailable`                                                                                                                                                                         |
| T10.11            | 能力错误映射（fs / http / process）；未声明能力 trap；timer 参数 trap                                                                                                                                                                            |
| T10.13            | business data = TLV Value（`metis-codec`）                                                                                                                                                                                                       |
| T10.14            | `Error` / `ErrorKind` 类型与构造器；SDK 文档（服务四态 + core 两格、business-only 导出、message 不分支）                                                                                                                                         |
| Creator 文档      | 作者契约五条（W5.6）：`?` / kind 分支 / business-only 导出 / business data 契约形状 / trap 不可捕获；「族 → 建议动作」表按 kind 清单重写                                                                                                         |
| 实现期配置        | message 长度上限（宿主侧截断 + journal 注记；登记 T10.10 / T10.11 / T10.14）；core-deadlock 路径同受该上限；business data 预算沿用 W6 / T10.13                                                                                                   |
| 扩展位            | 非 business 族的结构化 `data` 不立（将来需要走 package version）；`retryable` 维持不立；per-family data = 扩展位                                                                                                                                 |
| ADR               | 不新增；A5 语义继承（ADR-0026 已 superseded）；ADR-0018 S5 四态不动（调用方可见集合 = 四态 + core 两格，见 W5.2）                                                                                                                                |
