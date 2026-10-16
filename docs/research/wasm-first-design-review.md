# wasm-first 设计重审：Luau 妥协面盘点与推翻清单（2026-10）

> 调研日期：2026-10-16
> 触发：维护者指示（T13 开工）——「目前的设计有很多地方是为了服务 luau 的，用了 wasm 之后……之前为了 luau 做的妥协或者设计则都可以推翻，我们可以做得更优雅更方便」
> 方法：全套冻结设计文档与相关 ADR 逐节通读（清单 = §8），按 Luau 耦合性质五分类判定：A 消亡 / B 妥协可更优 / C 载体中性不动 / D 新机会 / E 硬张力
> 性质：point-in-time 报告。消费点 = T13-b 站表修订 / T13-c（wit-abi W4 章）/ T13-d（SDK 议题）/ T13-f（措辞批）；维护者拍板后结论固化进 wit-abi 各章与 ADR，本报告修订以 addendum 形式追加，不改写结论
> 关联：[ADR-0034](../decisions/0034-wasm-first-carrier.md)（转向拍板）、[wit-abi](../design/wit-abi.md)（W 系列开章）、[luau-abi](../design/luau-abi.md)（已归档，语义平移源）、[wasm-research](wasm-research.md) 补遗 5/6（证据基线）、台账 T13

---

## 0. 结论速览

**一句话**：为 Luau 服务的设计分两种命运——机制性设计（stdlib 卫生、require 体系、int64 边界机器、coroutine shim、frozen table、Luau 工具链）随载体**整体消亡**；语义性设计（事件/服务/能力/journal/实例/配置的模型与纪律）**原样存活**，但其 Luau 呈现层（两槽返回、tag 严格匹配、表达式串 schema、`plugin()` 糖）可全部换成 WIT 原生形态，且其中三处（能力对账、内建件、错误类型化）wasm 给出的是**结构性更优解**，不是平移。

**推翻面最大的五处**（按收益排序）：

1. **能力对账从启发式静态分析升级为编译期结构事实**（§3 B1）——analyze 管线的 Luau 半边（钉版 luau-lsp、AST 选型、14 项检查中的 6 项）整体不需要了。
2. **纯算内建件族消解进 guest 包生态**（§3 B2）——seam §4 的 json/yaml/regex/diff/uuid/hex/base64/timefmt 八件，Rust guest 用 Cargo 依赖即可，核心收缩到只剩效应能力与 crypto 硬件论据。
3. **A4 挂起/交错章的机制全废、语义平移**（§6 E3）——coroutine shim / SENTINEL / 裸 yield 检测 / 术语分层全消亡；`fanout`/`exclusive` 从 ABI 原语降为 guest 异步设施与 SDK 助手。这正是台账标记的「最高风险章」，分析结论：风险可控，优雅度大升。
4. **stdlib 卫生整章反转为「只进不授」**（§2 A1）——seam §1 全章（白名单建 VM / tombstone / 原位替换 / 硬化顺序 / 四层防线的 stdlib 面）失去对象；非确定性只剩 wasi:clocks/random 两个宿主可控进口。
5. **require 体系全链消亡**（§2 A2）——ADR-0003 纪律、A1.5 官方 require 全集裁剪、.luaurc 布线（D28.5/D29）、lib 反向闭包重启机器；存活物只有契约包的数据职能（转 WIT 包）与契约变更级联。

**需要维护者拍板的硬张力五个**（§6）：E1 递归 Value × WIT 拒绝递归（推荐 TLV 主线化）；E2 服务签名形态（推荐 v1 保单 Value）；E3 每实例投递政策（推荐自由交错平移 + 原语降级）；E4 global scope 与 preopen 不匹配（推荐 private 走 preopen、global 保自研；命名根复活立案）；E5 事件形状 DSL 继承者（推荐瘦身形状 DSL，修正 wit-abi W7 行「A7 消融」的预期）。

**防误推翻**：C 类十项（§4）载体中性，ADR-0034 保留资产明文覆盖，分析复核确认不动。

