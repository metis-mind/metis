# Metis — 接缝能力集设计（syscall 原语 × 胶水边界 × 出区通道）

> 状态：living design doc——六块全部讨论定论（2026-10-05）；冻结 ADR 随 Luau ABI 固化一并立（ABI 议题地图任务 3）。
> 来源：ABI 议题地图 B1+B2 合并议题（"接缝能力集"）。前提：产品模型（core = 插件管理与组合机器 + syscall 层 VM 基底，不提供 agent 领域服务）；载体分层 = [ADR-0024](../decisions/0024-carrier-layering.md)（盒管模型）。实测依据：`../research/wasm-research.md` 补遗 1（mlua/wasmtime 双 spike）。
> 相关 ADR：[0003](../decisions/0003-module-require-discipline.md) / [0007](../decisions/0007-yaml-config-subset.md) / [0008](../decisions/0008-creator-mode-self-modification.md) / [0011](../decisions/0011-actor-execution-model.md) / [0013](../decisions/0013-context-scope.md) / [0015](../decisions/0015-payload-value-model.md) / [0017](../decisions/0017-programmatic-spawn-deferred.md) / [0021](../decisions/0021-extension-language-runtime.md) / [0022](../decisions/0022-value-int64.md) / [0023](../decisions/0023-core-restart-semantics.md) / [0024](../decisions/0024-carrier-layering.md)

---

## 0. 设计目标与术语锚点

**设计目标（定调）**：syscall 层提供**广而抽象的领域中性能力、一律业务粒度接口**，使 Luau 保持薄编排（几十行：声明 + 组合 + 少量状态机）；密集逻辑必有去处（纯算内建件 / SDK 库层 / 服务组合 / wasm 盒），core 不提供任何 agent 领域服务。

**术语锚点**（本文档与 SDK 文档共用）：

| 术语                   | 含义                                                                                                                                                                                |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 宿主（host）           | Metis 核心（Rust 运行时）。Luau 是嵌入式语言：VM 由宿主创建、注入、驱动、回收；插件一切跨界调用经宿主路由（[ADR-0013](../decisions/0013-context-scope.md)）                         |
| stdlib                 | Luau 自带标准库（`string/table/math/os/coroutine/bit32/utf8/buffer/vector/integer/debug` + 全局函数）；无 `io`/`package`，`os` 天生仅 4 函数                                        |
| 原位替换               | 保签名、保语义、换实现：宿主把 stdlib 函数替换为自己的实现，插件写法零变化                                                                                                          |
| 注入缝                 | 非确定性进入 VM 的唯一通道 = 宿主注入的实现（时间/随机等），可被 journal 拦截                                                                                                       |
| 能力门                 | 效应能力的"manifest 声明 → 安装对账/审批 → 运行时强制"三段机制（手机权限式）                                                                                                        |
| sync 直挂 / async 两跳 | 见 §2；前者 = 跨边界不跨消息，后者 = 挂起 + 宿主侧路由往返（完成 = 宿主直接 resume 挂起协程，**非回插件自身邮箱排队**——setup 期挂起安全的精确含义见 [luau-abi](luau-abi.md) §A1.4） |
| tombstone              | 被移除 stdlib 件的占位函数：调用即报**可行动**错误（理由 + 替代件），是四层防线（§1.5）的兜底层                                                                                     |

## 1. Luau stdlib 卫生

目标：每个插件 VM = 确定性沙盒 + 宿主受控缝——非确定性只剩宿主注入的几个口，且全部可被 journal 拦截。

### 1.1 非确定性源处置

Luau 0.740 实测全局面里**效应级**非确定性源仅两处（无 `io`/`package`、`os.getenv` 上游即无）；表示级残留（指针身份串）登记 §1.6：

| 源                                | 处置                                                                                            | replay                                                                               |
| --------------------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `os.time` / `os.clock`            | 宿主注入受控时钟（原位替换）                                                                    | **record**：每次读取记 `nondeterminism_read`，replay 按史注入（墙钟/单调钟无法重算） |
| `math.random` / `math.randomseed` | `random` 原位替换为宿主确定性 PRNG（per-fiber 种子注入，host 闭包实现）；`randomseed` tombstone | **recompute**：fiber 启动记种子一条，replay 重算推进                                 |

