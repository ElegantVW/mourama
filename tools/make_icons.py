#!/usr/bin/env python3
"""Local geometric icons. No Imagine. Pink granite, 128px, transparent."""
from pathlib import Path
from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parents[1] / "assets" / "icons"
SIZE = 128
PINK = (232, 121, 160, 255)
PINK_D = (196, 77, 122, 255)
BLUSH = (240, 180, 200, 255)
SILVER = (192, 192, 200, 255)
GOLD = (255, 176, 32, 255)
COPPER = (201, 112, 64, 255)
TIN = (176, 196, 204, 255)
OK = (61, 214, 140, 255)
BG = (26, 10, 18, 0)


def canvas():
    return Image.new("RGBA", (SIZE, SIZE), BG)


def save(im, name):
    OUT.mkdir(parents=True, exist_ok=True)
    im.save(OUT / f"{name}.png")


def disc(draw, color, r=50, width=0):
    c = SIZE // 2
    box = [c - r, c - r, c + r, c + r]
    if width:
        draw.ellipse(box, outline=color, width=width)
    else:
        draw.ellipse(box, fill=color)


def hexagon(draw, color, r=52):
    c = SIZE // 2
    pts = []
    from math import cos, sin, pi
    for i in range(6):
        a = pi / 6 + i * pi / 3
        pts.append((c + r * cos(a), c + r * sin(a)))
    draw.polygon(pts, fill=color)


