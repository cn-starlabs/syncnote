# SyncNote — Dev Plan

## Overview

SyncNote is a web app with two note types:

1. **Personal notes** — each user has their own private notes, autosaved.
2. **Shared pages** — a user creates a page and invites others; anyone with
   access can type into it and see others' edits live.

## Stack

- **Leptos 0.8** (SSR, `cargo-leptos`) + **Axum 0.8** — matches the pattern
  used in `gaokaozhiyuan`.
- **sqlx 0.8** with **SQLite** — single file DB, no ops overhead.
- **tower-sessions** for session cookies, **argon2** for password hashing.
- **Tailwind v4** (CSS-first: `@import "tailwindcss"`, no `tailwind.config.js`),
  pinned via `LEPTOS_TAILWIND_VERSION` in `.env`.
- **axum::extract::ws** + **tokio::sync::broadcast** for live sync on shared
  pages (no external message broker needed at this scale).
- **pulldown-cmark** for Markdown → HTML rendering (personal notes and
  shared pages are both stored as Markdown source text).

## Data model (`migrations/0001_init.sql`)

- `users` (id, email, password_hash, display_name, created_at)
- `notes` (id, owner_id, title, body, updated_at) — personal notes, one row
  per note, `owner_id` scopes all queries. `body` is Markdown source text.
- `shared_pages` (id, owner_id, title, body, version, updated_at) — `body` is
  Markdown source text; `version` is an integer bumped on every save, used
  for the conflict check below.
- `shared_page_members` (page_id, user_id, role) — role is `owner` | `editor`
  | `viewer`; controls who can open/edit a given page.
- `shared_page_invites` (id, page_id, token, role, expires_at) — shareable
  invite links (`/shared/join/:token`) as an alternative to inviting by email.

SQLite is fine here because writes are scoped per-row (per note / per shared
page) — there's no cross-table hot path that needs Postgres-grade concurrent
write throughput.

## Sync design (shared pages)

- **Transport**: one WebSocket per open shared page. Server holds a
  `HashMap<page_id, tokio::sync::broadcast::Sender<Edit>>` in `AppState`,
  created lazily on first connect, dropped when the last client disconnects.
- **Edit message**: `{ page_id, body, version, editor_id }`, where `body`
  is the raw Markdown source (the editor is a plain textarea; formatting is
  rendered separately, see below). On receipt the
  server validates `version` against the DB row:
  - matches → write, bump `version`, broadcast the new `(body, version)` to
    all connected clients (including a distinct id so the sender can ignore
    its own echo).
  - stale (someone else saved in between) → reject the write, send the
    current `(body, version)` back to just that client so its editor can
    reconcile (last-write-wins at the field level, but the client always
    knows when it lost a race instead of silently overwriting).
- **Debounce**: client sends edits on a ~300–500ms debounce, not per
  keystroke, to keep broadcast volume and DB writes reasonable.
- **Personal notes** don't need the WebSocket path — a plain debounced
  autosave server function (`save_note(id, body)`) is enough since only the
  owner ever writes to them.
- **Rendering**: source textarea on one side, a preview pane on the other
  (or a toggle on mobile) rendered via `pulldown-cmark`. Since the sync
  payload is always the raw Markdown string, the preview is purely a local
  re-render on every update — it never affects the sync/versioning logic
  above.
- **Future upgrade path**: if simultaneous same-line typing ever needs true
  merge (not just "last save wins"), swap the body field for a CRDT (e.g.
  the `loro` crate) without touching the transport/auth layers.

## Admin user management

- `/app/admin/users` (a `AdminTabs` component switches between this and the
  existing invites page): lists all users with role/status/last-login, and
  per-row actions — reset password, lock/unlock, delete.
- `users.locked` column. A locked account is enforced in two places: at
  login time (`auth_fns::login` and `passkey_fns::finish_discoverable_login`
  reject with a clear "account has been locked" message before establishing
  a session), and in `auth::session::current_user` (treats a locked account
  as logged out on every request — so if an admin locks someone who's
  already mid-session, they're kicked out on their next page load/action
  without needing to hunt down and invalidate a specific session row).
