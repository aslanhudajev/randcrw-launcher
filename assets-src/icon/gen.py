#!/usr/bin/env python3
"""Generates the randcrw app icon SVG (macOS Big Sur style) with exact geometry.
usage: gen.py <out.svg> [--small]"""
import math, os, sys

out = sys.argv[1]
small = "--small" in sys.argv

W = 1024
# Big Sur grid: 824x824 body centred, continuous-corner squircle.
BODY = 824
OFF = (W - BODY) / 2

def continuous_rect(x, y, w, h, r):
    """Rounded rectangle with Apple-style continuous (curvature-smooth) corners."""
    X, Y = x + w, y + h
    f = lambda a, b: f"{a:.2f},{b:.2f}"
    d = [f"M{f(x + 1.52866483 * r, y)}", f"L{f(X - 1.52866471 * r, y)}"]
    # top-right
    d.append(f"C{f(X - 1.08849323 * r, y)} {f(X - 0.86840689 * r, y)} {f(X - 0.66993427 * r, y + 0.06549600 * r)}")
    d.append(f"C{f(X - 0.37282392 * r, y + 0.16905899 * r)} {f(X - 0.16905899 * r, y + 0.37282392 * r)} {f(X - 0.06549600 * r, y + 0.66993427 * r)}")
    d.append(f"C{f(X, y + 0.86840689 * r)} {f(X, y + 1.08849323 * r)} {f(X, y + 1.52866483 * r)}")
    d.append(f"L{f(X, Y - 1.52866471 * r)}")
    # bottom-right
    d.append(f"C{f(X, Y - 1.08849323 * r)} {f(X, Y - 0.86840689 * r)} {f(X - 0.06549600 * r, Y - 0.66993427 * r)}")
    d.append(f"C{f(X - 0.16905899 * r, Y - 0.37282392 * r)} {f(X - 0.37282392 * r, Y - 0.16905899 * r)} {f(X - 0.66993427 * r, Y - 0.06549600 * r)}")
    d.append(f"C{f(X - 0.86840689 * r, Y)} {f(X - 1.08849323 * r, Y)} {f(X - 1.52866483 * r, Y)}")
    d.append(f"L{f(x + 1.52866471 * r, Y)}")
    # bottom-left
    d.append(f"C{f(x + 1.08849323 * r, Y)} {f(x + 0.86840689 * r, Y)} {f(x + 0.66993427 * r, Y - 0.06549600 * r)}")
    d.append(f"C{f(x + 0.37282392 * r, Y - 0.16905899 * r)} {f(x + 0.16905899 * r, Y - 0.37282392 * r)} {f(x + 0.06549600 * r, Y - 0.66993427 * r)}")
    d.append(f"C{f(x, Y - 0.86840689 * r)} {f(x, Y - 1.08849323 * r)} {f(x, Y - 1.52866483 * r)}")
    d.append(f"L{f(x, y + 1.52866471 * r)}")
    # top-left
    d.append(f"C{f(x, y + 1.08849323 * r)} {f(x, y + 0.86840689 * r)} {f(x + 0.06549600 * r, y + 0.66993427 * r)}")
    d.append(f"C{f(x + 0.16905899 * r, y + 0.37282392 * r)} {f(x + 0.37282392 * r, y + 0.16905899 * r)} {f(x + 0.66993427 * r, y + 0.06549600 * r)}")
    d.append(f"C{f(x + 0.86840689 * r, y)} {f(x + 1.08849323 * r, y)} {f(x + 1.52866483 * r, y)}")
    return " ".join(d) + " Z"

RADIUS = BODY * float(os.environ.get("ICON_RADIUS", "0.23")) / 1.0
SQ = continuous_rect(OFF, OFF, BODY, BODY, RADIUS / 1.0)

def norm(v):
    l = math.sqrt(sum(a * a for a in v))
    return tuple(a / l for a in v)

LIGHT = norm((-0.5, -0.7, 0.75))  # screen coords, y down, z towards viewer

def mix(c1, c2, t):
    t = max(0.0, min(1.0, t))
    a = [int(c1[i:i + 2], 16) for i in (1, 3, 5)]
    b = [int(c2[i:i + 2], 16) for i in (1, 3, 5)]
    return "#" + "".join(f"{round(a[k] + (b[k] - a[k]) * t):02x}" for k in range(3))

