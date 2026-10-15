# 0034. 插件载体全面转向 wasm 组件（wasm-first）

- Status: accepted（整篇取代 [ADR-0021](0021-extension-language-runtime.md)；[ADR-0024](0024-carrier-layering.md) §1 分层定妆与 §2 盒的形态由本 ADR 局部取代（§3–§8 继续有效，§5/§6 适用范围扩展为全插件）；整篇取代 [ADR-0026](0026-luau-abi.md)——[luau-abi](../design/luau-abi.md) 归档 archived；ABI 重设计开章 = [wit-abi](../design/wit-abi.md)（living doc，W 系列））
- Date: 2026-10-15

## Context

**权重变更（2026-10-15 维护者拍板，本轮选型复审的合法输入）**：Creator 模式撤销约束地位（「不是重点，甚至是可以不考虑的」）；选型重点 = 热插拔 + 代码传播度 + 抗锁定；Luau 在非游戏生态冷门且选定难调头；wasm 语言中立 = 生态上限。推论：[ADR-0021](0021-extension-language-runtime.md) 淘汰表 wasm 行的死刑判决前提（E1：Creator 部署内无编译工具链）随 Creator 降权而撤销——判决在其语境下仍然成立，语境已变。

**证据基线**（[wasm-research](../research/wasm-research.md) 补遗 5，2026-10-15 核实）：WASI 0.3 已发布（2026-06-11，async 原生进 canonical ABI）；wit-bindgen 0.63.0（async 自 0.45 起为 default 正式特性）；wasip3 crate 0.9.0+wasi-0.3.0；`wasm32-wasip3` rustc target 入树（Tier 3，std 迁移过渡中）；Extism 判决 = 设计参照非技术选项（私有 ABI / 字节过内存 / 单厂商治理）；「0.3 零拷贝」语义修正 = 四层（值永远 1 次拷贝 / stream 每块 1 次 / 流转发中介 0 次 / resource 句柄真 0 次）——「中介 0 次」对本项目宿主中转架构最对口。

**保留资产**（语言无关，零改动平移）：fiber/actor/重启语义（ADR-0010/0011/0023）、Value 模型（ADR-0015/0022）、事件与服务语义（ADR-0013–0017/0018）、配置层（ADR-0002/0006/0007/0027）、实例模型（ADR-0028）、journal（ADR-0029，VM 族拦截点改指 wasm VM 操作）、接缝能力集语义（ADR-0025）、遏制四约束（ADR-0005，wasmtime 原生机制，实测 = wasm-research 补遗 1）。

**维护者拍板约束（2026-10-15 原话锚，verbatim）**：「我们应该全面转向使用 wasm，不再考虑使用lua的任何形式了」；「wasi+wit，这个是非常恶心的地方，我们开发的时候需要将其包起来，做成sdk提供给用户，让用户完全不知道wasi和wit，使用对应语言的软件包开发即可」；「目前似乎只在rust上有完美体验，其他语言的支持似乎都比较蹩脚。我们可以优先支持rust，其他语言可以后续再陆续支持」；「使用其他语言之后，就无法实现即写即用了，我们需要想一个优雅的工程方案来让这个过程不用太麻烦，更优雅」。

## Decision

### 1. 插件唯一载体 = wasm 组件

Lua 的任何形式出局（Luau 直写通道关闭；mlua 从未进入依赖图，无退出项——真实待处置物 = 空壳 crate `crates/metis-luau`，删除/改向登记归台账 T13-b；[ADR-0008](0008-creator-mode-self-modification.md) Creator 前提削弱登记，其本体地位复审归后续议题）。每插件独立组件实例（Store+Instance），实例即弃热换（实测 pooling 1.7 µs）。

### 2. 分层收敛

插件（组件）/ 核心内建件 / 外部服务 / T2 例外通道——ADR-0024 §1/§2 的管盒两层合并为单层；盒的概念消解进插件本体；内建件判据（原 §4 四判据）与 T2 例外通道地位（原 §7）继续有效。

### 3. ABI = WIT + canonical ABI

WIT 包入仓 SSOT、版本随 ADR 演进，package version 承接 abi_rev 纪律（原 A10 平移）；递归 Value 走线编码/句柄化（补遗 4 实测 WIT 拒绝递归类型、补遗 5 §5.4 修正零拷贝语义）；大负载零拷贝 = resource 句柄族；WASI 标准件（clocks/random/filesystem preopen 模型）直接复用进能力面。

### 4. SDK 原则

插件作者不见 WASI/WIT——官方 SDK crate（`metis-plugin`）宏藏 ABI（`#[metis::plugin]` + trait 实现形态；先例：proxy-wasm SDK / Zed `zed_extension_api` / Spin / Extism PDK）；world 固定，能力白名单由 manifest 声明对账，作者无法越权加 import。

