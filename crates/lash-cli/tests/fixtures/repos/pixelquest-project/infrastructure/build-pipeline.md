# Build Pipeline & CI/CD

@id: infra.build
@labels: tooling, devops, p1
@created: 2024-01-15

## Description

Continuous integration, automated testing, and release management. Uses GitHub Actions to build for multiple platforms and automate the release process.

@agent-note: Web (WASM) build is the primary target for early testing. Steam deployment requires Steamworks SDK integration.

## Tasks

- [x] Set up CI/CD
  - Workflow file: .github/workflows/build.yml
  - Build triggered on push to main and PRs
  - [x] GitHub Actions workflow
  - [x] Build matrix (platforms)
  - [x] Automated testing
- [ ] Configure build targets
  - WASM uses wasm-pack with target web
  - Windows requires MSVC toolchain (not MinGW)
  - macOS builds require code signing for distribution
  - [x] Web build (WASM)
  - [ ] Windows build
  - [ ] macOS build
  - [ ] Linux build
- [ ] Implement release automation
  - Use cargo-release for version management
  - Changelog format: Keep a Changelog (keepachangelog.com)
  - Assets bundled as single ZIP per platform
  - [ ] Version bumping
  - [ ] Changelog generation
  - [ ] Asset bundling
  - [ ] Platform packaging
- [ ] Add deployment pipeline
  - itch.io uses butler CLI for uploads
  - Steam deployment requires Steamworks SDK license
  - [ ] Deploy to itch.io #p1
  - [ ] Deploy to Steam #p2
