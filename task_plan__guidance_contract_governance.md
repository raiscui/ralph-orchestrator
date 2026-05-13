# 任务计划: agent guidance contract governance 最小闭环

## 目标
把 oh-my-codex 学习报告中的“契约治理先行”落到 Ralph 仓库: 先建立 guidance schema、prompt contract、agent asset manifest 和 verifier,不先扩大 runtime。

## 阶段
- [x] 阶段1: 续档默认 task_plan 并创建支线计划
- [ ] 阶段2: OpenSpec change: proposal/design/tasks/spec
- [ ] 阶段3: 实现 docs + manifest + verifier/test
- [ ] 阶段4: dogfood 文档和校验
- [ ] 阶段5: 全量验证和收尾记录

## 关键问题
1. 当前应该落地最佳方案还是先能用方案? 选择“最小闭环”: docs + manifest + verifier,暂不碰 runtime state/question/team。
2. 是否需要 OpenSpec? 需要。项目规则要求 spec 先行,并且这是治理能力变更。
3. 是否要接入 cargo test? 优先接入 cargo test,让漂移检查成为门禁。

## 做出的决定
- 新 change 名称暂定 `agent-guidance-contracts`。
- 资产 manifest 先覆盖 AGENTS、EXPERIENCE、核心 docs、OpenSpec change artifact、现有 skill/sop 同步入口。
- verifier 先做路径存在、type 合法、必填字段、docs 被 AGENTS 索引的最小检查。

## 遇到错误
- `omx explore` 已失败并记录到 `ERRORFIX__guidance_contract_governance.md`,已降级本地只读探索。

## 状态
**目前在阶段2** - 准备创建 OpenSpec change。

## [2026-05-11 17:19:40] [Session ID: omx-1778475786175-ogndry] [行动记录]: 创建 OpenSpec change 前的落地边界

- 当前要做:
  - 创建 `openspec/changes/agent-guidance-contracts`。
  - 写 proposal/design/tasks/spec,覆盖 guidance schema、prompt contract、manifest、verifier。
  - 先验证 OpenSpec change,再改 docs 或代码。
- 为什么这样做:
  - 这类治理能力一旦散落到文档和测试里,后续很容易漂移。
  - 用 OpenSpec 先固定范围,能避免直接复制 OMX 的 runtime 复杂度。

## [2026-05-11 17:23:45] [Session ID: omx-1778475786175-ogndry] [状态更新]: OpenSpec change 已通过校验

- 已完成:
  - 创建 `openspec/changes/agent-guidance-contracts/`。
  - 写入 `proposal.md`, `design.md`, `tasks.md`, `specs/agent-guidance-contracts/spec.md`。
  - 运行 `openspec validate agent-guidance-contracts --type change`,结果通过。
- 当前要做:
  - 实现 docs + manifest + verifier。
  - 先补测试,再补实现,保持最小闭环。
- 状态:
  - 阶段2 完成,进入阶段3。

## [2026-05-11 17:24:51] [Session ID: omx-1778475786175-ogndry] [行动记录]: 实现前确认 manifest 解析依赖与文件边界

- 当前要做:
  - 检查仓库是否已有 `toml` 解析依赖。
  - 决定 verifier 放在 `ralph-core` 的新模块还是现有 config/parser 模块。
  - 先写回归测试覆盖 manifest 漂移失败场景。
- 为什么这样做:
  - manifest 设计选用 TOML,实现前要确认是否需要新增依赖。
  - 不能为了省事把 verifier 塞进不相关模块,否则会制造新的职责混乱。

## [2026-05-11 17:29:49] [Session ID: omx-1778475786175-ogndry] [红灯记录]: verifier 测试先失败

- 已执行:
  - `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`
- 结果:
  - 编译失败,`verify_manifest_at` 未实现。
  - 这是预期红灯,说明测试已经约束 verifier 入口。
- 当前要做:
  - 新增 `toml` workspace 依赖。
  - 实现 `agent_guidance_manifest` 模块。
  - 写 docs 和真实 `agent-guidance-manifest.toml`。

## [2026-05-11 17:32:41] [Session ID: omx-1778475786175-ogndry] [错误记录]: invalid_type 测试构造过宽

- 现象:
  - `cargo test --package ralph-core --lib agent_guidance_manifest` 结果 7 passed,1 failed。
  - 失败测试为 `invalid_type_fails`。
- 当前假设:
  - 测试里的字符串替换过宽,把 `project-experience` id 也改成了含下划线的非法 id。
  - 因此 verifier 先报告 id 格式错误,还没有走到 invalid type 分支。
- 当前要做:
  - 将测试替换改为只改 `type = "experience"`。
  - 重跑 focused verifier tests。

## [2026-05-11 17:34:05] [Session ID: omx-1778475786175-ogndry] [验证记录]: focused verifier 测试通过

- 已完成:
  - 修复 `invalid_type_fails` 测试构造。
  - 运行 `cargo test --package ralph-core --lib agent_guidance_manifest`。
- 结果:
  - 8 passed,0 failed。
- 当前要做:
  - 校验 OpenSpec change。
  - 校验 Mermaid 图。
  - 跑更大范围测试。

## [2026-05-11 17:36:03] [Session ID: omx-1778475786175-ogndry] [行动记录]: 处理 ralph-core dead_code warning

- 现象:
  - `cargo test -p ralph-core` 通过,但 `agent_guidance_manifest` 模块在普通 lib 编译中出现 dead_code warnings。
