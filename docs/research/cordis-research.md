# Cordis 与 DeepSeek Harness 深度调研

> 调研日期：2026-09-28
> 调研目的：为 metis（Rust 核心 + Luau 扩展的自我进化 agent）提供架构参考
> 调研对象：
>
> - 论文：_A Programming Paradigm for Spatiotemporal Composability_（arXiv:2608.25512，北京大学 + DeepSeek-AI，92 页全文提取）
> - 源码：`deepseek-ai/deepseek-harness` monorepo（vendor/cordis 及 6 个框架插件、packages/extensions、packages/preset、各 capability 族 README 与 docs/）
> - 交叉验证：上游 `cordiverse/cordis` main 分支源码、Harness 官方文档 `cordis-primer`

---

## 第一部分：论文的核心理念

### 1. 问题定义：动态组合的两个正交维度

论文的出发点：现代软件（插件系统、自我进化 agent harness）越来越需要**运行时动态组合**，但其形式化基础缺失。动态组合被分解为两个正交维度：

| 维度                                       | 定义                                                                                               | 静态场景的退化形式          | 动态场景的难点                               |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------- | --------------------------- | -------------------------------------------- |
| **Temporal composability（时间可组合性）** | 移除组件时，它对共享环境做过的**每一处修改**（资源分配、事件注册、状态变更）都能被完全、安全地撤销 | 词法作用域（RAII、bracket） | 组件来去不定，副作用交错，无法靠词法结构界定 |
| **Spatial composability（空间可组合性）**  | 组件以结构化、可验证的方式**声明、发现并解析**彼此依赖，依赖变化时协调生命周期                     | 模块导入解析                | 依赖拓扑随运行时演化，可能出现/消失/换身份   |

### 2. 两个核心机制：effect 与 coeffect 的运行时提升

论文把 PL 理论中的经典概念从**编译期静态学科**"提升"（lift）为**运行时机制**：

**Revertible effects（可撤销效应）→ 时间可组合性**

- 一个 effect 被建模为 `Γ → Γ × (Γ → Γ)`：作用于当前 context，产出新 context **和一个显式逆操作**。提供逆操作使效应可撤销；把逆操作交还运行时使效应可追踪。
- 逆操作按 **twisted composition monoid** 组合：`(f₁,g₁) ∘ (f₂,g₂) ≔ (f₁∘f₂, g₂∘g₁)`，即逆操作以 LIFO 顺序累积。
- **效应迭代器**（effect iterator）是"具现化的限定续延"（reified delimited continuation），直接对应主流语言的 generator/yield：加载组件 = 运行迭代器并累积逆操作；卸载组件 = 应用累积的逆操作链。
- **关键权衡**：`ctx.effect` **不验证**逆操作的正确性——"逆操作确实撤销了效应"是组件作者的义务，运行时只保证调用它。这是刻意的工程取舍。

**Reactive coeffects（响应式余效应）→ 空间可组合性**

- coeffect context 是依赖偏函数 `Σ ≔ (k : K) ⇀ 𝒱k`（key → 值）。
- 组件把依赖声明为 specification（一个 key 集合）；每次 context 变化按 `notify` 分类：`activating`（依赖从不满 → 满）、`deactivating`（满 → 不满）、`neutral`，据此驱动组件激活/失活。
- 一个漂亮的理论协同：**coeffect 操作本身就是 effect**（`set(k,v)` 可撤销），因此依赖的提供与撤走天然可逆。

**Context paradigm（上下文范式）**

- effect context 与 coeffect context 统一为单一递归类型 `Γ∞ ≔ μΓ. Γ × (Γ→Γ) × Σ`。**组件与环境的一切交互都经由这一个实体中介**。
- 由此诱导出 **observational equivalence（观测等价）**：撤销是理想化的（`free` 不会恢复 `malloc` 前的堆布局），所以所有等式都在"没有观察者能区分"的意义下成立。这个商（quotient）使不同组件的 effect 交错执行而互不干扰。

### 3. Component 与 Fiber：动态组合演算

