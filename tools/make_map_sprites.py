"""Pack the generated terrain props for the Rust map renderer.

Run with Pillow installed: python tools/make_map_sprites.py
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets" / "terrain"
OUTPUT = ROOT / "assets" / "sprites"
CELL = 96
COLS = 4
# Keep in the order used by Prop in src/map_art.rs.
PROPS = (
    "pines", "deciduous", "hedge", "reeds",
    "farmhouse", "barn", "city_ruin", "rubble",
    "alpine_crag", "desert_rocks", "haystacks", "bridge_deck",
    "hedge_vertical", "alpine_ridge",
)


def load_prop(name: str) -> Image.Image:
    with Image.open(SOURCE / f"{name}.png") as source:
        sprite = source.convert("RGBA")
    alpha = sprite.getchannel("A")
    bounds = alpha.point(lambda value: 255 if value >= 32 else 0).getbbox()
    if bounds is None or alpha.getextrema()[0] == 255:
        raise ValueError(f"{name}.png must contain an object on a transparent background")
    left, top, right, bottom = bounds
    left = max(0, left - 2)
    top = max(0, top - 2)
    right = min(sprite.width, right + 2)
    bottom = min(sprite.height, bottom + 2)
    sprite = sprite.crop((left, top, right, bottom))
    if name == "bridge_deck":
        return sprite.resize((CELL, CELL), Image.Resampling.LANCZOS)
    sprite.thumbnail((CELL - 4, CELL - 4), Image.Resampling.LANCZOS)
    cell = Image.new("RGBA", (CELL, CELL))
    cell.paste(sprite, ((CELL - sprite.width) // 2, (CELL - sprite.height) // 2))
    return cell


def main():
    rows = (len(PROPS) + COLS - 1) // COLS
    atlas = Image.new("RGBA", (COLS * CELL, rows * CELL))
    for index, name in enumerate(PROPS):
        atlas.paste(load_prop(name), (index % COLS * CELL, index // COLS * CELL))
    OUTPUT.mkdir(parents=True, exist_ok=True)
    atlas.save(OUTPUT / "map_atlas.png")
    (OUTPUT / "map_atlas.rgba").write_bytes(atlas.tobytes())


if __name__ == "__main__":
    main()
