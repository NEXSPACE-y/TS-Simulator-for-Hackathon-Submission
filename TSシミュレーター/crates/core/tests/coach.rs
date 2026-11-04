//! AI先輩に渡す事実（操作の記録・講評）の確認。
//! AIには、ここで確かめた事実を言葉にさせるだけにする（正解はRust、言葉はAI）。

use ts_core::coach::Fact;
use ts_core::config::{LEG_AZIMUTHS, NAVI_MAX_CHARS};
use ts_core::instrument::Instrument;
use ts_core::math::Vec3;
use ts_core::sim::FactKind;
use ts_core::tripod::Tripod;
use ts_core::{Command, Lang, Mode, Simulator, Speed, Thumb};

const C: usize = 2;

/// 三脚頭が水平で測点の真上、器械も水平・中央の状態
fn level_sim() -> Simulator {
    let mut sim = Simulator::new(Mode::Field, 1);
    let feet = LEG_AZIMUTHS.map(|a| Vec3::new(650.0 * a.cos(), 650.0 * a.sin(), 0.0));
    let Ok(mut t) = Tripod::new(feet, [1380.0; 3]) else {
        unreachable!("対称な三脚は必ず作れる");
    };
    t.planted = [true; 3];
    sim.tripod = t;
    sim.inst = Instrument::default();
    sim
}

fn value<'a>(facts: &'a [Fact], key: &str) -> Option<&'a str> {
    facts.iter().find(|f| f.key == key).map(|f| f.value.as_str())
}

/// 整準ネジCを、親指を左へ（Cが上がる＝電子気泡のYが＋へ崩れる）回し続ける
fn turn_c_away(sim: &mut Simulator, t: &mut f64, frames: usize) {
    for _ in 0..frames {
        *t += 50.0;
        sim.frame(
            *t,
            &[Command::TurnScrew {
                screw: C,
                thumb: Thumb::Left,
                speed: Speed::Medium,
            }],
        );
    }
}

#[test]
fn review_facts_appear_only_after_complete() {
    let mut sim = level_sim();
    assert!(sim.ai_facts(FactKind::Review).is_none());
    sim.frame(0.0, &[Command::CompleteSetup]);
    let Some(facts) = sim.ai_facts(FactKind::Review) else {
        unreachable!("据付完了のあとは講評の事実がある");
    };
    assert_eq!(value(&facts, "結果"), Some("合格"));
    assert_eq!(value(&facts, "遠回り"), Some("特になし"));
}

#[test]
fn turning_the_wrong_way_is_logged_and_becomes_a_detour() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    // 4秒間、電子気泡が崩れる向きに回す
    turn_c_away(&mut sim, &mut t, 80);
    let log = sim.coach_log();
    assert!(log.screw_single_ms > 3_900.0, "1本ずつ回した時間：{}", log.screw_single_ms);
    assert!(log.screw_worse_ms > 3_000.0, "悪くなる向きの時間：{}", log.screw_worse_ms);
    assert_eq!(log.segments.len(), 1, "同じ操作を続けている間は1件の記録");
    sim.frame(t + 50.0, &[Command::CompleteSetup]);
    let Some(facts) = sim.ai_facts(FactKind::Review) else {
        unreachable!();
    };
    assert_eq!(value(&facts, "結果"), Some("不合格"));
    assert!(facts
        .iter()
        .any(|f| f.key == "遠回り" && f.value.contains("悪くなる向き")));
    assert!(value(&facts, "不合格の理由").is_some_and(|v| v.contains("電子気泡 Y")));
}

/// 定心桿を緩めて器械を東へ少しずらし、締める（4フレームずらす）
fn slide_once(sim: &mut Simulator, t: &mut f64) {
    *t += 50.0;
    sim.frame(*t, &[Command::SetClamp { tight: false }]);
    for _ in 0..4 {
        *t += 50.0;
        sim.frame(
            *t,
            &[Command::SlideInstrument {
                dir: [1.0, 0.0],
                speed: Speed::Fine,
            }],
        );
    }
    *t += 50.0;
    sim.frame(*t, &[Command::SetClamp { tight: true }]);
}

#[test]
fn time_is_counted_per_step() {
    // 現場モードは手順一覧を押さなくても、状態から手順が決まる
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    // まだずらしていない整準 → 手順5
    turn_c_away(&mut sim, &mut t, 20);
    // 定心桿を緩めてずらす → 手順6（締めたフレームから、ずらした後の整準 → 手順7）
    slide_once(&mut sim, &mut t);
    turn_c_away(&mut sim, &mut t, 10);
    let log = sim.coach_log();
    assert!(log.step_ms[4] > 900.0 && log.step_ms[4] < 1_100.0, "手順5：{}", log.step_ms[4]);
    assert!(log.step_ms[5] > 150.0 && log.step_ms[5] < 350.0, "手順6：{}", log.step_ms[5]);
    assert!(log.step_ms[6] > 400.0 && log.step_ms[6] < 650.0, "手順7：{}", log.step_ms[6]);
    let steps: Vec<u8> = log.visits.iter().map(|v| v.0).collect();
    assert_eq!(steps, vec![5, 6, 7]);
}

