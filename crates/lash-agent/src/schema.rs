//! Schema generation for Lash task format
//!
//! This module generates machine-readable and human-readable schema documentation
//! that describes the Lash task file format, annotations, and constraints.

use serde::{Deserialize, Serialize};

/// Schema representation for the Lash task format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LashSchema {
    /// Version of the schema
    pub version: String,
    /// Description of the format
    pub description: String,
    /// Allowed annotations
    pub annotations: Vec<AnnotationSpec>,
    /// Checkbox status values
    pub status_values: Vec<StatusSpec>,
    /// Constraints and limits
    pub constraints: Vec<ConstraintSpec>,
    /// Operations that can be performed
    pub operations: Vec<OperationSpec>,
}

/// Specification for a task file annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationSpec {
    /// Annotation key (e.g., "id", "labels")
    pub key: String,
    /// Description of the annotation
    pub description: String,
    /// Example value
    pub example: String,
    /// Whether this annotation is required
    pub required: bool,
}

/// Specification for a checkbox status value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSpec {
    /// Status symbol (e.g., `[ ]`, `[x]`)
    pub symbol: String,
    /// Name of the status (e.g., "open", "done")
    pub name: String,
    /// Description
    pub description: String,
}

/// Specification for a format constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSpec {
    /// Name of the constraint
    pub name: String,
    /// Description
    pub description: String,
    /// Example or value (e.g., "3-4 levels")
    pub value: String,
}

/// Specification for an allowed operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSpec {
    /// Operation name
    pub name: String,
    /// Description of what it does
    pub description: String,
    /// Example showing how to perform it
    pub example: String,
}

/// Generate the standard Lash schema
///
/// Returns a complete schema describing the Lash task file format,
/// including all annotations, status values, constraints, and operations.
///
/// # Examples
///
/// ```
/// use lash_agent::schema::generate_schema;
///
/// let schema = generate_schema();
/// assert_eq!(schema.version, "1.0");
/// assert!(!schema.annotations.is_empty());
/// ```
#[allow(clippy::too_many_lines)] // Schema definition is inherently large
pub fn generate_schema() -> LashSchema {
    LashSchema {
        version: "1.0".to_string(),
        description: "Lash task file format - hierarchical Markdown checkboxes with annotations"
            .to_string(),
        annotations: vec![
            AnnotationSpec {
                key: "id".to_string(),
                description: "Unique identifier within the file".to_string(),
                example: "@id: feature-auth".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "labels".to_string(),
                description: "Comma-separated tags for cross-cutting organization".to_string(),
                example: "@labels: backend, security".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "owner".to_string(),
                description: "Person or agent responsible for this task".to_string(),
                example: "@owner: alice".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "created".to_string(),
                description: "Creation date in YYYY-MM-DD format".to_string(),
                example: "@created: 2025-01-15".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "estimate".to_string(),
                description: "Time estimate for completion".to_string(),
                example: "@estimate: 2d".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "depends-on".to_string(),
                description: "Cross-file dependency reference".to_string(),
                example: "@depends-on: path/to/file.md#task:id".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "agent-note".to_string(),
                description: "Hints or instructions for AI agents".to_string(),
                example: "@agent-note: Use existing auth patterns".to_string(),
                required: false,
            },
            AnnotationSpec {
                key: "doc".to_string(),
                description: "Link to documentation resource (informational, non-blocking)"
                    .to_string(),
                example: "@doc: ../docs/design.md#section-7".to_string(),
                required: false,
            },
        ],
        status_values: vec![
            StatusSpec {
                symbol: "[ ]".to_string(),
                name: "open".to_string(),
                description: "Task not yet started".to_string(),
            },
            StatusSpec {
                symbol: "[>]".to_string(),
                name: "in-progress".to_string(),
                description: "Task actively being worked on".to_string(),
            },
            StatusSpec {
                symbol: "[x]".to_string(),
                name: "done".to_string(),
                description: "Task completed successfully".to_string(),
            },
            StatusSpec {
                symbol: "[-]".to_string(),
                name: "waived".to_string(),
                description: "Task marked as not applicable or cancelled".to_string(),
            },
            StatusSpec {
                symbol: "[!]".to_string(),
                name: "blocked".to_string(),
                description: "Task blocked by dependencies or external factors".to_string(),
            },
        ],
        constraints: vec![
            ConstraintSpec {
                name: "unique_ids".to_string(),
                description: "Task IDs must be unique within each file".to_string(),
                value: "per-file uniqueness".to_string(),
            },
            ConstraintSpec {
                name: "max_depth".to_string(),
                description: "Maximum nesting depth for task hierarchies".to_string(),
                value: "3-4 levels".to_string(),
            },
            ConstraintSpec {
                name: "status_consistency".to_string(),
                description: "Parent tasks complete only when all children are done or waived"
                    .to_string(),
                value: "hierarchical consistency".to_string(),
            },
            ConstraintSpec {
                name: "valid_dependencies".to_string(),
                description: "Dependency references must point to existing tasks".to_string(),
                value: "resolvable references".to_string(),
            },
            ConstraintSpec {
                name: "contextual_notes".to_string(),
                description: "Plain bullets (without checkboxes) nested under tasks provide context, requirements, or acceptance criteria. Notes cannot have children."
                    .to_string(),
                value: "informational only, not actionable".to_string(),
            },
        ],
        operations: vec![
            OperationSpec {
                name: "add_task".to_string(),
                description: "Add a new task to the hierarchy".to_string(),
                example: "- [ ] New task description".to_string(),
            },
            OperationSpec {
                name: "update_status".to_string(),
                description: "Mark a task as done, waived, or blocked".to_string(),
                example: "- [x] Completed task".to_string(),
            },
            OperationSpec {
                name: "add_subtask".to_string(),
                description: "Add a child task (indent with 2 spaces)".to_string(),
                example: "  - [ ] Subtask description".to_string(),
            },
            OperationSpec {
                name: "add_annotation".to_string(),
                description: "Add metadata annotation to a task or file".to_string(),
                example: "@labels: backend, api".to_string(),
            },
            OperationSpec {
                name: "add_dependency".to_string(),
                description: "Link to a task in another file".to_string(),
                example: "@depends-on: core/auth.md#task:login".to_string(),
            },
            OperationSpec {
                name: "waive_task".to_string(),
                description: "Mark a task as not applicable".to_string(),
                example: "- [-] Task no longer needed".to_string(),
            },
            OperationSpec {
                name: "add_doc_reference".to_string(),
                description: "Link to documentation resource for context".to_string(),
                example: "@doc: docs/design.md#section-name".to_string(),
            },
            OperationSpec {
                name: "add_contextual_note".to_string(),
                description: "Add inline context or requirements under a task using plain bullets (no checkbox). Notes are informational and not tracked for completion."
                    .to_string(),
                example: "- [ ] Implement payment gateway\n  - Use Stripe API v3\n  - Support credit card and ACH payments".to_string(),
            },
        ],
    }
}

