# Contract: Desktop Init Package Finalization

## `choose_agent_solution`

### Input

- `sessionId: string`
- `solutionId: string`
- `projectName: string`
- `targetClient: codex | claude-code`
- `selectedCapabilities: SelectedCapability[]`

### Output

- Updated `AgentSessionSnapshot`
- `bootstrapResult` on success or structured failure

### Guarantees

- Empty or invalid `projectName` blocks bootstrap.
- Unsupported `targetClient` blocks bootstrap.
- `projectName`, `targetClient`, and `selectedCapabilities` are persisted before bootstrap.
- Success requires generated files and target-client-aware postcheck.
