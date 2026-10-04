# WASM 能力曲线深度调研：盒层载体判决（2026-10）

> 调研日期：2026-10-04（全部网络核查均此日访问）
> 触发：用户指示——"wasm 值得一轮重度的调研，深入研究其能力曲线：发展到哪一步、有哪些能力、哪些能补全我们的遗憾、性能如何、灵活性如何"
> 方法：三路并行调研（运行时与 WASI 现状 / 语言矩阵与性能曲线 / 生产案例与工程教训）+ 本报告合成 Metis 映射
> 证据图例：🔍 = 网络核查（URL 见附录）；🧠 = 模型知识，未核实；💭 = 推断；📢 = 官方口径（区别于独立证据）
> 网络受限记录：fastly.com 博客直连被拒、fermyon.com 全站 301 至 akamai.com（WAF 403，重定向本身用作证据）、shopify.engineering 旧文 404、Roblox NCG 博客 404——相关数字已降级标注
> 关联：[ADR-0021](../decisions/0021-extension-language-runtime.md)（wasm 行局部复审的输入）、盒管模型（插件架构分层框架：管 = Luau 编排层、盒 = native 能力模块、固定盒 = 核心内建件）、journal/replay 正式设计、marketplace 格式设计（后两者为后续任务队列条目）。文中"接缝能力集" = syscall 原语清单议题

---

## 0. 结论速览

**判决建议：wasm 盒入局**——以"可换的 native 盒"身份进入盒管模型（人类开发者编译产出、进程内、实例即弃热换、IO 全走 host imports），附 §5.9 的条件清单。这与 ADR-0021 淘汰表 wasm 行不冲突：当初判的是"wasm 当**通用插件语言**"（Creator 模式无工具链可用，判决在该语境下仍然成立）；判决理由的前半句"限制多、上限低"也依然成立——§3.6 正是其现行证据，但盒层定位不依赖墙外能力（墙即设计），故与盒层载体身份不冲突。局部 supersede 改写的是该行的**适用范围**，而非宣称判决理由失效；归 ADR-0024。

**对用户五问的一页回答**：

| 问 | 答 |
| --- | --- |
| 发展到哪一步了 | 组件模型 + WASI 0.2 已稳定（2024-01）；**WASI 0.3 于 2026-06-11 正式发布**（原生 async 进 canonical ABI），wasmtime 46+ 默认开启（当前 v49）；工具链正典清晰（Rust `wasm32-wasip2` 直出组件；cargo-component 已弃用）；GC/EH/memory64/tail-call 已完工进 Wasm 3.0。**不是早期技术，是刚进入可押注期的成熟点** 🔍 |
| 有哪些能力 | 近原生计算（稳态 0.5–0.8× 原生）；**遏制四件套均有运行时原生机制**（独立实例/内存限额/指令中断/卸载清零，见 §2——官方文档口径，spike 复核项见 §5.10）；显式能力注入（WASI 每接口显式授予，与我们 E4 同构）；实例即弃经济成立（µs 级实例化）🔍 |
| 性能如何 | vs 原生平均慢 45–55%（SPEC CPU，USENIX ATC 2019，浏览器引擎时代基准，见 §3.1）；vs Luau 快 **5–20×**（💭 合成推断，**待 spike 实测**）；跨界线 = 单次任务 > 数 ms 进盒净赚；动态语言盒（JS/Python）是兼容层非性能层 🔍 |
| 灵活性如何 | **天花板在边界不在算力**：零拷贝共享、GPU、AES/SHA 指令直通（缺失，加解密差 3–10×）、裸协议栈、多线程（与限额冲突须禁用）都在墙外。判词：能做到"**近原生的安全计算**"，做不到"近硬件的系统编程"——而这堵墙与我们的遏制哲学同向 🔍 |
| 哪些能补我们的遗憾 | "可换的 native 盒"物理上成立（§2/§3）；replay 完整（§5.7）；插件作者的语言自由在静态语言梯队兑现（§5.1）；生态活力的真相与 Creator 定位（§4.3）→ 完整映射见 §5 |

**一页关键数字**：

