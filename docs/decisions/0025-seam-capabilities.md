# 0025. 接缝能力集冻结：stdlib 卫生 / 挂载语义 / 内建件清单 / 出区通道

- Status: accepted
- Date: 2026-10-10

## Context

ABI 议题地图 B1+B2 合并议题（"接缝能力集"）2026-10-05 讨论收官，定案全文、淘汰选项、成本锚点（mlua/wasmtime 双 spike 实测）与 FAQ 在 [seam-capabilities](../design/seam-capabilities.md)（living doc）。前提：产品模型（core = 插件管理与组合机器 + syscall 层 VM 基底，不提供 agent 领域服务）；载体分层 = [ADR-0024](0024-carrier-layering.md)。本 ADR 冻结该议题的决策面，与 Luau ABI 冻结（[ADR-0026](0026-luau-abi.md)）同批。

## Decision

**设计目标（定调）**：syscall 层提供**广而抽象的领域中性能力、一律业务粒度接口**，使 Luau 保持薄编排（几十行：声明 + 组合 + 少量状态机）；密集逻辑必有去处（纯算内建件 / SDK 库层 / 服务组合 / wasm 盒）。

### 1. stdlib 卫生（每个插件 VM = 确定性沙盒 + 宿主受控缝）

- **非确定性源处置**：`os.time`/`os.clock` 原位替换为宿主受控时钟（journal record 族）；`math.random` 原位替换为宿主确定性 PRNG（per-fiber 种子，replay recompute），`math.randomseed` tombstone。随机性来源原则：编排级 = PRNG/recompute；安全级（`crypto.random`）= CSPRNG/record。
- **os 面**：`os.date` 原位替换为薄壳（无参走注入时钟、有参纯格式化，引擎 = timefmt 内建件）；`os.difftime` 保留（纯函数）。
- **裁剪面**：`loadstring` tombstone（红线：动态 eval 绕过审批链与版本钉版）；`require` 保留 + 宿主包装；`print` 原位替换（改道宿主日志、自动带 fiber 身份）；`collectgarbage`/`gcinfo`/`newproxy`/`getfenv`/`setfenv`/`debug.info` tombstone（`debug.traceback` 保留——错误诊断质量）；`coroutine` 保留 + shim 包装（syscall 透明，[ADR-0026](0026-luau-abi.md) A4 节）；其余纯算件（`string`/`table`/`buffer`/`bit32`/`utf8`/`vector`/`integer`/`math` 余部/语言核心函数/`_G`/`_VERSION`）全部保留。
- **建 VM 姿势**：白名单 `new_with`（fail-closed 对 vendored Luau 升级）→ 宿主裁剪 + 注入 → `sandbox(true)` 冻结 → 才跑插件代码（硬化顺序铁律）。
- **四层防线**：写码期 SDK 类型定义标红 / 安装期静态检查 / 生成期 Creator 上下文清单 / 运行期 tombstone 可行动报错（兜底）。

### 2. 挂载语义与粒度规则

- **sync 直挂三判据**（须同时满足，缺省 = async 两跳）：纯算 / 内生有界（最坏耗时 µs~亚 ms 可预估、无病态输入）/ 无调用点决策（不需能力门、无政策可设）。
- **真分界线 = 作者是否需在调用点感知**（等结果 / 处理失败 / 声明能力）：全局库与原位替换 = 效应被宿主闭包内部消化；ctx 方法 = 需在调用点决策。
- **灰区（纯算大输入）**：SDK 写明输入预算 + 大输入场景真实存在的件配 async/流式双形态（per-item 决策）；选型即防护（线性/无病态复杂度实现优先）。
- **时间盒缺口纪律**：native 闭包不受 interrupt 时间盒保护 → native 函数不许无限阻塞；真不可预知耗时的件转 async 形态，永不放宽时间盒。

### 3. 性能议程

- **性能 = 编排并发度**（agent 负载是 IO 等待型，优化目标 = 等待不占地 + 更多编排同时推进）；**瓶颈 = 内存**（每 VM/每盒内存限额 = 并发槽单价）。
- **接口形状三约束**：流式优先（大输入/输出必有流式或分页形态）/ 批量友好（高频小件允许批量化）/ 不假设值传递廉价。
- **反目标**：不做 ns 级投机优化，不预调 blocking pool；限额与预算数值 = 实现期配置，不进 ABI。

