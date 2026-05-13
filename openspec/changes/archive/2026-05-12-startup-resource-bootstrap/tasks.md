## 1. Resource Catalog 与用户资源目录

- [x] 1.1 定义 resource catalog 的条目 schema,至少覆盖 workflow preset、backend preset、prompt template、example bundle,并为每类资源补结构化 metadata(summary / goal / selector hints)
- [x] 1.2 引入用户资源目录解析器与 embedded bundle 同步机制,明确首次释放、版本标记与“不覆盖用户已改文件”的策略
- [x] 1.3 把现有 embedded presets、`presets/minimal/*` 与 bootstrap prompt template 纳入 catalog,并补目录/注册测试

## 2. Bootstrap Selector 与启动解析

- [x] 2.1 在 `ralph run` 启动前引入 bootstrap selector v1(纯规则)阶段,仅在缺少显式 config source 时运行
- [x] 2.2 让 selector 产出可审计的 `resolved config` artifact,再用它启动真实 orchestration loop
- [x] 2.3 明确禁止“真实 run 启动后热切换整套 topology”,并补验证保证 selector 在 `EventLoop` / `Supervisor` 初始化前完成
- [x] 2.4 在文档与 artifact 中固定 v2 路线: 后续允许“规则优先 + LLM fallback selector”,但不把它混进当前 v1 实现

## 3. Prompt Source Resolver 与默认无文件启动

- [x] 3.1 抽象统一 prompt source resolver,覆盖 CLI prompt、config prompt、prompt file、prompt template、idle bootstrap
- [x] 3.2 让“无 `PROMPT.md` + 无 `ralph.yml`”走 bootstrap resolution,而不是普通 run 直接报错
- [x] 3.3 补针对串行 run、并行 run、idle-capable preset、自带 inline prompt preset 的回归测试

## 4. 结构化组合与文档

- [x] 4.1 定义单 workflow + 单 backend + overlays 的结构化 merge 规则,对 key 冲突给出显式报错或覆盖语义
- [x] 4.2 明确 examples 默认不参与 selector 候选,只作为 materialize/template 资源
- [x] 4.3 更新 `ralph init`、`doctor`、getting-started、preset 文档与 migration 文档,解释 catalog、用户资源目录与 bootstrap selector 语义
- [x] 4.4 在文档中明确 startup-only 边界,并链接 follow-up 的 runtime workflow / hat capability change