RAMP = [(0.0, "#3a1504"), (0.3, "#8a3a0e"), (0.55, "#d86e1c"), (0.78, "#ffa23c"), (0.92, "#ffcf85"), (1.0, "#fff0d6")]
def ramp(b):
    b = max(0.0, min(1.0, b))
    for (t0, c0), (t1, c1) in zip(RAMP, RAMP[1:]):
        if b <= t1:
            return mix(c0, c1, (b - t0) / (t1 - t0))
    return RAMP[-1][1]

def lambert(n):
    return max(0.0, sum(a * b for a, b in zip(n, LIGHT)))

# Nut geometry.
CX, CY = 512, (488 if small else 476)
R = float(os.environ.get("ICON_R", "300" if small else "262"))  # outer circumradius of the top face
ROT = math.radians(float(os.environ.get("ICON_ROT", "20")))
BEVEL = 0.80       # inner hex radius / R
DEPTH = 58         # visible thickness (viewed slightly from above)
HOLE = 0.40 * R
CSINK = HOLE + 20

def hexpts(r, cx=CX, cy=CY, rot=ROT):
    return [(cx + r * math.cos(rot + k * math.pi / 3), cy + r * math.sin(rot + k * math.pi / 3)) for k in range(6)]

outer = hexpts(R)
inner = hexpts(R * BEVEL)
P = lambda pts: " ".join(f"{x:.2f},{y:.2f}" for x, y in pts)

defs = []
body = []

# ---- background -------------------------------------------------------------------------
defs.append(f'''
<clipPath id="sq"><path d="{SQ}"/></clipPath>
<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#34466a"/>
  <stop offset=".45" stop-color="#1a2440"/>
  <stop offset="1" stop-color="#0a0f1d"/>
</linearGradient>
<radialGradient id="bgGlow" cx=".5" cy=".18" r=".7">
  <stop offset="0" stop-color="#6d8cc4" stop-opacity=".45"/>
  <stop offset="1" stop-color="#6d8cc4" stop-opacity="0"/>
</radialGradient>
<radialGradient id="halo" cx=".5" cy=".5" r=".5">
  <stop offset="0" stop-color="#ff9a3c" stop-opacity=".75"/>
  <stop offset=".5" stop-color="#f28c28" stop-opacity=".22"/>
  <stop offset="1" stop-color="#f28c28" stop-opacity="0"/>
</radialGradient>
<linearGradient id="planet" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#1d3358"/>
  <stop offset=".25" stop-color="#101b33"/>
  <stop offset="1" stop-color="#070b16"/>
</linearGradient>
<linearGradient id="atmo" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#7fd4ff" stop-opacity=".0"/>
  <stop offset=".5" stop-color="#7fd4ff" stop-opacity=".55"/>
  <stop offset="1" stop-color="#7fd4ff" stop-opacity="0"/>
</linearGradient>
<linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#ffffff" stop-opacity=".30"/>
  <stop offset=".25" stop-color="#ffffff" stop-opacity=".06"/>
  <stop offset=".8" stop-color="#ffffff" stop-opacity="0"/>
  <stop offset="1" stop-color="#ffffff" stop-opacity=".05"/>
</linearGradient>
<filter id="shadow" x="-10%" y="-10%" width="120%" height="125%">
  <feDropShadow dx="0" dy="12" stdDeviation="14" flood-color="#000" flood-opacity=".32"/>
</filter>
<filter id="blur30"><feGaussianBlur stdDeviation="30"/></filter>
<filter id="blur8"><feGaussianBlur stdDeviation="8"/></filter>
<filter id="blur3"><feGaussianBlur stdDeviation="3"/></filter>
<filter id="blur1"><feGaussianBlur stdDeviation="1"/></filter>
''')