- 判断:
  - verifier 不应该只是测试私有 helper,它是本 change 的正式治理能力。
- 当前要做:
  - 将 `agent_guidance_manifest` 模块导出为 public module。
  - 重跑格式化和 focused / ralph-core 测试,确认 warning 消失。

## [2026-05-11 17:40:28] [Session ID: omx-1778475786175-ogndry] [最终验证记录]: guidance contract governance 最小闭环完成

### 已完成
- OpenSpec:
  - `openspec validate agent-guidance-contracts --type change` 通过。
- 文档:
  - 新增 `docs/agent-guidance-schema.md`。
  - 新增 `docs/prompt-contract.md`。
  - 更新 `AGENTS.md` Project Knowledge Index。
- Manifest:
  - 新增 `agent-guidance-manifest.toml`。
- Verifier:
  - 新增 `crates/ralph-core/src/agent_guidance_manifest.rs`。
  - `ralph-core` 公开 `agent_guidance_manifest` 模块。
  - 新增 repository manifest dogfood 测试。
- 测试:
  - `cargo test --package ralph-core --lib agent_guidance_manifest`: 8 passed,0 failed。
  - `cargo test -p ralph-core smoke_runner`: 12 passed,0 failed。
  - `cargo test -p ralph-core`: 516 lib tests + integration/doctests passed。
  - `cargo test`: exit code 0,全量通过。
- Mermaid:
  - `openspec/changes/agent-guidance-contracts/design.md` 中 Mermaid 已用 `beautiful-mermaid-rs --ascii` 校验通过。

### 阶段状态
- [x] 阶段1: 续档默认 task_plan 并创建支线计划
- [x] 阶段2: OpenSpec change: proposal/design/tasks/spec
- [x] 阶段3: 实现 docs + manifest + verifier/test
- [x] 阶段4: dogfood 文档和校验
- [x] 阶段5: 全量验证和收尾记录

### 状态
**已完成** - 准备交付。

## [2026-05-11 17:42:01] [Session ID: omx-1778475786175-ogndry] [补充验证记录]: 格式、diff 和全局 OpenSpec 门禁通过

- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 17 passed,0 failed。
- 说明:
  - 这是在 `cargo test` 全量通过之后追加的最终静态门禁。
  - 本支线已完成“契约治理最小闭环”,不包括 runtime state/question/team 的后续阶段。

## [2026-05-11 18:50:36] [Session ID: omx-1778475786175-ogndry] [行动记录]: 启动 guidance governance 第二阶段

- 用户要求:
  - “按你建议继续”。
- 当前选择:
  - 第二阶段只推进 `agent-guidance-manifest.toml` 扩展和独立 verifier 入口。
  - 不进入 state operation layer,因为那需要另一个更大的 OpenSpec change。
- 当前要做:
  - 只读探索现有 CLI 子命令结构和项目自有 skill 分布。
  - 创建新 OpenSpec change `agent-guidance-catalog-cli`。
  - 规格通过后再实现。
- 边界:
  - 不归档已完成的 `agent-guidance-contracts`,只在新 change 中扩展第二阶段。
  - 不回滚工作区已有无关改动。

## [2026-05-11 18:52:23] [Session ID: omx-1778475786175-ogndry] [行动记录]: 创建第二阶段 OpenSpec change

- 当前要做:
  - 确认  是否已存在。
  - 创建新的 OpenSpec change,不修改已完成的  范围。
  - 写入 proposal/design/tasks/spec,先用规格锁定第二阶段边界。
- 为什么这样做:
  - 项目规则要求 spec 先行。
  - 第二阶段包含 CLI 和 skill catalog 行为,已经超出第一阶段最小闭环。
  - 单独 change 可以避免把 runtime state/question/team 等更大范围混进来。

## [2026-05-11 18:52:41] [Session ID: omx-1778475786175-ogndry] [修正记录]: 第二阶段 change 名称恢复

- 上一条行动记录中,因为未使用单引号 heredoc,两处反引号内容被 shell 误当作命令替换。
- 正确名称如下:
  - 新 change: `agent-guidance-catalog-cli`。
  - 已完成 change: `agent-guidance-contracts`。
- 当前要做不变:
  - 创建 `openspec/changes/agent-guidance-catalog-cli/`。
  - 先写 OpenSpec artifacts 并验证,再实现 CLI 和 skill catalog。

## [2026-05-11 18:53:36] [Session ID: omx-1778475786175-ogndry] [状态更新]: 第二阶段 OpenSpec change 已创建

- 已完成:
  - 运行 `openspec new change "agent-guidance-catalog-cli"`。
  - change 已创建在 `openspec/changes/agent-guidance-catalog-cli/`。
  - `openspec status --change "agent-guidance-catalog-cli"` 显示 0/4 artifacts complete。
- 当前要做:
  - 读取第一阶段 `agent-guidance-contracts` artifacts 和现有 manifest/verifier/CLI 代码。
  - 写入 proposal/design/spec/tasks。
  - 运行 `openspec validate agent-guidance-catalog-cli --type change`。
- 边界:
  - 只做 skill catalog + 独立 CLI verifier。
  - 不做 runtime state/question/team。

## [2026-05-11 18:54:59] [Session ID: omx-1778475786175-ogndry] [行动记录]: 验证第二阶段 OpenSpec artifacts

- 已完成:
  - 写入 `agent-guidance-catalog-cli` 的 proposal/design/spec/tasks。
- 当前要做:
  - 运行 `openspec validate agent-guidance-catalog-cli --type change`。
  - 抽取 design 中 Mermaid block,用 `beautiful-mermaid-rs --ascii` 验证。