**随机性来源原则**：编排级随机（统计性质即可，如 uuid、`math.random`）一律 PRNG/recompute；安全级随机（`crypto.random`）一律 CSPRNG/record。

replay 纪律前提：版本 pinning（replay 默认用录制期代码）已排除 recompute 的脆性主场景（`../research/journal-replay-research.md` 第七部分 M4）。

### 1.2 os 面四函数

| 函数             | 处置                                                                                                                                                                                                                                               |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `time` / `clock` | 宿主注入（§1.1）                                                                                                                                                                                                                                   |
| `date`           | **原位替换为薄壳**：无参走注入时钟、有参纯格式化；格式化引擎 = timefmt 内建件（§4）——兼容层（strftime 方言）/ 推荐路径（timefmt）分工，SDK 文档写明"壳只为兼容，新码用 timefmt"（零装机生态的兼容承诺；若 timefmt 普及后壳成赘肉，ADR 复审时可摘） |
| `difftime`       | 保留（纯函数）                                                                                                                                                                                                                                     |

### 1.3 裁剪面定案

| 项                                                                                                   | 处置                                          | 理由（一句）                                                                                                                                |
| ---------------------------------------------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `loadstring`                                                                                         | tombstone（红线）                             | 动态 eval 绕过审批链与 plugin_revision 钉版；合法需求 = 随包 .luau 文件                                                                     |
| `require`                                                                                            | 保留 + 宿主包装                               | 模块缝（[ADR-0003](../decisions/0003-module-require-discipline.md)），per-plugin 解析，细节归 A1/A3                                         |
| `print`                                                                                              | 原位替换：输出改道宿主日志，自动带 fiber 身份 | 长驻 server 的 print 须可归因                                                                                                               |
| `collectgarbage` / `gcinfo`                                                                          | tombstone                                     | GC 是宿主职责                                                                                                                               |
| `newproxy` / `getfenv` / `setfenv`                                                                   | tombstone                                     | legacy 环境操弄，与 sandbox 模型冲突                                                                                                        |
| `debug.info`                                                                                         | tombstone                                     | 内省邀脆弱元编程；`debug.traceback` 保留（错误诊断质量）。（`debug` 实测仅剩此两件；上游若加件由 §1.4 白名单兜住）                          |
| `coroutine`                                                                                          | 保留 + shim 包装                              | 库本身纯算；经 shim 可透明调 ctx（挂起冒泡到恢复者线程）——迭代器/生成器是流式消费主流形态（[luau-abi](luau-abi.md) §A4.3 D9.2，2026-10-09） |
| 其余全部（`string/table/buffer/bit32/utf8/vector/integer`/`math` 余部/语言核心函数/`_G`/`_VERSION`） | 保留                                          | 纯算件：无效应、无非确定性、VM 内自含                                                                                                       |

### 1.4 建 VM 姿势与硬化顺序

- **白名单建 VM**：`Lua::new_with(StdLib 白名单)`——fail-closed 对 vendored Luau 升级（上游新增全局件默认不存在）。注意 mlua 无条件安装 `require/collectgarbage/loadstring/_VERSION`——前三件仍须手工处置（包装/tombstone），`_VERSION` 保留。
- **硬化顺序铁律**：白名单建实例 → 宿主裁剪 + 注入（os/math/print/ctx/require 包装 + coroutine shim）→ `sandbox(true)` 冻结 → 才跑插件代码。
- sandbox 语义：stdlib 表与 `_G` 只读；插件在自己环境按名遮蔽只写进影子表（自欺不欺人，无害）。

### 1.5 四层防线（横切 ABI/SDK 纪律）

被裁剪/替换件的教学不靠运行期报错，报错是兜底：

