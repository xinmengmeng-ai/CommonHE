# Template Path Rules

## 目标

定义 `CommonHE` 模板在任意项目中复制后应如何组织路径与引用。

## 根原则

所有模板都应以“目标项目根目录”为默认参考点，而不是以 `CommonHE` 当前存放目录为参考点。

## 规则

### 1. 根目录级入口

若宿主工具以项目根目录文件作为入口，模板应支持生成：

- `<project-root>/AGENTS.md`

### 2. 协作目录

若采用 Codex 风格协作目录，模板应支持生成：

- `<project-root>/.codex/COORDINATOR-SUBAGENTS.md`
- `<project-root>/.codex/agents/*.md`

### 3. 文档目录

模板应默认支持生成：

- `<project-root>/docs/project_context.md`
- `<project-root>/docs/architecture/01-项目架构设计书.md`
- `<project-root>/docs/roadmap/01-实施路线图.md`
- `<project-root>/docs/workflow/*.md`

### 4. 引用写法

模板内文档引用时，应优先使用“相对于目标项目根目录成立”的路径逻辑。

例如：

- 写 `docs/project_context.md`
- 写 `.codex/agents/backend.md`

而不是写：

- `HarnessEngineering/CommonHE/...`
- 当前实验仓库的绝对临时路径

## 模板设计要求

任何模板只有在被复制到新项目后仍能维持正确路径关系，才算合格模板。

