// Three.js の描画ループ。React から切り離して、requestAnimationFrame で回す。
// 毎フレーム：bridge.frame()（操作を渡して結果を受け取る）→ 3Dに当てはめる → 描く。
// 3Dシーンは1つだけ。ステップが変わっても、カメラの寄り先が変わるだけ（SPEC 4章）。

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { Bridge, Frame } from '../bridge';
import { pointAt } from './axes';
import { World } from './models';

export type ViewName = 'overview' | 'instrument' | 'feet' | 'lcd';
export type Quality = 'light' | 'standard';

const QUALITY: Record<Quality, { pixelRatioCap: number; shadows: boolean; antialias: boolean; shadowSize: number }> = {
  light: { pixelRatioCap: 1, shadows: false, antialias: false, shadowSize: 512 },
  standard: { pixelRatioCap: 1.5, shadows: true, antialias: true, shadowSize: 1024 },
};

/** 望遠鏡の視野 [度] */
const TELESCOPE_FOV_DEG = 1.5;
/** 照準器でのぞいたときの視野 [度]（表示の広さだけ） */
const PEEP_SIGHT_FOV_DEG = 30;
/** カメラを寄せる時間 [s] */
const VIEW_ANIM_SEC = 0.7;

export class Engine {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly camera: THREE.PerspectiveCamera;
  private readonly telCamera: THREE.PerspectiveCamera;
  private readonly controls: OrbitControls;
  private readonly world: World;
  private readonly observer: ResizeObserver;
  private raf = 0;
  private telescopeEl: HTMLElement | null = null;
  private anim: {
    t: number;
    fromPos: THREE.Vector3;
    fromTarget: THREE.Vector3;
    toPos: THREE.Vector3;
    toTarget: THREE.Vector3;
  } | null = null;
  private lastTime = 0;
  private readonly tmp = new THREE.Vector3();
  private readonly tmp2 = new THREE.Vector3();

