# Metis — Context 面与事件系统设计

> 状态：议题 0（执行模型）✅ = [ADR-0011](../decisions/0011-actor-execution-model.md)；协作通道框架 ✅ 讨论定论；
> D2 ✅ = [ADR-0013](../decisions/0013-context-scope.md)；D3–D8 ✅ 全部确认，固化为 [ADR-0014](../decisions/0014-dispatch-semantics.md)~[0017](../decisions/0017-programmatic-spawn-deferred.md)；D5 由 [ADR-0022](../decisions/0022-value-int64.md) 修订。插件内执行模型（协作式自由交错）= [wit-abi](wit-abi.md) W4 章。
> 调研依据：`../research/cordis-research.md`；相关 ADR：[0001](../decisions/0001-inject-runtime-checks.md) / [0005](../decisions/0005-vm-topology.md) / [0010](../decisions/0010-fiber-core.md) / [0034](../decisions/0034-wasm-first-carrier.md)。
> Last reviewed: 2026-10

---

## 0. 执行模型：actor 语义（已定 → [ADR-0011](../decisions/0011-actor-execution-model.md)）

**一句话**：每插件实例 = 独立组件实例（Store+Instance）+ 邮箱 + worker 循环；跨插件一切交互经 host 两跳路由。

### 0.1 全局唯一 vs 每插件一份

依赖树只有一份，在 host；actor 只是执行容器，不持有任何框架状态。

| 全局唯一（host 持有）                                                                      | 每插件实例一份（actor 持有）                                                      |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| fiber arena（账本 + 状态机 + 父子关系）                                                    | 一个组件实例（Store+Instance，[ADR-0005](../decisions/0005-vm-topology.md) 拓扑） |
| 事件表 `name → Vec<Hook>`                                                                  | 一个邮箱（message channel）                                                       |
| 服务注册表 + epoch + 反向索引                                                              | 一个 worker（tokio 任务）                                                         |
| entry 树 / 配置树 / manifest 依赖边                                                        | ——                                                                                |
| 契约命名空间 + 事件形状总表（启用期对账数据源；WIT 形态 = [wit-abi](wit-abi.md) W7/W8 章） | ——                                                                                |

### 0.2 星型拓扑

actor 只与 host 通信，host 是路由器。事件派发、服务调用、dispose、定时器 tick 全部是 host→actor 的邮箱消息；插件之间从不直接通信。事件表、活性检查、快照、epoch 判断全部留在 host 一处——四种派发模式因此是「同一个循环的四个参数」。

### 0.3 worker 循环（语义示意）

```
loop {
    match mailbox.recv() {
        Setup(config)                       => 闸门接纳 → 投递 setup export，结果回报 loader
        HandleEvent { payload, reply }      => 闸门接纳 → 投递事件 export；reply 随任务完结回路由
        CallService { method, args, reply } => 闸门接纳 → 投递服务 export；完成路由回调用方
        Dispose                             => { 关闸 → 优雅窗口 → 实例即弃 → drain; ack(); break }
    }
}
```

投递即起跑：handler 挂起不占 worker，邮箱下一条在在飞预算内立即可接纳投递，多条任务同实例在飞交错。协作式铁律：同一瞬间至多一条任务在 CPU 上，切换仅在挂起点，两挂起点之间原子；同步 = 作者显式选择（SDK async mutex），宿主零同步原语。插件内执行模型（重入 / 背压 / 保险丝 / 卸载时序）的 SSOT = [wit-abi](wit-abi.md) W4 章。

### 0.4 爆炸半径三层保险

1. **单步执行硬截断**：fuel/epoch 计量（wasmtime 原生件）——防堵 tokio executor 与处置跑飞插件二合一（ADR-0011 时间盒语义的 wasm 形态；纪律平移：host import 实现不许无限阻塞）
2. **超时**：parallel/serial/服务调用带 deadline；跨 actor 等待环（A 等 B、B 等 A）检测到闭合即 fail-fast（当场报错 + 整圈路径 + fail-open 解圈，[wit-abi](wit-abi.md) W4.4），超时是最后保险丝
3. **bounded 邮箱**：堵死的插件让投递方背压/超时，不无限涨内存；背压面 = 每实例在飞任务预算（[wit-abi](wit-abi.md) W4.3）

### 0.5 性能结论（估算，非实测）