/// 何でも質問でAIに渡す「いまの手順」の番号
fn step_in_status(sim: &Simulator) -> Option<u8> {
    let facts = sim.ai_facts(FactKind::Status)?;
    let v = value(&facts, "いまの手順")?;
    v.strip_prefix("手順")?.chars().next()?.to_digit(10).map(|d| d as u8)
}

/// 画面の手順一覧で、いま選ばれている手順
fn step_on_screen(sim: &Simulator) -> Option<u8> {
    sim.ui_state().steps.iter().find(|s| s.current).map(|s| s.no)
}

#[test]
fn field_step_follows_the_legs() {
    let mut sim = Simulator::new(Mode::Field, 3);
    sim.frame(0.0, &[]);
    assert_eq!(step_in_status(&sim), Some(1), "1本も踏み込んでいない → 手順1");
    sim.frame(50.0, &[Command::PressLeg { leg: 0 }]);
    assert_eq!(step_in_status(&sim), Some(2), "1本だけ踏み込んだ → 手順2");
    sim.frame(100.0, &[Command::PressLeg { leg: 1 }]);
    assert_eq!(step_in_status(&sim), Some(3), "2本踏み込んだ → 手順3");
    assert_eq!(step_on_screen(&sim), Some(1), "画面の手順一覧は選んだ手順のまま");
}

#[test]
fn field_step_follows_the_bubbles_and_the_clamp() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    assert_eq!(step_in_status(&sim), Some(5), "両方合っていて、まだずらしていない → 手順5");

    // 脚の伸縮で円形気泡を円の外へ → 手順4
    let mut legs = level_sim();
    let mut tl = 0.0;
    for _ in 0..200 {
        tl += 50.0;
        legs.frame(
            tl,
            &[Command::ChangeLegLength {
                leg: 0,
                extend: true,
                speed: Speed::Medium,
            }],
        );
        if legs.snapshot().circular.is_some_and(|c| !c.in_circle) {
            break;
        }
    }
    assert!(legs.snapshot().circular.is_some_and(|c| !c.in_circle));
    assert_eq!(step_in_status(&legs), Some(4), "円形気泡が円の外 → 手順4");

    // 定心桿を緩めている → 手順6
    t += 50.0;
    sim.frame(t, &[Command::SetClamp { tight: false }]);
    assert_eq!(step_in_status(&sim), Some(6), "定心桿を緩めている → 手順6");
    for _ in 0..20 {
        t += 50.0;
        sim.frame(
            t,
            &[Command::SlideInstrument {
                dir: [1.0, 0.0],
                speed: Speed::Medium,
            }],
        );
    }
    t += 50.0;
    sim.frame(t, &[Command::SetClamp { tight: true }]);

    // ずらした後に電子気泡が外れた → 手順7
    turn_c_away(&mut sim, &mut t, 10);
    assert!(sim.snapshot().electronic.is_some_and(|e| !e.ok));
    assert_eq!(step_in_status(&sim), Some(7), "ずらした後の整準 → 手順7");

    // 次の操作どおりに整準して、電子気泡が合い、求心がずれたまま → 手順6
    for _ in 0..2_000 {
        if sim.snapshot().electronic.is_some_and(|e| e.ok) {
            break;
        }
        assert_eq!(step_in_status(&sim), Some(7), "整準している間は手順7");
        let cmds = commands_for(&sim, &sim.next_action());
        t += 50.0;
        sim.frame(t, &cmds);
    }
    let s = sim.snapshot();
    assert!(s.electronic.is_some_and(|e| e.ok) && !s.centering_ok, "求心ズレ{:?}", s.centering_mm);
    assert_eq!(step_in_status(&sim), Some(6), "電子気泡は合っていて求心がずれている → 手順6");
    assert_eq!(step_on_screen(&sim), Some(1), "画面の手順一覧は選んだ手順のまま");
}

