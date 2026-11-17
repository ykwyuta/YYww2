"""Build the Windows and web icons and title art from assets/branding (Pillow)."""

from pathlib import Path

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[1]
BRAND = ROOT / "assets" / "branding"
WEB = ROOT / "web"
TITLE_SIZE = (1280, 720)
ICO_SIZES = [(size, size) for size in (16, 24, 32, 48, 64, 128, 256)]


def main():
    with Image.open(BRAND / "icon.png") as source:
        icon = ImageOps.fit(source.convert("RGB"), (512, 512), Image.Resampling.LANCZOS)
    icon.save(BRAND / "steel-front.ico", sizes=ICO_SIZES)
    for size, name in (
        (192, "steel-front-192.png"),
        (512, "steel-front-512.png"),
        (180, "steel-front-apple.png"),
    ):
        icon.resize((size, size), Image.Resampling.LANCZOS).save(WEB / name)

    with Image.open(BRAND / "title.png") as source:
        art = ImageOps.fit(source.convert("RGB"), TITLE_SIZE, Image.Resampling.LANCZOS)
    (BRAND / "title.rgb").write_bytes(art.tobytes())
    art.save(WEB / "steel-front.jpg", quality=88, optimize=True)


if __name__ == "__main__":
    main()
