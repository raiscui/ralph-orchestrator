//! 声明式场景基础设施(候选6 试点)。
//!
//! 目标: 场景 = 数据(YAML), runner = 深模块。
//! 试点: setup(config/prompt) + 内置断言子集; 复杂场景暂由命令式承担。

pub mod scenario;

use std::path::PathBuf;

pub use scenario::{DeclarativeExpect, DeclarativeScenario, DeclarativeScenarioRunner, DeclarativeSetup};

/// 从编译期内嵌的 YAML 文本构建声明式场景 runner。
pub fn from_yaml(id: &str, yaml: &str) -> DeclarativeScenarioRunner {
    let spec: DeclarativeScenario = serde_yaml::from_str(yaml)
        .unwrap_or_else(|e| panic!("invalid declarative scenario {id}: {e}"));
    DeclarativeScenarioRunner::new(spec, PathBuf::from("."))
}