scene = []
scene.append(f'<rect x="0" y="0" width="{W}" height="{W}" fill="url(#bg)"/>')
scene.append(f'<rect x="0" y="0" width="{W}" height="{W}" fill="url(#bgGlow)"/>')
if not small:
    # stars
    import random
    rnd = random.Random(7)
    for _ in range(46):
        x = rnd.uniform(120, 904); y = rnd.uniform(120, 560)
        if (x - CX) ** 2 + (y - CY) ** 2 < (R + 40) ** 2:
            continue
        r = rnd.choice([1.4, 1.8, 2.2, 2.8, 3.4])
        o = rnd.uniform(.25, .8)
        scene.append(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r}" fill="#dfe9ff" opacity="{o:.2f}"/>')
    # planet horizon
    scene.append('<circle cx="512" cy="1640" r="900" fill="url(#planet)"/>')
    scene.append('<circle cx="512" cy="1640" r="900" fill="none" stroke="#8fd8ff" stroke-opacity=".28" stroke-width="3"/>')
    scene.append('<circle cx="512" cy="1640" r="906" fill="none" stroke="#8fd8ff" stroke-opacity=".12" stroke-width="14" filter="url(#blur8)"/>')
# halo + shadow
scene.append(f'<ellipse cx="{CX}" cy="{CY + 10}" rx="{R * 1.35:.0f}" ry="{R * 1.3:.0f}" fill="url(#halo)"/>')
scene.append(f'<ellipse cx="{CX}" cy="{CY + R * 0.95 + DEPTH:.0f}" rx="{R * 0.92:.0f}" ry="{R * 0.16:.0f}" fill="#000" opacity=".55" filter="url(#blur30)"/>')

# ---- nut: side walls ----------------------------------------------------------------------
nut = []
for k in range(6):
    a, b = outer[k], outer[(k + 1) % 6]
    ex, ey = b[0] - a[0], b[1] - a[1]
    nx, ny = ey, -ex  # outward for counter-clockwise? check below
    # outward normal: from centre to edge midpoint
    mx, my = (a[0] + b[0]) / 2 - CX, (a[1] + b[1]) / 2 - CY
    if nx * mx + ny * my < 0:
        nx, ny = -nx, -ny
    l = math.hypot(nx, ny); nx, ny = nx / l, ny / l
    if ny <= 0.05:
        continue
    quad = [a, b, (b[0], b[1] + DEPTH), (a[0], a[1] + DEPTH)]
    bri = 0.18 + 0.75 * lambert((nx, ny, 0.0))
    gid = f"side{k}"
    defs.append(f'''<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{a[0]:.1f}" y2="{a[1] + DEPTH:.1f}">
  <stop offset="0" stop-color="{ramp(bri + 0.08)}"/>
  <stop offset=".55" stop-color="{ramp(bri - 0.06)}"/>
  <stop offset="1" stop-color="{ramp(bri - 0.22)}"/>
</linearGradient>''')
    nut.append(f'<polygon points="{P(quad)}" fill="url(#{gid})" stroke="{ramp(bri - 0.3)}" stroke-width="1.2" stroke-linejoin="round"/>')
    # thin catch-light along the top of the wall (chamfer edge)
    nut.append(f'<line x1="{a[0]:.1f}" y1="{a[1] + 2:.1f}" x2="{b[0]:.1f}" y2="{b[1] + 2:.1f}" stroke="#ffd9a0" stroke-opacity=".45" stroke-width="2"/>')
    # bottom chamfer catch-light
    nut.append(f'<line x1="{a[0]:.1f}" y1="{a[1] + DEPTH - 3:.1f}" x2="{b[0]:.1f}" y2="{b[1] + DEPTH - 3:.1f}" stroke="{ramp(bri + 0.25)}" stroke-opacity=".7" stroke-width="3" stroke-linecap="round"/>')
    if nx > 0.35:  # cool Clank-green rim light on the right-hand wall edge
        nut.append(f'<line x1="{b[0] - 1.5:.1f}" y1="{b[1] + 4:.1f}" x2="{b[0] - 1.5:.1f}" y2="{b[1] + DEPTH - 6:.1f}" stroke="#9dffc0" stroke-opacity=".75" stroke-width="3" stroke-linecap="round"/>')

