// 画面の文（日本語・英語）。言語はコアが持ち（画面の状態の lang）、ここでは文を選ぶだけ。
// ヒント・注意・判定などの文はコアが作るので、ここには無い。ボタンの名前はコアの文と同じ言い方にそろえる。

import type { ReactNode } from 'react';
import { useUi } from './hooks';

export type Lang = 'ja' | 'en';

type Three = [string, string, string];
type Dir = { up: string; down: string; left: string; right: string };

const ja = {
  appName: 'TS実習シミュレーター',
  appDescription: 'トータルステーションの据付と観測を練習できる3Dシミュレーター',
  textbookMode: '教科書モード',
  fieldMode: '現場モード',

  // 最初の画面
  language: '言語 / Language',
  lead: (
    <>
      トータルステーションの<strong>据付</strong>から<strong>観測（BS・FS）</strong>までを、ブラウザの3Dで練習できます。
      求心と整準の「片方を直すと、もう片方がズレる」ループを体験してください。
    </>
  ) as ReactNode,
  modeAria: 'モード',
  textbookDesc: '教科書の一般的な手順を一歩ずつガイドします。手順外の操作をすると注意が出ます。',
  fieldDesc: '現場の手順。順番は自由で、最後に求心と電子気泡が許容内なら合格です。',
  quality: '画質',
  qualityStandard: '標準',
  qualityLight: '軽量',
  start: 'はじめる',
  keysTitle: 'キーボード操作',
  keys: (
    <>
      <li>
        整準ネジ A：<kbd>A</kbd>（親指を左）／<kbd>S</kbd>（親指を右）
      </li>
      <li>
        整準ネジ B：<kbd>K</kbd>／<kbd>L</kbd>　整準ネジ C：<kbd>Z</kbd>／<kbd>X</kbd>
      </li>
      <li>
        A・B を内側へ：<kbd>S</kbd>＋<kbd>K</kbd>　外側へ：<kbd>A</kbd>＋<kbd>L</kbd>
      </li>
      <li>
        脚を伸ばす：<kbd>1</kbd>
        <kbd>2</kbd>
        <kbd>3</kbd>　縮める：<kbd>Q</kbd>
        <kbd>W</kbd>
        <kbd>E</kbd>
      </li>
      <li>
        三脚・器械・望遠鏡を動かす：矢印キー　定心桿：<kbd>T</kbd>　細かく：<kbd>Shift</kbd>
      </li>
      <li>
        観測：照準器と望遠鏡の切り替え <kbd>P</kbd>　接眼レンズ <kbd>V</kbd>／<kbd>B</kbd>　合焦 <kbd>F</kbd>（近く）／
        <kbd>G</kbd>（遠く）　測量 <kbd>Enter</kbd>
      </li>
    </>
  ) as ReactNode,
  startFine: 'タイムは最初の操作で始まり、「据付完了」を押すと止まります。目安：3分で合格、2分で熟練。',

  // 上部
  paramCentering: '求心ズレ',
  paramTiltX: '電子気泡 X',
  paramTiltY: '電子気泡 Y',
  paramTime: 'タイム',
  paramLoops: 'ループ回数',
  senpai: 'AI先輩',
  senpaiTitle: 'AI先輩に質問する',
  qualityTitle: '画質の切り替え',
  qualityButton: (light: boolean) => `画質：${light ? '軽量' : '標準'}`,
  restart: '最初から',

  // 手順一覧
  stepsAria: '手順',
  part1: '第1部 据付',
  part2: '第2部 観測',
  stepsNoteField: '現場モード：順番は自由です。手順を押すと操作盤が切り替わります。',
  stepsNoteTextbook: '教科書モード：一歩ずつ進みます。手順外の操作をすると注意が出ます。',

  // 操作盤
  stepN: (n: number) => `手順${n}`,
  collapse: '折りたたむ',
  expand: '開く',
  moveFreeLegs: '踏み込んでいない脚を動かす',
  moveTripod: '三脚ごと動かす',
  noteAllPlanted: '3本とも踏み込んだので、脚は動かせません。',
  notePivot: '踏み込んだ脚を支点に、残りの脚を持ち上げて動かします。',
  noteLaser: 'レーザーの赤い点（右上の「求心」）を見ながら、測点の上へ運びます。',
  noteHead: '脚頭の中心が測点の真上に来るように運びます。',
  slideTitle: '器械をずらす（三脚頭の上）',
  slideNote: '定心桿を緩めている間だけ、器械をずらせます。ずらし終えたら締めます。',
  screwsTitle: '整準ネジ',
  legsTitle: '脚の伸縮',
  legsNote: '円形気泡（右上）が中心の円に入るように、気泡がある側の脚を縮めるか、反対側を伸ばします。',
  pressTitle: '脚を踏み込む',
  mountTitle: '器械を載せる',
  mounted: '器械は載っています',
  mountButton: '器械を載せて定心桿で固定する',
  finishSetup: '据付完了',
  goalLabel: 'この手順の目標',
  goalDone: '✓ できています',
  goalNotYet: 'まだです',
  nextStep: '次の手順へ',
  headOffset: '脚頭の中心と測点のずれ',
  headTilt: '脚頭の傾き',

  // 操作の部品
  speed: '速さ',
  speedAria: '操作の速さ',
  speedLabels: ['粗', '中', '微'] as Three,
  speedHint: 'Shiftキーを押している間は「微」',
  dir: { up: '奥へ', down: '手前へ', left: '左へ', right: '右へ' } as Dir,
  arrowKeys: '矢印キー',
  legNames: ['脚1（奥）', '脚2（左手前）', '脚3（右手前）'] as Three,
  screwNames: ['A（左手前）', 'B（右手前）', 'C（奥）'] as Three,
  legPressed: (name: string) => `${name} 踏み込み済み`,
  pressLeg: (name: string) => `${name}を踏み込む`,
  extend: '伸ばす',
  shorten: '縮める',
  thumbLeft: '◀ 親指を左',
  thumbRight: '親指を右 ▶',
  abIn: 'A・B を内側へ',
  abOut: 'A・B を外側へ',
  leftThumbRule: (
    <>
      左手親指の法則：2本を同時に内側または外側へ回すと、気泡は<strong>左手の親指を動かす向き</strong>に動きます。
    </>
  ) as ReactNode,
  screwDiagramAria: '整準ネジの並び（上から見た図）',
  you: '手前（あなた）',
  clampState: (tight: boolean) => `定心桿：${tight ? '締めている' : '緩めている'}`,
  loosen: '緩める',
  tighten: '締める',

  // 観測
  tablet: '電子平板',
  station: '機械点',
  backsight: 'BS点',
  choose: '選んでください',
  instHeight: '器械高 (m)',
  heightPlaceholder: '例 1.500',
  measureTape: '巻尺で測る',
  pointName: '点名',
  tabletNote: '据えた測点を機械点に、ターゲットが立っている点をBS点に選びます（3Dのターゲットを確認）。',
  set: 'セット',
  sightOk: '視準OK',
  sightAdjust: '視準を合わせる',
  sharp: 'くっきり',
  blurry: 'ぼやけている',
  inFocus: '合っている',
  outOfFocus: '合っていない',
  targetBs: 'BS（ターゲット板）',
  targetFs: 'FS（プリズム）',
  look: 'のぞく',
  peepSight: '照準器',
  telescope: '望遠鏡',
  peepNote: '照準器で目標を大まかに捉えたら、望遠鏡に切り替えます（Pキー）。照準器では測量できません。',
  teleNote: '望遠鏡の視野は狭いので、目標が見つからないときは照準器で大まかに向けます（Pキー）。',
  eyepieceTitle: '接眼レンズ（十字線）',
  crosshair: '十字線',
  turnLeft: '左に回す',
  turnRight: '右に回す',
  keyTitle: (k: string) => `${k}キー`,
  keyPair: (a: string, b: string) => `${a}＋${b}`,
  focusTitle: '合焦つまみ（ピント）',
  focus: 'ピント',
  near: '近くへ',
  far: '遠くへ',
  focusNote: '十字線は接眼レンズで、ターゲットのぼけは合焦つまみで直します。どちらかがぼけていると測量できません。',
  sightTitle: (target: string) => `望遠鏡：${target}を視準`,
  ha: '水平角 HA',
  va: '鉛直角 VA',
  offsetH: '十字線とのずれ 水平',
  offsetV: '／ 鉛直',
  teleSpeedLabels: ['粗動', '中', '微動'] as Three,
  teleDir: { up: '上へ', down: '下へ', left: '左へ', right: '右へ' } as Dir,
  teleNote2: '右上の視野で、十字線をターゲットの中心に合わせます。Enterキーでも測量できます。',
  measure: '測量',
  bsTitle: 'BSの観測（方向角の基準）',
  bearing: '方向角',
  hdObserved: '水平距離（観測）',
  hdCoord: '水平距離（座標から）',
  distanceDiff: '較差',
  fsTitle: (name: string) => `FSの観測結果（設計点 ${name}）`,
  observed: '観測',
  design: '設計',
  diff: '差',
  fsNote: (az: string, hd: string) => `方向角 ${az}・水平距離 ${hd} m`,

  // 結果画面
  passTitle: '据付 合格',
  failTitle: '据付 不合格',
  loopsN: (n: number) => `${n} 回`,
  centering: '求心ズレ',
  tiltX: '電子気泡 X',
  tiltY: '電子気泡 Y',
  resultTarget: '目安：3分で合格、2分で熟練。',
  goObservation: '第2部：観測へ進む',
  continueSetup: '続行する',
  askAgain: 'もう一度聞く',
  askReview: 'AI先輩の講評を聞く',

  // AI先輩
  askTitle: 'AI先輩に聞く',
  close: '閉じる',
  askNote: 'いまの状態を踏まえて答えます（質問は200字まで）。',
  askExamples: ['次は何をすればいい？', '求心ってなに？', 'なんで6と7を繰り返すの？'] as Three,
  askPlaceholder: '例：円形気泡が入らないときは？',
  askAria: 'AI先輩への質問',
  ask: '聞く',
  adviceSwitch: '操作中のひとこと（据付の途中で、つまずいたときにAI先輩が声をかける）',
  explain: 'AI先輩に計算を説明してもらう',
  thinking: '考えています…',
  aiNote: 'AIの言葉は参考です。数値と合否は、画面の表示が正しいものです。',
  instructionHead: '現場の指示の聞き取り（練習）',
  instructionNote: '先輩の口頭の指示を聞いて、そのとおりに電子平板をセットします。',
  hearInstruction: '先輩の指示を聞く',
  hearAnother: '別の言い方で聞く',
  replayVoice: 'もう一度聞く（音声）',
  fixedInstruction: '決まった言い方の指示：',
  instructionOk: '指示どおりにセットできました',
  instructionNg: '指示と違うところがあります',
  adviceLabel: 'AI先輩のひとこと',
  navi: 'ナビ',
  replay: 'もう一度',
  voice: (on: boolean) => `声：${on ? 'ON' : 'OFF'}`,

  // 3Dビュー
  views: { overview: '全体', instrument: '器械', feet: '足元', lcd: '液晶' },
  viewAria: '視点切替',
  zoom: '拡大',
  zoomTitle: '拡大表示を出す／隠す',
  webglError: (e: string) => `3Dを表示できませんでした（WebGLが使えない可能性があります）：${e}`,
  centeringCaption: '求心（上＝奥）',
  peekCaption: '脚頭の穴から（上＝奥）',
  peekSeen: '目標の円に入れば位置はOK',
  peekNotSeen: '測点が見えない',
  centeringAria: 'レーザー求心の拡大',
  circularCaption: '円形気泡',
  circularAria: '円形気泡の拡大',
  circularHint: '中心の円に入れば合格',
  lcdCaption: '液晶（電子気泡）',
  lcdAria: '液晶の電子気泡の拡大',
  scopeSizes: ['小', '中', '大'] as Three,
  scopeSizesAria: '視野の大きさ',
  peepView: '照準器の視野',
  teleView: '望遠鏡の視野',
  dragTitle: 'つかんで動かせます',
};

