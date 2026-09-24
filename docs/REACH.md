# Reach — office world, home phones

The world runs only on VANGUARDA. Phones are windows.

## Office LAN

Bind `0.0.0.0:4747`. A phone on `192.168.8.0/24` opens
`http://192.168.8.186:4747`.

## Home (primary): Tailscale

1. On this box: `sudo pacman -S tailscale && sudo systemctl enable --now tailscaled && sudo tailscale up`
2. On each phone: Tailscale app, same tailnet (max 5 people).
3. APK / browser: `http://vanguarda:4747` (MagicDNS) or the tailnet IPv4 printed by `tailscale ip -4`.

Game HTTP stays on the tailnet. Tailscale encrypts the path. Do not port-forward 4747 to the internet.

## Bulwark

When Aegis is raised, apply `policy/mourama.aegis` (port 4747 from
`192.168.8.0/24` and `100.64.0.0/10` only).

## Fallback

This box currently egresses as a public IPv4. A Huawei forward of 4747
→ `192.168.8.186` can work, then needs TLS, and moves when DHCP moves.
Do not store router passwords or Wi-Fi in this repo.
