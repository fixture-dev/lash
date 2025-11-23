# Enemy AI & Behavior

@id: features.enemy-ai
@status: in-progress
@labels: backend, ai, p0
@created: 2024-01-15

Enemy behavior systems including behavior trees, pathfinding, and difficulty scaling.

## Tasks

- [ ] Implement basic enemy types
  - [x] Walker enemy (patrols)
  - [x] Flyer enemy (aerial)
  - [ ] Shooter enemy (ranged)
  - [ ] Charger enemy (aggressive)
- [ ] Create behavior tree system @id: enemy-behavior-trees
  - [x] Behavior tree nodes
  - [x] Blackboard data structure
  - [ ] Tree evaluation
- [ ] Add pathfinding
  - [ ] A* algorithm implementation #p1
  - [ ] Navigation mesh generation #p1
  - [ ] Dynamic obstacle avoidance #p2
- [ ] Implement difficulty scaling
  - [ ] Enemy health scaling
  - [ ] Attack pattern variations
  - [ ] Spawn rate adjustments
