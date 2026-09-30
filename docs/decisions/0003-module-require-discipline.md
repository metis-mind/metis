# 0003. 模块系统与 require 纪律

- Status: accepted
- Date: 2026-09-29

## Context

Luau VM 无模块自省（require 由宿主提供，无缓存/依赖边可查），依赖边只能第一天记录，事后无法 retrofit。全量重启的替代方案会训练插件生态假设"别人都在重启"，且边记录成本极低。跨插件组合走 inject / event / entry 树（Harness capability seam），不走 import——否则消费者直接捕获提供者函数引用，epoch 指纹机制失效，热替换退化为全量重启。

被淘汰的选项：变更时全量重启（粗暴，生态坏假设）；插件间自由 require（破坏 temporal composability）。

## Decision

- **require 包装器 + 依赖边记录从第一天做**：记录每条 `(importer, specifier)` 边
- 失效集 = 反向传递闭包；旧引用残留靠 fiber 级重启化解；回滚靠保留旧 bytecode chunk
- require 规则：

| 引用方向                       | 规则                              |
| ------------------------------ | --------------------------------- |
| 插件内部文件互引（相对路径）   | ✅ 自由                           |
| 插件 → 共享 `lib/`（纯代码库） | ✅ 允许，v1 就做路径解析 + 边记录 |
| 插件 → 另一个插件的文件        | ❌ 路径逃逸插件根即拒绝，响亮失败 |

## Consequences

模块缓存随 VM 生灭（见 [ADR-0005](0005-vm-topology.md)）；仅共享 `lib/` 变更需要反向闭包定位。`lib/` 的引用语法细节留待核心设计阶段（别名机制，如 `@lib/...`）。
