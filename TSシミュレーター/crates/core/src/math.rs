//! 3次元のベクトルと回転行列（外部ライブラリを使わない最小限の実装）。
//!
//! 座標系：x = 東、y = 北、z = 上。単位は mm。

use core::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const Z: Vec3 = Vec3::new(0.0, 0.0, 1.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// 長さ1にする。長さがほぼ0なら None。
    pub fn normalized(self) -> Option<Vec3> {
        let n = self.norm();
        if n.is_finite() && n > 1e-12 {
            Some(self * (1.0 / n))
        } else {
            None
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// 3x3行列（行優先）。回転行列として使う。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    pub m: [[f64; 3]; 3],
}

impl Default for Mat3 {
    fn default() -> Self {
        Mat3::IDENTITY
    }
}

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3 {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub fn mul_vec(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }

    pub fn mul_mat(&self, o: &Mat3) -> Mat3 {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.m[i][k] * o.m[k][j]).sum();
            }
        }
        Mat3 { m: r }
    }

    pub fn transpose(&self) -> Mat3 {
        let m = &self.m;
        Mat3 {
            m: [
                [m[0][0], m[1][0], m[2][0]],
                [m[0][1], m[1][1], m[2][1]],
                [m[0][2], m[1][2], m[2][2]],
            ],
        }
    }

    /// 列ベクトル（この回転で各軸がどこを向くか）
    pub fn col(&self, j: usize) -> Vec3 {
        let j = j.min(2);
        Vec3::new(self.m[0][j], self.m[1][j], self.m[2][j])
    }

    /// 回転ベクトル（向き＝回転軸、長さ＝回転角[rad]）から回転行列を作る（ロドリゲスの公式）。
    pub fn from_rotvec(w: Vec3) -> Mat3 {
        let theta = w.norm();
        if theta < 1e-15 {
            return Mat3::IDENTITY;
        }
        let k = w * (1.0 / theta);
        let (s, c) = theta.sin_cos();
        let v = 1.0 - c;
        Mat3 {
            m: [
                [
                    c + k.x * k.x * v,
                    k.x * k.y * v - k.z * s,
                    k.x * k.z * v + k.y * s,
                ],
                [
                    k.y * k.x * v + k.z * s,
                    c + k.y * k.y * v,
                    k.y * k.z * v - k.x * s,
                ],
                [
                    k.z * k.x * v - k.y * s,
                    k.z * k.y * v + k.x * s,
                    c + k.z * k.z * v,
                ],
            ],
        }
    }

    /// 真上(0,0,1)を `n` に向ける最小の回転。`n` は長さ1であること。
    pub fn rotate_z_to(n: Vec3) -> Mat3 {
        let axis = Vec3::Z.cross(n);
        let s = axis.norm();
        let c = n.z.clamp(-1.0, 1.0);
        if s < 1e-15 {
            return Mat3::IDENTITY;
        }
        let angle = s.atan2(c);
        Mat3::from_rotvec(axis * (angle / s))
    }

    /// 数値誤差でずれた回転行列を直交化する（グラム・シュミット）。
    pub fn orthonormalized(&self) -> Option<Mat3> {
        let x = self.col(0).normalized()?;
        let y0 = self.col(1);
        let y = (y0 - x * x.dot(y0)).normalized()?;
        let z = x.cross(y);
        Some(Mat3 {
            m: [[x.x, y.x, z.x], [x.y, y.y, z.y], [x.z, y.z, z.z]],
        })
    }

    pub fn to_array(&self) -> [f64; 9] {
        let m = &self.m;
        [
            m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
        ]
    }
}

/// 連立一次方程式 A x = b をガウスの消去法（部分ピボット）で解く。解けなければ None。
#[allow(clippy::needless_range_loop)]
pub fn solve_linear<const N: usize>(mut a: [[f64; N]; N], mut b: [f64; N]) -> Option<[f64; N]> {
    for col in 0..N {
        let mut piv = col;
        for row in (col + 1)..N {
            if a[row][col].abs() > a[piv][col].abs() {
                piv = row;
            }
        }
        if !a[piv][col].is_finite() || a[piv][col].abs() < 1e-14 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        for row in (col + 1)..N {
            let f = a[row][col] / a[col][col];
            if f == 0.0 {
                continue;
            }
            for k in col..N {
                a[row][k] -= f * a[col][k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = [0.0; N];
    for i in (0..N).rev() {
        let mut s = b[i];
        for k in (i + 1)..N {
            s -= a[i][k] * x[k];
        }
        x[i] = s / a[i][i];
        if !x[i].is_finite() {
            return None;
        }
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3, eps: f64) -> bool {
        (a - b).norm() < eps
    }

    #[test]
    fn rotate_z_to_points_up_vector_to_target() {
        let n = Vec3::new(0.1, -0.05, 1.0).normalized().unwrap_or(Vec3::Z);
        let r = Mat3::rotate_z_to(n);
        assert!(close(r.mul_vec(Vec3::Z), n, 1e-12));
    }

    #[test]
    fn solve_linear_simple_system() {
        let a = [[2.0, 1.0], [1.0, 3.0]];
        let b = [3.0, 5.0];
        let x = solve_linear(a, b).unwrap_or([f64::NAN; 2]);
        assert!((x[0] - 0.8).abs() < 1e-12);
        assert!((x[1] - 1.4).abs() < 1e-12);
    }

    #[test]
    fn solve_linear_singular_returns_none() {
        let a = [[1.0, 2.0], [2.0, 4.0]];
        assert!(solve_linear(a, [1.0, 2.0]).is_none());
    }
}
