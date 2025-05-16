// Import the WASM module
import('./pkg')
  .then(async wasm => {
    const loadingEl = document.getElementById('loading');
    const canvasEl = document.getElementById('canvas');

    const engine = new wasm.Rust2DEngine("canvas");

    const width = canvasEl.width;
    const height = canvasEl.height;

    const size = 100;
    const totalObjects = 1000;

    // Generate and add objects, updating loading percentage
    for (let idx = 0; idx < totalObjects; idx++) {
      // Build random keyframes for this object
      const keyframes = [];
      let t = 0;
      for (let j = 0; j < 1000; j++) {
        t += Math.random() * 5000;
        const x = Math.random() * (width - size);
        const y = Math.random() * (height - size);
        keyframes.push(new wasm.Keyframe(t, x, y));
      }
      // Random color
      const color = '#' + Math.floor(Math.random() * 0xFFFFFF)
        .toString(16)
        .padStart(6, '0');

      // Add to engine
      engine.add_object(keyframes, size, color);

      // Update loading percentage
      const pct = Math.floor(((idx + 1) / totalObjects) * 100);
      loadingEl.textContent = `Loading... ${pct}%`;
    }

    // Generation complete: hide loader, show canvas, and reveal hit-indices area
    loadingEl.style.display = 'none';
    canvasEl.style.display = 'block';
    const hitsEl = document.getElementById('hit-indices');
    hitsEl.style.display = 'block';

    await debug();

    // Start the engine loop
    engine.run();
  })
  .catch(err => {
    console.error(err);
    const loadingEl = document.getElementById('loading');
    loadingEl.textContent = 'Failed to load!';
  });

async function debug() {
  let lastTime = performance.now();

  function animate() {
    const currentTime = performance.now();
    const deltaTime = (currentTime - lastTime) / 1000; // 초 단위로 변환
    lastTime = currentTime;

    // 메모리 사용량 표시 (디버깅용)
    displayMemoryUsage();

    requestAnimationFrame(animate);
  }

  function displayMemoryUsage() {
    if (window.performance && window.performance.memory) {
      const memoryInfo = window.performance.memory;
      const memoryUsed = Math.round(memoryInfo.usedJSHeapSize / (1024 * 1024));
      const memoryLimit = Math.round(memoryInfo.jsHeapSizeLimit / (1024 * 1024));

      console.log(`Memory: ${memoryUsed}MB / ${memoryLimit}MB`);
    }
  }
  animate();
}