"""YYMythology 風 3/4 斜め見下ろし（クォータービュー）スプライトアトラス生成スクリプト。

python tools/make_oblique_sprites.py
  -> assets/sprites/oblique_atlas.png (確認用PNG)
  -> assets/sprites/oblique_atlas.rle (Rust組み込み用 RLE圧縮RGBAバイナリ)

既存の AI生成済み透過RGBA画像（assets/sprites/*.png）はそのまま高品質セルとして取り込み、
未生成車種も同じ 3/4 クォータービュー投影（yaw=-35°, pitch=32°）で 3D 描画して統一する。
"""
import math
import os
import random
import struct
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SPRITES_DIR = os.path.join(ROOT, "assets", "sprites")
OUT_PNG = os.path.join(SPRITES_DIR, "oblique_atlas.png")
OUT_RLE = os.path.join(SPRITES_DIR, "oblique_atlas.rle")

CELL = 128
SS = 4
S = CELL * SS  # 512x512 作業キャンバス
COLS = 8

# 車両ファイルは通常 tank_<車両ID>.png。既存の例外だけ対応付ける。
LEGACY_VEHICLE_FILES = {
    "de_tiger1": "tank_tiger1.png",
    "ru_t34_85": "tank_t34_85.png",
}

NATION_PAL = {
    "de_grey": ((94, 100, 106), (72, 78, 84), (118, 124, 130)),
    "de_gelb": ((196, 174, 112), (104, 122, 70), (138, 78, 54)),
    "ru": ((94, 118, 66), (72, 92, 50), (116, 138, 84)),
    "us": ((104, 112, 74), (82, 90, 58), (124, 130, 88)),
    "uk": ((128, 124, 90), (96, 102, 74), (148, 142, 106)),
    "jp": ((142, 126, 82), (86, 102, 58), (116, 86, 54)),
    "it": ((196, 174, 120), (146, 124, 80), (112, 102, 72)),
}

# 3/4 クォータービュー投影（+X=前右下, +Y=右左下, +Z=上）
YAW = math.radians(36.0)
PITCH = math.radians(33.0)
CY, SY = math.cos(YAW), math.sin(YAW)
CP, SP = math.cos(PITCH), math.sin(PITCH)
LIGHT = (-0.52, -0.45, 0.72)


def norm(v):
    l = math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) or 1.0
    return (v[0] / l, v[1] / l, v[2] / l)


LIGHT_N = norm(LIGHT)


def project(pt, scale=212.0, cx=S * 0.46, cy=S * 0.60):
    x, y, z = pt
    # 車両正面(+X)が右下に、側面(+Y)が左下に、上(+Z)が画面上に向く
    xr = x * CY - y * SY
    yr = x * SY + y * CY
    sx = cx + xr * scale
    sy = cy + (yr * SP - z * CP) * scale
    depth = yr * CP + z * SP  # 大きいほど手前（カメラに近い）
    return (sx, sy, depth)


def face_normal(pts):
    ax, ay, az = pts[1][0] - pts[0][0], pts[1][1] - pts[0][1], pts[1][2] - pts[0][2]
    bx, by, bz = pts[2][0] - pts[0][0], pts[2][1] - pts[0][1], pts[2][2] - pts[0][2]
    return norm((ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx))


def add_poly(faces, pts3d, base_col, edge_col=(20, 20, 16, 255), edge_w=4, cull=False):
    n = face_normal(pts3d)
    # カメラ視線方向（概ね (-SY*CP, -CY*CP, ...)? 画面投影から直接 2D 符号面積を見る方が確実）
    proj = [project(p) for p in pts3d]
    area2 = 0.0
    for i in range(len(proj)):
        x1, y1, _ = proj[i]
        x2, y2, _ = proj[(i + 1) % len(proj)]
        area2 += (x1 * y2 - x2 * y1)
    if cull and area2 <= 0.0:
        return
    ndotl = max(-1.0, min(1.0, n[0] * LIGHT_N[0] + n[1] * LIGHT_N[1] + n[2] * LIGHT_N[2]))
    lit = 0.72 + 0.54 * max(0.0, ndotl) + 0.16 * n[2]
    col = (
        max(0, min(255, int(base_col[0] * lit))),
        max(0, min(255, int(base_col[1] * lit))),
        max(0, min(255, int(base_col[2] * lit))),
        255,
    )
    avg_d = sum(p[2] for p in proj) / len(proj)
    max_z = max(p[2] for p in pts3d)
    faces.append((avg_d, max_z, [(p[0], p[1]) for p in proj], col, edge_col, edge_w))


