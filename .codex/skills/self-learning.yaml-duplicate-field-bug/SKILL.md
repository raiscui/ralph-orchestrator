---
name: self-learning.yaml-duplicate-field-bug
description: |
  修复/规避 YAML schema 字段在顶层重复出现导致的 "duplicate field" 错误(serde_yaml 报错)。
  适用场景: (1) 你要把命令式 N 个 OR-group assertions 映射到 schema 的 Vec<Vec<T>> 字段(每个 group 一个 nested list); (2) 你在 YAML 顶层写了 N 次同一个字段名, 每个下面挂一组 list; (3) cargo test --lib 全过, 但 cargo run -- --list 报 "duplicate field X"; (4) 你用 `awk '^[a-z_]+:'` 或类似简单 grep 验证字段唯一性, 但漏掉了 2-space indent 的字段重复。
  方案: 合并所有 OR group 到 1 个字段下, 用 nested list 区分 group; 验证用 Python re.findall + Counter 检测全 indent levels, 不只是 0-indent 顶层 key。
author: Codex CLI
version: 1.0.0
date: 2026-08-13
---

# YAML 顶层字段重复: Vec<Vec<T>> 误写多个同名 key

## 问题
schema 是 `Vec<Vec<T>>`(多组 OR, 每组一个 nested list)。你写了 N 个命令式 OR-group 断言, 打算映射成 N 组 list, 但你把每组写成独立的顶层字段:

```yaml
expect:
  # 组 1: 第一个 OR-group (8 case variants)
  output_contains_any:
    - ["Read", "read", "Bash", "bash", "cat ", "test-data.txt", "Tool", "tool"]
  # 组 2: 第二个 OR-group (4 case variants)
  output_contains_any:
    - ["Approved", "approved", "NEEDS_CHANGES", "needs_changes"]
```

`cargo check` ok, `cargo test --lib` ok (单测不需要 parse YAML), 但 `cargo run -- --list` 触发 from_yaml 反序列化, `serde_yaml` 报:
```
invalid declarative scenario X: expect: duplicate field `output_contains_any` at line 79 column 3
```

## 触发条件
满足任一条, 应该用这个 skill:

1. 你在写 schema Vec<Vec<T>> 字段的 YAML, 打算映射多个 OR-group。
2. 你在顶层写了多个同名字段, 每个挂一组 list。
3. 你的 `cargo check` + `cargo test --lib` 都过, 但 `cargo run -- --list` 报 duplicate field。
4. 你之前用 awk `^[a-z_]+:` 校验字段唯一性, 但漏掉 2-space indent 的重复。

## 解决方案

### 1) 合并到 1 个字段, nested list 区分 group
正确写法:
```yaml
expect:
  # output_contains_any: 多组 (per-assertion-OR group) 合并到 1 个字段
  # 组 1 - 第一个 OR-group (8 case variants)
  # 组 2 - 第二个 OR-group (4 case variants)
  output_contains_any:
    - ["Read", "read", "Bash", "bash", "cat ", "test-data.txt", "Tool", "tool"]
    - ["Approved", "approved", "NEEDS_CHANGES", "needs_changes"]
```

每组是 schema 的 1 个 nested list; 各组之间是 AND (由 runner 的 `assertions.iter().all` 强制)。这与命令式 "多个 OR-group 全部断言" 的语义匹配。

### 2) 写完 YAML 必须跑 `--list` 验证
不只是依赖 `cargo check` + `cargo test --lib`。YAML 反序列化是运行时行为, 必须实际触发:
```bash
cargo run -p <crate> --quiet -- --list | grep <scenario-id>
```
若 `serde_yaml` 报 duplicate field, 立刻定位到 YAML 顶层重复字段并合并。

### 3) 用 Python re.findall + Counter 检测全 indent levels
简单 awk `^[a-z_]+:` 只匹配 0-indent 顶层 key, 漏掉 expect: 内 2-indent 的重复。Python 检测:
```python
import re
from collections import Counter
content = open("scenario.yaml").read()
keys = re.findall(r"^(\s*)([a-z_]+):", content, re.MULTILINE)
counts = Counter(k[1] for k in keys)
dupes = [(k, c) for k, c in counts.items() if c > 1]
print(f"dupes: {dupes if dupes else 'NONE'}")
```

注意: `re.findall` 会捕获 YAML literal block (`config: |`) 内的伪 key (像 `max_iterations:`)。这些不是真 schema 字段, 是 config 文件字符串内容。需要在检测后人工 review, 或写更精确的解析器 (如 ruamel.yaml)。

## 验证
- 写完每个 YAML 立刻跑 `cargo run -p ralph-e2e -- --list`。
- 跑 `cargo test -p ralph-e2e --lib` 全过(单测不 catch duplicate field, 但确认 YAML 反序列化在测试中也被触发)。
- 把 Python 检测脚本放进 `scripts/yaml_dupes_check.py` 或类似位置, 供未来 commit 复用。

## 反例
不要写:
```yaml
# 反例: 多个同名顶层字段(serde_yaml 拒绝)
expect:
  output_contains_any:
    - ["Read", "read", "Bash", "bash"]
  output_contains_any:
    - ["Approved", "approved"]
```

也避免:
```yaml
# 反例: 把 OR-group 拆成 N 个 boolean 字段(语义退化为 AND)
expect:
  has_marker_in_stdout: true
  has_marker_in_stderr: true
```
