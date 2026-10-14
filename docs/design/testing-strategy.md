# Metis — 测试与质量策略

> 状态：✅ 已冻结 = [ADR-0031](../decisions/0031-testing-strategy.md)（2026-10-14）；本文档为 living doc。演进纪律（承接 ADR-0026/0027）：additive 增补（新层件、新触发器、参数实测钉入）落本文档修订注；破坏式变更走新 ADR supersede。
> 定夺的紧凑重述与依据回溯见 ADR-0031。调研输入（均 point-in-time）：[评审经济学调研报告](../research/review-economics-research.md)（含入库补遗 A）+ [RQ6 仓内盘点](../research/review-economics-rq6-inventory.md) + 测试实践三简报（[oracle](../research/testing-practice-oracle.md) / [形式化与 DST](../research/testing-practice-formal-verification.md) / [并行测试工程](../research/testing-practice-parallel-engineering.md)）；入库验证 = [独立抽查记录](../research/review-economics-admission-check.md)（19/19），调研方自检存档 = [交叉验证](../research/review-economics-cross-verification.md)。
> ADR-0030 正文引用的「testing-strategy §5 / §6」即本文档 §5 与 §6（章节号保持不动）。

---

## 1. 教义与元规则

### 1.1 教义（评审经济学五条）

全部机制从五条教义推出；依据回溯见 ADR-0031，此处只写操作含义。

1. **注意力是唯一预算**：机器可判维度 100% 机器查，人审预算按风险定价、只配置在机器不可判的残余象限。
2. **信赖须挣得**：对门禁的信赖 = 一次验证 + 变更受控 + 逃逸率度量 + 定期复测 + 非零下限；无逃逸率度量的信赖是未定价信任。慢轨（§2.4）承载定期复测；cargo-mutants（§3.C）是测试套件的逃逸率代理度量。
3. **被审者不产证据**：执行者与检查者分离；汇报自述挂 agent 不可伪造的独立证据（CI 运行记录）；抽样框与议程生成不来自被审者。现状如实标注：本地钩子可被 `--no-verify` 绕过，远程 CI 不开 require checks（ADR-0030 条款 2）属落地后复核——当前信任锚点 = main ruleset 保审计轨迹 + 远程复核，不是合并前门禁。
4. **呈现须过入场测试**：每条呈现给维护者的信息须通过「是否要求一个决策或行动」测试，否则降级到常驻层；单次评审单元受容量预算约束；AI 摘要设置信闸门（质量不足显示警告，不给流畅的错摘要）。
5. **校准须主动维持**：批准式任务是退化条件——快照确认两阶段化、维护者保留手动实现配额（§7）；「又要 AI 全干、又要人能兜底」在实证上矛盾，配额占用产出时间是明示代价。

### 1.2 元规则（源自 1+N 开发模式，见 [ADR-0009](../decisions/0009-engineering-baseline.md)）

- agent 是主要开发者 → **每个门禁必须机器可判定**（验收不需要人类判断力介入）。
- **可自动修复优先**：门禁报错 → `just fix` → 再过，构成 agent 自修复闭环。
- **本地门禁是第一道**（commit 前必跑 `just ci`）；远程 CI 跑同一套门禁（push/PR，ubuntu + macOS 矩阵）作独立复核与 dependabot / 未来外部 PR 的门禁，远程定期例程 = 依赖审计 / flake / toolchain 更新（2026-10-13 修订，[ADR-0030](../decisions/0030-engineering-phase-2.md)）。
- 门禁必须快——慢到被绕开的门禁等于没有。
- **门禁面自保护**：修改门禁面文件清单（§4.2）内的文件走 HARD 人审门——当前形态 = AGENTS.md ASK 条款；机器闸（pre-commit 钩子确认标记）在实现期清单（ADR-0031 Consequences）。

## 2. 测试分层框架（L0–L4）

| 层 | 内容                          | 工具                                                                    | 验证目标                                             |
| -- | ----------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------- |
| L0 | 单元 / 快照                   | cargo-nextest + doctest + insta（模板自带）                             | 回归网底                                             |
| L1 | property-based，纯核心        | proptest / bolero                                                       | keyed diff、epoch 反向闭包、配置合并分层等纯函数内核 |
| L2 | 基于模型的状态测试 + 故障注入 | proptest state-machine / 自研 harness                                   | fiber 生命周期对偶参考模型；F 类恢复测试（§3.F）     |
| L3 | fuzz                          | cargo-fuzz / bolero                                                     | 配置解析、loader 协调、Luau ABI 边界                 |
| L4 | 形式化三层跳板                | Kani → Verus → Lean（model-first 试点）；hax/Aeneas 直译 = spike 对照组 | 「失败即存在性风险」的小内核（暴露点控制，§2.3）     |