## 1. 判定框架

每条设计逐项回答两问：这条设计服务的约束来自 Luau 还是来自系统本身？wasm 组件模型下该约束还在吗？

| 类 | 判据 | 处置 |
| --- | --- | --- |
| A 消亡 | 约束是 Luau 自身的（f64 数、table 形态、coroutine、stdlib、require、无包生态） | 机制整体删除，冻结 ADR 走 supersede，living doc 修订注 |
| B 妥协可更优 | 约束是真实的，但当年解法是给 Luau 打补丁；wasm 有原生答案 | 推翻解法、保留问题，按 wasm 原生形态重做 |
| C 不动 | 约束是系统自身的（热更、隔离、审批、审计），与载体无关 | 原样保留，最多换措辞 |
| D 新机会 | 当年因 Luau 明确放弃或不敢立的 | 重新立案评估 |
| E 硬张力 | wasm 也不给免费答案的真设计题 | 议题批出示，维护者拍板 |

## 2. A 类：随载体消亡（机制失去对象）

**A1. seam §1 Luau stdlib 卫生全章。** 白名单建 VM（`Lua::new_with`）、mlua 无条件安装件手工处置、tombstone 族（loadstring/collectgarbage/getfenv/debug.info…）、原位替换族（`os.time`/`math.random`/`print`/`os.date` 薄壳）、coroutine shim、`sandbox(true)` 硬化顺序铁律、§1.5 四层防线的 stdlib 面、§1.6 验证项（数字 locale / 指针身份串 / table hash 遍历序）。wasm 下无 stdlib 可裁——guest 的能力只来自宿主授予的 import，「注入缝」概念从「裁减 + 替换」反转为「只进不授」；时钟/随机 = wasi:clocks/wasi:random，宿主天然可控（测试确定性纪律的注入缝白拿）。锚：seam-capabilities §1 全章、§6.3 实现期登记（mlua 处置清单 / raw push 辅助）。

**A2. require 体系全链。** ADR-0003（require 纪律 + 依赖边第一天记录 + 反向传递闭包失效）、A1.5（官方 require-by-string 全集 + 两处 Metis 裁剪 + 循环报错 + 入口归一）、A3.11（宿主包装实现）、ADR-0019 C3（`@lib` alias + 单根 `.luaurc` + lib 两层）、A10.5 D28.5（部署根布线）、config-format D29 的 `.luaurc` 自动生成行、service-registry §7 的 lib 模块体系。wasm 下插件间代码共享 = 编译期依赖（crates），无运行期模块系统；跨插件 require 不是被禁，是结构上不存在。存活物二件：lib/ 的契约包职能（纯数据 = schema + 文档）转 WIT 包；契约变更级联（D22.6）数据驱动原样成立。注意 ADR-0003/0019 是冻结 ADR，机制死亡须 supersede 处置，不是修订注能覆盖的。

**A3. int64 边界机器全链。** FFlag 进程级旗标（须早于任何 VM 创建）、`push_int64_exact` ~15 行 unsafe 收口、严格 tag 匹配（integer/number 双向响亮）、`123i` 字面量纪律、`integer.*` 算术库、「只传不比算」口径、V1 验证（类型器区分 integer/number）、analyze #11（缺 `i` 后缀流入边界）。WIT `s64` 原生，全链消亡。`Value::Int(i64)` 本体存活（宿主数据货币，ADR-0034 保留资产），但 ADR-0022 的 Luau 对齐论据（RFC #153、原生映射）失效，rationale 换血 = 「YAML/JSON 整数 + WIT s64 双落点」；D7.3「Int 落盘 = 十进制字符串」不动（约束来自 JSON parser 不是 Luau）。锚：luau-abi §A6.1、ADR-0022、ADR-0015 Consequences 末条。

