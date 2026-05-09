import * as THREE from '../../../shared/three.module.js';

const canvas = document.getElementById('scene');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.setClearColor(0x06070b, 1);

const scene = new THREE.Scene();
scene.fog = new THREE.FogExp2(0x06070b, 0.038);

const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 100);
camera.position.set(0, 4.2, 11);

scene.add(new THREE.AmbientLight(0x8bb6ff, 0.42));
const lights = [
  [0x8be9ff, 7, -4, 5, 5],
  [0xff8fb8, 5, 4, 3, 4],
  [0xffd47a, 4, 0, 6, -5],
];
for (const [color, power, x, y, z] of lights) {
  const light = new THREE.PointLight(color, power, 22);
  light.position.set(x, y, z);
  scene.add(light);
}
const key = new THREE.DirectionalLight(0xffffff, 2.6);
key.position.set(4, 8, 5);
key.castShadow = true;
scene.add(key);

const stage = new THREE.Group();
scene.add(stage);

const floor = new THREE.Mesh(
  new THREE.CylinderGeometry(4.9, 5.4, 0.36, 96),
  new THREE.MeshStandardMaterial({ color: 0x111822, roughness: 0.28, metalness: 0.52 }),
);
floor.position.y = -1.2;
floor.receiveShadow = true;
stage.add(floor);

const ring = new THREE.Mesh(
  new THREE.TorusGeometry(3.9, 0.035, 12, 180),
  new THREE.MeshBasicMaterial({ color: 0x8be9ff, transparent: true, opacity: 0.9 }),
);
ring.position.y = -0.98;
ring.rotation.x = Math.PI / 2;
stage.add(ring);

const product = new THREE.Group();
stage.add(product);

const finishes = {
  obsidian: { shell: 0x151922, accent: 0x8be9ff, roughness: 0.22, metalness: 0.72 },
  pearl: { shell: 0xf4f0e6, accent: 0x8be9ff, roughness: 0.34, metalness: 0.16 },
  copper: { shell: 0xc56b3c, accent: 0xffd47a, roughness: 0.27, metalness: 0.65 },
  ice: { shell: 0xbdefff, accent: 0x94a8ff, roughness: 0.08, metalness: 0.18 },
};
let current = finishes.obsidian;

const shellMat = new THREE.MeshStandardMaterial();
const accentMat = new THREE.MeshStandardMaterial();
const glassMat = new THREE.MeshPhysicalMaterial({
  color: 0x9eeaff,
  roughness: 0.05,
  transmission: 0.35,
  thickness: 0.7,
  transparent: true,
  opacity: 0.7,
  emissive: 0x123a50,
  emissiveIntensity: 0.35,
});

function applyFinish(name) {
  current = finishes[name];
  shellMat.color.setHex(current.shell);
  shellMat.roughness = current.roughness;
  shellMat.metalness = current.metalness;
  shellMat.needsUpdate = true;
  accentMat.color.setHex(current.accent);
  accentMat.emissive.setHex(current.accent);
  accentMat.emissiveIntensity = 0.7;
  accentMat.roughness = 0.18;
  accentMat.metalness = 0.35;
  accentMat.needsUpdate = true;
}
applyFinish('obsidian');

function addPod(side) {
  const pod = new THREE.Group();
  const body = new THREE.Mesh(new THREE.CapsuleGeometry(0.58, 1.35, 12, 28), shellMat);
  body.rotation.z = side * 0.22;
  body.castShadow = true;
  pod.add(body);

  const face = new THREE.Mesh(new THREE.SphereGeometry(0.42, 32, 18), glassMat);
  face.position.set(side * 0.1, 0.38, 0.36);
  face.scale.set(0.82, 0.52, 0.18);
  face.castShadow = true;
  pod.add(face);

  const stem = new THREE.Mesh(new THREE.CylinderGeometry(0.18, 0.22, 1.45, 20), shellMat);
  stem.position.set(side * 0.18, -1.15, 0);
  stem.rotation.z = side * 0.12;
  stem.castShadow = true;
  pod.add(stem);

  const line = new THREE.Mesh(new THREE.TorusGeometry(0.49, 0.018, 8, 80), accentMat);
  line.position.z = 0.38;
  line.scale.y = 0.72;
  pod.add(line);

  pod.position.set(side * 0.72, 0.28, 0);
  pod.rotation.y = side * 0.22;
  product.add(pod);
}
addPod(-1);
addPod(1);

const bridge = new THREE.Mesh(new THREE.TorusGeometry(0.86, 0.055, 12, 80, Math.PI), shellMat);
bridge.position.y = 1.35;
bridge.rotation.z = Math.PI;
bridge.castShadow = true;
product.add(bridge);

const particles = new THREE.BufferGeometry();
const count = 380;
const positions = new Float32Array(count * 3);
for (let i = 0; i < count; i++) {
  const a = Math.random() * Math.PI * 2;
  const r = 3.8 + Math.random() * 4.2;
  positions[i * 3] = Math.cos(a) * r;
  positions[i * 3 + 1] = -0.5 + Math.random() * 4.8;
  positions[i * 3 + 2] = Math.sin(a) * r;
}
particles.setAttribute('position', new THREE.BufferAttribute(positions, 3));
scene.add(new THREE.Points(particles, new THREE.PointsMaterial({
  color: 0x8be9ff,
  size: 0.035,
  transparent: true,
  opacity: 0.55,
})));

document.querySelectorAll('[data-finish]').forEach(btn => {
  btn.onclick = () => {
    document.querySelectorAll('[data-finish]').forEach(b => b.classList.toggle('active', b === btn));
    applyFinish(btn.dataset.finish);
  };
});

const pointer = { down: false, x: 0, yaw: 0, targetYaw: 0 };
canvas.addEventListener('pointerdown', e => {
  pointer.down = true;
  pointer.x = e.clientX;
  canvas.setPointerCapture(e.pointerId);
});
canvas.addEventListener('pointermove', e => {
  if (!pointer.down) return;
  pointer.targetYaw += (e.clientX - pointer.x) * 0.008;
  pointer.x = e.clientX;
});
canvas.addEventListener('pointerup', e => {
  pointer.down = false;
  canvas.releasePointerCapture(e.pointerId);
});

function resize() {
  const w = canvas.clientWidth || window.innerWidth;
  const h = canvas.clientHeight || window.innerHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
window.addEventListener('resize', resize);
resize();

const clock = new THREE.Clock();
function animate() {
  const t = clock.getElapsedTime();
  pointer.yaw += (pointer.targetYaw - pointer.yaw) * 0.08;
  stage.rotation.y = pointer.yaw + t * 0.08;
  product.position.y = Math.sin(t * 1.4) * 0.08;
  ring.rotation.z += 0.006;
  camera.lookAt(0, 0.1, 0);
  renderer.render(scene, camera);
  requestAnimationFrame(animate);
}
animate();
