//! AI先輩に渡す「事実」：操作の記録とその集計、遠回りの判定。
//!
//! 正解はRust、言葉はAI：合否・数値・向き・きっかけはここ（コア）で決め、
//! AI（中継）には、ここで作った事実を言葉にさせるだけにする。
//! 基準タイムなどの目安は config.rs にまとめる（値はすべて仮）。

use crate::angle::{format_dms_signed, to_arcsec};
use crate::config::{
    CENTERING_TOLERANCE_MM, FIELD_STEP_REF_SEC, LEG_LENGTH_MAX_MM, LEG_LENGTH_MIN_MM, OP_LOG_MAX,
    REVIEW_LOOPS_REF, REVIEW_WRONG_WAY_SEC, SLIDE_BEFORE_LEVEL_SEC, STEP_VISITS_MAX,
    TEXTBOOK_STEP_REF_SEC, TILT_TOLERANCE_SEC,
};
use crate::i18n::{pick, Lang};
use crate::instrument::{CircularBubble, Instrument, InstrumentPose, Thumb};
use crate::steps::StepText;
use crate::tripod::{HeadPose, Tripod};

/// AIに渡す事実1件（見出しと値。どちらも短い日本語）
#[derive(Clone, Debug, PartialEq)]
pub struct Fact {
    pub key: String,
    pub value: String,
}

pub fn fact(key: impl Into<String>, value: impl Into<String>) -> Fact {
    Fact {
        key: key.into(),
        value: value.into(),
    }
}

/// 記録する操作の種類（整準ネジは1本ずつか2本同時かを分ける）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    MoveLegs,
    PressLeg,
    LegLength,
    Mount,
    Screw1,
    Screw2,
    Clamp,
    Slide,
}

impl LogKind {
    pub fn label(self) -> &'static str {
        match self {
            LogKind::MoveLegs => "三脚・脚を動かす",
            LogKind::PressLeg => "脚の踏み込み",
            LogKind::LegLength => "脚の伸縮",
            LogKind::Mount => "器械を載せる",
            LogKind::Screw1 => "整準ネジ1本",
            LogKind::Screw2 => "整準ネジ2本同時",
            LogKind::Clamp => "定心桿",
            LogKind::Slide => "器械ずらし",
        }
    }
}

/// 操作の記録の1区切り（操作の種類か手順が変わるごとに1件）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub step: u8,
    pub kind: LogKind,
    /// 始めた時刻（タイムの経過 ms）
    pub start_ms: f64,
    pub dur_ms: f64,
}

/// ループ（手順6の器械ずらし）を始める直前の状態
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopRecord {
    pub centering_mm: Option<f64>,
    /// 電子気泡 X・Y の大きいほう [秒]
    pub tilt_sec: Option<f64>,
}

/// 1フレーム分の操作のまとめ（コアの frame() が作って渡す）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSummary {
    pub step: u8,
    pub dt_ms: f64,
    /// このフレームの終わりのタイム [ms]
    pub t_ms: f64,
    /// 記録する操作（整準ネジ以外で最初に行ったもの）
    pub kind: Option<LogKind>,
    /// 回した整準ネジの本数
    pub screws: usize,
    /// 整準ネジを回して、目標（電子気泡、教科書モード手順3は求心）が悪くなった
    pub worse: bool,
    /// 整準ネジを回して、目標が良くなった
    pub better: bool,
    pub leg_length: bool,
    pub slide: bool,
    pub move_legs: bool,
    pub screw_limit: bool,
    pub clamp_refused: bool,
}

/// 操作の記録とその集計
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CoachLog {
    pub segments: Vec<Segment>,
    /// 記録が上限に達して、それ以降を残していないか
    pub truncated: bool,
    /// 手順ごとの時間（タイマーが動いている間）[ms]
    pub step_ms: [f64; 7],
    /// 手順の流れ（手順, その手順にいた時間 ms）
    pub visits: Vec<(u8, f64)>,
    pub screw_single_ms: f64,
    pub screw_dual_ms: f64,
    /// 目標が悪くなる向きに整準ネジを回していた時間 [ms]
    pub screw_worse_ms: f64,
    /// 悪くなる向きに続けて回している時間 [ms]（良くなる操作をすると0に戻る）
    pub wrong_run_ms: f64,
    pub leg_length_ms: f64,
    pub slide_ms: f64,
    pub move_legs_ms: f64,
    pub loops: Vec<LoopRecord>,
    /// 電子気泡が大きくずれたまま器械をずらし始めた回数
    pub slide_before_level: u32,
    /// 定心桿を締めたまま器械をずらそうとした回数
    pub clamp_refusals: u32,
    /// 整準ネジを回しきった回数
    pub screw_limit_hits: u32,
    /// 教科書モードの「手順外」の注意の回数
    pub textbook_warnings: u32,
    /// 不合格だった「据付完了」の回数
    pub failed_completions: u32,
    prev_screw_limit: bool,
    prev_clamp_refused: bool,
}

