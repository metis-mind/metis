# Metis — 接缝能力集设计（宿主能力 × 出区通道 × 能力门）

> 状态：living design doc——能力集语义本体已冻结 = [ADR-0025](../decisions/0025-seam-capabilities.md)；载体形态随 wasm-first 转向重写（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）。本文档保持 living：细则澄清继续沉淀于此；语义变更走 ADR-0025 演进纪律。
> 前提：插件唯一载体 = wasm 组件（[ADR-0034](../decisions/0034-wasm-first-carrier.md)）；fs 两档 = [ADR-0039](../decisions/0039-fs-capability-two-tier.md)（private 隐式 / global 声明；局部 supersede ADR-0036 命名根）；接口数据三分法 = W6/W7 章；ABI 形态 = [wit-abi](wit-abi.md)；作者面 = [plugin-sdk](plugin-sdk.md)。
> 相关 ADR：[0007](../decisions/0007-yaml-config-subset.md) / [0011](../decisions/0011-actor-execution-model.md) / [0013](../decisions/0013-context-scope.md) / [0018](../decisions/0018-service-registry.md) / [0023](../decisions/0023-core-restart-semantics.md) / [0024](../decisions/0024-carrier-layering.md) / [0025](../decisions/0025-seam-capabilities.md) / [0029](../decisions/0029-journal.md) / [0034](../decisions/0034-wasm-first-carrier.md) / [0036](../decisions/0036-fs-scope-three-layers.md) / [0039](../decisions/0039-fs-capability-two-tier.md)。
> Last reviewed: 2026-10

---

## 0. 设计目标与术语锚点

**设计目标（定调）**：宿主提供**广而抽象的领域中性能力、一律业务粒度接口**；插件内计算按编译后的原生速度执行，重活可以在插件内做，也可以放进服务组合或外部系统；core 不提供任何 agent 领域服务。

**能力来源 = 只进不授**：guest 的能力只来自宿主授予的 import。组件没有可裁剪的标准库，也没有「宿主替换标准库函数」这一层——不给的接口在 guest 里不存在。时钟、随机、文件系统等一切外部接触点都是显式 import，宿主天然可控。

**术语锚点**（本文档与 [wit-abi](wit-abi.md) / [plugin-sdk](plugin-sdk.md) 共用）：

| 术语             | 含义                                                                                                                                                                                                          |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 宿主（host）     | Metis 核心（Rust 运行时）。插件一切跨界调用经宿主路由（[ADR-0013](../decisions/0013-context-scope.md)）。                                                                                                     |
| guest / 插件组件 | 以 wasm 组件形态运行的插件实例（Store+Instance）。                                                                                                                                                            |
| 能力导入         | guest 经 WIT import 接触外部世界的接口；作者面由 SDK 映射为 ctx 方法或句柄（W3 章 = T13-j）。                                                                                                                 |
| 注入缝           | 宿主侧 import 实现接收注入的时钟 / 随机源；测试与生产走同一接口，确定性由构造参数保证（AGENTS.md Style）。                                                                                                    |
| 能力门           | 效应能力的「manifest 声明 → 启用期进口对账 → 运行期 fail-closed」（手机权限式三段机制）；对账 = 组件 import 清单 ⊆ SDK 封闭词汇表（[wasm-first-design-review](../research/wasm-first-design-review.md) B1）。 |
| 挂起点           | guest 任务的任意 await 处；宿主只见 import 边界的挂起与唤醒（[wit-abi](wit-abi.md) W4.1）。                                                                                                                   |

## 1. 确定性面

guest 的非确定性来源只剩宿主授予的少数 import；宿主不注入可替换的标准库层。

| 源             | 形态                                                         | 纪律                                                                                                                  |
| -------------- | ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| 时钟           | `wasi:clocks`（或宿主等价的注入实现）                        | 宿主侧实现接收注入时钟；测试用受控时钟，生产用系统时钟                                                                |
| 随机           | `wasi:random`（宿主实现；SDK 面 `crypto.random` 为能力别名） | 安全级随机一律 CSPRNG；编排级随机由 guest 自选 PRNG，种子经宿主注入                                                   |
| 宿主导入的实现 | 一切能力 import                                              | 实现不得无限阻塞（[wit-abi](wit-abi.md) W4.4）；guest 计算由 fuel/epoch 计量硬截断（ADR-0011 时间盒语义的 wasm 形态） |

