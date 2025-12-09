//! Task creation validation and placement
//!
//! This module provides the validation pipeline for creating new tasks
//! programmatically via CLI and TUI interfaces.
//!
//! The validation process follows these stages:
//! 1. **Validation** - Check all request fields, resolve parents, compute depth
//! 2. **Placement** - Determine exact insertion location
//! 3. **Emission** - Generate markdown for the new task
//! 4. **Orchestration** - Service that ties everything together

pub mod emitter;
pub mod placement;
pub mod service;
pub mod validation;

pub use emitter::MarkdownEmitter;
pub use placement::{PlacementInfo, PlacementResolver};
pub use service::TaskCreationService;
pub use validation::{TaskValidator, ValidationContext};