#[test]
fn field_time_is_not_counted_to_the_selected_step() {
    // 手順一覧を押さずに次の操作どおりに合格まで進めても、時間は手順1だけに付かない
    for seed in [1, 3, 7] {
        let sim = follow_hints_to_pass(Mode::Field, seed);
        let log = sim.coach_log();
        let total: f64 = log.step_ms.iter().sum();
        assert!(log.step_ms[0] < total * 0.5, "seed {}：手順1に{}ms（全体{}ms）", seed, log.step_ms[0], total);
        assert!(log.step_ms[3..].iter().sum::<f64>() > 0.0, "seed {}：整準の手順に時間が付く", seed);
        // 脚を踏み込んだ後に、手順1〜3へ戻ることはない
        let steps: Vec<u8> = log.visits.iter().map(|v| v.0).collect();
        let first_level = steps.iter().position(|s| *s >= 4);
        assert!(first_level.is_some(), "seed {}：手順の流れ {:?}", seed, steps);
        if let Some(i) = first_level {
            assert!(steps[i..].iter().all(|s| *s >= 4), "seed {}：手順の流れ {:?}", seed, steps);
        }
        let legs: Vec<u8> = steps.iter().copied().take_while(|s| *s < 4).collect();
        assert!(legs.windows(2).all(|w| w[0] < w[1]), "seed {}：脚の手順は1→2→3の順 {:?}", seed, steps);
    }
}

#[test]
fn loops_record_the_state_before_sliding() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    turn_c_away(&mut sim, &mut t, 80);
    // 電子気泡が大きく崩れたまま、定心桿を緩めてずらす
    t += 50.0;
    sim.frame(t, &[Command::SetClamp { tight: false }]);
    for _ in 0..5 {
        t += 50.0;
        sim.frame(
            t,
            &[Command::SlideInstrument {
                dir: [1.0, 0.0],
                speed: Speed::Medium,
            }],
        );
    }
    let log = sim.coach_log();
    assert_eq!(log.loops.len(), 1);
    assert!(log.loops[0].tilt_sec.is_some_and(|s| s > 60.0));
    assert_eq!(log.slide_before_level, 1);
}

#[test]
fn sliding_with_clamp_tight_counts_once_per_attempt() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[Command::SetClamp { tight: false }]);
    sim.frame(t + 10.0, &[Command::SetClamp { tight: true }]);
    t += 10.0;
    for _ in 0..10 {
        t += 50.0;
        sim.frame(
            t,
            &[Command::SlideInstrument {
                dir: [1.0, 0.0],
                speed: Speed::Medium,
            }],
        );
    }
    assert_eq!(sim.coach_log().clamp_refusals, 1);
}

// ---------------------------------------------------------------------------
// 次の操作（コアが向きを決める。AIはこれを言葉にするだけ）
// ---------------------------------------------------------------------------

use ts_core::coach::{level_hint, slide_hint};
use ts_core::math::Mat3;
use ts_core::tripod::HeadPose;

fn level_head() -> HeadPose {
    HeadPose {
        center: Vec3::new(0.0, 0.0, 1250.0),
        rot: Mat3::IDENTITY,
    }
}

#[test]
fn level_hint_follows_the_left_thumb_rule() {
    // A・Bを内側へ回すと気泡は＋X（左手親指の向き）。戻すには外側へ
    let mut inst = Instrument::default();
    assert!(inst.turn_screw(0, Thumb::Right, 0.05).is_ok());
    assert!(inst.turn_screw(1, Thumb::Left, 0.05).is_ok());
    let Some(h) = level_hint(Lang::Ja, &inst, &level_head()) else {
        unreachable!("傾いているので次の操作がある");
    };
    assert!(h.contains("A・Bを同時に外側へ"), "{}", h);
    assert!(h.contains("Xがいま+"), "{}", h);
}

#[test]
fn level_hint_uses_screw_c_for_y() {
    // Cを上げる（親指を左）と＋Y。戻すには親指を右へ
    let mut inst = Instrument::default();
    assert!(inst.turn_screw(2, Thumb::Left, 0.05).is_ok());
    let Some(h) = level_hint(Lang::Ja, &inst, &level_head()) else {
        unreachable!();
    };
    assert!(h.contains("整準ネジCを、親指を右へ"), "{}", h);
}

#[test]
fn slide_hint_points_back_to_the_station() {
    // レーザー点が測点の右（東）2 mm にある → 器械を左へずらす
    let h = slide_hint(Lang::Ja, [2.0, 0.0], true);
    assert!(h.contains("定心桿を緩めて"), "{}", h);
    assert!(h.contains("左へ約2.0 mm"), "{}", h);
    let h = slide_hint(Lang::Ja, [0.0, -1.5], false);
    assert!(h.contains("奥へ約1.5 mm"), "{}", h);
}

#[test]
fn status_facts_carry_the_next_action() {
    // 始めたばかり：三脚は測点から離れていて、踏み込んでいない
    let sim = Simulator::new(Mode::Field, 5);
    let Some(facts) = sim.ai_facts(FactKind::Status) else {
        unreachable!("いまの状態はいつでもある");
    };
    let next = value(&facts, "次の操作").unwrap_or("");
    assert!(next.contains("三脚ごと"), "{}", next);
    assert!(value(&facts, "左手親指の法則").is_some());
    assert!(facts.len() <= 60);
    assert!(facts
        .iter()
        .all(|f| f.value.chars().count() <= 240 && f.key.chars().count() <= 40));
}

