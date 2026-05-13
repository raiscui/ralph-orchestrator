# Prompt contract

本文件定义 Ralph 中 prompt-like 资产的最小行为契约。
这里的 prompt-like 资产包括 prompt、skill、hat instructions、workflow prompt、review prompt,以及最终面向用户的回复要求。

## 1. 核心原则

Prompt 的职责不是写越多指令越好。
Prompt 的职责是把行为边界、输入输出、验证要求和停止条件讲清楚。

最小契约如下:

- 先说目标结果,再说过程。
- 先区分事实、假设和结论,再给判断。
- 没有验证证据,不要声称完成。
- 能安全执行的本地可逆操作,由 agent 自己执行。
- 破坏性、凭证受限、外部生产环境、重大分支决策,必须升级给用户。

## 2. 输出契约

任何要求 agent 交付结果的 prompt,都应该让最终输出包含:

- outcome: 实际完成了什么
- evidence: 用什么命令、文件、日志或产物证明
- changed files: 如果改了文件,列出关键路径
- known gaps: 没跑的验证、无法确认的边界或遗留风险
- next suggestions: 任务完成后的后续建议

如果任务没有修改代码,`changed files` 可以省略。
如果没有已知缺口,明确说没有发现新的缺口,不要编造风险。

这些字段名也是 runtime prompt tests 的稳定锚点。
修改 `InstructionBuilder`、hat instructions、workflow prompt 或 final-response prompt 时,可以优化自然语言,但不要静默删除这些字段名。
如果确实要改字段名,必须同步更新 prompt contract 文档和对应 prompt tests。

## 3. Completion claim 门槛

出现以下表达前,必须先有新鲜验证证据:

- fixed
- done
- complete
- passing
- 已修复
- 已完成
- 测试通过

最低要求:

1. 识别能证明该声明的命令或证据。
2. 实际运行或读取该证据。
3. 检查退出码、失败数和错误输出。
4. 把关键结果写进最终回复或 worklog。

如果验证无法运行,必须说明原因和替代检查。

## 4. Bug 分析契约

分析 bug、性能回退、交互异常、布局异常时,必须按这个结构:

1. 现象: 已观察事实。
2. 假设: 候选原因。
3. 备选解释: 最强竞争解释。
4. 验证计划: 最小可证伪实验。
5. 结论: 只有被证据支撑后才能写成结论。

没有动态证据时,不要把候选假设写成根因。

## 5. Scope boundary

Prompt 必须说明自己的边界。

常见边界:

- 只读调查,不改文件。
- 只改当前模块,不碰 unrelated local changes。
- 先写 OpenSpec,不直接实现。
- 只做 docs + verifier,不改 runtime topology。
- 只处理当前 hook 报告,不接管工作区所有未提交改动。

边界应该帮助 agent 收敛,而不是成为逃避工作的借口。

## 6. Escalation 条件

遇到以下情况必须升级给用户:

- 需要删除或覆盖用户可能需要的数据。
- 需要访问或修改外部生产环境。
- 需要凭证、付费服务或不可逆操作。
- 需求有多个合理方向,且选择会显著改变产品行为。
- 当前证据不足,继续执行会变成猜测式修复。

普通本地读取、编辑、测试、格式化不需要询问。

## 7. Prompt author checklist

写或改 prompt-like 资产时,检查:

- [ ] 目标结果清楚。
- [ ] 输入和输出契约清楚。
- [ ] 验证方式清楚。
- [ ] 停止条件清楚。
- [ ] scope boundary 清楚。
- [ ] escalation 条件清楚。
- [ ] 没有把临时 workaround 写成长期规则。
- [ ] 没有要求 agent 依赖不可解析的 Markdown/YAML 注释作为机器 metadata。
