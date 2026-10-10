# 0007. 统一配置格式：YAML 1.2 受限子集

- Status: accepted（overlay 收缩为装配补丁层意向登记 = [config-format](../design/config-format.md) D36；机器写面 = overlay + 插件目录 `config.yml`（经审批事务，注释保留/格式稳定纪律归 C3）；正式 supersede 归任务 4（配置格式设计）冻结 ADR）
- Date: 2026-09-29

## Context

统一一种格式 → 必须服务系统内最深的结构（entry 树，子插件/tool 挂载是核心组合机制）→ TOML 深嵌套是结构性短板。JSON 无注释对审批 diff 是一票否决。YAML 的两个真实弱点（类型歧义、crate 生态）分别由 schema 校验和一次性选型兜底，不压过"人类与模型每天读写深树"的主场景。

被淘汰的选项：

- TOML（`[a.b.c.d]` 路径头 / `[[plugins]]` 套娃三层以上可读性崩坏；toml_edit 保注释往返的优势被"机器只写 agent 层"中和）
- JSON（无注释，审批不可读）
- KDL / RON / CUE 等（生态小、模型生疏）
- 双格式（违背统一原则）

## Decision

- **YAML 1.2 core schema，受限子集**：禁锚点/别名/自定义标签/多文档
- 扩展名统一 `.yml`
- 适用面：插件 manifest + 配置树（`metis.yml` 等价物）+ overlay 分层文件 + 模型配置补丁
- **机器序列化输出只落 agent overlay 层**；人类手写层永不被机器重写 → comment 保留问题架构性消解（Harness 分层遗产：bundle→profile→home→agent overlay）
- 所有进入 loader 的 YAML 必经 schema 校验，类型不符响亮失败（Norway 类问题由校验兜底）
- 日志/数据流用 JSONL，不进本决策
- parser crate 实现期选型（yaml-rust2 / saphyr 等候选；serde_yaml 已归档；受限子集降低 parser 要求）

## Consequences

配置的来源分层与作用范围两根轴的定论见 `../design/session-model.md` §3；机器写入约束（只能写 agent overlay 或 preset revision，永远碰不到人类层）随之确立。
