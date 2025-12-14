# Multi-File Project Example

This example demonstrates how to organize a larger project with multiple task files and cross-file dependencies.

## Structure

```
02-multi-file-project/
├── index.lash.md          # Root index file
├── backend/
│   ├── database.md        # Database schema tasks
│   └── api.md             # API endpoint tasks
└── frontend/
    └── components.md      # UI component tasks
```

## Key Concepts

### 1. Root Index File
The `index.lash.md` file serves as the entry point, providing a high-level overview and links to all task files.

### 2. Directory Organization
Tasks are organized by functional area (backend, frontend) in separate directories.

### 3. Cross-File Dependencies
Notice how `api.md` depends on `database.md`:
```markdown
@depends-on: backend/database.md
```

And how `components.md` depends on specific tasks in `api.md`:
```markdown
@depends-on: backend/api.md#task:blog-endpoints
```

### 4. Task Completion Logic
- The database schema must be complete before API work can finish
- UI components depend on API endpoints being available
- Parent tasks complete only when all dependencies are done

## Using This Example

```bash
# Navigate to this directory
cd examples/02-multi-file-project

# List all tasks
lash list

# Show dependency graph
lash graph

# Check for broken links
lash check-links

# View specific file
lash show backend/api.md

# Search across all files
lash search "authentication"
```

## Progressive Development

This example shows a realistic workflow:
1. Database schema completed first (all tasks done)
2. API partially complete (authentication in progress)
3. Frontend components started (building on API progress)

This mirrors real project development where backend work often precedes frontend implementation.
