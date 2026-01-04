# Power-ups & Item System

@id: features.power-ups
@labels: backend, gameplay, p1
@created: 2024-01-15

## Description

Item collection system, power-up effects, and game balance. Power-ups provide temporary buffs while permanent upgrades unlock new abilities and progression paths.

@agent-note: Balance tuning should wait until core gameplay loop is stable. Stacking behavior needs careful design to prevent exploits.

## Tasks

- [x] Design item system architecture
  - Item definitions in assets/items.json
  - Effects implemented as component modifiers
  - [x] Item component structure
  - [x] Inventory management
  - [x] Effect application system
- [ ] Implement core power-ups
  - Temp power-ups have visual timer indicator on HUD
  - Invincibility sprite should flash at 1 second remaining
  - [x] Health restore
  - [x] Speed boost
  - [ ] Invincibility
  - [ ] Double damage
- [ ] Add permanent upgrades
  - Upgrades persist to save file immediately
  - Max health caps at 10 hearts (20 HP)
  - [ ] Max health increase
  - [ ] Jump height boost
  - [ ] Dash unlock
- [ ] Balance power-up effects
  - Speed boost: 1.5x movement speed, 8 second duration
  - Invincibility: 10 second duration, no stacking (refresh only)
  - Double damage: 15 second duration, stacks additively
  - [ ] Duration tuning
  - [ ] Spawn frequency
  - [ ] Stacking behavior
