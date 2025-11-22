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

## Task 1: Schema Generation ✅

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** tasks.core-data-model.md#1
**Status:** COMPLETE

### Description

Generate machine-readable and human-readable schema documentation for the Lash task format.

### Subtasks

- [x] Define schema representation
  - [x] Task file structure (header, tasks, references)
  - [x] Annotation types (`@id`, `@labels`, etc.)
  - [x] Checkbox status values
  - [x] Dependency reference formats
  - [x] Depth limits and constraints
- [x] Implement schema serialization
  - [x] Plain text format (markdown)
  - [x] JSON Schema format
  - [x] Type definitions (for TypeScript/Python agents)
- [x] Generate schema examples
  - [x] Minimal valid file
  - [x] File with all annotation types
  - [x] File with dependencies
  - [x] Keep examples small (token-efficient)
- [x] Document allowed operations
  - [x] Adding new tasks
  - [x] Updating task status
  - [x] Adding dependencies
  - [x] Waiving tasks
- [x] Document constraints and rules
  - [x] Unique IDs within file
  - [x] Depth limits
  - [x] Status consistency (parents vs children)
  - [x] Valid dependency reference formats

### Success Criteria

- ✅ Schema is complete and accurate
- ✅ Examples are minimal yet comprehensive
- ✅ Operations are clearly documented
- ✅ Agents can understand format from schema alone

### Tests

- ✅ Unit: Validate schema completeness
- ✅ Manual: Review schema for clarity
- ✅ Integration: Generate schema and inspect output

### Implementation

Implemented in `crates/lash-agent/src/schema.rs` with comprehensive test coverage.

---

## Task 2: Prompt Template System ✅

**Priority:** CRITICAL
**Effort:** 2-3 days
**Depends on:** Task 1
**Status:** COMPLETE

### Description

Implement a template system for generating agent prompts with configurable content and format.

### Subtasks

- [x] Define `PromptTemplate` struct
  - [x] Template sections (schema, operations, examples, tasks)
  - [x] Configurable inclusion/exclusion
  - [x] Token budget tracking
- [x] Implement template rendering
  - [x] Plain text format (default)
  - [x] JSON format (structured)
  - [x] Claude skill format (future)
- [x] Define template sections
  - [x] **Schema section**: File format specification
  - [x] **Operations section**: How to safely modify files
  - [x] **Examples section**: Small, representative examples
  - [x] **Tasks section**: Filtered task list for agent
  - [x] **Context section**: Current project status
- [x] Implement section prioritization
  - [x] Schema (critical, always include)
  - [x] Operations (critical, always include)
  - [x] Examples (important, include if budget allows)
  - [x] Tasks (variable, truncate if needed)
  - [x] Context (optional, summarize if limited budget)
- [x] Add token budget enforcement
  - [x] Estimate token count per section (rough heuristic)
  - [x] Truncate low-priority sections to fit budget
  - [x] Warn if budget insufficient for critical sections
- [x] Implement prompt customization
  - [x] Filter tasks by owner
  - [x] Filter tasks by label
  - [x] Filter tasks by file path
  - [x] Include/exclude specific sections

### Success Criteria

- ✅ Prompts are clear, concise, and actionable
- ✅ Token budget is respected
- ✅ Customization options work correctly
- ✅ Generated prompts enable agent to use Lash safely

### Tests

- ✅ Unit: Test template rendering
- ✅ Unit: Test token budget enforcement
- ✅ Unit: Test section prioritization
- ✅ Integration: Generate prompts with various configurations
- ✅ Manual: Review generated prompts for quality

### Implementation

Implemented in `crates/lash-agent/src/prompt.rs` with comprehensive template system and token budget enforcement.

---

## Task 3: Token Minimization Utilities ✅

**Priority:** HIGH
**Effort:** 2-3 days
**Depends on:** Task 1, Task 2
**Status:** COMPLETE

### Description

Implement utilities for minimizing token usage in agent contexts.

### Subtasks

- [x] Implement token counter
  - [x] Use rough heuristic (words * 1.3) or simple tokenizer
  - [x] Count tokens for text strings
  - [x] Track cumulative token usage
- [x] Implement task summarizer
  - [x] Generate short summary of task file:
    - [x] "File: X, Y tasks, Z% complete, A blockers"
  - [x] Bullet list format for multiple tasks
  - [x] Configurable detail level (terse, normal, verbose)
- [x] Implement dependency summarizer
  - [x] "Task X depends on 3 tasks in file Y (2 done, 1 blocked)"
  - [x] List only critical blockers
  - [x] Use ID references instead of full titles
- [x] Implement ID-based reference system
  - [x] Assign short, stable IDs to tasks
  - [x] Use IDs in prompts instead of full descriptions
  - [x] Provide ID lookup table if needed
- [x] Implement context window manager
  - [x] Track total context size
  - [x] Prioritize information by importance
  - [x] Truncate or summarize low-priority info
  - [x] Warn if critical info omitted
- [x] Add compression strategies
  - [x] Remove unnecessary whitespace (while preserving structure)
  - [x] Abbreviate repetitive annotations
  - [x] Use compact JSON formats

