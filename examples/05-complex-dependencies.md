# Complex Dependencies Example

@id: complex-dependencies-demo
@labels: example, documentation, dependencies
@created: 2025-12-14

## Description

This file demonstrates advanced dependency patterns in Lash, including:
- Deeply nested task hierarchies (3+ levels)
- Blocked tasks using `[!]` status
- Waived tasks using `[-]` status
- Cross-file dependencies with `@depends-on`
- Circular dependency prevention
- Complex completion semantics

This is a realistic example showing how dependencies evolve in a real project.

## Tasks

### Database Migration System

Deep nesting showing parent-child dependencies with mixed statuses.

- [ ] Implement database migration system #backend #p0
  - Database migrations required before application deployment
  - Must be reversible for rollback safety
  - Track applied migrations in schema_version table
  - [x] Design migration framework
    - Framework choice: Custom solution based on Alembic patterns
    - Migration files stored in db/migrations/
    - Naming: YYYYMMDD_HHMM_description.sql
    - [x] Define migration file format
      - Each file contains: up() and down() functions
      - Checksums to detect manual edits
      - Dependency metadata for ordering
    - [x] Design migration tracking table
      - Columns: version, description, applied_at, checksum
      - Index on version for fast lookups
    - [x] Plan rollback strategy
      - down() functions must be tested
      - Rollback window: 7 days max
      - Alert on rollback attempts older than 7 days
  - [ ] Build migration runner
    - Runner validates checksums before applying
    - Runs in transaction for safety
    - Automatic retry on deadlock (max 3 attempts)
    - [x] Create migration parser
      - Parse SQL with comments
      - Extract metadata section
      - Validate syntax before execution
    - [ ] Implement apply logic
      - Check current schema version
      - Calculate migration path
      - Apply pending migrations in order
      - Update schema_version table
    - [ ] Add rollback capability
      - Verify down() function exists
      - Execute in reverse order
      - Handle partial rollback failures
  - [ ] Add migration testing
    - Test both up and down migrations
    - Verify idempotency (apply twice should succeed)
    - Test on copy of production data
    - [-] Manual testing on dev database (using automated tests instead)
    - [ ] Automated migration tests #testing
      - Fresh database: apply all migrations
      - Partial state: apply from arbitrary version
      - Rollback: apply then rollback each migration
    - [!] Load testing with production volume #blocked
      - Blocked: waiting for production data snapshot
      - Need 100GB+ test dataset
      - Must test migration time on large tables

### Distributed Tracing Implementation

Cross-file dependencies and blocked tasks.

- [ ] Add distributed tracing #backend #observability #p1
  - Using OpenTelemetry for vendor-neutral instrumentation
  - Send traces to Jaeger for storage and visualization
  - Sample 1% of requests (100% for errors)
  - @depends-on: infrastructure/monitoring.md#task:jaeger-deployment
  - [x] Instrument HTTP endpoints
    - Automatic instrumentation via middleware
    - Capture request/response headers
    - Record HTTP status codes
    - [x] Add tracing middleware
    - [x] Configure trace context propagation
    - [x] Test trace generation
  - [!] Instrument database queries #blocked
    - Blocked: waiting for ORM upgrade to v3.0
    - v3.0 adds built-in OpenTelemetry support
    - Workaround: manual span creation (decided against)
    - Current ORM version: 2.8, v3.0 ETA: Q1 2026
  - [ ] Instrument external API calls
    - Trace calls to payment gateway
    - Trace calls to shipping providers
    - Include retry attempts in traces
    - [x] Add HTTP client instrumentation
    - [ ] Configure service name mapping
      - Map external endpoints to service names
      - Example: api.stripe.com → "stripe-api"
    - [ ] Add error tracking
  - [-] Instrument message queue (waived for now)
    - Original plan: instrument Kafka consumers
    - Decision: focus on HTTP APIs first
    - May revisit in Q2 2026 if needed

### Feature Flag System

Showing waived tasks and alternative approaches.

