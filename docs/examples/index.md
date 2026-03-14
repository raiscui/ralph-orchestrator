# 示例

这里收录 Ralph 的实际用法示例。
如果你想快速挑一个合适的例子开始看,先看下面这张索引表。

## 本节内容

| 示例 | 说明 |
|---------|-------------|
| [简单任务](simple-task.md) | 传统模式的基础用法 |
| [TDD 工作流](tdd-workflow.md) | 使用 hats 做测试驱动开发 |
| [规格驱动开发](spec-driven.md) | 先写规格,再进入实现 |
| [多角色工作流](multi-hat.md) | 多角色协作的复杂编排 |
| [问题排查](debugging.md) | 用 Ralph 排查问题 |
| [真实并行范例中文总览](parallel-real-world-examples.zh-CN.md) | 中文版真实并行范例选型、矩阵与扩批建议 |

## 快速示例

### 传统模式

一个最简单的循环直到完成:

```bash
ralph init --backend claude

cat > PROMPT.md << 'EOF'
写一个计算阶乘的函数。
补上测试。
EOF

ralph run
```

### Hat 模式

使用 TDD 预设:

```bash
ralph init --preset tdd-red-green

cat > PROMPT.md << 'EOF'
实现一个 URL 校验函数。
必须处理:
- HTTP 和 HTTPS 协议
- IPv4 地址
- 域名
- 端口号
EOF

ralph run
```

### 内联提示词

如果你不想单独放 `PROMPT.md`,可以直接这样跑:

```bash
ralph run -p "给注册表单补输入校验"
```

### 自定义配置

覆盖默认值:

```bash
ralph run --max-iterations 50 -p "重构认证模块"
```

## 工作流示例

### 功能开发

```bash
# 用 feature 预设初始化
ralph init --preset feature

# 写一个更详细的提示词
cat > PROMPT.md << 'EOF'
# 功能: 用户仪表盘

新增一个用户仪表盘,包含:
- 个人资料摘要卡片
- 最近活动流
- 快捷操作按钮

使用 React 组件。
遵循现有 UI 风格。
EOF

# 运行 Ralph
ralph run
```

### 问题排查

```bash
# 用 debug 预设初始化
ralph init --preset debug

# 描述问题
ralph run -p "用户反馈 Safari 登录失败,报错: 'Invalid token'。请排查并修复。"
```

### 代码评审

```bash
# 用 review 预设初始化
ralph init --preset review

# 评审指定文件
ralph run -p "审查 src/api/auth.rs 的改动,重点看安全问题"
```

## 完整示例

更详细的说明在这些页面里:

- [简单任务](simple-task.md) — 传统模式的逐步示例
- [TDD 工作流](tdd-workflow.md) — 使用 hats 做红-绿-重构
- [规格驱动开发](spec-driven.md) — 从规格走到实现
- [多角色工作流](multi-hat.md) — 多角色协同编排
- [问题排查](debugging.md) — 问题排查工作流
- [真实并行范例中文总览](parallel-real-world-examples.zh-CN.md) — 面向中文读者的并行范例选型与总览
