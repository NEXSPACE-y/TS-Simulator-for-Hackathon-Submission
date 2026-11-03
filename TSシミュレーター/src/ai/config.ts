// AI先輩の中継（Vercel Functions）の場所。ビルドのときに環境変数 VITE_AI_RELAY_URL で変えられる。
// 中継の作り方は README の「AI先輩の中継」を参照。

export const AI_RELAY_URL: string =
  (import.meta.env.VITE_AI_RELAY_URL as string | undefined) ?? 'https://ts-simulator-ai.vercel.app/api/ai';

/** 返事を待つ時間の上限 [ms] */
export const AI_TIMEOUT_MS = 30_000;
