//! Dependency resolution and graph analysis
//!
//! This module provides data structures and algorithms for building and querying
//! the task dependency graph. It handles three types of dependencies:
//!
//! 1. **Hierarchy dependencies** - Implicit parent-child relationships from task nesting
//! 2. **Explicit dependencies** - Cross-file references via `@depends-on` annotations
//! 3. **Directory dependencies** - Directory-level organizational dependencies
//!
//! The core data structure is [`DependencyGraph`], which provides efficient O(1) lookups
//! for direct dependencies and O(E+V) traversal for transitive dependencies.
//!
//! Cycle detection is provided by [`CycleDetector`], which uses a three-color DFS algorithm
//! to find all cycles in the graph and provide actionable suggestions for resolving them.
//!
//! Dependency resolution is handled by [`DependencyResolver`], which parses `@depends-on`
//! annotations and resolves them to concrete task IDs, handling path resolution and error
//! collection for broken links.

pub mod cycle_detector;
pub mod graph;
pub mod resolver;

pub use cycle_detector::{Cycle, CycleDetector, CycleReport, CycleSuggestion, SuggestionAction};
pub use graph::{DependencyGraph, EdgeData, EdgeId, EdgeRef, NodeData};
pub use resolver::{
    DependencyResolver, ResolutionError, ResolutionErrorKind, ResolvedDependency, ResolverResult,
};
