# Metis — Luau ABI 设计（living doc）

> 状态：living design doc——ABI 议题地图 A 系列逐条收官入库；冻结 ADR 随 Luau ABI 固化（议题地图任务 3）一并立。
> 来源：ABI 议题地图 A 系列逐条讨论拍板。前提：产品模型（core = 插件管理与组合机器 + syscall 层 VM 基底，不提供 agent 领域服务）；载体分层 = [ADR-0024](../decisions/0024-carrier-layering.md)（盒管模型）；接缝能力集 = [seam-capabilities](seam-capabilities.md)。
> 相关 ADR：[0002](../decisions/0002-config-as-pure-data.md) / [0003](../decisions/0003-module-require-discipline.md) / [0004](../decisions/0004-plugin-forms.md) / [0006](../decisions/0006-config-schema-ssot.md) / [0007](../decisions/0007-yaml-config-subset.md) / [0011](../decisions/0011-actor-execution-model.md) / [0014](../decisions/0014-dispatch-semantics.md) / [0018](../decisions/0018-service-registry.md) / [0019](../decisions/0019-crate-layout.md) / [0023](../decisions/0023-core-restart-semantics.md) / [0024](../decisions/0024-carrier-layering.md)

---

## 0. 术语锚点（本文档新增；共享术语见 [seam-capabilities](seam-capabilities.md) §0）

| 术语          | 含义                                                                                                                                      |
| ------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 入口契约      | loader 加载插件入口模块后"第一眼"看到的形状：返回值形状、setup 签名、清理习语——插件作者每天要写的第一行代码                               |
| 两形态        | 纯代码插件 `plugins/foo.luau`（无 manifest）/ 包插件 `plugins/foo/`（manifest + `entry:` 缺省 `main.luau`），ADR-0004 冻结                |
| `plugin()` 糖 | 宿主注入的全局函数：`plugin(fn)` ≡ `{ setup = fn }`——写码层便利，不是第二种契约形态                                                       |
| manifest      | 包插件的自述文件 `manifest.yml`（§A2）：我是谁/怎么加载我/我需要什么/我提供什么/我带了什么                                                |
| 配置树        | 部署者的装配单（`metis.yml` 等价物）：装哪些插件、每个实例给什么 config 值、挂在哪——归任务 4，≠ manifest                                  |
| 契约包        | `lib/` 里的契约库：服务契约（方法集 + schema + 文档）独立于提供方存在（Seam 三角，ADR-0018 S6）                                           |
| `deps`        | setup 第三参数：宿主按 manifest inject 装配的依赖句柄表（冻结）。二分：ctx = 宿主契约面（人人相同）/ deps = manifest 派生面（各插件不同） |
| 烘焙          | 句柄生成方式：宿主按声明（注册表方法集 / 盒导出清单）逐方法生成 Rust 闭包装成冻结表；方法集快照随激活定格，epoch 重启刷新                 |
| scope         | fs 能力的范围标识三键：private（私域免绑定）/ workspace（部署绑定根）/ global（用户世界 − 两条豁免）                                      |
| 豁免          | carve-out：对一切 fs scope（含 global）读写同禁的两处——secrets store 与 journal                                                           |

## A1 插件包形态与入口契约（2026-10-06 收官）

### A1.1 入口返回形状：table 带 `setup` 单形态

- **契约**：入口模块必须 return 一个 table，`setup` 字段 = 插件体函数，签名 `setup(ctx, config)`（2026-10-07 §A3.3 修订：+`deps` 第三参数）。table 字段集封闭（v1 仅 `setup`），出现其他字段 = 加载期响亮报错（按 raw 字段判定；挂元表/`__index` 绕过同判——静态分析提前拦截归 A10 四层防线）。
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

- **严格统一，无特例**：两形态共享同一份入口契约（A1.1 返回形状、A1.2 清理习语、`setup` 签名以 A1.1 为准）。验收标准 = **成长路径零改写**：`mkdir foo/ && mv foo.luau foo/main.luau && touch foo/manifest.yml`（ADR-0004 名称解析以 manifest 存在为包形态锚点；空 manifest 全字段走缺省即合法，ADR-0006），入口文件一个字节不动、行为完全等价；manifest 字段只在需要其能力时才填。
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