- **Component** = 三元组：coeffect specification（要读什么）× coeffect provision（要提供什么）× witnessed effect function（激活时贡献的效应及其逆）。
- **Fiber** = 组件的运行时实例，各携带生命周期状态。
- 元理论（Preservation / Temporal / Spatial / Progress / Confluence）把单组件的局部保证提升到任意交错 fiber 组成的全系统——这就是**时空可组合性**的完整含义。

### 4. 论文对"自我进化 agent"的论证（§1.2.2 + §8）

- 未来 harness "may generate and deploy modifications to its own components while continuously serving requests"，每次自我修改都是一次动态组合。
- 没有 temporal composability：每次自修改都得整体重启（丢弃进程内状态、打断在途任务），更糟的是 **"a faulty self-modification can disable the very process needed to recover"**（错误的自修改可能瘫痪掉本应用来恢复的进程本身）。
- 没有 spatial composability：朴素的代码替换会静默破坏依赖方，或在重载时才暴露循环依赖。
- OS 进程/容器编排是**粒度错配**的粗粒度替代：重启丢弃状态、靠冗余副本补偿、无法表达同地址空间依赖、引入网络开销。
- 论文诚实标注：harness 场景是**动机与未来验证方向**，本文的验证案例是 Koishi（聊天机器人框架，4000+ 社区插件，4 年生产验证）。

### 5. 与相关工作的定位（论文 §7，指导我们取舍）

- **monadic effects（ZIO/Effect-TS）**：追踪需写进类型，服务撤走时已发生的操作留在原地；Cordis 给每个 effect 配逆、随 provider 来去重新解析。
- **Algebraic effects / OCaml 5**：目的是模块化解释 vs 目的是追踪与撤销；capability 是二等公民 vs 组件是一等公民。
- **React useEffect**：结构上最接近"效应+逆"，但 hook 只能顶层调用、effect 体不能 async/迭代，无法组合出复合逆。
- **Erlang/OTP**：前向状态迁移（`code_change/3`），监督重启而非撤销；更优雅但需手写迁移函数，且不能整体卸载回收。**论文承认：DSU 式前向迁移叠加在 revertible effects 之上是 future work**。
- **OSGi**：reactive coeffects 最接近的先例，但失活靠手写同步回调（忘了写就泄漏），异步交接只能对过期引用阻塞；Cordis 用累积撤销 + 惯性 UNLOADING 状态补齐。
- **webpack/Vite HMR**：需要开发者标注 accept 边界；Cordis 因 fiber 已界定全部效应，**无需开发者标注**。

---

## 第二部分：Cordis 核心库（vendored 4.0.4 分叉版）

> 注意：Harness vendor 的是 `cordis 4.0.0-rc.7` 上游 + 22 项本地加固（记录在 `vendor/README.md`），实际 package.json 版本 4.0.4。分叉要点：上游的 `EffectScope`/`Fork` 两个概念被合并为单一 `Fiber`；核心抽象全部是 fiber；有一套 fiber 生命周期重入加固。

### 1. 五个核心概念（Harness primer 的总结）

1. **插件**：函数插件（带 `inject`/`apply(ctx)`）或 `Service` 子类。
2. **Context**：服务仓库，服务占据稳定的 `ctx.<key>`（如 `ctx.tools`、`ctx.llm`）。
3. **inject 依赖声明**：加载顺序由服务可用性驱动，而非手动排序。
4. **五种事件派发模式**：`emit` / `waterfall` / `parallel` / `serial` / `bail`。
5. **注册是可逆 effect**：`ctx.effect()` / `ctx.on()` 安装的一切，卸载时精确回卷。"每个注册都应有对应的 disposer"。

### 2. Context：三位一体的容器

Context = **服务容器 + 事件总线 + 作用域树**的统一体。

