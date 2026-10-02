# Metis — 测试与质量策略（种子稿）

> 状态：🌱 种子稿，未定稿。正式大讨论排在结构设计收官、工程化落地之后（见 handoff §5）。
> 本文档目的：把讨论中已产生的框架、工具图谱、v1 子集与推迟项固化，**避免丢失**。
> 定稿时逐层细化：各层运行位置、门禁强度、触发条件。

---

## 1. 元规则（源自 1+N 开发模式，见 [ADR-0009](../decisions/0009-engineering-baseline.md)）

- agent 是主要开发者 → **每个门禁必须机器可判定**（验收不需要人类判断力介入）
- **可自动修复优先**：门禁报错 → `just fix` → 再过，构成 agent 自修复闭环
- **本地门禁是唯一测试门禁**（无远程测试 CI）；远程只跑固定/定期例程（tag/release、依赖检查、toolchain 更新）
- 门禁必须快——慢到被绕开的门禁等于没有

## 2. 测试分层框架（L0–L4）

| 层 | 内容                          | 工具                                                                     | 验证目标                                             |
| -- | ----------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------- |
| L0 | 单元 / 快照                   | cargo-nextest + doctest + insta（模板自带）                              | 回归网底                                             |
| L1 | property-based，纯核心        | proptest / bolero                                                        | keyed diff、epoch 反向闭包、配置合并分层等纯函数内核 |
| L2 | 基于模型的状态测试 + 故障注入 | proptest state-machine / 自研 harness                                    | fiber 生命周期对偶参考模型；F 类恢复测试（§3-F）     |
| L3 | fuzz                          | cargo-fuzz / bolero                                                      | 配置解析、loader 协调、Luau ABI 边界                 |
| L4 | 形式化                        | Lean4（hax/Aeneas 直译 vs model-first，试点定夺）；Kani 跳板；Verus 备选 | "失败即存在性风险"的小内核                           |

**L4 候选内核**（与 ADR 对应）：fiber 状态机不变量、disposer 栈 LIFO/幂等/异步汇合（[ADR-0003](../decisions/0003-module-require-discipline.md)/[0005](../decisions/0005-vm-topology.md)）、epoch 指纹失效正确性（[ADR-0001](../decisions/0001-inject-runtime-checks.md)）、keyed diff 协调、安装事务回滚（[ADR-0008](../decisions/0008-creator-mode-self-modification.md)）。

## 3. 检查/纠错工具图谱

### A. 静态检查（全部可自动修复 → agent 自修复闭环）

| 工具                                  | 抓什么                            | 跑在哪                          |
| ------------------------------------- | --------------------------------- | ------------------------------- |
| clippy（pedantic + 精选 restriction） | 惯用法错误、可疑逻辑              | pre-commit 快门禁               |
| rustfmt / dprint                      | 格式漂移                          | pre-commit（`--fix`）           |
| typos                                 | 拼写错误（代码+文档）             | pre-commit（`--write-changes`） |
| workflow lint                         | GitHub Actions workflow YAML 语法 | pre-commit                      |

### B. 依赖与供应链

| 工具           | 抓什么                                  | 跑在哪              |
| -------------- | --------------------------------------- | ------------------- |
| cargo-deny     | RUSTSEC 漏洞、许可证、重复版本、来源    | 本地门禁 + 远程定期 |
| cargo-machete  | 声明了但没用的依赖                      | 本地门禁            |
| cargo-outdated | 依赖落后报告                            | 远程定期            |
| cargo-vet 💤   | 依赖审计记录（1+N 下治理 agent 选依赖） | 推迟，见 §5         |
| cargo-geiger   | 依赖树 unsafe 占比                      | 信息性，可选        |

### C. 覆盖率与测试质量

| 工具           | 抓什么                                                                                       | 跑在哪                                                                                 |
| -------------- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| cargo-llvm-cov | 哪些路径没被测试踩到                                                                         | 本地按需；**诊断用，不设 % 门槛**（百分比门槛激励注水测试；核心 crate 地板线待大讨论） |
| cargo-mutants  | 变异测试：改坏代码看测试抓不抓得住——**检验测试本身有没有牙齿**；agent 写测试防"断言了个寂寞" | 核心 crate 定期/按需（贵）                                                             |

### D. 动态正确性（FFI + 并发，与本项目强相关）

