> 状态：point-in-time 调研简报（2026-10-14，仓内子代理执行；来源纪律：全部实际打开、未确认进清单）。任务 8 测试实践补差调研三份之一，主题 = AI 生成代码的测试分层与 oracle 问题；结论 = 不需再委托外部深度调研。姊妹篇 = [testing-practice-formal-verification.md](./testing-practice-formal-verification.md)、[testing-practice-parallel-engineering.md](./testing-practice-parallel-engineering.md)；上位报告 = [review-economics-research.md](./review-economics-research.md)。正文逐字未改；原文出处 = `scratch/测试实践调研/d11-测试分层与oracle.md`。

# d11 — AI agent 生成代码的测试分层与 oracle 问题（测试实践补差简报）

> 委托方：Metis 项目｜定位：「评审经济学」调研（`scratch/评审经济学调研报告/`）的补差维度简报，编号沿用其 d01–d10 序列｜调研执行日期：2026-10-14
> 范围声明：本简报只覆盖委托指定的四个「测试实践」缺口问题。旧报告 RQ3 章已覆盖的 mutation testing 学术脉络（Just 2014 / Papadakis 2018）、coverage 与断言质量实证（ICSE 2014 / ESEC/FSE 2015 / pseudo-tested methods）、DST/混沌工程、门禁完整性信任链，本文一律不重复，仅交叉引用。

---

## ① 调研范围与方法

**范围**：四个问题——(Q1) oracle 问题：agent 自写实现又自写测试的自证结构如何应对；(Q2) 测试分层（单元/集成/property/metamorphic/differential/快照）对 AI 生成代码缺陷形态的分工；(Q3) 覆盖率 + mutation 组合门禁的工业实践；(Q4) 快照与回归语料库治理。对齐基准：种子稿 `docs/design/testing-strategy.md` 与 HANDOFF.md §2 留存的 D1–D8 议题出示稿。

**方法**：与委托方原调研同一来源纪律——只引用实际用 fetch 打开并读到正文的页面；arXiv 官方 API 返回的摘要视为已打开（元数据与摘要页同源）；只看搜索摘要未打开的一律进 ④ 未确认清单。论断分级：【实证】（有实验或运行数据）/【通说】（行业共识、官方文档、教科书级）/【争议】/【观点】（个人署名立场）；厂商来源追加「｜利益相关」。工具官方文档（insta、cargo-mutants、proptest、Jest、Infection 等）按【通说】处理——它们证明的是「该模式被工具正式支持」，不证明有效性。

**网络可达性实况**（失败与受限如实记录）：

- arXiv abs 页与 arXiv API 间歇性 429 限流（并行请求触发）；放慢为串行后 abs 页恢复可用，API 后期仍多次 429。后果：两个检索目标（LLM 单元测试生成综述专文、CSUR 2018 metamorphic 综述）未能定位到可打开页面，进 ④。
- dl.acm.org 全站 Cloudflare 403（人机验证墙）：Goldstein et al.《Property-Based Testing in Practice》（ICSE-SEIP 2024）原文未打开，进 ④。
- dblp.org 被 Anubis 反爬拦截；api.semanticscholar.org 持续 429。后果：McKeeman 1998《Differential Testing for Software》与 Barr et al. 2015《The Oracle Problem in Software Testing: A Survey》均未能打开任何可核实版本，进 ④。
- cs.utah.edu（Csmith 论文 PDF）、cis.upenn.edu（PBT 论文 PDF 猜测路径）、cs.dartmouth.edu（McKeeman PDF）均 404/403——猜测路径未命中，不视为来源失败，视为未找到。
- github.com、testing.googleblog.com、martinfowler.com、kentcdodds.com、insta.rs、mutants.rs、docs.rs、infection.github.io、rust-fuzz.github.io、google.github.io、research.google、jestjs.io 均可达。

**材料结构说明**：本轮共实际打开 30 余个页面，进入论断清单的 25 项。凡引用 arXiv 预印本且摘要自注未过同行评审的，标注「预印本」；标注了正式发表场次的（FSE 2024、EASE 2024、MSR 2024、ISSRE 2026、FSE 2026、IEEE TSE、ESEC/FSE 2019）按该场次口径。

---

## ② 四个问题的发现

### Q1 Oracle 问题：agent 自证如何应对

**Q1.1 AI 生成测试的缺陷模式：实证链已经成形，方向一致——「能跑、有覆盖、没牙齿」。**