- [ ] Implement feature flag system #backend #p0
  - Enable gradual rollout of new features
  - Support user-based, percentage-based, and rule-based flags
  - Must support <10ms flag evaluation latency
  - [-] Build custom feature flag service (using LaunchDarkly instead)
    - Original plan: custom Redis-based solution
    - Decision: vendor solution more cost-effective
    - LaunchDarkly provides better admin UI
  - [x] Integrate LaunchDarkly SDK
    - SDK caches flags locally
    - Fallback to defaults if service unavailable
    - Automatic reconnection on network failure
    - [x] Add LaunchDarkly client library
    - [x] Configure SDK initialization
    - [x] Set up environment mapping (dev/staging/prod)
  - [ ] Create flag management interface
    - Admin UI provided by LaunchDarkly
    - Just need internal docs and processes
    - [x] Document flag creation process
    - [ ] Create flag naming convention guide
      - Format: feature-name-variant-description
      - Example: checkout-flow-express-v2
    - [ ] Set up flag approval workflow
  - [ ] Add flag usage monitoring
    - Track which flags are actively used
    - Alert on stale flags (unused for 90+ days)
    - [x] Set up flag telemetry
    - [ ] Create flag usage dashboard
    - [ ] Configure stale flag alerts

### Service Authentication

Complex dependency chain with external blockers.

- [ ] Implement service-to-service authentication #backend #security #p0
  - Services use mTLS for mutual authentication
  - Certificate rotation every 90 days
  - @depends-on: infrastructure/k8s-setup.md#task:cert-manager
  - [x] Set up certificate authority
    - Using cert-manager for automatic cert issuance
    - Root CA stored in AWS Secrets Manager
    - Intermediate CAs per environment
    - [x] Initialize root CA
    - [x] Create intermediate CA for staging
    - [x] Create intermediate CA for production
  - [!] Generate service certificates #blocked
    - Blocked: cert-manager not deployed to production yet
    - Staging environment ready for testing
    - Production deployment scheduled for next week
    - Workaround: API keys in production (temporary)
  - [ ] Implement mTLS in services
    - All gRPC services must use mTLS
    - HTTP services may use mTLS or JWT
    - @depends-on: backend/grpc-framework.md#task:tls-support
    - [x] Configure gRPC server for mTLS
    - [x] Configure gRPC client for mTLS
    - [!] Test certificate rotation #blocked
      - Blocked: need production cert-manager deployment
      - Can test in staging environment only
    - [ ] Add certificate validation logging
  - [ ] Monitor certificate expiration
    - Alert 30 days before expiration
    - Auto-renewal at 60 days before expiration
    - Manual renewal process if auto-renewal fails
    - [x] Set up expiration monitoring
    - [ ] Configure renewal alerts
    - [ ] Document manual renewal process

### Performance Optimization Pipeline

Showing progressive refinement with mixed statuses.

- [ ] Optimize API performance #backend #performance #p1
  - Current p95 latency: 800ms, target: <200ms
  - Must maintain current functionality
  - Database queries are primary bottleneck
  - [x] Profile current performance
    - Used Datadog APM for profiling
    - Identified top 10 slow endpoints
    - Most time spent in database queries (70%)
    - [x] Set up APM instrumentation
    - [x] Collect baseline metrics
    - [x] Identify bottlenecks
      - Bottleneck 1: N+1 queries in user profile endpoint
      - Bottleneck 2: Unindexed ORDER BY in product search
      - Bottleneck 3: Large JSON serialization in reports API
  - [ ] Optimize database queries
    - Strategy: add eager loading, indexes, query optimization
    - Each optimization deployed separately with A/B testing
    - [x] Fix N+1 queries
      - Added eager loading for user relationships
      - Reduced queries from 50+ to 3 per request
      - Improvement: 600ms → 250ms
    - [ ] Add missing indexes
      - Analyzed slow query log
      - Identified 8 missing indexes
      - [x] Add index on products.category_id
      - [x] Add index on orders.created_at
      - [ ] Add composite index on (user_id, status)
      - [ ] Add index on products.name (for search)
    - [ ] Optimize complex queries
      - Rewrite subqueries as JOINs
      - Use CTEs for clarity
      - Consider denormalization for hot paths
  - [ ] Implement caching layer
    - Redis cache with 5-minute TTL
    - Cache-aside pattern for simplicity
    - [x] Set up Redis cluster
    - [ ] Add caching middleware
    - [ ] Implement cache invalidation
      - Invalidate on writes
      - Broadcast invalidation in multi-instance setup
    - [!] Test cache under load #blocked
      - Blocked: need production-scale load testing environment
      - Staging environment too small (1/10th prod traffic)
      - Scheduled load test: next Friday
  - [ ] Measure improvements
    - Re-run performance tests
    - Validate <200ms p95 target met
    - [x] Collect new metrics
      - Current p95: 280ms (improvement but not goal yet)
      - Still need caching layer
    - [ ] Create before/after comparison report
    - [ ] Document optimizations for team