#[test]
fn next_action_after_leveling_is_complete() {
    let sim = level_sim();
    assert!(sim.next_action().contains("「据付完了」を押す"), "{}", sim.next_action());
}

#[test]
fn next_action_in_observation_starts_with_the_eyepiece() {
    let mut sim = level_sim();
    sim.frame(0.0, &[Command::CompleteSetup]);
    sim.frame(10.0, &[Command::StartObservation]);
    assert!(sim.next_action().contains("電子平板"), "{}", sim.next_action());
    sim.frame(
        20.0,
        &[Command::SetStation {
            station: 0,
            backsight: 1,
            height_m: 1.5,
        }],
    );
    let a = sim.next_action();
    // 観測を始めたときは、十字線がぼやけている（またはターゲットが視野から遠い）
    assert!(a.contains("接眼レンズ") || a.contains("照準器"), "{}", a);
}

// ---------------------------------------------------------------------------
// FSの座標の計算（観測の解説）
// ---------------------------------------------------------------------------

use ts_core::survey::direction_to;

fn aim_and_focus(sim: &mut Simulator, target: Vec3) {
    let Some(eye) = sim.snapshot().trunnion else {
        unreachable!();
    };
    let (az, z, _) = direction_to(eye, target);
    sim.set_telescope(az, z);
    let d_eyepiece = sim.obs.eye_dpt - sim.obs.eyepiece_dpt;
    assert!(sim.turn_eyepiece(d_eyepiece).is_ok());
    let d_focus = 1000.0 / (target - eye).norm() - sim.obs.focus_per_m;
    assert!(sim.turn_focus(d_focus).is_ok());
}

#[test]
fn calc_facts_show_each_step_with_the_core_numbers() {
    let mut sim = level_sim();
    assert!(sim.complete_setup().map(|j| j.passed).unwrap_or(false));
    assert!(sim.start_observation().is_ok());
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN);
    assert!(sim.set_station(0, 1, ih).is_ok());
    assert!(sim.ai_facts(FactKind::Calc).is_none(), "FSを観測するまでは無い");
    let bs = sim.snapshot().bs_target;
    aim_and_focus(&mut sim, bs);
    assert!(sim.observe().is_ok());
    let fs = sim.snapshot().fs_target;
    aim_and_focus(&mut sim, fs);
    assert!(sim.observe().is_ok());
    let Some(facts) = sim.ai_facts(FactKind::Calc) else {
        unreachable!("FSを観測したら計算の事実がある");
    };
    let Some(f) = sim.obs.fs else {
        unreachable!();
    };
    // 途中の値と答えは、観測記録と同じ数字
    let x = value(&facts, "4. X＝機械点のX＋ΔX").unwrap_or("");
    assert!(x.ends_with(&format!("{:.3} m", f.x_mm / 1000.0)), "{}", x);
    assert_eq!(
        value(&facts, "5. 観測−設計"),
        Some("X +6 mm・Y \u{2212}4 mm・H ±0 mm")
    );
    assert!(facts
        .iter()
        .all(|f| f.value.chars().count() <= 240 && f.key.chars().count() <= 40));
}

// ---------------------------------------------------------------------------
// 現場の指示の聞き取り（正解と判定はコア）
// ---------------------------------------------------------------------------

#[test]
fn instruction_is_set_and_judged_by_the_core() {
    let mut sim = level_sim();
    assert!(sim.complete_setup().map(|j| j.passed).unwrap_or(false));
    assert!(sim.start_observation().is_ok());
    assert!(sim.ai_facts(FactKind::Instruction).is_none());
    sim.frame(0.0, &[Command::RequestInstruction]);
    let Some(facts) = sim.ai_facts(FactKind::Instruction) else {
        unreachable!("指示を頼んだら事実がある");
    };
    // 機械点は据えた点、BS点はターゲットが立っている点
    assert_eq!(value(&facts, "機械点"), Some("T-1"));
    assert_eq!(value(&facts, "BS点"), Some("T-2"));
    assert!(value(&facts, "言い回し").is_some());

    // BSを間違え、器械高を測らずに入れた
    assert!(sim.set_station(0, 2, 1600.0).is_ok());
    let Some(r) = sim.obs.instruction_result.clone() else {
        unreachable!("指示のあとのセットは判定される");
    };
    assert!(!r.ok);
    assert!(r.items.iter().any(|s| s.contains("BS点：T-3（指示はT-2）")), "{:?}", r.items);
    assert!(r.items.iter().any(|s| s.contains("巻尺で測った値は")), "{:?}", r.items);

    // 指示どおり（器械高は巻尺で測った値を mm に丸めて入れる）
    sim.frame(10.0, &[Command::RequestInstruction]);
    assert!(sim.obs.instruction_result.is_none());
    let ih = sim.snapshot().instrument_height_mm.unwrap_or(f64::NAN).round();
    assert!(sim.set_station(0, 1, ih).is_ok());
    let Some(r) = sim.obs.instruction_result.clone() else {
        unreachable!();
    };
    assert!(r.ok, "{:?}", r.items);
}