impl CoachLog {
    /// 1フレーム分を記録する（タイマーが動いている据付中だけ呼ぶ）
    pub fn account(&mut self, f: &FrameSummary) {
        let dt = f.dt_ms.max(0.0);
        let si = (f.step as usize).clamp(1, 7) - 1;
        self.step_ms[si] += dt;
        let n = self.visits.len();
        match self.visits.last_mut() {
            Some(v) if v.0 == f.step => v.1 += dt,
            _ if n < STEP_VISITS_MAX => self.visits.push((f.step, dt)),
            _ => self.truncated = true,
        }

        let kind = match f.screws {
            0 => f.kind,
            1 => Some(LogKind::Screw1),
            _ => Some(LogKind::Screw2),
        };
        if let Some(kind) = kind {
            self.record_segment(f.step, kind, f.t_ms, dt);
        }
        match f.screws {
            0 => {}
            1 => self.screw_single_ms += dt,
            _ => self.screw_dual_ms += dt,
        }
        if f.screws > 0 {
            if f.worse {
                self.screw_worse_ms += dt;
                self.wrong_run_ms += dt;
            } else if f.better {
                self.wrong_run_ms = 0.0;
            }
        }
        if f.leg_length {
            self.leg_length_ms += dt;
        }
        if f.slide {
            self.slide_ms += dt;
        }
        if f.move_legs {
            self.move_legs_ms += dt;
        }
        // 押し続けて毎フレーム断られても、1回と数える
        if f.screw_limit && !self.prev_screw_limit {
            self.screw_limit_hits += 1;
        }
        if f.clamp_refused && !self.prev_clamp_refused {
            self.clamp_refusals += 1;
        }
        self.prev_screw_limit = f.screw_limit;
        self.prev_clamp_refused = f.clamp_refused;
    }

    fn record_segment(&mut self, step: u8, kind: LogKind, t_ms: f64, dt: f64) {
        if let Some(last) = self.segments.last_mut() {
            // 同じ操作を続けているなら（間が0.3秒以内）、その区切りを延ばす。
            // 現場モードは状態で手順が変わるので、途中で手順が変わっても1件のまま（手順は始めたときのもの）
            let gap = t_ms - dt - (last.start_ms + last.dur_ms);
            if last.kind == kind && gap <= 300.0 {
                last.dur_ms = (t_ms - last.start_ms).max(last.dur_ms);
                return;
            }
        }
        if self.segments.len() >= OP_LOG_MAX {
            self.truncated = true;
            return;
        }
        self.segments.push(Segment {
            step,
            kind,
            start_ms: (t_ms - dt).max(0.0),
            dur_ms: dt,
        });
    }

    /// ループ（器械ずらし）を始めるときの記録
    pub fn start_loop(&mut self, rec: LoopRecord) {
        if rec.tilt_sec.map(|t| t > SLIDE_BEFORE_LEVEL_SEC).unwrap_or(false) {
            self.slide_before_level += 1;
        }
        self.loops.push(rec);
    }
}

// ---------------------------------------------------------------------------
// 表示用の文字列（AIに渡す値）
// ---------------------------------------------------------------------------

/// 秒の表示。10秒未満は小数1桁。例：`4.2秒`、`48秒`
pub fn format_sec(ms: f64) -> String {
    let s = ms.max(0.0) / 1000.0;
    if s < 10.0 {
        format!("{:.1}秒", s)
    } else {
        format!("{:.0}秒", s)
    }
}

