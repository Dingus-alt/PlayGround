// Terraforge browser shell.
//
// The Rust/WebAssembly module owns the entire simulation and draws through a
// tiny immediate-mode Canvas2D API. This file owns the canvas, input, audio,
// saves and the DOM title screen.

const canvas = document.getElementById('game');
const ctx = canvas.getContext('2d', { alpha: false, desynchronized: true });
const titleEl = document.getElementById('title');
const loadingEl = document.getElementById('loading');
const errorEl = document.getElementById('error');
const saveNote = document.getElementById('save-note');
const playBtn = document.getElementById('play');
const newWorldBtn = document.getElementById('new-world');

const SAVE_KEY = 'terraforge.save.v1';
const MUTE_KEY = 'terraforge.muted.v1';
const FONT_STACK = '"Trebuchet MS", "Segoe UI", system-ui, -apple-system, sans-serif';

let wasm = null;
let memory = null;
let viewW = 640;
let viewH = 360;
let pixelScale = 1;
let dpr = 1;
let cssW = 1;
let cssH = 1;
let touchMode = false;
let titleOpen = true;
let running = false;
let lastTime = 0;
let frameCount = 0;
let pointerId = null;
let loadBytes = new Uint8Array(0);
let wasmReady = false;

// ---------------------------------------------------------------------------
// Color + geometry bridge
// ---------------------------------------------------------------------------

const colorCache = new Map();
function css(color) {
  const c = color >>> 0;
  let s = colorCache.get(c);
  if (s === undefined) {
    const r = (c >>> 24) & 255;
    const g = (c >>> 16) & 255;
    const b = (c >>> 8) & 255;
    const a = c & 255;
    s = a === 255 ? `rgb(${r},${g},${b})` : `rgba(${r},${g},${b},${(a / 255).toFixed(4)})`;
    if (colorCache.size > 4096) colorCache.clear();
    colorCache.set(c, s);
  }
  return s;
}

function clearCss(color) {
  const c = color >>> 0;
  return `rgba(${(c >>> 24) & 255},${(c >>> 16) & 255},${(c >>> 8) & 255},${(c & 255) / 255})`;
}

function str(ptr, len) {
  if (!len) return '';
  return new TextDecoder().decode(new Uint8Array(memory.buffer, ptr >>> 0, len >>> 0));
}

function fontFor(size, weight) {
  return `${weight || 500} ${size}px ${FONT_STACK}`;
}

// ---------------------------------------------------------------------------
// Minimap texture
// ---------------------------------------------------------------------------

let minimapCanvas = document.createElement('canvas');
let minimapCtx = minimapCanvas.getContext('2d');
let minimapW = 0;
let minimapH = 0;

function uploadMinimap(ptr, w, h) {
  if (!memory || w === 0 || h === 0) return;
  if (minimapCanvas.width !== w || minimapCanvas.height !== h) {
    minimapCanvas.width = w;
    minimapCanvas.height = h;
    minimapW = w;
    minimapH = h;
  }
  const size = w * h * 4;
  const src = new Uint8ClampedArray(memory.buffer, ptr >>> 0, size);
  minimapCtx.putImageData(new ImageData(src, w, h), 0, 0);
}

// ---------------------------------------------------------------------------
// Audio: tiny procedural synth. No assets, no network requests.
// ---------------------------------------------------------------------------

const audio = {
  ctx: null,
  master: null,
  sfxBus: null,
  musicBus: null,
  noiseBuf: null,
  mode: 0,
  step: 0,
  nextNote: 0,
  timer: null,
  muted: false,
  gesture: false,
};

const SCALES = {
  1: [0, 2, 4, 7, 9, 12, 16],        // day — bright pentatonic
  2: [0, 3, 5, 7, 10, 12, 15],       // night — minor pentatonic
  3: [0, 1, 5, 6, 7, 12, 13],        // boss — tense
  4: [0, 4, 7, 11, 14, 19],          // title — airy
  5: [0, 4, 7, 12, 16, 19, 24],      // victory — major fanfare
};

const MODE_ROOT = { 1: 220, 2: 146.83, 3: 110, 4: 196, 5: 261.63 };
const MODE_BEAT = { 1: 0.46, 2: 0.62, 3: 0.26, 4: 0.7, 5: 0.36 };