Siddiq et al.（EASE 2024）用 Codex/GPT-3.5/StarCoder 在两个基准上生成 JUnit 测试：Codex 在 HumanEval（竞赛式小函数）上行覆盖率超 80%，但在 EvoSuite SF110（真实开源类）上没有任何模型超过 2%；生成测试普遍存在 test smell，典型为 Duplicated Asserts 与 Empty Tests（空测试）【实证】（[Using Large Language Models to Generate JUnit Tests: An Empirical Study, EASE 2024](https://arxiv.org/abs/2305.00418)，访问于 2026-10-14）。

ULT（UnLeakedTestbench，2025 预印本）控制数据污染后用 3,909 个真实世界高圈复杂度 Python 函数测 LLM 测试生成：全部受测 LLM 平均仅达 accuracy 41.32%、语句覆盖 45.10%、分支覆盖 30.22%、**mutation score 40.21%**；对照其在 TestEval（较简单、可能泄漏）上的 91.79%/92.18%/82.04%/49.69%——即使在简单基准上，LLM 生成测试平均也只能杀死约一半变异体【实证·预印本】（[Benchmarking LLMs for Unit Test Generation from Real-World Functions](https://arxiv.org/abs/2508.00408)，访问于 2026-10-14）。这是对「agent 写测试防断言了个寂寞」（种子稿 §3.C）最直接的一组量化：LLM 测试的 mutation score 腰斩于覆盖率之下，覆盖绿灯与牙齿脱钩在 AI 测试上比在人类测试上更严重（对照旧报告 RQ3 的 pseudo-tested methods 实证）。

**Q1.2 工业界的应对范式一：非 LLM 过滤器做独立 oracle，LLM 只当生成侧。**

Meta TestGen-LLM（FSE 2024）是迄今最大规模的 LLM 测试生成工业部署报告：其核心架构不是「让 LLM 写好测试」，而是用一组**与 LLM 无关的机械过滤器**（能编译、可靠通过、提升覆盖率）验证产出，「thereby eliminating problems due to LLM hallucination」；实测 75% 生成用例可编译、57% 可可靠通过、25% 提升覆盖率——即未加过滤时约四成产出不可靠；test-a-thon 中工程师接受了 73% 通过过滤器的推荐【实证｜利益相关：作者均任职 Meta】（[Automated Unit Test Improvement using Large Language Models at Meta, FSE 2024](https://arxiv.org/abs/2402.09171)，访问于 2026-10-14）。

Meta ACH（2025 预印本，投稿 FSE 2025 Industry Track）把同一范式推进到 mutation 引导：从 10,795 个 Kotlin 类生成 9,095 个「当前测试抓不到」的定向变异体，再让 LLM 生成能杀死它们的测试；其中**等价变异体检测用一个独立 LLM agent 完成，裸精度 0.79/召回 0.47，加简单预处理后升至 0.95/0.96**；工程师接受 73% 的生成测试，但仅 36% 被判为与目标关注点（隐私）直接相关【实证·预印本｜利益相关：作者均任职 Meta】（[Mutation-Guided LLM-based Test Generation at Meta](https://arxiv.org/abs/2501.12862)，访问于 2026-10-14）。两条合读：工业界对「AI 测试不可信」的回答不是更强的模型，而是「不可信侧生成、机械侧过滤、人终审」的三段结构——与旧报告 RQ3 的生成-验证不对称不变量互为印证；且「等价/低价值变异体的机器预筛」已被证明可做到 0.95 精度，这为 D4 的 missed=0 门禁降噪提供了现成技术路线（见 Q3.3）。

**Q1.3 「测试即规格」（人写/审测试、agent 写实现）：学术实证与工业实践同向。**

Mathews & Nagappan 在 HumanEval/MBPP 上实验：把测试用例随问题陈述一起提供给 GPT-4/Llama 3，代码生成成功率一致提高【实证】（[Test-Driven Development for Code Generation](https://arxiv.org/abs/2402.13521)，访问于 2026-10-14；页面挂 DOI 10.1145/3691620.3695527，正式发表场次未逐字核实）。TENET（ISSRE 2026 已接收）把「开发者写的测试 = 可执行规格」推进到仓库级 agent 框架：用测试驾驭机制挑选简洁测试子集引导 agent 检索与迭代修复，RepoCod/RepoEval Pass@1 较最强基线 +9.49/+2.17 个百分点；论文自称给出了「测试套件特征如何影响 agent 在 TDD 设定下表现」的首个系统研究【实证】（[TENET: One Step Toward Test-Driven Development for Repository-Level Code Generation](https://arxiv.org/abs/2509.24148)，访问于 2026-10-14）。WebApp1K 基准（2025 预印本）进一步把测试同时用作 prompt 与验证器，发现 TDD 式生成的关键能力是 instruction following 与 in-context learning，而非一般编码熟练度【实证·预印本】（[Tests as Prompt: A Test-Driven-Development Benchmark for LLM Code Generation](https://arxiv.org/abs/2505.09027)，访问于 2026-10-14）。

对照侧证据：一项 vibe coding × TDD 的对照探索（2026 预印本，样本小）发现**全自动 agentic 工作流虽产出功能正确的代码，但会引入规格之外的实现决策，形成「未被测试覆盖的决策点」**；人机协作工作流则产出更高质量、更有组织的测试套件【实证·预印本·小样本】（[Vibe Coding: An Experiment with Test-Driven Development](https://arxiv.org/abs/2607.22406)，访问于 2026-10-14）。这正是「测试由独立一方书写」的反向论证：实现者自定规格时，规格外行为天然无测试。

工业实践侧，GitHub 官方 spec-kit 仓库（141k stars）把 spec-driven development 产品化为 standard flow（constitution → specify → plan → tasks → implement → converge），其 bug 修复扩展明文把诊断、修复、验证三者分离，「Missing verification is not a successful fix」【通说｜利益相关：GitHub 官方工具，stars 数为平台口径】（[github/spec-kit README](https://github.com/github/spec-kit)，访问于 2026-10-14）。AlphaCodium（CodiumAI）的 test-based 多阶段迭代流在 CodeContests 上把 GPT-4 pass@5 从 19% 提到 44%【实证｜利益相关：作者任职 CodiumAI】（[Code Generation with AlphaCodium](https://arxiv.org/abs/2401.08500)，访问于 2026-10-14）。SWE-bench 的评测范式（2,294 个真实 GitHub issue + 对应 PR，让模型改库解题）则以社区基础设施的形式把「真实 issue + 人写测试」固化为 agent 能力的通用度量基座【实证】（[SWE-bench: Can Language Models Resolve Real-World GitHub Issues?](https://arxiv.org/abs/2310.06770)，访问于 2026-10-14；「以 PR 测试作判定 oracle」为其论文方法，本简报仅打开摘要页，该细节按摘要未逐字核实处理）。

**Q1.4 独立测试生成（与实现解耦的测试来源）：有效证据存在，但「异构模型交叉」无直接证据。**

ConVerTest（2026 预印本）正面命名了同源风险：现有 LLM 测试生成「often rely on ground-truth code for verification, **risking bug propagation**」——用实现反推测试会把实现的 bug 传播进 oracle；其对策是在**没有任何实现**的条件下合成测试：self-consistency 多数投票 + chain-of-verification + 代码与测试双向执行一致性，测试 validity/行覆盖/mutation score 较基线最高提升 39%/28%/18%【实证·预印本】（[Consistency Meets Verification: Enhancing Test Generation Quality in LLMs Without Ground-Truth Solutions](https://arxiv.org/abs/2602.10522)，访问于 2026-10-14）。AgentCoder 用角色分离的多 agent 架构（programmer / test designer / test executor 三个专门 agent）在 HumanEval/MBPP 上以更低 token 开销超过单 agent 与 prompt 技巧路线【实证】（[AgentCoder: Multi-Agent-based Code Generation with Iterative Testing and Optimisation](https://arxiv.org/abs/2312.13010)，访问于 2026-10-14）。

需要如实收窄的两点：其一，这两项的「独立」是角色/流程级解耦，**不是异构模型交叉**——「用模型 A 写实现、模型 B 写测试，错误是否去相关」本轮未找到任何对照实验，进 ④。其二，「同一模型写的测试镜像同一模型的实现错误」作为量化命题，本轮只找到问题陈述（ConVerTest 的 bug propagation 定性表述与 Q1.3 的 untested decision points），未找到错误相关系数的测量研究，进 ④。

**Q1.5 小结**：oracle 问题的三层应对在 2024–2026 已有可引用形态——机械过滤器（TestGen-LLM/ACH，已生产验证）、规格先行（TDD-for-agents 实证 + spec-kit 工业实践）、流程级独立生成（AgentCoder/ConVerTest，预印本级）。三者共同的结构原理是旧报告 RQ3 的「oracle 独立且被检验」不变量；本轮新增的是：该原理在 LLM 时代的落地件与量化收益数字。

### Q2 测试分层对 AI 代码缺陷形态的分工

**Q2.1 金字塔与奖杯之争没有新硬证据，但缺陷形态数据给奖杯派加了砝码。**

经典分层论述无新实证：Google 测试博客的 70/20/10 金字塔建议与「理想反馈环 = 快 + 可靠 + 能隔离失败」三性质仍是通说基线【通说｜利益相关：Google 工程博客】（[Just Say No to More End-to-End Tests](https://testing.googleblog.com/2015/04/just-say-no-to-more-end-to-end-tests.html)，访问于 2026-10-14）；Fowler 站的 Practical Test Pyramid 强调「能下沉的测试尽量下沉、高层测试只测低层测不到的部分、避免层间重复」与 consumer-driven contract test【通说】（[The Practical Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html)，访问于 2026-10-14）；Kent C. Dodds 的奖杯派主张「mostly integration」——理由是沿金字塔上行单个测试的 confidence quotient 递增，而 mock 每用一个就移除一份对集成的信心【观点：前端测试教育者，语境为 JS 应用】（[Write tests. Not too many. Mostly integration.](https://kentcdodds.com/blog/write-tests)，访问于 2026-10-14）。

旧报告 d07 已引的缺陷形态证据（Apiiro：语法错误 −76%、权限类 +322%、架构类 +153%；BaxBench：功能正确与安全近似正交）在本轮没有新增独立数据源，但它与分层论的合读结论是直接的：**AI 代码的残余缺陷集中在单元层看不见的地方**（集成语义、权限边界、架构约束），语法与局部逻辑层恰是 AI 最不易错、且类型/linters 已兜底的地方。这使「单元测试数量优势换信心」的经典金字塔论证在 AI 代码上进一步弱化，而把预算投向集成/契约/系统层与 oracle 更强的层（property/metamorphic/differential）的论证增强——此合读判断为本简报推断，标注【推断】，依据为上述两条来源的组合。

**Q2.2 无规格 oracle 的三条技术路线均有 2024–2026 新证据，且都适合机器生成。**

_Metamorphic（蜕变测试）_：metamorphic testing 已被综述表述为解决 oracle 问题的主流技术之一【通说·workshop 论文】（[Metamorphic Relation Generation: State of the Art and Visions for Future Research](https://arxiv.org/abs/2406.05397)，访问于 2026-10-14）。关键的新证据是 MR-Coupler（FSE 2026）：利用方法间功能耦合自动构造蜕变关系、用 LLM 生成蜕变测试、再用测试放大与变异分析过滤，对 100 个人写蜕变测试任务的有效生成率超 90%，对 50 个真实 bug 检出 44%，误报较基线降 36.56%【实证】（[MR-Coupler: Automated Metamorphic Test Generation via Functional Coupling Analysis](https://arxiv.org/abs/2604.10126)，访问于 2026-10-14）。含义：蜕变关系是可以部分机器化的「弱 oracle」，对 agent 代码这类「无权威规格」场景天然适配，且生成成本可压给机器。

_Differential（参考实现/黄金样本交叉）_：Csmith 的自述仍是该路线最干净的工业级先例——随机生成无未定义行为的 C 程序，以「多编译器输出不一致」为 oracle 找编译器 bug【通说】（[csmith README](https://github.com/csmith-project/csmith)，访问于 2026-10-14；背后 PLDI 2011 论文本体未打开，见 ④）。对 Metis 的映射：L2 的「对偶参考模型」即 differential 结构（实现 vs 参考模型的输出交叉），种子稿已有此设计，本轮未找到需要修订它的新证据。

_Property-based_：工具链实践见 Q4.3（proptest 失败持久化已核实）；「PBT 工业落地实证」（Goldstein et al., ICSE-SEIP 2024）本轮未能打开，进 ④——PBT 层对 AI 代码的增量判断（properties 是人必须承担的 oracle 写作成本，agent 适合生成候选 properties 再由过滤器验证）目前只能作为结构推断，缺乏可引用实证，同样进 ④。

**Q2.3 快照层的定位边界有官方文档级表述：它是变更检测层，不是正确性 oracle。**

Jest 官方文档的 FAQ 写明：快照回答的是「输出是否变化」，「rather than giving guidance to design the code in the first place」；快照工件应作为代码的一部分提交并在 code review 中评审【通说】（[Jest Snapshot Testing 文档](https://jestjs.io/docs/snapshot-testing)，访问于 2026-10-14）。这支持种子稿把 insta 快照放在 L0「回归网底」的定位，同时给出一个必须在文档里明写的纪律：agent 生成的快照**不证明行为正确，只固化行为现状**——若 agent 把快照当规格引用，即为旧报告 RQ3 所列「oracle 从实现反推」的破坏方式。

**Q2.4 分层对 AI 缺陷形态的分工判断（综合②各条，标注为推断）**：单元/快照层 = 回归网与变更检测，防「无声漂移」；集成/契约层 = AI 残余缺陷的主战场（权限、边界、接线），奖杯派的 confidence 论证在此与缺陷形态数据合流；property/metamorphic/differential 层 = 无规格 oracle 的三条可机器化路线，其中 metamorphic 有 FSE 2026 的 LLM 生成实证，differential 有 Csmith 级工业先例，PBT 有成熟工具链但缺新近落地实证；e2e 层的经典批评（慢、flaky、定位难【通说｜利益相关】，同上 Google 博客）在 agent 时代多了一条新理由——e2e 的语义模糊失败会毒化 agent 自修复闭环（flaky → agent 误修实现），此点与 HANDOFF 中 D1 出示稿「flaky 毒化自修复」的框架理由一致，属仓内已有判断，本轮无新增外部证据。

### Q3 覆盖率 + mutation 组合门禁的实践

**Q3.1 覆盖率门禁的工业成熟形态是「增量门禁 + 渐进抬升」，不是全局百分比。**

Google 官方覆盖率最佳实践：不设普适数字（内部指引 60% acceptable/75%/90% 仅供参考）；**per-commit 覆盖率门禁 99% 是合理目标、90% 是好的下限**；「新代码必须达标」+「boy-scout rule」渐进改善存量；门禁机制可选项包括全量 vs 仅新代码、硬数字 vs 相对前版 delta；并警告「pressure to hit the metric almost never yields the desired outcome」【通说｜利益相关：Google 工程博客】（[Code Coverage Best Practices](https://testing.googleblog.com/2020/08/code-coverage-best-practices.html)，访问于 2026-10-14）。其基础设施论文（ESEC/FSE 2019）给出规模与关键设计：每天为 10 亿行、7 种语言计算覆盖率，**可行动性的关键是把覆盖率落到 changeset 与 code review 层级**，附五年采纳率与 512 份开发者问卷【实证｜利益相关】（[Code coverage at Google](https://research.google/pubs/pub48413/)，访问于 2026-10-14）。工具侧，diff-cover 把「diff 行覆盖率」做成可门禁的 CLI（`--fail-under=N` 非零退出），其自述信条即「If you touch a line of code, that line should be covered」【通说】（[diff_cover README](https://github.com/Bachmann1234/diff_cover)，访问于 2026-10-14）。

**Q3.2 Mutation 门禁有跨生态的工具级先例，且都带 diff 范围与超时处理。**

PHP 生态 Infection 提供 `--min-msi` / `--min-covered-msi` 两个 CI 阈值参数，文档明写「Can be used with CI server to automatically control tests quality」；`--git-diff-lines` 只变异 PR 改动的行（官方建议用于 feature 分支 MSI 检查）；`--with-timeouts` 把超时变异体计为逃逸（防超时掩盖测试缺口）、`--max-timeouts` 设硬上限【通说】（[Infection Command Line Options](https://infection.github.io/guide/command-line-options.html)，访问于 2026-10-14）。Rust 生态 cargo-mutants 官方 book 给出两种 CI 模式的明确定位：**只检查 PR 改动代码产生的变异体**（快得多，「a good way to ensure that newly merged code is well tested」）与**全量检查**；示例 GitHub Actions workflow 的语义即「发现未捕获变异体则失败」【通说】（[cargo-mutants: Continuous integration](https://mutants.rs/ci.html)，访问于 2026-10-14）。

**Q3.3 「missed=0 + 逐个 skip 注释」模式的先例与降噪路线。**

cargo-mutants 官方文档把豁免做成三档一等公民：函数级 `#[mutants::skip]` attribute（「so that the skip is visible in the code」）、config 文件路径过滤（整模块不可测时）、regex 过滤（永久豁免某类函数，如 `Debug` 实现）；`--list` 可预览全部豁免结果【通说】（[cargo-mutants: Skipping untestable code](https://mutants.rs/skip.html)，访问于 2026-10-14）。结合 Q3.2 的 CI 示例，「missed=0 门禁 + skip 在源码中可见」是工具作者设计好的官方用法，不是本项目首创——但「skip 必须逐个附理由注释」这一纪律层未见任何来源明文，属本项目的增量纪律（其评审入口 = 旧报告 RQ3 的门禁自保护不变量：修改豁免清单的变更需更强审查）。

误报/噪声方面，旧报告 RQ3 已引 Google 六年把 productive mutant 从 15% 提到 89%（TSE 2021）——说明裸跑 missed=0 在大盘上不可行，降噪是落地前提。本轮新增两条可操作的降噪路线：(a) 按类豁免（cargo-mutants regex，如 Debug impls）；(b) LLM 辅助等价变异体预筛——Meta ACH 的检测 agent 经简单预处理达 precision 0.95/recall 0.96【实证·预印本｜利益相关】（来源同 Q1.2）。这直接支撑 HANDOFF D4 出示稿「误报率先按需校准再确认周跑硬门禁」的次序设计。

**Q3.4 Goodhart 对冲的组合结构**。综合本轮与旧报告来源，对冲不是单点而是组合：覆盖率防「没测到」（低覆盖率保证大面积未测——Google 博客，【通说｜利益相关】），mutation 防「测了没断言」（Google 博客明指 mutation testing 是评估断言充分性的更好技术），diff 范围防「全仓注水」（touch-it-cover-it），ratchet 防「基线倒退」，豁免清单评审防「绕过门禁」（旧报告 RQ3 门禁自保护）。「覆盖率数字做放行证书」的 Marick/Papadakis 批评（旧报告已覆盖）针对的是孤立百分比门槛；「增量覆盖门禁 + missed=0（带豁免）+ 豁免评审」的组合在结构上回应了每一条。

### Q4 快照 / 回归语料库治理

**Q4.1 快照确认仪式化（「勾了等于看了」）的对策：工具生态已有三层机制。**

第一层是流程强制：Jest 在 CI 上默认不写快照（新快照在 CI 不应通过），快照必须随代码提交并在 code review 中评审；文档明文要求「treat snapshots as code」，并点名要对抗的退化模式——「fight against the habit of **regenerating snapshots when test suites fail instead of examining the root causes** of their failure」【通说】（[Jest Snapshot Testing 文档](https://jestjs.io/docs/snapshot-testing)，访问于 2026-10-14）。第二层是交互式逐个确认：cargo-insta 的 `review` 命令逐个快照展示 diff、逐个人工 accept/reject/skip，跳过则新快照文件留在原地待决；`pending-snapshots` 输出待审清单供编辑器集成【通说】（[cargo-insta 文档](https://insta.rs/docs/cli/)，访问于 2026-10-14）。第三层是消灭确认动作本身：expect-test 把快照内联进测试源码（`UPDATE_EXPECT=1` 就地修补），快照 diff 即代码 diff，评审仪式自然并入正常 diff 评审而非独立仪式；其自述主客户为 rust-analyzer【通说】（[expect-test crate 文档](https://docs.rs/expect-test)，访问于 2026-10-14）。

**Q4.2 快照膨胀的对策**：Jest 文档建议快照保持短小聚焦、用描述性命名（好名字让评审者能看出「过时的快照哪个才是对的」），并点名 `eslint-plugin-jest` 的 `no-large-snapshots` 与 `snapshot-diff` 两个辅助工具【通说】（同上 Jest 文档）。insta 侧提供：redactions/filters 机制（对动态字段做归一化，文档目录确认存在该两章；细节本轮未逐节打开）、`--unreferenced` 清理孤儿快照（`auto` 模式 = CI 中报错、本地删除）【通说】（[insta Settings 文档](https://insta.rs/docs/settings/)、[cargo-insta 文档](https://insta.rs/docs/cli/)，均访问于 2026-10-14）。insta 的更新模式由 `INSTA_UPDATE`（auto/always/new/unseen/no）与 `INSTA_FORCE_PASS` 控制——CI 侧设 `no` 即把「未评审快照」变成硬失败，与 Jest 的 CI 不写快照同构【通说】（同上 insta Settings 文档）。

**Q4.3 失败语料固化为回归集：两条官方最佳实践，均已在 Rust 生态可用。**

proptest 官方文档：发现的失败用例以 **seed** 形式持久化到 `proptest-regressions/` 目录，后续运行**先重放已固化用例再生成新用例**；官方明确建议把这些文件提交进版本控制，使 CI 与协作者同样重放；选 seed 而非序列化值的理由是对 strategy 演化鲁棒【通说】（[proptest book: Failure Persistence](https://altsysrq.github.io/proptest-book/proptest/failure-persistence.html)，访问于 2026-10-14）。OSS-Fuzz 的「理想集成」清单给出 fuzz 侧的对应纪律：fuzz target 由代码所有者维护在源码仓内、随其余测试一起构建（防 bit rot）；seed corpus 进版本控制并**定期用「曾触发 bug 或触达新代码」的输入扩充**；fuzz target 应经 standalone driver 跑语料作为项目常规回归测试（建议挂 sanitizers）【通说｜利益相关：Google 项目文档】（[Ideal integration with OSS-Fuzz](https://google.github.io/oss-fuzz/advanced-topics/ideal-integration/)，访问于 2026-10-14）。CI 侧的边界形态：rust-fuzz book 建议把 fuzz target 在 CI 里做有界冒烟（示例 300 秒/target），失败时上传 `fuzz/artifacts` 供事后调试【通说】（[Rust Fuzz Book: Fuzzing in CI](https://rust-fuzz.github.io/book/cargo-fuzz/ci.html)，访问于 2026-10-14）。

**Q4.4 小结**：快照治理的退化风险（仪式化确认、膨胀）在工具文档层有完整对策链——CI 禁止自动更新、逐个 diff 评审、内联化把确认并入代码评审、孤儿清理、动态字段归一化；proptest/fuzz 语料固化的「失败即回归资产」纪律有 proptest 官方与 OSS-Fuzz 官方双重先例，且都明示「语料进 VCS」。这些直接支撑 D5①（seed 可重放纪律）与 D1 快轨设计（固化语料的回归重放属于快轨，长程新探索属于慢轨）。

---

## ③ 对 Metis 种子稿 D1–D8 的映射判断

总览：本轮发现**没有任何一条动摇 D1–D8 的推荐方向**；对 D1/D3/D4/D5 各提供了工业先例、数值锚点或落地件；D2/D6/D7/D8 本轮范围外，无新增证据，维持出示稿。

| 议题                      | 判断                   | 关键依据                                                                                                                                            |
| ------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1 快慢双轨门禁           | **支持（强化）**       | Google per-commit 覆盖门禁 90%/99% 数值锚；Infection `--git-diff-lines` 与 cargo-mutants PR 模式的 diff 范围快轨先例；rust-fuzz 300s 冒烟先例       |
| D2 machete/outdated       | 范围外，无映射         | —                                                                                                                                                   |
| D3 覆盖率地板线 ratchet   | **支持（强化）**       | 「增量门禁 + boy-scout 渐进 + 无普适数字」= Google 官方实践；diff-cover `--fail-under` 为 ratchet 机制件；「数值实测钉入」与 Google「按业务定」一致 |
| D4 cargo-mutants missed=0 | **支持 + 两条 nuance** | 官方 CI 示例即 missed=0 语义；skip 三档豁免为先例；ACH 的 0.95 精度等价变异体预筛为降噪路线；nuance 见下                                            |
| D5 replay 维度            | **支持（强化）**       | proptest-regressions 进 VCS 官方建议 = seed 纪律先例；OSS-Fuzz corpus 治理 = 失败语料固化先例；Jest FAQ 给 D5② journal 快照的定位边界               |
| D6 压测基准               | 范围外，无映射         | —                                                                                                                                                   |
| D7 Kani/Lean 分工         | 范围外，无映射         | —                                                                                                                                                   |
| D8 F 类故障注入           | 范围外，无映射         | —                                                                                                                                                   |

**D1（快慢双轨）**：「快 = per-commit 有界/增量、慢 = 全量/长程」是本轮多个来源的共同形态：Google 把覆盖率可行动性的关键放在 changeset 层级【实证｜利益相关】；Infection 官方建议 feature 分支用 `--git-diff-lines` 跑 MSI【通说】；cargo-mutants 把「PR 改动代码的变异体」列为第一种 CI 模式【通说】；rust-fuzz 把 CI fuzz 定为有界冒烟【通说】。一个供拍板的细化：出示稿把 cargo-mutants 整体放慢轨，而两个生态的官方文档都把 **diff 范围 mutation** 设计为 PR 级用途——若慢轨噪声校准顺利，存在「快轨加 diff-scoped mutation」的升级路径（成本：需实测 Rust 侧 diff mutation 的耗时与噪声，当前无数据，仅登记为可能性）。

**D3（覆盖率 ratchet）**：出示稿「不设全局百分比、内核名单设地板线、实测钉入、只升不降、防注水归 D4」与 Google 官方实践逐条同构；diff-cover 提供了「touch-it-cover-it」增量门禁的机制件。可补充进定稿的一个机制细节：ratchet 的机器判定有两种实现——全仓百分比不降（存量债冻结）与 diff 行覆盖达标（增量零债），Google 博客把两者并列为可选机制【通说｜利益相关】；对 agent 流，后者更适合做 per-commit 快轨件（判定范围小、不依赖全仓基线稳定）。

**D4（missed=0 + skip 注释）**：先例核实结果——「uncaught mutant 则 CI 失败」是 cargo-mutants 官方 workflow 示例的语义，「skip 在源码中可见」是官方豁免设计，跨生态有 Infection `--min-msi` 同构件；**「missed=0」作为硬门禁不是本项目首创，但「skip 逐个附理由注释 + 豁免清单评审」的治理层未见先例，是本项目的增量纪律**。两条 nuance：(a) 学术侧对「mutation score 作放行证书」的批评（旧报告 RQ3 已引 Papadakis/Just 争议）针对的是百分比门槛的统计失真；missed=0 + 显式豁免的结构不依赖 score 的代表性，绕开了该批评，但代价是豁免清单成为新的攻击面——其评审必须挂在旧报告的门禁自保护不变量下（修改 skip 的变更触发更强审查）。(b) 误报率先校准的次序正确且必要：Google 85% unproductive 起点（旧报告）说明裸跑不可行；本轮新增的 ACH 数据说明 LLM 预筛等价变异体可达 0.95 精度，为「agent 辅助分流 skip 候选、人抽查 skip 清单」提供了技术可行性证据【实证·预印本｜利益相关】。

**D5（replay 维度）**：seed 固化纪律（①）有 proptest 官方先例且工具内建；journal 序列快照（②）的工具链成熟，但必须吸收 Q2.3 的定位纪律——快照证明「行为未变」而非「行为正确」，文档需明写，防止 agent 把快照绿当正确性证据；全套 DST 扩展位（③）本轮无新增证据，旧报告 RQ3 的 DST 论述继续有效，触发器设计维持。

**跨议题的一个新增输入（Q1 结论 → 任务 8 定稿）**：「人写/审测试、agent 写实现」的分工有实证（TENET +9.49pp、Mathews & Nagappan、WebApp1K）与工业实践（spec-kit、TestGen-LLM 过滤器架构）双侧支持；且 LLM 测试的 mutation score 腰斩于覆盖率（ULT：40–50%）说明**凡 agent 自写测试处，D4 式 mutation 校验是唯一有实证支撑的牙齿审计**。建议定稿时把「内核 crate 的测试来源」提升为明文分工变量：人写/审测试（规格属性强）vs agent 写测试 + mutation 门禁（回归属性强），两者可混用但必须在 crate 级显式声明。此为本简报基于 Q1 证据链的【推断】级建议，供拍板。

## ④ 未确认清单

1. **Goldstein et al.《Property-Based Testing in Practice》（ICSE-SEIP 2024，Jane Street/Keybase 等 PBT 工业落地实证）**：dl.acm.org 被 Cloudflare 403，UPenn/作者页猜测路径 404，Semantic Scholar 429。PBT 层对 AI 代码的增量判断（properties 写作成本、agent 生成 properties 的可行性）目前缺可引用实证。**确认方法**：经 ACM 订阅或作者主页获取原文；或直接做仓内 spike（L1 已立，实证需求不阻塞拍板）。
2. **「异构模型交叉测试」的有效性证据**（模型 A 写实现、模型 B 写测试，错误是否去相关）：本轮未找到任何对照实验；已有证据（AgentCoder 角色分离、ConVerTest 自一致性投票）均为同模型流程级解耦。**确认方法**：定向学术检索（关键词建议：cross-model test generation / model diversity oracle / independent LLM test generation）；若仍空白，该问题本身可作为 Metis 的仓内实验（双模型流水线 A/B 测缺陷逃逸率）。
3. **「同一 agent 的测试镜像其实现错误」的错误相关系数测量**：只有定性问题陈述（ConVerTest 的 bug propagation、vibe-TDD 研究的 untested decision points），无量化。**确认方法**：同 2，属同一缺口的两个侧面。
4. **McKeeman 1998《Differential Testing for Software》原文**：cs.dartmouth.edu 403。differential testing 的概念锚点本轮以 Csmith README（一手项目文档）替代，概念成立不受影响；原始文献的定义句未核实。**确认方法**：DTIC/ACM 渠道或图书馆。
5. **Barr et al. 2015《The Oracle Problem in Software Testing: A Survey》（IEEE TSE）**：IEEE Xplore 未尝试（历史经验为付费墙+反爬），arXiv 无收录（API 检索未命中）。oracle 分类法（specified/pseudo/derived oracle）本轮以 metamorphic 综述与 MR-Coupler 的相关表述替代支撑。**确认方法**：IEEE 订阅或 KCL/UCL 机构仓储。
6. **Chen et al.《Metamorphic Testing: A Review of Challenges and Opportunities》（ACM CSUR 2018）**：arXiv API 限流期未能定位可打开版本；metamorphic 的主流地位论断已用 2024 年 workshop 综述（2406.05397）支撑。**确认方法**：ACM 订阅。
7. **LLM 单元测试生成的专门综述**（用于校验 Q1.1 缺陷模式清单的完备性）：arXiv 检索后期持续 429 未能定位；已用 Wang et al.《Software Testing with LLMs》（IEEE TSE，102 篇研究综述）的总盘结论部分替代——test case preparation 是 LLM 测试研究最集中的任务【通说/实证·综述】（[Software Testing with Large Language Models: Survey, Landscape, and Vision](https://arxiv.org/abs/2307.07221)，访问于 2026-10-14），但该综述未提供缺陷模式的细分数据。**确认方法**：限流恢复后补检。
8. **insta redactions/filters 两章细节**（动态字段归一化的具体机制）：文档目录确认章节存在，本轮未逐节打开。**确认方法**：定稿引用前补开 `insta.rs/docs/redactions/` 与 `/filters/`。
9. **SWE-bench 以 PR 测试作判定 oracle 的方法细节**：仅打开摘要页，摘要未展开评测方法。**确认方法**：打开论文 HTML 版 §3。

## ⑤ 是否需要再委托一轮外部深度调研

**判断：不需要。**

理由四条：

1. **四个问题的核心论断均已被一手来源覆盖且方向一致**：Q1 有 TestGen-LLM/ACH（生产部署级）+ 一组 TDD-for-agents 实证 + ConVerTest/AgentCoder（预印本级）；Q2 有分层通说三源头 + MR-Coupler（FSE 2026）+ Csmith；Q3 有 Google 双件（博客 + ESEC/FSE 2019 论文）+ Infection/cargo-mutants/diff-cover 三生态工具先例；Q4 有 Jest/insta/expect-test/proptest/OSS-Fuzz 五件官方文档。没有发现任何与 D1–D8 推荐方向冲突的证据，也没有发现需要推翻旧报告 RQ3 不变量清单的证据。
2. **未确认清单的 9 项均为「补强」而非「决策依赖」**：Barr 综述、McKeeman、CSUR 有功能等价替代来源已支撑论断；PBT-in-practice 影响的是论证丰满度，不影响 L1/L2 已立的决策；insta 两章是引用前补读级别的事务。
3. **真正的证据空白（④之 2/3：异构模型交叉、错误相关系数）属学术前沿空白**——再委托一轮网络调研大概率仍找不到（本轮已用多组检索式交叉验证其缺失）。该缺口的正确填充方式不是外部调研而是**仓内实验**：Metis 的 1+N 流水线本身就是跑双模型 A/B 的天然场地，可在任务 8 定稿后立一个小型度量 spike（双模型交叉测试 vs 单模型自测的缺陷逃逸率对比）。
4. **时效风险可控**：本轮引用的 2024–2026 来源（TestGen-LLM、ACH、TENET、spec-kit 等）与任务 8 拍板周期（当前会话即续）同处一个时点截面；D1–D8 不依赖任何「以月为单位变化」的数字（评测榜单、厂商份额均未引用）。

若维护者仍希望补强，建议的**窄口径**追加项（不需要整轮委托，单次检索可完成）：(a) ④-1 的 PBT-in-practice 原文精读；(b) ④-7 的 LLM 测试生成专门综述定位；(c) insta redactions/filters 两章补读。三项合计约一次会话的工作量，可并入任务 8 定稿会话执行。

---

_简报完。来源 25 项全部经 fetch 实际打开（含 arXiv 官方 API 摘要）；访问日期均为 2026-10-14；失败与受限来源见 ① 实况与 ④ 清单。_
