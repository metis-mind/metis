# 0004. 插件形态：两形态 + 插件名解析

- Status: accepted（纯代码插件"inject 边加载时运行期发现"一格由 [ADR-0018](0018-service-registry.md) 局部取代；两形态合并为目录单形态意向登记 = [config-format](../design/config-format.md) D35，正式 supersede 归任务 4（配置格式设计）冻结 ADR）
- Date: 2026-09-29

## Context

core 极薄、连 tool 都是插件的系统，必然同时存在大量 20 行小插件（要零仪式）与复杂插件（要拆模块）。与 Python `foo.py` vs `foo/__init__.py` 同款成熟模式：名字不变，长大了搬进目录。

被淘汰的选项：

- sidecar 形态 `foo.luau + foo.yml`（两文件平铺 = 多余命名约定；需要 manifest 即已复杂化，应升文件夹）
- 一律文件夹 + manifest（小插件钝税；Creator 模式产出的简单 tool 最爱单文件）
- manifest 内嵌 front-matter（用户选定独立文件；目录形态下可读性更好，单文件零仪式靠"无 manifest"而非"内嵌"达成）

## Decision

**三层概念分离**：

| 单元    | 职责                                                    | 变更影响                  |
| ------- | ------------------------------------------------------- | ------------------------- |
| module  | 代码组织：一个 `.luau` 文件                             | require 解析、字节码缓存  |
| plugin  | 生命周期：一个 fiber、一个 config entry、热更新最小单位 | 失效/回滚                 |
| package | 分发与版本                                              | v1 不实现，目录形态即雏形 |

**两种插件形态**：

| 形态       | 路径               | 锚点           | 能力边界                                                                                              |
| ---------- | ------------------ | -------------- | ----------------------------------------------------------------------------------------------------- |
| 纯代码插件 | `plugins/foo.luau` | 文件自身       | 无 manifest → 无 config schema（配置树给它配值 → 响亮失败）、身份 = 文件名、inject 边加载时运行期发现 |
| 包插件     | `plugins/foo/`     | `manifest.yml` | 完整能力：schema、`entry:` 指定入口（缺省 `main.luau`）、子模块、version 预留                         |

**统一插件名解析**（配置树/人类/模型永远只引用无扩展名的插件名 `foo`）：

1. `plugins/foo/manifest.yml` 存在 → 包形态
2. `plugins/foo.luau` 存在 → 纯代码形态
3. 两者都不存在 → 响亮失败
4. 两者都存在 → 歧义，响亮失败

## Consequences

成长路径 `foo.luau` → `foo/` 无需改配置树引用；package 分发机制等分发时代到来再长回。