确定性纪律与 replay 无关：核心不立 replay 引擎（[journal](journal.md) D75），时间 / 随机 / IO 的注入缝服务于测试确定性与宿主可控性，不承担录制职责。

## 2. 能力面的形态

### 2.1 两类能力

| 类别     | 形态                      | 归属                                                                                                   |
| -------- | ------------------------- | ------------------------------------------------------------------------------------------------------ |
| 效应能力 | host import（timer 无门） | fs / http / timer / process——必须宿主中介（政策、预算、journal 挂载位）；DB = guest crate + private fs |
| 纯算能力 | guest 语言与包生态        | json / yaml / regex / diff / uuid / hex / base64 / timefmt 等由 guest 自行选依赖；核心不提供纯算内建件 |

纯算件不再需要「sync 直挂」：插件内计算就是原生代码，不存在旧载体时代的跨界税。SDK 可以提供便利 re-export，那是包形态题，不进核心机制（[plugin-sdk](plugin-sdk.md) S3）。

### 2.2 调用语义

- 一次能力调用 = 一次跨界 import；能挂起的 import 在 `await` 处让出当前任务，等待期间其他任务继续执行（[wit-abi](wit-abi.md) W4.1）。
- 不立「sync 直挂 / async 两跳」二分：一切效应能力都是 host import，形态统一；是否需要等待由接口本身的语义决定。
- 例外与兜底：import 实现不许无限阻塞；guest 内计算由 fuel/epoch 截断；流式读取的挂起 = 唯一真无界源，处置归 W6 章。

### 2.3 接口形状三约束

1. **流式优先**：凡输入 / 输出可能大的接口（fs、http body）必有流式或分页形态——整段进实例 = 一份数据独占一个并发槽。
2. **批量友好**：高频小项允许批量化；不设计「返回十万行」式的整表接口。
3. **不假设值传递廉价**：大 Value 的传递成本 = canonical ABI lift/lower + TLV 线编码；大负载走 resource 句柄族（W6 章）。

### 2.4 原子方法族

两插件写同一文件靠协议不靠互斥（宿主零同步原语 = [wit-abi](wit-abi.md) W4.2）。能力面提供少量原子方法，首件 = fs `append`；其余成员按真实消费者增补（扩展位）。

## 3. 性能议程

### 3.1 主张

- **性能 = 编排并发度**：agent 负载是 IO 等待型（单步 10² ms–s 级），优化目标是等待不占地、更多编排同时推进，不是单调用延迟。
- **瓶颈 = 内存**：并发度上限 = 存活组件实例数 × 每实例在飞任务预算；内存账 = 实例 VA/RSS + 在飞任务栈与捕获态（[wit-abi](wit-abi.md) W4.3）。每实例内存限额 = 并发槽单价，实测基线归实现期。
- **插件内 CPU 密集合法化**：不再要求「重活逐字节搬出插件」；算的事并行 = 同码多实例并跑或外部服务，等的事重叠 = guest async 原生。

### 3.2 反目标与登记

- 不做 ns 级投机优化（内联缓存、激进零拷贝先行）；不预调 blocking pool / `block_in_place`（实测瓶颈出现再捡）。
- 内存限额数值 = 实现期配置；ABI 只保证接口形状不假设无限内存。
- 压测基准（并发吞吐 + 单槽内存账）注入测试策略议题；组件单实例基线内存实测 = 实现期基准项。

## 4. 能力清单

### 4.1 效应能力（host import；timer 无能力门）

