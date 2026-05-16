# Data Model: 双 Agent 语义验收闭环

## AgentSolution

- `roleRationale`: 角色选择理由。
- `omittedRoleRationale`: 明显候选角色暂不选择理由。

## SemanticReviewResult

- `passed`
- `blockingIssues`
- `questionsForMengXingxing`
- `requiredRepairs`
- `reviewSummary`
- `confidence`

## Session Artifacts

- `meng-xingxing-output.json`
- `xing-mengmeng-review.json`
- `agent-dialogue-rounds.jsonl`
- `repair-decisions.json`
- `final-acceptance.json`
