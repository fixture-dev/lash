# Audio Engine

@id: systems.audio
@labels: backend, audio, p1
@created: 2024-01-15

## Description

Sound engine for music playback, sound effects, and spatial audio. Supports OGG and WAV formats with a flexible mixer for layered audio.

@agent-note: Dynamic music layers are stretch goal - focus on crossfade transitions first for smooth level transitions.

## Tasks

- [x] Set up audio engine
  - Use Web Audio API for browser, rodio crate for desktop
  - Max simultaneous sounds: 32 channels
  - [x] Audio context initialization
  - [x] Asset loading (OGG, WAV)
  - [x] Audio mixer
- [ ] Implement music system
  - Crossfade duration: 2 seconds default (configurable)
  - Music files should be 128kbps OGG for balance of quality/size
  - [x] Background music playback
  - [ ] Crossfade transitions
  - [ ] Dynamic music layers #p2
- [ ] Add sound effects
  - Pool size: 8 instances per sound effect
  - SFX should be 16-bit WAV, max 1 second duration
  - [x] One-shot sound playback
  - [ ] Looping sounds
  - [ ] Sound pooling
- [ ] Implement spatial audio
  - Max hearing distance: 800 pixels
  - Linear falloff model (not inverse square)
  - [ ] Distance attenuation #p1
  - [ ] Stereo panning #p1
- [ ] Add volume controls
  - Persist settings in localStorage (web) or config file (desktop)
  - Default volumes: master 80%, music 70%, sfx 100%
  - [ ] Master volume
  - [ ] Music volume
  - [ ] SFX volume
