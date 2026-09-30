# 0011. 执行模型：actor 语义 + tokio

- Status: accepted
- Date: 2026-09-30

## Context

插件内串行 = Cordis 单线程语义的按插件保留；模型血统是 Erlang（私有堆 + 消息传递 + actor 内串行），可扩展性已被四十年验证。嵌套 emit 无栈递归（邮箱模型结构免疫；Cordis 单线程同步 emit 恰有此风险）。性能（估算，非实测）：单跳 ~5–15 µs，VM 边界穿越是最大头且与线程实现无关；业务主延迟（LLM 调用 1–60 s）高 5–6 个数量级，派发永不成瓶颈。

被淘汰的选项：

- `Mutex<lua_State>`：重入即死锁（A 派发→B 处理→B 回调 A 时 A 的锁未释放），结构性缺陷
- 单线程核心（Node 模式）：重插件拖死全系统，与错误遏制哲学冲突，浪费多核
- 起步专用线程（每插件一 OS 线程）：百级规模可用（增量内存 ~3–10 MB），但与 host 双世界心智；留作池化之外的备选实现

## Decision

- 每插件实例 = 独立 `lua_State`（[ADR-0005](0005-vm-topology.md)）+ **邮箱** + **串行处理循环**（actor 语义）；同一时刻一个插件最多一个回调在执行，插件作者写无锁顺序代码；事件/服务调用/dispose/定时器 tick 全部经邮箱到达，串行性绝对
- **星型拓扑**：跨插件一切交互经 host 两跳路由，插件之间从不直接通信；事件表、活性检查、快照、epoch 判断全部留在 host——四种派发模式是"同一个循环的四个参数"
- 实现用 **tokio 任务**（与 host 单运行时，避免 std 线程 + tokio 双世界）；防堵 executor = **VM 级时间盒**（interrupt 回调查单次执行时长，硬截断；纪律：Rust native 函数不许无限阻塞）
- **爆炸半径三层保险**：VM 时间盒 / 调用超时（跨 actor 等待环不死锁、只超时，按监听器错误遏制）/ bounded 邮箱（背压收敛）
- 优化位登记：`block_in_place`、blocking-pool checkout（实测瓶颈出现再捡，语义不变）

## Consequences

actor 模型下跨 VM 必然消息往返，Cordis 的"同步/异步"区分整体消失；派发语义、payload 值类型等后续议题见 `../design/context-events.md`。