| 项 | 值 | 来源 |
| --- | --- | --- |
| wasmtime | v49.0.2（2026-10-02），月度列车，MSRV Rust 1.96 | 🔍 releases |
| WASI | 0.2.12 stable；**0.3.0（2026-06-11）**、0.3.1（2026-08-11） | 🔍 wasi.dev |
| 实例化成本 | 数 MB 大模块 ~2 ms → 池化后 **~5 µs**（400×） | 🔍 BA 官方博客 2022 |
| 中断开销 | epoch ≈ 10% 减速；fuel ≈ epoch 的 2 倍（但确定性强） | 🔍 wasmtime docs |
| 内存兜底 | 默认 2 GiB guard region（虚拟预留，物理按需） | 🔍 wasmtime security |
| wasm vs 原生 | 平均 0.5–0.8×（峰值 2.5×）；显式边界检查再砍 1.2–1.8× | 🔍 ATC'19 / wasmtime docs |
| 动态语言盒体积 | Javy ≥869KB；SpiderMonkey ~8MB；Python ~25–35MB | 🔍 READMEs（Python 🧠） |
| wizer 快照 | JS 冷启动 ~5 ms → **0.36 ms**（13×） | 🔍 Lin Clark 文 |

---

## 1. 技术与标准现状（发展到哪一步了）

### 1.1 运行时

- **wasmtime（主推）**：v49.0.2（2026-10-02）；Bytecode Alliance 非营利基金会治理（RFC 流程成文、fuzzing 常设、安全修复同日回补多版本线——2026-10-02 同发 v49.0.2/v48.0.5/v36.0.17 修 8 项 GHSA）。后端：Cranelift（默认优化）/ Winch（基线快编）/ Pulley（可移植解释器）；支持 AOT 预编译 `.cwasm`（编译移出关键路径）。🔍
- **安全姿态注意**：2026-09/10 补丁含三起 **fuel 计量绕过**与两起宿主内存耗尽（GHSA 公告载于 v49.0.1/v49.0.2 release notes）——计量机制被持续攻击面审计，跟进月度列车是硬义务。🔍
- **比照**：wasmer v7.5.0（活跃，但组件模型落后、主推自家 WASIX = 生态分叉信号；指令中断 2026 年才实验性引入）；wazero 纯 Go 出局（语言不匹配）。**wasmtime 是唯一能同时给出组件模型 + WASI 0.3 + fuel/epoch + pooling allocator + 官方插件教程的候选。** 🔍

### 1.2 组件模型与工具链正典

- **WIT**：接口描述语言（`package ns:name@version` + interface + world）；组件**自描述**（二进制内嵌 WIT，`wasm-tools component wit` 可内省）；`@since/@unstable/@deprecated` 门禁机制齐全。🔍
- **canonical ABI**：钉死跨语言类型语义（record/variant/list/string/result……0.3.1 起 `map<K,V>`）；跨边界按值拷贝，**资源（`resource`/`own`/`borrow`）按不透明句柄引用——资源天生是能力**。🔍
- **工具链正典（2026-10）**：`wasm32-wasip2` 是 rustup 一等 target，`cargo build --target wasm32-wasip2` **直接产出组件**；`wit_bindgen::generate!`（0.62.0，累计 2.68 亿下载）；wasm-tools 1.261.0；**cargo-component 官方弃用中**（"native tooling can be used directly"）。🔍
- **版本化纪律红线**：wasmtime 与 wit-bindgen 必须钉同一 WIT 版本，错配在实例化时报 `wrong type`（官方自认 "confusing"）——**Metis 的 WIT 包应作为 SSOT 入库、版本随 ADR 演进**（💭）。🔍

### 1.3 WASI 能力面

- **0.2（stable，0.2.12）**：io / clocks（wall+monotonic）/ random（安全+非安全）/ **filesystem（preopen 目录能力模型）** / sockets（TCP/UDP/DNS）/ cli / http（incoming+outgoing）。认证实现 = wasmtime + jco。**能力面判词：已覆盖盒所需的全部确定性系统能力，且每个接口都是显式授予——与 Metis 能力注入模型逐条同构。** 🔍
- **0.3（2026-06-11 正式发布，0.3.1 于 2026-08-11）**：异步从 wasi:io **下沉进 canonical ABI**——`async func`/`stream<T>`/`future<T>` 成原生类型，解决 0.2 的"三明治问题"（跨实例无法转发唤醒）；sockets 7 接口并为 2、http 资源 9 种并为 2。**wasmtime 46+ 默认开启**，0.2/0.3 组件按 world 自动分流。🔍
- **缺口**：wasi:keyvalue / wasi:logging 未进主仓 proposals（🧠）——**需要 kv/日志短期走自定义 WIT 接口，不等标准**；wasi-tls 仍 Phase 1（wasmtime 44+ 有 draft 实现）。🔍

### 1.4 提案地图（与插件场景相关的）

