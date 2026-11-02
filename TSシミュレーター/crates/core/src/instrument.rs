//! 器械（整準台＋本体）：整準ネジ・定心桿・三脚頭上のずれ。
//!
//! 整準ネジ・円形気泡・電子気泡・レーザー求心は、すべて
//! 「三脚頭の姿勢」＋「この構造体の値」から計算する（SPEC 6.1）。
//!
//! 整準ネジの並び（上から見て）：A＝左手前、B＝右手前、C＝奥。
//! 操作者は手前（南）に立ち、親指はネジの手前側にかけて回す。

use crate::angle::{round_arcsec, to_arcsec};
use crate::config::{
    CIRCULAR_IN_CIRCLE_MM, CIRCULAR_SENS_ANGLE, CIRCULAR_SENS_MM, CIRCULAR_VIAL_MAX_MM,
    HEAD_TOP_OFFSET_MM, LCD_OUTER_RANGE_SEC, SCREW_AZIMUTHS, SCREW_PITCH_MM, SCREW_RADIUS_MM,
    SCREW_TURN_LIMIT, SLIDE_LIMIT_MM, SLIDE_TILT_COUPLING_PER_MM, TILT_TOLERANCE_SEC,
    TRIBRACH_PIVOT_HEIGHT_MM, TRUNNION_HEIGHT_MM,
};
use crate::math::{Mat3, Vec3};
use crate::tripod::HeadPose;

/// ネジにかけた親指を動かす向き（操作者から見て）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thumb {
    Left,
    Right,
}

