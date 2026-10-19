# Metis — 服务注册表与 inject 设计

> 状态：S1–S6 ✅ 全部确认，固化为 [ADR-0018](../decisions/0018-service-registry.md)。
> 依据：[ADR-0001](../decisions/0001-inject-runtime-checks.md)（inject 运行时检查）/ [0010](../decisions/0010-fiber-core.md)（fiber 核心）/ [0011](../decisions/0011-actor-execution-model.md)（actor 模型）/ [0013](../decisions/0013-context-scope.md) / [0014](../decisions/0014-dispatch-semantics.md) / [0015](../decisions/0015-payload-value-model.md) / [0034](../decisions/0034-wasm-first-carrier.md)；调研：`../research/cordis-research.md`（Capability Seam 三角、Service 实现细节）；作者面 = [plugin-sdk](plugin-sdk.md)。
> Last reviewed: 2026-10

---

## 1. 服务的形态（S1 ✅）

**服务 = key + 命名方法集**；调用 `call(key, method, args) → Result`（作者面形态 = [plugin-sdk](plugin-sdk.md) S1/S4，ABI 面 = [wit-abi](wit-abi.md) W3 章）。

- **args/返回值 = 单个 `Value`**（惯例 Map 即命名参数）：跨实例边界一次转换，schema 校验单挂点
- **方法集入注册表**：有契约服务 = 宿主从契约载入方法集到注册表（W7 D167 / [wit-abi](wit-abi.md) W9 D183）——方法存在性响亮（`service-method-missing`，不走消息）+ schema 挂点 + 自省目录数据源；contractless 服务无方法集，missing method 由 guest catch-all 返回 `business`（W2 D131 已明记的退化）
- **key 命名**：扁平字符串；框架级服务占短名（`llm`/`tools`；实现 = 插件，core 不提供领域服务实现，见 §6 注），插件服务建议 `plugin-name/service` 前缀（约定不强制，与事件命名纪律同构）；重复注册响亮失败 = 命名冲突启用期暴露（W9 D180）
- **服务没有「属性」**：跨实例只有方法调用（消息），无共享对象——服务状态是提供方实例私有，访问必经方法（actor 纯化，Cordis 的字段穿透是 Proxy 遗物）
- **LLM 工具 = 服务方法的投影**：tool registry 插件把 inject 来的方法集翻译成模型 tool schema——插件写的服务自动成为模型可用的工具（schema 化后），无手工翻译层
- **唯一性（附议）**：**同一作用域内**任何时刻唯一（重复注册响亮）；部署树 = 一个作用域，实例 = 各自作用域（D68）。跨时间可替换（配置变更 + epoch+1）；同时多实现归实例作用域（[ADR-0013](../decisions/0013-context-scope.md) 推迟项已结案：多实现 = 不同实例作用域各一份，isolate 不立字段、语义由实例作用域天然给出；同作用域内唯一性原样，[instance-model](instance-model.md) §S4.2/§S4.3 D68/D69）
- 一个插件可提供多个服务：每 key 独立注册、独立 epoch

注册表数据结构（host 侧；v1 = 部署树单级 + 实例作用域）：

```
HashMap<ServiceKey, ServiceEntry {
    provider: FiberId,
    epoch: u64,
    methods: Vec<MethodName>,   // 有契约服务 = 宿主从契约载入；contractless = 空（W9 D183）
}>
+ 反向索引 HashMap<ServiceKey, Set<FiberId>>   // service → dependents
+ ScopeTag（实例作用域标识，[instance-model](instance-model.md) §S4.2 D68）
```

## 2. provide 与注册时机（S2 ✅）

**`ctx.provide(key, 方法表)` = fiber effect**；**注册 = 激活转换点的原子动作**：

