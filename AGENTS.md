# AGENTS.md — Mourama

Canonical repo: `ElegantVW/mourama` → `~/mourama`.
faeOS keeps only a thin launcher after `./build.sh install`. Do **not**
vendor this tree back into `faeos/`.

## North star (locked)

Iberian persistent hillfort. The office box is the world. Phones are
windows. Five seats. Pink faeOS chrome on Atlantic granite.

## Product laws

| Law | Rule |
|---|---|
| Voice | faeOS error-voice: `mourama: <one sentence>` then `next:`. All-ages. |
| Engine | One Rust binary. Authoritative tick. SQLite on this disk. |
| Window | Native `mourama-play` (egui). Icons are local PNGs, not Imagine. |
| Cap | Five accounts. The sixth is refused in code and in lore. |
| Secrets | Invites, session tokens, world.sqlite stay on disk `0600`. Never git. |
| Names | Portuguese folk nouns (citânia, mamoa, orvalho, serpe). English UI. |
| Taken | Do not name a building Hearth or a unit Pixie. |
| Reach | Tailscale + office LAN. Do not open `:4747` to the whole internet. |
| Sleep | Citânias keep breathing. Inhibit sleep. Lock (Seal) is fine. |

## Install

```
cd ~/mourama && ./build.sh install
```

Engine → `~/.local/lib/faeos/mourama`  
Web → `~/.local/lib/faeos/mourama-web`  
Launcher → `~/bin/mourama`

## Do not

- Vendor this tree into faeOS.
- Commit world saves, invites, keystores, APKs, or router secrets.
- Punch `0.0.0.0/0` on port 4747.
- Walk through Kur / Murmur.
- Imagine a new V.
