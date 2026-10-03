# 0021. 扩展语言运行时选型：Luau + mlua

- Status: accepted
- Date: 2026-10-03

## Context

**产品模型前提**（2026-10-03 用户纠正——此前文档从未明写，是本次选型评估的前提）：Metis core = 插件管理与组合机器 + syscall 层 VM 基底；core **不提供任何 agent 相关服务**。llm / tools / fs / session / memory 等一切领域功能 = 插件：扩展者自写，或组合他人插件；发行可捆绑参考实现，可替换。推论：**插件语言的上限 = 生态实现能力的上限**，选型必须在该模型下评估。

插件作者三语境（`../design/service-registry.md` §7）：平台开发者（monorepo）/ 第三方插件作者（自有 git 仓库）/ agent（Creator 模式，[ADR-0008](0008-creator-mode-self-modification.md)——部署内直接创作，**无编译工具链可用**）。更新模型：常态更新 = 插件热更新；核心自身更新 = 进程级干净重启（[ADR-0023](0023-core-restart-semantics.md)）。

结构性约束（已冻结）：每插件独立 VM 实例、卸载零残留、内存限额、指令级中断（[ADR-0005](0005-vm-topology.md) / [ADR-0011](0011-actor-execution-model.md)）；值边界 = 可序列化纯数据（[ADR-0015](0015-payload-value-model.md)）；显式 API 纪律、驱逐元编程拦截手法（[ADR-0013](0013-context-scope.md) / [ADR-0014](0014-dispatch-semantics.md) / [ADR-0016](0016-internal-hooks.md)）。

评估框架（八条，取自冻结 ADR 的要求）：

- **E1 插件载体 = 无工具链文本**：Creator 模式 agent 直接写文件即插件
- **E2 实例成本与卸载彻底性**
- **E3 故障遏制**：内存限额 + 指令级中断
- **E4 沙箱默认姿态**：无 ambient authority，能力 = 宿主显式注入
- **E5 热更循环速度**：编译快、旧字节码留存回滚
- **E6 值边界可序列化**
- **E7 作者体验**：20 行零仪式、LLM 写得动、类型系统与 LSP
- **E8 Rust 嵌入面成熟度与上游可持续**

被淘汰的选项（判决来源三类：用户判决 / 冻结约束 / 评估淘汰）：

| 选项                                   | 判决来源 | 淘汰理由                                                                                                                                                                                                                                                                                                                        |
| -------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| WebAssembly 组件模型（wasmtime）       | 用户判决 | 限制多、上限低；源语言 → wasm 必过编译工具链，Creator 模式部署内无工具链可用，正面违背 E1                                                                                                                                                                                                                                       |
| 子进程插件协议（VSCode 扩展宿主模式）  | 用户判决 | 性能不足：每次调用 IPC + 序列化；进程监管与部署重量                                                                                                                                                                                                                                                                             |
| 原生动态库（cdylib + dlopen）          | 冻结约束 | 无稳定 ABI；`dlclose` 卸载不安全（残留引用 → use-after-free；业界惯例永不卸载 = 泄漏，时间组合性死亡）；插件 panic/segfault 跨 FFI 炸穿 host；无内存限额与指令中断，逐项违反 [ADR-0005](0005-vm-topology.md) / [ADR-0011](0011-actor-execution-model.md)；Creator 模式 agent 必然写出崩溃代码，不可信代码进程内原生运行不可防守 |
| 每语言嵌一种 VM                        | 评估淘汰 | 每种语言都要各做一遍完整 ABI（ctx / 事件 / 服务 / 值转换 / 工具链），复杂度爆炸                                                                                                                                                                                                                                                 |
| PUC Lua 5.4/5.5                        | 评估淘汰 | 有原生 64 位整数（5.3 起），但无类型系统、无沙箱纪律、演进近乎冻结、无 require-by-string 模块系统、解释器性能弱于 Luau                                                                                                                                                                                                          |
| LuaJIT                                 | 评估淘汰 | 事实遗产化：锁定 Lua 5.1 语义、长期无正式版本                                                                                                                                                                                                                                                                                   |
| V8 / deno_core                         | 评估淘汰 | 二进制 +30 MB、isolate 内存 MB 级、生态动荡                                                                                                                                                                                                                                                                                     |
| QuickJS（quickjs-ng 分叉）             | 评估淘汰 | BigInt / 原生 class / 原生 async 是真实优势；但 JS 元编程能力（Proxy / 原型链 / getter）与显式 API 纪律（[ADR-0013](0013-context-scope.md) / [ADR-0014](0014-dispatch-semantics.md) / [ADR-0016](0016-internal-hooks.md)）正面冲突；ng 分叉治理年轻；无类型系统                                                                 |
| Python（PyO3 嵌入）                    | 评估淘汰 | GIL、部署重、著名不可沙箱——agent 写的代码要在部署内运行，此项即死刑                                                                                                                                                                                                                                                             |
| Rhai / Koto / Starlark / Wren 等小语种 | 评估淘汰 | 生态单薄、无沙箱实战、LLM 语料稀薄；Starlark 定位为配置/构建语言，表达力故意受限                                                                                                                                                                                                                                                |
| 纯配置无运行时                         | 评估淘汰 | 表达力不足（工具需要逻辑）                                                                                                                                                                                                                                                                                                      |

