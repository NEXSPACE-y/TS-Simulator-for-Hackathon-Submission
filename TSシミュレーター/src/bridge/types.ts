// WASMから受け取るJSONの型（Rust側 crates/wasm の ui_json() などと同じ形）

export type CommandName =
  | 'moveFreeLegs'
  | 'pressLeg'
  | 'changeLegLength'
  | 'turnScrew'
  | 'setClamp'
  | 'slideInstrument'
  | 'mountInstrument'
  | 'selectStep'
  | 'nextStep'
  | 'completeSetup'
  | 'continueSetup'
  | 'startObservation'
  | 'rotateTelescope'
  | 'setStation'
  | 'observe'
  | 'turnFocus'
  | 'turnEyepiece'
  | 'setPeepSight'
  | 'requestInstruction';

export interface StepView {
  no: number;
  label: string;
  short: string;
  done: boolean;
  current: boolean;
  locked: boolean;
}

export interface GoalView {
  text: string;
  satisfied: boolean;
  canNext: boolean;
  isLast: boolean;
}

export interface NoticeView {
  id: number;
  kind: 'warning' | 'info' | 'success';
  text: string;
}

export interface JudgementView {
  passed: boolean;
  reasons: string[];
  timeMs: number;
  timeText: string;
  loops: number;
  centeringText: string;
  tiltXText: string;
  tiltYText: string;
  ratingText: string | null;
}

export interface PointView {
  name: string;
  x: string;
  y: string;
  h: string;
}

export interface BacksightView {
  azimuthText: string;
  horizontalText: string;
  coordDistanceText: string;
  distanceDiffText: string;
}

export interface ForesightView {
  azimuthText: string;
  zenithText: string;
  horizontalText: string;
  xText: string;
  yText: string;
  hText: string;
  dxText: string;
  dyText: string;
  dhText: string;
}

export interface ObservationView {
  points: PointView[];
  design: PointView;
  station: number | null;
  backsight: number | null;
  heightText: string | null;
  bs: BacksightView | null;
  fs: ForesightView | null;
  /** 照準器でのぞいているか（false＝望遠鏡） */
  peepSight: boolean;
  /** 現場の指示の聞き取り（練習）。id は指示を頼むたびに変わる。fallback＝AIが使えないときの言い方 */
  instruction: { id: number; fallback: string } | null;
  /** 指示どおりにセットできたか */
  instructionResult: { ok: boolean; items: string[] } | null;
}

export interface UiState {
  /** 画面に出す文の言語（コアの文もこの言語で来る） */
  lang: 'ja' | 'en';
  mode: 'textbook' | 'field';
  phase: 'setup' | 'setupResult' | 'observation';
  currentStep: number;
  steps: StepView[];
  goal: GoalView | null;
  notice: NoticeView | null;
  planted: [boolean, boolean, boolean];
  mounted: boolean;
  clampTight: boolean;
  loops: number;
  judgement: JudgementView | null;
  observation: ObservationView;
  /** 操作中のひとこと（AI先輩）。id はきっかけが出るたびに変わる。fallback＝AIが使えないときの言い方 */
  advice: { id: number; fallback: string } | null;
  /** 教科書モードのナビ。id はきっかけが出るたびに変わる。speech＝読み上げ用の文、ai＝AIに言い方を整えてもらうか */
  navi: { id: number; text: string; speech: string; ai: boolean } | null;
}

/** AI先輩に渡す事実1件（コアが作る見出しと値。AIには、これを言葉にさせるだけ） */
export interface AiFact {
  k: string;
  v: string;
}

export interface DisplayConfig {
  centeringToleranceMm: number;
  tiltToleranceSec: number;
  lcdOuterRangeSec: number;
  lcdInnerRatio: number;
  circularInRatio: number;
  circularVialMaxMm: number;
  slideLimitMm: number;
  sightOkSec: number;
  targetHeightMm: number;
  legHingeRadiusMm: number;
  headTopOffsetMm: number;
  tribrachPivotHeightMm: number;
  screwRadiusMm: number;
  screwAzimuths: [number, number, number];
  trunnionHeightMm: number;
  /** 器械を載せる前に、脚頭の穴から測点が見える範囲（半径 mm） */
  headHoleViewMm: number;
  /** 教科書モード手順1：脚頭の中心と測点の距離の目標 [mm] */
  textbookRoughCenterMm: number;
}
