> 状态：point-in-time 调研简报（2026-10-14，仓内子代理执行；来源纪律：全部实际打开、未确认进清单）。任务 8 测试实践补差调研三份之二，主题 = 严格/形式化验证与确定性仿真在中小型 Rust 项目的落地（Kani/Verus/Lean/DST）；结论 = 不需再委托外部深度调研。姊妹篇 = [testing-practice-oracle.md](./testing-practice-oracle.md)、[testing-practice-parallel-engineering.md](./testing-practice-parallel-engineering.md)；上位报告 = [review-economics-research.md](./review-economics-research.md)。正文逐字未改；原文出处 = `scratch/测试实践调研/d12-形式化与DST.md`。

# D12：严格/形式化验证与确定性仿真在中小型 Rust 项目的落地调研

> 委托：验证并细化维护者判断——「引入 lean4 等严格的测试方式来做测试，覆盖率强，并且能够有效控制我们的暴露点（这点很重要）」。
> 背景：Metis = 1+N（1 人类维护者 + N 个 AI agent）开发的自我进化 agent 运行时；仓内已有 `docs/design/testing-strategy.md`（L0–L4 分层、L4 候选内核、议题 D7 的 agent 推荐 = Kani 跳板 + Lean 试点 fiber 状态机）与 `scratch/评审经济学调研报告/` RQ3 章（PCC 教训、mutation testing、DST/混沌工程、门禁完整性）。本简报只补「2026 年工具链现状与落地成本」，不重复 RQ3 已覆盖内容。
> 调研执行：2026-10-14，调研子代理。来源纪律：只引用实际用 fetch 打开的页面，行内标注【实证】/【通说】/【争议】/【观点】，厂商来源追加「｜利益相关」。

---

## 一、调研范围与方法

范围 = 委托的五个问题：Rust 形式化工具链 2026 现状、Lean4 路线现实性、「暴露点控制」机制设计、DST 最小落地形态、agent 时代形式化新证据。

方法：以 GitHub REST API 元数据（`pushed_at`、`archived`、star 数）作为维护状态的客观证据；以官方 README/文档作为能力边界的来源；以 arXiv 摘要与会议出版物清单作为实证来源；以厂商博客作为利益相关来源单独标注。

网络可达性实况（2026-10-14）：

- 可达：`api.github.com`、`raw.githubusercontent.com`、`arxiv.org`、`export.arxiv.org`、`verus-lang.github.io`、`model-checking.github.io`、`sled.rs`、`en.wikipedia.org`、`deepmind.google`、`antithesis.com`、`risingwave.com`。
- 失败/绕行：`awslabs/cedar-spec` 与 `aws/SampCert` 均 404，经 GitHub 搜索 API 定位到 `cedar-policy/cedar-spec` 与 `leanprover/SampCert`；`leanprover/lean4lean` 404，实际在 `digama0/lean4lean`；`cryspen/libcrux` 的 README 404（仓库已迁至 `celabshq/libcrux`，README 仍 404，仅取得元数据）；`amazon.science` 的 Cedar 博客返回「no textual content found」（正文未读到）；arXiv 检索「RefinedRust」返回空结果（未获得可用记录）。

---

## 二、Rust 形式化工具链现状盘点表（2026-10-14）