**A4. Luau 呈现层习语全族。** 多返回值 `res, err`（D12.1）+ `assert` 升级习语 + 「Ok 载荷永不为 nil」纪律 + V3 narrowing 实测结论；frozen table 容器（A3.1 递归只读、无元表）+ userdata 淘汰（D5.2）+ 烘焙方法集（A3.3，宿主逐方法生成 Rust 闭包）+「方法名写错 = 字段不存在当场炸」；`plugin()` 糖（A1.1，宿主注入全局函数）+ 入口 table 封闭集 + 裸函数响亮指路；流式三态 `chunk / nil,nil(EOF) / nil,err`（D14.3）；`on_config` 的入口 table 可选字段形态（D42——语义存活：live-apply/重挂载/合并取最新/校验先行，形态转 WIT world 可选 export）。WIT 原生答案：`result<T,E>`、生成绑定编译期查错、world exports、stream 自带 EOF。

**A5. A6.2 table 转换纪律的 Luau 半边。** Array/Map 严格二分（键恰为 1..n 连续整数判定）、元表拒绝（`__index` 默认字段快照抓不到）、混合表/稀疏表/非字符串键响亮、循环引用检测、NaN 键与整值浮点键归一化。WIT canonical ABI：`list<T>` 与 record/tuple 天生分词，值是树无环，无元表。存活的是深快照原则的理由 1/3（跨实例隔离 + journal 要求宿主持有自有数据），但实现变为 canonical ABI lift 或 TLV 解码，不再是「递归读 table」。锚：luau-abi §A6.2。

**A6. A6.4 buffer 体系。** Luau `buffer` 法定二进制形态、`readbytes`/`writebytes` 分立方法、`fs.invalid_utf8` kind、「`buffer` 词限 ctx 面」（D17.1）、「string = 我保证是文本 / buffer = 一堆字节」速查。WIT `list<u8>` 原生；`string` 由 canonical ABI 保证 UTF-8——UTF-8 裂缝按构造不存在。方法面 text/bytes 切割可作为 API 设计选择保留，但性质从「语言缺陷补偿」变为普通接口设计。

**A7. A4.3 全节与 A4 的 Luau 机制面。** coroutine shim（`coroutine.resume`/`wrap` 包装）、SENTINEL userdata、挂起冒泡链、`outstanding[co]` 防假注入、D9.3 被登记线程裸 yield 响亮杀、线程/协程术语分层、OS 映射教学条、D9.4 作者契约六条的协程条款。S1 spike 实证 guest 内 `futures::join!` 原生并发（补遗 6 V2）。A4 其余 Luau 机制：D1.6 调度规则纯函数化冻结进 ABI（replay 已死 = journal D75，且 guest 内调度器宿主不可冻结）、D27.1 ABI 第三面「确定性面」随之缩没；D8.1 单队列 FIFO 的宿主侧价值重估归 W4（E3）。语义资产（协作式心智、挂起点间原子、同步全显式）平移，见 E3。

**A8. A10 的 Luau 工具面。** 钉版 `luau-lsp analyze` CLI（D28.2）、自有规则 AST 来源选型、`.d.luau` 与 `export type` 模块生成（D19.2①/D28.4）、definitions 挂接（V5）、`.luaurc` 脚手架（D28.5）、`plugin()` 泛型 declare（V6）、两槽 narrowing 的类型定型（V3）、D28.3 检查项中的 Luau 专属六项（#1 `ctx.<ns>` 扫描、#2 scope 键↔命名空间、#9 err 槽裸丢弃、#10 kind 串枚举、#11 边界转换违规、#14 入口形状）。消亡规模 = `metis sdk` 的一半职责；存活面见 §7 T13-b 修订建议。

**A9. 盒作为独立范畴。** manifest `boxes` 字段与双向对账（D25.5①）、`ctx.box(name)` 访问器与盒句柄烘焙（A3.12）、「盒能力 = 属主包络」、盒流式扩展位、「等的事重叠 = fanout；算的事并行 = wasm 盒」教义——插件即组件后，盒概念消解进插件本体（ADR-0034 §2 已定）。存活晋升：盒的进口对账（D25.5②）升格为全插件通用机制（B1）；A6.6 的 TLV 琐碎签名设计从盒面晋升主线候选（E1①）。