- 单跳成本主项 = 边界转换（canonical ABI lift/lower + 线编码；成本分层与粒度规则 = [wasm-research](../research/wasm-research.md) §3.4/补遗 5 §5.4），与线程实现无关
- 量级账：派发总开销 = 监听器数 × 单跳成本。agent 单步操作（模型 TTFT、工具执行、IO）以 10² ms–s 计，派发低约 3 个数量级 → 不成瓶颈（不以「LLM 调用慢」为论据——flash 级模型 TTFT 已进入 10² ms 量级，论据 = 监听器规模 × 单跳成本的绝对账；单跳数值 = 实现期实测面）——届时「一个事件为什么有千个监听器」是设计问题而非性能问题
- **控制面纪律**：事件总线传控制信号、不传数据流——高频流式数据（如逐 token 输出）由核心自持或批量化，不逐 token 过插件总线
- 优化位登记：`block_in_place`、blocking-pool checkout（实测瓶颈出现再捡，语义不变）

## 1. 插件协作的两条通道（讨论定论）

「插件依赖另一个插件的结果」= 一个插件调用另一个插件，但**永远不是直接调用**——独立组件实例 + 无运行期模块系统（跨插件代码共享 = 编译期依赖）使直接引用结构上不可能（[ADR-0034](../decisions/0034-wasm-first-carrier.md) §1/§4）。只剩两条 host 中介通道：

| 需求                         | 通道            | 形态                                                         |
| ---------------------------- | --------------- | ------------------------------------------------------------ |
| 拿到 X 的结果/数据           | **service**     | 请求-响应：A→host→B→host→A；epoch 指纹保证 B 热更后 A 先重启 |
| 宣告 X 发生了                | `emit`          | fire-and-forget                                              |
| 一个处理者认领 X             | `serial`        | 首个终止值胜出（命令分发形态）                               |
| 所有人各出一份力             | `parallel`      | allSettled 收齐 `Vec<Result>`                                |
| X 被层层加工/可否决          | `waterfall`     | 顺序变换链                                                   |
| 「B 活着且可用」这个事实本身 | **inject 声明** | fiber Pending 等依赖齐；加载顺序由服务可用性驱动             |

外加半个隐式通道：**声明式组合**（entry 树挂载、preset）——回答「谁存在」，不是「谁调用谁」。

服务调用路径（7 步）：A 调代理 → 打包 `CallService` 发 host → host 查注册表（key→B、B Active；epoch 不在调用路径校验，陈旧由重启纪律解决——[ADR-0018](../decisions/0018-service-registry.md) S5）→ 转发 B 邮箱 → 投递 B 实例执行（错误遏制面 = [wit-abi](wit-abi.md) W5 章）→ 结果路由回 A → A 的调用返回（guest 侧 await 形态 = [wit-abi](wit-abi.md) W3/W4 章）。

## 2. 派发语义：五种收敛为四种（D3/D4/D8 ✅ = [ADR-0014](../decisions/0014-dispatch-semantics.md)）

actor 模型下跨组件实例必然消息往返，Cordis「同步/异步」区分整体消失；正交三轴：等不等结果 / 并发还是顺序 / 返回值语义。

| 模式        | 语义                                                                  | 对应 Cordis        |
| ----------- | --------------------------------------------------------------------- | ------------------ |
| `emit`      | fire-and-forget；监听器错误只记日志                                   | emit               |
| `parallel`  | 等全部（allSettled），返回每项 Result                                 | parallel           |
| `serial`    | 按注册顺序逐个调用；监听器返回**终止值**即短路并把值带回              | serial + bail 合并 |
| `waterfall` | 顺序变换链：返回 `Some(v)` 继续 / `None` 否决（后续含内建行为被跳过） | waterfall          |

从属决策：

- **bail 并入 serial**：Cordis 里 bail 只是「同步 serial」，同步性消失后语义完全重合
- **waterfall 用变换链+否决替代真洋葱**（D4）：Cordis 全部实际用例（config 解析、update veto）都是单向变换，洋葱 post 阶段无人用；actor 模型跨实例传 continuation 代价大；将来需要可加 post 阶段，不破语义
- **serial 遇错不短路**（D8）：错误按遏制哲学记录+继续，显式返回值是唯一短路条件

### 2.1 派生模式：选择器插件（selector pattern）

serial 的「注册顺序 + 首认领」只是 host 内置的默认路由策略；**可编程路由可由普通插件实现，无需新机制**：

