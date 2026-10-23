#[path = "models/snake.rs"]
mod snake;
use snake::Snake;
use std::time::Duration;
use terragame_terminal::{
    Application, Control, run,
    terrabox::{Color, Event, Key, Rect, Size, Style, Surface},
};
struct Game {
    snake: Snake,
}
fn style(fg: u8) -> Style {
    Style {
        foreground: Color::Indexed(fg),
        ..Default::default()
    }
}
impl Application for Game {
    fn event(&mut self, event: Event) -> Control {
        if let Event::Key(key) = event {
            if terragame_terminal::quit_key(key) {
                return Control::QUIT;
            }
            match key.key {
                Key::Up | Key::Char('w') => self.snake.turn((0, -1)),
                Key::Down | Key::Char('s') => self.snake.turn((0, 1)),
                Key::Left | Key::Char('a') => self.snake.turn((-1, 0)),
                Key::Right | Key::Char('d') => self.snake.turn((1, 0)),
                Key::Char('p' | ' ') => self.snake.paused = !self.snake.paused,
                Key::Char('n') => self.snake = Snake::default(),
                _ => return Control::default(),
            }
            return Control::REDRAW;
        }
        Control::default()
    }
    fn paused(&self) -> bool {
        self.snake.paused || self.snake.dead || self.snake.won
    }
    fn tick(&mut self, _: u64) -> bool {
        self.snake.step();
        true
    }
    fn render(&self, surface: &mut Surface, size: Size) {
        surface.clear(style(7));
        let mut c = surface.canvas(Rect::new(0, 0, size.width, size.height));
        c.put_str(
            0,
            0,
            &format!(
                "TerraSnake | Score {} | {}",
                self.snake.score,
                if self.snake.dead {
                    "GAME OVER"
                } else if self.snake.won {
                    "WON"
                } else if self.snake.paused {
                    "PAUSED"
                } else {
                    "RUNNING"
                }
            ),
            style(14),
        );
        c.put_str(
            0,
            1,
            "Arrows/WASD move | P pause | N restart | Q/Esc quit",
            style(7),
        );
        if size.width < 40 || size.height < 21 {
            c.put_str(
                0,
                3,
                "Need 40x21; resize (simulation size is fixed)",
                style(11),
            );
            return;
        }
        for y in 0..snake::HEIGHT {
            for x in 0..snake::WIDTH {
                let wall = x == 0 || y == 0 || x == snake::WIDTH - 1 || y == snake::HEIGHT - 1;
                c.put_char(
                    x as u16,
                    y as u16 + 3,
                    if wall { '#' } else { ' ' },
                    style(8),
                );
            }
        }
        c.put_char(
            self.snake.food.x as u16,
            self.snake.food.y as u16 + 3,
            '*',
            style(11),
        );
        for (i, p) in self.snake.body.iter().enumerate() {
            c.put_char(
                p.x as u16,
                p.y as u16 + 3,
                if i == 0 { '@' } else { 'o' },
                style(10),
            );
        }
    }
}
fn main() -> std::io::Result<()> {
    run(
        &mut Game {
            snake: Snake::default(),
        },
        Duration::from_millis(200),
        Duration::from_millis(100),
    )
}
