import * as THREE from '../../../shared/three.module.js';

const canvas = document.getElementById('scene');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x91d7ff);
scene.fog = new THREE.Fog(0x91d7ff, 18, 58);

const camera = new THREE.PerspectiveCamera(48, 1, 0.1, 120);
camera.position.set(14, 13, 18);

const hemi = new THREE.HemisphereLight(0xbfeeff, 0x47704b, 1.8);
scene.add(hemi);
const sun = new THREE.DirectionalLight(0xfff3c1, 4.2);
sun.position.set(10, 14, 8);
sun.castShadow = true;
sun.shadow.mapSize.set(1024, 1024);
sun.shadow.camera.left = -20;
sun.shadow.camera.right = 20;
sun.shadow.camera.top = 20;
sun.shadow.camera.bottom = -20;
scene.add(sun);

const world = new THREE.Group();
scene.add(world);

function mat(color, roughness = 0.75) {
  return new THREE.MeshStandardMaterial({ color, roughness, metalness: 0.02 });
}

const water = new THREE.Mesh(
  new THREE.CircleGeometry(34, 96),
  new THREE.MeshStandardMaterial({ color: 0x2d96c7, roughness: 0.28, metalness: 0.05 }),
);
water.rotation.x = -Math.PI / 2;
water.position.y = -0.36;
world.add(water);

const island = new THREE.Mesh(
  new THREE.CylinderGeometry(9.4, 11.5, 1.1, 9),
  mat(0x70bd62),
);
island.receiveShadow = true;
island.castShadow = true;
world.add(island);

const beach = new THREE.Mesh(new THREE.CylinderGeometry(10.5, 11.9, 0.18, 9), mat(0xe6c878));
beach.position.y = -0.44;
beach.receiveShadow = true;
world.add(beach);

let buildingCount = 0;
let treeCount = 0;

const buildingMats = [mat(0xf6efe2), mat(0xf7c66d), mat(0x82cef0), mat(0xff9c92)];
for (let x = -5; x <= 5; x += 2.2) {
  for (let z = -4; z <= 4; z += 2.2) {
    if (Math.hypot(x, z) > 6.9 || Math.random() < 0.22) continue;
    const h = 0.9 + Math.random() * 2.3;
    const body = new THREE.Mesh(new THREE.BoxGeometry(1.15, h, 1.15), buildingMats[buildingCount % buildingMats.length]);
    body.position.set(x, h / 2 + 0.12, z);
    body.castShadow = true;
    body.receiveShadow = true;
    world.add(body);
    const roof = new THREE.Mesh(new THREE.ConeGeometry(0.95, 0.55, 4), mat(0xc45648));
    roof.position.set(x, h + 0.62, z);
    roof.rotation.y = Math.PI / 4;
    roof.castShadow = true;
    world.add(roof);
    buildingCount++;
  }
}

function addTree(x, z, s = 1) {
  const trunk = new THREE.Mesh(new THREE.CylinderGeometry(0.09 * s, 0.12 * s, 0.7 * s, 6), mat(0x7b5634));
  trunk.position.set(x, 0.44 * s, z);
  trunk.castShadow = true;
  world.add(trunk);
  const crown = new THREE.Mesh(new THREE.ConeGeometry(0.48 * s, 1.2 * s, 7), mat(0x2f9b58));
  crown.position.set(x, 1.18 * s, z);
  crown.castShadow = true;
  world.add(crown);
  treeCount++;
}

for (let i = 0; i < 46; i++) {
  const a = Math.random() * Math.PI * 2;
  const r = 6.8 + Math.random() * 2.4;
  addTree(Math.cos(a) * r, Math.sin(a) * r, 0.82 + Math.random() * 0.5);
}

function addBoat(a, r, color) {
  const boat = new THREE.Group();
  const hull = new THREE.Mesh(new THREE.BoxGeometry(1.8, 0.32, 0.55), mat(color));
  hull.castShadow = true;
  boat.add(hull);
  const sail = new THREE.Mesh(new THREE.ConeGeometry(0.38, 1.4, 3), mat(0xffffff));
  sail.position.set(0.18, 0.86, 0);
  sail.rotation.z = -0.2;
  boat.add(sail);
  boat.position.set(Math.cos(a) * r, 0.04, Math.sin(a) * r);
  boat.rotation.y = -a + Math.PI / 2;
  boat.userData = { a, r, speed: 0.08 + Math.random() * 0.05 };
  world.add(boat);
  return boat;
}
const boats = [addBoat(0.2, 15, 0xff795f), addBoat(2.3, 18, 0xf7c66d), addBoat(4.5, 13, 0x82cef0)];

document.getElementById('buildings').textContent = buildingCount;
document.getElementById('trees').textContent = treeCount;

const pointer = { down: false, x: 0, y: 0, yaw: 0.68, pitch: 0.56, distance: 28 };
canvas.addEventListener('pointerdown', e => {
  pointer.down = true;
  pointer.x = e.clientX;
  pointer.y = e.clientY;
  canvas.setPointerCapture(e.pointerId);
});
canvas.addEventListener('pointermove', e => {
  if (!pointer.down) return;
  pointer.yaw -= (e.clientX - pointer.x) * 0.006;
  pointer.pitch = Math.max(0.18, Math.min(0.95, pointer.pitch - (e.clientY - pointer.y) * 0.004));
  pointer.x = e.clientX;
  pointer.y = e.clientY;
});
canvas.addEventListener('pointerup', e => {
  pointer.down = false;
  canvas.releasePointerCapture(e.pointerId);
});
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  pointer.distance = Math.max(16, Math.min(42, pointer.distance + e.deltaY * 0.018));
}, { passive: false });

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
  water.position.y = -0.38 + Math.sin(t * 1.5) * 0.025;
  water.rotation.z += 0.0008;
  boats.forEach(boat => {
    boat.userData.a += boat.userData.speed * 0.01;
    boat.position.set(Math.cos(boat.userData.a) * boat.userData.r, 0.05 + Math.sin(t * 2) * 0.04, Math.sin(boat.userData.a) * boat.userData.r);
    boat.rotation.y = -boat.userData.a + Math.PI / 2;
  });

  const cp = Math.cos(pointer.pitch);
  camera.position.set(
    Math.sin(pointer.yaw) * pointer.distance * cp,
    Math.sin(pointer.pitch) * pointer.distance,
    Math.cos(pointer.yaw) * pointer.distance * cp,
  );
  camera.lookAt(0, 1.2, 0);
  renderer.render(scene, camera);
  requestAnimationFrame(animate);
}
animate();