// ---------------------------------------------------------------------------
// 次の操作のとおりに操作すると、据付が合格まで行くこと（向きの言葉の確かめ）
// ---------------------------------------------------------------------------

const DIRS: [(&str, [f64; 2]); 8] = [
    ("右手前", [0.707, -0.707]),
    ("左手前", [-0.707, -0.707]),
    ("右奥", [0.707, 0.707]),
    ("左奥", [-0.707, 0.707]),
    ("手前", [0.0, -1.0]),
    ("奥", [0.0, 1.0]),
    ("右", [1.0, 0.0]),
    ("左", [-1.0, 0.0]),
];

/// 文の中の「〇〇へ」「〇〇の方へ」の向き
fn dir_in(text: &str) -> Option<[f64; 2]> {
    DIRS.iter()
        .find(|(w, _)| text.contains(&format!("{}へ", w)) || text.contains(&format!("{}の方へ", w)))
        .map(|(_, v)| *v)
}

/// 次の操作の文を、操作に直す（文の中身＝向きは、コアが決めたもの）
fn commands_for(sim: &Simulator, hint: &str) -> Vec<Command> {
    let screw = |screw, thumb| Command::TurnScrew {
        screw,
        thumb,
        speed: Speed::Fine,
    };
    let unplanted = sim.tripod.planted.iter().position(|p| !p).unwrap_or(0);
    let single = |name: &str| {
        ["A", "B", "C"]
            .iter()
            .position(|n| hint.starts_with(&format!("整準ネジ{}を、親指を{}へ", n, name)))
    };
    if hint.contains("「据付完了」を押す") {
        vec![Command::CompleteSetup]
    } else if hint.contains("「次の手順へ」を押す") {
        vec![Command::NextStep]
    } else if hint.contains("「器械を載せて定心桿で固定する」を押す") {
        vec![Command::MountInstrument]
    } else if let Some(k) = single("左") {
        vec![screw(k, Thumb::Left)]
    } else if let Some(k) = single("右") {
        vec![screw(k, Thumb::Right)]
    } else if hint.starts_with("三脚を") {
        dir_in(hint)
            .map(|dir| {
                vec![Command::MoveFreeLegs {
                    dir,
                    speed: Speed::Medium,
                }]
            })
            .unwrap_or_default()
    } else if hint.contains("踏み込む") {
        vec![Command::PressLeg { leg: unplanted }]
    } else if hint.contains("三脚ごと") || hint.contains("踏み込んでいない脚を") {
        let speed = if hint.contains("三脚ごと") {
            Speed::Medium
        } else {
            Speed::Fine
        };
        dir_in(hint)
            .map(|dir| vec![Command::MoveFreeLegs { dir, speed }])
            .unwrap_or_default()
    } else if let Some(i) = ["脚1", "脚2", "脚3"].iter().position(|l| hint.starts_with(l)) {
        vec![Command::ChangeLegLength {
            leg: i,
            extend: hint.contains("伸ばす"),
            speed: Speed::Fine,
        }]
    } else if hint.contains("A・Bを同時に内側へ") {
        vec![screw(0, Thumb::Right), screw(1, Thumb::Left)]
    } else if hint.contains("A・Bを同時に外側へ") {
        vec![screw(0, Thumb::Left), screw(1, Thumb::Right)]
    } else if hint.contains("整準ネジCを、親指を左へ") {
        vec![screw(C, Thumb::Left)]
    } else if hint.contains("整準ネジCを、親指を右へ") {
        vec![screw(C, Thumb::Right)]
    } else if hint.starts_with("定心桿を緩めて") {
        vec![Command::SetClamp { tight: false }]
    } else if hint.starts_with("器械を") {
        dir_in(hint)
            .map(|dir| {
                vec![Command::SlideInstrument {
                    dir,
                    speed: Speed::Fine,
                }]
            })
            .unwrap_or_default()
    } else if hint.starts_with("定心桿を締める") {
        vec![Command::SetClamp { tight: true }]
    } else {
        vec![]
    }
}

fn follow_hints_to_pass(mode: Mode, seed: u64) -> Simulator {
    let mut sim = Simulator::new(mode, seed);
    let mut t = 0.0;
    sim.frame(t, &[]);
    for _ in 0..40_000 {
        let hint = sim.next_action();
        let cmds = commands_for(&sim, &hint);
        assert!(!cmds.is_empty(), "操作に直せない次の操作：{}", hint);
        t += 50.0;
        sim.frame(t, &cmds);
        if let Some(j) = sim.judgement() {
            assert!(j.passed, "次の操作どおりに「据付完了」を押したら合格するはず：{:?}", j.reasons);
            return sim;
        }
    }
    unreachable!("次の操作どおりに進めても終わらなかった（最後の操作：{}）", sim.next_action());
}

