# 实验计划(Parallel Experimental Dev Engine)

## Run ID

- TODO: 本次运行的标识(例如 demo / bugfix-2026-02-11)

## 目标(Objective)

- TODO: 用一句话描述你要达成的最终状态

## 补充背景(Context, 可选)

- TODO: 当前现象/错误信息/日志关键片段(贴必要的即可)
- TODO: 你认为相关的模块/文件(如果不确定也没关系)

## 选择标准(Selection Criteria)

- TODO: 你要怎么决定"采纳哪个实验结果"
- 例: 风险最小 / 改动最少 / 性能最好 / 体验最佳

## 最终验收(Final Verification / 主工作区)

- TODO: 在主工作区集成后,需要跑哪些最终验收命令
- 例: `cargo test -p xxx`、`cargo clippy --all-targets --all-features -- -D warnings`

## 约束(Constraints, 可选)

- TODO: 不能改动的文件/模块(或明确允许改动的范围)
- TODO: 风险偏好(保守/激进)
- TODO: 时间预算(例如 30m/2h/1d)

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
   - `git add -A`
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
   - `git add -A`
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
