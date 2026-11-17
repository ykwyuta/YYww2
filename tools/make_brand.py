"""アプリアイコンと読み込み画面の絵を作る（Pillow が必要）。

python tools/make_brand.py
  -> assets/app_icon.ico, web/icon-192.png, web/icon-512.png, web/apple-touch-icon.png, web/key-art.jpg
"""
import math
import os
import random

from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def tank(d: ImageDraw.ImageDraw, cx: float, cy: float, s: float, body, dark, gun_angle=-0.08):
    """横から見た戦車のシルエット（s は車体長）"""
    h = s * 0.22
    # 履帯
    d.rounded_rectangle([cx - s * 0.5, cy, cx + s * 0.5, cy + h], radius=h / 2, fill=dark)
    for i in range(6):
        x = cx - s * 0.4 + i * s * 0.16
        d.ellipse([x - h * 0.33, cy + h * 0.2, x + h * 0.33, cy + h * 0.86], fill=body)
    # 車体（前面は傾斜）
    d.polygon([(cx - s * 0.48, cy), (cx + s * 0.38, cy), (cx + s * 0.52, cy - h * 0.55), (cx - s * 0.46, cy - h * 0.6)], fill=body)
    # 砲塔
    tw = s * 0.42
    ty = cy - h * 0.6
    d.polygon([(cx - tw * 0.55, ty), (cx + tw * 0.45, ty), (cx + tw * 0.35, ty - h * 0.75), (cx - tw * 0.5, ty - h * 0.8)], fill=body)
    # 砲身
    gx, gy = cx + tw * 0.4, ty - h * 0.4
    L = s * 0.62
    ex, ey = gx + math.cos(gun_angle) * L, gy + math.sin(gun_angle) * L
    d.line([(gx, gy), (ex, ey)], fill=body, width=max(2, int(h * 0.18)))
    d.ellipse([ex - h * 0.12, ey - h * 0.12, ex + h * 0.12, ey + h * 0.12], fill=body)


def icon(size: int) -> Image.Image:
    im = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    r = size * 0.18
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=r, fill=(46, 52, 34, 255))
    # 地平線と夕焼け
    for y in range(int(size * 0.62)):
        k = y / (size * 0.62)
        c = (int(200 - 120 * k), int(120 - 60 * k), int(50 - 10 * k), 255)
        d.line([(size * 0.06, y + size * 0.06), (size * 0.94, y + size * 0.06)], fill=c)
    d.ellipse([size * 0.56, size * 0.2, size * 0.86, size * 0.5], fill=(250, 200, 110, 255))
    d.rectangle([size * 0.06, size * 0.62, size * 0.94, size * 0.94], fill=(58, 62, 36, 255))
    tank(d, size * 0.46, size * 0.6, size * 0.62, (28, 30, 22, 255), (16, 16, 12, 255))
    # 角を丸く切る
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, size - 1, size - 1], radius=r, fill=255)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(im, (0, 0), mask)
    return out


def key_art(w=1600, h=900) -> Image.Image:
    rnd = random.Random(1943)
    im = Image.new("RGB", (w, h))
    d = ImageDraw.Draw(im)
    horizon = int(h * 0.62)
    for y in range(horizon):
        k = y / horizon
        d.line([(0, y), (w, y)], fill=(int(60 + 170 * k), int(50 + 90 * k), int(60 + 10 * k)))
    d.ellipse([w * 0.62, h * 0.30, w * 0.74, h * 0.30 + w * 0.12], fill=(255, 210, 130))
    # 煙
    smoke = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    sd = ImageDraw.Draw(smoke)
    for _ in range(40):
        x = rnd.uniform(0, w)
        y = rnd.uniform(h * 0.25, horizon)
        r = rnd.uniform(40, 140)
        sd.ellipse([x - r, y - r, x + r, y + r], fill=(30, 26, 24, 50))
    smoke = smoke.filter(ImageFilter.GaussianBlur(30))
    im.paste(smoke, (0, 0), smoke)
    d = ImageDraw.Draw(im)
    # 起伏のある地面
    pts = [(0, h)]
    for x in range(0, w + 20, 20):
        pts.append((x, horizon + math.sin(x * 0.004) * 18 + math.sin(x * 0.013) * 6))
    pts.append((w, h))
    d.polygon(pts, fill=(52, 50, 32))
    for i in range(5):
        y0 = horizon + 40 + i * 50
        d.line([(0, y0), (w, y0 + rnd.uniform(-20, 20))], fill=(60 + i * 3, 56 + i * 2, 34), width=24)
    for i, (x, y, s) in enumerate([(0.16, 0.70, 360), (0.42, 0.66, 220), (0.6, 0.64, 150), (0.78, 0.635, 100), (0.88, 0.63, 70)]):
        tank(d, w * x, h * y, s, (22, 22, 18), (12, 12, 10), gun_angle=-0.05 - 0.02 * i)
    # 縁を暗く
    vign = Image.new("L", (w, h), 0)
    ImageDraw.Draw(vign).ellipse([-w * 0.2, -h * 0.3, w * 1.2, h * 1.3], fill=255)
    vign = vign.filter(ImageFilter.GaussianBlur(120))
    dark = Image.new("RGB", (w, h), (8, 8, 6))
    return Image.composite(im, dark, vign)


def main():
    big = icon(512)
    os.makedirs(os.path.join(ROOT, "assets"), exist_ok=True)
    big.save(os.path.join(ROOT, "assets", "app_icon.ico"), sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    big.resize((192, 192), Image.LANCZOS).save(os.path.join(ROOT, "web", "icon-192.png"))
    big.save(os.path.join(ROOT, "web", "icon-512.png"))
    apple = Image.new("RGB", (180, 180), (46, 52, 34))
    a = icon(180)
    apple.paste(a, (0, 0), a)
    apple.save(os.path.join(ROOT, "web", "apple-touch-icon.png"))
    key_art().save(os.path.join(ROOT, "web", "key-art.jpg"), quality=86)


if __name__ == "__main__":
    main()