| 提案 | 状态 | 意义 |
| --- | --- | --- |
| GC / tail-call / multi-memory | 提案完工（WG 投票 2024-07-10，随 Wasm 3.0 发布） | GC 语言（OCaml/Dart 等）可写盒 |
| EH / memory64 | 提案完工（WG 投票 2025-07-23，随 Wasm 3.0 发布） | 零成本异常；>4GiB 盒可行（生态薄） |
| SIMD / relaxed-SIMD | 完工 | 数值盒性能；relaxed 有确定性模式 |
| 组件模型 async | 随 0.3 落地，wasmtime 默认开 | 盒的 IO 组合正路 |
| core threads + shared memory | Phase 4，但 **wasmtime 仅 Tier 2：与 Store 限额、pooling allocator 明确不兼容** | **必须禁用**——一共享内存，限额与池化同时破功 |
| stack switching | Phase 3（仅 x86_64 Linux） | 远期宿主-来宾协程切换 |

（全部 🔍 github.com/WebAssembly/proposals + docs.wasmtime.dev 提案矩阵）

---

## 2. 遏制审计：四条冻结约束逐条核对

这是本调研的第一个核心问题：**wasmtime 49 上四条冻结约束是否全部有原生支撑？答案：四条均有原生机制支撑，无需自研运行时补丁**——注意证据为官方文档口径（🔍📢），成本与边界如下表，spike 复核项列 §5.10。

| 冻结约束 | 裁决 | 机制 | 成本与限制 |
| --- | --- | --- | --- |
| ① 每插件独立实例 | ✅ | 每插件一 `Store`+`Instance`，`Engine` 全局共享编译产物（官方插件教程标准模式；Store 间内存隔离是规范级保证） | 宿主簿记 KB 级残差（🧠）；编译按 Engine 摊销 |
| ② 内存限额 | ✅ 双层 | 静态 max memory + pooling allocator 槽位上限（总盘）+ `Store::limiter`/`ResourceLimiter`（个体，`memory_growing` 回调可拒可 trap） | **不覆盖宿主簿记与 shared memory**（后者禁用即堵）；虚拟地址 GiB 级预留 ≠ 物理占用，需容量规划 |
| ③ 指令级中断 | ✅ 双通道 | **epoch**（外部线程驱动，~10% 减速，毫秒粒度——中断约束的默认解）；**fuel**（按指令计费，官方称**完全确定**：同程序同油量同断点）；均可 trap 或 async-yield | fuel ≈ epoch 的 2 倍耗时；fuel 的确定性声明与其 2026 年三起绕过 CVE 存在张力，按折让对待——仅确定性场景启用，spike 实测后定深模式 |
| ④ 卸载零残留 | ✅（运行时职责内） | drop Store → 线性内存/表/实例上下文**三类清零**（官方安全文档原文）+ 池槽 `madvise` 复位 + 句柄表回收；`own` 资源 LIFO 析构 | 清零是 "where it can" 的纵深防御非形式化承诺；**外部副作用（写过的文件、开过的 socket）回收是宿主职责——恰好一切 IO 走 host imports，宿主天然握有全量清单** |

（以上全部 🔍📢 docs.wasmtime.dev security / examples-interrupting-wasm / examples-fast-instantiation / ResourceLimiter 文档；BA 官方博客 2022-09-06——官方口径，spike 复核列 §5.10）

**实例即弃的经济性**：pooling allocator + CoW 堆镜像 + `InstancePre` 三件套，大模块实例化 ~2 ms → ~5 µs；每次实例化实际只写几 KB 内存。**"实例即弃"在 wasm 里是经济上成立的模式**——与遏制哲学同向。🔍

**残余风险三点**：fuel CVE 跟进（月度列车使跟进成本低，但必须跟）；shared-memory 类特性配置禁用；WIT 版本对齐纪律。

**并发路线被证据锁定**：禁用 shared-memory threads；并发 = **多实例 + 组件模型 async（默认开）+ 宿主调度**——与 actor 模型同构。🔍

---

## 3. 能力曲线：性能与灵活性

### 3.1 三轴总览

- **体积轴（跨四个数量级）**：AssemblyScript/Zig 数 KB → TinyGo ~10KB → Rust 数百 KB → Go 官方 ≥2MB → JS-SpiderMonkey ~8MB → Python ~25–35MB 🔍
- **启动轴**：预编译（`.cwasm`）+ 池化 + CoW + `InstancePre` 把静态语言盒压到 **µs 级**；wizer 快照把 JS 冷启动 ~5 ms → 0.36 ms 🔍
- **稳态轴**：wasm vs 原生平均 **0.5–0.8×**（SPEC CPU 慢 45–55%，峰值 2.5×；Jangda 等 USENIX ATC 2019 🔍）；**注意陷阱：内存保留不足回落到显式边界检查时，稳态再砍 1.2–1.8×**（wasmtime docs 🔍）——限额配置与性能不免费兼得，**必须 spike 实测选型**

