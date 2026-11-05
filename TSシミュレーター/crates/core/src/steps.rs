//! 手順の文言（SPEC 5章）。英語の文も持つ

use crate::i18n::{pick, Lang};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepText {
    pub label: &'static str,
    pub short: &'static str,
    pub label_en: &'static str,
    pub short_en: &'static str,
}

impl StepText {
    pub fn label_in(&self, lang: Lang) -> &'static str {
        pick(lang, self.label, self.label_en)
    }

    pub fn short_in(&self, lang: Lang) -> &'static str {
        pick(lang, self.short, self.short_en)
    }
}

/// 現場モード：第1部の手順（SPEC 5.1）
pub fn field_steps() -> [StepText; 7] {
    [
        StepText {
            label: "三脚を測点の上に置く",
            short: "三脚を置く",
            label_en: "Set the tripod over the point",
            short_en: "Set tripod",
        },
        StepText {
            label: "一点を踏み込み、残り二点で鉛直を仮合わせ",
            short: "一点踏み込み・仮合わせ",
            label_en: "Press in one leg and roughly plumb with the other two",
            short_en: "Press one leg, rough plumb",
        },
        StepText {
            label: "残り二点も踏み込む",
            short: "残り二点を踏み込む",
            label_en: "Press in the other two legs",
            short_en: "Press other two legs",
        },
        StepText {
            label: "三脚の足の伸縮で円形気泡を合わせる",
            short: "脚の伸縮で円形気泡",
            label_en: "Center the circular bubble by adjusting leg lengths",
            short_en: "Legs: circular bubble",
        },
        StepText {
            label: "整準ネジ3本で電子気泡を合わせる",
            short: "整準ネジで電子気泡",
            label_en: "Level the electronic bubble with the three leveling screws",
            short_en: "Screws: electronic bubble",
        },
        StepText {
            label: "求心がズレていれば、定心桿を緩めて三脚頭の上で器械をずらして合わせる",
            short: "器械をずらして求心",
            label_en: "If off center, loosen the mounting screw and slide the instrument on the tripod head",
            short_en: "Slide to center",
        },
        StepText {
            label: "再度、整準ネジで電子気泡を合わせる",
            short: "再度、整準",
            label_en: "Level the electronic bubble again with the leveling screws",
            short_en: "Level again",
        },
    ]
}

/// 教科書モード：第1部の手順（SPEC 5.3）
pub fn textbook_steps() -> [StepText; 7] {
    [
        StepText {
            label: "三脚を測点の上におおよそ置き、脚頭をだいたい水平に",
            short: "三脚をおおよそ置く",
            label_en: "Set the tripod roughly over the point with the head about level",
            short_en: "Rough tripod setup",
        },
        StepText {
            label: "器械を載せて定心桿で固定",
            short: "器械を載せて固定",
            label_en: "Mount the instrument and fix it with the mounting screw",
            short_en: "Mount instrument",
        },
        StepText {
            label: "整準ネジで求心点を測点に合わせる",
            short: "整準ネジで求心",
            label_en: "Bring the plummet point onto the point with the leveling screws",
            short_en: "Screws: centering",
        },
        StepText {
            label: "三脚の足の伸縮で円形気泡を中央へ",
            short: "脚の伸縮で円形気泡",
            label_en: "Center the circular bubble by adjusting leg lengths",
            short_en: "Legs: circular bubble",
        },
        StepText {
            label: "整準ネジで精密に整準",
            short: "精密に整準",
            label_en: "Level precisely with the leveling screws",
            short_en: "Precise leveling",
        },
        StepText {
            label: "求心がズレていれば定心桿を緩めて器械をずらす",
            short: "器械をずらして求心",
            label_en: "If off center, loosen the mounting screw and slide the instrument",
            short_en: "Slide to center",
        },
        StepText {
            label: "整準を確認。両方合うまで5〜6を繰り返す",
            short: "整準を確認",
            label_en: "Check the level. Repeat 5–6 until both are within tolerance",
            short_en: "Check level",
        },
    ]
}

/// 第2部：観測の手順（SPEC 5.2）
pub fn observation_steps() -> [StepText; 3] {
    [
        StepText {
            label: "電子平板で機械点とBSをセット",
            short: "機械点とBSをセット",
            label_en: "Set the station and backsight on the tablet",
            short_en: "Set station & backsight",
        },
        StepText {
            label: "BSを視準して測量",
            short: "BSを測量",
            label_en: "Sight the backsight and measure",
            short_en: "Measure backsight",
        },
        StepText {
            label: "FSのプリズムを視準して測量",
            short: "FSを測量",
            label_en: "Sight the foresight prism and measure",
            short_en: "Measure foresight",
        },
    ]
}
