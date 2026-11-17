"""YYww2 真上視点スプライトアトラス生成スクリプト（42車種 + 5種マップ要素）。

python tools/make_topdown_sprites.py
  -> assets/sprites/topdown_atlas.png (確認用PNG)
  -> assets/sprites/topdown_atlas.rle (Rust組み込み用 RLE圧縮RGBAバイナリ)

各セル 96x96px（4倍スーパーサンプリング 384x384 から縮小）、8列グリッド。
向きは右向き（angle = 0 rad = +X 方向が車両正面）。
"""
import math
import os
import random
import struct
from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_PNG = os.path.join(ROOT, "assets", "sprites", "topdown_atlas.png")
OUT_RLE = os.path.join(ROOT, "assets", "sprites", "topdown_atlas.rle")

CELL = 96
SS = 4
S = CELL * SS  # 384x384 作業キャンバス
COLS = 8

# 国別カラー（ベース装甲色, 迷彩1, 迷彩2, 暗部, ハイライト）
NATION_PAL = {
    "de_grey": ((92, 98, 102), (72, 78, 82), (110, 116, 120), (42, 46, 48), (138, 144, 148)),
    "de_gelb": ((184, 162, 104), (96, 114, 66), (124, 72, 52), (78, 68, 44), (218, 198, 138)),
    "ru": ((86, 106, 60), (66, 84, 46), (104, 122, 74), (40, 50, 28), (128, 148, 94)),
    "us": ((96, 104, 68), (78, 86, 54), (112, 118, 80), (46, 50, 32), (136, 144, 102)),
    "uk": ((118, 114, 82), (88, 94, 68), (138, 132, 98), (54, 52, 38), (158, 152, 116)),
    "jp": ((132, 118, 76), (78, 94, 54), (106, 82, 52), (56, 50, 32), (164, 148, 102)),
    "it": ((188, 166, 114), (138, 116, 74), (106, 98, 68), (82, 72, 48), (218, 198, 146)),
}


def shade(c, f):
    return (
        max(0, min(255, int(c[0] * f))),
        max(0, min(255, int(c[1] * f))),
        max(0, min(255, int(c[2] * f))),
        c[3] if len(c) > 3 else 255,
    )


def outline_poly(d: ImageDraw.ImageDraw, pts, fill, edge=(22, 22, 18, 255), w=4):
    d.polygon(pts, fill=fill)
    d.line(pts + [pts[0]], fill=edge, width=w, joint="curve")


def box_pts(cx, cy, hl, hw):
    return [(cx - hl, cy - hw), (cx + hl, cy - hw), (cx + hl, cy + hw), (cx - hl, cy + hw)]


def draw_rect(d, cx, cy, hl, hw, fill, edge=(22, 22, 18, 255), w=4):
    outline_poly(d, box_pts(cx, cy, hl, hw), fill, edge, w)


def draw_tracks(d, cx, cy, hl, hw, tw, dark=(36, 36, 32, 255), tread=(58, 56, 50, 255)):
    for sgn in (-1, 1):
        ty = cy + sgn * (hw - tw * 0.5)
        draw_rect(d, cx, ty, hl, tw * 0.52, dark, (18, 18, 16, 255), 4)
        step = max(10, int(hl * 0.14))
        x = cx - hl + step // 2
        while x < cx + hl - 4:
            d.line([(x, ty - tw * 0.42), (x, ty + tw * 0.42)], fill=tread, width=3)
            x += step


def draw_camo_blobs(d, cx, cy, hl, hw, pal, seed):
    rnd = random.Random(seed)
    for idx in range(5):
        col = pal[1] if idx % 2 == 0 else pal[2]
        bx = cx + rnd.uniform(-hl * 0.65, hl * 0.65)
        by = cy + rnd.uniform(-hw * 0.65, hw * 0.65)
        rx = rnd.uniform(hl * 0.14, hl * 0.28)
        ry = rnd.uniform(hw * 0.16, hw * 0.34)
        d.ellipse([bx - rx, by - ry, bx + rx, by + ry], fill=col + (155,))


def draw_engine_deck(d, cx, cy, hl, hw, pal, exhausts=2):
    ex = cx - hl * 0.58
    eh = hl * 0.24
    ew = hw * 0.56
    draw_rect(d, ex, cy, eh, ew, shade(pal[0], 0.78), (28, 28, 24, 255), 3)
    # 冷却ルーバー
    for k in range(-2, 3):
        lx = ex + k * (eh * 0.28)
        d.line([(lx, cy - ew * 0.72), (lx, cy + ew * 0.72)], fill=(34, 34, 30, 255), width=3)
    # 排気管
    if exhausts == 2:
        for sgn in (-1, 1):
            d.rounded_rectangle(
                [cx - hl - 6, cy + sgn * hw * 0.32 - 5, cx - hl + 12, cy + sgn * hw * 0.32 + 5],
                radius=3,
                fill=(78, 56, 44, 255),
                outline=(20, 20, 18, 255),
                width=2,
            )


