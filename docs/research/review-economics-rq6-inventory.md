> 状态：point-in-time 仓内盘点（2026-10-14）。RQ6（仓内机制盘点，调研委托中不外发的一项）的执行记录：评审经济学调研报告（[review-economics-research.md](./review-economics-research.md)）「已有 10 项机制」清单与缺口判断的仓内对账——漏项 14 条、须修正 4 处。行号引用以 2026-10-14 版文件为准（HANDOFF.md 行号随重写漂移）。正文逐字未改；原文出处 = 调研交付包 `rq6-仓内盘点.md`。

# RQ6 仓内盘点：外部调研报告「Metis 已有 10 项机制清单」与缺口判断的仓内对账

> 盘点日期：2026-10-14｜执行：仓内盘点子代理（有仓库访问权）｜对象：`scratch/评审经济学调研报告/评审经济学调研报告.md` 映射建议章（L485–543）与 RQ1 三分判定矩阵（L74–90）
> 方法：只用仓内实物对账，每条结论给 path:line 证据；不改任何仓内文件。读取面 = AGENTS.md、HANDOFF.md（§2/§4/§5/§6/§7）、justfile、.pre-commit-config.yaml、dprint.json、deny.toml、.github/（六个 workflow + dependabot.yml）、docs/decisions/0009/0012/0030、docs/design/testing-strategy.md、crates/xtask/src/（main.rs / commit.rs / agent_docs.rs）、.agents/skills/ 七个 SKILL.md、CONTRIBUTING.md、.gitignore。

---

## ① 10 项机制对账表

