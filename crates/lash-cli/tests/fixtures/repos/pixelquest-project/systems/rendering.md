# Graphics & Rendering Pipeline

@id: systems.rendering
@status: in-progress
@labels: backend, rendering, p0
@created: 2024-01-15

2D graphics rendering including sprite batching, camera systems, and shader effects.

## Tasks

- [x] Set up rendering pipeline
  - [x] OpenGL/WebGL context
  - [x] Shader compilation
  - [x] Texture loading
- [ ] Implement sprite batching
  - [x] Batch renderer design
  - [x] Texture atlas support
  - [ ] Z-ordering/sorting
- [ ] Add camera system
  - [x] Camera follow player
  - [ ] Smooth camera movement
  - [ ] Camera shake effects
  - [ ] Zoom controls #p2
- [ ] Implement visual effects
  - [ ] Particle system #p1
  - [ ] Screen transitions
  - [ ] Post-processing shaders #p2
- [ ] Optimize rendering performance
  - [ ] Frustum culling
  - [-] Occlusion culling
  - [ ] Render batching