#[test]
fn following_next_actions_reaches_a_pass() {
    for seed in [1, 2, 3, 7, 11, 42] {
        follow_hints_to_pass(Mode::Field, seed);
        follow_hints_to_pass(Mode::Textbook, seed);
    }
}


// ---------------------------------------------------------------------------
// 操作中のひとこと（きっかけと中身はコアが決める）
// ---------------------------------------------------------------------------

use ts_core::sim::AdviceKind;

fn idle(sim: &mut Simulator, t: &mut f64, frames: usize) {
    for _ in 0..frames {
        *t += 50.0;
        sim.frame(*t, &[]);
    }
}

#[test]
fn turning_the_wrong_way_brings_advice_with_the_right_way() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    // 整準の段階（円形気泡は入っている）で、ゆっくり逆向きに回し続ける
    for _ in 0..80 {
        t += 50.0;
        sim.frame(
            t,
            &[Command::TurnScrew {
                screw: C,
                thumb: Thumb::Left,
                speed: Speed::Fine,
            }],
        );
    }
    let Some(a) = sim.advice() else {
        unreachable!("逆向きに回し続けたら、ひとことが出る");
    };
    assert_eq!(a.kind, AdviceKind::WrongWay);
    let content = value(&a.facts, "ひとことの中身").unwrap_or("");
    // Cを親指左で上げすぎた → 戻すのは親指を右へ
    assert!(content.contains("逆") && content.contains("親指を右へ"), "{}", content);
    assert_eq!(a.fallback, content);
    assert!(sim.ui_state().advice.is_some());
    // 間をあけるので、すぐには次のひとことは出ない
    let id = a.id;
    turn_c_away(&mut sim, &mut t, 200);
    assert_eq!(sim.advice().map(|a| a.id), Some(id));
}

#[test]
fn wrong_way_advice_waits_until_the_leveling_stage() {
    // 三脚を置く前にネジを回しても、「逆向き」のひとことは出さない
    let mut sim = Simulator::new(Mode::Field, 3);
    let mut t = 0.0;
    sim.frame(t, &[]);
    turn_c_away(&mut sim, &mut t, 100);
    assert!(sim.advice().map(|a| a.kind != AdviceKind::WrongWay).unwrap_or(true));
}

#[test]
fn staying_long_in_a_step_brings_advice_once() {
    // 手順一覧は手順1のまま。状態（両方合っていて、まだずらしていない）から手順5になる
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    // 最初の操作でタイマーが動く
    sim.frame(t + 50.0, &[Command::SetClamp { tight: true }]);
    sim.frame(t + 60.0, &[Command::SetClamp { tight: false }]);
    sim.frame(t + 70.0, &[Command::SetClamp { tight: true }]);
    t += 70.0;
    idle(&mut sim, &mut t, 700);
    assert!(sim.advice().is_none(), "目安の40秒まではひとことは出ない");
    idle(&mut sim, &mut t, 200);
    let Some(a) = sim.advice() else {
        unreachable!("目安を過ぎたらひとことが出る");
    };
    assert_eq!(a.kind, AdviceKind::OverTime);
    assert!(value(&a.facts, "きっかけ").is_some_and(|v| v.contains("手順5")));
    let id = a.id;
    idle(&mut sim, &mut t, 1500);
    assert_eq!(sim.advice().map(|a| a.id), Some(id), "同じ手順では1回だけ");
}

#[test]
fn sliding_before_leveling_brings_advice() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[]);
    // 電子気泡を大きく崩す（逆向きのひとことが出ないよう、短く速く）
    for _ in 0..40 {
        t += 50.0;
        sim.frame(
            t,
            &[Command::TurnScrew {
                screw: C,
                thumb: Thumb::Left,
                speed: Speed::Coarse,
            }],
        );
    }
    let first = sim.advice().map(|a| a.id);
    t += 31_000.0;
    sim.frame(t, &[Command::SetClamp { tight: false }]);
    t += 50.0;
    sim.frame(
        t,
        &[Command::SlideInstrument {
            dir: [1.0, 0.0],
            speed: Speed::Fine,
        }],
    );
    t += 50.0;
    sim.frame(
        t,
        &[Command::SlideInstrument {
            dir: [1.0, 0.0],
            speed: Speed::Fine,
        }],
    );
    let Some(a) = sim.advice() else {
        unreachable!();
    };
    assert_ne!(Some(a.id), first);
    assert_eq!(a.kind, AdviceKind::SlideBeforeLevel);
}

// ---------------------------------------------------------------------------
// 教科書モードのナビ（きっかけと中身はコアが決める）
// ---------------------------------------------------------------------------