| # | 机制（报告 L493–504 表行） | 仓内真实形态与锚点 | 报告对应行 | 对账结论 |
|---|---|---|---|---|
| ① | 议题拍板制 | 实物 = ADR-0030 条款 4 常态三件套之首「议题拍板制（设计逐条用户拍板）」（docs/decisions/0030-engineering-phase-2.md:46）；出示格式纪律在 HANDOFF.md:269（背景→取舍分析、术语就地解释、小决策点合并成批一次拍板）；工作节奏流水线在 HANDOFF.md:287（议题地图 → 用户逐条确认 → 固化 → just ci → self-review → commit+push）；进行中实例 = 任务 8 D1–D8 议题出示留存（HANDOFF.md:34–43） | L495、L510 | **属实**。补充：报告称「议题无 actionable 入场测试」属实（见缺口 1 对账），但仓内存在出示格式纪律（HANDOFF.md:269），报告未提及 |
| ② | ADR 链（冻结 + supersede） | 纪律 = AGENTS.md:66（冻结、supersede、局部 supersede 标法、引用只写编号）；制度化来源 = docs/decisions/0012-docs-governance.md:14；实物 = docs/decisions/ 0001–0030 共 30 份；局部 supersede 实例 = docs/decisions/0009-engineering-baseline.md:3 Status 行；批次 supersede 实例 = docs/decisions/0029-journal.md 末节 7 条 | L496、L518 | **属实**。「ADR 无 freshness 元数据」对 docs/decisions/ 属实；但 AGENTS.md 自身有机器检查的 freshness 注（见漏项 L6），报告表行⑥/⑩均未计入 |
| ③ | 机器门禁统一入口（just ci + prek 钩子 + 远程 CI） | 统一入口 = justfile:76（`ci: lint test doc deny agent-check`）；纪律 = AGENTS.md:7（commit 前必跑）、AGENTS.md:39（禁绕过、禁拼临时管线）；本地钩子 = .pre-commit-config.yaml:8–34（fmt/lint/commit-convention/secret-scan 四件）；远程 = .github/workflows/ci.yml:49（quality-gate 同源复跑 prek 全文件）与 ci.yml:74（ubuntu+macOS 矩阵跑同一 `just ci`）；定位决策 = docs/decisions/0030-engineering-phase-2.md:29–31 | L497、L530 | **属实，附两处须修正**：(a) 报告 D3 附注「Metis 已有服务端 CI，与『不可绕过层在服务端』方向一致」须修正——直推 main 且不开 require status checks（0030:31），远程 CI 是落地后复核而非合并前门禁，仓内已自认（HANDOFF.md:52「当前不可绕过层 = 复核非门禁」）；(b) 「修改门禁/测试套件本身的变更无差别审查通道」须部分修正——`.github/`、CI workflow 文件、hosting 设置、新依赖四类面已有 ASK 人审闸（AGENTS.md:44；self-review SKILL.md:110–112），但 justfile/.pre-commit-config.yaml/deny.toml/dprint.json/crates/xtask/测试套件确实无特别通道 |
| ④ | self-review（机械 sweep + 子代理判断 pass） | 实物 = .agents/skills/self-review/SKILL.md 全文：分工 L13–17、作者机械 sweep L19–28、子代理判断 pass L30–67（含对抗三问 L48–59 须逐字进 prompt）、findings 分级 L78–89、规模与流程闸 L91–112；入口 = AGENTS.md:25、AGENTS.md:33（Done 定义）；定型决策 = docs/decisions/0030-engineering-phase-2.md:46（吸收 pr-preflight 流程面） | L498、L510 | **属实**。报告「审查 agent 与被审 agent 同内核同供应商」观察属实——仓内无多供应商/多模型交叉机制（SKILL.md 全文无） |
| ⑤ | HANDOFF 交接文档 | 定位 = docs/decisions/0012-docs-governance.md:8、16（易失、gitignored、每次交接重写）与 AGENTS.md:43、69；gitignore 实证 = .gitignore:20；交接纪律 = 0012:21（新会话先读、工作结束重写）；本体 = HANDOFF.md（当前 339 行） | L499、L518 | **属实，附两点补充**：(a) 退役节点已钉死——HANDOFF.md:288（任务 10 开工即退役、开工门槛 = 双流程完备 + HANDOFF 迁移完毕）与 HANDOFF.md:239（队列 11 含退役迁移方案，退役 = 迁移非删除，机制触点 6 处已枚举）；(b) 「无 TTL、无压缩仪式，只增不减」大体属实（每会话重写是事实上的压缩但无保真纪律），但压缩纪律已立项 = HANDOFF.md:240 队列 12 范围含「HANDOFF 压缩纪律（320 行且每会话 prepend 不压缩——过渡期也需闸门）」 |
| ⑥ | SSOT 纪律 + 指针机器检查 | 纪律 = AGENTS.md:18（rust-toolchain.toml 版本 SSOT）、AGENTS.md:37–38（Cargo.lock 禁手改、版本号单一存储点）；机器检查 = crates/xtask/src/agent_docs.rs：模块文档 L1–10、`.claude/skills` 指针 L198–224、AGENTS.md 预算 + freshness 注 L226–247、CLAUDE.md 单引用 L250–267；挂接 = justfile:57–61（agent-check 进 just ci） | L500 | **属实**。「机器只判结构不判语义、分层承诺未显式化」属实——agent_docs.rs 全部检查项均为结构/预算/指针类，无语义判断 |
| ⑦ | 汇报纪律（状态词/四要素/path:line） | 实物 = AGENTS.md:56–62（十状态词、验证四要素、path:line 证据、编号选项推荐第一、禁「可能大概应该是」）；吸收记录 = HANDOFF.md:270；来源标注 = AGENTS.md:62（ste-zh, MIT） | L501、L520 | **属实**。「约束格式不约束真实性、结果字段无独立证据锚」属实——AGENTS.md:58–60 只要求自述带证据位置，未要求链接 CI 运行记录类不可伪造锚 |
| ⑧ | 大 diff 解释性产物（explain-diff） | 实物 = .agents/skills/explain-diff/SKILL.md 全文（HTML 单文件 + 聊天 quiz）；入口 = AGENTS.md:31；来源标注 = SKILL.md:101–103 | L502、L520 | **属实**。「无置信闸门」属实——SKILL.md 全文无质量闸门；最近似物 = 诚实性元素（区分观察事实与解释 SKILL.md:70；交付时声明假设与验证限度 SKILL.md:95–97） |
| ⑨ | 快照测试人工确认（insta） | 入口 = justfile:53–55（`cargo insta review`，注释明「CI is read-only and never auto-accepts」）；纪律 = AGENTS.md:11（逐个批准、禁批量接受）、self-review SKILL.md:100–103（人批准、agent 只准备与解释）、CONTRIBUTING.md:56–57；`.snap.new` 禁提交 = .gitignore:12 | L503、L514 | **属实**。「批准式确认、无单次确认容量预算、无两阶段（先写预期再对照）设计」均属实——三处锚点均为单阶段批准式 |
| ⑩ | 文档治理入口侧（三问 + 长度预算） | 三问过滤器 = .agents/skills/doc-maintenance/SKILL.md:11–21；同改协同 = SKILL.md:42–46 与 AGENTS.md:53；长度预算 = crates/xtask/src/agent_docs.rs:21–22（SKILL body ≤20000 字符、AGENTS.md ≤250 行）；drift hunting = SKILL.md:48–54 | L504、L518 | **属实，附一处限定**：「入口闸门防增量不治存量」对 docs/ 属实；但 agent-check 的预算检查是**存量侧**机制——每次 `just ci` 对已入库的 AGENTS.md/skills 重查预算与 freshness 注（agent_docs.rs:226–247），报告把⑩整体定位为「入口侧」须加此限定 |

