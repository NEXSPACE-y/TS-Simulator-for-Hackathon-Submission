// 3Dのモデル（地面・測点・三脚・器械・ターゲット）。形はシンプルに、質感はテクスチャで。
// 位置や傾きはすべてコアの結果（Frame）をそのまま当てはめるだけ。
// TS本体は特定メーカーの製品に似せない（ロゴ・メーカー名なし、汎用の色と形）。

import * as THREE from 'three';
import type { DisplayConfig, Frame } from '../bridge';
import { dmsCircleAt, dmsSignedAt, flag } from '../bridge/format';
import { localAzimuth, pointAt, rotationAt, spanBetween } from './axes';
import { LcdScreen } from './lcd';
import {
  groundTexture,
  knurlTexture,
  markTexture,
  poleTexture,
  prismFrameTexture,
  targetTexture,
} from './textures';

/** 脚の上の段（付け根から固定の長さ）[m] */
const LEG_UPPER = 0.78;
/** 上の段と下の段の重なり [m] */
const LEG_OVERLAP = 0.12;

const COLORS = {
  legUpper: 0x6b5338,
  legLower: 0xb8bcc0,
  metalDark: 0x2c3036,
  head: 0x3a3f46,
  body: 0xe2e0da,
  bodyDark: 0x34383f,
  accent: 0x4d6f87,
  glass: 0x9fd0c8,
  laser: 0xff2a1a,
};

function mat(color: number, opts: THREE.MeshLambertMaterialParameters = {}): THREE.MeshLambertMaterial {
  return new THREE.MeshLambertMaterial({ color, ...opts });
}

function shadowed<T extends THREE.Object3D>(o: T): T {
  o.traverse((c) => {
    if ((c as THREE.Mesh).isMesh) {
      c.castShadow = true;
    }
  });
  return o;
}

interface Leg {
  upper: THREE.Mesh;
  lower: THREE.Mesh;
  clamp: THREE.Mesh;
  shoe: THREE.Mesh;
}

export class World {
  readonly scene = new THREE.Scene();
  readonly sun: THREE.DirectionalLight;
  readonly lcd = new LcdScreen();
  /** 液晶パネル（カメラの寄り先に使う） */
  readonly lcdPanel: THREE.Mesh;
  readonly instrument = new THREE.Group();
  readonly head = new THREE.Group();
  private readonly legs: Leg[] = [];
  private readonly alidade = new THREE.Group();
  private readonly telescope = new THREE.Group();
  private readonly knobs: THREE.Mesh[] = [];
  private readonly circBubble: THREE.Mesh;
  private readonly laserBeam: THREE.Mesh;
  private readonly laserSpot: THREE.Group;
  private readonly bsTarget = new THREE.Group();
  private readonly fsTarget = new THREE.Group();
  private targetsPlaced = false;
  private readonly tmpA = new THREE.Vector3();
  private readonly tmpB = new THREE.Vector3();
  private readonly tmpC = new THREE.Vector3();
  private readonly tmpD = new THREE.Vector3();
  private readonly tmpE = new THREE.Vector3();
  private readonly tmpF = new THREE.Vector3();
  private readonly tmpM = new THREE.Matrix4();