function initAudio() {
  if (audio.ctx) {
    if (audio.ctx.state === 'suspended') audio.ctx.resume().catch(() => {});
    return;
  }
  const Ctx = window.AudioContext || window.webkitAudioContext;
  if (!Ctx) return;
  try {
    audio.ctx = new Ctx();
  } catch (_) {
    return;
  }
  const ac = audio.ctx;
  audio.master = ac.createGain();
  audio.master.gain.value = audio.muted ? 0 : 0.8;
  audio.master.connect(ac.destination);
  audio.sfxBus = ac.createGain();
  audio.sfxBus.gain.value = 0.55;
  audio.sfxBus.connect(audio.master);
  audio.musicBus = ac.createGain();
  audio.musicBus.gain.value = 0.15;
  audio.musicBus.connect(audio.master);

  const len = Math.floor(ac.sampleRate * 0.9);
  audio.noiseBuf = ac.createBuffer(1, len, ac.sampleRate);
  const data = audio.noiseBuf.getChannelData(0);
  let seed = 0x1234567;
  for (let i = 0; i < len; i++) {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    data[i] = (seed / 0xffffffff) * 2 - 1;
  }

  if (audio.mode) startMusic(audio.mode);
}

function resumeAudio() {
  audio.gesture = true;
  initAudio();
  if (audio.ctx && audio.ctx.state === 'suspended') {
    audio.ctx.resume().catch(() => {});
  }
  if (audio.mode) startMusic(audio.mode);
}

function tone(freq, dur, type, gain, slideTo, when, bus) {
  const ac = audio.ctx;
  if (!ac) return;
  const t = when || ac.currentTime;
  const osc = ac.createOscillator();
  const g = ac.createGain();
  osc.type = type || 'sine';
  osc.frequency.setValueAtTime(Math.max(20, freq), t);
  if (slideTo) osc.frequency.exponentialRampToValueAtTime(Math.max(20, slideTo), t + dur);
  g.gain.setValueAtTime(0.0001, t);
  g.gain.exponentialRampToValueAtTime(Math.max(0.0002, gain), t + 0.008);
  g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  osc.connect(g);
  g.connect(bus || audio.sfxBus);
  osc.start(t);
  osc.stop(t + dur + 0.02);
}

function noise(dur, gain, cutoff, when) {
  const ac = audio.ctx;
  if (!ac || !audio.noiseBuf) return;
  const t = when || ac.currentTime;
  const src = ac.createBufferSource();
  src.buffer = audio.noiseBuf;
  const filter = ac.createBiquadFilter();
  filter.type = 'lowpass';
  filter.frequency.setValueAtTime(cutoff || 1800, t);
  filter.frequency.exponentialRampToValueAtTime(Math.max(120, (cutoff || 1800) * 0.35), t + dur);
  const g = ac.createGain();
  g.gain.setValueAtTime(gain, t);
  g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  src.connect(filter);
  filter.connect(g);
  g.connect(audio.sfxBus);
  src.start(t);
  src.stop(t + dur + 0.02);
}

