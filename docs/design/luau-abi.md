# Metis — Luau ABI 设计（living doc）

> 状态：living design doc——ABI 议题地图 A 系列逐条收官入库；冻结 ADR 随 Luau ABI 固化（议题地图任务 3）一并立。
> 来源：ABI 议题地图 A 系列逐条讨论拍板。前提：产品模型（core = 插件管理与组合机器 + syscall 层 VM 基底，不提供 agent 领域服务）；载体分层 = [ADR-0024](../decisions/0024-carrier-layering.md)（盒管模型）；接缝能力集 = [seam-capabilities](seam-capabilities.md)。
> 相关 ADR：[0002](../decisions/0002-config-as-pure-data.md) / [0003](../decisions/0003-module-require-discipline.md) / [0004](../decisions/0004-plugin-forms.md) / [0006](../decisions/0006-config-schema-ssot.md) / [0007](../decisions/0007-yaml-config-subset.md) / [0011](../decisions/0011-actor-execution-model.md) / [0014](../decisions/0014-dispatch-semantics.md) / [0018](../decisions/0018-service-registry.md) / [0019](../decisions/0019-crate-layout.md) / [0023](../decisions/0023-core-restart-semantics.md) / [0024](../decisions/0024-carrier-layering.md)

---

## 0. 术语锚点（本文档新增；共享术语见 [seam-capabilities](seam-capabilities.md) §0）

| 术语          | 含义                                                                                                                       |
| ------------- | -------------------------------------------------------------------------------------------------------------------------- |
| 入口契约      | loader 加载插件入口模块后"第一眼"看到的形状：返回值形状、setup 签名、清理习语——插件作者每天要写的第一行代码                |
| 两形态        | 纯代码插件 `plugins/foo.luau`（无 manifest）/ 包插件 `plugins/foo/`（manifest + `entry:` 缺省 `main.luau`），ADR-0004 冻结 |
| `plugin()` 糖 | 宿主注入的全局函数：`plugin(fn)` ≡ `{ setup = fn }`——写码层便利，不是第二种契约形态                                        |
| manifest      | 包插件的自述文件 `manifest.yml`（§A2）：我是谁/怎么加载我/我需要什么/我提供什么/我带了什么                                 |
| 配置树        | 部署者的装配单（`metis.yml` 等价物）：装哪些插件、每个实例给什么 config 值、挂在哪——归任务 4，≠ manifest                   |
| 契约包        | `lib/` 里的契约库：服务契约（方法集 + schema + 文档）独立于提供方存在（Seam 三角，ADR-0018 S6）                            |

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

## A2 manifest 格式（2026-10-06 收官）

### A2.0 边界：manifest ≠ 配置树

- **manifest** = 插件的**自述**（作者视角、随包走）：我是谁（name/version/description）、怎么加载我（entry）、我需要什么（inject/capabilities/config schema）、我提供什么（provides）、我带了什么（boxes）。
- **配置树**（`metis.yml` 等价物）= **部署者的装配单**：装哪些插件、每个实例给什么 config 值、挂在哪——归任务 4（配置格式设计）；惟 config 值的**形状契约**（如 A2.5 的秘密引用语法）随 schema 校验归本议题。
- Cordis 无独立 manifest（元数据背在插件对象上）；独立成纯数据文件的理由 = ADR-0006：Rust 必须读懂——加载前校验、审批展示、激活对账都发生在跑插件代码之前。
- **消费方全景**：loader（加载校验）→ 安装事务/审批（能力清单展示）→ 激活门（inject/provides 对账）→ 运行时能力门（强制）→ marketplace（分发展示）→ `metis sdk` analyze（静态分析）。

### A2.1 字段全集与未知字段政策

v1 顶层字段（封闭集）：

| 字段           | 缺省        | 一句                                           |
| -------------- | ----------- | ---------------------------------------------- |
| `name`         | 目录名      | 插件身份（ADR-0006）                           |
| `version`      | 无          | 分发用；semver 约束归任务 9                    |
| `entry`        | `main.luau` | 入口模块（ADR-0006）                           |
| `description`  | 无          | 审批/市场展示                                  |
| `config`       | 无          | config schema（ADR-0006 + A2.5 秘密标注）      |
| `provides`     | 无          | 服务面（A2.5）                                 |
| `inject`       | 无          | 依赖面（A2.5）                                 |
| `capabilities` | 无          | 能力面（A2.5）；纯代码插件无能力门资格（A1.3） |
| `boxes`        | 无          | 盒清单（A2.5）                                 |

