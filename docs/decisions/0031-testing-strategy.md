# 0031. 测试与质量策略：评审经济学教义与 D1–D12 定夺

- Status: accepted
- Date: 2026-10-14

## Context

任务 8（测试策略大讨论）分两站。第 1 站出示 D1–D8 议题未拍板；用户立「1+N 评审经济学」元问题（人不可能看 N 个 agent 的全部产出：看的部分怎么选、怎么表达，不看的部分如何自洽，看得越少越看不懂时心智模型如何维持），定夺 = 深度调研先行。第 2 站调研回流：外部网络调研 agent 交付评审经济学调研报告（RQ1–5 + RQ7 编排 + RQ8 文档防膨胀）；仓内补差测试实践三简报（oracle / 形式化与 DST / 并行测试工程）；入库前独立抽查 19 条行内来源 19/19 通过；RQ6 仓内盘点对账报告的机制清单与缺口判断（漏项 14 条、须修正 4 处）。吸收结论经用户整批拍板 = 教义五条 + D1–D12，本 ADR 冻结之。

调研输入全部入库（docs/research/，point-in-time）：评审经济学调研报告（review-economics-research.md，含入库补遗 A 两条）、入库抽查记录（review-economics-admission-check.md）、RQ6 仓内盘点（review-economics-rq6-inventory.md）、测试实践三简报（testing-practice-oracle.md / testing-practice-formal-verification.md / testing-practice-parallel-engineering.md）、交叉验证过程存档（review-economics-cross-verification.md）。报告 RQ5/RQ7 为 2026-10-14 时点截面，引用其中数字前复核原始来源。

## Decision

### 1. 教义（评审经济学五条）

1. **注意力是唯一预算**：机器可判维度 100% 机器查（税务稽查实证：机器可核验维度逃漏率 0.3% vs 自主申报 37%），人审预算按风险定价、只配置在机器不可判的残余象限。
2. **信赖须挣得**：对门禁的信赖 = 一次验证 + 变更受控 + 逃逸率度量 + 定期复测 + 非零下限（ISA 330 自动化控制依赖逻辑）；无逃逸率度量的信赖是未定价信任。
3. **被审者不产证据**：执行者与检查者分离（17 CFR §210.2-01 结构，「之一」口径见入库补遗 A）；汇报自述挂 agent 不可伪造的独立证据；抽样框与议程生成权与被审者分离。现状如实标注：本地钩子可被 `--no-verify` 绕过（aider #5376 / claude-code #40117 实证），远程 CI 不开 require status checks（ADR-0030 条款 2）属落地后复核——当前信任锚点 = main ruleset 保审计轨迹 + 远程复核，不是合并前门禁。
4. **呈现须过入场测试**：每条呈现给维护者的信息须要求一个决策或行动（ISA-18.2 cardinal rule / 14 CFR 25.1322 / SRE paging 三行业收敛）；单次评审单元受容量预算约束（200–400 行 / 60–90 分钟实证区间）；AI 摘要设置信闸门（质量不足显示警告而非流畅错摘要）。
5. **校准须主动维持**：批准式任务是退化条件（Endsley & Kiris 五级实验：被动加工侵蚀理解层）；快照确认两阶段化；维护者保留手动实现配额（航空业 SAFO 13002 政策先例）；「又要 AI 全干、又要人能兜底」实证上矛盾，配额占用产出时间是明示代价。

### 2. D1–D12 定夺（细节 SSOT = docs/design/testing-strategy.md 对应节）