| 层     | 机制                                                                                                    | 时机        |
| ------ | ------------------------------------------------------------------------------------------------------- | ----------- |
| 写码期 | SDK 发布环境类型定义：被移除件在定义里不存在，analyze 直接标红                                          | 写代码时    |
| 安装期 | `metis sdk` / 安装管线静态检查（[ADR-0024](../decisions/0024-carrier-layering.md) §6 结构校验大门位置） | 安装/上架时 |
| 生成期 | Creator 模式上下文携带沙盒环境清单（可用面 + 替代件）                                                   | LLM 生成时  |
| 运行期 | tombstone 可行动报错（理由 + 替代件）                                                                   | 兜底        |

联动：A10（`metis sdk` analyze 管线）。

### 1.6 实现期验证项（登记）

- 数字格式化 locale 无关性断言（`tostring`/`string.format` 跨机器一致，事关 journal payload 一致性）
- 指针身份串（`tostring` 默认地址表示、`string.format('%p')`）随进程堆布局逐次运行变化、且不经 `nondeterminism_read`——**不在确定性保证内**；replay 比对的归一化政策归任务 6（`../research/journal-replay-research.md` 开放问题 4 同族）
- table hash 遍历序：同构建确定；跨构建由 journal `code_ref.host_version` 覆盖

## 2. 挂载语义与粒度规则

### 2.1 分类判据

能力 sync 直挂须**同时**满足三条；任一不满足 → async 两跳。**缺省 = async**：sync 是逐件挣来的白名单优化。

1. **纯算**：无效应；失败 = 同步返回错误，无异步状态
2. **内生有界**：最坏耗时 µs~亚 ms 级可预估、无病态输入形态——sync 闭包是 native 代码，在 VM interrupt 时间盒覆盖**之外**（爆炸半径保险①只管 Luau 码，`context-events.md` §0.4），且占用共享 executor 线程
3. **无调用点决策**：不需能力门、无政策可设；journal 条目（如 `nondeterminism_read`）由宿主闭包自动记录，不要求作者感知（确定性件 replay 重算即可；需 record 的读取件由闭包内完成）

"有界"的落地 = 三件套：设计期算法复杂度账（O(n) 可知）→ 输入被 VM 内存限额硬封顶 → 实现期微基准断言钉死耗时上界。

### 2.2 分界线的精确表述

字面"纯算走全局库、效应走 ctx"不准确（`print`/`os.time` 是反例）。真分界线 = **作者是否需在调用点感知**：

| 形态              | 判据                                                                                         | 成员                                                    |
| ----------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| 全局库 / 原位替换 | 效应被宿主闭包**内部消化**：结果立得、归属自动附加（fiber 身份）、journal 自动记、无政策可设 | `print`、`os.time/clock`、`math.random`、全部纯算内建件 |
| ctx 方法          | 需在调用点**决策**：等结果（时间预算）、处理失败（Result）、声明能力（政策）                 | `ctx.fs/http/timer/sql`                                 |

### 2.3 形态与结构

- 全局库：无身份概念，随 §1.4 铁律冻结进 sandbox
- ctx 方法：经 Context（FiberId 随身，[ADR-0013](../decisions/0013-context-scope.md)）= 归属 / 能力门 / journal 拦截点的挂载位
- 写法：两类都是直线代码（coroutine 跨 host 挂起 spike 已证）；await 语义细则归 A4
- "为什么 `json.parse` 不用等、`fs.read` 要等"的结构性答案：前者结果当下立得，后者结果在未来

### 2.4 典型形态速查（判据优先，表是推论）

| 任务性质                                             | 去处                                                         | 形态     |
| ---------------------------------------------------- | ------------------------------------------------------------ | -------- |
| ns–µs 高频纯算                                       | 纯算内建件 sync 直挂                                         | 全局库   |
| 无感效应（立得、无政策）                             | 原位替换                                                     | 全局库   |
| 效应 / 核心长计算                                    | async 两跳                                                   | ctx 方法 |
| ms+ 业务粒度重物（可含效应，经 host imports 能力笼） | wasm 盒（[ADR-0024](../decisions/0024-carrier-layering.md)） | 盒桥     |

