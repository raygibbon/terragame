use std::collections::VecDeque;
/// FIFO with explicit backpressure. Full queues return ownership to the caller.
pub struct EventQueue<T> {
    events: VecDeque<T>,
    limit: usize,
}
impl<T> EventQueue<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            events: VecDeque::new(),
            limit,
        }
    }
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.events.len() == self.limit {
            Err(value)
        } else {
            self.events.push_back(value);
            Ok(())
        }
    }
    pub fn pop(&mut self) -> Option<T> {
        self.events.pop_front()
    }
    pub fn len(&self) -> usize {
        self.events.len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.events.drain(..)
    }
}
