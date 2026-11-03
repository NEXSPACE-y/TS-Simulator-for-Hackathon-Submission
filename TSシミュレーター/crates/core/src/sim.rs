//! シミュレーター全体の状態：モード・手順・タイマー・ループ回数・判定・観測。
//!
//! 表示側からは、1フレームに1回 `frame()` で操作をまとめて渡し、
//! `snapshot()`（毎フレーム変わる数値）と、必要なときだけ `ui_state()`（手順や結果など）を受け取る。

use core::f64::consts::PI;

use crate::angle::{arcsec, format_dms_circle, format_dms_signed, round_arcsec, to_arcsec};
use crate::coach::{self, fact, CoachLog, Fact, FrameSummary, LogKind, LoopRecord, ReviewInput};
use crate::config::*;
use crate::instrument::{
    tilt_within_tolerance, CircularBubble, ElectronicBubble, Instrument, InstrumentError,
    InstrumentPose, Thumb,
};
use crate::math::Vec3;
use crate::rng::Rng;
use crate::steps::{field_steps, observation_steps, textbook_steps, StepText};
use crate::survey::{
    backsight_record, backsight_target_world, direction_to, foresight_record,
    foresight_target_world, line_of_sight, sight_deviation, BacksightRecord, ForesightRecord,
};
use crate::tripod::{Tripod, TripodError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// 教科書モード：教科書の手順を一歩ずつガイドする
    Textbook,
    /// 現場モード：現場の手順。順番は自由
    Field,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// 第1部：据付
    Setup,
    /// 「据付完了」を押した後の結果表示
    SetupResult,
    /// 第2部：観測
    Observation,
}

/// 操作の速さ（粗・中・微）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    Coarse,
    Medium,
    Fine,
}

impl Speed {
    pub fn index(self) -> usize {
        match self {
            Speed::Coarse => 0,
            Speed::Medium => 1,
            Speed::Fine => 2,
        }
    }

    pub fn from_index(i: u32) -> Speed {
        match i {
            0 => Speed::Coarse,
            1 => Speed::Medium,
            _ => Speed::Fine,
        }
    }
}

/// 表示側から渡す操作。「押している間」の操作は、フレームの経過時間×速さだけ動く。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// 踏み込んでいない脚を動かす（全部踏み込んでいなければ三脚ごと）。dir＝[東, 北]
    MoveFreeLegs { dir: [f64; 2], speed: Speed },
    /// 脚を踏み込む
    PressLeg { leg: usize },
    /// 脚を伸ばす・縮める
    ChangeLegLength {
        leg: usize,
        extend: bool,
        speed: Speed,
    },
    /// 整準ネジを回す（親指の向き）
    TurnScrew {
        screw: usize,
        thumb: Thumb,
        speed: Speed,
    },
    /// 定心桿を締める・緩める
    SetClamp { tight: bool },
    /// 器械を三脚頭上でずらす。dir＝[東, 北]
    SlideInstrument { dir: [f64; 2], speed: Speed },
    /// 器械を三脚に載せて定心桿で固定する（教科書モード）
    MountInstrument,
    /// 手順を選ぶ（現場モード・観測）
    SelectStep { step: u8 },
    /// 次の手順へ（教科書モード）
    NextStep,
    /// 据付完了
    CompleteSetup,
    /// 不合格のあと続行する
    ContinueSetup,
    /// 観測へ進む
    StartObservation,
    /// 望遠鏡を回す。dir[0]：＋＝右回り、dir[1]：＋＝上へ
    RotateTelescope { dir: [f64; 2], speed: Speed },
    /// 合焦つまみを回す。dir：＋＝近くにピントが合う向き
    TurnFocus { dir: f64, speed: Speed },
    /// 接眼レンズを回す。dir：＋＝＋側（ディオプター）
    TurnEyepiece { dir: f64, speed: Speed },
    /// 照準器でのぞく（true）／望遠鏡でのぞく（false）
    SetPeepSight { on: bool },
    /// 電子平板で機械点・BS点・器械高をセット（器械高は入力どおりの m）
    SetStation {
        station: usize,
        backsight: usize,
        height_m: f64,
    },
    /// 観測（手順9＝BS、手順10＝FS）
    Observe,
}

/// 操作の種類（教科書モードの「手順外」の判定、現場モードの完了済みの記録に使う）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    MoveLegs,
    PressLeg,
    LegLength,
    Mount,
    Screw,
    Clamp,
    Slide,
}

/// 操作を受け付けなかった理由
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Refusal {
    NotInSetup,
    NotInObservation,
    AllLegsPlanted,
    LegAlreadyPlanted,
    LegLengthLimit,
    TripodShape,
    NotMounted,
    AlreadyMounted,
    ClampTight,
    ScrewLimit,
    SlideLimit,
    BadIndex,
    StepLocked,
    TextbookOrder,
    GoalNotMet,
    NotPassed,
    NoJudgement,
    SameStationAndBacksight,
    HeightOutOfRange,
    StationNotSet,
    BacksightNotObserved,
    UseTabletForStep8,
    SightOff { dh_sec: f64, dv_sec: f64 },
    PeepSight,
    ReticleBlurry,
    OutOfFocus,
}

impl Refusal {
    pub fn message(&self) -> String {
        match self {
            Refusal::NotInSetup => "いまは据付の操作はできません。".into(),
            Refusal::NotInObservation => "観測は据付が合格してから行います。".into(),
            Refusal::AllLegsPlanted => "3本とも踏み込んであるので、脚は動かせません。".into(),
            Refusal::LegAlreadyPlanted => "その脚はもう踏み込んであります。".into(),
            Refusal::LegLengthLimit => "脚はこれ以上伸縮できません。".into(),
            Refusal::TripodShape => {
                "三脚がその形にはなりません（脚が届かない・開きすぎなど）。".into()
            }
            Refusal::NotMounted => "器械がまだ載っていません。".into(),
            Refusal::AlreadyMounted => "器械はもう載っています。".into(),
            Refusal::ClampTight => "定心桿を締めているので、器械はずらせません。".into(),
            Refusal::ScrewLimit => "整準ネジがいっぱいまで回っています。".into(),
            Refusal::SlideLimit => "三脚頭の上で、これ以上はずらせません。".into(),
            Refusal::BadIndex => "その操作はできません。".into(),
            Refusal::StepLocked => "その手順はまだ選べません。".into(),
            Refusal::TextbookOrder => "教科書モードでは、手順は順番に進みます。".into(),
            Refusal::GoalNotMet => "この手順の目標をまだ満たしていません。".into(),
            Refusal::NotPassed => "据付が合格していません。".into(),
            Refusal::NoJudgement => "先に「据付完了」を押してください。".into(),
            Refusal::SameStationAndBacksight => "機械点とBS点に同じ点は選べません。".into(),
            Refusal::HeightOutOfRange => "器械高の値が範囲外です。".into(),
            Refusal::StationNotSet => "先に電子平板で機械点とBSをセットしてください。".into(),
            Refusal::BacksightNotObserved => "先にBSを観測してください。".into(),
            Refusal::UseTabletForStep8 => "手順8は電子平板の「セット」で行います。".into(),
            Refusal::SightOff { dh_sec, dv_sec } => format!(
                "視準がずれています（水平 {}・鉛直 {}。{:.0}″以内に合わせてください）。",
                format_dms_signed(arcsec(*dh_sec)),
                format_dms_signed(arcsec(*dv_sec)),
                SIGHT_OK_SEC
            ),
            Refusal::PeepSight => "照準器では測量できません。望遠鏡に切り替えてのぞいてください。".into(),
            Refusal::ReticleBlurry => {
                "十字線がぼやけています。接眼レンズを回して、十字線をくっきりさせてください。".into()
            }
            Refusal::OutOfFocus => {
                "ピントが合っていません。合焦つまみを回して、ターゲットをくっきりさせてください。".into()
            }
        }
    }
}

impl From<InstrumentError> for Refusal {
    fn from(e: InstrumentError) -> Self {
        match e {
            InstrumentError::NotMounted => Refusal::NotMounted,
            InstrumentError::ClampTight => Refusal::ClampTight,
            InstrumentError::ScrewLimit => Refusal::ScrewLimit,
            InstrumentError::SlideLimit => Refusal::SlideLimit,
            InstrumentError::BadIndex => Refusal::BadIndex,
        }
    }
}