- Reset password generates a random 12-char temp password server-side and
  returns it once in the response for the admin to copy — it's never
  recoverable afterward, matching how the argon2 hash is stored (one-way).
- An admin can't lock or delete their own account (checked server-side in
  `set_user_locked` / `delete_user`), to avoid an admin locking themselves
  out with no other admin around.
- Deleting a user cascades via existing `ON DELETE CASCADE` foreign keys
  (notes, passkeys, shared_page_members) — a shared page they *own* is
  deleted too (`shared_pages.owner_id` cascades), which also removes other
  members' access to it. No special-casing added for that; it's the same
  behavior as if the owner deleted the page themselves.

## Passkeys (WebAuthn)

- `webauthn-rs` (server) + `webauthn-rs-proto` with its `wasm` feature
  (browser client) — the proto crate's `From` impls convert directly between
  its JSON-safe types and the real `web_sys` Credential Management API types,
  so no manual base64url/ArrayBuffer handling is needed.
- Ceremony endpoints are ordinary Leptos server functions
  (`src/server/passkey_fns.rs`), not raw Axum routes — unlike attachments,
  passkey registration/login is just JSON string round-trips (the actual
  `navigator.credentials.create/get()` browser call happens client-side,
  between the "start" and "finish" server calls), which fits the server-fn
  convention fine. Challenge/response payloads cross the server-fn boundary
  as JSON `String`, not the native `webauthn-rs-proto` types — those types
  only need to be nameable inside `hydrate`-gated code (`client_passkey.rs`),
  consistent with the `web_sys`-not-in-ssr gotcha above.
- Registration/authentication state (`PasskeyRegistration`/`PasskeyAuthentication`)
  is stored server-side in the existing session (`danger-allow-state-serialisation`
  feature) between start/finish calls — safe because the session store is
  server-authoritative (SQLite-backed), not client-readable.
- Per-user WebAuthn handle is derived deterministically from our own `users.id`
  via `Uuid::from_u128` rather than a separate stored UUID column — a
  pragmatic simplification since that handle never leaves the server in a
  readable way.
- `passkeys` table: one row per registered credential (`user_id`, `label`,
  `passkey_json` — the serialized `Passkey`), so a user can register more
  than one (phone + laptop + security key, etc.) and revoke individually.
- **Dev gotcha**: `rp_id` must exactly match the browser's effective domain —
  browse via `http://localhost:3000`, not `http://127.0.0.1:3000`, or the
  ceremony fails even though the server binds `127.0.0.1`. Configurable via
  `SYNCNOTE_RP_ID` / `SYNCNOTE_RP_ORIGIN` for real deployment.
- Login is usernameless (discoverable-credential / "conditional UI"): the
  email field is tagged `autocomplete="username webauthn"`, and on page load
  we start a conditional `navigator.credentials.get()` in the background
  (`webauthn.start_discoverable_authentication`, gated behind webauthn-rs's
  `conditional-ui` feature — a "preview" feature per the crate, but widely
  supported by Chrome/Safari/Edge in practice). The browser offers matching
  passkeys as an autofill suggestion under that field; picking one resolves
  the pending promise. The server doesn't know who's authenticating until
  the response comes back — `identify_discoverable_authentication` extracts
  a user handle from it first, which we reverse into our `users.id` (see the
  `Uuid::from_u128` note above), *then* verify via
  `finish_discoverable_authentication`. If the browser doesn't support
  conditional mediation (checked via `PublicKeyCredentialExt::is_conditional_mediation_available`),
  this silently does nothing and the password form still works normally.
- `start_passkey_registration` (the high-level webauthn-rs function we use)
  hardcodes `residentKey: discouraged`, with no lighter-weight way to change
  it server-side short of `start_attested_resident_key_registration` (needs
  a manufacturer attestation CA list — real infrastructure, not worth it
  here). Chrome+platform-authenticator combos were observed to actually
  *honor* "discouraged" and create a non-discoverable credential, breaking
  usernameless login entirely (confirmed by testing against
  <https://webauthn.io>, which explicitly requests a resident key and works
  fine on the same browser/device). Fix: `residentKey` is a client-side
  request parameter the server never re-verifies afterward (there's no way
  for an RP to cryptographically confirm it was honored either — that's why
  `credProps.rk` exists purely as an *informational* extension), so
  `client_passkey.rs::register_passkey` deserializes the `CreationChallengeResponse`
  and overrides `authenticator_selection.resident_key` to `Required` before
  ever handing it to `navigator.credentials.create()`. No server changes
  needed. Passkeys registered *before* this fix remain non-discoverable and
  must be deleted and re-registered.

