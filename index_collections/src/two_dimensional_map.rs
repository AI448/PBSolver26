use std::{cell::UnsafeCell, collections::hash_map, ptr::NonNull};

use fxhash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug, std::marker::ConstParamTy)]
enum Axis {
    Row = 0,
    Column = 1,
}

impl Axis {
    #[inline(always)]
    fn iter() -> core::array::IntoIter<Self, 2> {
        [Self::Row, Self::Column].into_iter()
    }
}

impl<T> std::ops::Index<Axis> for [T; 2] {
    type Output = T;
    #[inline(always)]
    fn index(&self, index: Axis) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> std::ops::IndexMut<Axis> for [T; 2] {
    #[inline(always)]
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl std::ops::Not for Axis {
    type Output = Axis;
    #[inline(always)]
    fn not(self) -> Self::Output {
        [Axis::Column, Axis::Row][self]
    }
}

pub struct TwoDimensionalMap<ValueT> {
    hash_map: FxHashMap<[usize; 2], Box<UnsafeCell<Item<ValueT>>>>,
    headers: [Vec<Header<ValueT>>; 2],
    empty_items: Vec<Box<UnsafeCell<Item<ValueT>>>>,
}

#[derive(Clone)]
struct Link<ValueT> {
    previous: Option<NonNull<Item<ValueT>>>,
    next: Option<NonNull<Item<ValueT>>>,
}

impl<ValueT> Default for Link<ValueT> {
    fn default() -> Self {
        Self {
            previous: None,
            next: None,
        }
    }
}

struct Item<ValueT> {
    links: [Link<ValueT>; 2],
    index: [usize; 2],
    value: ValueT,
}

#[derive(Debug)]
struct Header<ValueT> {
    first: Option<NonNull<Item<ValueT>>>,
    last: Option<NonNull<Item<ValueT>>>,
    len: usize,
}

impl<ValueT> Default for Header<ValueT> {
    fn default() -> Self {
        Header {
            first: None,
            last: None,
            len: 0,
        }
    }
}

impl<ValueT> Default for TwoDimensionalMap<ValueT> {
    fn default() -> Self {
        Self {
            hash_map: FxHashMap::default(),
            headers: [Vec::default(), Vec::default()],
            empty_items: Vec::default(),
        }
    }
}

impl<ValueT> Clone for TwoDimensionalMap<ValueT>
where
    ValueT: Clone,
{
    fn clone(&self) -> Self {
        Self::from_iter(self.iter().map(|(index, value)| (index, value.clone())))
    }
}

impl<ValueT> TwoDimensionalMap<ValueT> {
    pub fn from_iter(elements: impl Iterator<Item = ([usize; 2], ValueT)>) -> Self {
        let mut map = Self::default();
        map.extend(elements);
        map
    }

    #[inline(always)]
    pub fn capacity(&self) -> [usize; 2] {
        [
            self.headers[Axis::Row].len(),
            self.headers[Axis::Column].len(),
        ]
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = ([usize; 2], &ValueT)> + Clone {
        self.hash_map.iter().map(|(_, boxed)| {
            let item = unsafe { boxed.as_ref_unchecked() };
            (
                [item.index[Axis::Row], item.index[Axis::Column]],
                &item.value,
            )
        })
    }

    #[inline(always)]
    pub fn iter_row(&self, i: usize) -> impl DoubleEndedIterator<Item = (usize, &ValueT)> + Clone {
        Iter::<{ Axis::Row }, ValueT>::new(&self, i)
    }

    #[inline(always)]
    pub fn iter_column(
        &self,
        j: usize,
    ) -> impl DoubleEndedIterator<Item = (usize, &ValueT)> + Clone {
        Iter::<{ Axis::Column }, ValueT>::new(&self, j)
    }

    #[inline(always)]
    pub fn get(&self, index: [usize; 2]) -> Option<&ValueT> {
        self.hash_map
            .get(&index)
            .map(|boxed_item| &unsafe { boxed_item.as_ref_unchecked() }.value)
    }

    #[inline(always)]
    pub fn get_mut(&mut self, index: [usize; 2]) -> Option<&mut ValueT> {
        self.hash_map
            .get_mut(&index)
            .map(|boxed_item| &mut boxed_item.get_mut().value)
    }

    pub fn insert(&mut self, index: [usize; 2], value: ValueT) {
        // 必要に応じてヘッダを拡張
        for axis in Axis::iter() {
            if index[axis] >= self.headers[axis].len() {
                self.headers[axis]
                    .resize_with(1 << usize::bit_width(index[axis]), || Header::default());
            }
            debug_assert!(index[axis] < self.headers[axis].len());
        }

        match self.hash_map.entry(index) {
            hash_map::Entry::Occupied(mut occupied) => {
                // 要素が存在する場合

                // 要素の value を更新する
                let item = occupied.get_mut().get_mut();
                item.value = value
            }
            hash_map::Entry::Vacant(vacant) => {
                // 要素が存在しない場合

                let links = [Axis::Row, Axis::Column].map(|axis| Link {
                    previous: self.headers[axis][index[axis]].last,
                    next: None,
                });

                // 要素を取得
                let mut boxed_item = if let Some(mut boxed_item) = self.empty_items.pop() {
                    // 空き要素が存在するならそれを再利用
                    let item = boxed_item.get_mut();
                    item.links = links;
                    item.index = index;
                    item.value = value;
                    boxed_item
                } else {
                    // 存在しないなら新たに構築
                    Box::new(UnsafeCell::new(Item {
                        links,
                        index,
                        value,
                    }))
                };

                // リンクを接続
                {
                    let item = boxed_item.get_mut();
                    for axis in Axis::iter() {
                        let header = &mut self.headers[axis][index[axis]];
                        // item 側のリンクは既に設定されているはず
                        debug_assert!(item.links[axis].previous == header.last);
                        debug_assert!(item.links[axis].next == None);

                        header.len += 1;
                        match header.last {
                            None => {
                                debug_assert!(header.first.is_none());
                                header.first = NonNull::new(item);
                            }
                            Some(mut last) => {
                                unsafe { last.as_mut() }.links[axis].next = NonNull::new(item);
                            }
                        }
                        header.last = NonNull::new(item);
                    }
                }

                // ハッシュテーブルに追加
                vacant.insert(boxed_item);
            }
        }
    }

    pub fn extend(&mut self, elements: impl Iterator<Item = ([usize; 2], ValueT)>) {
        for (index, value) in elements {
            self.insert(index, value);
        }
    }

    pub fn remove(&mut self, ij: [usize; 2]) {
        let Some(mut boxed) = self.hash_map.remove(&ij) else {
            return;
        };
        {
            let item = boxed.get_mut();
            for axis in Axis::iter() {
                let index = ij[axis];
                let header = &mut self.headers[axis][index as usize];
                header.len -= 1;
                match item.links[axis].previous {
                    None => {
                        debug_assert!(header.first.unwrap().as_ptr() == item as *mut Item<ValueT>);
                        header.first = item.links[axis].next;
                    }
                    Some(previous) => {
                        let previous_item = unsafe { &mut *previous.as_ptr() };
                        debug_assert!(previous_item.index[axis] == index);
                        debug_assert!(
                            previous_item.links[axis].next.unwrap().as_ptr()
                                == item as *mut Item<ValueT>
                        );
                        previous_item.links[axis].next = item.links[axis].next;
                        item.links[axis].previous = None;
                    }
                }
                match item.links[axis].next {
                    None => {
                        debug_assert!(header.last.unwrap().as_ptr() == item as *mut Item<ValueT>);
                        header.last = item.links[axis].previous;
                    }
                    Some(next) => {
                        let next_item = unsafe { &mut *next.as_ptr() };
                        debug_assert!(next_item.index[axis] == ij[axis]);
                        debug_assert!(
                            next_item.links[axis].previous.unwrap().as_ptr()
                                == item as *mut Item<ValueT>
                        );
                        next_item.links[axis].previous = item.links[axis].previous;
                        item.links[axis].next = None;
                    }
                }
            }
        }
        self.empty_items.push(boxed);
    }

    pub fn clear_row(&mut self, i: usize) {
        if i >= self.headers[Axis::Row].len() {
            return;
        }
        while self.headers[Axis::Row][i].len != 0 {
            let index = {
                let pointer = self.headers[Axis::Row][i].last.unwrap();
                unsafe { pointer.as_ref() }.index
            };
            debug_assert!(index[Axis::Row] == i);
            self.remove(index);
        }
        debug_assert!(self.headers[Axis::Row][i].first == None);
        debug_assert!(self.headers[Axis::Row][i].last == None);
        debug_assert!(self.headers[Axis::Row][i].len == 0);
    }

    pub fn clear_column(&mut self, j: usize) {
        if j >= self.headers[Axis::Column].len() {
            return;
        }
        while self.headers[Axis::Column][j].len != 0 {
            let index = {
                let pointer = self.headers[Axis::Column][j].last.unwrap();
                unsafe { pointer.as_ref() }.index
            };
            debug_assert!(index[Axis::Column] == j);
            self.remove(index);
        }
        debug_assert!(self.headers[Axis::Column][j].first == None);
        debug_assert!(self.headers[Axis::Column][j].last == None);
        debug_assert!(self.headers[Axis::Column][j].len == 0);
    }

    pub fn clear(&mut self) {
        for axis in Axis::iter() {
            self.headers[axis].clear();
        }
        self.hash_map.clear();
    }
}

struct Iter<'a, const A: Axis, T> {
    map: &'a TwoDimensionalMap<T>,
    index: usize,
    current: Option<NonNull<Item<T>>>,
}

impl<'a, const A: Axis, ValueT> Clone for Iter<'a, A, ValueT> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            map: self.map,
            index: self.index,
            current: self.current,
        }
    }
}

