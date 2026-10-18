# 0028. 实例模型冻结（S1–S5）与既有 ADR 局部 supersede 批次

- Status: accepted（S4 节 service_call 格「replay 政策不动」半句失效注 + Consequences 任务 6 行结案——**已吸收 = [ADR-0029](0029-journal.md) 批次 #5**（2026-10-13：replay 不立 + 内部流量族撤销；「跨实例调用结构性不存在」与扩展位重开句原位）；fleet 配置文件清单 + `values.yml` = [ADR-0040](0040-config-shared-variables.md)（2026-10-19））
- Date: 2026-10-12

## Context

任务 5（实例模型：多租户托管与动态子树）2026-10-11～12 五站收官（S1 正名与用例归位 / S2 租户托管 / S3 声明与挂载 / S4 实例配置与隔离 / S5 v1 范围收口；决策点 D53–D74）。本议题前身 = 「session 模型与 preset 机制」种子稿（[session-model](../design/session-model.md)，2026-10-11 标记已吸收）：重构定调——**session/persona 是 agent 领域概念，核心不该管理；核心只提供抽象机制**（租户托管 + 动态子树挂载），project = 部署层租户。定案全文、淘汰选项、种子稿条款映射表与联动登记汇总表在 [instance-model](../design/instance-model.md)（living doc）。前提：[ADR-0010](0010-fiber-core.md)（子 fiber = 父的 effect）/ [0013](0013-context-scope.md)（Context 作用域与 parent 链预留）/ [0017](0017-programmatic-spawn-deferred.md)（编程式 spawn 禁令与汇合登记）/ [0008](0008-creator-mode-self-modification.md)（审批事务管线）/ [0023](0023-core-restart-semantics.md)（核心重启语义）；Luau ABI 冻结 = [ADR-0026](0026-luau-abi.md)、配置格式冻结 = [ADR-0027](0027-config-format.md)。本 ADR 冻结实例模型的决策面，并同批吸收各站在旧 ADR 上登记的局部 supersede 意向（interim 注已在各 living doc 对应节）。

**定调（任务 5 全程）**：领域词不进核心词汇表（session/persona = 插件层用子树机制搭建的概念）；子树声明是数据、住插件侧，配置树保持纯装配单；一切可运行代码都在装配单审批面（休眠件新角色 = 只供子树挂载，capabilities 同进审批 diff）。

## Decision

### S1 正名与用例归位

- **核心提供且仅提供两层实例机制**：**租户托管**（静态、文件背书）与**动态子树挂载**（动态、声明背书）。session/persona/project 等领域词不进核心词汇表——session/persona = 插件层实体（「核心是否有 session 概念」结案 = 无），project = 部署层租户。命名定案：租户（tenant）/ 子树声明 / 子树实例 / 动态子树挂载（subtree mount）。
- **project 用例 = 单进程多租户**：一个 server 进程托管多个部署根；每租户 = 独立配置树 / `plugins/` 树 / secrets / journal / 审批域；淘汰多进程每项目一 server（管理统一优先，隔离由故障域补回）与单树多项目。
- **persona 用例 = 插件层归位**：(a) 纯数据路由（常态，零核心机制）/ (b) 子树实例（重武器）两读法；机制需求下限 = 声明须能表达「不同插件集 + 实例配置值」；persona 建模不进冻结面。

### S2 租户托管

- **租户来源**：`--config` = 单租户模式原样；`--fleet-dir <dir>` = 多租户模式（一级扫描、租户名 = 子目录名、无 `metis.yml` 子目录静默忽略 + `sdk check` 提示级列出）；两旗标互斥。淘汰 host 配置文件 / `--config` 可重复 / 递归扫描。
- **租户集 = boot-only**：增删 = 改目录 + 进程干净重启（ADR-0023 全量 unload → load 同一筋）；代价 = 中断全部租户运行中会话（会话 = 短命子树实例，连续性归插件层持久化）；运行时租户控制通道 = 扩展位。
- **隔离与故障域**：租户 = host 根 fiber 下独立租户树；注册表 / 事件 / 部署索引 / journal / secrets 每租户独立；**故障域 = 租户树**（fleet 模式单租户配置解析失败 = 该租户 Failed、其余照常 boot；单租户模式维持响亮拒启动）；跨租户配额不立（扩展位）；核心 panic = 进程级诚实边界。
- **跨租户 = 默认零共享**：无跨租户 inject / 事件 / fs；协作 = 网络；重资源每租户自实例，共享服务外挂出口；跨租户共享插件实例 = 扩展位。
- **管理面 v1 三件套**：host 日志面（租户状态变迁；Failed = warn 政策同 D25.1 精神）+ `sdk check` fleet 校验 + 端口每租户自管（零路由机器）；**journal = 每租户一本**（段文件在各租户部署根 `journal/`，条目本体无租户字段）；运行时 status 通道 = 扩展位。