## Decision

- **扩展语言运行时 = Luau**（每插件独立 `lua_State`，[ADR-0005](0005-vm-topology.md)）；Rust 绑定 = **mlua**（`vendored` 锁定版本）。兜底退路 = 直绑 Luau C API——API 面小且稳定，已评估可行。
- **多语言能力的三难结构**：任意语言 + 进程内热更 + 故障遏制，三者最多取二。wasm / 子进程已被用户判决出局；dylib 被遏制哲学出局；多 VM 被复杂度出局——唯一自洽的落地形态 = **编译到 Luau 字节码的前端语言**。登记 **TS→Luau SDK 轨道**为规划扩展位：roblox-ts 已在生产规模证明 TS→Luau 可行（TS 有全套 class / 泛型 / FP）；v1 不做；Creator 模式 Luau 直写通道永久保留；运行时对前端零感知，任何时候引入都是纯增量。
- **插件语言三 ceiling 与承接机制**：
  - **语言人体工学上限**（class / 泛型 / FP）→ TS→Luau 前端承接；Luau 自身亦有 classes RFC 原型在开发中。
  - **运行时语义上限**（64 位整数）→ **上游已采纳**：`luau-lang/rfcs#153`（2026-02-12 合并；修订系列 #174 / #175 / #176 / #182 至 2026-03-25，Roblox 工程师主导）：原生 `integer` 类型、`123i` / `0xABABi` 字面量、完整 `integer` 库（算术 signed / unsigned 双版本、位运算全套、移位旋转、比较含 unsigned 变体、`minsigned` / `maxsigned`）、`buffer.readinteger` / `buffer.writeinteger`、`type()` 返回 `"integer"`、C API `lua_pushinteger64` / `lua_tointeger64` / `luaL_checkinteger64`。**实现已进发布线**（0.740 release notes 已含 `integer.idiv` 跨后端修复条目）；**默认出厂状态待实测**（可能在 feature flag 后）——由 mlua spike 验证。`Value` 侧对接 = [ADR-0022](0022-value-int64.md)。
  - **性能上限** → syscall 层原生内建件承接物理热点（json / crypto 等）+ agent 负载 IO 主导（[ADR-0011](0011-actor-execution-model.md) 估算：业务主延迟比派发开销高 5–6 个数量级）+ 上游 NCG 原生代码生成（mlua `luau-jit` 特性可开）。
- **syscall 层边界原则**（2026-10-03 用户拍板）：为满足核心层任务的 VM 基底原语可以提供；agent 相关服务一律不提供。**能力清单不在本 ADR 范围**，另立议题讨论。
- **证据快照**（2026-10-03 网络核查）：Luau 周更不断（0.732 → 0.741，2026-07-28 → 2026-10-02）；每期多名 Roblox 工程师 commit + 外部贡献者；0.738 加入 isolated heaps 嵌入者支持（与每插件独立堆用法对口）；`coroutine.finally` 原型（RFC #187）；新类型求解器与 exact table 实验持续投入；仓库约 5.9k stars。mlua 0.12.1（2026-08-29 发布）：累计约 7.4M 下载，2019 年至今持续维护，`luau` / `luau-jit` / `async` / `serde` / `vendored` 特性齐全。对照组 wasmtime 49.0.2（2026-10-02，Bytecode Alliance 月更）生态健康——已被判决，但与健康度无关。

## Consequences

风险登记与复审触发器：

| 编号 | 风险                                                                | 缓解                                    | 复审触发器                                  |
| ---- | ------------------------------------------------------------------- | --------------------------------------- | ------------------------------------------- |
| R1   | 异步桥接：coroutine 跨 host 挂起恢复与 actor 邮箱结合——最大技术风险 | mlua spike 验证                         | spike 失败 → 触发选型复审                   |
| R2   | mlua 单维护者                                                       | vendored 锁版本；直绑 C API 兜底        | 维护崩坏 → 复审                             |
| R3   | Roblox 外生态薄                                                     | 本项目禁 C 模块、能力全走注入，几乎无损 | —                                           |
| R4   | LLM 写 Luau 语料少于 JS / Python                                    | SDK 模板 + luau-lsp 反馈环              | Creator 模式实证失败率显著偏高 → 拿数据复审 |
| R5   | 上游方向剧变（闭源 / 许可证），概率低                               | supersede 流程兜底                      | 剧变发生 → 走 supersede 流程                |

- **可逆性**：ABI 核心（`Value` / manifest / entry 树 / 服务 / 事件语义）全部是语言无关的数据；将来新增语言前端 = 编译器 / loader 前端的加法，非推翻
- **关联修正**（同一 change 落地）：`../design/service-registry.md` §1 / §6 / §7 措辞校正（消除"核心提供领域服务"的隐含前提）；[ADR-0015](0015-payload-value-model.md) 由 [ADR-0022](0022-value-int64.md) 局部取代