impl Thumb {
    /// 親指の向き → ネジの回転量の符号（＋＝上から見て時計回り＝その角が上がる）。
    ///
    /// 親指はネジの手前側にかかっているので、親指を右へ動かすと、上から見て反時計回りに回る。
    /// 反時計回りはネジが下がる向き。
    pub fn turn_sign(self) -> f64 {
        match self {
            Thumb::Right => -1.0,
            Thumb::Left => 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentError {
    NotMounted,
    ClampTight,
    ScrewLimit,
    SlideLimit,
    BadIndex,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instrument {
    /// 三脚に載っているか
    pub mounted: bool,
    /// 定心桿を締めているか
    pub clamp_tight: bool,
    /// 三脚頭上でのずれ（三脚頭の座標、mm）
    pub slide: [f64; 2],
    /// 整準ネジの回転量 [回転]（＋＝上から見て時計回り＝上がる）
    pub screw_turns: [f64; 3],
    /// 器械を三脚頭上でずらしたことで生じた、わずかな傾き（三脚頭の座標の x,y 方向へ軸が倒れる量、rad）
    pub seat_tilt: [f64; 2],
}

impl Default for Instrument {
    fn default() -> Self {
        Instrument {
            mounted: true,
            clamp_tight: true,
            slide: [0.0; 2],
            screw_turns: [0.0; 3],
            seat_tilt: [0.0; 2],
        }
    }
}

/// 整準ネジ k の先端の位置（整準台の座標。高さは中立からの変化分）
pub fn screw_tip_local(k: usize, height: f64) -> Vec3 {
    let a = SCREW_AZIMUTHS[k.min(2)];
    Vec3::new(SCREW_RADIUS_MM * a.cos(), SCREW_RADIUS_MM * a.sin(), height)
}

impl Instrument {
    /// 整準ネジの先端の上下量 [mm]
    pub fn screw_heights(&self) -> [f64; 3] {
        self.screw_turns.map(|t| t * SCREW_PITCH_MM)
    }

    /// 整準ネジを回す。`turns` は回す量（正の数）。限界まで回したら止まる。
    pub fn turn_screw(
        &mut self,
        k: usize,
        thumb: Thumb,
        turns: f64,
    ) -> Result<(), InstrumentError> {
        if !self.mounted {
            return Err(InstrumentError::NotMounted);
        }
        let cur = *self.screw_turns.get(k).ok_or(InstrumentError::BadIndex)?;
        let next = cur + thumb.turn_sign() * turns.abs();
        let clamped = next.clamp(-SCREW_TURN_LIMIT, SCREW_TURN_LIMIT);
        if (clamped - cur).abs() < 1e-15 && turns.abs() > 0.0 {
            return Err(InstrumentError::ScrewLimit);
        }
        self.screw_turns[k] = clamped;
        Ok(())
    }

    /// 器械を三脚頭上でずらす（三脚頭の座標の向き、mm）。
    /// 定心桿を締めている間はずらせない。三脚頭が傾いていると、ずらした分だけ少し気泡が崩れる。
    pub fn slide_by(&mut self, d: [f64; 2], head_tilt: f64) -> Result<(), InstrumentError> {
        if !self.mounted {
            return Err(InstrumentError::NotMounted);
        }
        if self.clamp_tight {
            return Err(InstrumentError::ClampTight);
        }
        let nx = (self.slide[0] + d[0]).clamp(-SLIDE_LIMIT_MM, SLIDE_LIMIT_MM);
        let ny = (self.slide[1] + d[1]).clamp(-SLIDE_LIMIT_MM, SLIDE_LIMIT_MM);
        let moved = [nx - self.slide[0], ny - self.slide[1]];
        if moved[0].abs() < 1e-12 && moved[1].abs() < 1e-12 {
            if d[0].abs() > 0.0 || d[1].abs() > 0.0 {
                return Err(InstrumentError::SlideLimit);
            }
            return Ok(());
        }
        self.slide = [nx, ny];
        let k = SLIDE_TILT_COUPLING_PER_MM * head_tilt.abs();
        self.seat_tilt[0] += moved[0] * k;
        self.seat_tilt[1] += moved[1] * k;
        Ok(())
    }

    /// 器械の鉛直軸の向き（三脚頭の座標）。整準ネジ3本の先端が作る面の法線。
    pub fn axis_local(&self) -> Option<Vec3> {
        let h = self.screw_heights();
        let a = screw_tip_local(0, h[0]);
        let b = screw_tip_local(1, h[1]);
        let c = screw_tip_local(2, h[2]);
        let n = (b - a).cross(c - a).normalized()?;
        let n = if n.z < 0.0 { -n } else { n };
        (n + Vec3::new(self.seat_tilt[0], self.seat_tilt[1], 0.0)).normalized()
    }

    /// 回転の中心（整準ネジの先端の面と鉛直軸の交点。三脚頭の座標）
    pub fn pivot_local(&self) -> Vec3 {
        let h = self.screw_heights();
        Vec3::new(
            self.slide[0],
            self.slide[1],
            HEAD_TOP_OFFSET_MM + TRIBRACH_PIVOT_HEIGHT_MM + (h[0] + h[1] + h[2]) / 3.0,
        )
    }

    /// 世界座標での器械の姿勢。載っていなければ None。
    pub fn pose(&self, head: &HeadPose) -> Option<InstrumentPose> {
        if !self.mounted {
            return None;
        }
        let n = self.axis_local()?;
        let rot = head.rot.mul_mat(&Mat3::rotate_z_to(n));
        Some(InstrumentPose {
            pivot: head.to_world(self.pivot_local()),
            rot,
        })
    }
}

/// 器械の姿勢（世界座標）。`rot` の x＝A→B の向き（電子気泡のX）、y＝C の側（電子気泡のY）、z＝鉛直軸。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstrumentPose {
    pub pivot: Vec3,
    pub rot: Mat3,
}

impl InstrumentPose {
    pub fn axis(&self) -> Vec3 {
        self.rot.col(2)
    }

    /// 器械から見た「真上」の向き。気泡はこの向き（高い側）へ動く。
    pub fn up_in_instrument(&self) -> Vec3 {
        self.rot.transpose().mul_vec(Vec3::Z)
    }

    /// 電子気泡の X・Y（rad）。正の値＝点が＋X（右）・＋Y（奥）側にある。
    pub fn tilt_xy(&self) -> (f64, f64) {
        let g = self.up_in_instrument();
        (g.x.atan2(g.z), g.y.atan2(g.z))
    }

    /// 鉛直からの傾き（rad）
    pub fn tilt(&self) -> f64 {
        let g = self.up_in_instrument();
        (g.x.hypot(g.y)).atan2(g.z)
    }

    /// レーザー求心点：鉛直軸を下に延ばして地面（z=0）と交わる点
    pub fn laser_point(&self) -> Option<Vec3> {
        let a = self.axis();
        if a.z < 0.5 {
            return None;
        }
        let t = self.pivot.z / a.z;
        let p = self.pivot - a * t;
        if p.is_finite() {
            Some(Vec3::new(p.x, p.y, 0.0))
        } else {
            None
        }
    }

    /// 望遠鏡の回転軸の中心（器械高の基準点）
    pub fn trunnion(&self) -> Vec3 {
        self.pivot + self.axis() * TRUNNION_HEIGHT_MM
    }

    /// 器械高（測点から望遠鏡の回転軸までの高さ、mm）
    pub fn instrument_height(&self) -> f64 {
        self.trunnion().z
    }
}

/// 円形気泡の状態
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircularBubble {
    /// 中心からの気泡のずれ（器械の座標、mm）。気泡管の範囲で止める
    pub offset_mm: [f64; 2],
    /// 気泡管の範囲を1としたときのずれ（表示用）
    pub norm: [f64; 2],
    /// 止める前のずれの大きさ [mm]
    pub raw_mm: f64,
    /// 中心の円に入っているか
    pub in_circle: bool,
}

impl CircularBubble {
    pub fn from_pose(pose: &InstrumentPose) -> CircularBubble {
        let g = pose.up_in_instrument();
        let tilt = (g.x.hypot(g.y)).atan2(g.z);
        let raw = tilt / CIRCULAR_SENS_ANGLE * CIRCULAR_SENS_MM;
        let dir = Vec3::new(g.x, g.y, 0.0).normalized().unwrap_or(Vec3::ZERO);
        let shown = raw.min(CIRCULAR_VIAL_MAX_MM);
        CircularBubble {
            offset_mm: [dir.x * shown, dir.y * shown],
            norm: [
                dir.x * shown / CIRCULAR_VIAL_MAX_MM,
                dir.y * shown / CIRCULAR_VIAL_MAX_MM,
            ],
            raw_mm: raw,
            in_circle: raw <= CIRCULAR_IN_CIRCLE_MM,
        }
    }
}

/// 電子気泡（液晶の◎）の状態
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElectronicBubble {
    pub x: f64,
    pub y: f64,
    /// ◎の中の点の位置（外側の円の半径を1とする）。外側の円の上で止める
    pub dot: [f64; 2],
    /// X・Yとも許容内か（表示する秒の値で判定）
    pub ok: bool,
}

impl ElectronicBubble {
    pub fn from_pose(pose: &InstrumentPose) -> ElectronicBubble {
        let (x, y) = pose.tilt_xy();
        let mut dx = to_arcsec(x) / LCD_OUTER_RANGE_SEC;
        let mut dy = to_arcsec(y) / LCD_OUTER_RANGE_SEC;
        let r = dx.hypot(dy);
        if r > 1.0 {
            dx /= r;
            dy /= r;
        }
        ElectronicBubble {
            x,
            y,
            dot: [dx, dy],
            ok: tilt_within_tolerance(x) && tilt_within_tolerance(y),
        }
    }
}

/// 表示する秒（整数に丸めた値）が許容内か
pub fn tilt_within_tolerance(rad: f64) -> bool {
    (round_arcsec(rad).abs() as f64) <= TILT_TOLERANCE_SEC
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angle::arcsec;

    fn level_head(h: f64) -> HeadPose {
        HeadPose {
            center: Vec3::new(0.0, 0.0, h),
            rot: Mat3::IDENTITY,
        }
    }

    #[test]
    fn neutral_screws_on_level_head_is_level() {
        let inst = Instrument::default();
        let pose = inst.pose(&level_head(1250.0));
        let Some(pose) = pose else {
            unreachable!("器械は載っている");
        };
        let (x, y) = pose.tilt_xy();
        assert!(x.abs() < 1e-12 && y.abs() < 1e-12);
        let Some(l) = pose.laser_point() else {
            unreachable!("鉛直なので必ず地面と交わる");
        };
        assert!(l.x.abs() < 1e-9 && l.y.abs() < 1e-9);
    }

    #[test]
    fn single_screw_tilts_toward_that_screw() {
        // ネジCを上げる（親指を左へ）→ 奥（C側）が高くなる → 気泡は＋Y（奥）へ
        let mut inst = Instrument::default();
        assert!(inst.turn_screw(2, Thumb::Left, 0.5).is_ok());
        let Some(pose) = inst.pose(&level_head(1250.0)) else {
            unreachable!();
        };
        let (x, y) = pose.tilt_xy();
        assert!(y > 0.0);
        assert!(x.abs() < 1e-12);
        // 鉛直軸はC（北）と反対の南へ倒れる
        assert!(pose.axis().y < 0.0);
    }

    #[test]
    fn screw_limit_stops_turning() {
        let mut inst = Instrument::default();
        assert!(inst
            .turn_screw(0, Thumb::Left, SCREW_TURN_LIMIT + 3.0)
            .is_ok());
        assert_eq!(inst.screw_turns[0], SCREW_TURN_LIMIT);
        assert_eq!(
            inst.turn_screw(0, Thumb::Left, 0.1),
            Err(InstrumentError::ScrewLimit)
        );
    }

    #[test]
    fn tolerance_uses_rounded_seconds() {
        assert!(tilt_within_tolerance(arcsec(15.4)));
        assert!(!tilt_within_tolerance(arcsec(15.5)));
        assert!(tilt_within_tolerance(arcsec(-15.4)));
        assert!(!tilt_within_tolerance(arcsec(-15.5)));
    }
}
