//! ANSI 颜色常量(展示概念的家)。
//!
//! 说明:
//! - 从 CLI `display::colors` 迁入, 供 job 执行器的可观测输出着色使用。
//! - CLI 侧的 `display::colors` 暂保留, 后续可改为从这里 re-export。

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
// ANSI bright black (常用作“灰色”前景色)
pub const GRAY: &str = "\x1b[90m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const CYAN: &str = "\x1b[36m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
