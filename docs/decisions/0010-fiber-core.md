# 0010. Fiber 核心数据结构

- Status: accepted（4/5；纯 crate 切法见 `../design/fiber.md` §9 待定）
- Date: 2026-09-29

## Context

核心是 temporal composability（旧实例零残留）与可推理性（保守重启换取可验证性）。机制细节与取舍见 `../design/fiber.md` 各节。

## Decision

完整设计见 `../design/fiber.md`。要点：

- Fiber = 插件实例的生命周期容器（账本），**不是并发原语**；fiber ≠ task，异步清理由 JoinSet 汇合
- 状态机 **enum 带数据**：`Pending / Loading / Active / Unloading / Disposed / Failed(Report)`；Loading 失败先 drain 半成品账 → 正常卸载与失败善后复用同一 drain
- Fiber 集中拥有于 **SlotMap arena**，世代 FiberId 防 ABA
- 账本 `IndexMap<u64, Disposer>` 反向 drain = LIFO；`FnOnce` + 状态机单入口 → 幂等结构性保证；结算期拒绝新 effect
- Disposer **统一异步签名** `Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>`
- **子 fiber = 父的一笔 effect**：子的卸载函数记进父账，LIFO 自动保证父死先死子
- epoch 指纹 + 反向索引驱动消费者拓扑序重启（[ADR-0001](0001-inject-runtime-checks.md) 具体化）
- 错误三层遏制（pcall / 独立 VM / Result+JoinHandle→Failed）；唯一上抛通道 `fiber.await()`

## Consequences

依赖热替换 = 完整 unload→reload，不做就地修补：保守但可推理（面向 Kani/Lean 的性质）。fiber-core 纯 crate 切法待定（用户要求再讨论）。
