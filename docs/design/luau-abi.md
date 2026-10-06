# Metis — Luau ABI 设计（living doc）

> 状态：living design doc——ABI 议题地图 A 系列逐条收官入库；冻结 ADR 随 Luau ABI 固化（议题地图任务 3）一并立。
> 来源：ABI 议题地图 A 系列逐条讨论拍板。前提：产品模型（core = 插件管理与组合机器 + syscall 层 VM 基底，不提供 agent 领域服务）；载体分层 = [ADR-0024](../decisions/0024-carrier-layering.md)（盒管模型）；接缝能力集 = [seam-capabilities](seam-capabilities.md)。
> 相关 ADR：[0002](../decisions/0002-config-as-pure-data.md) / [0003](../decisions/0003-module-require-discipline.md) / [0004](../decisions/0004-plugin-forms.md) / [0006](../decisions/0006-config-schema-ssot.md) / [0011](../decisions/0011-actor-execution-model.md) / [0018](../decisions/0018-service-registry.md) / [0019](../decisions/0019-crate-layout.md) / [0023](../decisions/0023-core-restart-semantics.md) / [0024](../decisions/0024-carrier-layering.md)

---

## 0. 术语锚点（本文档新增；共享术语见 [seam-capabilities](seam-capabilities.md) §0）

| 术语          | 含义                                                                                                                       |
| ------------- | -------------------------------------------------------------------------------------------------------------------------- |
| 入口契约      | loader 加载插件入口模块后"第一眼"看到的形状：返回值形状、setup 签名、清理习语——插件作者每天要写的第一行代码                |
| 两形态        | 纯代码插件 `plugins/foo.luau`（无 manifest）/ 包插件 `plugins/foo/`（manifest + `entry:` 缺省 `main.luau`），ADR-0004 冻结 |
| `plugin()` 糖 | 宿主注入的全局函数：`plugin(fn)` ≡ `{ setup = fn }`——写码层便利，不是第二种契约形态                                        |

## A1 插件包形态与入口契约（2026-10-06 收官）

### A1.1 入口返回形状：table 带 `setup` 单形态

- **契约**：入口模块必须 return 一个 table，`setup` 字段 = 插件体函数，签名 `setup(ctx, config)`。table 字段集封闭（v1 仅 `setup`），出现其他字段 = 加载期响亮报错（按 raw 字段判定；挂元表/`__index` 绕过同判——静态分析提前拦截归 A10 四层防线）。
- **`plugin()` 糖**：宿主注入的全局函数（与 `print`/`os.time` 同一批注入，seam §1.4 硬化顺序铁律覆盖），`plugin(fn)` 返回 `{ setup = fn }`；入口返回裸函数 = 响亮报错并指路 `plugin()`。**归一化发生在写码层，核心契约零归一机器**。对价：多一个宿主注入全局 + 作者层两种拼写 + SDK 类型定义维护面（归 A10）；收益：函数风格作者零 table 心智、Creator 场景最短路径。
- **淘汰的选项**：纯函数返回（无扩展位——将来给入口挂新行为字段即破契约）；函数/table 双形态归一（Cordis 路线：函数/类/`{apply}` 三形态靠 Registry 归一化抹平，归一机器的存在本身即多形态包袱的证据；且 table 形态引入"插件对象背元数据"的 Cordis 心智，元数据 SSOT 从结构保证降级为政策倡导）。
- **与 manifest 分工**：字段集封闭 → 元数据（name/version/config schema/能力声明）物理上唯一去处 = manifest，安装校验可机械执法。
- 命名 `setup` 沿用 [ADR-0002](../decisions/0002-config-as-pure-data.md) 既有用法（对应 Cordis 的 `apply`）；`plugin` 命名暂定，SDK 文档阶段可议。

### A1.2 setup/dispose 习语

- **自动记账为主**：经 ctx 注册的一切（事件监听/定时器/服务暴露/子 fiber/句柄/盒实例）= fiber 账本的自动条目，卸载自动摘除（Cordis 监听器所有权铁律 + ADR-0024 盒与属主同生共死）。
- **手动清理通道唯一**：setup 可**返回一个 disposer 函数**（可选；返回非函数非 nil = 响亮报错）。只覆盖框架看不见的语义收尾（flush 缓冲/持久化在途状态/告别消息）。返回值只有一格 → 一实例至多一条手动清理，重复注册在结构上不可能。
- **顺序**：setup 返回的 disposer 记账为**第一页**（早于 setup 体内一切注册）→ LIFO 下**最后执行**——先摘光监听/定时器，世界安静后再跑语义收尾（Cordis 验证的默认）。
- **错误处置**：disposer 抛错逐个吞掉、不中断其他清理；Rust `FnOnce` 结构性保证只跑一次。
- **dispose 可挂起**：disposer 统一异步签名（[fiber.md](fiber.md) §2）；dispose 里可等 ctx 效应调用（[ADR-0018](../decisions/0018-service-registry.md) S4"可 await 在途持久化"）；**结算期拒绝新账**——dispose 里再注册新监听 = 响亮错误。
- **无重启专有钩子**：热更新与核心重启同一清理契约（[ADR-0023](../decisions/0023-core-restart-semantics.md)）。
- **登记扩展位**：`ctx.on_cleanup(fn)` 注册式多清理——纯增量新 ctx 方法，出现真实需求再加。v1 不做的理由：组合写法（`return function() a(); b() end`）已覆盖多清理场景；注册式有**重复注册风险**（调用点落在重复执行路径即重复记账，后果作者自担）。

