# 0043. 只读 blob 大载荷车道与双车道 payload

- Status: accepted（新设计；不改 [ADR-0011](0011-actor-execution-model.md) 星形拓扑与 [ADR-0005](0005-vm-topology.md) 实例隔离；[instance-model](../design/instance-model.md) D68/D73 的可变资源跨实例面继续关闭）
- Date: 2026-10-19

## Context

插件之间会走大量通信，且全部经宿主两跳路由（[ADR-0011](0011-actor-execution-model.md) 星形拓扑）。值类型跨隔离线性内存每次必有一次拷贝（[wasm-research](../research/wasm-research.md) 补遗 5 §5.4）；fanout 的成本 = 1 次源快照 + N 次目标写入。小消息高频时瓶颈在每消息固定开销与目标实例邮箱串行化，不在 memcpy；大载荷或高 fanout 时拷贝与内存带宽成为主瓶颈。

resource 句柄本身 0 拷贝；但 wasmtime 的 resource handle 绑定 Store 的资源表，跨实例不能直接传同一个 handle。S3 spike（[wasm-research](../research/wasm-research.md) 补遗 10）验证了可行机制：宿主持有底层不可变数据（`Arc`），在目标实例的 `ResourceTable` 里重物化一个新句柄；数据 0 拷贝。该机制只适合只读不可变对象——可变资源的所有权 / 副作用 / 生命周期无法安全复制。

WASI 0.3 `stream<T>` 能做流式转发；但中途错误需要再包 `stream<result<...>>`（或配一个独立 future），且 v1 的 `byte-reader` / `byte-writer` 已覆盖宿主↔guest 大流，跨实例 fanout 已由 blob 车道解决。`stream<T>` 采用面留扩展位；理由是 API 形状与双轨成本，不是能力缺失。

## Decision

- **双车道 payload**：事件与服务的新增签名统一用 `variant payload { value(list<u8>), blob(own<blob>) }`。`value` = TLV 编码的 `Value`（小 / 中数据）；`blob` = 不可变大对象（大载荷 / fanout）。config、mount 声明与 business error data 保持 TLV。
- **`blob` = 宿主持有的不可变字节对象**：`duplicate() -> own<blob>`（引用计数 +1，0 拷贝）、`len()`、`read(offset, max) -> list<u8>`（按块拷贝）；`from-bytes(list<u8>)` 供 guest 创建（一次写拷贝）。
- **跨实例只允许只读 blob**：宿主在目标实例资源表重物化句柄（S3 机制）；生产者卸载不影响其他持有者。`byte-reader` / `byte-writer` / `process-handle` / `mount-handle` 等可变 / 有副作用资源维持 per-instance。
- **大负载流**：`byte-reader` / `byte-writer`（per-instance）承接 fs `open-read` / `open-write` 与 http `download`；EOF = `Ok(none)`；close 幂等；writer close = 原子提交，Drop = best-effort（未 close 弃 tmp）。
- **fanout**：宿主对每个接收方 `duplicate` blob 句柄；接收方不读 = 0 内容拷贝，读多少拷多少。邮箱 / 在飞预算按消息条数计，blob 内存预算按租户计。
- **journal**：blob / stream 内容默认不录；v1 不新增 kind（元数据 / 引用如需记录，归审计层扩展位）。
- **拓扑不变**：不立共享内存、宿主内存别名或插件直连通道；星形拓扑、能力门、fiber 账本与 journal 拦截点全部保留。拷贝下限 = 源 guest 快照 1 次 + 每个接收方 1 次（`value` 车道）；blob 车道把 fanout 的内容拷贝降为 0（句柄重物化），`read` 时按块 1 次。
- **局部取代 [ADR-0041](0041-crypto-capability-withdrawal.md) §3 的 import 计数**：v1 metis 功能 import 由 7 个增补为 9 个（新增 `blobs` / `streams`）；ADR-0041 的 crypto 撤回结论不变，Status 行记本 ADR。

## Consequences

- [wit-abi](../design/wit-abi.md) W6 章（D155–D162）冻结完整形状；联动清单 = W6.11（W1 / W3 / W4 / W5 / W7 / W8 / W9 / W10、`context-events` / `plugin-sdk` / `instance-model` / `seam-capabilities` 等）。
- T10.10：每实例 `ResourceTable`、blob 注册表（`Arc` + 引用计数）、跨 Store 重物化、析构与预算；T10.11：fs / http 的 reader / writer 与 blob 创建；T10.13：TLV 不含 blob；T10.14：`Payload` / `Blob` / `ByteReader` / `ByteWriter` 作者面。
- 安全边界：跨实例只读 blob 不携带可变状态所有权；可变资源跨实例继续拒绝。blob 内容不透明、不跨租户、默认不进 journal。
- 成本账：blob 句柄 0；`from-bytes` 1 次写；fanout 0；每个接收方 `read` 按其读取量 1 次；reader / writer 每块 1 次。共享内存仍禁（[ADR-0024](0024-carrier-layering.md) C2）。
- 扩展位：WASI `stream<T>` 采用；blob 写入型 / 可变跨实例；listener / timer resource 化；per-read timeout；http 流式上传。
- 复审触发器：真实大载荷跨插件消费者出现且 blob 句柄重物化有性能 / 生命周期问题 → 重新评估 `stream<T>` 或共享内存数据面（后者须新 ADR）。
