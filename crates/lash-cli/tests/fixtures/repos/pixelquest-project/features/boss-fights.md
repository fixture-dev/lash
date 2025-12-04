# Boss Encounters

@id: features.boss-fights
@status: in-progress
@labels: backend, gameplay, p1
@depends-on: features/enemy-ai.md#enemy-behavior-trees
@created: 2024-01-15

## Description

Special boss encounters with unique attack patterns, phases, and cinematics. Each world culminates in a boss fight that tests skills learned throughout the level.

@agent-note: Boss fights depend on the enemy AI behavior tree system. The World 1 boss is blocking the beta milestone - prioritize accordingly.

## Tasks

- [ ] Design boss framework
  - [x] Phase transition system
  - [x] Attack pattern scheduler
  - [ ] Vulnerability windows
- [ ] Implement World 1 boss @id: world-1-boss
  - [x] Forest guardian design
  - [ ] Three attack patterns
  - [ ] Environmental hazards
- [ ] Implement World 2 boss
  - [ ] Cave troll design
  - [ ] Rock throw pattern
  - [ ] Ground slam attack
- [ ] Add boss cinematics
  - [ ] Boss intro cutscene #p2
  - [ ] Victory celebration #p2
  - [ ] Defeat animation
