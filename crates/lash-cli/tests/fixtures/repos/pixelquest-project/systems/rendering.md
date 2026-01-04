# Graphics & Rendering Pipeline

@id: systems.rendering
@labels: backend, rendering, p0
@created: 2024-01-15

## Description

2D graphics rendering including sprite batching, camera systems, and shader effects. The pipeline supports WebGL for browser deployment and native OpenGL for desktop.

@agent-note: Sprite batching is critical for performance - aim for single draw call per layer. Camera shake adds impact to combat and explosions.

## Tasks

- [x] Set up rendering pipeline
  - WebGL 2.0 required (95%+ browser support)
  - Desktop uses OpenGL 3.3 core profile
  - [x] OpenGL/WebGL context
  - [x] Shader compilation
  - [x] Texture loading
- [ ] Implement sprite batching
  - Target: <10 draw calls per frame for main gameplay
  - Texture atlas max size: 4096x4096 (WebGL limit)
  - [x] Batch renderer design
  - [x] Texture atlas support
  - [ ] Z-ordering/sorting
- [ ] Add camera system
  - Smooth follow uses lerp with 0.1 smoothing factor
  - Camera shake: frequency 30Hz, max amplitude 8 pixels
  - Dead zone: 32 pixels around player before camera moves
  - [x] Camera follow player
  - [ ] Smooth camera movement
  - [ ] Camera shake effects
  - [ ] Zoom controls #p2
- [ ] Implement visual effects
  - Particle budget: max 500 particles on screen
  - Use object pooling for particle instances
  - [ ] Particle system #p1
  - [ ] Screen transitions
  - [ ] Post-processing shaders #p2
- [ ] Optimize rendering performance
  - Target: 60 FPS on Intel HD 4000 (minimum spec)
  - Profile using RenderDoc for desktop, Spector.js for web
  - [ ] Frustum culling
  - [-] Occlusion culling
  - [ ] Render batching
