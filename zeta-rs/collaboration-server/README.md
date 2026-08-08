# zeta-collaboration-server

`zeta-collaboration-server` is the small remote product host for durable Gama
collaboration. It owns bearer authentication, browser-origin policy, HTTP
framing, long-poll delivery, connection limits, and SQLite lifecycle. Ordered
room semantics remain in [`zeta-collaboration`](../collaboration/README.md).
It has no App Server, workspace, tool, terminal, or session authority.

## Running one host

Run the listener behind a TLS reverse proxy. The binary deliberately accepts
only an IP:port because it does not terminate TLS itself.

```sh
export ZETA_COLLABORATION_BEARER_TOKEN='a-random-visible-ascii-token-at-least-32-characters'
export ZETA_COLLABORATION_ALLOWED_ORIGIN='https://desktop.example'
cargo run -p zeta-collaboration-server -- 127.0.0.1:8421 /srv/zeta/collaboration.sqlite3
```

`ZETA_COLLABORATION_ALLOWED_ORIGIN` is a comma-separated allowlist. It must
include the exact browser origin serving Gama. The desktop toolbar asks for the
public HTTPS origin and bearer token, then creates or joins a room ID.

The host exposes:

- `POST /v1/document-collaboration/rooms/open`
- `POST /v1/document-collaboration/rooms/submit`
- `GET /v1/document-collaboration/rooms/{roomId}/updates?afterVersion=N`
- `POST /v1/document-collaboration/rooms/presence`
- `GET /v1/document-collaboration/rooms/{roomId}/presence?afterGeneration=N`
- `POST /v1/document-collaboration/rooms/invites`
- `GET /v1/document-collaboration/rooms/{roomId}/members`
- `POST /v1/document-collaboration/rooms/members/rotate-token`
- `POST /v1/document-collaboration/rooms/members/revoke`
- `GET /v1/document-collaboration/rooms/{roomId}/audit`

Every non-preflight request requires `Authorization: Bearer <token>`. The
deployment token bootstraps the persistent `server-admin` room owner; an owner
can issue a room-scoped bearer token for an owner, editor, or viewer. A scoped
token cannot access another room. JSON responses use `Cache-Control: no-store`;
browser requests receive CORS headers only for an allowed origin. `GET updates`
and `GET presence` wait up to 25 seconds when no change is available, then
return a canonical replay or current ephemeral selection set.

Multiple processes can share one SQLite database. A local submission wakes
same-host polls immediately; other hosts recheck persisted state every 250 ms,
so ordered writes and presence do not wait for the 25-second long-poll timeout.

## Current security boundary

The deployment bearer is only a bootstrap credential, not a room share
capability. Room access is enforced through persistent identities and roles;
issued tokens are stored only as SHA-256 hashes and are returned once when
created or rotated. Owner-visible audit events record room creation, member
invitation/revocation, credential rotation, and accepted submissions. The host
validates wire bounds, Gama envelopes, generic node/mark/selection structure,
and known core transaction kinds; the active browser profile validates its own
profile-specific schema. Do not expose this endpoint directly to untrusted
networks without TLS, a strong secret, and an origin allowlist.

`cargo test -p zeta-collaboration-server` exercises CORS preflight, auth,
room-role enforcement, credential rotation, audit access, presence, ordered
updates, and external updates observed through a second host.
