---
description: Zeta TypeScript frontend layers, services, adapters, and base boundaries.
applyTo: "**/src/**/*.ts,**/src/**/*.cts,**/test/**/*.ts"
---

# TypeScript Frontend Architecture Guidelines

## Dependency direction

Preserve `base → platform → editor → workbench`. Lower layers must not import, reference, specialize for, or derive defaults from higher layers.

Keep interfaces small and complete from the caller's point of view. Every method, option, and overload must add distinct semantics.

Prefer an existing canonical context or service over options and callbacks that merely expose the same state again.

## Frontend services

- Put a frontend domain service contract in a `common/*Service.ts` file and name its public interface and service identifier `I<Capability>Service`.
- Name each runtime implementation file after its exported class, including a meaningful runtime qualifier: `appServerSyntaxAnalysisService.ts` exports `AppServerSyntaxAnalysisService`.
- Align capability names, operation semantics, lifecycle, and error categories across the frontend service, transport protocol, and backend service so adapters remain thin and mechanical.
- Name adapters and tests after the contract or implementation they exercise.

Transport APIs, generated DTOs, and wire validation stay inside the runtime adapter. Product consumers depend on the frontend service contract and frontend-owned domain types, not transport representations.

## Base layer

Modules under `src/zeta/base` are domain-neutral. Higher-level features may depend on base; base must not import or mention them.

Define URI parsing, URI identity, resource collections, UUID validation, and lifecycle primitives in terms of their general contracts. Preserve exact URI identity by default; a domain that needs alternate semantics, such as ignoring fragments, selects that policy explicitly.

Domain identities and lifecycle rules remain in their owning domain.

Add general structures only for concrete domain-neutral consumers.
