# Contextual Notes: Examples and Best Practices

@id: examples.contextual-notes
@labels: documentation, examples
@status: done
@created: 2025-12-13

## Description

This document provides comprehensive examples and best practices for using **contextual notes** in Lash task files. Contextual notes are plain bullet points (without checkboxes) that provide inline context, requirements, acceptance criteria, or implementation hints.

## What Are Contextual Notes?

Contextual notes are a semantic feature that distinguishes between:

- **Actionable tasks** (`- [ ] Text`) - Items that need to be completed and tracked
- **Informational context** (`- Text`) - Requirements, constraints, hints, and acceptance criteria

This separation makes task files more readable and provides better context for both humans and AI agents working with the codebase.

## Basic Example

```markdown
- [ ] Implement user authentication
  - Use bcrypt for password hashing (min cost factor 12)
  - JWT tokens should expire after 24 hours
  - Support email and username login
  - [ ] Create User model with password field
  - [ ] Add login endpoint
  - [ ] Add registration endpoint
  - [ ] Implement JWT token generation
```

In this example:
- The three plain bullets provide requirements and constraints
- The four checkbox items are the actionable tasks to complete
- Notes appear before tasks (recommended convention)

## When to Use Notes vs. Child Tasks

### Use Contextual Notes For:

**1. Requirements and Constraints**
```markdown
- [ ] Design database schema
  - Must support multi-tenancy
  - Keep migration files under 1000 lines
  - Use UUID for primary keys
  - [ ] Create initial schema
  - [ ] Add indexes for foreign keys
```

**2. Acceptance Criteria**
```markdown
- [ ] Implement search functionality
  - Results must appear within 100ms for 95th percentile
  - Support fuzzy matching with max 2-character typos
  - Handle special characters and unicode
  - [ ] Add search endpoint
  - [ ] Implement ranking algorithm
  - [ ] Add result caching
```

**3. Implementation Hints**
```markdown
- [ ] Optimize image processing pipeline
  - Consider using SIMD instructions for batch operations
  - Profile before/after with realistic workloads
  - Existing pipeline is in src/image/process.rs
  - [ ] Benchmark current performance
  - [ ] Implement parallel processing
  - [ ] Add caching layer
```

**4. API or Library Specifics**
```markdown
- [ ] Integrate payment processing
  - Use Stripe API v3 (not v2)
  - Webhook secret stored in vault at ops/secrets/stripe
  - Test mode key starts with sk_test_
  - [ ] Set up Stripe account
  - [ ] Implement checkout flow
  - [ ] Add webhook handlers
```

**5. Context from Design Decisions**
```markdown
- [ ] Refactor configuration system
  - Moving from JSON to TOML per ADR-015
  - Must maintain backward compatibility for 2 versions
  - Configuration lives in ~/.myapp/config.toml
  - [ ] Create TOML parser
  - [ ] Migrate existing configs
  - [ ] Update documentation
```

### Use Child Tasks For:

**1. Multi-step Processes**
```markdown
- [ ] Set up CI/CD pipeline
  - [ ] Configure GitHub Actions
    - [ ] Add test workflow
    - [ ] Add build workflow
    - [ ] Add deploy workflow
  - [ ] Set up artifact storage
  - [ ] Configure deployment targets
```

**2. Independently Trackable Work**
```markdown
- [ ] Complete user profile feature
  - [ ] Backend API endpoints
  - [ ] Frontend components
  - [ ] Database migrations
  - [ ] End-to-end tests
```

**3. Work That Can Be Done in Parallel**
```markdown
- [ ] Prepare for v2.0 release
  - [ ] Update dependencies
  - [ ] Run security audit
  - [ ] Update changelog
  - [ ] Create release notes
```

## Advanced Examples

### Mixing Notes and Nested Tasks

```markdown
- [ ] Implement real-time collaboration
  - Use WebSocket for bidirectional communication
  - Must handle 100+ concurrent connections
  - Consider using operational transforms (OT) or CRDTs
  - [ ] Design collaboration protocol
    - Protocol should be stateless where possible
    - Include version negotiation for backward compatibility
    - [ ] Define message types
    - [ ] Design conflict resolution strategy
  - [ ] Implement WebSocket server
  - [ ] Add client-side sync logic
  - [ ] Test with concurrent users
```

### Using Notes for Edge Cases

```markdown
- [ ] Add file upload functionality
  - Max file size: 100MB for free users, 1GB for premium
  - Supported formats: images (jpg, png, gif), documents (pdf, docx)
  - Virus scanning required for all uploads
  - Store in S3 bucket: myapp-uploads-prod
  - [ ] Create upload endpoint
  - [ ] Add file validation
  - [ ] Integrate virus scanner
  - [ ] Implement progress tracking
```

### Technical Context for Complex Features

```markdown
- [ ] Migrate to microservices architecture
  - Current monolith is in src/legacy/
  - Target: 5-7 services (auth, users, content, billing, analytics)
  - Use gRPC for service-to-service communication
  - Maintain backward compatibility during transition
  - Migration should be complete by Q2 2025
  - [ ] Design service boundaries
    - Reference bounded contexts from DDD workshop notes
    - Keep database-per-service pattern
    - [ ] Document service interfaces
    - [ ] Create dependency graph
  - [ ] Extract authentication service
  - [ ] Extract user management service
  - [ ] Set up API gateway
  - [ ] Deploy to staging environment
```

## Best Practices

### 1. Keep Notes Concise