# ---- nut: bevel facets ---------------------------------------------------------------------
TILT = math.radians(40)
for k in range(6):
    a, b = outer[k], outer[(k + 1) % 6]
    c, d = inner[(k + 1) % 6], inner[k]
    mx, my = (a[0] + b[0]) / 2 - CX, (a[1] + b[1]) / 2 - CY
    l = math.hypot(mx, my); nx, ny = mx / l, my / l
    n3 = norm((nx * math.sin(TILT), ny * math.sin(TILT), math.cos(TILT)))
    bri = 0.12 + 0.95 * lambert(n3)
    gid = f"bev{k}"
    defs.append(f'''<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}">
  <stop offset="0" stop-color="{ramp(bri + 0.05)}"/>
  <stop offset=".5" stop-color="{ramp(bri + 0.10)}"/>
  <stop offset="1" stop-color="{ramp(bri - 0.04)}"/>
</linearGradient>''')
    nut.append(f'<polygon points="{P([a, b, c, d])}" fill="url(#{gid})"/>')
    # metal reflects the cool sky on upward-facing facets
    if ny < -0.05:
        nut.append(f'<polygon points="{P([a, b, c, d])}" fill="#a8cbff" opacity="{min(0.28, -ny * 0.3):.3f}" style="mix-blend-mode:soft-light"/>')

# facet seams
for k in range(6):
    nut.append(f'<line x1="{outer[k][0]:.1f}" y1="{outer[k][1]:.1f}" x2="{inner[k][0]:.1f}" y2="{inner[k][1]:.1f}" stroke="#5a2408" stroke-opacity=".35" stroke-width="1.5"/>')

# ---- nut: top face -------------------------------------------------------------------------
defs.append(f'''<linearGradient id="face" gradientUnits="userSpaceOnUse" x1="{CX - R * 0.7:.0f}" y1="{CY - R * 0.7:.0f}" x2="{CX + R * 0.7:.0f}" y2="{CY + R * 0.7:.0f}">
  <stop offset="0" stop-color="#ffc877"/>
  <stop offset=".42" stop-color="#f7942e"/>
  <stop offset="1" stop-color="#c05514"/>
</linearGradient>
<linearGradient id="sheen" gradientUnits="userSpaceOnUse" x1="{CX - R:.0f}" y1="{CY - R:.0f}" x2="{CX + R * 0.4:.0f}" y2="{CY + R * 0.4:.0f}">
  <stop offset="0" stop-color="#fff" stop-opacity="0"/>
  <stop offset=".38" stop-color="#fff" stop-opacity="0"/>
  <stop offset=".47" stop-color="#fff" stop-opacity=".28"/>
  <stop offset=".56" stop-color="#fff" stop-opacity="0"/>
  <stop offset="1" stop-color="#fff" stop-opacity="0"/>
</linearGradient>
<radialGradient id="hot" gradientUnits="userSpaceOnUse" cx="{CX - R * 0.35:.0f}" cy="{CY - R * 0.42:.0f}" r="{R * 0.9:.0f}">
  <stop offset="0" stop-color="#fff3d6" stop-opacity=".55"/>
  <stop offset=".45" stop-color="#ffd08a" stop-opacity=".12"/>
  <stop offset="1" stop-color="#ffd08a" stop-opacity="0"/>
</radialGradient>
<radialGradient id="hole" cx=".5" cy=".62" r=".6">
  <stop offset="0" stop-color="#1b2640"/>
  <stop offset=".7" stop-color="#0c1224"/>
  <stop offset="1" stop-color="#050810"/>
</radialGradient>
<linearGradient id="csink" gradientUnits="userSpaceOnUse" x1="{CX - CSINK:.0f}" y1="{CY - CSINK:.0f}" x2="{CX + CSINK:.0f}" y2="{CY + CSINK:.0f}">
  <stop offset="0" stop-color="#7a3310"/>
  <stop offset=".5" stop-color="#c2591a"/>
  <stop offset="1" stop-color="#ffc071"/>
</linearGradient>
<clipPath id="holeClip"><circle cx="{CX}" cy="{CY}" r="{HOLE:.1f}"/></clipPath>
<clipPath id="faceClip"><polygon points="{P(inner)}"/></clipPath>''')
nut.append(f'<polygon points="{P(inner)}" fill="url(#face)"/>')
nut.append(f'<polygon points="{P(inner)}" fill="url(#hot)"/>')
nut.append(f'<polygon points="{P(inner)}" fill="url(#sheen)"/>')
# brushed-metal concentric hint
if not small:
    for i, rr in enumerate(range(int(CSINK) + 18, int(R * BEVEL), 14)):
        nut.append(f'<circle cx="{CX}" cy="{CY}" r="{rr}" fill="none" stroke="#fff" stroke-opacity="{0.035 if i % 2 else 0.02}" stroke-width="2" clip-path="url(#faceClip)"/>')
