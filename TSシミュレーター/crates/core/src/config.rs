//! 許容値・寸法・操作の速さなどの数値を1か所にまとめた設定（SPEC 6.7）。
//!
//! - 「確定」の値は変えない。
//! - 「仮」の値は動かしてみて調整してよい（変えたら報告する）。
//!
//! 単位：長さは mm、角度は rad（秒・分で書いたものは変換して使う）。

use crate::angle::{arcmin, arcsec, deg};

// ---------------------------------------------------------------------------
// 許容値（SPEC 6.7）
// ---------------------------------------------------------------------------

/// 求心ズレの許容 [mm]（確定）
pub const CENTERING_TOLERANCE_MM: f64 = 0.5;
/// 電子気泡の許容 [秒]（X・Yそれぞれ。確定）
pub const TILT_TOLERANCE_SEC: f64 = 15.0;
/// 液晶◎の外側の円の範囲 [秒]（オーナー確認：±1′。±15″の中心の円が見える大きさにするため）
pub const LCD_OUTER_RANGE_SEC: f64 = 60.0;
/// 円形気泡の感度：この角度だけ傾くと…（仮：8′）
pub const CIRCULAR_SENS_ANGLE: f64 = arcmin(8.0);
/// …気泡がこの距離だけ動く [mm]（仮：2 mm）
pub const CIRCULAR_SENS_MM: f64 = 2.0;
/// 円形気泡が「中心の円に入った」とみなす気泡の移動量 [mm]（仮）
pub const CIRCULAR_IN_CIRCLE_MM: f64 = 1.0;
/// 円形気泡が気泡管の中で動ける最大の距離 [mm]（表示用の上限。仮）
pub const CIRCULAR_VIAL_MAX_MM: f64 = 6.0;
/// 液晶◎の中心の円の半径（外側の円を1としたとき。表示用）
pub const LCD_INNER_RATIO: f64 = TILT_TOLERANCE_SEC / LCD_OUTER_RANGE_SEC;
/// 円形気泡の中心の円の半径（気泡管の範囲を1としたとき。表示用）
pub const CIRCULAR_IN_RATIO: f64 = CIRCULAR_IN_CIRCLE_MM / CIRCULAR_VIAL_MAX_MM;
/// 器械を三脚頭上でずらせる範囲 [mm]（X・Yそれぞれ。仮：±20 mm）
pub const SLIDE_LIMIT_MM: f64 = 20.0;
/// 視準OKの範囲 [秒]（十字線とターゲット中心の差。水平・鉛直それぞれ。仮）
pub const SIGHT_OK_SEC: f64 = 20.0;

// ---------------------------------------------------------------------------
// 判定の表示精度（判定は画面に出る値で行う）
// ---------------------------------------------------------------------------

/// 求心ズレの表示の刻み [mm]
pub const CENTERING_DISPLAY_STEP_MM: f64 = 0.1;

// ---------------------------------------------------------------------------
// 合格タイムの目安（SPEC 5.4）
// ---------------------------------------------------------------------------

/// 熟練の目安 [ms]（2分）
pub const TIME_EXPERT_MS: f64 = 120_000.0;
/// 合格の目安 [ms]（3分）
pub const TIME_PASS_MS: f64 = 180_000.0;

// ---------------------------------------------------------------------------
// 三脚の寸法（仮）
// ---------------------------------------------------------------------------