export type Dict = typeof ja;

const en: Dict = {
  appName: 'TS Training Simulator',
  appDescription: 'A 3D simulator for practicing total station setup and observation',
  textbookMode: 'Textbook mode',
  fieldMode: 'Field mode',

  language: '言語 / Language',
  lead: (
    <>
      Practice a total station from <strong>setup</strong> to <strong>observation (backsight and foresight)</strong> in
      3D in your browser. Experience the loop where fixing centering upsets the level, and fixing the level upsets
      centering.
    </>
  ),
  modeAria: 'Mode',
  textbookDesc: 'Guides you through the standard textbook steps one at a time. A note appears if you do an operation from another step.',
  fieldDesc: 'The field procedure. Any order is fine; you pass if centering and the electronic bubble are within tolerance at the end.',
  quality: 'Graphics',
  qualityStandard: 'Standard',
  qualityLight: 'Light',
  start: 'Start',
  keysTitle: 'Keyboard controls',
  keys: (
    <>
      <li>
        Leveling screw A: <kbd>A</kbd> (thumb left) / <kbd>S</kbd> (thumb right)
      </li>
      <li>
        Leveling screw B: <kbd>K</kbd> / <kbd>L</kbd>　Leveling screw C: <kbd>Z</kbd> / <kbd>X</kbd>
      </li>
      <li>
        A and B inward: <kbd>S</kbd>+<kbd>K</kbd>　outward: <kbd>A</kbd>+<kbd>L</kbd>
      </li>
      <li>
        Extend legs: <kbd>1</kbd>
        <kbd>2</kbd>
        <kbd>3</kbd>　shorten: <kbd>Q</kbd>
        <kbd>W</kbd>
        <kbd>E</kbd>
      </li>
      <li>
        Move tripod / instrument / telescope: arrow keys　Mounting screw: <kbd>T</kbd>　Fine: <kbd>Shift</kbd>
      </li>
      <li>
        Observation: peep sight / telescope <kbd>P</kbd>　Eyepiece <kbd>V</kbd> / <kbd>B</kbd>　Focus <kbd>F</kbd> (near) /{' '}
        <kbd>G</kbd> (far)　Measure <kbd>Enter</kbd>
      </li>
    </>
  ),
  startFine:
    'The timer starts with your first operation and stops when you press "Finish setup". Target: pass within 3 minutes, expert within 2 minutes.',

  paramCentering: 'Centering',
  paramTiltX: 'Bubble X',
  paramTiltY: 'Bubble Y',
  paramTime: 'Time',
  paramLoops: 'Loops',
  senpai: 'AI Senpai',
  senpaiTitle: 'Ask AI Senpai',
  qualityTitle: 'Switch graphics quality',
  qualityButton: (light: boolean) => `Graphics: ${light ? 'Light' : 'Standard'}`,
  restart: 'Restart',

  stepsAria: 'Steps',
  part1: 'Part 1  Setup',
  part2: 'Part 2  Observation',
  stepsNoteField: 'Field mode: any order is fine. Tap a step to switch the control panel.',
  stepsNoteTextbook: 'Textbook mode: go one step at a time. A note appears if you do an operation from another step.',

  stepN: (n: number) => `Step ${n}`,
  collapse: 'Collapse',
  expand: 'Open',
  moveFreeLegs: 'Move the legs not pressed in',
  moveTripod: 'Move the whole tripod',
  noteAllPlanted: 'All three legs are pressed in, so the legs cannot move.',
  notePivot: 'The pressed-in leg is the pivot. Lift and move the other legs.',
  noteLaser: 'Watch the red laser dot (the "Centering" view at the top right) and carry the tripod over the point.',
  noteHead: 'Carry the tripod so that the head center comes right over the point.',
  slideTitle: 'Slide the instrument (on the tripod head)',
  slideNote: 'You can slide the instrument only while the mounting screw is loose. Tighten it when you are done.',
  screwsTitle: 'Leveling screws',
  legsTitle: 'Leg length',
  legsNote:
    'To bring the circular bubble (top right) into the center circle, shorten the leg on the bubble side or extend the opposite one.',
  pressTitle: 'Press in the legs',
  mountTitle: 'Mount the instrument',
  mounted: 'The instrument is mounted',
  mountButton: 'Mount the instrument and fix it with the mounting screw',
  finishSetup: 'Finish setup',
  goalLabel: 'Goal of this step',
  goalDone: '✓ Done',
  goalNotYet: 'Not yet',
  nextStep: 'Next step',
  headOffset: 'Head center to point',
  headTilt: 'Head tilt',

  speed: 'Speed',
  speedAria: 'Operation speed',
  speedLabels: ['Coarse', 'Medium', 'Fine'],
  speedHint: 'Hold Shift for "Fine"',
  dir: { up: 'Far', down: 'Near', left: 'Left', right: 'Right' },
  arrowKeys: 'Arrow keys',
  legNames: ['Leg 1 (far)', 'Leg 2 (near left)', 'Leg 3 (near right)'],
  screwNames: ['A (near left)', 'B (near right)', 'C (far)'],
  legPressed: (name: string) => `${name} pressed in`,
  pressLeg: (name: string) => `Press in ${name}`,
  extend: 'Extend',
  shorten: 'Shorten',
  thumbLeft: '◀ Thumb left',
  thumbRight: 'Thumb right ▶',
  abIn: 'A + B inward',
  abOut: 'A + B outward',
  leftThumbRule: (
    <>
      Left-thumb rule: when you turn two screws inward or outward together, the bubble moves{' '}
      <strong>in the direction your left thumb moves</strong>.
    </>
  ),
  screwDiagramAria: 'Leveling screw layout (top view)',
  you: 'Near side (you)',
  clampState: (tight: boolean) => `Mounting screw: ${tight ? 'tight' : 'loose'}`,
  loosen: 'Loosen',
  tighten: 'Tighten',

  tablet: 'Tablet',
  station: 'Station',
  backsight: 'Backsight',
  choose: 'Choose',
  instHeight: 'Instrument height (m)',
  heightPlaceholder: 'e.g. 1.500',
  measureTape: 'Measure with tape',
  pointName: 'Point',
  tabletNote:
    'Choose the point you set up on as the station, and the point with the target as the backsight (check the target in 3D).',
  set: 'Set',
  sightOk: 'On target',
  sightAdjust: 'Aim at the target',
  sharp: 'sharp',
  blurry: 'blurry',
  inFocus: 'in focus',
  outOfFocus: 'out of focus',
  targetBs: 'BS (target plate)',
  targetFs: 'FS (prism)',
  look: 'View through',
  peepSight: 'Peep sight',
  telescope: 'Telescope',
  peepNote:
    'Once you roughly catch the target in the peep sight, switch to the telescope (P key). You cannot measure with the peep sight.',
  teleNote: 'The telescope view is narrow, so if you cannot find the target, aim roughly with the peep sight (P key).',
  eyepieceTitle: 'Eyepiece (crosshair)',
  crosshair: 'Crosshair',
  turnLeft: 'Turn left',
  turnRight: 'Turn right',
  keyTitle: (k: string) => `${k} key`,
  keyPair: (a: string, b: string) => `${a}+${b}`,
  focusTitle: 'Focus knob',
  focus: 'Focus',
  near: 'Near',
  far: 'Far',
  focusNote:
    'Fix the crosshair with the eyepiece, and the target blur with the focus knob. You cannot measure if either is blurry.',
  sightTitle: (target: string) => `Telescope: sight the ${target}`,
  ha: 'Horizontal angle HA',
  va: 'Vertical angle VA',
  offsetH: 'Offset from crosshair: H',
  offsetV: '/ V',
  teleSpeedLabels: ['Coarse', 'Medium', 'Fine'],
  teleDir: { up: 'Up', down: 'Down', left: 'Left', right: 'Right' },
  teleNote2: 'In the view at the top right, put the crosshair on the target center. You can also measure with the Enter key.',
  measure: 'Measure',
  bsTitle: 'Backsight observation (bearing reference)',
  bearing: 'Bearing',
  hdObserved: 'Horizontal distance (observed)',
  hdCoord: 'Horizontal distance (from coordinates)',
  distanceDiff: 'Difference',
  fsTitle: (name: string) => `Foresight result (design point ${name})`,
  observed: 'Observed',
  design: 'Design',
  diff: 'Diff.',
  fsNote: (az: string, hd: string) => `Bearing ${az}, horizontal distance ${hd} m`,

  passTitle: 'Setup passed',
  failTitle: 'Setup failed',
  loopsN: (n: number) => `${n}`,
  centering: 'Centering error',
  tiltX: 'Electronic bubble X',
  tiltY: 'Electronic bubble Y',
  resultTarget: 'Target: pass within 3 minutes, expert within 2 minutes.',
  goObservation: 'Go to Part 2: Observation',
  continueSetup: 'Continue',
  askAgain: 'Ask again',
  askReview: "Get AI Senpai's review",

  askTitle: 'Ask AI Senpai',
  close: 'Close',
  askNote: 'Answers based on where you are now (questions up to 200 characters).',
  askExamples: ['What should I do next?', 'What is centering?', 'Why do I repeat steps 6 and 7?'],
  askPlaceholder: "e.g. What if the circular bubble won't center?",
  askAria: 'Question for AI Senpai',
  ask: 'Ask',
  adviceSwitch: 'Tips while operating (AI Senpai speaks up when you get stuck during setup)',
  explain: 'Have AI Senpai explain the calculation',
  thinking: 'Thinking…',
  aiNote: "The AI's words are for reference. The numbers and pass/fail on screen are what count.",
  instructionHead: 'Listening to field instructions (practice)',
  instructionNote: "Listen to the senpai's spoken instruction and set up the tablet exactly as told.",
  hearInstruction: 'Hear the instruction',
  hearAnother: 'Hear it another way',
  replayVoice: 'Play again (voice)',
  fixedInstruction: 'Standard instruction: ',
  instructionOk: 'Set up as instructed',
  instructionNg: 'Something differs from the instruction',
  adviceLabel: 'Tip from AI Senpai',
  navi: 'Navi',
  replay: 'Replay',
  voice: (on: boolean) => `Voice: ${on ? 'ON' : 'OFF'}`,

  views: { overview: 'Overview', instrument: 'Instrument', feet: 'Feet', lcd: 'LCD' },
  viewAria: 'Viewpoint',
  zoom: 'Zoom',
  zoomTitle: 'Show / hide the close-up views',
  webglError: (e: string) => `Could not show 3D (WebGL may not be available): ${e}`,
  centeringCaption: 'Centering (up=far)',
  peekCaption: 'Head hole (up=far)',
  peekSeen: 'Inside the goal circle = position OK',
  peekNotSeen: 'Point not visible',
  centeringAria: 'Laser plummet close-up',
  circularCaption: 'Circular bubble',
  circularAria: 'Circular bubble close-up',
  circularHint: 'Pass when inside the center circle',
  lcdCaption: 'LCD (electronic bubble)',
  lcdAria: 'LCD electronic bubble close-up',
  scopeSizes: ['S', 'M', 'L'],
  scopeSizesAria: 'View size',
  peepView: 'Peep sight view',
  teleView: 'Telescope view',
  dragTitle: 'Drag to move',
};

export function dict(lang: Lang): Dict {
  return lang === 'en' ? en : ja;
}

/** いまの言語の文 */
export function useT(): Dict {
  return dict(useUi().lang);
}
