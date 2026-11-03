//! SPEC 6.4「必ず守る挙動」と SPEC 11章のテスト。

use ts_core::angle::{arcsec, deg, to_arcsec};
use ts_core::config::{
    CENTERING_TOLERANCE_MM, FS_ACTUAL_OFFSET_MM, FS_DESIGN, HEAD_HOLE_VIEW_MM, LEG_AZIMUTHS,
    SCREW_PITCH_MM, SCREW_RADIUS_MM, SINK_MAX_MM, SINK_MIN_MM,
};
use ts_core::instrument::{Instrument, InstrumentPose};
use ts_core::math::{Mat3, Vec3};
use ts_core::sim::{judge_setup, Command, NoticeKind, OpticsView, Refusal, Snapshot};
use ts_core::survey::{direction_to, foresight_ground_world};
use ts_core::tripod::Tripod;
use ts_core::{Lang, Mode, Phase, Simulator, Speed, Thumb};

const A: usize = 0;
const B: usize = 1;
const C: usize = 2;

fn symmetric_feet(cx: f64, cy: f64, r: f64) -> [Vec3; 3] {
    LEG_AZIMUTHS.map(|a| Vec3::new(cx + r * a.cos(), cy + r * a.sin(), 0.0))
}

/// 三脚頭が水平で測点の真上、器械も水平・中央の状態
fn level_sim() -> Simulator {
    let mut sim = Simulator::new(Mode::Field, 1);
    let Ok(mut t) = Tripod::new(symmetric_feet(0.0, 0.0, 650.0), [1380.0; 3]) else {
        unreachable!("対称な三脚は必ず作れる");
    };
    t.planted = [true; 3];
    sim.tripod = t;
    sim.inst = Instrument::default();
    sim
}

fn snap(sim: &Simulator) -> Snapshot {
    sim.snapshot()
}

fn tilt_xy(sim: &Simulator) -> (f64, f64) {
    match sim.snapshot().electronic {
        Some(e) => (e.x, e.y),
        None => (f64::NAN, f64::NAN),
    }
}

fn centering(sim: &Simulator) -> [f64; 2] {
    sim.snapshot().centering.unwrap_or([f64::NAN; 2])
}

