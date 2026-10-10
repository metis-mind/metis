# 0026. Luau ABI 冻结（A1–A10）与既有 ADR 局部 supersede 批次

- Status: accepted
- Date: 2026-10-10

## Context

ABI 议题地图 A 系列十章（A1 插件包形态与入口契约 / A2 manifest 格式 / A3 ctx 形态与方法集 / A4 await 与交错语义 / A5 错误与 Result 习语 / A6 边界转换细则 / A7 schema DSL / A8 事件静态声明 / A9 deps-provide 与能力门槛 / A10 工具形态与 ABI 版本纪律）于 2026-10-06 至 2026-10-10 逐条讨论拍板收官。前提：扩展语言选型（[ADR-0021](0021-extension-language-runtime.md)）、载体分层（[ADR-0024](0024-carrier-layering.md)）、接缝能力集（[ADR-0025](0025-seam-capabilities.md)）。**总基调 = 严格制**：类型/形态精确匹配、零隐式 coercion；真实摩擦出现再经演进纪律（末节）放宽。六个工具链验证项已实测收官（Luau 0.740 / luau-lsp 1.70.1 / wasm-tools 1.261.0；SSOT = [luau-toolchain-spike](../research/luau-toolchain-spike.md) + [wasm-research](../research/wasm-research.md) 补遗 4）。

本 ADR 做三件事：① 冻结十章决策面（下文每节 = 一章浓缩；淘汰选项、示例、概念地基、联动登记全文在 [luau-abi](../design/luau-abi.md)（living doc）对应章，不复述）；② 立演进纪律；③ 同批吸收各章登记的对既有 ADR 的局部 supersede——逐条命名被取代的确切条款，旧 ADR 正文冻结不动、Status 行记录。

## Decision

### A1 插件包形态与入口契约

- **入口返回单形态 table `{ setup }`**，`setup(ctx, config, deps)`；字段集封闭（v1 仅 `setup`），出现其他字段 = 响亮报错（挂元表绕过同判）。`plugin(fn)` 糖 = 宿主注入全局函数（`{ setup = fn }` 的写码层归一），核心零归一机器；入口返回裸函数 = 响亮报错并指路 `plugin()`。
- **清理习语**：经 ctx 注册的一切（监听/定时器/服务/子 fiber/句柄/盒实例）自动入账本、卸载自动摘除；手动清理唯一通道 = setup 返回一个 disposer 函数（记账第一页、LIFO 最后执行；统一异步签名可挂起；抛错逐个吞掉不中断；结算期拒绝新账 = 响亮错误）。热更新与核心重启同一清理契约。
- **两形态严格统一**：纯代码插件（`foo.luau`）与包插件（`foo/`）共享同一入口契约，成长路径零改写；差异面只允许从"有无 manifest"派生——纯代码插件无 config schema、无 provide/inject（仅事件通道）、无能力门资格、无盒（结构性不可用）。含盒包 = 多了 `boxes/*.wasm` 制品的包插件，不是第三形态。
- **入口执行语义**：setup = fiber 第一个邮箱回调（激活门：Active 前邮箱排队不消费）；允许挂起 + 加载总预算（超限 = Loading 失败 → Failed）；死锁红线 = setup 里只能挂起"完成不依赖自身邮箱"的调用。
- **require** = Luau 官方 require-by-string 全集，Metis 零私有解析语义；叠加裁剪仅两处：解析结果不得逃逸插件根（`@lib` 共享库除外）；运行期只认部署根一份 `.luaurc`。裸名/绝对路径报错；循环 require = 响亮报错（复审触发器 = 上游 cycle RFC 落地）；入口模块归一（包内回引入口拿到同一实例）。

### A2 manifest 格式

