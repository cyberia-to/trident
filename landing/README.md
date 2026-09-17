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

## The set

The page plays `media/trident.pink.set.mp3` (20 min) from the bottom bar;
the file is a [release asset](https://github.com/cyberia-to/trident/releases/tag/set-2026-09-17),
not tracked in git, and `deploy.nu` never deletes it from the server. If the
server copy is missing the player falls back to the release URL. Browsers
allow sound only after a gesture: the script tries at once, otherwise the
first click, key, touch or scroll starts it.

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
