# 0014. 事件派发语义：四模式收敛与 fail-open 细则

- Status: accepted
- Date: 2026-10-02

## Context

actor 模型（[ADR-0011](0011-actor-execution-model.md)）下跨 VM 必然消息往返，Cordis 的"同步/异步"区分整体消失；Cordis 的五种派发方法（emit / parallel / serial / bail / waterfall）需要按新执行模型重新归约。派发语义的取舍空间由正交三轴张成：等不等结果 / 并发还是顺序 / 返回值语义。讨论全文见 `../design/context-events.md` §2、§7。

被淘汰的选项：

- **bail 保留为独立方法**：bail 的唯一特征是"同步 serial"，同步性消失后与 serial 语义逐字重合
- **waterfall 做真洋葱（post 阶段）**：Cordis 全部实际用例（config 解析、update veto）都是单向变换，post 阶段生态零用例（调研结论，静态非运行验证）；actor 模型下洋葱要求跨 VM 挂起/唤醒 continuation，形成分布式调用栈，与 VM 时间盒、超时语义正面冲突
- **serial/waterfall 遇错短路**（Cordis 做法）：坏插件成为链上拒绝服务点——认领/变换场景家族（命令分发、意图认领、路由偏好、兜底链）全体指向 fail-open 的优雅降级
- **监听器错误传播回派发方**：跨 actor 开回信通道而无实际价值——派发方拿到"某监听器崩了"无法据此做出更好决策；错误的归处是日志/tracing

## Decision

- **四种派发模式**：`emit`（fire-and-forget）/ `parallel`（allSettled，返回每项 `Result`）/ `serial`（按注册顺序逐个调，返回**终止值**即短路并带回）/ `waterfall`（顺序变换链：`Some(v)` 继续传递，`None` 否决——链中断，后续监听器与内建行为跳过）。host 一处实现，四模式 = 同一个派发循环的四个参数
- **错误语义统一为 fail-open**：emit 记日志；parallel 收集每项 Result；serial/waterfall 记日志后继续（serial 短路条件唯一 = 显式返回终止值；waterfall 抛错视为无变换、当前值原样下传）。顺序模式下唯一改变控制流的方式是显式返回值，错误永远只进日志不进控制流
- **fail-closed 硬闸不进插件事件总线**：依赖"插件活着且正确"的安全边界不是边界；硬闸归 core；总线不承担依赖插件活性的强制闸门
- **监听器细则**（§7）：listener = fiber effect 自动入账，卸载摘除；派发快照 + 逐项活性检查（竞态无锁化解，投递失败 = 正常跳过）；serial/waterfall 按注册顺序；单派发者维度保序（同一监听者收件顺序 = 派发入邮箱顺序），跨派发者不保序

## Consequences

- **选择器插件模式成立**（`../design/context-events.md` §2.1）：serial 的"注册顺序 + 首认领"只是 host 内置默认策略；可编程路由（打分 / LLM 路由 / 静态表）可由普通插件以"serial 认领入口 + 服务调用转交"组合实现——派发策略成为可替换的用户空间组件
- 监听器规模纪律：百级以内（≈毫秒级）；事件总线是控制面不是数据面，高频流式数据（逐 token）由核心自持或批量化（§0.5）
- waterfall 的 post 阶段留扩展位：将来可作为纯增量加入，不破只写变换的监听器
- 派发方 await 的形态（邮箱重入等）归 Luau ABI 设计；本 ADR 只定语义
- Cordis 新人认知负担再减一档：无 bail、无洋葱、无"同步派发"概念
