# Lash Demo Recordings

This directory contains [VHS](https://github.com/charmbracelet/vhs) tape files for creating scripted terminal recordings that demonstrate Lash functionality.

## Prerequisites

1. **Install VHS** (requires Go 1.18+):
   ```bash
   # macOS
   brew install vhs

   # Or via Go
   go install github.com/charmbracelet/vhs@latest
   ```

2. **Install ffmpeg** (required for video output):
   ```bash
   brew install ffmpeg
   ```

3. **Ensure `lash` is in your PATH**:
   ```bash
   cargo install --path .
   # Or add an alias: alias lash='cargo run --quiet --'
   ```

## Creating Recordings

### Record from a tape file

```bash
# From the project root
vhs demos/lash-demo.tape
```

This will generate the output files specified in the tape (GIF and/or MP4).

### Validate a tape without running

```bash
vhs validate demos/lash-demo.tape
```

### Create a new tape interactively

```bash
# Record your terminal session and generate a tape file
vhs record -o demos/my-recording.tape
```

## Output Files

After running `vhs demos/lash-demo.tape`, you'll find:

- `demos/lash-demo.gif` - Animated GIF (good for GitHub READMEs)
- `demos/lash-demo.mp4` - MP4 video (higher quality, smaller size)

## Playing Recordings

### GIF
Open in any image viewer or embed in Markdown:
```markdown
![Lash Demo](demos/lash-demo.gif)
```

### MP4
```bash
# macOS
open demos/lash-demo.mp4

# Or use any video player
mpv demos/lash-demo.mp4
vlc demos/lash-demo.mp4
```

### Publish to vhs.charm.sh

Share your recording with a public URL:
```bash
vhs demos/lash-demo.tape --publish
```

## Tape File Reference

VHS tapes use a simple DSL. Key commands:

| Command | Description |
|---------|-------------|
| `Output <path>` | Set output file (`.gif`, `.mp4`, `.webm`) |
| `Set Theme <name>` | Terminal theme (run `vhs themes` for list) |
| `Set TypingSpeed <time>` | Delay between keystrokes (e.g., `50ms`) |
| `Type "<text>"` | Type characters into terminal |
| `Enter` | Press Enter key |
| `Sleep <time>` | Pause (e.g., `2s`, `500ms`) |
| `Hide` / `Show` | Hide/show commands in output |
| `# comment` | Comment (not shown in output) |

See full documentation: https://github.com/charmbracelet/vhs

## Available Demos

| File | Description |
|------|-------------|
| `lash-demo.tape` | Comprehensive tour of Lash features (~20s) |

### lash-demo.tape Contents

The main demo showcases these commands in sequence:

1. **`lash status`** - Project-wide task summary with completion stats
2. **`lash show`** - Detailed view of a specific task file with metadata
3. **`lash search`** - Fuzzy search across all tasks with relevance scoring
4. **`lash explain`** - Detailed error code documentation
5. **`lash tui`** - Interactive terminal UI for task management

## Tips for Creating Good Demos

1. **Keep it short** - Aim for 10-30 seconds
2. **Use comments** - Explain what each section demonstrates
3. **Add pauses** - Give viewers time to read output
4. **Hide setup** - Use `Hide`/`Show` for cd, clear, etc.
5. **Test first** - Run commands manually before recording