**A10. 各文档散见 Luau 行。** context-events §0.1「一个 `lua_State`」行、§0.3 worker 伪码（`pcall(...)` / `run_luau_disposers()` / `drop(lua_state)`）、§1「禁跨插件 require 使直接引用结构上不可能」论据行、§3「与 Luau 类型一一对应」行；fiber.md §6 错误遏制表 Luau 行（pcall 语义）→ wasm trap 行、§9「禁 tokio/mlua」→ 禁 wasmtime、§10 修订注指针；service-registry §3「Luau 无静态分析工具链……故方向只能 manifest-first」论据行（结论由审批/激活门独立支撑，且 wasm 下被结构化强化）；config-format D35 布局行（`main.luau` / `boxes/`）、D42 的 Luau 示例；instance-model §0.2「每插件独立 `lua_State`」机制输入行、D65「独立 `lua_State` 天然成立」。T13-f 现登记范围（seam/context-events/instance-model）不覆盖 fiber/config-format/service-registry，需扩充（§7）。

## 3. B 类：Luau 妥协 → wasm 下可做得更优雅

**B1. 能力对账：启发式静态分析 → 编译期结构事实。** Luau 世界的能力门强制 = 运行期出现规则兜底（未声明调用响亮指路 manifest）+ analyze AST 扫描对账（D28.3 #1，炸级）——「声明 ≠ 观察」（A2.2）需要两层机器去维持，且扫描是启发式、有误报有漏报。wasm 世界：组件 import 清单可机械抠出（ADR-0024 §6 防线 1 既定），SDK 固定 world（作者无法越权加 import，ADR-0034 §4）→ 能力对账 = 集合包含判定，启用期一次，零启发式、不可博弈。「能力漂移」从 lint 题变类型题；D28.3 #1/#2/#3 随之消亡（A8）。这是 ADR-0034 §7 五道防线扩展为全插件后最大的免费红利，也是 T10.9 analyze 站缩编的主依据。

**B2. 纯算内建件族消解进 guest 包生态。** seam §4 的 json/yaml/regex/diff/uuid/hex/base64/timefmt 八件由宿主提供，根本理由 = Luau 无包生态 + 解释器慢 + 要 sync 直挂保高频（§2.6 成本锚点）。Rust guest 下这些是 `Cargo.toml` 一行依赖（serde_json/regex/uuid/time）。核心收缩后只剩：效应能力（fs/http/sql/timer/process——能力门与 journal 的挂载位，必须宿主）、crypto（AES/SHA 指令缺口的硬件论据存活，ADR-0024 §4 物理约束行）、WASI 标准件直接复用（clocks/random/preopen）。连带消解：seam §2.1 判据 2 的 sync 白名单机器大半、§2.5 灰区、§2.7 时间盒缺口纪律的主要选民、§3.2 约束②批量友好（值转换 ~300ns/entry 是 Luau 边界税）、ADR-0024 §4 四判据的主战场与毕业路径的纯算侧。SDK 可提供便利 re-export（metis-plugin 的 feature），那是包形态题，不进核心机制。

**B3. setup 死锁红线结构消解。** A1.4 的死锁红线（setup 里只能等「完成不依赖自身邮箱」的调用；sleep 必须做成宿主直接唤醒）是 actor 邮箱 + Luau coroutine 的推论。import 效应模型下，一切挂起 = 等宿主 import 完成，完成路径天然不经过自身邮箱——整类死锁形状消失；sleep 红线自动满足（timer import 完成于宿主侧）。残余圈 = 跨插件服务等待环，由既有四层处置覆盖（D1.7：启用期禁环 + lint + wait-for 检测 + 超时）。激活门（Loading 窗口不投递）原样保留。

**B4. fs 能力面借 WASI preopen 结构性收口。** A3.7 的两段式路径解析（词法归一 + 物理 canonicalize + openat2 TOCTOU 硬化）是 Luau 无 capability 文件系统时的自研边界。wasi:filesystem 的 preopen 模型：宿主授予预开目录，guest 在 preopen 内用 `std::fs` 常规 API，逃逸防护 = 运行时结构保证——private scope（`plugins/<name>/data/fs/`）可完整卸载给 preopen，自研解析机器对其退役。global scope 是减法语义（用户世界 − 两条豁免），preopen 表达不了减法 → 保自研（或走 host-mediated import）。附带的命名根复活议题见 E4。

