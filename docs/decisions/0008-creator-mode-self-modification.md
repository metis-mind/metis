# 0008. 自我修改路径：Creator 模式

- Status: accepted（💤 方向已定，实施推迟——当前优先自由组合与动态更新主路径）
- Date: 2026-09-29

## Context

Harness 实证教训：早期允许模型用 `node:vm` 直接 eval 临时插件，造成"第二条插件生命周期"，已退役。自我进化的安全性论证见论文："a faulty self-modification can disable the very process needed to recover"。

## Decision

照 Harness 已验证形态：

```
模型产出 插件文件 + 配置补丁
  → 暂存 → 校验（Luau 编译 + schema）
  → 审批门（人类优先，接口可插拔）
  → 原子提交 + 备份 → loader 协调生效，失败回滚
```

- **永远不给模型运行时 eval 定义插件的能力**
- 模型只有提议权，批准权在更高层
- 审批门 v1 不实现，接口预留（候选：CLI 子命令 / 文件落盘 / HTTP API）

## Consequences

模型可见的配置写入面只剩 agent overlay 层（[ADR-0007](0007-yaml-config-subset.md)）；审批 diff 保持数据级可审依赖 [ADR-0002](0002-config-as-pure-data.md)。
