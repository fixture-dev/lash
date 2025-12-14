# Software Project Example

This example demonstrates a realistic e-commerce platform rebuild with microservices architecture, showing how Lash handles complex software projects with multiple teams and cross-cutting concerns.

## Project Overview

**E-Commerce Platform v2.0** - A complete rewrite with:
- 4 backend microservices (auth, catalog, orders, payment)
- 3 frontend applications (component library, customer portal, admin dashboard)
- 3 infrastructure components (Kubernetes, CI/CD, monitoring)

This represents ~5-6 months of work for a team of 10-12 engineers.

## Structure

```
03-software-project/
├── index.lash.md                    # Root index
├── backend/
│   ├── auth-service.md              # Authentication & OAuth
│   ├── catalog-service.md           # Product catalog & search
│   ├── orders-service.md            # Order processing
│   └── payment-service.md           # Payment processing (PCI compliant)
├── frontend/
│   ├── component-library.md         # Shared React components
│   ├── customer-portal.md           # Customer-facing app
│   └── admin-dashboard.md           # Internal admin tools
└── infrastructure/
    ├── k8s-setup.md                 # Kubernetes cluster
    ├── cicd.md                      # CI/CD pipeline
    └── monitoring.md                # Observability stack
```

## Key Features Demonstrated

### 1. Module Dependencies

Notice the realistic dependency chains:
- Frontend apps depend on component library
- Customer portal depends on catalog service
- Orders service depends on both catalog and payment services

```markdown
@depends-on: backend/catalog-service.md#task:inventory-management
@depends-on: backend/payment-service.md#task:payment-processing
```

### 2. Cross-Cutting Concerns

Labels track work that spans multiple services:

- `#security` - 28 tasks across auth, payment, infrastructure
- `#performance` - 15 tasks in catalog, frontend, monitoring
- `#testing` - 35 tasks across all services
- `#documentation` - 12 tasks for API docs and guides

Query examples:
```bash
# Find all security work
lash list --label security

# Find high-priority performance tasks
lash list --label performance --label p0

# All testing tasks
lash list --label testing
```

### 3. Priority Levels

Tasks are tagged with priority:
- `#p0` - Must have for launch (59 tasks)
- `#p1` - Should have for launch (31 tasks)
- `#p2` - Nice to have (8 tasks)

### 4. Team Ownership

Services are assigned to teams:
- `@owner: backend-team-alpha` - Auth and Orders
- `@owner: backend-team-beta` - Catalog and Payment
- `@owner: frontend-team` - All frontend work
- `@owner: devops-team` - Infrastructure

### 5. Contextual Notes

Extensive use of contextual notes for requirements:

```markdown
- [ ] Implement rate limiting
  - Rate limiting: 5 failed attempts per 15 minutes per IP
  - Session management with Redis
  - CSRF protection on state-changing endpoints
  - [x] Add rate limiting middleware
  - [ ] Configure Redis sessions
```

### 6. Compliance Requirements

Special handling for regulated work:
- `#pci-compliance` for payment service
- `#security` for sensitive operations
- Agent notes warn about compliance requirements

## Using This Example

### Basic Queries

```bash
# List all tasks
lash list

# Show a specific service
lash show backend/auth-service.md

# Search for specific functionality
lash search "OAuth"
lash search "Redis"
```

### Filter by Team/Priority

```bash
# Backend team work
lash list --owner backend-team-alpha

# High priority items
lash list --label p0

# Frontend tasks that are still open
lash list --label frontend --status open
```

### Dependency Analysis

```bash
# View full dependency graph
lash graph

# Check for broken dependencies
lash check-links

# See what's blocking a specific task
lash show backend/orders-service.md
```

### Agent Integration

```bash
# Generate context for backend work
lash agent-prompt --label backend --label p0

# Get all security tasks for review
lash agent-prompt --label security --format json
```

## Realistic Complexity

This example shows:
- **98 total tasks** across 10 files
- **Mixed completion states** showing real progress
- **Realistic estimates** (2-6 weeks per service)
- **Technical depth** with specific technologies and constraints
- **Compliance considerations** (PCI-DSS, WCAG, OWASP)

## Project Phases

The project demonstrates natural progression:

1. **Foundation** (mostly complete)
   - Component library core
   - Auth service core
   - K8s cluster setup

2. **Active Development** (in progress)
   - Catalog service
   - Customer portal
   - CI/CD pipeline

3. **Upcoming** (not started)
   - Orders service
   - Payment service
   - Admin dashboard
   - Monitoring

This mirrors real software development where foundational work is done first, followed by feature development.
