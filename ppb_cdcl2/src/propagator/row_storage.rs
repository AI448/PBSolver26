use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowId {
    index: usize,
}

#[derive(Clone)]
pub struct RowStorage<T> {
    slots: Vec<Option<T>>,
    vacant_row_ids: VecDeque<RowId>,
}

impl<T> Default for RowStorage<T> {
    fn default() -> Self {
        Self {
            slots: Vec::default(),
            vacant_row_ids: VecDeque::default(),
        }
    }
}

impl<T> RowStorage<T> {
    pub fn len(&self) -> usize {
        self.slots.len() - self.vacant_row_ids.len()
    }

    pub fn allocate(&mut self, value: T) -> RowId {
        if let Some(row_id) = self.vacant_row_ids.pop_front() {
            debug_assert!(self.slots[row_id.index].is_none());
            self.slots[row_id.index] = Some(value);
            row_id
        } else {
            let row_id = RowId {
                index: self.slots.len(),
            };
            self.slots.push(Some(value));
            row_id
        }
    }

    pub fn deallocate(&mut self, row_id: RowId) {
        if self.slots[row_id.index].is_some() {
            self.slots[row_id.index] = None;
            self.vacant_row_ids.push_back(row_id);
        }
    }

    pub fn get(&self, row_id: RowId) -> Option<&T> {
        self.slots[row_id.index].as_ref()
    }

    pub fn get_mut(&mut self, row_id: RowId) -> Option<&mut T> {
        self.slots[row_id.index].as_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (RowId, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.as_ref().map(|value| (RowId { index }, value)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (RowId, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| slot.as_mut().map(|value| (RowId { index }, value)))
    }
}
