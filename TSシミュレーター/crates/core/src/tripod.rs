//! 三脚：足先の位置・脚の長さから、三脚頭の位置と傾きを求める。
//!
//! 三脚頭は剛体で、3本の脚は三脚頭の付け根（ヒンジ）で1方向にだけ回る。
//! 足先は地面に置かれた点。各脚について
//! - 足先〜付け根の長さ ＝ 脚の長さ
//! - 脚はヒンジの回転面（三脚頭の法線と、その脚の放射方向が作る面）の中にある
//!
//! の2つの条件があり、3本で6つ。三脚頭の未知数（位置3＋回転3）と同じ数なので、
//! ニュートン法で解く。この形だと、脚の長さを変えても三脚頭の中心軸はほぼ同じ地面の点を通る
//! （SPEC 6.4「三脚の足を伸縮すると、三脚頭が傾き、円形気泡が動く。求心点はあまり動かない」）。

use crate::config::{
    HEAD_TILT_MAX, LEG_AZIMUTHS, LEG_HINGE_RADIUS_MM, LEG_INCLINATION_MAX, LEG_INCLINATION_MIN,
};
use crate::math::{solve_linear, Mat3, Vec3};

/// 三脚頭の姿勢。`rot` は三脚頭の座標（x,y＝頭の面、z＝法線）から世界座標への回転。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadPose {
    /// 脚の付け根の面の中心（世界座標、mm）
    pub center: Vec3,
    pub rot: Mat3,
}

impl HeadPose {
    /// 三脚頭の法線（上向き）
    pub fn normal(&self) -> Vec3 {
        self.rot.col(2)
    }

    /// 三脚頭の傾き（鉛直からの角度、rad）
    pub fn tilt(&self) -> f64 {
        let n = self.normal();
        (n.x.hypot(n.y)).atan2(n.z)
    }

    /// 三脚頭の座標の点を世界座標に直す
    pub fn to_world(&self, local: Vec3) -> Vec3 {
        self.center + self.rot.mul_vec(local)
    }
}

/// 脚 i の放射方向（三脚頭の座標）
pub fn leg_radial(i: usize) -> Vec3 {
    let a = LEG_AZIMUTHS[i.min(2)];
    Vec3::new(a.cos(), a.sin(), 0.0)
}

