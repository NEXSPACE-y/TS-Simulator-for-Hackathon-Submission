// 注意・お知らせの表示（手順外の操作、操作できない理由など）

import { useEffect, useState } from 'react';
import type { NoticeView } from '../bridge';
import { useUi } from './hooks';

const SHOW_MS = 3500;

export function Toast() {
  const ui = useUi();
  const [shown, setShown] = useState<NoticeView | null>(null);
  const id = ui.notice?.id ?? 0;
  useEffect(() => {
    if (!ui.notice) return;
    setShown(ui.notice);
    const t = window.setTimeout(() => setShown(null), SHOW_MS);
    return () => window.clearTimeout(t);
    // id が変わったときだけ出し直す
  }, [id]);
  if (!shown) return null;
  return (
    <div className={`toast is-${shown.kind}`} role="status" aria-live="polite">
      {shown.text}
    </div>
  );
}
