/**
 * Snake WASM — JavaScript host
 *
 * Responsibilities:
 *  • Load the WASM module produced by wasm-pack
 *  • Drive the game loop via requestAnimationFrame
 *  • Render cells onto an HTML5 <canvas>
 *  • Forward keyboard / touch events to the Rust game instance
 *  • Persist the high-score in localStorage
 */

import init, { Game, Direction } from './pkg/snake_wasm.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Number of columns / rows in the game grid. */
const COLS = 20;
const ROWS = 20;

/** Pixel size of each cell – matches the CSS variable `--cell-size`. */
const CELL  = getCellSize();

const COLORS = {
  bg:      '#1a1a2e',
  grid:    '#1f1f38',
  body:    '#4ade80',
  head:    '#86efac',
  food:    '#f87171',
  border:  '#2a2a4a',
};

/** GameState values must match the Rust enum order. */
const STATE = { Idle: 0, Running: 1, GameOver: 2, Paused: 3 };

// ---------------------------------------------------------------------------
// DOM references
// ---------------------------------------------------------------------------

const canvas     = /** @type {HTMLCanvasElement} */ (document.getElementById('game-canvas'));
const ctx        = canvas.getContext('2d');
const overlay    = document.getElementById('overlay');
const overlayTitle   = document.getElementById('overlay-title');
const overlayMessage = document.getElementById('overlay-message');
const startBtn   = document.getElementById('start-btn');
const scoreEl    = document.getElementById('score');
const highScoreEl= document.getElementById('high-score');
const lengthEl   = document.getElementById('length');

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getCellSize() {
  const style = getComputedStyle(document.documentElement);
  const raw   = style.getPropertyValue('--cell-size').trim();
  return parseInt(raw, 10) || 24;
}

function setCanvasSize() {
  canvas.width  = COLS * CELL;
  canvas.height = ROWS * CELL;
}

function loadHighScore() {
  return parseInt(localStorage.getItem('snake-high-score') || '0', 10);
}