/// 脚の付け根（ヒンジ）の、三脚頭中心からの距離 [mm]
pub const LEG_HINGE_RADIUS_MM: f64 = 70.0;
/// 脚の付け根の向き（東から反時計回りの角度）。脚1＝奥、脚2＝左手前、脚3＝右手前
pub const LEG_AZIMUTHS: [f64; 3] = [deg(90.0), deg(210.0), deg(330.0)];
/// 脚の長さ（足先〜付け根）の範囲 [mm]
pub const LEG_LENGTH_MIN_MM: f64 = 900.0;
pub const LEG_LENGTH_MAX_MM: f64 = 1700.0;
/// 脚の長さの初期値 [mm] と、そのばらつき [mm]
pub const LEG_LENGTH_START_MM: f64 = 1380.0;
pub const LEG_LENGTH_START_JITTER_MM: f64 = 8.0;
/// 足先の開き（三脚の中心からの距離）の初期値 [mm] と、そのばらつき [mm]
pub const FOOT_RADIUS_START_MM: f64 = 660.0;
pub const FOOT_RADIUS_START_JITTER_MM: f64 = 15.0;
/// 足先の向きのばらつき [rad]
pub const FOOT_AZIMUTH_JITTER: f64 = deg(4.0);
/// 開始時に三脚が測点から離れている距離の範囲 [mm]
pub const START_OFFSET_MIN_MM: f64 = 250.0;
pub const START_OFFSET_MAX_MM: f64 = 450.0;
/// 脚の傾き（鉛直からの角度）として許す範囲。外れる操作は受け付けない
pub const LEG_INCLINATION_MIN: f64 = deg(5.0);
pub const LEG_INCLINATION_MAX: f64 = deg(45.0);
/// 三脚頭の傾きとして許す上限。超える操作は受け付けない
pub const HEAD_TILT_MAX: f64 = deg(15.0);
/// 脚の付け根は足先より最低これだけ高いこと [mm]。下回る操作は受け付けない
pub const LEG_MIN_RISE_MM: f64 = 100.0;
/// 踏み込んだときに足先が沈む量の範囲 [mm]（毎回ランダム。仮：数mm）
pub const SINK_MIN_MM: f64 = 2.0;
pub const SINK_MAX_MM: f64 = 6.0;

// ---------------------------------------------------------------------------
// 器械（整準台・本体）の寸法（仮）
// ---------------------------------------------------------------------------

/// 三脚頭の脚の付け根の面から、三脚頭の上面までの高さ [mm]
pub const HEAD_TOP_OFFSET_MM: f64 = 20.0;
/// 三脚頭の上面から、整準ネジの先端が作る面（中立時）までの高さ [mm]
pub const TRIBRACH_PIVOT_HEIGHT_MM: f64 = 30.0;
/// 整準ネジの、器械中心からの距離 [mm]
pub const SCREW_RADIUS_MM: f64 = 55.0;
/// 整準ネジの向き（東から反時計回り）。A＝左手前、B＝右手前、C＝奥
pub const SCREW_AZIMUTHS: [f64; 3] = [deg(210.0), deg(330.0), deg(90.0)];
/// 整準ネジ1回転で上下する量 [mm]
pub const SCREW_PITCH_MM: f64 = 0.5;
/// 整準ネジを回せる範囲 [回転]（中立から±）
pub const SCREW_TURN_LIMIT: f64 = 8.0;
/// 整準ネジの初期値のばらつき [回転]
pub const SCREW_START_JITTER: f64 = 0.6;
/// 整準ネジの先端の面から望遠鏡の回転軸（器械高の基準）までの高さ [mm]
pub const TRUNNION_HEIGHT_MM: f64 = 200.0;
/// 器械を三脚頭上でずらしたときの気泡の崩れ（三脚頭の上面のわずかな丸みによる分）
/// ずらした量 1 mm あたりの器械の傾きの変化 [rad]（オーナー確認：5 mm ずらすと 10″前後）
pub const SLIDE_TILT_PER_MM: f64 = arcsec(2.0);
/// 器械を三脚頭上でずらしたときの気泡の崩れ（三脚頭が傾いている分）
/// （三脚頭の傾き[rad] × ずらした量[mm] × この係数 が、器械の傾きの変化[rad]になる）
pub const SLIDE_TILT_COUPLING_PER_MM: f64 = 1.1e-3;
/// 開始時の器械のずれ（三脚頭上）のばらつき [mm]
pub const SLIDE_START_JITTER_MM: f64 = 5.0;

// ---------------------------------------------------------------------------
// 操作の速さ（ボタン・キーを押している間）。[粗, 中, 微]
// ---------------------------------------------------------------------------

/// 三脚・脚を動かす速さ [mm/s]
pub const SPEED_MOVE_LEGS: [f64; 3] = [300.0, 60.0, 10.0];
/// 脚の伸縮の速さ [mm/s]
pub const SPEED_LEG_LENGTH: [f64; 3] = [40.0, 8.0, 1.5];
/// 整準ネジを回す速さ [回転/s]
pub const SPEED_SCREW: [f64; 3] = [0.5, 0.1, 0.02];
/// 器械をずらす速さ [mm/s]
pub const SPEED_SLIDE: [f64; 3] = [10.0, 2.0, 0.3];
/// 望遠鏡を回す速さ [rad/s]（粗動・中・微動）
pub const SPEED_TELESCOPE: [f64; 3] = [deg(20.0), arcmin(5.0), arcsec(20.0)];
/// 合焦つまみを回す速さ [1/m/s]（ピントの合う距離の逆数が変わる速さ。仮）
pub const SPEED_FOCUS: [f64; 3] = [0.15, 0.02, 0.003];
/// 接眼レンズを回す速さ [ディオプター/s]（仮）
pub const SPEED_EYEPIECE: [f64; 3] = [2.0, 0.5, 0.1];
/// 1フレームとして扱う時間の上限 [ms]（タブ切替などで時間が飛んだとき用）
pub const MAX_FRAME_DT_MS: f64 = 100.0;