/// Generate schema as plain text Markdown
///
/// Creates a human-readable Markdown representation of the schema
/// suitable for inclusion in agent prompts.
///
/// # Examples
///
/// ```
/// use lash_agent::schema::generate_schema_text;
///
/// let schema_text = generate_schema_text();
/// assert!(schema_text.contains("# Lash Task File Format"));
/// assert!(schema_text.contains("## Allowed Operations"));
/// ```
pub fn generate_schema_text() -> String {
    let schema = generate_schema();
    let mut output = String::new();

    output.push_str("# Lash Task File Format\n\n");
    output.push_str(&format!("{}\n\n", schema.description));
    output.push_str(&format!("**Version:** {}\n\n", schema.version));

    // Annotations
    output.push_str("## Annotations\n\n");
    output.push_str("Task files support the following metadata annotations:\n\n");
    for ann in &schema.annotations {
        let required = if ann.required { " (required)" } else { "" };
        output.push_str(&format!(
            "- `@{}`: {}{}\n",
            ann.key, ann.description, required
        ));
        output.push_str(&format!("  - Example: `{}`\n", ann.example));
    }
    output.push('\n');

    // Status values
    output.push_str("## Task Status Values\n\n");
    for status in &schema.status_values {
        output.push_str(&format!(
            "- `{}` ({}) - {}\n",
            status.symbol, status.name, status.description
        ));
    }
    output.push('\n');

    // Constraints
    output.push_str("## Constraints\n\n");
    for constraint in &schema.constraints {
        output.push_str(&format!(
            "- **{}**: {} ({})\n",
            constraint.name, constraint.description, constraint.value
        ));
    }
    output.push('\n');

    // Operations
    output.push_str("## Allowed Operations\n\n");
    for op in &schema.operations {
        output.push_str(&format!("### {}\n\n", op.name));
        output.push_str(&format!("{}\n\n", op.description));
        output.push_str("```markdown\n");
        output.push_str(&op.example);
        output.push_str("\n```\n\n");
    }

    output
}