1. setup 内调用 provide 入待注册清单并入账；setup 失败 → 半成品 drain，不留半注册状态
2. setup 成功 → manifest 对账（声明 keys 全履行，缺 = 激活失败）→ **原子注册** → fiber 转 Active
3. **注册即可用、可用即 Active，一个布尔无中间态**——「已注册未 ready」窗口整类消失（增量上架被否：它引入三不管窗口）
4. 重复 key 双保险：静态（配置树扫出两 entry 提供同 key → 加载期响亮）+ 运行时防御（host 复查）。静态扫描的判定作用面 = 启用期对「已启用 ∪ Pending」集合判定，休眠件撞名不判定。
5. 注销 = 卸载自动摘除（账本 LIFO）；**v1 不做主动 withdraw**——临时退化（DB 断连）用方法返回业务错误表达；withdraw 登记扩展位（与 §5 check 谓词同族）
6. 必需 key 注册 → 反向索引唤醒 Pending 消费者 → 拓扑序激活（级联见 §5）

纪律：setup 只做激活必需的初始化；重活后置（内部任务 + 业务错误窗口）；setup 有 fuel/epoch 时间盒兜底，不会无限卡住激活门。

## 3. inject：manifest 双声明 SSOT（S3 ✅，闭环 inject 静态声明问题）

**结构性论证**：激活门（依赖不齐不启动，fiber `Pending`）要求依赖在**运行插件代码之前**可知——动态声明意味着先跑代码才知道依赖，跑代码时 fiber 已 Active，激活门失效。故问题不是「要不要静态」，是「静态到什么程度」。wasm 下组件 import 清单可机械抠出，manifest-first 被进口对账进一步强化（[wasm-first-design-review](../research/wasm-first-design-review.md) B1）。

**manifest 是唯一书写点，代码只消费声明的产物**：

```yaml
# 插件 manifest（示意形态，格式细节归 W2 章）
name: memory-viewer
provides: [] # 我暴露什么服务
inject:
    required: [memory] # 没有它我无法工作
    optional: [metrics] # 有它更好
```

- **inject：代码零声明**。setup 时 host 按声明装配依赖句柄注入（作者面形态 = SDK trait / W3 章）——未声明的服务**结构性拿不到**，不是报错纪律，是无从表达
- **provides：激活期强制对账**。声明了没 provide → 激活失败；provide 了没声明 → 报错。**漂移无法被发布**——系统层面消除双份维护（SSOT 要求的落地形态）
- **缺失处置三分**：提供方在**沿 parent 链可达集内**已启用但未就绪 → 消费者 Pending **等待**（就绪后经反向索引唤醒）；可达集内已安装但未启用 → Pending 等启用 + host warn 提醒；可达集内**无任何提供方** → **启用期响亮失败**（静态校验，杜绝打错字静默卡死；兄弟/子实例的提供方不可见，按「查无」同族处理）。后者要求 provides 也静态声明，否则供给不可见、校验无从谈起。

**actor 模型没有「过期服务引用」**：Cordis 的 epoch 重启解决「持有的服务对象引用过期」；Metis 的调用是 host 每次现查的路由消息，无可过期之物。epoch 重启的新理由：**激活门 + setup 期快照刷新**（插件 setup 缓存的派生状态在依赖热更后过期，干净重启重跑 setup 刷新——保守但可推理，[ADR-0010](../decisions/0010-fiber-core.md) 哲学）。

- **可选依赖精确语义**：静态声明；缺席**不阻塞激活**、调用时缺席返回框架错误；**出现/消失/热更统一重启消费者**（「依赖视图变化 → 重启」一条不变量无例外；惊扰面由反向索引限定 + 拓扑序 + 可选依赖只用于低频服务的纪律收敛）。**复审点**：若 SDK 类型系统能结构性禁掉 setup 期可用性探测（无 setup 探测型插件），可放宽为不重启。
- **W3 兑现注（2026-10-19）**：provide 观测 = `services.provide(key)` import（setup 内限定；宿主据此保留「声明了没 provide = 激活失败」「provide 了没声明 = 响亮」）；deps 目录 = `services.deps()`（manifest inject 声明集 + required / optional / available）；未声明 inject 的调用 = trap（S3 响亮 × [wit-abi](wit-abi.md) D139）；方法集仍归契约，无契约服务的 missing method = guest catch-all 返回 `business`，宿主只在契约已知时短路 `service-method-missing`（[wit-abi](wit-abi.md) W3 D147）。
- **W9 兑现注（2026-10-19）**：缺失三分 / 可见集 / 撞名 / 判环 / provide 履行对账 = [wit-abi](wit-abi.md) W9 D180–D185；S1「方法集入注册表」按「有契约服务 = 宿主从契约载入；contractless 无方法集」兑现，提供方不注册方法集（W9 D183）；S2 原子注册、S3 optional 重启、S4 恢复路径不变。

