"""Draws the RallyNVX icon candidates as SVG (and PNG with rsvg-convert): 108x108 adaptive layers (background, foreground) and a 512 preview."""
import os, subprocess, sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "."
os.makedirs(OUT, exist_ok=True)

def side_car(body, accent, roof, x=0, y=0, s=1.0, number="7"):
    """A WRC hatchback from the side, facing right, drawn in a 100 x 50 box."""
    return f'''<g transform="translate({x},{y}) scale({s})">
  <ellipse cx="50" cy="47.5" rx="46" ry="3" fill="#000" opacity=".28"/>
  <!-- rear wing -->
  <path d="M3,17 L22,15.5 L22,18 L5,19.5 Z" fill="#15171c"/><rect x="10" y="18" width="2" height="8" fill="#15171c"/><rect x="17" y="17.5" width="2" height="8" fill="#15171c"/>
  <!-- body -->
  <path d="M4,38 L6,30 Q8,26 16,25.5 L29,24.5 L38,14 Q40,12.5 44,12.5 L65,12.5 Q69,12.5 72,16 L79,24 L91,26.5 Q96,27.5 96,32 L96,38.5 Q96,40 94,40 L6,40 Q4,40 4,38 Z" fill="{body}"/>
  <!-- roof and livery -->
  <path d="M40,12.7 L67,12.7 Q69,12.7 71,14.6 L72.5,16.2 L38,16.2 Z" fill="{roof}"/>
  <path d="M6,33 L94,31 L94,34.5 L6,36.5 Z" fill="{accent}"/>
  <!-- windows -->
  <path d="M39.5,16.5 L45,13.8 L64,13.8 Q67.5,13.8 69.5,16.5 L74.5,22.5 L31,23.2 Z" fill="#111a24"/>
  <path d="M52,13.8 L53.4,13.8 L52.4,23 L51,23 Z" fill="{body}"/>
  <path d="M42,15.5 L47,14.2 L51,14.2 L49.5,18 L40,18.6 Z" fill="#fff" opacity=".18"/>
  <!-- roof scoop and lights -->
  <rect x="53" y="10.2" width="8" height="2.4" rx="1" fill="#15171c"/>
  <circle cx="92" cy="30.3" r="2.1" fill="#fff6c8"/><circle cx="87.2" cy="30.6" r="1.8" fill="#ffe27a"/>
  <rect x="4" y="29" width="3" height="3.4" rx="1" fill="#ff3b30"/>
  <!-- door number -->
  <circle cx="55" cy="29.3" r="3.6" fill="#fff"/><text x="55" y="31.6" font-family="sans-serif" font-weight="800" font-size="6.2" text-anchor="middle" fill="#15171c">{number}</text>
  <!-- wheels -->
  <circle cx="23.5" cy="39.5" r="8.3" fill="#12141a"/><circle cx="23.5" cy="39.5" r="5.1" fill="#c5c9d1"/><circle cx="23.5" cy="39.5" r="1.6" fill="#12141a"/>
  <circle cx="77" cy="39.5" r="8.3" fill="#12141a"/><circle cx="77" cy="39.5" r="5.1" fill="#c5c9d1"/><circle cx="77" cy="39.5" r="1.6" fill="#12141a"/>
  <path d="M15,39.5 A8.5,8.5 0 0 1 32,39.5" fill="none" stroke="{body}" stroke-width="2.4"/>
  <path d="M68.5,39.5 A8.5,8.5 0 0 1 85.5,39.5" fill="none" stroke="{body}" stroke-width="2.4"/>
</g>'''

def front_car(body, accent, x=0, y=0, s=1.0):
    """A rally car head-on, drawn in a 100 x 70 box (centre x = 50)."""
    return f'''<g transform="translate({x},{y}) scale({s})">
  <ellipse cx="50" cy="66" rx="40" ry="3.5" fill="#000" opacity=".35"/>
  <rect x="14" y="52" width="14" height="14" rx="3" fill="#0e1015"/><rect x="72" y="52" width="14" height="14" rx="3" fill="#0e1015"/>
  <path d="M10,52 L12,38 Q14,30 26,27 L34,14 Q36,10 42,10 L58,10 Q64,10 66,14 L74,27 Q86,30 88,38 L90,52 Q90,56 86,56 L14,56 Q10,56 10,52 Z" fill="{body}"/>
  <path d="M35,16 Q37,12.5 42,12.5 L58,12.5 Q63,12.5 65,16 L70,26 L30,26 Z" fill="#101823"/>
  <path d="M38,16.5 L46,13.2 L50,13.2 L47,25 L33,25 Z" fill="#fff" opacity=".16"/>
  <rect x="44" y="30" width="12" height="5" rx="1.5" fill="#15171c"/>
  <path d="M34,28 L66,28 L68,31 L32,31 Z" fill="{accent}"/>
  <rect x="38" y="6.5" width="24" height="3.2" rx="1.5" fill="#15171c"/>
  <circle cx="26" cy="41" r="6.4" fill="#fff6c8" stroke="#15171c" stroke-width="1.6"/><circle cx="74" cy="41" r="6.4" fill="#fff6c8" stroke="#15171c" stroke-width="1.6"/>
  <circle cx="39" cy="38.5" r="4.4" fill="#ffe27a" stroke="#15171c" stroke-width="1.3"/><circle cx="61" cy="38.5" r="4.4" fill="#ffe27a" stroke="#15171c" stroke-width="1.3"/>
  <path d="M16,50 L84,50 L82,55 L18,55 Z" fill="#15171c"/>
  <rect x="40" y="46" width="20" height="7" rx="1.5" fill="#e9ecef"/>
  <text x="50" y="52.2" font-family="sans-serif" font-weight="800" font-size="5.2" text-anchor="middle" fill="#15171c">NVX</text>
  <path d="M10,40 L8,40 L7,48 L10.5,48 Z" fill="{accent}"/><path d="M90,40 L92,40 L93,48 L89.5,48 Z" fill="{accent}"/>
</g>'''

