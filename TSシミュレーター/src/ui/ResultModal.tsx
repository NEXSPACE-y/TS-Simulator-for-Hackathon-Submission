// 「据付完了」を押したあとの結果画面（SPEC 5.4）。AI先輩の講評も、ここで聞ける。

import { SenpaiAnswer, useSenpai } from './aiCommon';
import { useSim, useUi } from './hooks';

export function ResultModal(props: { onRestart: () => void }) {
  const ui = useUi();
  const { bridge } = useSim();
  const j = ui.judgement;
  if (ui.phase !== 'setupResult' || !j) return null;
  return (
    <div className="modal-back" role="dialog" aria-modal="true" aria-labelledby="result-title">
      <div className={`modal ${j.passed ? 'is-pass' : 'is-fail'}`}>
        <h2 id="result-title">{j.passed ? '据付 合格' : '据付 不合格'}</h2>
        {j.ratingText && <p className="rating">{j.ratingText}</p>}
        <dl className="result-grid">
          <dt>タイム</dt>
          <dd>{j.timeText}</dd>
          <dt>ループ回数</dt>
          <dd>{j.loops} 回</dd>
          <dt>求心ズレ</dt>
          <dd>{j.centeringText}</dd>
          <dt>電子気泡 X</dt>
          <dd>{j.tiltXText}</dd>
          <dt>電子気泡 Y</dt>
          <dd>{j.tiltYText}</dd>
        </dl>
        {!j.passed && (
          <ul className="reasons">
            {j.reasons.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
        )}
        <p className="note">目安：3分で合格、2分で熟練。</p>
        <AiReview />
        <div className="modal-actions">
          {j.passed ? (
            <>
              <button type="button" className="primary" onClick={() => bridge.send('startObservation')}>
                第2部：観測へ進む
              </button>
              <button type="button" onClick={props.onRestart}>
                最初から
              </button>
            </>
          ) : (
            <button type="button" className="primary" onClick={() => bridge.send('continueSetup')}>
              続行する
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
          {state.status === 'error' ? 'もう一度聞く' : 'AI先輩の講評を聞く'}
        </button>
      )}
      <SenpaiAnswer state={state} />
    </section>
  );
}
