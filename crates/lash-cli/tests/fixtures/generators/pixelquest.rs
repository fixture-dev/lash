//! `PixelQuest` 2D Platformer - Realistic Game Development Fixture
//!
//! This module generates a comprehensive demo project for a fictional 2D platformer
//! game to showcase all of Lash's features in an engaging, realistic context.
//!
//! The project includes:
//! - 40-50 task files across game development areas
//! - Realistic game development tasks (features, systems, content, infrastructure)
//! - Rich cross-file dependencies
//! - Varied task statuses (open, done, waived)
//! - Comprehensive labels and annotations

use super::ProjectGenerator;
use std::path::Path;

/// Generate the `PixelQuest` 2D platformer demo project
///
/// Creates ~45 task files demonstrating:
/// - Game features (player movement, enemy AI, level generation, power-ups, boss fights)
/// - Core systems (rendering, audio, physics, input)
/// - Content creation (sprites, animations, music, sfx, levels)
/// - Infrastructure (build pipeline, asset pipeline, testing)
/// - Design documents (core loop, progression, story)
/// - Milestones (alpha, beta, release)
///
/// # Errors
///
/// Returns error if files cannot be written
pub fn generate_pixelquest_project(output_dir: &Path) -> std::io::Result<()> {
    let generator = ProjectGenerator::new("pixelquest", "PixelQuest: Retro 2D Platformer")
        .with_labels(vec!["game".into(), "platformer".into(), "demo".into()])
        .with_base_date("2024-08-01");

    // Build project incrementally by module
    let gen = add_features_module(generator);
    let gen = add_systems_module(gen);
    let gen = add_content_module(gen);
    let gen = add_infrastructure_module(gen);
    let gen = add_design_module(gen);
    let gen = add_milestones_module(gen);

    gen.generate_to(output_dir)
}

/// Add game features module (5 files)
fn add_features_module(gen: ProjectGenerator) -> ProjectGenerator {
    let gen = add_player_movement(gen);
    let gen = add_enemy_ai(gen);
    let gen = add_level_generation(gen);
    let gen = add_power_ups(gen);

    add_boss_fights(gen)
}

/// Player movement and controls
fn add_player_movement(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "features/player-movement.md",
        "features.player-movement",
        "Player Movement & Controls",
    )
    .with_labels(vec!["backend".into(), "gameplay".into(), "p0".into()])
    .with_description(
        "Core player movement mechanics including physics, controls, animations, and special moves.",
    )
    .add_task("Implement basic physics")
    .done()
    .add_subtask("Gravity and jumping", 'x', vec![])
    .add_subtask("Ground detection", 'x', vec![])
    .add_subtask("Velocity and acceleration", 'x', vec![])
    .end_task()
    .add_task("Add movement controls")
    .add_subtask("Left/right movement", 'x', vec![])
    .add_subtask("Jump mechanics", 'x', vec![])
    .add_subtask("Double jump", 'x', vec![])
    .add_subtask("Wall slide", ' ', vec!["p1".into()])
    .end_task()
    .add_task("Implement player animations")
    .add_subtask("Idle animation", 'x', vec![])
    .add_subtask("Walk cycle", 'x', vec![])
    .add_subtask("Jump animation", 'x', vec![])
    .add_subtask("Fall animation", ' ', vec![])
    .add_subtask("Land animation", ' ', vec![])
    .end_task()
    .add_task("Add special moves")
    .add_subtask("Dash ability", ' ', vec!["p1".into()])
    .add_subtask("Wall jump", ' ', vec!["p1".into()])
    .add_subtask("Ground pound", ' ', vec!["p2".into()])
    .end_task()
    .add_task("Polish movement feel")
    .add_subtask("Coyote time", ' ', vec![])
    .add_subtask("Jump buffering", ' ', vec![])
    .add_subtask("Air control tuning", ' ', vec![])
    .end_task()
    .done()
}