### S3 声明与挂载

- **声明形状 = Value 数据** `{ plugins: [{ use, config? }] }`（list 形允许同 key 多实例各带 overlay）；**素材约束**：`use` ⊆ 本租户装配单（enabled ∪ 休眠件）——休眠件新角色 = 只供子树挂载，审批面完整保持；引用未装配件 / 查无 = 挂载时响亮报错两分。淘汰自包含声明（违反目录单形态）与任意索引引用（审批后门）。
- **声明住处 = 两档分工**：审批档 = 提供方插件 `config.yml`（机器写走审批事务——ADR-0017「自我进化正当出口」在此兑现）；自管档 = 运行时构造或 `data/`（零审批）；核心不立任何新文件种类，住处选择归插件层。
- **mount API 与能力门**：`ctx.mount(declaration) -> handle` + 显式 unmount（句柄驱动）；**立门 `mount: true`**（capabilities map 新键——激活新代码的元权限，编排面能力门第一个例外；fail-closed 缺省原样）；mount 执行 = 标准启用管线复用（部署索引解析 → config 合成校验 → inject 解析 → setup，与 boot/热更同一条筋）。
- **生命周期**：挂载方 fiber = 父（ADR-0010 兑现到实例面）；父死 = 子树随账本 LIFO 结账；实例不参与全局重挂载协调（D41 作用面 = 部署树）。
- **快照语义 = 挂载时快照**：声明拷贝 + 配置合成结果 + 代码版本创建时固定，随实例死亡释放；声明演进 / `config.yml` 变更 / 被挂件热更只影响之后的新挂载；revision 引用计数机制不立（种子稿待决 #3/#5 结案）。
- **D7 汇合结案**：spawn（插件代码自由造新生命周期单元）维持关闭（内部任务 fiber = fiber effect 原样允许）；mount = 声明驱动 + 素材约束 + 能力门 = 编程式实例化唯一通道。

### S4 实例配置与隔离

- **实例配置合成 = `config.yml`（base）∪ 声明 `config:`（实例补丁）**，字段级稀疏合并（D38 语义复用）；校验 = 同一管线（manifest schema 校验 → 秘密引用解析 → 默认值合成，校验器单一实现），时机 = mount 时（启用管线原位），失败 = 挂载当场响亮报错；秘密引用政策零分叉（引用允许、写明文拒绝，不分住处档）；合成结果随实例快照固定，`config.yml` 后续热更只影响新挂载。**明记代价**：自管档声明（运行时构造、零审批）可携带秘密引用——秘密名绑定由挂载方代码运行期决定、不进审批 diff（缓解面 = `mount` 门 + 被挂件 schema `secret: true` 键 + 素材 ⊆ 装配单；收紧路径 = 秘密引用限审批档，登记扩展位）。
- **服务解析边界 = 实例作用域 + parent 链向上**：实例内 provide 住实例作用域、不进租户级注册表；inject 解析 = 沿 fiber parent 链先查自己作用域、逐级向上、租户注册表兜底；**可见性 = 向上单向**——兄弟实例互不可见、挂载方不能向下 inject（向下通信 = 事件总线，`serial` 派发 = 请求-响应形态）；判定面精确化 = provides 撞名 / 依赖判环 = 同作用域内，跨层 inject 边恒向上、结构性无环；mount handle 服务面出口 = 扩展位。
- **isolate / 部署侧重命名 / intercept / group 复审结案 = 四件维持不立**：isolate 语义由实例作用域 + 遮蔽序天然给出（不立字段；同时多实现 = 不同实例作用域各一份）；重命名维持不立（实例作用域已缓解撞名，「首个真实撞名需求」触发器存活）；intercept 职能由声明 `config:` overlay 覆盖；group 由子树实例天然嵌套覆盖。配置树 entry 封闭集 `{ enabled }` 不动；同插件多实例的实例面答案 = 声明 list 形，部署级维持复制目录。
- **epoch 失效波及 = 穿透到消费者粒度**：实例 inject 的提供方重挂载 → epoch 失效级联沿 inject 边进实例，实例内消费者 fiber 重启用、拿新烘焙句柄快照；实例快照三样不动（重跑 setup 用快照配置合成结果，不重读 `config.yml`）；与部署树消费者同一 epoch 纪律（先卸消费者、再卸提供方、加载反向）。
- **事件跨界 = 不立实例事件作用域**：实例内 on/emit 直上租户事件总线（每租户独立不动）；会话/persona 级路由 = payload 参数化 + selector 模式；实例 fiber 的 `internal/plugin` 生命周期事件照常进总线。明记代价 = 同租户跨实例事件互相可见（按 payload 过滤是监听者责任；审批覆盖 = manifest listen 名单）。
- **journal 条目不押实例字段**：条目只靠 fiber/plugin 标识 + 因果链；实例聚合 = 查询侧按子树根 fiber id 归组；实例标识 = 挂载根 fiber id；人读面 = mount/unmount 时 host 日志各一条。
- **跨实例服务调用 = 内部流量结案**：D68 边界下跨实例调用结构性不存在；`service_call` 族 replay 政策（重算不注入）不动；扩展位（跨租户共享实例 / handle 服务出口）开放时重开。

