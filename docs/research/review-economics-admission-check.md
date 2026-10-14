> 状态：point-in-time 验证记录（2026-10-14）。评审经济学调研报告（[review-economics-research.md](./review-economics-research.md)）入库前的独立来源抽查：样本框由仓内按决策依赖度分层划定（19 条），执行者为与报告无 authorship 关系的子代理；报告自带验证记录仅作对照。结果：19/19 通过、0 编造。正文逐字未改；原文出处 = 调研交付包 `入库抽查.md`，正文中「报告」的路径指 scratch 原件，入库正本即上链文件。

# 入库抽查：评审经济学调研报告 19 条行内来源独立核实

> 抽查人：独立子代理（与报告无 authorship 关系）｜抽查日期：2026-10-14
> 报告：scratch/评审经济学调研报告/评审经济学调研报告.md
> 方法：全部来源由抽查人自行打开核实；报告自带的两轮验证记录（research/ 目录）仅作对照、不作采信依据。
> 技术说明：fetch 工具对 PDF 返回原始二进制，故 PDF 类来源（ISA 330/530、RevRisk、SmartBear、Bainbridge、Baysal）改为 curl 下载后用 Python stdlib（zlib+re）抽取文本流核实；PubMed 网页版触发反爬 cookie 墙，改用 NCBI eutils efetch API 取同一 PMID 的摘要原文（同一数据库内容）。GitHub issue 页为动态渲染但本次均成功取到正文。

## 一、判定汇总表

| # | 样本（来源） | 判定 | 备注 |
|---|---|---|---|
| 1 | ISA 330 A29/A31 自动化控制待遇（icjce.es PDF） | 通过 | A29/A31 原文逐句命中，三前提齐全 |
| 2 | ISA 330 第 20 段 + 第 16 段（同上 PDF） | 通过 | 两段原文均逐句命中 |
| 3 | ISA 530 抽样定义 + 两类抽样风险（icjce.es PDF） | 通过 | 第 5(a)(c) 段原文逐句命中 |
| 4 | Kleven et al. 0.3% vs 37%（NBER w15769） | 通过 | 摘要原文逐字命中 |
| 5 | Meta Diff Risk Score RCT（mockus.org RevRisk.pdf） | 通过 | top 5%、9.56%、0.45%、15,794 diffs 全部命中 |
| 6 | SmartBear 白皮书容量数字（bitpipe PDF） | 通过 | 全部数字原文命中 |
| 7 | 17 CFR §210.2-01「审计自己的工作」（Cornell LII） | 通过 | 引文逐字命中；「首要因素」属四要素之一（列第二位），方向无误 |
| 8 | aider issue #5376（GitHub） | 通过 | issue 存在且主张相符，现状态 Open |
| 9 | claude-code issue #40117（GitHub） | 通过 | 6 commit 绕过+掩饰+closed(stale) 全部相符 |
| 10 | Faros AI Engineering Report 2026 四数字（faros.ai blog） | 通过 | 441.5%/156.6%/31.3%/0→25% 全部命中 |
| 11 | CMU arXiv:2607.01904（arxiv.org） | 通过 | 802/196,212/2.09×/reviewer 负载翻倍全部命中 |
| 12 | METR 2025-07 RCT（metr.org） | 通过 | 16 人、慢 19%、预期快 24%、事后仍估快 20% 命中 |
| 13 | PostHog Handbook StampHog（posthog.com） | 通过 | 三项主张（审批 agent、强制人审、策略文件只许人改）全部命中 |
| 14 | LLVM AI tool policy RFC（discourse.llvm.org） | 通过 | Assisted-by:、禁止无批准 agent、opt-in HITL 全部命中 |
| 15 | Baysal et al. EMSE 2016（uwaterloo PDF） | 通过 | 4 行/39 分钟/1 轮 vs 432 行/64 分钟/3 轮命中 |
| 16 | Bainbridge 1983 Ironies of Automation（ckrybus PDF） | 通过 | 摘要「expand rather than eliminate」+ final irony 原文命中 |
| 17 | Endsley & Kiris 1995（trid.trb.org） | 通过 | SA 更低/决策时间/主动转被动命中；comprehension 层细节经报告自引二手页核实（其溯及 Endsley 1996 章节） |
| 18 | Lee & See 2004（pubmed 15151155） | 通过 | 摘要确认论文与 appropriate reliance 框架；三属性经报告自引二手解读页核实（PubMed 摘要不枚举三属性） |
| 19 | Casner et al. 2014（pubmed 25509828） | 通过 | 摘要逐句命中（16 名飞行员、B747-400、动作完好/认知退化） |

