// コアの座標（x＝東、y＝北、z＝上、mm）から Three.js の座標（x、y＝上、z、m）への置き換え。
// 表示のための軸の入れ替えと単位の置き換えだけで、物理の計算はしない。

import * as THREE from 'three';

/** data[i..i+3] の点を Three.js の座標で out に入れる */
export function pointAt(data: Float64Array, i: number, out: THREE.Vector3): THREE.Vector3 {
  return out.set((data[i] ?? 0) / 1000, (data[i + 2] ?? 0) / 1000, -(data[i + 1] ?? 0) / 1000);
}

/** data[i..i+9] の回転行列（行優先）を Three.js の座標の回転にして out に入れる */
export function rotationAt(data: Float64Array, i: number, out: THREE.Matrix4): THREE.Matrix4 {
  const r = (k: number) => data[i + k] ?? 0;
  // P R Pᵀ（P：コア→Three.js の軸の入れ替え）
  out.set(
    r(0), r(2), -r(1), 0,
    r(6), r(8), -r(7), 0,
    -r(3), -r(5), r(4), 0,
    0, 0, 0, 1,
  );
  return out;
}

const UP = new THREE.Vector3(0, 1, 0);
const tmpDir = new THREE.Vector3();

/** 高さ1・Y軸向きの円柱を、a から b へ渡す */
export function spanBetween(mesh: THREE.Object3D, a: THREE.Vector3, b: THREE.Vector3): void {
  tmpDir.subVectors(b, a);
  const len = tmpDir.length();
  if (len < 1e-6) {
    mesh.visible = false;
    return;
  }
  mesh.visible = true;
  mesh.position.addVectors(a, b).multiplyScalar(0.5);
  mesh.quaternion.setFromUnitVectors(UP, tmpDir.divideScalar(len));
  mesh.scale.set(1, len, 1);
}

/** コアの方位（東から反時計回り、rad）・半径 → 器械や三脚頭の中の位置（Three.js のローカル座標） */
export function localAzimuth(az: number, r: number, y: number, out: THREE.Vector3): THREE.Vector3 {
  return out.set(r * Math.cos(az), y, -r * Math.sin(az));
}