- manifest = 插件自述纯数据（作者视角、随包走），≠ 配置树（部署者装配单，归任务 4（配置格式设计））。
- **顶层字段封闭集**：`name`（显式值必须 ≡ 目录名）/ `version` / `entry`（缺省 `main.luau`）/ `description` / `config` / `provides` / `inject` / `capabilities` / `boxes` / `events` / `abi_rev`（缺省 1，A10 节）。**未知字段 = 响亮报错**（顶层与嵌套同政策，报错带字段名 + "core 版本过旧？"提示）。有意不收：authors/license/repository 等市场元数据（归任务 9（marketplace 格式设计））、`runtime`、`min_core`。
- **原则两条**：错误尽早 = 校验器单一实现、多面暴露（编辑器 / `sdk check` / loader，报错措辞逐字一致）；声明 ≠ 观察（manifest 承载作者意图；机器的角色 = 生成初稿 + 对账漂移）。
- **service 撞名三层**：`plugin-name/service` 前缀约定（概率趋零，引导落 `sdk new` 模板）/ 契约 schema 对账机械检测 / 真撞 = 启用期响亮 + 本地解（部署者二选一 / isolate / 重命名）。**故意同名 = 契约竞标（特性）**，schema 对账 = 区分竞标与撞名的机械裁判。契约治理三层：平台内嵌（契约版本 ≡ 核心版本）/ 生态契约包 / 插件私有服务；混乱防火墙 = 分歧无法静默。
- **字段形状**：config schema（`secret: true` = 引用式秘密，配置树写明文 = 校验响亮拒绝；真值住部署根 secrets store，只进声明插件 VM）；provides = 全形 map（`contract:` 引用位）；inject = 双列表（`required` / `optional`）；capabilities = map 形（`fs` = scope map `{private, workspace, global}` × `read|write`、write ⊃ read；`http` = 域名白名单（精确 host + `*.` 子域通配）+ https 缺省（明文须显式 `http://` 标记）+ SSRF 最终解析结果判定 + 跨域重定向重新对账；`sqlite: true`；`process: true`；缺省 fail-closed）；boxes = 显式列表 + 双向对账与进口对账（盒能力 = 属主包络，时机 = 启用期见 A9 节）。
- **两层授权分工**：能力门 = 权限包络（安装时一次性审批、运行时机器强制）；动作裁决 = 插件层策略机器，核心不提供动态审批 syscall。
- **周期调度不立一等 cron 字段**：周期意图 = config 键约定；持久调度原则四件套 = 意图数据持真相 / timer 为易失投影 / 寻址走稳定词汇 / 归属匹配意图层级。

### A3 ctx 形态与方法集

- **容器形态**：ctx 及一切宿主装配句柄（deps/盒/process/流式句柄）= 宿主构建的**递归只读 frozen table**、无元表；方法 = 宿主逐方法安装的 Rust 闭包（身份闭包捕获）。淘汰 userdata（流式句柄族同此结案，A6 节）。
- **顶层组织**：编排面扁平（`on`/`emit`/`serial`/`parallel`/`waterfall`/`provide`/`exclusive`/`fanout`/`box`）；效应面命名空间化（`fs`/`workspace`/`global`/`http`/`timer`/`sql`/`process`）。效应命名空间在 ctx 上**恒定存在**（纯代码插件同形）；未声明能力调用 = 响亮报错指路 manifest。
- **`deps` = setup 第三参数**：按 manifest inject 装配的烘焙方法集冻结表（按注册表方法集逐方法生成闭包；点调用、单参数）；**可选依赖缺席 = 字段不存在**（`if deps.cache then` 成立；epoch 内缺席稳定）。
- **事件面**：四派发语义 = emit 忽略返回 / serial 注册序逐个调、首个终止值短路 / parallel 等全部、逐项结果 `{ ok, value/err }` / waterfall 变换链、nil 否决；`ctx.on` 返回摘除函数；不立 `ctx.once`（扩展位）。
- **`ctx.provide(key, 方法表)`**：注册 = 激活点捕获方法引用快照；Active 后 provide = 响亮报错。
- **timer 两种唤醒路径**：`after`/`every` 回调 = 邮箱消息；`sleep` = 宿主直接 resume 挂起协程（不排队，setup 内安全）。精度契约 = not-before 语义。
- **fs**：三 scope（private 免绑定 / workspace 部署绑定 / global = 用户世界 − 两条豁免）对应三命名空间（`ctx.fs`/`ctx.workspace`/`ctx.global`）；**两条豁免** = secrets store 与 journal 对一切 scope 读写同禁。两段式路径解析（词法归一 → 物理 canonicalize 后验边界/豁免）。方法集（三 scope 相同）：`read`（UTF-8 校验）/ `write`（tmp+rename 原子写，自动建父目录）/ `append`（原子追加）/ `list` / `delete` / `exists` + 二进制 `readbytes`/`writebytes` + 流式 `openread`/`openwrite`。
- **http 单方法** `ctx.http.request(opts)` + 流式 `ctx.http.stream`；安全政策 SSOT = A2 节 capabilities；透明 gzip 宿主吸收；强制超时。
- **sql 两方法** `exec`/`query`，只许参数化占位 `?`；每插件一个私有 db（不在本插件 fs 私域内，global scope 可达）；事务 = 扩展位。
- **process 开门**：`exec`（spawn + 收齐 + wait 的糖）/ `spawn`（句柄：`read_stdout`/`read_stderr`/`write`/`signal`/`wait`/`kill`）；`argv` XOR `command`；强制超时杀进程组；句柄/在途 exec = 账本条目，卸载 drain 无孤儿。
- **require 实现** = 每插件 VM 自定义 `Require`（mlua `create_require_function`）。**盒调用桥** = `ctx.box(name)` 访问器（未声明 = 响亮报错）+ 挂起式调用（盒内 ms 级计算不堵共享 executor）。

