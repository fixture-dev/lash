# Blog Platform Project

@id: blog-platform
@status: in-progress
@labels: web, fullstack, example
@created: 2025-12-14

## Description

A simple blog platform demonstrating multi-file task organization. This project shows how Lash handles dependencies between different task files and directories.

This example demonstrates:
- Root index file pointing to multiple task files
- Task organization by functional area (backend, frontend)
- Cross-file dependencies using @depends-on
- Directory-based project structure

## Tasks

### Backend Development
Server-side API and database work.

- [ ] [API Development](backend/api.md) @id:`backend.api` @labels:`backend, api, p0`
- [ ] [Database Schema](backend/database.md) @id:`backend.database` @labels:`backend, database, p0`

### Frontend Development
Client-side UI and user experience.

- [ ] [UI Components](frontend/components.md) @id:`frontend.components` @labels:`frontend, ui, p0`

## Notes

The root index provides a high-level map of the entire project. Use `lash list` to see all tasks across files, or `lash show backend/api.md` to drill into specific files.
