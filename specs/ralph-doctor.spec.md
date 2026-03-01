# Spec: `ralph doctor`(诊断常见启动失败,并提供安全修复路径)

## 背景

Ralph 的价值在于"能跑起来并且可收敛"。
但现实里最常见的失败,往往发生在真正进入 orchestration loop 之前,例如:

- 配置文件缺失/无效,导致启动直接失败。
- hats 拓扑有硬错误(没有 starting_event 订阅者、孤儿事件、dead-end),跑起来也无法闭环。
- 选择了某个 backend,但对应 CLI 根本不在 PATH。
- 工作区文件不可写(例如 `.agent/`、scratchpad、record-session 目录),导致运行时证据无法落盘。
- `config/all_hat.md` 等编译期内嵌的内容已更新,但当前可执行文件是旧构建,行为与源码不一致。

openclaw 的一个很强的工程启发是: 把这些失败做成 `doctor` 命令。
它的核心不是"打印更多日志",而是:

- 把问题分类为可诊断项。
- 给出可执行的修复建议。
- 对低风险修复提供 `--fix` 闭环。

本 spec 的目标是把这个模式迁移到 Ralph,并且优先落地一个"先能用"的最小版本。

---

## 目标(Goals)

1) 提供 `ralph doctor` 子命令,用于在运行前快速诊断常见问题。
2) 输出必须可执行:
   - 每个 error/warn 必须给出明确的下一步(命令或修改点)。
3) 提供 `--fix`:
   - 仅执行"低风险、可逆"的修复(例如创建目录/创建空文件)。
   - 不自动修改 `ralph.yml`(避免破坏用户意图)。
4) 允许在配置无效时仍能运行 doctor:
   - doctor 必须把"配置无效"本身当作诊断输出,而不是直接 panic 或吞掉上下文。

---

## 非目标(Non-Goals)

- 不做交互式 wizard(本轮先不引入复杂提示流程)。
- 不做网络更新/自升级。
- 不实现"高风险自动修复"(例如自动重写配置、自动删除文件)。
- 不把 doctor 做成替代 `ralph hats validate` 的唯一入口(doctor 会复用其能力)。

---

## 命令行接口(CLI)

### 基本用法

- `ralph doctor`
- `ralph doctor --fix`

### 参数

- `--fix`:
  - 对支持的修复项执行安全修复。
- `--strict`(可选,建议实现):
  - 将 warnings 视为 errors(用于 CI backpressure)。
- `--format <text|json>`:
  - `text`(默认): 维持现有可读风格输出(与 `ralph hats validate` 一致)。
  - `json`: 输出机器可读 JSON(用于 code agent/CI/TUI),避免解析 stdout 文本。
- `--json`:
  - `--format json` 的便捷别名。

说明:
- doctor 继承全局 `--config` 与 `--color` 选项。

---

## 输出格式与退出码

### 输出风格

- 必须使用与 `ralph hats validate` 一致的可读风格:
  - `  [ok] ...`
  - `  [warn] ...`
  - `  [err] ...`
- 每个 `[warn]`/`[err]` 后必须包含:
  - 问题是什么(一句话)
  - 怎么修(至少一个可执行命令或明确操作)

### JSON 输出(用于 code agent/CI)

当用户传入 `--format json` 或 `--json` 时:

- stdout MUST 只包含一个 JSON 对象(可 pretty),不得混入其它文本(避免污染解析)。
- JSON MUST 至少包含这些稳定字段(允许未来新增字段,但不可移除/改名):
  - `schema_version`: u32
  - `verdict`: `pass` | `fail_errors` | `fail_strict`
  - `counts`: `{ errors: number, warnings: number }`
  - `args`: `{ fix: bool, strict: bool, format: \"text\"|\"json\" }`
  - `checks`: 数组,每条至少包含:
    - `id`: 稳定 check_id(用于自动分类/分流)
    - `category`: 稳定类别(例如 config/hats/backend/workspace/context_window/events_marker/binary)
    - `status`: `ok` | `warn` | `err` | `skipped`
    - `message`: 与文本输出一致的可读信息(包含 Fix/Skipped/Fixed 等字样)
    - `fix`(可选): 从 `message` 中提取的 \"Fix:\" 建议(便于程序直接展示)

### 退出码

- 无 errors: exit code = 0
- 有 errors: exit code = 1
- `--strict` 下,只要有 warnings: exit code = 1

---

## 检查项(Checklist)

本节列出最小可用版本必须覆盖的检查。

### D1: 配置可加载性

doctor MUST 尝试加载 `--config` 指定的配置源(与 `ralph run` 的规则对齐: file/builtin/remote)。

- 若配置可加载:
  - 输出 `[ok] Config loaded: <source>`。
- 若配置不可加载:
  - 输出 `[err] Config invalid: <reason>`。
  - 并给出修复建议:
    - 默认 `ralph.yml` 缺失: 建议 `ralph init --list-presets` 或 `ralph init --preset <name>`。
    - YAML/字段错误: 建议定位行号(如果可用),并提示 `ralph hats validate` 作为二次验证。