### A4 await 与交错语义

- **插件内默认自由交错**：handler 挂起即让路、邮箱下一条先跑；**协作式** = 同一瞬间至多一条协程在 CPU 上、切换仅在挂起点、两挂起点之间原子（无并行、无数竞）。同步全显式，host 不提供隐式串行。本格即对 ADR-0011 的局部 supersede（末节批次 #1）。
- **`ctx.exclusive(name, fn)`** = 插件内具名互斥：同名函数体同一时刻至多一条协程在内（含挂起期间）；闭包圈定、返回/抛错自动释放。三规则：不嵌套（死锁结构不可表达）/ fan-out 子协程不进自家人占用的名字 / 强杀与卸载强制释放。不立全局互斥（扩展位结案不留）。
- **`ctx.fanout({...})`** = 一个 handler 里并发等多个 ctx 调用，返回各 Result；结构化纪律 = handler 结束时其 fan-out 子协程必须都已完结，未完结 host 取消。
- **调度规则纯函数化冻结进 ABI**（replay 免录恢复顺序）：每 fiber **单可调度队列 FIFO**——新消息（经在飞预算闸门接纳）/ 完成恢复 / fanout 子创建共用入队序，tie-break 不存在；**预算闸门在接纳处、完成永不挡**；预算数值 = 调度第四输入，replay 期必须用录制期值（钉版）。
- **术语分层**：作者视角「线程」= 核心登记并调度的执行流（handler 本体 / fanout 子）；「协程」= 插件自建裸 coroutine（控制流件）。**coroutine shim = syscall 透明**：裸协程里调 ctx 合法，SENTINEL 令牌冒泡到恢复者线程挂起；在飞协程独占恢复者（`outstanding` 弱键表防静默假注入）；纯算协程切换不进核心。被登记线程裸 yield = 响亮杀。
- **保险丝**：插件内等待面不立总 deadline（唯一真无界源 = 静默流 read，扩展位 = per-read timeout）；在飞预算超限 = **纯回压**不立响亮档；fail-fast 检测面八条 = exclusive 嵌套 / 自家人进占用名字 / wait-for 圈闭合 / 被登记线程裸 yield / fanout 子未随 handler 完结 / 卸载期注册新账 / 不可转换值跨界 / 未声明能力调用。
- **跨插件残余圈四层处置**：启用期 manifest required 依赖图禁环（硬拒，见 A9 节）/ 写码期 lint 教具 / 运行期 wait-for 即时成环检测（当场报错 + 整圈路径 + fail-open 解圈）/ 超时保险丝兜底。
- **卸载四段严格串行**：关闸 → 优雅窗口 → 强杀 → drain；Unloading 全程拒绝"比我长寿"的新账；在飞服务调用对手方卸载 = host 立即注入 `Err(Unavailable)`（不傻等 deadline）；邮箱与可调度队列残留丢弃（其中服务调用条目即错回调用方）。**结果保证分级**：服务调用 = 结果或显式 `Err`、绝无静默；事件/消息 = 尽力 + journal 可见；消息重投 = 否决。
- **fs `append` = 原子追加**（单次调用整体落盘不撕裂；跨插件顺序不承诺）；原子方法族其余全部扩展位（复用 fs scope 写权限，无新能力词）。

### A5 错误与 Result 习语

