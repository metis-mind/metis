> 状态：point-in-time 归档（2026-10-14）。本文件 = HANDOFF.md（仓库根、gitignored 的会话交接物；制度由 [ADR-0012](../decisions/0012-docs-governance.md) 立「易失交接物」定位，2026-10-14 随任务 11 收官退役 = [ADR-0032](../decisions/0032-operating-protocol.md)）最终版全文的逐字归档。分隔线以下为归档正文（HANDOFF.md 最终版，第三十二会话交接态），永不改写——更正走增补节；会话衔接现行面 = [operating-protocol](../design/operating-protocol.md) + [backlog](../backlog.md)。

---

# Metis — 项目 Handoff

> 交接时间：2026-10-14 第三十二会话（**任务 11 中途交接：议题出示 → 用户三条异议 → 改稿四点 → 全部拍板确认，零入库、执行未开始；执行包在 §2「任务 11 执行包」，续跑直接按它操作**）；上一站收官不变：任务 8 全收官（ADR-0031，`77ed21d`，已推 main）。本文档面向**无对话上下文的新 agent**，读完应能直接继续工作。
> 本文件是**易失交接物**（gitignored，不入库，看完即销毁/重写）——持久事实一律以 `docs/` 入库文档为准。**本文件自身是任务 11 的退役对象：收官 commit 将删除本文件并归档 `docs/research/handoff-era-archive.md`（Q10 已拍板）；本份重写已把 §2 旧过程记录压缩为 SHA 索引档（耐久内容全部在 docs/ADR，膨胀治理 = 火-2 对症）。**

---

## 1. 项目目标

开发一款**自我进化 agent**（代号 Metis，2026-09-30 前旧称 metawheel）：

- **Rust 实现核心**（fiber/context/service 运行时、loader、HMR、配置协调）
- **Luau 实现扩展**（ADR-0021）：扩展者可完全用 Luau 写插件；TS→Luau SDK 轨道登记为扩展位（v1 不做）
- 形态：核心是一个**长驻 server**，常态更新只动插件（热更新）；核心自身更新走干净重启（ADR-0023）

**产品模型**：core = 插件管理与组合机器 + syscall 层 VM 基底；core **不提供任何 agent 相关服务**；llm/tools/fs/session/memory 等一切领域功能 = 插件。由此**插件语言的上限 = 生态实现能力的上限**。

**载体分层（ADR-0024，盒管模型）**：管 = Luau（薄编排、文本即插件、毫秒热更）；盒 = wasm 组件（插件包内能力制品、无插件身份、实例即弃热换、IO 全走 host imports 可 journal）；固定盒 = 核心内建件；例外通道 = T2 进程插件（撤销一等位）。**接缝能力集（ADR-0025）**：syscall 层 = 广而抽象的领域中性能力、一律业务粒度接口 → Luau 薄编排。

参考对象：DeepSeek Harness 及其核心组件 Cordis；理论论文 _A Programming Paradigm for Spatiotemporal Composability_（arXiv 2608.25512）。详见调研报告。

## 2. 当前状态（2026-10-14 第三十二会话交接）

**阶段链**：调研 ✅ → 工程化基线 ✅ → 核心设计收官 ✅ → 开源迁移 ✅ → 载体分层（ADR-0024）✅ → 接缝能力集（ADR-0025）✅ → ABI 十章（ADR-0026）✅ → 配置格式（ADR-0027）✅ → 实例模型（ADR-0028）✅ → journal（ADR-0029）✅ → 工程化阶段 2（ADR-0030）✅ → 测试与质量策略（ADR-0031，`77ed21d`）✅ → **任务 11（1+N 运行协议 + HANDOFF 退役）进行中：拍板完毕、零入库、执行未开始**。

**工作树状态**：干净（HEAD = `77ed21d`，已推 main）。

### 历史收官索引（2026-10-14 压缩档——旧过程叙事已压，溯源 = commit SHA + docs/ADR 本体）

