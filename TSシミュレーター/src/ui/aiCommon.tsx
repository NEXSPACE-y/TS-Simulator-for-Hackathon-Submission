// AI先輩の部品（呼び出しの状態と、答えの表示）。AIはボタンを押したときなどにだけ呼ぶ（毎フレームは呼ばない）。

import { useCallback, useEffect, useRef, useState } from 'react';
import type { AiFact } from '../bridge';
import { askSenpai, type AiKind } from '../ai/client';
import { useUi } from './hooks';
import { useT } from './i18n';

export interface SenpaiState {
  status: 'idle' | 'loading' | 'done' | 'error';
  text: string;
}

const IDLE: SenpaiState = { status: 'idle', text: '' };

/** AI先輩を1回呼んで、答えを持っておく */
export function useSenpai() {
  const lang = useUi().lang;
  const [state, setState] = useState<SenpaiState>(IDLE);
  const ctl = useRef<AbortController | null>(null);
  useEffect(() => () => ctl.current?.abort(), []);

  const run = useCallback(async (kind: AiKind, facts: readonly AiFact[], question?: string) => {
    ctl.current?.abort();
    const c = new AbortController();
    ctl.current = c;
    setState({ status: 'loading', text: '' });
    const r = await askSenpai(kind, facts, question, c.signal, lang);
    if (c.signal.aborted) return;
    setState(r.ok ? { status: 'done', text: r.text } : { status: 'error', text: r.message });
  }, [lang]);

  const reset = useCallback(() => {
    ctl.current?.abort();
    setState(IDLE);
  }, []);

  return { state, run, reset };
}

/** AI先輩の答え（考え中・答え・使えないときの知らせ） */
export function SenpaiAnswer(props: { state: SenpaiState }) {
  const { status, text } = props.state;
  const t = useT();
  if (status === 'idle') return null;
  return (
    <div className={`senpai-answer is-${status}`} role="status" aria-live="polite">
      <div className="senpai-label">{t.senpai}</div>
      {status === 'loading' ? (
        <p className="senpai-text">{t.thinking}</p>
      ) : (
        <p className="senpai-text">{text}</p>
      )}
      {status === 'done' && <p className="senpai-note">{t.aiNote}</p>}
    </div>
  );
}
