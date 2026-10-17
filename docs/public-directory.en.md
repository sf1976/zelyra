# Serve static files from `public/`

`zelyra init` and `zelyra new` create `public/` as the explicit directory for
publicly reachable files. `zelyra serve` serves files from this directory when
no declared application route handles the request. `/` remains a normal
Zelyra route from `main.zyl`; `public/index.html` is available at `/index.html`
and does not replace the application entry route.
If `public/` is a file or symlink, startup fails with `E-WEB-006`.

```text
project/
├── main.zyl
└── public/
    ├── images/logo.png
    ├── css/site.css
    └── documents/terms.pdf
```

These files are available at `/images/logo.png`, `/css/site.css`, and
`/documents/terms.pdf`. Apache and Nginx can remain reverse proxies in front
of `zelyra serve`; they do not execute Zelyra source files.

## Limits and protection

- Only `GET` and `HEAD` serve static files. Application routes are handled
  first and take precedence.
- The public root must be a real directory. A symlink used as `public/` or
  anywhere in a requested file path is rejected. Symlinks whose target stays
  inside `public/` are also not served.
- Path segments are decoded and checked. `..`, encoded separators, backslashes,
  invalid UTF-8, and hidden files are not served; `.well-known/` is allowed for
  standardized web challenges.
- Individual files are limited to 4 MiB to keep a request from consuming
  unbounded memory. Larger files receive HTTP 413.
- Content types use a bounded extension map; unknown extensions use
  `application/octet-stream`. Responses include `X-Content-Type-Options:
  nosniff` and are cached for one hour.
- Files are public. Do not place `.env`, credentials, or private documents in
  `public/`.

Static serving does not provide directory listings, byte ranges, upload or
mutation endpoints. Use an appropriate application route or dedicated file
service for large downloads or private files.
