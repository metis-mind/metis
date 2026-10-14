> 状态：过程存档（point-in-time，2026-10-14 入库）。评审经济学调研报告（[review-economics-research.md](./review-economics-research.md)）调研方的自检记录（两轮交叉验证），随正本入库备查；此为被审方自检，入库采信依据以仓内独立抽查（[review-economics-admission-check.md](./review-economics-admission-check.md)）为准。正文逐字未改；报告其余过程件（d01–d10、verify1/verify2 等）留 scratch 未入库；原文出处 = 调研交付包 `research/review_econ_cross_verification.md`。

# 交叉验证结果（review_econ）

调研执行日期：2026-10-14。10 个维度简报（d01–d10）+ 2 份独立验证（verify1 数据组 10/10 PASS；verify2 锚点组 22 通过 / 6 部分符 / 0 编造）。

## 置信分级总览

| 层级 | 条目 |
|------|------|
| High Confidence（多源独立一致 + 一手核实） | SmartBear/Cisco 评审容量数据（200–400 LOC、60–90 分钟，标注利益相关）；Bacchelli & Bird 44%/14%；Drew 2014 ICU 告警 187/床/天、88.8% 假阳性；Bainbridge 1983 原文论点；ISA 315/330/530 依赖逻辑；Veracode 45%；Apiiro 10×/322%/153%；USENIX slopsquatting ~20%；Anthropic multi-agent 4×/15× token；Faros +441%/31% 无评审合并；14 CFR 25.1322 / ISA-18.2 / Google SRE 告警戒律三行业收敛；Endsley OOTL 实证；c2 ThreadMode/DocumentMode；Google SWE 第 10 章 freshness 机制 |
| Medium Confidence（单一权威来源一手核实） | Meta FSE'26 高危 diff RCT；Kleven 丹麦审计实验；Just et al. 73% 耦合效应；Cummings UAV 监督容量（监督 15–17 / 控制约 3；by-consent 4–5 vs by-exception 8–12）；CodeRabbit 470 PR（厂商，利益相关）；FoundationDB/TigerBeetle DST；PCC POPL'97 |
| Conflict Zone | ①小 PR 收益：Google/WebKit/di Biase 支持 vs Kudrjavets 845k PR 阴性结果（d02，保留为【争议】）；②告警数量是否根因：Drew 等数据 vs Bryant 2014「信噪比与时机才是根因」（d03，【争议】保留）；③AI 评审工具效果：CodeRabbit 厂商数据 vs 独立观察稀缺（d07，区分标注） |
| 验证修正项 | d01 的 ISA 530 来源为 icjce.es 镜像而非 IAASB 官网（写作时注明）；d04 PCC 署名应为 Necula 单著（非 Necula & Lee 合著 POPL'97）；d05 一处 issue 编号笔误（53576→5376）；claude-code #40117 状态为 closed(stale)；Meta RCT 两位作者有高校兼职（披露）；SAFO 13002 官方页失效、经 NTSB 报告独立确认 |

## 未确认清单汇总（各维度上报，写入最终报告附录）
d01: 9 项（含 PCAOB AS 2315、SmartBear 原始 Cisco 内部报告）；d02: Czerwonka 2015 原文（15% 数字经 CACM 访谈间接核实）；d03: 4 项（SEA-50 PDF、AC 25.1322-1、Barnett 2015、SemanticMerge 官网失效）；d04: 7 项（PLDI'98 retrospective、1978 原文全文等）；d05: 7 项（Netflix 2012 原文失效、TigerBeetle 博客被拦截、agent 改测试缺量化研究）；d06: 8 项（Strauch 2018 全文等）；d07: 若干；d08: 7 项（Gas Town 原文、Opsera 二手）；d09: 10 项（Sheridan/Contract Net 一手原文）；d10: WikiGardening 条目不存在（c2 无此页）、Parnas 1994 全文未直接打开（Crossref 元数据已确认）。

## 写作约束
- 所有引用沿用简报中的「分级 + 标题 + URL + 访问日期 + verbatim 摘录」记录；Conflict Zone 必须双侧呈现。
- 验证修正项须在终稿落实。
