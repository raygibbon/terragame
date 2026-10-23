use terrabox::{Position, Rect};
use terragame_world::Point;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Camera {
    pub origin: Point,
    pub viewport: Rect,
}
impl Camera {
    pub fn following(center: Point, viewport: Rect) -> Self {
        fn origin(center: i64, span: u16) -> i64 {
            let extent = i128::from(span.saturating_sub(1));
            (i128::from(center) - i128::from(span / 2))
                .clamp(i128::from(i64::MIN), i128::from(i64::MAX) - extent) as i64
        }
        Self {
            origin: Point::new(
                origin(center.x, viewport.width),
                origin(center.y, viewport.height),
            ),
            viewport,
        }
    }
    /// Local viewport cells, preserving representable coordinates at world edges.
    pub fn world_at(self, x: u16, y: u16) -> Point {
        Point::new(
            self.origin.x.saturating_add(i64::from(x)),
            self.origin.y.saturating_add(i64::from(y)),
        )
    }
    pub fn local(self, point: Point) -> Option<Position> {
        let x = i128::from(point.x) - i128::from(self.origin.x);
        let y = i128::from(point.y) - i128::from(self.origin.y);
        (x >= 0
            && y >= 0
            && x < i128::from(self.viewport.width)
            && y < i128::from(self.viewport.height))
        .then_some(Position {
            x: x as u16,
            y: y as u16,
        })
    }
    /// Absolute terminal coordinates; rejects overflow rather than wrapping.
    pub fn project(self, point: Point) -> Option<Position> {
        let p = self.local(point)?;
        Some(Position {
            x: p.x.checked_add(self.viewport.x)?,
            y: p.y.checked_add(self.viewport.y)?,
        })
    }
}