### 4. 内建件清单

- **纯算内建件**（sync 直挂全局库）：json、yaml、regex、diff、uuid、hex、base64、timefmt、crypto（v1 = sha256/sha512 + hmac + `crypto.random`（CSPRNG）+ AES-GCM；hash 配流式双形态；md5/sha1/签名族/xxhash 登记）。
- **内建效应能力**（async 两跳 ctx 方法）：sqlite、fs、http、timer、process。
- **判出示范**：gzip → 盒（主场景 http gzip 由 http 能力内部透明解压吸收；归档处理 = 业务粒度盒）；git 维持扩展位。
- 新增与毕业纪律 = [ADR-0024](0024-carrier-layering.md) §4（四判据 + 随核心发版 + 单向毕业路径），不重复。

### 5. 出区通道（闭门论证：四扇门，不存在第五扇）

效应出沙盒的路径可穷尽（Luau 无 io/package；盒 IO 全走 host imports）：**磁盘（fs/sqlite）、网络（http）、时间推进（timer）、子进程（process）**。LLM 调用不是 syscall 通道（产品模型红线）；用户/外部输入是入区，不在插件调用面。

- **fs**（`ctx.fs.*` 等）：三 scope 与两段式路径解析 SSOT = [ADR-0026](0026-luau-abi.md) A3 节；写 = 临时文件 + rename 原子写。
- **http**（`ctx.http.request`）：强制超时；透明 gzip；域名白名单 / SSRF / 跨域重定向政策 SSOT = [ADR-0026](0026-luau-abi.md) A2 节。
- **timer**（`ctx.timer.after/every/sleep`）：replay 按史注入；无能力门，`every` 最小间隔政策（数值实现期配置）。
- **process**（`ctx.process.*`）：能力门开门（`process: true` 无参数）；强制超时 / 进程组杀 / 无孤儿；**支配关系写实**：process 继承核心进程 OS 全权，fs 三 scope 与两条豁免、http 白名单均不经由它强制，外墙 = OS 级沙箱归部署组成。MCP stdio 监管机器 host 自持不变；编程式 spawn 插件派生仍推迟（[ADR-0017](0017-programmatic-spawn-deferred.md)，与本条无涉）。
- **journal kind 映射（syscall 部分）**：audit 族 = `fs_read`/`fs_write`/`sql_call`/`sql_result`；★ 族 = `http_request`/`http_response`/`timer_scheduled`/`timer_fired`/`nondeterminism_read`；元数据族 = `process_spawn`/`process_exit`（全录写死）。录制分族政策（★ 全录 / audit 默认降档）= [ADR-0026](0026-luau-abi.md) A6 节（A6.7）。
- **replay 分族原则**：边界输入族（http/timer/非确定读取）= record/注入；私有状态族（fs/sqlite）的 replay 政策 = journal 正式设计（任务 6）的头号输入，不在此硬答。
- **时间调度四梯度**：易失定时 = 核心原语（`ctx.timer.*`）；持久调度 = 插件层 schedule 服务（原则四件套见 [ADR-0026](0026-luau-abi.md) A2 节 / [luau-abi](../design/luau-abi.md) §A2.7）；声明式周期任务 = config 键约定；replay 中的时间 = 按史注入。

### 演进纪律

- 能力清单新增/毕业走 [ADR-0024](0024-carrier-layering.md) §4 四判据；能力词汇属 ABI 面（[ADR-0026](0026-luau-abi.md) A10 节），additive 增补随核心发版同步 bump `abi_rev`。
- living doc 保持 living：细则澄清与教学条继续沉淀于此；凡改语义的修订不允许只走 doc 修订注，必须按上一条或新 ADR supersede。

## Consequences

- 接缝能力集自此冻结；逐项定案细则（§4.2 逐件要点）、成本锚点（spike 实测数字）、灰区处置与 FAQ 以 living doc 为准。
- 实现期登记项（locale 断言、指针身份串归一化、基线内存实测、各预算数值）不进冻结面，归实现期清单（doc §1.6/§6.3）。
- journal kind 表 syscall 部分与私有状态族 replay 政策 = journal 正式设计（任务 6）输入；audit 族降档开关面 = 配置键，归任务 4（配置格式设计）。
