# Agent Integration Tasks

**Module:** `lash-agent`
**Dependencies:** tasks.core-data-model.md, tasks.dependency-resolution.md, tasks.cli-framework.md
**Effort:** 6-8 days
**Priority:** HIGH

## Overview

Make Lash optimally usable by AI agents (like Claude Code) through context-minimized prompts, structured output, and token-aware utilities. This module focuses on reducing token usage while maximizing agent effectiveness.

## Core Requirements

From design-doc.md section 11:
- Generate agent usage prompts (section 11.3)
- Token minimization strategies (section 11.2)
- Schema-first approach (section 11.2.1)
- Sparse context generation (section 11.2.2)
- ID-based references (section 11.2.4)

---

## Task 1: Schema Generation

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.core-data-model.md#1

### Description

Generate machine-readable and human-readable schema documentation for the Lash task format.

### Subtasks

- [ ] Define schema representation
  - [ ] Task file structure (header, tasks, references)
  - [ ] Annotation types (`@id`, `@labels`, etc.)
  - [ ] Checkbox status values
  - [ ] Dependency reference formats
  - [ ] Depth limits and constraints
- [ ] Implement schema serialization
  - [ ] Plain text format (markdown)
  - [ ] JSON Schema format
  - [ ] Type definitions (for TypeScript/Python agents)
- [ ] Generate schema examples
  - [ ] Minimal valid file
  - [ ] File with all annotation types
  - [ ] File with dependencies
  - [ ] Keep examples small (token-efficient)
- [ ] Document allowed operations
  - [ ] Adding new tasks
  - [ ] Updating task status
  - [ ] Adding dependencies
  - [ ] Waiving tasks
- [ ] Document constraints and rules
  - [ ] Unique IDs within file
  - [ ] Depth limits
  - [ ] Status consistency (parents vs children)
  - [ ] Valid dependency reference formats

### Success Criteria

- Schema is complete and accurate
- Examples are minimal yet comprehensive
- Operations are clearly documented
- Agents can understand format from schema alone

### Tests

- Unit: Validate schema completeness
- Manual: Review schema for clarity
- Integration: Generate schema and inspect output

---

## Task 2: Prompt Template System

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1

### Description

Implement a template system for generating agent prompts with configurable content and format.

### Subtasks

- [ ] Define `PromptTemplate` struct
  - [ ] Template sections (schema, operations, examples, tasks)
  - [ ] Configurable inclusion/exclusion
  - [ ] Token budget tracking
- [ ] Implement template rendering
  - [ ] Plain text format (default)
  - [ ] JSON format (structured)
  - [ ] Claude skill format (future)
- [ ] Define template sections
  - [ ] **Schema section**: File format specification
  - [ ] **Operations section**: How to safely modify files
  - [ ] **Examples section**: Small, representative examples
  - [ ] **Tasks section**: Filtered task list for agent
  - [ ] **Context section**: Current project status
- [ ] Implement section prioritization
  - [ ] Schema (critical, always include)
  - [ ] Operations (critical, always include)
  - [ ] Examples (important, include if budget allows)
  - [ ] Tasks (variable, truncate if needed)
  - [ ] Context (optional, summarize if limited budget)
- [ ] Add token budget enforcement
  - [ ] Estimate token count per section (rough heuristic)
  - [ ] Truncate low-priority sections to fit budget
  - [ ] Warn if budget insufficient for critical sections
- [ ] Implement prompt customization
  - [ ] Filter tasks by owner
  - [ ] Filter tasks by label
  - [ ] Filter tasks by file path
  - [ ] Include/exclude specific sections

### Success Criteria

- Prompts are clear, concise, and actionable
- Token budget is respected
- Customization options work correctly
- Generated prompts enable agent to use Lash safely

### Tests

- Unit: Test template rendering
- Unit: Test token budget enforcement
- Unit: Test section prioritization
- Integration: Generate prompts with various configurations
- Manual: Review generated prompts for quality

---

## Task 3: Token Minimization Utilities

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, Task 2

### Description

Implement utilities for minimizing token usage in agent contexts.

### Subtasks

- [ ] Implement token counter
  - [ ] Use rough heuristic (words * 1.3) or simple tokenizer
  - [ ] Count tokens for text strings
  - [ ] Track cumulative token usage
