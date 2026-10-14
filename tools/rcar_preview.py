#!/usr/bin/env python3
"""Software preview of a .rcar car (body + four wheels) from a few angles -> PNG (pure python, slow but dependency free).
Usage: rcar_preview.py car.rcar out.png [yaw_degrees ...]"""
import math, struct, sys, zlib

def read(path):
    d = open(path, 'rb').read(); assert d[:4] == b'RCAR'; o = 4; meshes = []
    for _ in range(2):
        nv, ni = struct.unpack_from('<II', d, o); o += 8
        vs = []
        for _ in range(nv):
            p = struct.unpack_from('<3f', d, o); n = struct.unpack_from('<3f', d, o + 12); c = struct.unpack_from('4B', d, o + 24); o += 28; vs.append((p, n, c))
        idx = struct.unpack_from('<%dI' % ni, d, o); o += 4 * ni
        meshes.append((vs, idx))
    return meshes

def render(meshes, yaw, W=640, H=320, wheels=((0.767, 1.237), (-0.767, 1.237), (0.767, -1.210), (-0.767, -1.210)), radius=0.326):
    img = [[(150, 170, 190)] * W for _ in range(H)]; zb = [[1e9] * W for _ in range(H)]
    cy, sy = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
    def cam(p):   # rotate about y by yaw, then look from +z with a little elevation
        x, y, z = p; x, z = x * cy - z * sy, x * sy + z * cy
        el = math.radians(14); y2 = y * math.cos(el) - z * math.sin(el); z2 = y * math.sin(el) + z * math.cos(el)
        return x, y2 - 0.7, z2 + 7.0
    sun = (0.4, 0.8, 0.45); sl = math.sqrt(sum(s * s for s in sun)); sun = tuple(s / sl for s in sun)
    f = 560.0
    def draw(vs, idx, off, flip):
        for t in range(0, len(idx), 3):
            tri = []
            for k in range(3):
                p, n, c = vs[idx[t + k]]
                p = (p[0] * flip + off[0], p[1] + off[1], p[2] + off[2]); n = (n[0] * flip, n[1], n[2])
                cx, cyy, cz = cam(p)
                if cz < 0.5: break
                nx, ny, nz = n; nx, nz = nx * cy - nz * sy, nx * sy + nz * cy
                sx, sy2 = W / 2 + f * cx / cz, H * 0.62 - f * cyy / cz
                sh = max(0.0, nx * sun[0] + ny * sun[1] + nz * sun[2]) * 0.8 + 0.3
                tri.append((sx, sy2, cz, c, sh))
            if len(tri) < 3: continue
            (x0, y0, z0, c0, s0), (x1, y1, z1, c1, s1), (x2, y2, z2, c2, s2) = tri
            area = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0)
            if abs(area) < 1e-6: continue
            for py in range(max(0, int(min(y0, y1, y2))), min(H, int(max(y0, y1, y2)) + 1)):
                for px in range(max(0, int(min(x0, x1, x2))), min(W, int(max(x0, x1, x2)) + 1)):
                    w0 = ((x1 - px) * (y2 - py) - (x2 - px) * (y1 - py)) / area
                    w1 = ((x2 - px) * (y0 - py) - (x0 - px) * (y2 - py)) / area
                    w2 = 1 - w0 - w1
                    if w0 < 0 or w1 < 0 or w2 < 0: continue
                    z = w0 * z0 + w1 * z1 + w2 * z2
                    if z < zb[py][px]:
                        zb[py][px] = z
                        sh = w0 * s0 + w1 * s1 + w2 * s2
                        col = [w0 * c0[i] + w1 * c1[i] + w2 * c2[i] for i in range(3)]
                        img[py][px] = tuple(min(255, int((col[i] / 255.0) ** (1 / 2.2) * 255 * sh)) for i in range(3))
    draw(*meshes[0], (0, 0, 0), 1.0)
    for x, z in wheels: draw(*meshes[1], (x, radius, z), 1.0 if x > 0 else -1.0)
    return img

def png(path, img):
    H, W = len(img), len(img[0]); raw = b''.join(b'\x00' + bytes(v for px in row for v in px) for row in img)
    def chunk(t, d): c = struct.pack('>I', len(d)) + t + d; return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W, H, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 6)) + chunk(b'IEND', b''))

if __name__ == '__main__':
    m = read(sys.argv[1]); yaws = [float(a) for a in sys.argv[3:]] or [35, 215]
    tiles = [render(m, y) for y in yaws]
    rows = [sum((t[r] for t in tiles), []) for r in range(len(tiles[0]))]
    png(sys.argv[2], rows)
