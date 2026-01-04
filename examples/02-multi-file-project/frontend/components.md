# UI Components

@id: frontend.components
@labels: frontend, ui, p0
@created: 2025-12-07
@owner: frontend-team
@depends-on: backend/api.md#task:blog-endpoints

## Description

React components for the blog platform UI. Uses modern React patterns with hooks and functional components.

Component library: Custom components with Tailwind CSS for styling.

## Tasks

- [x] Create base components
  - All components use TypeScript for type safety
  - Props validation with PropTypes as fallback
  - [x] Button component
  - [x] Input component
  - [x] Card component
- [ ] Build post components
  - Markdown rendering with react-markdown
  - Syntax highlighting for code blocks
  - [x] PostList component
  - [x] PostCard component
  - [ ] PostDetail component
  - [ ] PostEditor component
- [ ] Implement comment components
  - Thread depth indicator via left border color
  - Collapse/expand for nested threads
  - [ ] CommentList component
  - [ ] CommentForm component
  - [ ] CommentThread component
- [ ] Add authentication UI
  - Form validation with react-hook-form
  - Error messages displayed inline
  - [ ] LoginForm component
  - [ ] RegisterForm component
  - [ ] PasswordReset component
