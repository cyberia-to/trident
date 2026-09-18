#!/bin/bash
# sync the trident.pink landing from the public repo into the nginx docroot.
# runs from cron on cyberproxy as user cyber, once a minute.
umask 022
CLONE=/home/cyber/trident-pink-site
DOCROOT_FILE=/home/cyber/trident-pink-docroot
SET=https://github.com/cyberia-to/trident/releases/download/set-2026-09-17/trident.pink.set.mp3

# one run at a time: the first run downloads 48 MB and may outlive its minute
exec 9>/tmp/trident-pink-sync.lock
flock -n 9 || exit 0

[ -f "$DOCROOT_FILE" ] || exit 1          # delete that file to pause the sync
DOCROOT=$(cat "$DOCROOT_FILE")
[ -d "$DOCROOT" ] || exit 1
cd "$CLONE" || exit 1

# a network blip is not an error worth mailing about; the next minute retries
if git fetch --depth 1 origin master --quiet; then
  git reset --hard FETCH_HEAD --quiet || exit 1
fi
[ -f "$CLONE/landing/index.html" ] || exit 1

# -rltp, not -a: never carry the clone's modes or owner onto the docroot.
# --chmod fixes what nginx needs. no --delete: the docroot holds the set mp3.
rsync -rltp --chmod=D755,F644 --exclude=host --exclude=scripts --exclude=README.md "$CLONE/landing/" "$DOCROOT/"

# the set is a release asset, not a tracked file: fetch it once, heal it if it vanishes
MP3="$DOCROOT/media/trident.pink.set.mp3"
if [ ! -s "$MP3" ]; then
  curl -sfL -o "$MP3.part" "$SET" && mv "$MP3.part" "$MP3" && chmod 644 "$MP3"
fi

# keep the cron copy of this script current with the repo
SELF=/home/cyber/trident-pink-sync.sh
if ! cmp -s "$CLONE/landing/host/sync.sh" "$SELF"; then
  cp "$CLONE/landing/host/sync.sh" "$SELF" && chmod 755 "$SELF"
fi