---

## ② 漏项清单（仓内存在、具评审/门禁/防腐职能、未进 10 项清单）

| # | 机制 | 仓内锚点 | 评审/门禁/防腐职能 |
|---|---|---|---|
| L1 | commit-msg 钩子强制提交规范（含远程半程） | .pre-commit-config.yaml:21–25；crates/xtask/src/commit.rs:1–21（规则 SSOT：11 种 type 白名单 L29–31、header ≤100 字符 L25、body 散文 ≤7 行 L28、禁 HTML 注释/任务列表/裸 `---` L13–19）；AGENTS.md:51；远程半程 = .github/workflows/commits.yml:45–47（PR 标题+正文过同一 check-commit） | 每个 commit 的机器门禁 + 版本意图 SSOT（fix→PATCH 等，commit.rs:3–5）；防过程脚手架混入历史。名义上挂在 ③ 的 prek 链下，但职能独立（③ 的 just ci 不含提交信息规范） |
| L2 | prek pre-commit 钩子链与 CI quality-gate 同源复跑 | .pre-commit-config.yaml:1–7（本地钩子与 CI 共享同一来源的声明）、11–20（fmt/lint）；ci.yml:22–23、49（`prek run --all-files` 全文件只读复跑）；钩子安装 = justfile:16–19（`just setup`），devShell 自动 arm = CONTRIBUTING.md:17–19 | 提交前第一道机器门禁；CI 用同一配置文件复跑 = 门禁配置单一事实源，本地/远程无漂移面 |
| L3 | secret-scan 双半门禁（pr-guard） | 本地半 = .pre-commit-config.yaml:26–34（staged diff）；PR 半 = .github/workflows/pr-guard.yml 全文；实现 = crates/xtask/src/pr_guard.rs（main.rs:27 挂接）；入口 = justfile:68–72；示例秘密豁免纪律 = HANDOFF.md:313（`pr-guard:allow` marker 该行放置） | 秘密不进提交的硬门禁（token/私钥/.env 形状硬失败）；本地+PR 双侧覆盖 |
| L4 | cargo-deny 依赖门禁 + staleness fuse + 定期远程审计 | justfile:47–51（`-D advisory-not-detected -D license-exception-not-encountered`）；deny.toml:5–9（advisory 例外引信）、29–34（license 例外引信）、21–28（allow 清单）；定期 = .github/workflows/dependency-audit.yml:9、41（周三跑 `just deny`）；纪律 = adding-dependencies/SKILL.md:38–56 | 供应链门禁（advisory/license/bans/sources）；例外条目自清理引信 = 防腐设计——条件消失则 CI 失败迫使删除，「没人需要记得」 |
| L5 | adding-dependencies 的 ASK-first 人审闸 | AGENTS.md:44（ASK 面 = 新第三方依赖 / CI workflow 文件 / hosting 与分支保护设置）；adding-dependencies/SKILL.md:58–62（§4 Ask：先出示判断摘要再等答复）；进提交流程闸 = self-review/SKILL.md:110–112；实例 = HANDOFF.md:47（任务 8 ASK 面声明） | 仓内现有的「HARD 人审门」雏形：四类高危面要求人类显式批准。报告 D3 的部分既有实现 |
| L6 | agent-check 的存量预算 + freshness 机器检查 | crates/xtask/src/agent_docs.rs:21–22（两条预算常量）、226–247（AGENTS.md 行数预算 + 「Last reviewed」 freshness 注前五行必查）；挂接 = justfile:57–61；维护纪律 = rule-maintenance/SKILL.md:37–44 | 对 agent 面向文档的**存量侧**防膨胀闸（每次 just ci 对现有文件生效）+ freshness 元数据的机器承诺。报告⑩把长度预算定位为「入口侧」，此项予以更正 |
| L7 | GitHub hosting 层 ruleset 与合并策略收窄 | docs/decisions/0030-engineering-phase-2.md:31（main ruleset = 禁 force-push + 禁删分支；squash-only + 合并后删分支；**不开** require status checks）、52（执行记录） | 历史完整性的服务端硬边界：force-push/删分支禁令保住审计轨迹（本地钩子被 `--no-verify` 绕过时，证据仍留在 main 历史中）。注意：不是合并前门禁 |
| L8 | 火花捕捉机制 + HANDOFF 火花登记区 | 规则 = AGENTS.md:54（verbatim-first 捕捉、逐字迁移不压缩、长寿命升格、未消费不消失）；机制收官记录 = HANDOFF.md:256；登记区 = HANDOFF.md:258–262（火-1/2/3，各带消费点与已/未消费状态） | 防遗忘的议程保护：维护者前瞻性意图逐字留痕并带消费点登记，部分对冲缺口 1 的「议程设置权在 agent」——用户意图侧有不经 agent 议程的独立通道 |
| L9 | dprint 对 docs/research 的排版排除 | dprint.json:7（`"excludes": ["CHANGELOG.md", "docs/research/**"]`）；配套纪律 = AGENTS.md:68（research = 时点快照、更正走增补） | 排版纪律差异：时点快照文档不受整表重排（dprint 行为见 HANDOFF.md:309），保护 research 文档的 point-in-time 性 |
| L10 | 开工门槛（任务 10 双完备闸）与队列时序硬约束 | HANDOFF.md:288（开工门槛 = 审计流程 + 开发流程双完备 + HANDOFF 迁移完毕；HANDOFF 退役节点）；HANDOFF.md:239（队列 11 时序硬约束 = 任务 10 前收官）；HANDOFF.md:240（队列 12） | 阶段门性质的流程完备闸：「评审/开发机制先制度化再放量」已是节点规则——报告 D6 排序判断的既有对应物 |
| L11 | 扩展位/复审触发器登记机制 | HANDOFF.md:264（各扩展位/复审点均在 ADR/设计文档登记，枚举 ADR-0021 R1–R5 至 ADR-0029）；docs/decisions/0029-journal.md 演进纪律 #4（登记汇总表为 SSOT）与 Consequences（扩展位三条各带触发器）；docs/design/testing-strategy.md:90–103（推迟登记含触发条件） | 延迟决策的防丢登记：每个推迟项带显式重开触发器，防「缓议 = 默默消失」 |
| L12 | self-review §4 的 diff 规模预算 | .agents/skills/self-review/SKILL.md:91–95（约 100 行理想 / 300 可接受 / 1000 必拆；refactor 与 feature 分两 commit） | 评审单元粒度纪律：RQ2 容量实证（200–400 行硬边界）的仓内对应物。嵌在 ④ 内部但职能独立——约束提交粒度以保住可评审性 |
| L13 | rule-maintenance（规则自身的防腐回写） | .agents/skills/rule-maintenance/SKILL.md:12–24（四类触发器：错误重复/规则过期/工具变化/程序升格）、26–33（过滤器）、76–83（同 commit 协同 + commit body 记触发器）；承接 = CONTRIBUTING.md:74 | 指令文件防腐的写回机制（context rot 对策）；「规则变更留触发器于 commit body」使规则 churn 可评审 |
| L14 | 定期保鲜例程群（dependabot + flake/toolchain 周更） | .github/dependabot.yml:1–20（cargo + github-actions 双生态周更分组）；docs/decisions/0030-engineering-phase-2.md:20–21（flake-update 周一 / toolchain-update 周二带 in-job `just ci` 闸；owner guard 落 `metis-mind`）；AGENTS.md:14 | 依赖与工具链的防腐保鲜；toolchain-update 的 in-job just ci 是 bot PR 不触发 CI 下的替代门禁 |

