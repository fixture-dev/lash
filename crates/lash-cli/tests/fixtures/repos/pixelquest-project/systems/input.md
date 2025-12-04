# Input Handling System

@id: systems.input
@status: in-progress
@labels: backend, input, p0
@created: 2024-01-15

## Description

Controller mapping, input buffering, and multi-device support. The abstraction layer allows seamless switching between keyboard and gamepad.

@agent-note: Input buffering works with player movement's jump buffering - coordinate timing values between both systems.

## Tasks

- [x] Implement input abstraction
  - [x] Input action mapping
  - [x] Keyboard support
  - [x] Gamepad support
- [ ] Add input buffering
  - [ ] Action buffer queue
  - [ ] Buffer window tuning
- [ ] Implement rebindable controls
  - [ ] Control configuration UI #p1
  - [ ] Save/load bindings #p1
  - [ ] Default presets
- [ ] Add multi-device support
  - [ ] Device hot-swapping #p2
  - [-] Multiple gamepad support