| 去向                | 内容                                                                                                                                                  |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| A2（manifest 格式） | 纯代码插件无能力门资格 → `capabilities` 字段面只服务包插件；盒声明字段                                                                                |
| A3（ctx 形态）      | sleep API 直接唤醒形态红线；require 包装器实现；盒调用桥面（与 A6 共定）；+`deps` 第三参数（A1.1 签名修订）——**已落 §A3.6 / §A3.11 / §A3.12 / §A3.3** |
| A10（工具形态）     | 包级根 alias / `.luaurc` 机制（ADR-0019 单根约束）；`plugin()` 糖的类型定义                                                                           |
| 插件热更新设计      | 热更撞 Loading 中 setup 的处置（惯性原则参照）                                                                                                        |
| 实现期配置          | 加载总预算数值                                                                                                                                        |
| 扩展位              | `ctx.on_cleanup` 注册式清理                                                                                                                           |
| 复审触发器          | 上游 Luau cycle require RFC 落地                                                                                                                      |

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
  - `fs` = **scope map**（2026-10-07 §A3.7 修订：原为 `read | write` 单值枚举）：`{ private: read|write, workspace: read|write, global: read|write }`，简形 `fs: read|write` ≡ `{ private: … }`；write ⊃ read（淘汰布尔对——非法态需额外规则堵）。无路径参数：private 免绑定、workspace 部署绑定（任务 4）、global = 用户世界 − 两条豁免——SSOT = §A3.7。
  - `http: [api.openai.com, "*.anthropic.com"]` 域名白名单：精确 host + `*.` 子域通配（**任意深度子域、不含 apex**，apex 须单列；通配覆盖宽度 = 审批面审查点）；不表达端口。配套规则：**scheme = 缺省 https-only，明文须显式标记 `http://` 前缀**（审批面可见——本机 sidecar 正例 = `http://localhost`；写 `https://` = 响亮报错教学"https 为缺省，直接写 host"）；**SSRF = 解析结果非全局单播地址即拒**（覆盖 v4/v6 各非公开段与 `::ffff:` mapped 形态，比枚举段不漏），**白名单显式列出内网目标 = 显式授予**，判定对象 = 每次连接实际使用的**最终解析结果**、检查与 connect 之间不得重解析（防 DNS rebinding）；**跨域重定向重新对账**（新目标同样过上述两关，防 open-redirect 旁路）。
  - `sqlite: true`（私有 db，无参数）。
  - timer 无门（`every` 最小间隔 = 政策非门）；纯算内建件无需声明；`process: true` 无参数（2026-10-07 §A3.10 开门：原为 spawn v1 不给——命令不可预测，声明面不假装有粒度）。
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
| A3（ctx 形态）        | inject 句柄装配形态（provides/inject 声明面的消费侧）；capabilities 的运行时强制点——**已落 §A3.3 / §A3.2**                            |
| A7（schema DSL）      | provides 的 `contract:` 引用位形状与版本戳；config schema 词汇表（secret 已立）                                                       |
| A10（工具形态）       | `sdk new` 模板（provides 前缀约定）/ `sdk check` 校验器三面暴露 / `sdk analyze` 对账（能力/盒/硬编码间隔）；boxes map 形演进；ABI rev |
| 任务 4（配置格式）    | secrets store 细节；isolate/重命名；一等 cron 复审                                                                                    |
| 任务 6（journal）     | 秘密值身份注册脱敏                                                                                                                    |
| 任务 9（marketplace） | 契约目录发现性；version semver 约束；市场元数据字段；包身份/改名复审                                                                  |
| schedule 插件立项     | 持久调度原则四件套（A2.7）                                                                                                            |
| 扩展位                | boxes per-box 收窄；`runtime` 字段（随 ADR-0024 §2）                                                                                  |

## A3 ctx 形态与方法集（2026-10-07 收官）

