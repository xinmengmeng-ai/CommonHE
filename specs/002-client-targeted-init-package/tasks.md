# Tasks: 目标软件定向初始化协作包

## Phase 1: Speckit & Contracts

- [x] Create `specs/002-client-targeted-init-package/`
- [x] Update `AGENTS.md` marker to this plan
- [x] Update truth-source docs for target client and initialization package wording

## Phase 2: Desktop Contract

- [x] Extend `chooseAgentSolution` with project name, target client, and selected capabilities
- [x] Add UI steps after solution selection
- [x] Add target client/capability tests

## Phase 3: Bootstrap Generation

- [x] Persist project name, target client, and selected capabilities into session seed
- [x] Add Claude Code templates
- [x] Make template generation conditional by target client
- [x] Make postcheck conditional by target client

## Phase 4: Verification

- [x] Test Codex generation path
- [x] Test Claude Code generation path
- [x] Run regression suite
