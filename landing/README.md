# trident.pink — the landing

Lives inside the [trident](https://github.com/cyberia-to/trident) repo
as `landing/` — folded in from the standalone `trident-pink` repo
(full history preserved via `git subtree`). One page: **the provable
language** — native release downloads, proof modes, the Trinity demo,
language features, targets and family links (cyberia · soft3 · cyb · cyber).
Static HTML, no build; the hero video is re-encoded from `../media/tri.gif`.

Trinity's homepage pitch is **AI. Privacy. Quantum. One field.** It links to
the [arithmetic contract](../reference/trinity-arithmetic.md) and
[Trident source](../lib/std/trinity/inference.tri). Keep its research-demo
status visible: the current implementation demonstrates arithmetic on Triton;
secure FHE/private inference remain research goals and the quantum stage is
simulated. The contract defines current guarantees.

## Deploy

```bash
cd trident/landing
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