### D2: hats 拓扑校验

doctor MUST 复用 `ralph hats validate` 的核心校验逻辑,并在 doctor 输出中呈现结果。

- 如果 D1 加载失败,则 D2 可以跳过,但必须明确输出 `[err] Skipped hat validation: config invalid`。

### D3: backend 可用性(可执行文件存在)

doctor MUST 检查最终会被使用的 backend 是否可用。

- 最小要求:
  - 当 config 指定了 backend(或能推导出默认 backend)时,检查对应命令是否在 PATH。
  - 不可用时输出 `[err] Backend not found: <cmd>` 并给出安装/替代建议。

说明:
- 本轮不要求 doctor 精确模拟所有 auto-detect 细节,但必须能在"选了不存在的 backend"时提前失败。

### D3.5: context window guard(可选,配置驱动的 warn/block)

doctor SHOULD 支持一个“上下文窗口护栏”检查,借鉴 openclaw 的思路:

- 上下文窗口是硬资源,不足就应该提前 warn/block,避免启动后才失败(浪费时间与 token)。
- 由于 Ralph 无法从各 CLI 后端稳定获取模型上下文窗,因此该检查采用“配置驱动”:
  - 由用户在 `adapters.<backend>.context_window_tokens` 显式声明窗口大小(tokens)。

行为:

- 若 `adapters.<backend>.context_window_tokens` 未配置:
  - 输出 `[ok] Skipped context window guard: ... not set`。
  - 并提示如何开启(配置示例)。

- 若配置了 window:
  - 当 window < 32k tokens 时输出 `[warn]`。
  - 当 window < 16k tokens 时输出 `[err]`。
  - doctor SHOULD 进一步尝试做一次“prompt-fit”粗估:
    - 用 chars/4 粗估 tokens。
    - 当估算 prompt tokens >= window tokens 时输出 `[err]`(基本必炸)。
    - 当估算 prompt tokens 接近 window(例如 >= 85%)时输出 `[warn]`。

修复建议(Fix)必须可执行:

- 缩短 `PROMPT.md`/inline prompt。
- 限制记忆注入体积(例如设置 `memories.budget`)。
- 切换到更大上下文窗的模型,并更新 `context_window_tokens`。

### D4: scratchpad/工作区可写性

doctor MUST 检查 `config.core.scratchpad` 的父目录是否存在且可写。

- 若目录不存在:
  - 输出 `[warn] Scratchpad dir missing: <dir>`。
  - 在 `--fix` 下,应创建目录并输出 `[ok] Fixed: created <dir>`。

- 若文件不存在:
  - 输出 `[warn] Scratchpad missing: <path>`。
  - 在 `--fix` 下,应创建空文件并输出 `[ok] Fixed: created <path>`。

### D5: 当前 run 的 events marker 健康度(如果存在)

doctor SHOULD 检查 `.ralph/current-events` marker(若存在)的可读性与指向文件的可写性。

- marker 存在且指向文件可写: `[ok] Active events file: <path>`。
- marker 存在但不可解析/不可写: `[warn]/[err]` 并给出修复建议(例如权限、路径问题)。

### D6: 编译期内嵌配置的新鲜度提示(all_hat overlay)

doctor SHOULD 检查 `config/all_hat.md` 的修改时间是否晚于当前可执行文件。

- 若 `config/all_hat.md` 更新更晚:
  - 输出 `[warn] Binary may be stale (all_hat.md newer than executable)`。
  - 并给出明确命令:
    - `cargo build`
    - 或 `cargo install --path .`(如果适用)

说明:
- 若文件缺失(例如非源码 checkout),该项可以 `[ok] Skipped` 或 `[warn] Skipped`,但必须说清楚原因。

---

## `--fix` 修复边界

doctor MUST 保证 `--fix` 的行为满足:

- 仅执行低风险修复:
  - 创建目录/创建空文件。
  - 不删除、不覆盖已有内容(除非明确是"创建时为空")。
- 不自动修改 `ralph.yml`。
- 每个修复动作必须输出一条可审计的 `[ok] Fixed: ...`。

---

## 测试要求(Backpressure)

为避免 `doctor` 变成不可维护脚本,实现必须包含最少回归测试:

- 配置加载失败时,doctor 不应 panic,应返回错误并包含可理解的提示文本。
- `--fix` 能创建缺失 scratchpad 目录与文件(且不覆盖已有内容)。
- `--strict` 会把 warnings 变成非 0 退出码(如果实现该选项)。
- context window guard:
  - 当 `context_window_tokens < 16k` 时 doctor 必须失败(退出码=1)。

---

## 后续扩展(不在本轮落地)

- 交互式确认(`--yes`/`--non-interactive`)与更激进修复(`--force`)。
- 更完善的 backend 健康检查(版本探测、子命令探测、权限探测)。