- **身份规则**：`name` 显式值必须等于目录名——v1 插件身份 = 路径名（config 名字空间与前缀约定均建立其上），不一致 = 响亮报错；改名需求归任务 9 包身份议题再审。
- **有意不收**：authors/license/repository 等市场元数据（任务 9 立项再加）；`events`（事件静态声明归 A8）；`abi_rev`（A10）；`runtime`（A2.5）；`min_core`（提案否决——版本错配由未知字段报错带"core 版本过旧？"提示兜底，主演进通道归 A10 ABI rev，不为诊断改进增复杂度）。
- **未知字段 = 响亮报错**（顶层与嵌套同政策；报错带字段名 + "core 版本过旧？"提示）。主场景 = 手写/agent 生成的 typo——静默忽略会把"拼错的能力声明"变成"运行时莫名 deny"，报错远离病根。

### A2.2 设计原则二条（本轮沉淀，后续议题沿用）

- **错误尽早 = 校验器单一实现、多面暴露**。manifest 校验是 metis-loader 的纯数据算法（ADR-0019 纯 crate），同一实现暴露三面：编辑器实时（导出 schema 工件挂 yaml-language-server，打字即红线）→ `metis sdk check`（写码期/CI，报错措辞与加载期逐字一致）→ loader 加载期（兜底）。`sdk new` 脚手架从合法模板起步。与 seam §1.5 四层防线同一哲学在 manifest 侧的镜像；工具面归 A10。
- **声明 ≠ 观察**。manifest 承载**作者意图**，不是源码的扫描产物：意图型内容（description、required/default/secret、必需/可选）机器推不出；事实型内容（方法集、能力使用）虽可扫描，但全观察化 = 对账退化为自证、漂移检测死亡——双声明的价值恰在意图与事实的间隙（漂移 = bug 显形；TypeScript 边界标注类比：标注是被检查的意图）。机器的正确角色 = **生成初稿 + 对账漂移**（脚手架/analyze 建议草稿、作者确认意图；analyze 写码期炸漂移），不替作者表意图。

### A2.3 名字空间与撞名

- **config 撞名按构造免疫**：值挂在各插件自己的 entry 子树下，schema 在各插件自己的 manifest——同名键互不相干（同名局部变量类比）；插件名部署内唯一（loader 按名加载）顺便成为 config 的名字空间。多插件共享配置 = 显式共享设计，归 session 议题。
- **service 扁平名 = 可替换性的代价**：强制按提供方命名空间化会杀死 seam 三角（消费者写死提供方、实现不可换）。正确目标 = 概率趋零 + 撞语义可机械检测 + 真撞本地解：
  1. **概率趋零**：`plugin-name/service` 前缀约定（ADR-0018 S1"约定不强制"）——插件名本已唯一，作者只在自己名字空间起名则撞名结构性不可能；引导落在 `sdk new` 模板默认值里，不落规范权力（命名自由归作者，无中心注册）。
  2. **撞语义可机械检测**：正式契约住 `lib/` 契约包带 schema（A7）；同名不同 schema = 激活对账响亮失败——语义漂移从运营事故变成安装期一行报错。
  3. **真撞本地解**：时刻唯一 → 安装期响亮；部署者二选一 / isolate 作用域共存 / 部署侧重命名（后两者细节归任务 4），全程不改源码。
- **发现性归任务 9**：marketplace 契约目录（"起名前查得到"）是服务不是审批；v1 无分发面，部署内名字空间小、撞名率低。
- **故意同名 = 契约竞标（特性）**：两个实现同对一份契约 schema 对账、语义一致 → 可替换；意外同名 = 撞名 → 响亮。schema 对账 = 区分竞标与撞名的机械裁判。

### A2.4 契约治理三层

趋同力量 = 引力中心不是闸门（历史对照：Rust std + crates.io / Go stdlib + x，而非早期 npm 或审批制）：