// ---------------------------------------------------------------------------
// 教科書モードの各手順の目標（仮）
// ---------------------------------------------------------------------------

/// 手順1：三脚頭の中心と測点の水平距離の上限 [mm]
pub const TEXTBOOK_ROUGH_CENTER_MM: f64 = 30.0;
/// 手順1：三脚頭の傾きの上限 [rad]（だいたい水平）
pub const TEXTBOOK_ROUGH_HEAD_TILT: f64 = deg(2.0);
/// 手順3：整準ネジで合わせる求心ズレの上限 [mm]
pub const TEXTBOOK_SCREW_CENTER_MM: f64 = 1.0;
/// 手順外の操作をしたときの注意を、同じ操作で繰り返し出す間隔 [ms]
pub const WARNING_REPEAT_MS: f64 = 2500.0;

// ---------------------------------------------------------------------------
// 第2部：観測
// ---------------------------------------------------------------------------

/// 既知点・設計点（架空の座標。X＝北、Y＝東、H＝標高。単位 mm）
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnownPoint {
    pub name: &'static str,
    pub x_mm: f64,
    pub y_mm: f64,
    pub h_mm: f64,
}

/// 既知点の一覧（電子平板で選ぶもの）。架空の座標で、実在の現場とは関係ない。
pub const KNOWN_POINTS: [KnownPoint; 4] = [
    KnownPoint {
        name: "T-1",
        x_mm: 1_250_000.0,
        y_mm: 2_380_000.0,
        h_mm: 12_350.0,
    },
    KnownPoint {
        name: "T-2",
        x_mm: 1_285_214.0,
        y_mm: 2_401_537.0,
        h_mm: 12_350.0,
    },
    KnownPoint {
        name: "T-3",
        x_mm: 1_219_630.0,
        y_mm: 2_412_905.0,
        h_mm: 12_350.0,
    },
    KnownPoint {
        name: "T-4",
        x_mm: 1_226_118.0,
        y_mm: 2_344_772.0,
        h_mm: 12_350.0,
    },
];
/// 三脚を据える測点（KNOWN_POINTS の番号）。3Dの原点になる
pub const STATION_INDEX: usize = 0;
/// BSのターゲットが実際に置いてある点（KNOWN_POINTS の番号）
pub const BACKSIGHT_INDEX: usize = 1;
/// FSの設計点
pub const FS_DESIGN: KnownPoint = KnownPoint {
    name: "P-1",
    x_mm: 1_238_400.0,
    y_mm: 2_361_250.0,
    h_mm: 12_350.0,
};
/// FSのプリズムが実際に立っている位置の、設計点からのずれ [mm]（X, Y）
pub const FS_ACTUAL_OFFSET_MM: [f64; 2] = [6.0, -4.0];
/// プリズム高・ターゲット高 [mm]
pub const TARGET_HEIGHT_MM: f64 = 1500.0;
/// 器械高として入力できる範囲 [mm]
pub const INSTRUMENT_HEIGHT_INPUT_MIN_MM: f64 = 500.0;
pub const INSTRUMENT_HEIGHT_INPUT_MAX_MM: f64 = 2500.0;
/// 開始時の望遠鏡の鉛直角（天頂角）
pub const TELESCOPE_START_ZENITH: f64 = deg(90.0);
/// 望遠鏡の天頂角として動かせる範囲
pub const TELESCOPE_ZENITH_MIN: f64 = deg(30.0);
pub const TELESCOPE_ZENITH_MAX: f64 = deg(150.0);

// ---------------------------------------------------------------------------
// 望遠鏡の調整：合焦（ピント）・視度（十字線）（オーナー確認：2026-11-03 に追加。値はすべて仮）
// ---------------------------------------------------------------------------

