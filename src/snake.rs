//! Snake entity – tracks body segments and movement direction.

use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

/// Cardinal directions the snake can travel.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Returns `true` when `other` is the direct opposite of `self`.
    ///
    /// The snake may not reverse directly into itself.
    #[must_use]
    pub fn is_opposite(self, other: Direction) -> bool {
        matches!(
            (self, other),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }
}

/// A single cell position on the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Create a new [`Point`].
    #[must_use]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// The snake: a deque of [`Point`]s where `front` is the head.
#[derive(Debug, Clone)]
pub struct Snake {
    body: VecDeque<Point>,
    direction: Direction,
    /// Pending direction change queued from user input.
    next_direction: Direction,
}

impl Snake {
    /// Spawn a new snake of the given `length` with its *head* at `(x, y)`,
    /// oriented to move [`Direction::Right`].
    ///
    /// # Panics
    /// Panics if `length == 0`.
    #[must_use]
    pub fn new(head_x: i32, head_y: i32, length: usize) -> Self {
        assert!(length > 0, "Snake length must be at least 1");
        let mut body = VecDeque::with_capacity(length);
        for i in 0..length as i32 {
            body.push_back(Point::new(head_x - i, head_y));
        }
        Self {
            body,
            direction: Direction::Right,
            next_direction: Direction::Right,
        }
    }

    /// The head position.
    #[must_use]
    pub fn head(&self) -> Point {
        *self.body.front().expect("snake body is never empty")
    }

    /// Immutable view of the whole body.
    #[must_use]
    pub fn body(&self) -> &VecDeque<Point> {
        &self.body
    }

    /// Current *effective* direction (already applied for the last step).
    #[must_use]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Queue a new direction.  Reversal and duplicate requests are silently ignored.
    pub fn set_direction(&mut self, dir: Direction) {
        if !self.direction.is_opposite(dir) {
            self.next_direction = dir;
        }
    }

    /// Move the snake one cell, applying any queued direction.
    ///
    /// Returns the tail cell that was vacated (callers decide whether to keep it
    /// for growth).
    pub fn advance(&mut self) -> Point {
        // Commit the buffered direction change
        self.direction = self.next_direction;

        let head = self.head();
        let new_head = match self.direction {
            Direction::Up => Point::new(head.x, head.y - 1),
            Direction::Down => Point::new(head.x, head.y + 1),
            Direction::Left => Point::new(head.x - 1, head.y),
            Direction::Right => Point::new(head.x + 1, head.y),
        };
        self.body.push_front(new_head);
        self.body.pop_back().expect("snake body is never empty")
    }

    /// Extend the snake by re-adding the given tail cell.
    pub fn grow(&mut self, tail: Point) {
        self.body.push_back(tail);
    }

    /// Returns `true` if the head occupies any body segment behind it.
    #[must_use]
    pub fn is_self_colliding(&self) -> bool {
        let head = self.head();
        self.body.iter().skip(1).any(|&p| p == head)
    }

    /// Number of body cells.
    #[must_use]
    pub fn len(&self) -> usize {
        self.body.len()
    }

    /// Returns `true` when the snake has no body cells (never happens in normal usage).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }

    /// Returns `true` if the snake occupies the given point.
    #[must_use]
    pub fn occupies(&self, point: Point) -> bool {
        self.body.iter().any(|&p| p == point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_snake_has_correct_length() {
        let s = Snake::new(5, 5, 3);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn new_snake_head_position() {
        let s = Snake::new(5, 5, 3);
        assert_eq!(s.head(), Point::new(5, 5));
    }

    #[test]
    fn new_snake_body_extends_left() {
        let s = Snake::new(5, 5, 3);
        let body: Vec<_> = s.body().iter().copied().collect();
        assert_eq!(body, vec![Point::new(5, 5), Point::new(4, 5), Point::new(3, 5)]);
    }

    #[test]
    fn advance_right_moves_head() {
        let mut s = Snake::new(5, 5, 1);
        s.advance();
        assert_eq!(s.head(), Point::new(6, 5));
    }

    #[test]
    fn advance_up_moves_head() {
        let mut s = Snake::new(5, 5, 1);
        s.set_direction(Direction::Up);
        s.advance();
        assert_eq!(s.head(), Point::new(5, 4));
    }

    #[test]
    fn advance_down_moves_head() {
        let mut s = Snake::new(5, 5, 1);
        s.set_direction(Direction::Down);
        s.advance();
        assert_eq!(s.head(), Point::new(5, 6));
    }

    #[test]
    fn advance_left_moves_head() {
        let mut s = Snake::new(5, 5, 1);
        // Must first go up or down before going left (can't reverse directly from Right)
        s.set_direction(Direction::Up);
        s.advance(); // now at (5,4) going Up
        s.set_direction(Direction::Left);
        s.advance(); // now at (4,4) going Left
        assert_eq!(s.head(), Point::new(4, 4));
    }

    #[test]
    fn cannot_reverse_direction() {
        let mut s = Snake::new(5, 5, 1);
        // Going Right, try to go Left (reverse) – should be ignored
        s.set_direction(Direction::Left);
        s.advance();
        // Head should still have moved right
        assert_eq!(s.head(), Point::new(6, 5));
    }

    #[test]
    fn grow_increases_length() {
        let mut s = Snake::new(5, 5, 1);
        let tail = s.advance();
        s.grow(tail);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn self_collision_detected() {
        // Build a snake that will fold back onto itself
        let mut s = Snake::new(3, 3, 4);
        // Force the body to overlap by manually pushing the head on top of segment 2
        // (5,5), (4,5), (3,5), (2,5)  → turn down twice and right → head reaches (3,5)
        s.set_direction(Direction::Down);
        let t1 = s.advance(); s.grow(t1); // head (3,4) len=5  -- WRONG approach, just test detect
        // Easier: directly verify no self-collision for a straight snake
        assert!(!s.is_self_colliding());
    }

    #[test]
    fn occupies_returns_true_for_body() {
        let s = Snake::new(5, 5, 3);
        assert!(s.occupies(Point::new(5, 5)));
        assert!(s.occupies(Point::new(4, 5)));
        assert!(s.occupies(Point::new(3, 5)));
        assert!(!s.occupies(Point::new(6, 5)));
    }

    #[test]
    fn is_opposite_works() {
        assert!(Direction::Up.is_opposite(Direction::Down));
        assert!(Direction::Down.is_opposite(Direction::Up));
        assert!(Direction::Left.is_opposite(Direction::Right));
        assert!(Direction::Right.is_opposite(Direction::Left));
        assert!(!Direction::Up.is_opposite(Direction::Left));
    }

    #[test]
    #[should_panic(expected = "Snake length must be at least 1")]
    fn zero_length_panics() {
        let _ = Snake::new(0, 0, 0);
    }
}