**B5. 错误封套呈现层类型化。** err 封套 `{kind, message, data?}`、封闭 kind 枚举、「禁 message 匹配做控制流」、三分界判据、族清单与兜底格——语义全部存活（是 A5 的设计精华），形态升级为 WIT 错误类型（variant/record）：「kind 定控制流」从纪律变结构，analyze #10（kind 串必须在枚举内）由编译器白拿。错误通道 carve-out（D15.2：`error(v)` 任意值不可转换时落字符串摘要）消亡——WIT `result` 的 E 类型固定，不存在不可转换的错误值。服务四态（unavailable/method_missing/timeout/business）平移为错误 variant 四格，business data 走 `data` 字段（Value）。

**B6. Value::Bytes 提正成本翻转。** 旧世界提正税 = Luau 侧 buffer 映射 + 类型词 + 编码纪律，故立「首个跨插件二进制消费者」触发器（D4.1）。WIT `list<u8>` 原生，跨插件二进制与文本同权。建议 W6 章直接重裁，不等触发器——触发器是按 Luau 成本结构设的，成本结构已变。

**B7. 实例与 mount 机制顺风增强。** D65 快照三样的「代码版本」= 组件产物 hash，实例化 1.7 µs（ADR-0024 §8 spike）→ mount 消耗预算担忧降级；D57 租户 boot-only 的「重启秒级」更稳；D65「不抢夺运行中 Agent 的工具」语义由「实例即弃 + 旧实例照跑」天然兑现。非推翻，原设计白拿强化。

**B8. 「薄编排」哲学反转。** seam §0 设计目标「使 Luau 保持薄编排（几十行）」与「重活不在 Luau 逐字节循环」（36–125× 实测）的整套话术，反转为「插件内计算即原生速度」。ADR-0024 五层定妆已被 ADR-0034 §2 收敛为单层；seam §3 性能议程重写：「性能 = 编排并发度」存活，「并发槽单价」的账从 VM/盒两套（VA/RSS 容量账）收敛为组件实例一套。插件内 CPU 密集合法化；并行的答案 = 同码多实例并跑或外部服务。

**B9. 即写即用体验地板实测够格。** Luau 的「文本即插件」是 Creator 唯一通道的硬承诺，wasm 转向的最大体验疑虑。S1 spike 实测：增量重建 0.22–0.29 s + 实例化 1.7 µs（补遗 6 V1 + ADR-0024 §8）——`metis dev` watch（保存 → 构建 → 热换）的体验地板意外地好，ADR-0034 §6 方向的可行性证据已足。薄插件仪式感地板（W2 风险：文件 + 构建命令 + 秒级）由 SDK 脚手架与 `metis dev` 承接，归 T13-d/e。

## 4. C 类：载体中性，不动（防误推翻清单）

以下各项经逐条复核，约束来自系统本身（热更、隔离、审批、审计），ADR-0034 Context 保留资产明文覆盖——**不动**，至多随 T13-f 换措辞：