/// Enemy AI and behavior
fn add_enemy_ai(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "features/enemy-ai.md",
        "features.enemy-ai",
        "Enemy AI & Behavior",
    )
    .with_labels(vec!["backend".into(), "ai".into(), "p0".into()])
    .with_description(
        "Enemy behavior systems including behavior trees, pathfinding, and difficulty scaling.",
    )
    .add_task("Implement basic enemy types")
    .add_subtask("Walker enemy (patrols)", 'x', vec![])
    .add_subtask("Flyer enemy (aerial)", 'x', vec![])
    .add_subtask("Shooter enemy (ranged)", ' ', vec![])
    .add_subtask("Charger enemy (aggressive)", ' ', vec![])
    .end_task()
    .add_task("Create behavior tree system")
    .with_id("enemy-behavior-trees")
    .add_subtask("Behavior tree nodes", 'x', vec![])
    .add_subtask("Blackboard data structure", 'x', vec![])
    .add_subtask("Tree evaluation", ' ', vec![])
    .end_task()
    .add_task("Add pathfinding")
    .add_subtask("A* algorithm implementation", ' ', vec!["p1".into()])
    .add_subtask("Navigation mesh generation", ' ', vec!["p1".into()])
    .add_subtask("Dynamic obstacle avoidance", ' ', vec!["p2".into()])
    .end_task()
    .add_task("Implement difficulty scaling")
    .add_subtask("Enemy health scaling", ' ', vec![])
    .add_subtask("Attack pattern variations", ' ', vec![])
    .add_subtask("Spawn rate adjustments", ' ', vec![])
    .end_task()
    .done()
}

/// Level generation
fn add_level_generation(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "features/level-generation.md",
        "features.level-gen",
        "Procedural Level Generation",
    )
    .with_labels(vec!["backend".into(), "worldgen".into(), "p1".into()])
    .with_description("Procedural level generation algorithms and tile placement systems.")
    .add_task("Design level generation algorithm")
    .done()
    .add_subtask("Room-based generation", 'x', vec![])
    .add_subtask("Corridor connections", 'x', vec![])
    .add_subtask("Critical path analysis", 'x', vec![])
    .end_task()
    .add_task("Implement tile placement")
    .add_subtask("Platform placement rules", 'x', vec![])
    .add_subtask("Hazard distribution", ' ', vec![])
    .add_subtask("Collectible placement", ' ', vec![])
    .end_task()
    .add_task("Add biome variety")
    .add_subtask("Forest biome", ' ', vec![])
    .add_subtask("Cave biome", ' ', vec![])
    .add_subtask("Sky biome", ' ', vec![])
    .add_subtask("Lava biome", ' ', vec![])
    .end_task()
    .add_task("Validate level playability")
    .add_subtask("Reachability checks", ' ', vec!["p0".into()])
    .add_subtask("Difficulty estimation", ' ', vec!["p1".into()])
    .add_subtask("Seed-based regeneration", ' ', vec![])
    .end_task()
    .done()
}

/// Power-ups and items
fn add_power_ups(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "features/power-ups.md",
        "features.power-ups",
        "Power-ups & Item System",
    )
    .with_labels(vec!["backend".into(), "gameplay".into(), "p1".into()])
    .with_description("Item collection system, power-up effects, and game balance.")
    .add_task("Design item system architecture")
    .done()
    .add_subtask("Item component structure", 'x', vec![])
    .add_subtask("Inventory management", 'x', vec![])
    .add_subtask("Effect application system", 'x', vec![])
    .end_task()
    .add_task("Implement core power-ups")
    .add_subtask("Health restore", 'x', vec![])
    .add_subtask("Speed boost", 'x', vec![])
    .add_subtask("Invincibility", ' ', vec![])
    .add_subtask("Double damage", ' ', vec![])
    .end_task()
    .add_task("Add permanent upgrades")
    .add_subtask("Max health increase", ' ', vec![])
    .add_subtask("Jump height boost", ' ', vec![])
    .add_subtask("Dash unlock", ' ', vec![])
    .end_task()
    .add_task("Balance power-up effects")
    .add_subtask("Duration tuning", ' ', vec![])
    .add_subtask("Spawn frequency", ' ', vec![])
    .add_subtask("Stacking behavior", ' ', vec![])
    .end_task()
    .done()
}

