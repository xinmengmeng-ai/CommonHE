# Required Capabilities

## Gate Status

{{capability_gate_status}}

## Required Capabilities

{{required_capabilities_list}}

## Probe Summary

{{capability_probe_summary}}

## Selected In This Init Package

{{selected_capabilities_summary}}

## Runtime Rule

- 若本文件不是绿色状态，主控不得派工
- 若 session 记录显示 `doctor_failed`，当前线程只能修复环境或入口
- 若 session 记录显示 `precheck_failed`，当前线程只能修复依赖
- 任一能力缺失时，不得进入业务实施

## Browser Pair

- `agent-browser` 负责流程自动化与交互执行
- `chrome-devtools` 负责网络、控制台、DOM 与性能诊断
- 浏览器相关任务默认需要两项能力同时可用