- [ ] Implement task summarizer
  - [ ] Generate short summary of task file:
    - [ ] "File: X, Y tasks, Z% complete, A blockers"
  - [ ] Bullet list format for multiple tasks
  - [ ] Configurable detail level (terse, normal, verbose)
- [ ] Implement dependency summarizer
  - [ ] "Task X depends on 3 tasks in file Y (2 done, 1 blocked)"
  - [ ] List only critical blockers
  - [ ] Use ID references instead of full titles
- [ ] Implement ID-based reference system
  - [ ] Assign short, stable IDs to tasks
  - [ ] Use IDs in prompts instead of full descriptions
  - [ ] Provide ID lookup table if needed
- [ ] Implement context window manager
  - [ ] Track total context size
  - [ ] Prioritize information by importance
  - [ ] Truncate or summarize low-priority info
  - [ ] Warn if critical info omitted
- [ ] Add compression strategies
  - [ ] Remove unnecessary whitespace (while preserving structure)
  - [ ] Abbreviate repetitive annotations
  - [ ] Use compact JSON formats

### Success Criteria

- Token counting is reasonably accurate
- Summaries are informative yet concise
- ID references reduce token usage significantly
- Context manager effectively prioritizes content

### Tests

- Unit: Test token counter accuracy
- Unit: Test summarizer output quality
- Unit: Test context manager prioritization
- Integration: Generate token-minimized prompts, verify budget

---

## Task 4: Sparse Context Generation

**Priority:** HIGH
**Effort:** 2 days
**Depends on:** Task 3, tasks.dependency-resolution.md#1-4

### Description

Implement sparse context generation that includes only relevant information for a given agent task.

### Subtasks

- [x] Define `ContextBuilder` struct
  - [x] Target task or file
  - [x] Dependency graph
  - [x] Token budget
  - [x] Inclusion rules
- [x] Implement context selection algorithm
  - [x] Include target task/file (full detail)
  - [x] Include direct dependencies (summaries)
  - [x] Include blockers (full detail)
  - [x] Exclude completed dependencies (unless blocking)
  - [x] Exclude unrelated files
- [x] Implement dependency context
  - [x] For each dependency:
    - [x] Include ID, title, status
    - [x] Include file path
    - [x] Omit body and subtasks (unless blocker)
  - [x] Show dependency chain for blockers
- [x] Implement file context
  - [x] Include headers and metadata
  - [x] Include relevant task subtree
  - [x] Omit completed sibling branches
- [x] Add expansion/contraction logic
  - [x] If under budget: expand summaries to full details
  - [x] If over budget: contract full details to summaries
  - [x] Iteratively adjust until budget met
- [x] Generate context output
  - [x] Format as markdown or JSON
  - [x] Include section headers for clarity
  - [x] Add "context note" explaining what's included/excluded

### Success Criteria

- Sparse context includes all necessary information
- Token usage is minimal (typically 50-80% of full context)
- Agent can work effectively with sparse context
- Blockers are never omitted

### Tests

- [x] Unit: Test context selection algorithm
- [x] Unit: Test expansion/contraction logic
- [x] Integration: Generate sparse context for fixture tasks
- [x] Integration: Verify token budget is respected
- [x] Manual: Review contexts for completeness

---

## Task 5: Agent Prompt Command Implementation

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** Task 1-4, tasks.cli-commands.md#10

### Description

Integrate all agent utilities into the `lash agent-prompt` command (implementation details).

### Subtasks

- [ ] Implement prompt generation logic
  - [ ] Load schema
  - [ ] Load templates
  - [ ] Filter tasks based on flags
  - [ ] Build context
  - [ ] Render prompt
  - [ ] Output to stdout or file
- [ ] Implement format options
  - [ ] Plain text: human-readable markdown
  - [ ] JSON: structured data for programmatic access
  - [ ] Claude skill: JSON/YAML skill spec (future)
- [ ] Implement filtering options
  - [ ] `--for-owner <name>`: filter by owner
  - [ ] `--labels <labels>`: filter by labels
  - [ ] `--path <path>`: filter by file path
  - [ ] `--blocked`: show only blocked tasks
  - [ ] `--ready`: show only ready-to-start tasks