function playSfx(id, volume) {
  if (!audio.ctx || audio.ctx.state !== 'running' || audio.muted) return;
  const ac = audio.ctx;
  const t = ac.currentTime + 0.004;
  const v = Math.max(0, Math.min(1, volume == null ? 0.7 : volume));
  switch (id) {
    case 1: // jump
      tone(320, 0.16, 'triangle', 0.22 * v, 660, t);
      break;
    case 2: // swing
      noise(0.11, 0.16 * v, 2600, t);
      tone(180, 0.1, 'sine', 0.08 * v, 120, t);
      break;
    case 3: // hit
      tone(140, 0.14, 'square', 0.14 * v, 74, t);
      noise(0.1, 0.2 * v, 900, t);
      break;
    case 4: // mining tick
      noise(0.035, 0.09 * v, 3200, t);
      break;
    case 5: // block breaks
      noise(0.19, 0.26 * v, 2100, t);
      tone(110, 0.13, 'triangle', 0.1 * v, 60, t);
      break;
    case 6: // place block
      tone(170, 0.1, 'sine', 0.16 * v, 110, t);
      noise(0.05, 0.08 * v, 700, t);
      break;
    case 7: // pickup
      tone(660, 0.09, 'sine', 0.13 * v, 990, t);
      tone(990, 0.08, 'sine', 0.08 * v, 1320, t + 0.06);
      break;
    case 8: // hurt
      tone(260, 0.2, 'sawtooth', 0.15 * v, 90, t);
      noise(0.16, 0.1 * v, 1200, t);
      break;
    case 9: // death
      tone(300, 0.75, 'sawtooth', 0.17 * v, 55, t);
      tone(150, 0.85, 'triangle', 0.12 * v, 40, t + 0.05);
      break;
    case 10: // explosion
      noise(0.55, 0.5 * v, 1100, t);
      tone(90, 0.5, 'sine', 0.3 * v, 32, t);
      break;
    case 11: // craft
      tone(523, 0.1, 'triangle', 0.12 * v, 523, t);
      tone(784, 0.12, 'triangle', 0.12 * v, 784, t + 0.08);
      tone(1046, 0.16, 'triangle', 0.1 * v, 1046, t + 0.16);
      break;
    case 12: // heal
      [523, 659, 784, 1046].forEach((f, i) => tone(f, 0.16, 'sine', 0.11 * v, f, t + i * 0.07));
      break;
    case 13: // click
      tone(880, 0.045, 'square', 0.055 * v, 660, t);
      break;
    case 14: // boss roar
      tone(70, 1.1, 'sawtooth', 0.26 * v, 40, t);
      tone(105, 1.0, 'square', 0.1 * v, 48, t + 0.1);
      noise(0.8, 0.14 * v, 500, t);
      break;
    case 15: // victory
      [523, 659, 784, 1046, 1318].forEach((f, i) =>
        tone(f, i === 4 ? 0.7 : 0.22, 'triangle', 0.14 * v, f, t + i * 0.13)
      );
      break;
    case 16: // splash
      noise(0.3, 0.2 * v, 700, t);
      tone(400, 0.2, 'sine', 0.06 * v, 180, t);
      break;
    case 17: // banner / day change
      tone(392, 0.3, 'sine', 0.09 * v, 392, t);
      tone(587, 0.35, 'sine', 0.07 * v, 587, t + 0.12);
      break;
    default:
      break;
  }
}

function startMusic(mode) {
  audio.mode = mode;
  if (!audio.ctx || audio.ctx.state !== 'running') return;
  if (audio.timer !== null) return;
  audio.nextNote = audio.ctx.currentTime + 0.08;
  audio.step = 0;
  audio.timer = setInterval(musicScheduler, 180);
}

function stopMusic() {
  if (audio.timer !== null) {
    clearInterval(audio.timer);
    audio.timer = null;
  }
}

function setMusic(mode) {
  const changed = audio.mode !== mode;
  audio.mode = mode;
  if (!audio.ctx) return;
  if (mode === 0) {
    stopMusic();
    return;
  }
  if (changed) {
    audio.step = 0;
    audio.nextNote = audio.ctx.currentTime + 0.1;
  }
  startMusic(mode);
}

function musicScheduler() {
  const ac = audio.ctx;
  if (!ac) return;
  if (ac.state !== 'running' || audio.mode === 0 || audio.muted) {
    if (audio.mode === 0) stopMusic();
    return;
  }
  const beat = MODE_BEAT[audio.mode] || 0.5;
  while (audio.nextNote < ac.currentTime + 0.45) {
    scheduleBeat(audio.mode, audio.step, audio.nextNote, beat);
    audio.step++;
    audio.nextNote += beat;
  }
}

function scheduleBeat(mode, step, when, beat) {
  const scale = SCALES[mode] || SCALES[1];
  const root = MODE_ROOT[mode] || 220;
  const bus = audio.musicBus;
  const n = scale[(step * 3 + Math.floor(step / 4)) % scale.length];
  const octave = step % 8 === 7 ? 12 : 0;
  const freq = root * Math.pow(2, (n + octave) / 12);
  // Sparse, gentle melody.
  if (mode === 3) {
    tone(freq, beat * 0.85, 'square', 0.05, freq * 0.94, when, bus);
    if (step % 2 === 0) tone(root * 0.5, beat * 1.6, 'sawtooth', 0.06, root * 0.47, when, bus);
    if (step % 4 === 2) noise(0.1, 0.05, 1800, when);
  } else {
    if (step % 2 === 0) tone(freq, beat * 0.95, 'triangle', 0.075, freq, when, bus);
    if (step % 4 === 0) tone(root * 0.5, beat * 1.7, 'sine', 0.07, root * 0.5, when, bus);
    if (step % 8 === 6) tone(freq * 1.5, beat * 0.8, 'sine', 0.045, freq * 1.5, when, bus);
  }
}

