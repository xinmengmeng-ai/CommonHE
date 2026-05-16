# 必需能力清单

## 当前状态

{{capability_gate_status}}

## 必须存在的能力

{{required_capabilities_list}}

## 探测摘要

{{capability_probe_summary}}

## 本轮选择记录

{{selected_capabilities_summary}}

## 能力适用范围

{{capability_scope_notes}}

## 缺失时如何处理

- 任一能力缺失时，不得宣布初始化成功
- 任一能力缺失时，当前线程只允许修复依赖，不得进入业务实施
- 依赖修复完成后，先重新执行 `doctor` / `precheck`，再继续后续流程
