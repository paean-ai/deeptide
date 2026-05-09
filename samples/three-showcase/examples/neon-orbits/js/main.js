import * as THREE from '../../../shared/three.module.js';

const canvas = document.getElementById('scene');
const scene = new THREE.Scene();
scene.fog = new THREE.FogExp2(0x03050b, 0.035);

const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false, preserveDrawingBuffer: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
renderer.setClearColor(0x03050b, 1);
renderer.outputColorSpace = THREE.SRGBColorSpace;

const camera = new THREE.PerspectiveCamera(55, 1, 0.1, 120);
camera.position.set(0, 8, 18);

const root = new THREE.Group();
scene.add(root);

scene.add(new THREE.AmbientLight(0x7aa6ff, 0.38));
const key = new THREE.PointLight(0x74f7ff, 80, 40);
key.position.set(0, 3, 0);
scene.add(key);

const core = new THREE.Mesh(
  new THREE.IcosahedronGeometry(1.65, 3),
  new THREE.MeshStandardMaterial({
    color: 0x9df7ff,
    emissive: 0x37d9ff,
    emissiveIntensity: 1.6,
    roughness: 0.25,
    metalness: 0.55,
  }),
);
root.add(core);

const planets = [];
const palette = [0xff5d8f, 0xf6d36b, 0x70f28f, 0x74f7ff, 0xaa7dff, 0xff8f5a, 0xeaf7ff];
for (let i = 0; i < 7; i++) {
  const orbit = new THREE.Group();
  orbit.userData.speed = 0.15 + i * 0.035;
  orbit.rotation.x = -0.42 + i * 0.13;
  orbit.rotation.z = i * 0.32;
  const radius = 3.1 + i * 1.05;

  const ring = new THREE.Mesh(
    new THREE.TorusGeometry(radius, 0.012, 8, 160),
    new THREE.MeshBasicMaterial({ color: palette[i], transparent: true, opacity: 0.55 }),
  );
  orbit.add(ring);

  const planet = new THREE.Mesh(
    new THREE.SphereGeometry(0.24 + i * 0.025, 20, 16),
    new THREE.MeshStandardMaterial({
      color: palette[i],
      emissive: palette[i],
      emissiveIntensity: 0.55,
      roughness: 0.32,
      metalness: 0.25,
    }),
  );
  planet.position.x = radius;
  planet.userData.base = planet.scale.x;
  planet.userData.pulse = 0;
  orbit.add(planet);
  root.add(orbit);
  planets.push({ orbit, planet, radius });
}

const starCount = 900;
const starPositions = new Float32Array(starCount * 3);
for (let i = 0; i < starCount; i++) {
  const r = 20 + Math.random() * 45;
  const a = Math.random() * Math.PI * 2;
  const h = (Math.random() - 0.5) * 28;
  starPositions[i * 3] = Math.cos(a) * r;
  starPositions[i * 3 + 1] = h;
  starPositions[i * 3 + 2] = Math.sin(a) * r;
}
const starGeo = new THREE.BufferGeometry();
starGeo.setAttribute('position', new THREE.BufferAttribute(starPositions, 3));
const stars = new THREE.Points(starGeo, new THREE.PointsMaterial({
  color: 0xa9f6ff,
  size: 0.055,
  transparent: true,
  opacity: 0.75,
}));
scene.add(stars);
document.getElementById('particles').textContent = starCount;

let paused = false;
document.getElementById('toggle').onclick = () => {
  paused = !paused;
  document.getElementById('toggle').textContent = paused ? 'Resume' : 'Pause';
};

const pointer = { down: false, x: 0, y: 0, yaw: 0, pitch: 0.32, distance: 18 };
canvas.addEventListener('pointerdown', e => {
  pointer.down = true;
  pointer.x = e.clientX;
  pointer.y = e.clientY;
  canvas.setPointerCapture(e.pointerId);
});
canvas.addEventListener('pointermove', e => {
  if (!pointer.down) return;
  pointer.yaw -= (e.clientX - pointer.x) * 0.006;
  pointer.pitch = Math.max(-0.6, Math.min(0.95, pointer.pitch - (e.clientY - pointer.y) * 0.005));
  pointer.x = e.clientX;
  pointer.y = e.clientY;
});
canvas.addEventListener('pointerup', e => {
  pointer.down = false;
  canvas.releasePointerCapture(e.pointerId);
});
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  pointer.distance = Math.max(8, Math.min(34, pointer.distance + e.deltaY * 0.012));
}, { passive: false });
canvas.addEventListener('click', () => {
  const p = planets[Math.floor(Math.random() * planets.length)].planet;
  p.userData.pulse = 1;
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
  const dt = Math.min(0.05, clock.getDelta());
  if (!paused) {
    const t = clock.elapsedTime;
    core.rotation.x += dt * 0.35;
    core.rotation.y += dt * 0.52;
    root.rotation.y += dt * 0.05;
    stars.rotation.y -= dt * 0.025;
    planets.forEach((entry, i) => {
      entry.orbit.rotation.y += dt * entry.orbit.userData.speed;
      entry.planet.rotation.y += dt * 1.5;
      entry.planet.userData.pulse *= 0.9;
      const pulse = entry.planet.userData.pulse;
      const s = 1 + pulse * 1.2 + Math.sin(t * 2.4 + i) * 0.04;
      entry.planet.scale.setScalar(s);
    });
  }

  const cp = Math.cos(pointer.pitch);
  camera.position.set(
    Math.sin(pointer.yaw) * pointer.distance * cp,
    Math.sin(pointer.pitch) * pointer.distance + 4,
    Math.cos(pointer.yaw) * pointer.distance * cp,
  );
  camera.lookAt(0, 0, 0);
  renderer.render(scene, camera);
  requestAnimationFrame(animate);
}
animate();
