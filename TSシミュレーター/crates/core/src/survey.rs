//! 第2部：観測の計算（座標の変換、視準のズレ、BS・FSからの座標計算）。
//!
//! 測量の座標：X＝北、Y＝東、H＝標高（mm）。方向角は北から時計回り。
//! 3Dの世界座標：x＝東、y＝北、z＝上（mm）。原点は据える測点（KNOWN_POINTS[STATION_INDEX]）。

use core::f64::consts::PI;

use crate::config::{
    KnownPoint, BACKSIGHT_INDEX, FS_ACTUAL_OFFSET_MM, FS_DESIGN, KNOWN_POINTS, STATION_INDEX,
    TARGET_HEIGHT_MM,
};
use crate::math::Vec3;

/// 測量の座標 → 3Dの世界座標
pub fn survey_to_world(x_mm: f64, y_mm: f64, h_mm: f64) -> Vec3 {
    let st = &KNOWN_POINTS[STATION_INDEX];
    Vec3::new(y_mm - st.y_mm, x_mm - st.x_mm, h_mm - st.h_mm)
}

/// 既知点の地面の位置（世界座標）
pub fn point_world(p: &KnownPoint) -> Vec3 {
    survey_to_world(p.x_mm, p.y_mm, p.h_mm)
}

/// BSのターゲットの中心（世界座標）
pub fn backsight_target_world() -> Vec3 {
    point_world(&KNOWN_POINTS[BACKSIGHT_INDEX]) + Vec3::new(0.0, 0.0, TARGET_HEIGHT_MM)
}

/// FSのプリズムが実際に立っている地面の点（世界座標）
pub fn foresight_ground_world() -> Vec3 {
    survey_to_world(
        FS_DESIGN.x_mm + FS_ACTUAL_OFFSET_MM[0],
        FS_DESIGN.y_mm + FS_ACTUAL_OFFSET_MM[1],
        FS_DESIGN.h_mm,
    )
}

/// FSのプリズムの中心（世界座標）
pub fn foresight_target_world() -> Vec3 {
    foresight_ground_world() + Vec3::new(0.0, 0.0, TARGET_HEIGHT_MM)
}

/// -π〜π に直す
pub fn wrap_pi(a: f64) -> f64 {
    let x = (a + PI).rem_euclid(2.0 * PI) - PI;
    if x <= -PI {
        x + 2.0 * PI
    } else {
        x
    }
}

/// 望遠鏡の向き（方向角 hz：北から時計回り、天頂角 zenith）→ 視準線の単位ベクトル（世界座標）
pub fn line_of_sight(hz: f64, zenith: f64) -> Vec3 {
    let (sz, cz) = zenith.sin_cos();
    let (sh, ch) = hz.sin_cos();
    Vec3::new(sz * sh, sz * ch, cz)
}

/// from から to を見たときの（方向角, 天頂角, 斜距離）
pub fn direction_to(from: Vec3, to: Vec3) -> (f64, f64, f64) {
    let d = to - from;
    let horiz = d.x.hypot(d.y);
    let az = d.x.atan2(d.y).rem_euclid(2.0 * PI);
    let zenith = horiz.atan2(d.z);
    (az, zenith, d.norm())
}

/// 2点間の方向角（測量の座標から）
pub fn azimuth_between(a: &KnownPoint, b: &KnownPoint) -> f64 {
    (b.y_mm - a.y_mm)
        .atan2(b.x_mm - a.x_mm)
        .rem_euclid(2.0 * PI)
}

/// 2点間の水平距離（測量の座標から）
pub fn distance_between(a: &KnownPoint, b: &KnownPoint) -> f64 {
    (b.y_mm - a.y_mm).hypot(b.x_mm - a.x_mm)
}

/// 視準のズレ（十字線とターゲット中心の角度差、rad）。
/// 戻り値：（水平方向のズレ［視野の中での角度］, 鉛直方向のズレ）。正＝ターゲットが十字線の右・上にある。
pub fn sight_deviation(hz: f64, zenith: f64, eye: Vec3, target: Vec3) -> (f64, f64) {
    let (az, z, _) = direction_to(eye, target);
    let dh = wrap_pi(az - hz) * zenith.sin();
    let dv = zenith - z;
    (dh, dv)
}

