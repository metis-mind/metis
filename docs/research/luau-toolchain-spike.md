# Luau 工具链验证 spike（2026-10-10）

> 性质：point-in-time 实测报告——为 `../design/luau-abi.md` A10（工具形态与 ABI 版本纪律）的五个验证项跑的事实测试，作该章实测 SSOT 引用。原始测试文件与完整输出在 `scratch/a10-spike/`（gitignored 一次性工程），本报告收录全部结论性数据。
> 修正纪律：本报告结论为拍摄时点事实；后续工具链版本行为变化以 addendum 追加，不改写本节结论。

## 环境

- **Luau 0.740**（`luau-lang/luau` release `0.740`，= 项目 pinned 版本：mlua 0.12.2 vendored → luau0-src 0.22.0+luau740）；`luau-analyze` CLI 默认 `--solver=new`，V1/V3 另用 `--solver=old` 交叉验证。
- **luau-lsp 1.70.1**（`JohnnyMorganz/luau-lsp` 当时最新 stable）。
- 除说明外，测试文件均为 `--!strict`。

## V1：类型检查器区分 `integer` / `number`——**成立**

| 用例（`--!strict`） | luau-analyze 结果 |
| ------------------- | ----------------- |
| `local x: integer = 1` | TypeError: Expected this to be 'integer', but got 'number' |
| `f(v: integer)` 调 `f(42)` | TypeError（同上） |
| `f(v: integer)` 调 `f(42i)` | 通过 |
| `local n: number = 42i`（反向） | TypeError: Expected this to be 'number', but got 'integer' |

`--solver=old` 结果与 new 完全一致。对照组 `local n: number = 42` 通过（无输出）。运行时补充（`luau` REPL）：`type(42i)=="integer"`、`42i == 42` 为 `false`、integer/number 混合算术报错。

**结论**：`integer` 在 0.740 是合法且独立的类型词，与 `number` 双向不兼容；区分是静态 + 运行时双层的。→ §A6.2 登记的"若类型器不区分则靠 analyze 数据流兜底"退路撤销（§A10.4 D28.4）。

## V3：两槽关联 narrowing——**不成立**

公共测试形：`local res, err = f()`，`f` 签名 `(string?, { kind: string }?)`。

| 写法 | 之后 `local s: string = res` |
| ---- | ---------------------------- |
| `if err then return end` | **TypeError**（`res` 仍是 `string?`；old solver 同） |
| `assert(res)` | 通过 |
| `if res then ... end` | 通过 |

**结论**：类型器不理解"err 非 nil ⇒ res 非 nil"的跨槽相关性；作者侧必须 `assert(res)` 或 `if res then` 包一层。→ D15.1 assert 习语获得类型器层面的硬性依据（§A10.4 D28.4）。

## V4：`.luaurc` alias 解析一致性（luau-analyze vs luau-lsp）——**成立（有条件）**

测试：`.luaurc` 声明 `aliases: { "lib": "./lib/" }`，`main.luau` 经 `require("@lib/foo")` 引用并制造两处类型错误。

- 两工具报出同一批类型错误（措辞与列位置粒度不同），同以 `.luaurc` 为 alias SSOT。
- **分叉点**：alias 值写裸 `"lib/"`（无 `./` 前缀）时，luau-lsp 照常解析，luau-analyze 报 `Unknown require: unsupported path` 并连带报 unknown type。
- luau-analyze 0.740 无 `--base-luaurc` 旗标，靠文件层级自动发现 `.luaurc`；相对 require（`./lib/foo`）不依赖 `.luaurc` 两工具均可解析。

**结论**：一致，前提是 alias 值写规范的 `./`/`../` 前缀相对路径——配置写错时两工具行为分叉（严格者直接失败）。→ 脚手架按严格工具为准生成（§A10.5 D28.5）。

## V5：definition files 挂接——**部分成立**

- `luau-analyze` 0.740 CLI：**无任何 definitions 挂接旗标**（`--definitions` 报 Unrecognized option）。
- `luau-lsp analyze --platform=standard --definitions=ctx.d.luau main.luau`：定义生效，类型错误正常报出。CLI 支持 `--definitions=PATH`（裸 PATH 会注册为默认名 `@roblox`）与 `--definitions:@name=PATH` 具名挂接（可重复）；LSP 模式设置项 = `luau-lsp.types.definitionFiles`（**object** 形状：name → path 映射）。
- declare 语法实测：定义文件末尾不需要 `return`；允许 `export type`，且 export 出的类型**在用户代码里全局可见**（`local c: Ctx = ...` 直接受检）。

**结论**：需要 definitions 参与的命令行检查只能走 `luau-lsp analyze`；官方 CLI 无挂接手段。→ analyze 前端选型依据（§A10.2 D28.2）。

## V6：`plugin()` 糖的泛型推导——**泛型路线死，具体类型路线成立**

| declare 写法 | `plugin(function(ctx, config) ... end)` 参数推导 |
| ------------ | ------------------------------------------------ |
| 泛型 `declare plugin: <TCtx, TCfg>(fn: (TCtx, TCfg) -> ()) -> ...` | 参数推成 free type，被函数体用法结构化塑形——未声明的字段访问也静默通过，**等于不检查** |
| 具体类型（`export type Ctx/Config` + 非泛型 declare） | 精确推为 `Ctx`/`Config`，误用全部报出 |

**结论**：`plugin()` 糖的类型定义 = 每插件生成具体类型的非泛型 declare——D19.3 原定"per-plugin 生成"路线获实测支撑（§A10.4 D28.4）。

## 附：本报告验证项来源登记

V1 = §A6.2 登记（integer/number 区分）；V3 = §A5.5 登记（D12.1 两槽 narrowing）；V4/V5/V6 = §A7.5 登记（alias 一致性 / definitions 挂接 / `plugin()` 泛型推导）。WIT 递归 variant 支持度（§A6.6 登记）另见 [wasm-research](wasm-research.md) 补遗 4。