// ---------------------------------------------------------------------------
// Canvas render imports
// ---------------------------------------------------------------------------

const imports = {
  env: {
    js_rect(x, y, w, h, color) {
      ctx.fillStyle = css(color);
      ctx.fillRect(x, y, w, h);
    },
    js_rect_rot(cx, cy, w, h, angle, color) {
      ctx.save();
      ctx.translate(cx, cy);
      ctx.rotate(angle);
      ctx.fillStyle = css(color);
      ctx.fillRect(-w * 0.5, -h * 0.5, w, h);
      ctx.restore();
    },
    js_circle(x, y, r, color) {
      if (r <= 0) return;
      ctx.fillStyle = css(color);
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
    },
    js_ellipse(x, y, rx, ry, color) {
      if (rx <= 0 || ry <= 0) return;
      ctx.fillStyle = css(color);
      ctx.beginPath();
      ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
      ctx.fill();
    },
    js_ring(x, y, r, width, color) {
      if (r <= 0) return;
      ctx.strokeStyle = css(color);
      ctx.lineWidth = Math.max(0.4, width);
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.stroke();
    },
    js_line(x1, y1, x2, y2, width, color) {
      ctx.strokeStyle = css(color);
      ctx.lineWidth = Math.max(0.35, width);
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
    },
    js_tri(x1, y1, x2, y2, x3, y3, color) {
      ctx.fillStyle = css(color);
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.lineTo(x3, y3);
      ctx.closePath();
      ctx.fill();
    },
    js_poly(ptr, count, color) {
      if (!count || !memory) return;
      const flat = new Float32Array(memory.buffer, ptr >>> 0, count * 2);
      ctx.fillStyle = css(color);
      ctx.beginPath();
      ctx.moveTo(flat[0], flat[1]);
      for (let i = 1; i < count; i++) ctx.lineTo(flat[i * 2], flat[i * 2 + 1]);
      ctx.closePath();
      ctx.fill();
    },
    js_text(x, y, size, color, align, weight, ptr, len) {
      if (!len || size <= 0.1) return;
      const s = str(ptr, len);
      ctx.font = fontFor(size, weight);
      ctx.fillStyle = css(color);
      ctx.textAlign = align === 1 ? 'center' : align === 2 ? 'right' : 'left';
      ctx.textBaseline = 'middle';
      ctx.fillText(s, x, y);
    },
    js_text_width(ptr, len, size, weight) {
      if (!len) return 0;
      ctx.font = fontFor(size, weight);
      return ctx.measureText(str(ptr, len)).width;
    },
    js_gradient(x, y, w, h, c1, c2, vertical) {
      const g = vertical
        ? ctx.createLinearGradient(x, y, x, y + h)
        : ctx.createLinearGradient(x, y, x + w, y);
      g.addColorStop(0, clearCss(c1));
      g.addColorStop(1, clearCss(c2));
      ctx.fillStyle = g;
      ctx.fillRect(x, y, w, h);
    },
    js_glow(x, y, r, color) {
      if (r <= 0) return;
      const c = color >>> 0;
      const rr = (c >>> 24) & 255;
      const gg = (c >>> 16) & 255;
      const bb = (c >>> 8) & 255;
      const aa = (c & 255) / 255;
      const g = ctx.createRadialGradient(x, y, 0, x, y, r);
      g.addColorStop(0, `rgba(${rr},${gg},${bb},${aa})`);
      g.addColorStop(0.45, `rgba(${rr},${gg},${bb},${aa * 0.5})`);
      g.addColorStop(1, `rgba(${rr},${gg},${bb},0)`);
      const old = ctx.globalCompositeOperation;
      ctx.globalCompositeOperation = 'lighter';
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalCompositeOperation = old;
    },
    js_minimap(ptr, w, h) {
      uploadMinimap(ptr, w, h);
    },
    js_minimap_draw(x, y, w, h, alpha) {
      if (!minimapW) return;
      ctx.save();
      ctx.globalAlpha = Math.max(0, Math.min(1, alpha));
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(minimapCanvas, x, y, w, h);
      ctx.restore();
    },
    js_vignette(strength) {
      const s = Math.max(0, Math.min(1.2, strength));
      const cx = viewW * 0.5;
      const cy = viewH * 0.5;
      const inner = Math.min(viewW, viewH) * 0.28;
      const outer = Math.max(viewW, viewH) * 0.72;
      const g = ctx.createRadialGradient(cx, cy, inner, cx, cy, outer);
      g.addColorStop(0, 'rgba(0,0,0,0)');
      g.addColorStop(0.62, `rgba(0,0,0,${0.06 * s})`);
      g.addColorStop(1, `rgba(0,0,0,${0.62 * s})`);
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, viewW, viewH);
    },
    js_sfx(id, volume) {
      playSfx(id, volume);
    },
    js_music(mode) {
      setMusic(mode);
    },
    js_save(ptr, len) {
      try {
        const text = str(ptr, len);
        localStorage.setItem(SAVE_KEY, text);
        localStorage.setItem(SAVE_KEY + '.time', String(Date.now()));
        updateSaveNote(true);
      } catch (err) {
        console.warn('Terraforge: could not save', err);
      }
    },
    js_load_len() {
      try {
        const text = localStorage.getItem(SAVE_KEY);
        loadBytes = text ? new TextEncoder().encode(text) : new Uint8Array(0);
      } catch (_) {
        loadBytes = new Uint8Array(0);
      }
      return loadBytes.length;
    },
    js_load_read(ptr, len) {
      const n = Math.min(len, loadBytes.length);
      if (n > 0) new Uint8Array(memory.buffer, ptr >>> 0, n).set(loadBytes.subarray(0, n));
      return n;
    },
    js_show_title() {
      openTitle();
    },
    js_mute(muted) {
      audio.muted = !!muted;
      if (audio.master) {
        audio.master.gain.setTargetAtTime(audio.muted ? 0 : 0.8, audio.ctx.currentTime, 0.03);
      }
      const btn = document.getElementById('m-mute');
      if (btn) {
        btn.textContent = audio.muted ? '🔇' : '♪';
        btn.classList.toggle('on', audio.muted);
      }
      try {
        localStorage.setItem(MUTE_KEY, audio.muted ? '1' : '0');
      } catch (_) {}
    },
    js_log(ptr, len) {
      console.log('[terraforge]', str(ptr, len));
    },
  },
};

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

