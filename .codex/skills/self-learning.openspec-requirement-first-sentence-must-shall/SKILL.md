---
name: self-learning.openspec-requirement-first-sentence-must-shall
description: |
  修复 OpenSpec 在归档 change 时的校验失败：`openspec archive` 报错 “Requirement must contain SHALL or MUST”。
  适用场景：(1) delta specs 里有 `### Requirement:`，但该段落第一句是描述性文本；(2) 归档被 validator 卡住无法继续。
  方案：让每个 `### Requirement:` 标题后的第一句包含 MUST/SHALL，再重试归档或 validate。
author: Claude Code
version: 1.0.0
date: 2026-01-29
---

# OpenSpec：Requirement 首句必须包含 MUST/SHALL（archive validator）

## 问题
在使用 OpenSpec 归档变更时（`openspec archive <change>`），可能会遇到 validator 直接失败：

- `Validation errors in change delta specs:`
- `... must contain SHALL or MUST`

这会导致 change 无法归档，后续也无法把 delta specs 同步到 `openspec/specs/` 主规格里。

## 上下文 / 触发条件
满足以下任意一个现象，就应该用这个 skill：

1. 你执行 `openspec archive -y <change>` 或 `openspec archive <change>`，输出包含：
   - `must contain SHALL or MUST`
2. 报错定位到某个 delta spec 的 Requirement 标题，例如：
   - `ADDED "xxx" must contain SHALL or MUST`
3. 你检查对应的 `openspec/changes/<change>/specs/**/spec.md`，发现：
   - `### Requirement: ...` 下面的第一句话是“描述性陈述”，没有 MUST/SHALL。

## 解决方案
目标很简单：**让每个 `### Requirement:` 的“第一句”变成规范性陈述（含 MUST/SHALL）**。

### 步骤
1. 打开报错指向的 delta spec 文件：
   - `openspec/changes/<change>/specs/<capability>/spec.md`
2. 找到对应的标题：
   - `### Requirement: <title>`
3. 检查标题下面的第一句（紧挨着标题的第一行正文）。
4. 如果第一句不包含 MUST/SHALL，改成这种形式之一：
   - `In <context>, <subject> MUST <do something>.`
   - `<Subject> MUST <do something> when <condition>.`
5. 把“解释性/背景性文字”放到第二句或后面（它可以不包含 MUST/SHALL）。
6. 重新运行归档：
   - `openspec archive -y <change>`

## 验证
满足以下条件即可认为修复成功：

1. `openspec archive ...` 不再报 `must contain SHALL or MUST`
2. 归档成功，并输出类似：
   - `Specs updated successfully.`
   - `Change '<name>' archived as 'YYYY-MM-DD-<name>'.`

## 示例
下面是一个典型“前后对比”的最小改法（只改第一句）：

- 原先（会失败）：
  - `In parallel mode, task.start and task.resume are control-plane topics.`
- 修复后（可通过）：
  - `In parallel mode, task.start and task.resume MUST be treated as control-plane topics.`

## 备注
- 这条规则看起来像“格式要求”，但从规格写作角度也更健康：
  - Requirement 段落应当以规范性语句开头，避免“读者以为只是描述，而不是约束”。
- 目前经验是：validator 主要盯“标题后的第一句”。所以不要只在后面几行写 MUST，第一句仍是描述性陈述。

## 参考资料
- 无（基于本仓库实际运行 `openspec archive` 的输出与修复经验沉淀）。
