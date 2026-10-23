use crate::Camera;
use terrabox::{Rect, Size, Style, Surface, text_width};
use terragame_world::Point;
/// Fixed back-to-front order. None is transparent; a blank tile is opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Layer {
    Background,
    Terrain,
    Structures,
    Items,
    Entities,
    Effects,
    Overlay,
}
const COUNT: usize = 7;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    glyph: char,
    pub style: Style,
}
impl Tile {
    /// Width validation delegates to Terrabox; no second Unicode implementation.
    pub fn new(glyph: char, style: Style) -> Option<Self> {
        (!glyph.is_control() && text_width(&glyph.to_string()) == 1)
            .then_some(Self { glyph, style })
    }
    pub fn ascii(glyph: u8, style: Style) -> Option<Self> {
        (b' '..=b'~').contains(&glyph).then_some(Self {
            glyph: char::from(glyph),
            style,
        })
    }
    pub fn with_fallback(glyph: char, fallback: u8, style: Style) -> Option<Self> {
        Self::new(glyph, style).or_else(|| Self::ascii(fallback, style))
    }
    pub fn glyph(self) -> char {
        self.glyph
    }
}
/// Small configurable tile palette. Semantics and IDs belong to the game.
pub struct Palette {
    tiles: Vec<Tile>,
    limit: usize,
}
impl Palette {
    pub fn new(limit: usize) -> Self {
        Self {
            tiles: Vec::new(),
            limit,
        }
    }
    pub fn add(&mut self, tile: Tile) -> Option<usize> {
        if self.tiles.len() == self.limit {
            return None;
        }
        let id = self.tiles.len();
        self.tiles.push(tile);
        Some(id)
    }
    pub fn get(&self, id: usize) -> Option<Tile> {
        self.tiles.get(id).copied()
    }
    pub fn replace(&mut self, id: usize, tile: Tile) -> bool {
        if let Some(slot) = self.tiles.get_mut(id) {
            *slot = tile;
            true
        } else {
            false
        }
    }
}
/// Transient cell layers, not a second front/back terminal buffer. Compose into
/// Terrabox Surface; only Terrabox owns dirty-cell tracking and terminal writes.
pub struct Layers {
    size: Size,
    cells: Vec<Vec<Option<Tile>>>,
    max_cells: usize,
}
impl Layers {
    pub fn new(size: Size, max_cells: usize) -> Option<Self> {
        let count = usize::from(size.width) * usize::from(size.height);
        if count > max_cells {
            return None;
        }
        Some(Self {
            size,
            cells: (0..COUNT).map(|_| vec![None; count]).collect(),
            max_cells,
        })
    }
    pub fn size(&self) -> Size {
        self.size
    }
    pub fn storage_bytes(&self) -> usize {
        self.cells
            .iter()
            .map(|v| v.capacity() * std::mem::size_of::<Option<Tile>>())
            .sum()
    }
    /// Failed resize preserves the old surface intact. Caller controls memory cap.
    pub fn resize(&mut self, size: Size) -> bool {
        if self.size == size {
            return true;
        }
        let count = usize::from(size.width) * usize::from(size.height);
        if count > self.max_cells {
            return false;
        }
        self.size = size;
        for cells in &mut self.cells {
            cells.clear();
            cells.resize(count, None);
        }
        true
    }
    pub fn clear(&mut self) {
        for cells in &mut self.cells {
            cells.fill(None);
        }
    }
    pub fn put(&mut self, layer: Layer, x: u16, y: u16, tile: Tile) -> bool {
        if x >= self.size.width || y >= self.size.height {
            return false;
        }
        self.cells[layer as usize]
            [usize::from(y) * usize::from(self.size.width) + usize::from(x)] = Some(tile);
        true
    }
    pub fn put_world(&mut self, layer: Layer, camera: Camera, point: Point, tile: Tile) -> bool {
        camera
            .project(point)
            .is_some_and(|p| self.put(layer, p.x, p.y, tile))
    }
    pub fn compose(&self, target: &mut Surface, background: Style) {
        if target.size() != self.size {
            target.resize(self.size);
        }
        target.clear(background);
        let mut canvas = target.canvas(Rect::new(0, 0, self.size.width, self.size.height));
        for y in 0..self.size.height {
            for x in 0..self.size.width {
                let index = usize::from(y) * usize::from(self.size.width) + usize::from(x);
                if let Some(tile) = self.cells.iter().rev().find_map(|layer| layer[index]) {
                    canvas.put_char(x, y, tile.glyph, tile.style);
                }
            }
        }
    }
}