**Good:**
```markdown
- [ ] Implement caching layer
  - Use Redis for session storage
  - TTL: 24 hours for sessions, 1 hour for API responses
```

**Avoid:**
```markdown
- [ ] Implement caching layer
  - We need to use Redis because it's fast and supports TTL. The sessions should expire after 24 hours to balance security and user experience. API responses can expire faster at 1 hour since data changes frequently. Make sure to handle cache misses gracefully and implement proper error handling for Redis connection failures.
```

**Better approach for verbose context:** Use the `## Description` section or link to a design document:
```markdown
- [ ] Implement caching layer
  - See docs/caching-strategy.md for detailed rationale
  - Redis config in ops/redis.conf
  - [ ] Set up Redis instance
  - [ ] Implement cache wrapper
```

### 2. Place Notes Before Child Tasks

**Recommended:**
```markdown
- [ ] Add user notifications
  - Support email, SMS, and push notifications
  - Rate limit: max 10 notifications per user per hour
  - [ ] Create notification service
  - [ ] Add email templates
  - [ ] Implement delivery queue
```

**Works but less readable:**
```markdown
- [ ] Add user notifications
  - [ ] Create notification service
  - [ ] Add email templates
  - Support email, SMS, and push notifications
  - [ ] Implement delivery queue
  - Rate limit: max 10 notifications per user per hour
```

### 3. Don't Nest Notes Under Notes

**Invalid (will cause linter error):**
```markdown
- [ ] Implement feature X
  - Important constraints:
    - Must be backward compatible
    - Should handle edge cases
```

**Correct approach:**
```markdown
- [ ] Implement feature X
  - Must be backward compatible
  - Should handle edge cases
```

### 4. Use Notes for "Why" and "How", Tasks for "What"

**Good separation:**
```markdown
- [ ] Refactor authentication module
  - Current implementation has security vulnerabilities (see SEC-2024-03)
  - New approach uses industry-standard OAuth 2.0
  - Must complete before external security audit in January
  - [ ] Research OAuth 2.0 libraries
  - [ ] Design new auth flow
  - [ ] Implement OAuth integration
  - [ ] Migrate existing users
```

### 5. Notes Are Searchable

All contextual notes are indexed in the database and searchable via `lash search`:

```bash
# Find tasks with specific implementation requirements
lash search "Use Redis"

# Find tasks with performance constraints
lash search "< 100ms"

# Find API version requirements
lash search "API v3"
```

## Integration with Other Features

### Notes in TUI

In the terminal UI (`lash tui`):
- Notes appear with a `·` or `○` prefix (no checkbox)
- Notes are styled differently (dimmed/italic) for visual distinction
- Notes are not selectable/checkable in the task tree
- Notes appear in the detail pane under a "Notes:" section

### Notes in CLI Commands

```bash
# Show task details including notes
lash show features/authentication.md

# List tasks (notes hidden by default)
lash list --label backend

# List tasks with notes visible
lash list --label backend --show-notes
```

### Notes for AI Agents

Contextual notes are particularly valuable for AI agents:

```markdown
- [ ] Implement rate limiting
  - Use token bucket algorithm (not sliding window)
  - Existing implementation at src/middleware/rate_limit.rs needs replacement
  - Must preserve existing API contract for backward compatibility
  - [ ] Design new rate limiter
  - [ ] Add tests for edge cases
  - [ ] Deploy with feature flag
```

The notes provide context that helps agents:
- Understand implementation requirements
- Find relevant existing code
- Avoid making breaking changes
- Make informed technical decisions

## Validation Rules

The linter enforces these rules for contextual notes:

1. **Indentation:** Notes must use consistent 2-space indentation
2. **Depth:** Notes must be exactly 2 spaces deeper than their parent task
3. **Children:** Notes cannot have child items (neither tasks nor notes)
4. **Length:** Warning at 200 characters, error at 500 characters (keep notes concise)

Run `lash lint` to validate your task files.

## Common Patterns

### API Integration Tasks

```markdown
- [ ] Integrate third-party email service
  - Use SendGrid API (credentials in vault)
  - Template IDs: welcome=d-123, reset=d-456
  - Rate limit: 100 emails/minute on current plan
  - [ ] Add SendGrid SDK
  - [ ] Create email templates
  - [ ] Implement retry logic
  - [ ] Add monitoring
```

### Database Schema Changes

```markdown
- [ ] Add user preferences table
  - Columns: user_id (FK), preferences (JSONB), updated_at
  - Index on user_id for fast lookups
  - JSONB allows flexible schema evolution
  - [ ] Write migration script
  - [ ] Update User model
  - [ ] Add API endpoints
```

### Performance Optimization

```markdown
- [ ] Optimize dashboard query performance
  - Current P95: 2.3s, target: <500ms
  - Main bottleneck: N+1 queries in UserStats
  - Consider denormalizing user_stats table
  - [ ] Profile current queries
  - [ ] Add database indexes
  - [ ] Implement query caching
  - [ ] Re-measure performance
```

### Security Improvements

```markdown
- [ ] Implement content security policy
  - Follow OWASP CSP guidelines
  - Start with report-only mode
  - Must not break file upload functionality
  - [ ] Draft CSP header rules
  - [ ] Test in staging
  - [ ] Monitor violation reports
  - [ ] Enable enforcement mode
```

## Summary

Contextual notes are a powerful feature for:
- Documenting requirements inline with tasks
- Providing implementation hints for developers and AI agents
- Capturing acceptance criteria and constraints
- Improving task file readability
- Maintaining searchable context

Use them liberally to make your task files self-documenting and agent-friendly!
