# Build Pipeline & CI/CD

@id: infra.build
@status: in-progress
@labels: tooling, devops, p1
@created: 2024-01-15

## Description

Continuous integration, automated testing, and release management. Uses GitHub Actions to build for multiple platforms and automate the release process.

@agent-note: Web (WASM) build is the primary target for early testing. Steam deployment requires Steamworks SDK integration.

## Tasks

- [x] Set up CI/CD
  - [x] GitHub Actions workflow
  - [x] Build matrix (platforms)
  - [x] Automated testing
- [ ] Configure build targets
  - [x] Web build (WASM)
  - [ ] Windows build
  - [ ] macOS build
  - [ ] Linux build
- [ ] Implement release automation
  - [ ] Version bumping
  - [ ] Changelog generation
  - [ ] Asset bundling
  - [ ] Platform packaging
- [ ] Add deployment pipeline
  - [ ] Deploy to itch.io #p1
  - [ ] Deploy to Steam #p2
