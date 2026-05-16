# CommonHE

中文 | [English](README.md)

CommonHE 是 `星星的vibecoding启动器` 的公开工程包。它不是普通业务脚手架，而是一个 Windows 桌面启动器，用来为目标项目生成面向 Codex 的接手协作包，并内置 200+ 顶级 Agent 角色库，用于方案设计和团队编排。

它会帮助用户完成模型渠道选择、目标工作区选择、需求澄清、方案对比、方案确认，并把后续交给 Codex 接手所需的协作文件落地到目标项目中。

## v1.0 发布包

v1.0 启动器包已放在仓库内：

```text
release/CommonHE-v1.0.zip
```

包内包含：

- `commonhe-desktop.exe`：Windows 桌面启动器
- `resources/commonhe/`：启动器运行所需的初始化运行时资源

## 它能做什么

- 作为本地 Windows 桌面应用运行
- 在主流程继续前校验渠道、模型、API Key 和 Base URL
- 让用户选择目标 workspace
- 通过内置 Agent 对话澄清项目方向
- 内置 200+ 顶级 Agent 角色，用于方案设计和团队编排
- 输出三套方案，并通过内置选择器完成方案确认
- 为选中的 workspace 生成面向 Codex 的协作包
- 在初始化完成前执行本地门禁和包检查

## 范围说明

CommonHE v1.0 聚焦生成 Codex 接手协作包。它不会生成业务应用源码、不会自动安装目标项目依赖，也不会替用户直接实现目标业务系统。

Claude Code 和 Gemini CLI 目标包不属于 v1.0 发布范围。

## 环境要求

直接运行启动器包：

- Windows 10/11 x64

从源码构建：

- Windows 10/11 x64
- Node.js 与 npm
- Rust/Cargo
- PowerShell 5+ 或 PowerShell 7+

## 本地开发

安装桌面端依赖：

```powershell
cd apps\desktop
npm ci
```

启动开发模式：

```powershell
npm run dev
```

运行前端和 TypeScript 检查：

```powershell
npm test
```

运行 Rust 编译检查：

```powershell
cd src-tauri
cargo test --lib --no-run
```

## 构建

在仓库根目录运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-desktop.ps1
```

构建脚本会先同步运行时资源到：

```text
apps/desktop/src-tauri/resources/commonhe/
```

随后构建前端和 Tauri 桌面执行文件。

## 验证

运行桌面 smoke 测试：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests\desktop-smoke.tests.ps1
```

生成启动器 zip 后运行发布包验证：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests\release-package.tests.ps1 -ReleaseZipPath release\CommonHE-v1.0.zip -ReleaseName CommonHE-v1.0
```

## 项目结构

```text
apps/desktop/       Tauri + React 桌面启动器
config/             运行时配置与能力清单
core/               CommonHE 运行时协议与门禁
init/               初始化流程定义
templates/          接手协作包模板
tools/              PowerShell 编排脚本与 truth-source 检查
.specify/           Spec Kit 运行时文件
specs/              当前功能规格与契约
agency-agents-zh/   方案生成所需的 Agent 角色目录
scripts/            构建、资源同步、便携包发布脚本
tests/              自动化验收脚本
release/            版本化启动器包
```

## 注意事项

- 启动器面向本地 workspace 和本地运行时资源工作。
- API Key 由用户本地输入或本地读取，不包含在本仓库中。
- 生成到目标项目里的协作包不提交到本仓库。
- 右上角 GitHub 与 QQ 联系入口属于产品 UI 内容，已保留。

## 许可证

MIT