- 为什么这样做:
  - 先确认规格有效,再进入 Rust/CLI 实现。
  - Mermaid 图如果语法有误,应该在落盘阶段就暴露。

## [2026-05-11 18:55:51] [Session ID: omx-1778475786175-ogndry] [行动记录]: 先补 verifier 红灯测试

- 当前要做:
  - 在 `crates/ralph-core/src/agent_guidance_manifest.rs` 增加第二阶段测试。
  - 测试覆盖 report 计数、skill frontmatter、非法 skill root、缺失 name/description、重复 skill name。
- 为什么这样做:
  - 第二阶段改变 verifier 行为,必须先用测试锁定契约。
  - 这能避免只为了 CLI 输出去做表层 patch。

## [2026-05-11 19:02:44] [Session ID: omx-1778475786175-ogndry] [红灯记录]: 第二阶段 verifier 测试失败

- 已执行:
  - `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`。
- 结果:
  - 编译失败: `verify_manifest_at_with_report` 未实现。
- 结论:
  - 这是预期红灯,说明测试已经开始约束第二阶段报告入口。
- 当前要做:
  - 新增 `GuidanceManifestReport`。
  - 新增 report 版 verifier 入口。
  - 增加 skill root/frontmatter/duplicate name 检查。

## [2026-05-11 19:04:52] [Session ID: omx-1778475786175-ogndry] [验证记录]: core verifier 第二阶段测试通过

- 已完成:
  - 新增 `GuidanceManifestReport`。
  - 新增 `verify_default_manifest_with_report` 和 `verify_manifest_at_with_report`。
  - 增加 skill path root、frontmatter `name` / `description`、duplicate skill name 检查。
- 验证:
  - `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`。
  - 结果: 13 passed,0 failed。
- 当前要做:
  - 扩展 `agent-guidance-manifest.toml` 到项目自有 skills。

## [2026-05-11 19:08:30] [Session ID: omx-1778475786175-ogndry] [行动记录]: 增加 CLI verifier 回归测试

- 当前要做:
  - 新增 `crates/ralph-cli/tests/integration_verify.rs`。
  - 使用临时目录构造最小 guidance manifest 仓库。
  - 真实执行 `ralph verify agent-guidance --manifest ... --color never`。
- 为什么这样做:
  - core 单测已经验证 verifier 行为。
  - CLI 还需要独立证据,证明命令解析、退出码和输出摘要可用。

## [2026-05-11 19:09:24] [Session ID: omx-1778475786175-ogndry] [验证记录]: manifest catalog 和 CLI focused 测试通过

- 已完成:
  - `agent-guidance-manifest.toml` 增加 45 个项目 skill 条目。
  - `.agents/skills` 作为重复 OpenSpec skill 的 canonical active 入口。
  - `.codex/skills` 中重复 OpenSpec skill 标为 `archived`,避免 duplicate active skill name。
  - 新增 `ralph verify agent-guidance`。
  - 新增 `crates/ralph-cli/tests/integration_verify.rs`。
- 验证:
  - `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`: 13 passed,0 failed。
  - `cargo run -p ralph-cli -- verify agent-guidance --color never`: 通过,输出 Assets checked: 51, Skills checked: 35。
  - `cargo test -p ralph-cli verify_agent_guidance_command -- --nocapture`: 2 passed,0 failed。
- 当前要做:
  - 运行格式化和中/全量验证门禁。

## [2026-05-11 19:09:54] [Session ID: omx-1778475786175-ogndry] [错误记录]: 格式门禁失败,进入修复

- 现象:
  - `cargo fmt --check` 返回退出码 1。
  - diff 指向新增的 verifier 和 CLI 代码。
- 当前要做:
  - 运行 `cargo fmt`。
  - 重跑 `cargo fmt --check` 和 focused tests。
  - 确认格式修复没有改变功能结果。

## [2026-05-11 19:10:37] [Session ID: omx-1778475786175-ogndry] [行动记录]: 检查长期文档是否需要同步

- 当前要做:
  - 读取 `docs/agent-guidance-schema.md` 和 `docs/prompt-contract.md` 中与 manifest/verifier 相关的内容。
  - 判断是否需要补充 `ralph verify agent-guidance` 和 skill catalog 规则。
- 为什么这样做:
  - 本次实现已经从第一阶段的 cargo-test-only verifier 扩展到独立 CLI。
  - 如果长期文档仍只描述第一阶段,以后维护者会找不到新入口。

## [2026-05-11 19:11:25] [Session ID: omx-1778475786175-ogndry] [行动记录]: 同步 guidance 文档和 manifest

- 当前要做:
  - 在 `docs/agent-guidance-schema.md` 写明 skill catalog 的 root/frontmatter/duplicate 规则。
  - 在同一文档写明 `ralph verify agent-guidance` 是独立 verifier 入口。
  - 将 `agent-guidance-catalog-cli` change 本身登记到 `agent-guidance-manifest.toml`。
- 为什么这样做:
  - CLI 和 verifier 已经从第一阶段扩展到第二阶段,长期文档不能继续只写抽象 verifier。
  - 本阶段 OpenSpec change 也是 guidance governance 资产,应该进入 manifest。

## [2026-05-11 19:12:09] [Session ID: omx-1778475786175-ogndry] [行动记录]: 运行全量验证门禁