| 工具               | 抓什么                                             | 跑在哪                            |
| ------------------ | -------------------------------------------------- | --------------------------------- |
| **Miri**           | unsafe/FFI 边界的 UB（越界、别名违规、未初始化读） | Luau C API 边界测试必过，本地按需 |
| ASan / LSan / TSan | 内存错误、泄漏、数据竞争（C API 交互面）           | nightly，按需                     |
| loom / shuttle 💤  | 并发原语全排列交错测试                             | 触发：第一个手写无锁/原子结构出现 |
| **Kani**           | 有界模型检查，Rust 上直接小范围穷尽验证            | L4 跳板候选，跟 fiber 状态机走    |

### E. 性能回归

| 工具         | 抓什么                                 | 跑在哪                     |
| ------------ | -------------------------------------- | -------------------------- |
| criterion 💤 | 热路径性能回归（事件派发、fiber 操作） | 触发：MVP 后出现公认热路径 |

### F. 故障注入 / 恢复测试（项目特有类别）

不是现成工具，是测试**类别**。场景方向：安装事务中途 kill、插件文件写一半损坏、配置文件并发改写、VM 创建失败……验证**回滚与干净重启承诺**（[ADR-0001](../decisions/0001-inject-runtime-checks.md)/[0008](../decisions/0008-creator-mode-self-modification.md)；论文核心担忧："a faulty self-modification can disable the very process needed to recover"）。

归入 L2 集成层，随安装事务实现自带。具体场景清单待大讨论展开。

### G. API/版本纪律

| 工具                                      | 抓什么     | 跑在哪                                        |
| ----------------------------------------- | ---------- | --------------------------------------------- |
| cargo-semver-checks / cargo-public-api 💤 | API 面漂移 | 推迟（私有项目价值低；核心 crate 稳定后可议） |

## 4. v1 子集（大讨论前的工作假设）

- **全量上**：A 全部 + cargo-deny + cargo-machete
- **核心 crate 专项**：Miri（FFI 边界）+ cargo-mutants（定期）+ llvm-cov（诊断）
- **随实现引入**：F 恢复测试（跟安装事务走）、Kani（跟 fiber 状态机走）
- **明确推迟**：vet、criterion、loom、geiger、semver-checks/public-api

## 5. 推迟登记（含触发条件，防丢）

| 项                 | 何时捡回                                                                              |
| ------------------ | ------------------------------------------------------------------------------------- |
| loom / shuttle     | 第一个手写无锁/原子结构出现时                                                         |
| criterion          | MVP 后出现公认热路径时                                                                |
| cargo-vet          | 依赖数量或 agent 自选依赖频率失控时                                                   |
| fuzz（L3）全面铺开 | 配置格式 / Luau ABI 稳定后                                                            |
| Lean4 试点         | 结构设计收官 + 工程化落地后；先 spike 比较 hax/Aeneas 直译 vs model-first             |
| 覆盖率地板线       | 测试策略大讨论定夺                                                                    |
| workflow lint      | 随 workflows 恢复回归（GitHub 迁移后续，见 §6 第 8 条）                               |
| 轻 PR 评审面       | 用户 review 方案讨论时（[ADR-0009](../decisions/0009-engineering-baseline.md) 遗留①） |

## 6. 大讨论待决清单

1. 各层运行位置与门禁强度（pre-commit / pre-push / 远程定期）
2. 覆盖率：地板线设不设、作用于哪些 crate
3. cargo-mutants 的运行范围与频率
4. Kani 与 Lean 的分工（跳板值不值，还是直接 Lean 试点）
5. Lean 试点内核确认（建议 fiber 状态机）+ 路线选择 spike
6. F 类恢复测试的具体场景清单（随安装事务设计展开）
7. 与用户 review 方案的联动（agent 产出的人类评审面如何与机器门禁互补）
8. 远程 CI 重议：开源后 GitHub public repo CI 免费，"本地门禁是唯一测试门禁"的前提（自托管算力成本）已消失——远程跑测试从浪费变为外部 PR 的免费门禁（[ADR-0020](../decisions/0020-open-source-hosting.md)）；与 workflows 恢复同议题
9. release 流水线与 crates.io 发布形态：crates.io opt-in 已随 [ADR-0020](../decisions/0020-open-source-hosting.md) 激活；`metis` crate 名被占（图分区库），二进制 crate 改名 / cargo-dist 分发的取舍；与第 8 条同批