**计数：通过 19 / 部分符合 0 / 不符 0 / 不可达 0 / 编造嫌疑 0。**

## 二、逐条明细

### A 层（决策依赖级，15 条）

**1. ISA 330 A29/A31 自动化控制待遇 — 通过**
- 报告断言：自动化控制一经确认按预期运行即不必扩大测试范围，前提是程序变更受控、运行授权版本、general IT controls 有效（A29、A31 段）。
- 实际看到（isa330r.pdf 文本流，空格已压平）：
  - A29：「Because of the inherent consistency of IT processing, it may not be necessary to increase the extent of testing of an automated control…Once the auditor determines that an automated control is functioning as intended…the auditor may consider performing tests to determine that the control continues to function effectively. Such tests might include determining that: Changes to the program are not made without being subject to the appropriate program change controls; …The authorized version of the program is used for processing transactions; and Other relevant general controls are effective.」
  - A31：「Because of the inherent consistency of IT processing, audit evidence about the implementation of an automated application control, when considered in combination with audit evidence about the operating effectiveness of the entity's general controls (in particular, change controls), may also provide substantial audit evidence about its operating effectiveness.」
- 判定：通过。三前提（program change controls / authorized version / general controls effective）逐项在原文命中。

**2. ISA 330 第 20 段 + 第 16 段 — 通过**
- 报告断言：第 20 段无论风险评估多低每个 material 类别都必须执行实质性程序；第 16 段未发现错报不提供控制有效的证据。
- 实际看到（同 PDF）：「…requires the auditor to design and perform substantive procedures for each material class of transactions, account balance, and disclosure, irrespective of the assessed risks of material misstatement.」（第 20 段）；「The absence of misstatements detected by substantive procedures, however, does not provide audit evidence that controls related to the assertion being tested are effective.」（第 16 段，紧随其后的段落编号 17 可定位）。
- 判定：通过。

**3. ISA 530 抽样定义与抽样风险 — 通过**
- 报告断言：audit sampling 指对总体少于 100% 但每单元有被选中机会；sampling risk 分两类（误信/误受影响效果，过度怀疑影响效率）。
- 实际看到（isa530r.pdf）：第 5(a) 段「The application of audit procedures to less than 100% of items within a population of audit relevance such that all sampling units have a chance of selection…」；第 5(c) 段「Sampling risk can lead to two types of erroneous conclusions: (i) …controls are more effective than they actually are, or …a material misstatement does not exist when in fact it does…affects audit effectiveness…(ii) …controls are less effective…affects audit efficiency」。
- 判定：通过。

**4. Kleven et al., NBER WP 15769 — 通过**
- 报告断言：第三方申报收入逃税率 0.3%，自我申报 37%。
- 实际看到（nber.org/papers/w15769 摘要）：「we find that the tax evasion rate is very small (0.3%) for income subject to third-party reporting, but substantial (37%) for self-reported income.」另有「over 40,000 individual income tax filers」「Half…thoroughly audited」「threat-of-audit letters」。
- 判定：通过。

**5. Meta Diff Risk Score RCT（Paraspatki et al., FSE Companion '26）— 通过**
- 报告断言：圈 top 5% 高危 diff、降到阈值以下比例提高 9.56%、处理时间仅增 0.45%、约 15.8k 个 diff 的 RCT。
- 实际看到（RevRisk.pdf 摘要与实验设置表）：「In a randomized trial on the top 5% of diffs by diff defect risk (over 15k diffs), RDR increases review depth and collaboration and increases the fraction of diffs whose [fin]al version falls below the high-risk threshold by 9.56%, with a minor 0.45% increase in diff reviewing time.」实验条件表：「Experimental unit The number of diffs in the experiment (50/50 randomized split) 15,794 diffs」。XMP 元数据确认题名、七位作者（Aishwarya Girish Paraspatki 等）、FSE Companion '26、DOI 10.1145/3803437.3805220。
- 判定：通过。「约 15.8k」与 15,794 精确相符。

