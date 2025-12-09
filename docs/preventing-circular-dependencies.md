# Preventing Circular Dependencies in Task Creation

## Overview

The task creation modal includes infrastructure to prevent circular dependencies when selecting task dependencies. This prevents users from accidentally selecting a task or its descendants as dependencies, which would create an impossible dependency loop.

## Implementation

### MultiSelectState - Core Component

The `MultiSelectState` component in `/crates/lash-tui/src/components/multi_select.rs` has been enhanced with disabled indices functionality:

```rust
pub struct MultiSelectState {
    // ... other fields ...

    /// Disabled indices that cannot be selected (in `all_options`)
    pub disabled_indices: HashSet<usize>,
}
```

**Key Methods:**

1. `is_disabled(index: usize) -> bool` - Check if an option is disabled
2. `toggle_highlighted()` - Modified to skip disabled items (does nothing if current item is disabled)

### How It Works

When a user tries to select a disabled item (either the task itself or one of its descendants):
1. The highlighting still works - users can navigate to the item
2. But toggling selection (Space/Enter) has no effect
3. The UI should render disabled items differently (grayed out, strikethrough, etc.)

### Usage Pattern (For Future Implementation)

When the dependency selector is added to the task creation modal, follow this pattern:

```rust
// 1. When editing an existing task (not creating new)
let editing_task_id: Option<i64> = Some(task_db_id);

// 2. Fetch all descendants of the task being edited
let descendants = if let Some(task_id) = editing_task_id {
    repository.get_descendants(task_id)?
} else {
    Vec::new()
};

// 3. Build the list of all possible dependencies
let all_tasks = repository.get_all_tasks()?;
let dependency_options: Vec<MultiSelectOption> = all_tasks
    .iter()
    .map(|t| MultiSelectOption {
        id: t.full_id.clone(),
        label: t.title.clone(),
        description: Some(t.file_path.clone()),
    })
    .collect();

// 4. Create the multi-select state
let mut multi_select = MultiSelectState::new(dependency_options);

// 5. Mark self and descendants as disabled
if let Some(task_id) = editing_task_id {
    // Disable the task itself
    if let Some(self_index) = multi_select.all_options
        .iter()
        .position(|opt| opt.id == format!("{}#{}", file_id, local_id))
    {
        multi_select.disabled_indices.insert(self_index);
    }

    // Disable all descendants
    for descendant in descendants {
        if let Some(desc_index) = multi_select.all_options
            .iter()
            .position(|opt| opt.id == descendant.full_id)
        {
            multi_select.disabled_indices.insert(desc_index);
        }
    }
}
```

## Rendering Disabled Items

When rendering the dependency multi-select in the UI, disabled items should be visually distinct:

```rust
// In the rendering code (future implementation)
for (idx, option_index) in multi_select.filtered_indices.iter().enumerate() {
    let option = &multi_select.all_options[*option_index];
    let is_disabled = multi_select.is_disabled(*option_index);
    let is_highlighted = idx == multi_select.highlighted_index;

    let style = if is_disabled {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM)
    } else if is_highlighted {
        Style::default().bg(theme.highlight_bg())
    } else {
        Style::default().fg(theme.foreground())
    };

    let checkbox = if multi_select.selected_indices.contains(option_index) {
        "[x]"
    } else {
        "[ ]"
    };

    let indicator = if is_disabled {
        " (cannot select - would create circular dependency)"
    } else {
        ""
    };

    let line = format!("{} {}{}", checkbox, option.label, indicator);
    // ... render line with style ...
}
```

## Database Support

The `TaskRepository::get_descendants()` method in `/crates/lash-db/src/repository/tasks.rs` provides the backend support for this feature:

```rust
/// Get all descendants of a task (recursive)
///
/// Uses a recursive CTE to fetch all child tasks, grandchildren, etc.
pub fn get_descendants(&self, task_db_id: i64) -> DbResult<Vec<TaskRecord>>
```

This method efficiently fetches all descendants in a single database query using SQLite's recursive CTEs.

## Testing

Tests have been added to verify the disabled functionality:

- `test_disabled_indices` - Basic disable/enable behavior
- `test_disabled_indices_multiple` - Multiple disabled items
- `test_disabled_indices_empty` - Default state (nothing disabled)

Run tests with:
```bash
cargo test --package lash-tui --lib components::multi_select::tests
```

## Future Work

When implementing the actual dependency selector in the task creation modal:

1. Add a `MultiSelectState` field to `TaskCreationModalState` for dependencies
2. In `TaskCreationModalState::new()`, optionally accept an `editing_task_id: Option<i64>`
3. If editing, fetch descendants and populate `disabled_indices`
4. Update the rendering in `task_creation_modal.rs` to show disabled items appropriately
5. Update `to_request()` to extract selected dependencies from the multi-select state
6. Add integration tests for the full flow

## Benefits

- **Prevents circular dependencies** - Users cannot select tasks that would create a cycle
- **Clear UX** - Disabled items are visually distinct and provide feedback
- **No runtime errors** - Prevention happens at selection time, not submission
- **Efficient** - Single database query fetches all descendants
- **Testable** - Component behavior is fully unit tested

## Related Files

- `/crates/lash-tui/src/components/multi_select.rs` - Multi-select component
- `/crates/lash-db/src/repository/tasks.rs` - Task repository with `get_descendants()`
- `/crates/lash-tui/src/state.rs` - Application state including modal state
- `/crates/lash-tui/src/ui/task_creation_modal.rs` - Modal rendering
- `/tasks/tasks.task-creation-ui.md` - Task tracking for this feature