### 2.5 灰区（纯算大输入）处置

输入规模受 VM 内存限额天然封顶，但吞吐型操作（大文件 hash/gzip/正则长文）仍可能撞判据 2：

- **a+b 组合**：SDK 写明输入预算（a）；逐项画线时对"大输入场景真实存在"的件配 async/流式双形态（b——per-item 决策，非全局规则）
- 配套 = **选型即防护**：优先线性/无病态复杂度实现（正则 = 线性引擎，方言无 lookaround/backreference，从算法层消灭不可预估性）

### 2.6 成本锚点

SSOT = `../research/wasm-research.md` 补遗 1/2（实测）；两跳估算 = `context-events.md` §0.5。

| 路径                            | 成本                                                                                            |
| ------------------------------- | ----------------------------------------------------------------------------------------------- |
| Luau→host 函数调用（sync 闭包） | ~25ns + 值转换（int ~100ns；表深拷贝 ~300ns/entry）                                             |
| async 两跳机制                  | ~10–30µs 估算 + 效应本身                                                                        |
| 盒调用                          | 跨界 4.3ns（句柄必须缓存，否则 +26ns/次）+ 实例化 pooling 1.7µs；盒内标量任务比 Luau 省 36–125× |

推论：两跳税对 ns–µs 调用是 100–1000×（高频小件必须直挂）；对 ms 级任务是零头（重物走盒纯赚）。"重活不在 Luau 逐字节/逐元素循环"进 SDK 文档。

### 2.7 时间盒缺口纪律

native 闭包不受 interrupt 时间盒保护 → "native 函数不许无限阻塞"（执行模型既有纪律）+ 选型判据 + 双形态出路；未来出现真不可预知耗时的内建件 = 转 async 形态，永不放宽时间盒。

### 2.8 FAQ（讨论共识沉淀，SDK 文档沿用）

- **为什么存在两种调用？** 结果来源不同：纯算当下立得，效应结果在未来/外部世界。效应用 sync = 占 executor 线程干等 = 编排并发度死亡；纯算用 async = 白付两跳税 + 虚假挂起点。
- **作者怎么分辨 sync/async？** 写法无需分辨（直线代码，无 JS await 染色问题）；形态可辨（`json.*` = 立得，`ctx.*` = 要等）；语义区别（真实耗时 / 失败空间 / 能力门 / journal）由类型定义与 SDK 文档教。
- **插件能写多线程吗？** 不能也无需：Luau 无线程概念、盒禁 shared-memory（[ADR-0024](../decisions/0024-carrier-layering.md) C2）；线程两用途各有归宿——**等的事重叠 = `ctx.fanout` 并发等待**（2026-10-08 [luau-abi](luau-abi.md) §A4.1 兑现：单线程下等待本就重叠，fan-out 把多段等待组织进一个 handler），**算的事并行 = 盒实例并行 / 外部服务**。执行模型 = 插件内协作式自由交错（§A4.1）：同一瞬间至多一条协程在跑、切换仅在挂起点、无 lock 原语（互斥 = 显式 `ctx.exclusive`）；插件内交错序经冻结调度规则复现（§A4.1 D1.6；全局序 = journal 单流定序，§A6.7）。

## 3. 性能议程

### 3.1 主张

- **性能 = 编排并发度**：agent 负载是 IO 等待型（单步 10²ms–s 级，派发机制开销低约 3 个数量级，`context-events.md` §0.5）——优化目标是等待不占地 + 更多编排同时推进，不是单调用延迟
- **瓶颈 = 内存**：并发度上限 = 存活 VM + 盒实例数；**每 VM/每盒内存限额 = 并发槽单价**。盒容量账实测：VA = 槽×(预留+guard)、RSS ≤ 槽×max（`../research/wasm-research.md` 补遗 1）

### 3.2 接口形状三约束（画线尺子）

