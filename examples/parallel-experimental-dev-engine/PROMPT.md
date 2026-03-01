# 实验计划(Parallel Experimental Dev Engine)

## Run ID

- parallel-smoke (建议改成你自己的 run id,例如 demo / bugfix-2026-02-11)

## 目标(Objective)

- 验证并行 runtime 是否真的并发跑起来(>=2 个 runner 同时 running),并且能闭环收敛:
  - experiment.* -> review -> integration.* -> experiment.complete -> LOOP_COMPLETE

## 补充背景(Context, 可选)

- (可选) 当前现象/错误信息/日志关键片段(贴必要的即可)
- (可选) 你认为相关的模块/文件(如果不确定也没关系)

## 选择标准(Selection Criteria)

- 你必须先并行跑完本计划列出的并行自检实验(exp-par-001 与 exp-par-002),并且都通过审计:
  - `experiment.reviewed.evidence_ok=true`
- 如果两个实验结果都等价: 优先选择 `exp-par-001`(减少不确定性)

## 最终验收(Final Verification / 主工作区)

- 在主工作区集成后,需要跑的最终验收命令(必须真跑):
  - `rg -n "exp-par-00[12]" parallel_smoke_*.txt`

## 约束(Constraints, 可选)

- 你必须在第一批窗口里一次性发布两条 `experiment.task`(exp-par-001 与 exp-par-002),不要只发布一条.
- 在 exp-par-001 与 exp-par-002 都完成 `experiment.reviewed` 且 `evidence_ok=true` 之前,你不得进入 integration.
- 禁止 websearch/外部依赖.
- 每个实验只允许修改各自对应的 marker 文件,不要修改其他文件.
- 每个实验必须把改动压成 1 个 commit(便于 cherry-pick/审计/回滚).

## 实验任务(可选)

- (留空即可) 如果你不写实验任务条目,`ralph#1` 会先分析项目,再自动生成多条实验方案并派发给 runner.
- 推荐: 每个实验用一个 `###` 三级标题,并包含:
  - 实现(Implementation)
  - 验证(Verification)

> 小建议:
>
> - 如果你刚开始排查并行是否真的生效,建议先跑下面两条"并行自检"实验.
> - 这两条实验会产生 2-3 秒的持续 stdout+stderr:
>   - stdout: 验证多个 runner 输出交错.
>   - stderr: 验证灰色输出是否可见,以及 cassette 是否能录到 `stdout=false`.
> - 当你开始做真实任务时,删除这两条自检实验,再写你自己的 exp-001/exp-002.

### exp-par-001: parallel smoke (stdout+stderr interleaving)

#### 实现(Implementation)

1. 创建文件 `parallel_smoke_001.txt`,内容必须包含字符串: `exp-par-001`
2. 将改动提交为 1 个 commit(用命令级 git 身份,避免环境缺失导致 commit 失败):
   - `git add parallel_smoke_001.txt`
   - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-par-001: parallel smoke marker"`
3. 不要修改其他文件

#### 验证(Verification)

1. 持续输出(约 2-3 秒,用于观察 stdout+stderr 的交错输出):

   ```bash
   python3 - <<'PY'
import sys
import time
for i in range(8):
    print(f"exp-par-001 out tick {i}", flush=True)
    print(f"exp-par-001 err tick {i}", file=sys.stderr, flush=True)
    time.sleep(0.3)
PY
   ```

2. 文件校验:
   - `rg -n "exp-par-001" parallel_smoke_001.txt`
3. 产物校验:
   - `git show --name-only --oneline HEAD`

### exp-par-002: parallel smoke (stdout+stderr interleaving)

#### 实现(Implementation)

1. 创建文件 `parallel_smoke_002.txt`,内容必须包含字符串: `exp-par-002`
2. 将改动提交为 1 个 commit(用命令级 git 身份,避免环境缺失导致 commit 失败):
   - `git add parallel_smoke_002.txt`
   - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-par-002: parallel smoke marker"`
3. 不要修改其他文件

#### 验证(Verification)

1. 持续输出(约 2-3 秒,用于观察 stdout+stderr 的交错输出):

   ```bash
   python3 - <<'PY'
import sys
import time
for i in range(8):
    print(f"exp-par-002 out tick {i}", flush=True)
    print(f"exp-par-002 err tick {i}", file=sys.stderr, flush=True)
    time.sleep(0.3)
PY
   ```

2. 文件校验:
   - `rg -n "exp-par-002" parallel_smoke_002.txt`
3. 产物校验:
   - `git show --name-only --oneline HEAD`