**L4 候选内核**（与 ADR 对应）：fiber 状态机不变量、disposer 栈 LIFO/幂等/异步汇合（[ADR-0003](../decisions/0003-module-require-discipline.md)/[0005](../decisions/0005-vm-topology.md)）、epoch 指纹失效正确性（[ADR-0001](../decisions/0001-inject-runtime-checks.md)）、keyed diff 协调、安装事务回滚（[ADR-0008](../decisions/0008-creator-mode-self-modification.md)）。

### 2.1 L4 三层跳板分工（D7 定夺）

- **Kani = 实现级跳板**：零注解起步、反例是具体执行轨迹（agent 自修复闭环可直接消费）；随对应内核 crate 实现站同步引入，proof 住码旁、进慢轨。**硬边界 = 不支持并发**——fiber 状态机的 Kani 证明对象只能是单步转移与无交织不变量，交织覆盖归 L2/DST。
- **Verus = SMT 级函数正确性层**：LLM 证明生成的证据最密集档（AutoVerus / KVerus 出版线，KVerus 产出已被真实 Rust 内核上游接受）；试点对象 = disposer 栈 / epoch 指纹失效等局部内核；引入时点 = 内核 crate 实现站之后。
- **Lean = 设计级 model-first 试点**：内核 = fiber 状态机不变量（百行级模型）；Lean 模型证定理，与 Rust 实现的一致性走 Cedar 式差异测试桥（复用 L2 参考模型——L2 模型由此升格为 L4 的信任组件而非一次性脚手架）。hax/Aeneas 直译路线降级为 spike 对照组。spike 触发器 = fiber 状态机实现收官；spike 判据预定义四条：证明完成度、人/机时记录、与 L2 模型的复用度、失败诊断可否被 agent 直接消费。
- 措辞纪律：形式化给的是**边界内全称保证**，不是覆盖率语义；不当「更强的覆盖率」使用。

### 2.2 测试来源分工（D11 定夺）

crate 级显式声明两种测试来源：**人写/审测试**（规格属性强，oracle 独立于实现）与 **agent 写测试 + mutation 门禁**（回归属性强，牙齿由 §3.C 审计）。两者可混用，但每个内核 crate 在实现站收官时必须声明其构成。依据：LLM 生成测试的 mutation score 实测仅 40–50%（腰斩于覆盖率）——凡 agent 自写测试处，mutation 校验是唯一有实证支撑的牙齿审计；规格外实现决策天然无测试（「测试即规格」的反向论证）。

### 2.3 暴露点控制纪律

验证面的画法（seL4 / Cedar / Verus 页表先例）：**排除项写成显式清单、TCB 枚举到组件**——不写清单 = 绿灯幻觉。fiber 状态机试点的建议边界（实现站细化）：面内 = 状态转移纯函数与不变量；面外 = 调度器与 OS 线程、mlua/Luau C 边界（归 §3.D）、wasmtime、真实时钟与 IO（构造注入纪律天然在界外）、多租户配额策略。

### 2.4 各层运行位置（D1 定夺：快慢双轨）

- **快轨 `just ci`**（per-commit 本地必跑 + 远程矩阵同一套，现状不动）：L0 全量（含任务 10 起伴随实现站的 arch-test）+ L1/L2 有界配置（proptest 默认 case 数 + `proptest-regressions/` 固化 seed 必重放）+ L3 已有 corpus 回归。
- **慢轨 `just deep`**（本地按需 + 远程周跑）：长程 proptest / fuzz 定时冒烟 / sanitizer 按需 / cargo-mutants / Kani 全量 proof / llvm-cov 报告 + 内核地板线校验。慢轨失败产物 = issue（带失败指纹：测试名 + 首次失败 commit + 复现命令），不阻塞 commit。
- 三条流程纪律：① **新门禁件观察期**——任何新检查先以「只记录不阻塞」身份跑若干周，flaky 率达标再转硬门禁；② **flaky 台账先行**——快轨尽早产出 flaky 率遥测（哪条测试、跨 run 翻转率），flaky 失控 = 人与 agent 一起学会无视红；③ **merge queue 触发器**——同一工作日出现 ≥2 个并行 agent 工作流推 main 时，合并面必须引入「测组合」机制（merge queue 或等价物），两两组合的正确性无法由任何单 agent 自证。
- 创建时点：`just deep` recipe 与远程周跑 workflow 随第一个慢轨件（cargo-mutants 周跑）落地时创建；本定稿只立架构不立空 recipe（ADR-0031 Consequences 登记）。