### A3.0 边界与前提

ctx = 插件代码手里唯一的宿主句柄；Rust 侧形态 = `Context { fiber: FiberId, runtime: RuntimeHandle }`（[ADR-0013](../decisions/0013-context-scope.md)），身份随身，效应归属 / 能力门 / journal 拦截全挂在它上面。本题定**容器形态 + 方法清单 + 参数/返回形状**；不归本题：挂起与交错细则（A4）、失败封套形状（A5——本节示例的失败面一律从简）、schema 对账（A7）、类型定义（A10）。前提（已定不议）：编排面 = 事件四模式 + provide/inject（[ADR-0014](../decisions/0014-dispatch-semantics.md) / [0018](../decisions/0018-service-registry.md)）；`setup(ctx, config)` 签名（A1.1）；经 ctx 注册的一切自动入账本（A1.2）；inject 只写 manifest（ADR-0018 S3）；sleep 红线（A1.4）。

### A3.1 容器形态：frozen table

- **决策**：ctx 及一切宿主装配句柄（deps 句柄、盒句柄、process 句柄）= **宿主构建的 table，递归只读化**（`set_readonly`，与 stdlib 硬化同款机制，seam §1.4）；无元表、无 `__index` 魔法；方法 = 宿主逐个安装的 Rust 闭包（身份闭包捕获）。写即响亮报错。
- **淘汰 userdata**：① 方法集按 Rust 类型静态注册——deps/盒句柄的方法集是运行期按注册表/导出清单生成的，userdata 只能退回 `__index` 动态分发，反而引入隐式行为（ADR-0013 杀 Proxy 魔法的同向选择）；② `pairs`/`print` 不可内省，Creator 场景 agent 失去运行时自探索面；③ Luau 类型系统对 userdata 形状的表达弱于 table type（A10 类型定义成本）。userdata 留给真正有不透明原生状态的东西——流式句柄族（A6 再审）。
- **防篡改等效**：递归只读 + 无元表 ⇒ 与 userdata 同档（rawset 响亮报错；debug 库已裁，seam §1.3）；残留“遮蔽”（插件自造表指过来）只影响自己。**安全强制点与容器无关**：能力门 / journal / 归属全在 Rust 宿主侧。

### A3.2 顶层组织

```
ctx
 ├─ on / emit / serial / parallel / waterfall   -- 事件编排（A3.4，扁平）
 ├─ provide                                      -- 服务暴露（A3.5）
 ├─ box(name)                                    -- 盒句柄获取（A3.12）
 ├─ fs / workspace / global                      -- 文件三 scope（A3.7）
 ├─ http.request                                 -- A3.8
 ├─ timer.after / every / sleep                  -- A3.6
 ├─ sql.exec / query                             -- A3.9
 └─ process.exec / spawn                         -- A3.10
```

- **组织规则**：编排面扁平（最常用，沿用 ADR-0014 词汇）；效应面命名空间化（各带政策/参数面，审批文档与代码形状一一对应，同族方法留扩展位）。淘汰：全扁平（政策文档与调用形状打散）、全命名空间化（最常用路径多一跳零收益）。
- **出现规则**：效应命名空间在 ctx 上**恒定存在**（纯代码插件同形）；未声明能力 = **调用时宿主响亮报错指路 manifest**（§A2.5 fail-closed 原话兑现；纯代码插件报错指路升级包形态——A1.3“结构性不可用”= 能力永不可授予，政策层的结构保证）。与 deps 缺席纪律的**有意不对称**：deps 形状本来就是 manifest 派生的装配产物（S3“无从表达”）；ctx 命名空间 = 宿主契约面——形状恒定利文档/类型/教学，“指路 manifest”比裸 `index nil` 报错质量高。`ctx.box(name)` 未声明同政策（参数式响亮报错）。

### A3.3 inject 句柄装配：deps 第三参数