/// m 表示（符号つき・小数3桁）。例：`+12.345`、`−3.210`（マイナスは「−」U+2212）
pub fn format_signed_m(mm: f64) -> String {
    let v = mm / 1000.0;
    let s = format!("{:.3}", v.abs());
    if s == "0.000" {
        "±0.000".into()
    } else if v > 0.0 {
        format!("+{}", s)
    } else {
        format!("\u{2212}{}", s)
    }
}

/// 読み上げ用の文：数字の入ったかっこ（…）・(...) を除く（いまの角度などの数値は読まない。
/// 「脚3（右手前）」のような、数字の無いかっこ書きは残す）
pub fn speech_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut group = String::new();
    let mut depth = 0usize;
    for c in s.chars() {
        match c {
            '（' | '(' => {
                depth += 1;
                group.push(c);
            }
            '）' | ')' if depth > 0 => {
                group.push(c);
                depth -= 1;
                if depth == 0 {
                    if !group.chars().any(|g| g.is_ascii_digit()) {
                        out.push_str(&group);
                    } else if out.ends_with(' ') {
                        // 英語の「… (within 1mm).」の前の空白も除く
                        out.pop();
                    }
                    group.clear();
                }
            }
            _ if depth > 0 => group.push(c),
            _ => out.push(c),
        }
    }
    // 閉じていないかっこは、そのまま残す
    out.push_str(&group);
    out
}

/// 「次の操作」の種類（かっこ書きと数値を除いたもの。数値が変わっただけでは、別の操作としない）
pub fn action_key(s: &str) -> String {
    speech_text(s)
        .chars()
        .filter(|c| !c.is_ascii_digit() && !matches!(c, '.' | '+' | '-' | '\u{2212}' | '°' | '′' | '″'))
        .collect::<String>()
        .replace("約", "")
        .replace(" mm", "")
}

/// 手順の基準タイム [ms]（モードごと。仮）
pub fn step_ref_ms(textbook: bool, step: u8) -> f64 {
    let refs = if textbook {
        TEXTBOOK_STEP_REF_SEC
    } else {
        FIELD_STEP_REF_SEC
    };
    refs[(step as usize).clamp(1, 7) - 1] * 1000.0
}

/// 文字数で切る（AIに渡す値の長さの上限に合わせる）
pub fn clip(s: String, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s
    } else {
        let mut t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// 終わりのほうを残して切る（流れの記録用）
fn clip_front(items: &[String], sep: &str, max_chars: usize) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut len = 0;
    for s in items.iter().rev() {
        let add = s.chars().count() + sep.chars().count();
        if len + add > max_chars.saturating_sub(1) {
            out.push("…");
            break;
        }
        len += add;
        out.push(s);
    }
    out.reverse();
    out.join(sep)
}

/// 「据付完了」のあとの講評に渡す材料
pub struct ReviewInput<'a> {
    pub textbook: bool,
    pub passed: bool,
    pub rating: Option<&'a str>,
    pub time_text: &'a str,
    pub loops: u32,
    pub centering_text: &'a str,
    pub tilt_x_text: &'a str,
    pub tilt_y_text: &'a str,
    pub clamp_tight: bool,
    pub reasons: &'a [String],
    pub steps: &'a [StepText; 7],
    pub log: &'a CoachLog,
}

/// 遠回りした箇所（コアで判定する。AIはこの中から言葉にする）
pub fn detours(r: &ReviewInput) -> Vec<String> {
    let log = r.log;
    let mut out = Vec::new();
    if log.screw_worse_ms >= REVIEW_WRONG_WAY_SEC * 1000.0 {
        out.push(format!(
            "電子気泡が悪くなる向きに整準ネジを回していた時間が、合計{}",
            format_sec(log.screw_worse_ms)
        ));
    }
    if log.slide_before_level > 0 {
        out.push(format!(
            "電子気泡が±{:.0}″より大きくずれたまま、器械をずらした（{}回）",
            SLIDE_BEFORE_LEVEL_SEC, log.slide_before_level
        ));
    }
    if r.loops > REVIEW_LOOPS_REF {
        out.push(format!(
            "ループ（6⇄7）が{}回（目安は{}回以内）",
            r.loops, REVIEW_LOOPS_REF
        ));
    }
    if log.screw_limit_hits > 0 {
        out.push(format!("整準ネジを回しきった（{}回）", log.screw_limit_hits));
    }
    // 目安を一番大きく超えた手順を2つまで
    let mut over: Vec<(usize, f64)> = (0..7)
        .filter_map(|i| {
            let r_ms = step_ref_ms(r.textbook, (i + 1) as u8);
            let ms = log.step_ms[i];
            (ms > r_ms && r_ms > 0.0).then_some((i, ms / r_ms))
        })
        .collect();
    over.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (i, _) in over.into_iter().take(2) {
        out.push(format!(
            "手順{}「{}」に{}かかった（目安{}）",
            i + 1,
            r.steps[i].short,
            format_sec(log.step_ms[i]),
            format_sec(step_ref_ms(r.textbook, (i + 1) as u8))
        ));
    }
    if log.failed_completions > 0 && r.passed {
        out.push(format!(
            "許容に入る前に「据付完了」を押した（{}回）",
            log.failed_completions
        ));
    }
    if log.clamp_refusals > 0 {
        out.push(format!(
            "定心桿を締めたまま器械をずらそうとした（{}回）",
            log.clamp_refusals
        ));
    }
    if r.textbook && log.textbook_warnings > 0 {
        out.push(format!(
            "手順外の操作をして注意が出た（{}回）",
            log.textbook_warnings
        ));
    }
    out.truncate(5);
    out
}

