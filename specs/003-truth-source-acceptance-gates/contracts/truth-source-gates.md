# Contract: Truth Source Gates

## Command

```powershell
powershell -File tools/assert-commonhe-truth-source.ps1 -RepoRoot <repo-root>
powershell -File tools/assert-commonhe-truth-source.ps1 -GeneratedRoot <target> -TargetClient codex -AsJson
powershell -File tools/assert-commonhe-truth-source.ps1 -GeneratedRoot <target> -TargetClient claude-code -AsJson
```

## Output

```json
{
  "Passed": true,
  "Mode": "repo|generated",
  "TargetClient": "codex|claude-code",
  "Issues": []
}
```

## Postcheck Fields

```json
{
  "TruthSourceGatePassed": true,
  "TruthSourceGateIssues": []
}
```

## status.json Fields

```json
{
  "truth_source_gate_passed": true,
  "truth_source_gate_failed": false,
  "truth_source_gate_issues": []
}
```