## 3. 检查/纠错工具图谱

### A. 静态检查（全部可自动修复 → agent 自修复闭环）

| 工具                                  | 抓什么                            | 跑在哪                          |
| ------------------------------------- | --------------------------------- | ------------------------------- |
| clippy（pedantic + 精选 restriction） | 惯用法错误、可疑逻辑              | pre-commit 快门禁               |
| rustfmt / dprint                      | 格式漂移                          | pre-commit（`--fix`）           |
| typos                                 | 拼写错误（代码+文档）             | pre-commit（`--write-changes`） |
| workflow lint                         | GitHub Actions workflow YAML 语法 | pre-commit                      |

### B. 依赖与供应链

| 工具           | 抓什么                                  | 跑在哪                                                                      |
| -------------- | --------------------------------------- | --------------------------------------------------------------------------- |
| cargo-deny     | RUSTSEC 漏洞、许可证、重复版本、来源    | 本地门禁 + 远程定期                                                         |
| cargo-machete  | 声明了但没用的依赖                      | 本地门禁——定夺 = 进 `just lint`（秒级源码扫描）；落地走 adding-dependencies |
| cargo-outdated | 依赖落后报告                            | ✅ 结案 = dependabot 周更承接（ADR-0030）                                   |
| cargo-vet 💤   | 依赖审计记录（1+N 下治理 agent 选依赖） | 推迟，见 §5                                                                 |
| cargo-geiger   | 依赖树 unsafe 占比                      | 信息性，可选                                                                |

### C. 覆盖率与测试质量

| 工具           | 抓什么                                                                                         | 跑在哪                                                                                                                                                                                                                                                                                                                                                 |
| -------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| cargo-llvm-cov | 哪些路径没被测试踩到                                                                           | 内核 crate 名单（首名 = metis-fiber-core / metis-value，随实现站逐站认定）设行覆盖率地板线——数值在首个 crate 实现收官时实测钉入，此后只升不降（ratchet）；校验进快轨（拖慢则降级慢轨，触发器写实登记）；名单外维持诊断用；**不设全局百分比**（百分比门槛激励注水，防注水职责归 cargo-mutants）。增量机制件：diff 级覆盖（touch-it-cover-it）为快轨候选 |
| cargo-mutants  | 变异测试：改坏代码看测试抓不抓得住——**检验测试本身有没有牙齿**；agent 写测试防「断言了个寂寞」 | 内核 crate 名单 + **missed=0 门禁**（漏网变异体要么补测试要么 `#[mutants::skip]` 逐个注释理由——skip 清单属门禁面，改动触发 §1.2 门禁面自保护）；远程周跑 + 按需，不进 per-commit；误报率先按需校准再确认周跑硬门禁；降级触发器：周跑时长超预算或信噪比失控 → 转「只变异 diff 触及行」增量模式                                                          |

### D. 动态正确性（FFI + 并发，与本项目强相关）

| 工具               | 抓什么                                   | 跑在哪                                                                                                                                             |
| ------------------ | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Miri               | unsafe/FFI 边界的 UB                     | **待命条款**：workspace lint `unsafe_code = "forbid"`，本仓无 unsafe 供 Miri 解释；未来某 crate 局部放开 unsafe 须先过 ADR，该 crate 测试必过 Miri |
| ASan / LSan / TSan | 内存错误、泄漏、数据竞争（C API 交互面） | nightly，按需——Luau C 边界正确性归此 + API 契约测试                                                                                                |
| loom / shuttle 💤  | 并发原语全排列交错测试                   | 触发：第一个手写无锁/原子结构出现                                                                                                                  |
| **Kani**           | 有界模型检查                             | L4 跳板第一级，见 §2.1                                                                                                                             |

### E. 性能回归