### Cache Warming Strategy

Circular dependency prevention example.

- [ ] Implement cache warming #backend #performance #p1
  - Pre-populate cache with hot data on deployment
  - Prevents cold start performance issues
  - @depends-on: tasks above (caching layer must exist)
  - NOTE: This task depends on caching implementation above
  - This is OK because caching doesn't depend on warming (one-way dependency)
  - If warming also depended on cache metrics, we'd have a cycle (not allowed)
  - [ ] Identify hot data for warming
    - Top 100 products by views
    - User session data for logged-in users
    - Global config data
  - [ ] Build warming script
    - Run on application startup
    - Run after cache clear
    - Graceful degradation if warming fails
  - [ ] Automate warming on deploy
    - Trigger warming before routing traffic
    - Wait for warming completion (max 60 seconds)
    - Fall back to cold cache if timeout

## Cross-File Dependency Examples

This file references tasks in other example files to demonstrate cross-file dependencies:

```markdown
@depends-on: infrastructure/monitoring.md#task:jaeger-deployment
@depends-on: infrastructure/k8s-setup.md#task:cert-manager
@depends-on: backend/grpc-framework.md#task:tls-support
```

In a real project, these would point to actual files. The linter validates that:
1. Referenced files exist
2. Referenced task IDs exist within those files
3. No circular dependencies are created

## Dependency Completion Rules

Understanding how task completion works with complex dependencies:

### Rule 1: Leaf Tasks Complete Independently
```markdown
- [x] Task with no children
```
This can be marked `[x]` immediately when work is done.

### Rule 2: Parent Tasks Require All Children Complete
```markdown
- [ ] Parent task
  - [x] Child 1
  - [x] Child 2
  - [ ] Child 3 (still open)
```
Parent cannot be `[x]` until Child 3 is done or waived.

### Rule 3: Waived Tasks Don't Block Completion
```markdown
- [x] Parent task (can be marked done)
  - [x] Child 1
  - [-] Child 2 (waived)
  - [x] Child 3
```
Parent is complete because Child 2 is waived.

### Rule 4: Blocked Tasks Indicate External Dependencies
```markdown
- [ ] Parent task
  - [x] Child 1
  - [!] Child 2 (blocked by external factor)
```
Parent is blocked until Child 2 is unblocked or waived.

### Rule 5: Cross-File Dependencies Must Complete
```markdown
@depends-on: other-file.md#task:some-task
```
This task cannot complete until `some-task` in `other-file.md` is done or waived.

## Detecting Circular Dependencies

Lash prevents circular dependencies. These would be rejected:

### Example: File A depends on File B, File B depends on File A
```markdown
# file-a.md
@depends-on: file-b.md
```

```markdown
# file-b.md
@depends-on: file-a.md
```
**Error**: Circular dependency detected.

### Example: Task depends on its parent
```markdown
- [ ] Parent task @id:parent
  - [ ] Child task @depends-on:#task:parent
```
**Error**: Task cannot depend on ancestor.

### Example: Indirect circular dependency
```markdown
# file-a.md
@depends-on: file-b.md

# file-b.md
@depends-on: file-c.md

# file-c.md
@depends-on: file-a.md
```
**Error**: Circular dependency chain detected: A → B → C → A.

## Summary

This example demonstrates:
- **Nested tasks** up to 4 levels deep
- **Mixed statuses**: `[ ]` open, `[x]` done, `[-]` waived, `[!]` blocked
- **Cross-file dependencies** using `@depends-on`
- **Contextual notes** providing requirements and rationale
- **Real-world complexity** showing how dependencies evolve
- **Circular dependency prevention** (enforced by linter)

Use `lash graph` to visualize these dependencies and `lash check-links` to validate all references are correct.
