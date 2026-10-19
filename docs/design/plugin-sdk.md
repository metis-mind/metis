# Metis — 插件 SDK 作者面设计（living doc）

> 状态：living design doc——2026-10-17 开章（T13-d SDK 形态议题批拍板写入：七项形态定案 = S1–S7（D98–D104）；第八项 = 固化去处定案，无 D 号——本文件开章 + [sdk-precedent-survey](../research/sdk-precedent-survey.md) 落盘即其兑现）。本文档 = 插件作者面（`metis-plugin` crate 的宏形状、trait 族、异步与错误习语、类型生成）的 SSOT；ABI 层（宿主 ↔ 组件边界）= [wit-abi](wit-abi.md) W 系列，本文档住其上一层——ABI 走钉版纪律（W10 章），作者面随语义演进，分层防单一文档膨胀（[doc-lifecycle](doc-lifecycle.md)）。
> 前提（已定不议）：插件唯一载体 = wasm 组件、作者不见 WASI/WIT、`#[metis::plugin]` + trait 实现形态（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §1/§4）；接口数据三分法与 E3/E4 拍板（2026-10-16，SSOT = [台账](../backlog.md) T13 状态行）；实例内执行模型 = [wit-abi](wit-abi.md) W4 章（D93–D97）。
> 证据基线：[sdk-precedent-survey](../research/sdk-precedent-survey.md)（proxy-wasm / zed_extension_api / Spin / Extism PDK 宏藏 ABI 机制四路调研，2026-10-17）；[wasm-research](../research/wasm-research.md) 补遗 6/7（S1/S2 spike：guest 样板 31 行、同实例重入实证）。
> Last reviewed: 2026-10

---

## 0. 术语锚点

- **作者面**：插件作者直接读写的全部表面 = SDK 暴露的 trait / 宏 / 类型 / 错误习语；其反面 = ABI 面（WIT/canonical ABI），作者不见（ADR-0034 §4）。
- **封闭 import 词汇表**：组件可静态 import 的接口集合由 SDK 固定 world 决定，作者无法越权添加（S6 释读）。
- **catch-all 分发**：按名运行时分发事件/服务调用到单个钩子方法（`on_event(name, payload)`）；演进位 = 按契约生成强类型方法（S7）。
- **门面 re-export**：SDK 把第三方 crate 的关键件以自身路径再导出，作者依赖 SDK 单点钉版；作者自加依赖的自由不受影响（guest 包生态 = [wasm-first-design-review](../research/wasm-first-design-review.md) B2）。

## S1 D98 作者面骨架 = 实例 struct + Plugin trait（2026-10-17 定案，T13-d 拍板）

作者写 `struct` 持有实例状态 + `impl Plugin`；trait 钩子族（生命周期 setup/dispose、on_event/on_call/on_config）全默认实现，作者只覆写所需钩子。实例状态 = struct 本体，每实例一份，由 setup 构造——与实例模型（[ADR-0028](../decisions/0028-instance-model.md)）一等对齐。按名分发 v1 = catch-all（`on_event(ctx, name, payload: Payload)` 形态，W6 D160 / W8 D177），按事件/服务生成强类型方法 = 演进位（S7）。最小插件 = 空 struct + 一个覆写方法，约 15–20 行。

**W3 细化（2026-10-19）**：事件兴趣用 `ctx.on(name)` 登记（返回 `Listener` RAII；同名去重 + 引用计数，一个实例一个宿主条目），处理走 `on_event` catch-all；timer 用 `ctx.timer().after/every(ms, handler)` 注册闭包，无 `on_timer` trait 钩子（SDK 的 `on-timer` 导出是内部分发器）。

淘汰项 = Spin/Extism 谱系的自由函数宏（每导出点一个 fn）：实例状态被推给全局变量，与实例模型和 W4 同实例重入（共享堆，D93）结构性冲突——Zed 的 `static mut EXTENSION` 全局单例被先例调研点名为同一坑（[sdk-precedent-survey](../research/sdk-precedent-survey.md) §2.6）。先例地基：Zed `Extension` trait 全默认方法（唯一必需项 = `new()`）、默认值分「未实现即错」与「空即合理」两语义档位；proxy-wasm 全默认钩子族 + 只覆写所需。

不归本节：钩子族清单填实 = [wit-abi](wit-abi.md) W1（setup/dispose/on-config 形态 = D118）/ W3（ctx 与 on_call）/ W8（on_event 与静态声明；**已兑现（2026-10-19）= W8 D173–D179**）三章。薄插件仪式感地板（ADR-0034 风险 W2）分工：作者手写面 = `lib.rs` 单文件 + manifest 声明；crate-type/target/钉版依赖藏进脚手架模板，归 T13-e / T10.15。