use ts_core::sim::NaviKind;

#[test]
fn textbook_navi_starts_at_step_one_and_field_mode_has_none() {
    let mut sim = Simulator::new(Mode::Textbook, 4);
    sim.frame(0.0, &[]);
    let Some(n) = sim.navi() else {
        unreachable!("最初のフレームで、手順1に入ったナビが出る");
    };
    assert_eq!(n.kind, NaviKind::Enter);
    assert!(n.ai);
    assert!(n.text.starts_with("手順1"), "{}", n.text);
    assert!(!n.speech.contains('（'), "{}", n.speech);
    // 目標は操作盤に出ているので、読み上げでは言わない（AIには事実で渡す）
    assert!(!n.text.contains("目標"), "{}", n.text);
    let facts = sim.ai_facts(FactKind::Navi).unwrap_or_default();
    assert!(facts.iter().any(|f| f.key == "この手順の目標"));
    assert!(sim.ai_facts(FactKind::Navi).is_some());
    assert!(sim.ui_state().navi.is_some());

    let mut field = Simulator::new(Mode::Field, 4);
    field.frame(0.0, &[]);
    assert!(field.navi().is_none());
}

#[test]
fn textbook_navi_reads_out_of_step_warnings() {
    let mut sim = Simulator::new(Mode::Textbook, 4);
    sim.frame(0.0, &[]);
    // 手順1で器械を載せる（手順2の操作）
    sim.frame(5_000.0, &[Command::MountInstrument]);
    let Some(n) = sim.navi() else {
        unreachable!();
    };
    assert_eq!(n.kind, NaviKind::Warning);
    assert!(n.text.starts_with("注意"), "{}", n.text);
    assert!(!n.ai);
}

#[test]
fn textbook_navi_follows_the_whole_setup() {
    let mut sim = Simulator::new(Mode::Textbook, 2);
    let mut t = 0.0;
    sim.frame(t, &[]);
    let mut seen: Vec<(NaviKind, u8)> = Vec::new();
    let mut last_id = 0;
    for _ in 0..40_000 {
        if let Some(n) = sim.navi() {
            if n.id != last_id {
                last_id = n.id;
                let step = n.text.chars().nth(2).and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
                seen.push((n.kind, step));
                // 読み上げる文は、長さの目安に収める
                assert!(
                    n.speech.chars().count() <= NAVI_MAX_CHARS,
                    "ナビが長すぎる（{}字）：{}",
                    n.speech.chars().count(),
                    n.speech
                );
            }
        }
        let hint = sim.next_action();
        let cmds = commands_for(&sim, &hint);
        assert!(!cmds.is_empty(), "操作に直せない次の操作：{}", hint);
        t += 50.0;
        sim.frame(t, &cmds);
        if sim.judgement().is_some() {
            break;
        }
    }
    assert!(sim.judgement().is_some_and(|j| j.passed));
    // 7つの手順それぞれに入ったナビがあり、目標を満たしたナビと、次の操作のナビもある
    for step in 1..=7u8 {
        assert!(
            seen.iter().any(|(k, s)| *k == NaviKind::Enter && *s == step),
            "手順{}のナビが無い：{:?}",
            step,
            seen
        );
    }
    assert!(seen.iter().any(|(k, _)| *k == NaviKind::Done));
    assert!(seen.iter().any(|(k, _)| *k == NaviKind::Action));
    // 読みすぎない（1手順あたり平均10件まで）
    assert!(seen.len() <= 70, "ナビが多すぎる：{}件", seen.len());
}

// ---------------------------------------------------------------------------
// 英語の表示：画面に出る文に日本語が混ざらない（AIに渡す事実は日本語のまま）
// ---------------------------------------------------------------------------

fn has_japanese(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c, '\u{3000}'..='\u{30ff}' | '\u{4e00}'..='\u{9fff}' | '\u{ff01}'..='\u{ff60}')
    })
}

/// 画面に出る文（手順・目標・知らせ・判定・観測・ひとこと・ナビ・次の操作）
fn screen_texts(sim: &Simulator) -> Vec<String> {
    let ui = sim.ui_state();
    let mut out = vec![sim.next_action()];
    for s in &ui.steps {
        out.push(s.label.to_string());
        out.push(s.short.to_string());
    }
    out.extend(ui.goal.map(|g| g.text));
    out.extend(ui.notice.map(|n| n.text));
    if let Some(j) = ui.judgement {
        out.extend(j.reasons);
        out.extend([j.time_text, j.centering_text, j.tilt_x_text, j.tilt_y_text]);
        out.extend(j.rating_text);
    }
    out.extend(ui.observation.height_text);
    out.extend(ui.observation.instruction.map(|i| i.fallback));
    if let Some(r) = ui.observation.instruction_result {
        out.extend(r.items);
    }
    out.extend(ui.advice.map(|a| a.fallback));
    if let Some(n) = ui.navi {
        out.extend([n.text, n.speech]);
    }
    out
}

