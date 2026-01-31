use std::collections::{HashMap, HashSet};

// =============================================================================
// 并行 stdout 解析：统计“每个 instance 实际跑了多少个 job”
// =============================================================================

/// 从并行日志模式 stdout 中提取“实例→job_id 集合”。
///
/// 说明：
/// - 并行 runner 的日志行形如：`[writer#1:out:job=12] ...`
/// - 同一个 job 会输出多行，因此必须按 job_id 去重。
/// - 这里不依赖 `.ralph/events*.jsonl`，因为很多 hat 不一定会 publish 事件，但仍可能运行 job。
#[derive(Debug, Default, Clone)]
pub(in crate::scenarios) struct JobRunCounts {
    jobs_by_instance: HashMap<String, HashSet<u64>>,
}

impl JobRunCounts {
    pub(in crate::scenarios) fn from_stdout(stdout: &str) -> Self {
        let mut out = Self::default();

        for line in stdout.lines() {
            if let Some((instance_id, job_id)) = parse_parallel_job_line(line) {
                out.jobs_by_instance
                    .entry(instance_id)
                    .or_default()
                    .insert(job_id);
            }
        }

        out
    }

    pub(in crate::scenarios) fn runs_for_instance(&self, instance_id: &str) -> usize {
        self.jobs_by_instance
            .get(instance_id)
            .map(|s| s.len())
            .unwrap_or(0)
    }

    /// 按“hat 名称”聚合的运行次数（跨 instance 汇总）。
    ///
    /// 说明：
    /// - 并行 autoscale 可能产生动态实例（例如 `spec_writer#2`）。
    /// - 因此在断言“某个 hat 总共跑了多少次”时，应按 hat 名聚合，而不是只盯某个固定实例号。
    pub(in crate::scenarios) fn runs_for_hat(&self, hat_name: &str) -> usize {
        let prefix = format!("{hat_name}#");

        self.jobs_by_instance
            .iter()
            .filter(|(instance_id, _)| instance_id.starts_with(&prefix))
            .map(|(_, jobs)| jobs.len())
            .sum()
    }

    pub(in crate::scenarios) fn summary(&self) -> String {
        // 稳定排序：便于在失败时阅读（避免 HashMap 顺序抖动）
        let mut pairs = self
            .jobs_by_instance
            .iter()
            .map(|(k, v)| (k.as_str(), v.len()))
            .collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.0.cmp(b.0));

        pairs
            .into_iter()
            .map(|(k, n)| format!("{k}={n}"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(in crate::scenarios) fn hat_summary(&self) -> String {
        let mut by_hat: HashMap<String, usize> = HashMap::new();

        for (instance_id, jobs) in &self.jobs_by_instance {
            let hat = instance_id.split('#').next().unwrap_or(instance_id);
            *by_hat.entry(hat.to_string()).or_default() += jobs.len();
        }

        let mut pairs = by_hat.into_iter().collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));

        pairs
            .into_iter()
            .map(|(k, n)| format!("{k}={n}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(in crate::scenarios) fn parse_parallel_job_line(line: &str) -> Option<(String, u64)> {
    // 期望格式：
    // - [writer#1:out:job=12] ...
    // - [writer#1:err:job=12] ...
    let line = line.trim_start();
    if !line.starts_with('[') {
        return None;
    }

    let end = line.find(']')?;
    let inside = &line[1..end];

    let mut parts = inside.split(':');
    let instance_id = parts.next()?;
    let stream = parts.next()?;
    let job_part = parts.next()?;

    if stream != "out" && stream != "err" {
        return None;
    }

    let job_id = job_part.strip_prefix("job=")?.parse::<u64>().ok()?;
    Some((instance_id.to_string(), job_id))
}