1. **流式优先**：凡输入/输出可能大的接口（fs、http body、sqlite 结果集）必有流式/分页形态——整段进 VM = 一份数据独占一个并发槽；与 §2.5 双形态是同一决策的内存维度
2. **批量友好**：高频小件允许批量化（值转换 ~300ns/entry 是真实税；"返回十万行的表"式接口形状双输）
3. **不假设值传递廉价**：大 Value 零拷贝/Arc 表示归 A6；接口设计不默认跨界无成本

### 3.3 反目标与登记

- 不做 ns 级投机优化（内联缓存、激进零拷贝先行）；不预调 blocking pool / `block_in_place`（`context-events.md` §0.5 已登记"实测瓶颈出现再捡"）
- 内存限额数值 = 实现期配置；ABI 只保证接口形状不假设无限内存
- 压测基准（并发吞吐 + 单槽内存账）注入测试策略议题；Luau 单 VM 基线内存实测 = 实现期基准项（spike 未测）

## 4. 内建件清单

### 4.1 两族

| 族           | 形态                | 成员                                                                                                       |
| ------------ | ------------------- | ---------------------------------------------------------------------------------------------------------- |
| 纯算内建件   | sync 直挂全局库     | json、yaml、regex、diff、uuid、hex、base64、timefmt、crypto                                                |
| 内建效应能力 | async 两跳 ctx 方法 | sqlite、fs、http、timer、process（后四者见 §5；process = 2026-10-07 开门，[luau-abi](luau-abi.md) §A3.10） |

新增与毕业纪律 = [ADR-0024](../decisions/0024-carrier-layering.md) §4（四判据 + 随核心发版 + 单向毕业路径），不重复。

### 4.2 逐项定案

| 件           | 定案                                                                              | 要点                                                                                                                                                                                                                                               |
| ------------ | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| json         | 进（已锁）                                                                        | `json.encode/decode`（随 Luau 生态惯例）；流式/SAX 不进 v1（走盒）                                                                                                                                                                                 |
| crypto       | 进（已锁，wasm 双重印证）                                                         | v1 = sha256/sha512 + hmac + `crypto.random`（CSPRNG，record）+ AES-GCM；hash 配流式双形态（§2.5 首个实例）；md5/sha1/签名族/xxhash 登记                                                                                                            |
| regex        | 进                                                                                | 线性引擎（无病态输入）；方言无 lookaround/backreference，报错须可行动；预编译（宿主侧缓存）+ 一次性双 API                                                                                                                                          |
| diff         | 进（高置信中最弱：论据 = 自我修改链系统级高频——审批 diff / edit 工具 / 版本对账） | 结构化 hunks 为主、unified 文本派生；sync + 预算，超大走盒                                                                                                                                                                                         |
| uuid         | 进                                                                                | v4 + v7（时间序，journal/事件 ID 友好）；编排级随机 PRNG 派生（§1.1 原则），replay 免费                                                                                                                                                            |
| hex / base64 | 进                                                                                | 独立小库；base64 标准方言默认、urlsafe 可选                                                                                                                                                                                                        |
| timefmt      | 进                                                                                | `format(ts, fmt)` + ISO8601 parse；`os.date` 薄壳共享引擎（§1.2）；**v1 = UTC only**，IANA 时区登记扩展位                                                                                                                                          |
| **gzip**     | **判出 → 盒**                                                                     | 主场景（http gzip）由 http 能力内部透明解压吸收；归档处理 = 业务粒度盒。四判据"敢画出去"的示范                                                                                                                                                     |
| yaml         | 进（边缘：论据 = 同引擎一致性 + 边际成本≈零，非通用度）                           | `yaml.parse/stringify`；与 config 同引擎同子集（[ADR-0007](../decisions/0007-yaml-config-subset.md)），↔Value 映射同 [ADR-0015](../decisions/0015-payload-value-model.md)                                                                          |
| sqlite       | 进，**内建效应能力位**                                                            | `ctx.sql.*`；每插件私有 db（能力声明授予）；参数化查询（事务 = 扩展位，2026-10-07 [luau-abi](luau-abi.md) §A3.9 修订）；结果集分页参数 = 实现期配置（原为“分页/迭代器（§3.2 约束①首个应用）”，同修订）；**状态与 replay 关系登记任务 6**，不在此答 |
| git          | 扩展位维持                                                                        | 领域相关 + 重依赖（git2）+ 低频；真实场景走盒/外部服务                                                                                                                                                                                             |