- [ ] Implement token budget option
  - [ ] `--token-budget <n>`: enforce token limit
  - [ ] Default: no limit (include everything)
  - [ ] Show warning if budget insufficient
- [ ] Implement example inclusion
  - [ ] `--include-examples`: add example task files
  - [ ] `--examples-only`: schema + examples (no project tasks)
- [ ] Add validation
  - [ ] Verify generated prompt is well-formed
  - [ ] Check for required sections
  - [ ] Warn if critical info omitted

### Success Criteria

- Command generates useful prompts for agents
- All flags work as specified
- Output formats are correct
- Token budgets are enforced

### Tests

- Integration: Generate prompt with various flag combinations
- Integration: Test token budget limiting
- Integration: Validate JSON output
- Manual: Use generated prompt with actual agent, assess quality

---

## Task 6: Agent Workflow Examples

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** Task 5

### Description

Create example workflows and documentation for how agents should use Lash.

### Subtasks

- [ ] Document agent workflow
  - [ ] Step 1: Call `lash agent-prompt` to get instructions
  - [ ] Step 2: Read relevant task files
  - [ ] Step 3: Modify files (add/update tasks)
  - [ ] Step 4: Call `lash lint` to validate changes
  - [ ] Step 5: Call `lash index` to update DB (if needed)
- [ ] Create example prompts
  - [ ] Minimal prompt (schema only)
  - [ ] Task-focused prompt (for specific agent)
  - [ ] Exploration prompt (understand project structure)
- [ ] Document safety guidelines
  - [ ] Always lint before committing
  - [ ] Don't modify files outside project
  - [ ] Respect depth limits
  - [ ] Don't break dependency references
- [ ] Create agent error recovery guide
  - [ ] What to do if lint fails
  - [ ] How to fix broken dependencies
  - [ ] How to resolve cycles
- [ ] Add integration examples
  - [ ] Claude Code usage pattern
  - [ ] Custom script integration
  - [ ] CI/CD integration

### Success Criteria

- Workflows are clear and actionable
- Examples are realistic and helpful
- Safety guidelines prevent common mistakes
- Error recovery guide is comprehensive

### Tests

- Manual: Follow workflows with actual agent
- Manual: Test error recovery procedures
- Documentation review by users

---

## Task 7: Agent Feedback and Telemetry (Optional)

**Priority:** LOW
**Effort:** 1-2 days
**Depends on:** Task 5

### Description

Add optional telemetry to track agent usage and improve prompts over time.

### Subtasks

- [ ] Define telemetry events
  - [ ] Agent prompt generated (format, filters, token count)
  - [ ] Agent command executed (lint, index, etc.)
  - [ ] Agent errors (lint failures, broken deps)
- [ ] Implement telemetry collection
  - [ ] Opt-in (disabled by default)
  - [ ] Store locally in `.lash/telemetry.jsonl`
  - [ ] No external reporting (privacy-preserving)
- [ ] Add telemetry analysis tools
  - [ ] `lash telemetry stats`: show usage summary
  - [ ] `lash telemetry report`: generate analysis
  - [ ] Identify common agent errors
  - [ ] Suggest prompt improvements
- [ ] Implement privacy controls
  - [ ] No file content or task details (only metadata)
  - [ ] Clear opt-in consent
  - [ ] Easy opt-out
  - [ ] Data stays local

### Success Criteria

- Telemetry is opt-in and privacy-preserving
- Data is useful for improving agent integration
- Analysis tools provide actionable insights

### Tests

- Integration: Test telemetry collection
- Integration: Test analysis tools
- Manual: Review telemetry data for privacy compliance

---

## Non-Goals (for v1)

- Real-time agent collaboration (agents work independently)
- Agent authentication or access control
- Multi-agent coordination
- Agent-specific task assignment UI
- Integration with specific agent platforms (beyond examples)

---

## Open Questions

- **Token budget:** Default value? (Suggest 2000-4000 tokens for schema + examples)
- **Prompt format:** Markdown vs plain text for default?
- **ID format:** UUID, hash, or sequential numbers?
- **Telemetry:** Include in v1 or defer to v2?

---

## References

- Design doc section 11 (Agent Integration & Token Minimization)
- Design doc section 11.3 (Prompt Generation Command)
- tasks.cli-commands.md#10 (`lash agent-prompt` command spec)