/// Generate a minimal example task file
///
/// Returns a string containing a minimal but complete example of a valid
/// Lash task file, useful for showing agents the basic structure.
///
/// # Examples
///
/// ```
/// use lash_agent::schema::generate_minimal_example;
///
/// let example = generate_minimal_example();
/// assert!(example.contains("@id:"));
/// assert!(example.contains("- [ ]"));
/// ```
pub fn generate_minimal_example() -> String {
    r"# Feature: User Authentication

@id: feature-auth
@labels: backend, security
@owner: alice

## Description

Implement secure user authentication using industry-standard practices.
This includes password hashing with bcrypt and JWT tokens for session management.

## Tasks

- [ ] Implement login endpoint
  - Use bcrypt with cost factor 12 for password hashing
  - JWT tokens should expire after 24 hours
  - [ ] Add password hashing
  - [ ] Add JWT token generation
- [ ] Add user registration
  - Validate email format before storing
  - Send confirmation email on registration
- [x] Set up database schema
"
    .to_string()
}

/// Generate an example with dependencies
///
/// Returns a string showing how to use cross-file dependencies.
///
/// # Examples
///
/// ```
/// use lash_agent::schema::generate_dependency_example;
///
/// let example = generate_dependency_example();
/// assert!(example.contains("@depends-on:"));
/// ```
pub fn generate_dependency_example() -> String {
    r"# Feature: User Profile Page

@id: feature-profile
@labels: frontend
@depends-on: backend/auth.md#task:feature-auth

## Tasks

- [ ] Design profile UI
  - [ ] Create mockups
  - [ ] Get design approval
- [ ] Implement profile component
  - [ ] Fetch user data from API
  - [ ] Display user information
- [ ] Add edit functionality
"
    .to_string()
}

/// Generate an example with documentation references
///
/// Returns a string showing how to use `@doc` annotations
/// for linking to documentation resources.
///
/// # Examples
///
/// ```
/// use lash_agent::schema::generate_doc_reference_example;
///
/// let example = generate_doc_reference_example();
/// assert!(example.contains("@doc:"));
/// ```
pub fn generate_doc_reference_example() -> String {
    r"# Feature: Payment Processing

@id: feature-payments
@labels: backend, billing
@doc: ../docs/design-doc.md#payment-flow
@doc: ../docs/pci-compliance.md

## Tasks

- [ ] Implement payment gateway integration
  @doc: ../docs/stripe-api.md#webhooks
  - [ ] Set up webhook handlers
  - [ ] Implement payment intent creation
- [ ] Add invoice generation
  @doc: ../docs/invoice-template.md
- [x] Set up billing database schema
"
    .to_string()
}

