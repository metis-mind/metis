# Metis — 服务注册表与 inject 设计

> 状态：S1–S6 ✅ 全部确认（2026-10-02），固化为 [ADR-0018](../decisions/0018-service-registry.md)。
> 2026-10-02 讨论。依据：[ADR-0001](../decisions/0001-inject-runtime-checks.md)（inject 运行时检查）/ [0003](../decisions/0003-module-require-discipline.md)（require 纪律）/ [0010](../decisions/0010-fiber-core.md)（fiber 核心）/ [0011](../decisions/0011-actor-execution-model.md)（actor 模型）/ [0013](../decisions/0013-context-scope.md) / [0014](../decisions/0014-dispatch-semantics.md) / [0015](../decisions/0015-payload-value-model.md)；调研：`../research/cordis-research.md`（Capability Seam 三角、Service 实现细节）。

---

## 1. 服务的形态（S1 ✅）

**服务 = key + 命名方法集**；调用 `call(key, method, args) → Result`（Luau 语法归 ABI）。

- **args/返回值 = 单个 `Value`**（惯例 Map 即命名参数）：跨 VM 边界一次转换，schema 校验单挂点
- **方法集入注册表**：调不存在的方法 = host 当场框架错误（不走消息）；同时是 schema 挂点（ABI 任务）与自省目录数据源
- **key 命名**：扁平字符串；核心服务占短名（`llm`/`tools`），插件服务建议 `plugin-name/service` 前缀（约定不强制，与事件命名纪律同构）；重复注册响亮失败 = 命名冲突加载期暴露
- **服务没有"属性"**：跨 VM 只有方法调用（消息），无共享对象——服务状态是提供方 VM 私有，访问必经方法（actor 纯化，Cordis 的字段穿透是 Proxy 遗物）
- **LLM 工具 = 服务方法的投影**：tool registry 插件把 inject 来的方法集翻译成模型 tool schema——插件写的服务自动成为模型可用的工具（schema 化后），无手工翻译层
- **唯一性（附议）**：任何时刻全局唯一（重复注册响亮）；跨时间可替换（配置变更 + epoch+1）；同时多实现归 isolate（[ADR-0013](../decisions/0013-context-scope.md) 推迟项）
- 一个插件可提供多个服务：每 key 独立注册、独立 epoch

注册表数据结构（host 侧，v1 全局单级）：

```
HashMap<ServiceKey, ServiceEntry { provider: FiberId, epoch: u64, methods: Vec<MethodName> }>
+ 反向索引 HashMap<ServiceKey, Set<FiberId>>   // service → dependents
+ ScopeTag 预留位（isolate 加回时用）
```

## 2. provide 与注册时机（S2 ✅）

**`ctx.provide(key, 方法表)` = fiber effect**；**注册 = 激活转换点的原子动作**：

1. setup 内调用 provide 入待注册清单并入账；setup 失败 → 半成品 drain，不留半注册状态
2. setup 成功 → manifest 对账（声明 keys 全履行，缺 = 激活失败）→ **原子注册** → fiber 转 Active
3. **注册即可用、可用即 Active，一个布尔无中间态**——"已注册未 ready"窗口整类消失（增量上架被否：它引入三不管窗口）
4. 重复 key 双保险：静态（配置树扫出两 entry 提供同 key → 加载期响亮）+ 运行时防御（host 复查）
5. 注销 = 卸载自动摘除（账本 LIFO）；**v1 不做主动 withdraw**——临时退化（DB 断连）用方法返回业务错误表达；withdraw 登记扩展位（与 §5 check 谓词同族）
6. 必需 key 注册 → 反向索引唤醒 Pending 消费者 → 拓扑序激活（级联见 §5）

纪律：setup 只做激活必需的初始化；重活后置（内部任务 + 业务错误窗口）；setup 有 VM 时间盒兜底，不会无限卡住激活门。

## 3. inject：manifest 双声明 SSOT（S3 ✅，闭环 inject 静态声明问题）

**结构性论证**：激活门（依赖不齐不启动，fiber `Pending`）要求依赖在**运行插件代码之前**可知——动态声明意味着先跑代码才知道依赖，跑代码时 fiber 已 Active，激活门失效。故问题不是"要不要静态"，是"静态到什么程度"。

**manifest 是唯一书写点，代码只消费声明的产物**：

```yaml
# 插件 manifest（示意形态，格式细节归 ABI/配置任务）
name: memory-viewer
provides: []            # 我暴露什么服务
inject:
  required: [memory]    # 没有它我无法工作
  optional: [metrics]   # 有它更好
```