- 已完成:
  - `ralph verify agent-guidance --color never`: 通过,检查 52 assets / 35 skills。
  - `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`: 13 passed。
  - `cargo test -p ralph-cli verify_agent_guidance_command -- --nocapture`: 2 passed。
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
- 当前要做:
  - 运行 `cargo test`。
  - 运行 `cargo fmt --check`。
  - 运行 `git diff --check`。
  - 运行 `openspec validate --all --strict`。
- 停止条件:
  - 这些门禁全部通过后,再更新 tasks/worklog 并交付。

## [2026-05-11 19:13:39] [Session ID: omx-1778475786175-ogndry] [最终验证记录]: 第二阶段闭环完成

### 已完成
- OpenSpec change `agent-guidance-catalog-cli` 已创建并完成 4/4 artifacts。
- Manifest 已扩展到项目自有 skills。
- Core verifier 已支持 report、skill root、frontmatter 和 duplicate skill name 检查。
- CLI 已提供 `ralph verify agent-guidance`。
- CLI integration tests 已覆盖成功和失败路径。
- 长期文档已同步新 CLI 和 skill catalog 规则。

### 验证
- `ralph verify agent-guidance --color never`: 52 assets / 35 skills。
- `cargo test --package ralph-core --lib agent_guidance_manifest -- --nocapture`: 13 passed。
- `cargo test -p ralph-cli verify_agent_guidance_command -- --nocapture`: 2 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: 全量通过。
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 18 passed。

### 状态
**已完成** - 准备交付。

## [2026-05-11 19:14:12] [Session ID: omx-1778475786175-ogndry] [收尾记录]: LATER_PLANS 已复核

- 已复核 `LATER_PLANS__guidance_contract_governance.md`。
- 已追加说明:
  - 第二阶段中 skill manifest catalog 和独立 CLI verifier 已完成。
  - prompt contract 对齐和 state operation layer 仍保留为后续独立 change。
- `EPIPHANY_LOG__guidance_contract_governance.md` 不存在,本轮没有需要新增的灾难级风险记录。

## [2026-05-11 20:31:28] [Session ID: omx-1778475786175-ogndry] [行动记录]: 启动 prompt contract 对齐阶段

- 用户要求:
  - 继续处理两个后续建议。
- 当前选择:
  - 本轮只推进第 1 项: 另开 OpenSpec change,把 `docs/prompt-contract.md` 的 output contract 和现有 `InstructionBuilder` / hat prompt tests 对齐。
  - 第 2 项 state operation layer 只保留为后续独立 change,本轮不混入。
- 当前要做:
  - 只读定位 `InstructionBuilder`、hat prompt 构造路径和现有 prompt tests。
  - 创建新 OpenSpec change。
  - 先写 proposal/design/spec/tasks 并验证,再改代码或测试。
- 边界:
  - 不修改 runtime state/question/team。
  - 不回滚工作区已有无关改动。

## [2026-05-11 20:33:28] [Session ID: omx-1778475786175-ogndry] [行动记录]: 创建 prompt contract 对齐 change

- 当前要做:
  - 创建 OpenSpec change `prompt-contract-runtime-alignment`。
  - 读取 `crates/ralph-core/src/instructions.rs`、`hatless_ralph.rs`、`event_loop/tests.rs` 的真实 prompt 构造与断言。
  - 写 proposal/design/spec/tasks,把文档 contract 与 runtime prompt tests 对齐。
- 为什么这样做:
  - `docs/prompt-contract.md` 已经定义 output contract,但 runtime prompt 是否真正提示 outcome/evidence/changed files/known gaps/next suggestions 需要测试门禁固定。
  - state operation layer 属于另一个更大主题,本轮不混入。

## [2026-05-11 20:36:59] [Session ID: omx-1778475786175-ogndry] [验证记录]: prompt contract 对齐 OpenSpec artifacts 通过

- 已完成:
  - 创建 `openspec/changes/prompt-contract-runtime-alignment/`。
  - 写入 proposal/design/spec/tasks。
  - `openspec validate prompt-contract-runtime-alignment --type change` 通过。
  - design 中 2 个 Mermaid block 已用 `beautiful-mermaid-rs --ascii` 校验通过。
- 当前要做:
  - 先补 prompt output anchor 测试,制造红灯。
  - 再更新 `InstructionBuilder::build_custom_hat`。

## [2026-05-11 20:37:38] [Session ID: omx-1778475786175-ogndry] [红灯记录]: output anchor 测试失败

- 已执行:
  - `cargo test --package ralph-core --lib instructions::tests::test_custom_hat_with_rfc2119_patterns -- --exact --nocapture`。
- 结果:
  - 测试失败,缺少 `outcome:`。
- 结论:
  - 当前 `InstructionBuilder::build_custom_hat` 的 REPORT 阶段还没有对齐 `docs/prompt-contract.md` 的 output contract。
- 当前要做:
  - 修改 REPORT 模板,加入 outcome/evidence/changed files/known gaps/next suggestions。
  - 保留 evidence-before-completion 和 must-publish 原规则。

## [2026-05-11 20:38:13] [Session ID: omx-1778475786175-ogndry] [行动记录]: 同步文档和 manifest

- 已完成:
  - `InstructionBuilder` REPORT 阶段加入 output contract anchors。
  - focused unit/integration prompt tests 通过。
- 当前要做:
  - 更新 `docs/prompt-contract.md`,说明字段名是 runtime prompt tests 的稳定锚点。
  - 将 `prompt-contract-runtime-alignment` 登记到 `agent-guidance-manifest.toml`。
  - 运行 `ralph verify agent-guidance`。