- **签名 `setup(ctx, config, deps)`**（A1 两参数形态向后兼容）；`deps` = 冻结表，按 manifest inject 装配；无 inject（含纯代码插件）= 空冻结表，形状恒定。
- **服务句柄 = 烘焙方法集**：激活时按注册表方法集逐方法生成 Rust 闭包（捕获 service key + method name，内部走 CallService 两跳）装成冻结表；点调用、单参数（S1：args = 单 Value，惯例 Map）。烘焙快照与 epoch 纪律自洽（提供方热更 → 消费者重启 → 新 setup 拿新快照）；方法名写错 = 字段不存在当场炸（analyze 写码期更早拦，A10）。淘汰：冒号调用（多传无用 self）；`__index` 动态查（typo 推迟到调用时 + 每次访问多一跳宿主查询）。
- **装配位置**淘汰：挂 ctx（`ctx.deps.memory`——ctx 形状每插件不同，文档/类型失去稳定锚点；ctx/deps 二分正是“宿主契约面 vs manifest 派生面”）；访问器 `ctx.service("memory")`（未声明服务也能问一句，S3“结构性拿不到”从无从表达降级为运行期检查）。
- **可选依赖缺席 = 字段不存在**（`if deps.cache` 惯用法成立；epoch 内缺席稳定——出现/消失/热更统一重启消费者，S3）。**联动细化 [ADR-0018](../decisions/0018-service-registry.md) S5 失败表 #2 落点**：常驻缺席走结构性 nil；`Err(Unavailable)` 保留给运行期窗口（提供方崩溃/热更竞态，与 #4 同族）。不冲突——ADR 定“调用发生时”的错误全集，本条定“句柄存在性”的装配规则；任务 3 以新 ADR partial-supersede 0018 S5 失败表 #2 落点。

```luau
-- manifest: inject: { required: [memory], optional: [cache] }
local function setup(ctx, config, deps)
  local r = deps.memory.recall({ key = "greeting" })
  if deps.cache then deps.cache.store({ k = "x", v = r }) end
end
return { setup = setup }
```

### A3.4 事件面五方法

- **`ctx.on(name, fn) -> off`**：返回摘除函数（disposer 习语族，A1.2）；不调用也随卸载账本摘除。**不立 `ctx.once`**——组合写法覆盖（`local off; off = ctx.on("x", function(p) ... off() end)`），登记扩展位。
- **四派发**（payload = 单 Value，[ADR-0015](../decisions/0015-payload-value-model.md)；事件名扁平字符串，命名纪律同服务）：

| 方法                           | 语义                     | 返回                                      |
| ------------------------------ | ------------------------ | ----------------------------------------- |
| `ctx.emit(name, payload)`      | fire-and-forget          | 无（不挂起，不等监听者）                  |
| `ctx.serial(name, payload)`    | 注册序逐个调，终止值短路 | 首个终止值；无人认领 = nil                |
| `ctx.parallel(name, payload)`  | 等全部（allSettled）     | 逐项结果数组                              |
| `ctx.waterfall(name, payload)` | 顺序变换链               | 变换后值；中途否决 = nil；无监听者 = 原值 |

- **监听者返回约定**（ADR-0014 的 Luau 映射）：serial 返回非 nil = 终止值并短路；waterfall 返回新值继续链 / nil 否决；parallel 返回值进逐项结果；emit 返回值被忽略。

### A3.5 provide 形状与 setup 内限定

- `ctx.provide(key, 方法表)`：方法表 = 提供方自己的普通 table（宿主不冻结——提供方 VM 内部物）；**注册 = 激活点捕获方法函数引用快照**，事后改表不影响消费侧路由（与烘焙快照哲学一致）；方法收单 args、返单值；抛错/主动失败 → `Err(Business)`（封套 A5）。多服务 = 多次调用，逐 key 独立（S1 附议）。
- **setup 内限定**：Active 后 provide = 响亮报错。S2“注册 = 激活转换点原子动作 + manifest 对账”只在激活窗口成立；激活后补注册绕过对账、打破“注册即可用、可用即 Active 一个布尔”。S2 的显式化，非新约束。

### A3.6 timer 三方法与两种唤醒路径

