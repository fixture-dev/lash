# Forest Guardian Boss

@id: worlds.forest.boss
@created: 2024-01-22
@labels: content, boss, world1, p0
@status: in-progress
@depends-on: levels/world-1.md, ../../../features/boss-fights.md, ../../../features/enemy-ai.md

## Description

The Forest Guardian is the first boss encounter, teaching players to recognize attack patterns and find vulnerability windows. The arena includes environmental hazards that can be used strategically.

@agent-note: This boss is a key dependency for the beta milestone. Three-phase design with increasing complexity.

## Tasks

- [ ] Design boss mechanics
  - Phase 1: Single attack pattern, 6-second cooldown
  - Phase 2: Two attack combo, falling leaves hazard
  - Phase 3: Fast attacks, arena shrinks
  - [x] Define attack patterns
  - [x] Create phase transitions
  - [ ] Balance health and damage
  - [ ] Add vulnerability windows
- [ ] Create boss animations
  - Boss sprite: 64x64 pixels (4x player size)
  - All animations need telegraph frames (0.5 second windup)
  - [ ] Idle animation cycle
  - [ ] Attack animations
  - [ ] Damage reaction sprites
  - [ ] Death sequence
- [ ] Implement boss arena
  - Arena: 20x12 tiles, locked camera
  - Trees on edges are destructible for power-up drops
  - Escape route opens after boss defeat (5 second timer)
  - [ ] Design arena hazards
  - [ ] Add destructible elements
  - [ ] Create escape route trigger
