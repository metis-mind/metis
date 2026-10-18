# 0037. 组件实例成本账（局部取代 ADR-0005 Consequences）

- Status: accepted（局部取代 [ADR-0005](0005-vm-topology.md) Consequences 的成本估算；ADR-0005 Decision 本体不变）
- Date: 2026-10-18

## Context

[ADR-0005](0005-vm-topology.md) 的决策是每插件实例一个独立运行时实例，理由 = 卸载零残留、模块缓存随实例生灭、故障遏制、代码全局共享。当时载体是 Luau，Consequences 的成本账写着「每实例经验估算几十 ~ 一百 KB……百级插件约 10–30 MB」。这个账的量纲和数值都绑定在解释器载体上。

wasm-first 转向（[ADR-0034](0034-wasm-first-carrier.md)）更换载体后，实例 = 组件（Store + Instance），代码 = 编译后的组件模块（可预编译为 `.cwasm`），内存 = 线性内存 + wasmtime 的槽预留与 guard。ADR-0005 的其余 Consequences（事件 payload 必须可序列化；跨插件数据共享不可能）与 Decision 本体继续有效，只有成本账过期。

实测基线（[wasm-research](../research/wasm-research.md) 补遗 1，wasmtime 49.0.2；数值汇总 = [ADR-0024](0024-carrier-layering.md) §8）：pooling 实例化 1.7 µs；限额超限拦截全生效、稳态 0.999×；100 槽 × 64 MiB = +6.48 GiB VA、零 RSS，实触 100 MiB → +100.6 MiB RSS，drop 即 madvise 归还；fuel 1.022×、epoch 1.000×。

## Decision

局部取代 ADR-0005 Consequences 的成本估算条目（「每实例经验估算几十 ~ 一百 KB……长驻 server 可接受」），替代为组件实例成本账。ADR-0005 Decision 的载体词（`lua_State`）已由 [ADR-0034](0034-wasm-first-carrier.md) §1 替换为组件实例；本 ADR 只替换成本账，拓扑决策（每插件独立运行时实例）不变。

- **虚拟地址（VA）** = 槽数 ×（每槽预留 + guard）；pooling 的预留是 VA 承诺，不是 RSS 承诺。
- **实际内存（RSS）** ≤ 槽数 × 每槽上限；实际占用随实例真实触达增长，实例丢弃即归还（madvise）。
- **实例化成本** = µs 级（pooling 1.7 µs）；限额检查稳态零开销；fuel/epoch 计量约 1.0×。
- **代码共享** = 编译产物（模块 / `.cwasm`）全局共享，不随实例复制；与 ADR-0005 Context 的「字节码全局共享」同构，载体词换成组件编译产物。

槽数、每槽预留与上限 = 实现期配置，归 T10.10（组件运行时）；槽数 = 硬上限，耗尽行为（激活失败或排队）归 T10.10。本 ADR 固化账式，不预设数值。

## Consequences

- [ADR-0005](0005-vm-topology.md) Status 行记录 partial supersede；正文冻结。
- 容量规划公式进 T10.10 的配置面。
- 复审触发器：pooling 配置形态变化 / 组件模型 1.0 / wasmtime 内存模型变化。
