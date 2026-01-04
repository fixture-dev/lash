# Physics & Collision System

@id: systems.physics
@labels: backend, physics, p0
@created: 2024-01-15

## Description

2D physics simulation including collision detection, forces, and platformer-specific physics. Uses AABB collision with tile-based level geometry for efficient broad-phase detection.

@agent-note: Slopes and moving platforms are p1 but heavily requested. Consider implementing before beta if time permits.

## Tasks

- [x] Implement collision detection
  - Fixed timestep: 1/120 second (120Hz physics)
  - Sweep test for fast-moving objects (bullets, dash)
  - [x] AABB collision
  - [x] Tile-based collision
  - [x] Collision response
- [ ] Add physics simulation
  - Use semi-implicit Euler integration
  - Air friction coefficient: 0.98 (per frame)
  - Ground friction coefficient: 0.85 (per frame)
  - [x] Velocity integration
  - [x] Gravity application
  - [ ] Friction and drag
- [ ] Implement platformer physics
  - Slope max angle: 45 degrees (steeper = wall)
  - Moving platform sync: interpolate between physics steps
  - [x] One-way platforms
  - [ ] Slopes and ramps #p1
  - [ ] Moving platforms #p1
- [ ] Add trigger zones
  - Triggers don't affect physics, only fire callbacks
  - Support multiple overlapping triggers
  - [ ] Trigger detection
  - [ ] Event callbacks
- [ ] Optimize physics performance
  - Use quadtree with max 8 objects per node
  - Broad phase should handle 500+ entities at 60 FPS
  - [ ] Spatial partitioning
  - [ ] Narrow-phase optimization