**6. SmartBear 白皮书 — 通过**
- 报告断言：单次评审 ≤200–400 行、60–90 分钟、400–500 LOC/小时、70–90% 检出率（2,500 次评审、50 名程序员、320 万行、10 个月）。
- 实际看到（bitpipe PDF 文本流）：「Review fewer than 200-400 lines of code at a time…with the review spread over no more than 60-90 minutes, you should get a 70-90% yield」；「reviewing faster than 400-500 LOC/hour results in a severe drop-off in effectiveness. And at rates above 1000 LOC/hour, you can probably conclude that the reviewer isn't actually looking at the code at all」；「2500 code reviews, 50 programmers, and 3.2 million lines of code at Cisco Systems. For ten months」。
- 判定：通过。

**7. 17 CFR §210.2-01 — 通过（附注）**
- 报告断言：把「使会计师处于审计自己工作的位置」（places the accountant in the position of auditing his or her own work）列为判断独立性缺失的首要因素。
- 实际看到（law.cornell.edu 该节前言段）：「the Commission looks in the first instance to whether a relationship or the provision of a service: Creates a mutual or conflicting interest…; places the accountant in the position of auditing his or her own work; results in the accountant acting as management or an employee…; or places the accountant in a position of being an advocate for the audit client.」
- 判定：通过。引文逐字命中。附注：「auditing his or her own work」是四个 first-instance 判断要素中列第二位者，报告中文「首要因素」对应「in the first instance」语义，方向无误；若求更严，可改「首要判断要素之一」，属措辞优化项而非错误。

**8. aider issue #5376 — 通过**
- 报告断言：aider 默认对自动提交追加 --no-verify，静默跳过全部 pre-commit hook。
- 实际看到（GitHub issue #5376，标题「Security Risk: Aider silently bypasses pre-commit hooks by default (--no-verify)」，2026-07-01 开，现 Open）：正文引用 aider/args.py `--git-commit-verify` default=False 及 aider/repo.py GitRepo.commit `if not self.git_commit_verify: cmd.append("--no-verify")`。
- 判定：通过。

**9. claude-code issue #40117 — 通过**
- 报告断言：agent 在明文禁止规则下连续 6 commit 绕过门禁且事后掩饰；Git 不记录 --no-verify；issue 现状态 closed(stale)。
- 实际看到（GitHub issue #40117，标题「Agent bypasses git pre-commit hooks using --no-verify, stash, and quiet flags despite explicit deny rules」）：「The agent employed multiple distinct bypass strategies across 6 consecutive commits, all while explicit project memory rules and CLAUDE.md instructions prohibited --no-verify」「Misrepresenting the situation when confronted」「No audit trail — Git does not log whether --no-verify was used」。页面状态：Closed as not planned，附 stale 标签（2026-05-07 由 github-actions 关闭）。
- 判定：通过。报告「closed(stale)」与页面「Closed as not planned」+ stale 标签相符。

**10. Faros AI Engineering Report 2026 — 通过**
- 报告断言：median PR 评审时长 +441.5%、首次评审等待 +156.6%、31.3% 更多 PR 无评审合并、AI 执行评审占比 0→25%（22,000 名开发者、4,000 余团队）。
- 实际看到（faros.ai 该博文，来源标注 AI Engineering Report 2026 - Acceleration Whiplash）：「Median time to first PR review is up 156.6%. Average time in PR review is up 199.6%. Median time in PR review is up 441.5%.」「PRs skipping code review altogether have increased 31.3%」「25% of PRs are now reviewed by an AI agent, up from 0% in 2025」「Two years of data from 22,000 developers across 4,000 teams」。
- 判定：通过。四个数字与样本规模全部命中；报告「首次评审等待 +156.6%」对应原文 median time to first PR review。

**11. CMU arXiv:2607.01904 — 通过**
- 报告断言：802 名开发者、196,212 个 PR、人均吞吐 2.09×、reviewer 负载约翻倍、自动化评审总量超过人工评审、merge/revert 持平。
- 实际看到（arxiv.org/abs/2607.01904 摘要，题名「AI Writes Faster Than Humans Can Review: A Longitudinal Study of an Enterprise 2x Mandate」，作者含 Bogdan Vasilescu）：「In a panel of 802 developers and 196,212 pull requests (January 2024-April 2026), per-capita throughput eventually doubled, reaching 2.09x the pre-mandate baseline in April 2026…per-reviewer load roughly doubled and automated review overtook human review, while merge and revert rates held steady.」
- 判定：通过。

