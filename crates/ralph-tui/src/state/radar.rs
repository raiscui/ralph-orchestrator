//! Hat Graph Radar 切片: TuiState 的 radar 域。
//!
//! 说明:
//! - Radar 可视化状态机(结构匹配 + 因果边动画 + 扫描头)独立成片。
//! - 不依赖壳: "哪些 hat 在 Running" 由壳注入(running_hats 参数)。
//! - `mermaid_hat_node_id` 的唯一真相源在这里(原 state.rs 与 app.rs 各有一份)。

use ralph_proto::{Event, HatId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HatGraphRadarPoint {
    pub x: u16,
    pub y: u16,
}

/// Hat Graph Radar 的矩形区域（以终端 cell 为单位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HatGraphRadarRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Hat Graph Radar 的节点 meta：用于把“某个 hat 的 box”映射到字符画坐标。
#[derive(Debug, Clone)]
pub struct HatGraphRadarNodeMeta {
    /// Mermaid node id（parser identity），例如 `Hat_planner`。
    pub id: String,
    /// 节点展示 label（可能包含 emoji/中文）。
    pub label: String,
    /// 节点 box 的矩形范围（含边框）。
    pub box_rect: HatGraphRadarRect,
}

/// Hat Graph Radar 的边 meta：用于按最新 event 做“逐段点亮”动画。
#[derive(Debug, Clone)]
pub struct HatGraphRadarEdgeMeta {
    pub from: String,
    pub to: String,
    pub label: String,
    /// 有序 path 坐标序列（包含拐点/箭头/box-start marker 等关键格子）。
    pub path: Vec<HatGraphRadarPoint>,
}

/// Hat Graph Radar 的完整 meta（nodes + edges）。
#[derive(Debug, Clone, Default)]
pub struct HatGraphRadarMeta {
    pub nodes: Vec<HatGraphRadarNodeMeta>,
    pub edges: Vec<HatGraphRadarEdgeMeta>,
}

impl HatGraphRadarMeta {
    pub fn find_node(&self, id: &str) -> Option<&HatGraphRadarNodeMeta> {
        self.nodes.iter().find(|n| n.id == id)
    }

    fn edge_label_matches(edge_label: &str, topic: &str) -> bool {
        // physical view 里，CLI 可能会把同一对节点之间的多条边折叠成一条：
        // - label 形如：`integration.applied / integration.blocked / integration.rejected`
        // Radar 做因果边动画时需要把“单个 topic”匹配到这类“多 topic label”上。
        if edge_label == topic {
            return true;
        }

        edge_label
            .split(" / ")
            .any(|candidate| candidate.trim() == topic)
    }

    pub fn matching_edges(
        &self,
        from: &str,
        label: &str,
    ) -> impl Iterator<Item = &HatGraphRadarEdgeMeta> {
        self.edges
            .iter()
            .filter(move |e| e.from == from && Self::edge_label_matches(&e.label, label))
    }

    pub fn matching_edges_exact(
        &self,
        from: &str,
        label: &str,
        to: &str,
    ) -> impl Iterator<Item = &HatGraphRadarEdgeMeta> {
        self.edges.iter().filter(move |e| {
            e.from == from && e.to == to && Self::edge_label_matches(&e.label, label)
        })
    }
}

// =============================================================================
// Hat Graph Radar：事件线动画（按 Running 目标驱动）
// =============================================================================
//
// 你最新口径（2026-02-03）：
// - 线路需要先做 progressive reveal（从 source → target 逐段点亮）；
// - reveal 完成后，线路应保持“全亮”并持续显示，直到目标 hat 退出 Running（进入 Idle/Done/Failed）；
// - “指向的目标 box 不再 Running”时，必须立刻取消该线路高亮（不要残留）。
//
// 设计取舍：
// - cause event 采用 best-effort 推断：从“最近收到的业务事件”里找一条能够在 hats graph
//   中连到该 target hat 的边（from+topic+to 完全匹配）。
// - 动画本身是纯 UI 行为，不影响 orchestration。

/// Hat Graph Radar 边动画速度：每多少毫秒“点亮一个 cell”。
pub(crate) const HAT_GRAPH_EDGE_ANIMATION_STEP_MS: u64 = 30;
/// progressive reveal 的最大时长：用于把“很长的路径”加速到一个可读的时间窗口内。
const HAT_GRAPH_EDGE_ANIMATION_MAX_REVEAL_MS: u64 = 800;

/// Hat Graph Radar：扫描头（跑动高亮段）的移动速度（每多少毫秒前进一个 cell）。
///
/// 说明：
/// - 这是 reveal 完成后的“锦上添花”动效，目的是让用户一眼看出“这条边仍在生效/仍在运行态”；
/// - 速度不应跟随 reveal 的 step_ms（reveal 会为长路径自动加速，否则扫描会快到看不见）。
pub(crate) const HAT_GRAPH_EDGE_HEAD_STEP_MS: u64 = 60;

/// Hat Graph Radar：扫描头的长度（以 cell 数计）。
pub(crate) const HAT_GRAPH_EDGE_HEAD_LEN: usize = 16;

/// 推断“cause event”的回看窗口：只在这个时间范围内找最近事件（避免匹配到过旧的事件）。
const HAT_GRAPH_CAUSE_LOOKBACK: Duration = Duration::from_secs(10);

/// 保存最近事件的上限（按条数），避免无限增长。
const HAT_GRAPH_RECENT_EVENT_MAX: usize = 64;

/// Radar 侧用于推断 “cause event” 的最近事件记录（只存必要信息）。
#[derive(Debug, Clone)]
pub struct HatGraphRadarRecentEvent {
    pub source_hat: HatId,
    pub topic: String,
    pub observed_at: Instant,
}

/// 某个 target hat 当前正在播放的“cause event 边动画”。
#[derive(Debug, Clone)]
pub struct HatGraphRadarEdgeAnimation {
    pub target_hat: HatId,
    pub source_hat: HatId,
    pub topic: String,
    pub started_at: Instant,
    pub step_ms: u64,
}

/// Radar 边动画在“当前帧”应如何渲染（纯渲染计划，可单测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HatGraphRadarEdgeRenderPlan {
    /// 用 base 色从 path[0..base_steps] 做高亮（reveal 阶段为部分，reveal 后为全量）。
    pub base_steps: usize,
    /// 扫描头的起点（沿 path 的索引）。
    pub head_start: Option<usize>,
    /// 扫描头的长度（以 cell 数计）。
    pub head_len: usize,
    /// 扫描头是否允许环绕（reveal 完成后为 true）。
    pub head_wrap: bool,
}

