// 「据付完了」を押したあとの結果画面（SPEC 5.4）。AI先輩の講評も、ここで聞ける。

import { SenpaiAnswer, useSenpai } from './aiCommon';
import { useSim, useUi } from './hooks';
import { useT } from './i18n';

export function ResultModal(props: { onRestart: () => void }) {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const j = ui.judgement;
  if (ui.phase !== 'setupResult' || !j) return null;
  return (
    <div className="modal-back" role="dialog" aria-modal="true" aria-labelledby="result-title">
      <div className={`modal ${j.passed ? 'is-pass' : 'is-fail'}`}>
        <h2 id="result-title">{j.passed ? t.passTitle : t.failTitle}</h2>
        {j.ratingText && <p className="rating">{j.ratingText}</p>}
        <dl className="result-grid">
          <dt>{t.paramTime}</dt>
          <dd>{j.timeText}</dd>
          <dt>{t.paramLoops}</dt>
          <dd>{t.loopsN(j.loops)}</dd>
          <dt>{t.centering}</dt>
          <dd>{j.centeringText}</dd>
          <dt>{t.tiltX}</dt>
          <dd>{j.tiltXText}</dd>
          <dt>{t.tiltY}</dt>
          <dd>{j.tiltYText}</dd>
        </dl>
        {!j.passed && (
          <ul className="reasons">
            {j.reasons.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
        )}
        <p className="note">{t.resultTarget}</p>
        <AiReview />
        <div className="modal-actions">
          {j.passed ? (
            <>
              <button type="button" className="primary" onClick={() => bridge.send('startObservation')}>
                {t.goObservation}
              </button>
              <button type="button" onClick={props.onRestart}>
                {t.restart}
              </button>
            </>
          ) : (
            <button type="button" className="primary" onClick={() => bridge.send('continueSetup')}>
              {t.continueSetup}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/** AI先輩の講評：コアがまとめた操作の記録（遠回りした箇所など）を、AIに言葉にしてもらう */
function AiReview() {
  const { bridge } = useSim();
  const t = useT();
  const { state, run } = useSenpai();
  const ask = () => {
    const facts = bridge.aiFacts('review');
    if (facts) void run('review', facts);
  };
  const canAsk = state.status === 'idle' || state.status === 'error';
  return (
    <section className="senpai-box">
      {canAsk && (
        <button type="button" className="senpai-ask" onClick={ask}>
          {state.status === 'error' ? t.askAgain : t.askReview}
        </button>
      )}
      <SenpaiAnswer state={state} />
    </section>
  );
}