  constructor(private readonly config: DisplayConfig) {
    const s = this.scene;
    s.background = new THREE.Color(0xbfd3e2);
    s.fog = new THREE.Fog(0xbfd3e2, 60, 220);

    s.add(new THREE.HemisphereLight(0xe8f1ff, 0x5a4a38, 1.6));
    this.sun = new THREE.DirectionalLight(0xfff4e0, 2.2);
    this.sun.position.set(3, 6, 2.5);
    this.sun.target.position.set(0, 0.6, 0);
    this.sun.shadow.camera.left = -1.6;
    this.sun.shadow.camera.right = 1.6;
    this.sun.shadow.camera.top = 1.8;
    this.sun.shadow.camera.bottom = -1.6;
    this.sun.shadow.camera.near = 1;
    this.sun.shadow.camera.far = 12;
    this.sun.shadow.bias = -0.0006;
    s.add(this.sun, this.sun.target);

    // 地面
    const ground = new THREE.Mesh(
      new THREE.PlaneGeometry(400, 400),
      mat(0xffffff, { map: groundTexture() }),
    );
    ground.rotation.x = -Math.PI / 2;
    ground.position.y = -0.004;
    ground.receiveShadow = true;
    s.add(ground);

    // 測点（コンクリートの上の金属標）
    const block = new THREE.Mesh(new THREE.BoxGeometry(0.14, 0.04, 0.14), mat(0x9a9890));
    block.position.y = -0.0205;
    block.receiveShadow = true;
    const plate = new THREE.Mesh(new THREE.CircleGeometry(0.035, 32), mat(0xffffff, { map: markTexture() }));
    plate.rotation.x = -Math.PI / 2;
    plate.position.y = 0.0002;
    plate.receiveShadow = true;
    s.add(block, plate);

    // 三脚の脚
    const upperGeo = new THREE.CylinderGeometry(0.017, 0.02, 1, 8);
    const lowerGeo = new THREE.CylinderGeometry(0.012, 0.012, 1, 8);
    const clampGeo = new THREE.BoxGeometry(0.05, 0.05, 0.05);
    const shoeGeo = new THREE.ConeGeometry(0.016, 1, 8);
    for (let i = 0; i < 3; i++) {
      const leg: Leg = {
        upper: new THREE.Mesh(upperGeo, mat(COLORS.legUpper)),
        lower: new THREE.Mesh(lowerGeo, mat(COLORS.legLower)),
        clamp: new THREE.Mesh(clampGeo, mat(COLORS.metalDark)),
        shoe: new THREE.Mesh(shoeGeo, mat(COLORS.metalDark)),
      };
      for (const m of Object.values(leg)) {
        m.castShadow = true;
        s.add(m);
      }
      this.legs.push(leg);
    }

    // 三脚頭
    const headDisc = new THREE.Mesh(new THREE.CylinderGeometry(0.085, 0.09, 0.03, 6), mat(COLORS.head));
    headDisc.position.y = config.headTopOffsetMm / 1000 - 0.015;
    this.head.add(headDisc);
    for (let i = 0; i < 3; i++) {
      const lug = new THREE.Mesh(new THREE.BoxGeometry(0.05, 0.03, 0.03), mat(COLORS.metalDark));
      const a = (90 + 120 * i) * (Math.PI / 180);
      localAzimuth(a, config.legHingeRadiusMm / 1000, -0.004, lug.position);
      lug.rotation.y = a;
      this.head.add(lug);
    }
    s.add(shadowed(this.head));

    // 器械（原点＝整準ネジの先端の面と鉛直軸の交点）
    this.buildInstrument();
    this.lcdPanel = this.buildAlidade();
    const circ = this.buildCircularLevel();
    this.circBubble = circ;
    s.add(shadowed(this.instrument));

    // レーザー
    const laserMat = new THREE.MeshBasicMaterial({
      color: COLORS.laser,
      transparent: true,
      opacity: 0.55,
      depthWrite: false,
    });
    this.laserBeam = new THREE.Mesh(new THREE.CylinderGeometry(0.0007, 0.0007, 1, 6), laserMat);
    this.laserSpot = new THREE.Group();
    const spot = new THREE.Mesh(
      new THREE.CircleGeometry(0.0022, 16),
      new THREE.MeshBasicMaterial({ color: COLORS.laser }),
    );
    spot.rotation.x = -Math.PI / 2;
    const glow = new THREE.Mesh(
      new THREE.CircleGeometry(0.008, 24),
      new THREE.MeshBasicMaterial({
        color: COLORS.laser,
        transparent: true,
        opacity: 0.28,
        depthWrite: false,
      }),
    );
    glow.rotation.x = -Math.PI / 2;
    this.laserSpot.add(spot, glow);
    s.add(this.laserBeam, this.laserSpot);

    // BS・FS のターゲット
    this.buildTargets();
  }