  constructor(
    private readonly container: HTMLElement,
    private readonly bridge: Bridge,
    quality: Quality,
  ) {
    const q = QUALITY[quality];
    this.renderer = new THREE.WebGLRenderer({ antialias: q.antialias, powerPreference: 'high-performance' });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, q.pixelRatioCap));
    this.renderer.shadowMap.enabled = q.shadows;
    this.renderer.shadowMap.type = THREE.PCFShadowMap;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.domElement.className = 'three-canvas';
    container.appendChild(this.renderer.domElement);

    this.world = new World(bridge.config);
    this.world.sun.castShadow = q.shadows;
    this.world.sun.shadow.mapSize.set(q.shadowSize, q.shadowSize);

    this.camera = new THREE.PerspectiveCamera(45, 1, 0.02, 400);
    this.camera.position.set(1.7, 1.6, 2.6);
    this.telCamera = new THREE.PerspectiveCamera(TELESCOPE_FOV_DEG, 1, 0.5, 500);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.12;
    this.controls.minDistance = 0.15;
    this.controls.maxDistance = 12;
    this.controls.maxPolarAngle = Math.PI * 0.495;
    this.controls.target.set(0, 0.7, 0);
    this.controls.addEventListener('start', () => {
      this.anim = null;
    });

    this.observer = new ResizeObserver(() => this.resize());
    this.observer.observe(container);
    this.resize();

    const first = bridge.current();
    this.world.update(first, bridge.getUi().phase);
    this.setView('overview', true);
    this.raf = requestAnimationFrame(this.loop);
  }

  /** 望遠鏡の視野を描く場所（HTML要素）。null で描かない */
  setTelescopeElement(el: HTMLElement | null): void {
    this.telescopeEl = el;
  }

  /** 視点を切り替える（全体／器械／足元／液晶） */
  setView(view: ViewName, immediate = false): void {
    const f = this.bridge.current();
    const target = new THREE.Vector3();
    const offset = new THREE.Vector3();
    const head = pointAt(f.data, f.o.headCenter, this.tmp);
    const mounted = (f.data[f.o.instValid] ?? 0) > 0.5;
    const pivot = mounted ? pointAt(f.data, f.o.instPivot, this.tmp2) : head;
    switch (view) {
      case 'overview':
        target.set(head.x * 0.5, 0.75, head.z * 0.5);
        offset.set(1.5, 1.0, 2.3);
        break;
      case 'instrument':
        target.copy(pivot).add(new THREE.Vector3(0, 0.04, 0));
        offset.set(0.22, 0.22, 0.62);
        break;
      case 'feet':
        target.set(0, 0, 0);
        offset.set(0.12, 0.42, 0.42);
        break;
      case 'lcd': {
        this.world.lcdPanel.getWorldPosition(target);
        offset.set(0, 0.06, 0.32);
        break;
      }
    }
    const toPos = target.clone().add(offset);
    if (immediate) {
      this.camera.position.copy(toPos);
      this.controls.target.copy(target);
      this.anim = null;
      this.controls.update();
      return;
    }
    this.anim = {
      t: 0,
      fromPos: this.camera.position.clone(),
      fromTarget: this.controls.target.clone(),
      toPos,
      toTarget: target,
    };
  }

  dispose(): void {
    cancelAnimationFrame(this.raf);
    this.observer.disconnect();
    this.controls.dispose();
    this.world.dispose();
    this.renderer.dispose();
    // 画質の切り替えのたびに WebGL の文脈がたまらないよう、すぐに手放す
    this.renderer.forceContextLoss();
    this.renderer.domElement.remove();
  }

  private resize(): void {
    const w = Math.max(1, this.container.clientWidth);
    const h = Math.max(1, this.container.clientHeight);
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }

  private readonly loop = (now: number): void => {
    this.raf = requestAnimationFrame(this.loop);
    const dt = this.lastTime ? Math.min(0.1, (now - this.lastTime) / 1000) : 0;
    this.lastTime = now;
    const f = this.bridge.frame(now);
    const phase = this.bridge.getUi().phase;
    this.world.update(f, phase);
    this.animateCamera(dt);
    this.controls.update();
    this.render(f);
  };

  private animateCamera(dt: number): void {
    const a = this.anim;
    if (!a) return;
    a.t = Math.min(1, a.t + dt / VIEW_ANIM_SEC);
    const k = a.t * a.t * (3 - 2 * a.t);
    this.camera.position.lerpVectors(a.fromPos, a.toPos, k);
    this.controls.target.lerpVectors(a.fromTarget, a.toTarget, k);
    if (a.t >= 1) this.anim = null;
  }

  private render(f: Frame): void {
    const r = this.renderer;
    const canvas = r.domElement;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    r.setViewport(0, 0, w, h);
    r.setScissorTest(false);
    r.render(this.world.scene, this.camera);

    const el = this.telescopeEl;
    if (!el || (f.data[f.o.trunnionValid] ?? 0) < 0.5) return;
    const er = el.getBoundingClientRect();
    const cr = canvas.getBoundingClientRect();
    const vw = Math.round(er.width);
    const vh = Math.round(er.height);
    if (vw < 4 || vh < 4) return;
    const x = Math.round(er.left - cr.left);
    const y = Math.round(cr.bottom - er.bottom);
    pointAt(f.data, f.o.trunnion, this.telCamera.position);
    const dir = pointAt(f.data, f.o.telDir, this.tmp).multiplyScalar(1000);
    this.telCamera.lookAt(this.tmp2.copy(this.telCamera.position).add(dir));
    this.telCamera.fov = (f.data[f.o.peepSight] ?? 0) > 0.5 ? PEEP_SIGHT_FOV_DEG : TELESCOPE_FOV_DEG;
    this.telCamera.aspect = vw / vh;
    this.telCamera.updateProjectionMatrix();
    r.setScissorTest(true);
    r.setViewport(x, y, vw, vh);
    r.setScissor(x, y, vw, vh);
    r.render(this.world.scene, this.telCamera);
    r.setScissorTest(false);
    r.setViewport(0, 0, w, h);
  }
}

/** ステップごとのカメラの寄り先（SPEC 4章） */
export function viewForStep(mode: 'textbook' | 'field', step: number): ViewName {
  if (step >= 8) return 'overview';
  if (mode === 'field') {
    return (['overview', 'feet', 'overview', 'instrument', 'lcd', 'feet', 'lcd'] as const)[step - 1] ?? 'overview';
  }
  return (['overview', 'instrument', 'feet', 'instrument', 'lcd', 'feet', 'lcd'] as const)[step - 1] ?? 'overview';
}