def add_box(faces, x0, x1, y0, y1, z0, z1, col, front_slope=0.0, rear_slope=0.0, top_inset_y=0.0, edge_w=4):
    # 8頂点（前上が少し後ろに傾く傾斜装甲に対応）
    tx0 = x0 + rear_slope
    tx1 = x1 - front_slope
    ty0 = y0 + top_inset_y
    ty1 = y1 - top_inset_y
    b00 = (x0, y0, z0)
    b10 = (x1, y0, z0)
    b11 = (x1, y1, z0)
    b01 = (x0, y1, z0)
    t00 = (tx0, ty0, z1)
    t10 = (tx1, ty0, z1)
    t11 = (tx1, ty1, z1)
    t01 = (tx0, ty1, z1)
    # 全6面（後ろ・左・下・右・前・上）
    add_poly(faces, [b00, b01, t01, t00], col, edge_w=edge_w)  # rear (-X)
    add_poly(faces, [b10, b00, t00, t10], col, edge_w=edge_w)  # left (-Y)
    add_poly(faces, [b01, b11, t11, t01], col, edge_w=edge_w)  # right (+Y, 手前側)
    add_poly(faces, [b11, b10, t10, t11], col, edge_w=edge_w)  # front (+X, 手前正面)
    add_poly(faces, [t00, t01, t11, t10], col, edge_w=edge_w)  # top (+Z)


def add_cyl_y(faces, cx, y0, y1, cz, r, col, seg=12, edge_w=3):
    """Y軸方向の円筒（転輪や外部燃料タンクなど）"""
    ring0 = []
    ring1 = []
    for k in range(seg):
        a = k / seg * math.tau
        px = cx + math.cos(a) * r
        pz = cz + math.sin(a) * r
        ring0.append((px, y0, pz))
        ring1.append((px, y1, pz))
    for k in range(seg):
        nk = (k + 1) % seg
        add_poly(faces, [ring0[k], ring1[k], ring1[nk], ring0[nk]], col, edge_w=max(1, edge_w - 1))
    add_poly(faces, ring1, col, edge_w=edge_w)
    add_poly(faces, list(reversed(ring0)), col, edge_w=edge_w)


def add_cyl_x(faces, x0, x1, cy, cz, r, col, seg=10, edge_w=3):
    """X軸方向の円筒（砲身・マズルブレーキなど）"""
    ring0 = []
    ring1 = []
    for k in range(seg):
        a = k / seg * math.tau
        py = cy + math.cos(a) * r
        pz = cz + math.sin(a) * r
        ring0.append((x0, py, pz))
        ring1.append((x1, py, pz))
    for k in range(seg):
        nk = (k + 1) % seg
        add_poly(faces, [ring0[k], ring0[nk], ring1[nk], ring1[k]], col, edge_w=max(2, edge_w - 1))
    add_poly(faces, ring1, (24, 24, 20), edge_w=edge_w)


def add_cyl_z(faces, cx, cy, z0, z1, r, col, seg=12, edge_w=3):
    """Z軸方向の円筒（キューポラ・丸砲塔など）"""
    ring0 = []
    ring1 = []
    for k in range(seg):
        a = k / seg * math.tau
        px = cx + math.cos(a) * r
        py = cy + math.sin(a) * r
        ring0.append((px, py, z0))
        ring1.append((px, py, z1))
    for k in range(seg):
        nk = (k + 1) % seg
        add_poly(faces, [ring0[k], ring1[k], ring1[nk], ring0[nk]], col, edge_w=max(2, edge_w - 1))
    add_poly(faces, ring1, col, edge_w=edge_w)