## 5. 出区通道

### 5.1 闭门论证

效应出沙盒的路径**可穷尽**（Luau 无 io/package；盒 IO 全走 host imports）：**磁盘（fs/sqlite）、网络（http）、时间推进（timer）、子进程（process——原为 spawn v1 不给，2026-10-07 随 ctx 形态议题能力门开门，见 [luau-abi](luau-abi.md) §A3.10）**——仍不存在第五扇门。（判据：门 = 插件可达**外部世界或持久效应**的通道。`print` 的日志出区经宿主自持格式与归属、止于宿主观测面，不算门。）

边界澄清：LLM 调用**不是** syscall 通道（产品模型红线；journal `llm_*` 条目是插件层产物）；用户/外部输入是**入区**（host 网关，session 议题），不在插件调用面。

### 5.2 通道逐项

- **fs**（`ctx.fs.*` 等）：**三 scope（private/workspace/global），SSOT = [luau-abi](luau-abi.md) §A3.7**（2026-10-07 落位，原为“v1 = 每插件私有数据目录”单一形态）——preopen 哲学不变：private 免绑定、workspace 部署绑定（任务 4）、global = 用户世界 − 两条豁免（secrets store / journal）；读/写分档按 scope 声明；写 = 临时文件 + rename **原子写**（[ADR-0023](../decisions/0023-core-restart-semantics.md) 落盘要求的落点）
- **http**（`ctx.http.request`）：透明 gzip（§4.2 判出的主场景在此吸收）；**强制超时**（不允许无 deadline 请求）；响应默认进内存带预算，大 body/SSE 走流式（§3.2 约束①）；**域名白名单细化归 A2**（prompt 注入 → 数据外泄是 agent 头号攻击面，声明粒度是安全权重最高的一格）
  - 配套政策（**白名单政策 SSOT = [luau-abi](luau-abi.md) §A2.5**，2026-10-06 落位）：跨域重定向须重新对账（open-redirect 旁路）；SSRF = 解析结果非全局单播即拒（含 v6）+ 白名单显式授予内网 + 最终解析结果判定钉住；明文 http 须 `http://` 显式标记——细则以该节为准
- **timer**（`ctx.timer.after/every/sleep`）：replay 按史注入不按墙钟；注册为 fiber 的一笔 effect、卸载随 LIFO drain 自动取消（无孤儿）；**无能力门，但 `every` 设最小间隔政策**（机制在此，数值实现期配置；DSH 先例 5 分钟硬下限锚的是**持久调度**梯度，见 §5.5）；cron 形态登记；sleep = 挂起当前协程、宿主直接唤醒（2026-10-07 [luau-abi](luau-abi.md) §A3.6 增，A1.4 红线形态）
- **process**（`ctx.process.*`）：**开门**（原为 spawn v1 不给；形状 SSOT = [luau-abi](luau-abi.md) §A3.10）——能力声明 `process: true` 无参数；强制超时 / 进程组杀 / 无孤儿（账本 drain）/ journal 全量元数据；命令级审批归插件层策略机器（§A2.6 两层授权分工）。**支配关系**：process 继承核心进程 OS 全权——fs 三段式/两条豁免/http 白名单**均不经由它强制**（子进程里读 secrets store、curl 任意域物理可行）；审批面按 ⊇ `global: write` + 任意域 http 的权重展示；外墙 = OS 级沙箱，归部署组成（任务 4/实现期登记）。**编程式 spawn 插件派生仍推迟**（[ADR-0017](../decisions/0017-programmatic-spawn-deferred.md)，与本条无涉）；MCP stdio 监管机器 host 自持不变

### 5.3 汇总映射（喂 journal kind 表 + 能力声明面）

