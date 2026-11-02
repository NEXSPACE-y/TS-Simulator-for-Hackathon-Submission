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
import { ObservationPanel } from './ObservationPanel';

export function OperationPanel() {
  const ui = useUi();
  const [open, setOpen] = useState(true);
  const step = ui.steps.find((s) => s.no === ui.currentStep);
  return (
    <aside className={`panel ${open ? '' : 'is-closed'}`}>
      <header className="panel-head">
        <div>
          <span className="panel-step">手順{ui.currentStep}</span>
          <span className="panel-title">{step?.label ?? ''}</span>
        </div>
        <button type="button" className="ghost" onClick={() => setOpen(!open)} aria-expanded={open}>
          {open ? '折りたたむ' : '開く'}
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
  const { bridge } = useSim();
  const step = ui.currentStep;
  const anyPlanted = ui.planted.some(Boolean);
  const allPlanted = ui.planted.every(Boolean);
  const moveLegs = (dx: number, dy: number) => ({ kind: 'moveLegs' as const, dx, dy });
  const slide = (dx: number, dy: number) => ({ kind: 'slide' as const, dx, dy });

  const tripodMove = (
    <Section title={anyPlanted ? '踏み込んでいない脚を動かす' : '三脚ごと動かす'}>
      <DirPad make={moveLegs} disabled={allPlanted} />
      <p className="note">
        {allPlanted
          ? '3本とも踏み込んだので、脚は動かせません。'
          : anyPlanted
            ? '踏み込んだ脚を支点に、残りの脚を持ち上げて動かします。'
            : 'レーザーの赤い点（右上の「求心」）を見ながら、測点の上へ運びます。'}
      </p>
    </Section>
  );
  const slideCtl = (
    <Section title="器械をずらす（三脚頭の上）">
      <ClampControl />
      <DirPad make={slide} disabled={ui.clampTight} />
      <p className="note">定心桿を緩めている間だけ、器械をずらせます。ずらし終えたら締めます。</p>
    </Section>
  );
  const screws = (
    <Section title="整準ネジ">
      <ScrewControls disabled={!ui.mounted} />
    </Section>
  );
  const legs = (
    <Section title="脚の伸縮">
      <LegLengthControls />
      <p className="note">円形気泡（右上）が中心の円に入るように、気泡がある側の脚を縮めるか、反対側を伸ばします。</p>
    </Section>
  );
  const press = (
    <Section title="脚を踏み込む">
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
        <Section title="器械を載せる">
          <button type="button" className="primary" disabled={ui.mounted} onClick={() => bridge.send('mountInstrument')}>
            {ui.mounted ? '器械は載っています' : '器械を載せて定心桿で固定する'}
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
          据付完了
        </button>
      </div>
    </>
  );
}

/** 教科書モード：この手順の目標 */
function GoalBox() {
  const ui = useUi();
  const { bridge } = useSim();
  const g = ui.goal;
  if (!g) return null;
  return (
    <div className={`goal ${g.satisfied ? 'is-ok' : ''}`}>
      <div className="goal-label">この手順の目標</div>
      <div className="goal-text">{g.text}</div>
      <div className="goal-actions">
        <span className="goal-state">{g.satisfied ? '✓ できています' : 'まだです'}</span>
        {!g.isLast && (
          <button type="button" className="primary" disabled={!g.canNext} onClick={() => bridge.send('nextStep')}>
            次の手順へ
          </button>
        )}
      </div>
    </div>
  );
}

/** 教科書モード手順1：脚頭の位置と傾き（器械を載せる前なので数値で見る） */
function HeadReadout() {
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
        脚頭の中心と測点のずれ <span ref={off} />
      </div>
      <div>
        脚頭の傾き <span ref={tilt} />
      </div>
    </div>
  );
}
