// Import the WASM module
import('./pkg')
  .then(async wasm => {
    const loadingEl = document.getElementById('loading');
    const canvasEl  = document.getElementById('canvas');
    const hitsEl    = document.getElementById('hit-indices');

    loadingEl.style.display = 'block';
    canvasEl.style.display  = 'none';
    hitsEl.style.display    = 'none';

    const engine = new wasm.Rust2DEngine("canvas");
    const width  = canvasEl.width;
    const height = canvasEl.height;

    const totalObjects = 1_000;
    const size         = 100;
    const keyframesPer = 1000_000;
    
    // 객체 생성 방식 선택 변수
    const useLazyObject = true; // true: 지연 로딩 사용, false: 일반 객체 사용

    const startTime = performance.now();

    for (let idx = 0; idx < totalObjects; idx++) {
      const color = '#' + Math.floor(Math.random() * 0xFFFFFF)
        .toString(16)
        .padStart(6, '0');
        
      if (useLazyObject) {
        // 지연 로딩 객체 생성 - KeyframeStore 사용
        const chunkSize = 5000; // 청크 크기 (밀리초)
        const totalDuration = keyframesPer * 5000; // 전체 애니메이션 시간
        
        // 키프레임 패턴 생성 함수
        const patternFn = (startTime, endTime) => {
          const frames = [];
          let t = startTime;
          while (t < endTime) {
            const x = Math.random() * (width - size);
            const y = Math.random() * (height - size);
            frames.push([t, x, y]);
            t += Math.random() * 5000;
          }
          return frames;
        };
        
        // KeyframeStore 생성 및 객체 추가
        const keyframeStore = new wasm.KeyframeStore(chunkSize, totalDuration, patternFn);
        engine.add_lazy_object(keyframeStore, size, color);
      } 
      else {
        // 일반 객체 생성 - 모든 키프레임 미리 로드
        const keyframes = [];
        let t = 0;
        for (let j = 0; j < keyframesPer; j++) {
          t += Math.random() * 5000;
          const x = Math.random() * (width - size);
          const y = Math.random() * (height - size);
          keyframes.push(new wasm.Keyframe(t, x, y));
        }
        
        // 일반 객체 추가
        engine.add_object(keyframes, size, color);
      }
    }

    const endTime = performance.now();
    const timeElapsed = (endTime - startTime) / 1000; // 초 단위로 변환
    console.log(`Total initialization time: ${timeElapsed.toFixed(2)} seconds`);
    console.log(`Using lazy objects: ${useLazyObject}`);

    loadingEl.style.display = 'none';
    canvasEl.style.display = 'block';
    hitsEl.style.display = 'block';

    displayMemoryUsage();

    // Start the engine loop
    engine.run();
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