impl From<TripodError> for Refusal {
    fn from(_: TripodError) -> Self {
        Refusal::TripodShape
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// 手順外の操作など、注意
    Warning,
    /// 操作できなかった理由など
    Info,
    /// 成功の知らせ
    Success,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub id: u32,
    pub kind: NoticeKind,
    pub text: String,
}

/// 「据付完了」の判定結果
#[derive(Clone, Debug, PartialEq)]
pub struct SetupJudgement {
    pub passed: bool,
    pub reasons: Vec<String>,
    pub time_ms: f64,
    pub loops: u32,
    pub centering_mm: Option<f64>,
    pub tilt: Option<(f64, f64)>,
    pub clamp_tight: bool,
}

/// 据付の合否判定（SPEC 5.4）。画面に出る値（求心は0.1mm、気泡は1″に丸めた値）で判定する。
pub fn judge_setup(
    mounted: bool,
    centering_mm: Option<f64>,
    tilt: Option<(f64, f64)>,
    clamp_tight: bool,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if !mounted {
        reasons.push("器械が載っていません。".to_string());
    }
    match centering_mm {
        Some(c) => {
            let shown = tenths(c);
            let limit = tenths(CENTERING_TOLERANCE_MM);
            if shown > limit {
                reasons.push(format!(
                    "求心ズレ {} mm（許容 {} mm を {} mm 超えています）",
                    fmt_tenths(shown),
                    fmt_tenths(limit),
                    fmt_tenths(shown - limit)
                ));
            }
        }
        None if mounted => reasons.push("レーザー求心点が地面に届いていません。".to_string()),
        None => {}
    }
    if let Some((x, y)) = tilt {
        for (name, v) in [("X", x), ("Y", y)] {
            if !tilt_within_tolerance(v) {
                let over = round_arcsec(v).abs() - TILT_TOLERANCE_SEC as i64;
                reasons.push(format!(
                    "電子気泡 {} {}（許容 ±{}″ を {}″ 超えています）",
                    name,
                    format_dms_signed(v),
                    TILT_TOLERANCE_SEC as i64,
                    over
                ));
            }
        }
    }
    if mounted && !clamp_tight {
        reasons.push("定心桿が緩んだままです。".to_string());
    }
    (reasons.is_empty(), reasons)
}

/// 0.1刻みに丸めた値（10倍した整数）
fn tenths(v: f64) -> i64 {
    let t = (v * 10.0).round();
    if t.is_finite() {
        t as i64
    } else {
        i64::MAX / 2
    }
}

fn fmt_tenths(t: i64) -> String {
    let sign = if t < 0 { "-" } else { "" };
    let a = t.abs();
    format!("{}{}.{}", sign, a / 10, a % 10)
}

/// タイムの表示 例：`2:34.5`
pub fn format_time(ms: f64) -> String {
    let (m, s, tenth) = time_parts(ms);
    format!("{}:{:02}.{}", m, s, tenth)
}

/// タイムを（分, 秒, 0.1秒）に分ける（0.1秒に丸める）
pub fn time_parts(ms: f64) -> (i64, i64, i64) {
    let t = (ms.max(0.0) / 100.0).round();
    let t = if t.is_finite() { t as i64 } else { 0 };
    (t / 600, (t % 600) / 10, t % 10)
}

/// 求心ズレの表示用の値（0.1mmに丸めた mm）
pub fn centering_display_mm(mm: f64) -> f64 {
    tenths(mm) as f64 / 10.0
}

/// m 表示（小数3桁）。例：`1250.000`
pub fn format_m(mm: f64) -> String {
    format!("{:.3}", mm / 1000.0)
}

/// mm の差の表示（符号つき・整数）。例：`+6 mm`
pub fn format_diff_mm(mm: f64) -> String {
    let v = mm.round() as i64;
    if v > 0 {
        format!("+{} mm", v)
    } else if v < 0 {
        format!("\u{2212}{} mm", -v)
    } else {
        "±0 mm".into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct Timer {
    started: bool,
    accum_ms: f64,
    since: Option<f64>,
}

impl Timer {
    fn start(&mut self, now: f64) {
        if !self.started {
            self.started = true;
            self.since = Some(now);
        }
    }
    fn stop(&mut self, now: f64) {
        if let Some(s) = self.since.take() {
            self.accum_ms += (now - s).max(0.0);
        }
    }
    fn resume(&mut self, now: f64) {
        if self.started && self.since.is_none() {
            self.since = Some(now);
        }
    }
    fn elapsed(&self, now: f64) -> f64 {
        self.accum_ms + self.since.map(|s| (now - s).max(0.0)).unwrap_or(0.0)
    }
    fn running(&self) -> bool {
        self.since.is_some()
    }
}

/// 第2部の状態
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    /// 視準線の方向角（北から時計回り、rad）
    pub hz: f64,
    /// 天頂角（rad）
    pub zenith: f64,
    /// 水平角の読みが0になる方向角（BSを観測すると、読み＝方向角になるように変わる）
    pub circle_offset: f64,
    pub station: Option<usize>,
    pub backsight: Option<usize>,
    pub instrument_height_mm: Option<f64>,
    pub bs: Option<BacksightRecord>,
    pub fs: Option<ForesightRecord>,
    /// 照準器でのぞいているか（false＝望遠鏡）
    pub peep_sight: bool,
    /// 合焦つまみの位置：ピントの合う距離の逆数 [1/m]（0＝無限遠）
    pub focus_per_m: f64,
    /// 接眼レンズの位置 [ディオプター]
    pub eyepiece_dpt: f64,
    /// のぞく人の目に合う接眼レンズの位置 [ディオプター]
    pub eye_dpt: f64,
}

/// 望遠鏡のピントと十字線の見え方（毎フレーム。手順9・10）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpticsView {
    /// 照準器でのぞいているか
    pub peep_sight: bool,
    /// 望遠鏡で見たターゲットのぼけ（0＝くっきり〜1＝最大）
    pub target_blur: f64,
    /// 望遠鏡で見た十字線のぼけ（0〜1）
    pub reticle_blur: f64,
    pub focus_ok: bool,
    pub reticle_ok: bool,
}

/// 視準のズレ（毎フレーム）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SightView {
    /// 1＝BS、2＝FS
    pub target: u8,
    pub dh_sec: f64,
    pub dv_sec: f64,
    pub ok: bool,
}