/// 计算 Radar 边动画的渲染计划。
///
/// 规则：
/// - reveal 阶段：base 只亮到当前进度；head 贴着 reveal 前沿（更亮、更醒目）
/// - reveal 完成后：base 全亮；head 以固定速度循环移动（直到目标 hat 退出 Running 才会被上层清理）
pub(crate) fn plan_hat_graph_radar_edge_animation(
    elapsed: Duration,
    path_len: usize,
    reveal_step_ms: u64,
    head_step_ms: u64,
    head_len: usize,
) -> HatGraphRadarEdgeRenderPlan {
    if path_len == 0 {
        return HatGraphRadarEdgeRenderPlan {
            base_steps: 0,
            head_start: None,
            head_len: 0,
            head_wrap: false,
        };
    }

    let elapsed_ms = elapsed.as_millis();
    let reveal_step_ms = reveal_step_ms.max(1);
    let head_step_ms = head_step_ms.max(1);

    let total_steps = (elapsed_ms / u128::from(reveal_step_ms)) as usize;
    let revealed = total_steps.min(path_len);

    // reveal 阶段：head 贴着前沿，不环绕
    if revealed < path_len {
        if revealed == 0 {
            return HatGraphRadarEdgeRenderPlan {
                base_steps: 0,
                head_start: None,
                head_len: 0,
                head_wrap: false,
            };
        }

        let head_len = head_len.min(revealed);
        let head_start = revealed.saturating_sub(head_len);
        return HatGraphRadarEdgeRenderPlan {
            base_steps: revealed,
            head_start: Some(head_start),
            head_len,
            head_wrap: false,
        };
    }

    // reveal 完成：base 全亮；head 循环扫描
    let reveal_total_ms =
        u128::from(reveal_step_ms).saturating_mul(u128::try_from(path_len).unwrap_or(u128::MAX));
    let after_reveal_ms = elapsed_ms.saturating_sub(reveal_total_ms);
    let head_ticks = (after_reveal_ms / u128::from(head_step_ms)) as usize;
    let head_start = head_ticks % path_len;
    let head_len = head_len.min(path_len);

    HatGraphRadarEdgeRenderPlan {
        base_steps: path_len,
        head_start: Some(head_start),
        head_len,
        head_wrap: true,
    }
}

