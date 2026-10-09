# 0018. 服务注册表与 inject 设计

- Status: accepted（「插件服务 v1 文档约定起步」（S6）的可选升级通道 = 插件随包契约，2026-10-09 定案于 [luau-abi](../design/luau-abi.md) §A8.2 D21.2；正式局部 supersede 随 Luau ABI 冻结 ADR 一并立）
- Date: 2026-10-02

## Context

服务是插件协作两条通道之一（请求-响应；另一条事件见 [ADR-0014](0014-dispatch-semantics.md)）。actor 模型（[ADR-0011](0011-actor-execution-model.md)）下服务调用 = host 两跳路由的纯消息：无共享对象、无持久引用，Cordis 的 Proxy 服务容器形态不可携带（[ADR-0013](0013-context-scope.md)）。inject 声明的位置决定激活门可行性、审批可见性与服务拓扑的静态可推导性。讨论全文与附议细节见 `../design/service-registry.md`。

被淘汰的选项：

- **运行时动态 inject**：激活门要求依赖在运行插件代码之前可知；先跑代码才知依赖 = 门失效，结构性不成立
- **单 callable 服务 / 方法级注册**：前者把方法分派压进 payload 约定（每个服务重造一次方法集）；后者 inject 声明与 epoch 追踪粒度爆炸
- **Cordis `check` 谓词及同族的主动 withdraw（讨论造词）**：v1 无用户；原子注册已消掉"注册了没 ready"窗口，第三态只增复杂度
- **热更窗口的调用等待机制**：服务对象窄（依赖者正在死、可选依赖缺席即信息）；重试策略归 `lib/` helper，机制保持最小
- **失败自动重试（指数退避）**：与响亮失败哲学冲突——崩溃循环刷噪音、放大故障；Failed 是终止态，等配置修复或手动重载

## Decision

- **服务形态（S1）**：key（扁平字符串；核心短名，`plugin-name/service` 约定不强制）+ 命名方法集；`call(key, method, args) → Result`，args/返回 = `Value`；方法集入注册表（存在性响亮 + schema 挂点 + 自省数据源）；服务无属性，跨 VM 只有方法调用；时刻唯一、跨时可换、同时多实现归 isolate
- **provide（S2）**：`ctx.provide` = fiber effect；**注册 = 激活转换点的原子动作**（manifest 对账 → 注册 → Active），注册即可用无中间态；重复 key 静态扫描 + 运行时防御双响亮；注销 = 卸载自动摘除
- **inject（S3）**：**manifest 双声明 SSOT**——inject 代码零声明（deps 句柄按声明装配注入，未声明结构性不可达）；provides 激活期对账（漂移无法发布）。必需依赖缺席：提供方在配置树中存在 → Pending 等待；无任何提供方 → 加载响亮失败。可选依赖：缺席不阻塞激活；出现/消失/热更**统一重启**消费者（复审点：ABI 能结构禁 setup 期探测时放宽）。**纯代码插件（无 manifest，[ADR-0004](0004-plugin-forms.md)）因此不参与 provide/inject**——只有事件通道，要用服务走 `foo.luau → foo/` 成长路径；本条局部取代 ADR-0004 能力边界表的"inject 边加载时运行期发现"一格
- **调用语义（S5）**：`Result` = `Ok(Value)` / `Err(Unavailable)` / `Err(MethodMissing)` / `Err(Timeout)` / `Err(Business, Value)`；`Business` 是契约义务必须显式处理，框架错误表拓扑/配置问题。`Unavailable` = 可选依赖常态 + 崩溃窗口竞态，稳态必需依赖不可达；热更窗不等待。并发：同提供方串行；deadline 即边界、等待环只超时；**超时不取消**（扩展位）；epoch 调用路径零校验（现查路由 + 邮箱关闭竞态消解）
- **epoch 与故障生命周期（S4）**：check 谓词否决；epoch 单调不复用（防 ABA）；故障检测结构性（JoinHandle 终结 + 邮箱关闭，无心跳）；恢复 = 完整重载，触发源 = 配置修复/手动（自动重试否决）；**恢复路径 = 启动路径复用**（注册唤醒 Pending，级联展开，状态不迁移）；重启语义 = 强制发起 / 优雅执行（disposer 可 await 在途持久化）/ 超时强制 / 顺序保护（计划卸载时提供方等依赖者 settle）
- **Seam 三角（S6）**：契约独立于提供方存在；核心服务契约必须 `lib/` 契约包，插件服务 v1 文档约定起步 + manifest 方法集机器可查；方法 schema 化与事件 payload schema 共享 DSL（ABI 任务）；演化规则 = 加方法兼容 / 删改破坏（epoch+1 + `MethodMissing` 响亮）；模型自省目录登记 ABI tooling

## Consequences

- 闭环'inject 是否静态声明'这一遗留开放问题；服务拓扑纯静态可推导（加载校验 / 审批 diff / 拓扑激活 / 自省目录全套受益）
- 启动、热更、崩溃恢复共享同一条"注册唤醒 Pending"路径——无专门恢复代码
- 依赖能力扩张永远过审批闸门——自我进化 agent 的能力透明性
- 状态纪律入册：行为在插件，状态在服务；持久状态走落盘 seam，原子写（tmp+rename）
- `lib/` 契约包形态依赖 `lib/` 引用语法与 manifest 格式，归 crate 划分 + ABI 任务细化；v1 无包管理，分发按 [ADR-0004](0004-plugin-forms.md) 登记长回