/// 毎フレーム変わる数値のまとめ（表示はこれを見るだけ）
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub feet: [Vec3; 3],
    pub hinges: [Vec3; 3],
    pub leg_len: [f64; 3],
    pub planted: [bool; 3],
    pub head_center: Vec3,
    pub head_rot: crate::math::Mat3,
    pub head_tilt_deg: f64,
    /// 三脚頭の中心と測点の水平距離 [mm]
    pub head_offset_mm: f64,
    pub mounted: bool,
    pub clamp_tight: bool,
    pub slide: [f64; 2],
    pub screw_turns: [f64; 3],
    /// 整準ネジのつまみの見た目の回転角（上から見て時計回り＝＋、rad）
    pub screw_angles: [f64; 3],
    pub pose: Option<InstrumentPose>,
    pub laser: Option<Vec3>,
    /// レーザー求心点 − 測点（mm）
    pub centering: Option<[f64; 2]>,
    pub centering_mm: Option<f64>,
    pub centering_ok: bool,
    pub circular: Option<CircularBubble>,
    pub electronic: Option<ElectronicBubble>,
    pub instrument_height_mm: Option<f64>,
    /// 器械高 [m]（表示用）
    pub instrument_height_m: Option<f64>,
    pub elapsed_ms: f64,
    pub timer_running: bool,
    pub loops: u32,
    pub telescope_hz: f64,
    /// 器械から見た照準部の向き（rad）。器械自体の向きを除いた分で、3Dの照準部を回す量
    pub telescope_hz_local: f64,
    pub telescope_zenith: f64,
    pub telescope_dir: Vec3,
    pub trunnion: Option<Vec3>,
    /// 水平角の読み（rad）
    pub hr_reading: f64,
    pub sight: Option<SightView>,
    pub optics: Option<OpticsView>,
    pub bs_target: Vec3,
    pub fs_target: Vec3,
    pub ui_revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StepView {
    pub no: u8,
    pub label: &'static str,
    pub short: &'static str,
    pub done: bool,
    pub current: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GoalView {
    pub text: String,
    pub satisfied: bool,
    pub can_next: bool,
    pub is_last: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JudgementView {
    pub passed: bool,
    pub reasons: Vec<String>,
    pub time_ms: f64,
    pub time_text: String,
    pub loops: u32,
    pub centering_text: String,
    pub tilt_x_text: String,
    pub tilt_y_text: String,
    pub rating_text: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PointView {
    pub name: &'static str,
    pub x_text: String,
    pub y_text: String,
    pub h_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BacksightView {
    pub azimuth_text: String,
    pub horizontal_text: String,
    pub coord_distance_text: String,
    pub distance_diff_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForesightView {
    pub azimuth_text: String,
    pub zenith_text: String,
    pub horizontal_text: String,
    pub x_text: String,
    pub y_text: String,
    pub h_text: String,
    pub dx_text: String,
    pub dy_text: String,
    pub dh_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationView {
    pub points: Vec<PointView>,
    pub design: PointView,
    pub station: Option<usize>,
    pub backsight: Option<usize>,
    pub height_text: Option<String>,
    pub bs: Option<BacksightView>,
    pub fs: Option<ForesightView>,
    /// 照準器でのぞいているか（false＝望遠鏡）
    pub peep_sight: bool,
}

/// 変化の少ない画面の状態（手順・注意・結果など）
#[derive(Clone, Debug, PartialEq)]
pub struct UiState {
    pub mode: Mode,
    pub phase: Phase,
    pub current_step: u8,
    pub steps: Vec<StepView>,
    pub goal: Option<GoalView>,
    pub notice: Option<Notice>,
    pub planted: [bool; 3],
    pub mounted: bool,
    pub clamp_tight: bool,
    pub loops: u32,
    pub judgement: Option<JudgementView>,
    pub observation: ObservationView,
}

/// 変化を検出するための署名（これが変わったら ui_revision を上げる）
#[derive(Clone, Debug, PartialEq, Default)]
struct UiSignature {
    phase: u8,
    step: u8,
    field_done: [bool; 7],
    planted: [bool; 3],
    mounted: bool,
    clamp: bool,
    notice_id: u32,
    judgement: bool,
    obs_rev: u32,
    goal_ok: bool,
    loops: u32,
}

pub struct Simulator {
    mode: Mode,
    phase: Phase,
    rng: Rng,
    pub tripod: Tripod,
    pub inst: Instrument,
    now_ms: f64,
    last_frame_ms: Option<f64>,
    timer: Timer,
    loops: u32,
    slid_since_loosen: bool,
    any_slide: bool,
    field_done: [bool; 7],
    selected_step: u8,
    guided_step: u8,
    notice: Option<Notice>,
    notice_seq: u32,
    notice_time: f64,
    judgement: Option<SetupJudgement>,
    pub obs: Observation,
    obs_rev: u32,
    ui_revision: u64,
    ui_sig: UiSignature,
    /// AI先輩のための操作の記録
    coach: CoachLog,
    /// frame() の中で操作を反映している間だけ true（このフレームの操作を集める）
    recording: bool,
    frame_ops: Vec<OpKind>,
    frame_screws: [bool; 3],
    frame_refusals: Vec<Refusal>,
}

/// AI先輩に渡す事実の種類
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactKind {
    /// 「据付完了」のあとの講評
    Review,
    /// いまの状態と次の操作（何でも質問）
    Status,
}

impl Simulator {
    pub fn new(mode: Mode, seed: u64) -> Simulator {
        let mut rng = Rng::new(seed);
        let tripod = random_tripod(&mut rng);
        let mut inst = Instrument::default();
        if mode == Mode::Field {
            // 現場モード：手順2の前の時点で、器械は定心桿で三脚に固定済み（SPEC 5.1）
            randomize_instrument(&mut inst, &mut rng);
        } else {
            inst.mounted = false;
            inst.clamp_tight = false;
        }
        let circle_offset = rng.range(0.0, 2.0 * PI);
        let mut sim = Simulator {
            mode,
            phase: Phase::Setup,
            rng,
            tripod,
            inst,
            now_ms: 0.0,
            last_frame_ms: None,
            timer: Timer::default(),
            loops: 0,
            slid_since_loosen: false,
            any_slide: false,
            field_done: [false; 7],
            selected_step: 1,
            guided_step: 1,
            notice: None,
            notice_seq: 0,
            notice_time: f64::NEG_INFINITY,
            judgement: None,
            obs: Observation {
                hz: 0.0,
                zenith: TELESCOPE_START_ZENITH,
                circle_offset,
                station: None,
                backsight: None,
                instrument_height_mm: None,
                bs: None,
                fs: None,
                peep_sight: false,
                focus_per_m: 0.0,
                eyepiece_dpt: 0.0,
                eye_dpt: 0.0,
            },
            obs_rev: 0,
            ui_revision: 1,
            ui_sig: UiSignature::default(),
            coach: CoachLog::default(),
            recording: false,
            frame_ops: Vec::with_capacity(8),
            frame_screws: [false; 3],
            frame_refusals: Vec::with_capacity(4),
        };
        sim.ui_sig = sim.signature();
        sim
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn loops(&self) -> u32 {
        self.loops
    }

    pub fn judgement(&self) -> Option<&SetupJudgement> {
        self.judgement.as_ref()
    }

    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    pub fn ui_revision(&self) -> u64 {
        self.ui_revision
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.timer.elapsed(self.now_ms)
    }

    pub fn timer_running(&self) -> bool {
        self.timer.running()
    }

    /// 1フレーム分の処理。`now_ms` は単調に増える時刻（performance.now() など）。
    pub fn frame(&mut self, now_ms: f64, commands: &[Command]) {
        let now = if now_ms.is_finite() {
            now_ms
        } else {
            self.now_ms
        };
        let dt_ms = match self.last_frame_ms {
            Some(t) => (now - t).clamp(0.0, MAX_FRAME_DT_MS),
            None => 0.0,
        };
        self.now_ms = now.max(self.now_ms);
        self.last_frame_ms = Some(now);
        let dt = dt_ms / 1000.0;
        let before = self.coach_objective();
        self.frame_ops.clear();
        self.frame_screws = [false; 3];
        self.frame_refusals.clear();
        self.recording = true;
        for c in commands {
            self.apply(*c, dt);
        }
        self.recording = false;
        self.update_coach(dt_ms, before);
        self.refresh_revision();
    }

    /// 操作を1つ反映する。`dt` は「押している間」の操作に使う経過時間 [s]。
    pub fn apply(&mut self, cmd: Command, dt: f64) {
        let held = matches!(
            cmd,
            Command::MoveFreeLegs { .. }
                | Command::ChangeLegLength { .. }
                | Command::TurnScrew { .. }
                | Command::SlideInstrument { .. }
                | Command::RotateTelescope { .. }
                | Command::TurnFocus { .. }
                | Command::TurnEyepiece { .. }
        );
        if held && dt <= 0.0 {
            return;
        }
        let result = match cmd {
            Command::MoveFreeLegs { dir, speed } => {
                let d = unit2(dir);
                let s = SPEED_MOVE_LEGS[speed.index()] * dt;
                self.move_free_legs([d[0] * s, d[1] * s])
            }
            Command::PressLeg { leg } => self.press_leg(leg),
            Command::ChangeLegLength { leg, extend, speed } => {
                let s = SPEED_LEG_LENGTH[speed.index()] * dt;
                self.change_leg_length(leg, if extend { s } else { -s })
            }
            Command::TurnScrew {
                screw,
                thumb,
                speed,
            } => self.turn_screw(screw, thumb, SPEED_SCREW[speed.index()] * dt),
            Command::SetClamp { tight } => self.set_clamp(tight),
            Command::SlideInstrument { dir, speed } => {
                let d = unit2(dir);
                let s = SPEED_SLIDE[speed.index()] * dt;
                self.slide_instrument([d[0] * s, d[1] * s])
            }
            Command::MountInstrument => self.mount_instrument(),
            Command::SelectStep { step } => self.select_step(step),
            Command::NextStep => self.next_step(),
            Command::CompleteSetup => self.complete_setup().map(|_| ()),
            Command::ContinueSetup => self.continue_setup(),
            Command::StartObservation => self.start_observation(),
            Command::RotateTelescope { dir, speed } => {
                let s = SPEED_TELESCOPE[speed.index()] * dt;
                self.rotate_telescope(dir[0].clamp(-1.0, 1.0) * s, dir[1].clamp(-1.0, 1.0) * s)
            }
            Command::TurnFocus { dir, speed } => {
                self.turn_focus(dir.clamp(-1.0, 1.0) * SPEED_FOCUS[speed.index()] * dt)
            }
            Command::TurnEyepiece { dir, speed } => {
                self.turn_eyepiece(dir.clamp(-1.0, 1.0) * SPEED_EYEPIECE[speed.index()] * dt)
            }
            Command::SetPeepSight { on } => self.set_peep_sight(on),
            Command::SetStation {
                station,
                backsight,
                height_m,
            } => self.set_station(station, backsight, height_m * 1000.0),
            Command::Observe => self.observe(),
        };
        if let Err(r) = result {
            if self.recording {
                self.frame_refusals.push(r);
            }
            self.notify(NoticeKind::Info, r.message());
        }
    }

    // ------------------------------------------------------------------
    // 第1部：据付の操作
    // ------------------------------------------------------------------

    fn ensure_setup(&self) -> Result<(), Refusal> {
        if self.phase == Phase::Setup {
            Ok(())
        } else {
            Err(Refusal::NotInSetup)
        }
    }

    /// 操作が成功したあとの共通処理：タイマー開始、手順外の注意、完了済みの記録
    fn after_op(&mut self, kind: OpKind) {
        self.timer.start(self.now_ms);
        if self.recording {
            self.frame_ops.push(kind);
        }
        match self.mode {
            Mode::Textbook => {
                if !textbook_allows(self.guided_step, kind) {
                    let steps = textbook_steps();
                    let cur = &steps[(self.guided_step as usize).saturating_sub(1).min(6)];
                    let text = match textbook_step_for(kind) {
                        Some(n) => format!(
                            "注意：いまは手順{}「{}」です。この操作は手順{}で行います。",
                            self.guided_step, cur.short, n
                        ),
                        None => format!(
                            "注意：いまは手順{}「{}」です。",
                            self.guided_step, cur.short
                        ),
                    };
                    if self.notify(NoticeKind::Warning, text) {
                        self.coach.textbook_warnings += 1;
                    }
                }
            }
            Mode::Field => {
                let planted = self.tripod.planted_count();
                match kind {
                    OpKind::PressLeg => {
                        for i in 0..planted.min(3) {
                            self.field_done[i] = true;
                        }
                    }
                    OpKind::LegLength if planted == 3 => self.field_done[3] = true,
                    OpKind::Screw => {
                        if self.any_slide {
                            self.field_done[6] = true;
                        } else {
                            self.field_done[4] = true;
                        }
                    }
                    OpKind::Slide => self.field_done[5] = true,
                    _ => {}
                }
            }
        }
    }

    /// 踏み込んでいない脚の足先を動かす（mm、[東, 北]）。1本も踏み込んでいなければ三脚ごと動く。
    pub fn move_free_legs(&mut self, d: [f64; 2]) -> Result<(), Refusal> {
        self.ensure_setup()?;
        if self.tripod.planted_count() == 3 {
            return Err(Refusal::AllLegsPlanted);
        }
        let mut feet = self.tripod.feet;
        for (i, f) in feet.iter_mut().enumerate() {
            if !self.tripod.planted[i] {
                f.x += d[0];
                f.y += d[1];
            }
        }
        let lens = self.tripod.leg_len;
        self.tripod.try_set(feet, lens)?;
        self.after_op(OpKind::MoveLegs);
        Ok(())
    }

    /// 脚を踏み込む。足先が数mm沈む（毎回少し違う）。
    pub fn press_leg(&mut self, leg: usize) -> Result<(), Refusal> {
        self.ensure_setup()?;
        if leg >= 3 {
            return Err(Refusal::BadIndex);
        }
        if self.tripod.planted[leg] {
            return Err(Refusal::LegAlreadyPlanted);
        }
        let sink = self.rng.range(SINK_MIN_MM, SINK_MAX_MM);
        let mut feet = self.tripod.feet;
        feet[leg].z -= sink;
        let lens = self.tripod.leg_len;
        // 沈んだ形が成り立たないときは、沈まずに踏み込んだことにする
        let _ = self.tripod.try_set(feet, lens);
        self.tripod.planted[leg] = true;
        self.after_op(OpKind::PressLeg);
        Ok(())
    }

    /// 脚の長さを変える（mm）
    pub fn change_leg_length(&mut self, leg: usize, delta_mm: f64) -> Result<(), Refusal> {
        self.ensure_setup()?;
        if leg >= 3 {
            return Err(Refusal::BadIndex);
        }
        let mut lens = self.tripod.leg_len;
        let next = (lens[leg] + delta_mm).clamp(LEG_LENGTH_MIN_MM, LEG_LENGTH_MAX_MM);
        if (next - lens[leg]).abs() < 1e-12 {
            if delta_mm != 0.0 {
                return Err(Refusal::LegLengthLimit);
            }
            return Ok(());
        }
        lens[leg] = next;
        let feet = self.tripod.feet;
        self.tripod.try_set(feet, lens)?;
        self.after_op(OpKind::LegLength);
        Ok(())
    }

    /// 整準ネジを回す（回転数）
    pub fn turn_screw(&mut self, screw: usize, thumb: Thumb, turns: f64) -> Result<(), Refusal> {
        self.ensure_setup()?;
        self.inst.turn_screw(screw, thumb, turns)?;
        if self.recording {
            self.frame_screws[screw.min(2)] = true;
        }
        self.after_op(OpKind::Screw);
        Ok(())
    }

    /// 定心桿を締める・緩める
    pub fn set_clamp(&mut self, tight: bool) -> Result<(), Refusal> {
        self.ensure_setup()?;
        if !self.inst.mounted {
            return Err(Refusal::NotMounted);
        }
        if self.inst.clamp_tight == tight {
            return Ok(());
        }
        self.inst.clamp_tight = tight;
        if !tight {
            self.slid_since_loosen = false;
        }
        self.after_op(OpKind::Clamp);
        Ok(())
    }

    /// 器械を三脚頭上でずらす（mm、[東, 北]）。三脚頭の面に沿って動く。
    pub fn slide_instrument(&mut self, d: [f64; 2]) -> Result<(), Refusal> {
        self.ensure_setup()?;
        let local = self
            .tripod
            .head
            .rot
            .transpose()
            .mul_vec(Vec3::new(d[0], d[1], 0.0));
        let tilt = self.tripod.head.tilt();
        let before = (!self.slid_since_loosen).then(|| self.loop_record());
        self.inst.slide_by([local.x, local.y], tilt)?;
        if let Some(rec) = before {
            // ループ回数：手順6（器械ずらし）を行った回数（SPEC 5.1）
            self.loops += 1;
            self.slid_since_loosen = true;
            self.coach.start_loop(rec);
        }
        self.any_slide = true;
        self.after_op(OpKind::Slide);
        Ok(())
    }

    /// 器械を載せて定心桿で固定する（教科書モードの手順2）
    pub fn mount_instrument(&mut self) -> Result<(), Refusal> {
        self.ensure_setup()?;
        if self.inst.mounted {
            return Err(Refusal::AlreadyMounted);
        }
        let mut inst = Instrument::default();
        randomize_instrument(&mut inst, &mut self.rng);
        self.inst = inst;
        self.after_op(OpKind::Mount);
        Ok(())
    }

    fn select_step(&mut self, step: u8) -> Result<(), Refusal> {
        match self.phase {
            Phase::Setup => {
                if self.mode == Mode::Textbook {
                    return Err(Refusal::TextbookOrder);
                }
                if !(1..=7).contains(&step) {
                    return Err(Refusal::StepLocked);
                }
                self.selected_step = step;
                Ok(())
            }
            Phase::Observation => {
                let ok = match step {
                    8 => true,
                    9 => self.obs.station.is_some(),
                    10 => self.obs.bs.is_some(),
                    _ => false,
                };
                if !ok {
                    return Err(Refusal::StepLocked);
                }
                self.selected_step = step;
                Ok(())
            }
            Phase::SetupResult => Err(Refusal::StepLocked),
        }
    }

    fn next_step(&mut self) -> Result<(), Refusal> {
        self.ensure_setup()?;
        if self.mode != Mode::Textbook {
            return Ok(());
        }
        let snap = self.snapshot();
        let (_, ok) = self.textbook_goal(self.guided_step, &snap);
        if !ok {
            return Err(Refusal::GoalNotMet);
        }
        if self.guided_step < 7 {
            self.guided_step += 1;
        }
        Ok(())
    }

    /// 据付完了：合否を判定し、タイマーを止める
    pub fn complete_setup(&mut self) -> Result<&SetupJudgement, Refusal> {
        self.ensure_setup()?;
        self.timer.stop(self.now_ms);
        let snap = self.snapshot();
        let tilt = snap.electronic.map(|e| (e.x, e.y));
        let (passed, reasons) =
            judge_setup(snap.mounted, snap.centering_mm, tilt, snap.clamp_tight);
        self.judgement = Some(SetupJudgement {
            passed,
            reasons,
            time_ms: self.timer.elapsed(self.now_ms),
            loops: self.loops,
            centering_mm: snap.centering_mm,
            tilt,
            clamp_tight: snap.clamp_tight,
        });
        self.phase = Phase::SetupResult;
        if passed {
            self.field_done = [true; 7];
            self.guided_step = 7;
        } else {
            self.coach.failed_completions += 1;
        }
        self.judgement.as_ref().ok_or(Refusal::NoJudgement)
    }

    /// 不合格のあと、据付を続ける（タイマーも再開）
    pub fn continue_setup(&mut self) -> Result<(), Refusal> {
        if self.phase != Phase::SetupResult {
            return Err(Refusal::NoJudgement);
        }
        if self.judgement.as_ref().map(|j| j.passed).unwrap_or(false) {
            return Err(Refusal::BadIndex);
        }
        self.judgement = None;
        self.phase = Phase::Setup;
        self.timer.resume(self.now_ms);
        Ok(())
    }

    /// 第2部へ進む（据付が合格していること）
    pub fn start_observation(&mut self) -> Result<(), Refusal> {
        let passed = self.judgement.as_ref().map(|j| j.passed).unwrap_or(false);
        if self.phase != Phase::SetupResult || !passed {
            return Err(Refusal::NotPassed);
        }
        self.phase = Phase::Observation;
        self.selected_step = 8;
        self.obs.hz = 0.0;
        self.obs.zenith = TELESCOPE_START_ZENITH;
        // 合焦と接眼レンズは、前に使った人の設定のまま（どちらも合っていない）
        self.obs.peep_sight = false;
        self.obs.focus_per_m = 1.0 / self.rng.range(FOCUS_START_MIN_M, FOCUS_START_MAX_M);
        self.obs.eye_dpt = self.rng.range(-EYE_RANGE_DPT, EYE_RANGE_DPT);
        let off = self
            .rng
            .range(EYEPIECE_START_OFF_MIN_DPT, EYEPIECE_START_OFF_MAX_DPT);
        // 範囲の外に出る向きなら、反対側へずらす
        let up = self.obs.eye_dpt + off;
        self.obs.eyepiece_dpt = if up <= EYEPIECE_RANGE_DPT && self.rng.range(0.0, 1.0) < 0.5 {
            up
        } else {
            self.obs.eye_dpt - off
        };
        self.obs_rev += 1;
        Ok(())
    }

    // ------------------------------------------------------------------
    // 第2部：観測の操作
    // ------------------------------------------------------------------

    fn ensure_observation(&self) -> Result<(), Refusal> {
        if self.phase == Phase::Observation {
            Ok(())
        } else {
            Err(Refusal::NotInObservation)
        }
    }

    /// 望遠鏡を回す（rad）。dh：＋＝右回り、dv：＋＝上へ
    pub fn rotate_telescope(&mut self, dh: f64, dv: f64) -> Result<(), Refusal> {
        self.ensure_observation()?;
        self.obs.hz = (self.obs.hz + dh).rem_euclid(2.0 * PI);
        self.obs.zenith = (self.obs.zenith - dv).clamp(TELESCOPE_ZENITH_MIN, TELESCOPE_ZENITH_MAX);
        Ok(())
    }

    /// 合焦つまみを回す（ピントの合う距離の逆数を d [1/m] だけ変える。＋＝近くへ）
    pub fn turn_focus(&mut self, d: f64) -> Result<(), Refusal> {
        self.ensure_observation()?;
        let max = 1.0 / FOCUS_MIN_DISTANCE_M;
        self.obs.focus_per_m = (self.obs.focus_per_m + d).clamp(0.0, max);
        Ok(())
    }

    /// 接眼レンズを回す（d [ディオプター]）
    pub fn turn_eyepiece(&mut self, d: f64) -> Result<(), Refusal> {
        self.ensure_observation()?;
        self.obs.eyepiece_dpt =
            (self.obs.eyepiece_dpt + d).clamp(-EYEPIECE_RANGE_DPT, EYEPIECE_RANGE_DPT);
        Ok(())
    }

    /// 照準器でのぞく（true）／望遠鏡でのぞく（false）
    pub fn set_peep_sight(&mut self, on: bool) -> Result<(), Refusal> {
        self.ensure_observation()?;
        if self.obs.peep_sight != on {
            self.obs.peep_sight = on;
            self.obs_rev += 1;
        }
        Ok(())
    }

    /// 望遠鏡の向きを直接決める（テスト用）
    pub fn set_telescope(&mut self, hz: f64, zenith: f64) {
        self.obs.hz = hz.rem_euclid(2.0 * PI);
        self.obs.zenith = zenith.clamp(TELESCOPE_ZENITH_MIN, TELESCOPE_ZENITH_MAX);
    }

    /// 電子平板：機械点・BS点・器械高をセット（手順8）
    pub fn set_station(
        &mut self,
        station: usize,
        backsight: usize,
        height_mm: f64,
    ) -> Result<(), Refusal> {
        self.ensure_observation()?;
        if station >= KNOWN_POINTS.len() || backsight >= KNOWN_POINTS.len() {
            return Err(Refusal::BadIndex);
        }
        if station == backsight {
            return Err(Refusal::SameStationAndBacksight);
        }
        if !height_mm.is_finite()
            || !(INSTRUMENT_HEIGHT_INPUT_MIN_MM..=INSTRUMENT_HEIGHT_INPUT_MAX_MM)
                .contains(&height_mm)
        {
            return Err(Refusal::HeightOutOfRange);
        }
        self.obs.station = Some(station);
        self.obs.backsight = Some(backsight);
        self.obs.instrument_height_mm = Some(height_mm);
        self.obs.bs = None;
        self.obs.fs = None;
        self.obs_rev += 1;
        self.selected_step = 9;
        self.notify(
            NoticeKind::Success,
            format!(
                "機械点 {}・BS {}・器械高 {} m をセットしました。",
                KNOWN_POINTS[station].name,
                KNOWN_POINTS[backsight].name,
                format_m(height_mm)
            ),
        );
        Ok(())
    }

    /// 観測する（手順9＝BS、手順10＝FS）
    pub fn observe(&mut self) -> Result<(), Refusal> {
        self.ensure_observation()?;
        let snap = self.snapshot();
        let trunnion = snap.trunnion.ok_or(Refusal::NotMounted)?;
        match self.selected_step {
            9 => {
                let (Some(st), Some(bs)) = (self.obs.station, self.obs.backsight) else {
                    return Err(Refusal::StationNotSet);
                };
                check_optics(&snap)?;
                check_sight(&snap)?;
                let (_, _, slope) = direction_to(trunnion, backsight_target_world());
                let reading = snap.hr_reading;
                let rec = backsight_record(
                    &KNOWN_POINTS[st],
                    &KNOWN_POINTS[bs],
                    reading,
                    self.obs.zenith,
                    slope,
                );
                // 方向角をセット：以後、水平角の読み＝方向角になる
                self.obs.circle_offset -= rec.orientation;
                self.obs.bs = Some(rec);
                self.obs.fs = None;
                self.obs_rev += 1;
                self.selected_step = 10;
                self.notify(
                    NoticeKind::Success,
                    format!(
                        "BSを観測しました。方向角 {} をセットしました。",
                        format_dms_circle(rec.azimuth)
                    ),
                );
                Ok(())
            }
            10 => {
                let (Some(st), Some(ih)) = (self.obs.station, self.obs.instrument_height_mm) else {
                    return Err(Refusal::StationNotSet);
                };
                if self.obs.bs.is_none() {
                    return Err(Refusal::BacksightNotObserved);
                }
                check_optics(&snap)?;
                check_sight(&snap)?;
                let (_, _, slope) = direction_to(trunnion, foresight_target_world());
                let rec = foresight_record(
                    &KNOWN_POINTS[st],
                    ih,
                    snap.hr_reading,
                    self.obs.zenith,
                    slope,
                    TARGET_HEIGHT_MM,
                    &FS_DESIGN,
                );
                self.obs.fs = Some(rec);
                self.obs_rev += 1;
                self.notify(NoticeKind::Success, "FSを観測しました。".into());
                Ok(())
            }
            _ => Err(Refusal::UseTabletForStep8),
        }
    }

    // ------------------------------------------------------------------
    // 出力
    // ------------------------------------------------------------------

    fn current_step(&self) -> u8 {
        match (self.phase, self.mode) {
            (Phase::Observation, _) => self.selected_step,
            (_, Mode::Textbook) => self.guided_step,
            (_, Mode::Field) => self.selected_step,
        }
    }

    /// 毎フレーム変わる数値
    pub fn snapshot(&self) -> Snapshot {
        let t = &self.tripod;
        let pose = self.inst.pose(&t.head);
        let laser = pose.as_ref().and_then(|p| p.laser_point());
        let centering = laser.map(|l| [l.x, l.y]);
        let centering_mm = centering.map(|c| c[0].hypot(c[1]));
        let centering_ok = centering_mm
            .map(|c| tenths(c) <= tenths(CENTERING_TOLERANCE_MM))
            .unwrap_or(false);
        let circular = pose.as_ref().map(CircularBubble::from_pose);
        let electronic = pose.as_ref().map(ElectronicBubble::from_pose);
        let trunnion = pose.as_ref().map(|p| p.trunnion());
        let hr_reading = (self.obs.hz - self.obs.circle_offset).rem_euclid(2.0 * PI);
        // 器械の y 軸（C の側）の方位を引くと、器械の中での照準部の向きになる
        let telescope_hz_local = match pose.as_ref() {
            Some(p) => {
                let y = p.rot.col(1);
                (self.obs.hz - y.x.atan2(y.y)).rem_euclid(2.0 * PI)
            }
            None => self.obs.hz,
        };
        let sight = match (self.phase, self.selected_step, trunnion) {
            (Phase::Observation, 9, Some(eye)) => {
                Some(sight_view(1, &self.obs, eye, backsight_target_world()))
            }
            (Phase::Observation, 10, Some(eye)) => {
                Some(sight_view(2, &self.obs, eye, foresight_target_world()))
            }
            _ => None,
        };
        // ピントは、いま視準しようとしているターゲットまでの距離で決まる
        let optics = match (self.phase, self.selected_step, trunnion) {
            (Phase::Observation, 9, Some(eye)) => Some(optics_view(
                &self.obs,
                (backsight_target_world() - eye).norm() / 1000.0,
            )),
            (Phase::Observation, 10, Some(eye)) => Some(optics_view(
                &self.obs,
                (foresight_target_world() - eye).norm() / 1000.0,
            )),
            _ => None,
        };
        let head_center = t.head.center;
        Snapshot {
            feet: t.feet,
            hinges: t.hinge_points(),
            leg_len: t.leg_len,
            planted: t.planted,
            head_center,
            head_rot: t.head.rot,
            head_tilt_deg: t.head.tilt() * 180.0 / PI,
            head_offset_mm: head_center.x.hypot(head_center.y),
            mounted: self.inst.mounted,
            clamp_tight: self.inst.clamp_tight,
            slide: self.inst.slide,
            screw_turns: self.inst.screw_turns,
            screw_angles: self.inst.screw_turns.map(|v| v * 2.0 * PI),
            pose,
            laser,
            centering,
            centering_mm,
            centering_ok,
            circular,
            electronic,
            instrument_height_mm: pose.as_ref().map(|p| p.instrument_height()),
            instrument_height_m: pose.as_ref().map(|p| p.instrument_height() / 1000.0),
            elapsed_ms: self.timer.elapsed(self.now_ms),
            timer_running: self.timer.running(),
            loops: self.loops,
            telescope_hz: self.obs.hz,
            telescope_hz_local,
            telescope_zenith: self.obs.zenith,
            telescope_dir: line_of_sight(self.obs.hz, self.obs.zenith),
            trunnion,
            hr_reading,
            sight,
            optics,
            bs_target: backsight_target_world(),
            fs_target: foresight_target_world(),
            ui_revision: self.ui_revision,
        }
    }

    /// 教科書モードの手順の目標と、満たしているか
    fn textbook_goal(&self, step: u8, s: &Snapshot) -> (String, bool) {
        let all_planted = s.planted.iter().all(|p| *p);
        let c_ok = |lim: f64| {
            s.centering_mm
                .map(|c| tenths(c) <= tenths(lim))
                .unwrap_or(false)
        };
        let tilt_ok = s.electronic.map(|e| e.ok).unwrap_or(false);
        match step {
            1 => (
                format!(
                    "3本とも踏み込み、脚頭の中心を測点から{}mm以内、脚頭の傾きを{}°以内にする",
                    TEXTBOOK_ROUGH_CENTER_MM,
                    TEXTBOOK_ROUGH_HEAD_TILT * 180.0 / PI
                ),
                all_planted
                    && s.head_offset_mm <= TEXTBOOK_ROUGH_CENTER_MM
                    && s.head_tilt_deg <= TEXTBOOK_ROUGH_HEAD_TILT * 180.0 / PI,
            ),
            2 => (
                "器械を載せて、定心桿で固定する".into(),
                s.mounted && s.clamp_tight,
            ),
            3 => (
                format!(
                    "整準ネジで、レーザー求心点を測点に合わせる（ズレ{}mm以内）",
                    TEXTBOOK_SCREW_CENTER_MM
                ),
                c_ok(TEXTBOOK_SCREW_CENTER_MM),
            ),
            4 => (
                "三脚の足の伸縮で、円形気泡を中心の円に入れる".into(),
                s.circular.map(|c| c.in_circle).unwrap_or(false),
            ),
            5 => (
                format!(
                    "整準ネジで、電子気泡をX・Yとも±{}″以内にする",
                    TILT_TOLERANCE_SEC
                ),
                tilt_ok,
            ),
            6 => (
                format!(
                    "定心桿を緩めて器械をずらし、求心ズレを{}mm以内にして締める",
                    CENTERING_TOLERANCE_MM
                ),
                c_ok(CENTERING_TOLERANCE_MM) && s.clamp_tight,
            ),
            _ => (
                format!(
                    "求心ズレ{}mm以内・電子気泡±{}″以内の両方がそろったら「据付完了」",
                    CENTERING_TOLERANCE_MM, TILT_TOLERANCE_SEC
                ),
                c_ok(CENTERING_TOLERANCE_MM) && tilt_ok && s.clamp_tight,
            ),
        }
    }

    /// 変化の少ない画面の状態
    pub fn ui_state(&self) -> UiState {
        let snap = self.snapshot();
        let current = self.current_step();
        let mut steps = Vec::new();
        let setup_texts: [StepText; 7] = match self.mode {
            Mode::Textbook => textbook_steps(),
            Mode::Field => field_steps(),
        };
        let passed = self.judgement.as_ref().map(|j| j.passed).unwrap_or(false)
            || self.phase == Phase::Observation;
        for (i, t) in setup_texts.iter().enumerate() {
            let no = (i + 1) as u8;
            let done = match self.mode {
                Mode::Textbook => passed || no < self.guided_step,
                Mode::Field => passed || self.field_done[i],
            };
            steps.push(StepView {
                no,
                label: t.label,
                short: t.short,
                done,
                current: self.phase != Phase::Observation && current == no,
                locked: false,
            });
        }
        for (i, t) in observation_steps().iter().enumerate() {
            let no = (i + 8) as u8;
            let done = match no {
                8 => self.obs.station.is_some(),
                9 => self.obs.bs.is_some(),
                _ => self.obs.fs.is_some(),
            };
            let locked = self.phase != Phase::Observation
                || (no == 9 && self.obs.station.is_none())
                || (no == 10 && self.obs.bs.is_none());
            steps.push(StepView {
                no,
                label: t.label,
                short: t.short,
                done,
                current: self.phase == Phase::Observation && current == no,
                locked,
            });
        }
        let goal = if self.mode == Mode::Textbook && self.phase == Phase::Setup {
            let (text, satisfied) = self.textbook_goal(self.guided_step, &snap);
            Some(GoalView {
                text,
                satisfied,
                can_next: satisfied && self.guided_step < 7,
                is_last: self.guided_step >= 7,
            })
        } else {
            None
        };
        UiState {
            mode: self.mode,
            phase: self.phase,
            current_step: current,
            steps,
            goal,
            notice: self.notice.clone(),
            planted: self.tripod.planted,
            mounted: self.inst.mounted,
            clamp_tight: self.inst.clamp_tight,
            loops: self.loops,
            judgement: self.judgement.as_ref().map(judgement_view),
            observation: self.observation_view(),
        }
    }

    fn observation_view(&self) -> ObservationView {
        let pv = |p: &KnownPoint| PointView {
            name: p.name,
            x_text: format_m(p.x_mm),
            y_text: format_m(p.y_mm),
            h_text: format_m(p.h_mm),
        };
        ObservationView {
            points: KNOWN_POINTS.iter().map(pv).collect(),
            design: pv(&FS_DESIGN),
            station: self.obs.station,
            backsight: self.obs.backsight,
            height_text: self.obs.instrument_height_mm.map(format_m),
            peep_sight: self.obs.peep_sight,
            bs: self.obs.bs.map(|b| BacksightView {
                azimuth_text: format_dms_circle(b.azimuth),
                horizontal_text: format_m(b.horizontal_mm),
                coord_distance_text: format_m(b.coord_distance_mm),
                distance_diff_text: format_diff_mm(b.horizontal_mm - b.coord_distance_mm),
            }),
            fs: self.obs.fs.map(|f| ForesightView {
                azimuth_text: format_dms_circle(f.azimuth),
                zenith_text: format_dms_circle(f.zenith),
                horizontal_text: format_m(f.horizontal_mm),
                x_text: format_m(f.x_mm),
                y_text: format_m(f.y_mm),
                h_text: format_m(f.h_mm),
                dx_text: format_diff_mm(f.dx_mm),
                dy_text: format_diff_mm(f.dy_mm),
                dh_text: format_diff_mm(f.dh_mm),
            }),
        }
    }

    /// お知らせを出す。同じ文を続けて出すときは間をあける（出したら true）
    fn notify(&mut self, kind: NoticeKind, text: String) -> bool {
        if let Some(last) = &self.notice {
            if last.text == text && self.now_ms - self.notice_time < WARNING_REPEAT_MS {
                return false;
            }
        }
        self.notice_seq += 1;
        self.notice = Some(Notice {
            id: self.notice_seq,
            kind,
            text,
        });
        self.notice_time = self.now_ms;
        true
    }

    // ------------------------------------------------------------------
    // AI先輩：操作の記録と事実
    // ------------------------------------------------------------------

    /// 操作の記録（テスト・確認用）
    pub fn coach_log(&self) -> &CoachLog {
        &self.coach
    }

    /// 整準ネジで近づけたい値と、その許容（教科書モード手順3は求心ズレ mm、ほかは電子気泡の大きさ 秒）
    fn coach_objective(&self) -> Option<(f64, f64)> {
        let pose = self.inst.pose(&self.tripod.head)?;
        if self.mode == Mode::Textbook && self.guided_step == 3 {
            let l = pose.laser_point()?;
            Some((l.x.hypot(l.y), TEXTBOOK_SCREW_CENTER_MM))
        } else {
            let (x, y) = pose.tilt_xy();
            Some((to_arcsec(x).hypot(to_arcsec(y)), TILT_TOLERANCE_SEC))
        }
    }

    /// ループ（器械ずらし）を始める直前の求心ズレと電子気泡
    fn loop_record(&self) -> LoopRecord {
        let pose = self.inst.pose(&self.tripod.head);
        LoopRecord {
            centering_mm: pose
                .as_ref()
                .and_then(|p| p.laser_point())
                .map(|l| l.x.hypot(l.y)),
            tilt_sec: pose.map(|p| {
                let (x, y) = p.tilt_xy();
                to_arcsec(x).abs().max(to_arcsec(y).abs())
            }),
        }
    }

    /// このフレームの操作を記録する（据付中でタイマーが動いている間だけ）
    fn update_coach(&mut self, dt_ms: f64, before: Option<(f64, f64)>) {
        if self.phase != Phase::Setup || !self.timer.running() {
            return;
        }
        let after = self.coach_objective();
        let screws = self.frame_screws.iter().filter(|s| **s).count();
        let (worse, better) = match (before, after) {
            (Some((b, _)), Some((a, tol))) if screws > 0 => (a > b + 1e-6 && a > tol, a < b - 1e-6),
            _ => (false, false),
        };
        let kind = self.frame_ops.iter().find_map(|k| match k {
            OpKind::MoveLegs => Some(LogKind::MoveLegs),
            OpKind::PressLeg => Some(LogKind::PressLeg),
            OpKind::LegLength => Some(LogKind::LegLength),
            OpKind::Mount => Some(LogKind::Mount),
            OpKind::Clamp => Some(LogKind::Clamp),
            OpKind::Slide => Some(LogKind::Slide),
            OpKind::Screw => None,
        });
        let has = |k: OpKind| self.frame_ops.contains(&k);
        let summary = FrameSummary {
            step: self.current_step(),
            dt_ms,
            t_ms: self.timer.elapsed(self.now_ms),
            kind,
            screws,
            worse,
            better,
            leg_length: has(OpKind::LegLength),
            slide: has(OpKind::Slide),
            move_legs: has(OpKind::MoveLegs),
            screw_limit: self.frame_refusals.contains(&Refusal::ScrewLimit),
            clamp_refused: self.frame_refusals.contains(&Refusal::ClampTight),
        };
        self.coach.account(&summary);
    }

    /// AI先輩に渡す事実（まだ出せないときは None）
    pub fn ai_facts(&self, kind: FactKind) -> Option<Vec<Fact>> {
        match kind {
            FactKind::Review => self.review_facts(),
            FactKind::Status => Some(self.status_facts()),
        }
    }

    /// いまの手順の名前
    fn step_label(&self, step: u8) -> &'static str {
        let i = (step as usize).saturating_sub(1);
        let t = match (self.phase, self.mode) {
            (Phase::Observation, _) => observation_steps().get(i.saturating_sub(7)).copied(),
            (_, Mode::Textbook) => textbook_steps().get(i).copied(),
            (_, Mode::Field) => field_steps().get(i).copied(),
        };
        t.map(|t| t.label).unwrap_or("")
    }

    /// いまの状態と次の操作（何でも質問のときにAIに渡す）
    fn status_facts(&self) -> Vec<Fact> {
        let s = self.snapshot();
        let step = self.current_step();
        let mut f = vec![
            fact(
                "モード",
                match self.mode {
                    Mode::Textbook => "教科書モード（教科書の手順を一歩ずつ）",
                    Mode::Field => "現場モード（現場の手順。順番は自由）",
                },
            ),
            fact(
                "いまの場面",
                match self.phase {
                    Phase::Setup => "第1部：据付",
                    Phase::SetupResult => "「据付完了」を押したあとの結果画面",
                    Phase::Observation => "第2部：観測",
                },
            ),
            fact(
                "いまの手順",
                format!("手順{}「{}」", step, self.step_label(step)),
            ),
        ];
        if self.phase != Phase::Observation {
            f.push(fact("タイム", format_time(s.elapsed_ms)));
            f.push(fact(
                "求心ズレ",
                s.centering_mm
                    .map(|c| {
                        format!(
                            "{:.1} mm（許容{} mm以内）",
                            centering_display_mm(c),
                            CENTERING_TOLERANCE_MM
                        )
                    })
                    .unwrap_or_else(|| "---（器械がまだ載っていない）".into()),
            ));
            f.push(fact(
                "電子気泡",
                s.electronic
                    .map(|e| {
                        format!(
                            "X {}・Y {}（許容±{:.0}″以内）",
                            format_dms_signed(e.x),
                            format_dms_signed(e.y),
                            TILT_TOLERANCE_SEC
                        )
                    })
                    .unwrap_or_else(|| "---".into()),
            ));
            f.push(fact(
                "円形気泡",
                match s.circular {
                    Some(c) if c.in_circle => "中心の円に入っている",
                    Some(_) => "中心の円に入っていない",
                    None => "---",
                },
            ));
            let planted = s.planted.iter().filter(|p| **p).count();
            f.push(fact("脚の踏み込み", format!("{}本／3本", planted)));
            f.push(fact(
                "器械",
                match (s.mounted, s.clamp_tight) {
                    (false, _) => "まだ載っていない",
                    (true, true) => "載っている・定心桿は締めている",
                    (true, false) => "載っている・定心桿は緩めている",
                },
            ));
            f.push(fact("ループ回数", format!("{}回", self.loops)));
            if self.mode == Mode::Textbook && self.phase == Phase::Setup {
                let (goal, ok) = self.textbook_goal(self.guided_step, &s);
                f.push(fact(
                    "この手順の目標",
                    format!("{}（{}）", goal, if ok { "できている" } else { "まだ" }),
                ));
            }
        } else {
            let name = |i: Option<usize>| {
                i.and_then(|i| KNOWN_POINTS.get(i))
                    .map(|p| p.name)
                    .unwrap_or("まだ選んでいない")
            };
            f.push(fact("電子平板の機械点", name(self.obs.station)));
            f.push(fact("電子平板のBS点", name(self.obs.backsight)));
            f.push(fact(
                "電子平板の器械高",
                self.obs
                    .instrument_height_mm
                    .map(|h| format!("{} m", format_m(h)))
                    .unwrap_or_else(|| "まだ入れていない".into()),
            ));
            f.push(fact(
                "観測",
                format!(
                    "BS {}・FS {}",
                    if self.obs.bs.is_some() { "観測済み" } else { "まだ" },
                    if self.obs.fs.is_some() { "観測済み" } else { "まだ" }
                ),
            ));
            if let Some(o) = s.optics {
                f.push(fact(
                    "のぞき方",
                    if o.peep_sight { "照準器" } else { "望遠鏡" },
                ));
                f.push(fact(
                    "十字線・ピント",
                    format!(
                        "十字線は{}・ピントは{}",
                        if o.reticle_ok { "くっきり" } else { "ぼやけている" },
                        if o.focus_ok { "合っている" } else { "合っていない" }
                    ),
                ));
            }
            if let Some(v) = s.sight {
                f.push(fact(
                    "十字線とターゲットのずれ",
                    format!(
                        "水平{:+.0}″・鉛直{:+.0}″（{:.0}″以内で視準OK）",
                        v.dh_sec, v.dv_sec, SIGHT_OK_SEC
                    ),
                ));
            }
        }
        f.push(fact("次の操作", self.next_action()));
        f.push(fact(
            "整準ネジの並び",
            "A＝左手前、B＝右手前、C＝奥（操作する人は手前に立つ）",
        ));
        f.push(fact(
            "左手親指の法則",
            "整準ネジ2本を同時に内側または外側へ回すと、気泡は左手の親指を動かす向きに動く",
        ));
        f.push(fact(
            "6⇄7を繰り返すわけ",
            "整準すると器械の傾きが変わってレーザー求心点が動き、器械をずらすと気泡が少し崩れる。だから求心と電子気泡の両方が許容に入るまで繰り返す",
        ));
        f
    }

    /// 次の操作（コアが決める。AIはこれを言葉にするだけ）
    pub fn next_action(&self) -> String {
        let s = self.snapshot();
        match self.phase {
            Phase::SetupResult => {
                if self.judgement.as_ref().map(|j| j.passed).unwrap_or(false) {
                    "結果画面の「第2部：観測へ進む」を押す".into()
                } else {
                    "結果画面の「続行する」を押して、不合格の理由を直す".into()
                }
            }
            Phase::Observation => self.observation_action(&s),
            Phase::Setup => match self.mode {
                Mode::Field => self.field_action(&s),
                Mode::Textbook => self.textbook_action(&s),
            },
        }
    }

    /// 据付の仕上げ：円形気泡 → 電子気泡 → 求心 → 定心桿 → 据付完了
    fn finish_action(&self, s: &Snapshot) -> String {
        if s.circular.map(|c| !c.in_circle).unwrap_or(false) {
            return coach::leg_hint(&self.tripod, &self.inst)
                .unwrap_or_else(|| "脚の伸縮で、円形気泡を中心の円に入れる".into());
        }
        if !s.electronic.map(|e| e.ok).unwrap_or(false) {
            return coach::level_hint(&self.inst, &self.tripod.head)
                .unwrap_or_else(|| "整準ネジで、電子気泡をX・Yとも±15″以内に合わせる".into());
        }
        if !s.centering_ok {
            if let Some(c) = s.centering {
                return coach::slide_hint(c, s.clamp_tight);
            }
        }
        if !s.clamp_tight {
            return "定心桿を締める".into();
        }
        "求心と電子気泡の両方が許容に入ったので「据付完了」を押す".into()
    }

    fn field_action(&self, s: &Snapshot) -> String {
        let planted = s.planted.iter().filter(|p| **p).count();
        if planted < 3 {
            let p = match s.laser {
                Some(l) => [l.x, l.y],
                None => [s.head_center.x, s.head_center.y],
            };
            let d = p[0].hypot(p[1]);
            let dir = coach::direction_words([-p[0], -p[1]]);
            return if planted == 0 {
                if d > HINT_PLACE_MM {
                    format!(
                        "三脚ごと{}へ約{:.0} mm運んで、レーザー点を測点に近づける",
                        dir, d
                    )
                } else {
                    "レーザー点が測点に近いので、どれか1本の脚を踏み込む".into()
                }
            } else if d > HINT_ROUGH_CENTER_MM {
                format!(
                    "踏み込んでいない脚を{}の方へ動かして、レーザー点を測点に近づける（いまのずれ約{:.0} mm）",
                    dir, d
                )
            } else {
                "レーザー点が測点に近いので、残りの脚も踏み込む".into()
            };
        }
        self.finish_action(s)
    }

    fn textbook_action(&self, s: &Snapshot) -> String {
        let (_, goal_ok) = self.textbook_goal(self.guided_step, s);
        if goal_ok && self.guided_step < 7 {
            return "この手順の目標はできているので「次の手順へ」を押す".into();
        }
        let head = &self.tripod.head;
        match self.guided_step {
            1 => {
                let all = s.planted.iter().all(|p| *p);
                if !all && s.head_offset_mm > TEXTBOOK_ROUGH_CENTER_MM {
                    let c = s.head_center;
                    format!(
                        "三脚を{}へ約{:.0} mm運んで、脚頭の中心を測点の真上に近づける",
                        coach::direction_words([-c.x, -c.y]),
                        s.head_offset_mm
                    )
                } else if s.head_tilt_deg > TEXTBOOK_ROUGH_HEAD_TILT * 180.0 / PI {
                    coach::leg_hint(&self.tripod, &self.inst)
                        .unwrap_or_else(|| "脚の伸縮で、脚頭をだいたい水平にする".into())
                } else if !all {
                    "脚を3本とも踏み込む".into()
                } else {
                    "脚頭の中心が測点から離れたまま踏み込んだので、「最初から」やり直すとよい".into()
                }
            }
            2 => if s.mounted {
                "定心桿を締める"
            } else {
                "「器械を載せて定心桿で固定する」を押す"
            }
            .into(),
            3 => coach::screw_center_hint(&self.inst, head)
                .unwrap_or_else(|| "整準ネジで、レーザー点を測点に合わせる".into()),
            4 => coach::leg_hint(&self.tripod, &self.inst)
                .unwrap_or_else(|| "脚の伸縮で、円形気泡を中心の円に入れる".into()),
            5 => coach::level_hint(&self.inst, head)
                .unwrap_or_else(|| "整準ネジで、電子気泡をX・Yとも±15″以内に合わせる".into()),
            6 => match s.centering {
                Some(c) if !s.centering_ok => coach::slide_hint(c, s.clamp_tight),
                _ => "定心桿を締める".into(),
            },
            _ => self.finish_action(s),
        }
    }

    fn observation_action(&self, s: &Snapshot) -> String {
        let step = self.selected_step;
        if step == 8 {
            return "電子平板で、機械点（据えた点）とBS点（ターゲットが立っている点）を選び、器械高を巻尺で測って入れて「セット」を押す".into();
        }
        if step == 10 && self.obs.fs.is_some() {
            return "FSの観測は終わり。観測した座標と設計値との差を確かめる".into();
        }
        let target = if step == 9 {
            backsight_target_world()
        } else {
            foresight_target_world()
        };
        let (Some(o), Some(v), Some(eye)) = (s.optics, s.sight, s.trunnion) else {
            return "器械が載っていない".into();
        };
        let dir = sight_words(v.dh_sec, v.dv_sec);
        let rough = v.dh_sec.abs().max(v.dv_sec.abs()) > HINT_ROUGH_SIGHT_SEC;
        if o.peep_sight {
            return if rough {
                format!("照準器をのぞいたまま、望遠鏡を{}へ回してターゲットを捉える", dir)
            } else {
                "ターゲットを捉えたので、望遠鏡に切り替える（Pキー）".into()
            };
        }
        if !o.reticle_ok {
            return "接眼レンズを回して、十字線をくっきりさせる".into();
        }
        if rough {
            return format!(
                "ターゲットが望遠鏡の視野から遠い（{}の方）ので、照準器（Pキー）で大まかに向ける",
                dir
            );
        }
        if !o.focus_ok {
            let d_m = (target - eye).norm() / 1000.0;
            let near = d_m > 0.0 && self.obs.focus_per_m < 1.0 / d_m;
            return format!(
                "合焦つまみを「{}」へ回して、ターゲットのピントを合わせる",
                if near { "近くへ" } else { "遠くへ" }
            );
        }
        if !v.ok {
            return format!(
                "望遠鏡を{}へ動かして、十字線をターゲットの中心に合わせる（いま水平{:+.0}″・鉛直{:+.0}″）",
                dir, v.dh_sec, v.dv_sec
            );
        }
        "十字線がターゲットに合ったので「測量」を押す".into()
    }

    fn review_facts(&self) -> Option<Vec<Fact>> {
        let j = self.judgement.as_ref()?;
        let v = judgement_view(j);
        let steps = match self.mode {
            Mode::Textbook => textbook_steps(),
            Mode::Field => field_steps(),
        };
        Some(coach::review_facts(&ReviewInput {
            textbook: self.mode == Mode::Textbook,
            passed: v.passed,
            rating: v.rating_text.as_deref(),
            time_text: &v.time_text,
            loops: v.loops,
            centering_text: &v.centering_text,
            tilt_x_text: &v.tilt_x_text,
            tilt_y_text: &v.tilt_y_text,
            clamp_tight: j.clamp_tight,
            reasons: &v.reasons,
            steps: &steps,
            log: &self.coach,
        }))
    }

    fn signature(&self) -> UiSignature {
        let goal_ok = if self.mode == Mode::Textbook && self.phase == Phase::Setup {
            let snap = self.snapshot();
            self.textbook_goal(self.guided_step, &snap).1
        } else {
            false
        };
        UiSignature {
            phase: match self.phase {
                Phase::Setup => 0,
                Phase::SetupResult => 1,
                Phase::Observation => 2,
            },
            step: self.current_step(),
            field_done: self.field_done,
            planted: self.tripod.planted,
            mounted: self.inst.mounted,
            clamp: self.inst.clamp_tight,
            notice_id: self.notice.as_ref().map(|n| n.id).unwrap_or(0),
            judgement: self.judgement.is_some(),
            obs_rev: self.obs_rev,
            goal_ok,
            loops: self.loops,
        }
    }

    fn refresh_revision(&mut self) {
        let sig = self.signature();
        if sig != self.ui_sig {
            self.ui_sig = sig;
            self.ui_revision += 1;
        }
    }
}

/// 望遠鏡を動かす向きの言葉（ずれ＝ターゲットが十字線の右・上にあると＋）
fn sight_words(dh_sec: f64, dv_sec: f64) -> &'static str {
    let h = dh_sec.abs() > SIGHT_OK_SEC;
    let v = dv_sec.abs() > SIGHT_OK_SEC;
    match (h, v, dh_sec > 0.0, dv_sec > 0.0) {
        (true, true, true, true) => "右上",
        (true, true, true, false) => "右下",
        (true, true, false, true) => "左上",
        (true, true, false, false) => "左下",
        (true, false, true, _) => "右",
        (true, false, false, _) => "左",
        (false, true, _, true) => "上",
        (false, true, _, false) => "下",
        (false, false, _, _) => "そのまま",
    }
}

fn sight_view(target: u8, obs: &Observation, eye: Vec3, goal: Vec3) -> SightView {
    let (dh, dv) = sight_deviation(obs.hz, obs.zenith, eye, goal);
    let dh_sec = to_arcsec(dh);
    let dv_sec = to_arcsec(dv);
    SightView {
        target,
        dh_sec,
        dv_sec,
        ok: dh_sec.abs() <= SIGHT_OK_SEC && dv_sec.abs() <= SIGHT_OK_SEC,
    }
}

/// ピントと十字線の見え方。`distance_m`＝器械からターゲットまでの距離 [m]
fn optics_view(obs: &Observation, distance_m: f64) -> OpticsView {
    let target_per_m = if distance_m > 0.0 { 1.0 / distance_m } else { 0.0 };
    let defocus = (obs.focus_per_m - target_per_m).abs();
    let reticle_off = (obs.eyepiece_dpt - obs.eye_dpt).abs();
    OpticsView {
        peep_sight: obs.peep_sight,
        // 見え方は平方根に比例：合う手前でも、わずかなぼけが見える
        target_blur: (defocus / FOCUS_BLUR_FULL_PER_M).min(1.0).sqrt(),
        reticle_blur: (reticle_off / EYEPIECE_BLUR_FULL_DPT).min(1.0).sqrt(),
        focus_ok: defocus <= FOCUS_TOLERANCE_PER_M,
        reticle_ok: reticle_off <= EYEPIECE_TOLERANCE_DPT,
    }
}

/// 望遠鏡でのぞいていて、十字線もターゲットもくっきり見えていること
fn check_optics(snap: &Snapshot) -> Result<(), Refusal> {
    match snap.optics {
        Some(o) if o.peep_sight => Err(Refusal::PeepSight),
        Some(o) if !o.reticle_ok => Err(Refusal::ReticleBlurry),
        Some(o) if !o.focus_ok => Err(Refusal::OutOfFocus),
        Some(_) => Ok(()),
        None => Err(Refusal::BadIndex),
    }
}

fn check_sight(snap: &Snapshot) -> Result<(), Refusal> {
    match snap.sight {
        Some(s) if s.ok => Ok(()),
        Some(s) => Err(Refusal::SightOff {
            dh_sec: s.dh_sec,
            dv_sec: s.dv_sec,
        }),
        None => Err(Refusal::BadIndex),
    }
}

fn judgement_view(j: &SetupJudgement) -> JudgementView {
    let rating_text = if j.passed {
        Some(if j.time_ms <= TIME_EXPERT_MS {
            "熟練（2分以内）".to_string()
        } else if j.time_ms <= TIME_PASS_MS {
            "合格の目安（3分以内）".to_string()
        } else {
            "合格（目安の3分を超えました）".to_string()
        })
    } else {
        None
    };
    JudgementView {
        passed: j.passed,
        reasons: j.reasons.clone(),
        time_ms: j.time_ms,
        time_text: format_time(j.time_ms),
        loops: j.loops,
        centering_text: j
            .centering_mm
            .map(|c| format!("{} mm", fmt_tenths(tenths(c))))
            .unwrap_or_else(|| "---".into()),
        tilt_x_text: j
            .tilt
            .map(|t| format_dms_signed(t.0))
            .unwrap_or_else(|| "---".into()),
        tilt_y_text: j
            .tilt
            .map(|t| format_dms_signed(t.1))
            .unwrap_or_else(|| "---".into()),
        rating_text,
    }
}

/// 教科書モードの手順 step で許される操作か
fn textbook_allows(step: u8, kind: OpKind) -> bool {
    use OpKind::*;
    match step {
        1 => matches!(kind, MoveLegs | PressLeg | LegLength),
        2 => matches!(kind, Mount | Clamp),
        3 => matches!(kind, Screw),
        4 => matches!(kind, LegLength),
        5 => matches!(kind, Screw),
        6 => matches!(kind, Clamp | Slide),
        7 => matches!(kind, Screw | Clamp | Slide),
        _ => false,
    }
}

/// 教科書モードで、その操作を主に行う手順
fn textbook_step_for(kind: OpKind) -> Option<u8> {
    match kind {
        OpKind::MoveLegs | OpKind::PressLeg => Some(1),
        OpKind::Mount => Some(2),
        OpKind::LegLength => Some(4),
        OpKind::Screw => Some(5),
        OpKind::Clamp | OpKind::Slide => Some(6),
    }
}

fn unit2(d: [f64; 2]) -> [f64; 2] {
    let n = d[0].hypot(d[1]);
    if !n.is_finite() || n < 1e-12 {
        [0.0, 0.0]
    } else if n > 1.0 {
        [d[0] / n, d[1] / n]
    } else {
        d
    }
}

fn randomize_instrument(inst: &mut Instrument, rng: &mut Rng) {
    inst.mounted = true;
    inst.clamp_tight = true;
    inst.slide = [
        rng.jitter(SLIDE_START_JITTER_MM),
        rng.jitter(SLIDE_START_JITTER_MM),
    ];
    inst.screw_turns = [
        rng.jitter(SCREW_START_JITTER),
        rng.jitter(SCREW_START_JITTER),
        rng.jitter(SCREW_START_JITTER),
    ];
    inst.seat_tilt = [0.0; 2];
}

/// 開始時の三脚：測点から少し離れた所に、少し不揃いに立っている
fn random_tripod(rng: &mut Rng) -> Tripod {
    for _ in 0..20 {
        let dist = rng.range(START_OFFSET_MIN_MM, START_OFFSET_MAX_MM);
        let ang = rng.range(0.0, 2.0 * PI);
        let (cx, cy) = (dist * ang.cos(), dist * ang.sin());
        let feet = LEG_AZIMUTHS.map(|a| {
            let r = FOOT_RADIUS_START_MM + rng.jitter(FOOT_RADIUS_START_JITTER_MM);
            let az = a + rng.jitter(FOOT_AZIMUTH_JITTER);
            Vec3::new(cx + r * az.cos(), cy + r * az.sin(), 0.0)
        });
        let lens = [0, 1, 2].map(|_| LEG_LENGTH_START_MM + rng.jitter(LEG_LENGTH_START_JITTER_MM));
        if let Ok(t) = Tripod::new(feet, lens) {
            return t;
        }
    }
    default_tripod()
}

fn default_tripod() -> Tripod {
    let feet = LEG_AZIMUTHS.map(|a| {
        Vec3::new(
            300.0 + FOOT_RADIUS_START_MM * a.cos(),
            FOOT_RADIUS_START_MM * a.sin(),
            0.0,
        )
    });
    match Tripod::new(feet, [LEG_LENGTH_START_MM; 3]) {
        Ok(t) => t,
        Err(_) => Tripod {
            feet,
            leg_len: [LEG_LENGTH_START_MM; 3],
            planted: [false; 3],
            head: crate::tripod::HeadPose {
                center: Vec3::new(300.0, 0.0, 1250.0),
                rot: crate::math::Mat3::IDENTITY,
            },
        },
    }
}
