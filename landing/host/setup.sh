#!/bin/bash
# one-time install on cyberproxy as user cyber: clone, docroot, cron. idempotent.
# the noVNC console mangles : | @ _ and shifted punctuation, so both lines avoid them:
#   wget raw.githubusercontent.com/cyberia-to/trident/master/landing/host/setup.sh
#   bash setup.sh
umask 022
set -u
REPO=https://github.com/cyberia-to/trident.git
CLONE=/home/cyber/trident-pink-site
SYNC=/home/cyber/trident-pink-sync.sh
DOCROOT=/var/www/html/trident.pink
DOCROOT_FILE=/home/cyber/trident-pink-docroot
WWW=/var/www/html

if [ ! -d "$DOCROOT" ]; then
  echo "no $DOCROOT on this host; the vhost for trident.pink must point there"
  exit 1
fi
if [ ! -w "$DOCROOT" ]; then
  echo "$DOCROOT is not writable by $(whoami). run: sudo chown -R cyber $DOCROOT   then re-run"
  exit 1
fi

# the repo is a compiler, the site is one directory of it: shallow and sparse,
# with a plain shallow clone as the fallback for an old git.
if [ ! -d "$CLONE/.git" ]; then
  git clone --depth 1 --filter=blob:none --sparse -b master "$REPO" "$CLONE" \
    && git -C "$CLONE" sparse-checkout set landing \
    || { rm -rf "$CLONE"; git clone --depth 1 -b master "$REPO" "$CLONE" || exit 1; }
fi
chmod 755 "$CLONE"
# always bring the clone to the tip before copying anything out of it
git -C "$CLONE" fetch --depth 1 origin master --quiet && git -C "$CLONE" reset --hard FETCH_HEAD --quiet || exit 1
echo "clone at $(git -C "$CLONE" rev-parse --short HEAD)"
[ -f "$CLONE/landing/index.html" ] || { echo "no landing/index.html in the clone"; exit 1; }

echo "$DOCROOT" > "$DOCROOT_FILE"
echo "docroot $DOCROOT"

cp "$CLONE/landing/host/sync.sh" "$SYNC"
chmod 755 "$SYNC"

TMP=$(mktemp)
crontab -l 2>/dev/null | grep -v trident-pink-sync > "$TMP"
echo "* * * * * $SYNC >/dev/null 2>&1" >> "$TMP"
crontab "$TMP"
rm -f "$TMP"
rm -f /home/cyber/s.sh /home/cyber/setup.sh

"$SYNC"

echo "--- crontab ---"
crontab -l
echo "--- docroot ---"
ls -la "$DOCROOT" "$DOCROOT/media"
echo "player in page: $(grep -c set-audio "$DOCROOT/index.html")"
if crontab -l 2>/dev/null | grep -q trident-pink-sync \
   && [ "$(stat -c %a "$WWW")" = "755" ] \
   && [ "$(stat -c %a "$DOCROOT/index.html")" = "644" ] \
   && [ -s "$DOCROOT/media/trident.pink.set.mp3" ] \
   && grep -q rltp "$SYNC"; then
  echo "SETUP OK"
else
  echo "SETUP FAILED"
  exit 1
fi
