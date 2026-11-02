//! 計算コア（ts_core）をTypeScriptに公開する薄い層。計算は書かない。
//!
//! やり取りは1フレームにつき1回：
//! - TS → Rust：操作の列（Float64Array。1件 = [コード, a, b, c] の4つ）
//! - Rust → TS：毎フレーム変わる数値の列（Float64Array。並びは `frame_layout_json()`）
//!
//! 手順・結果などの変化の少ない状態は、`ui_revision` が変わったときだけ `ui_state_json()` で受け取る。

use ts_core::angle::{arcsec, Dms};
use ts_core::config;
use ts_core::instrument::Thumb;
use ts_core::sim::{
    centering_display_mm, time_parts, Command, Mode, NoticeKind, Phase, Simulator, Snapshot, Speed,
    UiState,
};
use wasm_bindgen::prelude::*;

/// 操作1件あたりの数値の個数
const COMMAND_STRIDE: usize = 4;

/// 操作のコード（TS側は `command_codes_json()` から読む）
const COMMAND_CODES: [(&str, u32); 15] = [
    ("moveFreeLegs", 1),
    ("pressLeg", 2),
    ("changeLegLength", 3),
    ("turnScrew", 4),
    ("setClamp", 5),
    ("slideInstrument", 6),
    ("mountInstrument", 7),
    ("selectStep", 8),
    ("nextStep", 9),
    ("completeSetup", 10),
    ("continueSetup", 11),
    ("startObservation", 12),
    ("rotateTelescope", 13),
    ("setStation", 14),
    ("observe", 15),
];

fn index(v: f64) -> usize {
    if v.is_finite() && v >= 0.0 {
        v as usize
    } else {
        usize::MAX
    }
}

fn speed(v: f64) -> Speed {
    Speed::from_index(if v.is_finite() && v >= 0.0 {
        v as u32
    } else {
        2
    })
}

/// 数値の列から操作を1件読む
fn decode(code: f64, a: f64, b: f64, c: f64) -> Option<Command> {
    let cmd = match code as u32 {
        1 => Command::MoveFreeLegs {
            dir: [a, b],
            speed: speed(c),
        },
        2 => Command::PressLeg { leg: index(a) },
        3 => Command::ChangeLegLength {
            leg: index(a),
            extend: b > 0.0,
            speed: speed(c),
        },
        4 => Command::TurnScrew {
            screw: index(a),
            thumb: if b > 0.0 { Thumb::Right } else { Thumb::Left },
            speed: speed(c),
        },
        5 => Command::SetClamp { tight: a > 0.0 },
        6 => Command::SlideInstrument {
            dir: [a, b],
            speed: speed(c),
        },
        7 => Command::MountInstrument,
        8 => Command::SelectStep {
            step: index(a).min(255) as u8,
        },
        9 => Command::NextStep,
        10 => Command::CompleteSetup,
        11 => Command::ContinueSetup,
        12 => Command::StartObservation,
        13 => Command::RotateTelescope {
            dir: [a, b],
            speed: speed(c),
        },
        14 => Command::SetStation {
            station: index(a),
            backsight: index(b),
            height_m: c,
        },
        15 => Command::Observe,
        _ => return None,
    };
    Some(cmd)
}

fn b(v: bool) -> f64 {
    if v {
        1.0
    } else {
        0.0
    }
}

fn dms4(rad: f64, circle: bool) -> [f64; 4] {
    let d = if circle {
        Dms::from_rad_circle(rad)
    } else {
        Dms::from_rad(rad)
    };
    [
        if d.negative { -1.0 } else { 1.0 },
        d.deg as f64,
        d.min as f64,
        d.sec as f64,
    ]
}

const NAN3: [f64; 3] = [f64::NAN; 3];