## [2026-05-11 20:38:45] [Session ID: omx-1778475786175-ogndry] [行动记录]: 运行 prompt 对齐 focused 验证

- 已完成:
  - `ralph verify agent-guidance --color never` 通过,检查 53 assets / 35 skills。
  - OpenSpec tasks 已更新到当前状态。
- 当前要做:
  - 运行 `InstructionBuilder` focused tests。
  - 运行 `EventLoop::build_prompt` 相关 focused tests。
  - 运行 `cargo test -p ralph-core smoke_runner`。

## [2026-05-11 20:39:11] [Session ID: omx-1778475786175-ogndry] [行动记录]: 运行 prompt 对齐全量门禁

- 已完成:
  - `cargo test --package ralph-core --lib instructions::tests -- --nocapture`: 5 passed。
  - `cargo test --package ralph-core --lib event_loop::tests::test_custom_hat_with_instructions_uses_build_custom_hat -- --exact --nocapture`: 1 passed。
  - `cargo test --package ralph-core --lib event_loop::tests::test_build_prompt_uses_ghuntley_style_for_all_hats -- --exact --nocapture`: 1 passed。
  - `cargo test -p ralph-core smoke_runner`: 12 passed。
- 当前要做:
  - 运行 `cargo test`。
  - 运行 `cargo fmt --check`。
  - 运行 `git diff --check`。
  - 运行 `openspec validate --all --strict`。

## [2026-05-11 20:45:00] [Session ID: omx-1778475786175-ogndry] [行动记录]: 接手 prompt contract 对齐收尾验证

- 当前要做:
  - 继续读取上一轮正在运行的 `cargo test` 结果。
  - 如果测试失败,按实际错误修复并记录到 ERRORFIX。
  - 如果测试通过,继续执行 `cargo fmt --check`、`git diff --check`、`openspec validate --all --strict`。
- 为什么这样做:
  - 不能只依赖交接摘要或中途输出判断完成。
  - prompt contract 对齐已经改了 runtime prompt 和 tests,必须拿到全量门禁证据后才能交付。

## [2026-05-11 20:46:10] [Session ID: omx-1778475786175-ogndry] [错误记录]: prompt 对齐阶段格式门禁失败

- 现象:
  - `cargo fmt --check` 返回退出码 1。
  - diff 指向 `crates/ralph-core/src/event_loop/tests.rs` 中新增 output contract anchor 断言的排版。
- 当前要做:
  - 只对 `crates/ralph-core/src/event_loop/tests.rs` 执行 rustfmt。
  - 重跑 `cargo fmt --check`、`git diff --check`、`openspec validate --all --strict`。
- 为什么这样做:
  - 错误是格式问题,不需要改语义。
  - 只格式化相关文件,避免影响无关本地修改。

## [2026-05-11 20:47:20] [Session ID: omx-1778475786175-ogndry] [错误记录]: 单文件 rustfmt 未覆盖 cargo fmt 全部期望

- 现象:
  - focused prompt test 已通过。
  - `cargo fmt --check` 仍在同一文件 `crates/ralph-core/src/event_loop/tests.rs` 报另一个断言排版 diff。
- 当前要做:
  - 执行项目级 `cargo fmt`。
  - 重跑 `cargo fmt --check`。
  - 然后继续 `git diff --check` 和 `openspec validate --all --strict`。
- 为什么这样做:
  - 文件已经属于本轮 touched 范围,项目级 rustfmt 是最稳的格式真相源。

## [2026-05-11 20:49:30] [Session ID: omx-1778475786175-ogndry] [最终验证记录]: prompt contract 对齐闭环完成

### 已完成
- OpenSpec change `prompt-contract-runtime-alignment` 已创建并补齐 artifacts。
- `InstructionBuilder::build_custom_hat` REPORT 段已加入 output contract anchors。
- `InstructionBuilder` unit test 和 `EventLoop::build_prompt` prompt test 已覆盖 output anchors。
- `docs/prompt-contract.md` 已同步 runtime prompt tests 锚点说明。
- `agent-guidance-manifest.toml` 已登记本 change。
- state operation layer 仍保留为后续独立 change,未混入本轮。

### 验证
- `openspec validate prompt-contract-runtime-alignment --type change`: 通过。
- `beautiful-mermaid-rs --ascii`: design 中 2 个 Mermaid block 通过。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 53 assets / 35 skills。
- `cargo test --package ralph-core --lib instructions::tests -- --nocapture`: 5 passed。
- `cargo test --package ralph-core --lib event_loop::tests::test_custom_hat_with_instructions_uses_build_custom_hat -- --exact --nocapture`: 1 passed。
- `cargo test --package ralph-core --lib event_loop::tests::test_build_prompt_uses_ghuntley_style_for_all_hats -- --exact --nocapture`: 1 passed。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: 全量通过。
- `cargo fmt --check`: 通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 19 passed。

### 状态
**已完成** - 准备交付。

## [2026-05-11 22:46:00] [Session ID: omx-1778475786175-ogndry] [行动记录]: 启动 state operation layer 独立 change

- 用户要求:
  - 按上次建议继续。
- 当前选择:
  - 本轮推进后续第 2 项: 另开 state operation layer 的 OpenSpec change。
  - 只做 OpenSpec artifacts 和验证,不混入 guidance catalog/CLI 或 prompt contract alignment。
