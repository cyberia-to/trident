# Deploy the trident.pink landing → cyberproxy:/var/www/html/trident.pink/
#
#   nu scripts/deploy.nu
#
# the set (media/trident.pink.set.mp3) is not in git: it is a release asset.
# put it on the server once, the deploy never deletes it:
#   curl -L -o /tmp/set.mp3 https://github.com/cyberia-to/trident/releases/download/set-2026-09-17/trident.pink.set.mp3
#   scp /tmp/set.mp3 cyberproxy:/var/www/html/trident.pink/media/trident.pink.set.mp3

def main [] {
  let root = (
    if ($"($env.PWD)/index.html" | path exists) { $env.PWD }
    else { error make {msg: "run from trident/landing"} }
  )
  print "→ rsync → cyberproxy:/var/www/html/trident.pink/"
  ^ssh cyberproxy "mkdir -p /var/www/html/trident.pink"
  ^rsync -az --delete --exclude ".git" --exclude "scripts" --exclude "media/*.mp3" $"($root)/" "cyberproxy:/var/www/html/trident.pink/"
  print "✓ deployed → https://trident.pink/"
}