def draw_gun(d, x0, y0, length, width, pal, muzzle_brake=False):
    w2 = width / 2
    # 砲身基部スリーブ
    draw_rect(d, x0 + length * 0.16, y0, length * 0.18, w2 * 1.45, shade(pal[0], 0.85), (20, 20, 18, 255), 3)
    # 主砲身
    draw_rect(d, x0 + length * 0.52, y0, length * 0.48, w2, shade(pal[0], 0.92), (18, 18, 16, 255), 3)
    # ハイライト筋
    d.line([(x0 + 6, y0 - w2 * 0.3), (x0 + length - 6, y0 - w2 * 0.3)], fill=pal[4] + (190,), width=max(2, int(width * 0.18)))
    if muzzle_brake:
        mx = x0 + length
        draw_rect(d, mx - 2, y0, 10, w2 * 1.75, shade(pal[3], 1.15), (16, 16, 14, 255), 3)
        d.line([(mx - 3, y0 - w2 * 1.4), (mx - 3, y0 + w2 * 1.4)], fill=(20, 20, 18, 255), width=3)


def make_hull_sprite(spec: dict) -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    pal = NATION_PAL[spec["pal"]]
    cx, cy = S // 2, S // 2
    hl = spec.get("hl", 124)
    hw = spec.get("hw", 74)
    tw = spec.get("tw", 24)
    seed = spec.get("seed", 1)

    # 特二式カミの前部・後部水陸両用フロート
    if spec.get("amphib"):
        draw_rect(d, cx + hl * 0.92, cy, 34, hw * 0.72, shade(pal[0], 0.95), (22, 22, 18, 255), 4)
        outline_poly(
            d,
            [(cx + hl * 1.16, cy - hw * 0.68), (cx + hl * 1.44, cy - hw * 0.25), (cx + hl * 1.44, cy + hw * 0.25), (cx + hl * 1.16, cy + hw * 0.68)],
            pal[0],
        )
        draw_rect(d, cx - hl * 0.98, cy, 28, hw * 0.68, shade(pal[0], 0.88), (22, 22, 18, 255), 4)

    # T28 二重履帯
    if spec.get("quad_tracks"):
        draw_tracks(d, cx, cy, hl, hw, tw * 0.85)
        draw_tracks(d, cx, cy, hl * 0.95, hw - tw * 0.95, tw * 0.75)
    else:
        draw_tracks(d, cx, cy, hl, hw, tw)

    # 主車体
    body_w = hw - tw * 0.68
    body_l = hl * 0.92
    draw_rect(d, cx, cy, body_l, body_w, pal[0], (20, 20, 18, 255), 4)

    # 迷彩
    if spec.get("camo", True):
        mask = Image.new("L", (S, S), 0)
        ImageDraw.Draw(mask).polygon(box_pts(cx, cy, body_l - 4, body_w - 4), fill=255)
        camo_layer = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        draw_camo_blobs(ImageDraw.Draw(camo_layer), cx, cy, body_l, body_w, pal, seed)
        clipped = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        clipped.paste(camo_layer, (0, 0), mask)
        im = Image.alpha_composite(im, clipped)
        d = ImageDraw.Draw(im)

    # 前面傾斜装甲ハイライト
    outline_poly(
        d,
        [(cx + body_l * 0.56, cy - body_w + 3), (cx + body_l - 3, cy - body_w + 5), (cx + body_l - 3, cy + body_w - 5), (cx + body_l * 0.56, cy + body_w - 3)],
        shade(pal[4], 0.92),
        (28, 28, 24, 255),
        3,
    )
    # 操縦手ハッチ・前方機銃
    d.rectangle([cx + body_l * 0.62, cy - body_w * 0.55, cx + body_l * 0.78, cy - body_w * 0.2], fill=shade(pal[0], 0.8), outline=(24, 24, 20, 255), width=2)
    d.ellipse([cx + body_l * 0.76, cy + body_w * 0.25, cx + body_l * 0.88, cy + body_w * 0.45], fill=pal[3], outline=(20, 20, 18, 255), width=2)

    # 機関室
    draw_engine_deck(d, cx, cy, body_l, body_w, pal)

    # 予備燃料タンク（ソ連系）
    if spec.get("drums"):
        for sgn in (-1, 1):
            draw_rect(d, cx - body_l * 0.56, cy + sgn * (body_w + 6), 24, 8, shade(pal[0], 0.85), (20, 20, 18, 255), 3)

    # 側面シュルツェン（ドイツ系）
    if spec.get("skirts"):
        for sgn in (-1, 1):
            sy = cy + sgn * (hw + 2)
            draw_rect(d, cx - 4, sy, hl * 0.84, 5, shade(pal[0], 0.9), (18, 18, 16, 255), 3)

    # 多砲塔の副砲塔（T-35, KV-5, O-I）
    if spec.get("sub_turrets"):
        for ox, oy, r in spec["sub_turrets"]:
            tx, ty = cx + ox, cy + oy
            d.ellipse([tx - r, ty - r, tx + r, ty + r], fill=shade(pal[0], 1.05), outline=(20, 20, 18, 255), width=3)
            draw_gun(d, tx + r * 0.4, ty, r * 1.4, 7, pal)

    # 無砲塔車両・自走砲の戦闘室／オープントップ
    if not spec.get("turret", True):
        cas_l = spec.get("cas_l", body_l * 0.44)
        cas_w = spec.get("cas_w", body_w * 0.78)
        cas_x = cx + spec.get("cas_x", 10)
        if spec.get("open_top"):
            # オープントップ戦闘室（内壁＋弾薬箱＋防盾）
            draw_rect(d, cas_x, cy, cas_l, cas_w, shade(pal[0], 0.95), (20, 20, 18, 255), 4)
            draw_rect(d, cas_x - 4, cy, cas_l * 0.76, cas_w * 0.74, (44, 42, 34, 255), (22, 22, 18, 255), 3)
            # 弾薬ラック
            for sgn in (-1, 1):
                draw_rect(d, cas_x - cas_l * 0.35, cy + sgn * cas_w * 0.5, 14, 7, (112, 92, 56, 255), (24, 22, 18, 255), 2)
        else:
            # 密閉式戦闘室（StuG III, ISU-152, T28, Semovente 75/18, L3/33）
            draw_rect(d, cas_x, cy, cas_l, cas_w, shade(pal[0], 1.08), (18, 18, 16, 255), 4)
            d.ellipse([cas_x - cas_l * 0.4, cy - cas_w * 0.45, cas_x + cas_l * 0.1, cy - cas_w * 0.05], fill=shade(pal[0], 0.85), outline=(22, 22, 18, 255), width=2)
            d.rectangle([cas_x - cas_l * 0.3, cy + cas_w * 0.12, cas_x + cas_l * 0.25, cy + cas_w * 0.48], fill=shade(pal[0], 0.85), outline=(22, 22, 18, 255), width=2)

    return im.resize((CELL, CELL), Image.LANCZOS)


