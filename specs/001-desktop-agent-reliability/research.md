# Research: 桌面主Agent可靠性修复与测试框架

## 决策 1：发送消息必须走非阻塞执行链

- **Decision**: 将桌面主路径中的 LLM 请求执行改为非阻塞后台执行，并在前端建立明确 loading/disabled 状态。
- **Rationale**: 当前桌面窗口“未响应”属于 flow-blocking bug；只加 spinner 不能解决主线程被卡住的问题。
- **Alternatives considered**:
  - 仅在前端添加 loading：无法解决窗口假死。
  - 把阻塞调用塞进更长超时：只会放大问题。

## 决策 2：三方案输出改为 readiness 驱动

- **Decision**: 用结构化 `ConversationReadinessState` 判定是否可出方案。
- **Rationale**: 方案输出必须建立在“已经理解 enough + 用户确认总结正确”的前提上，不能被固定轮次驱动。
- **Alternatives considered**:
  - 增大轮次阈值：仍然是假判断。
  - 完全依赖模型自由发挥：缺少程序侧门禁，不可验证。

## 决策 3：桌面主流程复用既有 bootstrap/postcheck

- **Decision**: 方案确认后接入现有 orchestrator/bootstrap/postcheck，而不是只结束内存会话。
- **Rationale**: 当前兼容路径已经能稳定生成协议与 docs，桌面主路径应复用同一真源生成能力。
- **Alternatives considered**:
  - 桌面端手写简化落盘：会与兼容路径产物分叉。
  - 只生成提示不落盘：属于假成功。

## 决策 4：测试框架以 `tmp/` 工作区为核心

- **Decision**: 建立统一自动测试框架，所有临时工作区、夹具、模拟数据、产物快照统一进入 `tmp/`。
- **Rationale**: 既能覆盖真实落盘，又能避免污染正式目录。
- **Alternatives considered**:
  - 直接把测试产物写进仓库根目录：会污染正式工作区。
  - 只保留人工测试说明：无法满足自动回归要求。

## 决策 5：Speckit 计划使用仓库模板落地

- **Decision**: 在当前 Windows 环境下，Speckit 的 spec/plan/tasks 直接按仓库模板落盘到 `specs/`，并把 CLI `bash/sh` 缺口作为工具链问题记录。
- **Rationale**: 当前机器可以使用 Speckit 结构和模板，但默认脚本运行层不可用，不应因此放弃 spec-driven 开发。
- **Alternatives considered**:
  - 等 bash 环境补齐后再规划：会阻塞本轮修复。
