//! Minimal f32 vector, quaternion and 4x4 matrix helpers (column-major, `m[col][row]`).
use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V3 { pub x: f32, pub y: f32, pub z: f32 }
pub const fn v3(x: f32, y: f32, z: f32) -> V3 { V3 { x, y, z } }

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub fn dot(self, o: V3) -> f32 { self.x * o.x + self.y * o.y + self.z * o.z }
    pub fn cross(self, o: V3) -> V3 { v3(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x) }
    pub fn len(self) -> f32 { self.dot(self).sqrt() }
    pub fn norm(self) -> V3 { let l = self.len(); if l > 1e-9 { self * (1.0 / l) } else { V3::ZERO } }
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() && self.z.is_finite() }
}
impl Add for V3 { type Output = V3; fn add(self, o: V3) -> V3 { v3(self.x + o.x, self.y + o.y, self.z + o.z) } }
impl Sub for V3 { type Output = V3; fn sub(self, o: V3) -> V3 { v3(self.x - o.x, self.y - o.y, self.z - o.z) } }
impl Mul<f32> for V3 { type Output = V3; fn mul(self, k: f32) -> V3 { v3(self.x * k, self.y * k, self.z * k) } }
impl Neg for V3 { type Output = V3; fn neg(self) -> V3 { v3(-self.x, -self.y, -self.z) } }

