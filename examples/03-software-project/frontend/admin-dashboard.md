# Admin Dashboard

@id: frontend.admin
@labels: frontend, p1
@created: 2025-10-15
@owner: frontend-team
@estimate: 4 weeks
@depends-on: frontend/component-library.md, backend/orders-service.md

## Description

Internal admin dashboard for managing products, orders, and customers. Built with React and React Query for data fetching.

Target users: Internal staff and customer support team.

## Tasks

- [ ] Dashboard setup
  - React 18 with TypeScript
  - React Query for server state
  - React Router for navigation
  - [ ] Initialize React project
  - [ ] Set up routing
  - [ ] Configure React Query
  - [ ] Add authentication flow
- [ ] Product management
  - Bulk operations for efficiency
  - Rich text editor for descriptions
  - Image upload with drag-and-drop
  - [ ] Product list with filters
  - [ ] Product creation form
  - [ ] Product editing interface
  - [ ] Bulk product operations
  - [ ] Image upload and management
- [ ] Order management
  - Real-time order updates via WebSocket
  - Export orders to CSV
  - [ ] Order list with advanced filtering
  - [ ] Order detail view
  - [ ] Update order status
  - [ ] Process refunds
  - [ ] Export functionality
- [ ] Customer management
  - Search by email, name, or order ID
  - Customer lifetime value calculation
  - [ ] Customer list
  - [ ] Customer detail page
  - [ ] Order history per customer
  - [ ] Support note system
- [ ] Analytics & reporting
  - Charts using recharts library
  - Date range selectors
  - [ ] Sales dashboard
  - [ ] Product performance reports
  - [ ] Customer analytics
  - [ ] Export reports to PDF
