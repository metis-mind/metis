> 状态：point-in-time 调研简报（2026-10-14，仓内子代理执行；来源纪律：全部实际打开、未确认进清单）。任务 8 测试实践补差调研三份之三，主题 = 1+N 并行开发模式下的测试工程与质量治理；结论 = 不需再委托外部深度调研。姊妹篇 = [testing-practice-oracle.md](./testing-practice-oracle.md)、[testing-practice-formal-verification.md](./testing-practice-formal-verification.md)；上位报告 = [review-economics-research.md](./review-economics-research.md)。正文逐字未改；原文出处 = `scratch/测试实践调研/d13-1N并行测试工程.md`。

# D13 调研简报：1+N 并行开发模式下的测试工程与质量治理

> 状态：调研简报（scratch 工作区，未入库 docs/research/）。调研日期 2026-10-14。
> 委托边界：补齐仓内《评审经济学调研报告》（scratch/评审经济学调研报告/）RQ5/RQ7 未展开的「测试工程」维度；评审路由、worktree 生态、人因容量等已有内容不重复。
> 对账目标：测试策略种子稿 `docs/design/testing-strategy.md` 与任务 8 议题 D1（快慢双轨门禁强度）、D4（cargo-mutants missed=0 门禁），及 HANDOFF 登记的缺口「门禁变更 HARD 人审门」「测试套件膨胀」。

---

## 1. 调研范围与方法

五个问题按委托优先级：① 并行 agent 流的 CI/门禁经济学（merge queue、批量测试、双轨先例、CI 成本）；② 测试套件膨胀治理（测试评审、删除纪律、mutation 体检频率）；③ flaky 测试治理（检出/隔离/隔离毒化防御）；④ 回归资产治理（快照、fuzz 语料、golden 文件）；⑤ 门禁变更保护（CI 配置/lint 规则的强审通道）。

方法：仓内两份基线材料先读（种子稿全文 + 评审经济学报告 RQ5 L294–351、RQ7 L351–425，另有 HANDOFF.md §2 任务 8 记录确认 D1–D8 编号），外部来源一律以 fetch 实际打开的页面为准，每条实质性论断行内标注【实证】/【通说】/【争议】/【观点】，厂商来源追加「｜利益相关」。

网络可达性实况（如实记录）：docs.github.com、research.google、testing.googleblog.com、engineering.atspotify.com、llvm.org、google.github.io、kubernetes.dev、rustc-dev-guide.rust-lang.org、research.tudelft.nl、microsoft.com、insta.rs、bors.tech、aviator.co、trunk.io 可达；uber.com 返回 406（拒绝抓取）；graydon2.dreamwidth.org 返回 403；engineering.fb.com 旧文章 404（站点迁移，改用 arXiv 论文版替代）；usenix.org 目标论文 404 且站内搜索无果；DuckDuckGo HTML 检索触发验证码、Bing 返回无关结果——**搜索引擎在本环境不可用**，定位失败一律进 §4 未确认清单。

本轮共实际打开来源 13 个（含 2 个失败后放弃的目标）。分级约定：官方文档对自家机制的描述记【实证·一手】但注意其只证明「机制存在与参数」，不证明「效果」；厂商工程博客的数据记【实证｜利益相关】；学术论文记【实证·独立学术】（有厂商作者时注明）。

---

## 2. 发现（按五个问题）

### Q1 并行 agent 流的 CI/门禁经济学