function showError(err) {
  console.error(err);
  loadingEl.classList.add('hidden');
  errorEl.hidden = false;
  errorEl.textContent =
    'Terraforge failed to start.\n\n' +
    (err && err.message ? err.message : String(err)) +
    '\n\nIf you opened index.html directly, serve the folder over HTTP instead (./start.sh).';
}

function resize() {
  cssW = Math.max(1, window.innerWidth);
  cssH = Math.max(1, window.innerHeight);
  dpr = Math.min(window.devicePixelRatio || 1, 2);

  const aspect = cssW / cssH;
  if (aspect >= 1) {
    viewH = 360;
    viewW = 360 * aspect;
  } else {
    viewW = 300;
    viewH = 300 / aspect;
  }
  // Very tall phones get a little extra horizontal breathing room.
  if (aspect < 0.55) {
    viewW = 280;
    viewH = 280 / aspect;
  }

  canvas.width = Math.max(1, Math.round(cssW * dpr));
  canvas.height = Math.max(1, Math.round(cssH * dpr));
  pixelScale = canvas.width / viewW;

  ctx.setTransform(pixelScale, 0, 0, pixelScale, 0, 0);
  ctx.lineJoin = 'round';
  ctx.lineCap = 'round';
  ctx.imageSmoothingEnabled = true;

  if (wasmReady) wasm.game_resize(viewW, viewH);
}

function screenToView(clientX, clientY) {
  const r = canvas.getBoundingClientRect();
  return {
    x: ((clientX - r.left) / Math.max(1, r.width)) * viewW,
    y: ((clientY - r.top) / Math.max(1, r.height)) * viewH,
  };
}

function setTouchMode(on) {
  touchMode = !!on;
  document.body.classList.toggle('touch', touchMode);
  if (wasmReady) wasm.game_set_touch_mode(touchMode ? 1 : 0);
}