- 选择器插件作为目标事件的（唯一/首位）serial 监听器认领输入，路由策略是其内部自由（静态表 / 打分 / LLM 路由）
- 候选发现三选一或组合：parallel 派发能力询问事件（**事件表即注册表**，无需新增 list API）/ 自身 config 静态路由表（声明式、可审批、可热更）/ 注册事件（有启动顺序竞态，非必要不选）
- 选中后经服务调用转交被选插件；选择器返回 nil 或崩溃 → 链自然落到内建兜底（D8 fail-open 在此处恰好正确）
- 边界：选择器不能改 host 派发语义，力量来自「站在入口」而非「修改路由器」；独占入口靠组合部署（preset/entry 树）保证
- 含义：**派发策略本身成为可替换的用户空间组件**（everything-is-a-plugin 的又一兑现）；host 永远只提供四种硬编码默认策略

## 3. payload 值类型（D5 ✅ = [ADR-0015](../decisions/0015-payload-value-model.md)，闭环 payload 类型遗留问题；`Int(i64)` 提正 = [ADR-0022](../decisions/0022-value-int64.md)）

进程内派发**不需要字节编解码**：payload 以 Rust 值在 host 与各组件实例间传递，只在实例边界做 canonical ABI lift/lower 与递归 Value 的 TLV 编解码（[wit-abi](wit-abi.md) W6 章）。定的是数据模型，不是线路格式。

| 方案                                              | 评                                                                                                                                                                                                                 |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **A. 自定义最小 `Value`（JSON 数据模型）** ✅采纳 | `Null/Bool/Float(f64)/String/Array/Map`；与 YAML 1.2 core schema（[ADR-0007](../decisions/0007-yaml-config-subset.md)）及 WIT canonical 类型双落点对齐；`Int(i64)`（[ADR-0022](../decisions/0022-value-int64.md)） |
| B. `serde_json::Value`                            | 零维护，但 number 语义绕（i64/u64/f64 三分对数据模型无独立意义），config 侧还得再定一个类型                                                                                                                        |
| C. 扩展版（+`Int(i64)`/`Bytes`）                  | 真实需求但 v1 无用户，登记扩展位；`Int(i64)` 由 [ADR-0022](../decisions/0022-value-int64.md) 提正（WIT `s64` 原生），`Bytes` 仍留扩展位                                                                            |

采纳 A，且 **config 值与事件 payload 共用同一 `Value`**——schema 校验、审批 diff、事件日志全套基础设施只吃一种数据模型（「数据大一统」）。

> `Value` 增加 `Int(i64)`（[ADR-0022](../decisions/0022-value-int64.md)）——WIT `s64` 原生映射；2⁵³ 精度天花板由原生 64 位整数承接。

## 4. `internal/*` 钩子处置（D6 ✅ = [ADR-0016](../decisions/0016-internal-hooks.md)）

机制保留（框架自身操作可挂中间件 = everything-is-a-plugin 的关键），逐条处置：

| Cordis 钩子                                           | v1 处置                        | 理由                                                                                                                       |
| ----------------------------------------------------- | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| `internal/config` / `internal/update`                 | 💤 机制留、钩子不开            | 唯一内建用户是配置内嵌求值（[ADR-0002](../decisions/0002-config-as-pure-data.md) 已裁为纯数据）；复活时同步启用            |
| `internal/get` / `internal/set` / `internal/listener` | ❌ 永久删除                    | Proxy 时代的服务读写拦截管道；显式 API 没有可拦截的「属性读写」                                                            |
| `internal/service`                                    | ❌ 删除                        | inject 重激活由 epoch + 反向索引在 Rust 层驱动（[ADR-0001](../decisions/0001-inject-runtime-checks.md)），不需插件可见钩子 |
| `internal/plugin` / `internal/status`                 | ✅ **v1 开**                   | fiber 生命周期事件（emit），工具插件/状态观测第一需求，成本极低                                                            |
| `internal/dispatch`                                   | 替换为 tracing instrumentation | 不占钩子位；顺带落实「Model-visible ⟺ logged」                                                                             |

命名纪律：`internal/` 前缀保留给框架；插件事件自由命名（文档建议 `plugin-name/event` 约定，不强制）。

## 5. Context 的 Rust 形态（D2 ✅ = [ADR-0013](../decisions/0013-context-scope.md)）