---

## ③ 缺口判断对账（报告 L508–520，六条）

**缺口 1（议程设置权、抽样框与证据生成三位一体在被审者；L510）——属实，附两点补充。**
- 「议题是 agent 提的」属实：议题出示 = agent 侧动作（HANDOFF.md:287 工作节奏；任务 8 实例 HANDOFF.md:34–43，D1–D8 全部 agent 出示待拍板）。
- 「议题无 actionable 入场测试」属实：AGENTS.md 全文（L1–69）无议题入场规则；HANDOFF.md:269 仅有出示格式纪律（背景+取舍分析、术语解释、批量拍板），无「每条须要求一个决策或行动」的测试。
- 补充一：自检/交接/汇报/导览确为 agent 产出（④⑤⑦⑧锚点见对账表），「输入端几乎全部经过被审者」属实。
- 补充二（报告未见）：火花登记区（HANDOFF.md:258–262 + AGENTS.md:54）是维护者意图不经 agent 议程的独立留痕通道，对该缺口有微弱对冲。

**缺口 2（抽查存在但未定价；L512）——属实。**
- 仓内无抽查率、风险分层、不可预测性、非零下限的任何参数化机制（全读取面无对应物）。
- 「维护者事实上在抽查」与仓内结构一致：设计面 = 逐条拍板 100% 覆盖（0030:46；HANDOFF.md:287）；代码面 = self-review 子代理判断 pass 每 commit 全量（SKILL.md:30–34）+ 机器门禁全量；维护者对代码产出的事后抽查无制度化、无参数——被「抽」的只有这一层，恰好是报告所指。