### 3.2 语言 → wasm 矩阵（速查）

| 语言 | 体积 | 启动 | 稳态 vs 原生 | 组件模型 | 判词 |
| --- | --- | --- | --- | --- | --- |
| **Rust** | 数百 KB | µs | 0.5–0.8× | 一等公民（wasm32-wasip2 直出） | **首选**（宿主同源，工具链复用度最高） |
| **C/C++** | 数十 KB | µs | 0.5–0.8× | 一等公民 | 首选 |
| Zig | 数 KB | µs | 同档（💭） | 社区级 | 可做 |
| AssemblyScript | 数–数十 KB | µs | 同档（💭） | 弱 | "JS 手感不付套娃税"的正解（npm 只剩 AS 子集） |
| TinyGo | ~10KB | µs–百 µs | 数值强/goroutine 弱 | 好（wasip2） | Go 的插件形态 |
| Go 官方 | ≥2MB | 百 µs–ms | 0.3–0.7× | 弱 | 与"多盒即弃"结构性不合 |
| C#/.NET | 数 MB | ms | AOT 0.3–0.5× | 实验性 | 2026 仍浏览器优先，风险高 |
| Python（componentize-py） | 25–35MB（🧠） | 十–百 ms | 0.2–0.5× CPython | 有 | **兼容层非性能层**；仅纯 Python 包（numpy 类不可用） |
| JS（Javy/ComponentizeJS） | ≥869KB / ~8MB | wizer 后亚 ms | 解释器档，热点比 V8 慢 10–100× | 弱/好 | 同上 |
| Kotlin/Wasm | 数百 KB | — | 厂商口径"逼近 JVM"📢 | Beta | 观察 |

（全部 🔍 各 README/官方文档 2026-10-04；未标处 🧠）

**生态信号**：组件生态在**收敛**——wit-bindgen 删 Java 生成器、弃 TinyGo 生成器转官方 Go；主力线 = Rust/C/C++/Go/JS/Python。选型押收敛侧，回避长尾。🔍

### 3.3 动态语言税：三层，吞吐层无解

1. **体积税**：wasm 内禁 JIT（无法动态生成机器码 🔍 Lin Clark 文），整个引擎打进模块（869KB–35MB）
2. **启动税**：wizer 预初始化快照可治（5 ms → 0.36 ms 🔍）
3. **吞吐税**：只有解释器档，JS 热点比 V8 慢 10–100×（🧠）——**无法工程消除，只能绕开**

**判词**：动态语言盒存在的唯一理由是"生态绑定"（必须跑某段已有 JS/Python 代码）。作为重盒载体，三层税叠加后全面劣于"Luau 薄编排 + 静态 wasm 盒/内建件"——**Creator 模式保留 Luau 直写的判决获得数字背书**（💭）。

### 3.4 host↔guest 边界成本与粒度规则

- 数值直传：数十 ns 级（🧠）；**字符串/缓冲必经 canonical ABI 拷入拷出**（分配+memcpy 各一次；Extism 产品化为 "bytes-in → bytes-out" 🔍）
- 推论（💭）：盒接口按**大颗粒批式**设计（一次调用处理一整批），严禁逐 token/逐元素细颗粒——**与我们邮箱模型的粒度规则完全同构**

### 3.5 与 Luau 的量化对照（💭 推断链已标注，待 spike 实测）

- 解释型动态语言 vs 原生通常差 10–30×（🧠）；wasm 静态语言 vs 原生差 1.5–2×（🔍）→ **wasm 盒 ≈ Luau 解释器的 5–20 倍速**（数值/文本密集；💭 待 spike 实测）。注意两点折让：(a) 通用档位对 Luau 可能方向性高估——Luau 解释器官方自述"某些负载可追平 LuaJIT 解释器"（🔍 luau.org/performance），属解释器中的快档；(b) 判决相关对照实为 wasm vs Luau-**NCG**（ADR-0021 已登记 `luau-jit` 可开，NCG 追回约 2–3×，🧠 原博客 404）——全档对照只有 spike 能给
- Luau 常驻宿主零装载成本；wasm 盒每次付 µs 实例化 + 一次 ABI 拷贝 → **跨界线：任务粒度 < 数百 µs 留 Luau；> 数 ms 的 CPU 密集任务进盒净赚**（分界同样待 spike 标定）
- Luau 编译器吞吐 950K 行/秒/单核（🔍 luau.org/performance）——Creator 模式"改完即跑"，wasm 盒（需外部编译链）无法复刻，E1 的性能侧注脚

### 3.6 天花板清单：边界，不是算力

