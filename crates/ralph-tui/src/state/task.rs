//! 任务统计切片: TuiState 的 task 域。
//!
//! 说明:
//! - 独立变化 + 独立测试: widget 只通过窄接口面对任务计数/活跃任务。
//! - 纯数据 + 显示格式化, 无外部依赖。

/// 单个任务的摘要信息(用于 TUI 展示)。
#[derive(Debug, Clone)]
pub struct TaskSummary {
    /// Task identifier (e.g., "task-1737372000-a1b2").
    pub id: String,
    /// Task title/description.
    pub title: String,
    /// Task status (e.g., "open", "closed", "blocked").
    pub status: String,
}

impl TaskSummary {
    /// Creates a new task summary.
    pub fn new(id: impl Into<String>, title: impl Into<String>, status: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: status.into(),
        }
    }
}

/// 任务统计(用于 TUI 展示)。
#[derive(Debug, Clone, Default)]
pub struct TaskCounts {
    /// Total number of tasks.
    pub total: usize,
    /// Number of open tasks.
    pub open: usize,
    /// Number of closed tasks.
    pub closed: usize,
    /// Number of ready (unblocked) tasks.
    pub ready: usize,
}

impl TaskCounts {
    /// Creates new task counts.
    pub fn new(total: usize, open: usize, closed: usize, ready: usize) -> Self {
        Self {
            total,
            open,
            closed,
            ready,
        }
    }
}

/// 任务域切片: 计数 + 活跃任务。
#[derive(Debug, Clone, Default)]
pub struct TaskSlice {
    pub task_counts: TaskCounts,
    pub active_task: Option<TaskSummary>,
}

impl TaskSlice {
    pub fn counts(&self) -> &TaskCounts {
        &self.task_counts
    }

    pub fn active(&self) -> Option<&TaskSummary> {
        self.active_task.as_ref()
    }

    pub fn set_counts(&mut self, counts: TaskCounts) {
        self.task_counts = counts;
    }

    pub fn set_active(&mut self, task: Option<TaskSummary>) {
        self.active_task = task;
    }

    pub fn has_open_tasks(&self) -> bool {
        self.task_counts.open > 0
    }

    /// 进度显示文本("x/y tasks" 或 "No tasks")。
    pub fn progress_display(&self) -> String {
        if self.task_counts.total == 0 {
            "No tasks".to_string()
        } else {
            format!(
                "{}/{} tasks",
                self.task_counts.closed, self.task_counts.total
            )
        }
    }
}