- **多返回值 `res, err`**：挂起式调用失败经值通道返回，不立异常主通道。**Ok 载荷永不为 nil**（唯一例外 = 流式 read 族 EOF 位）；`assert` 升级习语白拿。纪律作用面 = 效应/服务/盒/process 两槽调用；编排面（事件派发/fanout join）走 ADR-0014 fail-open 与收集制。sync 直挂件失败 = 抛错。
- **err = 数据表封套 `{ kind, message, data? }`**（普通数据表非 frozen）：**kind 定控制流，message 只进日志/人读，data 按族查表**。
- **kind = 封闭枚举冻结进 ABI**：能力族 = 两级串 `"族.具体"`；服务四态与 business = 单级通用词（`unavailable`/`method_missing`/`timeout`/`business`，ADR-0018 S5 四态平移，不立第五格）。`Err(Business, Value)` → `kind = "business"` + data = 提供方自定义 Value（其形状 SSOT = 服务契约文档；**唯一例外** = 宿主注入的契约违约封套，data = ABI 固定形状 `{ violation: string }`）。
- **族清单生成规则 = 三条分界判据**：① Err vs Ok 状态字段看"载荷是否完整成立"（HTTP 4xx/5xx、进程非零 exit = Ok 载荷分支）；② Err vs 响亮报错看"静态可否查"（契约违反 = 响亮；运行期数据决定 = Err）；③ 细分粒度看"动作是否不同"，开放空间必设诚实兜底格。
- **效应族 kind 表**：fs×7（`fs.not_found`/`permission_denied`/`invalid_path`/`invalid_utf8`/`invalid_type`/`too_large`/`io`）/ http×6（`http.timeout`/`connect`/`policy`/`too_large`/`invalid`/`error`）/ sql×2（`sql.constraint`/`sql.error`）/ process×3（`process.spawn_failed`/`timeout`/`gone`）/ timer 无 Err 面。
- **流式 read 三态**：chunk / `nil, nil` = EOF / `nil, err` = 中途错误；不立新 kind，复用属主族枚举。**盒错误二态**：业务错误 = `business` 透传（data = 盒自定义 Value）；trap = `box.trap`（宿主产生）。**能力缺失 = 响亮报错**（契约违反族），与部署政策拒绝（Err `http.policy`）两界不混。
- **不立 retryable 标识**（宿主知识不足 / 诱导裸重试 / 模糊 bit）；替代物 = kind 分格 + Creator 文档"族→建议动作"对照表 + data 演进通道。**覆盖性纪律**：兜底格必设 + 禁 message 匹配做控制流（子类提格走 ABI rev）。
- **处置三习语**：分支 / `assert` / `error(err)` 透传升级（Lua 版 `?`）；**`return nil, err` 不是合法升级通道**（provide 返单值契约下第二值被丢 = 静默吞错）。throw 归宿总表：setup = 加载失败；事件 listener = fail-open 记日志（parallel 项 = `{ ok = false, err }` 收集）；provide 方法 = 消费方收 `Err(business)`；fanout 子未捕获 = 同上收集；disposer = 逐个吞掉。`error(v)` 映射 = data 深快照；v 不可转换或超预算 = 落字符串化摘要、不二次失败（错误通道自身必须无敌）。

### A6 边界转换细则