def make_turret_sprite(spec: dict) -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    pal = NATION_PAL[spec["pal"]]
    cx, cy = S // 2, S // 2
    gun_len = spec.get("gun_len", 108)
    gun_w = spec.get("gun_w", 12)
    mb = spec.get("muzzle_brake", False)

    if not spec.get("turret", True):
        # 無砲塔自走砲は防盾と砲身のみを砲塔ピボット中心で描画（微調整旋回用）
        draw_rect(d, cx + 6, cy, 14, 18, shade(pal[0], 1.12), (20, 20, 18, 255), 3)
        draw_gun(d, cx + 8, cy, gun_len, gun_w, pal, mb)
        return im.resize((CELL, CELL), Image.LANCZOS)

    tur_l = spec.get("tur_l", 56)
    tur_w = spec.get("tur_w", 46)
    shape = spec.get("tur_shape", "hex")

    # 先に砲身
    draw_gun(d, cx + tur_l * 0.55, cy, gun_len, gun_w, pal, mb)
    if spec.get("coax_gun"):
        draw_gun(d, cx + tur_l * 0.55, cy + 16, gun_len * 0.55, 7, pal, False)

    # 砲塔本体
    if shape == "round":
        d.ellipse([cx - tur_l, cy - tur_w, cx + tur_l, cy + tur_w], fill=shade(pal[0], 1.1), outline=(18, 18, 16, 255), width=4)
    elif shape == "box":
        draw_rect(d, cx - 4, cy, tur_l, tur_w, shade(pal[0], 1.1), (18, 18, 16, 255), 4)
    else:
        pts = [
            (cx - tur_l, cy - tur_w * 0.72),
            (cx + tur_l * 0.3, cy - tur_w),
            (cx + tur_l, cy - tur_w * 0.58),
            (cx + tur_l, cy + tur_w * 0.58),
            (cx + tur_l * 0.3, cy + tur_w),
            (cx - tur_l, cy + tur_w * 0.72),
        ]
        outline_poly(d, pts, shade(pal[0], 1.1), (18, 18, 16, 255), 4)

    # 砲塔迷彩
    if spec.get("camo", True):
        d.ellipse([cx - tur_l * 0.45, cy - tur_w * 0.55, cx + tur_l * 0.25, cy + tur_w * 0.3], fill=pal[1] + (130,))

    # 砲塔後部無線箱（ファイアフライなど）
    if spec.get("bustle"):
        draw_rect(d, cx - tur_l - 12, cy, 14, tur_w * 0.55, shade(pal[0], 0.92), (20, 20, 18, 255), 3)

    # 砲塔シュルツェン（IV号H型）
    if spec.get("tur_skirts"):
        d.arc([cx - tur_l - 12, cy - tur_w - 12, cx + tur_l + 6, cy + tur_w + 12], 80, 280, fill=shade(pal[0], 0.9), width=6)

    # キューポラ・ハッチ
    cup_r = max(10, int(tur_w * 0.34))
    d.ellipse([cx - tur_l * 0.55 - cup_r, cy - tur_w * 0.35 - cup_r, cx - tur_l * 0.55 + cup_r, cy - tur_w * 0.35 + cup_r], fill=shade(pal[4], 0.9), outline=(20, 20, 18, 255), width=3)
    d.ellipse([cx - tur_l * 0.55 - cup_r * 0.55, cy - tur_w * 0.35 - cup_r * 0.55, cx - tur_l * 0.55 + cup_r * 0.55, cy - tur_w * 0.35 + cup_r * 0.55], fill=shade(pal[0], 0.8), outline=(24, 24, 20, 255), width=2)

    return im.resize((CELL, CELL), Image.LANCZOS)