/// 「据付完了」のあとの講評（AI先輩）に渡す事実
pub fn review_facts(r: &ReviewInput) -> Vec<Fact> {
    let log = r.log;
    let mut f = vec![
        fact(
            "モード",
            if r.textbook {
                "教科書モード（教科書の手順を一歩ずつ）"
            } else {
                "現場モード（現場の手順。順番は自由）"
            },
        ),
        fact("結果", if r.passed { "合格" } else { "不合格" }),
    ];
    if let Some(rating) = r.rating {
        f.push(fact("評価", rating));
    }
    f.push(fact(
        "タイム",
        format!("{}（目安：3分で合格、2分で熟練）", r.time_text),
    ));
    f.push(fact(
        "ループ回数",
        format!("{}回（手順6で器械をずらした回数）", r.loops),
    ));
    f.push(fact(
        "最終の求心ズレ",
        format!(
            "{}（許容{} mm以内）",
            r.centering_text, CENTERING_TOLERANCE_MM
        ),
    ));
    f.push(fact(
        "最終の電子気泡",
        format!(
            "X {}・Y {}（許容±{:.0}″以内）",
            r.tilt_x_text, r.tilt_y_text, TILT_TOLERANCE_SEC
        ),
    ));
    f.push(fact(
        "定心桿",
        if r.clamp_tight {
            "締めている"
        } else {
            "緩んだまま"
        },
    ));
    if !r.passed && !r.reasons.is_empty() {
        f.push(fact("不合格の理由", clip(r.reasons.join("／"), 200)));
    }
    for (i, t) in r.steps.iter().enumerate() {
        let ms = log.step_ms[i];
        if ms <= 0.0 {
            continue;
        }
        f.push(fact(
            format!("手順{}「{}」の時間", i + 1, t.short),
            format!(
                "{}（目安{}）",
                format_sec(ms),
                format_sec(step_ref_ms(r.textbook, (i + 1) as u8))
            ),
        ));
    }
    let flow: Vec<String> = log
        .visits
        .iter()
        .filter(|(_, ms)| *ms >= 500.0)
        .map(|(s, ms)| format!("{}({})", s, format_sec(*ms)))
        .collect();
    if !flow.is_empty() {
        f.push(fact("手順の流れ（手順番号と時間）", clip_front(&flow, "→", 200)));
    }
    f.push(fact(
        "整準ネジを回した時間",
        format!(
            "1本ずつ{}・2本同時{}",
            format_sec(log.screw_single_ms),
            format_sec(log.screw_dual_ms)
        ),
    ));
    if log.screw_single_ms + log.screw_dual_ms > 0.0 {
        f.push(fact(
            "気泡が悪くなる向きに回していた時間",
            format_sec(log.screw_worse_ms),
        ));
    }
    f.push(fact("脚の伸縮の時間", format_sec(log.leg_length_ms)));
    f.push(fact("器械をずらした時間", format_sec(log.slide_ms)));
    for (i, l) in log.loops.iter().take(5).enumerate() {
        let c = l
            .centering_mm
            .map(|c| format!("求心ズレ{:.1} mm", c))
            .unwrap_or_else(|| "求心ズレ---".into());
        let t = l
            .tilt_sec
            .map(|t| format!("電子気泡（X・Yの大きいほう）{:.0}″", t))
            .unwrap_or_else(|| "電子気泡---".into());
        f.push(fact(format!("ループ{}回目をずらす前", i + 1), format!("{}・{}", c, t)));
    }
    let ops: Vec<String> = log
        .segments
        .iter()
        .filter(|s| s.dur_ms >= 300.0 || matches!(s.kind, LogKind::PressLeg | LogKind::Clamp | LogKind::Mount))
        .map(|s| {
            if s.dur_ms >= 300.0 {
                format!("{}({})", s.kind.label(), format_sec(s.dur_ms))
            } else {
                s.kind.label().to_string()
            }
        })
        .collect();
    if !ops.is_empty() {
        f.push(fact("操作の流れ（最後のほう）", clip_front(&ops, "→", 200)));
    }
    let ds = detours(r);
    if ds.is_empty() {
        if r.passed {
            f.push(fact("遠回り", "特になし"));
        }
    } else {
        for d in ds {
            f.push(fact("遠回り", d));
        }
    }
    f
}

