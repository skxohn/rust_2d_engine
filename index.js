// Import the WASM module
import('./pkg')
  .then(async wasm => {
    const loadingEl = document.getElementById('loading');
    const canvasEl  = document.getElementById('canvas');
    const hitsEl    = document.getElementById('hit-indices');

    loadingEl.style.display = 'block';
    canvasEl.style.display  = 'none';
    hitsEl.style.display    = 'none';

    const engine = await new wasm.Rust2DEngine("canvas");
    const width  = canvasEl.width;
    const height = canvasEl.height;

    const totalObjects = 10;
    const size         = 100;
    const keyframesPer = 1000;
    
    const startTime = performance.now();

    try {
        await engine.generate_objects(
          totalObjects, 
          keyframesPer, 
          size
        );
        
        const endTime = performance.now();
        const timeElapsed = (endTime - startTime) / 1000; // 초 단위로 변환
        console.log(`Total initialization time: ${timeElapsed.toFixed(2)} seconds`);
        console.log(`Created ${totalObjects} objects with ${keyframesPer} frames each`);

        loadingEl.style.display = 'none';
        canvasEl.style.display = 'block';
        hitsEl.style.display = 'block';

        displayMemoryUsage();

        // Start the engine loop
        engine.run();
      } catch (error) {
        console.error("Error generating objects:", error);
        loadingEl.textContent = 'Failed to generate objects!';
      }

  })
  .catch(err => {
    console.error(err);
    const loadingEl = document.getElementById('loading');
    loadingEl.textContent = 'Failed to load!';
  });

function displayMemoryUsage() {
  if (window.performance && window.performance.memory) {
    const memoryInfo = window.performance.memory;
    const memoryUsed = Math.round(memoryInfo.usedJSHeapSize / (1024 * 1024));
    const memoryLimit = Math.round(memoryInfo.jsHeapSizeLimit / (1024 * 1024));

    console.log(`Memory: ${memoryUsed}MB / ${memoryLimit}MB`);
  }
}
