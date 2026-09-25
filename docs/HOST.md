# Host kit — live door

Public IPv4 of this box is checked live with `curl -4 https://ifconfig.me` (do not freeze an old one in git).

## Squarespace (you)

1. https://www.squarespace.com — sign in.
2. Open the site that owns the domain.
3. Settings → Domains. Write the name down and send it (example: `example.com`).
4. DNS → add **A** record:
   - Host: `play`
   - Value: the IPv4 from `curl -4 https://ifconfig.me` on VANGUARDA
   - TTL: default
5. Save.

We will use `https://play.<your-domain>` in the client.

## Huawei (`http://192.168.8.1`)

1. Log in from the office LAN. Do not paste the password anywhere.
2. Port forwarding / Virtual server / NAT.
3. Two TCP rules to `192.168.8.186`: **80→80**, **443→443**.
4. WAN IP on the status page must equal the public IPv4. If WAN is private, HTTPS cannot land.

## This box (needs sudo)

```
sudo loginctl enable-linger evenweaker
sudo ~/mourama/scripts/keep-awake.sh
sudo pacman -S caddy jdk17-openjdk gradle android-tools mingw-w64-gcc
```

Then Caddy reverse-proxies `https://play.<domain>` → `127.0.0.1:4747`.
WAN only forwards 80/443. Game port 4747 stays off the public list.

## Invite a friend

```
mourama invite
```

Send: the APK or Windows zip + the invite word + `https://play.<domain>`.
Four seats remain after R0d45.