| 任务                                 | 冻结     | commit                                                                                                                         |
| ------------------------------------ | -------- | ------------------------------------------------------------------------------------------------------------------------------ |
| A 系列 ABI 十章                      | ADR-0026 | `9e86619`/`0a9ce79`/`1558106`/`467d9db`/`f624c5e`/`0c5cd95`/`da4357a`/`5c6cad5`/`31715ba`/`ac4ddfa`/`1cf7d79` + 固化 `5a33629` |
| 任务 4 配置格式                      | ADR-0027 | `03fc75a`/`13b6e09`/`feb0689`/`e4ea690` + `5325bb8`                                                                            |
| 任务 5 实例模型                      | ADR-0028 | `7d1339a`/`7a76556`/`37328bf`/`2a1f6a4` + `6d81171`                                                                            |
| 任务 6 journal                       | ADR-0029 | `68a3750`/`53a7846`/`0fc4962`/`7a3032c` + `ea5a64f`                                                                            |
| 任务 7 工程化 2 + 三资源吸收         | ADR-0030 | `026d15a` + `b24c1d2`                                                                                                          |
| 任务 8 测试策略（教义五条 + D1–D12） | ADR-0031 | 调研入库 `bc13c4f` + 火花规则 `100cc1f` + 冻结 `77ed21d`                                                                       |

每站均 = 用户拍板 + `just ci` 全绿 + self-review 子代理判断 pass 全修后 commit+push；过程教训已蒸馏进 §7 Pitfalls。

### 设计收官表

| 设计块                    | 状态      | 落地                                                                                                  |
| ------------------------- | --------- | ----------------------------------------------------------------------------------------------------- |
| Fiber                     | ✅ 全部   | ADR-0010；§9 = ADR-0019；§10 = ADR-0023                                                               |
| Context 面与事件系统      | ✅ 全部   | ADR-0011/0013/0014–0017 + context-events.md；D5 由 ADR-0022 修订；ADR-0011 首格由 ADR-0026 局部取代   |
| 服务注册表 / inject       | ✅ 全部   | ADR-0018 + service-registry.md；四格由 ADR-0026 局部取代/细化；A9 行三由 ADR-0027 批次 #14 收窄       |
| crate 划分 + lib 模块体系 | ✅ 全部   | ADR-0019；五个 crate 骨架就位                                                                         |
| 扩展语言选型              | ✅        | ADR-0021（+ ADR-0022）；wasm 行局部 supersede 见 ADR-0024 §7                                          |
| 载体分层（盒管模型）      | ✅        | ADR-0024；spike 实测 = wasm-research.md 补遗 1                                                        |
| 接缝能力集                | ✅ 冻结   | ADR-0025 + seam-capabilities.md（living）                                                             |
| Luau ABI                  | ✅ 冻结   | ADR-0026 + luau-abi.md（living）；局部取代见 ADR-0027/0028 批次                                       |
| 配置格式                  | ✅ 冻结   | ADR-0027 + config-format.md（living）                                                                 |
| 实例模型                  | ✅ 冻结   | ADR-0028 + instance-model.md（living）                                                                |
| Journal                   | ✅ 冻结   | ADR-0029 + journal.md（living）；v1 = 写死族骨架；扩展位三条（replay 引擎/可配审计层/插件面查询接口） |
| 测试与质量策略            | ✅ 冻结   | ADR-0031 + testing-strategy.md（living）；教义五条 + D1–D12；调研输入七件 = docs/research/            |
| 1+N 运行协议（任务 11）   | 🔵 进行中 | 执行包见下；产物 = operating-protocol.md + docs/backlog.md + ADR-0032                                 |

### 任务 11 执行包（2026-10-14 第三十二会话拍板定稿，待执行）

**拍板经过**：首批 Q1–Q10 一批出示 → 用户「没异议的不提」= Q3/Q6/Q7/Q8/Q9/Q10 按推荐通过；Q1 通过附多 agent 同改 md 顾虑（已答疑，答案须作明文原则进协议）；Q2/Q4/Q5 异议 → 改稿四点出示 → 用户「确认」全按推荐 + 一条定调（火-4）。**以下全部已拍板，勿重议；执行即可。**

**A. 最终定案十二条**：

