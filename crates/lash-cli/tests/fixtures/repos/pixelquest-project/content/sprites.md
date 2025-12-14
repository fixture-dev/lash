# Sprite Art

@id: content.sprites
@status: in-progress
@labels: art, sprites, p0
@created: 2024-01-15

## Description

Character sprites, tile sets, UI elements, and visual assets. All sprites use a consistent 16x16 pixel grid with a limited retro color palette.

@agent-note: Tilesets should be completed before level design tasks. Enemy sprites should match their AI behavior patterns.

## Tasks

- [ ] Create player sprites
  - Base size: 16x24 pixels (taller than standard tile)
  - Export at 1x, 2x, and 4x scales for different resolutions
  - [x] Player idle sprite
  - [x] Player walk frames
  - [x] Player jump sprite
  - [ ] Player attack sprite
- [ ] Design enemy sprites
  - Use consistent 16x16 bounding box for collision
  - Enemy palette should contrast with environment
  - [x] Walker enemy sprite
  - [x] Flyer enemy sprite
  - [ ] Shooter enemy sprite
  - [ ] Charger enemy sprite
- [ ] Create tile sets
  - Each tileset: 256 tiles in 16x16 grid
  - Include auto-tile variants for terrain edges
  - Animated tiles (water, lava) need 4-frame loops
  - [x] Forest tileset
  - [ ] Cave tileset
  - [ ] Sky tileset
  - [ ] Lava tileset
- [ ] Design UI elements
  - UI at 2x native resolution (32px icons)
  - Follow 8-color palette restriction for UI consistency
  - [x] Health bar
  - [ ] Menu buttons
  - [ ] Inventory icons
