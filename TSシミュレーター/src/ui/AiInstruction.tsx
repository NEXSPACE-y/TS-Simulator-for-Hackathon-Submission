// 現場の指示の聞き取り（練習）：AI先輩が口頭風に指示を出し、そのとおりに電子平板をセットできるかを見る。
// どの点を指示するか（正解）と、指示どおりかの判定はコア。AIは言い回しを作るだけ。
// 指示は、ブラウザの読み上げ（音声合成）でも聞ける。

import { useEffect } from 'react';
import { SenpaiAnswer, useSenpai } from './aiCommon';
import { useSim, useUi } from './hooks';
import { speak } from './speech';

export function AiInstruction() {
  const ui = useUi();
  const { bridge } = useSim();
  const { state, run } = useSenpai();
  const ins = ui.observation.instruction;
  const id = ins?.id ?? 0;

  // 新しい指示を頼んだら（id が変わったら）、AIに言い回しを作ってもらう
  useEffect(() => {
    if (id === 0) return;
    const facts = bridge.aiFacts('instruction');
    if (facts) void run('instruct', facts);
  }, [id, bridge, run]);

  // AIが使えないときは、コアの決まった言い方で練習できるようにする
  const text = state.status === 'done' ? state.text : state.status === 'error' ? (ins?.fallback ?? '') : '';
  useEffect(() => {
    if (text) speak(text);
  }, [text]);

  return (
    <div className="instruction">
      <div className="instruction-head">現場の指示の聞き取り（練習）</div>
      <p className="note">先輩の口頭の指示を聞いて、そのとおりに電子平板をセットします。</p>
      <div className="button-row">
        <button type="button" disabled={state.status === 'loading'} onClick={() => bridge.send('requestInstruction')}>
          {id === 0 ? '先輩の指示を聞く' : '別の言い方で聞く'}
        </button>
        {text && (
          <button type="button" onClick={() => speak(text)}>
            もう一度聞く（音声）
          </button>
        )}
      </div>
      <SenpaiAnswer state={state} />
      {state.status === 'error' && ins && (
        <p className="senpai-fallback">
          決まった言い方の指示：<strong>{ins.fallback}</strong>
        </p>
      )}
    </div>
  );
}

/** 指示どおりにセットできたか（セットのあと、観測の操作盤の上に出す） */
export function InstructionResultBox() {
  const ui = useUi();
  const r = ui.observation.instructionResult;
  if (!r) return null;
  return (
    <div className={`instruction-result ${r.ok ? 'is-ok' : 'is-ng'}`} role="status">
      <div className="instruction-head">{r.ok ? '指示どおりにセットできました' : '指示と違うところがあります'}</div>
      <ul>
        {r.items.map((s) => (
          <li key={s}>{s}</li>
        ))}
      </ul>
    </div>
  );
}
