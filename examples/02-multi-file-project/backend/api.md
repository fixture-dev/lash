# API Development

@id: backend.api
@status: in-progress
@labels: backend, api, p0
@created: 2025-12-05
@owner: backend-team
@depends-on: backend/database.md

## Description

REST API endpoints for the blog platform. Built with FastAPI, includes authentication, CRUD operations, and pagination.

API follows RESTful conventions with proper HTTP status codes and JSON responses.

## Tasks

- [x] Set up API framework
  - FastAPI with Pydantic models for validation
  - CORS enabled for frontend development
  - [x] Install FastAPI
  - [x] Configure CORS
  - [x] Add request logging
- [ ] Implement authentication
  - JWT tokens with 24-hour expiry
  - Refresh tokens stored in Redis
  - bcrypt for password hashing (cost factor 12)
  - [x] User registration endpoint
  - [x] Login endpoint
  - [ ] Token refresh endpoint
  - [ ] Password reset flow
- [ ] Create blog endpoints
  - Pagination: 20 items per page default
  - Sort by created_at DESC by default
  - [x] GET /posts (list posts)
  - [x] POST /posts (create post)
  - [ ] GET /posts/:id (get single post)
  - [ ] PUT /posts/:id (update post)
  - [ ] DELETE /posts/:id (delete post)
- [ ] Add comment endpoints
  - Nested comments up to 3 levels deep
  - [x] POST /posts/:id/comments
  - [ ] GET /posts/:id/comments
  - [ ] DELETE /comments/:id