def make_decoy_wreck() -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cx, cy = S // 2, S // 2
    # 折れた木枠と破れた迷彩布
    for ang, ln, wd, col in [
        (0.25, 110, 12, (124, 96, 62, 255)),
        (-0.4, 96, 10, (108, 84, 54, 255)),
        (0.85, 78, 8, (118, 92, 58, 255)),
    ]:
        ca, sa = math.cos(ang), math.sin(ang)
        pts = [(cx - ca * ln - sa * wd, cy - sa * ln + ca * wd), (cx + ca * ln - sa * wd, cy + sa * ln + ca * wd), (cx + ca * ln + sa * wd, cy + sa * ln - ca * wd), (cx - ca * ln + sa * wd, cy - sa * ln - ca * wd)]
        outline_poly(d, pts, col, (32, 24, 16, 255), 3)
    outline_poly(d, [(cx - 72, cy - 34), (cx + 12, cy - 52), (cx + 68, cy + 16), (cx - 42, cy + 48)], (134, 122, 82, 235), (36, 32, 22, 255), 3)
    outline_poly(d, [(cx - 24, cy - 12), (cx + 46, cy - 28), (cx + 28, cy + 28)], (88, 102, 64, 230), (32, 34, 24, 255), 3)
    return im.resize((CELL, CELL), Image.LANCZOS)


def make_drone_body() -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cx, cy = S // 2, S // 2
    arm = 112
    for k in range(4):
        a = math.pi / 4 + k * math.pi / 2
        tx, ty = cx + math.cos(a) * arm, cy + math.sin(a) * arm
        d.line([(cx, cy), (tx, ty)], fill=(28, 30, 26, 255), width=26)
        d.line([(cx, cy), (tx, ty)], fill=(96, 104, 82, 255), width=16)
        d.ellipse([tx - 24, ty - 24, tx + 24, ty + 24], fill=(74, 80, 68, 255), outline=(22, 22, 18, 255), width=4)
        d.ellipse([tx - 9, ty - 9, tx + 9, ty + 9], fill=(160, 166, 154, 255), outline=(22, 22, 18, 255), width=2)
    # 機体胴体
    outline_poly(d, [(cx - 78, cy - 28), (cx + 42, cy - 36), (cx + 88, cy - 18), (cx + 88, cy + 18), (cx + 42, cy + 36), (cx - 78, cy + 28)], (112, 122, 92, 255), (22, 22, 18, 255), 4)
    draw_rect(d, cx + 60, cy, 24, 20, (76, 84, 88, 255), (22, 22, 18, 255), 3)
    # 光学カメラレンズ（機首下部張り出し）
    d.ellipse([cx + 76, cy - 12, cx + 104, cy + 12], fill=(42, 58, 72, 255), outline=(20, 20, 18, 255), width=3)
    d.ellipse([cx + 84, cy - 6, cx + 98, cy + 6], fill=(96, 190, 230, 255))
    # ステータスLED
    d.ellipse([cx - 64, cy - 8, cx - 54, cy + 2], fill=(255, 190, 60, 255))
    d.ellipse([cx - 64, cy + 4, cx - 54, cy + 14], fill=(90, 225, 255, 255))
    return im.resize((CELL, CELL), Image.LANCZOS)