- **Luau↔Rust 无序列化**（tagged union 栈槽位查标签取载荷）；三层 Value 辨析 = Luau VM 值 / mlua `Value`（注册表句柄镜像）/ metis `Value`（Rust 拥有的数据货币）。
- **integer**：写路径 = raw push 收口（metis-luau 内 `push_int64_exact`，~15 行 unsafe 一处，调用面无感；移除触发器 = mlua 上游修写路径）；读路径零处理。**严格边界纪律**：契约槽位声明类型 ↔ VM 标签精确匹配、零 coercion，报错指路写法；契约写作连带指南 = 手写字面量高频槽位声明 `Float`、宿主注入往返值声明 `Int`。算术面口径：VM 运算符对 integer 报错，算术走 `integer.*` 库，大整数"只传不比算"。
- **table 一律调用点深快照**（递归读成 metis `Value` 树；注册面 provide/on/timer 回调按引用捕获函数 = 例外）。**不可转换值响亮失败**：嵌套 function/userdata/thread / 循环引用 / 带元表的表（报错指路显式铺平）；错误通道 carve-out = `error(v)` 落字符串化摘要（A5 节）。**Array/Map 严格二分 + 契约优先**（键恰 1..n 连续整数 = Array；全字符串键 = Map；混合/稀疏 = 响亮失败；空表判 Array）。成本结构 = 1+N（1 次源 VM 深读 + N 次目标 VM 建表），中间 `Value` 不可变经 `Arc` 共享。
- **Rust 侧表示**：`Value` = 朴素 owned tree + 接缝 `Arc<Value>`；保持 `Send + Sync` 显式约束；Map 容器 = `BTreeMap`（确定性迭代序，journal/replay 白送）。
- **二进制**：`Value::Bytes` 维持扩展位（触发器 = 首个跨插件二进制消费者；预定形态 `Bytes(Arc<[u8]>)` ↔ buffer）；**buffer = Luau 侧二进制法定形态**（VM-local，永不跨插件；schema 词汇限 ctx 面）。方法面切割：fs `readbytes`/`writebytes` 分立；`fs.read` 保 UTF-8 契约（失败 = `Err(fs.invalid_utf8)` 指路 `readbytes`）；sql 不立 BLOB 列（遇到 = `Err(sql.error)`；列映射 NULL→Null / INTEGER→`Int(i64)` / REAL→Float / TEXT→String）；大二进制归流式。
- **流式句柄族**：拉式 `read()` 挂起 + 三态返回；全族 frozen table 烘焙（userdata 淘汰结案）；chunk 唯一形态 = buffer；族面 = fs `openread`/`openwrite`（close = tmp+rename 原子提交，未 close 弃 tmp）/ http `ctx.http.stream` / process 句柄（已就位）；close 幂等 + 插件卸载强 close。形态速查：**string = "我保证是文本"（边界 UTF-8 校验背书）；buffer = "一堆字节"（无承诺）**。
- **盒线编码**：**每方法一个琐碎 WIT 导出**（export 名 = 方法名；`list<u8>` 进、`result<list<u8>, list<u8>>` 出）+ 自描述值格式 = **TLV 家族**（tag 字节 + 内联载荷；i64 内联；单 codec crate 双 target 零漂移；字节流带格式版本字段，版本不识 = 解码前响亮失败；位布局实现期定稿、**首盒发布前冻结**）。盒流式 v1 不立（触发器 = 首个流式盒消费者；预定形态 = host imports 回调流）。
- **journal 记录格式草案**（正式设计归任务 6（journal/replay 正式设计））：核心 journal = **核心自己的飞行记录仪**，服务对象三层 = 核心崩溃恢复 / 插件层平台能力 / 部署者审计；四需求按层重分配（fork 退出核心需求集 = 插件层功能）；kind 表去 agent 化。录制粒度 = **★ 族（replay 必需）全录 + audit 族 v1 默认降档**（可配开/聚合摘要；`process_spawn`/`process_exit` 不降档写死）+ 大 body 阈值 blob 引用；生成型秘密 = kind 感知加密按 source 子类分流（`crypto.random` 系加密落盘 / 时钟明文）；payload 落盘 = **JSONL + 格式版本字段 + Int 十进制字符串惯例**（读侧 kind/schema 感知还原）。

### A7 schema DSL

- **载体 = YAML 纯数据**；**记法 = YAML 管结构 + 表达式串管形状**：基元词与字符串字面量 / `array<T>` / `map<T>` / record `{...}` / union `A | B` / `T?`（= `T | nil` 糖）+ 签名 `(参数) -> 返回`；产生式就此冻结（新增叶子/产生式 = ABI rev）；parser ~百行零依赖。**服务方法返回禁含 nil**（"Ok 载荷永不为 nil"的 schema 面原位约束）。
- **词汇表** = `Value` 变体全集直接映射（`nil`/`boolean`/`string`/`integer`/`number`/`array<T>`/`map<T>`/record/union/`T?`/`any`）+ `buffer`/`function` **限 ctx 面** + **类型引用**叶子 `契约名.类型名`（成环/未知 = 响亮）；`bytes` 不立词。
- **契约包** = `lib/<name>/` 三件套（contract.yml + README.md + 可选 init.luau）；`version:` 单整数、破坏性变更才 bump（生态契约必填、平台内嵌禁写）；第三来源 = 插件随包契约（A8 节）。manifest `contract:` 引用 = 裸名（生态引用必带 `@N`）。
- **对账**：实际注册 ≡ manifest `methods:` ⊆ 契约方法集（缺 = 消费方运行期 `method_missing`，"加方法兼容"由此成立；超出 = 启用期响亮）；形状判定 = 类型引用展开 + 规范化四条（record 字段序无关 / union 成员序无关 / `T?` 展开 / 空白无关）后结构相等。
- **运行期校验全量**（声明了 `contract:` 的服务调用）：参数不符 = 调用方响亮报错；返回不符 = `Err(business)` + data = `{ violation }`，违约现场进 host 侧 journal；`errors:` 节不进运行期校验；`any` 槽位免校验（Nil 返回 blanket 检查除外）。
- **生成物单源**：schema → `.d.luau` / `export type` 模块 / 盒 WIT 琐碎签名 / 文档骨架四生成物；方向永远单向（schema → Luau 类型，绝不反向）；生态只接触生成物（Roblox 机器 dump 先例）。**ctx 方法集 = 核心自描述用同一 DSL**。