| 工具         | 抓什么                                 | 跑在哪                                                                                                                                            |
| ------------ | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| criterion 💤 | 热路径性能回归（事件派发、fiber 操作） | 触发：MVP 后出现公认热路径；压测三件套（热更循环后内存/句柄 ≤ 基线×宽松倍数、事件吞吐数量级上界、fiber 规模内存归位）随实现进慢轨，抓失控不抓回归 |

### F. 故障注入 / 恢复测试（项目特有类别）

测试**类别**而非现成工具，归入 L2 集成层，随安装事务实现自带。验证**回滚与干净重启承诺**（[ADR-0001](../decisions/0001-inject-runtime-checks.md)/[0008](../decisions/0008-creator-mode-self-modification.md)）。

**F 类场景清单（D8 定夺）**= 各实现站「崩溃窗口写实清单」的机器验证版（journal D86 模式先例），住本节扩充 + 故障点×恢复承诺矩阵（回滚干净 = ADR-0008 / unclean 自证 = D85 / fresh-start = ADR-0023），不立独立设计文档。故障注入钩子 = 一等测试基建（failpoint 式，feature-gate 隔离；自建 vs fail-rs 推迟到安装事务站走 adding-dependencies）。

### G. API/版本纪律

| 工具                                      | 抓什么     | 跑在哪                                        |
| ----------------------------------------- | ---------- | --------------------------------------------- |
| cargo-semver-checks / cargo-public-api 💤 | API 面漂移 | 推迟（私有项目价值低；核心 crate 稳定后可议） |

### H. flaky 治理与回归资产（D10/D12 定夺；机制而非工具）

flaky 三件套：

1. **失败分类闸**：agent 见测试失败，第一步是 flaky 判定（重跑 N 次 / 查台账），不是直接改代码。
2. **合法动作白名单**：flaky 失败允许 = 重跑判定、标记隔离、建 issue；**禁止 = 改产品代码**；只有同代码稳定复现的失败才进修复通路。
3. **隔离复活路径**：隔离 = 继续跑不阻塞 + 工单驱动修复，修复后回快轨；flaky 惯犯限期制——超限期（初值 4 周，实现期校准）未修复移出快轨，再超期删除。

回归资产纪律：

- **seed 可重放纪律**（L1/L2）：失败日志必带 seed，禁止「重跑一次试试」——结构地基 = AGENTS.md 时间/随机/IO 构造注入纪律。
- journal 条目序列进 insta 快照（核心行为链回归网，随 journal 实现站落地）。
- 失败语料固化进版本控制（proptest `proptest-regressions/` 进 VCS、先重放固化用例再生成新用例；OSS-Fuzz 语料治理先例）；语料孤儿机器可判——`insta --unreferenced=auto` 进 `just ci`（CI 中 reject、本地 delete）。
- **快照定位纪律**：快照 = 变更检测层，证明「行为未变」而非「行为正确」；agent 把快照绿当正确性证据 = oracle 从实现反推（评审经济学报告 RQ3 前提不变量「Policy 显式性」行的破坏方式）。快照确认走两阶段（§7）。
- 膨胀遥测（诊断位，不设门禁）：测试/产品行数比、每 crate 测试数增长率、flaky 台账进慢轨报告。
- 测试删除驱动挂 mutation 体检：周跑产出中「杀不死任何变异体且长期零失败」的测试清单 = 删除候选（免逐条人审）。

## 4. v1 子集与门禁面清单

### 4.1 v1 子集

- **全量上**：A 全部 + cargo-deny + cargo-machete（machete 落地走 adding-dependencies）。
- **内核 crate 名单专项**：cargo-mutants（远程周跑 + 按需）+ llvm-cov（诊断 + 名单地板线）；Miri 待命（§3.D）。
- **随实现引入**：F 恢复测试（跟安装事务走）、Kani（跟 fiber 状态机等内核 crate 实现站走）。
- **明确推迟**：vet、criterion、loom、geiger、semver-checks/public-api（触发器见 §3 各行与 §5）。

### 4.2 门禁面文件清单（D9 定夺，SSOT）

修改下列文件 = 触碰门禁本身，走 HARD 人审门（当前形态 = AGENTS.md ASK 条款；pre-commit 机器闸在实现期清单）：