v1 简化：inject 由 manifest 固定，entry 层不覆盖（Cordis 两处可写，我们一处）；同插件多实例的 inject 分化见用户再加。

## 4. 调用语义：错误与并发（S5 ✅）

### 失败全景与 Result 形状

| # | 死法                                   | 归类                                                |
| - | -------------------------------------- | --------------------------------------------------- |
| 1 | 调用未声明 inject 的 key               | 响亮报错（S3 纪律），**不经 Result**                |
| 2 | key 已声明但服务当前不在               | `Err(Unavailable)`                                  |
| 3 | 方法名不存在                           | `Err(MethodMissing)`（host 查注册表拦下，不走消息） |
| 4 | 投递时提供方邮箱已关                   | `Err(Unavailable)`（竞态无锁化解）                  |
| 5 | 提供方 handler 抛错/主动返回错误       | `Err(Business, Value)`                              |
| 6 | 超 deadline / 提供方被 fuel/epoch 截断 | `Err(Timeout)`                                      |

- **`Err(Business)` 是契约的一部分**（如 query 对不存在 key 返回 not_found），调用方必须显式处理；三个框架错误表拓扑/配置问题，合理反应是降级或传播
- 观测分工：调用方统一拿 `Err(Business)`；host 日志区分契约内错误与 handler 崩溃（后者进故障计数）——不给调用方暴露第三种类型
- **`Err(Unavailable)` 的精确语义（附议修正）**：可选依赖的常态答案 + 提供方**崩溃窗口**的竞态兜底（host 发现崩溃 → 启动依赖者卸载之间的固有延迟，任何异步系统不可消除）；**稳态必需依赖设计上不可达**（激活门 + 卸载顺序双保险）；**热更窗口不提供等待机制**——服务对象窄（依赖者正在死、可选依赖缺席即信息），重试策略归未来 `lib/` helper；扩展位：实测痛点出现再加 host 侧有界等待（机制与 Pending 激活门同族）
- **W5 兑现注（2026-10-19）**：S5 四态 = [wit-abi](wit-abi.md) W5 D137 的 `service-unavailable` / `service-method-missing` / `service-timeout` + 共享 `business`；宿主框架族 `core-deadlock` / `core-contract-violation` 不占服务第五格，但出现在服务调用路径的调用方可见错误集合中（四态 + core 两格）；注册表语义不变。
- **W3 兑现注（2026-10-19）**：per-call deadline 的作者面 = `services.call(service, method, args, timeout-ms)`；args / 返回 = 单个 TLV `Value`；缺席 timeout = 全局默认，0 / 非法 = `core-contract-violation` Err（[wit-abi](wit-abi.md) W3 D147）。

### 并发四条款

1. **同一提供方按到达顺序开始处理，不保证完成序**：handler 挂起期间后一条调用可进入同一实例（[wit-abi](wit-abi.md) W4.1 自由交错）；吞吐上限 = 单实例处理能力；扩容 = 多实例或外部服务（实例作用域各一份，同作用域内唯一性原样，[instance-model](instance-model.md) §S4.3 D69）
2. **deadline 即边界**：全局默认 + per-call 覆盖（作者面形态归 W3 章）；等待环检测到闭合即 fail-fast（[wit-abi](wit-abi.md) W4.4），超时是最后保险丝（[ADR-0011](../decisions/0011-actor-execution-model.md)）
3. **超时不取消**：deadline 到 = 调用方不再等，提供方自然跑完、结果丢弃；跨实例取消登记扩展位（中断已产生部分副作用的执行，危险远大于省下的算力）
4. **交错是明文执行模型**：插件不得依赖「一个调用没完下一个不开始」——等待期间邮箱下一条先跑（[wit-abi](wit-abi.md) W4.1）；需要互斥 = SDK async mutex 助手，宿主零同步原语（W4.2）