fn approx(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

// ---------------------------------------------------------------------------
// 左手親指の法則（SPEC 6.4 / 11章）
// ---------------------------------------------------------------------------

#[test]
fn left_thumb_rule_inward_moves_bubble_right() {
    // A（左手）・B（右手）を同時に内側へ：左手の親指は右へ動く → 気泡も右（＋X）へ
    let mut sim = level_sim();
    let (x0, y0) = tilt_xy(&sim);
    assert!(sim.turn_screw(A, Thumb::Right, 0.05).is_ok());
    assert!(sim.turn_screw(B, Thumb::Left, 0.05).is_ok());
    let (x1, y1) = tilt_xy(&sim);
    assert!(
        x1 > x0 + arcsec(30.0),
        "X {} → {}",
        to_arcsec(x0),
        to_arcsec(x1)
    );
    assert!(approx(y1, y0, arcsec(0.5)), "Yは変わらない");
    // 液晶の点も右へ
    let dot = sim
        .snapshot()
        .electronic
        .map(|e| e.dot[0])
        .unwrap_or(f64::NAN);
    assert!(dot > 0.0);
    // 円形気泡も右へ
    let circ = sim
        .snapshot()
        .circular
        .map(|c| c.offset_mm[0])
        .unwrap_or(f64::NAN);
    assert!(circ > 0.0);
}

#[test]
fn left_thumb_rule_outward_moves_bubble_left() {
    // A・Bを同時に外側へ：左手の親指は左へ動く → 気泡も左（−X）へ
    let mut sim = level_sim();
    let (x0, _) = tilt_xy(&sim);
    assert!(sim.turn_screw(A, Thumb::Left, 0.05).is_ok());
    assert!(sim.turn_screw(B, Thumb::Right, 0.05).is_ok());
    let (x1, _) = tilt_xy(&sim);
    assert!(x1 < x0 - arcsec(30.0));
}

#[test]
fn left_thumb_rule_holds_on_tilted_head_too() {
    let mut sim = level_sim();
    let feet = sim.tripod.feet;
    assert!(sim.tripod.try_set(feet, [1395.0, 1372.0, 1380.0]).is_ok());
    let (x0, _) = tilt_xy(&sim);
    assert!(sim.turn_screw(A, Thumb::Right, 0.02).is_ok());
    assert!(sim.turn_screw(B, Thumb::Left, 0.02).is_ok());
    let (x1, _) = tilt_xy(&sim);
    assert!(x1 > x0);
}

#[test]
fn single_screw_changes_tilt_in_its_direction() {
    // 整準ネジ1本だけを回すと、そのネジの方向に傾きが変わる
    for (k, az) in [(A, deg(210.0)), (B, deg(330.0)), (C, deg(90.0))] {
        let mut sim = level_sim();
        // 親指を左へ＝そのネジが上がる → 気泡（高い側）はそのネジの方向へ
        assert!(sim.turn_screw(k, Thumb::Left, 0.1).is_ok());
        let (x, y) = tilt_xy(&sim);
        let dir = y.atan2(x);
        let diff = (dir - az).sin().abs();
        assert!(diff < 1e-6, "ネジ{}：気泡の向き {}°", k, dir.to_degrees());
        assert!(x * az.cos() + y * az.sin() > 0.0);
    }
}

// ---------------------------------------------------------------------------
// レーザー求心（SPEC 6.3 / 11章）
// ---------------------------------------------------------------------------

#[test]
fn laser_offset_is_h_tan_theta() {
    // 鉛直軸上の高さhの点が測点の真上にあり、軸が θ 傾いているとき、ズレは h×tanθ
    for (h, theta) in [
        (1500.0, arcsec(30.0)),
        (1450.0, deg(0.5)),
        (1300.0, deg(2.0)),
    ] {
        let axis = Vec3::new(theta.sin(), 0.0, theta.cos());
        let pose = InstrumentPose {
            pivot: Vec3::new(0.0, 0.0, h),
            rot: Mat3::rotate_z_to(axis),
        };
        let Some(l) = pose.laser_point() else {
            unreachable!("小さい傾きなら必ず地面と交わる");
        };
        let off = l.x.hypot(l.y);
        assert!(approx(off, h * theta.tan(), 1e-9), "h={h} θ={theta}");
    }
}

#[test]
fn laser_offset_from_screw_tilt_matches_h_tan_theta() {
    // 実際に整準ネジで傾けた場合も、回転の中心の高さ h と傾き θ から h×tanθ になる
    let mut sim = level_sim();
    assert!(sim.turn_screw(C, Thumb::Left, 1.0).is_ok());
    let s = snap(&sim);
    let Some(pose) = s.pose else {
        unreachable!();
    };
    let theta = pose.tilt();
    // ネジCを1回転上げると、Cと反対側の辺を軸に pitch / (1.5 r) だけ傾く
    let expect_theta = (SCREW_PITCH_MM / (1.5 * SCREW_RADIUS_MM)).atan();
    assert!(approx(theta, expect_theta, 1e-9));
    let h = pose.pivot.z;
    let c = s.centering_mm.unwrap_or(f64::NAN);
    assert!(
        approx(c, h * theta.tan(), 1e-6),
        "{} vs {}",
        c,
        h * theta.tan()
    );
}

#[test]
fn laser_offset_from_instrument_center_is_instrument_height_tan_theta() {
    // SPEC 6.3 の式そのもの：器械が θ 傾くと、レーザー求心点は器械の中心（望遠鏡の回転軸）の
    // 真下から 器械高×tanθ だけズレる。器械高は画面に出す値（測点から回転軸までの高さ）を使う
    let mut sim = level_sim();
    for (screw, thumb, turns) in [
        (C, Thumb::Left, 1.0),
        (A, Thumb::Right, 0.7),
        (B, Thumb::Left, 1.3),
    ] {
        assert!(sim.turn_screw(screw, thumb, turns).is_ok());
        let s = snap(&sim);
        let (Some(pose), Some(laser), Some(ih), Some(center)) =
            (s.pose, s.laser, s.instrument_height_mm, s.trunnion)
        else {
            unreachable!("器械は載っている");
        };
        let theta = pose.tilt();
        assert!(theta > arcsec(60.0), "傾いていること θ={theta}");
        let off = (laser.x - center.x).hypot(laser.y - center.y);
        assert!(
            approx(off, ih * theta.tan(), 1e-6),
            "{off} vs {}",
            ih * theta.tan()
        );
    }
}

#[test]
fn leveling_moves_laser_point() {
    // 整準すると傾きが変わるので、レーザー求心点が動く
    let mut sim = level_sim();
    let c0 = centering(&sim);
    assert!(sim.turn_screw(A, Thumb::Right, 0.3).is_ok());
    let c1 = centering(&sim);
    let moved = (c1[0] - c0[0]).hypot(c1[1] - c0[1]);
    assert!(moved > 1.0, "moved {moved}");
}

// ---------------------------------------------------------------------------
// 器械を載せる前：脚頭の穴から真下をのぞく（教科書モード手順1）
// ---------------------------------------------------------------------------

/// 教科書モードで、脚頭の中心が測点から (cx, cy) mm の位置にある三脚（器械はまだ載せていない）
fn textbook_tripod_at(cx: f64, cy: f64) -> Simulator {
    let mut sim = Simulator::new(Mode::Textbook, 1);
    let Ok(t) = Tripod::new(symmetric_feet(cx, cy, 650.0), [1380.0; 3]) else {
        unreachable!("対称な三脚は必ず作れる");
    };
    sim.tripod = t;
    sim
}

#[test]
fn peek_shows_the_station_on_the_opposite_side_of_the_offset() {
    // 脚頭が測点の奥（北）20 mm にあると、のぞいた測点は穴の手前（南）20 mm に見える
    let s = snap(&textbook_tripod_at(0.0, 20.0));
    assert!(!s.mounted && s.laser.is_none(), "器械を載せる前はレーザーがない");
    let Some(p) = s.peek else {
        unreachable!("器械を載せる前は、のぞいた見え方がある");
    };
    assert!(approx(p[0], -s.head_center.x, 1e-9) && approx(p[1], -s.head_center.y, 1e-9));
    assert!(approx(p[0], 0.0, 0.5) && approx(p[1], -20.0, 0.5), "{:?}", p);
    assert!(s.peek_visible);
}

#[test]
fn station_is_not_seen_outside_the_head_hole_view() {
    let near = snap(&textbook_tripod_at(HEAD_HOLE_VIEW_MM - 5.0, 0.0));
    assert!(near.peek_visible, "見える範囲の中なら測点が見える");
    let far = snap(&textbook_tripod_at(0.0, -(HEAD_HOLE_VIEW_MM + 5.0)));
    assert!(far.peek.is_some() && !far.peek_visible, "見える範囲の外では測点は見えない");
}

#[test]
fn moving_the_tripod_moves_the_station_in_the_hole() {
    let mut sim = textbook_tripod_at(0.0, 0.0);
    let Some(before) = snap(&sim).peek else {
        unreachable!();
    };
    // 三脚ごと奥（北）へ運ぶと、測点は穴の手前へ動いて見える
    let mut t = 0.0;
    for _ in 0..10 {
        t += 50.0;
        sim.frame(
            t,
            &[Command::MoveFreeLegs {
                dir: [0.0, 1.0],
                speed: Speed::Fine,
            }],
        );
    }
    let Some(after) = snap(&sim).peek else {
        unreachable!();
    };
    assert!(after[1] < before[1] - 1.0, "{:?} → {:?}", before, after);
    assert!(approx(after[0], before[0], 0.5), "{:?} → {:?}", before, after);
}

#[test]
fn peek_ends_when_the_instrument_is_mounted() {
    let mut sim = textbook_tripod_at(0.0, 0.0);
    assert!(snap(&sim).peek.is_some());
    sim.frame(50.0, &[Command::MountInstrument]);
    let s = snap(&sim);
    assert!(s.mounted && s.peek.is_none() && !s.peek_visible, "器械を載せたら、のぞく見え方は終わる");
    assert!(s.laser.is_some(), "器械を載せたらレーザーの表示に戻る");
    // 現場モードは最初から器械が載っているので、のぞく見え方はない
    assert!(snap(&Simulator::new(Mode::Field, 1)).peek.is_none());
}

// ---------------------------------------------------------------------------
// 器械ずらし・定心桿（SPEC 6.4 / 11章）
// ---------------------------------------------------------------------------

#[test]
fn sliding_changes_centering_by_that_amount() {
    // 求心ズレは、ずらした量だけ変わる。違いは、ずらしたことで器械がわずかに傾いた分
    // （回転の中心の高さ × 傾きの変化）だけ
    let mut sim = level_sim();
    assert!(sim.turn_screw(C, Thumb::Left, 0.2).is_ok());
    let c0 = centering(&sim);
    let (Some(p0), true) = (snap(&sim).pose, sim.set_clamp(false).is_ok()) else {
        unreachable!();
    };
    assert!(sim.slide_instrument([3.0, -1.5]).is_ok());
    let c1 = centering(&sim);
    let Some(p1) = snap(&sim).pose else {
        unreachable!();
    };
    // 鉛直軸の倒れ（水平成分 ÷ 鉛直成分）の変化を、向きも含めて見る
    let lean = |p: &InstrumentPose| {
        let a = p.axis();
        [a.x / a.z, a.y / a.z]
    };
    let (l0, l1) = (lean(&p0), lean(&p1));
    let tilt_shift = p1.pivot.z * (l1[0] - l0[0]).hypot(l1[1] - l0[1]);
    let extra = (c1[0] - c0[0] - 3.0).hypot(c1[1] - c0[1] + 1.5);
    assert!(
        approx(extra, tilt_shift, 1e-6),
        "extra {extra} mm vs {tilt_shift} mm"
    );
    assert!(extra < 0.1, "ずらした量からの違いは0.1mm未満 extra {extra} mm");
}

#[test]
fn clamp_tight_prevents_sliding() {
    let mut sim = level_sim();
    assert!(sim.inst.clamp_tight);
    let before = sim.inst.clone();
    let c0 = centering(&sim);
    assert_eq!(sim.slide_instrument([5.0, 5.0]), Err(Refusal::ClampTight));
    assert_eq!(sim.inst, before);
    assert_eq!(centering(&sim), c0);
    assert_eq!(sim.loops(), 0);
}

#[test]
fn sliding_breaks_bubble_slightly_and_more_on_tilted_head() {
    // 水平な三脚頭：ずらすと傾きがわずかに変わる（オーナー確認：5mmで10″前後）
    let mut sim = level_sim();
    let t0 = tilt_xy(&sim);
    assert!(sim.set_clamp(false).is_ok());
    assert!(sim.slide_instrument([5.0, 0.0]).is_ok());
    let t1 = tilt_xy(&sim);
    let level_change = to_arcsec((t1.0 - t0.0).hypot(t1.1 - t0.1));
    assert!(
        (8.0..=12.0).contains(&level_change),
        "水平な三脚頭で5mm：{level_change}″"
    );

    // 傾いた三脚頭：さらに崩れる（ただし数十秒まで）
    let mut sim = level_sim();
    let feet = sim.tripod.feet;
    assert!(sim.tripod.try_set(feet, [1400.0, 1380.0, 1380.0]).is_ok());
    let t0 = tilt_xy(&sim);
    assert!(sim.set_clamp(false).is_ok());
    assert!(sim.slide_instrument([5.0, 0.0]).is_ok());
    let t1 = tilt_xy(&sim);
    let change = to_arcsec((t1.0 - t0.0).hypot(t1.1 - t0.1));
    assert!(
        change > level_change && change < 120.0,
        "傾いた三脚頭で5mm：{change}″（水平では{level_change}″）"
    );
}

#[test]
fn loop_count_counts_slide_sessions() {
    let mut sim = level_sim();
    assert!(sim.set_clamp(false).is_ok());
    assert!(sim.slide_instrument([1.0, 0.0]).is_ok());
    assert!(sim.slide_instrument([1.0, 0.0]).is_ok());
    assert!(sim.set_clamp(true).is_ok());
    assert_eq!(sim.loops(), 1);
    assert!(sim.set_clamp(false).is_ok());
    assert_eq!(sim.loops(), 1, "緩めただけでは数えない");
    assert!(sim.slide_instrument([0.0, 1.0]).is_ok());
    assert_eq!(sim.loops(), 2);
}

// ---------------------------------------------------------------------------
// 三脚（SPEC 6.4）
// ---------------------------------------------------------------------------

#[test]
fn leg_length_moves_circular_bubble_but_not_laser_much() {
    let mut sim = level_sim();
    let s0 = snap(&sim);
    assert!(sim.change_leg_length(1, 10.0).is_ok());
    let s1 = snap(&sim);
    let b0 = s0.circular.map(|c| c.offset_mm).unwrap_or([f64::NAN; 2]);
    let b1 = s1.circular.map(|c| c.offset_mm).unwrap_or([f64::NAN; 2]);
    let bubble_moved = (b1[0] - b0[0]).hypot(b1[1] - b0[1]);
    let l0 = s0.centering.unwrap_or([f64::NAN; 2]);
    let l1 = s1.centering.unwrap_or([f64::NAN; 2]);
    let laser_moved = (l1[0] - l0[0]).hypot(l1[1] - l0[1]);
    assert!(bubble_moved > 1.0, "気泡 {bubble_moved} mm");
    assert!(laser_moved < 1.0, "求心 {laser_moved} mm");
}

#[test]
fn pressing_leg_sinks_a_few_mm_and_shifts_slightly() {
    let mut sim = Simulator::new(Mode::Field, 99);
    let z0 = sim.tripod.feet[2].z;
    let s0 = snap(&sim);
    assert!(sim.press_leg(2).is_ok());
    let sink = z0 - sim.tripod.feet[2].z;
    assert!((SINK_MIN_MM..=SINK_MAX_MM).contains(&sink), "sink {sink}");
    let s1 = snap(&sim);
    assert_ne!(s0.electronic, s1.electronic);
    assert_eq!(sim.press_leg(2), Err(Refusal::LegAlreadyPlanted));
}

#[test]
fn planted_leg_does_not_move_but_others_do() {
    let mut sim = Simulator::new(Mode::Field, 5);
    assert!(sim.press_leg(0).is_ok());
    let f0 = sim.tripod.feet;
    assert!(sim.move_free_legs([20.0, 0.0]).is_ok());
    let f1 = sim.tripod.feet;
    assert_eq!(f0[0], f1[0]);
    assert!(approx(f1[1].x - f0[1].x, 20.0, 1e-9));
    assert!(approx(f1[2].x - f0[2].x, 20.0, 1e-9));
    assert!(sim.press_leg(1).is_ok());
    assert!(sim.press_leg(2).is_ok());
    assert_eq!(sim.move_free_legs([1.0, 0.0]), Err(Refusal::AllLegsPlanted));
}

// ---------------------------------------------------------------------------
// 6⇄7のループが収束する（SPEC 11章）
// ---------------------------------------------------------------------------

/// A・Bを同時に回す（t>0：内側へ）
fn turn_pair(sim: &mut Simulator, t: f64) {
    if t > 0.0 {
        let _ = sim.turn_screw(A, Thumb::Right, t);
        let _ = sim.turn_screw(B, Thumb::Left, t);
    } else if t < 0.0 {
        let _ = sim.turn_screw(A, Thumb::Left, -t);
        let _ = sim.turn_screw(B, Thumb::Right, -t);
    }
}

/// Cを回す（s>0：上げる）
fn turn_c(sim: &mut Simulator, s: f64) {
    if s > 0.0 {
        let _ = sim.turn_screw(C, Thumb::Left, s);
    } else if s < 0.0 {
        let _ = sim.turn_screw(C, Thumb::Right, -s);
    }
}

/// 整準ネジで電子気泡を合わせる（手順5・7）
fn level_with_screws(sim: &mut Simulator) {
    // ネジの回転と気泡の動きの関係（水平付近）
    let kx = 2.0 * SCREW_PITCH_MM / (SCREW_RADIUS_MM * 3f64.sqrt()); // A・B 1回転あたり X
    let ky = SCREW_PITCH_MM / (1.5 * SCREW_RADIUS_MM); // C 1回転あたり Y
    for _ in 0..30 {
        let (x, y) = tilt_xy(sim);
        if x.abs() < arcsec(2.0) && y.abs() < arcsec(2.0) {
            break;
        }
        turn_pair(sim, -x / kx);
        turn_c(sim, -y / ky);
    }
}

/// 器械をずらして求心を合わせる（手順6）
fn center_by_sliding(sim: &mut Simulator) {
    let _ = sim.set_clamp(false);
    for _ in 0..5 {
        let c = centering(sim);
        if c[0].hypot(c[1]) < 0.05 {
            break;
        }
        let _ = sim.slide_instrument([-c[0], -c[1]]);
    }
    let _ = sim.set_clamp(true);
}

fn run_loop(sim: &mut Simulator) -> bool {
    for _ in 0..8 {
        level_with_screws(sim);
        let s = snap(sim);
        if s.centering_ok && s.electronic.map(|e| e.ok).unwrap_or(false) && s.clamp_tight {
            return true;
        }
        center_by_sliding(sim);
    }
    false
}

#[test]
fn six_seven_loop_converges() {
    let mut sim = level_sim();
    let feet = sim.tripod.feet;
    // 円形気泡は合わせたが、三脚頭がまだ少し（数分）傾いている状態から
    assert!(sim.tripod.try_set(feet, [1381.5, 1379.0, 1380.0]).is_ok());
    assert!(sim.snapshot().head_tilt_deg > 0.03);
    sim.inst.screw_turns = [0.3, -0.2, 0.4];
    sim.inst.slide = [2.0, -3.0];

    level_with_screws(&mut sim);
    // 整準した直後は、求心がずれている（だからループが要る）
    let after_level = snap(&sim).centering_mm.unwrap_or(0.0);
    assert!(
        after_level > CENTERING_TOLERANCE_MM,
        "after level {after_level}"
    );

    assert!(run_loop(&mut sim));
    assert!(
        sim.loops() >= 1 && sim.loops() <= 5,
        "loops {}",
        sim.loops()
    );
    let passed = sim.complete_setup().map(|j| j.passed).unwrap_or(false);
    assert!(passed);
}

/// 手順1・2：踏み込んでいない脚を動かして、レーザー求心点を測点に近づける
fn rough_center_with_legs(sim: &mut Simulator, within_mm: f64) {
    for _ in 0..30 {
        let c = centering(sim);
        if c[0].hypot(c[1]) < within_mm {
            break;
        }
        let step = [(-c[0]).clamp(-100.0, 100.0), (-c[1]).clamp(-100.0, 100.0)];
        if sim.move_free_legs(step).is_err() {
            break;
        }
    }
}

/// 手順4：脚の伸縮で円形気泡を合わせる（気泡がある＝高い側の脚を縮める）
fn level_with_legs(sim: &mut Simulator) {
    for _ in 0..60 {
        let Some(b) = sim.snapshot().circular else {
            return;
        };
        if b.raw_mm < 0.3 {
            break;
        }
        for (leg, a) in LEG_AZIMUTHS.iter().enumerate() {
            let toward = b.offset_mm[0] * a.cos() + b.offset_mm[1] * a.sin();
            let _ = sim.change_leg_length(leg, -toward * 0.8);
        }
    }
}

#[test]
fn six_seven_loop_converges_from_random_starts() {
    for seed in 0..25u64 {
        let mut sim = Simulator::new(Mode::Field, seed);
        let start_tilt = sim.snapshot().head_tilt_deg;
        assert!(start_tilt < 3.0, "seed {seed} start tilt {start_tilt}°");
        // 手順1：三脚を測点の上に置く
        rough_center_with_legs(&mut sim, 3.0);
        // 手順2：一点を踏み込み、残り二点で鉛直を仮合わせ
        assert!(sim.press_leg(0).is_ok());
        rough_center_with_legs(&mut sim, 1.0);
        // 手順3：残り二点も踏み込む
        assert!(sim.press_leg(1).is_ok());
        assert!(sim.press_leg(2).is_ok());
        // 手順4：脚の伸縮で円形気泡
        level_with_legs(&mut sim);
        let circ = sim
            .snapshot()
            .circular
            .map(|c| c.in_circle)
            .unwrap_or(false);
        assert!(circ, "seed {seed}: 円形気泡が入らない");
        // 手順5〜7
        assert!(run_loop(&mut sim), "seed {seed} did not converge");
        assert!(sim.loops() <= 5, "seed {seed} loops {}", sim.loops());
        let passed = sim.complete_setup().map(|j| j.passed).unwrap_or(false);
        assert!(passed, "seed {seed}");
    }
}

// ---------------------------------------------------------------------------
// 据付完了の判定（SPEC 5.4 / 11章）
// ---------------------------------------------------------------------------

#[test]
fn judge_boundaries() {
    let ok_tilt = Some((arcsec(15.4), arcsec(-15.4)));
    // 求心：表示が 0.5 mm なら合格、0.6 mm なら不合格
    assert!(judge_setup(Lang::Ja, true, Some(0.54), ok_tilt, true).0);
    assert!(judge_setup(Lang::Ja, true, Some(0.5), ok_tilt, true).0);
    let (p, r) = judge_setup(Lang::Ja, true, Some(0.55), ok_tilt, true);
    assert!(!p);
    assert!(r.iter().any(|s| s.contains("求心ズレ 0.6 mm")), "{r:?}");
    // 電子気泡：表示が15″なら合格、16″なら不合格（X・Yそれぞれ）
    assert!(!judge_setup(Lang::Ja, true, Some(0.1), Some((arcsec(15.5), 0.0)), true).0);
    let (p, r) = judge_setup(Lang::Ja, true, Some(0.1), Some((0.0, arcsec(-16.0))), true);
    assert!(!p);
    assert!(
        r.iter()
            .any(|s| s.contains("電子気泡 Y") && s.contains("1″ 超えて")),
        "{r:?}"
    );
}

#[test]
fn judge_fails_when_clamp_loose() {
    let (p, r) = judge_setup(Lang::Ja, true, Some(0.0), Some((0.0, 0.0)), false);
    assert!(!p);
    assert!(r.iter().any(|s| s.contains("定心桿")));

    let mut sim = level_sim();
    assert!(sim.set_clamp(false).is_ok());
    let passed = sim.complete_setup().map(|j| j.passed).unwrap_or(true);
    assert!(!passed);
    assert_eq!(sim.phase(), Phase::SetupResult);
    // 不合格なら続行できる
    assert!(sim.continue_setup().is_ok());
    assert_eq!(sim.phase(), Phase::Setup);
}

#[test]
fn level_sim_passes() {
    let mut sim = level_sim();
    let passed = sim.complete_setup().map(|j| j.passed).unwrap_or(false);
    assert!(passed);
}

// ---------------------------------------------------------------------------
// タイマー（SPEC 5.4）
// ---------------------------------------------------------------------------

#[test]
fn timer_starts_on_first_operation_and_stops_on_complete() {
    let mut sim = level_sim();
    sim.frame(1000.0, &[]);
    sim.frame(1500.0, &[]);
    assert_eq!(sim.elapsed_ms(), 0.0);
    assert!(!sim.timer_running());
    sim.frame(
        2000.0,
        &[Command::TurnScrew {
            screw: 0,
            thumb: Thumb::Left,
            speed: Speed::Fine,
        }],
    );
    assert!(sim.timer_running());
    sim.frame(5000.0, &[]);
    assert!(approx(sim.elapsed_ms(), 3000.0, 1e-9));
    sim.frame(6000.0, &[Command::CompleteSetup]);
    let t = sim.judgement().map(|j| j.time_ms).unwrap_or(f64::NAN);
    assert!(approx(t, 4000.0, 1e-9));
    sim.frame(9000.0, &[]);
    assert!(approx(sim.elapsed_ms(), 4000.0, 1e-9));
}

#[test]
fn select_step_does_not_start_timer() {
    let mut sim = level_sim();
    sim.frame(100.0, &[Command::SelectStep { step: 5 }]);
    sim.frame(900.0, &[]);
    assert!(!sim.timer_running());
    assert_eq!(sim.ui_state().current_step, 5);
}

// ---------------------------------------------------------------------------
// モード
// ---------------------------------------------------------------------------

#[test]
fn textbook_mode_warns_on_out_of_step_operation() {
    let mut sim = Simulator::new(Mode::Textbook, 3);
    assert!(!sim.snapshot().mounted);
    // 手順1で器械を載せる（手順2の操作）→ 注意
    assert!(sim.mount_instrument().is_ok());
    let n = sim.notice().cloned();
    assert!(
        matches!(n, Some(ref x) if x.kind == NoticeKind::Warning),
        "{n:?}"
    );
    // 目標を満たしていないと次へ進めない
    sim.frame(0.0, &[Command::NextStep]);
    assert_eq!(sim.ui_state().current_step, 1);
}

#[test]
fn textbook_mode_advances_when_goal_met() {
    let mut sim = Simulator::new(Mode::Textbook, 3);
    let Ok(mut t) = Tripod::new(symmetric_feet(0.0, 0.0, 650.0), [1380.0; 3]) else {
        unreachable!();
    };
    t.planted = [true; 3];
    sim.tripod = t;
    sim.frame(0.0, &[Command::NextStep]);
    assert_eq!(sim.ui_state().current_step, 2);
    sim.frame(10.0, &[Command::MountInstrument, Command::NextStep]);
    assert_eq!(sim.ui_state().current_step, 3);
    assert!(sim
        .notice()
        .map(|n| n.kind != NoticeKind::Warning)
        .unwrap_or(true));
}

#[test]
fn field_mode_starts_mounted_and_clamped() {
    let sim = Simulator::new(Mode::Field, 11);
    let s = sim.snapshot();
    assert!(s.mounted && s.clamp_tight);
    assert!(s.laser.is_some());
}

// ---------------------------------------------------------------------------
// 第2部：観測（SPEC 8章 / 11章）
// ---------------------------------------------------------------------------

fn aim(sim: &mut Simulator, target: Vec3) {
    let Some(eye) = sim.snapshot().trunnion else {
        unreachable!();
    };
    let (az, z, _) = direction_to(eye, target);
    sim.set_telescope(az, z);
}

/// 接眼レンズで十字線を、合焦つまみでターゲットを、くっきりさせる（ふつうの操作で回す）
fn adjust_optics(sim: &mut Simulator, target: Vec3) {
    let Some(eye) = sim.snapshot().trunnion else {
        unreachable!();
    };
    let d_eyepiece = sim.obs.eye_dpt - sim.obs.eyepiece_dpt;
    assert!(sim.turn_eyepiece(d_eyepiece).is_ok());
    let d_focus = 1000.0 / (target - eye).norm() - sim.obs.focus_per_m;
    assert!(sim.turn_focus(d_focus).is_ok());
}

fn optics(sim: &Simulator) -> OpticsView {
    let Some(o) = sim.snapshot().optics else {
        unreachable!("手順9・10ではピントと十字線の見え方がある");
    };
    o
}

fn observation_ready(sim: &mut Simulator) {
    let passed = sim.complete_setup().map(|j| j.passed).unwrap_or(false);
    assert!(passed);
    assert!(sim.start_observation().is_ok());
    assert_eq!(sim.phase(), Phase::Observation);
}

#[test]
fn bs_fs_observation_gives_known_coordinates() {
    let mut sim = level_sim();
    observation_ready(&mut sim);
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    let bs = sim.snapshot().bs_target;
    aim(&mut sim, bs);
    adjust_optics(&mut sim, bs);
    assert!(sim.observe().is_ok());
    // BSを観測したら、水平角の読み＝方向角になる
    let Some(rec) = sim.obs.bs else {
        unreachable!();
    };
    assert!(approx(sim.snapshot().hr_reading, rec.azimuth, 1e-9));
    assert!(approx(rec.horizontal_mm, rec.coord_distance_mm, 0.01));

    let fs = sim.snapshot().fs_target;
    aim(&mut sim, fs);
    adjust_optics(&mut sim, fs);
    assert!(sim.observe().is_ok());
    let Some(f) = sim.obs.fs else {
        unreachable!();
    };
    // 既知の答え：プリズムが実際に立っている位置
    let expect_x = FS_DESIGN.x_mm + FS_ACTUAL_OFFSET_MM[0];
    let expect_y = FS_DESIGN.y_mm + FS_ACTUAL_OFFSET_MM[1];
    assert!(
        approx(f.x_mm, expect_x, 0.01),
        "X {} vs {}",
        f.x_mm,
        expect_x
    );
    assert!(
        approx(f.y_mm, expect_y, 0.01),
        "Y {} vs {}",
        f.y_mm,
        expect_y
    );
    assert!(approx(f.h_mm, FS_DESIGN.h_mm, 0.01), "H {}", f.h_mm);
    assert!(approx(f.dx_mm, FS_ACTUAL_OFFSET_MM[0], 0.01));
    assert!(approx(f.dy_mm, FS_ACTUAL_OFFSET_MM[1], 0.01));
    let g = foresight_ground_world();
    assert!(g.norm() > 1000.0);
}

#[test]
fn observation_reflects_centering_error() {
    // 器械が東へ3mmずれて据わっていれば、FSの座標もおよそ3mmずれる
    let mut sim = level_sim();
    sim.inst.slide = [3.0, 0.0];
    // 判定は不合格になるので、観測の計算だけを確かめるため判定を通さずに観測へ
    let passed = sim.complete_setup().map(|j| j.passed).unwrap_or(true);
    assert!(!passed);
    // 合格した器械で同じ計算をすると誤差なし、ずれた器械では誤差が出ることを確認する
    let mut good = level_sim();
    observation_ready(&mut good);
    let ih = good.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(good.set_station(0, 1, ih).is_ok());
    // ここで器械をずらした状態を再現（観測中は据付の操作はできないため、直接書き換える）
    good.inst.slide = [3.0, 0.0];
    let bs = good.snapshot().bs_target;
    aim(&mut good, bs);
    adjust_optics(&mut good, bs);
    assert!(good.observe().is_ok());
    let fs = good.snapshot().fs_target;
    aim(&mut good, fs);
    adjust_optics(&mut good, fs);
    assert!(good.observe().is_ok());
    let Some(f) = good.obs.fs else {
        unreachable!();
    };
    let err = (f.dx_mm - FS_ACTUAL_OFFSET_MM[0]).hypot(f.dy_mm - FS_ACTUAL_OFFSET_MM[1]);
    assert!(err > 1.0 && err < 6.0, "err {err}");
}

#[test]
fn observe_refused_when_sight_is_off() {
    let mut sim = level_sim();
    observation_ready(&mut sim);
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    let bs = sim.snapshot().bs_target;
    aim(&mut sim, bs);
    adjust_optics(&mut sim, bs);
    // 1′ずらす
    let hz = sim.obs.hz + arcsec(60.0);
    let z = sim.obs.zenith;
    sim.set_telescope(hz, z);
    assert!(matches!(sim.observe(), Err(Refusal::SightOff { .. })));
    assert!(sim.obs.bs.is_none());
}

// ---------------------------------------------------------------------------
// 望遠鏡の調整：照準器・視度・合焦（オーナー確認：2026-11-03 に追加）
// ---------------------------------------------------------------------------

#[test]
fn observation_starts_with_reticle_and_focus_out() {
    // 前に使った人の設定のまま：十字線もピントも合っていない（毎回ちがう開始状態でも）
    for seed in 1..=20 {
        let mut sim = Simulator::new(Mode::Field, seed);
        let level = level_sim();
        sim.tripod = level.tripod.clone();
        sim.inst = level.inst.clone();
        observation_ready(&mut sim);
        let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
        // セットすると手順9（BSの視準）へ進む
        assert!(sim.set_station(0, 1, ih).is_ok());
        let o = optics(&sim);
        assert!(!o.reticle_ok && !o.focus_ok, "seed {seed}: {o:?}");
        assert!(o.reticle_blur > 0.0 && o.target_blur > 0.0);
        assert!(!o.peep_sight);
    }
}

#[test]
fn observe_needs_sharp_reticle_and_focus() {
    let mut sim = level_sim();
    observation_ready(&mut sim);
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    let bs = sim.snapshot().bs_target;
    aim(&mut sim, bs);
    // 十字線がぼやけていると測量できない
    assert_eq!(sim.observe(), Err(Refusal::ReticleBlurry));
    let d = sim.obs.eye_dpt - sim.obs.eyepiece_dpt;
    assert!(sim.turn_eyepiece(d).is_ok());
    assert!(optics(&sim).reticle_ok);
    // ピントが合っていないと測量できない
    assert_eq!(sim.observe(), Err(Refusal::OutOfFocus));
    adjust_optics(&mut sim, bs);
    assert!(sim.observe().is_ok());
    assert!(sim.obs.bs.is_some());
}

#[test]
fn turning_focus_toward_target_sharpens_it() {
    let mut sim = level_sim();
    observation_ready(&mut sim);
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    let bs = sim.snapshot().bs_target;
    aim(&mut sim, bs);
    let Some(eye) = sim.snapshot().trunnion else {
        unreachable!();
    };
    let want = 1000.0 / (bs - eye).norm();
    let dir = (want - sim.obs.focus_per_m).signum();
    // ピントの合う向きへ少しずつ回すと、ぼけは減っていき、やがて合う
    let mut last = optics(&sim).target_blur;
    let mut ok = false;
    for _ in 0..2000 {
        assert!(sim.turn_focus(dir * 0.0005).is_ok());
        let o = optics(&sim);
        if o.focus_ok {
            ok = true;
            break;
        }
        assert!(o.target_blur <= last + 1e-12, "{} > {last}", o.target_blur);
        last = o.target_blur;
    }
    assert!(ok, "ピントが合うところがある");
    // 回しすぎると、またぼける
    assert!(sim.turn_focus(dir * 0.02).is_ok());
    assert!(!optics(&sim).focus_ok);
}

#[test]
fn focus_must_be_redone_for_fs_but_reticle_stays() {
    // BS（約41m）でピントを合わせても、FS（約22m）では合わせ直しが要る。十字線はそのまま
    let mut sim = level_sim();
    observation_ready(&mut sim);
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    let bs = sim.snapshot().bs_target;
    aim(&mut sim, bs);
    adjust_optics(&mut sim, bs);
    assert!(sim.observe().is_ok());
    let fs = sim.snapshot().fs_target;
    aim(&mut sim, fs);
    let o = optics(&sim);
    assert!(o.reticle_ok, "十字線は合わせ直さなくてよい");
    assert!(!o.focus_ok, "FSではピントを合わせ直す");
    assert_eq!(sim.observe(), Err(Refusal::OutOfFocus));
    adjust_optics(&mut sim, fs);
    assert!(sim.observe().is_ok());
}

#[test]
fn peep_sight_is_for_rough_aiming_only() {
    let mut sim = level_sim();
    observation_ready(&mut sim);
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    let bs = sim.snapshot().bs_target;
    aim(&mut sim, bs);
    adjust_optics(&mut sim, bs);
    let r0 = sim.ui_revision();
    sim.frame(0.0, &[Command::SetPeepSight { on: true }]);
    assert!(optics(&sim).peep_sight);
    assert!(sim.ui_revision() > r0, "のぞき方の切り替えは画面の状態に出る");
    // 照準器のままでは測量できない
    assert_eq!(sim.observe(), Err(Refusal::PeepSight));
    assert!(sim.set_peep_sight(false).is_ok());
    assert!(sim.observe().is_ok());
}

#[test]
fn optics_are_turned_only_during_observation() {
    let mut sim = level_sim();
    assert_eq!(sim.turn_focus(0.01), Err(Refusal::NotInObservation));
    assert_eq!(sim.turn_eyepiece(0.5), Err(Refusal::NotInObservation));
    assert_eq!(sim.set_peep_sight(true), Err(Refusal::NotInObservation));
}

#[test]
fn setup_operations_are_refused_during_observation() {
    let mut sim = level_sim();
    observation_ready(&mut sim);
    assert_eq!(
        sim.turn_screw(0, Thumb::Left, 0.1),
        Err(Refusal::NotInSetup)
    );
}

#[test]
fn ui_revision_changes_only_on_discrete_changes() {
    let mut sim = level_sim();
    sim.frame(0.0, &[]);
    let r0 = sim.ui_revision();
    sim.frame(16.0, &[]);
    assert_eq!(sim.ui_revision(), r0);
    sim.frame(32.0, &[Command::SetClamp { tight: false }]);
    assert!(sim.ui_revision() > r0);
}