| 能力 | 现状 | 处置 |
| --- | --- | --- |
| 零拷贝共享内存结构 | 无稳定方案（shared-everything 提案 Phase 1） | 接口批式设计 |
| GPU / 本地推理本体 | 无稳定 wasi:gpu；GEMM 撞 SIMD 128 位封顶 ≈ 0.2–0.5× 原生 BLAS（🧠） | **下沉为外部服务（部署组成）**，盒只做前处理 |
| AES-NI/SHA-NI 直通 | 无对应 wasm 指令，加解密密集差 3–10×（🧠） | **crypto 必须是宿主内建件**（印证接缝能力集议题的既有判决） |
| 裸协议栈（TLS/QUIC） | socket 必须宿主中介；加密热点同上 | 分层：socket 宿主、协议逻辑盒、加密内建件 |
| 多线程 | 与限额/池化不兼容 | 禁用；多实例+async 替代 |
| FFI 回调/指针富集结构 | 回调 = 完整跨界调用；结构必须扁平化序列化；`resource` 句柄可传活对象不传结构 | 接口设计约束 |
| 进程/信号/mmap/io_uring | 设计性缺席 | 恰是遏制的另一面 |

**上限判词（💭）：wasm 盒能做到"近原生的安全计算"，做不到"近硬件的系统编程"。墙不是障碍，是与遏制哲学同向的同一种设计。**

---

## 4. 生产案例与工程教训

### 4.1 案例速查表

| 系统 | wasm 角色 | 热更/生命周期 | 作者体验一句话 |
| --- | --- | --- | --- |
| proxy-wasm/Envoy | 代理 filter | 配置下发+重新实例化 | **矩阵地狱**：ABI×SDK×host 对齐才能跑；filter 至今标 experimental 🔍 |
| Shopify Functions | 结账热路径同步函数 | 重新部署即换 | 声明 GraphQL 输入即可；但官方劝退 JS："大购物车会失败，请用 Rust" 🔍 |
| Fastly Compute | **每请求 fresh 沙箱** | 版本化 clone/activate/rollback | 实例即弃在超大规模生产成立（无状态场景） 🔍📢 |
| Fermyon Spin | trigger 微服务 | spin build/up | 语言×特性能力矩阵文档 = 范本；**Fermyon 已并入 Akamai，项目在 CNCF 存续** 🔍 |
| wasmCloud | 分布式 workload（v1 曾 actor） | v2：K8s CRD；wash dev 热重载 | **v2 自我推翻 v1 全部自研抽象**（见 §4.2） 🔍 |
| Zellij | 一等窗格插件（自家 UI 全插件） | 运行时 reload | dogfooding 最深，但**官方只支持 Rust** 🔍 |
| Lapce | 编辑器插件 | proxy 进程内实例 | 实为 wasmtime 自研栈（非 Extism）；文档两年未更新 🔍 |
| Extism | 通用嵌入插件框架 | 框架不管生命周期 | 16 宿主 SDK + 10 PDK；**XTP Bindgen：schema→绑定生成是 N 语言 SDK 唯一可持续解** 🔍 |
| CosmWasm | 链上不可信合约 | migrate+治理授权 | **最完整公开 ABI 文档**；审核三件套最成熟 🔍 |
| Polkadot | （叛逃） | — | **自研 RISC-V PolkaVM 替代 wasm**，README 逐条控诉通用引擎 🔍 |

### 4.2 四个关键故事

**① wasmCloud v2.0（2026-03）自白——对 Metis 双重验证**。v1 与 Metis 最同构（wasm actor + 进程外 capability provider + 自研编排）；v2 官方自我否定式复盘：进程外 provider "introduced network overhead, created separate deployment concerns, and was a **consistent source of operational friction**"、"mental model 里的纳秒调用实际承受**传输失败、消息丢失、网络延迟**"→ v2 全面改为**进程内 host plugins** + 编排交 K8s + 网络显式化 + actor 术语消失。🔍 含义：(a) **能力必须进程内、IO 必须走 host imports**——我们"能力内建于宿主"的路线与冻结的 actor 决策获外部同构验证；(b) **整插件进程外化（T2 例外通道）的运维痛感有前车之鉴**——撤销其一等位的判断获外部证据支持。另注：连 wasmCloud 的**遏制三件套（内存强制/准入/失控兜底）也拖到 2026-08/09（v2.8/2.9）才补齐**——遏制在任何 wasm 宿主里都不是免费的，我们从第一天冻结是对的。🔍

**② Shopify Wasm API——ABI 设计的最优参照**。64-bit NaN-boxed i64 惰性值表示（4bit tag + 14bit 长度 + 32bit 数据/指针），把 JSON parser 逐出插件二进制（官方原话）；错误码扩展"不算破坏性变更"政策；经历过一次 v1→v2 破坏性迁移（带官方 migration guide）。🔍 对 A6（Value 线编码）是直接设计参照。

