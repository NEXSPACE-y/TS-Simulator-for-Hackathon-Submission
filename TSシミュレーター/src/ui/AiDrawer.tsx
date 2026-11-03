// AI先輩に何でも質問（上部の「AI先輩」ボタンで出し入れ）。
// 質問と一緒に、コアがまとめた「いまの状態と次の操作」を渡す。答えはAIが言葉にするだけ。

import { useState } from 'react';
import { SenpaiAnswer, useSenpai } from './aiCommon';
import { useSim } from './hooks';
import { useT } from './i18n';

const MAX_QUESTION = 200;

export function AiDrawer(props: { onClose: () => void; adviceOn: boolean; onAdvice: (on: boolean) => void }) {
  const { bridge } = useSim();
  const t = useT();
  const { state, run } = useSenpai();
  const [question, setQuestion] = useState('');
  const busy = state.status === 'loading';

  const send = (text: string) => {
    const q = text.trim().slice(0, MAX_QUESTION);
    if (q === '' || busy) return;
    void run('ask', bridge.aiFacts('status') ?? [], q);
  };

  return (
    <aside className="ai-drawer" aria-label={t.askTitle}>
      <header className="ai-drawer-head">
        <span className="ai-drawer-title">{t.askTitle}</span>
        <button type="button" className="ghost" onClick={props.onClose}>
          {t.close}
        </button>
      </header>
      <p className="note">{t.askNote}</p>
      <div className="ai-examples">
        {t.askExamples.map((e) => (
          <button
            key={e}
            type="button"
            disabled={busy}
            onClick={() => {
              setQuestion(e);
              send(e);
            }}
          >
            {e}
          </button>
        ))}
      </div>
      <form
        className="ai-ask-form"
        onSubmit={(e) => {
          e.preventDefault();
          send(question);
        }}
      >
        <input
          type="text"
          value={question}
          maxLength={MAX_QUESTION}
          placeholder={t.askPlaceholder}
          aria-label={t.askAria}
          onChange={(e) => setQuestion(e.target.value)}
        />
        <button type="submit" className="primary" disabled={busy || question.trim() === ''}>
          {t.ask}
        </button>
      </form>
      <SenpaiAnswer state={state} />
      <label className="ai-switch">
        <input type="checkbox" checked={props.adviceOn} onChange={(e) => props.onAdvice(e.target.checked)} />
        <span>{t.adviceSwitch}</span>
      </label>
    </aside>
  );
}