/// 脚 i の付け根の位置（三脚頭の座標）
pub fn hinge_local(i: usize) -> Vec3 {
    leg_radial(i) * LEG_HINGE_RADIUS_MM
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripodError {
    /// 解が見つからない（脚が届かない等）
    NoSolution,
    /// 解はあるが、三脚として不自然な形（脚が寝すぎ・立ちすぎ、頭が傾きすぎ）
    BadShape,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tripod {
    /// 足先の位置（世界座標、mm）。z は地面からの高さ（踏み込むと沈んでマイナス）
    pub feet: [Vec3; 3],
    /// 脚の長さ（足先〜付け根、mm）
    pub leg_len: [f64; 3],
    /// 踏み込み済みか
    pub planted: [bool; 3],
    pub head: HeadPose,
}

impl Tripod {
    /// 足先と脚の長さから三脚を作る
    pub fn new(feet: [Vec3; 3], leg_len: [f64; 3]) -> Result<Tripod, TripodError> {
        let guess = initial_guess(&feet, &leg_len);
        let head = solve_head(&feet, &leg_len, &guess)?;
        check_shape(&feet, &head)?;
        Ok(Tripod {
            feet,
            leg_len,
            planted: [false; 3],
            head,
        })
    }

    /// 足先・脚の長さを変えてみて、三脚として成り立てば反映する。成り立たなければ何も変えない。
    pub fn try_set(&mut self, feet: [Vec3; 3], leg_len: [f64; 3]) -> Result<(), TripodError> {
        let head = solve_head(&feet, &leg_len, &self.head)?;
        check_shape(&feet, &head)?;
        self.feet = feet;
        self.leg_len = leg_len;
        self.head = head;
        Ok(())
    }

    /// 脚の付け根の位置（世界座標）
    pub fn hinge_points(&self) -> [Vec3; 3] {
        [0, 1, 2].map(|i| self.head.to_world(hinge_local(i)))
    }

    pub fn planted_count(&self) -> usize {
        self.planted.iter().filter(|p| **p).count()
    }
}

fn initial_guess(feet: &[Vec3; 3], leg_len: &[f64; 3]) -> HeadPose {
    let c = (feet[0] + feet[1] + feet[2]) * (1.0 / 3.0);
    let spread: f64 = feet
        .iter()
        .map(|f| (f.x - c.x).hypot(f.y - c.y))
        .sum::<f64>()
        / 3.0;
    let horiz = (spread - LEG_HINGE_RADIUS_MM).max(0.0);
    let len = leg_len.iter().sum::<f64>() / 3.0;
    let h = (len * len - horiz * horiz).max(1.0).sqrt();
    HeadPose {
        center: Vec3::new(c.x, c.y, c.z + h),
        rot: Mat3::IDENTITY,
    }
}

/// 6つの条件の残差
fn residual(feet: &[Vec3; 3], leg_len: &[f64; 3], center: Vec3, rot: &Mat3) -> [f64; 6] {
    let mut r = [0.0; 6];
    for i in 0..3 {
        let p = center + rot.mul_vec(hinge_local(i));
        let d = feet[i] - p;
        r[2 * i] = d.norm() - leg_len[i];
        let tangent = rot.mul_vec(Vec3::Z.cross(leg_radial(i)));
        r[2 * i + 1] = d.dot(tangent);
    }
    r
}

fn perturbed(center: Vec3, rot: &Mat3, k: usize, h: f64) -> (Vec3, Mat3) {
    match k {
        0 => (center + Vec3::new(h, 0.0, 0.0), *rot),
        1 => (center + Vec3::new(0.0, h, 0.0), *rot),
        2 => (center + Vec3::new(0.0, 0.0, h), *rot),
        3 => (
            center,
            Mat3::from_rotvec(Vec3::new(h, 0.0, 0.0)).mul_mat(rot),
        ),
        4 => (
            center,
            Mat3::from_rotvec(Vec3::new(0.0, h, 0.0)).mul_mat(rot),
        ),
        _ => (
            center,
            Mat3::from_rotvec(Vec3::new(0.0, 0.0, h)).mul_mat(rot),
        ),
    }
}

fn max_abs(r: &[f64; 6]) -> f64 {
    r.iter().fold(0.0_f64, |m, v| m.max(v.abs()))
}

/// 三脚頭の姿勢をニュートン法で解く。`guess` は直前の姿勢（近い解に収束させるため）。
#[allow(clippy::needless_range_loop)]
pub fn solve_head(
    feet: &[Vec3; 3],
    leg_len: &[f64; 3],
    guess: &HeadPose,
) -> Result<HeadPose, TripodError> {
    let mut center = guess.center;
    let mut rot = guess.rot;
    for _ in 0..50 {
        let r0 = residual(feet, leg_len, center, &rot);
        if !r0.iter().all(|v| v.is_finite()) {
            return Err(TripodError::NoSolution);
        }
        if max_abs(&r0) < 1e-9 {
            return Ok(HeadPose { center, rot });
        }
        // 中心差分でヤコビアンを作る（位置は mm、回転は rad で微小量を変える）
        let mut jac = [[0.0; 6]; 6];
        for k in 0..6 {
            let h = if k < 3 { 1e-4 } else { 1e-7 };
            let (cp, rp) = perturbed(center, &rot, k, h);
            let (cm, rm) = perturbed(center, &rot, k, -h);
            let rp = residual(feet, leg_len, cp, &rp);
            let rm = residual(feet, leg_len, cm, &rm);
            for row in 0..6 {
                jac[row][k] = (rp[row] - rm[row]) / (2.0 * h);
            }
        }
        let rhs = r0.map(|v| -v);
        let dx = solve_linear(jac, rhs).ok_or(TripodError::NoSolution)?;
        // 1回で大きく動きすぎないように制限する
        let mut step_pos = Vec3::new(dx[0], dx[1], dx[2]);
        let mut step_rot = Vec3::new(dx[3], dx[4], dx[5]);
        let pos_len = step_pos.norm();
        if pos_len > 200.0 {
            step_pos = step_pos * (200.0 / pos_len);
        }
        let rot_len = step_rot.norm();
        if rot_len > 0.2 {
            step_rot = step_rot * (0.2 / rot_len);
        }
        center = center + step_pos;
        rot = Mat3::from_rotvec(step_rot)
            .mul_mat(&rot)
            .orthonormalized()
            .ok_or(TripodError::NoSolution)?;
    }
    let r = residual(feet, leg_len, center, &rot);
    if max_abs(&r) < 1e-6 {
        Ok(HeadPose { center, rot })
    } else {
        Err(TripodError::NoSolution)
    }
}

fn check_shape(feet: &[Vec3; 3], head: &HeadPose) -> Result<(), TripodError> {
    if head.normal().z <= 0.0 || head.tilt() > HEAD_TILT_MAX {
        return Err(TripodError::BadShape);
    }
    for (i, foot) in feet.iter().enumerate() {
        let p = head.to_world(hinge_local(i));
        let d = p - *foot;
        if d.z <= 100.0 {
            return Err(TripodError::BadShape);
        }
        let incl = (d.x.hypot(d.y)).atan2(d.z);
        if !(LEG_INCLINATION_MIN..=LEG_INCLINATION_MAX).contains(&incl) {
            return Err(TripodError::BadShape);
        }
        // 足先は付け根より外側（放射方向）にあること
        let outward = head.rot.mul_vec(leg_radial(i));
        if (*foot - p).dot(outward) <= 0.0 {
            return Err(TripodError::BadShape);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angle::arcmin;

    fn symmetric_feet(cx: f64, cy: f64, r: f64) -> [Vec3; 3] {
        [0, 1, 2].map(|i| {
            let a = LEG_AZIMUTHS[i];
            Vec3::new(cx + r * a.cos(), cy + r * a.sin(), 0.0)
        })
    }

    fn ground_point_of_axis(head: &HeadPose) -> (f64, f64) {
        let n = head.normal();
        let t = head.center.z / n.z;
        let p = head.center - n * t;
        (p.x, p.y)
    }

    #[test]
    fn symmetric_tripod_is_level_and_centered() -> Result<(), TripodError> {
        let t = Tripod::new(symmetric_feet(0.0, 0.0, 650.0), [1380.0; 3])?;
        assert!(t.head.tilt() < 1e-9);
        assert!(t.head.center.x.abs() < 1e-6 && t.head.center.y.abs() < 1e-6);
        let expect_h = (1380.0_f64.powi(2) - (650.0_f64 - LEG_HINGE_RADIUS_MM).powi(2)).sqrt();
        assert!((t.head.center.z - expect_h).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn solution_satisfies_constraints() -> Result<(), TripodError> {
        let feet = [
            Vec3::new(10.0, 640.0, -3.0),
            Vec3::new(-590.0, -310.0, 0.0),
            Vec3::new(560.0, -350.0, -1.0),
        ];
        let lens = [1360.0, 1400.0, 1385.0];
        let t = Tripod::new(feet, lens)?;
        let r = residual(&feet, &lens, t.head.center, &t.head.rot);
        assert!(max_abs(&r) < 1e-8);
        Ok(())
    }

    #[test]
    fn extending_a_leg_tilts_head_but_axis_ground_point_barely_moves() -> Result<(), TripodError> {
        let feet = symmetric_feet(0.0, 0.0, 650.0);
        let mut t = Tripod::new(feet, [1380.0; 3])?;
        let before = ground_point_of_axis(&t.head);
        t.try_set(feet, [1400.0, 1380.0, 1380.0])?;
        let after = ground_point_of_axis(&t.head);
        // 20mm伸ばすと三脚頭ははっきり傾く（円形気泡が動く）
        assert!(t.head.tilt() > arcmin(30.0));
        // 中心軸と地面の交点はほとんど動かない
        let moved = (after.0 - before.0).hypot(after.1 - before.1);
        assert!(moved < 1.0, "moved {moved} mm");
        Ok(())
    }

    #[test]
    fn longer_leg_raises_its_side() -> Result<(), TripodError> {
        let feet = symmetric_feet(0.0, 0.0, 650.0);
        let mut t = Tripod::new(feet, [1380.0; 3])?;
        t.try_set(feet, [1400.0, 1380.0, 1380.0])?;
        // 脚1（奥＝北）が長くなると北側が高くなり、法線は南へ傾く
        assert!(t.head.normal().y < 0.0);
        Ok(())
    }

    #[test]
    fn impossible_shape_is_rejected_and_state_kept() -> Result<(), TripodError> {
        let feet = symmetric_feet(0.0, 0.0, 650.0);
        let mut t = Tripod::new(feet, [1380.0; 3])?;
        let before = t.clone();
        // 足を極端に広げる（脚が届かない）
        let far = symmetric_feet(0.0, 0.0, 2000.0);
        assert!(t.try_set(far, [1380.0; 3]).is_err());
        assert_eq!(t, before);
        Ok(())
    }
}