1. **Q1**：新建 `docs/design/operating-protocol.md`（中文 living）+ 收官冻结 **ADR-0032**；AGENTS.md 只加指针。
2. **Q1 顾虑答案（进协议原则章）**：协议 doc = 教义面 read-mostly，写操作只在拍板/收官点经直推 main 串行；运行状态不住协议 doc（住 backlog）；backlog 按「一任务一块」切分，并行窗口内每 agent 只写自己任务块，「同一时刻一个任务只有一个写者」结构性避冲突；文档结构性改动视同共享契约串行（接 Q5 原则）。
3. **Q2 改稿**：新建 **`docs/backlog.md`**（中文、git 追踪、agent 全权维护）= 任务队列 + 遗留登记 + 火花区 + 依赖图四位一体。**GitHub Issues 回归对外接口本位**——不建 label、不把队列迁 issue；外部事项被采纳时才以「链接条目」进台账（单写者、无双向同步）。首批出示的 Issues 方案已废弃。
4. **Q2.2（新增）**：AGENTS.md Documentation 节升级为**完整文档地图**（decisions 编号即索引不逐件列；design/research/backlog 逐件一行 = 路径 + 一句话职能 + 状态）；xtask `agent-check` 扩展**孤儿检查**（docs/design、docs/research、docs/backlog.md 的相对路径须在 AGENTS.md 文本出现，缺失即失败）——**ASK 已授权碰 `crates/xtask`**。
5. **Q3**：衔接协议落 backlog——启动序列 = 读 AGENTS.md 地图 → `docs/backlog.md` 活跃面 → 任务涉及 living doc 状态头；收尾 = 更新任务块状态行（状态词+已完成范围+下一步+未拍板项+评审面三数）随当次 commit 入库。**原则（用户原话）：「我们不能依赖任何非 git 追踪的文件。不然会非常容易出现悬垂指针，导致文档腐烂。」**
6. **Q4 改稿**：火花区住 `docs/backlog.md`（verbatim-first 不变；消费时升格进对应 docs/ 并记去向、条目压行留痕）；AGENTS.md spark 规则改指向；火-1/2/3 逐字迁入 + 新增火-4/火-5（§5 登记区）。
7. **Q5 改稿**：**弹性并发窗口模型**（废弃升级阶梯）。常态 1+1；开窗闸门 = RQ7 十二前提转写检查单（本批任务过切分三属性 independent/bounded/verifiable + 无共享契约交叉 + 维护者评审预算余量 + worktree/bootstrap 就绪）；窗口形态 = N=2–3、每 agent 独立 worktree、明确关闭条件（本批合并完毕）；关窗回落 1+1，不维持常备并发；遥测回答「哪类窗口开得值」。worktree bootstrap 自动化进实现期清单（届时再建）。设计动因 = 火-5。
8. **Q6**：变更注入 SOP——三杠杆按代价升序 queue（默认，边界处投递）→ interrupt（方向错误）→ rewind（污染落地，先 commit）；低频、保序、局部（只 replan 受影响 agent）；两陷阱入章（queue 非严格 FIFO 可在下个 pause 打断在途；agent→agent 便条静默取消接收方在途 turn——steer/interrupt 必须显式、默认非破坏）。
9. **Q7**：评审容量遥测三字段（拍板决策点数 / self-review findings 数 / 评审耗时量级）住 backlog 任务块状态行，诊断位不设门禁。
10. **Q8 五条**：(a) AGENTS.md Reporting 加「验证声明优先挂 agent 不可伪造锚（CI 运行 URL / commit SHA）」；(b) actionable 入场测试（议题每条须要求一个决策或行动，纯通报降级台账不占拍板批）进 Reporting + 协议；(c) explain-diff skill 加置信闸门条（质量不足时明示「无法可靠摘要」而非流畅低质摘要）；(d) 每实现站维护者全文抽查 ≥1 个 commit 原始 diff（ISA 330 非零下限 1+1 版，初值一站一件）；(e) 校准配额立原则不钉参数（每实现站维护者亲手实现/调试至少一个片段）；抽样框独立化 = 扩展位（触发器 = 实现期代码上量后按缺陷逃逸率先验分层再立项）。
11. **Q9 触点清零**（清单见 B）+ **Q10**：收官 commit 即完成迁移——HANDOFF.md 删除 + 归档 `docs/research/handoff-era-archive.md`（point-in-time 声明 + 最终版全文逐字）。
12. **用户定调（火-4）**：backlog.md = **临时方案**，长期需要一个工具承担任务状态管理，而非并发 agent 直接读写该文件——协议扩展位登记（消费点 = 任务系统工具化立项）。

