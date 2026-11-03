// 右の操作盤。ステップごとに中身が変わる（SPEC 4章）。

import { useCallback, useRef, useState } from 'react';
import type { Frame } from '../bridge';
import { fixed } from '../bridge/format';
import {
  ClampControl,
  DirPad,
  LegLengthControls,
  PressLegButtons,
  ScrewControls,
  Section,
  SpeedSelector,
} from './controls';
import { setText, useFrame, useSim, useUi } from './hooks';
import { useT } from './i18n';
import { ObservationPanel } from './ObservationPanel';

export function OperationPanel() {
  const ui = useUi();
  const t = useT();
  const [open, setOpen] = useState(true);
  const step = ui.steps.find((s) => s.no === ui.currentStep);
  return (
    <aside className={`panel ${open ? '' : 'is-closed'}`}>
      <header className="panel-head">
        <div>
          <span className="panel-step">{t.stepN(ui.currentStep)}</span>
          <span className="panel-title">{step?.label ?? ''}</span>
        </div>
        <button type="button" className="ghost" onClick={() => setOpen(!open)} aria-expanded={open}>
          {open ? t.collapse : t.expand}
        </button>
      </header>
      {open && (
        <div className="panel-body">
          {ui.phase === 'observation' ? <ObservationPanel /> : <SetupControls />}
        </div>
      )}
    </aside>
  );
}

function SetupControls() {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const step = ui.currentStep;
  const anyPlanted = ui.planted.some(Boolean);
  const allPlanted = ui.planted.every(Boolean);
  const moveLegs = (dx: number, dy: number) => ({ kind: 'moveLegs' as const, dx, dy });
  const slide = (dx: number, dy: number) => ({ kind: 'slide' as const, dx, dy });

  const tripodMove = (
    <Section title={anyPlanted ? t.moveFreeLegs : t.moveTripod}>
      <DirPad make={moveLegs} disabled={allPlanted} />
      <p className="note">
        {allPlanted ? t.noteAllPlanted : anyPlanted ? t.notePivot : ui.mounted ? t.noteLaser : t.noteHead}
      </p>
    </Section>
  );
  const slideCtl = (
    <Section title={t.slideTitle}>
      <ClampControl />
      <DirPad make={slide} disabled={ui.clampTight} />
      <p className="note">{t.slideNote}</p>
    </Section>
  );
  const screws = (
    <Section title={t.screwsTitle}>
      <ScrewControls disabled={!ui.mounted} />
    </Section>
  );
  const legs = (
    <Section title={t.legsTitle}>
      <LegLengthControls />
      <p className="note">{t.legsNote}</p>
    </Section>
  );
  const press = (
    <Section title={t.pressTitle}>
      <PressLegButtons />
    </Section>
  );

  let body: React.ReactNode;
  if (ui.mode === 'field') {
    body = {
      1: tripodMove,
      2: (
        <>
          {press}
          {tripodMove}
        </>
      ),
      3: press,
      4: legs,
      5: screws,
      6: slideCtl,
      7: screws,
    }[step];
  } else {
    body = {
      1: (
        <>
          <HeadReadout />
          {tripodMove}
          {legs}
          {press}
        </>
      ),
      2: (
        <Section title={t.mountTitle}>
          <button type="button" className="primary" disabled={ui.mounted} onClick={() => bridge.send('mountInstrument')}>
            {ui.mounted ? t.mounted : t.mountButton}
          </button>
        </Section>
      ),
      3: screws,
      4: legs,
      5: screws,
      6: slideCtl,
      7: (
        <>
          {screws}
          {slideCtl}
        </>
      ),
    }[step];
  }

  return (
    <>
      {ui.goal && <GoalBox />}
      <SpeedSelector />
      {body}
      <div className="panel-foot">
        <button type="button" className="complete" onClick={() => bridge.send('completeSetup')}>
          {t.finishSetup}
        </button>
      </div>
    </>
  );
}

/** 教科書モード：この手順の目標 */
function GoalBox() {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const g = ui.goal;
  if (!g) return null;
  return (
    <div className={`goal ${g.satisfied ? 'is-ok' : ''}`}>
      <div className="goal-label">{t.goalLabel}</div>
      <div className="goal-text">{g.text}</div>
      <div className="goal-actions">
        <span className="goal-state">{g.satisfied ? t.goalDone : t.goalNotYet}</span>
        {!g.isLast && (
          <button type="button" className="primary" disabled={!g.canNext} onClick={() => bridge.send('nextStep')}>
            {t.nextStep}
          </button>
        )}
      </div>
    </div>
  );
}

/** 教科書モード手順1：脚頭の位置と傾き（器械を載せる前なので数値で見る） */
function HeadReadout() {
  const t = useT();
  const off = useRef<HTMLSpanElement>(null);
  const tilt = useRef<HTMLSpanElement>(null);
  useFrame(
    useCallback((f: Frame) => {
      setText(off.current, `${fixed(f.data[f.o.headOffsetMm], 0)} mm`);
      setText(tilt.current, `${fixed(f.data[f.o.headTiltDeg], 1)}°`);
    }, []),
  );
  return (
    <div className="readout">
      <div>
        {t.headOffset} <span ref={off} />
      </div>
      <div>
        {t.headTilt} <span ref={tilt} />
      </div>
    </div>
  );
}