function saveHighScore(score) {
  localStorage.setItem('snake-high-score', String(score));
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/**
 * Render the current game state.
 *
 * @param {Game}   game
 * @param {number} highScore
 */
function render(game, highScore) {
  const W = canvas.width;
  const H = canvas.height;

  // Background
  ctx.fillStyle = COLORS.bg;
  ctx.fillRect(0, 0, W, H);

  // Optional subtle grid lines
  ctx.strokeStyle = COLORS.grid;
  ctx.lineWidth   = 0.5;
  for (let c = 0; c <= COLS; c++) {
    ctx.beginPath();
    ctx.moveTo(c * CELL, 0);
    ctx.lineTo(c * CELL, H);
    ctx.stroke();
  }
  for (let r = 0; r <= ROWS; r++) {
    ctx.beginPath();
    ctx.moveTo(0, r * CELL);
    ctx.lineTo(W, r * CELL);
    ctx.stroke();
  }

  const cells = game.cells();   // Uint8Array: 0=empty 1=body 2=head 3=food

  for (let i = 0; i < cells.length; i++) {
    const cell = cells[i];
    if (cell === 0) continue;

    const col = i % COLS;
    const row = Math.floor(i / COLS);
    const px  = col * CELL;
    const py  = row * CELL;
    const pad = 1;

    switch (cell) {
      case 1: // body
        ctx.fillStyle = COLORS.body;
        ctx.beginPath();
        ctx.roundRect(px + pad, py + pad, CELL - pad * 2, CELL - pad * 2, 4);
        ctx.fill();
        break;
      case 2: // head
        ctx.fillStyle = COLORS.head;
        ctx.beginPath();
        ctx.roundRect(px + pad, py + pad, CELL - pad * 2, CELL - pad * 2, 6);
        ctx.fill();
        // Eye dots
        ctx.fillStyle = COLORS.bg;
        ctx.beginPath();
        ctx.arc(px + CELL * 0.3, py + CELL * 0.35, CELL * 0.1, 0, Math.PI * 2);
        ctx.arc(px + CELL * 0.7, py + CELL * 0.35, CELL * 0.1, 0, Math.PI * 2);
        ctx.fill();
        break;
      case 3: // food
        ctx.fillStyle = COLORS.food;
        ctx.beginPath();
        ctx.arc(
          px + CELL / 2,
          py + CELL / 2,
          CELL / 2 - 2,
          0,
          Math.PI * 2
        );
        ctx.fill();
        // Shine
        ctx.fillStyle = 'rgba(255,255,255,0.3)';
        ctx.beginPath();
        ctx.arc(px + CELL * 0.35, py + CELL * 0.3, CELL * 0.12, 0, Math.PI * 2);
        ctx.fill();
        break;
    }
  }

  // HUD
  scoreEl.textContent    = String(game.score);
  highScoreEl.textContent= String(highScore);
  lengthEl.textContent   = String(game.snake_length);
}

// ---------------------------------------------------------------------------
// Overlay helpers
// ---------------------------------------------------------------------------

function showOverlay(title, message, btnText) {
  overlayTitle.textContent   = title;
  overlayMessage.textContent = message;
  startBtn.textContent       = btnText;
  overlay.classList.remove('hidden');
}

function hideOverlay() {
  overlay.classList.add('hidden');
}

// ---------------------------------------------------------------------------
// Game loop
// ---------------------------------------------------------------------------

/**
 * Main entry point, called once the WASM module is initialised.
 *
 * @param {Game} game
 */
function startLoop(game) {
  let highScore = loadHighScore();
  highScoreEl.textContent = String(highScore);

  showOverlay('🐍 Snake', 'Use arrow keys or WASD to move', 'Press Enter to Start');

  function loop() {
    const state = game.state;   // 0=Idle 1=Running 2=GameOver 3=Paused

    if (state === STATE.Running) {
      game.tick();

      const score = game.score;
      if (score > highScore) {
        highScore = score;
        saveHighScore(highScore);
      }
    }

    render(game, highScore);

    requestAnimationFrame(loop);
  }

  requestAnimationFrame(loop);

  // ------------------------------------------------------------------
  // Input: keyboard
  // ------------------------------------------------------------------
  document.addEventListener('keydown', (e) => {
    // Prevent page scrolling for game keys
    if (['ArrowUp','ArrowDown','ArrowLeft','ArrowRight',' '].includes(e.key)) {
      e.preventDefault();
    }

    const prevState = game.state;
    game.key_down(e.key);

    // Transition: Idle → Running  or  GameOver → Running
    if (prevState !== STATE.Running && game.state === STATE.Running) {
      hideOverlay();
    }

    // Entering Pause
    if (prevState === STATE.Running && game.state === STATE.Paused) {
      showOverlay('Paused', 'Press P or Esc to resume', 'Resume');
    }

    // Exiting Pause
    if (prevState === STATE.Paused && game.state === STATE.Running) {
      hideOverlay();
    }
  });

  // ------------------------------------------------------------------
  // Input: start button
  // ------------------------------------------------------------------
  startBtn.addEventListener('click', () => {
    if (game.state === STATE.Idle || game.state === STATE.GameOver) {
      game.start();
      hideOverlay();
    } else if (game.state === STATE.Paused) {
      game.toggle_pause();
      hideOverlay();
    }
  });

  // ------------------------------------------------------------------
  // Input: mobile D-pad
  // ------------------------------------------------------------------
  document.querySelectorAll('.dpad-btn').forEach((btn) => {
    btn.addEventListener('touchstart', (e) => {
      e.preventDefault();
      const key = btn.dataset.key;
      game.key_down(key);
      if (game.state === STATE.Running) hideOverlay();
    }, { passive: false });

    btn.addEventListener('click', () => {
      const key = btn.dataset.key;
      game.key_down(key);
      if (game.state === STATE.Running) hideOverlay();
    });
  });

  // ------------------------------------------------------------------
  // Game-over detection  (polled every frame above; also check here)
  // ------------------------------------------------------------------
  setInterval(() => {
    if (game.state === STATE.GameOver) {
      const score = game.score;
      if (score > highScore) {
        highScore = score;
        saveHighScore(highScore);
      }
      if (overlay.classList.contains('hidden') || overlayTitle.textContent !== 'Game Over') {
        showOverlay(
          'Game Over',
          `Score: ${score}`,
          'Play Again'
        );
      }
    }
  }, 100);
}

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

async function bootstrap() {
  await init();
  setCanvasSize();

  const game = new Game(COLS, ROWS);
  startLoop(game);
}

bootstrap().catch(console.error);
