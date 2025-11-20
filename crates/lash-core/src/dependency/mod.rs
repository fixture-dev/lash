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

pub mod graph;

pub use graph::{DependencyGraph, EdgeData, EdgeId, EdgeRef, NodeData};
