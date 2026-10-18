# Metis — WIT ABI 设计（living doc）

> 状态：living design doc——2026-10-15 开章（[ADR-0034](../decisions/0034-wasm-first-carrier.md) wasm-first 转向同批）。W 系列章节随设计批逐章收官，章末联动登记汇总惯例同前。各章状态见目录表。
> 前提：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；接缝能力集 = [seam-capabilities](seam-capabilities.md)；遏制四约束 = [ADR-0005](../decisions/0005-vm-topology.md)；边界拷贝语义四层 = [wasm-research](../research/wasm-research.md) 补遗 5 §5.4。
> 拍板锚（2026-10-16 两轮）：接口数据三分法 / E4 fs 三层——SSOT = 台账 T13 状态行，议题背景 = [wasm-first-design-review](../research/wasm-first-design-review.md) §6/§7；W 各章收官时吸收进正文，此行随吸收删除（E3 并发投递 已吸收 = §W4，2026-10-17；三分法①/TLV 边界已吸收 = §W1.3，2026-10-18；E4 fs 三层待 W3/W9 吸收）。
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
| W2  | manifest 格式                                                                                        | A2                          | 未开始                     |
| W3  | ctx 形态：WIT import 接口集与 SDK 映射（作者不见 WASI/WIT = ADR-0034 §4）                            | A3                          | 未开始                     |
| W4  | async 与交错语义（组件模型 async 映射；S1/S2 spike 双 PASS 托底 = 补遗 6/7）                         | A4                          | ✅ 2026-10-17              |
| W5  | 错误与 result 习语（`result<T,E>` 原生）                                                             | A5                          | 未开始                     |
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
- 组件文件名缺省 `plugin.wasm`；manifest 可给包内相对路径指定其他文件名（字段名与形状 = W2）。指定值必须是包根下的单个相对路径分量（不进入 `lib/` / `data/`，不含 `..`、绝对路径、符号链接与大小写折叠碰撞），且目标必须是 wasm 组件（`.wasm` 扩展或组件 magic 校验；非组件 = 安装错误）。一个包只有一个插件组件：包内容区（不含 `lib/` 与 `data/`）内除被指名的组件外若还有 `.wasm` = 安装期响亮错误；未指定时 `plugin.wasm` 必须存在，其他 `.wasm` 同理。
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

签名示意（非可编译 WIT：`list<u8>` = TLV 编码的 Value；`error` = W5 章的共享错误类型，W5 落地前 WIT 包不冻结）：

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
- ABI 面不变：三个生命周期导出恒在场（SDK 默认补全）；config = 空 map；无 config schema、无能力门资格（imports 仍按固定 world 对账，效应能力 fail-closed）、无 provide/inject、不可 mount（config-format D35；[instance-model](instance-model.md) D63）。
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

| 机制                                                           | 死因                                                                                                                                                      |
| -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `plugin()` 糖 / 入口返回 table / 裸函数报错（A1.1）            | 入口形状 = 固定 world 的导出集；作者面糖归 SDK 宏（S1），ABI 无入口返回物                                                                                 |
| `setup` 返回 disposer 函数（A1.2）                             | 跨组件边界不能传闭包；手动清理独立为 `dispose` 导出（D118/D121）                                                                                          |
| table 字段集封闭 / `on_config` 作可选字段（A1.1）              | world 导出集封闭；`on-config` 是恒在导出，可选性 = SDK 默认实现；元数据唯一去处 = manifest                                                                |
| 单文件形态与成长路径搬家（A1.3）                               | 目录单形态已由 D35 定；成长路径 = 加 manifest，组件文件不动（D116/D123）                                                                                  |
| require 体系 / `.luaurc` / 入口模块缓存 / 循环 require（A1.5） | guest 是编译产物，模块解析在编译期完成；机制无对应物（[ADR-0003](../decisions/0003-module-require-discipline.md) 的 supersede 归 W2/T13-i，已登记未落地） |
| 加载期「编译入口 chunk」步骤与字节码缓存（A1.5）               | 组件编译 / 实例化由 wasmtime 承担（`.cwasm` 缓存归 T10.10）                                                                                               |
| `setup(ctx, config)` 的 ABI ctx 参数（A1.1）                   | imports 即 ctx（W3）；ABI 不传能力表（D118）；作者面 `ctx` 句柄保留（SDK 合成）                                                                           |
| `ctx.on_cleanup(fn)` 注册式多清理扩展位（A1.2）                | guest 语言设施（Drop / 组合清理）自管；宿主效应账本仍在，只是不承载 guest 清理回调                                                                        |

**语义平移保留**：A1.4 的「setup 里只能等完成不依赖自身邮箱的调用」红线不是消亡项，只是表述换载体（W1.4 D119）。

### W1.11 联动登记汇总（章末）