### A1.3 两形态统一面

- **严格统一，无特例**：两形态共享同一份入口契约（A1.1 返回形状、A1.2 清理习语、`setup(ctx, config)` 签名）。验收标准 = **成长路径零改写**：`mkdir foo/ && mv foo.luau foo/main.luau && touch foo/manifest.yml`（ADR-0004 名称解析以 manifest 存在为包形态锚点；空 manifest 全字段走缺省即合法，ADR-0006），入口文件一个字节不动、行为完全等价；manifest 字段只在需要其能力时才填。
- **淘汰的选项**：单文件"顶层脚本即 setup"特例——契约出现第三形态（校验/教学/工具链多认一种写法）；成长路径从"零改写"变"重构"（顶层代码须包回 setup 函数）。两关足够；它省下的仪式已被 `plugin()` 糖压到底。
- **差异面 = 只允许从"有无 manifest"派生**：

| 能力                            | 纯代码              | 包插件  | 出处                                           |
| ------------------------------- | ------------------- | ------- | ---------------------------------------------- |
| 入口契约                        | ✅ 同一             | ✅ 同一 | 本节                                           |
| config schema                   | ❌（config 恒空表） | ✅      | ADR-0004 / 0006                                |
| provide/inject                  | ❌（仅事件通道）    | ✅      | ADR-0018                                       |
| 子模块 `./x`                    | 无文件可引          | ✅      | ADR-0003                                       |
| `@lib/x` 共享库                 | ✅                  | ✅      | ADR-0003 / 0019                                |
| 能力门声明（fs/http/sqlite 等） | ❌                  | ✅      | 派生推论（见下）；字段面归 A2（manifest 格式） |
| 盒 `boxes/*.wasm`               | ❌                  | ✅      | ADR-0024 §2；声明字段归 A2                     |

- **派生确认**：纯代码插件无 manifest → 无能力门声明面 → fs/http/sqlite 等能力门方法对其**结构性不可用**（单文件形态 = 纯编排 + 事件通道 + 纯算内建件；碰效应 = 升级成包，成长路径纯文件搬迁）。与 ADR-0004"无 manifest → 无 config schema"推论方式同源；能力门字段面落地归 A2。
- **含盒包归位**：含盒包 = 目录里多了 `boxes/*.wasm` 制品的包插件，**不是第三形态**。入口永远 Luau；盒无插件身份、与属主同生共死（ADR-0024 §2）。盒的声明字段归 A2；调用桥形态归 A3（ctx 形态）/A6（边界转换）。

### A1.4 入口执行语义

- **串行位**：setup = fiber 的第一个邮箱回调（[ADR-0011](../decisions/0011-actor-execution-model.md) actor 串行铁律覆盖）；期间到达的事件/tick 排队，Active 后才处理。
- **允许挂起 + 加载总预算**：setup 内可写任意 ctx 效应调用（直线写法，seam §2.3 spike 已证）；加载有墙钟总预算（**数值 = 实现期配置**，与 seam §6.3 同档登记），超限 = Loading 失败 → interrupt + drain 半成品账 → Failed（复用 [fiber.md](fiber.md) §3 已冻结路径）。
- **淘汰的选项**：setup 禁挂起同步特区（"启动读持久化状态"是最普遍的 boot 模式，禁挂起 = 判它绕路刑，且制造 A1.3 枪毙过的特例语法区）；不设预算（病态 setup 串行等待可拖死启动链，inject 依赖方全卡 Pending）。
- **死锁红线**：setup 里安全的挂起对象 = **完成不依赖自身邮箱的调用**——ctx 效应调用（宿主执行、完成直接 resume 挂起协程，即 seam §0 "async 两跳" 的确切含义：宿主侧路由往返，非回插件自身邮箱排队）与服务调用（经宿主两跳路由到**提供方**邮箱——必需依赖的提供方经激活门保证已 Active、独立运转，可选依赖缺席即返 `Err(Unavailable)`；跨 actor 等待环只超时，ADR-0018 S5）。唯一死锁形态 = 等**自己**邮箱里一条消息（自己的 tick/事件）：setup 没跑完它进不来，它不来 setup 醒不了——actor 模型的直接推论，契约层面点破。
- **A3 注意项**：sleep 类 API 若提供，必须做成"宿主持表、到期直接唤醒协程"形态，绝不做成"等 tick 消息"——否则在 setup 里埋死锁雷。
- **登记**：热更请求撞上 Loading 中的 setup → 归插件热更新设计（默认参照 Cordis 惯性原则：进行中的加载跑完落定再响应新变化）。

