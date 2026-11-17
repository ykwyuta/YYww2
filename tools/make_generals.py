"""Pack the generated general portraits as 128px BGRA cards (requires Pillow)."""

from pathlib import Path

from PIL import Image, ImageOps

SOURCE = Path(__file__).resolve().parents[1] / "assets" / "generals"
SIZE = 128
# Keep this order in sync with src/generals.rs.
PORTRAITS = (
    "rin_asagiri", "yui_mizuno", "shiori_fujimori",
    "akane_kujo", "rei_tachibana", "mio_kisaragi",
    "leon_vogel", "mikhail_ivanov", "julian_arden",
    "kai_kuroda", "daisuke_shiba", "naoto_kagami",
    "sena_kirihara", "lucas_wren",
    "isuzu_takamine", "emil_falk",
)


def main():
    pixels = bytearray()
    for name in PORTRAITS:
        with Image.open(SOURCE / f"{name}.png") as source:
            portrait = ImageOps.fit(source.convert("RGB"), (SIZE, SIZE), Image.Resampling.LANCZOS)
        pixels.extend(portrait.convert("RGBA").tobytes("raw", "BGRA"))
    (SOURCE / "portraits.bgra").write_bytes(pixels)


if __name__ == "__main__":
    main()