// ---------------------------------------------------------------------------
// 次の操作（どのネジをどちらへ・どちらへずらすか）。実際に少し動かしてみて、良くなるほうを選ぶ
// ---------------------------------------------------------------------------

/// 向きの言葉。操作する人は手前（南）に立って奥（北）を向く：右＝東、奥＝北。`v`＝[東, 北]
pub fn direction_words(lang: Lang, v: [f64; 2]) -> &'static str {
    let (e, n) = (v[0], v[1]);
    let (ae, an) = (e.abs(), n.abs());
    if ae < 0.4 * an {
        if n >= 0.0 {
            pick(lang, "奥", "the far side")
        } else {
            pick(lang, "手前", "the near side")
        }
    } else if an < 0.4 * ae {
        if e >= 0.0 {
            pick(lang, "右", "the right")
        } else {
            pick(lang, "左", "the left")
        }
    } else {
        match (e >= 0.0, n >= 0.0) {
            (true, true) => pick(lang, "右奥", "the far right"),
            (false, true) => pick(lang, "左奥", "the far left"),
            (true, false) => pick(lang, "右手前", "the near right"),
            (false, false) => pick(lang, "左手前", "the near left"),
        }
    }
}

/// 整準ネジの回し方の候補
#[derive(Clone, Copy)]
struct ScrewMove {
    turns: &'static [(usize, Thumb)],
    text: &'static str,
    en: &'static str,
}

/// 電子気泡の X（A→B の向き）を直す：A・B を同時に
const LEVEL_X: [ScrewMove; 2] = [
    ScrewMove {
        turns: &[(0, Thumb::Right), (1, Thumb::Left)],
        text: "整準ネジA・Bを同時に内側へ回す（親指どうしを近づける）",
        en: "Turn leveling screws A and B inward together (thumbs toward each other)",
    },
    ScrewMove {
        turns: &[(0, Thumb::Left), (1, Thumb::Right)],
        text: "整準ネジA・Bを同時に外側へ回す（親指どうしを離す）",
        en: "Turn leveling screws A and B outward together (thumbs apart)",
    },
];

/// 電子気泡の Y（C の向き）を直す：C だけ
const LEVEL_Y: [ScrewMove; 2] = [
    ScrewMove {
        turns: &[(2, Thumb::Left)],
        text: "整準ネジCを、親指を左へ動かす向きに回す",
        en: "Turn leveling screw C so that your thumb moves left",
    },
    ScrewMove {
        turns: &[(2, Thumb::Right)],
        text: "整準ネジCを、親指を右へ動かす向きに回す",
        en: "Turn leveling screw C so that your thumb moves right",
    },
];