| # | 资产 | 一句复核 |
| --- | --- | --- |
| C1 | fiber 全家（状态机 / 账本 LIFO / disposer 统一异步签名 / epoch 指纹 / 反向索引 / 子 fiber = 父 effect / 激活门） | 生命周期语义与载体无关；wasm 实例即弃反而让 drain 更便宜 |
| C2 | actor 星型拓扑 + 邮箱 + 四派发 + fail-open 错误遏制 | 组合语义；VM 时间盒换 fuel/epoch 原生件（ADR-0024 §8 实测 1.022×） |
| C3 | manifest 双声明 SSOT + Pending/Failed 显式状态 + 缺失三分 + 启用期判环 | 审批与激活门的结构前提；wasm 下 manifest-first 被 import 对账强化（B1） |
| C4 | Value 数据大一统（config/payload/journal 同型）+ 严格制哲学 | 对齐轴从「与 Luau 一一对应」换成「YAML/JSON + WIT canonical 双落点」；模型本体不动 |
| C5 | 实例模型全家（租户 / mount / 快照 / 实例作用域 / parent 链） | 见 B7 顺风增强 |
| C6 | 配置格式全家（`config.yml` / overlay / secrets / 两档卸载 / diff 协调 / on_config 语义） | 呈现形态转 WIT export（A4），语义不动 |
| C7 | journal 骨架 + D7.3 格式纪律（JSONL + Int 字符串惯例 + 格式版本） | 约束来自 JSON parser 与审计哲学；VM 族拦截点改指 wasm 操作（ADR-0034 已登记） |
| C8 | 出区四门 + 能力门 fail-closed + 两层授权分工（A2.6）+ 能力间接化合法（D26.2） | 信任模型的骨架；wasm 下四门映射 = wasi 件 + host import 自研件（E4） |
| C9 | 时间戳毫秒纪律 | ADR-0022 保留：纳秒连 i64 也撑不过 2262 年 |
| C10 | ADR-0008 审批管线（暂存 → 校验 → 审批 → 原子提交） | Creator 权重降但管线载体中性；不可读制品时代审批面更重要；校验步内容换（组件 validate + import 对账）；地位复议题 T13-f 已登记 |

## 5. D 类：wasm 白送的新机会（当年明确放弃或不敢立的）

**D1. 拦截完备性。** 盒时代的「IO 全走 host imports = 天然全量可拦截」（ADR-0024 调研结论）现在对**一切插件**成立——journal 可配审计层扩展位（J2 预制方案）重开时拦截点均匀无特例；replay 扩展位（D92）重开时 wasmtime 确定性工具箱（fuel/wasi-virt/NaN 规范化）白拿。登记原位，不进 v1。

**D2. 隔离与经济的原生兑现。** ADR-0005 遏制四约束（独立实例 / 内存限额 / 指令中断 / 卸载零残留）从「自研达到」变「原生白拿」：Store+Instance per plugin、pooling 实例化 1.7 µs、限额稳态零开销、fuel 1.022×（ADR-0024 §8 实测）。ADR-0005 决策本体不动，Consequences 的成本账换血（每实例几十~一百 KB 的 Luau 账 → VA = 槽×(预留+guard)、RSS ≤ 槽×max 的组件账）。

**D3. SDK 编译器即第一道对账。** `#[metis::plugin]` 宏 + trait 实现形态（ADR-0034 §4）让「写错方法名 / 传错参数形状 / 漏处理错误 variant」在 `cargo check` 即炸——Luau 时代需要钉版 luau-lsp + 自扫 AST + 类型生成挂接三层机器才勉强够到的事。先例：proxy-wasm SDK / Zed `zed_extension_api` / Spin / Extism PDK。

**D4. marketplace 结构校验大门的语言无关兑现。** 组件自描述进口清单 + 导出对 schema + 禁用特性检查（ADR-0024 §6 防线 3）原设计只对盒，现在全插件适用；与确定性构建信任链（ADR-0034 §6 远端构建场）同管线。

## 6. E 类：硬张力与待拍板议题

**E1. 递归 Value × WIT 拒绝递归（W6/W7 共同地基，须最先拍）。** 事实面：服务参数与事件 payload = `Value`（递归树）；WIT 拒绝一切递归形态（V2 实测五种全拒，wasm-tools 1.261.0，补遗 4）——「WIT 原生富类型直接描述 Value」在数学上不可能。三条路：

1. **（推荐）TLV 琐碎签名全主线化**：A6.6 已拍板设计（`list<u8>` 进、`result<list<u8>, list<u8>>` 出、每方法一导出、单 codec crate 双 target、格式版本字段）从盒面晋升为主线边界设计；Value 的线编码 = TLV，契约形状描述留配套 schema 面（E5），运行期校验（D19.1）保留在宿主两跳中点。优点：已拍板、已验证、演进权自控；代价：WIT 层看不见业务类型。
2. 两层制：非 Value 服务走富 WIT 类型（端到端类型安全），Value 服务走 TLV。表达力最好，代价 = 两套心智与两套对账机器。
3. 有界深度 WIT value variant：引入深度预算新概念，预算数值永远拍脑袋，不推荐。

