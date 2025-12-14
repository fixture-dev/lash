# Procedural Level Generation

@id: features.level-gen
@status: in-progress
@labels: backend, worldgen, p1
@created: 2024-01-15

## Description

Procedural level generation algorithms and tile placement systems. Uses room-based generation with corridor connections to create varied layouts while ensuring all areas remain reachable.

@agent-note: Reachability checks are p0 priority - generated levels must be completable. Seed-based regeneration allows players to share level codes.

## Tasks

- [x] Design level generation algorithm
  - Use Binary Space Partitioning for initial room layout
  - Min room size: 8x6 tiles, max room size: 20x15 tiles
  - [x] Room-based generation
  - [x] Corridor connections
  - [x] Critical path analysis
- [ ] Implement tile placement
  - Hazard density: 5-15% of non-platform tiles per room
  - Collectibles placed using Poisson disc sampling
  - [x] Platform placement rules
  - [ ] Hazard distribution
  - [ ] Collectible placement
- [ ] Add biome variety
  - Each biome has unique tileset in assets/tiles/{biome}/
  - Biome affects enemy spawns and hazard types
  - [ ] Forest biome
  - [ ] Cave biome
  - [ ] Sky biome
  - [ ] Lava biome
- [ ] Validate level playability
  - Use flood fill from spawn to all required objectives
  - Max player jump: 4 tiles horizontal, 3 tiles vertical
  - Seeds are u64 displayed as 12-character alphanumeric code
  - [ ] Reachability checks #p0
  - [ ] Difficulty estimation #p1
  - [ ] Seed-based regeneration