## S2 D99 绑定生成位置 = Zed 式（generate!/export! 住 SDK crate 内）（2026-10-17 定案，T13-d 拍板）

wit-bindgen 的 `generate!` / `export!` 与 Guest impl 全住 `metis-plugin` crate 内；`#[metis::plugin]` 挂 `impl Plugin` 块，展开 = 实例工厂注册 + 入口 glue；作者 crate 零生成物。WIT SSOT 目录随 SDK crate = `crates/metis-plugin/wit/`（[wit-abi](wit-abi.md) W1.2 D117 已确认）；宿主 `wasmtime::component::bindgen!` 消费同一目录树（Zed 先例 = 同一 WIT 目录树两视角：guest 只对最新快照生成，宿主对全快照生成并按版本分发）。

淘汰项 = Spin 式（WIT vendor 在宏 crate 内，`generate!` 推迟到用户 crate 展开）：其收益（作者可见生成类型做 escape hatch）在 TLV 主线边界（E1①/E2① 拍板：`list<u8>` 进 / `result<list<u8>, list<u8>>` 出）下无对象——canonical ABI 面极小且固定，作者永不接触生成的 WIT 类型；代价 = 每插件重复展开 + WIT 路径双写风险。附带收益：作者 crate 编译管线无生成步骤，增量构建地板（[wasm-first-design-review](../research/wasm-first-design-review.md) B9 实测 0.22–0.29 s）不劣化。

演进术 = trait 默认方法（Zed 先例：WIT world 加 export → trait 加带默认体的方法 → 旧插件照常编译运行），ABI 演进对作者面的冲击被结构性压低。

## S3 D100 crate 拓扑与钉版面（2026-10-17 定案，T13-d 拍板）

拓扑 = `crates/metis-plugin`（作者面 facade：Plugin trait / ctx / Error / Exclusive / futures 门面，内藏 WIT 绑定）+ `crates/metis-plugin-macros`（proc-macro——属性宏发布受 Rust 强制：proc-macro 与普通代码不能同 crate 发布；Spin/Extism 同此两 crate 形态）。Value/TLV 依赖 `metis-codec`（T10.13，宿主/guest 双 target）。

wit-bindgen = `metis-plugin` 的**私有钉版依赖**（Zed 先例：作者不见 wit-bindgen；版本随 SDK 统一演进）——ADR-0034 风险 W3 钉版纪律（wit-bindgen/wasm-tools/wasmtime 对齐）在 SDK 侧的落点。内建件便利 re-export（serde_json 等）= feature 面（design-review B2 登记），v1 不做。

目标 = **WASI 0.3**（2026-10-19 维护者确认）；guest std 拖带的 WASI 0.3 基线导入（clocks / random / filesystem async 等）由宿主统一链接 wasmtime-wasi 0.3（归 T10.10）；WASI 基线白名单 = W3 D154 / W9 的进口对账面，SDK 不暴露 WASI 名。`wasm32-wasip3` target / std p3 迁移的工具链落地归 T10.10 / T10.15（ADR-0034 W7）；wasip2 时代的 14 导入 papercut（[wasm-research](../research/wasm-research.md) 补遗 6）为历史证据，0.3 基线由 T10.10 核实。no_std / build-std 裁剪 = 扩展位，触发器 = 插件二进制体积/冷启动投诉成串。

## S4 D101 异步面 = 全 async trait + futures 门面 + Exclusive 助手（2026-10-17 定案，T13-d 拍板）

trait 方法全 `async fn`（Rust 原生 async fn in trait）。fanout = guest 异步设施原生（`futures::join!` / `select!`），经 `metis_plugin::futures` 门面 re-export 钉版（E3 拍板 = 宿主零同步原语、同步归 guest 语言设施；作者自加依赖自由 = design-review B2）。exclusive = `metis_plugin::Exclusive<T>` async mutex（E3 拍板 = SDK async mutex 助手；动机实证 = S2 spike R2b：跨 await 读改写丢失更新，[wasm-research](../research/wasm-research.md) 补遗 7）；wait-for 可见性上报宿主 = 扩展位（E3 拍板原话：上报是仪表不是机制），v1 不做。

形态先例教训：Spin 在 wasip1/2 时代被迫 SDK 内置 executor 桥接（`spin-executor` crate），wasip3 组件模型原生 async 后整套删除（[sdk-precedent-survey](../research/sdk-precedent-survey.md) §3.3）——metis 起步即原生 async（S1 实证），跳过 executor 时代。宿主侧注脚：wasmtime `concurrency_support(true)` 下同步 export 调用亦被运行时拒绝（补遗 7 papercut 7），全 async 是宿主配置强制方向，SDK 面同步方法无存在意义。