- 当前要做:
  - 调查现有 `.omx/state`、runtime state、record/session、diagnostics 相关实现和文档。
  - 创建新 change,命名暂定 `state-operation-layer`。
  - 写 proposal、design、spec、tasks。
  - 校验 Mermaid 和 OpenSpec。
- 边界:
  - 不做 Rust 代码实现。
  - 不修改已有 runtime 行为。
  - 不回滚工作区已有无关改动。

## [2026-05-11 22:48:30] [Session ID: omx-1778475786175-ogndry] [行动记录]: 补齐 state-operation-layer OpenSpec artifacts

- 已完成:
  - 执行 `openspec new change state-operation-layer`。
  - change 已创建在 `openspec/changes/state-operation-layer/`。
- 当前要做:
  - 阅读现有 specs 列表和 manifest 规则,确定新 capability 名称。
  - 写 proposal、design、spec、tasks。
  - 用 `beautiful-mermaid-rs --ascii` 校验 design 中 Mermaid。
  - 运行 `openspec validate state-operation-layer --type change`。
- 边界:
  - 只做 OpenSpec change artifacts。
  - 不实现 Rust state operation 模块。
  - 不把 state operation layer 塞进 `agent-guidance-catalog-cli` 或 `prompt-contract-runtime-alignment`。

## [2026-05-11 22:52:00] [Session ID: omx-1778475786175-ogndry] [验证记录]: state-operation-layer artifacts 初步通过

- 已完成:
  - 写入 `state-operation-layer` proposal、design、spec、tasks。
  - design 中 Mermaid flowchart 通过 `beautiful-mermaid-rs --ascii`。
  - design 中 Mermaid sequence diagram 通过 `beautiful-mermaid-rs --ascii`。
  - `openspec validate state-operation-layer --type change` 通过。
  - `agent-guidance-manifest.toml` 已登记 `state-operation-layer-change`。
- 当前要做:
  - 运行 `ralph verify agent-guidance --color never`。
  - 运行 `openspec validate --all --strict`。
  - 运行 `git diff --check`。
  - 更新记录文件并交付。

## [2026-05-11 22:54:00] [Session ID: omx-1778475786175-ogndry] [最终验证记录]: state-operation-layer OpenSpec 闭环完成

### 已完成
- OpenSpec change `state-operation-layer` 已创建。
- proposal、design、spec、tasks 已补齐。
- design 明确 v1 的状态根目录建议为 `.ralph/state/`。
- spec 明确五个标准 operation: `state_read`、`state_write`、`state_clear`、`state_list_active`、`state_get_status`。
- spec 明确不替代 `.agent/memories.md`、`.agent/tasks.jsonl`、`.ralph/events*.jsonl`、record-session 和 diagnostics。
- `agent-guidance-manifest.toml` 已登记 `state-operation-layer-change`。
- `tasks.md` 已全勾选当前 OpenSpec 设计阶段和记录项。未来实现 tasks 保持未勾选,作为后续实现阶段入口。

### 验证
- `beautiful-mermaid-rs --ascii`: design 中 2 个 Mermaid block 通过。
- `openspec validate state-operation-layer --type change`: 通过。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 54 assets / 35 skills。
- `openspec validate --all --strict`: 20 passed。
- `git diff --check`: 通过。

### EPIPHANY 复核
- 本轮没有新增灾难级风险或架构级事故。
- 不创建 `EPIPHANY_LOG__guidance_contract_governance.md`。

### 状态
**已完成** - 准备交付。

## [2026-05-12 09:10:00] [Session ID: omx-1778475786175-ogndry] [行动记录]: 进入 state-operation-layer 实现阶段

- 用户输入:
  - "继续"。
- 当前选择:
  - 使用 OpenSpec change `state-operation-layer` 继续。
  - 从 `tasks.md` 的 Future implementation tasks 开始实现。
- 当前要做:
  - 读取 `openspec instructions apply --change state-operation-layer --json`。
  - 重读 proposal/design/spec/tasks,确认实现边界。
  - 先补 `ralph-core` 单测制造红灯。
  - 再实现 state operation 数据结构、mode 校验、路径解析、原子写、per-path 写入串行化和五个 core operation。
- 验证计划:
  - focused `ralph-core` state operation tests。
  - `cargo test -p ralph-core smoke_runner`。
  - `cargo test`。
  - `cargo fmt --check`。
  - `git diff --check`。
  - `openspec validate --all --strict`。

## [2026-05-12 09:14:00] [Session ID: omx-1778475786175-ogndry] [行动记录]: 同步 state-operation-layer artifacts 到实现阶段

- 观察:
  - 之前 OpenSpec proposal/design 是设计阶段版本,里面写了“不实现 Rust state operation 模块”。
  - 当前用户要求继续,apply 指令显示 3.x 是实现任务。
- 当前要做:
  - 更新 proposal/design/tasks 的措辞,把本阶段调整为实现 `ralph-core` core operation。
  - 保留“不新增 CLI/MCP adapter、不接 runtime”的非目标。
- 为什么这样做:
  - 规格文件不能和实际实现目标相反。
  - 这能避免后续 archive 或 review 时误判本 change 越界。

## [2026-05-12 09:23:00] [Session ID: omx-1778475786175-ogndry] [验证记录]: state operation core focused tests 通过

- 红灯证据:
  - 首次运行 `cargo test --package ralph-core --lib state_operations -- --nocapture` 编译失败。
  - 失败点是 `StateOperationStore`、`StateMode`、`RunOutcome`、`LifecycleOutcome`、`StateWriteRequest`、`StateClearRequest` 等未实现。
