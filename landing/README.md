# trident.pink — the landing

One page: **the provable language** — hero with the trident girl
(`trident/media/tri.gif` re-encoded to mp4), `cargo install trident-lang`
CTA, the cycle-count table, six cards, Neptune, family links
(cyberia · soft3 · cyb · cyber). Static HTML, no build. Source of truth
for copy: `~/cyber/trident/README.md`.

## Deploy

```bash
nu scripts/deploy.nu
```

## Server

- nginx: `/etc/nginx/sites-enabled/trident.pink` (from `scripts/nginx-trident.pink.conf`)
- TLS: certbot
- analytics: lytics tracker, `/lytics/` proxied to 127.0.0.1:8091 on cyberproxy

## DNS (Namecheap)

| type | host | value |
|------|------|-------|
| A | @ | 167.235.28.94 |
| CNAME | www | trident.pink |