**E2. 服务签名形态（S1 复审）。** S1「args/返回 = 单 Value」的动机 = 跨边界一次转换 + schema 单挂点 + Luau table 自然映射。wasm 下可设想服务 = 富类型 WIT interface（MethodMissing 与运行期 shape 校验消解进绑定层），但：宿主动态路由须按契约动态 lift/lower 任意 WIT 值——wasmtime `component::Val` 路线**未验证**（确认方法 = W4/W6 开工前小 spike）；数据大一统轴削弱（config 仍 Value，服务参数类型化）；E1 若选①则服务面本就走 TLV。选项：1. **（推荐）v1 保单 Value + E1①**，富类型服务留 ABI 演进位不封死——S1 的「一次转换、单挂点」论据在宿主动态路由架构下依然成立，富类型的主收益（编译期对账）已被 SDK 宏 + 契约对账覆盖大半；2. 富类型优先，接受动态 Val spike 与大一统削弱。

**E3. 每实例投递政策与 A4 语义平移（W4 章核心）。** 技术事实：guest 内并发（补遗 6 V2，`futures::join!` 500/501 ms）与单宿主线程多实例并发在飞（V3，502 ms）已验证；**同实例重入**（第二条消息在首条 handler 挂起期间投递进同一实例）S1 未显式验证 = W4 开工 spike 项（宿主侧需 `concurrency_support(true)`，补遗 6 papercut 3）。选项：1. **（推荐）自由交错语义平移**（与 A4.1 作者心智兼容：协作式、切换仅在挂起点、两挂起点间原子），但原语降级——`fanout` = guest 异步运行时原生（`join!`），`exclusive` = SDK 助手（async mutex；可选上报宿主做 wait-for 可见性，上报是仪表不是机制）；死锁兜底 = 跨插件 wait-for（宿主全见）+ 超时；2. 实例级串行化（run-to-completion，挂起不投递新消息）——最简单但回退到 A4.1 枪毙过的头阻问题（30 秒 LLM 调用卡全插件邮箱）；3. 声明式（按 handler/事件名标注重入许可）——灵活但引入新声明面。D1.2 三条结构规则（不嵌套 / 自家人 / 强放）随 guest 侧实现重估，部分可由 SDK 类型系统表达；D10.2 在飞预算概念存活（背压面 = 每实例在飞 async 任务数）。

**E4. global scope 与 preopen 的不匹配，及命名根复活议题。** 推荐：**private 走 preopen**（结构性收口，B4），**global 保自研**两段式解析（减法语义 preopen 表达不了），http/timer/process/sqlite 同留 host import 自研面（SSRF/白名单/预算政策需要宿主中介；wasi:http 的 proxy 形态与本项目政策粒度不匹配，W-能力章评估）。另立案：config-format D44 当年砍掉的 workspace 绑定，在 preopen 模型下「部署者显式绑定目录」是一等原生概念——命名根扩展位（§A3.7 登记）可按 WASI 原生形态复活评估，归 W-能力章，本报告不预决。

**E5. 事件形状 DSL 继承者（修正 wit-abi W7 行预期）。** 事件名 = 动态字符串，永远进不了 WIT world 静态面；emit/on = import（string name + TLV payload）。契约命名空间三来源存活，但 `contract.yml` 的表达式串消亡后，**Value 形状描述失去住处**（服务签名面随 E1① 同样有此需求）。选项：1. **（推荐）瘦身形状 DSL 继承**：契约包保留 `types:`/`events:` 节，记法在 A7 表达式串基础上裁掉 config/服务签名面（那些转 WIT），只服务 Value 形状——A7 是半消融而非全消融；2. 事件形状也用非递归 WIT 近似 + TLV 承载——形状表达力受损（`any`/递归不可写），不推荐。wit-abi 目录 W7 行「A7 schema DSL 消融」的措辞预期需按拍板结果修订。