- TS 实现手法：每个 Context 都是 **Proxy**，属性读写被拦截到服务解析逻辑；各核心模块通过 `declare module` 接口增广把方法挂到 ctx 上。
- `extend(meta)`：原型链子上下文，遮蔽父级属性。
- `isolate(name, label?)`：服务隔离域。同 label 合流、异 label 独立——同一服务 key 可以有多个隔离实现。
- `intercept(name, config)`：给某服务名下所有后代插件的配置做合并。
- **Rust 映射**：Proxy/原型链/声明合并都是 TS 表达形式，应替换为显式 API（`ctx.get::<T>(key)`）+ 父指针链 + 类型系统/宏在编译期表达 inject 纪律。

### 3. Fiber：效应域 + 生命周期状态机

**Fiber 是整个框架的心脏**：一次插件应用的运行时实例。

**状态机**：`PENDING → LOADING → ACTIVE → UNLOADING → (DISPOSED | FAILED)`

- 状态大多是**派生**的，由 **epoch（依赖指纹）** 驱动：epoch = 各依赖提供者 uid 拼成的指纹串；`INACTIVE` 表示依赖未齐。
- 依赖指纹从 INACTIVE 变有效 → 启动加载；反之 → 启动卸载。**依赖热替换 = 完整 unload → reload**。
- **惯性（inertia）**：进行中的加载/卸载一定跑完再响应新变化，避免状态撕裂。

**效应收集与撤销**：

- `fiber.effect(execute)` 接受四种形态：单个 disposer / `Promise<Disposer>` / 同步迭代器 / 异步迭代器（生成器每 yield 一个 disposer 收集一个）。
- 撤销：**LIFO 逆序**执行；重复 dispose **幂等**（共享同一个 disposal task）；异步清理可**汇合**（加入别人已启动的清理，不重跑）。
- 注册时机：wrapper **先于** setup 体进入清理列表——重入的 owner unload 一定能看到它。
- 卸载期间**拒绝注册新 effect**（`INACTIVE_EFFECT` 错误）。

**父子关系与错误遏制**：

- 子 fiber 的 dispose 本身就是父 fiber 上的一个 effect → 父卸载自动带走全部子孙。
- 插件回调抛错 → 记录 + 状态落 FAILED，**不传播给父 fiber**；disposer 错误逐个吞掉不中断其他清理。
- 唯一向上抛错的通道是 `fiber.await()`。
- 唯一放任崩溃的情况：logger 本身抛错（"诚实的结局"）。

### 4. Registry：插件归一化

- 三种插件形态：函数 / 类 / `{ apply }` 对象；共享元数据 `name`、`Config`（Standard Schema，**拒绝异步校验**）、`inject`、`provide`、`intercept`。
- 同一 callback 挂 N 次 = N 个 fiber 共享一个 Runtime——**重复挂载允许**。
- uid 单调分配、不复用（因此同值替换服务也能被依赖方识别）。

### 5. Events：五种派发模式 + internal/* 钩子

- `emit`（同步 fire-and-forget）/ `parallel`（allSettled）/ `serial`（顺序，可 bail）/ `bail`（同步 serial）/ `waterfall`（洋葱模型，**不调 next 即否决后续含内建行为**）。
- **监听器所有权铁律**：`ctx.on()` 注册的监听器是 fiber 的 effect，fiber 卸载自动摘除。
- **`internal/*` 钩子是框架自身行为的扩展点**：
    - `internal/config`（waterfall：激活前解析原始配置——lazy `!!js` 求值挂在这里）
    - `internal/update`（waterfall：配置更新，veto 可阻止重启）
    - `internal/service`（服务绑定变化通知）
    - `internal/get` / `internal/set`（拦截服务读写）
    - `internal/listener`（bail：可替换监听器注册——框架借此实现监听器作用域对齐）
    - `internal/plugin` / `internal/status`（fiber 生命周期）
    - `internal/dispatch`（事件诊断）

### 6. Service：provide/inject 的实现细节

- `Service` 基类构造函数即注册：`ctx.reflect.provide(name, self)`，注册是 fiber effect，**fiber 卸载自动注销**。
- 注销的顺序刻意安排：先删实现 → notify 依赖者 → **等所有依赖 fiber settle** → 最后才从自己的 store 删除（保证依赖方清理期间自己仍可访问服务）。
- `check` 谓词把"值存在"与"可用"分离：check 失败时依赖者退回 PENDING。
- 读取沿 fiber 父链上溯；未声明 inject 的读取在开发期即报错（`cannot get property "x" without inject`）。
- **Rust 映射**：trait object 注册表 + "service → 依赖者"反向索引；isolate → `HashMap<ServiceId, ScopeTag>`。

