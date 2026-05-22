# CommonHE

[中文](README.zh-CN.md) | English

CommonHE is the public engineering package for `Xingxing's vibecoding launcher`. It is not a general-purpose business scaffold. It is a Windows desktop launcher that generates a Codex handoff collaboration package for a target project, with a built-in catalog of 200+ top-tier agent roles for solution planning and team composition.

It helps users select a model provider, choose a target workspace, clarify requirements, compare implementation options, confirm a solution, and write the collaboration files Codex needs to continue the project.

## v1.0.1 Release Package

The v1.0.1 launcher package is included in this repository:

```text
release/CommonHE-v1.0.1.zip
```

The package contains:

- `commonhe-desktop.exe`: the Windows desktop launcher
- `resources/commonhe/`: the runtime resources required by the launcher

## What It Does

- Runs as a local Windows desktop application
- Validates provider, model, API key, and base URL before the main flow continues
- Lets the user choose the target workspace
- Clarifies project direction through the built-in agent conversation
- Includes 200+ built-in top-tier agent roles for solution planning and team composition
- Produces three solution options and confirms the final choice through the built-in selector
- Generates a Codex-oriented collaboration package for the selected workspace
- Runs local gates and package checks before initialization is considered complete

## Scope

CommonHE v1.0.1 focuses on generating Codex handoff collaboration packages. It does not generate business application source code, automatically install dependencies for the target project, or directly implement the user's business system.

Claude Code and Gemini CLI target packages are outside the v1.0.1 release scope.

## Requirements

To run the packaged launcher:

- Windows 10/11 x64

To build from source:

- Windows 10/11 x64
- Node.js and npm
- Rust/Cargo
- PowerShell 5+ or PowerShell 7+

## Local Development

Install desktop dependencies:

```powershell
cd apps\desktop
npm ci
```

Start development mode:

```powershell
npm run dev
```

Run frontend and TypeScript checks:

```powershell
npm test
```

Run Rust compile checks:

```powershell
cd src-tauri
cargo test --lib --no-run
```

## Build

Run from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-desktop.ps1
```

The build script first syncs runtime resources into:

```text
apps/desktop/src-tauri/resources/commonhe/
```

Then it builds the frontend and the Tauri desktop executable.

## Verification

Run the desktop smoke test:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests\desktop-smoke.tests.ps1
```

After generating the launcher zip, run release-package verification:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests\release-package.tests.ps1 -ReleaseZipPath release\CommonHE-v1.0.1.zip -ReleaseName CommonHE-v1.0.1
```

## Project Structure

```text
apps/desktop/       Tauri + React desktop launcher
config/             Runtime configuration and capability lists
core/               CommonHE runtime protocol and gates
init/               Initialization flow definitions
templates/          Handoff collaboration package templates
tools/              PowerShell orchestration scripts and truth-source checks
.specify/           Spec Kit runtime files
specs/              Current feature specifications and contracts
agency-agents-zh/   Agent role catalog used during solution generation
scripts/            Build, resource-sync, and portable-package scripts
tests/              Automated acceptance scripts
release/            Versioned launcher packages
```

## Notes

- The launcher works with local workspaces and local runtime resources.
- API keys are provided or loaded locally by the user and are not included in this repository.
- Generated target-project collaboration packages are not committed to this repository.
- The GitHub and QQ entry points in the upper-right corner are part of the product UI and are intentionally preserved.

## License

MIT
