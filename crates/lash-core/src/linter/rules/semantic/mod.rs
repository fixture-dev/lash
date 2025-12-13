//! Semantic validation rules
//!
//! This module contains rules that validate the semantic correctness of
//! Lash Markdown files. Semantic rules ensure logical consistency, valid
//! data formats, and proper task relationships.
//!
//! Unlike syntax rules which focus on formatting and structure, semantic
//! rules validate the meaning and relationships within the parsed data:
//!
//! - **ID uniqueness** - No duplicate task IDs within a file
//! - **Status consistency** - Parent-child status relationships make sense
//! - **Auto-waiving** - Children of waived parents are automatically waived
//! - **Label format** - Labels match the required pattern
//! - **Date format** - Dates are valid and properly formatted
//! - **Estimate format** - Time estimates follow the required pattern
//! - **Owner format** - Owner names are reasonable
//! - **Empty titles** - Tasks have non-empty titles
//! - **Description length** - Descriptions are within reasonable length limits
//! - **Doc fragment validation** - Fragment references point to existing headings
//! - **Note indentation** - Contextual notes have correct indentation
//! - **Note length** - Contextual notes are within length limits
//! - **Note nesting** - Contextual notes don't have nested children
//! - **Note ordering** - Contextual notes appear before child tasks (style)

pub mod auto_waive;
pub mod broken_doc_fragment;
pub mod description_length;
pub mod duplicate_id;
pub mod empty_title;
pub mod note_indentation;
pub mod note_length;
pub mod note_nesting;
pub mod note_ordering;
pub mod status_consistency;
pub mod valid_date;
pub mod valid_doc_reference;
pub mod valid_estimate;
pub mod valid_label;
pub mod valid_owner;

pub use auto_waive::AutoWaiveRule;
pub use broken_doc_fragment::BrokenDocFragmentRule;
pub use description_length::DescriptionLengthRule;
pub use duplicate_id::DuplicateIdRule;
pub use empty_title::EmptyTitleRule;
pub use note_indentation::NoteIndentationRule;
pub use note_length::NoteLengthRule;
pub use note_nesting::NoteNestingRule;
pub use note_ordering::NoteOrderingRule;
pub use status_consistency::StatusConsistencyRule;
pub use valid_date::ValidDateRule;
pub use valid_doc_reference::ValidDocReferenceRule;
pub use valid_estimate::ValidEstimateRule;
pub use valid_label::ValidLabelRule;
pub use valid_owner::ValidOwnerRule;
