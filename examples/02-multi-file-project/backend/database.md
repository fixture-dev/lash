# Database Schema

@id: backend.database
@status: done
@labels: backend, database, p0
@created: 2025-12-01
@owner: backend-team

## Description

Database schema design for the blog platform. Uses PostgreSQL with proper indexing and constraints.

## Tasks

- [x] Design core tables
  - Primary keys: UUID v4
  - Timestamps: created_at, updated_at on all tables
  - Soft deletes with deleted_at column
  - [x] Users table
  - [x] Posts table
  - [x] Comments table
- [x] Add indexes
  - Index strategy: B-tree for lookups, GIN for full-text search
  - [x] User email index (unique)
  - [x] Post author index
  - [x] Comment post_id index
- [x] Create migrations
  - Using Alembic for migration management
  - [x] Initial schema migration
  - [x] Add constraints migration