- 已实现:
  - 新增 `crates/ralph-core/src/state_operations.rs`。
  - 新增 `StateMode`、`RunOutcome`、`LifecycleOutcome`、`RuntimeStateRecord`。
  - 新增 `StateOperationStore`。
  - 实现 `.ralph/state` 和 session state path resolver。
  - 实现 temp file + rename 原子写。
  - 实现同 path 写入锁。
  - 实现 `state_read`、`state_write`、`state_clear`、`state_list_active`、`state_get_status`。
  - `ralph-core` 已公开导出 state operation core API。
- focused 验证:
  - `cargo test --package ralph-core --lib state_operations -- --nocapture`: 9 passed,0 failed。
  - 第二次 focused test 无 warning。
- 决策:
  - 本 change 不新增 CLI/MCP adapter。
  - 后续如果需要 `ralph state ...` 或 MCP tools,应单独基于这个 core operation layer 接入。
- 当前要做:
  - 运行 `cargo fmt --check`。
  - 运行 `cargo test -p ralph-core smoke_runner`。
  - 运行 `cargo test`。
  - 运行 `git diff --check` 和 `openspec validate --all --strict`。

## [2026-05-12 09:25:00] [Session ID: omx-1778475786175-ogndry] [错误记录]: state operation 实现阶段格式门禁失败

- 现象:
  - `cargo fmt --check` 返回退出码 1。
  - diff 全部指向 `crates/ralph-core/src/state_operations.rs` 的排版。
- 当前要做:
  - 执行 `cargo fmt`。
  - 重跑 focused state operation tests。
  - 继续 smoke/full 验证。
- 结论:
  - 这是格式问题,不是语义失败。

## [2026-05-12 09:35:00] [Session ID: omx-1778475786175-ogndry] [最终验证记录]: state-operation-layer core 实现完成

### 已完成
- `state-operation-layer` OpenSpec artifacts 已同步到实现阶段。
- `crates/ralph-core/src/state_operations.rs` 已新增。
- `crates/ralph-core/src/lib.rs` 已公开 state operation core API。
- OpenSpec `tasks.md` 已无未勾选任务。
- 本轮未新增 CLI/MCP adapter,该边界已记录到 `LATER_PLANS__guidance_contract_governance.md`。

### 验证
- 红灯: 首次 focused test 编译失败,缺少目标类型和函数。
- `cargo test --package ralph-core --lib state_operations -- --nocapture`: 9 passed。
- `cargo fmt --check`: 通过。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: 全量通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 20 passed。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 54 assets / 35 skills。

### EPIPHANY 复核
- 本轮没有新增灾难级风险或架构级事故。
- 不创建 `EPIPHANY_LOG__guidance_contract_governance.md`。

### 状态
**已完成** - 准备交付。

## [2026-05-12 10:00:00] [Session ID: omx-1778475786175-ogndry] [行动记录]: 启动 ralph-state-cli-adapter 独立 change

- 用户输入: "继续"。
- 上次未完成的下一步:
  - 基于已完成的 `ralph-core` state operation core API,另开 `ralph-state-cli-adapter` change。
  - 为 CLI 增加 `ralph state ...` 只读/清理入口。
- 本阶段目标:
  - 创建 OpenSpec change `ralph-state-cli-adapter`。
  - 明确 CLI adapter 只能调用 `StateOperationStore`,不能在 CLI 中重新实现 JSON 读写、路径解析、merge、clear、atomic write。
  - 实现并验证基础命令: status/read/clear。
- 非目标:
  - 不接 MCP state tools。
  - 不接 runtime question obligation。
  - 不接 capability invocation runtime 状态。
  - 不新增第二套 state 文件格式。
- 当前计划:
  - [ ] 阶段1: 检查现有 state operation core 和 CLI 结构。
  - [ ] 阶段2: 创建并填写 OpenSpec artifacts。
  - [ ] 阶段3: 写 CLI adapter 测试,先得到可解释红灯。
  - [ ] 阶段4: 实现 CLI adapter。
  - [ ] 阶段5: 运行 focused tests、smoke、全量验证和 OpenSpec 校验。
- 当前状态:
  - 准备进入阶段1。

## [2026-05-12 10:03:00] [Session ID: omx-1778475786175-ogndry] [阶段更新]: 阶段1完成,准备创建 OpenSpec change

- 阶段1结果:
  - [x] 已确认 CLI 命令组织位置: `crates/ralph-cli/src/main.rs`。
  - [x] 已确认 CLI integration test 模式: `crates/ralph-cli/tests/integration_verify.rs`。
  - [x] 已确认 core 单一真相源: `StateOperationStore` 和 `StateMode` 已由 `ralph-core` 导出。
- 新增观察:
  - `omx explore` 因外部 503 不可用,已回落到普通只读 shell 检查。
  - 工作树已有大量在途改动,本阶段只触碰 `ralph-state-cli-adapter` 所需文件。
- 下一步:
  - 创建 `openspec/changes/ralph-state-cli-adapter/`。
  - 写 proposal/design/spec/tasks。

## [2026-05-12 10:09:00] [Session ID: omx-1778475786175-ogndry] [阶段更新]: 阶段2 OpenSpec artifacts 通过

- 已完成:
  - [x] `openspec/changes/ralph-state-cli-adapter/proposal.md`。
  - [x] `openspec/changes/ralph-state-cli-adapter/design.md`。
  - [x] `openspec/changes/ralph-state-cli-adapter/specs/state-cli-adapter/spec.md`。
  - [x] `openspec/changes/ralph-state-cli-adapter/tasks.md`。
  - [x] `agent-guidance-manifest.toml` 登记 `ralph-state-cli-adapter-change`。
