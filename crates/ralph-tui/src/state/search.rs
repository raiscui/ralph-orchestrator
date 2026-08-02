//! 搜索切片: TuiState 的 search 域。
//!
//! 说明:
//! - 纯状态 + 纯算法: matches 计算不依赖壳(mode/output/parallel)。
//! - 壳负责"从哪里拿行"(串行迭代 / 并行 job buffer),切片负责"怎么找/怎么导航"。

/// 搜索状态(输入框文本 + 已提交查询 + 匹配位置 + 导航索引)。
#[derive(Debug, Default)]
pub struct SearchSlice {
    /// 输入框当前文本(用户正在输入)。
    pub input: String,
    /// Current search query (None when no active search).
    pub query: Option<String>,
    /// Match positions as (line_index, char_offset) pairs.
    pub matches: Vec<(usize, usize)>,
    /// Index into matches vector for current match.
    pub current_match: usize,
    /// Whether search input mode is active (user is typing query).
    pub search_mode: bool,
}

impl SearchSlice {
    /// 在当前查询的匹配集合上计算新的 matches(纯算法)。
    ///
    /// `lines` 是待搜索的每行文本(由壳按 mode 收集)。
    pub fn search_lines(&mut self, query: &str, lines: &[String]) {
        self.query = Some(query.to_string());
        self.matches.clear();
        self.current_match = 0;

        let query_lower = query.to_lowercase();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();

            let mut search_start = 0;
            while let Some(pos) = line_lower[search_start..].find(&query_lower) {
                let char_offset = search_start + pos;
                self.matches.push((line_idx, char_offset));
                search_start = char_offset + query_lower.len();
            }
        }
    }

    /// 前进到下一个匹配(循环), 返回新位置。
    pub fn next(&mut self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_match = (self.current_match + 1) % self.matches.len();
        Some(self.matches[self.current_match])
    }

    /// 后退到上一个匹配(循环), 返回新位置。
    pub fn prev(&mut self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current_match == 0 {
            self.current_match = self.matches.len() - 1;
        } else {
            self.current_match -= 1;
        }
        Some(self.matches[self.current_match])
    }

    /// 当前匹配位置。
    pub fn current(&self) -> Option<(usize, usize)> {
        self.matches.get(self.current_match).copied()
    }

    /// 清空搜索状态。
    pub fn clear(&mut self) {
        self.input.clear();
        self.query = None;
        self.matches.clear();
        self.current_match = 0;
        self.search_mode = false;
    }
}
