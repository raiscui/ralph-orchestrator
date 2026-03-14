//! HatJob 与输出数据结构。

use crate::config::HatBackend;
use ralph_proto::{HatId, HatInstanceId, SessionStrategy};
use std::time::Duration;

/// stdout/stderr 标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// 执行输出的最小流式单位（按行）。
#[derive(Debug, Clone)]
pub struct HatJobOutputChunk {
    /// 归属的 job_id（用于 TUI 按 job 分段展示）。
    pub job_id: u64,
    pub instance_id: HatInstanceId,
    pub stream: OutputStream,
    pub line: String,
}

/// Job backend 选择。
#[derive(Debug, Clone)]
pub enum JobBackend {
    /// 使用全局 cli.backend。
    Default,
    /// 使用 hat 自己的 backend 配置。
    Hat(HatBackend),
}

/// HatJob 运行中控制消息（in-flight control）。
///
/// 说明：
/// - 该通道用于实现 "Steer": 在同一 job/turn 进行期间追加输入。
/// - 只有部分后端支持该能力（例如 Codex App Server 的 `turn/steer`）。
#[derive(Debug, Clone)]
pub enum HatJobControl {
    /// 追加一条用户输入到当前 in-flight turn。
    Steer { input: String },
}

/// 一次 headless CLI invocation 的描述。
#[derive(Debug, Clone)]
pub struct HatJob {
    /// 运行时 job id（用于日志归因/工作区目录命名等）。
    pub job_id: u64,
    /// 归属的实例 id（例如 writer#1）。
    pub instance_id: HatInstanceId,
    /// 归属的 hat 类型（例如 writer）。
    pub hat_id: HatId,
    /// 本次要执行的 prompt。
    pub prompt: String,
    /// 持续会话(app_server 等)在“首 turn 之后”可使用的增量输入。
    ///
    /// 说明:
    /// - 首 turn 仍然发送 `prompt`(完整上下文)。
    /// - 后续 turn 可只发送“新事件 + 极短续聊提示”，避免重复注入整段大 prompt。
    /// - 不支持持续会话的后端可忽略该字段,继续只使用 `prompt`。
    pub continuation_prompt: Option<String>,
    /// 后端选择规则。
    pub backend: JobBackend,
    /// 会话策略(一次性 exec vs 持续 mcp/app_server).
    ///
    /// 说明:
    /// - 该字段来自事件的 `session_strategy` 合并结果.
    /// - 方案1(只升级,不降级): instance 一旦升级,后续 job 将保持在同级或更强策略.
    pub session_strategy: SessionStrategy,
    /// “检测超时”的窗口（None 表示不启用检测）。
    ///
    /// 说明：
    /// - 不是硬超时：到时间不会立刻终止；
    /// - 会结合 `output_stale_timeout` 判断输出是否停滞：
    ///   - 若输出已停滞超过阈值：判定超时并终止
    ///   - 若输出仍在变化：判定通过，并把检测窗口重新计时
    pub timeout: Option<Duration>,
    /// 输出停滞阈值（None 表示不做“停滞判断”，将回退为硬超时行为）。
    pub output_stale_timeout: Option<Duration>,
    /// 工作目录（例如 worktree 根目录）；None 表示使用当前目录。
    pub workdir: Option<std::path::PathBuf>,
}

/// 一次 job 执行完成后的摘要。
#[derive(Debug, Clone)]
pub struct HatJobResult {
    /// 用于事件解析的输出(必须是 stdout-only 或已抽取的 assistant 文本)。
    ///
    /// 重要:
    /// - 该字段会被 `EventParser` 解析,并用于路由/收敛判断。
    /// - 绝不能把 stderr(例如 prompt transcript/后端日志/示例 `<event ...>` 文本)混入这里,
    ///   否则可能触发“假事件/假 completion/重复路由”等 flaky 回归。
    pub output_for_parsing: String,
    /// stderr 可观测输出(不参与事件解析)。
    ///
    /// 说明:
    /// - 该字段是 best-effort: 某些执行器可能只做“流式转发 + cassette 落盘”,并不在结果里累积。
    /// - 字段存在的主要目的,是把“可解析输出”和“可观测 stderr”在类型层面拆开,避免误用。
    pub observed_stderr: String,
    /// 是否成功（exit code == 0 且未超时/未取消）。
    pub success: bool,
    /// 退出码（可能为空，例如被信号终止）。
    pub exit_code: Option<i32>,
    /// 是否因超时而终止。
    pub timed_out: bool,
    /// 是否因取消而终止。
    pub canceled: bool,
}
