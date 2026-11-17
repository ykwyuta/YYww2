"""Pack the 5×2 expression sheets as 96px BGRA portraits (requires Pillow)."""

from PIL import Image, ImageOps

from make_generals import PORTRAITS, SOURCE

SHEETS = SOURCE / "expressions"
SIZE = 96
COLUMNS = 5
ROWS = 2


def main():
    pixels = bytearray()
    for name in PORTRAITS:
        with Image.open(SHEETS / f"{name}.png") as source:
            sheet = source.convert("RGB")
        w, h = sheet.size
        for row in range(ROWS):
            for col in range(COLUMNS):
                cell = sheet.crop((
                    col * w // COLUMNS, row * h // ROWS,
                    (col + 1) * w // COLUMNS, (row + 1) * h // ROWS,
                ))
                portrait = ImageOps.fit(cell, (SIZE, SIZE), Image.Resampling.LANCZOS, centering=(0.5, 0.25))
                pixels.extend(portrait.convert("RGBA").tobytes("raw", "BGRA"))
    (SHEETS / "expressions.bgra").write_bytes(pixels)


if __name__ == "__main__":
    main()
