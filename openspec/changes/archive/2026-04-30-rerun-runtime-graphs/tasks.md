## 1. Change Boundary And Data Model

- [x] 1.1 明确 `ralph hats graph` 与 Rerun runtime graph 的职责边界
- [x] 1.2 定义 runtime graph 的节点类型、边类型和核心属性
- [x] 1.3 定义 V1 / V2 的边界,并把“不在 V1 承诺的内容”写清楚

## 2. V1 Live Runtime Graph

- [x] 2.1 盘点并接线现有 live observers 可提供的图更新事件
- [x] 2.2 设计 V1 的 Runtime Topology Graph 视图
- [x] 2.3 设计 V1 的 Workflow Event Graph 视图
- [x] 2.4 设计 V1 的 Delivery / Reply Trace 最小可行视图
- [x] 2.5 定义 V1 的用户入口、命名与 artifact 形式

## 3. V2 Durable Replay Graph

- [x] 3.1 盘点当前 durable 证据缺口,尤其是 `target_instance` 与 fanout recipients
- [x] 3.2 设计 delivery-level durable records,支持离线 replay graph
- [x] 3.3 设计 create/spawn lineage 和 lifecycle control edges 的 durable 证据
- [x] 3.4 定义 replay graph 的重建顺序、时间轴和过滤语义

## 4. Documentation And Validation

- [x] 4.1 记录 Rerun graph 与现有 Mermaid graph 的关系,避免用户混淆
- [x] 4.2 为 proposal / design 中的 V1 / V2 分层提供实施指南
- [x] 4.3 校验设计里的图示与术语,确保后续实现时不会失去边界
