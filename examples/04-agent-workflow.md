# Agent-Driven Development Workflow

@id: agent-workflow-demo
@labels: agent, example, documentation
@status: in-progress
@created: 2025-12-14

## Description

This file demonstrates how AI agents (like Claude Code) can effectively use Lash for task tracking. It shows agent-specific patterns, the `lash agent-prompt` command, and how agents should update tasks.

Key agent workflows:
- Querying relevant tasks before starting work
- Marking tasks complete after implementation
- Adding new discovered tasks
- Using contextual notes for future context

@agent-note: This file is self-documenting. Agents should follow the patterns shown here when working with Lash task files.

## Tasks

### Agent Setup & Discovery

These tasks show how an agent should orient itself in a project.

- [x] Understand project structure
  - Agent should run `lash list` first to see available tasks
  - Use `lash show <path>` to drill into specific files
  - Check `lash graph` to understand dependencies
  - [x] Scan root index file
  - [x] Identify relevant task files
  - [x] Understand label taxonomy

- [x] Get focused task context #agent
  - The `lash agent-prompt` command generates agent-optimized context
  - Includes only relevant tasks and dependencies
  - Token-minimized for efficiency
  - [x] Run `lash agent-prompt --label agent --label p0`
  - [x] Review generated prompt structure
  - [x] Understand task format specification

### Implementing Features

These tasks show the agent workflow for feature implementation.

- [ ] Implement user authentication #agent #p0
  - Agent workflow: read spec → implement → test → mark done
  - Always validate changes with `lash lint` before marking complete
  - Add contextual notes for future agents/developers
  - [ ] Read authentication requirements #agent
    - Requirements stored in docs/auth-spec.md
    - Must support email/password and OAuth
    - JWT tokens with 24-hour expiry
    - @agent-note: Check existing auth implementation at src/auth/ before starting
  - [ ] Write authentication code #agent
    - Use bcrypt for password hashing (cost factor 12)
    - Store JWT secret in environment variable
    - Implement rate limiting (5 attempts per 15 min)
  - [ ] Add authentication tests #agent
    - Test both success and failure cases
    - Mock external OAuth providers
    - Coverage target: >80% for auth module
  - [ ] Document authentication API #agent
    - Update OpenAPI spec at docs/api.yaml
    - Add usage examples to README
    - Include security considerations

### Agent Best Practices

Tasks demonstrating how agents should manage their work.

- [x] Check tasks before starting work #agent
  - Prevents duplicate work and conflicts
  - Command: `lash list --label agent --status open`
  - Shows what's already in progress
  - [x] Query open agent tasks
  - [x] Check for related completed work
  - [x] Identify blocking dependencies

- [ ] Update tasks as work progresses #agent
  - Mark subtasks `[x]` as they complete
  - Parent tasks auto-complete when all children done
  - Always run `lash lint` to validate format
  - [x] Mark subtasks complete incrementally
  - [ ] Add contextual notes for context
    - Notes help future agents understand decisions
    - Include rationale for non-obvious choices
    - Reference external docs or issues
  - [ ] Validate with linter before finalizing

- [ ] Add discovered tasks #agent
  - During implementation, agents often discover new work
  - Add as subtasks with appropriate labels
  - Include contextual notes for context
  - Example: Found edge case during testing
  - Example: Discovered missing error handling
  - Example: Identified performance optimization opportunity

### Agent Prompt Generation

These tasks show how to use `lash agent-prompt` effectively.

- [x] Generate minimal context prompts #agent
  - Basic usage: `lash agent-prompt`
  - Returns task format specification
  - Includes safe operations guide
  - [x] Test default prompt generation
  - [x] Verify schema accuracy

- [x] Filter prompts by label #agent
  - Target specific work: `lash agent-prompt --label backend`
  - Multiple labels: `lash agent-prompt --label agent --label p0`
  - Shows only relevant tasks
  - [x] Generate backend-specific prompt
  - [x] Generate high-priority prompt

- [ ] Include examples in prompts #agent
  - Add `--include-examples` flag for sample files
  - Useful when agent is unfamiliar with format
  - Increases token usage but improves accuracy
  - [x] Generate prompt with examples
  - [ ] Compare token usage with/without examples
  - [ ] Determine optimal example count

### Error Handling & Recovery

How agents should handle errors and validation failures.