| 件      | 形态                        | 说明                                                                                                                     |
| ------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| fs      | 句柄 + 流式                 | 两档 scope 见 §5.2；原子写 = 临时文件 + rename（[ADR-0023](../decisions/0023-core-restart-semantics.md) 落盘要求的落点） |
| http    | 请求 / 响应 + 流式          | 强制超时；透明解压；域名白名单；SSRF 硬化（§5.2）                                                                        |
| timer   | `after` / `every` / `sleep` | 注册为 fiber 的一笔 effect，卸载随 LIFO drain 自动取消；**无能力门**，`every` 设最小间隔政策（数值实现期）               |
| process | 句柄 + 流式                 | 能力声明 `process: true` 无参数；强制超时 / 进程组杀 / 无孤儿；支配关系见 §5.2                                           |

DB 不是宿主能力：插件自带 guest crate（SQLite / redb / sled 等），库文件住 private fs 内，schema / 迁移自管（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）。

### 4.2 标准件与内建件

| 件                 | 形态        | 说明                                                                                                                                                                                        |
| ------------------ | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| clocks / random    | WASI 标准件 | 直接复用；宿主控制实现（§1）                                                                                                                                                                |
| filesystem preopen | WASI 标准件 | private 由 preopen 承载；global 走 host-mediated 自研逃逸舱（§5.2）                                                                                                                         |
| crypto             | host import | v1 = sha256 / sha512 + hmac + `crypto.random`（CSPRNG）+ AES-GCM；hash 配流式形态；md5 / sha1 / 签名族 / xxhash 登记扩展位。留在核心的论据 = AES/SHA 指令缺口的硬件约束（ADR-0024 §4 站位） |
| git                | 扩展位维持  | 领域相关 + 重依赖 + 低频；真实场景走 guest 包或外部服务                                                                                                                                     |

## 5. 出区通道

### 5.1 闭门论证

效应出沙盒的路径**可穷尽**：**磁盘（fs）、网络（http）、时间推进（timer）、子进程（process）**——不存在第五扇门。判据：门 = 插件可达**外部世界或持久效应**的通道。`print` / 日志类输出经宿主观测面消化，不算门。

边界澄清：LLM 调用**不是** syscall 通道（产品模型红线；journal `llm_*` 条目本就不属核心 kind）；用户 / 外部输入是**入区**（host 网关，session 议题），不在插件调用面。

### 5.2 通道逐项

- **fs：两档 scope（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）。**
  - **private**：`plugins/<name>/data/fs/` 以 WASI preopen 授予，**隐式读写**，不声明、不可收窄；guest 在 preopen 内用常规文件 API；逃逸防护 = 运行时结构保证，自研两段式解析对 private 退役。
  - **global**：减法语义（用户世界 − `secrets.yml` + `journal/` 两条豁免），preopen 表达不了减法 → host-mediated 自研逃逸舱；manifest 声明 `fs: read|write`（语义 = global；形状 = [wit-abi](wit-abi.md) W2.5 D129），缺省 = 无 global；TOCTOU 硬化与路径解析留在宿主实现。
  - **命名根不立**：原「manifest 声明 + 部署者绑定」中间档撤回（ADR-0039）——插件可能服务多个 project，静态绑定一次配不齐；需要用户世界访问走 global。
  - 写 = 临时文件 + rename 原子写。workspace / session 语义归插件层服务投影，核心不做一一映射。
- **http**：`ctx.http.request` 形态（作者面由 SDK 映射）。透明解压；**强制超时**（不允许无 deadline 请求）；响应默认进内存带预算，大 body / SSE 走流式；域名白名单、跨域重定向须重新对账（open-redirect 旁路）、SSRF = 解析结果非全局单播即拒（含 v6）+ 白名单显式授予内网 + 最终解析结果判定钉住；明文 http 须 `http://` 显式标记。
- **timer**：`ctx.timer.after` / `every` / `sleep`。注册为 fiber 的一笔 effect、卸载随 LIFO drain 自动取消（无孤儿）；**无能力门，但 `every` 设最小间隔政策**（机制在此，数值实现期配置）；cron 形态 = 维持不立（[config-format](config-format.md) §C8.2 D51）；sleep = 挂起当前任务、宿主直接唤醒。
- **process**：`ctx.process.*`。能力声明 `process: true` 无参数；强制超时 / 进程组杀 / 无孤儿（账本 drain）/ journal 全量元数据；命令级审批归插件层策略机器。
  - **支配关系**：process 继承核心进程 OS 全权——fs scope 分段 / 两条豁免 / http 白名单**均不经由它强制**（子进程里读 secrets store、访问任意网络物理可行）；审批面按 ⊇ `fs: write`（global 写档）+ 任意域 http 的权重展示；外墙 = OS 级沙箱，归部署组成。
  - **编程式 spawn 插件派生仍推迟**（[ADR-0017](../decisions/0017-programmatic-spawn-deferred.md)，与本条无涉）；MCP stdio 监管机器 host 自持不变。