- `.github/workflows/`（全部）、`.github/dependabot.yml`
- `justfile`、`rust-toolchain.toml`、`.pre-commit-config.yaml`、`deny.toml`、`dprint.json`
- `crates/xtask/`（check-commit / pr_guard / agent_docs 等门禁实现）
- 测试套件的豁免面：`#[mutants::skip]` 清单、insta 快照的批量接受、proptest 回归文件的删除
- hosting 设置（ruleset / 合并策略）——走 GitHub 设置面，ASK 条款已覆盖

## 5. 推迟登记（含触发条件，防丢）

| 项                                  | 何时捡回                                                                                           |
| ----------------------------------- | -------------------------------------------------------------------------------------------------- |
| loom / shuttle                      | 第一个手写无锁/原子结构出现时                                                                      |
| criterion                           | MVP 后出现公认热路径时                                                                             |
| cargo-vet                           | 依赖数量或 agent 自选依赖频率失控时                                                                |
| fuzz（L3）全面铺开                  | 配置格式 / Luau ABI 稳定后                                                                         |
| Verus 试点                          | disposer / epoch 局部内核实现收官后；优先 LLM 证明生成工具链                                       |
| Lean model-first spike              | fiber 状态机实现收官；判据 = §2.1 四条                                                             |
| DST 全套（虚拟时间快进/脚本化故障） | 事件系统收官后实测到难复现并发 bug；轻量前身 = L2 按 sled/turmoil 配方升级（虚拟时间 + seed 重放） |
| merge queue                         | ≥2 个并行 agent 工作流并发推 main（§2.4 纪律 ③）                                                   |
| `just deep` recipe + 远程周跑       | 第一个慢轨件（cargo-mutants 周跑）落地时                                                           |
| Miri                                | 某 crate 局部放开 unsafe（须先过 ADR）                                                             |
| ~~workflow lint~~                   | ✅ 已随 workflows 恢复回归（2026-10-13，[ADR-0030](../decisions/0030-engineering-phase-2.md)）     |
| ~~轻 PR 评审面~~                    | ✅ 结案 = 不立（ADR-0030 条款 4；触发器 = 外部贡献放开或维护者主动要求）                           |
| Windows native CI                   | 首个真实 Windows 用户需求（ADR-0030 条款 2）                                                       |
| require status checks               | 外部 PR 放开时（ADR-0030 条款 2）                                                                  |

## 6. 定稿记录（原大讨论待决清单）

2026-10-14 整批拍板通过（教义五条 + D1–D12）；议题出示留存与调研回流记录在 HANDOFF（gitignored），依据与来源纪律见 ADR-0031 Context。

| 原待决                               | 定夺                            | 落点        |
| ------------------------------------ | ------------------------------- | ----------- |
| 1. 各层运行位置与门禁强度            | D1 快慢双轨                     | §2.4        |
| 2. 覆盖率地板线                      | D3 内核名单 + ratchet           | §3.C        |
| 3. cargo-mutants 范围与频率          | D4 missed=0 + 周跑              | §3.C        |
| 4. Kani 与 Lean 分工                 | D7 三层跳板                     | §2.1        |
| 5. Lean 试点内核                     | D7（fiber 状态机，model-first） | §2.1        |
| 6. F 类恢复测试场景清单              | D8 实现站清单模式               | §3.F        |
| 7–9. review 联动 / 远程 CI / release | ✅ 已于 ADR-0030 结案           | —           |
| 新增：门禁变更保护                   | D9 HARD 人审门                  | §4.2        |
| 新增：flaky 治理                     | D10 三件套                      | §3.H        |
| 新增：测试来源分工                   | D11 crate 级声明                | §2.2        |
| 新增：测试膨胀与回归资产             | D12 遥测 + 孤儿拒绝             | §3.H        |
| 新增：replay 维度与快照定位          | D5 立原则不立层                 | §2.4 / §3.H |
| 新增：压测基准                       | D6 三件套抓失控                 | §3.E        |
| 新增：依赖扫描回归                   | D2 machete 进 lint              | §3.B        |

## 7. 校准维持（教义 5 的机制位）

- **快照确认两阶段**：`just snapshot-review` 流程加前置步——先写预期变化（一句话进 commit body 或评审记录），再逐条对照批准；确认批量受容量预算约束（批准式确认是实测的退化条件，两阶段把批准式任务改回主动加工任务）。
- **手动实现配额**：维护者定期亲手实现与调试代码（航空业 SAFO 13002 / FltDAWG 先例——退化主因是练习机会被剥夺而非自动化本身）；配额形式归维护者自律，不设机器闸。
