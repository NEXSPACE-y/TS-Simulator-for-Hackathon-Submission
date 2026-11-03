// 左の手順一覧：現在地・完了済み・未着手が一目で分かる表示（SPEC 4章）

import { useSim, useUi } from './hooks';
import { useT } from './i18n';

export function StepList() {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const selectable = (no: number, locked: boolean) =>
    !locked && ((ui.phase === 'setup' && ui.mode === 'field' && no <= 7) || (ui.phase === 'observation' && no >= 8));
  return (
    <nav className="steps" aria-label={t.stepsAria}>
      {[
        { title: t.part1, items: ui.steps.filter((s) => s.no <= 7) },
        { title: t.part2, items: ui.steps.filter((s) => s.no >= 8) },
      ].map((g) => (
        <div className="step-group" key={g.title}>
          <div className="step-group-title">{g.title}</div>
          <ol>
            {g.items.map((s) => {
              const state = s.current ? 'current' : s.done ? 'done' : s.locked ? 'locked' : 'todo';
              const can = selectable(s.no, s.locked);
              return (
                <li key={s.no} className={`step is-${state}`}>
                  <button
                    type="button"
                    disabled={!can}
                    aria-current={s.current ? 'step' : undefined}
                    onClick={() => bridge.send('selectStep', s.no)}
                    title={s.label}
                  >
                    <span className="step-mark">{s.done ? '✓' : s.no}</span>
                    <span className="step-text">
                      <span className="step-label">{s.label}</span>
                      <span className="step-short">{s.short}</span>
                    </span>
                  </button>
                </li>
              );
            })}
          </ol>
        </div>
      ))}
      {ui.mode === 'field' && ui.phase === 'setup' && (
        <p className="steps-note">{t.stepsNoteField}</p>
      )}
      {ui.mode === 'textbook' && ui.phase === 'setup' && (
        <p className="steps-note">{t.stepsNoteTextbook}</p>
      )}
    </nav>
  );
}
