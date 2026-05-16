# Data Model: 目标软件定向初始化协作包

## DesktopSolutionFinalization

- `sessionId`
- `solutionId`
- `projectName`
- `targetClient`: `codex | claude-code`
- `selectedCapabilities`

## SelectedCapability

- `id`
- `label`
- `recommended`
- `selected`
- `status`: `pending | installed | fallback | skipped | failed`
- `detail`

## TargetClientProfile

- `id`
- `displayName`
- `entryFile`
- `agentDirectory`
- `skillDirectory`
- `configFiles`

## Session Persistence

Persist these fields into:

- `.commonhe/session/answers.json`
- `.commonhe/session/decision.json`
- `.commonhe/session/status.json`
- `.commonhe/session/generated-values.json`
