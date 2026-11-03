// 最初の画面：言語・モード・画質を選ぶ

import { useState } from 'react';
import type { Quality } from '../render/engine';
import { useT, type Lang } from './i18n';

export function StartScreen(props: {
  quality: Quality;
  onQuality: (q: Quality) => void;
  lang: Lang;
  onLang: (lang: Lang) => void;
  onStart: (mode: 'textbook' | 'field') => void;
}) {
  const t = useT();
  const [mode, setMode] = useState<'textbook' | 'field'>('field');
  return (
    <div className="start">
      <div className="start-card">
        <label className="lang-select">
          <span>{t.language}</span>
          <select value={props.lang} onChange={(e) => props.onLang(e.target.value === 'en' ? 'en' : 'ja')}>
            <option value="ja">日本語</option>
            <option value="en">English</option>
          </select>
        </label>
        <h1>{t.appName}</h1>
        <p className="lead">{t.lead}</p>
        <div className="mode-cards" role="radiogroup" aria-label={t.modeAria}>
          <button
            type="button"
            role="radio"
            aria-checked={mode === 'textbook'}
            className={`mode-card ${mode === 'textbook' ? 'is-on' : ''}`}
            onClick={() => setMode('textbook')}
          >
            <span className="mode-name">{t.textbookMode}</span>
            <span className="mode-desc">{t.textbookDesc}</span>
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={mode === 'field'}
            className={`mode-card ${mode === 'field' ? 'is-on' : ''}`}
            onClick={() => setMode('field')}
          >
            <span className="mode-name">{t.fieldMode}</span>
            <span className="mode-desc">{t.fieldDesc}</span>
          </button>
        </div>
        <div className="quality">
          <span>{t.quality}</span>
          {(['standard', 'light'] as const).map((q) => (
            <button key={q} type="button" className={props.quality === q ? 'is-on' : ''} onClick={() => props.onQuality(q)}>
              {q === 'standard' ? t.qualityStandard : t.qualityLight}
            </button>
          ))}
        </div>
        <button type="button" className="primary big" onClick={() => props.onStart(mode)}>
          {t.start}
        </button>
        <details className="keys">
          <summary>{t.keysTitle}</summary>
          <ul>{t.keys}</ul>
        </details>
        <p className="fine">{t.startFine}</p>
      </div>
    </div>
  );
}