- 验证:
  - `beautiful-mermaid-rs --ascii < block-1.mmd`: 通过。
  - `beautiful-mermaid-rs --ascii < block-2.mmd`: 通过。
  - `openspec validate ralph-state-cli-adapter --type change`: 通过。
- 下一步:
  - 进入阶段3,先写 CLI integration tests 制造红灯。

## [2026-05-12 10:14:00] [Session ID: omx-1778475786175-ogndry] [红灯记录]: state CLI tests 先失败

- 新增测试文件:
  - `crates/ralph-cli/tests/integration_state.rs`。
- 红灯命令:
  - `cargo test -p ralph-cli --test integration_state -- --nocapture`。
- 红灯结果:
  - 4 个测试全部失败。
  - 失败原因一致: `error: unrecognized subcommand 'state'`。
- 结论:
  - 测试覆盖的是当前缺失的 CLI adapter,不是 core state operation 问题。
- 下一步:
  - 在 `crates/ralph-cli/src/main.rs` 新增 `Commands::State`、参数结构和 handler。
  - handler 只调用 `StateOperationStore`。

## [2026-05-12 10:23:00] [Session ID: omx-1778475786175-ogndry] [阶段更新]: state CLI adapter focused tests 通过

- 已实现:
  - `Commands::State(StateArgs)`。
  - `StateCommands::{Status, Read, Clear}`。
  - `ralph state status [--mode <mode>] [--session-id <id>] [--json]`。
  - `ralph state read <mode> [--session-id <id>] [--json]`。
  - `ralph state clear <mode> [--session-id <id>] [--all-sessions]`。
- 边界:
  - handler 使用 `StateOperationStore`。
  - mode 解析使用 `StateMode`。
  - CLI 不直接读写 state JSON。
  - v1 不暴露 `write`。
- focused 验证:
  - 初始红灯: `cargo test -p ralph-cli --test integration_state -- --nocapture` 失败于 `unrecognized subcommand 'state'`。
  - 实现后: 同一命令 4 passed。
  - 补 help 契约后: 同一命令 5 passed。
- 下一步:
  - 运行 `cargo fmt --check`。
  - 如有格式漂移,执行 `cargo fmt` 后重跑 focused tests。
  - 继续 smoke/full/OpenSpec/guidance 验证。

## [2026-05-12 10:34:00] [Session ID: omx-1778475786175-ogndry] [最终验证记录]: ralph-state-cli-adapter 完成

### 已完成
- `ralph-state-cli-adapter` OpenSpec change 已创建并补齐 artifacts。
- `agent-guidance-manifest.toml` 已登记新 change。
- `crates/ralph-cli/src/main.rs` 已新增 `ralph state` command group。
- `crates/ralph-cli/tests/integration_state.rs` 已新增 5 个 integration tests。
- OpenSpec `tasks.md` 已全勾选。

### 验证
- 红灯: 初始 focused test 失败于 `unrecognized subcommand 'state'`。
- `cargo test -p ralph-cli --test integration_state -- --nocapture`: 5 passed。
- `cargo fmt --check`: 通过。
- `cargo test -p ralph-core smoke_runner`: 12 passed。
- `cargo test`: 全量通过。
- `git diff --check`: 通过。
- `openspec validate --all --strict`: 21 passed。
- `cargo run -p ralph-cli -- verify agent-guidance --color never`: 55 assets / 35 skills。

### EPIPHANY 复核
- 没有新增灾难级风险或架构级事故。
- 不创建 `EPIPHANY_LOG__guidance_contract_governance.md`。

### 状态
**已完成** - 准备交付。

## [2026-05-12 10:45:00] [Session ID: omx-1778475786175-ogndry] [行动记录]: 归档 ralph-state-cli-adapter

- 用户要求:
  - 执行 `openspec archive ralph-state-cli-adapter`。
- 已观察到的失败:
  - 用户本地执行时 OpenSpec 提示 `Proceed with spec updates? (Y/n)`,随后交互被关闭。
  - 失败原因是交互确认未完成,不是任务未完成。
- 当前要做:
  - 查询 `openspec archive` 的非交互确认参数。
  - 使用非交互确认归档 `ralph-state-cli-adapter`。
  - 运行 OpenSpec strict 校验。
  - 更新记录文件。

## [2026-05-12 10:48:00] [Session ID: omx-1778475786175-ogndry] [最终记录]: ralph-state-cli-adapter 已归档

- 已执行:
  - `openspec archive ralph-state-cli-adapter --yes`。
- 归档结果:
  - archived path: `openspec/changes/archive/2026-05-12-ralph-state-cli-adapter/`。
  - main spec created: `openspec/specs/state-cli-adapter/spec.md`。
- 归档后修正:
  - `agent-guidance-manifest.toml` 的 `ralph-state-cli-adapter-change` path 改为 archive path,status 改为 `archived`。
  - `openspec/specs/state-cli-adapter/spec.md` 的 Purpose 从 TBD 改为实际说明。
- 验证:
  - `openspec validate --all --strict`: 21 passed。
  - `cargo run -p ralph-cli -- verify agent-guidance --color never`: 55 assets / 35 skills。
  - `git diff --check`: 通过。
  - `openspec list --json`: active changes 中不再包含 `ralph-state-cli-adapter`。
- EPIPHANY 复核:
  - 没有新增灾难级风险或架构级事故。
