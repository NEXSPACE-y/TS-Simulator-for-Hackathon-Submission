// AI先輩による観測の解説：FSの座標がどう計算されたかを、実際の数字で順に説明してもらう。
// 途中の値（水平距離・高低差・ΔX・ΔYなど）はコアが計算したもの。AIは言葉にするだけ。

import { SenpaiAnswer, useSenpai } from './aiCommon';
import { useSim } from './hooks';
import { useT } from './i18n';

export function AiExplain() {
  const { bridge } = useSim();
  const t = useT();
  const { state, run } = useSenpai();
  const ask = () => {
    const facts = bridge.aiFacts('calc');
    if (facts) void run('explain', facts);
  };
  const canAsk = state.status === 'idle' || state.status === 'error';
  return (
    <div className="senpai-box">
      {canAsk && (
        <button type="button" className="senpai-ask" onClick={ask}>
          {state.status === 'error' ? t.askAgain : t.explain}
        </button>
      )}
      <SenpaiAnswer state={state} />
    </div>
  );
}