## Attachments (files & images)

- `attachments` table: `owner_id`, `scope` (`note` | `shared_page`), `scope_id`,
  `filename`, `content_type`, `byte_size`, `stored_name` (random token used
  as the on-disk filename, decoupled from the user-supplied `filename`).
- Files live on local disk under `./uploads/` (path configurable via
  `SYNCNOTE_UPLOADS_DIR`) — consistent with the single-instance deployment
  decision, no object storage needed.
- Plain Axum routes, not Leptos server functions (multipart isn't a good fit
  for the server-fn call convention): `POST /api/upload` (multipart: `scope`,
  `scope_id`, `file`; checked against edit permission on that note/page, capped
  at 10MB) and `GET /attachments/{id}` (checked against read permission,
  streams the file with its stored content-type).
- Client-side: browser `fetch` + `FormData` from the `hydrate` build
  (`src/client_upload.rs`) posts the selected file and, on success, inserts
  `![name](url)` (images) or `[name](url)` (other files) into the Markdown
  body — reusing the existing render/sync pipeline unchanged.

## Auth & sharing

- Email + password (argon2 hash), signup is invite-code gated: `invite_codes`
  (code, uses_left, expires_at, note) — same shape as gaokaozhiyuan's, but
  here every code is created by an admin from `/app/admin/invites` rather
  than pre-seeded. First admin account is created via `SYNCNOTE_BOOTSTRAP_ADMIN=email:password`
  in `.env` on first run while the `users` table is empty (see `db::bootstrap_admin_if_requested`).
  `users.is_admin` gates the admin UI; `RequireAdmin` mirrors `RequireAuth`.
- These signup invite codes are a separate concept from the shared-page
  invite links below — same idea (a code/token unlocking access) applied at
  two different layers (whole-app signup vs. a single shared page).
- Sharing a page: owner generates an invite link (`shared_page_invites`)
  with a role (`editor`/`viewer`) and optional expiry; recipient clicking it
  while logged in (or after a quick signup) is added to
  `shared_page_members`.
- All page/note server functions check membership/ownership before
  reading or writing — no client-trusted IDs.

## Phases

**Phase 0 — Scaffold**
- `cargo leptos new` layout on top of the existing `Cargo.toml`/`src/main.rs`.
- Wire Axum + Leptos SSR boilerplate, Tailwind v4 pipeline, `.env`, sqlx
  pool, `migrations/0001_init.sql`, session middleware.

**Phase 1 — Auth**
- Signup / login / logout pages, session-backed `current_user` extractor,
  password hashing, basic account page.

**Phase 2 — Personal notes**
- CRUD for `notes`: list, create, open/edit, delete.
- Markdown editor: textarea + `pulldown-cmark` preview pane (split view,
  toggle on narrow screens).
- Debounced autosave server function; simple optimistic-UI save indicator.

**Phase 3 — Shared pages (data + CRUD)**
- CRUD for `shared_pages` + `shared_page_members`.
- Create page, view page (read-only if not yet real-time), member list,
  role management (owner can change/revoke roles).

**Phase 4 — Live sync**
- WebSocket route (`/ws/page/:id`), `AppState` broadcast map, client-side
  WS hook in the page editor, version-checked writes as described above.