| 工具 | 范式 | 维护状态【实证，GitHub API】 | 具名采用 | 能验证 / 不能验证 | 成本量级与 agent 适配 |
|---|---|---|---|---|---|
| Kani | 位精确有界模型检查（MIR→CBMC） | 活跃，pushed 2026-10-13，3.5k star，ASE'26 论文（[kani](https://github.com/model-checking/kani)，访问于 2026-10-14） | AWS s2n-quic、Firecracker 安全边界、Hifitime、Tokio Bytes；Rust 标准库验证挑战赛多个条目由 Kani 解出（[Kani 博客索引](https://model-checking.github.io/kani-verifier-blog/)、[verify-rust-std README](https://github.com/model-checking/verify-rust-std)，访问于 2026-10-14） | 能：UB、panic、溢出、函数合约，界内穷尽即证明；不能：**不支持并发**（官方明确指向 Loom/Shuttle），性质过大需手工拆解调试【通说，官方文档】（[Kani 工具对比页](https://model-checking.github.io/kani/tool-comparison.html)，访问于 2026-10-14） | harness 写法接近 proptest，零注解起步；反例是具体执行轨迹，适合 agent 自修复闭环【观点，综合官方文档】 |
| Verus | SMT 演绎验证，spec/proof 用 Rust 语法写 | 活跃，pushed 2026-10-14，3.3k star（[verus](https://github.com/verus-lang/verus)，访问于 2026-10-14） | Anvil（K8s 控制器活性，OSDI'24 最佳论文）、VeriSMo（机密 VM，OSDI'24 最佳论文）、CortenMM（SOSP'25 最佳论文）、Atmosphere 内核、Asterinas `vostd`、Vest/Verdict/OwlC、持久内存存储、页表实现、并发分配器等【实证，出版物清单】（[Verus publications and projects](https://verus-lang.github.io/verus/publications-and-projects/)，访问于 2026-10-14） | 能：函数级正确性、并发（`state_machines!`）、裸指针；不能：仅支持 Rust 子集（官方自述「正在扩大」），需要写 spec/引理【通说，官方 README】（[verus README](https://github.com/verus-lang/verus)，访问于 2026-10-14） | 注解成本中；LLM 证明生成文献最密集（见 §3.5），是「agent 写证明」证据最强的一档 |
| Creusot | 演绎验证，翻译到 Why3/Coma | 活跃，pushed 2026-10-13，1.9k star，LGPL-2.1（[creusot](https://github.com/creusot-rs/creusot)，访问于 2026-10-14） | CreuSAT（已验证 SAT 求解器）、Krabka（Kafka broker 安全算法）【通说，官方 README】（同上） | 默认证 panic/溢出/断言安全，加注解可证功能正确性；生态偏学术【通说】（同上） | 注解成本中；LLM 辅助文献缺位，agent 适配中低 |
| Prusti | 基于 Viper 的静态验证器 | 仓库迁至 `prusti/prusti`，pushed 2026-10-06 仍活跃，但官方 README 自称「prototype」【实证/通说】（[prusti](https://github.com/prusti/prusti)，访问于 2026-10-14） | 未见具名工业采用【实证：本轮调研未找到】 | 默认证溢出/panic 不可达，注解可证契约【通说】（同上） | 成本中；agent 适配低（无 LLM 文献、prototype 自述） |
| hax（Cryspen） | Rust→形式语言抽取（F* 稳定；Lean 自 0.4.0 起走 Charon+Aeneas；Rocq/ProVerif/SSProve/EasyCrypt 实验性） | 活跃，pushed 2026-10-13（[hax README](https://github.com/cryspen/hax)，访问于 2026-10-14） | libcrux：「The formally verified crypto library for Rust」，活跃（pushed 2026-10-13）【实证，仓库元数据】（[libcrux](https://github.com/celabshq/libcrux)，访问于 2026-10-14） | 目标支持近乎全 Rust，例外：`&mut` 不得出现在返回类型与别名场景；不支持项逐条以 issue 登记【通说，官方 README】（同 hax） | 抽取自动、证明在目标证明器里手写；密码学域外工程先例少 |
| Aeneas（+Charon） | MIR→纯 lambda 演算的功能化翻译，后端 F*/Coq/HOL4/Lean（Lean 与 HOL4 最成熟） | 活跃，pushed 2026-10-14（[aeneas README](https://github.com/AeneasVerif/aeneas)，访问于 2026-10-14） | hax 的 Lean 推荐链路即 Aeneas；翻译本身有 ICFP'22 形式化与 ICFP'24 符号借检查 soundness 论文【实证】（同上） | 限安全 Rust 子集；嵌套循环的 break/continue、`&mut` 实例化泛型不支持；unsafe 与并发待分离逻辑工作【通说，官方 README】（同上） | Lean 端有部分函数/终止性外置证明与 progress 战术支持，但证明仍主要手写，成本高 |
| RustBelt 谱系 | Coq/Iris 语义模型（RustBelt 2018→RustHornBelt） | 本轮未取得 2026 维护证据 | — | 定位是语言语义基础而非工程验证工具 | 不进工程候选清单；RefinedRust 检索未果，见未确认清单 |

生态注脚：AWS 与 Rust Foundation 合作的 Rust 标准库验证仓库接受 Kani、Flux、ESBMC（GOTO Transcoder）、KMIR、VeriFast 五种工具，多个挑战条目已标 Resolved 并有奖金【实证｜利益相关（AWS 主导）】（[verify-rust-std README](https://github.com/model-checking/verify-rust-std)，访问于 2026-10-14）。这是「Rust 形式化已有多工具竞技场」的旁证，也显示 Kani 是其中实战产出最多的工具。

---

## 三、五个问题的发现

### 3.1 Q1：工具链盘点与「agent 写代码、机器出证明」的适配度

盘点表见 §二。围绕委托重点（自动化程度与失败诊断质量）补三条判断：

1. **Kani 是当前唯一「零证明工程」入口**：写 harness（`kani::any()` + assert）即得界内穷尽证明，失败给出具体反例轨迹——诊断是 agent 可直接消费的形式【通说，官方文档】（[kani README](https://github.com/model-checking/kani)、[工具对比页](https://model-checking.github.io/kani/tool-comparison.html)，访问于 2026-10-14）。代价是两条硬边界：不支持并发；性质规模受求解器限制，需要「调试证明」的技能【通说】（同对比页）。
2. **Verus 的失败诊断与自动化处于中间档**：SMT 求解失败给不出 Lean 式证明项，但注解/引理/错误定位都发生在 Rust 语法内；2024–2026 出现专门的证明调试与自动化论文（CAV'24 proof actions、CADE'25 Cazamariposas、FMCAD'26 tunable automation）【实证】（[Verus publications](https://verus-lang.github.io/verus/publications-and-projects/)，访问于 2026-10-14）。
3. **Lean 路线（hax/Aeneas 直译）的自动化最弱**：翻译是自动的，但生成的 Lean 代码上的定理证明要手写 tactic 脚本；Aeneas 提供了 progress 战术与标准库模型来缓解【通说】（[aeneas README](https://github.com/AeneasVerif/aeneas)，访问于 2026-10-14）。对「机器出证明」工作流，这是三档中最差的一档——除非证明对象极小（如状态机不变量），手写量可控。

### 3.2 Q2：Lean4 路线现实性

**Lean4 验证实际软件（非纯数学）的具名案例（2024–2026，本轮实际打开的）：**

- **Cedar**（AWS 授权策略引擎）：`cedar-policy/cedar-spec` = Lean 定义式实现 + 定理证明 + 与 Rust 生产实现之间的差异随机测试（DRT）基础设施；仓库活跃（pushed 2026-10-13），主语言 Lean【实证】（[cedar-spec](https://github.com/cedar-policy/cedar-spec)，访问于 2026-10-14）。README 自述其方法论见两篇 AWS 官方博客（正文未能打开，见未确认清单）。
- **SampCert**（已验证差分隐私库）：`leanprover/SampCert`，活跃（pushed 2026-09-25）【实证，仓库元数据】（[leanprover/SampCert](https://github.com/leanprover/SampCert)，访问于 2026-10-14）。
- **lean4lean**：用 Lean 4 重写 Lean 4 内核/外部检查器，活跃（pushed 2026-10-10）【实证】（[digama0/lean4lean](https://github.com/digama0/lean4lean)，访问于 2026-10-14）。对「agent 写证明 + 内核检验」结构是直接加固（见 §3.5）。

**Rust→Lean 直译链路成熟度**：2026 年的关键事实是 hax 官方把 Lean 推荐路线让位给 Charon+Aeneas（`cargo hax into lean`，自 hax 0.4.0 起），hax 自研 Lean 后端降级为 legacy【通说，官方 README】（[hax README](https://github.com/cryspen/hax)，访问于 2026-10-14）。Aeneas 侧 Lean 是其最成熟后端之一，但被验证代码限安全 Rust 子集，unsafe/并发明确不在当前能力内【通说】（[aeneas README](https://github.com/AeneasVerif/aeneas)，访问于 2026-10-14）。结论：链路存在且活跃维护，但「直译实现级代码然后证」在 2026 年仍是高成本路线；libcrux 的实战主要走 F* 后端。

**model-first 工业先例**：Cedar 就是教科书案例——Lean 写定义式模型并对模型证定理，Rust 手写生产实现，两者一致性不靠翻译正确性证明而靠 DRT（ fuzz + property-based + differential）持续压测【实证】（[cedar-spec README](https://github.com/cedar-policy/cedar-spec)，访问于 2026-10-14）。这个「证明留在模型层、实现层用差异测试桥接」的架构，直接回答了 Metis 该走哪条 Lean 路线（见 §四）。

**成本公开数据**：每千行被证代码的 Lean 投入无公开数据（本轮未找到，见未确认清单）。唯一的公开成本锚仍是 seL4（Isabelle 时代）：约 1 万行 C 对应约 50 万行证明（约 50:1），作者方声称成本约 $400/LoC，低于同类高保障非验证内核的 $1,000/LoC、约为低保障内核的 2 倍【实证（二手综述，原始为 Klein et al. 系列论文）】（[Wikipedia: seL4](https://en.wikipedia.org/wiki/SeL4)，访问于 2026-10-14）。对 Metis 的含义：Lean 试点内核必须保持在百行级，否则成本失控。

### 3.3 Q3：「暴露点控制」的机制设计

**验证边界的画法，2026 年的三个可用先例：**

- seL4：证明覆盖 C 实现的功能正确性，但明确不覆盖启动代码、编译器、部分平台与硬件；官方把「保证相对于规约与假设成立」写成显式 Limitations，且 15 年以上已验证部分无公开 bug 报告【实证（二手）】（[Wikipedia: seL4](https://en.wikipedia.org/wiki/SeL4)，访问于 2026-10-14）。教训：**排除项要写成清单，不写就是不诚实**。
- Cedar：边界画在「语义模型」而非「实现」——Lean 证的是授权语义的定理；Rust 引擎的正确性由 DRT 桥保证【实证】（[cedar-spec](https://github.com/cedar-policy/cedar-spec)，访问于 2026-10-14）。这把 TCB 从「翻译器+编译器+内核」收缩为「Lean 内核 + spec 本身 + 差异测试的采样质量」。
- Verus 项目群：页表项目的证明「相对于硬件内存翻译子系统的模型」成立【实证】（[Verus publications](https://verus-lang.github.io/verus/publications-and-projects/)，访问于 2026-10-14）——硬件被显式建模为假设而非被信任为事实。

**PCC 教训（TCB 小而笨，见 RQ3）在 2026 工具链下的兑现**：Lean 的 TCB = 小内核 + spec 文本；且有 lean4lean 这类独立外部检查器在开发中【实证】（[digama0/lean4lean](https://github.com/digama0/lean4lean)，访问于 2026-10-14）。Verus/Kani 的 TCB 包含 SMT/SAT 求解器（Z3/CBMC），体量远大于 Lean 内核——这是为自动化支付的信任代价【观点，综合各官方文档】。直译路线（hax/Aeneas）额外把翻译器放进 TCB；Aeneas 有翻译与借检查的形式化论文（ICFP'22/'24），但端到端「翻译后定理 ⇒ 原 Rust 语义」并非机器全链路验证【通说，据官方 README 列出的论文范围】（[aeneas README](https://github.com/AeneasVerif/aeneas)，访问于 2026-10-14）。

**落到 fiber 状态机不变量的建议边界**【观点，本调研综合】：

- 验证面内：fiber 状态转移纯函数（状态 × 事件 → 新状态 × 副作用指令）、不变量陈述（含 disposer 栈 LIFO/幂等、epoch 指纹失效规则）、转移的终止性/无停滞性（safety 优先，liveness 后置）。
- 验证面外（写成显式排除清单）：调度器与 OS 线程、mlua/Luau C API 边界（归 Miri/L2）、wasmtime、真实时钟与 IO（AGENTS.md 已要求构造注入，天然在界外）、多租户配额策略。
- 一致性桥：Lean 模型 ↔ Rust 实现之间用 Cedar 式差异测试微型化——复用 L2 已规划的 fiber 生命周期参考模型做双写对拍；这样 L2 的模型不再是「一次性测试脚手架」，而是 L4 的信任组件。

### 3.4 Q4：DST 的最小落地形态

**FoundationDB/TigerBeetle 之外的中小项目先例（本轮实际打开的）：**

- **sled 仿真指南**：面向单项目作者的 DST 配方——核心逻辑写成 `receive(msg, at)` / `tick(at)` 纯状态机，事件进优先队列按虚拟时间派发，笔记本上每秒跑数千个负载，对比 Jepsen 每 5 分钟一个用例【观点（作者实践指南）】（[sled simulation guide](https://sled.rs/simulation.html)，访问于 2026-10-14）。这正是「虚拟时间 + 脚本化故障 + seed 重放」轻量版的原始文献之一。
- **madsim / RisingWave**：madsim 以替换 tokio/tonic/etcd 等依赖的方式做全系统确定性仿真【通说，官方 README】（[madsim](https://github.com/madsim-rs/madsim)，访问于 2026-10-14）。RisingWave 的收益数据：确定性端到端测试约 2 分钟 vs 真实部署约 8 分钟，仿真比真实执行快约 4–5 倍，CI 每 PR 并行 16 个 seed；恢复测试与扩缩容测试发现大量并发 bug（panic、死锁、计算错误）【实证｜利益相关（厂商博客）】（[Applying Deterministic Simulation: The RisingWave Story](https://www.risingwave.com/blog/applying-deterministic-simulation-the-risingwave-story-part-2-of-2/)，访问于 2026-10-14）。同文也记录了代价：测试时长持续增长、需要更聪明的调度策略、仅限 Rust 组件。维护风险：madsim 最近一次提交在 2026-02-16，距今约 8 个月【实证，GitHub API】（[madsim 元数据](https://api.github.com/repos/madsim-rs/madsim)，访问于 2026-10-14）。
- **turmoil**（tokio-rs 官方组织）：单线程多宿主确定性仿真，注入延迟/丢包/分区/崩溃/撕裂写，seed 化 RNG；2026 年已扩展为 `turmoil` + `turmoil-net` + `turmoil-fs` + `turmoil-io-uring` 家族，提供 `tokio::net`、`std::fs`、`io_uring` 的确定性 drop-in 替换【通说，官方 README】（[turmoil](https://github.com/tokio-rs/turmoil)，访问于 2026-10-14）；维护活跃（pushed 2026-10-05）【实证，GitHub API】（[turmoil 元数据](https://api.github.com/repos/tokio-rs/turmoil)，访问于 2026-10-14）。drop-in 替换形态对 Metis 尤其相关：不需要自研 harness 即可获得网络与文件系统故障面。
- **Antithesis**（商业 DST 平台）：自建确定性 hypervisor，故障注入 + 完美重放 + 自动根因分析；客户自述包括 Jane Street、Turso、Confluent、etcd、Ethereum Foundation（the Merge）、Filecoin；2026 年营销已转向「作为 agent skill 进入开发循环」【实证｜利益相关（厂商官网与 testimonials）】（[antithesis.com](https://antithesis.com/)，访问于 2026-10-14）。对 Metis 是价格锚而非可选项（单进程项目用自建轻量版即可）。

**DST 与形式化的关系**：互补而非替代【观点，本调研综合】。DST 探索系统级行为空间、发现「未知的未知」，其断言是采样型 oracle；形式化在边界内给出全称保证，但边界本身不证自明。Cedar 把两者缝在一起：Lean 证模型定理 + DRT 压实现一致性【实证】（[cedar-spec](https://github.com/cedar-policy/cedar-spec)，访问于 2026-10-14）。对 Metis 的落点：AGENTS.md 的时间/随机/IO 构造注入纪律就是 DST 的结构地基——L2 的 fiber 模型测试可按 sled/turmoil 配方升级为「虚拟时间 + 脚本化故障 + seed 重放」，不需要全量仿真基础设施。

### 3.5 Q5：agent 时代的形式化新证据

**LLM 辅助证明生成（Rust/Verus 侧，证据密集）：**

- **AutoVerus**（OOPSLA 2025，杰出制品奖）：多 agent LLM 网络模拟专家证明三阶段（初稿→通用提示精化→按验证错误调试），150 个非平凡证明任务中自动生成正确证明超过 90%，过半在 30 秒内或 3 次 LLM 调用内完成【实证（论文自述指标）】（[arXiv:2409.13082](https://arxiv.org/abs/2409.13082)，访问于 2026-10-14）。
- **KVerus**（ASE 2026）：检索增强 + 依赖感知分析，单文件基准 80.2%（对比 AutoVerus 56.9%），仓库级跨文件基准 51.0%（对比多轮提示基线 4.5%）；在 Asterinas Rust OS 内核上产出的证明被上游接受，覆盖内存管理模块 23 个此前未验证函数【实证（论文自述指标）】（[arXiv:2605.03822](https://arxiv.org/abs/2605.03822)，访问于 2026-10-14）。这是「机器生成的证明进入真实 Rust 系统内核」的首个具名案例。
- 同一出版页上还有：RAG-Verus、AlphaVerus（ICML 2025，自我改进的翻译+树精化）、VeriStruct、ExVerus（ICML 2026，用反例修证明）、VeruSAGE（NeurIPS 2026，agent 化验证研究）、以及 Microsoft 的「Automated Proof Generation via Self-Evolution」（ICLR 2025）【实证（出版物清单）】（[Verus publications](https://verus-lang.github.io/verus/publications-and-projects/)，访问于 2026-10-14）。结论：**「agent 写 Verus 证明」在 2025–2026 已是有一条完整出版线的现实，不是愿景**。

**LLM × Lean（数学侧，证据强度高但域不同）：**

- **DeepSeek-Prover-V2**（2025）：Lean 4 定理证明开源模型，miniF2F-test 88.9%，PutnamBench 658 题解出 49【实证（论文自述指标）】（[arXiv:2504.21801](https://arxiv.org/abs/2504.21801)，访问于 2026-10-14）。注意规模：671B 参数，训练推理成本不在中小项目可复现范围内。
- **AlphaProof**（Google DeepMind，2024-07）：Lean + AlphaZero 式强化学习，IMO 2024 达银牌水平（28/42），每步证明由 Lean 验证后反哺训练；方法论 2025-11 发表于 Nature【实证｜利益相关（DeepMind 官方博客）】（[AlphaProof 博客](https://deepmind.google/discover/blog/ai-solves-imo-problems-at-silver-medal-level/)，访问于 2026-10-14）。
- **LeanDojo**（NeurIPS 2023 oral）：开源 Lean 交互与检索增强证明基础设施，benchmark 含 98,734 条定理，ReProver 训练只需一个 GPU 周【实证】（[arXiv:2306.15626](https://arxiv.org/abs/2306.15626)，访问于 2026-10-14）。

**「agent 写证明 + 内核检验」= 证明领域的 1+N**：成立，且这正是 PCC「生成难、检验易」不对称（RQ3）在 LLM 时代的兑现——LLM 生成的证明项不需要被信任，只需要通过内核检查；Lean 侧有 lean4lean 外部检查器进一步缩小信任面【实证/观点，综合】（[digama0/lean4lean](https://github.com/digama0/lean4lean)，访问于 2026-10-14）。结构性差别要记录：Verus 侧检验器 = Verus + Z3，TCB 大于 Lean 内核；Lean 侧数学证明自动化证据强，但**软件（非数学）证明的 LLM 生成在 Lean 上尚无 Verus 同等级别的出版证据**【实证（本轮未找到），见未确认清单】。

---

## 四、对 D7 推荐的对账（Kani 跳板 + Lean 试点 fiber 状态机）

**支持的成分：**

1. Kani 作实现级跳板、随内核实现同步引入——强支持。零注解起步、CI 现成（GitHub Action）、失败诊断是具体反例，且有 Rust 标准库验证挑战赛的实战记录【实证】（[kani](https://github.com/model-checking/kani)、[verify-rust-std](https://github.com/model-checking/verify-rust-std)，访问于 2026-10-14）。注意一条硬约束必须在 D7 写明：Kani 不支持并发【通说】（[工具对比页](https://model-checking.github.io/kani/tool-comparison.html)，访问于 2026-10-14）——fiber 状态机的 Kani 证明对象只能是「单步转移 + 无并发交织下的不变量」，交织覆盖留给 L2/DST。
2. Lean 试点选 fiber 状态机不变量——支持，但路线选择已有答案倾向：model-first（Cedar 模式）。理由：2026 年 Rust→Lean 直译链路的事实标准已是 Aeneas（hax 官方让位），其能力边界（安全 Rust 子集、无 unsafe/并发）与 Metis fiber 内核基本相容，但证明仍手写、成本高；model-first 有生产级工业先例、TCB 最小、且能复用 L2 参考模型【实证/观点，综合 §3.2、§3.3】。

**动摇/需修正的成分：**

1. 委托方判断中「覆盖率强」的措辞需要修正：形式化给的是**边界内全称保证**（对已陈述性质不存在反例），不是覆盖率语义；把它当「更强的覆盖率」会在门禁设计里错位（覆盖率问题的实证教训见 RQ3：coverage 与有效性弱相关）【观点，本调研综合】。
2. D7 原推荐把 Lean 当唯一的「严格」终点，低估了 Verus 一档：2025–2026 的 LLM 证明生成出版线几乎全部压在 Verus 上（AutoVerus/KVerus/VeruSAGE…），且 KVerus 已有上游接受的实证【实证】（[Verus publications](https://verus-lang.github.io/verus/publications-and-projects/)、[arXiv:2605.03822](https://arxiv.org/abs/2605.03822)，访问于 2026-10-14）。若 D7 的目标是「agent 写证明、机器检验、最小人类介入」，Verus 是 2026 年证据最强的一层。
3. 「暴露点控制」判断成立，但机制要显式化才成立：先例（seL4/Cedar/Verus 页表）的共同点是**排除项写成清单、TCB 枚举到组件**；没有这张清单，verified core 反而扩大暴露面（绿灯幻觉，RQ3 已述）【观点，本调研综合】。

**修正建议（决策点，按推荐度排序）：**

1. **（推荐）L4 改为三层跳板**：Kani（实现级有界证明，随 fiber 状态机落地，零注解）→ Verus（SMT 级函数正确性，优先用 LLM 证明生成工具链试点，对准 disposer 栈/epoch 失效这类局部内核）→ Lean 仅作设计级 model-first 试点（fiber 状态机不变量，百行级模型），模型与 Rust 实现的一致性用 Cedar 式差异测试桥接（复用 L2 参考模型）。直译路线（hax→Aeneas→Lean）降级为 spike 对照组而非主线。
2. 维持原推荐（Kani + Lean 试点，直译 vs model-first 由 spike 定夺）：可行，但把 2026 年证据最强的 Verus/LLM 证明层留空，Lean 试点的人时风险无对冲。
3. 全 Verus 单一路线（放弃 Lean）：自动化收益最大，但失去设计级模型的独立审计面（Verus spec 与代码同源同语言，设计级审查的「第二种语言表达」价值消失），且 VerusBelt（语义基础）2026 年才发表，长期生态未定【实证/观点】（[Verus publications](https://verus-lang.github.io/verus/publications-and-projects/)，访问于 2026-10-14）。

无论选哪条，建议给 Lean/Verus 试点预定义 spike 判据：证明完成度、人/机时记录、与 L2 模型的复用度、失败诊断可否被 agent 直接消费——这四项正好对应 1+N 的核心矛盾。

---

## 五、未确认清单

以下来源未打开或检索未果，相关论断均不进入正文结论：

1. Amazon Science 博客「How we built Cedar with automated reasoning and differential testing」与 AWS 博客「Lean Into Verified Software Development」：链接存在于 cedar-spec README，正文抓取失败（无文本内容）。Cedar 的团队规模、人时投入、生产部署范围均未确认。
2. libcrux README（`celabshq/libcrux` 与 `cryspen/libcrux` 均 404）：libcrux 的具体算法覆盖面、下游集成（如浏览器）均未确认；本轮仅取得仓库元数据与自述描述。
3. RefinedRust：arXiv API 检索返回空记录，其 2026 维护状态与能力未确认。
4. LeanSAT 等 Lean4 验证工具链项目的现状：本轮未打开。
5. Harmonic Aristotle、Kimina-Prover 等 2025 年 Lean 定理证明系统：未打开；对 Q5 结论（数学侧证据强）方向性影响有限，但数字未确认。
6. KVerus、VeruSAGE、ExVerus 等论文全文：仅读摘要与出版物清单条目，方法细节（如 Asterinas 证明的审查强度）未确认。
7. Lean 侧软件验证的每千行成本：公开数据本轮未找到，倾向于「不存在公开数据」，未确认。
8. turmoil 的具名采用者清单：README 未列，未确认。
9. seL4 的 $400/LoC 与 50:1 数字：来自 Wikipedia 二手综述（原始出处为 Klein et al. 系列），未直接打开原始论文。

---

## 六、是否需要再委托一轮外部深度调研

**判断：不需要。** 理由：五个调研问题的证据链已足以更新 D7（§四给出了可执行的修正建议与判据）；剩余不确定性分为两类——(a) 文献补不全的（Lean 软件验证成本无公开数据，再查大概率仍无）；(b) 文献不该替代的（hax/Aeneas 对 Metis fiber 内核的实际适配性、Verus 对 fiber/多租户抽象的表达能力、LLM 证明生成在我们代码风格下的命中率——这些只能靠动手 spike 回答）。

若未来要补，缺口问题清单（窄范围、按需触发）：

1. D7 大讨论前刷新一次「Lean 侧软件（非数学）证明的 LLM 生成工具」现状——该领域半年一变，2026-10 的证据到期快。
2. Cedar 两篇官方博客正文（本轮被反爬拦住）：重点抽 Cedar 的团队规模与维护成本，作为 model-first 路线的成本锚。
3. Verus 对 async/executor 风格代码的支持现状（fiber 内核若走 Verus 层，这是第一道技术闸门）——优先用 spike 回答，文献兜底。
4. madsim 维护活跃度复查（若 L2/DST 选型考虑它；turmoil 当前更活跃）。
