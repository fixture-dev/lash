# Physics & Collision System

@id: systems.physics
@status: in-progress
@labels: backend, physics, p0
@created: 2024-01-15

## Description

2D physics simulation including collision detection, forces, and platformer-specific physics. Uses AABB collision with tile-based level geometry for efficient broad-phase detection.

@agent-note: Slopes and moving platforms are p1 but heavily requested. Consider implementing before beta if time permits.

## Tasks

- [x] Implement collision detection
  - [x] AABB collision
  - [x] Tile-based collision
  - [x] Collision response
- [ ] Add physics simulation
  - [x] Velocity integration
  - [x] Gravity application
  - [ ] Friction and drag
- [ ] Implement platformer physics
  - [x] One-way platforms
  - [ ] Slopes and ramps #p1
  - [ ] Moving platforms #p1
- [ ] Add trigger zones
  - [ ] Trigger detection
  - [ ] Event callbacks
- [ ] Optimize physics performance
  - [ ] Spatial partitioning
  - [ ] Narrow-phase optimization
