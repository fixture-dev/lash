# Input Handling System

@id: systems.input
@labels: backend, input, p0
@created: 2024-01-15

## Description

Controller mapping, input buffering, and multi-device support. The abstraction layer allows seamless switching between keyboard and gamepad.

@agent-note: Input buffering works with player movement's jump buffering - coordinate timing values between both systems.

## Tasks

- [x] Implement input abstraction
  - Actions: jump, attack, dash, interact, pause, menu_up/down/left/right
  - Use gilrs crate for cross-platform gamepad support
  - [x] Input action mapping
  - [x] Keyboard support
  - [x] Gamepad support
- [ ] Add input buffering
  - Buffer window synced with player movement (67ms default)
  - Queue max length: 3 actions
  - Clear buffer on action consumption
  - [ ] Action buffer queue
  - [ ] Buffer window tuning
- [ ] Implement rebindable controls
  - Store bindings as JSON in user config directory
  - Detect conflicts when rebinding (warn, don't prevent)
  - [ ] Control configuration UI #p1
  - [ ] Save/load bindings #p1
  - [ ] Default presets
- [ ] Add multi-device support
  - Auto-detect most recently used device
  - Show appropriate button prompts (keyboard vs gamepad icons)
  - [ ] Device hot-swapping #p2
  - [-] Multiple gamepad support