def make_all():
    # resources
    im = canvas(); d = ImageDraw.Draw(im)
    d.rounded_rectangle([34, 40, 94, 96], 8, fill=COPPER)
    d.polygon([(34, 48), (64, 28), (94, 48)], fill=PINK_D)
    save(im, "res-cobre")

    im = canvas(); d = ImageDraw.Draw(im)
    d.rounded_rectangle([36, 44, 92, 92], 6, fill=TIN)
    d.rounded_rectangle([44, 36, 84, 52], 4, fill=SILVER)
    save(im, "res-estanho")

    im = canvas(); d = ImageDraw.Draw(im)
    d.polygon([(64, 24), (78, 88), (50, 88)], fill=GOLD)
    d.ellipse([40, 28, 64, 52], fill=PINK)
    d.ellipse([64, 32, 90, 58], fill=PINK_D)
    save(im, "res-seara")

    im = canvas(); d = ImageDraw.Draw(im)
    d.ellipse([44, 28, 84, 68], fill=BLUSH)
    d.polygon([(44, 52), (84, 52), (64, 104)], fill=PINK)
    save(im, "res-orvalho")

    im = canvas(); d = ImageDraw.Draw(im)
    d.rounded_rectangle([28, 48, 72, 88], 6, fill=COPPER)
    d.rounded_rectangle([56, 40, 100, 80], 6, fill=TIN)
    d.ellipse([50, 44, 78, 72], fill=GOLD)
    save(im, "res-bronze")

    # units
    im = canvas(); d = ImageDraw.Draw(im)
    disc(d, PINK_D, 54)
    d.ellipse([48, 28, 80, 60], fill=BLUSH)
    d.polygon([(40, 100), (64, 58), (88, 100)], fill=PINK)
    d.line([(78, 40), (104, 24)], fill=GOLD, width=6)
    save(im, "unit-pastor")

    im = canvas(); d = ImageDraw.Draw(im)
    disc(d, PINK_D, 54)
    d.ellipse([40, 40, 72, 88], fill=PINK)
    d.polygon([(72, 48), (108, 36), (88, 72)], fill=BLUSH)
    save(im, "unit-trasgo")

    im = canvas(); d = ImageDraw.Draw(im)
    disc(d, PINK_D, 54)
    d.polygon([(28, 72), (64, 28), (100, 72), (80, 72), (64, 52), (48, 72)], fill=SILVER)
    d.ellipse([56, 44, 72, 60], fill=GOLD)
    save(im, "unit-falcao")

    im = canvas(); d = ImageDraw.Draw(im)
    disc(d, PINK_D, 54)
    d.ellipse([36, 48, 96, 100], fill=(90, 50, 40, 255))
    d.ellipse([52, 32, 84, 64], fill=(90, 50, 40, 255))
    d.ellipse([58, 40, 70, 52], fill=GOLD)
    save(im, "unit-javali")

    im = canvas(); d = ImageDraw.Draw(im)
    disc(d, PINK_D, 54)
    d.polygon([(64, 24), (96, 100), (32, 100)], fill=GOLD)
    d.ellipse([50, 36, 78, 64], fill=BLUSH)
    save(im, "unit-guerreiro")

    im = canvas(); d = ImageDraw.Draw(im)
    disc(d, PINK_D, 54)
    d.arc([24, 36, 104, 100], 200, 20, fill=OK, width=10)
    d.ellipse([76, 40, 100, 64], fill=OK)
    d.ellipse([84, 46, 92, 54], fill=(20, 20, 20, 255))
    save(im, "unit-serpe")

    # buildings
    names = {
        "bldg-casa": lambda d: (hexagon(d, PINK_D, 56), d.ellipse([48, 48, 80, 80], fill=PINK)),
        "bldg-eira": lambda d: d.ellipse([24, 24, 104, 104], outline=GOLD, width=10),
        "bldg-mina": lambda d: d.polygon([(24, 96), (64, 28), (104, 96)], fill=COPPER),
        "bldg-veio": lambda d: (d.line([(32, 96), (52, 48), (76, 72), (96, 32)], fill=TIN, width=10),),
        "bldg-fonte": lambda d: (d.ellipse([36, 36, 92, 92], outline=PINK, width=8), d.ellipse([56, 56, 72, 72], fill=BLUSH)),
        "bldg-curral": lambda d: d.rounded_rectangle([32, 32, 96, 96], 8, outline=SILVER, width=8),
        "bldg-muralha": lambda d: (d.rectangle([28, 48, 100, 100], fill=SILVER), d.rectangle([40, 28, 52, 48], fill=SILVER), d.rectangle([76, 28, 88, 48], fill=SILVER)),
        "bldg-anta": lambda d: (d.rectangle([36, 64, 92, 96], fill=SILVER), d.polygon([(28, 64), (64, 28), (100, 64)], fill=PINK_D)),
        "bldg-celeiro": lambda d: (d.rectangle([36, 56, 92, 100], fill=GOLD), d.polygon([(28, 56), (64, 24), (100, 56)], fill=PINK_D)),
    }
    for name, fn in names.items():
        im = canvas(); d = ImageDraw.Draw(im)
        fn(d)
        save(im, name)

    # tiles
    im = canvas(); d = ImageDraw.Draw(im)
    hexagon(d, (58, 36, 48, 255), 56)
    save(im, "tile-outeiro")

    im = canvas(); d = ImageDraw.Draw(im)
    hexagon(d, SILVER, 56)
    d.ellipse([48, 48, 80, 80], fill=PINK_D)
    save(im, "tile-mamoa")

    im = canvas(); d = ImageDraw.Draw(im)
    hexagon(d, PINK, 56)
    d.ellipse([52, 52, 76, 76], fill=PINK_D)
    save(im, "tile-citania")

    im = canvas(); d = ImageDraw.Draw(im)
    hexagon(d, PINK_D, 56)
    d.line([(40, 40), (88, 88)], fill=GOLD, width=6)
    d.line([(88, 40), (40, 88)], fill=GOLD, width=6)
    save(im, "tile-veiled")

    # app mark
    im = canvas(); d = ImageDraw.Draw(im)
    hexagon(d, (26, 10, 18, 255), 60)
    hexagon(d, PINK, 44)
    d.ellipse([52, 52, 76, 76], fill=BLUSH)
    save(im, "app")

    print(f"wrote {len(list(OUT.glob('*.png')))} icons → {OUT}")


if __name__ == "__main__":
    make_all()
