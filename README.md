# Mourama

Iberian persistent hillfort. Five seats. The office box is the world.

You keep a **citânia**. Resources are **cobre**, **estanho**, **seara**, and
**orvalho**. Cobre + estanho make bronze. Orvalho pays for **encanto**
(veil) and a **serpe** bind (conquest). Wild hills are **mamoas**. Travel
is the between.

This is a faeOS engine (`ElegantVW/mourama`), not a faeOS in-tree app.

## Play

```
cd ~/mourama && ./build.sh install
mourama invite          # one-time code, max 5 seats
mourama play            # native window (Linux)
```

Server stays on `:4747`. The window is `mourama-play`, not a browser.

Windows zip (when mingw is installed): `./build.sh windows` → `~/Downloads/mourama-windows.zip`.
APK (when JDK+SDK are installed): `./build.sh apk` → `~/Downloads/mourama.apk`.

## Keep the box awake

```
sudo ./build.sh keep-awake
```

Lock with Seal (`Mod+Ctrl+l`). Suspend (`Mod+Ctrl+Shift+l`) stops the world.

The user unit is enabled. For it to survive logout:

```
sudo loginctl enable-linger evenweaker
```


## Reach

Phones at home join the same Tailscale tailnet as this box. See
[docs/REACH.md](docs/REACH.md). World design: [docs/DESIGN.md](docs/DESIGN.md).
