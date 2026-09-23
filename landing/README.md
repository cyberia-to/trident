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

Merge to `master` and the page is live within a minute: cyberproxy keeps a shallow
sparse clone of this repo (`/home/cyber/trident-pink-site`) and a cron job as
`cyber` runs `host/sync.sh` every minute — fetch, reset, `rsync -rltp
--chmod=D755,F644` of `landing/` into `/var/www/html/trident.pink/`, no
`--delete`, never `-a`. The set mp3 is fetched once from the release. Delete
`/home/cyber/trident-pink-docroot` to pause the sync.

One-time install, in the Hetzner console as `cyber`. The noVNC console mangles
`:` `|` `@` `_`, so type ONE line at a time and press Enter after each:

```
wget raw.githubusercontent.com/cyberia-to/trident/master/landing/host/setup.sh
```
```
bash setup.sh
```

Expect `clone at <sha>`, the crontab line, a docroot listing, `player in page: 1`
and `SETUP OK`. The script deletes itself; re-running means wget again.

With ssh access the manual path still works: `nu scripts/deploy.nu`.

## Server

- nginx: `/etc/nginx/sites-enabled/trident.pink` (from `scripts/nginx-trident.pink.conf`)
- TLS: certbot
- analytics: lytics tracker, `/lytics/` proxied to 127.0.0.1:8091 on cyberproxy

## DNS (Namecheap)

| type | host | value |
|------|------|-------|
| A | @ | 167.235.28.94 |
| CNAME | www | trident.pink |
