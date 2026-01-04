# Authentication Service

@id: backend.auth
@labels: backend, auth, p0, security
@created: 2025-09-15
@owner: backend-team-alpha
@estimate: 4 weeks

## Description

OAuth 2.0 compliant authentication service supporting multiple identity providers (Google, GitHub, email/password). Implements JWT tokens with refresh token rotation for security.

Security requirements:
- OWASP Top 10 compliance
- Rate limiting on all endpoints
- Audit logging for all auth events
- Support for MFA (TOTP)

@agent-note: All auth endpoints require security review before deployment. Password hashing uses Argon2id, not bcrypt (per ADR-023).

## Tasks

- [x] Core authentication logic
  - Argon2id for password hashing (time cost: 2, memory: 19MB, parallelism: 1)
  - JWT signing with RS256 (2048-bit keys)
  - Access token TTL: 15 minutes, refresh token: 30 days
  - [x] User registration with email verification
  - [x] Login with email/password
  - [x] JWT token generation
  - [x] Refresh token rotation
- [ ] OAuth provider integration
  - Supported providers: Google, GitHub, Microsoft
  - Callback URLs must be whitelisted in config
  - Store provider tokens encrypted in database
  - [x] Google OAuth integration #p0
  - [ ] GitHub OAuth integration #p0
  - [ ] Microsoft OAuth integration #p1
  - [ ] Provider token refresh logic
- [ ] Multi-factor authentication
  - TOTP implementation per RFC 6238
  - QR code generation for authenticator apps
  - Backup codes (10 codes, single-use)
  - [ ] TOTP enrollment flow
  - [ ] TOTP verification at login
  - [ ] Backup code generation
  - [ ] Recovery flow if MFA device lost
- [ ] Security hardening #security
  - Rate limiting: 5 failed attempts per 15 minutes per IP
  - Session management with Redis
  - CSRF protection on state-changing endpoints
  - [x] Implement rate limiting
  - [x] Add CSRF protection
  - [ ] Session timeout handling
  - [ ] Suspicious activity detection
- [ ] Testing & documentation #testing #documentation
  - Target: 85% code coverage minimum
  - API documentation with OpenAPI 3.0
  - [x] Unit tests for core auth logic
  - [ ] Integration tests for OAuth flows
  - [ ] Security penetration testing
  - [ ] API documentation