| 去向           | 内容                                                                                                        |
| -------------- | ----------------------------------------------------------------------------------------------------------- |
| W2（T13-i）    | 组件文件名的 manifest 字段（缺省 `plugin.wasm`，D116）；manifest 元数据唯一去处；零仪式缺省；身份 = 目录名  |
| W3（T13-j）    | ctx = imports（D118）；作者面签名合成；`on-call` 服务调用导出；生命周期导出与 ctx import 接口集的边界       |
| W5（T13-k）    | `error` 类型定义；生命周期返回的 `result` 形状已固定（D118）                                                |
| W7（T13-m）    | 随包契约 `lib/`（包内容区的一部分，D35）；W1 只定 `manifest.yml` + 组件文件 + `lib/` 的包布局边界           |
| W8（T13-n）    | `on-event` 导出 additive 增补到同一 world（D118）                                                           |
| W10（T13-p）   | package version 的数值与比较、多版本快照、ABI 版本纪律（D117）                                              |
| T10.10         | 实例化 / Store 限额 / fuel / epoch；加载预算与 dispose 预算的数值；`.cwasm` 缓存                            |
| T10.12         | 安装期 start / world 校验；包内容复制与未知文件处置；卸载                                                   |
| T10.14         | SDK 默认实现（`setup` = `Ok(())`、`on-config` = `Ok(false)`、`dispose` = `Ok(())`）；作者面签名；`Ctx` 合成 |
| T10.15         | 可部署包布局与组件文件名（D116）；预检 start 拒绝（D122）；零仪式源码合成 manifest                          |
| config-format  | D42 `on_config` 的 ABI 签名已兑现（D118）；D29 `plugin.wasm` 占位已兑现（D116）；D35 零仪式 ABI 面（D123）  |
| build-pipeline | D106/D108 的组件文件名 / 包布局指针已兑现（D116）；D108 start 政策已确认（D122）                            |
| instance-model | D63 零仪式无 mount 资格（D123 原位）                                                                        |
| plugin-sdk     | S2 WIT 目录确认（D117）；S1 生命周期钩子形态（D118）；`on_config` 归 W1                                     |
| ADR-0005       | Consequences 成本账局部取代（[ADR-0037](../decisions/0037-component-instance-cost-ledger.md)，D124）        |
| 实现期配置     | 加载总预算（W1.5）；`dispose` 墙钟预算（W1.6）                                                              |
| 扩展位         | 编译期组合 world（S6 登记，前提 = W2）；多组件包（W1/W2 包布局面）；多 ABI 版本共存（W10）                  |

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
- **fail-fast 清单（宿主可见面重订）**：wait-for 圈闭合（当场报错 + 整圈路径 + fail-open 解圈 = [ADR-0014](../decisions/0014-dispatch-semantics.md)）；卸载期注册新账（W4.5）；未声明能力调用 = 进口对账编译期结构事实（[wasm-first-design-review](../research/wasm-first-design-review.md) B1——运行期出现规则兜底退役为结构保证）；不可转换值跨界（W6 面）。A4 清单的插件内四项消亡或归 SDK：exclusive 嵌套 / 自家人进名（SDK 类型与纪律面）；被登记线程裸 yield（无对应物——宿主无「无登记挂起」可检，观测点缺失即消亡理由）；fan-out 子任务未随 handler 完结（guest 结构化纪律，宿主无观测点）。
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
| W5 章（错误）        | fail-fast 清单的错误面接口（W4.4）——wait-for 圈 / 卸载期新账的错误族归置                                                                                                                                                                                   |
| W6 章（边界转换）    | 流 `read()` 无界源登记（W4.4）；不可转换值跨界 fail-fast 格（W4.4）；流式句柄族 drain 强 close 铁律平移面（W4.5 D11.2）                                                                                                                                    |
| journal.md           | `fanout_cancelled` 处置 = **已兑现 = [ADR-0035](../decisions/0035-journal-fanout-cancelled-supersede.md)**（2026-10-18）；`unload_gate_closed`/`unload_killed` 两族存活（W4.5 语义原位，条目措辞随实现期载体词）                                           |
| 实现期配置           | 每实例在飞 async 任务预算（W4.3，超限 = 纯回压）；优雅窗口 + drain 总预算（W4.5）；host 侧事件在途投递界（A4 D10.2 原位存活）                                                                                                                              |
| 扩展位               | exclusive 上报 wait-for 可见性（W4.2，触发器 = 首个死锁调试痛点）；原子方法族其余成员（A4 D1.8 平移，触发器 = 首个真实消费者）；per-read timeout 参数（W4.4 → W6 章面，触发器 = 真实需求）                                                                 |
| seam-capabilities    | 旧 FAQ（引 `ctx.fanout`/`ctx.exclusive`）与旧「挂起点 = ctx 方法调用处」行 = 已消亡/重定义——**已清写 = T13-f**（2026-10-18）                                                                                                                               |
| ADR-0011             | 「独立解释器状态」「interrupt 时间盒」两格载体面 partial supersede——**同批落地 = Status 行注**（2026-10-17）                                                                                                                                               |
| context-events §0/§1 | worker 伪码、三层保险行与服务调用路径行清写——**已清写 = T13-c/T13-f**（2026-10-18）                                                                                                                                                                        |
| Creator 文档         | 作者契约四条平移（协作式 / 挂起窗口 / 同步全显式 / 在飞即成本，W4.1）+「等的事重叠 = guest async 原生；算的事并行 = 多实例/外部服务」分工条                                                                                                                |