```luau
local cancel1 = ctx.timer.after(5_000, function() print("once") end)
local cancel2 = ctx.timer.every(60_000, function() print("tick") end)
ctx.timer.sleep(2_000)   -- 挂起当前协程 2 秒，期间邮箱其他消息排队
```

- **两种唤醒路径**（A1.4 红线的落实；setup 死锁红线的结构答案）：

|          | after/every 回调                                               | sleep                                                                |
| -------- | -------------------------------------------------------------- | -------------------------------------------------------------------- |
| 唤醒形态 | **邮箱消息**（tick 与其他事件串行排队，actor 铁律，seam §5.5） | **宿主直接 resume 挂起协程**，不排队                                 |
| 本质     | 世界来的新刺激                                                 | 当前处理的继续（与 `ctx.fs.read` 完成同一路径，seam §0“async 两跳”） |
| setup 内 | 注册可、回调进不来（排队）                                     | ✅ 可直接挂起（安全 = 不依赖自身邮箱）                               |

- **无线程心智**：宿主一只钟；after = 预约一条一次性邮箱消息，every = 订阅宿主时钟事件源；回调跑在插件自己 fiber 上与其他一切串行（无锁的来源）。登记 = 账本条目，卸载 drain 自动取消（无孤儿，seam §5.2）；after/every 返回 cancel 函数（off 习语族）。
- **慢回调合并丢弃**：上一条 tick 未处理，新 tick 跳过不补（邮箱内同一定时器至多一条排队；DSH“错过间隔不补发”同哲学——tick 是投影不是欠账）。
- **精度契约**：timer = **not-before** 语义（不早于）；实际延迟 = 排队深度 + 在跑 handler 余量；无硬实时（编排层定位；硬实时 = 外部服务层，[ADR-0024](../decisions/0024-carrier-layering.md)）。`every` 最小间隔政策（seam §5.2，数值实现期配置）= 吞吐保护 + 精度诚实：不承诺模型给不了的准度。
- 单位毫秒；journal 复用 `timer_scheduled`/`timer_fired` kind（seam §5.3，sleep 同族，细节任务 6）。

### A3.7 fs：三 scope、两段式路径解析、五方法

- **能力形状 = scope map**（§A2.5 已同步修订）：`fs: { private: write, workspace: read, global: read }`；简形 `fs: write` ≡ `{ private: write }`。scope 键 v1 封闭集 = `{private, workspace, global}`；命名任意根 = 扩展位（绑定机制归任务 4，词汇扩展走 A10 ABI rev）。
- **scope ↔ ctx 命名空间对应规则**：`private` → `ctx.fs.*`；`workspace` → `ctx.workspace.*`；`global` → `ctx.global.*`；将来命名根 → `ctx.<名字>.*`。审批读 manifest 即可推出插件代码里会出现哪个命名空间；analyze 对账漂移（A10）。

|          | private       | workspace                         | global                                                            |
| -------- | ------------- | --------------------------------- | ----------------------------------------------------------------- |
| 路径形态 | 相对根        | 相对根                            | **绝对**（相对路径响亮报错——没有根可相对）                        |
| 边界     | 本插件私域    | 部署绑定目录（含 symlink 收根内） | 用户世界 + 核心数据根 **− 两条豁免**                              |
| 绑定     | 免（core 给） | 部署树（任务 4）                  | 免                                                                |
| 审批权重 | 低            | 中（展示绑定后真实路径）          | **最高档展示**（write = 可读写任意插件私域与 db，审批面逐字写明） |

