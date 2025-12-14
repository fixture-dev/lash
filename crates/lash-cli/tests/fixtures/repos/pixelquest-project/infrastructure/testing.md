# Testing Framework

@id: infra.testing
@status: in-progress
@labels: testing, qa, p1
@created: 2024-01-15

## Description

Unit tests, integration tests, and automated playtesting. Test coverage focuses on physics, AI, and input systems to ensure consistent game feel.

@agent-note: Physics tests are complete - use as template for AI behavior tests. Automated playthrough is a stretch goal for regression testing.

## Tasks

- [x] Set up test framework
  - Use cargo test for unit/integration tests
  - Coverage tracked with cargo-tarpaulin
  - Target coverage: 80% for core systems
  - [x] Unit test infrastructure
  - [x] Integration test setup
  - [x] Test coverage reporting
- [ ] Write unit tests
  - Physics tests in tests/physics/, use as template
  - Mock time/input for deterministic AI tests
  - [x] Physics tests
  - [ ] AI behavior tests
  - [ ] Input handling tests
- [ ] Create integration tests
  - Test levels in tests/fixtures/levels/
  - Save files are JSON, test round-trip serialization
  - [ ] Level loading tests
  - [ ] Save/load tests
  - [ ] Combat system tests
- [ ] Implement playtesting automation
  - Replay format: msgpack-serialized input frames
  - Performance target: maintain 60 FPS for 30+ minutes
  - [ ] Replay recording #p2
  - [ ] Automated playthrough #p2
  - [ ] Performance profiling