### 5.3 能力声明与 journal 映射

| 通道    | 能力声明                   | journal kind                                 |
| ------- | -------------------------- | -------------------------------------------- |
| fs      | `fs: read/write`（global） | v1 不立逐调用族（可配审计层 = 扩展位）       |
| http    | `http`（域名细化）         | v1 不立逐调用族（可配审计层 = 扩展位）       |
| timer   | 无门                       | v1 不立逐调用族（可配审计层 = 扩展位）       |
| process | `process`（无参数）        | `process_spawn` / `process_exit`（写死全录） |

插件自带 DB 的读写走 private fs，无宿主 DB 句柄与 `sql_call` / `sql_result` 族（[ADR-0039](../decisions/0039-fs-capability-two-tier.md)）。

journal kind 表 SSOT = [journal](journal.md) D78；v1 = 写死族骨架（D84），能力使用族仅 `process_*` 两条存活。

### 5.4 时间调度三梯度

| 梯度                                | 形态                                                      | 归属                                                                                                                 |
| ----------------------------------- | --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| 易失定时（秒–分钟，热更/卸载即消）  | `ctx.timer.after/every`                                   | 核心原语（§5.2）                                                                                                     |
| 持久调度（跨热更/重启的提醒）       | 插件层 schedule 服务 = 插件自带 DB + timer 原语 + journal | 插件层议题（DSH 先例：日志持真相、定时器为易失投影、every ≥ 5 min、错过间隔不补发、墙钟回拨不早发 / 前跳算 overdue） |
| 声明式周期任务（cron 式，更可审批） | v1 = config 键约定                                        | 一等声明式调度 = 维持不立（[config-format](config-format.md) §C8.2 D51）                                             |

主被动模型注：timer 不破「插件被动调用」模型——tick 是邮箱消息的一种（[ADR-0011](../decisions/0011-actor-execution-model.md)），`every` = 订阅宿主时钟事件源；主被动分界 = **谁拥有循环**，线程式 `while + sleep` 结构性封死。

## 6. 联动与登记

### 6.1 WIT ABI 章回填

| 章                  | 回填                                                                                                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| W2（T13-i）manifest | **已兑现（2026-10-19）= wit-abi W2 D129**：`capabilities` 字段面 = `{ fs, http, process, mount }`；`fs: read/write` 标量 = global（private 隐式）；sqlite 撤回；timer 无门；纯算件无需声明 |
| W3（T13-j）ctx      | ctx 效应方法面 = fs / http / timer / process；`fs` = private（隐式）/ global（`fs` 标量声明）；DB = guest crate + private fs；编排面 = 事件 / 服务调用 / `mount` / `unmount`               |
| W5（T13-k）错误     | 能力失败的 `result` 形态；unavailable / method_missing / timeout / business 四格                                                                                                           |
| W6（T13-l）边界     | 递归 Value 的 TLV 线编码；流式句柄族；resource 句柄生命周期                                                                                                                                |
| W9（T13-o）能力门   | 进口对账机械化（组件 import 清单 ⊆ SDK 封闭词汇表）+ 运行期 fail-closed；编排面不立能力门的既有立场，唯一例外 = `mount: true`                                                              |
| W10（T13-p）版本    | 能力词汇 additive 增补 → WIT package version 演进（原 `abi_rev` 纪律的承接）                                                                                                               |

### 6.2 实现期登记

- timer 最小间隔 / http 预算与超时 = 实现期配置。
- 组件单实例基线内存与并发槽单价 = 实现期基准项。
- crypto 的流式 hash 形态与扩展位成员清单 = 实现期细化。
