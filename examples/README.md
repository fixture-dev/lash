# Lash Examples & Tutorials

This directory contains practical examples demonstrating Lash's task tracking features, from simple TODO lists to complex multi-file projects with dependencies.

## Examples Overview

### 1. Simple TODO List
**File**: `01-simple-todo.md`
**Complexity**: Beginner
**Concepts**: Basic task structure, checkboxes, nested tasks, labels

A weekend TODO list showing Lash's minimal viable structure. Perfect for getting started.

```bash
lash show examples/01-simple-todo.md
```

**What you'll learn**:
- Basic task file structure (metadata, description, tasks)
- Task statuses: `[ ]` open, `[x]` done, `[-]` waived
- Simple nesting (parent-child tasks)
- Using labels for categorization

---

### 2. Multi-File Project
**Directory**: `02-multi-file-project/`
**Complexity**: Intermediate
**Concepts**: Root index, multiple task files, cross-file dependencies, directory organization

A blog platform project split across backend and frontend task files.

```bash
cd examples/02-multi-file-project
lash list
lash graph
```

**What you'll learn**:
- Organizing tasks across multiple files
- Using `index.lash.md` as project entry point
- Cross-file dependencies with `@depends-on`
- Directory-based project structure
- Task completion logic across files

**Structure**:
```
02-multi-file-project/
├── index.lash.md          # Project overview
├── backend/
│   ├── database.md        # Database tasks (complete)
│   └── api.md             # API tasks (in progress)
└── frontend/
    └── components.md      # UI tasks (in progress)
```

---

### 3. Software Project
**Directory**: `03-software-project/`
**Complexity**: Advanced
**Concepts**: Microservices, cross-cutting concerns, team ownership, realistic complexity

A realistic e-commerce platform rewrite with 10 task files, 98 tasks, and complex dependencies.

```bash
cd examples/03-software-project
lash list --label backend
lash list --label security
lash graph
```

**What you'll learn**:
- Managing large projects (10+ files, 100+ tasks)
- Cross-cutting concerns with labels (`#security`, `#performance`, `#testing`)
- Module dependencies (auth → catalog → orders → payment)
- Priority levels (`#p0`, `#p1`, `#p2`)
- Team ownership (`@owner`)
- Realistic estimates and timelines
- Compliance requirements (PCI-DSS, WCAG)

**Structure**:
```
03-software-project/
├── index.lash.md
├── backend/               # 4 microservices
│   ├── auth-service.md
│   ├── catalog-service.md
│   ├── orders-service.md
│   └── payment-service.md
├── frontend/              # 3 applications
│   ├── component-library.md
│   ├── customer-portal.md
│   └── admin-dashboard.md
└── infrastructure/        # DevOps & observability
    ├── k8s-setup.md
    ├── cicd.md
    └── monitoring.md
```

---

### 4. Agent Workflow
**File**: `04-agent-workflow.md`
**Complexity**: Intermediate
**Concepts**: AI agent integration, `lash agent-prompt`, agent best practices, token minimization

Demonstrates how AI agents (like Claude Code) should use Lash for task tracking.

```bash
lash show examples/04-agent-workflow.md
lash agent-prompt --label agent --label p0
```

**What you'll learn**:
- Using `lash agent-prompt` for focused context
- Agent workflow: query → implement → validate → update
- Token minimization strategies
- Using `#agent` labels for agent-suitable tasks
- Common agent mistakes and how to avoid them
- Error handling and recovery
- Adding discovered work during implementation

**Key workflows**:
1. Agent gets context: `lash agent-prompt --label agent`
2. Agent reads task: `lash show features/authentication.md`
3. Agent implements feature
4. Agent validates: `lash lint features/authentication.md`
5. Agent marks complete and adds context notes

---

### 5. Complex Dependencies
**File**: `05-complex-dependencies.md`
**Complexity**: Advanced
**Concepts**: Deep nesting, blocked tasks, waived tasks, cross-file dependencies, circular dependency prevention