- **inject：代码零声明**。setup 时 host 按声明装配依赖句柄注入（`deps.memory:call(...)` 形态归 ABI）——未声明的服务**结构性拿不到**，不是报错纪律，是无从表达
- **provides：激活期强制对账**。声明了没 provide → 激活失败；provide 了没声明 → 报错。**漂移无法被发布**——系统层面消除双份维护（SSOT 要求的落地形态；Luau 无静态分析工具链且运行时探测驱动不了激活门，故方向只能 manifest-first）
- **缺失处置两分**：提供方在配置树中存在但未就绪 → 消费者 Pending **等待**（就绪后经反向索引唤醒）；配置树中**无任何提供方** → **加载响亮失败**（静态校验，杜绝打错字静默卡死）——后者要求 provides 也静态声明，否则供给不可见、校验无从谈起

**actor 模型没有"过期服务引用"**：Cordis 的 epoch 重启解决"持有的服务对象引用过期"；Metis 的调用是 host 每次现查的路由消息，无可过期之物。epoch 重启的新理由：**激活门 + setup 期快照刷新**（插件 setup 缓存的派生状态在依赖热更后过期，干净重启重跑 setup 刷新——保守但可推理，[ADR-0010](../decisions/0010-fiber-core.md) 哲学）。

**可选依赖精确语义**：静态声明；缺席**不阻塞激活**、调用时缺席返回框架错误；**出现/消失/热更统一重启消费者**（"依赖视图变化 → 重启"一条不变量无例外；惊扰面由反向索引限定 + 拓扑序 + 可选依赖只用于低频服务的纪律收敛）。**复审点**：若 ABI 能结构性禁掉 setup 期可用性探测（无 setup 探测型插件），可放宽为不重启。

v1 简化：inject 由 manifest 固定，entry 层不覆盖（Cordis 两处可写，我们一处）；同插件多实例的 inject 分化见用户再加。

## 4. 调用语义：错误与并发（S5 ✅）

### 失败全景与 Result 形状

| # | 死法                                 | 归类                                                |
| - | ------------------------------------ | --------------------------------------------------- |
| 1 | 调用未声明 inject 的 key             | 响亮报错（S3 纪律），**不经 Result**                |
| 2 | key 已声明但服务当前不在             | `Err(Unavailable)`                                  |
| 3 | 方法名不存在                         | `Err(MethodMissing)`（host 查注册表拦下，不走消息） |
| 4 | 投递时提供方邮箱已关                 | `Err(Unavailable)`（竞态无锁化解）                  |
| 5 | 提供方 handler 抛错/主动返回错误     | `Err(Business, Value)`                              |
| 6 | 超 deadline / 提供方被 VM 时间盒截断 | `Err(Timeout)`                                      |

- **`Err(Business)` 是契约的一部分**（如 query 对不存在 key 返回 not_found），调用方必须显式处理；三个框架错误表拓扑/配置问题，合理反应是降级或传播
- 观测分工：调用方统一拿 `Err(Business)`；host 日志区分契约内错误与 handler 崩溃（后者进故障计数）——不给调用方暴露第三种类型
- **`Err(Unavailable)` 的精确语义（附议修正）**：可选依赖的常态答案 + 提供方**崩溃窗口**的竞态兜底（host 发现崩溃 → 启动依赖者卸载之间的固有延迟，任何异步系统不可消除）；**稳态必需依赖设计上不可达**（激活门 + 卸载顺序双保险）；**热更窗口不提供等待机制**——服务对象窄（依赖者正在死、可选依赖缺席即信息），重试策略归未来 `lib/` helper；扩展位：实测痛点出现再加 host 侧有界等待（机制与 Pending 激活门同族）

### 并发四条款

1. **同一提供方串行**：调用按到达顺序开始处理；吞吐上限 = 单实例处理能力；扩容归 isolate（推迟项）
2. **deadline 即边界**：全局默认 + per-call 覆盖（语法归 ABI）；等待环不死锁只超时（[ADR-0011](../decisions/0011-actor-execution-model.md)）
3. **超时不取消**：deadline 到 = 调用方不再等，提供方自然跑完、结果丢弃；跨 VM 取消登记扩展位（中断已产生部分副作用的执行，危险远大于省下的算力）
4. **交错留缝**：handler 等待期间是否让出邮箱归 ABI/实现优化；**语义上插件不得依赖"一个调用没完下一个不开始"**

### epoch 与调用：构造性消解，零校验

host 每次调用现查注册表，路由永远指向当前活着的实例；路由后处理前提供方被卸载 → 邮箱关闭 → `Err(Unavailable)`。陈旧快照由重启纪律在调用路径**之外**解决——热路径无锁，竞态靠构造消解（派发细则的同一把戏，[ADR-0014](../decisions/0014-dispatch-semantics.md)）。

## 5. epoch 与故障生命周期（S4 ✅）

机制本体 = `fiber.md` §5 五步（不重开）。边角条款：

