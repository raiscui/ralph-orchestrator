## [2026-08-02 13:30:00] [Session ID: omx-1785579233065-awidzo] ERRORFIX: e2e 并行场景 LOOP_COMPLETE 检测口径分裂

### 现象
- 3 个 live 场景稳定失败(新旧代码一致): parallel-emit-spawn-instance / parallel-app-server-idle-start-live / parallel-app-server-steer-multi-turn(+live)
- 统一失败断言: LOOP_COMPLETE detected, termination_reason=None
- 但 human-log 显示协调者已输出 LOOP_COMPLETE

### 原因(已用动态证据验证)
- 产品代码收敛正常: supervisor 内部检测 promise_ok=true(临时 eprintln 证实)
- e2e 的 detect_termination_reason 从 ralph 进程完整 stdout 检测:
  1. 并行显示行带 [ralph#1:out:job=2] 前缀 → 不满足"独占一行 == LOOP_COMPLETE"
  2. 事件回显行 payload 含 LOOP_COMPLETE 说明文本 → promise_in_event_tags=true 触发安全拒绝
  3. err 行 prompt 回显整行 LOOP_COMPLETE → 假收敛风险
- 根因: e2e 检测口径与并行显示格式不匹配, 与产品代码无关

### 修复
- crates/ralph-e2e/src/executor.rs detect_termination_reason:
  - 并行模式(检测到 [ralph#1:out: 行): 只取协调者 out 显示行 → 剥离前缀 → 排除 <event 事件行 → contains_promise
  - 串行模式: 原逻辑不变
- 新增 3 个测试: 前缀行检测 / 事件+err 干扰不误判 / 串行回退

### 验证
- cargo test -p ralph-e2e: 504+3+38 全过
- live 重跑: app-server-idle-start-live ✅ / app-server-steer-multi-turn(+live) ✅ / emit-spawn ✅
- 剩余失败: steer-live-reply-multi-turn(answers 未出现, steer 时序/LLM 行为, 非检测)

### 教训
- e2e 从进程 stdout 检测协议时, 必须匹配显示层格式(前缀/回显), 否则"产品已收敛但测试报失败"
- 区分"产品 bug"与"测试口径 bug"的决定性证据: supervisor 内部检测结果(eprintln 直出)

## [2026-08-02 14:30:00] [Session ID: omx-1785579233065-awidzo] ERRORFIX: 会话定向事件被改投 ralph#2, steer 上下文丢失

### 现象
- parallel-app-server-steer-live-reply-multi-turn 失败: answers 未出现(answer164=false, answer15=false)
- 实际输出: ralph#2 编造 marker 占位符("E2E_..."),两次答案都是 164(第二个应为 15)

### 原因(动态证据链)
1. E2E-DEBUG 路由日志: step2 路由时 primary_state=Running
2. events.jsonl: ralph#2 创建 reason="ralph_secondary_fallback"(routing 的 choose_ralph_instance_for_delivery)
3. routing.rs rewrite_target_for_busy_ralph: target=ralph#1 + turn_action=Start(非 steer/interrupt)+ 无 source → ralph#1 Running → 改投 ralph#2
4. 竞态: tokio::select! 公平调度, tick(读外部事件)可能先于 instance_rx 的 StateChanged(idle)处理 → 状态还是 Running
5. ralph#2 是新 app-server session, 看不到 ralph#1 会话里 steer 注入的输入 → 模型编造

### 修复
- routing.rs rewrite_target_for_busy_ralph: 事件显式携带 session_strategy 时不再改投(会话绑定, 与 steer/interrupt 同类豁免)
- 新增单元测试: busy_ralph_session_directed_event_stays_on_primary

### 验证
- busy_ralph 5 测试 + routing 77 测试全过
- live 回归: app-server 5 场景全过(idle-start(+live)/steer-multi-turn(+live)/steer-live-reply) + emit-spawn ✅
- core 645+ 全过, workspace check/clippy 干净

### 教训
- select! 公平调度的竞态窗口: 事件路由前要确认实例状态已刷新; 状态更新与外部事件读取之间无优先级
- 会话绑定事件(session_strategy)改投 = 上下文丢失, 路由必须尊重会话语义
