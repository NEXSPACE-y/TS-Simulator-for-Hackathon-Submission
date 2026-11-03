// AI先輩に何でも質問（上部の「AI先輩」ボタンで出し入れ）。
// 質問と一緒に、コアがまとめた「いまの状態と次の操作」を渡す。答えはAIが言葉にするだけ。

import { useState } from 'react';
import { SenpaiAnswer, useSenpai } from './aiCommon';
import { useSim } from './hooks';

const EXAMPLES = ['次は何をすればいい？', '求心ってなに？', 'なんで6と7を繰り返すの？'] as const;
const MAX_QUESTION = 200;

export function AiDrawer(props: { onClose: () => void; adviceOn: boolean; onAdvice: (on: boolean) => void }) {
  const { bridge } = useSim();
  const { state, run } = useSenpai();
  const [question, setQuestion] = useState('');
  const busy = state.status === 'loading';

  const send = (text: string) => {
    const q = text.trim().slice(0, MAX_QUESTION);
    if (q === '' || busy) return;
    void run('ask', bridge.aiFacts('status') ?? [], q);
  };

  return (
    <aside className="ai-drawer" aria-label="AI先輩に聞く">
      <header className="ai-drawer-head">
        <span className="ai-drawer-title">AI先輩に聞く</span>
        <button type="button" className="ghost" onClick={props.onClose}>
          閉じる
        </button>
      </header>
      <p className="note">いまの状態を踏まえて答えます（質問は200字まで）。</p>
      <div className="ai-examples">
        {EXAMPLES.map((e) => (
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
          placeholder="例：円形気泡が入らないときは？"
          aria-label="AI先輩への質問"
          onChange={(e) => setQuestion(e.target.value)}
        />
        <button type="submit" className="primary" disabled={busy || question.trim() === ''}>
          聞く
        </button>
      </form>
      <SenpaiAnswer state={state} />
      <label className="ai-switch">
        <input type="checkbox" checked={props.adviceOn} onChange={(e) => props.onAdvice(e.target.checked)} />
        <span>操作中のひとこと（据付の途中で、つまずいたときにAI先輩が声をかける）</span>
      </label>
    </aside>
  );
}
