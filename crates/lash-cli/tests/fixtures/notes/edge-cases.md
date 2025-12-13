# Edge Cases for Contextual Notes

@id: notes.edge-cases
@labels: testing, edge-cases
@status: in-progress
@created: 2025-12-13

## Tasks

- [ ] Task with many notes
  - First note with basic information
  - Second note with more details
  - Third note with additional context
  - Fourth note with requirements
  - Fifth note with acceptance criteria
  - [ ] Subtask after many notes

- [ ] Task with very long note
  - This is a very long contextual note that provides extensive information about the requirements and implementation details for this particular task and it should test how the system handles notes that approach or exceed the warning threshold of 200 characters which is configured in the linter rules
  - [ ] Subtask with short note
    - Brief implementation detail

- [ ] Multiple notes before and after subtasks
  - First set of notes before any subtasks
  - More context about the parent task
  - [ ] First subtask
  - Note between subtasks is invalid and should be caught by linter
  - [ ] Second subtask
    - Note for second subtask
  - Final note after all subtasks is also invalid

- [ ] Task with notes at maximum depth
  - Parent level note
  - [ ] First child
    - Child level note
    - [ ] Second child
      - Grandchild level note

- [ ] Empty task with no notes or subtasks

- [x] Completed task with notes
  - Notes should be preserved for completed tasks
  - Historical context is valuable

- [-] Waived task with notes
  - Explanation of why this was waived
  - Original requirements are documented here