/// Generate an example showing contextual notes
///
/// Returns a string demonstrating how to use plain bullets (without checkboxes)
/// as contextual notes under tasks. Notes provide inline requirements, acceptance
/// criteria, or implementation hints without being tracked for completion.
///
/// # Examples
///
/// ```
/// use lash_agent::schema::generate_contextual_notes_example;
///
/// let example = generate_contextual_notes_example();
/// assert!(example.contains("- [ ]")); // Task with checkbox
/// assert!(example.contains("  - Use")); // Contextual note (plain bullet)
/// ```
pub fn generate_contextual_notes_example() -> String {
    r"# Feature: Procedural Level Generation

@id: feature-level-gen
@labels: gameplay, procedural

## Tasks

- [ ] Implement terrain generator
  - Use Perlin noise for natural-looking terrain
  - Ensure seed-based reproducibility for testing
  - Target 64x64 minimum map size
  - [ ] Add height map generation
  - [ ] Add biome distribution
- [ ] Create room placement algorithm
  - Rooms should not overlap
  - Maintain minimum corridor width of 2 tiles
  - [ ] Implement BSP tree partitioning
  - [ ] Add room connectivity validation
- [x] Define tile types
  - Wall, floor, door, water, lava
"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_schema() {
        let schema = generate_schema();
        assert_eq!(schema.version, "1.0");
        assert!(!schema.annotations.is_empty());
        assert!(!schema.status_values.is_empty());
        assert!(!schema.constraints.is_empty());
        assert!(!schema.operations.is_empty());
    }

    #[test]
    fn test_schema_has_required_annotations() {
        let schema = generate_schema();
        let keys: Vec<String> = schema.annotations.iter().map(|a| a.key.clone()).collect();
        assert!(keys.contains(&"id".to_string()));
        assert!(keys.contains(&"labels".to_string()));
        assert!(keys.contains(&"depends-on".to_string()));
        assert!(keys.contains(&"doc".to_string()));
    }

    #[test]
    fn test_schema_has_all_status_values() {
        let schema = generate_schema();
        let symbols: Vec<String> = schema
            .status_values
            .iter()
            .map(|s| s.symbol.clone())
            .collect();
        assert!(symbols.contains(&"[ ]".to_string()));
        assert!(symbols.contains(&"[>]".to_string()));
        assert!(symbols.contains(&"[x]".to_string()));
        assert!(symbols.contains(&"[-]".to_string()));
        assert!(symbols.contains(&"[!]".to_string()));
    }

    #[test]
    fn test_generate_schema_text() {
        let text = generate_schema_text();
        assert!(text.contains("# Lash Task File Format"));
        assert!(text.contains("## Annotations"));
        assert!(text.contains("## Task Status Values"));
        assert!(text.contains("## Constraints"));
        assert!(text.contains("## Allowed Operations"));
    }

    #[test]
    fn test_generate_minimal_example() {
        let example = generate_minimal_example();
        assert!(example.contains("@id:"));
        assert!(example.contains("@labels:"));
        assert!(example.contains("- [ ]"));
        assert!(example.contains("- [x]"));
    }

    #[test]
    fn test_generate_dependency_example() {
        let example = generate_dependency_example();
        assert!(example.contains("@depends-on:"));
        assert!(example.contains("#task:"));
    }

    #[test]
    fn test_generate_doc_reference_example() {
        let example = generate_doc_reference_example();
        assert!(example.contains("@doc:"));
        assert!(example.contains("../docs/design-doc.md#payment-flow"));
        // Has both file-level and task-level doc refs
        assert!(example.contains("@doc: ../docs/pci-compliance.md"));
        assert!(example.contains("@doc: ../docs/stripe-api.md#webhooks"));
    }

    #[test]
    fn test_schema_serialization() {
        let schema = generate_schema();
        let json = serde_json::to_string(&schema).unwrap();
        let deserialized: LashSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema.version, deserialized.version);
        assert_eq!(schema.annotations.len(), deserialized.annotations.len());
    }

    #[test]
    fn test_schema_has_contextual_notes_constraint() {
        let schema = generate_schema();
        let constraint_names: Vec<String> =
            schema.constraints.iter().map(|c| c.name.clone()).collect();
        assert!(constraint_names.contains(&"contextual_notes".to_string()));

        // Verify the constraint description mentions key details
        let notes_constraint = schema
            .constraints
            .iter()
            .find(|c| c.name == "contextual_notes")
            .expect("contextual_notes constraint should exist");
        assert!(notes_constraint.description.contains("Plain bullets"));
        assert!(notes_constraint.description.contains("without checkboxes"));
    }

    #[test]
    fn test_schema_has_add_contextual_note_operation() {
        let schema = generate_schema();
        let operation_names: Vec<String> =
            schema.operations.iter().map(|o| o.name.clone()).collect();
        assert!(operation_names.contains(&"add_contextual_note".to_string()));

        // Verify the operation example shows the pattern
        let notes_op = schema
            .operations
            .iter()
            .find(|o| o.name == "add_contextual_note")
            .expect("add_contextual_note operation should exist");
        assert!(notes_op.example.contains("- [ ]")); // Task with checkbox
        assert!(notes_op.example.contains("  - Use")); // Plain bullet note
    }

    #[test]
    fn test_generate_contextual_notes_example() {
        let example = generate_contextual_notes_example();
        // Has tasks with checkboxes
        assert!(example.contains("- [ ]"));
        assert!(example.contains("- [x]"));
        // Has contextual notes (plain bullets under tasks)
        assert!(example.contains("  - Use Perlin noise"));
        assert!(example.contains("  - Rooms should not overlap"));
        // Notes appear before child tasks (convention)
        let perlin_pos = example.find("Use Perlin noise").unwrap();
        let height_map_pos = example.find("Add height map generation").unwrap();
        assert!(perlin_pos < height_map_pos);
    }

    #[test]
    fn test_minimal_example_includes_contextual_notes() {
        let example = generate_minimal_example();
        // Verify the minimal example now includes contextual notes
        assert!(example.contains("  - Use bcrypt"));
        assert!(example.contains("  - JWT tokens should expire"));
        assert!(example.contains("  - Validate email format"));
    }

    // --- Mutant-killing test ---

    #[test]
    fn test_schema_text_no_required_marker_for_optional_annotations() {
        // Kills mut-000095 (ann.required → !ann.required):
        // All annotations in the schema are optional (required=false), so
        // "(required)" must NOT appear anywhere in the annotations section.
        // If the negation were applied, every annotation would show "(required)".
        let text = generate_schema_text();

        // None of the current annotations have required=true, so the text must
        // not contain the "(required)" suffix for any annotation line.
        assert!(
            !text.contains("(required)"),
            "No annotation is required, so '(required)' should not appear in schema text"
        );

        // Additionally verify the schema itself has no required=true annotations,
        // to document the expectation explicitly.
        let schema = generate_schema();
        assert!(
            schema.annotations.iter().all(|a| !a.required),
            "All standard annotations should be optional"
        );
    }
}
