# 0035. Journal `fanout_cancelled` 族 supersede（wasm 载体吸收）

- Status: accepted
- Date: 2026-10-18

## Context

[ADR-0029](0029-journal.md) D78 冻结的 v1 kind 表含 `fanout_cancelled`（handler 结束强制取消子协程：handler 指路 + 取消计数），来源 = 任务 6 J1 对旧 ABI 的登记。

wasm-first 转向后，fan-out 不再是宿主 ABI 原语：等的事重叠 = guest 异步设施原生（`join!`/`select!` 一族），宿主不提供 `fanout` 原语，也没有观测点。[wit-abi](../design/wit-abi.md) W4.2 D94 定案「宿主零同步原语」，W4.6 明细表将该 kind 登记为「随下一文档批同批 supersede 小条」（2026-10-17 拍板）。T13-f 兜底批执行该处置。

## Decision

supersede [ADR-0029](0029-journal.md) D78 kind 表的 `fanout_cancelled` 格：该 kind 不立，无替代条目。

journal v1 写死族骨架的其余部分不变（生命周期 + 错误现场其余项 + 控制 + `process_*`）。格式版本不 bump：该 kind 从未实现、从未产出条目，删除对既有读侧没有可观察影响。

## Consequences

- [journal](../design/journal.md) D78 表移除该行，并在「载体吸收」记录中保留一句理由与去向。
- 卸载期可见性继续由 `unload_gate_closed` / `unload_killed` 两族覆盖，不因本 supersede 出现观测缺口。
- 重开条件：若未来宿主重新引入可观测的 fan-out 原语（当前无此计划），按 ADR-0029 演进纪律新增 kind。