| 通道          | 形态                       | 能力声明                                                    | journal kind                                                       | replay                                               |
| ------------- | -------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------- |
| fs            | `ctx.fs.*` 等三 scope      | `fs`（三 scope × 读/写分档，[luau-abi](luau-abi.md) §A3.7） | `fs_read` / `fs_write`（audit 族）                                 | 私有状态族 → 任务 6                                  |
| http          | `ctx.http.request`         | `http`（域名细化归 A2）                                     | `http_request` / `http_response` ★                                 | record/注入（网关纪律 M9 兑现）                      |
| timer         | `ctx.timer.*`              | 无门                                                        | `timer_scheduled` / `timer_fired` ★                                | 按史注入                                             |
| sqlite        | `ctx.sql.*`                | `sqlite`                                                    | `sql_call` / `sql_result`（audit 族）                              | 私有状态族 → 任务 6                                  |
| process       | `ctx.process.*`            | `process`（无参数）                                         | `process_spawn` / `process_exit`（元数据族；流式内容政策归任务 6） | record/注入到头（副作用出视野不可回滚，任务 6 登记） |
| 时间/随机读取 | 原位替换 / `crypto.random` | 无门                                                        | `nondeterminism_read` ★                                            | record/注入（PRNG 部分 recompute）                   |

（kind 表全文 = `../research/journal-replay-research.md` §7.3；本表 = syscall 部分回填。2026-10-08 修订注：kind 表去 agent 化 + 录制分族政策（★ 全录 / audit 降档）已定案 = [luau-abi](luau-abi.md) §A6.7——调研 §7.3 的 `llm_*`/`session_*` 不属核心 kind。）

### 5.4 replay 分族原则

**边界输入族**（http / timer / 非确定读取）= record/注入；**私有状态族**（fs / sqlite）= 与 journal 正式设计一并定——"插件把状态外置后，重放时的世界算什么"是任务 6 的头号输入，不在此硬答。

### 5.5 时间调度四梯度

| 梯度                                | 形态                                                 | 归属                                                                                                                                        |
| ----------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| 易失定时（秒–分钟，热更/卸载即消）  | `ctx.timer.after/every`                              | 核心原语（§5.2）                                                                                                                            |
| 持久调度（跨热更/重启的提醒）       | 插件层 schedule 服务 = sqlite + timer 原语 + journal | 插件层议题登记（DSH `dsh-schedule` 已验证此路线：日志持真相、定时器为易失投影、every ≥5min、错过间隔不补发、墙钟回拨不早发/前跳算 overdue） |
| 声明式周期任务（cron 式，更可审批） | v1 = config 键约定（[luau-abi](luau-abi.md) §A2.7）  | 一等声明式调度复审归任务 4                                                                                                                  |
| replay 中的时间                     | 按史注入                                             | 已定（§5.3）                                                                                                                                |

主被动模型注：timer 不破"插件被动调用"模型——tick 是邮箱消息的一种（[ADR-0011](../decisions/0011-actor-execution-model.md)），`every` = 订阅宿主时钟这个事件源；主被动分界 = **谁拥有循环**，线程式 `while+sleep` 已结构性封死（§2.8 多线程条）。

## 6. 联动与登记

### 6.1 ABI 议题回填