/// レーザー点を動かす（教科書モード手順3）：1本ずつと、A・B同時
const ANY_SCREW: [ScrewMove; 8] = [
    ScrewMove {
        turns: &[(0, Thumb::Left)],
        text: "整準ネジAを、親指を左へ動かす向きに回す",
        en: "Turn leveling screw A so that your thumb moves left",
    },
    ScrewMove {
        turns: &[(0, Thumb::Right)],
        text: "整準ネジAを、親指を右へ動かす向きに回す",
        en: "Turn leveling screw A so that your thumb moves right",
    },
    ScrewMove {
        turns: &[(1, Thumb::Left)],
        text: "整準ネジBを、親指を左へ動かす向きに回す",
        en: "Turn leveling screw B so that your thumb moves left",
    },
    ScrewMove {
        turns: &[(1, Thumb::Right)],
        text: "整準ネジBを、親指を右へ動かす向きに回す",
        en: "Turn leveling screw B so that your thumb moves right",
    },
    ScrewMove {
        turns: &[(2, Thumb::Left)],
        text: "整準ネジCを、親指を左へ動かす向きに回す",
        en: "Turn leveling screw C so that your thumb moves left",
    },
    ScrewMove {
        turns: &[(2, Thumb::Right)],
        text: "整準ネジCを、親指を右へ動かす向きに回す",
        en: "Turn leveling screw C so that your thumb moves right",
    },
    LEVEL_X[0],
    LEVEL_X[1],
];

/// 試しに回す量 [回転]（行き過ぎないよう、向きだけがわかる小さな量）
const TRY_TURNS: f64 = 0.001;
/// 試しに伸縮する量 [mm]
const TRY_LEG_MM: f64 = 0.1;

/// 候補を試しに回して、目標の値が一番小さくなるものを選ぶ（良くならなければ None）
fn best_screw_move<'a>(
    inst: &Instrument,
    head: &HeadPose,
    moves: &'a [ScrewMove],
    objective: impl Fn(&InstrumentPose) -> Option<f64>,
) -> Option<&'a ScrewMove> {
    let now = objective(&inst.pose(head)?)?;
    let mut best: Option<(&ScrewMove, f64)> = None;
    for m in moves {
        let mut trial = inst.clone();
        if m
            .turns
            .iter()
            .any(|(k, t)| trial.turn_screw(*k, *t, TRY_TURNS).is_err())
        {
            continue;
        }
        let Some(v) = trial.pose(head).and_then(|p| objective(&p)) else {
            continue;
        };
        if v < now && best.map(|(_, b)| v < b).unwrap_or(true) {
            best = Some((m, v));
        }
    }
    best.map(|(m, _)| m)
}

/// 電子気泡を合わせる整準ネジの回し方（大きくずれているほうの軸から）
pub fn level_hint(lang: Lang, inst: &Instrument, head: &HeadPose) -> Option<String> {
    let (x, y) = inst.pose(head)?.tilt_xy();
    let (xs, ys) = (to_arcsec(x), to_arcsec(y));
    let (moves, axis, name): (&[ScrewMove], usize, &str) = if xs.abs() >= ys.abs() {
        (&LEVEL_X, 0, "X")
    } else {
        (&LEVEL_Y, 1, "Y")
    };
    let m = best_screw_move(inst, head, moves, |p| {
        let (a, b) = p.tilt_xy();
        Some(to_arcsec(if axis == 0 { a } else { b }).abs())
    })?;
    let now = format_dms_signed(if axis == 0 { x } else { y });
    Some(tr!(
        lang,
        "{}（電子気泡の{}がいま{}。0に近づける）",
        "{} (electronic bubble {} is now {}; bring it to 0)",
        pick(lang, m.text, m.en),
        name,
        now
    ))
}

/// レーザー点を測点に近づける整準ネジの回し方（教科書モード手順3）
pub fn screw_center_hint(lang: Lang, inst: &Instrument, head: &HeadPose) -> Option<String> {
    let m = best_screw_move(inst, head, &ANY_SCREW, |p| {
        p.laser_point().map(|l| l.x.hypot(l.y))
    })?;
    Some(tr!(
        lang,
        "{}（レーザー点を測点に近づける）",
        "{}, to bring the laser dot toward the point",
        pick(lang, m.text, m.en)
    ))
}

pub const LEG_LABELS: [&str; 3] = ["脚1（奥）", "脚2（左手前）", "脚3（右手前）"];
pub const LEG_LABELS_EN: [&str; 3] = ["Leg 1 (far)", "Leg 2 (near left)", "Leg 3 (near right)"];

