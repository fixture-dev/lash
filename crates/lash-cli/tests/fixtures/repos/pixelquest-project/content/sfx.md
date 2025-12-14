# Sound Effects

@id: content.sfx
@status: in-progress
@labels: audio, sfx, p1
@created: 2024-01-15

## Description

Game sound effects for actions, UI, and ambience. Sounds are designed for a retro aesthetic with synthesized chiptune-style effects.

@agent-note: Player feedback sounds (jump, land, damage) are highest priority for game feel. Combat sounds should feel impactful.

## Tasks

- [ ] Create player sounds
  - All SFX under 0.5 seconds duration
  - Footsteps alternate between 2-3 variations
  - [x] Jump sound
  - [x] Land sound
  - [ ] Footstep sounds
  - [ ] Damage sound
  - [ ] Death sound
- [ ] Design combat sounds
  - Vary pitch +/- 5% for repeated sounds to avoid fatigue
  - Hit impact should layer with enemy-specific reaction sound
  - [ ] Sword swing
  - [ ] Hit impact
  - [ ] Enemy death
- [ ] Add item sounds
  - Ascending pitch for consecutive collectibles
  - Power-up sounds 2x longer than standard SFX (1 second)
  - [x] Coin collect
  - [ ] Power-up pickup
  - [ ] Health restore
- [ ] Create UI sounds
  - Keep UI sounds subtle (lower volume than gameplay)
  - Menu navigation sounds should be distinct but not annoying
  - [ ] Button click
  - [ ] Menu navigate
  - [ ] Pause/unpause