---

## 第三部分：声明式层（Loader / HMR / Include / Group）

### 1. cordis.yml：entry 即 fiber 的完整规格

```yaml
- id: <树内稳定 id，缺省自动生成>
  name: <插件模块 specifier>
  config: <插件配置，可为 !!js 表达式>
  group: true # config 字段即子 entry 列表
  disabled: true # 或 !!js 表达式，父链级联
  inject: [...] # 依赖声明 + intercept 配置
  isolate: { name: true | 'label' } # 本地 realm / 共享 realm
  intercept: { ... } # 后代插件配置合并
```

- **`!!js` 表达式**：js-yaml 自定义标签，以 Context 为作用域 eval；**lazy resolution**——直到 fiber 激活（inject 就绪）后才经 `internal/config` waterfall 求值；原始表达式始终保留供写回/HMR 转移/重激活。
- **patch 层叠**：`- insert:` 插入行；`- id: <row>` 按 id 定位并**整体替换**字段（不深合并）；删除 = `disabled: true`；层序 = bundles 按序 → profile patch → home patch → CLI overlay，全部施加到空 entry list。

### 2. Reconciliation：update vs remount

`Entry.update` 对所有键 deepEqual 得到 changes：

- 无变化 → no-op
- **仅 volatile 路径变化** → 快路径：就地提交引用，不重启（schema `.volatile()` 标注 + 深冻结快照 + 按路径通知属主 fiber）
- `config` 变化 → `internal/update` waterfall（可 veto）→ 插件热重启
- `inject/isolate/intercept/disabled` 变化 → context 重建 + inject 机制驱动重激活
- **`name` 变化不会重新 import**——换插件的规范路径是 remove + create（类型层面刻意排除）

### 3. HMR：三阶段事务

1. **变更分类**：框架模块（externals 闭包）→ 整树退出；已加载模块 → 局部刷新；配置文件 → include.refresh()。
2. **依赖爬取定位受影响插件**：沿模块的 import 边做 accepted/declined 不动点迭代，以**插件入口文件为原子 reload 单元**，闭包与 accepted 相交才列入 reload。
3. **事务式重载**：ESM/CJS 双缓存**先备份**；重导入任一失败 → 恢复缓存 + 用旧插件重建全部 stale entry（系统绝不处于半重载状态）。
4. **状态迁移的刻意保守**：只迁移**原始 config（含未求值的 `!!js`）和 entry 身份**；服务状态不拷贝——新插件重新 provide，依赖方由 inject epoch 机制自动重启。

### 4. Include：外部文件挂载 + 运行时写回

- 解析失败**保留运行中树**（内容只在成功时提交）。
- 运行时变更（自杀式 disabled、config 自更新、tree 增删改）经 **debounced write** 写回配置文件：tmp+rename 原子替换、Windows 句柄占用退避重试、串行写队列。
- **entry/group/tree 变更刻意保持非事务性**：单行激活失败不影响兄弟行，失败 entry 留在 store 中可重试。

---

## 第四部分：Harness 的 "Everything-is-a-Plugin" 实践

### 1. 字面含义

"Every part of the product is a plugin, including the model adapter, the tool registry, the session log, and the agent loop itself… **There is no privileged core to patch**."

- agent loop 可替换（`ctx.agentLoop`）
- 每个模型工具是挂在 `ctx.tools` 上的 Consumer 插件
- 每个 UI 面板是 `ui-*` client 插件（连插件管理页面自身也是）
- session 持久化是 seam（jsonl backend / sqlite 全文检索是可选行）

### 2. Capability Seam 三角（最重要的包结构模式）

一个可替换能力分三个角色：