/// ピントを合わせられる一番近い距離 [m]（遠い側は無限遠）
pub const FOCUS_MIN_DISTANCE_M: f64 = 1.3;
/// ピントが合ったとみなす範囲：「ピントの合う距離の逆数」と「ターゲットまでの距離の逆数」の差 [1/m]
/// （22 m なら約 21〜24 m、41 m なら約 37〜47 m）
pub const FOCUS_TOLERANCE_PER_M: f64 = 0.003;
/// ターゲットのぼけが最大に見える差 [1/m]（表示用。ぼけの見え方は差の平方根に比例させ、
/// 少しのずれでもわずかにぼけて見えるようにする）
pub const FOCUS_BLUR_FULL_PER_M: f64 = 0.3;
/// 観測を始めたときにピントの合っている距離の範囲 [m]（前に使った人の設定のまま、という想定）
pub const FOCUS_START_MIN_M: f64 = 4.0;
pub const FOCUS_START_MAX_M: f64 = 16.0;
/// 接眼レンズを回せる範囲 [ディオプター]（±）
pub const EYEPIECE_RANGE_DPT: f64 = 5.0;
/// のぞく人の目に合う接眼レンズの位置の範囲 [ディオプター]（±。毎回ランダム）
pub const EYE_RANGE_DPT: f64 = 2.0;
/// 十字線がくっきり見えるとみなす範囲 [ディオプター]
pub const EYEPIECE_TOLERANCE_DPT: f64 = 0.25;
/// 十字線のぼけが最大に見えるずれ [ディオプター]（表示用。ぼけの見え方はずれの平方根に比例させる）
pub const EYEPIECE_BLUR_FULL_DPT: f64 = 4.0;
/// 観測を始めたときに、接眼レンズが目に合う位置からずれている量の範囲 [ディオプター]
pub const EYEPIECE_START_OFF_MIN_DPT: f64 = 1.5;
pub const EYEPIECE_START_OFF_MAX_DPT: f64 = 3.0;

// ---------------------------------------------------------------------------
// AI先輩（操作の記録・講評）。値はすべて仮（オーナーの確認待ち）
// ---------------------------------------------------------------------------

/// 手順ごとの基準タイム [秒]（現場モード。合計で合格の目安の3分）
pub const FIELD_STEP_REF_SEC: [f64; 7] = [20.0, 20.0, 15.0, 35.0, 40.0, 25.0, 25.0];
/// 手順ごとの基準タイム [秒]（教科書モード。合計で3分）
pub const TEXTBOOK_STEP_REF_SEC: [f64; 7] = [30.0, 5.0, 25.0, 35.0, 40.0, 25.0, 20.0];
/// 講評で「ループが多い」とする回数（これを超えたら遠回り）
pub const REVIEW_LOOPS_REF: u32 = 3;
/// 講評で「悪くなる向きに回していた」とする合計時間 [秒]
pub const REVIEW_WRONG_WAY_SEC: f64 = 3.0;
/// 「電子気泡が大きくずれたまま器械をずらした」とする電子気泡（X・Yの大きいほう）[秒]
pub const SLIDE_BEFORE_LEVEL_SEC: f64 = 60.0;
/// 操作の記録の上限（件）
pub const OP_LOG_MAX: usize = 400;
/// 手順の流れの記録の上限（件）
pub const STEP_VISITS_MAX: usize = 60;
/// 次の操作の目安：三脚ごと運ぶか、もう踏み込むか（レーザー点と測点の距離 mm。現場モード）
pub const HINT_PLACE_MM: f64 = 30.0;
/// 次の操作の目安：残りの脚を踏み込んでよいレーザー点のずれ [mm]（現場モード）
pub const HINT_ROUGH_CENTER_MM: f64 = 3.0;
/// 次の操作の目安：望遠鏡の視野から遠いとみなす視準のずれ [秒]（照準器を使う目安）
pub const HINT_ROUGH_SIGHT_SEC: f64 = 1800.0;
/// 現場の指示の聞き取り：巻尺で測った値と入力した器械高の差がこれ以内なら「指示どおり」[mm]（仮）
pub const INSTRUCTION_HEIGHT_TOL_MM: f64 = 2.0;
/// 次の操作の目安：踏み込む前は、目標（脚頭の位置・傾き）のこの割合の内側を狙う（教科書モード手順1）
pub const HINT_MARGIN: f64 = 0.5;