/// Boss fights
fn add_boss_fights(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "features/boss-fights.md",
        "features.boss-fights",
        "Boss Encounters",
    )
    .with_labels(vec!["backend".into(), "gameplay".into(), "p1".into()])
    .depends_on("features/enemy-ai.md#enemy-behavior-trees")
    .with_description(
        "Special boss encounters with unique attack patterns, phases, and cinematics.",
    )
    .add_task("Design boss framework")
    .add_subtask("Phase transition system", 'x', vec![])
    .add_subtask("Attack pattern scheduler", 'x', vec![])
    .add_subtask("Vulnerability windows", ' ', vec![])
    .end_task()
    .add_task("Implement World 1 boss")
    .with_id("world-1-boss")
    .add_subtask("Forest guardian design", 'x', vec![])
    .add_subtask("Three attack patterns", ' ', vec![])
    .add_subtask("Environmental hazards", ' ', vec![])
    .end_task()
    .add_task("Implement World 2 boss")
    .add_subtask("Cave troll design", ' ', vec![])
    .add_subtask("Rock throw pattern", ' ', vec![])
    .add_subtask("Ground slam attack", ' ', vec![])
    .end_task()
    .add_task("Add boss cinematics")
    .add_subtask("Boss intro cutscene", ' ', vec!["p2".into()])
    .add_subtask("Victory celebration", ' ', vec!["p2".into()])
    .add_subtask("Defeat animation", ' ', vec![])
    .end_task()
    .done()
}

/// Add game systems module (4 files)
fn add_systems_module(gen: ProjectGenerator) -> ProjectGenerator {
    let gen = add_rendering_system(gen);
    let gen = add_audio_system(gen);
    let gen = add_physics_system(gen);

    add_input_system(gen)
}

/// Rendering system
fn add_rendering_system(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "systems/rendering.md",
        "systems.rendering",
        "Graphics & Rendering Pipeline",
    )
    .with_labels(vec!["backend".into(), "rendering".into(), "p0".into()])
    .with_description(
        "2D graphics rendering including sprite batching, camera systems, and shader effects.",
    )
    .add_task("Set up rendering pipeline")
    .done()
    .add_subtask("OpenGL/WebGL context", 'x', vec![])
    .add_subtask("Shader compilation", 'x', vec![])
    .add_subtask("Texture loading", 'x', vec![])
    .end_task()
    .add_task("Implement sprite batching")
    .add_subtask("Batch renderer design", 'x', vec![])
    .add_subtask("Texture atlas support", 'x', vec![])
    .add_subtask("Z-ordering/sorting", ' ', vec![])
    .end_task()
    .add_task("Add camera system")
    .add_subtask("Camera follow player", 'x', vec![])
    .add_subtask("Smooth camera movement", ' ', vec![])
    .add_subtask("Camera shake effects", ' ', vec![])
    .add_subtask("Zoom controls", ' ', vec!["p2".into()])
    .end_task()
    .add_task("Implement visual effects")
    .add_subtask("Particle system", ' ', vec!["p1".into()])
    .add_subtask("Screen transitions", ' ', vec![])
    .add_subtask("Post-processing shaders", ' ', vec!["p2".into()])
    .end_task()
    .add_task("Optimize rendering performance")
    .add_subtask("Frustum culling", ' ', vec![])
    .add_subtask("Occlusion culling", '-', vec![])
    .add_subtask("Render batching", ' ', vec![])
    .end_task()
    .done()
}

/// Audio system
fn add_audio_system(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("systems/audio.md", "systems.audio", "Audio Engine")
        .with_labels(vec!["backend".into(), "audio".into(), "p1".into()])
        .with_description("Sound engine for music playback, sound effects, and spatial audio.")
        .add_task("Set up audio engine")
        .done()
        .add_subtask("Audio context initialization", 'x', vec![])
        .add_subtask("Asset loading (OGG, WAV)", 'x', vec![])
        .add_subtask("Audio mixer", 'x', vec![])
        .end_task()
        .add_task("Implement music system")
        .add_subtask("Background music playback", 'x', vec![])
        .add_subtask("Crossfade transitions", ' ', vec![])
        .add_subtask("Dynamic music layers", ' ', vec!["p2".into()])
        .end_task()
        .add_task("Add sound effects")
        .add_subtask("One-shot sound playback", 'x', vec![])
        .add_subtask("Looping sounds", ' ', vec![])
        .add_subtask("Sound pooling", ' ', vec![])
        .end_task()
        .add_task("Implement spatial audio")
        .add_subtask("Distance attenuation", ' ', vec!["p1".into()])
        .add_subtask("Stereo panning", ' ', vec!["p1".into()])
        .end_task()
        .add_task("Add volume controls")
        .add_subtask("Master volume", ' ', vec![])
        .add_subtask("Music volume", ' ', vec![])
        .add_subtask("SFX volume", ' ', vec![])
        .end_task()
        .done()
}