| 角色                   | 职责                                                                   | 规则                                                                  |
| ---------------------- | ---------------------------------------------------------------------- | --------------------------------------------------------------------- |
| **Service Definition** | 拥有 `ctx.<key>` 和词汇类型的 Cordis Service（抽象类或 registry 服务） | **绝不能是 TS interface**                                             |
| **Service Provider**   | 提供/注册实现的插件                                                    | 同一组合恰好挂一个 executor，挂两个会在 load 时因重复注册**响亮失败** |
| **Consumer**           | 模型工具与插件编程所面向的一方                                         | **只 inject 服务 key，永不 import provider 类型**                     |

- 三者演化速率不同就分包（`dsh-shell` / `dsh-bash-local` / `dsh-tool-bash` 是参考模板）。
- 价值："one provider swap changes the whole product"——把 fs/subprocess provider 指向远程沙箱，Bash、PTY、LSP 全部随之迁移。

### 3. Profile / Bundle / Preset 组合层

- **bundle** = "Cordis 配置行 + 所挂载代码"的分发格式（npm 包 + `dsh.bundle.patch`）。
- **profile** = 命名组合：`dsh.profile.bundles` 有序堆叠 bundle + 自己的 `cordis.patch.yml`。
- **准入检查**：peer 依赖必须匹配运行时版本，被拒的行变 detached `disabled: true`；必须 entry 失败 → 整个 app dispose 非零退出，可选 entry 失败仅告警。
- **preset** = per-session 组合：一条普通插件行，`config.plugins` 是与 profile 语法完全相同的子插件 YAML；**eager 激活 + revision 引用计数保留**——更新声明 retire 旧 revision，但运行中 Agent 保留旧树直到最后引用释放（"编辑影响之后创建的 Agent，不抢夺运行中 Agent 的工具"）。

### 4. 运行时自我修改：四条路径及其演变教训 ★

这是 metis 最该借鉴的部分：

**(a) 只读自省**（`cordis_inspect_list` / `cordis_inspect_query`）：模型可看到服务/事件目录（带源码 JSDoc）、live Loader 树、单条 entry 的 Config JSON Schema、自己的 live 工具 schema。两级设计控制 token 开销。

**(b) 持久化挂载/卸载**（`plugin_manager` 工具）：`set_plugin`/`install_bundle`/`remove_bundle` 等。**写 profile 的 `cordis.patch.yml`**，HMR 即时生效，跨重启存活，影响该 profile 所有 session。**每次调用（含只读 list）都要 danger-full-access 权限或逐次审批**；安装的代码以宿主权限在进程内执行；构建脚本需单独批准。

**(c) 新插件创作（Creator 模式）**：模型用 inspect 发现 API → 读包的 `packageDir` → 把**包代码 + Loader YAML 补丁写成 workspace 文件** → `install_bundle` 安装 → HMR 激活。配套三个技能包（插件开发、组合编辑、组合参考）。

**(d) 进程内动态定义（已退役给程序化消费者）**：早期允许模型用 `node:vm` 直接 define/run 临时插件，**后被 (c) 取代**。退役理由极具参考价值：进程内临时定义造成"第二条插件生命周期"，而持久化 bundle 路径天然获得 profile 锁、审批、HMR 与重启持久化。

> **核心教训：让 LLM 通过"写文件 + 受审批的安装事务"来修改自身，比给它一个 eval 工具更安全、更可持久、更好审计。**

### 5. 工程纪律（packages/AGENTS.md）

- 注册一律走 `ctx.effect()`/`ctx.on()` 并返回 disposer。
- "Model-visible ⟺ logged"：进模型请求的内容必须能从 session 日志重建。
- 误配置响亮失败；无可硬编码调参（部署相关选择必须是 Config 字段）。
- "Plugins, not loop changes"：新行为挂在文档化扩展点上。
- registry 贡献必须通过 HMR-safety 测试（dispose fiber 后观察到移除）。

---

## 第五部分：对 Rust + Luau 重实现的映射

### 1. 本质机制（必须保留语义）