def make_drone_rotor() -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cx, cy = S // 2, S // 2
    r = 128
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(210, 214, 206, 68), outline=(230, 235, 225, 125), width=4)
    d.ellipse([cx - r * 0.72, cy - r * 0.72, cx + r * 0.72, cy + r * 0.72], outline=(180, 184, 176, 55), width=3)
    # ブレード残像
    for k in range(2):
        a = k * math.pi
        pts = [(cx, cy), (cx + math.cos(a - 0.14) * r * 0.96, cy + math.sin(a - 0.14) * r * 0.96), (cx + math.cos(a + 0.14) * r * 0.96, cy + math.sin(a + 0.14) * r * 0.96)]
        d.polygon(pts, fill=(40, 44, 38, 175))
    d.ellipse([cx - 14, cy - 14, cx + 14, cy + 14], fill=(150, 154, 146, 255), outline=(24, 24, 20, 255), width=3)
    return im.resize((CELL, CELL), Image.LANCZOS)


def make_dragons_teeth() -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cx, cy = S // 2, S // 2
    # 土台パッチ
    d.ellipse([cx - 158, cy - 158, cx + 158, cy + 158], fill=(64, 58, 42, 195))
    # 有刺鉄線リング
    for ang in range(0, 360, 36):
        rad = math.radians(ang)
        bx, by = cx + math.cos(rad) * 86, cy + math.sin(rad) * 86
        d.ellipse([bx - 36, by - 36, bx + 36, by + 36], outline=(82, 64, 48, 220), width=4)
    # 5基のコンクリート角錐（真上からの4面ベベル）
    offsets = [(-78, -74), (78, -68), (-68, 78), (82, 82), (0, 0)]
    for ox, oy in offsets:
        px, py = cx + ox, cy + oy
        r = 56
        # 左上明・右下暗
        d.polygon([(px - r, py - r), (px + r, py - r), (px, py)], fill=(204, 200, 190, 255))
        d.polygon([(px - r, py - r), (px, py), (px - r, py + r)], fill=(176, 174, 164, 255))
        d.polygon([(px + r, py - r), (px + r, py + r), (px, py)], fill=(134, 132, 124, 255))
        d.polygon([(px - r, py + r), (px, py), (px + r, py + r)], fill=(108, 106, 98, 255))
        d.rectangle([px - r, py - r, px + r, py + r], outline=(36, 36, 32, 255), width=4)
        # 頂点の露出した錆鉄筋
        d.ellipse([px - 7, py - 7, px + 7, py + 7], fill=(120, 72, 46, 255), outline=(28, 24, 20, 255), width=2)
    return im.resize((CELL, CELL), Image.LANCZOS)


def make_hq_bunker() -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cx, cy = S // 2, S // 2
    # 土嚢周囲壁
    for i in range(-5, 6):
        for sy in (-146, 146):
            d.rounded_rectangle([cx + i * 26 - 12, cy + sy - 12, cx + i * 26 + 12, cy + sy + 12], radius=6, fill=(164, 148, 112, 255), outline=(44, 40, 30, 255), width=3)
        for sx in (-146, 146):
            d.rounded_rectangle([cx + sx - 12, cy + i * 26 - 12, cx + sx + 12, cy + i * 26 + 12], radius=6, fill=(154, 138, 104, 255), outline=(44, 40, 30, 255), width=3)
    # 主コンクリートトーチカ屋根（斜角ベベル）
    draw_rect(d, cx, cy, 132, 132, (148, 146, 138, 255), (28, 28, 24, 255), 5)
    draw_rect(d, cx, cy, 92, 92, (176, 174, 166, 255), (42, 42, 36, 255), 4)
    # 観測スリット庇
    draw_rect(d, cx + 106, cy, 16, 62, (120, 118, 112, 255), (24, 24, 20, 255), 3)
    # 擬装網（左上角）
    outline_poly(d, [(cx - 134, cy - 134), (cx + 12, cy - 134), (cx - 46, cy - 38), (cx - 134, cy + 16)], (74, 92, 48, 225), (30, 38, 22, 255), 3)
    for k in range(-120, 0, 24):
        d.line([(cx + k, cy - 130), (cx - 130, cy + k)], fill=(48, 62, 32, 180), width=3)
    # 屋根上無線塔基部と弾薬箱
    draw_rect(d, cx - 22, cy - 22, 22, 22, (74, 78, 74, 255), (22, 22, 18, 255), 3)
    d.line([(cx - 44, cy - 44), (cx, cy)], fill=(42, 44, 42, 255), width=4)
    d.line([(cx - 44, cy), (cx, cy - 44)], fill=(42, 44, 42, 255), width=4)
    draw_rect(d, cx + 56, cy + 108, 24, 14, (112, 92, 56, 255), (28, 24, 18, 255), 3)
    return im.resize((CELL, CELL), Image.LANCZOS)