**merge queue 的机制形态已收敛，且 bors 生态正向 GitHub 原生收敛。** GitHub merge queue 的公开参数面：build concurrency（1–100，节流并发 CI 构建总量）、merge limits（同批合并的最小/最大 PR 数与成组超时）、「only merge non-failing pull requests」开关（关闭时允许组内最后一个 PR 通过即整批过——官方明示这是给间歇性失败测试的泄压阀）、插队导致全部在途组重建【实证·一手（厂商文档）】（[GitHub Docs: Managing a merge queue](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue)，访问于 2026-10-14）。bors 官方站首页明示其公共实例正在退役、让位 GitHub Merge Queues；bors 的核心价值主张一句话即「单独测每个 PR 不够，必须测合并后的组合」【实证·一手（项目文档）】（[bors.tech](https://bors.tech/essay/2017/02/02/pitch/)，访问于 2026-10-14）。对 1+N 的含义是结构性的：N 个 agent 并发推同一主干时，「两两组合的正确性」无法由任何单 agent 自证，必须由队列层机器化——这是并行扩容后必须补的门禁件，与评审带宽无关。

**吞吐经济学的具名参数：rust-lang 是「快慢双轨」最完整的一手先例，而且是三轨。** PR 每次 push 跑轻量套件（约 40 分钟，Linux 单平台 lint + 部分测试）；合并前由 bors 串行队列跑全量 auto 套件（数十个 job、数小时）；另有按需 try build（指定 job 模式、perf/Crater 联动）。文档明示经济学约束：「全量约 2 小时、串行测试 → 每天最多合并约 10 个 PR」，对策是 rollup——团队成员人工判断把一批琐碎 PR 合成一个 rollup 一次测试，「以减少 CI 成本与合并延迟」【实证·一手（项目文档）】（[rustc-dev-guide: Testing with CI](https://rustc-dev-guide.rust-lang.org/tests/ci.html)，访问于 2026-10-14）。同页还给了两个门禁工程细节：新 CI job 可加 `optional-` 前缀配 `continue_on_error` 先非阻塞运行观察期（对 flaky 易感的新检查特别有用）；工具类子模块允许失败（toolstate 记录、release 前一周转强制）——「门禁分级准入 + 允许失败名单有自动收紧触发器」两件都是具名先例【实证·一手（项目文档）】（同上）。

**批量测试/选择层的工业先例与参数。** Kubernetes 的 Tide 把通过两阶段评审的 PR 攒成「tide pools」成批跑测试、成批合并【实证·一手（项目文档）】（[Kubernetes OWNERS 文档](https://www.kubernetes.dev/docs/guide/owners/)，访问于 2026-10-14）。Meta 单仓规模下「每次变更跑全部受影响测试不可行」，预测性测试选择上线后测试基础设施总成本减半，同时保证 >95% 的单测试失败与 >99.9% 的坏变更仍被报告，且模型显式吃进 flaky 非确定性【实证·学术（作者为 Facebook 员工）】（[arXiv:1810.05286 Predictive Test Selection](https://arxiv.org/abs/1810.05286)，访问于 2026-10-14）。Microsoft 的 THEO 用成本模型动态跳过测试（预期运行成本超过预期删除成本则跳过），对三个主力产品回放历史开发期：测试执行量减 50%、每年省数百万美元、产品质量不降【实证·学术（Microsoft Research）】（[The Art of Testing Less without Sacrificing Quality, ICSE 2015](https://www.microsoft.com/en-us/research/publication/the-art-of-testing-less-without-sacrificing-quality/)，访问于 2026-10-14）。Google 的 pre-submit（门禁提交，跑受影响测试）/post-submit（全量、决定可否发布）双层切分见 Q3 引用源【实证｜利益相关】。

**「慢门禁被绕开」的实测证据：绕开发生在人与制度两侧。** 人侧：Google 实测 84% 的 pass→fail 转移是 flaky 引起，「无视 flaky 测试里的合法失败是常态」，官方用波音 737 驾驶舱报警被飞行员无视作类比——报警疲劳直接等价于门禁失效【实证｜利益相关：Google 工程博客，数据为其内部遥测】（[Flaky Tests at Google and How We Mitigate Them](https://testing.googleblog.com/2016/05/flaky-tests-at-google-and-how-we.html)，访问于 2026-10-14）。制度侧：rust-lang 的 `optional-` 非阻塞准入期与 GitHub merge queue 的「允许失败 PR 成组」开关，都是把「会被绕开的检查」显式降级的制度化先例【实证·一手】（同上两文档）。CI 成本失控的具名金额证据本轮只有 Microsoft THEO 的「每年数百万美元」量级（节省侧）与 rust-lang 的「每天约 10 个合并」吞吐墙（约束侧）；Travis CI 2020 终止开源免费层事件未能打开原文，进未确认清单。

**agent 流的特化采用：merge queue + flaky 隔离已被打包成商品，但无独立采用率数据。** Trunk 同站同时售卖 Merge Queue 与 Flaky Tests 两款产品（自动隔离 flaky、不阻塞 main），Aviator 博客 2026 年的选题（「Shipping Code Without Human Verification」「How to Review AI-Generated Pull Requests at Scale」）显示该赛道正围绕 AI 产出流营销【实证（产品存在与定位）｜利益相关】（[Trunk Flaky Tests](https://trunk.io/flaky-tests)、[Aviator Blog](https://www.aviator.co/blog/)，均访问于 2026-10-14）。厂商把这两件打包卖，旁证了「高吞吐产出流 → 排队 + flaky 治理」是公认痛点组合；但「agent 团队实测收益」未见厂商外数据【通说·否定发现，本轮检索范围】。

### Q2 测试套件自身的膨胀治理

**规模锚点与 mutation 体检的工业频率答案。** Google 代码库约 20 亿行、每日执行超 1.5 亿个测试；在此规模下传统全量 mutation testing 不可行，其落地形态是：**增量（只变异 code review 中变更的行）+ 过滤（剔除无关变异体、限制每行/每评审的变异体数）+ 按算子历史表现选型**，服务于 24,000+ 开发者、1,000+ 项目的评审流，产出变异体量比全量低数量级且 actionability 更高【实证·学术（作者含 Google 员工）】（[Practical Mutation Testing at Scale: A view from Google, TSE 2021](https://research.google/pubs/practical-mutation-testing-at-scale-a-view-from-google/)，访问于 2026-10-14）。即：工业界对「测试套件的体检频率」的答案是**随评审增量常开、强过滤**，而非「定期全量跑」——这个答案的前提是被测体量超出全量可行性，小仓不必照抄（对账见 §3）。

**「谁审测试」：有实证结论，无 1+N 路由先例。** Spadini 等（ICSE 2018）分析 30 万+ Gerrit 评审并访谈 12 名开发者：测试代码的评审方式与生产代码「非常不同」，测试代码普遍被视为次要；评审工具内的导航（在测试与生产文件间跳转对照）是主要障碍【实证·独立学术】（[When Testing Meets Code Review, TU Delft 页面](https://research.tudelft.nl/en/publications/when-testing-meets-code-review-why-and-how-developers-review-test/)，访问于 2026-10-14）。在公开文献范围内，**没有任何具名团队公开过「测试走单独评审通道/评审人」的机制**——评审经济学的既有路由变量（作者身份、风险目录）不区分测试与代码【通说·否定发现（本轮检索范围；仓内 RQ5 的否定发现同构）】。

**配比纪律与价值框架（可转为 agent 产出纪律）。** Google 工程实践的通行配比建议 70/20/10（单元/集成/E2E），反模式具名：冰淇淋筒（E2E 为主）与沙漏（缺集成层）【通说｜利益相关】；同文的价值框架「失败的测试不直接产生用户价值，只有修 bug 产生」可直接改写为 1+N 纪律：测试数量不是产出 KPI，测试的唯一价值通路是「抓到 bug → 修复」闭环【观点｜利益相关】（[Just Say No to More End-to-End Tests](https://testing.googleblog.com/2015/04/just-say-no-to-more-end-to-end-tests.html)，访问于 2026-10-14）。评论区有反驳认为该配比不可普适（E2E 对复杂系统不可替代），宜作参考系而非定律【争议·评论区，弱证据】（同上页面评论区）。

**膨胀的自然稳态含恒定 flaky 率。** Google 自述 flaky 的插入率约等于修复率，全库约 16% 的测试有 flaky 史【实证｜利益相关】（同 Q3 Google 2016 源）。含义：agent 无限产测试时，「膨胀治理」与「flaky 治理」是同一个问题——不删测试，flaky 存量随套件规模线性增长。

**删除/合并侧的工具先例存在，团队级纪律未找到。** insta 的 `--unreferenced` 选项提供孤儿快照的机器判定与四档处置（ignore/warn/reject/delete；`auto` = CI 中 reject、本地 delete）【实证·一手（工具文档）】（[Cargo Insta CLI](https://insta.rs/docs/cli/)，访问于 2026-10-14）。测试代码本体的删除/合并纪律（「多久审一次哪些测试该删」），未找到具名团队公开实践【否定发现，进 §4】。

### Q3 flaky 测试治理

**基线数字（2016–2019，仍是被引基准）。** Google：全部测试运行约 1.5% 报 flaky，约 16% 的测试有 flaky 史，84% 的 pass→fail 转移涉 flaky【实证｜利益相关】（Google 2016，同上）。Spotify：仅上线「每团队可见的 flaky/快慢测试表」、不做任何其他干预，两个月内全组织 flaky 率从 6% 降到 4%——可视化本身是干预手段【实证｜利益相关】（[Test Flakiness – Spotify Engineering](https://engineering.atspotify.com/2019/11/test-flakiness-methods-for-identifying-and-dealing-with-flaky-tests/)，访问于 2026-10-14）。

**检测/隔离/quarantine 机制谱系（十年收敛到同一套件）。** ① 重跑钝化：Google 的「标记 flaky 后连续失败 3 次才报失败」（官方自承副作用：15 分钟集成测试的合法失败发现延迟变 45 分钟+）【实证｜利益相关】；② 自动隔离：flaky 率超阈值自动移出关键路径 + 自动建 bug 单（Google 自承「可能掩盖真实竞态」）【实证｜利益相关】；③ 可视化区分 flaky vs 基础设施故障（Spotify Odeneye 散点图）与 PR 期预检（Spotify Flakybot：合并前高频重跑新测试验证不 flaky）【实证｜利益相关】；④ 新门禁准入观察期（rust-lang `optional-` 先例，见 Q1）；⑤ 2026 年产品化形态：Trunk 的 auto-quarantine「隔离测试继续跑但不阻塞 main」+ 失败指纹分组 + 状态史追踪 + 工单集成【实证（功能存在）｜利益相关】（Trunk，同上）。agent 生成测试的 flaky 率实证：**未找到任何公开数据**【否定发现，进 §4】——这是本主题最重要的文献真空之一。

**flaky 毒化自修复闭环（HANDOFF D5 所指）的防御：无直接实证，只能从机制证据推。** 支持性事实链：quarantine 会掩盖真 bug（Google 自承）；flaky 失败与真失败在表层不可区分（Google 定义：同代码既过又挂）；人会无视反复误报（84% 数字 + 737 类比）。推论（本调研综合，标注【观点】）：agent 的自修复闭环必须先过「失败分类」闸—— flaky 失败允许的合法动作集合应收敛为「重跑 N 次判定 / 标记隔离 / 建 issue」，**不含修改产品代码**；只有「同代码稳定复现的失败」才允许进入改代码通路。该设计在公开文献中未见具名实现，属 Metis 需自证的首创面（与仓内 RQ5「明示人审预算属首创」同性质）。

### Q4 回归资产治理

**fuzz 语料的修剪是成熟自动化先例。** libFuzzer 官方文档：语料兼作回归检查（直接喂旧输入跑一遍即回归模式）；`-merge=1` 在保持完整覆盖率的前提下最小化语料；`-reduce_inputs` 默认开启（缩小输入同时保持特征集）；fork 模式把子进程语料回并主语料【实证·一手（工具文档）】（[libFuzzer 文档](https://llvm.org/docs/LibFuzzer.html)，访问于 2026-10-14）。ClusterFuzz（Google 生产 fuzzing 设施，30,000 VM 规模；截至 2022-05 在 Google 发现 25,000+ bug、在 550+ OSS-Fuzz 项目发现 36,000+ bug）内置：crash 自动去重、testcase 自动最小化、回归二分定位、bug 单自动开/关【实证·一手（项目文档）】（[ClusterFuzz 文档首页](https://google.github.io/clusterfuzz/)，访问于 2026-10-14）。即 fuzz 社区对「语料只增不减」的答案是**覆盖率保持下的自动修剪**，无需人审介入。

**快照资产的评审与清理机制。** insta 工作流 = pending snapshot（与存储参照有差异的快照）经 `cargo insta review` 逐条 diff 交互确认（accept/reject/skip）——快照变更被设计成「必须过一次显式确认闸」，与代码评审同构；孤儿快照（测试已删、快照残留）由 `--unreferenced` 机器判定并支持 CI 中 reject【实证·一手（工具文档）】（Cargo Insta CLI，同上）。golden 文件/快照语料的**团队级膨胀治理**（总量上限、定期整仓清理战役）未见具名公开实践【否定发现，进 §4】。

**执行期修剪 vs 资产删除的分野。** 证据厚的一侧是「资产不删、少跑」：Meta 预测性选择、Microsoft THEO 都是执行期经济学（Q1）；证据薄的一侧是「资产本体删除」：只有工具级机制（libFuzzer merge、insta unreferenced），无团队级治理文本。对 Metis 的含义：回归资产治理应先上工具级自动修剪（成本低、有先例），团队级删除纪律留作遥测驱动（先观测比率，再定阈值）——详见 §3。

### Q5 门禁变更保护

**GitHub 官方加固文档直接给出「门禁文件强审」的标准做法。** 「使用 CODEOWNERS 监控变更」一节：把 `.github/workflows` 目录加入 code owners，则这些文件的任何变更须指定 reviewer 批准后才能合并【实证·一手（厂商文档）】；同文档另有两个相邻机制：org/repo 级开关可禁止 GitHub Actions 创建或批准 PR（官方定性为「若无适当监督即合并则有安全风险」）；dependency-review action 可在 PR 上强制依赖变更审查【实证·一手（厂商文档）】（[Secure use reference — GitHub Actions](https://docs.github.com/en/actions/security-for-github-actions/security-guides/security-hardening-for-github-actions)，访问于 2026-10-14）。

**Kubernetes OWNERS 是「门禁即代码、门禁自审」的最完整具名先例。** OWNERS 文件按目录指定 reviewer/approver，两阶段（/lgtm 与 /approve 分离，至少两双眼）；关键自指条款：OWNERS 文件「适用于目录内一切，**包括 OWNERS 文件自身**」——改门禁配置本身就要过该目录 approver；emeritus 与活跃度清理（年贡献 <10 Devstats 即移出）防止门禁人名单腐化；别名机制弃用 GitHub Teams 的理由是「Teams 变更不可公开审计」【实证·一手（项目文档）】（[Kubernetes OWNERS 文档](https://www.kubernetes.dev/docs/guide/owners/)，访问于 2026-10-14）。

**「同一 PR 顺手降阈值」的具名防御实例：未找到以该名义公开者**【否定发现，进 §4】。机制等价物存在：上述 CODEOWNERS-on-workflows 与 OWNERS 自覆盖正是对该威胁模型的一般性防御（门禁文件变更被强制路由到 owner）；PostHog「StampHog 策略文件只允许人类修改」是同构条款（已在仓内 RQ5 覆盖，不重复展开）。反向对账证据：rust-lang 的 CI 配置（jobs.yml）修改走普通 PR 流程、无更强通道（rustc-dev-guide 明示「改 CI 就是改 jobs.yml 提交 PR」）【实证·一手（项目文档）】——说明「门禁变更强审」即使在大型开源项目也非默认配置，Metis 的 ASK-first 规则本就处于更严一档，缺的只是机器化执行。

---

## 3. 对 Metis 的对账判断

### 3.1 D1（快慢双轨门禁强度）：外部证据全面支持，且可补上四个参数化细节

双轨/多轨制有四个独立具名先例，形态高度同构：rust-lang 三轨（约 40 分钟 PR 轻量 / 数小时串行全量 / 按需 try）、Google pre-submit vs post-submit、Meta 选择层（成本减半）、Microsoft THEO（执行量减半）【均实证，出处见 Q1】。D1 推荐形态（快轨 `just ci` per-commit + 慢轨 `just deep` 按需/周跑 + 慢轨失败产物 = issue 不阻塞 commit）与该谱系一致，**维持推荐**。四个可补细节：

1. **慢轨失败产物 = issue 的设计与 Google 自动建单、Trunk 工单集成同构，成立**；建议补一句「issue 须带失败指纹（test 名 + 首次失败 commit + 复现命令）」，对应 Trunk 失败指纹分组与 rust-lang RLA（自动提取错误行贴 PR）的先例【实证，同上】。
2. **新门禁件先非阻塞观察期再进快轨**（rust-lang `optional-` 先例）：任何新加的慢轨/快轨检查先以「只记录不阻塞」身份跑若干周，flaky 率达标再转硬门禁——这是把「flaky 门禁会被绕开」的前馈防御写进流程而非靠自觉【实证，同上】。
3. **flaky 台账先行**：Spotify 证据表明仅「每团队可见的 flaky 表」两个月降 2 个百分点；Metis 快轨应尽早产出 flaky 率遥测（哪个测试、跨 run 的翻转率），作为双轨寿命的先行指标——Google 84% 数字说明快轨 flaky 失控 = 人与 agent 一起学会无视红，门禁名存实亡【实证，同上】。
4. **merge queue 登记为未来触发项**：当前直推 main + 无 PR 流程下不适用；1+N 并行后多个 agent 并发推 main，「组合正确性」回归（bors 的核心命题）。触发条件建议写死：同一工作日出现 ≥2 个 agent 工作流时，合并面必须引入「测组合」机制（GitHub merge queue 或等价物），否则语义冲突漏检率随 N 上升【实证（机制）+ 观点（触发条件设定）】。

### 3.2 D4（cargo-mutants missed=0 门禁）：方向成立，Google 证据提示补降级条款

Google 的工业答案是「增量 + 过滤、随评审常开」，其前提是被测体量（20 亿行、日 1.5 亿测试）使全量不可行【实证，同上】。Metis 仓小几个数量级，周跑全量 missed=0 在算力上可行，D4 推荐形态**维持**；但应吸收 Google 过滤思想补一条降级触发器：**当变异体总数 × 内核 crate 数使周跑时长超过预算（或误报率校准期发现 missed=0 的信噪比失控）时，转为「只变异 diff 触及行 + `#[mutants::skip]` 逐个注释理由」的增量模式**——这正是 Google 形态在小仓的投影，且与 D4 已有的「误报率先按需校准再确认周跑硬门禁」一脉相承。

### 3.3 缺口①：门禁变更 HARD 人审门——先例充足，落地形态须适配直推 main

先例链完整：GitHub 官方推荐 CODEOWNERS 覆盖 `.github/workflows`；K8s OWNERS 自覆盖 + 两阶段 + 活跃度清理；GitHub org 级禁 Actions 开 PR 开关【均实证，见 Q5】。**适配问题**：GitHub CODEOWNERS 依赖 PR 评审流程，Metis 直推 main 无 PR 流程，不能直接搬。建议落地形态（三选一，推荐①）：

1. （推荐）**pre-commit 钩子闸**：维护「门禁面文件清单」（`.github/workflows/`、justfile、`rust-toolchain.toml`、lint 配置、xtask `check-commit`、prek 配置），提交触及清单内文件时钩子要求显式人类确认标记（如 commit trailer `Gate-Change: approved-by-human` 或交互式确认），无标记则拒 commit——机器可判定、与现有 commit-msg 钩子同层，成本最低。
2. 慢轨对账器：`just deep` 增加「门禁面文件哈希对账」——记录门禁面文件的基准哈希，diff 时要求人工确认记录进 HANDOFF。缺点：事后闸，非事前闸。
3. 等 1+N 引入 merge queue/PR 流程后迁回标准 CODEOWNERS。缺点：远水不解近渴。

同时把 AGENTS.md 现有「ASK before touching CI workflow files」的口径从 workflows 扩到整个门禁面清单，与钩子共用同一份清单文件（SSOT）。

### 3.4 缺口②：测试套件膨胀——外部只有半截答案，Metis 须自设计

外部给了：配比纪律（70/20/10 金字塔【通说】）、价值框架（测试不产生价值、修 bug 才产生【观点】）、资产侧自动修剪工具（libFuzzer merge、insta unreferenced【实证】）、flaky 插入≈修复的稳态警告【实证】。外部没给：测试代码的删除/合并纪律、「谁审测试」的路由机制（否定发现）。建议（标注【观点】，为本调研综合推断）：

1. **遥测先行不立阈值**：测试行数/产品行数比、每 crate 测试数增长率进慢轨报告（诊断位，不设门禁）——对齐种子稿「覆盖率诊断用不设门槛」的既有哲学。
2. **删除驱动挂到 mutation 体检上**：D4 周跑的产出除 missed 外，「杀不死任何变异体且长期零失败」的测试清单作为删除候选——mutation 结果是测试价值的机器可判定代理，免维护者逐条评审。
3. **flaky 惯犯限期制**：flaky 台账中超过限期（如 4 周）未修复的测试移出快轨（隔离档），再超期删除——Google「插入≈修复」稳态的主动对策；隔离档语义同 Trunk「继续跑不阻塞」。
4. **agent 纪律条款**：同一 commit 新增测试须声明其守护的行为/不变量（无声明 = 注水嫌疑），快照变更必须过 `cargo insta review` 类显式确认闸（工具先例在手）。

### 3.5 缺口③（呼应 HANDOFF D5）：flaky 毒化自修复的防御设计

综合 Q3 证据链，建议进种子稿 §3 新增 flaky 治理行（三件套）：**失败分类闸**（agent 见测试失败，第一步是 flaky 判定——重跑 N 次/查台账，而非直接改代码）；**合法动作白名单**（flaky 失败允许：重跑、标记隔离、建 issue；禁止：改产品代码；稳定复现失败才允许进修复通路）；**隔离有复活路径**（隔离 = 工单驱动修复，修复后回快轨；防 Google 自承的「掩盖真实竞态」）。其中分类闸与白名单在公开文献无具名先例，属首创面，应以慢轨遥测（flaky 率、误改回滚率）验证有效性后再硬化。

### 3.6 缺口④：回归资产入库纪律

直接可落的工具先例：fuzz/proptest crash 语料进仓时定期 `merge` 最小化（libFuzzer 先例，Rust 侧 bolero/cargo-fuzz 同语义）；insta `--unreferenced=auto` 进 `just ci`（CI 中 reject 孤儿快照、本地 delete）；语料入库纪律一句话：「语料条目须被现行测试引用，孤儿资产机器可判、自动拒绝」【实证（工具）+ 观点（纪律措辞）】。

---

## 4. 未确认清单（未打开或未找到的，一律不得引用为事实）

1. **Uber「Keeping Master Green at Scale」**：uber.com 406 拒抓、USENIX 站 404——其投机执行（speculative execution）与批量 bisect 细节未核实。若补开，价值在 Q1 的队列吞吐参数。
2. **Travis CI 2020 终止开源免费层事件**（CI 成本/滥用具名案例）：搜索引擎在本环境不可用（DDG 验证码、Bing 垃圾结果），未打开原文。
3. **Bors「Not Rocket Science Rule」graydon 原文**（graydon2.dreamwidth.org 403）：仅 bors.tech 官方 pitch 页可用，原文金句未逐字核。
4. **agent 生成测试的 flaky 率实证**：本轮检索范围内公开文献为零——否定发现本身是结论，但「确认不存在」需更强检索（学术库）才能坐实。
5. **测试代码评审路由机制（「谁审测试」）**：Spadini 2018 描述了现状（测试评审与代码评审不同、被视为次要），无 1+N 下的具名机制；是否有 2024–2026 新实践未经学术库检索坐实。
6. **团队级快照/golden 语料膨胀治理**（总量上限、定期清理战役）：未见具名公开实践。
7. **「同一 PR 顺手降阈值」的具名事故或防御案例**：未见以该名义公开者；只有一般性机制等价物（CODEOWNERS/OWNERS 自覆盖）。
8. **ClusterFuzz 语料修剪（corpus pruning）的机制细节页**：公开文档站无此页（404，导航全表已核），只有首页特性列表佐证 minimization/去重/二分/自动开关单。
9. **merge queue 产品在 agent 团队的独立采用率/收益数据**：仅厂商自述（Aviator/Trunk），无第三方。
10. **Meta engineering.fb.com 原博客 404**（站点迁移）：已用 arXiv:1810.05286 论文版替代，博客独有的工程细节（部署时间线等）未核。

## 5. 是否需要再委托一轮外部深度调研

**判断：不需要。**

理由：① 五个调研问题中四个（Q1/Q3/Q4/Q5）已拿到可操作的具名先例与参数，且相互交叉印证（双轨制四例、flaky 机制谱系五件、门禁自审两例），证据饱和度足够支撑任务 8 定稿；② 两个真空（agent 生成测试的 flaky 率、测试评审路由机制）属于**公开文献层面的否定发现**——再委托一轮外部调研大概率仍为空，因为这两个问题本质上要求「正在跑 1+N 的团队公开遥测」，而这样的团队目前不写这类报告；③ 这两个缺口恰恰只能由 Metis 自己在 1+N 试点期用遥测回答（flaky 率台账、误改回滚率、测试/产品行数比），应转化为试点期的数据采集项，而不是再调研。

残留可补项（不需整轮委托，定点即可）：未确认清单第 1–3 条（Uber 论文、Travis 案例、graydon 原文）属锦上添花，可在任务 8 定稿前以约 30 分钟定点补开；第 4–5 条若在 ADR-0031 冻结前想坐实「首创性」声明，可做一次学术库（Google Scholar / Semantic Scholar）定向检索，预计 1 小时内。

若维护者仍想再委托，缺口问题清单（按价值排序）：① 2024–2026 有没有团队在公开渠道（工程博客/会议 talk）报告过 AI 生成测试的 flaky 率或删除率遥测；② Uber SubmitQueue/投机执行的论文细节与现行参数；③ 「门禁文件变更被降级合并」的具名事故复盘（任何行业）；④ 学术侧 test suite minimization/reduction 在 2020 年后有无工业落地复盘。
