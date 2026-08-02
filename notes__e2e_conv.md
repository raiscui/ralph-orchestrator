# e2e 收敛稳定性调查

## 背景
- 3 个 live 场景失败(新旧代码一致): parallel-emit-spawn-instance / parallel-app-server-idle-start-live / parallel-app-server-steer-multi-turn(+live)
- 统一模式: termination_reason=None(协调者未输出 LOOP_COMPLETE)
- 证据: emit-spawn 事件流完整(spawn.task → spawn.done), 但无 loop.terminate