  private buildInstrument(): void {
    const g = this.instrument;
    const pivotH = this.config.tribrachPivotHeightMm / 1000;
    // 整準台の底板（三角形に近い形）
    const base = new THREE.Mesh(new THREE.CylinderGeometry(0.075, 0.08, 0.012, 3), mat(COLORS.bodyDark));
    base.position.y = -pivotH + 0.006;
    base.rotation.y = Math.PI / 6;
    g.add(base);
    // 整準ネジ（つまみ）
    const knobGeo = new THREE.CylinderGeometry(0.013, 0.013, 0.014, 16);
    const knobMat = [
      mat(0xffffff, { map: knurlTexture() }),
      mat(COLORS.accent),
      mat(COLORS.accent),
    ];
    const r = this.config.screwRadiusMm / 1000;
    for (const az of this.config.screwAzimuths) {
      const knob = new THREE.Mesh(knobGeo, knobMat);
      localAzimuth(az, r, -pivotH * 0.45, knob.position);
      const stem = new THREE.Mesh(new THREE.CylinderGeometry(0.004, 0.004, pivotH, 6), mat(COLORS.metalDark));
      localAzimuth(az, r, -pivotH * 0.5, stem.position);
      g.add(knob, stem);
      this.knobs.push(knob);
    }
    // 整準台の上部
    const top = new THREE.Mesh(new THREE.CylinderGeometry(0.07, 0.076, 0.02, 3), mat(COLORS.bodyDark));
    top.position.y = 0.006;
    top.rotation.y = Math.PI / 6;
    g.add(top);
  }

  /** 円形気泡（整準台の手前側） */
  private buildCircularLevel(): THREE.Mesh {
    const vial = new THREE.Group();
    localAzimuth(-Math.PI / 2, 0.088, 0.01, vial.position);
    const housing = new THREE.Mesh(new THREE.CylinderGeometry(0.013, 0.013, 0.006, 20), mat(COLORS.metalDark));
    const glass = new THREE.Mesh(
      new THREE.CylinderGeometry(0.011, 0.011, 0.0012, 20),
      new THREE.MeshLambertMaterial({ color: COLORS.glass, transparent: true, opacity: 0.85 }),
    );
    glass.position.y = 0.0035;
    const ring = new THREE.Mesh(
      new THREE.RingGeometry(0.011 * this.config.circularInRatio * 0.92, 0.011 * this.config.circularInRatio, 24),
      new THREE.MeshBasicMaterial({ color: 0x111111 }),
    );
    ring.rotation.x = -Math.PI / 2;
    ring.position.y = 0.0042;
    const bubble = new THREE.Mesh(
      new THREE.CircleGeometry(0.0032, 16),
      new THREE.MeshBasicMaterial({ color: 0xf6fff8 }),
    );
    bubble.rotation.x = -Math.PI / 2;
    bubble.position.y = 0.0044;
    vial.add(housing, glass, ring, bubble);
    this.instrument.add(vial);
    return bubble;
  }