function updateSaveNote(hasSave) {
  if (!saveNote) return;
  let exists = hasSave;
  if (exists == null) {
    try {
      exists = !!localStorage.getItem(SAVE_KEY);
    } catch (_) {
      exists = false;
    }
  }
  saveNote.hidden = !exists;
}

function openTitle() {
  titleOpen = true;
  titleEl.classList.remove('hidden');
  document.body.classList.add('title-open');
  updateSaveNote();
  const has = wasmReady ? !!wasm.game_has_save() : false;
  updateSaveNote(has);
}

function hideTitle() {
  titleOpen = false;
  titleEl.classList.add('hidden');
  document.body.classList.remove('title-open');
}

function startGame(fresh) {
  if (!wasmReady) return;
  resumeAudio();
  hideTitle();
  if (fresh) {
    const seed = (Math.random() * 0xffffffff) >>> 0;
    wasm.game_new_world(seed);
  } else {
    wasm.game_play();
  }
  wasm.game_set_touch_mode(touchMode ? 1 : 0);
  if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
}

async function boot() {
  // Restore mute preference before the game asks for any sounds.
  try {
    audio.muted = localStorage.getItem(MUTE_KEY) === '1';
  } catch (_) {}

  resize();
  document.body.classList.add('title-open');
  updateSaveNote();

  const importsObj = imports;
  const wasmUrl = new URL('../pkg/terraforge.wasm', import.meta.url);
  let instance = null;
  try {
    const response = await fetch(wasmUrl);
    if (!response.ok) throw new Error(`Could not load ${wasmUrl.pathname} (${response.status})`);
    const contentType = response.headers.get('content-type') || '';
    if (WebAssembly.instantiateStreaming && contentType.includes('application/wasm')) {
      try {
        const result = await WebAssembly.instantiateStreaming(response.clone(), importsObj);
        instance = result.instance;
      } catch (streamErr) {
        console.warn('Streaming instantiation failed, falling back to ArrayBuffer', streamErr);
      }
    }
    if (!instance) {
      const bytes = await response.arrayBuffer();
      const result = await WebAssembly.instantiate(bytes, importsObj);
      instance = result.instance;
    }
  } catch (err) {
    showError(err);
    return;
  }

  wasm = instance.exports;
  memory = wasm.memory;
  wasmReady = true;

  const coarse = window.matchMedia && window.matchMedia('(pointer: coarse)').matches;
  const hasTouch = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
  setTouchMode(!!(coarse && hasTouch));

  const seed = (Math.random() * 0xffffffff) >>> 0;
  try {
    wasm.game_init(viewW, viewH, seed);
  } catch (err) {
    showError(err);
    return;
  }

  if (audio.muted) wasm.game_set_touch_mode(touchMode ? 1 : 0);
  openTitle();
  updateSaveNote(!!wasm.game_has_save());

  if (audio.muted) {
    // Keep the DOM mute button in sync without waiting for a toggle.
    const btn = document.getElementById('m-mute');
    if (btn) btn.textContent = '🔇';
  }

  running = true;
  lastTime = performance.now();
  requestAnimationFrame(frame);
}

