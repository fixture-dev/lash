# Color Handling in Lash CLI

This document describes how Lash handles color output across all CLI commands.

## Priority Order

Lash determines whether to enable color output using the following priority:

1. **`--no-color` flag**: Explicitly disables colors (highest priority)
2. **`--json` flag**: JSON output never contains ANSI codes
3. **`NO_COLOR` environment variable**: Standard Unix convention to disable colors
4. **TTY detection**: Automatically disables colors when stdout is not a TTY (e.g., piped output)

## Implementation Details

### Core Functions

#### `supports_color()` - `/Users/fohara/src/lash/crates/lash-cli/src/theme.rs:355-363`

```rust
pub fn supports_color() -> bool {
    // NO_COLOR environment variable takes precedence
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // Check if stdout is a TTY
    atty::is(atty::Stream::Stdout)
}
```

This function checks:
- If `NO_COLOR` environment variable is set (any value), returns `false`
- If stdout is not a TTY (e.g., piped to file or another command), returns `false`
- Otherwise returns `true`

#### Color Decision in main.rs - `/Users/fohara/src/lash/crates/lash-cli/src/main.rs:87-92`

```rust
// Determine if colors should be enabled based on:
// 1. --no-color flag
// 2. --json flag (JSON shouldn't have ANSI codes)
// 3. NO_COLOR environment variable
// 4. Whether stdout is a TTY
let colors_enabled = !cli.no_color && !cli.json && supports_color();
```

The main application logic combines:
1. User's explicit `--no-color` flag
2. Whether output mode is JSON
3. Result of `supports_color()` (NO_COLOR env var + TTY detection)

### Theme Loading

When colors are enabled, themes are loaded with this priority:

1. **CLI argument**: `--color-scheme SCHEME_NAME`
2. **User config**: `~/.config/lash/config.toml` with `color_scheme` field
3. **Default**: "Base2Tone Desert"

If colors are disabled, `CliTheme::load()` returns `None`, and all formatting methods return unstyled text.

## Command Integration

All commands that produce colored output respect the color settings:

### Text Commands
- `lash list` - Task lists with colored status badges
- `lash search` - Search results with highlighted matches
- `lash show` - File contents with syntax highlighting
- `lash lint` - Validation errors with colored severity levels
- `lash index` - Progress reports with status colors
- `lash check-index` - Verification reports with status indicators
- `lash graph` - Graph output (when using TTY)
- `lash check-links` - Link validation with colored results

### JSON Commands
These commands output JSON and never include ANSI codes:
- `lash --json list`
- `lash --json search QUERY`
- `lash --json index`
- Any command with `--json` global flag

## Usage Examples

### Disable colors with flag
```bash
lash --no-color list
```

### Disable colors with environment variable
```bash
NO_COLOR=1 lash list
```

### Force JSON (no colors)
```bash
lash --json list
```

### Piped output (auto-disables colors)
```bash
lash list > output.txt  # No ANSI codes in file
lash list | grep "backend"  # No ANSI codes in pipe
```

### Override color scheme (but still respect no-color)
```bash
# Uses Dracula theme
lash --color-scheme Dracula list

# --no-color takes priority over color scheme
lash --no-color --color-scheme Dracula list  # Still no colors
```

## TextFormatter Behavior

The `TextFormatter` struct handles color application:

```rust
pub struct TextFormatter {
    theme: Option<CliTheme>,
    verbosity: Verbosity,
}
```

- If `theme` is `None`, all styling methods return unstyled text
- If `theme` is `Some(theme)`, styling methods apply colors using `owo-colors`

Example methods:
- `format_success(msg)` - Green for success
- `format_error(msg)` - Red for errors
- `format_warning(msg)` - Yellow for warnings
- `format_info(msg)` - Blue for info
- `format_task_status(text, status)` - Status-specific colors
- `format_label(label)` - Cyan for labels/tags
- `format_muted(text)` - Gray for secondary text

## Testing

Comprehensive integration tests verify color handling in `/Users/fohara/src/lash/crates/lash-cli/tests/color_handling_test.rs`:

1. `test_no_color_flag_disables_colors` - Verifies `--no-color` works
2. `test_no_color_env_var_disables_colors` - Verifies `NO_COLOR` env var
3. `test_no_color_flag_overrides_color_scheme` - Verifies priority
4. `test_json_output_never_has_colors` - Ensures JSON has no ANSI
5. `test_json_overrides_color_scheme` - JSON takes priority over colors
6. `test_list_command_respects_no_color` - List command compliance
7. `test_search_command_respects_no_color` - Search command compliance
8. `test_index_command_respects_no_color` - Index command compliance
9. `test_check_index_command_respects_no_color` - Check-index compliance
10. `test_show_command_respects_no_color` - Show command compliance
11. `test_no_color_env_var_priority` - NO_COLOR overrides color scheme

All tests verify that output contains no ANSI escape codes (`\x1b[`) when colors should be disabled.

## Compliance

Lash follows standard Unix conventions for color output:

- **NO_COLOR standard**: Respects the `NO_COLOR` environment variable as defined at [no-color.org](https://no-color.org/)
- **TTY detection**: Automatically disables colors for non-interactive output
- **Explicit control**: Users can always force disable with `--no-color`
- **JSON safety**: JSON output never contains ANSI codes for machine readability

## Implementation Files

Key files implementing color handling:

- `/Users/fohara/src/lash/crates/lash-cli/src/theme.rs` - Theme and color support
- `/Users/fohara/src/lash/crates/lash-cli/src/formatter.rs` - Output formatting with colors
- `/Users/fohara/src/lash/crates/lash-cli/src/main.rs` - Main color decision logic
- `/Users/fohara/src/lash/crates/lash-cli/tests/color_handling_test.rs` - Comprehensive tests

All commands consistently use these modules to ensure uniform color handling across the entire CLI.
