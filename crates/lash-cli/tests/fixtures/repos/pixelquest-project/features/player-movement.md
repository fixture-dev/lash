# Player Movement & Controls

@id: features.player-movement
@status: in-progress
@labels: backend, gameplay, p0
@created: 2024-01-15

## Description

Core player movement mechanics including physics, controls, animations, and special moves. The movement system prioritizes responsive controls and "game feel" with features like coyote time and jump buffering.

@agent-note: Movement polish tasks (coyote time, buffering) are lower priority but critical for player satisfaction. Consider these before release.

## Tasks

- [x] Implement basic physics
  - [x] Gravity and jumping
  - [x] Ground detection
  - [x] Velocity and acceleration
- [ ] Add movement controls
  - [x] Left/right movement
  - [x] Jump mechanics
  - [x] Double jump
  - [ ] Wall slide #p1
- [ ] Implement player animations
  - [x] Idle animation
  - [x] Walk cycle
  - [x] Jump animation
  - [ ] Fall animation
  - [ ] Land animation
- [ ] Add special moves
  - [ ] Dash ability #p1
  - [ ] Wall jump #p1
  - [ ] Ground pound #p2
- [ ] Polish movement feel
  - [ ] Coyote time
  - [ ] Jump buffering
  - [ ] Air control tuning
