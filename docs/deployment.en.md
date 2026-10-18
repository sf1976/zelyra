# Production deployment with Apache or Nginx

New projects include a `public/` directory. Use it as the document root for
public files such as images, stylesheets, JavaScript, and `robots.txt`. Apache
or Nginx can serve files that exist there and forward the remaining requests
to Zelyra `serve`. Zelyra `serve` can also serve files from this directory;
declared Zelyra routes take precedence.

Bind Zelyra to loopback. Install the
[systemd project service](service.en.md) on a fixed port and set
`ZELYRA_ALLOWED_HOSTS` to the public hostname, for example
`app.example.com`. Do not expose the Zelyra port directly to the Internet. TLS
terminates at the web server; the proxy must preserve the public `Host` and
set or overwrite `X-Forwarded-Proto` itself.

## Apache

Enable `mod_proxy`, `mod_proxy_http`, `mod_rewrite`, and `mod_headers`. Adjust
paths, hostnames, and port. This example serves existing files from `public/`
directly and proxies other paths to Zelyra:

```apache
<VirtualHost *:443>
    ServerName app.example.com
    DocumentRoot /srv/zelyra-app/public

    <Directory /srv/zelyra-app/public>
        Require all granted
        Options -Indexes -FollowSymLinks
        AllowOverride None
    </Directory>

    RewriteEngine On
    RewriteCond %{REQUEST_FILENAME} -f [OR]
    RewriteCond %{REQUEST_FILENAME} -d
    RewriteRule ^ - [END]
    RewriteRule ^ http://127.0.0.1:3000%{REQUEST_URI} [P,L]

    ProxyPreserveHost On
    RequestHeader set X-Forwarded-Proto "https"
</VirtualHost>
```

Configure TLS certificates according to your distribution. If needed, route
`/.well-known/` explicitly instead of enabling directory listings or symlink
following broadly.

## Nginx

```nginx
server {
    listen 443 ssl;
    server_name app.example.com;
    root /srv/zelyra-app/public;
    index index.html;
    autoindex off;
    disable_symlinks on from=$document_root;

    location / {
        try_files $uri $uri/ @zelyra;
    }

    location @zelyra {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

Add TLS certificate and key configuration for your system. Avoid `alias`
configurations that expose files outside `public/`. Trust `X-Forwarded-*` only
when the application port is reachable exclusively through the local proxy.

## Limits

- `public/` is not automatically added to Zelyra routes or compiler imports.
  `serve` serves files up to 32 MiB through GET/HEAD only; dotfiles are denied
  except beneath `.well-known/`.
- Static HTML gets `default-src 'self'`, which blocks inline scripts and styles.
  Load those assets from local files instead.
- The built-in handler checks canonical paths and rejects targets outside the
  real `public/` directory. It does not replace TLS termination or production
  reverse-proxy configuration.
- The examples assume a local Zelyra service on port 3000.
- Forwarded requests must match the project's host allowlist.
- These examples do not replace testing with your Apache/Nginx version, TLS
  configuration, file permissions, and firewall.