  /** 上部（照準部）と望遠鏡。返り値は液晶パネル */
  private buildAlidade(): THREE.Mesh {
    const a = this.alidade;
    a.position.y = 0.018;
    const plate = new THREE.Mesh(new THREE.CylinderGeometry(0.072, 0.072, 0.03, 24), mat(COLORS.body));
    plate.position.y = 0.015;
    a.add(plate);
    // 支柱（左右）
    for (const x of [-0.068, 0.068]) {
      const post = new THREE.Mesh(new THREE.BoxGeometry(0.03, 0.19, 0.1), mat(COLORS.body));
      post.position.set(x, 0.125, 0);
      a.add(post);
    }
    // 取っ手
    const bar = new THREE.Mesh(new THREE.BoxGeometry(0.16, 0.014, 0.03), mat(COLORS.bodyDark));
    bar.position.set(0, 0.235, 0);
    a.add(bar);
    // 表示・キーのパネル（手前＝南側＝ +z）
    const panelBox = new THREE.Mesh(new THREE.BoxGeometry(0.12, 0.085, 0.022), mat(COLORS.bodyDark));
    panelBox.position.set(0, 0.075, 0.062);
    panelBox.rotation.x = -0.18;
    a.add(panelBox);
    const screen = new THREE.Mesh(
      new THREE.PlaneGeometry(0.09, 0.056),
      new THREE.MeshBasicMaterial({ map: this.lcd.texture }),
    );
    screen.position.set(0, 0.082, 0.0745);
    screen.rotation.x = -0.18;
    a.add(screen);
    // 望遠鏡（回転軸＝器械高の基準）
    const t = this.telescope;
    t.position.y = this.config.trunnionHeightMm / 1000 - 0.018;
    const tube = new THREE.Mesh(new THREE.CylinderGeometry(0.03, 0.03, 0.15, 20), mat(COLORS.bodyDark));
    tube.rotation.x = Math.PI / 2;
    const objective = new THREE.Mesh(new THREE.CylinderGeometry(0.036, 0.032, 0.03, 20), mat(COLORS.bodyDark));
    objective.rotation.x = Math.PI / 2;
    objective.position.z = -0.085;
    const lens = new THREE.Mesh(new THREE.CircleGeometry(0.03, 20), mat(0x203a48));
    lens.position.z = -0.1005;
    lens.rotation.y = Math.PI;
    const eyepiece = new THREE.Mesh(new THREE.CylinderGeometry(0.016, 0.02, 0.04, 16), mat(COLORS.metalDark));
    eyepiece.rotation.x = Math.PI / 2;
    eyepiece.position.z = 0.09;
    const axle = new THREE.Mesh(new THREE.CylinderGeometry(0.016, 0.016, 0.13, 12), mat(COLORS.accent));
    axle.rotation.z = Math.PI / 2;
    t.add(tube, objective, lens, eyepiece, axle);
    a.add(t);
    this.instrument.add(a);
    return screen;
  }

  private buildTargets(): void {
    const pole = new THREE.CylinderGeometry(0.012, 0.012, 1, 8);
    const poleMat = mat(0xffffff, { map: poleTexture() });
    const h = this.config.targetHeightMm / 1000;
    // BS：ターゲット板
    // ポールは板の下端から地面まで（板の後ろ側に置き、視準の邪魔をしない）
    const bsPole = new THREE.Mesh(pole, poleMat);
    bsPole.scale.y = h - 0.1;
    bsPole.position.set(0, -(h + 0.1) / 2, -0.02);
    const board = new THREE.Mesh(new THREE.PlaneGeometry(0.2, 0.2), mat(0xffffff, { map: targetTexture(), side: THREE.DoubleSide }));
    this.bsTarget.add(bsPole, board);
    // FS：プリズム
    const fsPole = new THREE.Mesh(pole, poleMat);
    fsPole.scale.y = h - 0.07;
    fsPole.position.set(0, -(h + 0.07) / 2, -0.03);
    const frame = new THREE.Mesh(new THREE.PlaneGeometry(0.14, 0.14), mat(0xffffff, { map: prismFrameTexture(), side: THREE.DoubleSide }));
    frame.position.z = -0.012;
    const prism = new THREE.Mesh(
      new THREE.CircleGeometry(0.03, 24),
      new THREE.MeshPhongMaterial({ color: 0x3b4d5c, specular: 0xffffff, shininess: 90 }),
    );
    prism.position.z = 0.001;
    const crossMat = new THREE.MeshBasicMaterial({ color: 0x111111 });
    const crossH = new THREE.Mesh(new THREE.PlaneGeometry(0.06, 0.002), crossMat);
    const crossV = new THREE.Mesh(new THREE.PlaneGeometry(0.002, 0.06), crossMat);
    crossH.position.z = crossV.position.z = 0.002;
    this.fsTarget.add(fsPole, frame, prism, crossH, crossV);
    this.scene.add(this.bsTarget, this.fsTarget);
  }

  /** コアの結果を3Dに当てはめる */
  /** 形・材質・画像を GPU から片付ける */
  dispose(): void {
    this.scene.traverse((obj) => {
      const mesh = obj as Partial<THREE.Mesh>;
      mesh.geometry?.dispose();
      const mats = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : [];
      for (const m of mats) {
        for (const v of Object.values(m)) {
          if (v instanceof THREE.Texture) v.dispose();
        }
        m.dispose();
      }
    });
  }