**缺口 3（低评审占比退化无对冲；L514）——属实。**
- 全仓无任何机制要求维护者定期亲手实现或调试代码（AGENTS.md/skills/HANDOFF §6–7 均无）。
- 「ADR 是被动读物」属实（②为阅读物）；「⑨批准式确认恰是退化条件配置」属实（justfile:53–55、self-review SKILL.md:100–103 均为单阶段批准式，无主动加工设计）；①议题拍板同为批准式（HANDOFF.md:287「逐条确认」）。

**缺口 4（RQ7 前提需求端未满足；L516）——属实，附一处重要补充。**
- 「例外管理通道（结构化状态协议、目标级语义摘要 dashboard）未制度化」属实（无对应物）；「评审容量实测无数据、扩张信号无从观测」属实。
- 「HANDOFF 是串行衔接装置」属实且有仓内自述：HANDOFF.md:255「现 1+1 串行 = 安全基线（HANDOFF = 串行化装置，单写者零冲突），非失败形态」。
- 补充（报告未见）：「机制先于扩容」的排序已制度化为节点规则——HANDOFF.md:288 开工门槛（审计流程 + 开发流程双完备 + HANDOFF 迁移完毕才开工）与 HANDOFF.md:239 队列 11 时序硬约束（任务 10 前收官）。仓内不是「扩容后再补」的裸奔状态，而是把「满足前提」设成了开工闸。

