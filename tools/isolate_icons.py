#!/usr/bin/env python3
"""Key lime-green sheets and save each blob as a 128px PNG."""
from pathlib import Path
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "icons"
SIZE = 128


def rgba(path):
    return np.array(Image.open(path).convert("RGBA"))


def not_green(a):
    r, g, b = a[:, :, 0].astype(np.int16), a[:, :, 1].astype(np.int16), a[:, :, 2].astype(np.int16)
    return ~((g > 130) & (g > r + 25) & (g > b + 25))


def components(mask):
    h, w = mask.shape
    seen = np.zeros_like(mask, dtype=np.uint8)
    out = []
    for y in range(h):
        for x in range(w):
            if not mask[y, x] or seen[y, x]:
                continue
            stack = [(y, x)]
            seen[y, x] = 1
            ys, xs = [y], [x]
            while stack:
                cy, cx = stack.pop()
                for dy, dx in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    ny, nx = cy + dy, cx + dx
                    if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = 1
                        stack.append((ny, nx))
                        ys.append(ny)
                        xs.append(nx)
            y0, y1, x0, x1 = min(ys), max(ys), min(xs), max(xs)
            area = len(ys)
            if area < (h * w) * 0.002:
                continue
            out.append((y0, x0, y1, x1, area))
    out.sort(key=lambda t: (t[0] // max(h // 8, 1), t[1]))
    return out


def save_item(a, box, name):
    y0, x0, y1, x1, _ = box
    pad = int(0.08 * max(y1 - y0, x1 - x0, 1))
    y0 = max(0, y0 - pad)
    x0 = max(0, x0 - pad)
    y1 = min(a.shape[0] - 1, y1 + pad)
    x1 = min(a.shape[1] - 1, x1 + pad)
    crop = a[y0 : y1 + 1, x0 : x1 + 1].copy()
    m = not_green(crop)
    crop[..., 3] = np.where(m, 255, 0).astype(np.uint8)
    im = Image.fromarray(crop)
    im.thumbnail((SIZE, SIZE), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    canvas.paste(im, ((SIZE - im.width) // 2, (SIZE - im.height) // 2))
    OUT.mkdir(parents=True, exist_ok=True)
    canvas.save(OUT / f"{name}.png")


def isolate(path, names):
    a = rgba(path)
    # work smaller for CC speed
    scale = 4 if a.shape[1] > 800 else 2
    small = a[::scale, ::scale]
    mask = not_green(small)
    boxes = components(mask)
    print(path, "blobs", len(boxes), "want", len(names))
    for i, box in enumerate(boxes):
        full = (
            box[0] * scale,
            box[1] * scale,
            min(a.shape[0] - 1, box[2] * scale + scale),
            min(a.shape[1] - 1, box[3] * scale + scale),
            box[4],
        )
        if i < len(names):
            save_item(a, full, names[i])
            print(" ", i, names[i], full[:4])
    return boxes


def main():
    # The source sheets are a local screenshot drop from an agent session, so
    # there is no portable default: this used to hardcode an absolute path into
    # one machine's home directory, which published the username and the local
    # layout in a public repo (audit F-5) and meant the tool could not run
    # anywhere else. Take the directory as an argument, or MOURAMA_ICON_SHEETS.
    import os
    import sys

    argv = sys.argv[1:]
    if not argv:
        argv = [os.environ["MOURAMA_ICON_SHEETS"]] if os.environ.get("MOURAMA_ICON_SHEETS") else []
    if not argv:
        sys.exit(
            "usage: isolate_icons.py <dir-with-1.jpg-2.jpg-3.jpg>\n"
            "   or: MOURAMA_ICON_SHEETS=<dir> isolate_icons.py\n"
            "The sheets are a local screenshot drop and are not in the repo."
        )
    base = Path(argv[0]).expanduser()
    if not base.is_dir():
        sys.exit(f"not a directory: {base}")
    # app mark
    a = rgba(base / "1.jpg")
    mask = not_green(a)
    ys, xs = np.where(mask)
    box = (ys.min(), xs.min(), ys.max(), xs.max(), int(mask.sum()))
    save_item(a, box, "app")
    print("app", box[:4])

    # units/resources sheet has extras; pick unique by index after listing
    isolate(
        base / "2.jpg",
        [
            "res-cobre",
            "res-estanho",
            "res-seara",
            "res-orvalho",
            "res-bronze",
            "unit-pastor",
            "unit-trasgo",
            "skip-dup-cobre",
            "skip-dup-seara",
            "skip-dup-bronze",
            "unit-falcao",
            "unit-javali",
            "unit-guerreiro",
            "unit-serpe",
        ],
    )
    isolate(
        base / "3.jpg",
        [
            "bldg-casa",
            "bldg-eira",
            "bldg-mina",
            "bldg-veio",
            "bldg-fonte",
            "bldg-curral",
            "bldg-anta",
            "bldg-celeiro",
            "tile-outeiro",
            "tile-mamoa",
            "tile-citania",
            "tile-veiled",
        ],
    )
    # muralha: same stone ring as curral until we have a dedicated wall
    src = OUT / "bldg-curral.png"
    if src.is_file():
        Image.open(src).save(OUT / "bldg-muralha.png")
    # drop skip_* files
    for p in OUT.glob("skip-*.png"):
        p.unlink()


if __name__ == "__main__":
    main()