/// 円形気泡（器械が無ければ脚頭の傾き）を合わせる脚の伸縮。試しに伸縮して、良くなるものを選ぶ
pub fn leg_hint(lang: Lang, tripod: &Tripod, inst: &Instrument) -> Option<String> {
    let measure = |t: &Tripod| -> Option<f64> {
        match inst.pose(&t.head) {
            Some(p) => Some(CircularBubble::from_pose(&p).raw_mm),
            None => Some(t.head.tilt()),
        }
    };
    let now = measure(tripod)?;
    let mut best: Option<(usize, bool, f64)> = None;
    for leg in 0..3 {
        for extend in [true, false] {
            let mut t = tripod.clone();
            let mut lens = t.leg_len;
            let d = if extend { TRY_LEG_MM } else { -TRY_LEG_MM };
            lens[leg] = (lens[leg] + d).clamp(LEG_LENGTH_MIN_MM, LEG_LENGTH_MAX_MM);
            if t.try_set(t.feet, lens).is_err() {
                continue;
            }
            let Some(v) = measure(&t) else { continue };
            if v < now && best.map(|(_, _, b)| v < b).unwrap_or(true) {
                best = Some((leg, extend, v));
            }
        }
    }
    let (leg, extend, _) = best?;
    let what = if inst.mounted {
        pick(lang, "円形気泡を中心の円に近づける", "to bring the circular bubble toward the center circle")
    } else {
        pick(lang, "脚頭を水平に近づける", "to bring the tripod head closer to level")
    };
    Some(match lang {
        Lang::Ja => format!(
            "{}を{}（{}）",
            LEG_LABELS[leg],
            if extend { "伸ばす" } else { "縮める" },
            what
        ),
        Lang::En => format!(
            "{} {} {}",
            if extend { "Extend" } else { "Shorten" },
            LEG_LABELS_EN[leg],
            what
        ),
    })
}

/// 現場の指示の言い回し（仮。オーナーに現場の言葉づかいを確認してもらう）
pub const INSTRUCTION_STYLES: [&str; 4] = [
    "短く、ぶっきらぼうに",
    "ていねいに、順を追って",
    "現場の略語を多めに（BS、器械高、据えた点など）",
    "理由をひとこと添えて",
];

pub fn instruction_style(style: u8) -> &'static str {
    INSTRUCTION_STYLES
        .get((style as usize).saturating_sub(1))
        .copied()
        .unwrap_or(INSTRUCTION_STYLES[0])
}

/// AIが使えないときの、決まった言い方の指示
pub fn instruction_fallback(lang: Lang, station: &str, backsight: &str) -> String {
    tr!(
        lang,
        "機械点は{}、BSは{}な。器械高は巻尺で測って入れといて。",
        "Station is {}, backsight is {}. Measure the instrument height with the tape and enter it.",
        station,
        backsight
    )
}