def make_oblique_vehicle(spec: dict) -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    pal = NATION_PAL[spec["pal"]]
    base, camo1, _ = pal

    hl = spec.get("hl", 0.95)
    hw = spec.get("hw", 0.56)
    hh = spec.get("hh", 0.34)
    tw = spec.get("tw", 0.14)
    sloped = spec.get("sloped", 0.22)

    # 接地影
    sh_pts = [project((x, y, 0.0))[:2] for x, y in [(-hl * 1.05, -hw * 1.05), (hl * 1.08, -hw * 1.05), (hl * 1.08, hw * 1.15), (-hl * 1.05, hw * 1.15)]]
    d.polygon(sh_pts, fill=(0, 0, 0, 105))

    faces = []
    track_col = (54, 52, 46)
    wheel_col = (84, 80, 70)

    # 履帯と転輪
    track_positions = [-hw + tw * 0.5, hw - tw * 0.5]
    if spec.get("quad_tracks"):
        track_positions = [-hw + tw * 0.45, -hw + tw * 1.45, hw - tw * 1.45, hw - tw * 0.45]

    for ty in track_positions:
        add_box(faces, -hl, hl, ty - tw * 0.48, ty + tw * 0.48, 0.02, hh * 0.62, track_col, front_slope=0.06, rear_slope=0.05, edge_w=3)
    # 手前側面の転輪列
    wheel_n = spec.get("wheels", 5)
    wr = hh * 0.26
    for k in range(wheel_n):
        wx = -hl * 0.72 + k * (hl * 1.44 / max(1, wheel_n - 1))
        add_cyl_y(faces, wx, hw - tw * 0.85, hw + 0.015, wr + 0.02, wr, wheel_col, seg=10, edge_w=2)

    # 特二式カミの前後フロート
    if spec.get("amphib"):
        add_box(faces, hl * 0.96, hl * 1.32, -hw * 0.78, hw * 0.78, 0.05, hh * 0.82, base, front_slope=0.22, edge_w=4)
        add_box(faces, -hl * 1.24, -hl * 0.96, -hw * 0.72, hw * 0.72, 0.08, hh * 0.78, base, rear_slope=0.14, edge_w=4)

    # 車体本体
    bw = hw - tw * 0.68
    add_box(faces, -hl * 0.94, hl * 0.96, -bw, bw, hh * 0.28, hh, base, front_slope=sloped, rear_slope=0.06, top_inset_y=0.03, edge_w=4)
    # 機関室グリル＆前部迷彩パネル
    add_box(faces, -hl * 0.82, -hl * 0.42, -bw * 0.62, bw * 0.62, hh, hh + 0.025, camo1, edge_w=3)
    add_box(faces, hl * 0.42, hl * 0.72, -bw * 0.46, bw * 0.15, hh, hh + 0.02, camo1, edge_w=2)

    # 側面シュルツェン
    if spec.get("skirts"):
        for sgn in (-1, 1):
            sy = sgn * (hw + 0.02)
            add_box(faces, -hl * 0.82, hl * 0.78, sy - 0.02, sy + 0.02, hh * 0.20, hh * 0.82, camo1, edge_w=3)

    # 予備燃料タンク（ソ連系）
    if spec.get("drums"):
        for sgn in (-1, 1):
            add_cyl_y(faces, -hl * 0.58, sgn * (bw + 0.01), sgn * (bw + 0.14), hh * 0.92, 0.09, base, seg=10, edge_w=2)

    # 副砲塔（多砲塔）
    if spec.get("sub_turrets"):
        for ox, oy, sr in spec["sub_turrets"]:
            add_cyl_z(faces, ox, oy, hh, hh + 0.22, sr, base, seg=10, edge_w=3)
            add_cyl_x(faces, ox + sr * 0.7, ox + sr * 2.1, oy, hh + 0.12, 0.03, (70, 70, 64), seg=8, edge_w=2)

    # 砲塔 or 戦闘室/自走砲
    gun_len = spec.get("gun_len", 0.82)
    gun_r = spec.get("gun_r", 0.042)
    if spec.get("turret", True):
        tl = spec.get("tur_l", 0.42)
        tw2 = spec.get("tur_w", 0.34)
        th = spec.get("tur_h", 0.32)
        tx = spec.get("tur_x", 0.05)
        if spec.get("tur_shape") == "round":
            add_cyl_z(faces, tx, 0.0, hh, hh + th, (tl + tw2) * 0.5, base, seg=14, edge_w=4)
        else:
            add_box(faces, tx - tl, tx + tl, -tw2, tw2, hh, hh + th, base, front_slope=0.08, rear_slope=0.06, top_inset_y=0.06, edge_w=4)
        # キューポラ
        add_cyl_z(faces, tx - tl * 0.38, -tw2 * 0.3, hh + th, hh + th + 0.11, tw2 * 0.34, camo1, seg=10, edge_w=3)
        # 砲身＋マズルブレーキ
        gz = hh + th * 0.54
        gx0 = tx + tl * 0.85
        add_cyl_x(faces, gx0, gx0 + gun_len, 0.0, gz, gun_r, base, seg=8, edge_w=3)
        if spec.get("coax_gun"):
            add_cyl_x(faces, gx0, gx0 + gun_len * 0.55, 0.12, gz - 0.02, 0.024, base, seg=8, edge_w=2)
        if spec.get("muzzle_brake"):
            add_box(faces, gx0 + gun_len - 0.04, gx0 + gun_len + 0.09, -gun_r * 1.7, gun_r * 1.7, gz - gun_r * 1.5, gz + gun_r * 1.5, (72, 70, 64), edge_w=3)
    else:
        cl = spec.get("cas_l", 0.44)
        cw = spec.get("cas_w", 0.38)
        ch = spec.get("cas_h", 0.34)
        cx0 = spec.get("cas_x", 0.06)
        if spec.get("open_top"):
            # オープントップ戦闘室（周囲の装甲板＋内部の砲架と長砲身）
            add_box(faces, cx0 - cl, cx0 + cl, -cw, -cw + 0.06, hh, hh + ch, base, front_slope=0.05, edge_w=3)
            add_box(faces, cx0 - cl, cx0 + cl, cw - 0.06, cw, hh, hh + ch, base, front_slope=0.05, edge_w=3)
            add_box(faces, cx0 + cl - 0.08, cx0 + cl, -cw, cw, hh, hh + ch * 1.05, base, front_slope=0.05, edge_w=4)
            add_box(faces, cx0 - 0.12, cx0 + 0.18, -0.14, 0.14, hh, hh + ch * 0.65, (64, 62, 54), edge_w=3)
            gz = hh + ch * 0.62
        else:
            add_box(faces, cx0 - cl, cx0 + cl, -cw, cw, hh, hh + ch, base, front_slope=0.14, rear_slope=0.06, top_inset_y=0.07, edge_w=4)
            gz = hh + ch * 0.50
        gx0 = cx0 + cl * 0.72
        add_cyl_x(faces, gx0, gx0 + gun_len, 0.0, gz, gun_r, base, seg=8, edge_w=3)
        if spec.get("muzzle_brake"):
            add_box(faces, gx0 + gun_len - 0.04, gx0 + gun_len + 0.09, -gun_r * 1.7, gun_r * 1.7, gz - gun_r * 1.5, gz + gun_r * 1.5, (72, 70, 64), edge_w=3)

    # 奥行きソート（背面→手前）でポリゴンを描画
    faces.sort(key=lambda f: (f[0], f[1]))
    for _, _, pts2d, col, edge_col, edge_w in faces:
        d.polygon(pts2d, fill=col)
        d.line(pts2d + [pts2d[0]], fill=edge_col, width=edge_w, joint="curve")

    # 巨大戦車（マウス・T28・TOG II*等）がセル枠からはみ出す場合は綺麗に収める
    bbox = im.getbbox()
    if bbox and (bbox[0] < 16 or bbox[1] < 16 or bbox[2] > S - 16 or bbox[3] > S - 16):
        cr = im.crop(bbox)
        cw, ch = cr.size
        sc = min((S - 32) / cw, (S - 32) / ch)
        nw, nh = max(1, int(round(cw * sc))), max(1, int(round(ch * sc)))
        cr = cr.resize((nw, nh), Image.LANCZOS)
        out = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        out.paste(cr, ((S - nw) // 2, (S - nh) // 2), cr)
        im = out

    return im.resize((CELL, CELL), Image.LANCZOS)


def make_decoy_wreck_oblique() -> Image.Image:
    im = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    faces = []
    # 崩れた木枠と破れた迷彩布
    add_box(faces, -0.75, 0.70, -0.45, -0.25, 0.0, 0.10, (124, 96, 62), edge_w=3)
    add_box(faces, -0.55, 0.65, 0.22, 0.42, 0.0, 0.10, (112, 86, 54), edge_w=3)
    add_box(faces, -0.35, 0.40, -0.36, 0.36, 0.04, 0.24, (132, 120, 80), front_slope=0.18, edge_w=3)
    add_cyl_x(faces, 0.20, 1.05, 0.06, 0.10, 0.04, (108, 86, 56), seg=8, edge_w=2)
    faces.sort(key=lambda f: (f[0], f[1]))
    for _, _, pts2d, col, edge_col, edge_w in faces:
        d.polygon(pts2d, fill=col)
        d.line(pts2d + [pts2d[0]], fill=edge_col, width=edge_w, joint="curve")
    return im.resize((CELL, CELL), Image.LANCZOS)


VEHICLES = [
    # Germany (0..6)
    {"id": "de_panzer3_j", "pal": "de_grey", "hl": 0.88, "hw": 0.52, "hh": 0.32, "gun_len": 0.68},
    {"id": "de_panzer4_h", "pal": "de_gelb", "hl": 0.92, "hw": 0.54, "hh": 0.33, "skirts": True, "gun_len": 0.80, "muzzle_brake": True},
    {"id": "de_panther_g", "pal": "de_gelb", "hl": 1.00, "hw": 0.58, "hh": 0.36, "sloped": 0.28, "skirts": True, "gun_len": 0.96, "muzzle_brake": True},
    {"id": "de_tiger1", "pal": "de_gelb", "hl": 1.02, "hw": 0.66, "hh": 0.38, "gun_len": 0.92, "muzzle_brake": True},
    {"id": "de_luchs", "pal": "de_gelb", "hl": 0.74, "hw": 0.45, "hh": 0.28, "wheels": 4, "tur_l": 0.32, "tur_w": 0.26, "tur_h": 0.26, "gun_len": 0.52, "gun_r": 0.028},
    {"id": "de_stug3_g", "pal": "de_gelb", "hl": 0.88, "hw": 0.52, "hh": 0.26, "skirts": True, "turret": False, "cas_l": 0.42, "cas_w": 0.38, "cas_h": 0.28, "gun_len": 0.78, "muzzle_brake": True},
    {"id": "de_maus", "pal": "de_gelb", "hl": 1.18, "hw": 0.72, "hh": 0.46, "sloped": 0.22, "skirts": True, "wheels": 6, "tur_l": 0.54, "tur_w": 0.46, "tur_h": 0.38, "gun_len": 1.02, "gun_r": 0.06, "coax_gun": True},
    # USSR (7..13)
    {"id": "ru_t70", "pal": "ru", "hl": 0.76, "hw": 0.46, "hh": 0.28, "sloped": 0.20, "wheels": 4, "tur_l": 0.32, "tur_w": 0.26, "tur_h": 0.26, "gun_len": 0.56, "gun_r": 0.032},
    {"id": "ru_t34_76", "pal": "ru", "hl": 0.90, "hw": 0.54, "hh": 0.32, "sloped": 0.28, "drums": True, "tur_l": 0.38, "tur_w": 0.30, "tur_h": 0.28, "gun_len": 0.74},
    {"id": "ru_t34_85", "pal": "ru", "hl": 0.94, "hw": 0.56, "hh": 0.33, "sloped": 0.28, "drums": True, "tur_shape": "round", "gun_len": 0.88},
    {"id": "ru_is2", "pal": "ru", "hl": 1.02, "hw": 0.62, "hh": 0.37, "sloped": 0.24, "drums": True, "tur_shape": "round", "tur_l": 0.48, "tur_w": 0.40, "tur_h": 0.34, "gun_len": 0.96, "gun_r": 0.056, "muzzle_brake": True},
    {"id": "ru_t35", "pal": "ru", "hl": 1.16, "hw": 0.58, "hh": 0.34, "wheels": 6, "tur_l": 0.38, "tur_w": 0.32, "tur_h": 0.30, "gun_len": 0.62, "sub_turrets": [(0.56, -0.22, 0.14), (0.56, 0.22, 0.14), (-0.54, -0.22, 0.13), (-0.54, 0.22, 0.13)]},
    {"id": "ru_isu152", "pal": "ru", "hl": 1.02, "hw": 0.62, "hh": 0.32, "drums": True, "turret": False, "cas_l": 0.52, "cas_w": 0.44, "cas_h": 0.38, "cas_x": 0.10, "gun_len": 0.78, "gun_r": 0.068, "muzzle_brake": True},
    {"id": "ru_kv5", "pal": "ru", "hl": 1.16, "hw": 0.70, "hh": 0.42, "drums": True, "wheels": 6, "tur_l": 0.52, "tur_w": 0.42, "tur_h": 0.40, "gun_len": 0.94, "gun_r": 0.054, "sub_turrets": [(0.68, -0.24, 0.16), (0.68, 0.24, 0.13)]},
    # USA (14..20)
    {"id": "us_m5_stuart", "pal": "us", "hl": 0.78, "hw": 0.48, "hh": 0.32, "wheels": 4, "tur_l": 0.34, "tur_w": 0.28, "tur_h": 0.28, "gun_len": 0.56, "gun_r": 0.032},
    {"id": "us_m4_sherman", "pal": "us", "hl": 0.90, "hw": 0.55, "hh": 0.37, "sloped": 0.24, "tur_shape": "round", "tur_l": 0.40, "tur_w": 0.34, "tur_h": 0.30, "gun_len": 0.72},
    {"id": "us_m4a3e8", "pal": "us", "hl": 0.94, "hw": 0.58, "hh": 0.38, "sloped": 0.24, "tur_shape": "round", "tur_l": 0.42, "tur_w": 0.36, "tur_h": 0.31, "gun_len": 0.88, "muzzle_brake": True},
    {"id": "us_m26_pershing", "pal": "us", "hl": 1.00, "hw": 0.62, "hh": 0.36, "sloped": 0.24, "tur_l": 0.46, "tur_w": 0.38, "tur_h": 0.32, "gun_len": 0.94, "gun_r": 0.048, "muzzle_brake": True},
    {"id": "us_m22_locust", "pal": "us", "hl": 0.70, "hw": 0.44, "hh": 0.26, "sloped": 0.20, "wheels": 4, "tur_l": 0.30, "tur_w": 0.24, "tur_h": 0.24, "gun_len": 0.50, "gun_r": 0.028},
    {"id": "us_m7_priest", "pal": "us", "hl": 0.90, "hw": 0.55, "hh": 0.30, "turret": False, "open_top": True, "cas_l": 0.52, "cas_w": 0.42, "cas_h": 0.34, "gun_len": 0.64, "gun_r": 0.056},
    {"id": "us_t28", "pal": "us", "hl": 1.16, "hw": 0.76, "hh": 0.30, "quad_tracks": True, "wheels": 6, "turret": False, "cas_l": 0.66, "cas_w": 0.44, "cas_h": 0.32, "cas_x": 0.08, "gun_len": 1.02, "gun_r": 0.054, "muzzle_brake": True},
    # UK (21..27)
    {"id": "uk_crusader3", "pal": "uk", "hl": 0.88, "hw": 0.50, "hh": 0.28, "sloped": 0.20, "tur_l": 0.38, "tur_w": 0.30, "tur_h": 0.26, "gun_len": 0.68, "gun_r": 0.036},
    {"id": "uk_cromwell", "pal": "uk", "hl": 0.90, "hw": 0.55, "hh": 0.32, "tur_l": 0.40, "tur_w": 0.34, "tur_h": 0.29, "gun_len": 0.72},
    {"id": "uk_churchill7", "pal": "uk", "hl": 1.06, "hw": 0.58, "hh": 0.38, "wheels": 6, "tur_l": 0.42, "tur_w": 0.35, "tur_h": 0.32, "gun_len": 0.74},
    {"id": "uk_firefly", "pal": "uk", "hl": 0.94, "hw": 0.56, "hh": 0.37, "sloped": 0.24, "tur_shape": "round", "tur_l": 0.44, "tur_w": 0.35, "tur_h": 0.30, "gun_len": 1.02, "muzzle_brake": True},
    {"id": "uk_tetrarch", "pal": "uk", "hl": 0.72, "hw": 0.44, "hh": 0.27, "wheels": 4, "tur_l": 0.30, "tur_w": 0.25, "tur_h": 0.24, "gun_len": 0.52, "gun_r": 0.030},
    {"id": "uk_bishop", "pal": "uk", "hl": 0.88, "hw": 0.54, "hh": 0.30, "turret": False, "cas_l": 0.42, "cas_w": 0.38, "cas_h": 0.46, "cas_x": -0.06, "gun_len": 0.62, "gun_r": 0.052},
    {"id": "uk_tog2", "pal": "uk", "hl": 1.24, "hw": 0.60, "hh": 0.40, "wheels": 7, "tur_l": 0.42, "tur_w": 0.35, "tur_h": 0.34, "tur_x": 0.20, "gun_len": 0.94},
    # Japan (28..34)
    {"id": "jp_hago", "pal": "jp", "hl": 0.74, "hw": 0.44, "hh": 0.29, "wheels": 4, "tur_l": 0.30, "tur_w": 0.24, "tur_h": 0.25, "gun_len": 0.48, "gun_r": 0.028},
    {"id": "jp_chiha_shinhoto", "pal": "jp", "hl": 0.82, "hw": 0.50, "hh": 0.32, "tur_l": 0.38, "tur_w": 0.30, "tur_h": 0.28, "gun_len": 0.64, "gun_r": 0.036},
    {"id": "jp_chihe", "pal": "jp", "hl": 0.85, "hw": 0.52, "hh": 0.33, "sloped": 0.18, "tur_l": 0.39, "tur_w": 0.32, "tur_h": 0.29, "gun_len": 0.68, "gun_r": 0.036},
    {"id": "jp_chinu", "pal": "jp", "hl": 0.88, "hw": 0.54, "hh": 0.34, "tur_l": 0.42, "tur_w": 0.34, "tur_h": 0.32, "gun_len": 0.78},
    {"id": "jp_kami", "pal": "jp", "hl": 0.80, "hw": 0.48, "hh": 0.30, "amphib": True, "wheels": 4, "tur_l": 0.30, "tur_w": 0.25, "tur_h": 0.25, "gun_len": 0.52, "gun_r": 0.028},
    {"id": "jp_horo", "pal": "jp", "hl": 0.84, "hw": 0.52, "hh": 0.30, "turret": False, "open_top": True, "cas_l": 0.44, "cas_w": 0.38, "cas_h": 0.34, "gun_len": 0.62, "gun_r": 0.062},
    {"id": "jp_oi", "pal": "jp", "hl": 1.16, "hw": 0.70, "hh": 0.44, "wheels": 6, "tur_l": 0.48, "tur_w": 0.40, "tur_h": 0.36, "gun_len": 0.90, "gun_r": 0.054, "sub_turrets": [(0.62, -0.24, 0.14), (0.62, 0.24, 0.14), (-0.60, 0.0, 0.14)]},
    # Italy (35..41)
    {"id": "it_l6_40", "pal": "it", "hl": 0.72, "hw": 0.44, "hh": 0.28, "wheels": 4, "tur_l": 0.28, "tur_w": 0.22, "tur_h": 0.24, "gun_len": 0.46, "gun_r": 0.026},
    {"id": "it_m13_40", "pal": "it", "hl": 0.82, "hw": 0.50, "hh": 0.32, "tur_l": 0.35, "tur_w": 0.29, "tur_h": 0.27, "gun_len": 0.60, "gun_r": 0.036},
    {"id": "it_semovente_75_18", "pal": "it", "hl": 0.84, "hw": 0.52, "hh": 0.27, "turret": False, "cas_l": 0.42, "cas_w": 0.38, "cas_h": 0.26, "gun_len": 0.60, "gun_r": 0.048, "muzzle_brake": True},
    {"id": "it_p26_40", "pal": "it", "hl": 0.90, "hw": 0.56, "hh": 0.34, "sloped": 0.24, "tur_l": 0.40, "tur_w": 0.33, "tur_h": 0.30, "gun_len": 0.76, "muzzle_brake": True},
    {"id": "it_l3_33", "pal": "it", "hl": 0.60, "hw": 0.38, "hh": 0.22, "wheels": 4, "turret": False, "cas_l": 0.30, "cas_w": 0.26, "cas_h": 0.20, "gun_len": 0.34, "gun_r": 0.022},
    {"id": "it_semovente_90_53", "pal": "it", "hl": 0.86, "hw": 0.52, "hh": 0.28, "turret": False, "open_top": True, "cas_l": 0.44, "cas_w": 0.38, "cas_h": 0.32, "cas_x": -0.08, "gun_len": 0.94, "gun_r": 0.044},
    {"id": "it_p43_bis", "pal": "it", "hl": 0.96, "hw": 0.58, "hh": 0.35, "sloped": 0.26, "tur_l": 0.44, "tur_w": 0.36, "tur_h": 0.32, "gun_len": 0.90, "gun_r": 0.048, "muzzle_brake": True},
]


def load_or_render(filename: str, fallback_fn) -> Image.Image:
    path = os.path.join(SPRITES_DIR, filename)
    if os.path.exists(path):
        with Image.open(path) as im:
            return im.convert("RGBA").resize((CELL, CELL), Image.LANCZOS)
    return fallback_fn()


def encode_rle(im: Image.Image) -> bytes:
    w, h = im.size
    raw = im.tobytes("raw", "RGBA")
    pixels = [raw[i : i + 4] for i in range(0, len(raw), 4)]
    out = bytearray(b"YYSP")
    out.extend(struct.pack("<HH", w, h))
    n = len(pixels)
    i = 0
    while i < n:
        run = 1
        while i + run < n and run < 129 and pixels[i + run] == pixels[i]:
            run += 1
        if run >= 2:
            out.append(126 + run)
            out.extend(pixels[i])
            i += run
        else:
            start = i
            i += 1
            while i < n and (i - start) < 128:
                if i + 1 < n and pixels[i] == pixels[i + 1]:
                    break
                i += 1
            out.append((i - start) - 1)
            for k in range(start, i):
                out.extend(pixels[k])
    return bytes(out)


def main():
    os.makedirs(SPRITES_DIR, exist_ok=True)
    # セル配置:
    # 0..41 : 42車種（3/4 クォータービュー）
    # 42    : decoy_tank（3/4 張りぼて戦車）
    # 43    : decoy_wreck
    # 44    : recon_drone
    # 45    : dragons_teeth
    # 46    : hq_bunker
    cells = []
    for vehicle in VEHICLES:
        key = vehicle["id"]
        filename = LEGACY_VEHICLE_FILES.get(key, f"tank_{key}.png")
        cells.append(load_or_render(filename, lambda: make_oblique_vehicle(vehicle)))
    for key in ("decoy_tank", "decoy_wreck", "recon_drone", "dragons_teeth", "hq_bunker"):
        cells.append(load_or_render(f"{key}.png", make_decoy_wreck_oblique))

    rows = math.ceil(len(cells) / COLS)
    atlas = Image.new("RGBA", (COLS * CELL, rows * CELL), (0, 0, 0, 0))
    for idx, cell in enumerate(cells):
        gx = (idx % COLS) * CELL
        gy = (idx // COLS) * CELL
        atlas.paste(cell, (gx, gy))

    atlas.save(OUT_PNG, "PNG")
    rle = encode_rle(atlas)
    with open(OUT_RLE, "wb") as f:
        f.write(rle)
    print(f"oblique_atlas: {atlas.size[0]}x{atlas.size[1]}, cells={len(cells)}, rle={len(rle)} bytes")


if __name__ == "__main__":
    main()
