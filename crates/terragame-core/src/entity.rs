use crate::queue::EventQueue;
/// A store-local identity. Never mix IDs from different stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId {
    slot: u32,
    generation: u64,
}
impl EntityId {
    pub fn slot(self) -> u32 {
        self.slot
    }
    pub fn generation(self) -> u64 {
        self.generation
    }
}
struct Slot<T> {
    generation: u64,
    value: Option<T>,
}
/// Slot order is deterministic. Slots are reused LIFO; generations never wrap.
pub struct EntityStore<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    limit: u32,
    len: usize,
}
impl<T> EntityStore<T> {
    pub fn new(limit: u32) -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            limit,
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn allocated_slots(&self) -> usize {
        self.slots.len()
    }
    /// Payload heap allocations are owned by T and excluded from this estimate.
    pub fn storage_bytes(&self) -> usize {
        self.slots.capacity() * std::mem::size_of::<Slot<T>>()
            + self.free.capacity() * std::mem::size_of::<u32>()
    }
    pub fn spawn(&mut self, value: T) -> Result<EntityId, T> {
        let index = if let Some(index) = self.free.pop() {
            index
        } else {
            if self.slots.len() >= self.limit as usize {
                return Err(value);
            }
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                value: None,
            });
            index
        };
        let slot = &mut self.slots[index as usize];
        slot.value = Some(value);
        self.len += 1;
        Ok(EntityId {
            slot: index,
            generation: slot.generation,
        })
    }
    pub fn get(&self, id: EntityId) -> Option<&T> {
        let slot = self.slots.get(id.slot as usize)?;
        (slot.generation == id.generation)
            .then_some(slot.value.as_ref())
            .flatten()
    }
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut T> {
        let slot = self.slots.get_mut(id.slot as usize)?;
        if slot.generation == id.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }
    pub fn remove(&mut self, id: EntityId) -> Option<T> {
        let slot = self.slots.get_mut(id.slot as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let value = slot.value.take()?;
        self.len -= 1;
        // An exhausted generation retires the slot permanently.
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(id.slot);
        }
        Some(value)
    }
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value.as_ref().map(|value| {
                (
                    EntityId {
                        slot: index as u32,
                        generation: slot.generation,
                    },
                    value,
                )
            })
        })
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value.as_mut().map(|value| {
                    (
                        EntityId {
                            slot: index as u32,
                            generation: slot.generation,
                        },
                        value,
                    )
                })
            })
    }
}
pub enum Command<T> {
    Spawn(T),
    Remove(EntityId),
}
pub enum Applied<T> {
    Spawned(EntityId),
    Removed(EntityId, T),
    RejectedSpawn(T),
    Stale(EntityId),
}
pub struct Commands<T>(EventQueue<Command<T>>);
impl<T> Commands<T> {
    pub fn new(limit: usize) -> Self {
        Self(EventQueue::new(limit))
    }
    pub fn push(&mut self, command: Command<T>) -> Result<(), Command<T>> {
        self.0.push(command)
    }
    /// Apply at a system boundary after iteration. No silently dropped payloads.
    pub fn apply(&mut self, store: &mut EntityStore<T>, mut report: impl FnMut(Applied<T>)) {
        for command in self.0.drain() {
            report(match command {
                Command::Spawn(value) => match store.spawn(value) {
                    Ok(id) => Applied::Spawned(id),
                    Err(value) => Applied::RejectedSpawn(value),
                },
                Command::Remove(id) => match store.remove(id) {
                    Some(value) => Applied::Removed(id, value),
                    None => Applied::Stale(id),
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_generations_retire_without_aliasing() {
        let mut store = EntityStore::new(1);
        let id = store.spawn(7).unwrap();
        store.slots[id.slot as usize].generation = u64::MAX;
        let final_id = EntityId {
            slot: id.slot,
            generation: u64::MAX,
        };
        assert_eq!(store.remove(final_id), Some(7));
        assert!(store.get(final_id).is_none());
        assert_eq!(store.spawn(8), Err(8));
        assert_eq!(store.len(), 0);
    }
}
