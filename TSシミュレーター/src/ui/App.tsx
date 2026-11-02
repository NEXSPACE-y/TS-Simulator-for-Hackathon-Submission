// 画面全体の組み立て（上部・左・中央・右・結果画面）

import { useEffect, useMemo, useState } from 'react';
import { ModeId, type Bridge } from '../bridge';
import type { Quality } from '../render/engine';
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
        return { phase: ui.phase, mode: ui.mode, step: ui.currentStep, clampTight: ui.clampTight };
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
    </SimContext.Provider>
  );
}

function Simulator(props: { quality: Quality; onQuality: () => void; onRestart: () => void }) {
  const ui = useUi();
  return (
    <div className={`app phase-${ui.phase}`}>
      <TopBar quality={props.quality} onQuality={props.onQuality} onRestart={props.onRestart} />
      <StepList />
      <main className="center">
        <Viewport quality={props.quality} />
      </main>
      <OperationPanel />
      <ResultModal onRestart={props.onRestart} />
      <Toast />
    </div>
  );
}
