# Enemy AI & Behavior

@id: features.enemy-ai
@labels: backend, ai, p0
@created: 2024-01-15

## Description

Enemy behavior systems including behavior trees, pathfinding, and difficulty scaling. Each enemy type uses the behavior tree system for consistent, extensible AI patterns.

@agent-note: The behavior tree system (task #enemy-behavior-trees) is a dependency for boss fights. Ensure tree evaluation is complete before starting boss AI work.

## Tasks

- [ ] Implement basic enemy types
  - Each enemy type defined in assets/enemies/*.json
  - Collision boxes should match visual sprite closely
  - [x] Walker enemy (patrols)
  - [x] Flyer enemy (aerial)
  - [ ] Shooter enemy (ranged)
  - [ ] Charger enemy (aggressive)
- [ ] Create behavior tree system @id: enemy-behavior-trees
  - Use existing crate: bevy_behavior for tree structure
  - Max tree depth: 5 nodes to prevent stack overflow
  - Tick rate: once per frame (not physics step)
  - [x] Behavior tree nodes
  - [x] Blackboard data structure
  - [ ] Tree evaluation
- [ ] Add pathfinding
  - A* with max 1000 node limit per search
  - Cache nav mesh per level, regenerate only on tile changes
  - Consider Jump Point Search for uniform cost grids
  - [ ] A* algorithm implementation #p1
  - [ ] Navigation mesh generation #p1
  - [ ] Dynamic obstacle avoidance #p2
- [ ] Implement difficulty scaling
  - Easy: 0.75x health, 0.8x damage, 1.2x reaction time
  - Normal: 1.0x all values (baseline)
  - Hard: 1.25x health, 1.2x damage, 0.8x reaction time
  - [ ] Enemy health scaling
  - [ ] Attack pattern variations
  - [ ] Spawn rate adjustments
