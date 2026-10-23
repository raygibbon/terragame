#[path = "models/sandbox.rs"]
mod sandbox;
use sandbox::{Kind, Sandbox};
use std::{cell::RefCell, time::Duration};
use terragame_terminal::{
    Application, Camera, Control, Layer, Layers, Palette, Tile, run,
    terrabox::{Color, Event, Key, Rect, Size, Style, Surface},
};
fn style(fg: u8) -> Style {
    Style {
        foreground: Color::Indexed(fg),
        ..Default::default()
    }
}
// Only cache/composition scratch uses interior mutability. Render never advances
// gameplay; dimensions cannot change entity state, RNG or simulation results.
struct Demo {
    game: Sandbox,
    view: RefCell<Layers>,
    terrain: RefCell<terragame_world::ChunkCache<Vec<u8>>>,
    palette: Palette,
}
impl Application for Demo {
    fn paused(&self) -> bool {
        self.game.paused
    }
    fn tick(&mut self, tick: u64) -> bool {
        self.game.update(tick);
        true
    }
    fn event(&mut self, event: Event) -> Control {
        if let Event::Key(key) = event {
            if terragame_terminal::quit_key(key) {
                return Control::QUIT;
            }
            match key.key {
                Key::Up | Key::Char('w') => self.game.move_player(0, -1),
                Key::Down | Key::Char('s') => self.game.move_player(0, 1),
                Key::Left | Key::Char('a') => self.game.move_player(-1, 0),
                Key::Right | Key::Char('d') => self.game.move_player(1, 0),
                Key::Char('p' | ' ') => self.game.paused = !self.game.paused,
                Key::Char('e') => self.game.gather(),
                _ => return Control::default(),
            };
            return Control::REDRAW;
        }
        Control::default()
    }
    fn render(&self, surface: &mut Surface, size: Size) {
        let mut layers = self.view.borrow_mut();
        if !layers.resize(size) {
            surface.clear(style(7));
            surface
                .canvas(Rect::new(0, 0, size.width, size.height))
                .put_str(0, 0, "Viewport exceeds demo memory cap; resize", style(11));
            return;
        }
        layers.clear();
        let view = Rect::new(0, 4, size.width, size.height.saturating_sub(5));
        let camera = Camera::following(self.game.position(), view);
        let layout = terragame_world::ChunkLayout::new(16).unwrap();
        let mut cache = self.terrain.borrow_mut();
        cache.focus(layout.locate(self.game.position()).0);
        for y in 0..view.height {
            for x in 0..view.width {
                let p = camera.world_at(x, y);
                let (key, index) = layout.locate(p);
                let glyph = cache.get_or_insert_with(key, |k| {
                    (0..256)
                        .map(|i| {
                            layout
                                .world_at(k, i)
                                .map(|p| {
                                    if terragame_world::coordinate_hash(9, p.x, p.y)
                                        .is_multiple_of(17)
                                    {
                                        b','
                                    } else {
                                        b'.'
                                    }
                                })
                                .unwrap_or(b' ')
                        })
                        .collect()
                })[index];
                layers.put(
                    Layer::Terrain,
                    x,
                    y + view.y,
                    self.palette.get(usize::from(glyph == b',')).unwrap(),
                );
            }
        }
        for (id, object) in self.game.entities.iter() {
            let p = self.game.spatial.position(id).unwrap();
            let (glyph, color) = match object.kind {
                Kind::Player => (b'@', 15),
                Kind::Creature => (b'g', 10),
                Kind::Resource => (b'*', 11),
            };
            layers.put_world(
                if object.kind == Kind::Resource {
                    Layer::Items
                } else {
                    Layer::Entities
                },
                camera,
                p,
                Tile::ascii(glyph, style(color)).unwrap(),
            );
        }
        // The player must remain visible if an ambient object shares its cell.
        layers.put_world(
            Layer::Effects,
            camera,
            self.game.position(),
            Tile::ascii(b'@', style(15)).unwrap(),
        );
        layers.compose(surface, style(7));
        let mut c = surface.canvas(Rect::new(0, 0, size.width, size.height));
        c.put_str(
            0,
            0,
            &format!(
                "TerraGame Sandbox | {} | Tick {}",
                if self.game.paused {
                    "PAUSED"
                } else {
                    "RUNNING"
                },
                self.game.tick
            ),
            style(14),
        );
        c.put_str(
            0,
            1,
            &format!(
                "Entities {} | Nearby {} | Active {} | Dormant {} | Gathered {}",
                self.game.entities.len(),
                self.game.nearby,
                self.game.regions.keys().count(),
                self.game.dormant_count(),
                self.game.gathered
            ),
            style(7),
        );
        c.put_str(
            0,
            2,
            &format!(
                "Rates: creatures 1 / environment 10 / coarse 100 ticks | env {} coarse {}",
                self.game.environment_updates, self.game.coarse_updates
            ),
            style(11),
        );
        c.put_str(
            0,
            3,
            &format!(
                "Position {},{} | region +{} -{} | cache {}",
                self.game.position().x,
                self.game.position().y,
                self.game.activations,
                self.game.deactivations,
                cache.len()
            ),
            style(7),
        );
        c.put_str(
            0,
            size.height.saturating_sub(1),
            "WASD/arrows move | E gather * | P pause | Q/Esc quit",
            style(14),
        );
    }
}
impl Default for Demo {
    fn default() -> Self {
        let mut palette = Palette::new(2);
        palette.add(Tile::ascii(b'.', style(8)).unwrap());
        palette.add(Tile::ascii(b',', style(2)).unwrap());
        Demo {
            game: Sandbox::default(),
            view: RefCell::new(
                Layers::new(
                    Size {
                        width: 80,
                        height: 24,
                    },
                    262144,
                )
                .unwrap(),
            ),
            terrain: RefCell::new(terragame_world::ChunkCache::new(64)),
            palette,
        }
    }
}
fn main() -> std::io::Result<()> {
    let mut app = Demo::default();
    run(
        &mut app,
        Duration::from_millis(100),
        Duration::from_millis(250),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_renderer_is_independent_of_simulation_and_terminal_dimensions() {
        let mut a = Demo::default();
        let mut b = Demo::default();
        let mut surface = Surface::new(Size {
            width: 1,
            height: 1,
        });
        for tick in 1..=110 {
            a.tick(tick);
            b.tick(tick);
            for size in [
                Size {
                    width: 0,
                    height: 0,
                },
                Size {
                    width: 10,
                    height: 4,
                },
                Size {
                    width: 80,
                    height: 24,
                },
                Size {
                    width: 100,
                    height: 30,
                },
            ] {
                surface.resize(size);
                a.render(&mut surface, size);
            }
            let snapshot = |d: &Demo| {
                d.game
                    .entities
                    .iter()
                    .map(|(id, o)| (id, o.kind, d.game.spatial.position(id)))
                    .collect::<Vec<_>>()
            };
            assert_eq!(snapshot(&a), snapshot(&b));
            assert_eq!(a.game.tick, b.game.tick);
        }
    }
}