- **新提供方加载失败**：消费者停在 `Pending` 等依赖（顺序铁律：提供者先活消费者才启）。**失败是显式状态不是悬挂**——Pending/Failed 都流经 `internal/plugin`/`internal/status` 事件（[ADR-0016](../decisions/0016-internal-hooks.md)），无静默卡死；**恢复是配置驱动闭环**：修配置/回滚 → 提供方起 → 唤醒 Pending → 自动收敛
- **check 谓词 v1 否决**：S2 原子注册已消掉"注册了没 ready"窗口（注册 = 可用，一布尔）；扩展位与 withdraw 同族，典型候选用户 = 长连接服务"重连期间暂停接客"
- **epoch 单调不复用**：key 消失后计数器不重置，重来继续 ++——任何 `(key, epoch)` 全局唯一指认一个 incarnation，防 ABA（与 arena 世代 FiberId 同思想）

### 故障生命周期三问（附议）

**"可恢复失败"的正确提法**：fiber 模型里恢复从来不是就地修复，**任何恢复都是完整重载**；分类在触发源——配置修复 ✅ 主路径 / 手动重载 ✅ / 自动重试（指数退避）❌ v1 不做（崩溃循环 + 日志噪音，违背响亮失败；登记扩展位）。

**依赖者关闭如何保证——检测是结构性的，不是心跳**：host 拥有 arena、注册表、反向索引与每个 actor 的任务句柄；插件死亡 = worker 任务终结（panic/时间盒/setup 失败）= JoinHandle 完成 + 邮箱关闭，host 作为路由器必然观察到。登记链：fiber 转 Failed → 摘除其服务 key（epoch 失效）→ 反向索引查依赖者 → 拓扑序卸载。崩溃场景差异：S2 的"清理期间提供方可用"只覆盖**计划内**卸载——崩溃时依赖者 disposer 调死服务拿 `Err(Unavailable)`，清理不因此中断（disposer 错误逐个吞掉、drain 继续）。

**恢复后如何重启依赖者——恢复路径 = 启动路径复用**：Pending 纤维记着缺的 key；任何 key 注册时 host 查反向索引唤醒等待者 → 消费者 setup 重跑（重新 inject、记新指纹）→ 它自己提供的服务又唤醒下一级——**启动、热更、崩溃恢复共享同一条"注册唤醒 Pending"路径**，无特殊分支；级联由反向索引天然排出拓扑序；状态不迁移（新实例 setup 重建一切）。

### 重启语义四段论（附议）

|              |                                                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------------------------------ |
| **强制发起** | 插件不能否决——epoch 一致性不变量不容谈判（veto 通道未开，[ADR-0016](../decisions/0016-internal-hooks.md)）         |
| **优雅执行** | 卸载走插件自己的 disposer 链（异步汇合）：**在途持久化任务入账，disposer 可 await 它完成**——通知收尾，不是 SIGKILL |
| **超时强制** | VM 时间盒兜底，deadline 到强制丢弃（记日志）；持久化慢的插件自担风险 → 增量落盘纪律                                |
| **顺序保护** | 计划内卸载：提供方等所有依赖者 settle 才下架——**清理期间服务仍可用**，最后的持久化能写完                           |

**状态三分纪律**（重启频繁引出的编程模型）：持久状态（丢不起）→ 显式落盘 seam / 状态服务；可重建状态（丢了无碍）→ 插件内 cache，重启即丢 setup 重建；会话状态 → session 服务持有（`session-model.md` 轴 2）。哲学一句：**行为在插件，状态在服务**。持久化写法纪律：原子写（tmp + rename，崩溃撕不坏）；状态服务应提供原子原语。

## 6. Capability Seam 三角的 Metis 形态（S6 ✅）

Provider/Consumer 两角已锁（§2/§3：manifest 双声明 + 只认 key + 禁跨插件 require 使"import provider"物理不可能）。**Definition 角**：

1. **原则**：契约（key + 方法集 + 语义）**独立于提供方存在**——这是防"契约漂移 → 插件的插件"的关键。核心服务（`llm`/`tools` 等框架级）契约**必须**有 `lib/` 契约包；插件自定义服务 v1 允许文档约定起步，但 manifest 方法集使存在性永远机器可查
2. **载体形态登记**：`lib/` 契约包细节与"纯 manifest 契约包"候选的取舍，依赖 manifest 格式（`lib/` 引用语法已定 = [ADR-0019](../decisions/0019-crate-layout.md)）——归 ABI 任务
3. **schema 化路径**：方法签名级校验归 ABI 任务的 schema DSL，**与事件 payload schema 共享同一套**；Definition 包届时从"常量+文档"升级为"常量+schema+文档"
4. **契约演化规则**：加方法 = 兼容（消费者重启但旧代码照常工作）；删方法/改签名 = 破坏（epoch+1 全消费者重启 + 调用期 `Err(MethodMissing)` 当场响亮）。schema 化后安装/审批时静态判定变更级别，审批界面显示"此更新破坏 N 个消费者"
5. **自省目录**：注册表 + 双声明使服务目录（谁提供什么/方法集/谁依赖谁）纯推导可得；对模型暴露（Harness `cordis_inspect` 两级形态：列表省 token、详查带文档）登记 ABI tooling