/// Physics system
fn add_physics_system(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "systems/physics.md",
        "systems.physics",
        "Physics & Collision System",
    )
    .with_labels(vec!["backend".into(), "physics".into(), "p0".into()])
    .with_description("2D physics simulation including collision detection, forces, and platformer-specific physics.")
    .add_task("Implement collision detection")
    .done()
    .add_subtask("AABB collision", 'x', vec![])
    .add_subtask("Tile-based collision", 'x', vec![])
    .add_subtask("Collision response", 'x', vec![])
    .end_task()
    .add_task("Add physics simulation")
    .add_subtask("Velocity integration", 'x', vec![])
    .add_subtask("Gravity application", 'x', vec![])
    .add_subtask("Friction and drag", ' ', vec![])
    .end_task()
    .add_task("Implement platformer physics")
    .add_subtask("One-way platforms", 'x', vec![])
    .add_subtask("Slopes and ramps", ' ', vec!["p1".into()])
    .add_subtask("Moving platforms", ' ', vec!["p1".into()])
    .end_task()
    .add_task("Add trigger zones")
    .add_subtask("Trigger detection", ' ', vec![])
    .add_subtask("Event callbacks", ' ', vec![])
    .end_task()
    .add_task("Optimize physics performance")
    .add_subtask("Spatial partitioning", ' ', vec![])
    .add_subtask("Narrow-phase optimization", ' ', vec![])
    .end_task()
    .done()
}

/// Input system
fn add_input_system(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("systems/input.md", "systems.input", "Input Handling System")
        .with_labels(vec!["backend".into(), "input".into(), "p0".into()])
        .with_description("Controller mapping, input buffering, and multi-device support.")
        .add_task("Implement input abstraction")
        .done()
        .add_subtask("Input action mapping", 'x', vec![])
        .add_subtask("Keyboard support", 'x', vec![])
        .add_subtask("Gamepad support", 'x', vec![])
        .end_task()
        .add_task("Add input buffering")
        .add_subtask("Action buffer queue", ' ', vec![])
        .add_subtask("Buffer window tuning", ' ', vec![])
        .end_task()
        .add_task("Implement rebindable controls")
        .add_subtask("Control configuration UI", ' ', vec!["p1".into()])
        .add_subtask("Save/load bindings", ' ', vec!["p1".into()])
        .add_subtask("Default presets", ' ', vec![])
        .end_task()
        .add_task("Add multi-device support")
        .add_subtask("Device hot-swapping", ' ', vec!["p2".into()])
        .add_subtask("Multiple gamepad support", '-', vec![])
        .end_task()
        .done()
}

/// Add content creation module (5 files)
fn add_content_module(gen: ProjectGenerator) -> ProjectGenerator {
    let gen = add_sprites(gen);
    let gen = add_animations(gen);
    let gen = add_music(gen);
    let gen = add_sfx(gen);

    add_levels(gen)
}

/// Sprite art
fn add_sprites(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("content/sprites.md", "content.sprites", "Sprite Art")
        .with_labels(vec!["art".into(), "sprites".into(), "p0".into()])
        .with_description("Character sprites, tile sets, UI elements, and visual assets.")
        .add_task("Create player sprites")
        .add_subtask("Player idle sprite", 'x', vec![])
        .add_subtask("Player walk frames", 'x', vec![])
        .add_subtask("Player jump sprite", 'x', vec![])
        .add_subtask("Player attack sprite", ' ', vec![])
        .end_task()
        .add_task("Design enemy sprites")
        .add_subtask("Walker enemy sprite", 'x', vec![])
        .add_subtask("Flyer enemy sprite", 'x', vec![])
        .add_subtask("Shooter enemy sprite", ' ', vec![])
        .add_subtask("Charger enemy sprite", ' ', vec![])
        .end_task()
        .add_task("Create tile sets")
        .add_subtask("Forest tileset", 'x', vec![])
        .add_subtask("Cave tileset", ' ', vec![])
        .add_subtask("Sky tileset", ' ', vec![])
        .add_subtask("Lava tileset", ' ', vec![])
        .end_task()
        .add_task("Design UI elements")
        .add_subtask("Health bar", 'x', vec![])
        .add_subtask("Menu buttons", ' ', vec![])
        .add_subtask("Inventory icons", ' ', vec![])
        .end_task()
        .done()
}