# data.rs と完全一致する 42 車種定義
VEHICLES = [
    # Germany (0..6)
    {"id": "de_panzer3_j", "pal": "de_grey", "hl": 106, "hw": 66, "tw": 20, "turret": True, "tur_l": 46, "tur_w": 38, "gun_len": 88, "gun_w": 9, "seed": 11},
    {"id": "de_panzer4_h", "pal": "de_gelb", "hl": 114, "hw": 70, "tw": 22, "skirts": True, "tur_skirts": True, "turret": True, "tur_l": 50, "tur_w": 42, "gun_len": 102, "gun_w": 11, "muzzle_brake": True, "seed": 12},
    {"id": "de_panther_g", "pal": "de_gelb", "hl": 126, "hw": 74, "tw": 24, "skirts": True, "turret": True, "tur_l": 54, "tur_w": 44, "gun_len": 126, "gun_w": 11, "muzzle_brake": True, "seed": 13},
    {"id": "de_tiger1", "pal": "de_gelb", "hl": 126, "hw": 84, "tw": 28, "turret": True, "tur_shape": "round", "tur_l": 56, "tur_w": 52, "gun_len": 118, "gun_w": 13, "muzzle_brake": True, "seed": 14},
    {"id": "de_luchs", "pal": "de_gelb", "hl": 92, "hw": 56, "tw": 18, "turret": True, "tur_l": 36, "tur_w": 30, "gun_len": 64, "gun_w": 7, "seed": 15},
    {"id": "de_stug3_g", "pal": "de_gelb", "hl": 108, "hw": 68, "tw": 20, "skirts": True, "turret": False, "cas_l": 54, "cas_w": 48, "gun_len": 98, "gun_w": 11, "muzzle_brake": True, "seed": 16},
    {"id": "de_maus", "pal": "de_gelb", "hl": 148, "hw": 94, "tw": 30, "skirts": True, "turret": True, "tur_shape": "box", "tur_l": 68, "tur_w": 58, "gun_len": 132, "gun_w": 16, "coax_gun": True, "seed": 17},
    # USSR (7..13)
    {"id": "ru_t70", "pal": "ru", "hl": 94, "hw": 58, "tw": 18, "turret": True, "tur_l": 36, "tur_w": 30, "gun_len": 70, "gun_w": 8, "seed": 21},
    {"id": "ru_t34_76", "pal": "ru", "hl": 112, "hw": 70, "tw": 22, "drums": True, "turret": True, "tur_l": 46, "tur_w": 38, "gun_len": 94, "gun_w": 11, "seed": 22},
    {"id": "ru_t34_85", "pal": "ru", "hl": 118, "hw": 72, "tw": 22, "drums": True, "turret": True, "tur_shape": "round", "tur_l": 54, "tur_w": 46, "gun_len": 112, "gun_w": 12, "seed": 23},
    {"id": "ru_is2", "pal": "ru", "hl": 128, "hw": 80, "tw": 26, "drums": True, "turret": True, "tur_shape": "round", "tur_l": 58, "tur_w": 50, "gun_len": 124, "gun_w": 15, "muzzle_brake": True, "seed": 24},
    {"id": "ru_t35", "pal": "ru", "hl": 144, "hw": 74, "tw": 20, "turret": True, "tur_l": 46, "tur_w": 40, "gun_len": 82, "gun_w": 11, "sub_turrets": [(64, -22, 18), (64, 22, 18), (-58, -22, 16), (-58, 22, 16)], "seed": 25},
    {"id": "ru_isu152", "pal": "ru", "hl": 126, "hw": 80, "tw": 24, "drums": True, "turret": False, "cas_l": 64, "cas_w": 56, "cas_x": 14, "gun_len": 102, "gun_w": 18, "muzzle_brake": True, "seed": 26},
    {"id": "ru_kv5", "pal": "ru", "hl": 144, "hw": 90, "tw": 28, "drums": True, "turret": True, "tur_shape": "box", "tur_l": 62, "tur_w": 52, "gun_len": 122, "gun_w": 15, "sub_turrets": [(82, -26, 20), (82, 26, 16)], "seed": 27},
    # USA (14..20)
    {"id": "us_m5_stuart", "pal": "us", "hl": 96, "hw": 60, "tw": 18, "turret": True, "tur_l": 38, "tur_w": 32, "gun_len": 68, "gun_w": 8, "seed": 31},
    {"id": "us_m4_sherman", "pal": "us", "hl": 112, "hw": 70, "tw": 22, "turret": True, "tur_shape": "round", "tur_l": 48, "tur_w": 42, "gun_len": 92, "gun_w": 11, "seed": 32},
    {"id": "us_m4a3e8", "pal": "us", "hl": 118, "hw": 74, "tw": 24, "turret": True, "tur_shape": "round", "tur_l": 52, "tur_w": 44, "gun_len": 110, "gun_w": 11, "muzzle_brake": True, "seed": 33},
    {"id": "us_m26_pershing", "pal": "us", "hl": 126, "hw": 78, "tw": 26, "turret": True, "tur_l": 56, "tur_w": 48, "gun_len": 118, "gun_w": 13, "muzzle_brake": True, "seed": 34},
    {"id": "us_m22_locust", "pal": "us", "hl": 88, "hw": 54, "tw": 16, "turret": True, "tur_l": 34, "tur_w": 28, "gun_len": 62, "gun_w": 7, "seed": 35},
    {"id": "us_m7_priest", "pal": "us", "hl": 112, "hw": 70, "tw": 20, "turret": False, "open_top": True, "cas_l": 64, "cas_w": 52, "gun_len": 82, "gun_w": 14, "seed": 36},
    {"id": "us_t28", "pal": "us", "hl": 146, "hw": 98, "tw": 24, "quad_tracks": True, "turret": False, "cas_l": 82, "cas_w": 56, "cas_x": 12, "gun_len": 126, "gun_w": 15, "muzzle_brake": True, "seed": 37},
    # UK (21..27)
    {"id": "uk_crusader3", "pal": "uk", "hl": 110, "hw": 62, "tw": 18, "turret": True, "tur_l": 44, "tur_w": 36, "gun_len": 84, "gun_w": 9, "seed": 41},
    {"id": "uk_cromwell", "pal": "uk", "hl": 114, "hw": 70, "tw": 22, "turret": True, "tur_shape": "box", "tur_l": 48, "tur_w": 42, "gun_len": 90, "gun_w": 11, "seed": 42},
    {"id": "uk_churchill7", "pal": "uk", "hl": 134, "hw": 74, "tw": 24, "turret": True, "tur_shape": "box", "tur_l": 50, "tur_w": 44, "gun_len": 92, "gun_w": 11, "seed": 43},
    {"id": "uk_firefly", "pal": "uk", "hl": 116, "hw": 70, "tw": 22, "turret": True, "tur_shape": "round", "tur_l": 50, "tur_w": 42, "bustle": True, "gun_len": 126, "gun_w": 11, "muzzle_brake": True, "seed": 44},
    {"id": "uk_tetrarch", "pal": "uk", "hl": 90, "hw": 56, "tw": 16, "turret": True, "tur_l": 34, "tur_w": 28, "gun_len": 64, "gun_w": 8, "seed": 45},
    {"id": "uk_bishop", "pal": "uk", "hl": 110, "hw": 68, "tw": 20, "turret": False, "cas_l": 56, "cas_w": 48, "cas_x": -6, "gun_len": 80, "gun_w": 13, "seed": 46},
    {"id": "uk_tog2", "pal": "uk", "hl": 158, "hw": 76, "tw": 24, "turret": True, "tur_shape": "box", "tur_l": 52, "tur_w": 44, "gun_len": 118, "gun_w": 11, "seed": 47},
    # Japan (28..34)
    {"id": "jp_hago", "pal": "jp", "hl": 90, "hw": 56, "tw": 18, "turret": True, "tur_l": 34, "tur_w": 28, "gun_len": 62, "gun_w": 7, "seed": 51},
    {"id": "jp_chiha_shinhoto", "pal": "jp", "hl": 102, "hw": 62, "tw": 20, "turret": True, "tur_l": 44, "tur_w": 36, "gun_len": 80, "gun_w": 9, "seed": 52},
    {"id": "jp_chihe", "pal": "jp", "hl": 106, "hw": 66, "tw": 20, "turret": True, "tur_l": 46, "tur_w": 38, "gun_len": 84, "gun_w": 9, "seed": 53},
    {"id": "jp_chinu", "pal": "jp", "hl": 110, "hw": 68, "tw": 20, "turret": True, "tur_shape": "box", "tur_l": 50, "tur_w": 42, "gun_len": 96, "gun_w": 11, "seed": 54},
    {"id": "jp_kami", "pal": "jp", "hl": 100, "hw": 58, "tw": 18, "amphib": True, "turret": True, "tur_l": 36, "tur_w": 30, "gun_len": 64, "gun_w": 7, "seed": 55},
    {"id": "jp_horo", "pal": "jp", "hl": 106, "hw": 66, "tw": 20, "turret": False, "open_top": True, "cas_l": 54, "cas_w": 46, "gun_len": 78, "gun_w": 16, "seed": 56},
    {"id": "jp_oi", "pal": "jp", "hl": 146, "hw": 90, "tw": 28, "turret": True, "tur_shape": "box", "tur_l": 60, "tur_w": 52, "gun_len": 116, "gun_w": 15, "sub_turrets": [(78, -28, 17), (78, 28, 17), (-74, 0, 17)], "seed": 57},
    # Italy (35..41)
    {"id": "it_l6_40", "pal": "it", "hl": 88, "hw": 54, "tw": 16, "turret": True, "tur_l": 32, "tur_w": 26, "gun_len": 58, "gun_w": 6, "seed": 61},
    {"id": "it_m13_40", "pal": "it", "hl": 102, "hw": 62, "tw": 20, "turret": True, "tur_l": 40, "tur_w": 34, "gun_len": 76, "gun_w": 9, "seed": 62},
    {"id": "it_semovente_75_18", "pal": "it", "hl": 106, "hw": 66, "tw": 20, "turret": False, "cas_l": 54, "cas_w": 48, "gun_len": 78, "gun_w": 12, "muzzle_brake": True, "seed": 63},
    {"id": "it_p26_40", "pal": "it", "hl": 114, "hw": 70, "tw": 22, "turret": True, "tur_l": 48, "tur_w": 40, "gun_len": 96, "gun_w": 11, "muzzle_brake": True, "seed": 64},
    {"id": "it_l3_33", "pal": "it", "hl": 74, "hw": 46, "tw": 14, "turret": False, "cas_l": 38, "cas_w": 32, "gun_len": 44, "gun_w": 5, "seed": 65},
    {"id": "it_semovente_90_53", "pal": "it", "hl": 108, "hw": 66, "tw": 20, "turret": False, "open_top": True, "cas_l": 56, "cas_w": 46, "cas_x": -10, "gun_len": 114, "gun_w": 11, "seed": 66},
    {"id": "it_p43_bis", "pal": "it", "hl": 122, "hw": 74, "tw": 22, "turret": True, "tur_l": 52, "tur_w": 44, "gun_len": 114, "gun_w": 12, "muzzle_brake": True, "seed": 67},
]


