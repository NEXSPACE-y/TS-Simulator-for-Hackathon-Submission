// 画面全体の組み立て（上部・左・中央・右・結果画面）

import { useEffect, useMemo, useState } from 'react';
import { ModeId, type Bridge } from '../bridge';
import type { Quality } from '../render/engine';
import { AdviceBubble } from './AdviceBubble';
import { AiDrawer } from './AiDrawer';
import { DebugLog, isDebugMode } from './DebugLog';
import { SimContext, useUi } from './hooks';
import { InputManager } from './input';
import { OperationPanel } from './OperationPanel';
import { ResultModal } from './ResultModal';
import { StartScreen } from './StartScreen';
import { StepList } from './StepList';
import { Toast } from './Toast';
import { TopBar } from './TopBar';
import { Viewport } from './Viewport';

const QUALITY_KEY = 'ts-sim-quality';
const ADVICE_KEY = 'ts-sim-advice';

/** 操作中のひとこと（AI先輩）を出すか。最初は出す */
function loadAdvice(): boolean {
  try {
    return window.localStorage.getItem(ADVICE_KEY) !== 'off';
  } catch {
    return true;
  }
}

function saveAdvice(on: boolean): void {
  try {
    window.localStorage.setItem(ADVICE_KEY, on ? 'on' : 'off');
  } catch {
    // 保存できなくても動作には関係ない
  }
}

function loadQuality(): Quality {
  try {
    const v = window.localStorage.getItem(QUALITY_KEY);
    return v === 'light' ? 'light' : 'standard';
  } catch {
    return 'standard';
  }
}

function saveQuality(q: Quality): void {
  try {
    window.localStorage.setItem(QUALITY_KEY, q);
  } catch {
    // 保存できなくても動作には関係ない
  }
}

function newSeed(): number {
  return (Math.random() * 0xffffffff) >>> 0;
}

export function App(props: { bridge: Bridge }) {
  const { bridge } = props;
  const [started, setStarted] = useState(false);
  const [quality, setQuality] = useState<Quality>(loadQuality);
  const input = useMemo(
    () =>
      new InputManager(bridge, () => {
        const ui = bridge.getUi();
        return {
          phase: ui.phase,
          mode: ui.mode,
          step: ui.currentStep,
          clampTight: ui.clampTight,
          peepSight: ui.observation.peepSight,
        };
      }),
    [bridge],
  );
  useEffect(() => {
    input.attach(window);
    return () => input.dispose();
  }, [input]);

  const changeQuality = (q: Quality) => {
    setQuality(q);
    saveQuality(q);
  };

  const ctx = useMemo(() => ({ bridge, input }), [bridge, input]);
  // URL に ?debug を付けたときだけ、タッチの記録を画面の隅に出す
  const debug = useMemo(isDebugMode, []);

  return (
    <SimContext.Provider value={ctx}>
      {started ? (
        <Simulator
          quality={quality}
          onQuality={() => changeQuality(quality === 'standard' ? 'light' : 'standard')}
          onRestart={() => {
            input.releaseAll();
            setStarted(false);
          }}
        />
      ) : (
        <StartScreen
          quality={quality}
          onQuality={changeQuality}
          onStart={(mode) => {
            bridge.restart(mode === 'textbook' ? ModeId.Textbook : ModeId.Field, newSeed());
            setStarted(true);
          }}
        />
      )}
      {debug && <DebugLog />}
    </SimContext.Provider>
  );
}

function Simulator(props: { quality: Quality; onQuality: () => void; onRestart: () => void }) {
  const ui = useUi();
  const [aiOpen, setAiOpen] = useState(false);
  const [adviceOn, setAdviceOn] = useState(loadAdvice);
  return (
    <div className={`app phase-${ui.phase}`}>
      <TopBar
        quality={props.quality}
        onQuality={props.onQuality}
        onRestart={props.onRestart}
        aiOpen={aiOpen}
        onAi={() => setAiOpen(!aiOpen)}
      />
      <StepList />
      <main className="center">
        <Viewport quality={props.quality} />
        <AdviceBubble enabled={adviceOn} />
      </main>
      <OperationPanel />
      <ResultModal onRestart={props.onRestart} />
      {aiOpen && (
        <AiDrawer
          onClose={() => setAiOpen(false)}
          adviceOn={adviceOn}
          onAdvice={(on) => {
            setAdviceOn(on);
            saveAdvice(on);
          }}
        />
      )}
      <Toast />
    </div>
  );
}