**③ 审核分发三件套在两个独立生态各自收敛**：确定性构建（CosmWasm rust-optimizer 容器化，1.6MB→126KB，链上字节码↔公开源码可复现验证）+ 二进制结构校验（cosmwasm-check）+ 签名（wasmCloud cosign+OIDC，OCI registry 分发）。**无此三件套的 marketplace 没有信任故事**——marketplace 格式设计任务直接可抄。🔍

**④ PolkaVM 叛逃——通用引擎成本的诚实警示**。Polkadot 判定通用 wasm 引擎在 VA 空间浪费（每实例 GiB 级虚拟预留）、编译速度、实例开销、**确定性 gas 计量**上不达标，自研 RISC-V VM（RV32EM，单遍 O(n) 编译、每实例基线 ≤128KB、语义版本化）。🔍 对我们的含义（💭）：链的痛点是指令级确定性（细粒度），我们的确定性粒度 = **邮箱消息**（粗粒度），gas 一条大部分不适用；VA 预留与实例开销两条对"实例即弃"模式部分适用——接受方式 = pooling 配置把"槽数 × 单槽预留"写成**显式容量规划**（官方示例即此形态），物理页按需提交，归 spike 的容量规划实测项；"fuel 开销 2× + CVE 史"意味着指令级确定性重放需实测——journal/replay 设计注意。

### 4.3 生态活力的真相（最重要的一条软证据）

**"语言无关"不自动产生生态**：Zellij 官方只支持 Rust、proxy-wasm SDK 仓百级星、Lapce 文档腐化——纯技术供给（ABI/SDK/框架）从未产生作者群。**有市场（Shopify App Store 分发+收费管道）或有资产（链上合约）的地方才有生态**。🔍+💭

对 Metis 的三重含义（💭）：(a) **Creator 模式（agent 直写 Luau）是最现实的插件供给来源**——Luau 通道的定位背书；(b) wasm 盒面向人类开发者的"原生能力"窄面，**不应背负生态 KPI**；(c) marketplace 格式设计必须把分发与激励当一等设计，不是 ABI 做完就有生态。

---

## 5. 对 Metis 的映射（盒管模型定妆建议）

### 5.1 分层定妆表（建议）

| 层 | 角色 | 技术 | 可换性 | 状态 |
| --- | --- | --- | --- | --- |
| **管** | 薄编排/组合（<数百 µs 任务） | Luau（+未来 TS 前端） | 毫秒热更；Creator 唯一通道 | 不变 |
| **盒** | 业务粒度重物（>数 ms 任务） | **wasm 组件**（人写，Rust/C/C++/Zig/AS/TinyGo 梯队） | 实例即弃热换（µs–ms 级）；IO 全走 host imports | **本调研建议入局** |
| **固定盒** | 高频细粒度通用件（ns–µs 调用粒度） | 核心内建件（json/crypto/正则/diff/sqlite…） | 构建期确定，随核心发版 | 逐项画线中（接缝能力集议题） |
| **外部服务** | wasm 撞墙区（GPU/推理本体/巨型依赖） | 部署组成（sidecar/MCP/远端 API） | 各自部署节奏 | 非插件形态 |
| **例外通道** | wasm 也盖不住的物理需求 | T2 进程插件（原生语言整插件独立进程，零 Luau） | 进程重启 | **撤销一等位**，单独立项才做 |

### 5.2 同一 ABI 双实现

插件 ABI（ctx/服务/事件/Value）对 Luau 经 mlua 直挂，对 wasm 盒形式化为 **WIT world**：`import` = ctx 能力面（宿主注入），`export` = 插件入口（setup/服务方法/事件处理）。WIT 包 = ABI 的 SSOT 入库，版本随 ADR 演进；宿主侧 `wasmtime::component::bindgen!` 与盒侧 `wit_bindgen::generate!` 从同一 WIT 生成——**schema 先行、绑定生成**（Extism XTP 验证的唯一可持续路径，也印证 schema DSL 与工具形态两议题的方向）。🔍+💭

### 5.3 WASI × 接缝能力集映射表（WASI = 别人设计好的 syscall 层，取舍直接参照）