### S5 v1 范围与收口

- **v1 设计范围 = 任务 5 全部机制**（S2 租户托管 + S3 声明与挂载 + S4 实例配置与隔离）：mount 是 ADR-0017 登记的自我进化正当出口，裁掉它 Creator 模式不能激活新组合；四站机制互相咬合，无可裁子集。
- **种子稿待决 #4 结案 = 消解**：isolate/intercept/group/重命名不立 = 永久结案非推迟，无字段可谈「同期」；isolate 语义随子树天然在场。种子稿 §5「v1 最小集 / preset 留设计位」与 §6「v1 范围是否扩大」登记同案结案（被本范围取代）。
- **扩展位不进 v1**、原样存活（SSOT = living doc 各站联动登记汇总表）：运行时租户控制通道 / 跨租户配额 / 跨租户共享插件实例 / 运行时 status 通道 / handle 服务面出口 / 秘密引用限审批档。
- **实现排期不进冻结面**：实现期清单归 doc 各站登记与任务队列原位。

### 演进纪律（冻结面的变更规则）

1. **ABI 面**（capabilities `mount` 键、mount/unmount 方法族、判定面语义精确化等触及插件可见面的项）随 [ADR-0026](0026-luau-abi.md) 演进纪律：additive → `abi_rev` +1（本批 #1 同批执行一次 bump）；破坏式 → 新 ADR supersede。
2. **机制面破坏式变更**：新 ADR supersede（点名取代本 ADR 正文格）。
3. **living doc 保持 living**：细则澄清、教学条、实现期登记继续沉淀于 [instance-model](../design/instance-model.md)；凡改语义的修订不允许只走 doc 修订注，必须按本条 1/2 走。
4. 各站登记的扩展位与复审触发器以 doc 各站联动登记汇总表为 SSOT；触发器到来时按本条 1/2 走。

### 对既有 ADR 的局部 supersede（同批吸收）

各旧 ADR 正文冻结不动、Status 行记录本条；以下逐条命名被取代/兑现/结案的确切条款：

