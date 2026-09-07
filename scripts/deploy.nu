# Deploy the trident.pink landing → cyberproxy:/var/www/html/trident.pink/
#
#   nu scripts/deploy.nu

def main [] {
  let root = (
    if ($"($env.PWD)/index.html" | path exists) { $env.PWD }
    else { error make {msg: "run from trident-pink root"} }
  )
  print "→ rsync → cyberproxy:/var/www/html/trident.pink/"
  ^ssh cyberproxy "mkdir -p /var/www/html/trident.pink"
  ^rsync -az --delete --exclude ".git" --exclude "scripts" $"($root)/" "cyberproxy:/var/www/html/trident.pink/"
  print "✓ deployed → https://trident.pink/"
}