## S5 D102 错误面 = Result 贯穿 + 三格封套 Error（2026-10-17 定案，T13-d 拍板）

作者面 = `Result<T, metis_plugin::Error>` 贯穿一切效应调用与钩子返回。`Error` = 三格封套：kind 封闭枚举 + message + `data: Option<Value>`（[wasm-first-design-review](../research/wasm-first-design-review.md) B5：err 封套语义全存活，形态类型化）；kind 定控制流——「禁 message 匹配做控制流」从纪律变结构（编译器强制）。服务四态（unavailable / method_missing / timeout / business）是 kind 枚举的服务子集；完整枚举 + core 族三格（deadlock / contract-violation / mount-failed）= [wit-abi](wit-abi.md) W5 D136–D141。guest 导出只许 `business`（W5 D138）。

淘汰项 = proxy-wasm 模式：host 失败大量 `.unwrap()` → guest trap（wasm 下 panic = abort = 杀实例，靠宿主重启兜底）——WIT `result` 一等错误通道下，失败以 `Result` 贯穿作者面（[sdk-precedent-survey](../research/sdk-precedent-survey.md) §1.6）。

分工登记：底层承载（WIT record/enum vs TLV 封套）归 [wit-abi](wit-abi.md) W5 章（T13-k）落定（已兑现 = D136），作者面形状不随承载变。本节现在就冻结的理由：SDK 骨架（S1/S4）的每个方法签名长在错误面之上。

## S6 D103 「world 固定」释读与能力映射原则（2026-10-17 定案，T13-d 拍板）

ADR-0034 §4「world 固定，能力白名单由 manifest 声明对账，作者无法越权加 import」与 design-review B1「import 清单机械抠出 → 能力对账 = 集合包含判定」的合成释读：

- world = 单一固定，import 词汇表封闭。「作者无法越权加 import」= 编译期结构事实：SDK 绑定之外无符号可链接；手工添加的进口在安装期结构对账被机械抠出（启用时响亮）——B1 集合包含判定的准确含义 = 组件 import 清单 ⊆ 宿主词汇表（防越权加 import），粒度到接口、不到能力实例（[wit-abi](wit-abi.md) W9 D180–D182）。import 不授权能力：能力使用由 manifest 声明 + 运行期门决定。
- 能力白名单 = manifest 声明 → 运行期能力门 fail-closed + 启用期对账（信任模型骨架 = design-review C8 存活）；越权调用 = trap（[wit-abi](wit-abi.md) W5 D139），不是 S5 类型化错误——S5 只覆盖已声明但运行期失败的路径。先例 = Spin（声明全在 manifest、SDK 零编译期检查、越权 = 「Destination not allowed」运行期类型化错误）与 Zed（运行期 CapabilityGranter 门控）——两大先例均选运行期门，SDK 面不设编译期能力检查机器。

映射风格三原则（本节定原则，具体接口集归 W3 章 = T13-j；**已兑现 = W3 D142–D154 + W6 D155–D162**，2026-10-19）：① 有生命周期 / 大对象实体 = 句柄 / resource 形态（Spin `key_value::Store::open_default()` 先例；metis = `process-handle` / `mount-handle` / `Blob` / `ByteReader` / `ByteWriter` / `Listener` / `TimerHandle`）；② 无状态服务 = ctx 方法/自由函数（Spin `llm::infer` / `variables::get` 先例；metis = `ctx.fs()` / `ctx.http()` / `ctx.timer()` / `ctx.service()` 等）；③ 原始生成绑定保留 escape hatch 但标注「正常不需使用」（Spin rustdoc 先例；metis = `metis_plugin::raw`）。生效层 = 运行期门（固定 world，宿主 import 实现逐调用 fail-closed；W3 D154）。

扩展位 = manifest 声明 → 编译期模块化组合 world（越权 = 编译期不存在）：先例无此形态；W2 已落地（2026-10-19），维持扩展位不启用——world 固定 = wit-abi W1.2 D117，能力声明 = wit-abi W2.5 D129 的运行期门。

## S7 D104 契约类型生成 = 编译期 proc-macro，时序随 W7 章（2026-10-17 定案，T13-d 拍板）

生成形态 = 编译期 proc-macro `metis_plugin::types!()`（与 `#[metis::plugin]` 同宏 crate），读契约包（types:/events: 节 = E5① 瘦身形状 DSL）生成 Rust 类型——SSOT 单写、无 check-in 生成物。淘汰项 = CLI 生成 .rs check-in（SSOT 双写 + 生成物腐化面）。先例两面：Extism 无 IDL 时编码错位运行期才炸、补救 = XTP Bindgen 外挂生成器——证明类型生成是刚需、外挂形态是反例；Zed ABI 面以 JSON string 承载演进型数据、Rust 面 serde 自由——「线格式稳定 + 类型面演进」的分层先例。

