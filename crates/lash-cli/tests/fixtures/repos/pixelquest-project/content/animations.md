# Animation Sequences

@id: content.animations
@status: in-progress
@labels: art, animation, p1
@created: 2024-01-15

## Description

Character animations, frame sequences, and timing. Animations are frame-based with configurable timing to match the responsive feel of classic platformers.

@agent-note: Attack and death animations are higher priority than environmental animations. Particle effects can be implemented alongside the rendering pipeline particle system.

## Tasks

- [ ] Animate player actions
  - Frame timing: 83ms per frame (12 FPS)
  - Attack animation must sync with hitbox activation on frame 3
  - Death animation holds final frame for 0.5s before respawn
  - [x] Walk cycle (8 frames)
  - [x] Jump animation (4 frames)
  - [ ] Attack animation (6 frames)
  - [ ] Death animation (8 frames)
- [ ] Animate enemies
  - Enemy animations loop seamlessly (last frame connects to first)
  - Attack windup should telegraph for player reaction time
  - [x] Walker patrol cycle
  - [ ] Flyer flight cycle
  - [ ] Attack animations
- [ ] Create environmental animations
  - Use GPU shader animations where possible to save memory
  - Keep environmental animations subtle (2-4 pixels movement)
  - [ ] Water ripple effect #p2
  - [ ] Torch flame #p2
  - [-] Grass sway
- [ ] Add particle effects
  - Max 50 particles per effect instance
  - Particles use additive blending for glow effects
  - [ ] Dust particles
  - [ ] Hit sparks
  - [ ] Power-up glow