| WASI 接口 | 对应 syscall 候选 | 参照点 |
| --- | --- | --- |
| clocks（wall+monotonic） | 时钟 | **我们不能直接给真实钟**——需虚拟化注入；wasmtime 有 `wasi-virt` 虚拟化层（时钟/文件系统）🔍，replay 关键工具 |
| random（两接口） | 随机数 | 同理需播种虚拟化 |
| filesystem（**preopen 目录能力模型**） | fs | 与"config 路径白名单"政策同构，直接可抄 🔍 |
| sockets / http | http client | 盒的网络全走宿主中介 = 我们的效应 syscall 同构 |
| cli（env/args） | env（已拒） | WASI 也是显式白名单——印证 env 不进的判决 |
| 0.3 async（stream/future） | SSE 流式扩展位 | wasm 盒的流式 IO 形态答案 🔍 |
| （无 keyvalue/logging 标准） | sqlite/log | 短期走自定义 WIT 接口，不等标准 🔍 |

### 5.4 内建件 vs wasm 盒的分工线（粒度判据）

- **内建件（sync 直挂，ns–µs）**：超高频细粒度通用小件——json/crypto/正则/diff/uuid/时间。另注：wasm 缺 AES/SHA 直通指令（差 3–10×），**crypto 必须内建件**，双重印证
- **wasm 盒（业务粒度，单次 >数 ms）**：向量检索、文档解析、协议逻辑、领域 SDK 封装——跨界成本被任务时长摊薄，净赚数倍至一个量级（vs Luau，💭 待 spike）
- **Luau 管**：编排、胶合、状态机（<数百 µs）
- **外部服务**：GPU/推理本体/巨型依赖（wasm 撞墙区）

### 5.5 ABI 版本化与工具链纪律（案例教训入库）

- **反面教材**：proxy-wasm 四 ABI 版本 × 四 SDK × 五 host 矩阵分发——**ABI 必须窄、版本化必须第一天设计** 🔍
- **三种可抄机制**：导出式版本信号（proxy-wasm `proxy_abi_version_x_y_z` / CosmWasm `interface_version_8`）；一次性迁移剧本（Shopify v1→v2 + migration guide）；WIT 包版本门禁（`@since/@deprecated`） 🔍
- **跨边界铁律**：不共享内存所有权，分配/释放归口写进 ABI（proxy-wasm `proxy_on_memory_allocate` / CosmWasm `allocate/deallocate`+Region 三元组） 🔍
- **Value 线编码参照**：Shopify NaN-boxed i64 惰性表示（窄值表示把解析器逐出插件二进制）→ Value 边界转换议题连带 🔍

### 5.6 marketplace 信任三件套（marketplace 格式设计的直接输入）

确定性构建（rust-optimizer 式容器化，源码↔字节码可复现验证）+ 二进制结构校验（cosmwasm-check 式能力审计）+ 签名（cosign OIDC 式，OCI registry 分发）。🔍

### 5.7 journal/replay 红利（vs T2 的决定性优势）

wasm 盒的一切 IO 走 host imports → **天然全量可拦截、可进 journal**（T2 的"replay 不透明"整条消失）。加码：wasmtime 官方有**确定性执行专页**（fuel 确定性中断、NaN 规范化开关、relaxed-SIMD 确定性模式、`memory.grow` 非确定消除三法、`wasi-virt` 虚拟化时钟/文件系统）🔍——journal/replay 设计的工具箱是现成的。注意（💭）：我们 replay 粒度 = 邮箱消息（粗），多数场景不必开 fuel；指令级确定性重放是可选深模式，需实测（fuel 2× 开销 + CVE 史）。

### 5.8 Creator 红线与生态预期管理

- **Creator 模式永走 Luau 直写**（E1：wasm 需外部编译链，结构性进不来）；动态语言 wasm 盒也只是兼容层——Luau 直写通道的地位在数字上不可撼动（§3.3/§3.5）
- **wasm 盒不背生态 KPI**（§4.3）：它是人类开发者的原生能力窄面；生态供给主力 = Creator 模式；marketplace 的分发/激励设计归 marketplace 格式设计任务

### 5.9 入局条件清单与风险登记

| # | 条件/风险 | 处置 |
| --- | --- | --- |
| C1 | fuel CVE 持续修复中 | 跟进 wasmtime 月度列车（多版本线回补使单次成本低；量级 = ~12 次/年升级，每次全量 ci + deny + Cranelift 大依赖树审计——登记为常态化运维成本）；deny 闸门拦 advisory |
| C2 | shared-memory 特性 | 配置禁用（与限额/池化不兼容） |
| C3 | 内存限额与性能的组合（显式边界检查砍 20–80%） | **wasmtime spike 实测选型**（guard 预留 vs 限额策略） |
| C4 | WIT × wasmtime × wit-bindgen 版本对齐（错配报 `wrong type`，诊断差；失守即退化为 §4.1 proxy-wasm 式乘积矩阵） | WIT 包 SSOT 入库；加载失败把 WIT 指纹打进错误消息 |
| C5 | ABI 版本化机制 | 第一天设计（§5.5 三机制选型归工具形态与 ABI 版本纪律议题） |
| C6 | WASI 0.2 vs 0.3 选型 | 决策点归 ADR-0024（建议：ABI 语义对齐 0.3，spike 双验；wasmtime 可按 world 分流） |
| C7 | 动态语言盒滥用 | 文档定位"兼容层非性能层"；默认引导静态语言梯队 |
| C8 | wasmtime 依赖体量与供应链 | adding-dependencies 流程过一遍（Cranelift 依赖树大）；vendored 锁版本策略对齐 mlua 先例 |