- [x] Handle lint errors gracefully #agent
  - Always run `lash lint` after editing task files
  - Parse JSON output with `--json` flag
  - Fix errors before marking tasks complete
  - [x] Implement lint error detection
  - [x] Parse error messages
  - [x] Auto-fix common issues

- [ ] Recover from broken dependencies #agent
  - Error: `@depends-on` points to non-existent task
  - Solution: Run `lash check-links` to find issues
  - Either fix the reference or remove dependency
  - [x] Detect broken dependency
  - [ ] Fix dependency reference
  - [ ] Validate fix with check-links

- [ ] Handle merge conflicts #agent
  - Task files may conflict when multiple agents work in parallel
  - Resolution strategy: preserve both changes, reconcile manually
  - Use `lash lint` to validate merged result

## Example Agent Workflow

Here's a complete example of an agent implementing a feature:

### Step 1: Get Context
```bash
# Agent queries for relevant work
lash agent-prompt --label agent --label backend --format json
```

### Step 2: Read Task File
Agent reads the task file to understand requirements:
```bash
lash show features/authentication.md
```

### Step 3: Implement Feature
Agent writes code, following contextual notes and requirements.

### Step 4: Test Implementation
Agent runs tests to verify correctness.

### Step 5: Update Task File
Agent marks completed subtasks:

```markdown
- [ ] Implement user authentication #agent #p0
  - [x] Read authentication requirements #agent
  - [x] Write authentication code #agent
  - [x] Add authentication tests #agent
  - [ ] Document authentication API #agent
```

### Step 6: Validate Changes
```bash
# Ensure task file is valid
lash lint features/authentication.md

# Verify no broken dependencies
lash check-links
```

### Step 7: Add Discovered Work (if needed)
If agent finds additional work needed:

```markdown
- [ ] Implement user authentication #agent #p0
  - [x] Read authentication requirements #agent
  - [x] Write authentication code #agent
  - [x] Add authentication tests #agent
  - [ ] Document authentication API #agent
  - [ ] Add rate limiting middleware #agent
    - Discovered during security review
    - Prevent brute force attacks
    - Use Redis for distributed rate limiting
```

## Agent-Specific Labels

Use these labels to help agents find relevant work:

- `#agent` - Tasks suitable for agent execution
- `#agent-review` - Tasks needing agent review before human approval
- `#agent-blocked` - Tasks blocked on human input
- `#agent-skip` - Tasks that should not be automated

## Token Minimization Strategies

Agents should minimize token usage:

1. **Use `lash agent-prompt`** instead of reading full files
   - Generates minimal, focused context
   - Includes only relevant tasks and dependencies

2. **Use task IDs** instead of full descriptions
   - Reference tasks by ID: `auth.login#task:implement-jwt`
   - Saves tokens in agent-to-agent communication

3. **Filter by labels** to reduce context
   - `lash list --label agent --label p0`
   - Shows only high-priority agent work

4. **Use JSON output** for programmatic parsing
   - `lash list --json` for structured data
   - Easier to process than human-readable format

## Common Agent Mistakes to Avoid

### Don't: Skip validation
```markdown
# Agent marks task complete without running lash lint
# Later: linter finds errors, task must be redone
```

### Do: Always validate
```markdown
# Agent completes work, runs lash lint, confirms success
# Task is properly marked complete with valid format
```

### Don't: Create tasks without context
```markdown
- [ ] Fix bug #agent
```

### Do: Add contextual notes
```markdown
- [ ] Fix authentication bug #agent
  - Bug: JWT tokens expire immediately instead of after 24h
  - Root cause: TTL set to 0 in config
  - Fix: Update config/auth.yaml line 42
```

### Don't: Ignore dependencies
```markdown
# Agent starts work without checking dependencies
# Later: discovers dependency not met, work wasted
```

### Do: Check dependencies first
```markdown
# Agent runs: lash show features/authentication.md
# Sees: @depends-on: features/database.md
# Checks: lash show features/database.md
# Confirms: dependency complete, safe to proceed
```

## Summary

Agents working with Lash should:
1. Use `lash agent-prompt` for focused context
2. Always validate with `lash lint` before committing
3. Add contextual notes for future context
4. Mark tasks incrementally as work completes
5. Check dependencies before starting work
6. Use labels to filter relevant tasks
7. Handle errors gracefully and report issues

This workflow ensures agents work efficiently while maintaining task file integrity and providing value to human developers.
