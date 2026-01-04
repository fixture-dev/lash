# Order Management Service

@id: backend.orders
@labels: backend, p0
@created: 2025-09-25
@owner: backend-team-alpha
@estimate: 5 weeks
@depends-on: backend/catalog-service.md#task:inventory-management, backend/payment-service.md#task:payment-processing

## Description

Order processing service handling cart management, checkout flow, order fulfillment, and returns. Integrates with payment and shipping providers.

Must handle:
- High-volume flash sales (10,000+ orders/minute)
- Distributed transactions across services
- Eventual consistency for inventory

## Tasks

- [ ] Shopping cart
  - Persistent carts stored in PostgreSQL
  - Anonymous carts expire after 30 days
  - Cart merge on user login
  - [ ] Create cart endpoints
  - [ ] Add/remove items
  - [ ] Apply discount codes
  - [ ] Cart expiration cleanup job
- [ ] Checkout flow
  - Saga pattern for distributed transactions
  - Rollback on payment or inventory failure
  - Idempotent checkout (prevent duplicate orders)
  - [ ] Design checkout saga
  - [ ] Implement inventory reservation
  - [ ] Process payment
  - [ ] Create order record
  - [ ] Send confirmation email
- [ ] Order fulfillment
  - Integration with shipping providers (FedEx, UPS, USPS)
  - Webhook endpoints for shipment tracking
  - Automatic fulfillment for digital goods
  - [ ] Shipping provider integration
  - [ ] Generate shipping labels
  - [ ] Track shipment status
  - [ ] Handle delivery confirmation
- [ ] Returns & refunds
  - 30-day return window
  - Automatic restocking on returns
  - Partial refunds for damaged items
  - [ ] Initiate return request
  - [ ] Process refund
  - [ ] Update inventory
  - [ ] Track return shipping
- [ ] Order history & tracking
  - Real-time order status updates
  - Email notifications for status changes
  - [ ] Order detail page API
  - [ ] Order history listing
  - [ ] Status change notifications