/// Animation frames
fn add_animations(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "content/animations.md",
        "content.animations",
        "Animation Sequences",
    )
    .with_labels(vec!["art".into(), "animation".into(), "p1".into()])
    .with_description("Character animations, frame sequences, and timing.")
    .add_task("Animate player actions")
    .add_subtask("Walk cycle (8 frames)", 'x', vec![])
    .add_subtask("Jump animation (4 frames)", 'x', vec![])
    .add_subtask("Attack animation (6 frames)", ' ', vec![])
    .add_subtask("Death animation (8 frames)", ' ', vec![])
    .end_task()
    .add_task("Animate enemies")
    .add_subtask("Walker patrol cycle", 'x', vec![])
    .add_subtask("Flyer flight cycle", ' ', vec![])
    .add_subtask("Attack animations", ' ', vec![])
    .end_task()
    .add_task("Create environmental animations")
    .add_subtask("Water ripple effect", ' ', vec!["p2".into()])
    .add_subtask("Torch flame", ' ', vec!["p2".into()])
    .add_subtask("Grass sway", '-', vec![])
    .end_task()
    .add_task("Add particle effects")
    .add_subtask("Dust particles", ' ', vec![])
    .add_subtask("Hit sparks", ' ', vec![])
    .add_subtask("Power-up glow", ' ', vec![])
    .end_task()
    .done()
}

/// Music tracks
fn add_music(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("content/music.md", "content.music", "Music & Soundtrack")
        .with_labels(vec!["audio".into(), "music".into(), "p1".into()])
        .with_description("Background music, level themes, and boss battle tracks.")
        .add_task("Compose main theme")
        .done()
        .add_subtask("Title screen theme", 'x', vec![])
        .add_subtask("Main menu music", 'x', vec![])
        .end_task()
        .add_task("Create level themes")
        .add_subtask("Forest theme (upbeat)", 'x', vec![])
        .add_subtask("Cave theme (mysterious)", ' ', vec![])
        .add_subtask("Sky theme (ethereal)", ' ', vec![])
        .add_subtask("Lava theme (intense)", ' ', vec![])
        .end_task()
        .add_task("Compose boss music")
        .add_subtask("World 1 boss theme", ' ', vec![])
        .add_subtask("World 2 boss theme", ' ', vec![])
        .add_subtask("Final boss theme", ' ', vec![])
        .end_task()
        .add_task("Add ambient tracks")
        .add_subtask("Victory jingle", ' ', vec![])
        .add_subtask("Death jingle", ' ', vec![])
        .add_subtask("Power-up jingle", ' ', vec![])
        .end_task()
        .done()
}

/// Sound effects
fn add_sfx(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("content/sfx.md", "content.sfx", "Sound Effects")
        .with_labels(vec!["audio".into(), "sfx".into(), "p1".into()])
        .with_description("Game sound effects for actions, UI, and ambience.")
        .add_task("Create player sounds")
        .add_subtask("Jump sound", 'x', vec![])
        .add_subtask("Land sound", 'x', vec![])
        .add_subtask("Footstep sounds", ' ', vec![])
        .add_subtask("Damage sound", ' ', vec![])
        .add_subtask("Death sound", ' ', vec![])
        .end_task()
        .add_task("Design combat sounds")
        .add_subtask("Sword swing", ' ', vec![])
        .add_subtask("Hit impact", ' ', vec![])
        .add_subtask("Enemy death", ' ', vec![])
        .end_task()
        .add_task("Add item sounds")
        .add_subtask("Coin collect", 'x', vec![])
        .add_subtask("Power-up pickup", ' ', vec![])
        .add_subtask("Health restore", ' ', vec![])
        .end_task()
        .add_task("Create UI sounds")
        .add_subtask("Button click", ' ', vec![])
        .add_subtask("Menu navigate", ' ', vec![])
        .add_subtask("Pause/unpause", ' ', vec![])
        .end_task()
        .done()
}