### A1.5 require 宿主包装（A1 侧行为规则；实现归 A3）

语义基座 = **Luau 官方 require-by-string 全集**（Implemented RFC：[new-require-by-string-semantics](https://github.com/luau-lang/rfcs/blob/master/docs/new-require-by-string-semantics.md) + [amended-require-resolution](https://github.com/luau-lang/rfcs/blob/master/docs/amended-require-resolution.md) + [require-by-string-aliases](https://github.com/luau-lang/rfcs/blob/master/docs/require-by-string-aliases.md) + [abstract-module-paths-and-init-dot-luau](https://github.com/luau-lang/rfcs/blob/master/docs/abstract-module-paths-and-init-dot-luau.md)），**Metis 零私有解析语义**——官方工具链（luau CLI / luau-lsp 的补全、跳转、类型检查）开箱可用。

- **specifier 规则 = 官方全集**：`./x` `../x`（相对发起文件）+ `@alias/x`（`.luaurc` 的 `aliases` 表声明）；**裸名/绝对路径报错**（与官方行为一致——官方故意预留裸名命名空间给未来包管理器）。根锚定诉求：扁平包 `./x` 的解析点即根，是 v1 答案；深嵌套的根 alias 机制受 ADR-0019 单根 `.luaurc` 约束（见下），归 A10 设计。文件相对比根锚定更抗重构（子目录整棵搬家内部引用不断）。
- **Metis 叠加的裁剪仅两处**（对官方结果集的裁剪，非语义分叉）：解析结果不得逃逸插件根（`@lib` 共享库除外，[ADR-0003](../decisions/0003-module-require-discipline.md)）；运行期只认部署根一份 `.luaurc`——插件子树嵌套 `.luaurc` 运行期不生效（[ADR-0019](../decisions/0019-crate-layout.md)：alias 覆盖 = 契约欺骗攻击面；插件仓的 `.luaurc` 是开发期产物、不随包分发）。
- **解析跟官方**：省略扩展名（`.lua` 同理）；目录 → `init.luau`（其内部相对 require 按父目录解析——官方 abstract-module-paths 语义；回引包内子模块用 `@self/x`）；歧义即错（`x.lua`/`x.luau` 并存、`x.luau` 与 `x/init.luau` 并存）；`@lib` 大小写不敏感（ADR-0019）。官方保留位避开：裸 `@`（ADR-0019）与 `@self`（= 当前模块路径，不可覆盖）。登记：包入口本身用 `init.luau`（ADR-0004 `entry:` 可改）时，其内部相对 require 解析基上移到 `plugins/` 一级，与 `main.luau` 入口不同——SDK 文档写明。
- **不可 require 之物**：wasm 盒（上游 Luau 本就不能 require 任何非 Luau 制品——require 产物只有 Luau chunk，拉进发起方 VM 执行；盒走核心桥，见 A1.3）；数据文件（经 `ctx.fs` 能力门）。**require 只搬 Luau 代码。**
- **循环 require = 响亮报错**（跟官方现行：检测到环即 error）。上游有 cycle 支持 RFC（[support-for-cyclic-requires](https://github.com/luau-lang/rfcs/blob/master/docs/support-for-cyclic-requires.md)，export table 打结），但绑定尚未落地的 class 特性、仍是 proposal——**复审触发器**：上游 cycle RFC 落地。
- **入口模块归一**：loader 加载入口时登记进该 VM 的 require 缓存；包内回引入口拿到同一模块实例，不重复执行。
- **登记**：包级根 alias 诉求与 `.luaurc` 脚手架机制（须在 ADR-0019 单根约束下设计，如平台在部署根布线命名空间化 alias）归 A10（工具形态与 ABI 版本纪律）。

### A1.6 联动登记汇总

| 去向                | 内容                                                                        |
| ------------------- | --------------------------------------------------------------------------- |
| A2（manifest 格式） | 纯代码插件无能力门资格 → `capabilities` 字段面只服务包插件；盒声明字段      |
| A3（ctx 形态）      | sleep API 直接唤醒形态红线；require 包装器实现；盒调用桥面（与 A6 共定）    |
| A10（工具形态）     | 包级根 alias / `.luaurc` 机制（ADR-0019 单根约束）；`plugin()` 糖的类型定义 |
| 插件热更新设计      | 热更撞 Loading 中 setup 的处置（惯性原则参照）                              |
| 实现期配置          | 加载总预算数值                                                              |
| 扩展位              | `ctx.on_cleanup` 注册式清理                                                 |
| 复审触发器          | 上游 Luau cycle require RFC 落地                                            |