### A8 事件静态声明

- **manifest `events:` = publish/listen 双纯名单**（扁平事件名）；形状不住名单——形状定义住处唯一 = **契约命名空间**。
- **强制程度 = 意图层声明 + analyze 对账漂移**；运行期不强制名单（事件 = 开放广播语义）。**唯一运行期强制 = 插件 emit `internal/` 前缀 = 响亮报错**（四派发同判）+ 两条关洞：插件目录取名 `internal` = 保留；契约定义 `internal/` 下事件名 = 响亮。
- **契约命名空间三来源逻辑合并**：平台内嵌（核心自带 `lib/`，禁写 version）/ 部署 `lib/` 生态契约（必填 version，引用带 `@N`）/ **插件随包契约** `contract.yml`（name ≡ 插件目录名、禁写 version、无 init.luau；随包进同一审批事务）。命名空间派生**事件形状总表** = emit 校验与类型生成的共同数据源。契约包加 `types:`/`events:` 两节；类型引用可出现在一切表达式串位、**不带版本钉**（版本钉职能由 provides 侧 `@N` 承担）。
- **同名规则**：三来源撞名 = 规范化后结构相等则相安无事、不同 = 响亮；**按契约名引用，永不按插件名引用**（Seam 三角在事件面的延伸，可替换性立身点）。**名字即绑定**：listen 永不写形状（类型生成覆盖）；publish 的名字必须有出处（总表已有或自己随包定义，缺失两分见下）。
- **对账时机 = 启用期**；部署契约索引（含休眠件，纯数据）只管"谁存在"。**缺失两分 = 依赖语义**：来源未启用 → Pending 等待（反向索引唤醒，无超时，warn 政策见 A9 节）；索引查无 → 启用期当场响亮。**冲突判据** = 引用展开 + 规范化后结构相等；`any` 递归相容、**具体胜出**（any 永不覆盖具体定义）。
- **emit 校验跟名字走**（无论发射方是否声明 publish；不符 = emit 方响亮报错，现场进 journal）；listen 面运行期零校验；serial 终止值 / waterfall 链中值 v1 不校验（扩展位）。
- **生命周期**：类型引用在各插件自己的启用事务里解析冻结、两次启用间恒定；已有名字的形状变更 = 破坏 → 反向索引**级联重启用全部引用方**（审批面静态判定变更级别并显示破坏面）；卸载 = 随包契约离开命名空间、悬空引用下次启用响亮；blocking-uninstall 扩展位。
- **不立通配**；引导写法 = 静态名 + 动态部分进 payload（`ctx.emit("job/done", { job_id = ... })`）；审计写实 = 自由态/`any` 事件 payload ★ 族全录可查。

### A9 deps/provide 与能力门槛

- **对账时机统一 = 启用期**：一切跨实体判定在启用期；**安装期 = 纯数据部署索引**（manifest/盒清单与进口清单/随包契约的解析登记，含休眠件；manifest 字段校验 = 唯一留安装期的判定）。解析失败两分：启用对象 = 启用当场响亮；休眠件 = 索引标记损坏、不拖垮启动 + host warn 一条。
- **inject 缺失三分**：已启用未就绪 = Pending 等就绪（无 warn）；已启用但 Failed / 已安装未启用 = Pending 等人为动作 + **warn 提醒**（进 Pending 当场一条、等待中途转 Failed 补一条、每 boot 一次不周期重发）；索引查无 = 启用期响亮。可选依赖不适用（S3 既定不阻塞激活）。
- **provides 对账 = 启用期**：contract 引用缺失两分（来源未启用 = Pending 等启用 + warn；索引查无 = 响亮指路两条路）；`@N` 不符 / 超出契约 / 规范化后结构不同 = 响亮。**provides 撞名 = 启用期响亮，判定作用面 = 已启用 ∪ Pending（含本次启用者）**；休眠件之间撞名不判定。
- **依赖禁环 = 启用期**：required inject 图判环（节点 = 已启用 ∪ Pending ∪ 本次启用者）；**环不给 Pending**（结构性错误无自愈路径），本次启用者响亮失败、报错列出环上全部插件；运行期 wait-for 即时检测原位不动。
- **盒对账 = 启用期**：双向对账（声明没文件 / 有文件未声明 = 响亮）+ 进口对账（盒自描述进口清单 ⊆ manifest 能力声明，超集响亮）；运行期 host imports 授予 = 属主能力包络不变。
- **能力门强制点总表**：fs（两段式路径解析处）/ http（每请求白名单 + 跨域重定向重对账 + SSRF）/ sqlite / process（每调用）/ 盒（imports 包络 + 启用期进口对账）；timer 与纯算内建件与时间/随机读取无门；缺省 fail-closed。**编排面（事件 / provide / inject）不立能力门**——能力间接化（经服务调用间接获得效应）合法且是设计本意（效应由提供方自己的能力包络承担）；**审批面 = capabilities + inject + listen 三栏并示**。

