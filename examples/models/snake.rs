use std::collections::VecDeque;
use terragame_core::random::Random;
use terragame_world::Point;
pub const WIDTH: i64 = 40;
pub const HEIGHT: i64 = 18;
pub struct Snake {
    pub body: VecDeque<Point>,
    pub food: Point,
    pub score: u32,
    pub dead: bool,
    pub won: bool,
    pub paused: bool,
    direction: (i64, i64),
    pending: (i64, i64),
    random: Random,
}
impl Default for Snake {
    fn default() -> Self {
        Self {
            body: [Point::new(8, 8), Point::new(7, 8), Point::new(6, 8)].into(),
            food: Point::new(12, 8),
            score: 0,
            dead: false,
            won: false,
            paused: false,
            direction: (1, 0),
            pending: (1, 0),
            random: Random::new(12),
        }
    }
}
impl Snake {
    pub fn turn(&mut self, direction: (i64, i64)) {
        if direction != (-self.direction.0, -self.direction.1) {
            self.pending = direction;
        }
    }
    pub fn step(&mut self) {
        if self.dead || self.won || self.paused {
            return;
        }
        self.direction = self.pending;
        let next = self.body[0]
            .offset(self.direction.0, self.direction.1)
            .unwrap();
        let growing = next == self.food;
        if next.x < 1
            || next.x >= WIDTH - 1
            || next.y < 1
            || next.y >= HEIGHT - 1
            || self
                .body
                .iter()
                .take(self.body.len() - usize::from(!growing))
                .any(|&p| p == next)
        {
            self.dead = true;
            return;
        }
        self.body.push_front(next);
        if growing {
            self.score += 1;
            let free: Vec<_> = (1..HEIGHT - 1)
                .flat_map(|y| (1..WIDTH - 1).map(move |x| Point::new(x, y)))
                .filter(|p| !self.body.contains(p))
                .collect();
            if free.is_empty() {
                self.won = true;
            } else {
                self.food = free[self.random.below(free.len() as u64).unwrap() as usize];
            }
        } else {
            self.body.pop_back();
        }
    }
}