**方法发现三层（附议）**：契约文档（Definition）→ manifest 声明（机器可读）→ 运行时自省目录（live catalog，模型可查）——消费者作者无需读提供方源码。

## 7. lib/ 布局、作者语境与包管理（附议；crate 部分已由 [ADR-0019](../decisions/0019-crate-layout.md) 细化，余下归 ABI 任务）

**三个作者语境**（2026-10-02 用户指正：先前讨论隐含"插件作者在部署内开发"的误解——只有平台开发与 agent 创作在部署内，第三方作者在自己仓库）：

| 语境                  | 工作区                            | 与部署的关系                                                                                                                                                                       |
| --------------------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 平台开发者            | metis monorepo                    | 核心插件（bundled）+ 契约包 `lib/` 随核心版本演进；dogfood 部署根在仓库内——"开发即生产"**只在此语境成立**                                                                          |
| 第三方插件作者        | **自有 git 仓库**（一插件一仓库） | 仓库内含自己的 `.luaurc`（开发期产物，alias 指向本机契约包来源）；完成品安装进部署 = 复制进 `plugins/` + 审批事务（`metis install` 工具化归将来）；**插件的 `.luaurc` 不随包分发** |
| agent（Creator 模式） | 部署内 `plugins/`                 | 唯一直接在部署内创作的作者（[ADR-0008](../decisions/0008-creator-mode-self-modification.md) 审批事务）                                                                             |

**运行期 vs 开发期 `.luaurc`**：host 的 require 包装器只认部署根 alias 配置；插件仓库的 `.luaurc` 仅服务作者本机 LSP，运行期不可见（不在部署内）——防契约欺骗因此是物理事实而非忽略机制。v1 第三方作者的契约包来源 = 本机 metis 检出路径 alias；**v1 插件作者体验是"维护者时代"体验，第三方时代以分发机制为门槛**（[ADR-0004](../decisions/0004-plugin-forms.md) 推迟项）。

```
<部署根>/                  ← 平台/部署环境
├── .luaurc               ← 平台布线：lib → ./lib（部署者可追加；运行期唯一生效的一份）
├── plugins/              ← 插件（ADR-0004：foo.luau 或 foo/manifest.yml）
├── lib/                  ← 共享纯代码库（契约包、工具库）
└── config/               ← entry 树 YAML
```

- 核心契约包随核心版本演进（v1 monorepo 内 `lib/`；**内嵌二进制 + 磁盘数据目录两层**，[ADR-0019](../decisions/0019-crate-layout.md) C3）
- **lib 模块在每个插件 VM 里是独立副本**（模块缓存随 VM 生灭，[ADR-0005](../decisions/0005-vm-topology.md)）→ lib 必须**纯代码**（函数/常量/schema），顶层囤可变全局状态 = 各插件看到不同世界
- lib 变更 = 反向传递闭包定位失效插件集 → 重启（[ADR-0003](../decisions/0003-module-require-discipline.md)）→ 管理员级变更走审批；v1 lib 无独立版本号
- **v1 无包管理**：插件 = 目录，安装 = 放文件 + 配置树登记（走 Creator 审批事务）。LuaRocks（C 模块生态不适配沙箱 Luau）与 Wally/pesde（Roblox 生态）是参考非答案；分发时代按 [ADR-0004](../decisions/0004-plugin-forms.md) 登记长回——包 = manifest + 纯 Luau + 依赖声明（机器可读，依赖解析数据基础白送），安装 = fetch + verify + Creator 事务，主要用户是 agent 自己

## 8. 决策点状态

| #  | 决策点                                                                    | 状态                                                                             |
| -- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| S1 | 服务 = key + 命名方法集；无属性；时刻唯一                                 | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S2 | provide = fiber effect；注册 = 激活点原子动作；无 withdraw                | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S3 | inject/provides manifest 双声明 SSOT；可选依赖统一重启                    | ✅ [ADR-0018](../decisions/0018-service-registry.md)（闭环 inject 静态声明问题） |
| S4 | 故障生命周期：结构检测 / 拓扑卸载 / 恢复=启动复用；check 否决；epoch 单调 | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S5 | Result 四分；热更窗不等待；串行 / deadline / 不取消 / 交错留缝            | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
| S6 | Definition 独立存在；核心契约 lib/ 包；演化规则；自省目录                 | ✅ [ADR-0018](../decisions/0018-service-registry.md)                             |
