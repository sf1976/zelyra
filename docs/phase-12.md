# Zelyra 0.1 — Phase 12: Request validation and secure response defaults

[Deutsch](phase-12.de.md) · English

API request processing now has a small, predictable validation boundary.
Non-`GET`/`DELETE` API bodies may use `application/json` or
`application/x-www-form-urlencoded`; unsupported media types return
`415 Unsupported Media Type` as a structured JSON error. JSON bodies continue
to require an object at the API boundary, after which every declared field is
converted and checked against its Zelyra type.

The HTTP parser validates `Content-Length`, reads complete bodies across
multiple network reads, rejects malformed or incomplete bodies, and limits
request bodies to one mebibyte. Headers are limited to 64 KiB. A body
exceeding the limit receives `413 Payload Too Large`. The limit applies before
API handler execution.

Responses include these secure defaults:

~~~http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: same-origin
~~~

The defaults apply to HTML, JSON, redirects, errors, and empty preflight
responses. Application-specific headers can still be added by the runtime's
response construction; these defaults are not a replacement for TLS,
authentication, authorization, CSRF protection, or a content security policy
appropriate to a deployment.

CORS behavior from [Phase 11](phase-11.md) remains unchanged. Request body
validation is independent of CORS, so direct non-browser clients receive the
same validation behavior.