function frame(now) {
  if (!running) return;
  requestAnimationFrame(frame);

  let dt = (now - lastTime) / 1000;
  lastTime = now;
  if (!isFinite(dt) || dt < 0) dt = 0;
  dt = Math.min(dt, 0.05);

  // Reset the transform and clear before the module paints the whole view.
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.setTransform(pixelScale, 0, 0, pixelScale, 0, 0);

  try {
    wasm.game_tick(dt);
  } catch (err) {
    running = false;
    showError(err);
    return;
  }

  frameCount++;
  if (frameCount === 2) {
    loadingEl.classList.add('hidden');
    setTimeout(() => loadingEl.remove(), 400);
    window.__terra.ready = true;
    window.dispatchEvent(new Event('terraforge-ready'));
  }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

const KEY = {
  KeyA: 0, KeyD: 1, KeyS: 2, KeyW: 3, Space: 4,
  ShiftLeft: 5, ShiftRight: 5, KeyE: 6, KeyC: 7, KeyM: 8, Escape: 9,
  Digit1: 10, Digit2: 11, Digit3: 12, Digit4: 13, Digit5: 14,
  Digit6: 15, Digit7: 16, Digit8: 17, Digit9: 18, Digit0: 19,
  KeyN: 20, Enter: 21,
  ArrowLeft: 22, ArrowRight: 23, ArrowUp: 24, ArrowDown: 25,
};

const PREVENT = new Set([
  'Space', 'Enter', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown',
  'KeyW', 'KeyA', 'KeyS', 'KeyD', 'ShiftLeft', 'ShiftRight', 'Tab',
]);

window.addEventListener('keydown', (e) => {
  if (!wasmReady) return;
  if (e.metaKey || e.ctrlKey || e.altKey) return;

  if (titleOpen) {
    if (e.code === 'Enter' || e.code === 'Space') {
      e.preventDefault();
      startGame(false);
    }
    return;
  }

  const code = KEY[e.code];
  if (code === undefined) return;
  if (PREVENT.has(e.code)) e.preventDefault();
  if (e.repeat) return;
  resumeAudio();
  wasm.game_key(code, 1);
});

window.addEventListener('keyup', (e) => {
  if (!wasmReady) return;
  const code = KEY[e.code];
  if (code === undefined) return;
  if (PREVENT.has(e.code)) e.preventDefault();
  wasm.game_key(code, 0);
});

window.addEventListener('blur', () => {
  if (!wasmReady) return;
  for (let i = 0; i <= 25; i++) wasm.game_key(i, 0);
  wasm.game_pointer_up(0);
  wasm.game_pointer_up(2);
});

canvas.addEventListener('contextmenu', (e) => e.preventDefault());

canvas.addEventListener('pointerdown', (e) => {
  if (!wasmReady || titleOpen) return;
  e.preventDefault();
  resumeAudio();
  if (e.pointerType === 'touch') {
    if (!touchMode) setTouchMode(true);
  }
  const p = screenToView(e.clientX, e.clientY);
  wasm.game_pointer_move(p.x, p.y);
  if (e.pointerType === 'mouse') {
    const button = e.button === 2 ? 2 : e.button === 1 ? 1 : 0;
    wasm.game_pointer_down(button);
  } else {
    if (pointerId !== null) wasm.game_pointer_up(0);
    pointerId = e.pointerId;
    wasm.game_pointer_down(0);
    try { canvas.setPointerCapture(e.pointerId); } catch (_) {}
  }
});

canvas.addEventListener('pointermove', (e) => {
  if (!wasmReady || titleOpen) return;
  const p = screenToView(e.clientX, e.clientY);
  wasm.game_pointer_move(p.x, p.y);
});

function releasePointer(e) {
  if (!wasmReady) return;
  if (e && e.pointerType === 'mouse') {
    const button = e.button === 2 ? 2 : e.button === 1 ? 1 : 0;
    wasm.game_pointer_up(button);
  } else {
    wasm.game_pointer_up(0);
    if (pointerId === e.pointerId) pointerId = null;
  }
}

canvas.addEventListener('pointerup', releasePointer);
canvas.addEventListener('pointercancel', releasePointer);
window.addEventListener('pointerup', (e) => {
  if (e.target !== canvas) releasePointer(e);
});

canvas.addEventListener('wheel', (e) => {
  if (!wasmReady || titleOpen) return;
  e.preventDefault();
  wasm.game_wheel(e.deltaY < 0 ? 1 : -1);
}, { passive: false });

// ---------------------------------------------------------------------------
// Touch buttons
// ---------------------------------------------------------------------------

function holdButton(id, keyCode) {
  const el = document.getElementById(id);
  if (!el) return;
  const down = (e) => {
    e.preventDefault();
    if (!wasmReady) return;
    resumeAudio();
    if (!touchMode) setTouchMode(true);
    wasm.game_key(keyCode, 1);
    el.classList.add('on');
    try { el.setPointerCapture(e.pointerId); } catch (_) {}
  };
  const up = (e) => {
    e.preventDefault();
    el.classList.remove('on');
    if (wasmReady) wasm.game_key(keyCode, 0);
  };
  el.addEventListener('pointerdown', down);
  el.addEventListener('pointerup', up);
  el.addEventListener('pointercancel', up);
  el.addEventListener('lostpointercapture', up);
}

holdButton('m-left', 0);   // A
holdButton('m-right', 1);  // D
holdButton('m-jump', 4);   // Space

function tapButton(id, fn) {
  const el = document.getElementById(id);
  if (!el) return;
  el.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    if (!wasmReady) return;
    resumeAudio();
    if (!touchMode) setTouchMode(true);
    if (titleOpen) return;
    fn();
  });
}

