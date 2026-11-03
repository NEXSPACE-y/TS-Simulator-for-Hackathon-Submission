//! AI先輩に渡す事実（操作の記録・講評）の確認。
//! AIには、ここで確かめた事実を言葉にさせるだけにする（正解はRust、言葉はAI）。

use ts_core::coach::Fact;
use ts_core::config::LEG_AZIMUTHS;
use ts_core::instrument::Instrument;
use ts_core::math::Vec3;
use ts_core::sim::FactKind;
use ts_core::tripod::Tripod;
use ts_core::{Command, Mode, Simulator, Speed, Thumb};

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

#[test]
fn time_is_counted_per_step() {
    let mut sim = level_sim();
    let mut t = 0.0;
    sim.frame(t, &[Command::SelectStep { step: 5 }]);
    turn_c_away(&mut sim, &mut t, 20);
    sim.frame(t, &[Command::SelectStep { step: 7 }]);
    turn_c_away(&mut sim, &mut t, 10);
    let log = sim.coach_log();
    assert!(log.step_ms[4] > 900.0 && log.step_ms[4] < 1_100.0, "手順5：{}", log.step_ms[4]);
    assert!(log.step_ms[6] > 400.0 && log.step_ms[6] < 600.0, "手順7：{}", log.step_ms[6]);
    let steps: Vec<u8> = log.visits.iter().map(|v| v.0).collect();
    assert_eq!(steps, vec![5, 7]);
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