fn assert_english(sim: &Simulator, what: &str) {
    for s in screen_texts(sim) {
        assert!(!has_japanese(&s), "{}：{}", what, s);
    }
}

/// 英語の文が入っても、AIに渡す事実は中継の上限（見出し40字・値240字）に収まる
fn assert_facts_fit(sim: &Simulator, what: &str) {
    for kind in [FactKind::Status, FactKind::Advice, FactKind::Navi, FactKind::Instruction, FactKind::Calc] {
        for f in sim.ai_facts(kind).unwrap_or_default() {
            assert!(
                f.key.chars().count() <= 40 && f.value.chars().count() <= 240,
                "{}：{} = {}",
                what,
                f.key,
                f.value
            );
        }
    }
}

#[test]
fn english_screen_has_no_japanese_through_a_whole_run() {
    for mode in [Mode::Field, Mode::Textbook] {
        for seed in [1, 3, 7] {
            // 同じ初期状態の日本語と英語を、同じ操作で並べて進める（操作は日本語の次の操作から作る）
            let mut ja = Simulator::new(mode, seed);
            let mut en = Simulator::new(mode, seed);
            en.set_lang(Lang::En);
            let mut t = 0.0;
            ja.frame(t, &[]);
            en.frame(t, &[]);
            let mut passed = false;
            for i in 0..40_000 {
                assert_english(&en, &format!("{:?} seed {} 据付中", mode, seed));
                if i % 25 == 0 {
                    assert_facts_fit(&en, "据付中の事実");
                }
                let cmds = commands_for(&ja, &ja.next_action());
                assert!(!cmds.is_empty(), "操作に直せない次の操作：{}", ja.next_action());
                t += 50.0;
                ja.frame(t, &cmds);
                en.frame(t, &cmds);
                if let Some(j) = en.judgement() {
                    passed = j.passed;
                    break;
                }
            }
            assert!(passed, "{:?} seed {}：英語でも同じ操作で合格する", mode, seed);
            assert_english(&en, "結果画面");
            assert_eq!(en.snapshot().centering_mm, ja.snapshot().centering_mm, "言語で動きは変わらない");

            // 観測：指示 → 機械点・BSのセット → BS → FS
            assert!(en.start_observation().is_ok());
            assert_english(&en, "観測の始め");
            t += 50.0;
            en.frame(t, &[Command::RequestInstruction]);
            assert_english(&en, "指示");
            let ih = en.snapshot().instrument_height_mm.unwrap_or(f64::NAN).round();
            assert!(en.set_station(1, 0, ih).is_ok());
            assert_english(&en, "セットのあと");
            let bs = en.snapshot().bs_target;
            aim_and_focus(&mut en, bs);
            assert!(en.observe().is_ok());
            assert_english(&en, "BSのあと");
            let fs = en.snapshot().fs_target;
            aim_and_focus(&mut en, fs);
            assert!(en.observe().is_ok());
            assert_english(&en, "FSのあと");
            assert_facts_fit(&en, "観測のあとの事実");
        }
    }
}

#[test]
fn english_refusals_failures_and_advice_have_no_japanese() {
    // すぐ「据付完了」を押した不合格の理由
    let mut sim = Simulator::new(Mode::Field, 5);
    sim.set_lang(Lang::En);
    sim.frame(0.0, &[]);
    sim.frame(50.0, &[Command::CompleteSetup]);
    let reasons = sim.ui_state().judgement.map(|j| j.reasons).unwrap_or_default();
    assert!(!reasons.is_empty());
    assert_english(&sim, "不合格の理由");

    // 教科書モードで手順を飛ばそうとした（断る理由）・手順外の操作（注意）
    let mut sim = Simulator::new(Mode::Textbook, 2);
    sim.set_lang(Lang::En);
    sim.frame(0.0, &[Command::SelectStep { step: 5 }]);
    assert!(sim.ui_state().notice.is_some());
    assert_english(&sim, "断る理由");
    sim.frame(50.0, &[Command::MountInstrument]);
    assert_english(&sim, "手順外の注意");

    // 操作中のひとこと（逆向きに回し続けた）
    let mut sim = level_sim();
    sim.set_lang(Lang::En);
    let mut t = 0.0;
    sim.frame(t, &[]);
    turn_c_away(&mut sim, &mut t, 80);
    assert!(sim.ui_state().advice.is_some(), "ひとことが出る");
    assert_english(&sim, "ひとこと");

    // 器械を載せる前に「据付完了」
    let mut sim = Simulator::new(Mode::Textbook, 2);
    sim.set_lang(Lang::En);
    sim.frame(0.0, &[Command::CompleteSetup]);
    assert_english(&sim, "器械なしの判定");
}
