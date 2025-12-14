# Asset Processing Pipeline

@id: infra.assets
@status: in-progress
@labels: tooling, assets, p1
@created: 2024-01-15

## Description

Automated asset import, optimization, and conversion. Converts source assets into optimized formats for each build target.

@agent-note: Texture atlases are generated from sprite sheets automatically. Audio should be normalized before OGG conversion.

## Tasks

- [ ] Set up asset importer
  - Source assets in assets/src/, output to assets/dist/
  - Use imagemagick for image conversions
  - [x] Image format conversion
  - [x] Sprite sheet packing
  - [ ] Audio format conversion
- [ ] Implement texture optimization
  - Use pngquant for lossy compression (quality 80-90)
  - Atlas max size: 4096x4096, with 2px padding between sprites
  - [x] PNG compression
  - [ ] Texture atlas generation
  - [-] Mipmap generation
- [ ] Add audio processing
  - Target loudness: -16 LUFS for music, -12 LUFS for SFX
  - OGG quality 6 (128kbps equivalent)
  - Use ffmpeg for audio pipeline
  - [ ] Audio normalization
  - [ ] Format conversion (OGG)
  - [ ] Compression settings
- [ ] Create asset validation
  - Required assets defined in assets/manifest.json
  - Metadata schema in assets/schema.json
  - [ ] Check missing assets
  - [ ] Validate asset metadata
  - [ ] Detect unused assets #p2
