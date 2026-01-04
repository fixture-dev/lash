# Mixed Task and Note Hierarchy

@id: notes.mixed
@labels: testing, hierarchy
@created: 2025-12-13

## Description

This file tests various patterns of tasks and notes mixed together.

## Tasks

- [ ] Pattern 1: Notes then tasks
  - Note A for pattern 1
  - Note B for pattern 1
  - [ ] Subtask A
  - [ ] Subtask B

- [ ] Pattern 2: Tasks then notes (invalid)
  - [ ] Subtask A
  - Note after task (this violates ordering convention)

- [ ] Pattern 3: Alternating (invalid)
  - Note A
  - [ ] Subtask A
  - Note B (between tasks, invalid)
  - [ ] Subtask B
  - Note C (after all tasks, invalid)

- [ ] Pattern 4: Notes at each level
  - Top level note
  - [ ] Child task
    - Child note A
    - Child note B
    - [ ] Grandchild task
      - Grandchild note

- [ ] Pattern 5: Only notes, no subtasks
  - Just a note providing context
  - Another note with requirements
  - Final note with acceptance criteria

- [ ] Pattern 6: Deeply nested with notes
  - Root note
  - [ ] Level 1 task
    - Level 1 note
    - [ ] Level 2 task
      - Level 2 note

- [ ] Pattern 7: Multiple consecutive notes
  - Note 1
  - Note 2
  - Note 3
  - Note 4
  - Note 5

- [ ] Pattern 8: Notes with special characters
  - Note with "quotes" and 'apostrophes'
  - Note with [markdown](https://example.com) link
  - Note with `code` formatting
  - Note with **bold** and *italic* text

- [ ] Pattern 9: Empty parent with child that has notes
  - [ ] Child with notes
    - Child note 1
    - Child note 2