impl<'a, const A: Axis, ValueT> Iter<'a, A, ValueT> {
    #[inline(always)]
    fn new(map: &'a TwoDimensionalMap<ValueT>, index: usize) -> Self {
        Self {
            map,
            index,
            current: None,
        }
    }
}

impl<'a, const A: Axis, ValueT> std::iter::Iterator for Iter<'a, A, ValueT> {
    type Item = (usize, &'a ValueT);
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.current = match self.current {
            None => self.map.headers[A][self.index].first,
            Some(current) => unsafe { &*current.as_ptr() }.links[A].next,
        };
        return self.current.as_ref().map(|&current| {
            let item = unsafe { current.as_ref() };
            debug_assert!(item.index[A] == self.index);
            (item.index[!A], &item.value)
        });
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.map.headers[A][self.index].len;
        return (len, Some(len));
    }
}

impl<'a, const A: Axis, ValueT> std::iter::DoubleEndedIterator for Iter<'a, A, ValueT> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.current = match self.current {
            None => self.map.headers[A][self.index].last,
            Some(current) => unsafe { &*current.as_ptr() }.links[A].previous,
        };
        return self.current.as_ref().map(|&current| {
            let item = unsafe { current.as_ref() };
            debug_assert!(item.index[A] == self.index);
            (item.index[!A], &item.value)
        });
    }
}