  update(f: Frame, phase: 'setup' | 'setupResult' | 'observation'): void {
    const d = f.data;
    const o = f.o;
    if (!this.targetsPlaced) {
      pointAt(d, o.bsTarget, this.bsTarget.position);
      pointAt(d, o.fsTarget, this.fsTarget.position);
      for (const t of [this.bsTarget, this.fsTarget]) {
        t.lookAt(0, t.position.y, 0);
      }
      this.targetsPlaced = true;
    }

    // 三脚の脚：付け根から上の段、足先から下の段
    for (let i = 0; i < 3; i++) {
      const leg = this.legs[i];
      if (!leg) continue;
      const foot = pointAt(d, o.feet + i * 3, this.tmpA);
      const hinge = pointAt(d, o.hinges + i * 3, this.tmpB);
      const len = (d[o.legLen + i] ?? 1000) / 1000;
      const dir = this.tmpC.subVectors(foot, hinge).normalize();
      const upperEnd = this.tmpD.copy(hinge).addScaledVector(dir, LEG_UPPER);
      spanBetween(leg.upper, hinge, upperEnd);
      const lowerLen = Math.max(0.1, len - LEG_UPPER + LEG_OVERLAP);
      const lowerTop = this.tmpE.copy(foot).addScaledVector(dir, -lowerLen);
      const shoeTop = this.tmpF.copy(foot).addScaledVector(dir, -0.06);
      spanBetween(leg.lower, shoeTop, lowerTop);
      spanBetween(leg.shoe, shoeTop, foot);
      leg.clamp.position.copy(upperEnd).addScaledVector(dir, -0.03);
      leg.clamp.quaternion.copy(leg.upper.quaternion);
    }

    // 三脚頭
    pointAt(d, o.headCenter, this.head.position);
    this.head.quaternion.setFromRotationMatrix(rotationAt(d, o.headRot, this.tmpM));

    // 器械
    const mounted = flag(f, o.instValid);
    this.instrument.visible = mounted;
    if (mounted) {
      pointAt(d, o.instPivot, this.instrument.position);
      this.instrument.quaternion.setFromRotationMatrix(rotationAt(d, o.instRot, this.tmpM));
      for (let k = 0; k < 3; k++) {
        const knob = this.knobs[k];
        if (knob) knob.rotation.y = -(d[o.screwAngles + k] ?? 0);
      }
      const cn = o.circNorm;
      const vialR = 0.011 - 0.0032;
      this.circBubble.position.x = (d[cn] ?? 0) * vialR;
      this.circBubble.position.z = -(d[cn + 1] ?? 0) * vialR;
      // 照準部は望遠鏡の向き（北から時計回り）に回る。器械自体の向きはコアが除いてある
      this.alidade.rotation.y = -(d[o.telHzLocal] ?? 0);
      this.telescope.rotation.x = Math.PI / 2 - (d[o.telZenith] ?? Math.PI / 2);
    }

    // レーザー
    const laserOn = mounted && flag(f, o.laserValid);
    this.laserBeam.visible = laserOn;
    this.laserSpot.visible = laserOn;
    if (laserOn) {
      const from = pointAt(d, o.instPivot, this.tmpA);
      const to = pointAt(d, o.laser, this.tmpB);
      spanBetween(this.laserBeam, to, from);
      this.laserSpot.position.copy(to);
      this.laserSpot.position.y = 0.0006;
    }

    // 液晶
    if (!mounted) {
      this.lcd.draw({ mode: 'off', dotX: 0, dotY: 0, innerRatio: 0, line1: '', line2: '' });
    } else if (phase === 'observation') {
      this.lcd.draw({
        mode: 'angle',
        dotX: 0,
        dotY: 0,
        innerRatio: 0,
        line1: `HA ${dmsCircleAt(f, o.hrDms)}`,
        line2: `VA ${dmsCircleAt(f, o.vrDms)}`,
      });
    } else {
      this.lcd.draw({
        mode: 'level',
        dotX: d[o.lcdDot] ?? 0,
        dotY: d[o.lcdDot + 1] ?? 0,
        innerRatio: this.config.lcdInnerRatio,
        line1: `X${dmsSignedAt(f, o.tiltXDms)}`,
        line2: `Y${dmsSignedAt(f, o.tiltYDms)}`,
      });
    }
  }
}
