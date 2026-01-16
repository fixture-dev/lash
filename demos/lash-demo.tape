# Lash Demo - A comprehensive tour of Lash's core features
# This tape demonstrates key functionality including CLI commands and TUI

# Output settings
Output demos/lash-demo.gif
Output demos/lash-demo.mp4

# Terminal settings
Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 650
Set Theme "Dracula"
Set TypingSpeed 35ms
Set Padding 20

# Require lash to be installed
Require lash

# Navigate to the playground
Hide
Type "cd ~/src/lash/playground && clear"
Enter
Sleep 500ms
Show

# --- Demo 1: Project Status Overview ---
# View project-wide task summary with status breakdown
Type "lash status"
Sleep 300ms
Enter
Sleep 1.5s
Hide
Type "clear"
Enter
Show

# --- Demo 2: Show File Details ---
# Display detailed info about a specific task file
Type "lash show features/player-movement.md"
Sleep 300ms
Enter
Sleep 2s
Hide
Type "clear"
Enter
Show

# --- Demo 3: Fuzzy Search ---
# Search tasks across all files with relevance scoring
Type "lash search 'boss'"
Sleep 300ms
Enter
Sleep 1.5s
Hide
Type "clear"
Enter
Show

# --- Demo 4: Error Explanation ---
# Get detailed help for any error code
Type "lash explain E_PARSE_INVALID_CHECKBOX"
Sleep 300ms
Enter
Sleep 2s
Hide
Type "clear"
Enter
Show

# --- Demo 5: Interactive TUI ---
# Launch the full-featured terminal UI for task management
Type "lash tui"
Sleep 300ms
Enter
Sleep 1s

# Navigate down and open task detail modal
Tab 2
Down 6
Sleep 500ms
Enter
Sleep 4s

# Exit TUI
Type "q"
Sleep 500ms
