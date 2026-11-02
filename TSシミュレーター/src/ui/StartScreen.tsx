// 最初の画面：モードと画質を選ぶ

import { useState } from 'react';
import type { Quality } from '../render/engine';

export function StartScreen(props: {
  quality: Quality;
  onQuality: (q: Quality) => void;
  onStart: (mode: 'textbook' | 'field') => void;
}) {
  const [mode, setMode] = useState<'textbook' | 'field'>('field');
  return (
    <div className="start">
      <div className="start-card">
        <h1>TS実習シミュレーター</h1>
        <p className="lead">
          トータルステーションの<strong>据付</strong>から<strong>観測（BS・FS）</strong>までを、ブラウザの3Dで練習できます。
          求心と整準の「片方を直すと、もう片方がズレる」ループを体験してください。
        </p>
        <div className="mode-cards" role="radiogroup" aria-label="モード">
          <button
            type="button"
            role="radio"
            aria-checked={mode === 'textbook'}
            className={`mode-card ${mode === 'textbook' ? 'is-on' : ''}`}
            onClick={() => setMode('textbook')}
          >
            <span className="mode-name">教科書モード</span>
            <span className="mode-desc">教科書の一般的な手順を一歩ずつガイドします。手順外の操作をすると注意が出ます。</span>
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={mode === 'field'}
            className={`mode-card ${mode === 'field' ? 'is-on' : ''}`}
            onClick={() => setMode('field')}
          >
            <span className="mode-name">現場モード</span>
            <span className="mode-desc">現場の手順。順番は自由で、最後に求心と電子気泡が許容内なら合格です。</span>
          </button>
        </div>
        <div className="quality">
          <span>画質</span>
          {(['standard', 'light'] as const).map((q) => (
            <button key={q} type="button" className={props.quality === q ? 'is-on' : ''} onClick={() => props.onQuality(q)}>
              {q === 'standard' ? '標準' : '軽量'}
            </button>
          ))}
        </div>
        <button type="button" className="primary big" onClick={() => props.onStart(mode)}>
          はじめる
        </button>
        <details className="keys">
          <summary>キーボード操作</summary>
          <ul>
            <li>整準ネジ A：<kbd>A</kbd>（親指を左）／<kbd>S</kbd>（親指を右）</li>
            <li>整準ネジ B：<kbd>K</kbd>／<kbd>L</kbd>　整準ネジ C：<kbd>Z</kbd>／<kbd>X</kbd></li>
            <li>A・B を内側へ：<kbd>S</kbd>＋<kbd>K</kbd>　外側へ：<kbd>A</kbd>＋<kbd>L</kbd></li>
            <li>脚を伸ばす：<kbd>1</kbd><kbd>2</kbd><kbd>3</kbd>　縮める：<kbd>Q</kbd><kbd>W</kbd><kbd>E</kbd></li>
            <li>三脚・器械・望遠鏡を動かす：矢印キー　定心桿：<kbd>T</kbd>　細かく：<kbd>Shift</kbd></li>
            <li>
              観測：照準器と望遠鏡の切り替え <kbd>P</kbd>　接眼レンズ <kbd>V</kbd>／<kbd>B</kbd>　合焦 <kbd>F</kbd>（近く）／<kbd>G</kbd>（遠く）　測量 <kbd>Enter</kbd>
            </li>
          </ul>
        </details>
        <p className="fine">タイムは最初の操作で始まり、「据付完了」を押すと止まります。目安：3分で合格、2分で熟練。</p>
      </div>
    </div>
  );
}