1. **[ADR-0026](0026-luau-abi.md)** A2 节顶层字段封闭集条与字段形状条 capabilities 格、A3 节 ctx 方法集面、A9 节能力门强制点总表条「编排面（事件 / provide / inject）不立能力门」句——additive 增补：capabilities map + `mount: true` 键（子树挂载元权限；编排面能力门第一个例外，间接化论证在「激活新代码」处断开）；ctx 编排面 + `ctx.mount(declaration) -> handle` / `unmount` 方法族。按 A10 节 bump 纪律同批执行核心 `abi_rev` +1（新词汇 + 新 ctx 方法；rev 数值落点 = 实现期编译期常量，归实现期清单）。
2. **[ADR-0026](0026-luau-abi.md)** A2 节字段形状条 config schema 格「`secret: true` = 引用式秘密，配置树写明文 = 校验响亮拒绝」——扩展：秘密引用写处/解析源 = 插件目录 `config.yml` ∪ 子树声明 `config:` overlay（实例配置合成经同一管线；引用允许 / 写明文拒绝不变）。明记代价见 S4 节。
3. **[ADR-0026](0026-luau-abi.md)** A9 节「inject 缺失三分」「provides 撞名 = 启用期响亮，判定作用面 = 已启用 ∪ Pending（含本次启用者）」「依赖禁环」三条——精确化：提供方可见集 = 沿 fiber parent 链可达集（兄弟/子实例提供方不可见 = 查无同族响亮）；撞名与判环判定作用面 = **同作用域内**已启用 ∪ Pending；跨层 inject 边恒向上、结构性无环。部署树单作用域下行为不变（向后兼容）。
4. **[ADR-0026](0026-luau-abi.md)** A6 节 journal 记录格式草案条（living doc §A6.7「journal 属主 = host 进程（每个 host 实例一本）」文）与 Consequences「`session` 字段复审随任务 5」——兑现/结案：属主 = host 托管 N 本、每租户一本（段文件在各租户部署根 `journal/`）；核心无 session 概念结案；条目不押实例字段（fiber/plugin 标识 + 因果链，实例聚合归查询侧按子树根 fiber id）。
5. **[ADR-0026](0026-luau-abi.md)** A2 节 service 撞名三层条「真撞 = 启用期响亮 + 本地解（部署者二选一 / isolate / 重命名）」——结案记录：isolate/重命名维持不立（任务 4 结案 + 任务 5 复审确认：isolate 语义由实例作用域 + parent 链遮蔽天然给出；部署树内撞名仍 = 部署者二选一；重命名的「首个真实撞名需求」触发器存活）。
6. **[ADR-0027](0027-config-format.md)** C1 节「`--config path/to/metis.yml` 选定文件即选定根」、C3 节「boot 解析失败 = 响亮拒启动」、C6 节 secrets「解析失败两分」——多租户细化：`--fleet-dir <dir>` 舰队目录模式新增（一级扫描、租户名 = 子目录名、两旗标互斥）；fleet 模式下单租户配置（`metis.yml` / `overlay.yml` / `secrets.yml`）解析失败 = 该租户 Failed、其余照常 boot；单租户模式各项原语义不变。
7. **[ADR-0027](0027-config-format.md)** C2 节 D31 已知牺牲条与 D32 四件不立条的「复审触发器 = 首个真实撞名需求或任务 5 子树」——结案记录：任务 5 子树已落地，复审结案 = 维持不立（isolate / intercept / group / 重命名；entry 封闭集 `{ enabled }` 不动；同插件多实例的实例面答案 = 声明 list 形，部署级维持复制目录）；冻结语义不变（本条目 = 触发器兑现记录，先例 = ADR-0027 批次 #13 同类结案记录）。

另：**[ADR-0013](0013-context-scope.md)** 与 **[ADR-0017](0017-programmatic-spawn-deferred.md)** 以 Status 行注记录复审点结案（不进批次条目）：0013 = D2 强制复审点结案（isolate/intercept 维持不立，isolate 语义由实例作用域天然给出；「Context 预留 parent 链」已消费 = 实例服务解析沿 parent 链先查自己作用域再向上）；0017 = D7 汇合点结案（spawn 的第一真实用户 = mount——声明驱动 + 素材约束 + 能力门 = 编程式实例化唯一通道；通用 `ctx:spawn` 维持关闭，其扩展位原样）。

## Consequences

- 实例模型五站（S1–S5，D53–D74）自此冻结；全文 rationale、淘汰选项、种子稿条款映射与代码示例以 living doc 为准，doc 头部状态行已指向本 ADR。
- **ABI 面影响**：capabilities + `mount: true` 与 `ctx.mount`/`unmount` 方法族进字段面 = additive → 核心 `abi_rev` +1（批次 #1 执行）；用了新面的插件写新号（ADR-0026 A10 节既有纪律）。
- 实现期清单（实例作用域注册表 = ScopeTag 预留位消费 / parent 链解析序 / 实例配置合成器 / epoch 级联穿透实例内消费者粒度 / mount·unmount API 与启用管线复用 / 声明素材约束校验器 / mount 消耗预算 / mount·unmount host 日志点 / fleet 目录扫描与 `sdk check` fleet 校验）不进冻结面，归 doc 各站登记。
- 后续任务输入：任务 6（journal/replay）= journal 每租户一本（S2）+ 条目不押实例字段（S4）+ 跨实例调用 = 内部流量（S4）；任务 10（实现）= 上述实现期清单；扩展位 SSOT = living doc 各站联动登记汇总表。
- 本次同批更新：ADR-0013/0017 Status 行结案注；ADR-0026/0027 Status 行记录批次条目号；[instance-model](../design/instance-model.md) 头部状态行转冻结指向；[luau-abi](../design/luau-abi.md) 与 [config-format](../design/config-format.md) 相应修订注补批次号。
