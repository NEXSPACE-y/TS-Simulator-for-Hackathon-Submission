// 操作中のひとこと（AI先輩）。きっかけ（目安の時間を過ぎた・逆向きに回している など）と中身はコアが決め、
// AIは言い方を整えるだけ。AIが使えないときは、コアの決まった言い方をそのまま出す。
// 3Dビューの左下（視点切替の上）に出し、しばらくすると消える（閉じるボタンでも消せる）。

import { useEffect, useRef, useState } from 'react';
import { useSenpai } from './aiCommon';
import { useSim, useUi } from './hooks';
import { useT } from './i18n';

/** ひとことを出しておく時間 [ms] */
const SHOW_MS = 15_000;

export function AdviceBubble(props: { enabled: boolean }) {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const { state, run, reset } = useSenpai();
  const [shownId, setShownId] = useState(0);
  const id = ui.advice?.id ?? 0;
  const active = props.enabled && ui.phase === 'setup';
  // すでに扱ったきっかけ（出し直したり、切っていた間のものを後から出したりしない）
  const handled = useRef(id);

  // 新しいきっかけが出たら（id が変わったら）、AIに言い方を整えてもらう
  useEffect(() => {
    if (!active) {
      handled.current = id;
      return;
    }
    if (id === 0 || id === handled.current) return;
    handled.current = id;
    const facts = bridge.aiFacts('advice');
    if (!facts) return;
    setShownId(id);
    void run('advice', facts);
  }, [id, active, bridge, run]);

  const text =
    state.status === 'done' ? state.text : state.status === 'error' ? (ui.advice?.fallback ?? '') : '';

  // しばらくしたら消す
  useEffect(() => {
    if (!text) return;
    const t = window.setTimeout(() => setShownId(0), SHOW_MS);
    return () => window.clearTimeout(t);
  }, [text, shownId]);

  if (!active || shownId === 0 || shownId !== id || !text) return null;
  return (
    <div className="advice-bubble" role="status" aria-live="polite">
      <div className="advice-head">
        <span className="senpai-label">{t.adviceLabel}</span>
        <button
          type="button"
          className="ghost"
          onClick={() => {
            setShownId(0);
            reset();
          }}
        >
          {t.close}
        </button>
      </div>
      <p className="senpai-text">{text}</p>
    </div>
  );
}
