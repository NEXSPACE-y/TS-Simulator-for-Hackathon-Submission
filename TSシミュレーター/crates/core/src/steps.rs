//! 手順の文言（SPEC 5章）

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepText {
    pub label: &'static str,
    pub short: &'static str,
}

/// 現場モード：第1部の手順（SPEC 5.1）
pub fn field_steps() -> [StepText; 7] {
    [
        StepText {
            label: "三脚を測点の上に置く",
            short: "三脚を置く",
        },
        StepText {
            label: "一点を踏み込み、残り二点で鉛直を仮合わせ",
            short: "一点踏み込み・仮合わせ",
        },
        StepText {
            label: "残り二点も踏み込む",
            short: "残り二点を踏み込む",
        },
        StepText {
            label: "三脚の足の伸縮で円形気泡を合わせる",
            short: "脚の伸縮で円形気泡",
        },
        StepText {
            label: "整準ネジ3本で電子気泡を合わせる",
            short: "整準ネジで電子気泡",
        },
        StepText {
            label: "求心がズレていれば、定心桿を緩めて三脚頭の上で器械をずらして合わせる",
            short: "器械をずらして求心",
        },
        StepText {
            label: "再度、整準ネジで電子気泡を合わせる",
            short: "再度、整準",
        },
    ]
}

/// 教科書モード：第1部の手順（SPEC 5.3。本番前に教本で要確認）
pub fn textbook_steps() -> [StepText; 7] {
    [
        StepText {
            label: "三脚を測点の上におおよそ置き、脚頭をだいたい水平に",
            short: "三脚をおおよそ置く",
        },
        StepText {
            label: "器械を載せて定心桿で固定",
            short: "器械を載せて固定",
        },
        StepText {
            label: "整準ネジで求心点を測点に合わせる",
            short: "整準ネジで求心",
        },
        StepText {
            label: "三脚の足の伸縮で円形気泡を中央へ",
            short: "脚の伸縮で円形気泡",
        },
        StepText {
            label: "整準ネジで精密に整準",
            short: "精密に整準",
        },
        StepText {
            label: "求心がズレていれば定心桿を緩めて器械をずらす",
            short: "器械をずらして求心",
        },
        StepText {
            label: "整準を確認。両方合うまで5〜6を繰り返す",
            short: "整準を確認",
        },
    ]
}

/// 第2部：観測の手順（SPEC 5.2）
pub fn observation_steps() -> [StepText; 3] {
    [
        StepText {
            label: "電子平板で機械点とBSをセット",
            short: "機械点とBSをセット",
        },
        StepText {
            label: "BSを視準して測量",
            short: "BSを測量",
        },
        StepText {
            label: "FSのプリズムを視準して測量",
            short: "FSを測量",
        },
    ]
}