- **Context 不是大对象，是 fiber 视角的 runtime 句柄**：`Context { fiber: FiberId, runtime: RuntimeHandle }`，所有方法（`on/emit/provide/get/...`）都是对 runtime 的调用，权限与归属由 FiberId 天然携带
- **作用域树 = fiber 树本身**（`parent` 已在 Fiber 数据结构上），不另造 context 树
- Cordis `extend/isolate/intercept` 处置：`extend` 的意义被 per-fiber Context 天然覆盖；`isolate/intercept` 是声明式 entry 特性，与 inject 静态声明问题同族 → **归配置格式与实例模型**；v1 服务解析 = 全局注册表单级查找，Context 预留 parent 链（isolate/intercept 不立——isolate 语义由实例作用域 + parent 链遮蔽天然给出，intercept 职能由声明 `config:` overlay 覆盖；parent 链已消费 = 实例服务解析沿 parent 链先查自己作用域再向上——[instance-model](instance-model.md) §S4.2/§S4.3 D68/D69）

## 6. 编程式 spawn（D7 ✅ = [ADR-0017](../decisions/0017-programmatic-spawn-deferred.md)）

v1 只走声明式 entry 树，`ctx:spawn` 推迟。理由：警惕第二条生命周期路径（Harness eval 退役教训的一般化）；插件要动态性，用自己的 fiber effect 挂内部任务（JoinHandle 入账）即可覆盖；机制已备（子 fiber = 父的 effect），真需要时开箱即用。

**汇合点**：preset 的 session 子树挂载 = spawn + 子树作用域的交集 → 归实例模型（[instance-model](instance-model.md)）。（结案：**D7 半** = mount 通道唯一 + spawn 维持关闭——[instance-model](instance-model.md) §S3.6 D66；**D2 半**（isolate/intercept 复审）= D69。）

## 7. 监听器生命周期与派发安全细则（随 D3 确认，固化于 [ADR-0014](../decisions/0014-dispatch-semantics.md)）

1. **listener = fiber effect**：`ctx:on()` 返回 disposer 自动入账，卸载摘除
2. **派发快照 + 逐项活性检查**：快照监听器列表，逐项投递前查 arena `state == Active`；快照后卸载 → 邮箱已关 → 投递失败 = 正常跳过（竞态无锁化解）
3. **顺序保证**：serial/waterfall 按注册顺序；同一监听者收事件顺序 = 派发入邮箱顺序（单派发者维度保序，跨派发者不保）
4. **错误遏制分模式**：emit 记日志；parallel 收集每项 Result；serial/waterfall 记日志后继续

**W3 兑现注（2026-10-19）**：监听登记住宿主（`events.listen` / `unlisten`），`Ctx::on(name)` 只登记兴趣并返回 `Listener`（同名去重 + 引用计数：一个实例一个宿主条目；Drop / `off()` 减引用，归零 unlisten；`detach()` 交宿主账本），处理走 `Plugin::on_event` catch-all；`on-event(listener-id, name, payload)` 导出形状归 W8；`emit` async 但只 await 入队（不等监听者完成）；派发满时背压，不静默丢弃（[wit-abi](wit-abi.md) W3 D146 / D152）。

## 8. 决策点状态

| #  | 决策点                                                                        | 状态                                                                                                                                                  |
| -- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1 | 执行模型 actor 语义 + tokio                                                   | ✅ [ADR-0011](../decisions/0011-actor-execution-model.md)                                                                                             |
| D2 | Context 形态 = fiber 句柄；作用域特性 v1 全裁，isolate/intercept 推迟到任务 5 | ✅ [ADR-0013](../decisions/0013-context-scope.md)                                                                                                     |
| D3 | 五种收敛为四语义（bail 并入 serial）                                          | ✅ [ADR-0014](../decisions/0014-dispatch-semantics.md)                                                                                                |
| D4 | waterfall = 变换链+否决，非真洋葱                                             | ✅ [ADR-0014](../decisions/0014-dispatch-semantics.md)                                                                                                |
| D5 | payload = 自定义 JSON 数据模型 `Value`，config 共用                           | ✅ [ADR-0015](../decisions/0015-payload-value-model.md)（闭环 payload 类型遗留问题）+ [ADR-0022](../decisions/0022-value-int64.md)（`Int(i64)` 提正） |
| D6 | internal 钩子 v1 开 plugin/status，机制保留                                   | ✅ [ADR-0016](../decisions/0016-internal-hooks.md)                                                                                                    |
| D7 | 编程式 spawn v1 推迟                                                          | ✅ [ADR-0017](../decisions/0017-programmatic-spawn-deferred.md)                                                                                       |
| D8 | serial 遇错不短路，值短路唯一                                                 | ✅ [ADR-0014](../decisions/0014-dispatch-semantics.md)                                                                                                |
