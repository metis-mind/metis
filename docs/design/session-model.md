# Metis — Session 模型与配置作用范围（种子稿）

> 状态：🌱 种子稿，未定稿。由 2026-09-30 Context/事件系统讨论派生（用户提问：多个 session 在跑时，只为自己环境做的修改会影响其他 session，怎么办？调研里是否有说明？）。
> 正式设计排在任务 5（配置格式）之后。本文档固化讨论结论与 Harness 调研答案，避免丢失。

---

## 1. 问题

配置树（entry 树）是 server 全局唯一的声明式拓扑。多 session 并发时，某 session 为自身环境做的修改若落到全局树，会波及所有 session。

## 2. Harness 的三层答案（调研第四部分，已确认覆盖此问题）

1. **全局修改故意全局**：持久化挂载/卸载写 profile 的 `cordis.patch.yml`，"影响该 profile 所有 session"——所以走审批事务（即 [ADR-0008](../decisions/0008-creator-mode-self-modification.md) 照抄的路径）。全局影响是这层的语义，不是 bug。
2. **preset = per-session 组合**：一条普通插件行，`config.plugins` 是与 profile 语法完全相同的子插件 YAML；eager 激活 + **revision 引用计数**——声明更新 retire 旧 revision，运行中 Agent 保留旧树直到最后引用释放（"编辑影响之后创建的 Agent，不抢夺运行中 Agent 的工具"）。
3. **isolate/intercept**（Cordis 层）：同 service key 在不同子树解析到不同实现 / 给某服务名下所有后代合并配置——"拓扑不变、实现按子树换"的细粒度答案，论文 spatial composability 的落地机制。

## 3. 配置的两根正交轴（讨论定论）

"全局/profile"和"server/session"是两根轴，Harness 术语容易把它们混在一起。

### 轴 1：来源分层（谁写的 → 合并顺序 + 谁有权重写）

Harness 四层：bundle（插件作者出厂默认）→ profile（部署者选的组合）→ home（本机用户覆盖）→ CLI overlay（本次启动覆盖）。**四层全是 server 范围**。

metis v1：只两层（[ADR-0007](../decisions/0007-yaml-config-subset.md) 已定）——人类手写层（机器永不重写）+ agent overlay 层（机器只写这层，Creator 事务落盘点）。profile 的 v1 等价物 = 启动 `--config` 选人类层文件，零新机制；bundle/profile 全套是包分发时代的 machinery，等分发来了再长回（[ADR-0004](../decisions/0004-plugin-forms.md)）。

### 轴 2：作用范围（影响谁 → 爆炸半径）

| 范围                        | 载体                                         | 语义                                    |
| --------------------------- | -------------------------------------------- | --------------------------------------- |
| 整个 server（所有 session） | 全局 entry 树                                | 改动 = 审批事务，影响全部，故意如此     |
| 之后的 session              | preset（session 级子树 + revision 引用计数） | future-only，不抢运行中 session         |
| 仅此会话                    | session 运行时状态                           | 不是配置，是 session 服务持有的普通数据 |

### 判别两问

1. **它变了，别的 session 该跟着变吗？** 该 → 全局树；不该 → session 范围
2. **它是拓扑/静态设置，还是会话动态偏好？** 动态偏好 → runtime state，根本不进 YAML（loader 协调为"管理员级变更"设计，不承载每会话高频变动）

机器（模型）写入约束：只能写 agent overlay（全局，走审批）或 preset revision（session 级）；永远碰不到人类层。

## 4. 与既有决策的汇合点

- preset 的 session 子树挂载 = **编程式 spawn（D7）+ 子树作用域（D2/isolate）的交集** → 本议题正式设计时，D2/D7 一起回来
- 机制地基已备：子 fiber = 父的 effect（[ADR-0010](../decisions/0010-fiber-core.md)）；Context parent 链预留（[ADR-0013](../decisions/0013-context-scope.md)）；inject 驱动的加载排序（[ADR-0001](../decisions/0001-inject-runtime-checks.md)）
- preset 复用 entry 树 YAML 语法与 loader 机制 → 必须在配置格式（任务 5）定稿后设计

## 5. v1 范围建议与待决清单

v1 最小集：全局两层 + session 纯运行时状态（session 服务插件持有）；preset 留设计位。

正式设计时展开：

1. session 的实体形态（session 服务插件？entry 树里的位置？多 session 的标识与隔离边界）
2. preset 声明/修订的存储位置与审批关系（是否也走 Creator 事务）
3. revision 引用计数的 Rust 表达（回收时机、与 fiber arena 的关系）
4. isolate/intercept 是否与 preset 同期引入
5. "运行中 session 持有旧树"在 actor 模型下的语义（旧 revision 的 fiber 树如何与新声明共存）