/// Level design
fn add_levels(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("content/levels.md", "content.levels", "Level Design")
        .with_labels(vec!["design".into(), "levels".into(), "p0".into()])
        .with_description("Hand-crafted level layouts for each world and tutorial sequences.")
        .add_task("Design tutorial levels")
        .done()
        .add_subtask("Tutorial 1: Basic movement", 'x', vec![])
        .add_subtask("Tutorial 2: Combat intro", 'x', vec![])
        .add_subtask("Tutorial 3: Power-ups", 'x', vec![])
        .end_task()
        .add_task("Create World 1 levels")
        .add_subtask("World 1-1: Forest entrance", 'x', vec![])
        .add_subtask("World 1-2: Treetop platforms", 'x', vec![])
        .add_subtask("World 1-3: Forest depths", ' ', vec![])
        .add_subtask("World 1-4: Boss arena", ' ', vec![])
        .end_task()
        .add_task("Create World 2 levels")
        .add_subtask("World 2-1: Cave entrance", ' ', vec![])
        .add_subtask("World 2-2: Underground lake", ' ', vec![])
        .add_subtask("World 2-3: Crystal caverns", ' ', vec![])
        .add_subtask("World 2-4: Boss arena", ' ', vec![])
        .end_task()
        .add_task("Design secret areas")
        .add_subtask("Hidden rooms", ' ', vec!["p2".into()])
        .add_subtask("Bonus challenges", ' ', vec!["p2".into()])
        .end_task()
        .done()
}

/// Add infrastructure module (3 files)
fn add_infrastructure_module(gen: ProjectGenerator) -> ProjectGenerator {
    let gen = add_build_pipeline(gen);
    let gen = add_asset_pipeline(gen);

    add_testing_infra(gen)
}

/// Build and CI/CD
fn add_build_pipeline(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "infrastructure/build-pipeline.md",
        "infra.build",
        "Build Pipeline & CI/CD",
    )
    .with_labels(vec!["tooling".into(), "devops".into(), "p1".into()])
    .with_description("Continuous integration, automated testing, and release management.")
    .add_task("Set up CI/CD")
    .done()
    .add_subtask("GitHub Actions workflow", 'x', vec![])
    .add_subtask("Build matrix (platforms)", 'x', vec![])
    .add_subtask("Automated testing", 'x', vec![])
    .end_task()
    .add_task("Configure build targets")
    .add_subtask("Web build (WASM)", 'x', vec![])
    .add_subtask("Windows build", ' ', vec![])
    .add_subtask("macOS build", ' ', vec![])
    .add_subtask("Linux build", ' ', vec![])
    .end_task()
    .add_task("Implement release automation")
    .add_subtask("Version bumping", ' ', vec![])
    .add_subtask("Changelog generation", ' ', vec![])
    .add_subtask("Asset bundling", ' ', vec![])
    .add_subtask("Platform packaging", ' ', vec![])
    .end_task()
    .add_task("Add deployment pipeline")
    .add_subtask("Deploy to itch.io", ' ', vec!["p1".into()])
    .add_subtask("Deploy to Steam", ' ', vec!["p2".into()])
    .end_task()
    .done()
}

/// Asset processing
fn add_asset_pipeline(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "infrastructure/asset-pipeline.md",
        "infra.assets",
        "Asset Processing Pipeline",
    )
    .with_labels(vec!["tooling".into(), "assets".into(), "p1".into()])
    .with_description("Automated asset import, optimization, and conversion.")
    .add_task("Set up asset importer")
    .add_subtask("Image format conversion", 'x', vec![])
    .add_subtask("Sprite sheet packing", 'x', vec![])
    .add_subtask("Audio format conversion", ' ', vec![])
    .end_task()
    .add_task("Implement texture optimization")
    .add_subtask("PNG compression", ' ', vec![])
    .add_subtask("Texture atlas generation", ' ', vec![])
    .add_subtask("Mipmap generation", '-', vec![])
    .end_task()
    .add_task("Add audio processing")
    .add_subtask("Audio normalization", ' ', vec![])
    .add_subtask("Format conversion (OGG)", ' ', vec![])
    .add_subtask("Compression settings", ' ', vec![])
    .end_task()
    .add_task("Create asset validation")
    .add_subtask("Check missing assets", ' ', vec![])
    .add_subtask("Validate asset metadata", ' ', vec![])
    .add_subtask("Detect unused assets", ' ', vec!["p2".into()])
    .end_task()
    .done()
}

/// Testing infrastructure
fn add_testing_infra(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "infrastructure/testing.md",
        "infra.testing",
        "Testing Framework",
    )
    .with_labels(vec!["testing".into(), "qa".into(), "p1".into()])
    .with_description("Unit tests, integration tests, and automated playtesting.")
    .add_task("Set up test framework")
    .done()
    .add_subtask("Unit test infrastructure", 'x', vec![])
    .add_subtask("Integration test setup", 'x', vec![])
    .add_subtask("Test coverage reporting", 'x', vec![])
    .end_task()
    .add_task("Write unit tests")
    .add_subtask("Physics tests", 'x', vec![])
    .add_subtask("AI behavior tests", ' ', vec![])
    .add_subtask("Input handling tests", ' ', vec![])
    .end_task()
    .add_task("Create integration tests")
    .add_subtask("Level loading tests", ' ', vec![])
    .add_subtask("Save/load tests", ' ', vec![])
    .add_subtask("Combat system tests", ' ', vec![])
    .end_task()
    .add_task("Implement playtesting automation")
    .add_subtask("Replay recording", ' ', vec!["p2".into()])
    .add_subtask("Automated playthrough", ' ', vec!["p2".into()])
    .add_subtask("Performance profiling", ' ', vec![])
    .end_task()
    .done()
}