| 层                 | 谁定义                                                     | 收敛机制                                                                                                                                                     |
| ------------------ | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 平台契约（第一方） | 平台 `lib/` 内嵌层，**契约版本 ≡ 核心版本**（ADR-0019 C3） | 入库标准苛刻：跨插件高共享 + 趋同比实现自由更重要 + 平台长期维护；清单少而精（llm/memory/session 级）；演化规则 = ADR-0018 S6（加方法兼容/删改破坏 epoch+1） |
| 生态契约（第三方） | 任何人可写契约包（纯 schema + 文档，无实现）               | 采用度 + marketplace 发现性（任务 9）；与内嵌层同名冲突响亮（ADR-0019）                                                                                      |
| 插件私有服务       | 作者文档约定                                               | 无需趋同                                                                                                                                                     |

- 核心不**实现**领域服务（红线不动）；平台**定义**领域词汇并捆绑可替换参考实现（默认插件集，ADR-0024 §4）。
- **混乱防火墙 = 分歧无法静默**：同名不同 schema = 对账爆炸，分歧没有机会腐烂成生态问题。
- **定义权与采纳权分离**：部署者可用 isolate 跑自己的契约；生态可挑战平台契约——引力中心可被拉动。

### A2.5 字段形状

- **config**（ADR-0006 形状 + 秘密扩展）：
  - 三方分工：作者定义 schema（核心只定词汇表 type/required/default/volatile/secret，零领域键）→ 部署者填值（配置树）→ 核心加载前校验注入（缺 required/类型错/多给键 = 响亮失败，此时插件代码一行未跑）。
  - **引用式秘密**：schema 标 `secret: true` → 配置树该键只许写引用 `{ secret: "openai-prod" }`，**写明文 = 校验响亮拒绝**（防呆）；真值住部署根 secrets store（gitignored/权限锁/人类手写层机器永不写，ADR-0007 分层纪律延伸）；loader 解析引用（查不到 = 响亮失败），真值只进该插件 VM；秘密值按**身份**注册进脱敏管线（精确匹配，补 seam §6.2 模式匹配之不足）。store 细节归任务 4；脱敏注册归任务 6；轮转 v1 走配置变更重挂路径。
- **provides = 全形 map**：`memory: { methods: [remember, recall] }`。淘汰简形 `memory: [...]`（字段集封死，契约引用 `contract: lib/memory` 将来无处落位；全形的 map 值为其预留兄弟字段，归 A7）。多服务 = map 多键。
- **inject = 双列表**：`required: [llm, memory]` / `optional: [cache]`。淘汰 map 带选项（v1 无第二维度，YAGNI；真出现时经 A10 ABI rev 演进）。
- **capabilities = map 形**（淘汰 list 形——域名白名单必须是一等数据，审批面逐条展示）：
  - `fs: read | write` 单值枚举（write ⊃ read；淘汰布尔对——非法态需额外规则堵）。无路径参数：v1 每插件私有目录天然隔绝；共享 workspace 归 session 议题。
  - `http: [api.openai.com, "*.anthropic.com"]` 域名白名单：精确 host + `*.` 子域通配（**任意深度子域、不含 apex**，apex 须单列；通配覆盖宽度 = 审批面审查点）；不表达端口。配套规则：**scheme = 缺省 https-only，明文须显式标记 `http://` 前缀**（审批面可见——本机 sidecar 正例 = `http://localhost`；写 `https://` = 响亮报错教学"https 为缺省，直接写 host"）；**SSRF = 解析结果非全局单播地址即拒**（覆盖 v4/v6 各非公开段与 `::ffff:` mapped 形态，比枚举段不漏），**白名单显式列出内网目标 = 显式授予**，判定对象 = 每次连接实际使用的**最终解析结果**、检查与 connect 之间不得重解析（防 DNS rebinding）；**跨域重定向重新对账**（新目标同样过上述两关，防 open-redirect 旁路）。
  - `sqlite: true`（私有 db，无参数）。
  - timer 无门（`every` 最小间隔 = 政策非门）；纯算内建件无需声明；spawn v1 不给。
  - **缺省 fail-closed**：节缺省 = 无能力；键缺省 = 无该能力；未声明调用 = 运行时响亮报错指路 manifest；`sdk analyze` 写码期对账。