### epoch 与调用：构造性消解，零校验

host 每次调用现查注册表，路由永远指向当前活着的实例；路由后处理前提供方被卸载 → 邮箱关闭 → `Err(Unavailable)`。陈旧快照由重启纪律在调用路径**之外**解决——热路径无锁，竞态靠构造消解（派发细则的同一把戏，[ADR-0014](../decisions/0014-dispatch-semantics.md)）。

## 5. epoch 与故障生命周期（S4 ✅）

机制本体 = `fiber.md` §5 五步（不重开）。边角条款：

- **新提供方加载失败**：消费者停在 `Pending` 等依赖（顺序铁律：提供者先活消费者才启）。**失败是显式状态不是悬挂**——Pending/Failed 都流经 `internal/plugin`/`internal/status` 事件（[ADR-0016](../decisions/0016-internal-hooks.md)），无静默卡死；**恢复是配置驱动闭环**：修配置/回滚 → 提供方起 → 唤醒 Pending → 自动收敛
- **check 谓词 v1 否决**：S2 原子注册已消掉「注册了没 ready」窗口（注册 = 可用，一布尔）；扩展位与 withdraw 同族，典型候选用户 = 长连接服务「重连期间暂停接客」
- **epoch 单调不复用**：key 消失后计数器不重置，重来继续 ++——任何 `(key, epoch)` 全局唯一指认一个 incarnation，防 ABA（与 arena 世代 FiberId 同思想）

### 故障生命周期三问（附议）

**「可恢复失败」的正确提法**：fiber 模型里恢复从来不是就地修复，**任何恢复都是完整重载**；分类在触发源——配置修复 ✅ 主路径 / 手动重载 ✅ / 自动重试（指数退避）❌ v1 不做（崩溃循环 + 日志噪音，违背响亮失败；登记扩展位）。

**依赖者关闭如何保证——检测是结构性的，不是心跳**：host 拥有 arena、注册表、反向索引与每个 actor 的任务句柄；插件死亡 = worker 任务终结（panic/时间盒/setup 失败）= JoinHandle 完成 + 邮箱关闭，host 作为路由器必然观察到。登记链：fiber 转 Failed → 摘除其服务 key（epoch 失效）→ 反向索引查依赖者 → 拓扑序卸载。崩溃场景差异：S2 的「清理期间提供方可用」只覆盖**计划内**卸载——崩溃时依赖者 disposer 调死服务拿 `Err(Unavailable)`，清理不因此中断（disposer 错误逐个吞掉、drain 继续）。

**恢复后如何重启依赖者——恢复路径 = 启动路径复用**：Pending 纤维记着缺的 key；任何 key 注册时 host 查反向索引唤醒等待者 → 消费者 setup 重跑（重新 inject、记新指纹）→ 它自己提供的服务又唤醒下一级——**启动、热更、崩溃恢复共享同一条「注册唤醒 Pending」路径**，无特殊分支；级联由反向索引天然排出拓扑序；状态不迁移（新实例 setup 重建一切）。

### 重启语义四段论（附议）

|              |                                                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------------------------------ |
| **强制发起** | 插件不能否决——epoch 一致性不变量不容谈判（veto 通道未开，[ADR-0016](../decisions/0016-internal-hooks.md)）         |
| **优雅执行** | 卸载走插件自己的 disposer 链（异步汇合）：**在途持久化任务入账，disposer 可 await 它完成**——通知收尾，不是 SIGKILL |
| **超时强制** | fuel/epoch 时间盒兜底，deadline 到强制丢弃（记日志）；持久化慢的插件自担风险 → 增量落盘纪律                        |
| **顺序保护** | 计划内卸载：提供方等所有依赖者 settle 才下架——**清理期间服务仍可用**，最后的持久化能写完                           |

