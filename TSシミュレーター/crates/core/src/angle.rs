//! 角度の単位変換と度分秒（DMS）の表示。
//!
//! 内部はすべてラジアン。度分秒は表示の直前だけで使う。

use core::f64::consts::PI;

/// 1ラジアンあたりの秒
pub const SEC_PER_RAD: f64 = 180.0 * 3600.0 / PI;

pub const fn deg(d: f64) -> f64 {
    d * PI / 180.0
}

pub const fn arcmin(m: f64) -> f64 {
    m * PI / (180.0 * 60.0)
}

pub const fn arcsec(s: f64) -> f64 {
    s * PI / (180.0 * 3600.0)
}

pub fn to_arcsec(rad: f64) -> f64 {
    rad * SEC_PER_RAD
}

/// 秒単位に丸めた値（四捨五入、0から遠い側へ）
pub fn round_arcsec(rad: f64) -> i64 {
    let s = to_arcsec(rad).round();
    if s.is_finite() {
        s as i64
    } else {
        0
    }
}

/// 度分秒（秒は整数に丸める）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Dms {
    pub negative: bool,
    pub deg: u32,
    pub min: u32,
    pub sec: u32,
}

impl Dms {
    /// 符号つきの度分秒。丸めた結果が0なら符号は＋。
    pub fn from_rad(rad: f64) -> Dms {
        if !rad.is_finite() {
            return Dms::default();
        }
        let total = round_arcsec(rad.abs()).max(0) as u64;
        Dms {
            negative: rad < 0.0 && total > 0,
            deg: (total / 3600) as u32,
            min: ((total % 3600) / 60) as u32,
            sec: (total % 60) as u32,
        }
    }

    /// 0〜360°に直した度分秒（水平角の表示用）
    pub fn from_rad_circle(rad: f64) -> Dms {
        if !rad.is_finite() {
            return Dms::default();
        }
        let full = 360 * 3600;
        let total = round_arcsec(rad.rem_euclid(2.0 * PI)).rem_euclid(full) as u64;
        Dms {
            negative: false,
            deg: (total / 3600) as u32,
            min: ((total % 3600) / 60) as u32,
            sec: (total % 60) as u32,
        }
    }

    /// 例：`+0°00′12″`、`−0°00′08″`（マイナスは「−」U+2212）
    pub fn to_signed_string(self) -> String {
        let sign = if self.negative { "\u{2212}" } else { "+" };
        format!("{}{}°{:02}′{:02}″", sign, self.deg, self.min, self.sec)
    }

    /// 例：`123°45′06″`
    pub fn to_unsigned_string(self) -> String {
        format!("{}°{:02}′{:02}″", self.deg, self.min, self.sec)
    }
}

/// 符号つきの度分秒の文字列。例：`+0°00′12″`
pub fn format_dms_signed(rad: f64) -> String {
    Dms::from_rad(rad).to_signed_string()
}

/// 0〜360°の度分秒の文字列
pub fn format_dms_circle(rad: f64) -> String {
    Dms::from_rad_circle(rad).to_unsigned_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dms_positive_small() {
        assert_eq!(format_dms_signed(arcsec(12.0)), "+0°00′12″");
    }

    #[test]
    fn dms_negative_uses_minus_sign() {
        assert_eq!(format_dms_signed(arcsec(-8.0)), "\u{2212}0°00′08″");
    }

    #[test]
    fn dms_rounding_carries_to_minutes_and_degrees() {
        // 59.6″ → 1′00″
        assert_eq!(format_dms_signed(arcsec(59.6)), "+0°01′00″");
        // 0°59′59.5″ → 1°00′00″
        assert_eq!(format_dms_signed(arcsec(59.0 * 60.0 + 59.5)), "+1°00′00″");
        // 負の側も同じく繰り上がる
        assert_eq!(format_dms_signed(-arcsec(3599.7)), "\u{2212}1°00′00″");
    }

    #[test]
    fn dms_rounding_down() {
        assert_eq!(format_dms_signed(arcsec(14.4)), "+0°00′14″");
        assert_eq!(format_dms_signed(arcsec(-14.4)), "\u{2212}0°00′14″");
    }

    #[test]
    fn dms_zero_and_tiny_negative_have_plus_sign() {
        assert_eq!(format_dms_signed(0.0), "+0°00′00″");
        assert_eq!(format_dms_signed(arcsec(-0.3)), "+0°00′00″");
    }

    #[test]
    fn dms_minutes_and_degrees() {
        assert_eq!(
            format_dms_signed(deg(1.0) + arcmin(2.0) + arcsec(3.0)),
            "+1°02′03″"
        );
        assert_eq!(format_dms_signed(-(arcmin(6.0))), "\u{2212}0°06′00″");
    }

    #[test]
    fn dms_circle_wraps_into_0_360() {
        assert_eq!(format_dms_circle(deg(-90.0)), "270°00′00″");
        assert_eq!(format_dms_circle(deg(360.0) - arcsec(0.2)), "0°00′00″");
        assert_eq!(
            format_dms_circle(deg(123.0) + arcmin(45.0) + arcsec(6.0)),
            "123°45′06″"
        );
    }

    #[test]
    fn non_finite_is_zero() {
        assert_eq!(Dms::from_rad(f64::NAN), Dms::default());
    }
}
