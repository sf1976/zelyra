# Produktionsbetrieb mit Apache oder Nginx

Neue Projekte enthalten den Ordner `public/`. Verwende ihn als DocumentRoot
für öffentliche Dateien wie Bilder, Stylesheets, JavaScript und `robots.txt`.
Apache oder Nginx liefert vorhandene Dateien direkt aus und leitet übrige
Anfragen an Zelyra `serve` weiter. Der Zelyra-Server selbst liefert derzeit
keine beliebigen Dateien aus `public/` aus.

Zelyra sollte nur an Loopback gebunden sein. Installiere dafür den
[systemd-Projektdienst](service.de.md) mit einem festen Port und setze
`ZELYRA_ALLOWED_HOSTS` auf den öffentlichen Hostnamen, zum Beispiel
`app.example.com`. Öffne den Zelyra-Port nicht direkt im Internet. TLS endet am
Webserver; der Proxy muss den öffentlichen `Host` weitergeben und
`X-Forwarded-Proto` selbst setzen oder überschreiben.

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

- `public/` ist ein Verzeichnisvertrag für den vorgeschalteten Webserver; es
  wird nicht automatisch in Zelyra-Routen oder Compilerimporte aufgenommen.
- Die Beispiele nehmen einen lokalen Zelyra-Dienst auf Port 3000 an.
- Weitergereichte Anfragen müssen zum Projekt-Host-Allowlistwert passen.
- Diese Beispielkonfigurationen ersetzen keinen Test mit der konkreten
  Apache-/Nginx-Version, TLS-Konfiguration, Dateirechten und Firewall.
- Die integrierte Auslieferung statischer Dateien durch `zelyra serve` samt
  eigener Pfad- und Symlink-Prüfung bleibt für einen späteren Release-Schnitt.
