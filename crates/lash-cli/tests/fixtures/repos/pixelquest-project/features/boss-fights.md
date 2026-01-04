# Boss Encounters

@id: features.boss-fights
@labels: backend, gameplay, p1
@depends-on: features/enemy-ai.md#enemy-behavior-trees
@created: 2024-01-15

## Description

Special boss encounters with unique attack patterns, phases, and cinematics. Each world culminates in a boss fight that tests skills learned throughout the level.

@agent-note: Boss fights depend on the enemy AI behavior tree system. The World 1 boss is blocking the beta milestone - prioritize accordingly.

## Tasks

- [ ] Design boss framework
  - Phase transitions trigger at 75%, 50%, 25% HP thresholds
  - Vulnerability windows: 2-3 seconds after attack patterns
  - Boss arena locked during fight (no scrolling)
  - [x] Phase transition system
  - [x] Attack pattern scheduler
  - [ ] Vulnerability windows
- [ ] Implement World 1 boss @id: world-1-boss
  - Design doc: docs/bosses/forest-guardian.md
  - Health: 500 HP (Easy), 750 HP (Normal), 1000 HP (Hard)
  - Arena size: 20x12 tiles with destructible trees
  - [x] Forest guardian design
  - [ ] Three attack patterns
  - [ ] Environmental hazards
- [ ] Implement World 2 boss
  - Must be significantly harder than World 1 boss
  - Introduce multi-phase attack combos
  - [ ] Cave troll design
  - [ ] Rock throw pattern
  - [ ] Ground slam attack
- [ ] Add boss cinematics
  - Skip button after first viewing (accessibility)
  - Cinematics use in-game assets, not pre-rendered video
  - [ ] Boss intro cutscene #p2
  - [ ] Victory celebration #p2
  - [ ] Defeat animation
