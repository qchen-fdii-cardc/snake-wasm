//! Core game logic exposed to JavaScript via `wasm_bindgen`.

use rand::Rng;
use wasm_bindgen::prelude::*;

use crate::snake::{Direction, Point, Snake};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Current phase of the game.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    /// Waiting for the player to start.
    Idle,
    /// Actively running.
    Running,
    /// The snake hit a wall or itself.
    GameOver,
    /// Player paused the game.
    Paused,
}

// ---------------------------------------------------------------------------
// Game
// ---------------------------------------------------------------------------

/// The complete game instance exposed to JavaScript.
///
/// ## Usage (JavaScript)
/// ```js
/// const game = Game.new(20, 20);
/// game.change_direction("ArrowUp");
/// game.tick();
/// const cells = game.cells();          // Uint8Array
/// const score = game.score();
/// const state = game.state();          // 0=Idle 1=Running 2=GameOver 3=Paused
/// ```
#[wasm_bindgen]
pub struct Game {
    width: u32,
    height: u32,
    snake: Snake,
    food: Point,
    state: GameState,
    score: u32,
    /// Number of ticks since the last movement (used for speed throttling).
    tick_counter: u32,
    /// How many ticks must pass before the snake moves.
    ticks_per_move: u32,
}

#[wasm_bindgen]
impl Game {
    /// Create a new game on a `width × height` grid.
    ///
    /// The snake starts at the centre with length 3, moving right.
    /// A piece of food is placed at a random free cell.
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Self {
        let cx = (width / 2) as i32;
        let cy = (height / 2) as i32;
        let snake = Snake::new(cx, cy, 3);
        let mut game = Self {
            width,
            height,
            snake,
            food: Point::new(0, 0), // will be overwritten
            state: GameState::Idle,
            score: 0,
            tick_counter: 0,
            ticks_per_move: 10,
        };
        game.food = game.random_free_cell();
        game
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    /// Board width in cells.
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Board height in cells.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Current score.
    #[wasm_bindgen(getter)]
    pub fn score(&self) -> u32 {
        self.score
    }

    /// Current [`GameState`] as a `u8`:
    /// `0` = Idle, `1` = Running, `2` = GameOver, `3` = Paused.
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> u8 {
        self.state as u8
    }

    /// Snake length (number of cells).
    #[wasm_bindgen(getter)]
    pub fn snake_length(&self) -> usize {
        self.snake.len()
    }

    /// Food position as `[x, y]` packed in a `Uint32Array`.
    pub fn food_pos(&self) -> js_sys::Uint32Array {
        let arr = js_sys::Uint32Array::new_with_length(2);
        arr.set_index(0, self.food.x as u32);
        arr.set_index(1, self.food.y as u32);
        arr
    }

    /// Flat array of all snake body cells `[x0, y0, x1, y1, …]`.
    pub fn snake_cells(&self) -> js_sys::Uint32Array {
        let body = self.snake.body();
        let arr = js_sys::Uint32Array::new_with_length((body.len() * 2) as u32);
        for (i, p) in body.iter().enumerate() {
            arr.set_index((i * 2) as u32, p.x as u32);
            arr.set_index((i * 2 + 1) as u32, p.y as u32);
        }
        arr
    }

    /// Flat `Uint8Array` representing every cell on the board:
    /// - `0` = empty
    /// - `1` = snake body
    /// - `2` = snake head
    /// - `3` = food
    ///
    /// Index formula: `y * width + x`.
    pub fn cells(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(self.cells_vec().as_slice())
    }

    // ------------------------------------------------------------------
    // Input / control
    // ------------------------------------------------------------------

    /// Start or restart the game.
    pub fn start(&mut self) {
        let cx = (self.width / 2) as i32;
        let cy = (self.height / 2) as i32;
        self.snake = Snake::new(cx, cy, 3);
        self.score = 0;
        self.tick_counter = 0;
        self.ticks_per_move = 10;
        self.food = self.random_free_cell();
        self.state = GameState::Running;
    }

    /// Toggle pause / resume.
    pub fn toggle_pause(&mut self) {
        self.state = match self.state {
            GameState::Running => GameState::Paused,
            GameState::Paused => GameState::Running,
            other => other,
        };
    }

    /// Forward a keyboard key string to the game.
    ///
    /// Accepted keys: `"ArrowUp"`, `"ArrowDown"`, `"ArrowLeft"`, `"ArrowRight"`,
    /// `"w"`, `"a"`, `"s"`, `"d"` (case-sensitive).
    ///
    /// Also handles `"Enter"` / `" "` for start/restart and `"p"` / `"Escape"` for pause.
    pub fn key_down(&mut self, key: &str) {
        match key {
            "ArrowUp" | "w" | "W" => self.change_direction(Direction::Up),
            "ArrowDown" | "s" | "S" => self.change_direction(Direction::Down),
            "ArrowLeft" | "a" | "A" => self.change_direction(Direction::Left),
            "ArrowRight" | "d" | "D" => self.change_direction(Direction::Right),
            "Enter" | " " => {
                if self.state == GameState::Idle || self.state == GameState::GameOver {
                    self.start();
                }
            }
            "p" | "P" | "Escape" => self.toggle_pause(),
            _ => {}
        }
    }

    /// Change the snake's requested direction (ignores 180-degree reversals).
    pub fn change_direction(&mut self, dir: Direction) {
        if self.state == GameState::Running {
            self.snake.set_direction(dir);
        }
    }

    // ------------------------------------------------------------------
    // Simulation
    // ------------------------------------------------------------------

    /// Advance the simulation by one *render* tick.
    ///
    /// The snake only moves every `ticks_per_move` calls; call this on every
    /// `requestAnimationFrame` and let the game decide when to actually step.
    pub fn tick(&mut self) {
        if self.state != GameState::Running {
            return;
        }

        self.tick_counter += 1;
        if self.tick_counter < self.ticks_per_move {
            return;
        }
        self.tick_counter = 0;
        self.step();
    }

    /// Force a single game step (used internally and in tests).
    pub fn step(&mut self) {
        if self.state != GameState::Running {
            return;
        }

        let old_tail = self.snake.advance();

        let head = self.snake.head();

        // Wall collision
        if !self.in_bounds(head) {
            self.state = GameState::GameOver;
            return;
        }

        // Self collision
        if self.snake.is_self_colliding() {
            self.state = GameState::GameOver;
            return;
        }

        // Food eaten
        if head == self.food {
            self.snake.grow(old_tail);
            self.score += 10;
            // Speed up every 50 points (minimum 2 ticks per move)
            if self.score.is_multiple_of(50) && self.ticks_per_move > 2 {
                self.ticks_per_move -= 1;
            }
            self.food = self.random_free_cell();
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl Game {
    /// Internal helper: builds the cell buffer as a plain `Vec<u8>`.
    ///
    /// - `0` = empty, `1` = snake body, `2` = snake head, `3` = food.
    pub(crate) fn cells_vec(&self) -> Vec<u8> {
        let total = (self.width * self.height) as usize;
        let mut buf = vec![0u8; total];

        for p in self.snake.body().iter() {
            if self.in_bounds(*p) {
                buf[(p.y as u32 * self.width + p.x as u32) as usize] = 1;
            }
        }
        let head = self.snake.head();
        if self.in_bounds(head) {
            buf[(head.y as u32 * self.width + head.x as u32) as usize] = 2;
        }
        if self.in_bounds(self.food) {
            buf[(self.food.y as u32 * self.width + self.food.x as u32) as usize] = 3;
        }
        buf
    }

    /// Returns `true` when `p` is within the board boundaries.
    fn in_bounds(&self, p: Point) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width as i32 && p.y < self.height as i32
    }

    /// Pick a random cell that is not currently occupied by the snake.
    ///
    /// Falls back to `(0, 0)` if the board is completely full (extremely unlikely).
    fn random_free_cell(&self) -> Point {
        let mut rng = rand::thread_rng();
        for _ in 0..1000 {
            let x = rng.gen_range(0..self.width as i32);
            let y = rng.gen_range(0..self.height as i32);
            let p = Point::new(x, y);
            if !self.snake.occupies(p) {
                return p;
            }
        }
        // Fallback (board nearly full)
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let p = Point::new(x, y);
                if !self.snake.occupies(p) {
                    return p;
                }
            }
        }
        Point::new(0, 0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn running_game() -> Game {
        let mut g = Game::new(20, 20);
        g.start();
        g
    }

    #[test]
    fn new_game_is_idle() {
        let g = Game::new(20, 20);
        assert_eq!(g.state, GameState::Idle);
    }

    #[test]
    fn start_sets_running() {
        let g = running_game();
        assert_eq!(g.state, GameState::Running);
    }

    #[test]
    fn start_resets_score() {
        let mut g = running_game();
        g.score = 100;
        g.start();
        assert_eq!(g.score, 0);
    }

    #[test]
    fn start_resets_snake_length() {
        let mut g = running_game();
        g.start();
        assert_eq!(g.snake_length(), 3);
    }

    #[test]
    fn tick_does_nothing_when_idle() {
        let mut g = Game::new(20, 20);
        let head_before = g.snake.head();
        g.tick();
        assert_eq!(g.snake.head(), head_before);
    }

    #[test]
    fn tick_does_nothing_when_paused() {
        let mut g = running_game();
        g.toggle_pause();
        let head_before = g.snake.head();
        for _ in 0..20 {
            g.tick();
        }
        assert_eq!(g.snake.head(), head_before);
    }

    #[test]
    fn step_advances_snake() {
        let mut g = running_game();
        let head_before = g.snake.head();
        g.step();
        let head_after = g.snake.head();
        // Default direction is Right
        assert_eq!(head_after.x, head_before.x + 1);
        assert_eq!(head_after.y, head_before.y);
    }

    #[test]
    fn wall_collision_ends_game() {
        let mut g = Game::new(10, 10);
        g.start();
        // Move right until we hit the wall
        for _ in 0..20 {
            g.step();
        }
        assert_eq!(g.state, GameState::GameOver);
    }

    #[test]
    fn snake_grows_on_food() {
        let mut g = running_game();
        let initial_len = g.snake_length();
        // Place food directly in front of the head
        let head = g.snake.head();
        g.food = Point::new(head.x + 1, head.y);
        g.step();
        assert_eq!(g.snake_length(), initial_len + 1);
    }

    #[test]
    fn score_increases_on_food() {
        let mut g = running_game();
        let head = g.snake.head();
        g.food = Point::new(head.x + 1, head.y);
        g.step();
        assert_eq!(g.score, 10);
    }

    #[test]
    fn toggle_pause_pauses_and_resumes() {
        let mut g = running_game();
        g.toggle_pause();
        assert_eq!(g.state, GameState::Paused);
        g.toggle_pause();
        assert_eq!(g.state, GameState::Running);
    }

    #[test]
    fn toggle_pause_does_not_affect_idle() {
        let mut g = Game::new(20, 20);
        g.toggle_pause();
        assert_eq!(g.state, GameState::Idle);
    }

    #[test]
    fn cells_returns_correct_size() {
        let g = Game::new(10, 15);
        assert_eq!(g.cells_vec().len(), 150);
    }

    #[test]
    fn cells_marks_food_and_head() {
        let mut g = running_game();
        // Place food in top-left corner and start going up/left away from it
        g.food = Point::new(0, 0);
        let cells = g.cells_vec();
        assert_eq!(cells[0], 3); // food at (0,0)
        // Head should be marked as 2
        let head = g.snake.head();
        let idx = (head.y as u32 * g.width + head.x as u32) as usize;
        assert_eq!(cells[idx], 2);
    }

    #[test]
    fn in_bounds_works() {
        let g = Game::new(10, 10);
        assert!(g.in_bounds(Point::new(0, 0)));
        assert!(g.in_bounds(Point::new(9, 9)));
        assert!(!g.in_bounds(Point::new(10, 5)));
        assert!(!g.in_bounds(Point::new(5, 10)));
        assert!(!g.in_bounds(Point::new(-1, 5)));
    }

    #[test]
    fn key_down_changes_direction() {
        let mut g = running_game();
        g.key_down("ArrowUp");
        g.step();
        // After stepping up the y should have decreased
        let head = g.snake.head();
        // The snake started at centre (10,10), moved up once
        assert_eq!(head.y, (20 / 2) - 1);
    }

    #[test]
    fn key_down_start_from_idle() {
        let mut g = Game::new(20, 20);
        assert_eq!(g.state, GameState::Idle);
        g.key_down("Enter");
        assert_eq!(g.state, GameState::Running);
    }

    #[test]
    fn key_down_pause() {
        let mut g = running_game();
        g.key_down("p");
        assert_eq!(g.state, GameState::Paused);
    }

    #[test]
    fn key_down_wasd() {
        let mut g = running_game();
        g.key_down("w");
        g.step();
        let head = g.snake.head();
        assert_eq!(head.y, (20 / 2) - 1);
    }

    #[test]
    fn speed_increases_at_50_points() {
        let mut g = running_game();
        let initial_speed = g.ticks_per_move;
        // Simulate eating 5 food items (50 points)
        for _ in 0..5 {
            let head = g.snake.head();
            g.food = Point::new(head.x + 1, head.y);
            g.step();
        }
        assert!(g.ticks_per_move < initial_speed);
    }

    #[test]
    fn game_over_does_not_step() {
        let mut g = running_game();
        // Force game over by walking into wall
        for _ in 0..20 {
            g.step();
        }
        assert_eq!(g.state, GameState::GameOver);
        let head = g.snake.head();
        g.step(); // extra step should do nothing
        assert_eq!(g.snake.head(), head);
    }

    #[test]
    fn random_free_cell_is_free() {
        let g = running_game();
        for _ in 0..50 {
            let p = g.random_free_cell();
            assert!(!g.snake.occupies(p));
        }
    }

    #[test]
    fn ticks_per_move_throttles_movement() {
        let mut g = running_game();
        g.ticks_per_move = 5;
        g.tick_counter = 0;
        let head_before = g.snake.head();
        // Only 4 ticks – should not have moved yet
        for _ in 0..4 {
            g.tick();
        }
        assert_eq!(g.snake.head(), head_before);
        // 5th tick triggers movement
        g.tick();
        assert_ne!(g.snake.head(), head_before);
    }
}