- **两条豁免（carve-out）**：**secrets store** 与 **journal** 对一切 scope（含 global）读写同禁。前者 = 引用式秘密的地基（真值只进声明插件 VM 的值，fs 视野里物理不存在，§A2.5）；后者 = 审计完整性（写可达 = 可篡改）+ 生成型秘密明文风险（seam §6.2）——journal 的正当访问面 = 查询接口（任务 6 自立），不是裸文件。**其余核心数据根（各插件私域、db 文件）global 正常可达**——维护/调试是 agent 本职；梯度自洽：自己私域走 `ctx.fs` 窄门，别人私域要 `global` 响门。
- **路径解析两段式**（单一实现，§A2.2 校验器哲学在 fs 层的镜像；一切 scope 过同一台解析器，per-scope 政策仅是参数）：
  1. **词法归一**（纯字符串，不碰盘）：解掉 `.`/`..`/重复分隔符 → 标准形；confined scope 判结果是否仍在根内，逃逸即拒——杀 `./x/../../../etc` 类词法逃逸。
  2. **物理 canonicalize**（解 symlink 后再验）：confined scope 验“仍在 canonical 根内”；global 验“未命中豁免清单”——**豁免必须验在 canonical 之后**，否则放个指向 secrets store 的软链即绕过。
     TOCTOU 硬化（检查与 open 之间的路径偷换：`openat2(RESOLVE_BENEATH)` / open 后 fstat 复核）归实现期清单；ABI 只冻结两段式语义。
- **五方法**（三 scope 同方法集）：`read(path) -> string` / `write(path, content)`（原子写 tmp+rename，seam §5.2；**自动建父目录**——私域/绑定根内无共享语义，省一个 mkdir）/ `list(dir?) -> array` / `delete(path)` / `exists(path) -> bool`。append = read+write 组合（actor 串行无并发写者）；glob/mkdir 等一律不立。大文件/二进制 = A6 的 Bytes/buffer 扩展位；read 带大小预算（实现期配置）。

### A3.8 http 单方法

```luau
local resp = ctx.http.request({
  method = "POST",              -- 缺省 GET
  url = "https://api.openai.com/v1/chat/completions",
  headers = { ["content-type"] = "application/json" },
  body = json.encode(payload),
  timeout_ms = 30_000,          -- 可覆盖，但必须有上限（seam §5.2 强制超时）
})
-- resp = { status = 200, headers = {...}, body = "..." }
```

- 单方法 + opts table：HTTP 的语义复杂度天然是命名参数；get/post 糖归 SDK 插件层，核心保持一格。
- 安全政策 **SSOT = §A2.5**（域名白名单/https 缺省/SSRF 最终解析判定/跨域重定向重对账/响应预算），不在此重复；透明 gzip 宿主吸收（seam §5.2）。
- SSE/大 body 流式 = 扩展位（seam §3.2 约束①；wasm 侧答案 = WASI 0.3 async，ADR-0024 §3）；v1 响应进内存带预算。

### A3.9 sql 两方法与私有 db

```luau
ctx.sql.exec("CREATE TABLE IF NOT EXISTS intents (id TEXT PRIMARY KEY, payload TEXT)")
ctx.sql.exec("INSERT INTO intents VALUES (?, ?)", { id, json.encode(payload) })
local rows = ctx.sql.query("SELECT * FROM intents WHERE due < ?", { now })
-- rows = { { id = "...", payload = "..." }, ... }
```

- 两方法按有无结果集分：`exec`（DDL/DML → 受影响行数）/ `query`（→ 行数组，列名→值）。**只许参数化占位 `?`**：注入防护 + 类型转换单挂点；分页参数 = 实现期配置（seam §6.3）。
- **每插件一个私有 db**（`sqlite: true` 无参数，§A2.5）：物理位置 core 管理、**不在本插件 fs 私域子树内**（自己的 `ctx.fs` 够不到 = 手滑写花结构性不存在）；global scope 可达（维护/调试正道，§A3.7）。生命周期随插件私有数据政策（卸载保留与否归任务 4）。
- **价值定位**：fs+JSON 管整张读写小状态；ctx.sql 管多记录条件查询（索引/WHERE/排序/分页——编排层手写 = 在最慢的一层重新发明 sqlite）；共享归服务层（物理共享 db = schema 归属/迁移/锁三烂摊子 + 提供方可替换性死亡；“共享关系存储”收敛后的正当形态 = 平台契约 + 参考实现插件，§A2.4 三层已留位）。核心提供而非盒自包的理由：journal 粒度（`sql_call`/`sql_result` vs 盒内经 fs 的 `fs_write` 字节流，审计与 replay 同降档）+ WAL/崩溃一致性白拿。近消费者：memory、schedule（seam §5.5 梯度二）、session/历史。
- **事务 = 扩展位**：候选 = 原子 batch（一组语句一次调用、不可挂起，无脚枪）/ tx 回调；带真实用例再审。