tapButton('m-bag', () => wasm.game_toggle_inventory());
tapButton('m-map', () => wasm.game_toggle_map());
tapButton('m-pause', () => wasm.game_toggle_pause());
tapButton('m-place', () => wasm.game_toggle_place());
tapButton('m-mute', () => {
  const muted = !audio.muted;
  wasm.game_toggle_mute();
  if (audio.muted !== muted) {
    // Keep JS/audio and Rust state aligned even if the call was a no-op.
    audio.muted = muted;
    if (audio.master) audio.master.gain.setTargetAtTime(muted ? 0 : 0.8, audio.ctx.currentTime, 0.03);
  }
  const btn = document.getElementById('m-mute');
  if (btn) {
    btn.textContent = audio.muted ? '🔇' : '♪';
    btn.classList.toggle('on', audio.muted);
  }
});

playBtn.addEventListener('click', () => startGame(false));
newWorldBtn.addEventListener('click', () => startGame(true));

// Double-tap on the title is a no-op but prevents mobile zoom.
titleEl.addEventListener('dblclick', (e) => e.preventDefault());

window.addEventListener('resize', () => {
  resize();
  if (wasmReady) {
    // Let Rust re-clamp its camera for the new viewport immediately.
    wasm.game_resize(viewW, viewH);
  }
});

window.addEventListener('orientationchange', () => {
  setTimeout(() => {
    resize();
  }, 120);
});

// Resume audio on any first interaction (autoplay policies).
window.addEventListener('pointerdown', resumeAudio, { once: true });
window.addEventListener('keydown', resumeAudio, { once: true });

// ---------------------------------------------------------------------------
// Debug / automation API
// ---------------------------------------------------------------------------

function readDebugJson() {
  if (!wasmReady) return null;
  const len = wasm.game_debug_state();
  const ptr = wasm.game_debug_ptr();
  return JSON.parse(new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len)));
}

window.__terra = {
  ready: false,
  get wasm() { return wasm; },
  get view() { return { w: viewW, h: viewH, scale: pixelScale, dpr }; },
  get touch() { return touchMode; },
  state: readDebugJson,
  play() { startGame(false); },
  newWorld(seed) {
    resumeAudio();
    hideTitle();
    wasm.game_new_world((seed == null ? Math.random() * 0xffffffff : seed) >>> 0);
  },
  setTouch(on) { setTouchMode(on); },
  key(code, down) { if (wasmReady) wasm.game_key(code, down ? 1 : 0); },
  pointer(x, y) { if (wasmReady) wasm.game_pointer_move(x, y); },
  pointerDown(button) { if (wasmReady) wasm.game_pointer_down(button || 0); },
  pointerUp(button) { if (wasmReady) wasm.game_pointer_up(button || 0); },
  wheel(d) { if (wasmReady) wasm.game_wheel(d); },
  tick(count, dt) {
    if (!wasmReady) return;
    const n = Math.max(1, count || 1);
    const step = dt || 1 / 60;
    for (let i = 0; i < n; i++) wasm.game_tick(step);
  },
  debug: {
    give(item, count) { wasm.game_debug_give(item, count); },
    teleport(x, y) { wasm.game_debug_teleport(x, y); },
    time(t) { wasm.game_debug_time(t); },
    heal() { wasm.game_debug_heal(); },
    spawnBoss() { wasm.game_debug_spawn_boss(); },
    killBoss() { wasm.game_debug_kill_boss(); },
    craft(index) { wasm.game_debug_craft(index); },
  },
  save() { if (wasmReady) wasm.game_save(); },
  toggleInventory() { if (wasmReady) wasm.game_toggle_inventory(); },
  toggleMap() { if (wasmReady) wasm.game_toggle_map(); },
  togglePause() { if (wasmReady) wasm.game_toggle_pause(); },
  togglePlace() { if (wasmReady) wasm.game_toggle_place(); },
  hasSave() { return wasmReady ? !!wasm.game_has_save() : false; },
  clearSave() {
    try {
      localStorage.removeItem(SAVE_KEY);
      localStorage.removeItem(SAVE_KEY + '.time');
    } catch (_) {}
    updateSaveNote(false);
  },
};

boot();