/// Add design documents module (3 files)
fn add_design_module(gen: ProjectGenerator) -> ProjectGenerator {
    let gen = add_core_loop(gen);
    let gen = add_progression(gen);

    add_story(gen)
}

/// Core gameplay loop
fn add_core_loop(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "design/core-loop.md",
        "design.core-loop",
        "Core Gameplay Loop",
    )
    .with_labels(vec!["design".into(), "gameplay".into(), "p0".into()])
    .with_description("Core gameplay mechanics, pacing, and player experience design.")
    .add_task("Define core loop")
    .done()
    .add_subtask("Explore → Fight → Collect → Progress", 'x', vec![])
    .add_subtask("Session length target (20-30 min)", 'x', vec![])
    .end_task()
    .add_task("Design difficulty curve")
    .add_subtask("Tutorial pacing", 'x', vec![])
    .add_subtask("Enemy introduction schedule", ' ', vec![])
    .add_subtask("Skill check placement", ' ', vec![])
    .end_task()
    .add_task("Plan player progression")
    .add_subtask("Health upgrade path", ' ', vec![])
    .add_subtask("Ability unlock sequence", ' ', vec![])
    .add_subtask("Collectible distribution", ' ', vec![])
    .end_task()
    .add_task("Balance risk/reward")
    .add_subtask("Secret area rewards", ' ', vec![])
    .add_subtask("Risk vs. safety paths", ' ', vec![])
    .end_task()
    .done()
}

/// Progression system
fn add_progression(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "design/progression.md",
        "design.progression",
        "Player Progression",
    )
    .with_labels(vec!["design".into(), "gameplay".into(), "p0".into()])
    .with_description("Difficulty curve, unlock progression, and meta-progression systems.")
    .add_task("Design unlock progression")
    .add_subtask("Ability unlock gates", 'x', vec![])
    .add_subtask("World unlock requirements", 'x', vec![])
    .add_subtask("Optional content gates", ' ', vec![])
    .end_task()
    .add_task("Balance difficulty scaling")
    .add_subtask("Enemy difficulty by world", ' ', vec![])
    .add_subtask("Platform challenge scaling", ' ', vec![])
    .add_subtask("Boss difficulty tuning", ' ', vec![])
    .end_task()
    .add_task("Add meta-progression")
    .add_subtask("Collectible tracking", ' ', vec!["p2".into()])
    .add_subtask("Achievement system", ' ', vec!["p2".into()])
    .add_subtask("Unlockable cosmetics", '-', vec![])
    .end_task()
    .done()
}

/// Story and narrative
fn add_story(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("design/story.md", "design.story", "Story & Narrative")
        .with_labels(vec!["design".into(), "narrative".into(), "p2".into()])
        .with_description("Game narrative, character backstory, and world building.")
        .add_task("Write main storyline")
        .add_subtask("Opening narrative", 'x', vec![])
        .add_subtask("World 1 story beats", ' ', vec![])
        .add_subtask("World 2 story beats", ' ', vec![])
        .add_subtask("Ending narrative", ' ', vec![])
        .end_task()
        .add_task("Develop character backstory")
        .add_subtask("Player character origin", ' ', vec![])
        .add_subtask("Boss character motivations", ' ', vec![])
        .end_task()
        .add_task("Create world lore")
        .add_subtask("Forest realm history", ' ', vec!["p2".into()])
        .add_subtask("Cave realm secrets", ' ', vec!["p2".into()])
        .end_task()
        .add_task("Write NPC dialogue")
        .waived()
        .add_subtask("Tutorial NPC", '-', vec![])
        .add_subtask("Shop keeper", '-', vec![])
        .end_task()
        .done()
}

/// Add milestones module (3 files)
fn add_milestones_module(gen: ProjectGenerator) -> ProjectGenerator {
    let gen = add_alpha_milestone(gen);
    let gen = add_beta_milestone(gen);

    add_release_milestone(gen)
}