#[derive(Clone, Copy, Debug)]
pub struct Q { pub w: f32, pub x: f32, pub y: f32, pub z: f32 }
impl Q {
    pub const IDENT: Q = Q { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
    pub fn axis_angle(axis: V3, a: f32) -> Q {
        let n = axis.norm(); let s = (a * 0.5).sin();
        Q { w: (a * 0.5).cos(), x: n.x * s, y: n.y * s, z: n.z * s }
    }
    pub fn mul(self, o: Q) -> Q {
        Q { w: self.w * o.w - self.x * o.x - self.y * o.y - self.z * o.z,
            x: self.w * o.x + self.x * o.w + self.y * o.z - self.z * o.y,
            y: self.w * o.y - self.x * o.z + self.y * o.w + self.z * o.x,
            z: self.w * o.z + self.x * o.y - self.y * o.x + self.z * o.w }
    }
    pub fn conj(self) -> Q { Q { w: self.w, x: -self.x, y: -self.y, z: -self.z } }
    pub fn normalize(self) -> Q {
        let l = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if l < 1e-9 { Q::IDENT } else { Q { w: self.w / l, x: self.x / l, y: self.y / l, z: self.z / l } }
    }
    pub fn rotate(self, v: V3) -> V3 {
        let u = v3(self.x, self.y, self.z); let t = u.cross(v) * 2.0;
        v + t * self.w + u.cross(t)
    }
    /// Advances the orientation by a world-space angular velocity.
    pub fn integrate(self, omega: V3, dt: f32) -> Q {
        let dq = Q { w: 0.0, x: omega.x, y: omega.y, z: omega.z }.mul(self);
        Q { w: self.w + 0.5 * dt * dq.w, x: self.x + 0.5 * dt * dq.x, y: self.y + 0.5 * dt * dq.y, z: self.z + 0.5 * dt * dq.z }.normalize()
    }
}

pub type Mat4 = [[f32; 4]; 4];
pub fn mat_mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut r = [[0.0; 4]; 4];
    for c in 0..4 { for row in 0..4 { r[c][row] = (0..4).map(|k| a[k][row] * b[c][k]).sum(); } }
    r
}
pub fn transform(m: Mat4, v: [f32; 4]) -> [f32; 4] {
    let mut r = [0.0; 4];
    for row in 0..4 { r[row] = (0..4).map(|k| m[k][row] * v[k]).sum(); }
    r
}
pub fn flatten(m: Mat4) -> [f32; 16] { let mut o = [0.0; 16]; for c in 0..4 { for r in 0..4 { o[c * 4 + r] = m[c][r]; } } o }
pub fn look_at(eye: V3, target: V3, up: V3) -> Mat4 {
    let f = (target - eye).norm(); let s = f.cross(up).norm(); let u = s.cross(f);
    [[s.x, u.x, -f.x, 0.0], [s.y, u.y, -f.y, 0.0], [s.z, u.z, -f.z, 0.0], [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0]]
}
pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fovy * 0.5).tan();
    let mut m = [[0.0; 4]; 4];
    m[0][0] = f / aspect; m[1][1] = f; m[2][2] = far / (near - far); m[2][3] = -1.0; m[3][2] = near * far / (near - far);
    m
}
pub fn ortho(l: f32, r: f32, b: f32, t: f32, n: f32, f: f32) -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    m[0][0] = 2.0 / (r - l); m[1][1] = 2.0 / (t - b); m[2][2] = 1.0 / (n - f);
    m[3][0] = -(r + l) / (r - l); m[3][1] = -(t + b) / (t - b); m[3][2] = n / (n - f); m[3][3] = 1.0;
    m
}
/// General 4x4 inverse (cofactor expansion); returns identity for singular input.
pub fn inverse(m: Mat4) -> Mat4 {
    let a = flatten(m); // a[c*4+r]
    let mut inv = [0.0f32; 16];
    inv[0] = a[5]*a[10]*a[15] - a[5]*a[11]*a[14] - a[9]*a[6]*a[15] + a[9]*a[7]*a[14] + a[13]*a[6]*a[11] - a[13]*a[7]*a[10];
    inv[4] = -a[4]*a[10]*a[15] + a[4]*a[11]*a[14] + a[8]*a[6]*a[15] - a[8]*a[7]*a[14] - a[12]*a[6]*a[11] + a[12]*a[7]*a[10];
    inv[8] = a[4]*a[9]*a[15] - a[4]*a[11]*a[13] - a[8]*a[5]*a[15] + a[8]*a[7]*a[13] + a[12]*a[5]*a[11] - a[12]*a[7]*a[9];
    inv[12] = -a[4]*a[9]*a[14] + a[4]*a[10]*a[13] + a[8]*a[5]*a[14] - a[8]*a[6]*a[13] - a[12]*a[5]*a[10] + a[12]*a[6]*a[9];
    inv[1] = -a[1]*a[10]*a[15] + a[1]*a[11]*a[14] + a[9]*a[2]*a[15] - a[9]*a[3]*a[14] - a[13]*a[2]*a[11] + a[13]*a[3]*a[10];
    inv[5] = a[0]*a[10]*a[15] - a[0]*a[11]*a[14] - a[8]*a[2]*a[15] + a[8]*a[3]*a[14] + a[12]*a[2]*a[11] - a[12]*a[3]*a[10];
    inv[9] = -a[0]*a[9]*a[15] + a[0]*a[11]*a[13] + a[8]*a[1]*a[15] - a[8]*a[3]*a[13] - a[12]*a[1]*a[11] + a[12]*a[3]*a[9];
    inv[13] = a[0]*a[9]*a[14] - a[0]*a[10]*a[13] - a[8]*a[1]*a[14] + a[8]*a[2]*a[13] + a[12]*a[1]*a[10] - a[12]*a[2]*a[9];
    inv[2] = a[1]*a[6]*a[15] - a[1]*a[7]*a[14] - a[5]*a[2]*a[15] + a[5]*a[3]*a[14] + a[13]*a[2]*a[7] - a[13]*a[3]*a[6];
    inv[6] = -a[0]*a[6]*a[15] + a[0]*a[7]*a[14] + a[4]*a[2]*a[15] - a[4]*a[3]*a[14] - a[12]*a[2]*a[7] + a[12]*a[3]*a[6];
    inv[10] = a[0]*a[5]*a[15] - a[0]*a[7]*a[13] - a[4]*a[1]*a[15] + a[4]*a[3]*a[13] + a[12]*a[1]*a[7] - a[12]*a[3]*a[5];
    inv[14] = -a[0]*a[5]*a[14] + a[0]*a[6]*a[13] + a[4]*a[1]*a[14] - a[4]*a[2]*a[13] - a[12]*a[1]*a[6] + a[12]*a[2]*a[5];
    inv[3] = -a[1]*a[6]*a[11] + a[1]*a[7]*a[10] + a[5]*a[2]*a[11] - a[5]*a[3]*a[10] - a[9]*a[2]*a[7] + a[9]*a[3]*a[6];
    inv[7] = a[0]*a[6]*a[11] - a[0]*a[7]*a[10] - a[4]*a[2]*a[11] + a[4]*a[3]*a[10] + a[8]*a[2]*a[7] - a[8]*a[3]*a[6];
    inv[11] = -a[0]*a[5]*a[11] + a[0]*a[7]*a[9] + a[4]*a[1]*a[11] - a[4]*a[3]*a[9] - a[8]*a[1]*a[7] + a[8]*a[3]*a[5];
    inv[15] = a[0]*a[5]*a[10] - a[0]*a[6]*a[9] - a[4]*a[1]*a[10] + a[4]*a[2]*a[9] + a[8]*a[1]*a[6] - a[8]*a[2]*a[5];
    let det = a[0]*inv[0] + a[1]*inv[4] + a[2]*inv[8] + a[3]*inv[12];
    let mut id = [[0.0; 4]; 4]; for i in 0..4 { id[i][i] = 1.0; }
    if det.abs() < 1e-20 { return id; }
    let d = 1.0 / det; let mut r = [[0.0; 4]; 4];
    for c in 0..4 { for row in 0..4 { r[c][row] = inv[c * 4 + row] * d; } }
    r
}

#[cfg(test)]
#[path = "math_tests.rs"]
mod tests;