- **D1 快慢双轨**：快轨 `just ci`（per-commit + 远程矩阵，现状不动）= L0 全量 + L1/L2 有界配置 + L3 已有语料回归；慢轨 `just deep`（本地按需 + 远程周跑）= 长程 proptest / fuzz 冒烟 / sanitizer / cargo-mutants / Kani / llvm-cov 报告 + 地板线校验；慢轨失败产物 = 带失败指纹的 issue，不阻塞 commit。流程纪律三条：新门禁件非阻塞观察期、flaky 台账先行、merge queue 触发器（≥2 并行 agent 工作流推 main 必上「测组合」）。
- **D2 依赖扫描回归**：cargo-machete 进 `just lint`（落地走 adding-dependencies）；cargo-outdated 结案 = dependabot 周更承接（ADR-0030）。
- **D3 覆盖率**：不设全局百分比；内核 crate 名单（首名 metis-fiber-core / metis-value）设行覆盖率地板线，数值首个 crate 实现收官时实测钉入、只升不降（ratchet）；diff 级增量覆盖为快轨候选件；防注水职责归 D4。
- **D4 cargo-mutants**：内核名单 + missed=0 门禁（漏网变异体补测试或 `#[mutants::skip]` 逐个注释理由；skip 清单属门禁面，改动走 D9）；远程周跑 + 按需；误报率先校准再硬化；降级触发器 = 周跑超时或信噪比失控 → 转 diff 增量模式。
- **D5 replay 维度**：立原则不立层——seed 可重放纪律（失败日志必带 seed，禁「重跑一次试试」）、journal 条目序列进 insta 快照、失败语料固化进版本控制；快照定位纪律 = 快照证「行为未变」非「行为正确」；全套 DST = 扩展位（触发器 = 事件系统收官后实测难复现并发 bug）。
- **D6 压测基准**：三件套随实现（热更循环后内存/句柄 ≤ 基线×宽松倍数、事件吞吐数量级上界、fiber 规模内存归位），抓失控不抓回归，跑慢轨；criterion 触发器不动。
- **D7 形式化三层跳板**：Kani（实现级有界证明，随内核 crate 实现站引入；硬边界 = 不支持并发）→ Verus（SMT 级函数正确性，优先 LLM 证明生成工具链试点）→ Lean 仅 model-first 设计级试点（fiber 状态机不变量、百行级模型、Cedar 式差异测试桥复用 L2 参考模型）；hax/Aeneas 直译降级为 spike 对照；spike 判据四条预定义（证明完成度 / 人时机时 / L2 复用度 / 诊断可消费性）。措辞纪律：形式化 = 边界内全称保证，不是覆盖率语义；暴露点控制 = 排除项显式清单 + TCB 枚举到组件。
- **D8 F 类场景清单**：各实现站「崩溃窗口写实清单」的机器验证版 + 故障点×恢复承诺矩阵，住 testing-strategy §3.F；故障注入钩子 = 一等测试基建（feature-gate 隔离；自建 vs fail-rs 推迟走 adding-dependencies）。
- **D9 门禁变更 HARD 人审门**：门禁面文件清单（SSOT = testing-strategy §4.2）+ 当前形态 = AGENTS.md ASK 条款扩展到该清单；机器闸（pre-commit 钩子要求人类确认标记）进实现期清单。
- **D10 flaky 治理三件套**：失败分类闸（先判 flaky 再动作）+ 合法动作白名单（flaky 失败禁改产品代码，稳定复现失败才进修复通路）+ 隔离复活路径（继续跑不阻塞 + 工单驱动回快轨；惯犯限期制）。
- **D11 测试来源分工**：crate 级显式声明——人写/审测试（规格属性强）vs agent 写测试 + mutation 门禁（回归属性强），可混用须声明。依据 = LLM 生成测试 mutation score 实测 40–50%（腰斩于覆盖率）。
- **D12 测试膨胀遥测与回归资产**：测试/产品行数比、flaky 台账进慢轨报告（诊断位不设门禁）；语料孤儿机器拒绝（`insta --unreferenced=auto` 进 `just ci`）；测试删除候选挂 mutation 体检产出。

### 3. 演进纪律

承接 ADR-0026/0027：additive 增补落 living doc（docs/design/testing-strategy.md）修订注；破坏式变更走新 ADR supersede；参数类（地板线数值、慢轨预算、惯犯限期）实测钉入后只升不降（ratchet）。

## Consequences

- 结案：testing-strategy 原 §6 待决 1–6 全部结案（7–9 已于 ADR-0030 结案）；种子稿转冻结 living doc（状态头已指向本 ADR）。
- 实现期清单（SSOT）：cargo-machete 进 `just lint`（走 adding-dependencies）；`just deep` recipe + 远程周跑 workflow（随第一个慢轨件 cargo-mutants 周跑落地时创建，届时碰 `.github/workflows/`）；llvm-cov 地板线校验（首个内核 crate 收官实测钉入数值）；`insta --unreferenced=auto` 进 `just ci`；flaky 台账与膨胀遥测（慢轨报告件）；门禁面 pre-commit 机器闸；Kani 随内核 crate 实现站引入；proptest / bolero / fuzz 引擎 / failpoint 件等 dev 依赖届时逐个走 adding-dependencies。
- 登记触发器：merge queue（≥2 并行 agent 工作流推 main）；DST 全套（事件系统收官后实测难复现并发 bug）；Miri（crate 局部放开 unsafe，须先过 ADR）；Verus 试点（disposer/epoch 内核收官）；Lean spike（fiber 状态机实现收官）。
- ASK 面已授权（2026-10-14 随拍板）：`just deep` recipe 与远程周跑 workflow（届时碰 `.github/workflows/`）；门禁面 pre-commit 机器闸（届时碰 `.pre-commit-config.yaml`）。
- AGENTS.md 同批两处修订：snapshot-review 改两阶段（先写预期再对照批准）；ASK 面从「CI workflow 文件」扩展到门禁面清单（SSOT = testing-strategy §4.2）。
- 首创面如实登记：明示人审预算/抽样评审、flaky 毒化自修复防御、测试评审路由——公开文献无具名先例（调研否定发现），有效性以慢轨遥测（缺陷逃逸率、flaky 率、误改回滚率）验证后再硬化。
- 评审经济学向编排轴与文档轴的延伸（RQ7 同构框架、RQ8 防膨胀机制清单）归队列 11 / 队列 12 处理，不在本 ADR 范围。
