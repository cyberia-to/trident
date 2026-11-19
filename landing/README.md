# trident.pink — the landing

One screen of the doors' family (nox.band, joy.technology, cy.army and the others): the pitch, three facts, the
links and the ladder of soft3, in pink, with Trisha beside the copy. `index.html`, `fonts/`, `favicon.svg` and
`robots.txt` are built by the apparatus (`apparatus/sites/doors/build.py`, the `trident.pink` door) and copied
here; edit the door there, not this file. The media stay here.

The proof is stated as measured on the 0.4 line — 16 to 64 KB, 0.05 to 1.1 s on a laptop, verified in under a
third of a second (`soft3/audit/proof-latency-2026-10.md`). The page before 2026-11-19 led with "3 ms · 2.2 KB",
which was the public execution certificate of 0.3.0 and no proof size.

The page no longer plays the set; its files remain release assets and the sync still keeps them in the docroot.

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