Shows advanced dependency patterns including 4-level nesting, blocked tasks `[!]`, and cross-file dependencies.

```bash
lash show examples/05-complex-dependencies.md
lash graph --scope examples/05-complex-dependencies.md
lash check-links
```

**What you'll learn**:
- Deep task hierarchies (3-4 levels)
- Blocked tasks `[!]` with external dependencies
- Waived tasks `[-]` for skipped work
- Cross-file dependencies with `@depends-on`
- Task completion rules with complex dependencies
- Circular dependency detection and prevention
- Real-world dependency evolution

**Dependency patterns demonstrated**:
- Database migration system (4-level nesting)
- Distributed tracing (blocked tasks waiting on infrastructure)
- Feature flags (waived tasks, alternative approaches)
- Service authentication (cross-file dependencies)
- Performance optimization (progressive refinement)

---

### 6. Contextual Notes
**File**: `contextual-notes.md`
**Complexity**: Intermediate
**Concepts**: Contextual notes, requirements, acceptance criteria, implementation hints

Comprehensive guide to using plain bullet points (without checkboxes) for inline context.

```bash
lash show examples/contextual-notes.md
```

**What you'll learn**:
- Difference between tasks (`- [ ]`) and notes (`-`)
- When to use notes vs. child tasks
- Using notes for requirements and constraints
- Using notes for acceptance criteria
- Using notes for implementation hints
- Best practices for concise, useful notes
- How notes integrate with search and TUI

---

## Quick Start Guide

### Installing Lash
```bash
# Installation instructions will be here once published
cargo install lash
```

### Running Examples

1. **View an example**:
```bash
lash show examples/01-simple-todo.md
```

2. **List tasks in an example**:
```bash
lash list --path examples/02-multi-file-project
```

3. **See dependencies**:
```bash
cd examples/03-software-project
lash graph
```

4. **Generate agent prompt**:
```bash
lash agent-prompt --label agent --label p0
```

5. **Validate an example**:
```bash
lash lint examples/01-simple-todo.md
```

---

## Learning Path

### New to Lash?
1. Start with `01-simple-todo.md` - Learn basic structure
2. Read `contextual-notes.md` - Understand notes vs tasks
3. Explore `02-multi-file-project/` - See multi-file organization

### Building a project?
1. Study `03-software-project/` - See realistic complexity
2. Adapt structure to your needs
3. Use labels for cross-cutting concerns

### Integrating with AI agents?
1. Read `04-agent-workflow.md` - Learn agent patterns
2. Practice with `lash agent-prompt` command
3. Implement agent-friendly task structure

### Advanced usage?
1. Review `05-complex-dependencies.md` - Master dependencies
2. Learn `lash graph` for visualization
3. Use `lash check-links` for validation

---

## Common Patterns

### Organizing by Feature
```
project/
├── index.lash.md
└── features/
    ├── authentication.md
    ├── user-profiles.md
    └── notifications.md
```

### Organizing by Team
```
project/
├── index.lash.md
├── backend/
├── frontend/
├── infrastructure/
└── design/
```

### Organizing by Milestone
```
project/
├── index.lash.md
└── milestones/
    ├── alpha.md
    ├── beta.md
    └── release.md
```

---

## Additional Resources

- **Design Document**: `../docs/design-doc.md` - Complete Lash specification
- **CLAUDE.md**: `../CLAUDE.md` - Project guidelines and conventions
- **Playground**: `../playground/` - PixelQuest game project demo

---

## Contributing Examples

Have a great Lash example to share? Contributions welcome!

Guidelines for new examples:
- Include a clear README explaining what the example demonstrates
- Use realistic, practical scenarios
- Add contextual notes explaining key concepts
- Ensure all examples pass `lash lint`
- Progress from simple to complex concepts
- Include inline comments for educational value

---

## Questions?

- Check the design doc for detailed specifications
- Look at the playground for a full-scale example project
- Examine existing examples for patterns

Happy task tracking!
