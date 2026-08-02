## [2026-08-02 13:30:00] [Session ID: omx-1785579233065-awidzo] 任务名称: e2e 收敛稳定性调查与修复

### 任务内容
- 排查 3 个 live 场景的 LOOP_COMPLETE 收敛失败(旧代码同样失败)
- 证据链: 保留 workspace → human-log(协调者已输出 LOOP_COMPLETE)→ supervisor 临时 eprintln(内部检测 promise_ok=true)→ e2e detect 诊断(in_tags=true 事件回显干扰)
- 修复 e2e 检测口径(只取协调者 out 行 + 排除事件/err 行)
- 3 个场景从失败变通过, 1 个剩余失败确认是 steer 时序问题

### 完成过程
- baseline A/B 对照(HEAD worktree 构建)排除回归嫌疑
- 临时 eprintln 直出(避开 tracing 配置)拿到决定性证据
- 最小可证伪: 单元测试复现 contains_promise 通过, 锁定问题在 e2e 检测侧

### 总结感悟
- live e2e 失败排查顺序: 事件流完整性 → 内部检测日志 → 测试口径
- e2e 检测要匹配显示层格式; "产品收敛但测试失败"往往是检测口径问题

## [2026-08-02 14:30:00] [Session ID: omx-1785579233065-awidzo] 任务名称: steer-live-reply 收敛修复(会话定向路由)

### 任务内容
- 定位: ralph#2(secondary fallback)接收 step2, 新会话丢失 steer 上下文
- 修复: rewrite_target_for_busy_ralph 豁免 session_strategy 定向事件
- 测试: busy_ralph_session_directed_event_stays_on_primary
- 验证: live 5/5 app-server 场景 + emit-spawn 全过

### 完成过程
- 证据: E2E-DEBUG 路由日志(primary_state=Running)→ events.jsonl(reason=ralph_secondary_fallback)→ routing.rs 代码确认
- select! 公平调度竞态是深层原因, 修复落在语义层(会话定向不改投)

### 总结感悟
- live 场景全绿: e2e 的 parallel 验证现在完全可靠
- 修复同时消除了"外部事件在 job 完成窗口被误投"的竞态类别
