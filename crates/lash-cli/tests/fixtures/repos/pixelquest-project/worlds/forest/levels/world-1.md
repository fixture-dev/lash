# World 1: Enchanted Forest Levels

@id: worlds.forest.levels.world1
@created: 2024-01-20
@labels: content, levels, world1, p0
@status: in-progress
@depends-on: ../../../features/player-movement.md, ../../../systems/physics.md

## Description

The first world introduces core mechanics in a forgiving forest environment. Levels progress from simple platforming to more complex combinations of movement and combat.

@agent-note: Level 1-1 is the tutorial zone. Ensure all basic mechanics are naturally taught before Level 1-2's increased challenge.

## Tasks

- [x] Level 1-1: Forest Entrance
  - 32x24 tiles (2 screens), no enemies, purely platforming
  - Teaches: jump, move, interact
  - [x] Design introductory area layout
  - [x] Place basic platforming challenges
  - [x] Add collectible placement
  - [x] Create checkpoint positions
- [ ] Level 1-2: Treetop Canopy
  - 48x36 tiles (3 screens), introduces walker enemies
  - 2 hidden secrets (1 health upgrade, 5 collectibles)
  - [x] Design vertical climbing section
  - [x] Add swinging vine mechanics
  - [ ] Place hidden secrets #p2
  - [ ] Balance enemy placement
- [ ] Level 1-3: Forest Depths
  - 64x48 tiles (4 screens), introduces flyer enemies
  - First use of lighting effects (torches, glowing mushrooms)
  - Mini-boss: enhanced walker with 3x health
  - [ ] Create underground transition
  - [ ] Design water hazard sections
  - [ ] Add ambient lighting effects
  - [ ] Place mini-boss encounter
- [ ] Level 1-4: Ancient Tree Boss
  - Boss arena only (20x12 tiles), no platforming section
  - Post-boss: 30-second escape sequence with crumbling platforms
  - [ ] Design boss arena layout
  - [ ] Create escape sequence path
  - [ ] Add reward chest placement