- **boxes = 显式列表**：`boxes: [parser]`。**双向对账**：声明了没文件 = 响亮（打包不完整安装期炸，而非运行期）；有文件未声明 = 响亮（防残留盒无意加载）；`sdk analyze` 对账 Luau 侧 `box()` 调用（写码期炸）。淘汰目录惯例（纯观察无意图可对账——声明≠观察）；淘汰 map 形（per-box 参数无落位需求，YAGNI；能力收窄/资源限额若出现经 A10 ABI rev 演进）。
  - **盒能力 = 属主包络**：host imports 授予范围 = 属主 capabilities 同一面；**安装期进口对账**——盒自描述进口清单 ⊆ manifest 能力声明，超集响亮（ADR-0024 §6 能力笼落位）；per-box 收窄登记扩展位；时钟/随机 = 无门基础件。
- **`runtime` 字段 v1 不立**：入口永远 Luau、盒是制品非运行时，无组成可声明；真实消费场景 = ADR-0024 §2 wasm 一等插件扩展位，到来时自带声明需求，老核心靠未知字段响亮报错天然拒绝。

### A2.6 FAQ：能力门 vs agent 运行时审批（两层授权）

- 能力门 = **权限包络**（代码能做什么，potential，安装时一次性审批、运行时机器强制）；典型 agent 审批 = **动作裁决**（这次该不该，actual，调用时逐次人审）。两层互补，谁也不是谁的提前版（手机 OS 同构：manifest 权限 + "仅这一次"）。
- 静态层是 agent 系统的真正边界：无人值守自动行为（timer/事件链无现场人类）；自我进化能力扩张 = manifest diff = 审批闸门（ADR-0018 推论）；组合调用无单一意图现场。
- 动态裁决住**插件层策略机器**（ADR-0014 选择器插件模式：serial 认领 + 服务调用转交插人工环节，可替换）；核心不提供动态审批 syscall；**结构约束优先于动态裁决**（私有目录/域名白名单缩小残余裁决面）。
- 完整答案 = 静态包络 + 结构拦截（可选策略插件）+ journal 全程可审计。

### A2.7 周期调度（v1 不立一等 cron 字段）

- 周期意图 = **config 键约定**（作者 schema 自声明 `poll_interval` 带 default、部署者可覆盖、审批 diff 可见；"多久跑一次"是部署政策，不该由作者写死进包）；审计牙 = `timer_scheduled` journal 条目 + `sdk analyze` 硬编码间隔告警。淘汰 manifest 一等 `schedule` 字段（第三台调度机器，与 seam §5.5 梯度一易失定时原语/梯度二持久调度插件重叠，且把部署政策错绑进作者包）。
- **持久调度原则四件套**（消解"插件持持久调度"的概念错位，schedule 插件立项前提）：
  1. **意图数据持真相**——持久的是意图记录（sqlite 数据），不是定时器；
  2. **timer 为易失投影**——哪个 scheduler 在跑由谁投影，热更/重启后读记录重投影（复用 ADR-0023 启动路径）；
  3. **寻址走稳定词汇**——触发目标 = 事件/契约，不寻址插件实例（插件可换，同一契约接班）；
  4. **归属匹配意图层级**——插件级意图随插件死（连带清理，私有状态族同政策）；用户级意图由用户级长驻插件持有；无无主调度。
- 一等声明式调度需求复审归任务 4（与梯度二持久调度插件立项同批）。

### A2.8 联动登记汇总

| 去向                  | 内容                                                                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| A3（ctx 形态）        | inject 句柄装配形态（provides/inject 声明面的消费侧）；capabilities 的运行时强制点                                                    |
| A7（schema DSL）      | provides 的 `contract:` 引用位形状与版本戳；config schema 词汇表（secret 已立）                                                       |
| A10（工具形态）       | `sdk new` 模板（provides 前缀约定）/ `sdk check` 校验器三面暴露 / `sdk analyze` 对账（能力/盒/硬编码间隔）；boxes map 形演进；ABI rev |
| 任务 4（配置格式）    | secrets store 细节；isolate/重命名；一等 cron 复审                                                                                    |
| 任务 6（journal）     | 秘密值身份注册脱敏                                                                                                                    |
| 任务 9（marketplace） | 契约目录发现性；version semver 约束；市场元数据字段；包身份/改名复审                                                                  |
| schedule 插件立项     | 持久调度原则四件套（A2.7）                                                                                                            |
| 扩展位                | boxes per-box 收窄；`runtime` 字段（随 ADR-0024 §2）                                                                                  |