def encode_rle(im: Image.Image) -> bytes:
    """Header: b'YYSP' + u16 width + u16 height + PackBits-style RGBA pixel runs."""
    w, h = im.size
    raw = im.tobytes("raw", "RGBA")
    pixels = [raw[i : i + 4] for i in range(0, len(raw), 4)]
    out = bytearray(b"YYSP")
    out.extend(struct.pack("<HH", w, h))

    n = len(pixels)
    i = 0
    while i < n:
        # 同じピクセルの連続長を数える
        run = 1
        while i + run < n and run < 129 and pixels[i + run] == pixels[i]:
            run += 1
        if run >= 2:
            out.append(126 + run)  # 128..255 -> 2..129
            out.extend(pixels[i])
            i += run
        else:
            # 非連続区間
            start = i
            i += 1
            while i < n and (i - start) < 128:
                if i + 1 < n and pixels[i] == pixels[i + 1]:
                    break
                i += 1
            cnt = i - start
            out.append(cnt - 1)  # 0..127 -> 1..128
            for k in range(start, i):
                out.extend(pixels[k])
    return bytes(out)


def main():
    os.makedirs(os.path.dirname(OUT_PNG), exist_ok=True)
    # セル配置:
    # 0..41   : 42車種の車体 (hull)
    # 42..83  : 42車種の砲塔/砲身 (turret)
    # 84      : decoy_wreck
    # 85      : drone_body
    # 86      : drone_rotor
    # 87      : dragons_teeth
    # 88      : hq_bunker
    cells = []
    for v in VEHICLES:
        cells.append(make_hull_sprite(v))
    for v in VEHICLES:
        cells.append(make_turret_sprite(v))
    cells.extend([
        make_decoy_wreck(),
        make_drone_body(),
        make_drone_rotor(),
        make_dragons_teeth(),
        make_hq_bunker(),
    ])

    total = len(cells)
    rows = math.ceil(total / COLS)
    atlas = Image.new("RGBA", (COLS * CELL, rows * CELL), (0, 0, 0, 0))
    for idx, cell in enumerate(cells):
        gx = (idx % COLS) * CELL
        gy = (idx // COLS) * CELL
        atlas.paste(cell, (gx, gy), cell)

    atlas.save(OUT_PNG, "PNG")
    rle = encode_rle(atlas)
    with open(OUT_RLE, "wb") as f:
        f.write(rle)
    print(f"topdown_atlas: {atlas.size[0]}x{atlas.size[1]}, cells={total}, rle={len(rle)} bytes")


if __name__ == "__main__":
    main()