fn sanitize_mermaid_identifier(raw: &str) -> String {
    // 说明：
    // - Radar 的 meta 里边/节点引用的是 Mermaid “节点 ID”（例如 Hat_builder）；
    // - 该规则必须与 `ralph-cli` / `ralph-tui::app.rs` 的生成逻辑一致，否则匹配不到边。
    //
    // 规则：保守地只允许 ASCII [A-Za-z0-9_]，其余字符全部移除。
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        }
    }

    if out.is_empty() {
        "hat".to_string()
    } else {
        out
    }
}

fn mermaid_hat_node_id(hat_id: &str) -> String {
    // 与 `crates/ralph-cli/src/hats.rs#mermaid_hat_node_id` / `crates/ralph-tui/src/app.rs` 保持一致：
    // - 加前缀避免与 Start/Complete 等节点名冲突；
    // - 避免 hat_id 以数字开头触发 Mermaid 标识符解析歧义。
    format!("Hat_{}", sanitize_mermaid_identifier(hat_id))
}

#[derive(Debug, Clone)]
pub struct HatGraphRadar {
    /// 小窗（雷达）展示：更紧凑的 ASCII 图（通常 padding=0）。
    pub ascii_compact: String,
    /// 大窗（放大）展示：更可读的 ASCII 图（通常默认 padding）。
    pub ascii_full: String,
    /// compact 视图的 meta（可选：渲染器不支持/注入失败时允许降级为无高亮/无动画）。
    pub meta_compact: Option<HatGraphRadarMeta>,
    /// full 视图的 meta（可选：同上）。
    pub meta_full: Option<HatGraphRadarMeta>,
}

// ============================================================================
// RadarSlice: 切片状态与行为
// ============================================================================

/// Radar 域切片。
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_field_names)] // hat_graph_ 前缀与 TuiState 历史命名一致, 语义清晰优先
pub struct RadarSlice {
    pub hat_graph_radar: Option<HatGraphRadar>,
    pub hat_graph_zoomed: bool,
    pub hat_graph_recent_events: VecDeque<HatGraphRadarRecentEvent>,
    pub hat_graph_edge_animations: HashMap<HatId, HatGraphRadarEdgeAnimation>,
}

impl RadarSlice {
    /// 替换整个 radar 结构(由 CLI 生成 ASCII 图后注入)。
    pub fn set_radar(&mut self, radar: HatGraphRadar) {
        self.hat_graph_radar = Some(radar);
    }
    pub(crate) fn record_event(&mut self, event: &Event, now: Instant) {
        // 说明：
        // - 你明确指出 event 线路动画应是“因果可视化”，不是 gate/human 这类控制面噪音；
        // - 因此这里只记录“业务事件”，并且必须能推导出发布者 hat（source/source_instance）。
        let topic = event.topic.as_str();
        if topic.starts_with("gate.") || topic == "human.message" || topic == "reply.human.message"
        {
            return;
        }

        let source_hat = if let Some(source_hat) = event.source.clone() {
            source_hat
        } else if let Some(source_instance) = event.source_instance.as_ref()
            && let Some(hat_id) = source_instance.split_hat_id()
        {
            HatId::new(hat_id)
        } else {
            return;
        };

        self.hat_graph_recent_events
            .push_back(HatGraphRadarRecentEvent {
                source_hat,
                topic: topic.to_string(),
                observed_at: now,
            });

        // 容量上限：按条数裁剪（保证常数级内存）。
        while self.hat_graph_recent_events.len() > HAT_GRAPH_RECENT_EVENT_MAX {
            let _ = self.hat_graph_recent_events.pop_front();
        }
    }

