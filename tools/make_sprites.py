"""アセット画像をゲーム向け RGBA 透過スプライト（512x512）とスプライトシートに変換する。

python tools/make_sprites.py
  -> assets/sprites/*.png, assets/sprites/atlas.png, assets/sprites/atlas.json
"""
import collections
import json
import math
import os
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UNITS_DIR = os.path.join(ROOT, "assets", "units")
SPRITES_DIR = os.path.join(ROOT, "assets", "sprites")
SPRITE_SIZE = 512
CONTENT_MAX = 472


def color_dist(a: tuple[int, int, int], b: tuple[int, int, int]) -> float:
    return math.sqrt((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2)


def make_transparent_sprite(im: Image.Image) -> Image.Image:
    im = im.convert("RGBA")
    if im.getchannel("A").getextrema()[0] < 255:
        return center_sprite(im)

    w, h = im.size
    px = im.load()

    # 四隅からスタジオ背景色を推定
    corners = [px[0, 0][:3], px[w - 1, 0][:3], px[0, h - 1][:3], px[w - 1, h - 1][:3]]
    bg = tuple(sum(c[i] for c in corners) // 4 for i in range(3))

    # 1. 外周から背景色に近いピクセルを BFS で抜く（内部の暗い履帯や線画は絶対に侵食しない）
    visited = bytearray(w * h)
    bg_mask = bytearray(w * h)
    q = collections.deque()
    for x in range(w):
        q.append((x, 0))
        q.append((x, h - 1))
    for y in range(1, h - 1):
        q.append((0, y))
        q.append((w - 1, y))

    hard_thresh = 16.0
    soft_thresh = 30.0
    while q:
        x, y = q.popleft()
        idx = y * w + x
        if visited[idx]:
            continue
        visited[idx] = 1
        if color_dist(px[x, y][:3], bg) <= hard_thresh:
            bg_mask[idx] = 1
            if x > 0 and not visited[idx - 1]:
                q.append((x - 1, y))
            if x + 1 < w and not visited[idx + 1]:
                q.append((x + 1, y))
            if y > 0 and not visited[idx - w]:
                q.append((x, y - 1))
            if y + 1 < h and not visited[idx + w]:
                q.append((x, y + 1))

    # 2. 背景除去と境界アンチエイリアス
    out = im.copy()
    opx = out.load()
    for y in range(h):
        for x in range(w):
            idx = y * w + x
            if bg_mask[idx]:
                opx[x, y] = (0, 0, 0, 0)
            else:
                adj_bg = any(
                    0 <= x + dx < w and 0 <= y + dy < h and bg_mask[(y + dy) * w + (x + dx)]
                    for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1))
                )
                if adj_bg:
                    r, g, b, _ = opx[x, y]
                    d = color_dist((r, g, b), bg)
                    if d < soft_thresh:
                        a = int(255 * max(0.0, min(1.0, (d - hard_thresh) / (soft_thresh - hard_thresh))))
                        opx[x, y] = (r, g, b, a)

    return center_sprite(out, preserve_alpha=False)


def center_sprite(im: Image.Image, *, preserve_alpha: bool = True) -> Image.Image:
    # 非透明領域でタイトクロップし 512x512 中央に揃える
    bbox = im.getbbox()
    if not bbox:
        return im.resize((SPRITE_SIZE, SPRITE_SIZE), Image.LANCZOS)
    cropped = im.crop(bbox)
    cw, ch = cropped.size
    scale = min(CONTENT_MAX / cw, CONTENT_MAX / ch)
    nw, nh = max(1, int(round(cw * scale))), max(1, int(round(ch * scale)))
    resized = cropped.resize((nw, nh), Image.LANCZOS)
    canvas = Image.new("RGBA", (SPRITE_SIZE, SPRITE_SIZE), (0, 0, 0, 0))
    position = ((SPRITE_SIZE - nw) // 2, (SPRITE_SIZE - nh) // 2)
    canvas.paste(resized, position, None if preserve_alpha else resized)
    return canvas


def main():
    os.makedirs(SPRITES_DIR, exist_ok=True)
    files = sorted(f for f in os.listdir(UNITS_DIR) if f.endswith(".png"))
    cols = 4
    rows = max(1, math.ceil(len(files) / cols))
    atlas = Image.new("RGBA", (cols * SPRITE_SIZE, rows * SPRITE_SIZE), (0, 0, 0, 0))
    meta = {"sprite_size": SPRITE_SIZE, "columns": cols, "rows": rows, "sprites": {}}

    for idx, name in enumerate(files):
        src = os.path.join(UNITS_DIR, name)
        dst = os.path.join(SPRITES_DIR, name)
        with Image.open(src) as im:
            sprite = make_transparent_sprite(im)
            sprite.save(dst, "PNG")
            gx, gy = (idx % cols) * SPRITE_SIZE, (idx // cols) * SPRITE_SIZE
            atlas.paste(sprite, (gx, gy))
            key = os.path.splitext(name)[0]
            meta["sprites"][key] = {
                "file": name,
                "x": gx,
                "y": gy,
                "w": SPRITE_SIZE,
                "h": SPRITE_SIZE,
            }
            print(f"sprite: {name} -> ({gx}, {gy})")

    atlas.save(os.path.join(SPRITES_DIR, "atlas.png"), "PNG")
    with open(os.path.join(SPRITES_DIR, "atlas.json"), "w", encoding="utf-8") as f:
        json.dump(meta, f, ensure_ascii=False, indent=2)
    print(f"atlas: {cols * SPRITE_SIZE}x{rows * SPRITE_SIZE} ({len(files)} sprites)")


if __name__ == "__main__":
    main()