| Cordis 机制                            | Rust 表达                                                                                         |
| -------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Fiber（效应域 + 状态机）               | fiber arena（`SlotMap<FiberId, Fiber>`），每 fiber 持有 disposer 栈、依赖指纹、状态               |
| LIFO 撤销 + 幂等 + 异步汇合            | `IndexMap<u64, Disposer>` 反向 drain + `OnceCell<JoinHandle>` 共享清理任务                        |
| epoch 依赖指纹                         | `Option<DependencyFingerprint>`，service→依赖者反向索引驱动 notify                                |
| 错误遏制边界                           | 插件体/disposer 的 panic·error 捕获 → FAILED 状态，唯一上抛通道 `await()`                         |
| 五种事件派发 + internal/* 钩子         | 事件表 `name → Vec<Hook>`（Hook 携带 owner fiber 与作用域过滤谓词），围绕核心操作的可否决中间件链 |
| 声明式 entry 树 + keyed reconciliation | 纯数据结构算法，直接移植                                                                          |
| 原始配置/有效配置分离                  | raw config 静止存储/写回/跨重挂载传递；effective config 激活时经 hook 链解析                      |
| volatile 通道                          | schema 标注路径 + 稳定可变引用 + 失败回退 remount                                                 |
| Service Definition seam 三角           | trait object 注册表，Consumer 只依赖 trait                                                        |
| patch 层叠组合                         | 同样的 YAML 树 + 层序语义（用户自定义永远是最上层 patch）                                         |

### 2. TS/Node 特有（应舍弃或替换）

- **Proxy 属性拦截 / 原型链 / 声明合并** → 显式 API + 父指针 + 宏/代码生成
- **`!!js` with(ctx) eval** → `!!luau`：带环境表的 `load()` 在目标插件 context 内惰性求值
- **Node ModuleLoader 内部 API、ESM/CJS 双缓存、chokidar** → Luau 无模块自省能力，需**自建模块注册表**：require 包装器埋点记录 `module → imports` 边，reload 时按闭包相交法计算失效集
- **生成器作为 effect 形态** → `impl Iterator<Item = Disposer>` / `Stream`
- **npm/pnpm 分发** → 需要自己的包格式与安装事务（借鉴"失败恢复快照"与"构建脚本单独审批"的事务设计）
- **Typert 类型图（TS 绑定）** → 另选 IDL/Schema 方案（保留"生成物 freshness-gated in CI"的纪律）

### 3. Luau 侧的关键设计问题（待讨论）

1. **插件形态**：Luau module = entry？每个 Luau 插件是 table（含 `apply(ctx, config)` + `inject` + `Config` schema）还是函数？
2. **沙箱边界画在哪**：Harness 的教训——注入的服务仍有真实权限，**全局隔离 ≠ 权限约束**。Luau 的 sandbox（无 Node 全局、能力白名单）比 `node:vm` 更接近真沙箱，但权限边界仍应画在**服务注入层**。
3. **自我修改路径**：照抄 Harness 演变结论——模型**写文件 + 受审批的安装事务**落盘，HMR 生效；不提供运行时 eval 工具。
4. **Config 校验**：schemastery（TS）→ Luau 侧需要等价的声明式 schema（含 `.volatile()` 标注）？
5. **状态迁移**：照抄"只迁移 raw config 与 entry 身份"的保守取舍；DSU 式前向状态迁移是论文承认的 future work，可作为远期增强。

---

## 附录：主要信息来源

- 论文全文（PDF 本地提取）：https://arxiv.org/pdf/2608.25512
- Harness 仓库：https://github.com/deepseek-ai/deepseek-harness（vendor/README.md、docs/architecture.md、docs/cordis-primer.md、packages/*/README.md、bundle 的实际 cordis.patch.yml）
- 上游 Cordis：https://github.com/cordiverse/cordis（packages/core 的 fiber.ts/context.ts/registry.ts）
- Harness 设计决策 Agent Notes：capability-seams（2026-06-13）、creator-persistent-plugin-management（2026-09-16）、declarative-agent-presets（2026-09-18）、self-referential-cordis-toolset
- 未验证/存疑点见各部分标注；论文定理编号因 PDF 提取噪声可能有 ±1 偏差。