/// BSの観測記録
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BacksightRecord {
    /// 観測した水平角（観測時の読み）
    pub reading: f64,
    pub zenith: f64,
    pub slope_mm: f64,
    pub horizontal_mm: f64,
    /// 座標から計算したBSの方向角
    pub azimuth: f64,
    /// 座標から計算したBSまでの水平距離
    pub coord_distance_mm: f64,
    /// 水平角の読みに足すと方向角になる量
    pub orientation: f64,
}

/// FSの観測結果
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForesightRecord {
    pub azimuth: f64,
    pub zenith: f64,
    pub slope_mm: f64,
    pub horizontal_mm: f64,
    pub x_mm: f64,
    pub y_mm: f64,
    pub h_mm: f64,
    /// 設計値との差（観測 − 設計）
    pub dx_mm: f64,
    pub dy_mm: f64,
    pub dh_mm: f64,
}

/// BSの観測から方向角の基準を決める
pub fn backsight_record(
    station: &KnownPoint,
    backsight: &KnownPoint,
    reading: f64,
    zenith: f64,
    slope_mm: f64,
) -> BacksightRecord {
    let azimuth = azimuth_between(station, backsight);
    BacksightRecord {
        reading,
        zenith,
        slope_mm,
        horizontal_mm: slope_mm * zenith.sin(),
        azimuth,
        coord_distance_mm: distance_between(station, backsight),
        orientation: wrap_pi(azimuth - reading),
    }
}

/// FSの観測から座標を計算する。
/// `azimuth` は方向角（BSでセット済みの水平角）、`instrument_height_mm` は入力した器械高。
pub fn foresight_record(
    station: &KnownPoint,
    instrument_height_mm: f64,
    azimuth: f64,
    zenith: f64,
    slope_mm: f64,
    target_height_mm: f64,
    design: &KnownPoint,
) -> ForesightRecord {
    let horizontal = slope_mm * zenith.sin();
    let vertical = slope_mm * zenith.cos();
    let x = station.x_mm + horizontal * azimuth.cos();
    let y = station.y_mm + horizontal * azimuth.sin();
    let h = station.h_mm + instrument_height_mm + vertical - target_height_mm;
    ForesightRecord {
        azimuth: azimuth.rem_euclid(2.0 * PI),
        zenith,
        slope_mm,
        horizontal_mm: horizontal,
        x_mm: x,
        y_mm: y,
        h_mm: h,
        dx_mm: x - design.x_mm,
        dy_mm: y - design.y_mm,
        dh_mm: h - design.h_mm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angle::deg;

    #[test]
    fn world_origin_is_station() {
        let p = point_world(&KNOWN_POINTS[STATION_INDEX]);
        assert!(p.norm() < 1e-9);
    }

    #[test]
    fn azimuth_convention_north_clockwise() {
        let a = KnownPoint {
            name: "a",
            x_mm: 0.0,
            y_mm: 0.0,
            h_mm: 0.0,
        };
        let east = KnownPoint {
            name: "e",
            x_mm: 0.0,
            y_mm: 10.0,
            h_mm: 0.0,
        };
        let south = KnownPoint {
            name: "s",
            x_mm: -10.0,
            y_mm: 0.0,
            h_mm: 0.0,
        };
        assert!((azimuth_between(&a, &east) - deg(90.0)).abs() < 1e-12);
        assert!((azimuth_between(&a, &south) - deg(180.0)).abs() < 1e-12);
    }

    #[test]
    fn line_of_sight_matches_direction_to() {
        let from = Vec3::new(1.0, 2.0, 1500.0);
        let to = Vec3::new(20_000.0, -5_000.0, 1400.0);
        let (az, z, _) = direction_to(from, to);
        let d = line_of_sight(az, z);
        let expect = (to - from).normalized().unwrap_or(Vec3::ZERO);
        assert!((d - expect).norm() < 1e-12);
        let (dh, dv) = sight_deviation(az, z, from, to);
        assert!(dh.abs() < 1e-12 && dv.abs() < 1e-12);
    }
}