/// 器械ずらしで求心を合わせる向き（`centering`＝レーザー点 − 測点 [東, 北] mm）
pub fn slide_hint(lang: Lang, centering: [f64; 2], clamp_tight: bool) -> String {
    let d = centering[0].hypot(centering[1]);
    let dir = direction_words(lang, [-centering[0], -centering[1]]);
    if clamp_tight {
        tr!(
            lang,
            "定心桿を緩めて、器械を{}へ約{:.1} mmずらし、終わったら締める",
            "Loosen the mounting screw, slide the instrument about {1:.1} mm toward {0}, then tighten it",
            dir,
            d
        )
    } else {
        tr!(
            lang,
            "器械を{}へ約{:.1} mmずらし、終わったら定心桿を締める",
            "Slide the instrument about {1:.1} mm toward {0}, then tighten the mounting screw",
            dir,
            d
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steps::field_steps;

    fn frame(step: u8, t_ms: f64, screws: usize, worse: bool) -> FrameSummary {
        FrameSummary {
            step,
            dt_ms: 100.0,
            t_ms,
            kind: None,
            screws,
            worse,
            better: !worse && screws > 0,
            leg_length: false,
            slide: false,
            move_legs: false,
            screw_limit: false,
            clamp_refused: false,
        }
    }

    #[test]
    fn segments_merge_while_the_same_operation_continues() {
        let mut log = CoachLog::default();
        for i in 1..=10 {
            log.account(&frame(5, i as f64 * 100.0, 1, false));
        }
        for i in 11..=15 {
            log.account(&frame(5, i as f64 * 100.0, 2, false));
        }
        assert_eq!(log.segments.len(), 2);
        assert_eq!(log.segments[0].kind, LogKind::Screw1);
        assert!((log.segments[0].dur_ms - 1000.0).abs() < 1e-9);
        assert!((log.screw_single_ms - 1000.0).abs() < 1e-9);
        assert!((log.screw_dual_ms - 500.0).abs() < 1e-9);
        assert!((log.step_ms[4] - 1500.0).abs() < 1e-9);
    }

    #[test]
    fn wrong_way_time_resets_when_it_gets_better() {
        let mut log = CoachLog::default();
        for i in 1..=5 {
            log.account(&frame(5, i as f64 * 100.0, 1, true));
        }
        assert!((log.wrong_run_ms - 500.0).abs() < 1e-9);
        log.account(&frame(5, 600.0, 1, false));
        assert_eq!(log.wrong_run_ms, 0.0);
        assert!((log.screw_worse_ms - 500.0).abs() < 1e-9);
    }

    #[test]
    fn held_refusal_counts_once() {
        let mut log = CoachLog::default();
        for i in 1..=5 {
            let mut f = frame(6, i as f64 * 100.0, 0, false);
            f.clamp_refused = true;
            log.account(&f);
        }
        assert_eq!(log.clamp_refusals, 1);
    }

    #[test]
    fn detours_list_what_went_long() {
        let steps = field_steps();
        let mut log = CoachLog::default();
        log.step_ms[4] = 80_000.0;
        log.screw_worse_ms = 5_000.0;
        log.start_loop(LoopRecord {
            centering_mm: Some(2.0),
            tilt_sec: Some(90.0),
        });
        let r = ReviewInput {
            textbook: false,
            passed: true,
            rating: None,
            time_text: "3:10.0",
            loops: 5,
            centering_text: "0.3 mm",
            tilt_x_text: "+0°00′05″",
            tilt_y_text: "−0°00′03″",
            clamp_tight: true,
            reasons: &[],
            steps: &steps,
            log: &log,
        };
        let d = detours(&r);
        assert!(d.iter().any(|s| s.contains("悪くなる向き")));
        assert!(d.iter().any(|s| s.contains("ずれたまま")));
        assert!(d.iter().any(|s| s.contains("ループ（6⇄7）が5回")));
        assert!(d.iter().any(|s| s.contains("手順5")));
        let f = review_facts(&r);
        assert!(f.iter().all(|x| x.value.chars().count() <= 240 && x.key.chars().count() <= 40));
        assert!(f.len() <= 60);
    }

    #[test]
    fn direction_words_follow_the_operator_view() {
        assert_eq!(direction_words(Lang::Ja, [1.0, 0.0]), "右");
        assert_eq!(direction_words(Lang::Ja, [0.0, -1.0]), "手前");
        assert_eq!(direction_words(Lang::Ja, [-1.0, 1.0]), "左奥");
        assert_eq!(direction_words(Lang::Ja, [0.1, 1.0]), "奥");
    }

    #[test]
    fn signed_m_has_sign_and_three_decimals() {
        assert_eq!(format_signed_m(12_345.4), "+12.345");
        assert_eq!(format_signed_m(-3_210.0), "\u{2212}3.210");
        assert_eq!(format_signed_m(0.0004), "±0.000");
    }

    #[test]
    fn speech_text_drops_parentheses_and_action_key_drops_numbers() {
        let a = "整準ネジCを、親指を右へ動かす向きに回す（電子気泡のYがいま+0°01′49″。0に近づける）";
        assert_eq!(speech_text(a), "整準ネジCを、親指を右へ動かす向きに回す");
        assert_eq!(speech_text("脚3（右手前）を縮める（円形気泡を中心の円に近づける）"), "脚3（右手前）を縮める（円形気泡を中心の円に近づける）");
        let b = "器械を左へ約2.0 mmずらし、終わったら定心桿を締める";
        let c = "器械を左へ約0.7 mmずらし、終わったら定心桿を締める";
        assert_eq!(action_key(b), action_key(c));
        assert_ne!(action_key(b), action_key("器械を右へ約0.7 mmずらし、終わったら定心桿を締める"));
    }

    #[test]
    fn clip_front_keeps_the_end() {
        let items: Vec<String> = (0..100).map(|i| format!("手順{}", i)).collect();
        let s = clip_front(&items, "→", 50);
        assert!(s.starts_with('…'));
        assert!(s.ends_with("手順99"));
        assert!(s.chars().count() <= 50);
    }
}