**缺口 5（文档存量生命周期整体缺席；L518）——对 docs/ 属实；「整体缺席」须修正。**
- 对 docs/（decisions/design/research）属实：无 freshness 元数据、无 attic、无状态机、无 gardener 巡检、无 TTL。
- 须修正处：(a) agent 面向文档已有存量侧机器闸——agent-check 预算 + freshness 注检查（agent_docs.rs:21–22、226–247）每次 `just ci` 对已入库文件生效；(b) HANDOFF 有退役节点（HANDOFF.md:288）与每会话重写纪律（0012:21）；(c) docs/research 有时点快照 + 增补不重写纪律（AGENTS.md:68）与 dprint 排除（dprint.json:7）；(d) 该缺口仓内已自认并立项——队列 12（HANDOFF.md:240，范围含存量生命周期「压缩/退役/考古层处置」+ 机器膨胀信号「长度预算/重复检测/引用计数；agent-check size budgets 先例」+ HANDOFF 压缩纪律），起源 = 火-2 逐字存档（HANDOFF.md:261）。
- 「HANDOFF 天然 ThreadMode 堆积而无 DocumentMode 转化环」属实（HANDOFF.md §2 固化记录逐站 prepend 堆积，无转化机制）。

**缺口 6（呈现层信噪比戒律只制度化了一半；L520）——属实，附两个既有半成品。**
- 「每条呈现物是否要求一个决策或行动」的入场测试未制度化：属实（无对应物）。每日打断总量预算未制度化：属实。⑧导览无置信闸门：属实（explain-diff SKILL.md 全文无）。
- 半成品一：AGENTS.md:61「决策点编号选项、推荐第一标（推荐）」= actionable 形状对决策点类的强制；半成品二：HANDOFF.md:269「信息量不多的决策点合并成批一次拍板」= 打断减量的原始形态。两者都不到「入场测试 + 预算」的制度化程度，报告判断成立。

---

## ④ D1–D10 候选对策的仓内适配备注（报告 L528–537）

> 编号防撞说明：报告对策记「报告 D#」；仓内任务 8 议题另有一套 D1–D12（HANDOFF.md:36–43、53），两者无映射关系，下表凡引用仓内议题均写「任务 8 D#」。