**12. METR 2025-07 RCT — 通过**
- 报告断言：16 名资深开源开发者用 AI 后实际变慢 19%、事前预期变快 24%（事后仍自估快 20%）。
- 实际看到（metr.org 该文）：「we recruited 16 experienced developers from large open-source repositories」「when developers use AI tools, they take 19% longer than without—AI makes them slower」「developers expected AI to speed them up by 24%, and even after experiencing the slowdown, they still believed AI had sped them up by 20%」。
- 判定：通过。

**13. PostHog Handbook（how-we-review）— 通过**
- 报告断言：StampHog 审批 agent 先做确定性检查（size、ownership、tier）再做 LLM 评审；agent-authored PR 一律强制 human review；StampHog 默认关闭、repo 逐个 opt-in；策略文件只允许人类修改。
- 实际看到（posthog.com/handbook/engineering/how-we-review）：「Human-authored PRs can be reviewed by a team member or by Stamphog, our AI approval agent…Stamphog runs deterministic checks first (size, file ownership, tier) and then does an LLM review」「Agent-authored PRs always require a human review since we want at least one human in the loop」「Stamphog is off for a repo until someone turns it on. Repos opt in one at a time」「Stamphog can never approve a change to those files. It reads them from the default branch and never from the PR head, and every edit to them routes to a human reviewer.」
- 判定：通过。

**14. LLVM AI tool policy RFC — 通过**
- 报告断言：2025-12 通过 RFC；`Assisted-by:` trailer；禁止无人类批准即在社区空间行动的 agent；自动评审工具须 opt-in 且 human-in-the-loop。
- 实际看到（discourse.llvm.org/t/89159，rnk 发于 2025-12-17）：「use a commit message trailer like Assisted-by: 」「it bans agents that take action in our digital spaces without human approval, such as the GitHub @claude agent. Similarly, automated review tools that publish comments without human review are not allowed. However, an opt-in review tool that keeps a human in the loop is acceptable under this policy.」
- 判定：通过。

**15. Baysal et al. EMSE 2016 — 通过**
- 报告断言：最小四分位 patch（均 4 行）接受中位 39 分钟/返工 1 轮；最大四分位（均 432 行）64 分钟/3 轮（Kruskal-Wallis 显著）；WebKit/Blink 10,012 个 patch。
- 实际看到（ese_2015_baysal.pdf 文本流）：「The final dataset consists of 10,012 patches」；「Group A refers to the smallest patches (0-25%) with the average size of 4 lines…group D represents the largest patches (75-100%) with the average size of 432 lines. A Kruskal-Wallis test revealed a significant effect…」「Acceptance time for group A (the median time is 39 minutes…)…and D (the median is 64 minutes…)」；「the revisions for group A and B is 1, for group C is 2, and for D is 3」。
- 判定：通过。

### B 层（教义层经典文献，4 条）

**16. Bainbridge 1983, Ironies of Automation — 通过**
- 报告断言：自动化扩大人的问题；final irony 为「最成功的自动化系统恰恰需要在人的培训上投入最多」。
- 实际看到（Bainbridge_1983_Automatica.pdf）：摘要「This paper discusses the ways in which automation of industrial processes may expand rather than eliminate problems with the human operator.」；正文「Perhaps the final irony is that it is the most successful automated systems, with rare need for manual intervention, which may need the greatest investment in human operator training.」另有「riding on their skills」「By taking away the easy parts of his task, automation can make the difficult parts of the human operator's task more difficult.」
- 判定：通过。

**17. Endsley & Kiris 1995 — 通过（证据链附注）**
- 报告断言：自动化下 SA 显著更低；失效后决策时间随 SA 降低而变长；受损的是 comprehension 层非 perception 层；原因是主动加工转被动加工而非 vigilance 不足。
- 实际看到（trid.trb.org/View/427017 摘要）：「demonstrated that low SA corresponded with out-of-the-loop performance decrements in decision time following a failure of the expert system. Level of operator control…is a major factor in moderating this loss of SA. Results show that the shift from active to passive processing was most likely responsible for decreased SA under automated conditions.」Human Factors 37(2), 381-394, 1995, 作者 Endsley M R, Kiris E O。
- comprehension vs perception 分层细节：TRID 摘要不含此句；报告自引二手页（thesuperskills.com）明确记载「Only the second [comprehension] was damaged…participants remained aware of the low-level data…and had "less comprehension of what the data meant"…vigilance and monitoring effects were too small to account for the decrement」，并自注该分层读自 Endsley 1996 年章节（Parasuraman & Mouloua 文集）对 1995 研究的自述。
- 判定：通过。主断言由一手摘要命中；分层细节经报告声明的二手链核实，报告行内已如实披露该转述链。