### 5. 语言梯队

Rust 一等公民先行（当前唯一体验完整的工具链——维护者原话带 hedge「似乎只在 rust 上有完美体验」，工业先例 = Zed/Spin）；后续 JS/TS（jco/ComponentizeJS，快车道候选）→ Go（TinyGo）→ Python（componentize-py）→ C#（componentize-dotnet）——每语言进站各带工效学 spike，不设日历承诺。

### 6. 即写即用工程面

`metis dev` watch 守护（保存即部署 = 主答案；先例 = spin watch / wash dev / Zed dev extension）+ 构建缓存 + 部署内工具链服务（可选件，Creator 模式的体面退场形态）+ 远端构建场（marketplace 配套，与确定性构建信任链同管线）。

### 7. 信任模型升级

ADR-0024 §6 五道防线适用范围从盒层扩展为全插件（进口对账 / 行为透明 / 来源链 / 分级信任 / 纪律兜底）。

### 8. 落地纪律

S1 spike（guest async 工效学：async import 往返 + 单宿主线程多实例并发在飞）先行，结果回填 Consequences W1 行并作为 wit-abi W4 章开工输入；T10 站表修订（T10.3/9/10/11 + 新增 SDK/构建管线站）随本 ADR 同批或紧后批落地。

## Consequences

**风险登记**：

| #  | 风险                                                                         | 处置                                                                                                                                                                                                                                                                                                            | 复审触发器                                                                   |
| -- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| W1 | guest async 工效学（A4 等价交错语义映射组件模型 async 的表达力）             | **S1 spike 已验证 = PASS**（2026-10-15，wit-bindgen 0.63 / wasmtime 49.0.3 / rustc 1.98.1 + wasm32-wasip2：guest 内并发 501 ms ≈ 500 ms、单宿主线程双实例并发在飞 502 ms、guest 样板 31 行；证据 = [wasm-research](../research/wasm-research.md) 补遗 6）；兜底 = Extism 式 core-module 私有 ABI（补遗 5 §5.2） | 残留摩擦项（wasip2 std 拖带导入 / 文档缺口）归 wit-abi W3/W4 章与 SDK 站消化 |
| W2 | 薄插件仪式感地板（单文件零仪式不可恢复；地板 = 文件 + 构建命令 + 秒级）      | JS 快车道 + `metis dev` 承接                                                                                                                                                                                                                                                                                    | 作者投诉成串 → 快车道优先级前置                                              |
| W3 | 组件模型演进中途（0.2→0.3 过渡、wit-bindgen/wasm-tools/wasmtime 版本对齐痛） | WIT 包 SSOT + 钉版纪律（原 ADR-0024 C3 平移）                                                                                                                                                                                                                                                                   | 组件模型 1.0 发布 → 评估                                                     |
| W4 | wasmtime 依赖体量（Cranelift 依赖树大）                                      | 进项目走 adding-dependencies 流程（原 C5 平移）                                                                                                                                                                                                                                                                 | —                                                                            |
| W5 | fuel 计量 CVE 史                                                             | 月度列车跟进 + deny 闸门（原 C1 平移）                                                                                                                                                                                                                                                                          | —                                                                            |
| W6 | shared-memory 与限额/池化不兼容                                              | 配置禁用（原 C2 平移）                                                                                                                                                                                                                                                                                          | —                                                                            |
| W7 | Rust `wasm32-wasip3` target Tier 3 / std p3 迁移过渡中                       | 落地形态 = wasip2 target + wit-bindgen async（S1 实证，补遗 6）；wasip3 crate 直调 = 过渡可选件，**未验证**，归 W4 章开工输入                                                                                                                                                                                   | target 升 Tier 2 / std 迁移完成 → 迁移评估                                   |

**复审触发器**：S1 失败；组件模型 1.0；PolkaVM 类替代技术成熟；marketplace 纯 Rust 整插件需求（原扩展位激活评估随转向消解——插件本就全是组件）。

**关联修正**（同一 change 落地）：ADR-0021/0024/0026 Status 行；[luau-abi](../design/luau-abi.md) 归档 archived；[wit-abi](../design/wit-abi.md) 开章；台账 T13 立 + T10 方向变更注；AGENTS.md 文档清单。

**遗留议题登记**：seam-capabilities / context-events / instance-model 的 Luau 措辞修订注随 WIT ABI 设计批；TS→Luau SDK 轨道扩展位（ADR-0021 登记）随转向废止，TS 承接改走第 5 条语言梯队的 JS/TS 快车道。