**B. 触点清零清单（普查已验证 = 8 处 + 本体）**：

- `docs/decisions/0012-docs-governance.md`：三处进 ADR-0032 supersede 批次（Decision 分类学格的 HANDOFF 半格 + Consequences 交接纪律句 + 分类学新增 `docs/backlog.md` 类）；**正文冻结不动，只改 Status 行**加批次注（范例 = ADR-0021 Status 行）。
- `docs/decisions/0030-engineering-phase-2.md` L46-47 条款 4 三件套之「HANDOFF 收工记录」件 → supersede = 「backlog 任务块状态行收尾」；只改 Status 行。
- `docs/design/testing-strategy.md:175-176`（§6 定稿记录「议题出示留存与调研回流记录在 HANDOFF（gitignored）」）→ 改写写实（当时会话交接物，已退役，考古 = docs/research/handoff-era-archive.md）。**章节号不动纪律**（ADR-0030 正文引用 §5/§6）。
- `AGENTS.md` 四处：① NEVER 节删 HANDOFF 行；② Style spark 条改指 `docs/backlog.md` 火花登记区；③ Documentation 节删 HANDOFF 条 + 升级完整文档地图；④ Reporting 节加两条（独立证据锚 + actionable 入场测试）。行数预算 250（现 69）。
- `CONTRIBUTING.md:80` 删 HANDOFF 句 + 加 backlog 一句英语指针。
- `.gitignore` L19-20：删注释行 + `/HANDOFF.md` 行。
- `.agents/skills/adding-dependencies/SKILL.md` L53-56：「roadmap / handoff」→ 指 `docs/backlog.md`。
- `.agents/skills/explain-diff/SKILL.md`：加置信闸门条（位置候选 = Prose 条 L68-70 附近或 Handoff 节）。
- `HANDOFF.md` 本体删除（delete_path，最后一步）。

**C. 新建三件内容要点**：

1. `docs/backlog.md`：头部状态行（living 活状态，ADR-0032 立）+ 读写纪律（agent 全权维护 / 维护者聊天驱动 / 收官任务压单行 / 并发窗口一任务一写者 / **临时方案——长期工具化 = 火-4**）；依赖图区（mermaid 一张，任务变更时重生成）；任务区（一任务一块：状态词 / 依赖 / 当前进展状态行含评审面三数 / 产出指针 ADR+SHA）；内外分类。初始迁移：T9（deferred = 无限期延后，重开 = 用户明示）、T10（blocked by T11+T12；开工门槛 = 双完备 + 迁移完毕）、T11（本任务，收官即压行）、T12（输入 = RQ8）；遗留五条随迁（默认插件集 / global 可达性复审 / Value::Bytes 触发器 / 秘密引用收紧裁量 / org 级 Actions 开关含操作步骤）；火花区火-1～火-5。
2. `docs/design/operating-protocol.md`：①定位与原则（教义/状态两面分离；git 追踪原则；外部服务只作对外接口不作状态 SSOT）；②常态工作循环 1+1（议题出示 actionable 入场测试 → 批量拍板 → 固化 → just ci → self-review → commit+push；会话启动/收尾序列；**台账摘要义务** = 议题出示时附活跃任务数/阻塞链/待拍板项）；③弹性并发窗口；④变更注入 SOP；⑤评审面运行纪律（Q8 五条）；⑥扩展位登记（台账工具化 = 火-4 / dashboard 正式形态 / 抽样框独立化）；附录 A 开窗检查单（RQ7 十二前提转写 + 仓内映射 + 验收方式）；附录 B 外部关键参考（§8 链接类迁移）。
3. `docs/decisions/0032-*.md`：Context（任务 11 三件套；RQ7 + 映射建议输入；开工门槛）+ Decision（协议冻结面原则 + HANDOFF 退役 + 分类学新增 backlog 类）+ supersede 批次三条 + Consequences（**任务 10 开工门槛自此成立**；扩展位；首创面如实登记）。

