# Tasks: 桌面主Agent可靠性修复与测试框架

**Input**: Design documents from `/specs/001-desktop-agent-reliability/`  
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: 本 feature 明确要求先建立自动测试框架，因此测试任务是必选项。  
**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: 建立 Speckit feature、计划引用和测试框架入口约束

- [x] T001 Create `specs/001-desktop-agent-reliability/` feature artifacts and `.specify/feature.json`
- [x] T002 Update `AGENTS.md` Speckit markers and repo guidance to require Speckit-first planning plus `docs/` vs `tmp/` conventions
- [x] T003 [P] Add truth-source documentation updates in `docs/CommonHE开发目标.md` and `init/init-flow.md` for Speckit usage and testing framework rules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: 为桌面主路径建立可测试的状态机和 bootstrap 接口抽象

- [x] T004 Add desktop main-flow helper/state modules in `apps/desktop/src/agentFlow.ts` and related files for readiness + loading rules
- [x] T005 [P] Extend Tauri session contracts in `apps/desktop/src/tauriApi.ts` and `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs`
- [x] T006 [P] Introduce bootstrap result bridge in `apps/desktop/src-tauri/src/commonhe_bridge/commands.rs` and related backend modules
- [x] T007 Add foundational tests for readiness and bootstrap contracts in `apps/desktop/src/appState.test.ts` and Rust tests

**Checkpoint**: 前后端已经拥有稳定的 readiness / bootstrap 合同，用户故事可以继续。

---

## Phase 3: User Story 1 - 稳定和星星自然对话 (Priority: P1) 🎯 MVP

**Goal**: 修复对话命名错误和发送时假死问题  
**Independent Test**: 进入对话页后能看到 `星星` 命名；点击发送后立即出现 loading，窗口保持可响应

### Tests for User Story 1

- [x] T008 [P] [US1] Add UI naming/loading logic tests in `apps/desktop/src/appState.test.ts`
- [x] T009 [P] [US1] Add backend non-blocking session tests in `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs`

### Implementation for User Story 1

- [x] T010 [US1] Rename visible conversation copy from `主Agent` to `星星` in `apps/desktop/src/App.tsx`
- [x] T011 [US1] Add explicit send loading state and disabled submit behavior in `apps/desktop/src/App.tsx`
- [x] T012 [US1] Refactor blocking agent execution path to non-blocking backend flow in `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs`
- [x] T013 [US1] Update styles for loading and disabled interaction feedback in `apps/desktop/src/styles.css`

---

## Phase 4: User Story 2 - 在真正理解需求后再出方案 (Priority: P1)

**Goal**: 用 readiness 机制替代固定轮次阈值  
**Independent Test**: 信息不足时持续追问；只有在关键信息齐备且用户确认后才出三方案

### Tests for User Story 2

- [x] T014 [P] [US2] Add readiness decision tests in `apps/desktop/src/appState.test.ts`
- [x] T015 [P] [US2] Add Rust session progression tests in `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs`

### Implementation for User Story 2

- [x] T016 [US2] Add `ConversationReadinessState` tracking to `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs`
- [x] T017 [US2] Update prompt/response handling to require summary confirmation before solutions in `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs`
- [x] T018 [US2] Surface readiness and current missing information in `apps/desktop/src/tauriApi.ts` and `apps/desktop/src/App.tsx`

---

## Phase 5: User Story 3 - 选定方案后必须生成真实初始化结果 (Priority: P1)

**Goal**: 确认方案后真实落盘并执行 postcheck  
**Independent Test**: 在 `tmp/` 工作区完成一次完整初始化后，工作区存在初始化产物且成功/失败状态真实可信

### Tests for User Story 3

- [x] T019 [P] [US3] Add backend bootstrap result tests in `apps/desktop/src-tauri/src/commonhe_bridge/agent.rs` and related modules
- [x] T020 [P] [US3] Add PowerShell integration tests for workspace generation in `tests/desktop-main-flow.tests.ps1`

### Implementation for User Story 3

- [x] T021 [US3] Bridge solution confirmation to orchestrator bootstrap/postcheck in `apps/desktop/src-tauri/src/commonhe_bridge/commands.rs` and supporting modules
- [x] T022 [US3] Extend desktop frontend to display real bootstrap/postcheck progress and results in `apps/desktop/src/App.tsx`
- [x] T023 [US3] Block success handoff unless target workspace has generated files and postcheck passes in frontend and backend state handling

---

## Phase 6: User Story 4 - 有内建测试框架而不是依赖人工兜底 (Priority: P2)

**Goal**: 建立 `tmp/` 测试夹具和统一自动测试框架  
**Independent Test**: 运行统一测试入口即可自动完成主流程回归并将全部测试数据限制在 `tmp/`

### Tests for User Story 4

- [x] T024 [P] [US4] Add fixture validation checks in `tests/desktop-main-flow.tests.ps1`
- [x] T025 [P] [US4] Add documentation path guard tests in `tests/desktop-smoke.tests.ps1`

### Implementation for User Story 4

- [x] T026 [US4] Create reusable temporary workspace harness under `tmp/desktop-main-flow/` and `tests/desktop-main-flow.tests.ps1`
- [x] T027 [US4] Document `docs/` vs `tmp/` truth-source/testing boundaries in `docs/CommonHE开发目标.md`, `README.md`, and `用户使用手册.md`
- [x] T028 [US4] Update build/smoke/test scripts as needed to include the new desktop main-flow framework

---

## Phase 7: Polish & Cross-Cutting Concerns

- [x] T029 [P] Refresh manual testing guidance in `MANUAL-TEST.md` to reflect the automated framework-first workflow
- [x] T030 Run full verification suite (`npm test`, `npm run build`, `cargo test`, PowerShell tests, desktop build) and record results in docs or test output

## Dependencies & Execution Order

- Phase 1 → Phase 2 → US1/US2/US3/US4 → Polish
- US1, US2, US3 are all flow-blocking; US4 can overlap after Phase 2 but must complete before final handoff
- US3 depends on Phase 2 contracts and on some readiness/session work from US2

## Implementation Strategy

### MVP First

1. Finish Setup + Foundational
2. Finish US1
3. Finish US2
4. Finish US3
5. Validate a real generated workspace in `tmp/`

### Final Hardening

1. Add US4 testing framework and docs rules
2. Run full regression and build validation
