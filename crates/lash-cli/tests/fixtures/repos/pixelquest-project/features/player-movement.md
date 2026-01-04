# Player Movement & Controls

@id: features.player-movement
@labels: backend, gameplay, p0
@created: 2024-01-15

## Description

Core player movement mechanics including physics, controls, animations, and special moves. The movement system prioritizes responsive controls and "game feel" with features like coyote time and jump buffering.

@agent-note: Movement polish tasks (coyote time, buffering) are lower priority but critical for player satisfaction. Consider these before release.

## Tasks

- [x] Implement basic physics
  - Gravity constant: 980 pixels/sec^2 (adjustable in config)
  - Terminal velocity capped at 600 pixels/sec
  - [x] Gravity and jumping
  - [x] Ground detection
  - [x] Velocity and acceleration
- [ ] Add movement controls
  - Input polling at 60Hz minimum for responsive feel
  - Acceleration ramp-up: 0.15 seconds to max speed
  - [x] Left/right movement
  - [x] Jump mechanics
  - [x] Double jump
  - [ ] Wall slide #p1
- [ ] Implement player animations
  - All animations at 12 FPS for retro pixel art style
  - Use sprite sheet at assets/sprites/player.png
  - [x] Idle animation
  - [x] Walk cycle
  - [x] Jump animation
  - [ ] Fall animation
  - [ ] Land animation
- [ ] Add special moves
  - Dash should be 8 frames (0.13 seconds) with i-frames
  - Wall jump angle: 45 degrees from wall normal
  - Ground pound cancels horizontal momentum
  - [ ] Dash ability #p1
  - [ ] Wall jump #p1
  - [ ] Ground pound #p2
- [ ] Polish movement feel
  - Coyote time window: 6 frames (100ms)
  - Jump buffer window: 4 frames (67ms)
  - Reference: Celeste movement feel GDC talk
  - [ ] Coyote time
  - [ ] Jump buffering
  - [ ] Air control tuning