### Success Criteria

- ✅ Token counting is reasonably accurate
- ✅ Summaries are informative yet concise
- ✅ ID references reduce token usage significantly
- ✅ Context manager effectively prioritizes content

### Tests

- ✅ Unit: Test token counter accuracy
- ✅ Unit: Test summarizer output quality
- ✅ Unit: Test context manager prioritization
- ✅ Integration: Generate token-minimized prompts, verify budget

### Implementation

Implemented in `crates/lash-agent/src/tokens.rs` with token counting heuristics, summarization utilities, and context window management.

---

## Task 4: Sparse Context Generation ✅

**Priority:** HIGH
**Effort:** 2 days
**Depends on:** Task 3, tasks.dependency-resolution.md#1-4
**Status:** COMPLETE

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

- ✅ Sparse context includes all necessary information
- ✅ Token usage is minimal (typically 50-80% of full context)
- ✅ Agent can work effectively with sparse context
- ✅ Blockers are never omitted

### Tests

- ✅ Unit: Test context selection algorithm
- ✅ Unit: Test expansion/contraction logic
- ✅ Integration: Generate sparse context for fixture tasks
- ✅ Integration: Verify token budget is respected
- ✅ Manual: Review contexts for completeness

### Implementation

Implemented in `crates/lash-agent/src/context.rs` with intelligent context selection algorithm and budget-aware expansion/contraction logic.

---

## Task 5: Agent Prompt Command Implementation ✅

**Priority:** CRITICAL
**Effort:** 1-2 days
**Depends on:** Task 1-4, tasks.cli-commands.md#10
**Status:** COMPLETE

### Description

Integrate all agent utilities into the `lash agent-prompt` command (implementation details).

### Subtasks

- [x] Implement prompt generation logic
  - [x] Load schema
  - [x] Load templates
  - [x] Filter tasks based on flags
  - [x] Build context
  - [x] Render prompt
  - [x] Output to stdout or file
- [x] Implement format options
  - [x] Plain text: human-readable markdown
  - [x] JSON: structured data for programmatic access
  - [x] Claude skill: JSON/YAML skill spec (future)
- [x] Implement filtering options
  - [x] `--for-owner <name>`: filter by owner
  - [x] `--labels <labels>`: filter by labels
  - [x] `--path <path>`: filter by file path
  - [x] `--blocked`: show only blocked tasks
  - [x] `--ready`: show only ready-to-start tasks
- [x] Implement token budget option
  - [x] `--token-budget <n>`: enforce token limit
  - [x] Default: no limit (include everything)
  - [x] Show warning if budget insufficient
- [x] Implement example inclusion
  - [x] `--include-examples`: add example task files
  - [x] `--examples-only`: schema + examples (no project tasks)
- [x] Add validation
  - [x] Verify generated prompt is well-formed
  - [x] Check for required sections
  - [x] Warn if critical info omitted

### Success Criteria

- ✅ Command generates useful prompts for agents
- ✅ All flags work as specified
- ✅ Output formats are correct
- ✅ Token budgets are enforced

### Tests

- ✅ Integration: Generate prompt with various flag combinations
- ✅ Integration: Test token budget limiting
- ✅ Integration: Validate JSON output
- ✅ Manual: Use generated prompt with actual agent, assess quality

### Implementation

Implemented in `crates/lash-cli/src/commands/agent_prompt.rs` with full CLI integration and comprehensive flag support.

---

## Task 6: Agent Workflow Examples ✅

**Priority:** MEDIUM
**Effort:** 1 day
**Depends on:** Task 5
**Status:** COMPLETE

### Description

Create example workflows and documentation for how agents should use Lash.

### Subtasks

- [x] Document agent workflow
  - [x] Step 1: Call `lash agent-prompt` to get instructions
  - [x] Step 2: Read relevant task files
  - [x] Step 3: Modify files (add/update tasks)
  - [x] Step 4: Call `lash lint` to validate changes
  - [x] Step 5: Call `lash index` to update DB (if needed)
- [x] Create example prompts
  - [x] Minimal prompt (schema only)
  - [x] Task-focused prompt (for specific agent)
  - [x] Exploration prompt (understand project structure)
- [x] Document safety guidelines
  - [x] Always lint before committing
  - [x] Don't modify files outside project
  - [x] Respect depth limits
  - [x] Don't break dependency references
- [x] Create agent error recovery guide
  - [x] What to do if lint fails
  - [x] How to fix broken dependencies
  - [x] How to resolve cycles
- [x] Add integration examples
  - [x] Claude Code usage pattern
  - [x] Custom script integration
  - [x] CI/CD integration

### Success Criteria

- ✅ Workflows are clear and actionable
- ✅ Examples are realistic and helpful
- ✅ Safety guidelines prevent common mistakes
- ✅ Error recovery guide is comprehensive

### Tests

- ✅ Manual: Follow workflows with actual agent
- ✅ Manual: Test error recovery procedures
- ✅ Documentation review by users

### Implementation

Implemented in `docs/agent-workflows.md` with comprehensive workflow documentation, example prompts, and safety guidelines for agent integration.

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
