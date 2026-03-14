## 1. 协议与路由

- [x] 1.1 在并行 all-hat prompt / 文档约定中新增 `reply.hat.message` 的用途说明,明确它是 hat-to-hat 的显式答案回流 topic,不是默认 final answer 机制
- [x] 1.2 在并行路由层为 `reply.hat.message` 增加 requester-return 分支,按 `reply` 查找原请求事件并解析 `source_instance`
- [x] 1.3 当 requester 成功解析时,把答案事件定向投递给原请求方实例,同时保留原始 `reply` 关联信息

## 2. 失败收口与可观测性

- [x] 2.1 为 unknown reply id 和“原请求无 `source_instance`”实现 fail-closed 行为,禁止把未解析答案当普通 workflow event 广播出去
- [x] 2.2 在事件日志或诊断输出中记录 requester-return 的解析结果,至少覆盖成功目标实例与未解析原因

## 3. 验证与示例

- [x] 3.1 补充单元测试,覆盖 `reply.hat.message` 成功回到 requester 的路径
- [x] 3.2 补充单元或集成测试,覆盖 unknown reply / missing source_instance 的 fail-closed 行为
- [x] 3.3 增加一个并行集成或 E2E 场景,验证“同一被调用 hat 同时回答案并继续 workflow”时两条通道都正常工作
- [x] 3.4 更新相关示例或文档,说明 `reply.human.message` 与 `reply.hat.message` 的职责边界
