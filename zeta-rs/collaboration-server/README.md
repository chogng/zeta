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

The host exposes only:

- `POST /v1/document-collaboration/rooms/open`
- `POST /v1/document-collaboration/rooms/submit`
- `GET /v1/document-collaboration/rooms/{roomId}/updates?afterVersion=N`

Every non-preflight request requires `Authorization: Bearer <token>`. JSON
responses use `Cache-Control: no-store`; browser requests receive CORS headers
only for an allowed origin. `GET updates` waits up to 25 seconds when no update
is available, then returns the canonical ordered replay or a resync snapshot.

This is a single logical host design. Multiple processes sharing the database
remain durable, but only the process accepting a submission can wake its own
long polls immediately; other processes discover the change on their next
poll timeout. Deploy a single primary host until a shared notification backend
is introduced.

## Current security boundary

The bearer token authorizes a deployment, and the random room ID is the share
capability. There are no per-user identities, per-room ACLs, token rotation,
or audit records yet. The host validates wire bounds/JSON and delegates full
Gama document/transaction validation to the client schema adapter. Do not
expose this endpoint directly to untrusted networks without TLS, a strong
secret, and an origin allowlist.

`cargo test -p zeta-collaboration-server` exercises CORS preflight, auth,
remote create/join/submit/update flow, and SQLite-backed room ordering.