### A10 工具形态与 ABI 版本纪律

- **ABI 边界判据 = 跨边界且写进文档承诺的语义**。三个面：插件↔核心（入口契约 / manifest 与 contract DSL / ctx 方法集与参数返回形状 / `res, err` 两槽与 kind 封闭枚举 / 能力词汇表 / 事件语义）；核心↔盒（TLV 位布局、WIT 琐碎签名形态）；确定性面（调度规则与在飞预算数值）。**不进 ABI**：实现内部；journal 落盘格式（自带格式版本字段）；analyze 启发式规则。
- **`abi_rev` = manifest 单整数、min 语义**（"本插件要求的最低 ABI rev"，缺省 1；核心当前 rev = 编译期常量 N，N ≥ k 放行、N < k 响亮报错指路升级核心）。**bump 纪律**：ABI 面任何 additive 变更（新 kind 格 / 新词汇 / 新可选字段 / 新 ctx 方法）rev +1；核心承诺兼容全部旧 rev（sunset 机制 v1 不立）；未知字段响亮报错保留（管 typo），两者不冲突。纯代码插件无声明位 = 永远按当前核心规则跑。
- **三个版本面各自独立**：ABI rev / TLV 格式版本 / journal 格式版本，互不同步、各 bump 各的。
- **工具面 = `metis sdk` 四子命令**：`new`（脚手架：manifest 骨架 + 开发期 `.luaurc` + 类型挂接说明）/ `check`（纯数据校验，与 loader 启用期同一实现）/ `analyze`（读代码对账 + lint 教具）/ `types`（四生成物 + 每插件 config/deps 类型）。`sdk box *` 族 v1 不立（随 wasm 里程碑：包装 wasm32-wasip2 管线 + wasm-tools 验收）。
- **analyze 前端 = 钉版 `luau-lsp analyze` CLI**（V5 实测：官方 luau-analyze 0.740 CLI 无 definitions 挂接旗标）；分工原则 = 类型语义问生态工具、自有规则自己扫（AST 来源 = 实现期选型）。**检查项总表 14 项三档**（炸×7 / 警告×5 / 提示×2；清单与逐项定级以 doc §A10.3 为 SSOT）。
- **类型生成**：Int 槽 `.d.luau` 直写 `integer`（V1 实测类型器区分成立）；Result 签名 `-> (T?, err?)`——收窄须 `assert(res)` 或 `if res then`（V3 实测两槽关联 narrowing 不成立）；`plugin()` 糖 = 每插件具体类型的非泛型 declare（V6：泛型被用法静默塑形 = 不检查）；definitions 挂接 = luau-lsp CLI `--definitions` / LSP settings 双路径。
- **`.luaurc` 脚手架**：运行期只认部署根一份（A1 节）；sdk 在部署根布线 `@lib` → `./lib/`（alias 值带 `./` 前缀，V4 实测按严格工具为准）；插件仓内 `.luaurc` = 开发期产物不随包分发。

### 演进纪律（冻结面的变更规则）

