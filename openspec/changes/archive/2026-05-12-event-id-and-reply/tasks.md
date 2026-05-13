## 1. 事件协议与解析

- [x] 1.1 为事件协议补齐可引用主键与单值 `reply` 关联字段,确保 `Event.id` / `Event.reply` 可序列化并能表达 in-reply-to 语义
- [x] 1.2 在事件解析层支持 `<event ... reply="EVENT_ID">` 属性解析,并把空字符串 `reply=""` 规范化为无 reply
- [x] 1.3 在事件发布/路由入口为缺失 id 的事件补齐稳定可引用 id,同时保留 agent 显式提供的 id

## 2. Prompt 与文档暴露

- [x] 2.1 在并行模式的 incoming events prompt 中显式展示 `id=<event_id>`,让 hat 可以在后续输出中准确填写 `reply="<id>"`
- [x] 2.2 在 incoming events prompt 中同时展示已有 `reply=` 关联,提升调试、回放与多轮协作时的可读性
- [x] 2.3 更新并行 all-hat / coordinator 协议说明,明确 reply 是单值事件关联字段,供后续 reply.human.message / reply.hat.message 等语义复用

## 3. 测试与验证

- [x] 3.1 补充 parser 单元测试,覆盖 `reply` 属性解析、多行 opening tag 与空 reply 归一化
- [x] 3.2 补充并行 prompt 单元测试,覆盖 incoming events 展示 `id=` 与 `reply=` 的行为
- [x] 3.3 通过定向与仓库级验证确认 event id / reply 协议变更不会破坏现有并行运行时行为