### A3.10 process：出区子进程门开门

- **概念三分**（不撞历史决策）：`ctx.spawn` 插件派生（[ADR-0017](../decisions/0017-programmatic-spawn-deferred.md) 推迟，不动）；T2 进程插件载体（ADR-0024 撤销一等位，不动）；**进程效应调用**（纯出区，无插件身份）= 本节。命名空间用 `process` 避撞 ADR-0017 的 spawn 词汇；方法名归位自然词汇：`ctx.process.exec` / `ctx.process.spawn`。**seam §5.1 修订联动**：子进程门由“v1 不给”改为能力门开门（§5.2/§5.3 已同步）。
- **能力 `process: true` 无参数**：命令内容不可预测，声明面不假装有粒度；命令级审批 = 插件层策略机器（ADR-0014 选择器模式 + §A2.6 两层授权分工不变）。exec/spawn 一把钥匙（拆分无意义：exec 经 `sh -c "… &"` 等效逃逸）。
- **调用约定**（两形态共用）：`argv`（直跑，不过 shell，无注入面）XOR `command`（经 sh -c）——给且只给一个，否则响亮报错；`cwd` 缺省 = 绑定的 workspace 根，未绑 = 私域；`env` = 继承核心 env + 覆盖（secrets 走引用式不在 env，§A2.5——泄露面结构上小；env 收紧政策归任务 4 部署绑定层）；输出带预算，超出截断。

```luau
-- 一次性：exec = spawn + 收齐输出 + wait 的糖；强制超时（与 http 同纪律），超时杀进程组
local r = ctx.process.exec({ argv = { "git", "status", "--short" }, timeout_ms = 60_000 })
-- r = { code = 0, stdout = "...", stderr = "..." }

-- 长驻/交互：spawn 返回句柄（冻结表烘焙，§A3.1 统一模型）
local p = ctx.process.spawn({ argv = { "npm", "run", "dev" }, cwd = root, env = { PORT = "3000" } })
local chunk = p.read_stdout()     -- 挂起读一块；EOF = nil（read_stderr 同理，分流不合并）
p.write("y\n")                   -- 写 stdin
p.signal("term")
local exit = p.wait()             -- 挂起等退出 → { code = 0 }
p.kill()                          -- 杀进程组
```

- **结构纪律**：句柄/在途 exec = 账本条目，卸载 drain 杀**进程组**（无孤儿，与 timer 同铁律）；未读输出 = 宿主管道缓冲自然背压 + 预算上限；更富的流式形态（行/事件）归 SDK 组合。
- **journal**：`process_spawn`/`process_exit` 元数据 + exec 输出（带预算）；spawn 流式内容默认不进（写入量账 seam §6.2，粒度政策任务 6）。**行为透明是核心对不可预测命令能给的真正护栏：藏不住。** replay = record/inject 到头，副作用出视野不可回滚（任务 6 登记）。
- 与 MCP 的关系：ADR-0024 的 MCP stdio 监管机器 = host 自持物，与插件侧 process 能力井水不犯河水。
- **实现节奏**：ABI 形状本节冻结，落地随默认插件集（shell tool = 参考实现近需求）——“ABI 先行、实现后随”，与 workspace 绑定同模式。

### A3.11 require 宿主包装实现确认

A1.5 行为规则的实现机制确认（spike §1 PASS，无新决策）：**每插件 VM 挂自定义 `Require` 实现**（mlua `create_require_function`），导航协议内落实两条 Metis 裁剪（插件根逃逸拒绝；`@lib` 白名单 + 部署根单份 `.luaurc`）；loader 加载入口后登记进该 VM require 缓存（A1.5 入口归一）。spike 的坑（入口 chunk 名须映射真实文件）已规避：loader 本就加载真实文件。mlua 无条件安装件（`require`/`collectgarbage`/`loadstring`/`_VERSION`）的手工处置进 metis-luau 实现清单（seam §1.4/§6.3 已登记）。