/// Alpha milestone
fn add_alpha_milestone(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("milestones/alpha.md", "milestone.alpha", "Alpha Milestone")
        .with_labels(vec!["milestone".into(), "p0".into()])
        .depends_on("features/player-movement.md")
        .depends_on("systems/physics.md")
        .depends_on("systems/rendering.md")
        .with_description("Alpha release: core gameplay loop playable from start to finish.")
        .add_task("Core gameplay complete")
        .done()
        .add_subtask("Player movement working", 'x', vec![])
        .add_subtask("Basic enemies implemented", 'x', vec![])
        .add_subtask("Physics stable", 'x', vec![])
        .end_task()
        .add_task("First world playable")
        .done()
        .add_subtask("Tutorial levels complete", 'x', vec![])
        .add_subtask("World 1 levels built", 'x', vec![])
        .add_subtask("World 1 boss functional", 'x', vec![])
        .end_task()
        .add_task("Essential systems working")
        .done()
        .add_subtask("Rendering pipeline", 'x', vec![])
        .add_subtask("Audio playback", 'x', vec![])
        .add_subtask("Input handling", 'x', vec![])
        .end_task()
        .add_task("Alpha playtesting")
        .done()
        .add_subtask("Internal playtest round", 'x', vec![])
        .add_subtask("Bug fixing pass", 'x', vec![])
        .add_subtask("Balance adjustments", 'x', vec![])
        .end_task()
        .done()
}

/// Beta milestone
fn add_beta_milestone(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file("milestones/beta.md", "milestone.beta", "Beta Milestone")
        .with_labels(vec!["milestone".into(), "p0".into()])
        .depends_on("milestones/alpha.md")
        .depends_on("features/boss-fights.md#world-1-boss")
        .with_description("Beta release: all features complete, full content, ready for polish.")
        .add_task("All worlds complete")
        .add_subtask("World 2 levels built", 'x', vec![])
        .add_subtask("World 2 boss implemented", ' ', vec![])
        .add_subtask("All biomes functional", ' ', vec![])
        .end_task()
        .add_task("Feature complete")
        .add_subtask("All power-ups implemented", ' ', vec![])
        .add_subtask("Special moves unlockable", ' ', vec![])
        .add_subtask("Boss fights polished", ' ', vec![])
        .end_task()
        .add_task("Content complete")
        .add_subtask("All sprite art finished", ' ', vec![])
        .add_subtask("All animations done", ' ', vec![])
        .add_subtask("Music tracks complete", ' ', vec![])
        .add_subtask("Sound effects complete", ' ', vec![])
        .end_task()
        .add_task("Beta playtesting")
        .add_subtask("External playtest round", ' ', vec![])
        .add_subtask("Feedback integration", ' ', vec![])
        .add_subtask("Performance optimization", ' ', vec![])
        .end_task()
        .done()
}

/// Release milestone
fn add_release_milestone(gen: ProjectGenerator) -> ProjectGenerator {
    gen.add_file(
        "milestones/release.md",
        "milestone.release",
        "Release Milestone",
    )
    .with_labels(vec!["milestone".into(), "p0".into()])
    .depends_on("milestones/beta.md")
    .with_description("v1.0 release: polished, marketed, and ready to ship.")
    .add_task("Final polish")
    .add_subtask("Visual polish pass", ' ', vec![])
    .add_subtask("Audio mix and mastering", ' ', vec![])
    .add_subtask("UI/UX refinements", ' ', vec![])
    .add_subtask("Performance tuning", ' ', vec![])
    .end_task()
    .add_task("Marketing materials")
    .add_subtask("Trailer video", ' ', vec![])
    .add_subtask("Screenshots", ' ', vec![])
    .add_subtask("Store page copy", ' ', vec![])
    .add_subtask("Press kit", ' ', vec![])
    .end_task()
    .add_task("Release preparation")
    .add_subtask("Build all platforms", ' ', vec![])
    .add_subtask("Test on target hardware", ' ', vec![])
    .add_subtask("Prepare patch pipeline", ' ', vec![])
    .end_task()
    .add_task("Launch")
    .add_subtask("Submit to stores", ' ', vec![])
    .add_subtask("Launch day monitoring", ' ', vec![])
    .add_subtask("Community engagement", ' ', vec![])
    .end_task()
    .done()
}
