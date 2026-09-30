# Consuming the Estimator9 API from Angular

Everything the frontend needs: how authentication works, what the interceptor
has to do, and the responses worth handling specially.

**Base URL:** `http://localhost:3000` in development.
**Interactive reference:** `http://localhost:3000/swagger-ui`
**Machine-readable spec:** `http://localhost:3000/api-docs/openapi.json`

---

## The one thing that will cost you an afternoon

Every request must be sent with **`withCredentials: true`**.

The refresh token lives in an httpOnly cookie. Without this flag the browser
sends your login happily, gets the cookie, and then silently omits it on
`/auth/refresh` — which looks like a broken refresh endpoint, not a missing
flag. Set it once, in the interceptor, for every request.

The API's CORS policy allows credentials and names `http://localhost:4200`
explicitly. If your dev server runs on another port, it must be added to
`CORS_ALLOWED_ORIGINS` on the API side — a wildcard is not possible when
credentials are involved.

---

## Two tokens, deliberately different

| | Access token | Refresh token |
|---|---|---|
| Where | JSON body → **keep in memory** | httpOnly cookie, set by the API |
| Lifetime | 15 minutes | 7 days, rotated on every use |
| Sent as | `Authorization: Bearer …` | automatically by the browser |
| Readable by script | yes | **no** |

**Do not put the access token in `localStorage`.** Keep it in a service field.
It dies with the tab, which is the intent — the refresh cookie is what survives
a reload, and script cannot read it, so an XSS hole cannot steal a long-lived
credential.

On app start you will have no access token but may still hold a valid cookie.
Call `POST /auth/refresh` first; if it succeeds you are signed in.

---

## The flow

```
POST /auth/login          { email, password }
  → 200 { access_token, token_type, expires_at, user }
  → Set-Cookie: refresh token (httpOnly, Path=/auth)

GET  /auth/me             Bearer token
  → 200 the signed-in user, with roles and is_superadmin

POST /auth/refresh        no body — the cookie is the credential
  → 200 a new access_token, and a rotated cookie

POST /auth/logout         no body
  → 204, cookie cleared
```

Each refresh token works **once**. The API rotates it and revokes the old one,
so do not run two refreshes concurrently — serialise them, or the second
presents a token the first already spent. A replayed token ends the whole
session deliberately, and both tabs get logged out.

---

## The interceptor

```ts
intercept(req: HttpRequest<unknown>, next: HttpHandler) {
  const authed = req.clone({
    withCredentials: true,
    setHeaders: this.auth.accessToken
      ? { Authorization: `Bearer ${this.auth.accessToken}` }
      : {},
  });

  return next.handle(authed).pipe(
    catchError((error: HttpErrorResponse) => {
      // 401: the token is missing or expired. Refresh once, then retry.
      if (error.status === 401 && !this.refreshing) {
        return this.auth.refresh().pipe(
          switchMap(() => next.handle(this.withFreshToken(req))),
          catchError(() => this.auth.signOutAndRedirect()),
        );
      }

      // 403: authenticated but not permitted. Retrying will never help.
      // Refreshing here is the classic mistake — it produces an infinite loop
      // on a page the user simply cannot see.
      return throwError(() => error);
    }),
  );
}
```

`401` and `403` mean genuinely different things and the API is careful to
distinguish them. Treat them differently or you will loop.

---

## Responses worth handling specially

Every error has the same shape, including rate limits and unknown routes:

```json
{ "error": "invalid credentials" }
```

| Status | Meaning | What the UI should do |
|---|---|---|
| **401** | no/expired/invalid token, or wrong credentials | refresh once, then sign out |
| **403** | authenticated, lacks the permission | show "not available", never retry |
| **404** | no such record, or unknown endpoint | |
| **409** | email already in use | field-level error on the email input |
| **422** | validation — password too short, malformed email, nothing to change | field-level errors |
| **423** | correct password, **account locked** | show the unlock time from the message |
| **429** | rate limited | back off using the `Retry-After` header (seconds) |
| **503** | database unavailable, transient | retry with backoff; `Retry-After` is set |

`Retry-After` and `Location` are exposed to script; other headers are not
readable cross-origin.

**423 is worth a dedicated message.** It only ever appears when the password
was *correct* and the account is locked after repeated failures, so "your
password is wrong" would be actively misleading. The body carries the time the
lock lifts.

---

## Permissions in the UI

`GET /auth/me` returns `roles` and `is_superadmin`. Use them to decide what to
render, but treat it as cosmetic — the API enforces permissions regardless, and
a hidden button is not a security control.

`GET /roles` and `GET /permissions` (both need `roles.read`) give the full
vocabulary with descriptions, for an admin screen's role picker. `superadmin`
appears in that list flagged `is_superadmin: true`; render it as **unassignable**
rather than offering it, since assigning it always returns 403.

Permissions are baked into the access token, so a role granted to someone while
they are signed in takes effect on their **next token**, within 15 minutes, not
on their next request.

---

## Generating a client

```bash
npx @openapitools/openapi-generator-cli generate \
  -i http://localhost:3000/api-docs/openapi.json \
  -g typescript-angular \
  -o src/app/api
```

The spec is OpenAPI 3.1. If your generator only supports 3.0, either use a
generator that handles 3.1 or write the handful of services by hand — there are
fewer than twenty endpoints.

Regenerate after any API change; `docs/openapi.json` in the repo is kept in
step with the code and is the same document.

---

## Endpoints

| Method | Path | Needs |
|---|---|---|
| POST | `/auth/login` | — |
| POST | `/auth/refresh` | cookie |
| POST | `/auth/logout` | cookie |
| GET | `/auth/me` | token |
| PATCH | `/auth/me` | token |
| POST | `/auth/password/change` | token + current password |
| POST | `/auth/password/forgot` | — |
| POST | `/auth/password/reset` | emailed token |
| GET | `/users` | `users.read` |
| GET | `/users/{id}` | `users.read` |
| POST | `/users` | `users.manage` |
| PATCH | `/users/{id}` | `users.manage` |
| DELETE | `/users/{id}` | `users.manage` |
| PUT | `/users/{id}/roles` | `users.manage` |
| GET | `/roles` | `roles.read` |
| GET | `/permissions` | `roles.read` |
| GET | `/health` | — |

`DELETE /users/{id}` deactivates rather than deletes, and ends that user's
sessions immediately.

---

## In development

Password reset emails are not sent. They are appended to `logs/mail.log` on the
API side — open that file to get the reset link while testing the flow.