**状态三分纪律**（重启频繁引出的编程模型）：持久状态（丢不起）→ 显式落盘 seam / 状态服务；可重建状态（丢了无碍）→ 插件内 cache，重启即丢 setup 重建；会话状态 → session 服务持有（[instance-model](instance-model.md) 判别两问）。哲学一句：**行为在插件，状态在服务**。持久化写法纪律：原子写（tmp + rename，崩溃撕不坏）；状态服务应提供原子原语。

## 6. Capability Seam 三角的 Metis 形态（S6 ✅）

Provider/Consumer 两角已锁（§2/§3：manifest 双声明 + 只认 key + 无运行期模块系统使「import provider」物理不可能）。**Definition 角**：

1. **原则**：契约（key + 方法集 + 语义）**独立于提供方存在**——这是防「契约漂移 → 插件的插件」的关键。框架级服务（`llm`/`tools` 等；实现是插件——core 不提供任何 agent 领域服务实现）契约**必须**有 `lib/` 契约包；插件自定义服务 v1 允许文档约定起步（无契约 = 无形状对账），一旦声明契约，方法集住契约，存在性机器可查（[wit-abi](wit-abi.md) W7 D167）

> 产品模型注（纠正此前隐含前提）：core = 插件管理与组合机器 + syscall 层组件运行时基底（为满足核心层任务的原语可提供），**不提供任何 agent 领域服务**；llm/tools 等框架级能力的实现是插件（发行可捆绑参考实现，可替换），契约仍必须 `lib/` 契约包。产品模型结论继续有效（载体转向 = [ADR-0034](../decisions/0034-wasm-first-carrier.md)）。

2. **载体形态**：契约包 = `contract.yml` + 文档（[wit-abi](wit-abi.md) W7 D164，2026-10-19）；WIT 只承载平台固定 world。方法集与事件形状住契约包（W7 章 = T13-m）。`lib/` = 部署级契约包目录，插件随包契约同形。
3. **schema 化路径**：方法签名级校验归 WIT 富类型（平台封闭面）与瘦身形状 DSL（Value 形状面），**与事件 payload 形状共享同一套契约包**；Definition 包从「常量 + 文档」升级为「类型 + 文档」——W7/W8 章（T13-m/T13-n）。**W7 兑现注（2026-10-19）**：契约包 = `contract.yml` + 文档；四节 = `types` / `methods` / `errors` / `events`；作者类型由 `types!()` 生成（[wit-abi](wit-abi.md) W7 D164–D170）。**W8 兑现注（2026-10-19）**：事件形状总表 = 三来源契约 `events:` 并集；事件声明面 = 审批面、运行期不强制（[wit-abi](wit-abi.md) W8 D173–D179）。
4. **契约演化规则**：加方法 = 兼容；删方法 / 改签名 = 破坏（契约版本 +1；动态消费者级联重启用，编译期消费者须重编译；调用期 `service-method-missing` 当场响亮）。schema 化后启用 / 审批时静态判定变更级别，审批界面显示「此更新破坏 N 个消费者 / M 个需重编译」（[wit-abi](wit-abi.md) W7 D171）。
5. **自省目录**：注册表 + 双声明使服务目录（谁提供什么/方法集/谁依赖谁）纯推导可得；对模型暴露（Harness `cordis_inspect` 两级形态：列表省 token、详查带文档）登记工具面。

**方法发现三层（附议）**：契约文档（Definition）→ 契约方法集（机器可读，W7 D167）→ 运行时自省目录（live catalog，模型可查）——消费者作者无需读提供方源码。

## 7. lib/ 布局、作者语境与包管理（附议；crate 部分已由 [ADR-0019](../decisions/0019-crate-layout.md) 细化，契约面归 W7 章）

**三个作者语境**（先前讨论隐含「插件作者在部署内开发」的误解——只有平台开发与 agent 创作在部署内，第三方作者在自己仓库）：