def svg(w, body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {w}" width="{w}" height="{w}">{body}</svg>'

VARIANTS = {}

# A: sunset stage, red/white car sliding, dust cloud
VARIANTS["a_sunset"] = (
    '''<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b1055"/><stop offset=".55" stop-color="#d9456b"/><stop offset="1" stop-color="#ffb347"/></linearGradient></defs>
<rect width="108" height="108" fill="url(#g)"/><circle cx="74" cy="40" r="15" fill="#ffd27a" opacity=".9"/>
<path d="M0,70 L18,52 L30,64 L48,44 L66,66 L84,50 L108,70 L108,108 L0,108 Z" fill="#3a1d5b" opacity=".85"/>
<path d="M0,84 L108,76 L108,108 L0,108 Z" fill="#241034"/>''',
    '''<g opacity=".85"><ellipse cx="22" cy="78" rx="16" ry="7" fill="#e8c7a1" opacity=".75"/><ellipse cx="12" cy="72" rx="11" ry="6" fill="#f3dcc2" opacity=".7"/><ellipse cx="30" cy="70" rx="9" ry="5" fill="#f3dcc2" opacity=".6"/></g>
<g transform="rotate(-7 54 60)">''' + side_car("#e63946", "#f1faee", "#f1faee", 17, 36, 0.74, "7") + "</g>")

# B: neon night, yellow/blue car
VARIANTS["b_neon"] = (
    '''<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0a1128"/><stop offset="1" stop-color="#1b3a6b"/></linearGradient></defs>
<rect width="108" height="108" fill="url(#g)"/>
<g stroke="#2de2e6" stroke-width="1.6" stroke-linecap="round" opacity=".75"><path d="M6,30 L44,30"/><path d="M0,44 L36,44"/><path d="M10,92 L50,92"/><path d="M20,100 L60,100"/></g>
<g stroke="#ff2a6d" stroke-width="1.6" stroke-linecap="round" opacity=".75"><path d="M64,20 L104,20"/><path d="M70,100 L108,100"/></g>
<path d="M0,86 L108,80 L108,108 L0,108 Z" fill="#050914"/>''',
    '''<g transform="rotate(-3 54 60)">''' + side_car("#ffd60a", "#0a5cff", "#0a5cff", 17, 38, 0.74, "NVX") + "</g>")

# C: flat bold badge: white car on a red-orange disc with a dark ring
VARIANTS["c_badge"] = (
    '''<rect width="108" height="108" fill="#15171c"/><circle cx="54" cy="54" r="46" fill="#ff5a1f"/><circle cx="54" cy="54" r="46" fill="none" stroke="#ffd166" stroke-width="3"/>
<clipPath id="disc"><circle cx="54" cy="54" r="45"/></clipPath><path clip-path="url(#disc)" d="M0,66 L108,54 L108,108 L0,108 Z" fill="#c63d0f" opacity=".7"/>''',
    side_car("#ffffff", "#15171c", "#15171c", 22, 33, 0.64, "1"))

# D: head-on, night stage with light beams
VARIANTS["d_headon"] = (
    '''<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#070b14"/><stop offset=".6" stop-color="#16213a"/><stop offset="1" stop-color="#3d2f24"/></linearGradient>
<linearGradient id="beam" x1="0" y1="1" x2="0" y2="0"><stop offset="0" stop-color="#fff6c8" stop-opacity=".7"/><stop offset="1" stop-color="#fff6c8" stop-opacity="0"/></linearGradient></defs>
<rect width="108" height="108" fill="url(#g)"/>
<path d="M54,60 L22,0 L48,0 Z" fill="url(#beam)"/><path d="M54,60 L60,0 L86,0 Z" fill="url(#beam)"/>
<path d="M36,108 L54,56 L72,108 Z" fill="#2b2118"/><path d="M0,108 L48,58 L54,58 L108,108 Z" fill="#3a2d21" opacity=".8"/>''',
    front_car("#1e6bff", "#ffd60a", 19, 31, 0.7))

# E: gravel road in perspective, car from the side-rear angle is hard: instead a big italic wordmark under a car
VARIANTS["e_wordmark"] = (
    '''<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff7a00"/><stop offset="1" stop-color="#ff2e63"/></linearGradient></defs>
<rect width="108" height="108" fill="url(#g)"/>
<path d="M-4,92 L112,60 L112,112 L-4,112 Z" fill="#1d1f27"/>
<g stroke="#fff" stroke-width="1.2" opacity=".35"><path d="M0,30 L40,30"/><path d="M8,38 L36,38"/><path d="M70,22 L108,22"/></g>''',
    '''<g transform="rotate(-8 54 50)">''' + side_car("#ffffff", "#1d1f27", "#1d1f27", 12, 20, 0.84, "7") + '''</g>
<text x="54" y="96" font-family="sans-serif" font-weight="900" font-style="italic" font-size="15" text-anchor="middle" fill="#ffffff" letter-spacing="1">RALLYNVX</text>''')

def render(name, bg, fg):
    full = svg(108, bg + fg)
    open(os.path.join(OUT, f"{name}.svg"), "w").write(full)
    open(os.path.join(OUT, f"{name}_bg.svg"), "w").write(svg(108, bg))
    open(os.path.join(OUT, f"{name}_fg.svg"), "w").write(svg(108, fg))
    for suffix, size in (("", 512), ("_bg", 432), ("_fg", 432)):
        subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), os.path.join(OUT, f"{name}{suffix}.svg"), "-o", os.path.join(OUT, f"{name}{suffix}.png")], check=True)