# inner edge highlight of the face (top-left) and shade (bottom-right)
nut.append(f'<polygon points="{P(inner)}" fill="none" stroke="#fff4dc" stroke-opacity=".35" stroke-width="2.5" stroke-linejoin="round"/>')
# countersink + hole
nut.append(f'<circle cx="{CX}" cy="{CY}" r="{CSINK:.1f}" fill="url(#csink)"/>')
nut.append(f'<circle cx="{CX}" cy="{CY}" r="{HOLE:.1f}" fill="url(#hole)"/>')
# far wall of the bore: orange metal fading into the dark
defs.append(f'''<linearGradient id="wall" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#b8521a"/>
  <stop offset=".35" stop-color="#5a230a"/>
  <stop offset=".62" stop-color="#140c10" stop-opacity="0"/>
</linearGradient>''')
nut.append(f'<circle cx="{CX}" cy="{CY}" r="{HOLE:.1f}" fill="url(#wall)"/>')
nut.append(f'<ellipse cx="{CX}" cy="{CY + HOLE * 0.30:.1f}" rx="{HOLE * 0.93:.1f}" ry="{HOLE * 0.72:.1f}" fill="#070a14" opacity=".85" clip-path="url(#holeClip)" filter="url(#blur8)"/>')
# inner top shadow and bottom reflected light in the bore
nut.append(f'<circle cx="{CX}" cy="{CY - 16}" r="{HOLE:.1f}" fill="none" stroke="#000" stroke-opacity=".55" stroke-width="22" clip-path="url(#holeClip)" filter="url(#blur8)"/>')
nut.append(f'<path d="M{CX - HOLE * 0.72:.1f},{CY + HOLE * 0.69:.1f} A{HOLE:.1f},{HOLE:.1f} 0 0 0 {CX + HOLE * 0.72:.1f},{CY + HOLE * 0.69:.1f}" fill="none" stroke="#ffb35c" stroke-opacity=".55" stroke-width="3" stroke-linecap="round"/>')
nut.append(f'<circle cx="{CX}" cy="{CY}" r="{CSINK:.1f}" fill="none" stroke="#fff1d0" stroke-opacity=".30" stroke-width="2"/>')

# outer crisp edge
nut.append(f'<polygon points="{P(outer)}" fill="none" stroke="#fff0d2" stroke-opacity=".55" stroke-width="2.4" stroke-linejoin="round" clip-path="url(#topHalf)"/>')
defs.append(f'<clipPath id="topHalf"><rect x="0" y="0" width="{CX + 40}" height="{CY - 20}"/></clipPath>')
# specular glint on the brightest bevel
gl = outer[4]
nut.append(f'<ellipse cx="{(outer[3][0] + outer[4][0]) / 2 + 30:.1f}" cy="{(outer[3][1] + outer[4][1]) / 2 + 14:.1f}" rx="60" ry="9" transform="rotate({math.degrees(math.atan2(outer[4][1] - outer[3][1], outer[4][0] - outer[3][0])):.1f} {(outer[3][0] + outer[4][0]) / 2 + 30:.1f} {(outer[3][1] + outer[4][1]) / 2 + 14:.1f})" fill="#fff" opacity=".55" filter="url(#blur3)"/>')

inner_scene = "\n".join(scene) + "\n<g>" + "\n".join(nut) + "</g>"

svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{W}" viewBox="0 0 {W} {W}">
<!-- randcrw app icon (Ratchet & Clank: ReWrite). Generated by assets-src/icon/gen.py; edit the generator, not this file. -->
<defs>{"".join(defs)}</defs>
<g filter="url(#shadow)">
  <path d="{SQ}" fill="#0a0f1d"/>
</g>
<g clip-path="url(#sq)">
{inner_scene}
</g>
<path d="{SQ}" fill="none" stroke="url(#rim)" stroke-width="4"/>
</svg>
'''
open(out, "w").write(svg)
print("wrote", out)