/// 毎フレームの数値を、名前つきで順番に渡す（レイアウトと書き出しの両方に使う）
fn visit(s: &Snapshot, f: &mut dyn FnMut(&'static str, &[f64])) {
    let flat3 = |v: &[ts_core::math::Vec3; 3]| -> [f64; 9] {
        let mut o = [0.0; 9];
        for (i, p) in v.iter().enumerate() {
            o[i * 3..i * 3 + 3].copy_from_slice(&p.to_array());
        }
        o
    };
    f("feet", &flat3(&s.feet));
    f("hinges", &flat3(&s.hinges));
    f("legLen", &s.leg_len);
    f("planted", &s.planted.map(b));
    f("headCenter", &s.head_center.to_array());
    f("headRot", &s.head_rot.to_array());
    f("headTiltDeg", &[s.head_tilt_deg]);
    f("headOffsetMm", &[s.head_offset_mm]);
    f("mounted", &[b(s.mounted)]);
    f("clampTight", &[b(s.clamp_tight)]);
    f("slide", &s.slide);
    f("screwTurns", &s.screw_turns);
    f("screwAngles", &s.screw_angles);
    f("instValid", &[b(s.pose.is_some())]);
    f(
        "instPivot",
        &s.pose.map(|p| p.pivot.to_array()).unwrap_or(NAN3),
    );
    f(
        "instRot",
        &s.pose.map(|p| p.rot.to_array()).unwrap_or([f64::NAN; 9]),
    );
    f("laserValid", &[b(s.laser.is_some())]);
    f("laser", &s.laser.map(|l| l.to_array()).unwrap_or(NAN3));
    f("centering", &s.centering.unwrap_or([f64::NAN; 2]));
    f("centeringMm", &[s.centering_mm.unwrap_or(f64::NAN)]);
    f(
        "centeringDisplayMm",
        &[s.centering_mm.map(centering_display_mm).unwrap_or(f64::NAN)],
    );
    f("centeringOk", &[b(s.centering_ok)]);
    f("circValid", &[b(s.circular.is_some())]);
    f(
        "circOffsetMm",
        &s.circular.map(|c| c.offset_mm).unwrap_or([f64::NAN; 2]),
    );
    f(
        "circNorm",
        &s.circular.map(|c| c.norm).unwrap_or([f64::NAN; 2]),
    );
    f(
        "circIn",
        &[b(s.circular.map(|c| c.in_circle).unwrap_or(false))],
    );
    f("tiltValid", &[b(s.electronic.is_some())]);
    f("tiltX", &[s.electronic.map(|e| e.x).unwrap_or(f64::NAN)]);
    f("tiltY", &[s.electronic.map(|e| e.y).unwrap_or(f64::NAN)]);
    f(
        "lcdDot",
        &s.electronic.map(|e| e.dot).unwrap_or([f64::NAN; 2]),
    );
    f("tiltOk", &[b(s.electronic.map(|e| e.ok).unwrap_or(false))]);
    f(
        "tiltXDms",
        &dms4(s.electronic.map(|e| e.x).unwrap_or(0.0), false),
    );
    f(
        "tiltYDms",
        &dms4(s.electronic.map(|e| e.y).unwrap_or(0.0), false),
    );
    f(
        "instHeightMm",
        &[s.instrument_height_mm.unwrap_or(f64::NAN)],
    );
    f("instHeightM", &[s.instrument_height_m.unwrap_or(f64::NAN)]);
    f("elapsedMs", &[s.elapsed_ms]);
    let (m, sec, tenth) = time_parts(s.elapsed_ms);
    f("elapsedParts", &[m as f64, sec as f64, tenth as f64]);
    f("timerRunning", &[b(s.timer_running)]);
    f("loops", &[s.loops as f64]);
    f("telHz", &[s.telescope_hz]);
    f("telHzLocal", &[s.telescope_hz_local]);
    f("telZenith", &[s.telescope_zenith]);
    f("telDir", &s.telescope_dir.to_array());
    f("trunnionValid", &[b(s.trunnion.is_some())]);
    f(
        "trunnion",
        &s.trunnion.map(|t| t.to_array()).unwrap_or(NAN3),
    );
    f("hrDms", &dms4(s.hr_reading, true));
    f("vrDms", &dms4(s.telescope_zenith, true));
    f(
        "sightTarget",
        &[s.sight.map(|v| v.target as f64).unwrap_or(0.0)],
    );
    f(
        "sightDhSec",
        &[s.sight.map(|v| v.dh_sec).unwrap_or(f64::NAN)],
    );
    f(
        "sightDvSec",
        &[s.sight.map(|v| v.dv_sec).unwrap_or(f64::NAN)],
    );
    f(
        "sightDhDms",
        &s.sight
            .map(|v| dms4(arcsec(v.dh_sec), false))
            .unwrap_or([f64::NAN; 4]),
    );
    f(
        "sightDvDms",
        &s.sight
            .map(|v| dms4(arcsec(v.dv_sec), false))
            .unwrap_or([f64::NAN; 4]),
    );
    f("sightOk", &[b(s.sight.map(|v| v.ok).unwrap_or(false))]);
    f("bsTarget", &s.bs_target.to_array());
    f("fsTarget", &s.fs_target.to_array());
    f("uiRevision", &[s.ui_revision as f64]);
}

// ---------------------------------------------------------------------------
// JSON（serde を使わず、必要な分だけ書き出す）
// ---------------------------------------------------------------------------

fn js(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn jopt_str(s: &Option<String>) -> String {
    s.as_ref().map(|v| js(v)).unwrap_or_else(|| "null".into())
}

fn jopt_usize(v: Option<usize>) -> String {
    v.map(|x| x.to_string()).unwrap_or_else(|| "null".into())
}

fn jnum(v: f64) -> String {
    if v.is_finite() {
        format!("{}", v)
    } else {
        "null".into()
    }
}

fn ui_json(u: &UiState) -> String {
    let steps: Vec<String> = u
        .steps
        .iter()
        .map(|s| {
            format!(
                r#"{{"no":{},"label":{},"short":{},"done":{},"current":{},"locked":{}}}"#,
                s.no,
                js(s.label),
                js(s.short),
                s.done,
                s.current,
                s.locked
            )
        })
        .collect();
    let goal = u
        .goal
        .as_ref()
        .map(|g| {
            format!(
                r#"{{"text":{},"satisfied":{},"canNext":{},"isLast":{}}}"#,
                js(&g.text),
                g.satisfied,
                g.can_next,
                g.is_last
            )
        })
        .unwrap_or_else(|| "null".into());
    let notice = u
        .notice
        .as_ref()
        .map(|n| {
            let kind = match n.kind {
                NoticeKind::Warning => "warning",
                NoticeKind::Info => "info",
                NoticeKind::Success => "success",
            };
            format!(
                r#"{{"id":{},"kind":"{}","text":{}}}"#,
                n.id,
                kind,
                js(&n.text)
            )
        })
        .unwrap_or_else(|| "null".into());
    let judgement = u
        .judgement
        .as_ref()
        .map(|j| {
            let reasons: Vec<String> = j.reasons.iter().map(|r| js(r)).collect();
            format!(
                r#"{{"passed":{},"reasons":[{}],"timeMs":{},"timeText":{},"loops":{},"centeringText":{},"tiltXText":{},"tiltYText":{},"ratingText":{}}}"#,
                j.passed,
                reasons.join(","),
                jnum(j.time_ms),
                js(&j.time_text),
                j.loops,
                js(&j.centering_text),
                js(&j.tilt_x_text),
                js(&j.tilt_y_text),
                jopt_str(&j.rating_text)
            )
        })
        .unwrap_or_else(|| "null".into());
    let o = &u.observation;
    let point = |p: &ts_core::sim::PointView| {
        format!(
            r#"{{"name":{},"x":{},"y":{},"h":{}}}"#,
            js(p.name),
            js(&p.x_text),
            js(&p.y_text),
            js(&p.h_text)
        )
    };
    let points: Vec<String> = o.points.iter().map(point).collect();
    let bs = o
        .bs
        .as_ref()
        .map(|b| {
            format!(
                r#"{{"azimuthText":{},"horizontalText":{},"coordDistanceText":{},"distanceDiffText":{}}}"#,
                js(&b.azimuth_text),
                js(&b.horizontal_text),
                js(&b.coord_distance_text),
                js(&b.distance_diff_text)
            )
        })
        .unwrap_or_else(|| "null".into());
    let fs = o
        .fs
        .as_ref()
        .map(|f| {
            format!(
                r#"{{"azimuthText":{},"zenithText":{},"horizontalText":{},"xText":{},"yText":{},"hText":{},"dxText":{},"dyText":{},"dhText":{}}}"#,
                js(&f.azimuth_text),
                js(&f.zenith_text),
                js(&f.horizontal_text),
                js(&f.x_text),
                js(&f.y_text),
                js(&f.h_text),
                js(&f.dx_text),
                js(&f.dy_text),
                js(&f.dh_text)
            )
        })
        .unwrap_or_else(|| "null".into());
    let observation = format!(
        r#"{{"points":[{}],"design":{},"station":{},"backsight":{},"heightText":{},"bs":{},"fs":{}}}"#,
        points.join(","),
        point(&o.design),
        jopt_usize(o.station),
        jopt_usize(o.backsight),
        jopt_str(&o.height_text),
        bs,
        fs
    );
    let mode = match u.mode {
        Mode::Textbook => "textbook",
        Mode::Field => "field",
    };
    let phase = match u.phase {
        Phase::Setup => "setup",
        Phase::SetupResult => "setupResult",
        Phase::Observation => "observation",
    };
    format!(
        r#"{{"mode":"{}","phase":"{}","currentStep":{},"steps":[{}],"goal":{},"notice":{},"planted":[{},{},{}],"mounted":{},"clampTight":{},"loops":{},"judgement":{},"observation":{}}}"#,
        mode,
        phase,
        u.current_step,
        steps.join(","),
        goal,
        notice,
        u.planted[0],
        u.planted[1],
        u.planted[2],
        u.mounted,
        u.clamp_tight,
        u.loops,
        judgement,
        observation
    )
}

// ---------------------------------------------------------------------------
// 公開するもの
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct Sim {
    inner: Simulator,
    commands: Vec<Command>,
    out: Vec<f64>,
}

#[wasm_bindgen]
impl Sim {
    /// mode：0＝教科書モード、1＝現場モード
    #[wasm_bindgen(constructor)]
    pub fn new(mode: u32, seed: u32) -> Sim {
        let mode = if mode == 0 {
            Mode::Textbook
        } else {
            Mode::Field
        };
        Sim {
            inner: Simulator::new(mode, seed as u64),
            commands: Vec::with_capacity(16),
            out: Vec::with_capacity(160),
        }
    }

    /// 1フレーム分：操作の列を渡し、毎フレームの数値の列を受け取る
    pub fn frame(&mut self, now_ms: f64, commands: &[f64]) -> Vec<f64> {
        self.commands.clear();
        for c in commands.as_chunks::<COMMAND_STRIDE>().0 {
            if let Some(cmd) = decode(c[0], c[1], c[2], c[3]) {
                self.commands.push(cmd);
            }
        }
        self.inner.frame(now_ms, &self.commands);
        let snap = self.inner.snapshot();
        self.out.clear();
        let out = &mut self.out;
        visit(&snap, &mut |_, v| out.extend_from_slice(v));
        self.out.clone()
    }

    #[wasm_bindgen(js_name = uiRevision)]
    pub fn ui_revision(&self) -> f64 {
        self.inner.ui_revision() as f64
    }

    /// 手順・結果などの状態（JSON）
    #[wasm_bindgen(js_name = uiStateJson)]
    pub fn ui_state_json(&self) -> String {
        ui_json(&self.inner.ui_state())
    }
}

/// 毎フレームの数値の並び（JSON：[{"name","offset","length"}...]）
#[wasm_bindgen(js_name = frameLayoutJson)]
pub fn frame_layout_json() -> String {
    let snap = Simulator::new(Mode::Field, 0).snapshot();
    let mut items = Vec::new();
    let mut offset = 0usize;
    visit(&snap, &mut |name, v| {
        items.push(format!(
            r#"{{"name":{},"offset":{},"length":{}}}"#,
            js(name),
            offset,
            v.len()
        ));
        offset += v.len();
    });
    format!("[{}]", items.join(","))
}

/// 操作のコードと1件あたりの個数（JSON）
#[wasm_bindgen(js_name = commandCodesJson)]
pub fn command_codes_json() -> String {
    let items: Vec<String> = COMMAND_CODES
        .iter()
        .map(|(n, c)| format!("{}:{}", js(n), c))
        .collect();
    format!(
        r#"{{"stride":{},"codes":{{{}}}}}"#,
        COMMAND_STRIDE,
        items.join(",")
    )
}

/// 表示に使う設定値（JSON）
#[wasm_bindgen(js_name = displayConfigJson)]
pub fn display_config_json() -> String {
    use config::*;
    format!(
        r#"{{"centeringToleranceMm":{},"tiltToleranceSec":{},"lcdOuterRangeSec":{},"lcdInnerRatio":{},"circularInRatio":{},"circularVialMaxMm":{},"slideLimitMm":{},"sightOkSec":{},"targetHeightMm":{},"legHingeRadiusMm":{},"headTopOffsetMm":{},"tribrachPivotHeightMm":{},"screwRadiusMm":{},"screwAzimuths":[{},{},{}],"trunnionHeightMm":{}}}"#,
        jnum(CENTERING_TOLERANCE_MM),
        jnum(TILT_TOLERANCE_SEC),
        jnum(LCD_OUTER_RANGE_SEC),
        jnum(LCD_INNER_RATIO),
        jnum(CIRCULAR_IN_RATIO),
        jnum(CIRCULAR_VIAL_MAX_MM),
        jnum(SLIDE_LIMIT_MM),
        jnum(SIGHT_OK_SEC),
        jnum(TARGET_HEIGHT_MM),
        jnum(LEG_HINGE_RADIUS_MM),
        jnum(HEAD_TOP_OFFSET_MM),
        jnum(TRIBRACH_PIVOT_HEIGHT_MM),
        jnum(SCREW_RADIUS_MM),
        jnum(SCREW_AZIMUTHS[0]),
        jnum(SCREW_AZIMUTHS[1]),
        jnum(SCREW_AZIMUTHS[2]),
        jnum(TRUNNION_HEIGHT_MM),
    )
}

/// 度分秒の文字列（符号つき）。つなぎ目の確認用
#[wasm_bindgen(js_name = formatDmsSigned)]
pub fn format_dms_signed(rad: f64) -> String {
    ts_core::angle::format_dms_signed(rad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_and_frame_have_same_length() {
        let mut sim = Sim::new(1, 3);
        let v = sim.frame(0.0, &[]);
        let layout = frame_layout_json();
        let total: usize = layout
            .split("\"length\":")
            .skip(1)
            .filter_map(|s| s.split('}').next())
            .filter_map(|s| s.parse::<usize>().ok())
            .sum();
        assert_eq!(v.len(), total);
    }

    #[test]
    fn commands_are_decoded() {
        assert!(matches!(
            decode(4.0, 1.0, 1.0, 2.0),
            Some(Command::TurnScrew {
                screw: 1,
                thumb: Thumb::Right,
                speed: Speed::Fine
            })
        ));
        assert!(decode(99.0, 0.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn json_escapes_quotes() {
        assert_eq!(js("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn ui_state_json_is_object() {
        let sim = Sim::new(0, 1);
        let j = sim.ui_state_json();
        assert!(j.starts_with('{') && j.ends_with('}'));
        assert!(j.contains("\"mode\":\"textbook\""));
    }
}