for n, (bg, fg) in VARIANTS.items(): render(n, bg, fg)
print("drawn", list(VARIANTS))


# ---- the chosen icon: Android resources (adaptive layers + legacy PNGs) ----
def android(res_dir, name="e_wordmark"):
    bg, fg = VARIANTS[name]
    # the adaptive foreground keeps the car and the word inside the 66 dp safe zone: the artwork is scaled about the centre
    fg_adaptive = fg.replace('font-size="15"', 'font-size="13.5"').replace('y="96"', 'y="93"')
    fg_adaptive = f'<g transform="translate(54 54) scale(0.66) translate(-54 -54)">{fg_adaptive}</g>'
    sizes = {"mdpi": (48, 108), "hdpi": (72, 162), "xhdpi": (96, 216), "xxhdpi": (144, 324), "xxxhdpi": (192, 432)}
    tmp = os.path.join(OUT, "android_tmp"); os.makedirs(tmp, exist_ok=True)
    layers = {"bg": svg(108, bg), "fg": svg(108, fg_adaptive), "full": svg(108, bg + fg_adaptive)}
    for k, v in layers.items(): open(os.path.join(tmp, k + ".svg"), "w").write(v)
    for dens, (legacy, adaptive) in sizes.items():
        d = os.path.join(res_dir, f"mipmap-{dens}"); os.makedirs(d, exist_ok=True)
        for k, out, px in (("bg", "ic_launcher_background.png", adaptive), ("fg", "ic_launcher_foreground.png", adaptive), ("full", "ic_launcher.png", legacy)):
            subprocess.run(["rsvg-convert", "-w", str(px), "-h", str(px), os.path.join(tmp, k + ".svg"), "-o", os.path.join(d, out)], check=True)
        # the round legacy icon: the same picture in a circle
        subprocess.run(["magick", os.path.join(d, "ic_launcher.png"), "(", "-size", f"{legacy}x{legacy}", "xc:none", "-fill", "white", "-draw", f"circle {legacy / 2},{legacy / 2} {legacy / 2},0", ")", "-compose", "DstIn", "-composite", os.path.join(d, "ic_launcher_round.png")], check=True)
    any_dir = os.path.join(res_dir, "mipmap-anydpi-v26"); os.makedirs(any_dir, exist_ok=True)
    xml = '<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n    <background android:drawable="@mipmap/ic_launcher_background"/>\n    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>\n</adaptive-icon>\n'
    for n in ("ic_launcher.xml", "ic_launcher_round.xml"): open(os.path.join(any_dir, n), "w").write(xml)
    subprocess.run(["rsvg-convert", "-w", "512", "-h", "512", os.path.join(tmp, "full.svg"), "-o", os.path.join(OUT, "store_icon.png")], check=True)

if len(sys.argv) > 2 and sys.argv[2] == "android":
    android(sys.argv[3])
