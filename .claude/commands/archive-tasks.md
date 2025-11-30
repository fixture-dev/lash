---
description: Archive completed tasks from a task file to archived version
args:
  - name: file_path
    description: Path to the task file to archive (e.g., tasks/tasks.reasoning.md)
    required: true
---

Please review {{file_path}} and comprehensively cut all sections from the file which contain completed work. The removed sections should then be copied into a file which follows this naming convention: `tasks/archived/archived.{original file name}`. These edits should result in a streamlined original task file which only contains scheduled but not yet completed tasks.

**Guidelines for archiving:**

1. **Identify completed sections:**
   - Look for sections marked with ✅, COMPLETE, or similar completion indicators
   - Include the entire section with all subsections and metadata
   - Preserve the full context including acceptance criteria, files modified/created, test results, and completion dates

2. **Archive file structure:**
   - Create `tasks/archived/` directory if it doesn't exist
   - Archive file should be named `archived.{original_filename}` (e.g., `tasks/archived/archived.tasks.reasoning.md`)
   - Append archived sections to the end of the archive file (newest first)
   - Add a timestamp header when archiving: `## Archived on {YYYY-MM-DD}`
   - Preserve all markdown formatting, code blocks, and links

3. **Original file cleanup:**
   - Remove completed sections entirely
   - Update any references or summaries to point to archived file
   - Add brief summary of archived work with link to archive
   - Preserve section structure and numbering for remaining tasks

4. **Preservation requirements:**
   - Keep all git commit references
   - Keep all file paths and line numbers
   - Keep test coverage statistics
   - Keep completion dates and implementation notes

**Example transformation:**

Original file section:
```markdown
### Task 2C: Implement Causal Analysis ✅ COMPLETE (2025-11-13)
[... full implementation details, tests, files created ...]
```

After archiving:
- Original file: Replace with summary referencing archive
- Archive file: Full section moved to `tasks/archived/archived.tasks.reasoning.md` with timestamp

Please perform this archiving operation now for {{file_path}}.