| 语境                  | 工作区                            | 与部署的关系                                                                                                                                                                   |
| --------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 平台开发者            | metis monorepo                    | 捆绑参考实现插件（组件插件，可替换，非核心层内置）+ 契约包 `lib/` 随核心版本演进；dogfood 部署根在仓库内——「开发即生产」**只在此语境成立**                                     |
| 第三方插件作者        | **自有 git 仓库**（一插件一仓库） | 仓库内含自己的开发期契约副本（复制到包内 `lib/`，指向本机契约包 / SDK）；完成品安装进部署 = 复制进 `plugins/` + 审批事务（`metis install` 工具化归将来）；开发期配置不随包分发 |
| agent（Creator 模式） | 部署内 `plugins/`                 | 直接在部署内创作的作者（[ADR-0008](../decisions/0008-creator-mode-self-modification.md) 审批事务；Creator 权重已降，审批管线载体中性）                                         |

**运行期 vs 开发期契约来源**：host 只认部署根的 `lib/` 契约包与插件随包契约；插件仓库的开发期来源仅服务本机开发（复制进包内 `lib/` 后才参与构建与运行），运行期不可见（不在部署内）——防契约欺骗因此是物理事实而非忽略机制。v1 第三方作者的契约包副本来源 = 本机 metis 检出（编译期只读包内副本；[wit-abi](wit-abi.md) W7 D170）；**v1 插件作者体验是「维护者时代」体验，第三方时代以分发机制为门槛**（[ADR-0004](../decisions/0004-plugin-forms.md) 推迟项）。

```
<部署根>/                  ← 平台/部署环境
├── metis.yml             ← 装配单（配置树正本）
├── lib/                  ← 部署级契约包（contract.yml + 文档）
└── plugins/              ← 插件目录（ADR-0004 + D35：目录单形态）
```

（部署根布局 SSOT = [config-format](config-format.md) §C1.1 D29：`metis.yml` 单文件、插件 = 目录单形态、`secrets.yml`/`journal/`/`overlay.yml` 等件。）

- 框架级契约包随核心版本演进（v1 monorepo 内 `lib/`；契约包 SSOT 入仓）；契约包版本面 = W7 D165 / W10 D191，工具链钉版纪律 = W10 D192 已兑现
- **契约包无运行期模块副本**：跨插件代码共享 = 编译期依赖；`lib/` 只承载契约数据（`contract.yml` + 文档），不承载可执行模块——不存在「各插件看到不同世界」的模块状态问题
- 契约变更 = 契约版本（单整数）+ 规范化形状比较 → 动态消费者走反向索引级联重启用 / 编译期消费者须重编译（随包契约同名冲突，[wit-abi](wit-abi.md) W7 D171）；管理员级变更走审批；v1 lib 无独立版本号（契约版本 ≡ 核心版本 + 生态契约 `version:` 字段，W7 章）
- **v1 无包管理**：插件 = 目录，安装 = 放文件 + 配置树登记（走 Creator 审批事务）。各语言包管理生态（Cargo / npm 等）属 guest 侧；分发时代按 [ADR-0004](../decisions/0004-plugin-forms.md) 登记长回——包 = manifest + 组件 + 依赖声明（机器可读，依赖解析数据基础白送），安装 = fetch + verify + Creator 事务，主要用户是 agent 自己

## 8. 决策点状态

| #  | 决策点                                                                    | 状态                                                                             |
| -- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| S1 | 服务 = key + 命名方法集；无属性；时刻唯一                                 | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S2 | provide = fiber effect；注册 = 激活点原子动作；无 withdraw                | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S3 | inject/provides manifest 双声明 SSOT；可选依赖统一重启                    | ✅ [ADR-0018](../decisions/0018-service-registry.md)（闭环 inject 静态声明问题） |
| S4 | 故障生命周期：结构检测 / 拓扑卸载 / 恢复=启动复用；check 否决；epoch 单调 | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S5 | Result 四分；热更窗不等待；按到达开始 / deadline / 不取消 / 交错明文      | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S6 | Definition 独立存在；框架级契约 lib/ 包；演化规则；自省目录               | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