**D. 执行顺序**：① write backlog.md → ② write operating-protocol.md → ③ write ADR-0032 → ④ **归档件用 terminal cat 组装**（头部声明 + HANDOFF.md 全文逐字 → handoff-era-archive.md，`cmp` 复检；**禁用 edit_file 改正文**——落盘即格式化；dprint 已排除 docs/research/**）→ ⑤ edit AGENTS.md/CONTRIBUTING/.gitignore/两 skill/testing-strategy（改完 grep 复检落点）→ ⑥ edit ADR-0012/0030 Status 行 → ⑦ xtask agent_docs.rs 孤儿检查 + 单元测试 → ⑧ `direnv exec . just ci` → ⑨ self-review 子代理 → ⑩ delete HANDOFF.md → ⑪ commit+push。

**E. ASK 面记录（已授权）**：`crates/xtask`（孤儿检查）、AGENTS.md/CONTRIBUTING/skills/.gitignore 改写、ADR supersede 批次。**不碰** justfile/hooks/workflows/deny/dprint 配置。

**F. 杂项**：commit 措辞惯例 = `docs(scope): subject`（见 `git log`；type 白名单 = `crates/xtask/src/commit.rs`）；建议 `docs(process): add 1+N operating protocol, retire HANDOFF (ADR-0032)`；body 散文单段 ≤7 行、行首避开裸 `word:`。**未确认项**：mermaid 在用户日常视图是否渲染——固化后用户看一眼 backlog，不行补纯文本缩进树冗余。

## 3. 文档地图

- `docs/decisions/` — **31 条 ADR**（0001–0031；局部 supersede 标法见 AGENTS.md Documentation 节；批次范例 = 0026/0027/0028/0029 末节）
- `docs/design/`：fiber / context-events / service-registry / seam-capabilities（ADR-0025）/ luau-abi（ADR-0026）/ config-format（ADR-0027）/ instance-model（ADR-0028）/ journal（ADR-0029）/ testing-strategy（ADR-0031）——全部冻结 living；session-model = 已吸收历史 SSOT
- `docs/research/`：cordis-research / journal-replay-research / wasm-research（含补遗 1–4）/ luau-toolchain-spike / 评审经济学七件（正本 + admission-check + cross-verification + rq6-inventory + testing-practice 三简报）
- **将随任务 11 收官新增**：`docs/backlog.md`（活状态台账）、`docs/design/operating-protocol.md`（ADR-0032 living）、`docs/decisions/0032-*.md`、`docs/research/handoff-era-archive.md`

## 4. 后续任务队列（形态将随任务 11 收官迁 docs/backlog.md）

9. ~~marketplace 格式设计~~ → **deferred（2026-10-14 用户：无限期延后；重开条件 = 用户明示）**
10. 实现（**开工门槛 = 审计流程 ✅ + 开发流程（任务 11 收官即完备）+ HANDOFF 迁移完毕（同收官 commit）**；实现序 = metis-value → metis-fiber-core → 事件系统（含 journal 拦截点）→ 服务注册表/inject → loader+配置 → Luau 绑定 → 安装事务；各实现期清单 SSOT = ADR-0027/0028/0031 Consequences）
11. **1+N 运行协议 + HANDOFF 退役** → 🔵 **进行中**（执行包 = §2）
12. 文档防膨胀机制（输入 = RQ8；范围 = 入口闸门已有 + 存量生命周期 + 机器膨胀信号 + 压缩仪式；症状三分类 = 火-2）

## 5. 遗留问题登记（形态同上随迁；仅列开放项）

- **默认插件集**（开箱即用参考实现）——原则已定（ADR-0024 §4）；shell tool / schedule 插件候选在案；核心设计链已收官，可立项
- **global 核心数据根可达性复审**（self-review 提出）：若收紧，改 luau-abi §A3.7 一行
- **Value::Bytes 提正触发器**（A6.4）：首个跨插件二进制消费者 → 走 ADR 演进
- **秘密引用限审批档收紧裁量**（D67 明记代价；用户裁量未做）
- **org 级「允许 Actions 开 PR」开关**：org Settings → Actions → General → Workflow permissions 勾开（repo 级随后同开），或 `gh auth refresh -h github.com -s admin:org` 后走 API；未开前 flake-update/toolchain-update 周更在开 PR 一步 403，其余 CI 不受影响

各类**扩展位/复审触发器**均已在对应 ADR/设计文档内登记（SSOT 原位不动，不迁）。

**火花登记区**（机制 = AGENTS.md Style 节 spark-capture 规则；引文逐字迁移不压缩、长寿命升格入仓、未消费不静默消失、议题出示前回读；任务 11 收官后迁 `docs/backlog.md`）

- 火-1（2026-10-14；已消费 = 调研 prompt v2；消费中 = 任务 11）：「我现在提这个模式的调研，是因为其形态类似，逻辑相同，结构基本相似」+「我们的流程是动态的，有可能开发到一个阶段时会有新的需求，或者方向发生改变，现在这样就是一个活生生的例子」
- 火-2（2026-10-14；已消费 = 调研 prompt v2；**未消费 = 任务 12**）：「我们的整个结构，落到文档的地方，不管是否是用户看的地方，我们都需要有一个机制来防止其膨胀。目前HANDOFF的膨胀就很严重，以及docs里面的文档的膨胀也很严重，里面有一些是重复逻辑的出现，有一些是没有记录价值信息的堆叠，以及一些我没有想到的问题。这里是文档维护的责任，也是我们在架构设计之初就应该考虑的问题，这应该也是调研的一部分」
- 火-3（2026-10-14；已消费 = ADR-0031 D7 三层跳板 + 测试实践三简报）：「测试需要分为单元测试和集成测试，单元测试主要测试逻辑功能，集成测试主要结合业务测试，综合考虑测试覆盖率，而具体的测试方案也极多，我们需要考虑开发方案的不同，我们使用的是 1 + N 开发方式，需要结合我们的开发特点来做。这里我提出其中一个细节的方向，引入 lean4 等严格的测试方式来做测试，覆盖率强，并且能够有效控制我们的暴露点（这点很重要，调研中应该有报告）。当然，这应该只算测试方案的一个小部分，其他的方向还需要你做一些思考，并判断调研深度。」
- 火-4（2026-10-14；消费点 = 任务系统工具化立项，未消费）：「先做临时方案吧，后面我们应该需要一个工具去做这个事情，而不是让并发的agent去读写这个内容。」；上下文锚 = 任务 11 改稿拍板回复（backlog.md 定调）
- 火-5（2026-10-14；已消费 = Q5 弹性并发窗口改稿；长寿命价值 = 窗口模型辩护依据）：「这里逐个升级让我感觉奇怪的地方就是，很多时候我们并不是每时每刻都需要多agent协作，更多的时候是发现这里可以并发协作，或者有时候比较急，就可能需要开多并发。这也是让我觉得棘手的地方，需求的临时性而不是长期性。当然并发需求是长期的，但是大量并发需求是偶然且短期的。」

## 6. 关键约束与偏好

- 用户偏好：**先调研/讨论后动手**；中文交流
- **议题出示格式**：先背景再取舍分析；不假设用户了解外部机制（引用就地一句话简述）；术语首次出现处给解释；英文更直观的术语保留英文原词；小决策点合并成批一次拍板；没异议的部分用户不重述（只讲异议）
- **汇报纪律 = AGENTS.md Reporting 节**（状态词固定十词带范围 / 验证四要素 / 证据 `path:line` / 编号选项推荐第一标「（推荐）」/ 禁「可能大概」写「未确认」+确认方法）
- **git 追踪原则（2026-10-14 用户原话）**：「我们不能依赖任何非 git 追踪的文件。不然会非常容易出现悬垂指针，导致文档腐烂。」——运行状态 SSOT 一律 git 追踪；外部服务只作对外接口
- **出示推荐前先对账既定方向**（grep 相关 ADR/调研关键词再出示）
- **A 系列总基调 = 严格制**；**检查时机偏好 = 启用期**；**显式状态之外要 warn 人读面提醒**；**定义唯一住处 + 重复税零容忍**；**音量/成本论据押结构界，不押「典型负载」**
- **大方向疑问随时可推翻近日定案，agent 不得因沉没成本护短**；收缩处置 = 全文留档转扩展位预制方案
- **核心不提供任何 agent 领域服务**；**T2 子进程插件已撤销一等位**；**1b 仅登记扩展位不主动提议**
- 用户对 Lua 不熟——胶水/ABI 议题给具体 Luau 代码例子；概念性提问的解答当场沉淀进设计文档
- 用户的追问 = 议题补洞信号——判析出一般规则就沉淀为 D 条
- agent 终端是无 direnv hook 的裸 shell：工具链命令用 `direnv exec . <cmd>`（**含 `git commit`**）
- 提交规范：改良版 Conventional Commits（`type(scope): subject`，ASCII subject ≤100 字符），commit-msg 钩子强制；无 PR 直推 main，commit+push 一体
- **新依赖先问**（adding-dependencies 技能）
- **工作节奏**：议题出示 → 批量拍板 → 固化（living doc + 冻结 ADR）→ `direnv exec . just ci` → self-review（作者机械 sweep + 子代理判断 pass）→ commit+push
- **HANDOFF = 过渡层工具，任务 11 收官即退役**（2026-10-14 钉死）；任务 10 开工门槛 = 审计流程 ✅ + 开发流程（任务 11）+ HANDOFF 迁移完毕（同收官 commit）

## 7. Pitfalls（务必遵守）

- **不要给模型运行时 eval 定义插件的能力**（Harness 走过并已退役此路）；**loadstring 已 tombstone**（seam §1.3 同一条红线）
- **不要照搬 Node 特有部分**（Proxy 拦截、原型链、声明合并、`!!js` eval、ESM/CJS 双缓存等，完整清单见调研报告）
- **不要假设 Cordis 有 `ready`/`dispose`/`fork` 公开生命周期事件**——vendored 分叉版是纯 Fiber 模型，只有 internal/* 钩子；**Cordis 核心也无 timer 原语**
- **不要把论文对 harness 的论述当已验证结论**（那是动机，实证在 Koishi）
- `cordis.js.org` 文档站抓不到内容；实现细节以 vendored 源码为准（`/home/duskgrow/project/demo/deepseek-harness/vendor/cordis`）
- 论文只能从 PDF 提取（定理编号可能有 ±1 偏差）；vendor README 写 4.0.0-rc.7，实际 package.json 是 4.0.4
- **分拆提交陷阱**：workspace `members = crates/*` glob 会收纳未跟踪 crate 目录——docs 与 crates/lockfile 分两个 commit 时钩子会改写旧 lockfile 与 prek 暂存还原冲突。要么单 commit，要么 crates+lockfile 先行
- **上游 yank 会随时阻断 `just deny`**——`cargo update -p <crate>` 机械修，lockfile 变更随当前 change 单 commit
- 改名后必须 `cargo clean`（xtask/insta 烘焙绝对路径）
- **`AGENTS.md.bak`（仓库根，未跟踪）是 Cadmus 的 AGENTS.md 参考物**——切勿误 `git add -A` 提交
- **grep 工具默认跳过 gitignored 文件**（HANDOFF.md 与 `scratch/` 搜不到不代表不存在，用 read_file 直读）；**include_pattern 相对项目根、须带根目录名前缀**（如 `metis/docs/**/*.md`）
- **Nix（rust-overlay）工具链不能 `rustup target add`**——构建 wasm guest 需 rustup 自管工具链
- **Luau 整数链路**：mlua 的 Value::Integer 写路径走 f64 丢精度——大整数禁止经 `mlua::Value` 中转；精确注入收口在 `metis-luau` 转换层内部（raw push，代码原型 = wasm-research 补遗 3）；移除触发器 = mlua 上游适配写路径
- **入库文档引用实测数据只能指入库 SSOT**（wasm-research 补遗），**不指 `scratch/`**；入库文档引用会话编号必须首现处附自解释内容名
- **`just fmt`（dprint）对触碰的 markdown 整表重排**——核对内容改动用 `git diff -w`；**edit_file 的 old_text 以 fmt 后现状为准**；**批量 edit 中某条失败时，报错条与后续条全部跳过、前序已应用条不回滚**——失败后逐条核对再带上下文重试；**new_text 只含目标句、别把相邻上下文复制进去**；**表格行 edit 的 old/new 要么同时含行尾 `|` 要么同时不含，改完 grep 行尾空单元格复检**
- **兑现登记行时先 grep 同一延迟目标的全部指向处**，兑现注落点后回 grep 验证零悬挂；**冻结批次起草时 grep「归冻结批次」类登记指针全部收进批次 manifest**；**扫描模式的词序要放宽**（「批次归冻结」≠「归冻结批次」）；**方向重定的连带清扫要覆盖政策陈述句，不只登记指针**
- **冻结 ADR 的定调/浓缩必须与 living doc 原文逐字对账**（批次条目引号内文本必须从被触 ADR 正文复制）
- **SSOT 汇总表的「已同步」声称必须属实**（写完逐行回验，宁少列不虚列）
- **文档里的示例秘密形态会触 secret-scan 钩子**——示例占位把 `pr-guard:allow` marker 放该行即过
- **子代理可能无最终消息返回**（No response from subagent + partial output）——带 session_id 追加短消息续跑即可
- **commit-msg 钩子两处形状纪律**：body 散文单段 ≤7 行；行首裸 `token:` 形状会被当 footer——换行避开行首 `word:` 起手
- **CI 文件的位置形状各有归属**：`dependabot.yml` 住 `.github/` 根而非 `workflows/`；org 级 Actions 开关未开时 repo 级设置 API 返回 409
- **反转修订的注记链终态必须唯一**：同一对象多次修订时统扫全部提及点逐一推到终态；被撤机制的引用点清零 grep 覆盖 docs/ 全域
- **「已落盘」声明必须对应实际工具调用返回**——写「已执行/已落」前回数本轮工具调用；handoff 类文件的编辑落点当轮 grep/read_file 复检
- **`edit_file` 落盘即格式化**（对 markdown 整文跑：表格重 padding、强调符改写、列表缩进改写）；**dprint.json 的 `docs/research/**` 排除对 edit_file 无效**——point-in-time 文档禁用 edit_file 改正文，改用 cat/printf 组装并 cmp 原件复检

## 8. 关键参考

- 调研报告：`docs/research/`（cordis / journal-replay / wasm 含补遗 1–4 / luau-toolchain-spike / 评审经济学七件）
- ADR-0024～0031：载体分层 / 接缝能力集 / Luau ABI / 配置格式 / 实例模型 / journal / 工程化阶段 2 / 测试策略（细节 SSOT = 对应 living doc）
- **吸收的外部资源**：Fenng/Tech-Doc-Style-Chinese（MIT → chinese-doc-style skill）；dualface/ste-zh（MIT → AGENTS.md Reporting 节）；geoffreylitt explain-diff gist（无 license → explain-diff skill）
- Luau require-by-string RFC（A1 语义基座）：`docs/new-require-by-string-semantics.md` + `docs/amended-require-resolution.md`；`docs/support-for-cyclic-requires.md` = proposal
- **`scratch/`（gitignored 一次性产物）**：`scratch/mlua-spike/RESULTS.md`、`scratch/wasmtime-spike/RESULTS.md`、`scratch/评审经济学调研报告/`（调研交付包原始件）、`scratch/测试实践调研/`；入库文档不得引用 scratch
- Luau RFC 仓：`https://github.com/luau-lang/rfcs`；DSH 实体（本地）：`/home/duskgrow/project/demo/deepseek-harness`；兄弟项目 Cadmus：`/home/duskgrow/project/rust/Cadmus`；工程化模板：`../rust-template`
- Harness：`https://github.com/deepseek-ai/deepseek-harness`；上游 Cordis：`https://github.com/cordiverse/cordis`；论文 PDF：`https://arxiv.org/pdf/2608.25512`
- org `metis-mind` 双仓：`metis`（public）+ `marketplace`（空壳占位）；remote = `git@github.com:metis-mind/metis.git`；git 身份 = `duskgrow <git@duskgrow.dev>`（repo-local）；gh CLI 已认证