1. **additive 变更**（新 kind 格 / 新词汇 / 新可选字段 / 新 ctx 方法 / 新产生式叶子 / 新能力词）：随核心发版 + `abi_rev` +1；核心兼容全部旧 rev。
2. **破坏式变更**：新 ADR supersede（点名取代本 ADR 正文格）+ rev bump。
3. **living doc 保持 living**：细则澄清、教学条、实现期登记继续沉淀于 [luau-abi](../design/luau-abi.md)；**凡改语义的修订不允许只走 doc 修订注**，必须按本条 1/2 走。
4. 各章登记的扩展位（`ctx.once` / `ctx.on_cleanup` / `Value::Bytes` 提正 / 盒流式 / sql 事务与命名 db / http 流式上传 / fs 命名根 / 原子方法族其余成员 / per-read timeout / ABI rev sunset / blocking-uninstall / 事件返回形状等）与复审触发器（上游 cycle require RFC 落地 / mlua 上游修写路径等）以 doc 各章联动登记为 SSOT；触发器到来时按本条 1/2 走。

### 对既有 ADR 的局部 supersede（同批吸收）

各旧 ADR 正文冻结不动、Status 行记录本条；以下逐条命名被取代/细化/结案的确切条款：

1. **[ADR-0011](0011-actor-execution-model.md)** Decision 首格「同一时刻一个插件最多一个回调在执行，……串行性绝对」——收窄为：同一瞬间至多一条协程在 CPU 上（协作式）；插件内多条回调可在飞交错，切换仅在挂起点（本 ADR A4 节）。「插件作者写无锁顺序代码」保留（无 lock 原语、两挂起点间顺序）。
2. **[ADR-0018](0018-service-registry.md)** S5 失败表 `Err(Unavailable)` 格的落点——细化：可选依赖常驻缺席 = deps 字段不存在（结构性 nil，不经错误通道）；`Err(Unavailable)` 保留给运行期窗口（提供方崩溃 / 热更竞态 / 卸载窗口，本 ADR A3/A4 节）。
3. **[ADR-0018](0018-service-registry.md)** S5「deadline 即边界、等待环只超时」的依赖环处置——收窄：required 依赖环 = 启用期硬拒（结构性错误，本 ADR A9 节）；运行期 wait-for 即时成环检测与超时保险丝保留（本 ADR A4 节）。
4. **[ADR-0018](0018-service-registry.md)** S3「无任何提供方 → 加载响亮失败」与 S2「重复 key 静态扫描」的时机措辞——统一精确化为「启用期」（对账时机统一 = 启用期，本 ADR A9 节）。
5. **[ADR-0018](0018-service-registry.md)** S6「插件服务 v1 文档约定起步」格——补可选升级通道 = 插件随包契约 `contract.yml`（本 ADR A8 节）；核心服务契约必须 `lib/` 契约包、schema 共享 DSL、演化规则各格不变。
6. **[ADR-0022](0022-value-int64.md)** Decision「过渡纪律」条——结案：vendored Luau 0.740 已出厂整数、写路径 raw push 收口定案（本 ADR A6 节），2⁵³ 过渡映射未启用即退役；该条「ABI 版本纪律归 tooling 议题」已兑现 = `abi_rev`（本 ADR A10 节）。
7. **[ADR-0024](0024-carrier-layering.md)** Consequences 议题地图回填行中「能力政策声明对账 = 盒能力笼的**安装面**」——「安装面」措辞取代为「启用期」（本 ADR A9 节）；同 ADR Consequences 风险登记表 C4「ABI 版本化第一天设计归 A10」已兑现 = 本 ADR A10 节 `abi_rev` 机制。

## Consequences

- Luau ABI 十章自此冻结；全文 rationale、淘汰选项、代码示例、作者契约条（§A4.3 D9.4 六条 / §A5.4 D15.3 五条）与概念地基以 living doc 为准，doc 头部状态行已指向本 ADR。
- 实现期清单（raw push 辅助、coroutine shim、部署索引、启用期对账器、表达式串 parser、四生成器、luau-lsp 钉版、`abi_rev` 比较等）与实现期配置（一切预算/限额/超时数值）不进冻结面——ABI 只冻结形状与语义，数值归实现期。
- journal 记录格式草案（A6 节 A6.7）= 任务 6（journal/replay 正式设计）的头号输入；`session` 字段复审随任务 5（session 模型）；配置树绑定、secrets store 细节、audit 降档开关面归任务 4（配置格式设计）；契约目录发现性与市场元数据归任务 9（marketplace 格式设计）。
- 本次同批更新：ADR-0011/0018/0022/0024 Status 行记录局部 supersede；[luau-abi](../design/luau-abi.md) / [seam-capabilities](../design/seam-capabilities.md)（[ADR-0025](0025-seam-capabilities.md)）/ [service-registry](../design/service-registry.md) 三份 living doc 的状态行与登记行兑现注。
