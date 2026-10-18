# Produktionsbetrieb mit Apache oder Nginx

Neue Projekte enthalten den Ordner `public/`. Verwende ihn als DocumentRoot
für öffentliche Dateien wie Bilder, Stylesheets, JavaScript und `robots.txt`.
`zelyra serve` kann Dateien aus diesem Verzeichnis direkt ausliefern; deklarierte
Zelyra-Routen haben Vorrang. Apache oder Nginx kann Dateien alternativ direkt
ausliefern und übrige Anfragen an Zelyra `serve` weiterleiten.

Zelyra sollte nur an Loopback gebunden sein. Installiere dafür den
[systemd-Projektdienst](service.de.md) mit einem festen Port und setze
`ZELYRA_ALLOWED_HOSTS` auf den öffentlichen Hostnamen, zum Beispiel
`app.example.com`. Öffne den Zelyra-Port nicht direkt im Internet. TLS endet am
Webserver; der Proxy muss den öffentlichen `Host` weitergeben und
`X-Forwarded-Proto` selbst setzen oder überschreiben.
Zelyra prüft den ursprünglichen `Host` gegen `ZELYRA_ALLOWED_HOSTS` und
ignoriert `X-Forwarded-Host`; der Proxy muss den öffentlichen Host in `Host`
bewahren. Der Anwendungsport darf ausschließlich über diesen vertrauenswürdigen
Proxy erreichbar sein. Ein vom Client geliefertes `X-Forwarded-Proto` darf der
Proxy nicht anhängen, sondern muss es an der Proxy-Grenze überschreiben.

## Apache

Aktiviere `mod_proxy`, `mod_proxy_http`, `mod_rewrite` und `mod_headers`. Passe
Pfade, Hostnamen und Port an. Dieses Beispiel liefert nur vorhandene Dateien
aus `public/` direkt aus und proxyt alle anderen Pfade an Zelyra:

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

Konfiguriere TLS-Zertifikate nach den Vorgaben deiner Distribution. Leite
`/.well-known/` bei Bedarf gezielt weiter, statt Directory-Listing oder
Symlink-Verfolgung pauschal zu aktivieren.

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

Ergänze TLS-Zertifikat und -Schlüssel gemäß deiner Systemkonfiguration.
Vermeide `alias`-Konstruktionen, die Dateien außerhalb von `public/` erreichbar
machen. Vertraue `X-Forwarded-*` nur, wenn der Anwendungsport ausschließlich
vom lokalen Proxy erreichbar ist.

## Grenzen

- `public/` wird weder automatisch in Zelyra-Routen noch in Compilerimporte
  aufgenommen. `serve` liefert nur Dateien bis 32 MiB und nur über GET/HEAD aus;
  Dotfiles sind gesperrt, mit Ausnahme von `.well-known/`.
- Statische HTML-Dateien erhalten `default-src 'self'`; Inline-Skripte und
  Inline-Styles sind dadurch blockiert. Binde solche Assets als lokale Dateien
  ein.
- Die eingebaute Auslieferung prüft kanonische Pfade und weist Ziele außerhalb
  des echten `public/`-Verzeichnisses zurück. Sie ersetzt keine TLS-Terminierung
  oder vorgeschaltete Produktionskonfiguration.
- Die Beispiele nehmen einen lokalen Zelyra-Dienst auf Port 3000 an.
- Weitergereichte Anfragen müssen zum Projekt-Host-Allowlistwert passen.
- `X-Forwarded-Host` ändert den akzeptierten Host nicht. Die Allowlist wird
  gegen den Anfrage-Header `Host` geprüft.
- Diese Beispielkonfigurationen ersetzen keinen Test mit der konkreten
  Apache-/Nginx-Version, TLS-Konfiguration, Dateirechten und Firewall.