| 议题          | 回填                                                                                                                                                                                                                                                                                       |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A2 manifest   | `capabilities` 字段面 = { fs（三 scope × 读/写，A3 修订）、http（可域名细化）、sqlite、process（A3 开门） }；纯算内建件无需声明                                                                                                                                                            |
| A3 ctx        | ctx 效应方法面 = fs（三 scope）/ http / timer / sql / **process**（2026-10-07 开门）——已全落 [luau-abi](luau-abi.md) §A3；编排面（事件/服务调用）同 §A3                                                                                                                                    |
| A4 await      | **A4 全章已收官**（2026-10-08 §A4.1 + 2026-10-09 §A4.2–§A4.6，[luau-abi](luau-abi.md)）：挂起点 = ctx 方法调用处；插件内自由交错 + `ctx.exclusive` / fan-out / 调度规则冻结进 ABI（单队列 FIFO）；coroutine shim / 保险丝总表 / 卸载细则                                                   |
| A5 错误       | **A5 全章已收官**（2026-10-09，[luau-abi](luau-abi.md) §A5.0–§A5.5）：多返回值 `res, err`（Ok 载荷永不为 nil，流式 EOF 位例外）/ err 封套 `{kind, message, data?}` 封闭枚举 / 族清单（三条分界判据 + 效应族表 + 流式三态 + 盒二态）/ 处置习语与 throw 归宿；sync 件 = 同步错误返回维持既定 |
| A6 边界转换   | **A6 已收官**（2026-10-08，[luau-abi](luau-abi.md) §A6.0–§A6.8）：大 Value 零拷贝/Arc 表示、Bytes 扩展位与 buffer 关系、流式句柄族、盒线编码 TLV、journal 格式草案                                                                                                                         |
| A7 schema DSL | **A7 已收官**（2026-10-09，[luau-abi](luau-abi.md) §A7.0–§A7.5）：YAML 骨架 + 表达式串记法；类型词汇表（`buffer`/`function` 限 ctx 面）；契约包三件套与对账；运行期校验；生成物单源（`.d.luau`/WIT/文档骨架）                                                                              |
| A8 事件声明   | **A8 已收官**（2026-10-09，[luau-abi](luau-abi.md) §A8.0–§A8.5）：manifest events 双名单（意图层 + analyze 对账）；形状定义唯一住处 = 契约命名空间（核心内嵌 / lib/ 生态 / 插件随包三来源）；启用期对账 + emit 跟名校验；插件 emit `internal/` = 响亮；不立通配                            |
| A9 能力门槛   | **A9 已收官**（2026-10-09，[luau-abi](luau-abi.md) §A9.0–§A9.4）：对账时机统一 = 启用期（D24——本行原“安装面”登记措辞平移）；inject 缺失三分 + warn 提醒 / 依赖禁环启用期 / 盒对账启用期（D25）；能力门强制点总表 + 编排面（事件/服务）不立能力门 = 能力间接化合法（D26）                   |
| A10 工具形态  | `metis sdk` analyze 管线 = 四层防线写码期/安装期（§1.5）                                                                                                                                                                                                                                   |

### 6.2 journal 正式设计（任务 6）输入

- kind 表 syscall 部分 = §5.3；私有状态族 replay 政策 = 头号输入；`nondeterminism_read` 已覆盖时间/CSPRNG 读取
- **写入量账**：§5.3 的 per-call 条目在忙插件下达日百万级（~0.2–0.5 GB/天/插件），单条大 body 可达 10⁷–10⁸B——需录制粒度政策（全量内联 vs blob 引用 vs chunk；http 大 body/SSE 与 LLM 流式同族，参 `../research/journal-replay-research.md` 开放问题 2）——**2026-10-08 已定案**：[luau-abi](luau-abi.md) §A6.7 D7.1（★ 全录 + audit 默认降档 + 阈值 blob 引用）
- **生成型秘密的落盘政策**：`crypto.random`（record）生成的密钥/token/nonce 会明文进 `nondeterminism_read`——M6 脱敏管线的模式匹配抓不到生成型秘密，需 kind 感知政策（append 加密/擦除 + replay 注入侧解密）——**2026-10-08 已定案**：[luau-abi](luau-abi.md) §A6.7 D7.2（kind 感知加密，按 source 子类分流）

### 6.3 实现期登记

- §1.6 locale 断言与 table 序；§3.3 Luau 单 VM 基线内存实测；timer 最小间隔 / http 预算与超时 / sqlite 分页参数 = 实现期配置
- mlua 无条件安装件（`require/collectgarbage/loadstring/_VERSION`）的手工处置进 `metis-luau` 转换层实现清单（§1.4）
- 整数注入的 raw push 辅助（`../research/wasm-research.md` 补遗 1 integer 条）与本文档无涉但同属转换层——实现时一并落