## 7. 对 T13 剩余工作清单的修订建议

**T13-b（站表修订批）新增输入**：

- T10.3（schema DSL → metis-loader）重定义：YAML parser 选型（存活）+ manifest/config schema（存活）+ WIT 包加载与契约命名空间合并（新，wit-parser 系）+ Value 形状 DSL 继承者（挂 E5）+ 反向索引与级联（存活）；表达式串 parser / 规范化比较器 / 运行期 walk 校验器的存亡随 E1 拍板。
- T10.9（diff/reload + 部署索引）：静态面存活（树 diff / 索引 / 判环预检 #8 / emit 对账 #4–#7 随 E5）；luau-lsp 钉版 / AST 选型 / 四生成器（A8）全撤，换 WIT 工具链（wit-bindgen / wasm-tools 验收）；`abi_rev` 比较 → WIT package version 比较。
- T10.10（metis-luau 转换层）：crate 删除或改向 `metis-component`（wasmtime 嵌入：Store/Instance 管理、实例即弃池化、fuel/epoch、host imports 注册）；原锚点（push_int64_exact / shim spike / mlua 处置件）全失效。
- T10.11（能力面）：语义锚点存活（seam 能力集 + 预算 + TOCTOU 硬化）；ctx 暴露形态 = WIT imports 宿主实现 + wasi 标准件挂接（preopen/clocks/random）；流式句柄族（A6.5）按 WASI 0.3 `stream<T>` 重做；UTF-8 校验与 buffer 面消亡（A6）。
- 新增 SDK 站输入增补：`exclusive`/`fanout` 的 SDK 助手形态（E3）、内建件便利 re-export（B2）、wasip2 std 拖带 14 个导入的 world 收敛（补遗 6 papercut 1）。

**T13-f（措辞修订批）范围扩充**：现登记 = seam-capabilities / context-events / instance-model + ADR-0008 复议。分析显示应扩为两半：

- f1（living doc 修订注批）：seam-capabilities（§1 全章消亡注 + §2/§3/§4 收缩）、context-events（§0.1/§0.3/§1/§3 各行）、instance-model（§0.2/D65）、fiber（§6 Luau 行 / §9 mlua 措辞）、config-format（D29 `.luaurc` 行 / D35 布局行 / D42 示例）、service-registry（§3 论据行 / §7 lib 体系）。
- f2（冻结 ADR 处置批）：ADR-0003（require 纪律机制死亡 = supersede）、ADR-0019 C3（lib 模块体系死亡 = supersede 该格）、ADR-0015/0022（Luau 对齐论据换血 = 批次注记）、ADR-0005（Consequences 成本账换血）、ADR-0011（伪码 pcall 与时间盒措辞）；可与 W 章设计批合并落地，不必独立成批。

**wit-abi 目录行**：W7 行「A7 schema DSL 消融」按 E5 拍板结果修订预期；W4 行开工输入 += E3 选项批与同实例重入 spike 项。

## 8. 通读清单（验证范围）

本报告全部结论以以下文档逐节通读为据（未通读者不出结论）：docs/backlog.md（T13 块）；docs/decisions/ 0003 / 0004 / 0005 / 0007 / 0008 / 0010 / 0011 / 0015 / 0019 / 0022 / 0024 / 0034；docs/design/ luau-abi（全文 1444 行）/ wit-abi / fiber / context-events / seam-capabilities / service-registry / config-format / instance-model / journal；docs/research/wasm-research.md 补遗 6。未通读：ADR-0001/0002/0006/0009/0012/0013/0014/0016/0017/0018/0020/0021/0023/0025–0033 正文（经设计文档转引覆盖其要点；其中 0013/0014/0016/0017/0018 的机制本体已在 context-events 与 service-registry 通读面内）。wasm-research 补遗 1–5 仅按转引使用，未逐节重读。