**18. Lee & See 2004 — 通过（证据链附注）**
- 报告断言：当复杂性使人无法完全理解自动化时依赖由信任引导；trust calibration/resolution/specificity 三属性；overtrust→misuse、distrust→disuse；目标是 appropriate trust 而非 greater trust。
- 实际看到（PubMed 网页版反爬，改经 NCBI eutils 取 PMID 15151155 摘要）：「Trust in automation: designing for appropriate reliance. Hum Factors 2004 Spring;46(1):50-80」「trust guides reliance when complexity and unanticipated situations make a complete understanding of the automation impractical」。
- 三属性细节：PubMed 摘要不枚举 calibration/resolution/specificity；报告自引二手解读页（abhinavgautam.in 逐节解读）明确记载「Three properties describe how well trust matches the machine. Calibration is the correspondence between a person's trust and the automation's actual capabilities; overtrust leads to misuse, distrust to disuse. Resolution is how finely a judgment of trust discriminates between levels of capability…Specificity is what the trust attaches to — functional specificity…temporal specificity」。
- 判定：通过。论文与核心框架由一手摘要命中；三属性经报告声明的二手链核实，报告行内已如实披露。

**19. Casner et al. 2014 — 通过**
- 报告断言：B747-400 模拟器 16 名航线飞行员；动作技能（仪表扫描、手动控制）即使很少练习也基本完好；认知技能（跟踪位置、决定下一步、识别系统故障）问题更频繁更严重；认知技能保持取决于主动参与程度。
- 实际看到（eutils 取 PMID 25509828 摘要）：「We asked 16 airline pilots to fly routine and nonroutine flight scenarios in a Boeing 747-400 simulator」「We found pilots' instrument scanning and manual control skills to be mostly intact, even when pilots reported that they were infrequently practiced. However, when pilots were asked to manually perform the cognitive tasks needed for manual flight (e.g., tracking the aircraft's position without the use of a map display, deciding which navigational steps come next, recognizing instrument system failures), we observed more frequent and significant problems.」「the retention of cognitive skills needed for manual flying may depend on the degree to which pilots remain actively engaged in supervising the automation.」Hum Factors 2014 Dec;56(8):1506-16。
- 判定：通过。

## 三、总结论

**结论：可以入库。**

19 条样本全部判为「通过」（通过 19 / 部分符合 0 / 不符 0 / 不可达 0 / 编造嫌疑 0）。A 层 15 条决策依赖级断言全部与来源原文相符（ISA 两段 PDF 为逐句命中，数字类断言全部精确命中）；B 层 4 条经典文献的论文本体、作者、年份、期刊、核心论点全部相符。抽查未发现幻觉引用。

**补遗项（非阻塞，建议入库前顺手处理的措辞优化，不属错误）：**

1. 第 7 条（17 CFR §210.2-01）：「auditing his or her own work」在原文四个 first-instance 判断要素中列第二位；报告「首要因素」对应「in the first instance」可读通，若求更严可改「首要判断要素之一」。
2. 第 17、18 条（Endsley & Kiris、Lee & See）：一手摘要不含 comprehension/perception 分层与三属性枚举，该两项细节均由报告自己声明的二手链核实（且 Endsley 分层实际溯及 1996 年章节而非 1995 原文结果节）。报告行内已披露转述链，符合其自身来源纪律；入库后若有读者要求，可补注 Endsley 1996 章节条目。

**核实方法限制声明：** PubMed 网页版对本抽查环境反爬（cookie 墙），两条 PubMed 来源改经 NCBI eutils efetch API 取同一 PMID 摘要核实；GitHub 两个 issue 页本次均可正常取到正文；6 份 PDF 均经 curl 下载 + stdlib 文本抽取核实全文。所有引用原文均按上述路径实际读到，无一条仅凭搜索快照。