| 报告对策 | 仓内适配：已有部分实现 / 对应物 / 冲突 |
|---|---|
| D1 抽样框独立化（L528） | 无既有实现。可复用件：任务 8 D9 候选的「门禁面文件清单」（HANDOFF.md:53）是现成的最高风险分层类；注意仓内设计面为 100% 拍板制、self-review 为每 commit 全量，抽样机制的实际作用面 = 维护者对代码产出的事后审查层。抽查率参数化在仓内无任何先例 |
| D2 汇报挂独立证据（L529） | 部分既有，与报告「加固非新建」定位一致：⑦已要求 path:line/完整命令级证据（AGENTS.md:58–60）；HANDOFF 固化记录自发携带 commit SHA（HANDOFF.md:28、34 等各站记录）。缺 = CI 运行记录类 agent 不可伪造锚；仓内 push CI（ci.yml:9–12）已产出可链接的运行记录，挂接成本低 |
| D3 门禁变更 HARD 人审门（L530） | **部分既有 + 已有对应议题候选**：ASK 闸覆盖四类高危面（AGENTS.md:44；self-review SKILL.md:110–112）；任务 8 D9 候选 = 同一对策的仓内版（HANDOFF.md:53「直推 main 下推荐 prek 钩子闸形态 = 门禁面文件清单 + 人类确认标记，与 AGENTS.md ASK 条款共用 SSOT 清单」，待拍板）。**须修正报告附注前提**：「Metis 已有服务端 CI，与『不可绕过层在服务端』方向一致」不成立——直推 main + 不开 require checks（0030:31）下服务端是落地后复核，仓内已自认（HANDOFF.md:52）；ruleset 保住的是审计轨迹（0030:31 禁 force-push/删分支）而非合并前阻断。适配重心应放在本地门禁面文件（justfile/.pre-commit-config.yaml/deny.toml/dprint.json/crates/xtask/测试套件）的确认闸 |
| D4 门禁注入式抽检（L531） | 无既有实现。相邻物：cargo-mutants 规划（testing-strategy.md:54、86；任务 8 D4 推荐 missed=0 门禁 + 远程周跑，HANDOFF.md:39）——那是对产品测试套的变异测试，若落地可充当「测试套件逃逸率」的代理度量，与 D4 同构不同对象；对门禁脚本本身的注入无对应物。self-review SKILL.md:74 的 mutation thinking 是评审者思想实验，非机器度量 |
| D5 校准配额 + 两阶段快照确认（L532） | 无既有实现，亦无仓内种子。快照确认现状 = 单阶段人工批准（justfile:53–55；AGENTS.md:11；self-review SKILL.md:100–103），改为「先写预期再对照」无既有接口但改动面小（snapshot-review 流程加一步）；手动实现配额在仓内无任何对应物 |
| D6 例外通道先于扩容（L533） | **排序已制度化，通道本体未建**：开工门槛双完备（HANDOFF.md:288）+ 队列 11 时序硬约束（HANDOFF.md:239，工作项含编排机制 + HANDOFF 退役迁移方案）。缺 = 结构化状态协议/dashboard 的设计本身（归队列 11）；「评审队列长期为空」扩张信号无观测手段属实——仓内无评审队列概念（无 PR 流，0030:46 轻 PR 评审面不立） |
| D7 文档止血两件套 + 状态机/attic（L534） | 部分既有：freshness 注 + 机器检查限于 AGENTS.md（agent_docs.rs:239–247）；research 增补不重写纪律（AGENTS.md:68）+ dprint 排除（dprint.json:7）与「时点快照」定位自洽；队列 12 已立项且范围覆盖废弃标记/状态机/attic 方向（HANDOFF.md:240）。缺 = diff 级 freshness frontmatter、废弃标记条款、attic、gardener 巡检。报告「阈值-only 检查有公开失败先例」的提醒适配仓内：AGENTS.md freshness 注即阈值-only（只查存在性，agent_docs.rs:242–246），同理可被骗过 |
| D8 HANDOFF transient 化 + evergreen 重写循环（L535） | 部分既有：易失声明（0012:8、16；AGENTS.md:43、69）+ 退役节点钉死（HANDOFF.md:288，退役 = 迁移非删除）+ 火花逐字迁移不压缩（AGENTS.md:54；HANDOFF.md:258）= 「保原文优先」的一个点状实例。缺 = TTL/到期 attic 机制、压缩三原则（保原文优先/压缩者不自评/原文可恢复）的制度化；队列 12 已含「HANDOFF 压缩纪律」（HANDOFF.md:240） |
| D9 actionable 入场测试 + 打断预算（L536） | 半成品既有：决策点编号选项 + 推荐第一（AGENTS.md:61）；议题出示格式 + 小决策点合并成批（HANDOFF.md:269）。缺 = 每条入场测试（议题与状态汇报两级）与每日打断预算；改造落点 = AGENTS.md Reporting 节与 HANDOFF.md:269 格式纪律 |
| D10 导览置信闸门（L537） | 无既有实现。最近似物 = explain-diff 的诚实性元素：区分观察事实与解释（SKILL.md:70）、交付时声明假设与验证限度（SKILL.md:95–97）、quiz 暴露理解缺口（SKILL.md:72–87）。缺 = 质量不足时返回警告而非低质摘要的闸门；改造落点 = explain-diff SKILL.md 工作流第 5 步（SKILL.md:33–37） |

---

## 附：本次盘点的覆盖与限制

- 覆盖：报告映射建议章 L485–543 全文、RQ1 三分矩阵 L74–90；仓内读取面见文首。10 项机制逐项核实完毕；任务给出的 8 条漏项候选线索全部核实为存在（commit-msg 钩子 / prek 钩子链 / cargo-deny / agent-check 预算 / ruleset / 火花机制与登记区 / dprint 排除 / ASK-first），另补充 6 条（L3 secret-scan、L10 开工门槛、L11 触发器登记、L12 diff 规模预算、L13 rule-maintenance、L14 保鲜例程群），漏项合计 14 条。
- 限制：hosting 侧实际配置（ruleset/squash-only）以 ADR-0030:31、52 的文字记录为据，未调 GitHub API 复核当前实况——若托管侧配置此后被改动，本条失效。HANDOFF.md 为 gitignored 易失文档，其行号随每次重写漂移，引用以 2026-10-14 第三十会话交接版为准。