    pub(crate) fn maybe_start_edge_animation(
        &mut self,
        target_hat: HatId,
        now: Instant,
    ) {
        // 说明：
        // - 只有 Radar + meta 存在时，才有条件做“因果边动画”；
        // - 这里使用 meta 做“结构匹配”，避免靠字符串/ANSI 解析导致脆弱。
        let Some(radar) = self.hat_graph_radar.as_ref() else {
            return;
        };
        let Some(meta) = radar.meta_full.as_ref().or(radar.meta_compact.as_ref()) else {
            return;
        };

        // 目标节点：Hat_{id}
        let target_node_id = mermaid_hat_node_id(target_hat.as_str());

        // 从最近事件里倒序找：谁能在图上连到 target（from+topic+to 完全匹配）。
        let mut cause: Option<(HatId, String)> = None;
        for e in self.hat_graph_recent_events.iter().rev() {
            if now.saturating_duration_since(e.observed_at) > HAT_GRAPH_CAUSE_LOOKBACK {
                break;
            }

            let from_node_id = mermaid_hat_node_id(e.source_hat.as_str());
            let topic = e.topic.as_str();
            let matches = meta
                .matching_edges_exact(&from_node_id, topic, &target_node_id)
                .next()
                .is_some();
            if matches {
                cause = Some((e.source_hat.clone(), e.topic.clone()));
                break;
            }
        }

        let Some((source_hat, topic)) = cause else {
            return;
        };

        // 计算 step_ms：
        // - 默认 `HAT_GRAPH_EDGE_ANIMATION_STEP_MS`（30ms / cell）
        // - 如果路径很长，则加速（缩小 step_ms），让 reveal 在一个合理窗口内完成
        let from_node_id = mermaid_hat_node_id(source_hat.as_str());
        let max_len = meta
            .matching_edges_exact(&from_node_id, topic.as_str(), &target_node_id)
            .map(|edge| edge.path.len())
            .max()
            .unwrap_or(0);

        let step_ms = if max_len == 0 {
            HAT_GRAPH_EDGE_ANIMATION_STEP_MS
        } else {
            let adaptive = HAT_GRAPH_EDGE_ANIMATION_MAX_REVEAL_MS / max_len as u64;
            adaptive.clamp(1, HAT_GRAPH_EDGE_ANIMATION_STEP_MS.max(1))
        };

        self.hat_graph_edge_animations.insert(
            target_hat.clone(),
            HatGraphRadarEdgeAnimation {
                target_hat,
                source_hat,
                topic,
                started_at: now,
                step_ms,
            },
        );
    }

    /// 每帧（render tick）推进 Radar 的可视化状态：
    /// - 清理过旧的 recent events（用于 cause 推断）
    /// - 清理无效的边动画（目标不再 Running）
    pub(crate) fn tick(&mut self, now: Instant, running_hats: Option<&HashSet<String>>) {
        // 1) recent events：只保留 lookback 窗口内的（越界的直接丢弃）
        while let Some(front) = self.hat_graph_recent_events.front() {
            if now.saturating_duration_since(front.observed_at) > HAT_GRAPH_CAUSE_LOOKBACK {
                let _ = self.hat_graph_recent_events.pop_front();
            } else {
                break;
            }
        }

        // 2) edge animations：并行模式只保留 Running 目标的动画;
        //    串行模式(None)保留全部(原语义)。
        if let Some(hats) = running_hats {
            self.hat_graph_edge_animations.retain(|target_hat, _anim| {
                hats.contains(target_hat.as_str())
            });
        }
    }

}
