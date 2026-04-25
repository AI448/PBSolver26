#![feature(adt_const_params)]
#![feature(uint_bit_width)]
#![feature(unsafe_cell_access)]
#![feature(slice_swap_unchecked)]

mod map;
pub use map::Map;

mod set;
pub use set::Set;

mod comparator;
pub use comparator::{
    Comparator, IndexComparator, NaturalComparator, NaturalPartialComparator, ReverseComparator,
    ValueComparator,
};

mod heap_sort;
pub use heap_sort::{
    down_heap, down_heap_with_callback, up_heap, up_heap_with_callback, update_heap,
    update_heap_with_callback,
};

mod heaped_map;
pub use heaped_map::HeapedMap;

mod priority_queue;
pub use priority_queue::PriorityQueue;

mod sorted_map;
pub use sorted_map::{Iterator, SortedMap};

mod two_dimensional_map;
pub use two_dimensional_map::TwoDimensionalMap;

mod sorted_index_map;
pub use sorted_index_map::SortedIndexMap;