### 5.10 后续行动

1. **wasmtime spike**（紧随本报告，需走 adding-dependencies 流程——wasmtime 是重依赖）：实测实例化成本 / fuel vs epoch 开销 / 限额×性能组合（含容量规划：槽数×预留的 VA 与物理账）/ host imports 延迟 / `.cwasm` 预编译管线 / WASI 0.2 与 0.3 双验 / **Luau（解释与 NCG 两档）vs wasm 微基准**（§3.5 推断的复核）
2. **接缝能力集清单回填**：crypto 内建件双重印证（§3.6）；流式/SSE 的 wasm 侧答案 = 0.3 async（§5.3）；sqlite 定位内建件候选不变
3. **ABI 议题回填点**：manifest 格式议题（`runtime` 字段）、Value 边界转换议题（线编码参照 NaN-boxed）、schema DSL 议题（WIT = 盒侧形式化）、工具形态与 ABI 版本纪律议题（ABI rev 机制选型；`metis sdk` 包装 wasm32-wasip2 管线 + wasm-tools 验收）
4. **ADR-0024 起草**：盒管模型定妆 + ADR-0021 wasm 行/子进程行局部 supersede + T2 例外通道登记 + wasm 复审触发器（组件模型能力质变/wasi:gpu 落地/PolkaVM 类替代技术成熟）

---

## 附：主要证据来源（均 2026-10-04 访问）

- 运行时与提案：github.com/bytecodealliance/wasmtime/releases（v49.0.0–v49.0.2）；github.com/wasmerio/wasmer/releases；docs.wasmtime.dev（security / stability-wasm-proposals / examples-interrupting-wasm / examples-fast-instantiation / examples-fast-execution / examples-fast-compilation / examples-deterministic-wasm-execution / wasip2-plugins）；github.com/WebAssembly/proposals
- WASI 与组件模型：wasi.dev/releases/wasi-p2、wasi.dev/releases/wasi-p3、wasi.dev/roadmap；github.com/WebAssembly/WASI；component-model.bytecodealliance.org；crates.io（wasm-tools / wit-bindgen / cargo-component）；doc.rust-lang.org/rustc/platform-support/wasm32-wasip2.html
- 语言矩阵：github.com/bytecodealliance/{javy,ComponentizeJS,componentize-py,wit-bindgen}；github.com/tinygo-org/tinygo；tinygo.org WASI 指南；go.dev/wiki/WebAssembly；github.com/dotnet/runtime（mono/wasm/features.md）；peps.python.org/pep-0011；pyodide.org；assemblyscript.org；kotlinlang.org/docs/wasm-overview.html；luau.org/performance
- 性能基准：Jangda 等 USENIX ATC 2019（usenix.org）；bytecodealliance.org/articles/wasmtime-10-performance（2022-09-06）；bytecodealliance.org/articles/making-javascript-run-fast-on-webassembly（Lin Clark）
- 生产案例：github.com/proxy-wasm/spec；envoyproxy.io wasm filter 文档；shopify.dev/docs/apps/build/functions（+ programming-languages/webassembly-for-functions）；github.com/Shopify/shopify-function-wasm-api；fastly.com/products/edge-compute；docs.fastly.com；github.com/spinframework/spin；www.fermyon.com（301→akamai.com 记录）；github.com/wasmCloud/wasmCloud；wasmcloud.com/blog/wasmcloud-v2-is-here/ + RSS + cosign 签名博文（2025-09-02）；zellij.dev/documentation/plugins；github.com/zellij-org/zellij；github.com/lapce/lapce（+ lapce-proxy/Cargo.toml）；docs.lapce.dev；github.com/extism/extism；extism.org；github.com/CosmWasm/cosmwasm；github.com/near/near-sdk-js；github.com/paritytech/polkavm
- 核查失败记录：fastly.com 博客（拒绝）、fermyon.com（WAF 403）、shopify.engineering 旧文（404）、Roblox NCG 博客（404）、bytecodealliance.org/articles/winch（404）、golang 组件模型 issue（未取到）——相关条目均已降级标注
