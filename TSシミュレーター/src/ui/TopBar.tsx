// 上部：パラメーター（求心ズレ / 電子気泡X・Y / タイム / ループ回数）（SPEC 4章）

import { useCallback, useRef } from 'react';
import type { Frame } from '../bridge';
import { dmsSignedAt, fixed, flag, timeAt } from '../bridge/format';
import { setText, useFrame, useUi } from './hooks';

export function TopBar(props: { onRestart: () => void; quality: 'light' | 'standard'; onQuality: () => void }) {
  const ui = useUi();
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
        <span className="brand-name">TS実習シミュレーター</span>
        <span className="brand-mode">{ui.mode === 'textbook' ? '教科書モード' : '現場モード'}</span>
      </div>
      <dl className="params">
        <div>
          <dt>求心ズレ</dt>
          <dd ref={cent} />
        </div>
        <div>
          <dt>電子気泡 X</dt>
          <dd ref={tx} />
        </div>
        <div>
          <dt>電子気泡 Y</dt>
          <dd ref={ty} />
        </div>
        <div>
          <dt>タイム</dt>
          <dd ref={time} />
        </div>
        <div>
          <dt>ループ回数</dt>
          <dd ref={loops} />
        </div>
      </dl>
      <div className="top-actions">
        <button type="button" className="ghost" onClick={props.onQuality} title="画質の切り替え">
          画質：{props.quality === 'standard' ? '標準' : '軽量'}
        </button>
        <button type="button" className="ghost" onClick={props.onRestart}>
          最初から
        </button>
      </div>
    </header>
  );
}