**W7 兑现（2026-10-19，D170）**：生成覆盖 = 契约四节（`types:` / `methods:` / `errors:` / `events:`）+ manifest `config:` schema，不只 types/events；映射与来源解析 = [wit-abi](wit-abi.md) W7 D170；typed decode 直走 TLV；`any` → `metis_plugin::Value`；反向生成（Rust → schema）与契约 → WIT 投影不立（后者 = 扩展位）。

时序：T10.14 v1 先落 `Value` 手动面（不闸站）；类型生成随 [wit-abi](wit-abi.md) W7 章（T13-m，瘦身形状 DSL 解析器产出）进站；`any` 回落 = `metis_plugin::Value`（即三分法②拍板的 `metis::Value`；guest 侧 Value 的家 crate 归 T10.13/T10.14 实现期定）。

## 联动登记汇总（章末）

| 去向           | 登记内容                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| W1 章（T13-h） | **已兑现（2026-10-18）**：WIT 目录 = `crates/metis-plugin/wit/`（D117）；生命周期钩子 = `setup` / `on-config` / `dispose`（D118）                                                                                                                                                                                                                                                                                                               |
| W2 章（T13-i） | **已兑现（2026-10-19）= wit-abi W2 D125–D134**：能力声明节 = D129；编译期组合 world 维持扩展位（不启用）                                                                                                                                                                                                                                                                                                                                        |
| W3 章（T13-j） | **已兑现（2026-10-19）= W3 D142–D154**：metis 功能 import = 9 个（`events` / `services` / `timer` / `http` / `fs` / `process` / `instance` / `blobs` / `streams`），private fs = WASI 0.3 `wasi:filesystem`（作者面 `ctx.plugin_fs()`）；S6 三原则落地（resource / ctx 方法 / `metis_plugin::raw`）；能力层 = 运行期门；`Ctx` 零尺寸 + 句柄 RAII；crypto 撤回 = [ADR-0041](../decisions/0041-crypto-capability-withdrawal.md)（作者自加 crate） |
| W6 章（T13-l） | **已兑现（2026-10-19）= W6 D155–D162**：作者面 `Payload`（value / blob 双车道）/ `Blob` / `ByteReader` / `ByteWriter` / `Value::Bytes`；typed decode 输入；大负载 = blob / reader                                                                                                                                                                                                                                                               |
| W5 章（T13-k） | **已兑现（2026-10-19）**：错误底层承载 = [wit-abi](wit-abi.md) W5 D136–D141；S5 作者面投影不变（三格封套 + `ErrorKind` 1:1），S5/S6 的 kind 面与越权错误已按 W5 D137/D139 同步                                                                                                                                                                                                                                                                  |
| W7 章（T13-m） | **已兑现（2026-10-19）= wit-abi W7 D163–D172**：契约包 = `contract.yml` + 文档；`types!()` 覆盖契约四节 + config schema（D170）；`map` / `any` 保留（Value 形状面）                                                                                                                                                                                                                                                                             |
| W8 章（T13-n） | **已兑现（2026-10-19）= wit-abi W8 D173–D179**：`on_event` catch-all 定形；SDK 默认 `Ok(None)`；声明面 = 审批面；强类型事件方法维持 S7 扩展位                                                                                                                                                                                                                                                                                                   |
| T10.13         | metis-codec = Value/TLV 双 target 依赖（S3/S7）                                                                                                                                                                                                                                                                                                                                                                                                 |
| T10.14         | S1–S7 实现站；D11 声明随站                                                                                                                                                                                                                                                                                                                                                                                                                      |
| T13-e/T10.15   | 薄插件地板承接（S1）：脚手架模板藏 crate-type/target/钉版依赖                                                                                                                                                                                                                                                                                                                                                                                   |

## 扩展位登记（五条）

| 扩展位                             | 住处  | 触发器 / 前提                       |
| ---------------------------------- | ----- | ----------------------------------- |
| 内建件便利 re-export（feature 面） | S3    | 首批插件重复手写同一 serde 组合成串 |
| wait-for 可见性上报宿主            | S4    | 首个跨插件死锁定位需求              |
| no_std / build-std 裁剪            | S3    | 插件二进制体积/冷启动投诉成串       |
| 编译期模块化组合 world             | S6    | W2 章 manifest 格式落地后重估       |
| 强类型事件/服务方法生成            | S1/S7 | W7/W8 已兑现；首个强类型方法需求    |