- Presence indicator (who's currently viewing/editing) — nice-to-have if
  time allows, riding on the same socket.

**Phase 5 — Invite links & polish**
- Shareable invite links with role + expiry, join flow for existing/new
  users.
- Dark mode (matches the pattern from gaokaozhiyuan: `@variant dark`,
  localStorage toggle, no-flash boot script).
- Empty states, error handling on WS reconnect (auto-retry with backoff).

**Phase 6 — Tests + deploy**
- Integration tests for auth and the version-conflict path in particular
  (two clients racing a save).
- Deployment target TBD (ask before provisioning anything).

## Scale

Confirmed single-instance deployment — SQLite + in-process
`tokio::sync::broadcast` for shared-page sync is the right fit, no need for
a separate message broker or a networked DB.

## Phase 7 — Math Formula Support (LaTeX / KaTeX)

### Objective
Enable inline (`$...$`) and block (`$$...$$`) math formulas in personal notes and collaborative shared pages without breaking raw HTML sanitization or code block formatting.

### Architecture & Implementation
1. **Markdown Processing Pipeline (`src/components/markdown.rs`)**:
   - Extends the `pulldown-cmark` pipeline with formula syntax parsing.
   - Ignores code spans (`<code>...</code>`) and code blocks (`<pre><code>...</code></pre>`) so programming code using `$` isn't mangled.
   - Wraps inline math into `<span class="katex-math-inline" data-expr="...">...</span>`.
   - Wraps block math into `<div class="katex-math-block" data-expr="...">...</div>`.
   - HTML attribute values are safely escaped to prevent attribute injection.
2. **KaTeX Integration (`src/app.rs`)**:
   - Injects KaTeX stylesheet and runtime library in the document `<head>` via CDN.
   - Defines `window.renderMathInSyncNote()` to scan for unrendered math elements and render them via KaTeX without throwing.
3. **Reactive Leptos Preview Re-rendering**:
   - An `Effect` in `MarkdownPreview` triggers `trigger_katex_render()` whenever the markdown body signal updates, ensuring live rendering during typing and live WebSocket synchronization.
4. **Styling (`style/tailwind.css`)**:
   - Added styling for centered block math with scrollable overflow and inline math formatting.

## Phase 8 — UI/UX & Editor Experience Polish

### 1. Theme Switcher (Dark / Light Mode)
- Added an interactive theme toggle in [`src/components/nav.rs`](src/components/nav.rs) with Sun/Moon icons, updating the HTML document root `dark` class and persisting state to `localStorage`.

### 2. Modern Action Toolbar & View Mode Controls
- **Editor Quick Toolbar**: In both [`NoteEditor`](src/pages/note_editor.rs) and [`SharedPageEditor`](src/pages/shared_page_editor.rs), added quick-insert chips for Bold, Italic, Headings, Code Blocks, Inline Math `$f(x)$`, Block Math `$$ Block $$`, Tasks, and Tables.
- **View Mode Switcher**: Added responsive `Edit`, `Split`, and `Preview` modes allowing focused writing or full-width reading.
- **Word & Character Count**: Added a live word and character count indicator in the editor toolbar.

### 3. Safety & Feedback Polish
- **Delete Note Confirmation Modal**: Added a modal in [`src/pages/dashboard.rs`](src/pages/dashboard.rs) to prevent accidental note deletion.
- **Empty State**: Added an empty state with a direct CTA button in the notes dashboard.
- **Unified Title Card Inputs**: Applied high-contrast card styling with focus rings to both personal notes and shared pages.

## Phase 9 — Collaboration Polish

### 1. Real-Time WebSocket Connection Status Indicator
- Extended [`client_ws.rs`](src/client_ws.rs) with a `WsStatus` enum (`Connecting`, `Connected`, `Disconnected`, `Error`) and connected `onopen`, `onclose`, and `onerror` event listeners.
- Integrated a live status badge in [`SharedPageEditor`](src/pages/shared_page_editor.rs):
  - 🟢 **Live**: Emerald pulsing indicator when the WebSocket is actively connected.
  - 🟡 **Connecting…**: Amber pinging indicator during initial connection or reconnection.
  - 🔴 **Offline**: Rose indicator when the WebSocket connection is dropped or closed.

### 2. Collaborator Avatars & Presence Badges
- In [`SharedPageEditor`](src/pages/shared_page_editor.rs), added live collaborator avatar badges with member initial bubbles and count indicators (e.g., `+3`) to show everyone who has access to the shared page.

### 3. Shared Page Management & Role Badging
- Added color-coded role badges (`owner` in purple, `editor` in blue, `viewer` in neutral slate) in [`SharedPagesListPage`](src/pages/shared_pages_list.rs).
- Added an owner-only delete button with a permanent deletion confirmation modal in the shared pages list.



