// 上部：パラメーター（求心ズレ / 電子気泡X・Y / タイム / ループ回数）（SPEC 4章）

import { useCallback, useRef } from 'react';
import type { Frame } from '../bridge';
import { dmsSignedAt, fixed, flag, timeAt } from '../bridge/format';
import { setText, useFrame, useUi } from './hooks';
import { useT } from './i18n';

export function TopBar(props: {
  onRestart: () => void;
  quality: 'light' | 'standard';
  onQuality: () => void;
  aiOpen: boolean;
  onAi: () => void;
}) {
  const ui = useUi();
  const t = useT();
  const cent = useRef<HTMLSpanElement>(null);
  const tx = useRef<HTMLSpanElement>(null);
  const ty = useRef<HTMLSpanElement>(null);
  const time = useRef<HTMLSpanElement>(null);
  const loops = useRef<HTMLSpanElement>(null);
  useFrame(
    useCallback((f: Frame) => {
      const valid = flag(f, f.o.tiltValid);
      setText(cent.current, flag(f, f.o.laserValid) ? `${fixed(f.data[f.o.centeringDisplayMm], 1)} mm` : '---');
      setText(tx.current, valid ? dmsSignedAt(f, f.o.tiltXDms) : '---');
      setText(ty.current, valid ? dmsSignedAt(f, f.o.tiltYDms) : '---');
      setText(time.current, timeAt(f));
      setText(loops.current, `${f.data[f.o.loops] ?? 0}`);
    }, []),
  );
  return (
    <header className="topbar">
      <div className="brand">
        <span className="brand-name">{t.appName}</span>
        <span className="brand-mode">{ui.mode === 'textbook' ? t.textbookMode : t.fieldMode}</span>
      </div>
      <dl className="params">
        <div>
          <dt>{t.paramCentering}</dt>
          <dd ref={cent} />
        </div>
        <div>
          <dt>{t.paramTiltX}</dt>
          <dd ref={tx} />
        </div>
        <div>
          <dt>{t.paramTiltY}</dt>
          <dd ref={ty} />
        </div>
        <div>
          <dt>{t.paramTime}</dt>
          <dd ref={time} />
        </div>
        <div>
          <dt>{t.paramLoops}</dt>
          <dd ref={loops} />
        </div>
      </dl>
      <div className="top-actions">
        <button
          type="button"
          className={`ghost ai-toggle ${props.aiOpen ? 'is-on' : ''}`}
          aria-expanded={props.aiOpen}
          onClick={props.onAi}
          title={t.senpaiTitle}
        >
          {t.senpai}
        </button>
        <button type="button" className="ghost" onClick={props.onQuality} title={t.qualityTitle}>
          {t.qualityButton(props.quality === 'light')}
        </button>
        <button type="button" className="ghost" onClick={props.onRestart}>
          {t.restart}
        </button>
      </div>
    </header>
  );
}
