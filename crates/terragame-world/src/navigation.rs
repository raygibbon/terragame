//! Bounded local A*. Callers share an expansion budget across all requests in a tick.
use crate::Point;
use std::collections::{BTreeMap, BinaryHeap};

#[derive(Clone, Debug, Default)]
pub struct LocalPath {
    pub steps: Vec<Point>,
    pub expanded: usize,
}
/// Cardinal moves, stable coordinate ties, no planetary search. The goal must
/// be walkable. Results exclude start; an exhausted search returns no path.
pub fn local_path(
    start: Point,
    goal: Point,
    radius: u64,
    budget: &mut usize,
    mut walkable: impl FnMut(Point) -> bool,
) -> LocalPath {
    let distance = |a: Point, b: Point| a.x.abs_diff(b.x).saturating_add(a.y.abs_diff(b.y));
    let mut result = LocalPath::default();
    if distance(start, goal) > radius || start == goal || *budget == 0 {
        return result;
    }
    let mut open = BinaryHeap::new();
    let mut costs = BTreeMap::new();
    let mut parents = BTreeMap::new();
    costs.insert(start, 0u64);
    open.push(std::cmp::Reverse((distance(start, goal), 0u64, start)));
    while let Some(std::cmp::Reverse((_, cost, current))) = open.pop() {
        if costs.get(&current) != Some(&cost) {
            continue;
        }
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        result.expanded += 1;
        if current == goal {
            let mut p = goal;
            while p != start {
                result.steps.push(p);
                p = parents[&p];
            }
            result.steps.reverse();
            return result;
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let Some(next) = current.offset(dx, dy) else {
                continue;
            };
            if distance(start, next) > radius || !walkable(next) {
                continue;
            }
            let next_cost = cost + 1;
            if costs.get(&next).is_none_or(|&old| next_cost < old) {
                costs.insert(next, next_cost);
                parents.insert(next, current);
                open.push(std::cmp::Reverse((
                    next_cost.saturating_add(distance(next, goal)),
                    next_cost,
                    next,
                )));
            }
        }
    }
    result
}