### A3.12 盒调用桥面（Luau 侧形状）

```luau
local parser = ctx.box("parser")   -- manifest boxes: [parser]；未声明 = 响亮报错（fail-closed）
local ast = parser.parse(source)   -- 挂起式调用：盒内 ms 级计算不堵共享 executor
```

- **访问器 `ctx.box(name)`**：盒不是服务依赖（不卡激活门），是按需获取的能力句柄——与能力门同走 ctx 挂载位（seam §2.3）；analyze 对账 `box()` 调用（§A2.5/A10）形状衔接。
- 句柄 = 冻结表烘焙方法集（按盒导出清单生成——§A3.1 统一模型第三次兑现）；实例池/句柄缓存 = 宿主内部事（ADR-0024 §8：句柄必须缓存，否则 +26ns/次）。
- **调用挂起**：盒内 ms 级 CPU 密集 → 卸载 blocking 池 + 两跳 resume（与效应调用同形态，作者直线写法不变）。
- 归 A6：参数/返回线编码（NaN-boxed 参照）、大 Value 零拷贝/Arc、流式；归 A7：WIT 形式化。

### A3.13 联动登记汇总

| 去向                | 内容                                                                                                                                                         |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A4（await/交错）    | 挂起点 = ctx 方法调用处（seam §6.1 已登记）；handler 等待期间邮箱让路语义；事件回调与协程恢复的交错细则                                                      |
| A5（错误/Result）   | 一切效应/服务/盒/process 调用的失败封套形状（本节示例失败面一律从简）                                                                                        |
| A6（边界转换）      | 流式句柄族形态统一再审（http SSE/大 body、fs 大文件、process 流式；userdata 届时再审）；盒线编码/零拷贝/Bytes                                                |
| A7（schema DSL）    | deps 烘焙方法集的 schema 来源（contract 对账）；盒 WIT                                                                                                       |
| A8（事件静态声明）  | manifest `events` 字段（on/emit 面的静态化）                                                                                                                 |
| A9（能力门槛）      | 安装面对账：capabilities 政策声明 = syscall 与盒能力笼同一对账机制（seam §6.1 已登记）                                                                       |
| A10（工具/ABI rev） | ctx/deps/`plugin()` 糖类型定义；analyze 对账（scope 键↔命名空间、`box()`、能力漂移）；词汇扩展通道（fs 命名根、process 参数化、sql 命名 db/事务、http 流式） |
| ADR-0018 S5         | 失败表 #2 落点细化（常驻缺席 = nil 字段；`Err(Unavailable)` 归运行期窗口）——任务 3 冻结 ADR 时吸收                                                           |
| seam §5.1/5.2/5.3   | 子进程门开门 + kind 表补 `process_spawn`/`process_exit`（本 change 已同步）                                                                                  |
| §A2.5               | capabilities 词汇修订：fs → scope map 三键 + 简形；+`process`（本 change 已同步）                                                                            |
| 任务 4（配置格式）  | workspace/命名根绑定机制；db 卸载保留政策；env 收紧政策；cwd 政策                                                                                            |
| 任务 6（journal）   | kind 表补 process 两条；spawn 流式内容粒度政策；process replay 族；sleep 复用 timer kind；journal 查询接口（豁免的正当访问面）                               |
| 默认插件集立项      | shell tool = process 能力参考实现（近需求）                                                                                                                  |
| 扩展位              | `ctx.once`；fs 命名根；process 流式富形态；sql 事务（batch/tx）与命名 db；http 流式；userdata 流式句柄（A6）                                                 |
| 实现期配置          | http 超时/预算；fs read 预算；timer `every` 最小间隔；process 输出预算；TOCTOU 硬化手段；加载总预算（A1 已登记）                                             |
