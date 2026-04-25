#[inline(always)]
fn parent_of(position: usize) -> usize {
    debug_assert!(position != 0);
    return (position + 1) / 2 - 1;
}

#[inline(always)]
fn left_of(position: usize) -> usize {
    return (position + 1) * 2 - 1;
}

#[inline(always)]
fn right_of(position: usize) -> usize {
    (position + 1) * 2
}

#[inline(always)]
pub fn up_heap<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    less: impl Fn(&ValueT, &ValueT) -> bool,
) {
    up_heap_with_callback(
        array,
        position,
        less,
        #[inline(always)]
        |_, _| (),
    );
}

#[inline(always)]
pub fn down_heap<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    less: impl std::ops::Fn(&ValueT, &ValueT) -> bool,
) {
    down_heap_with_callback(
        array,
        position,
        less,
        #[inline(always)]
        |_, _| (),
    );
}

#[inline(always)]
pub fn update_heap<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    less: impl std::ops::Fn(&ValueT, &ValueT) -> bool,
) {
    update_heap_with_callback(
        array,
        position,
        less,
        #[inline(always)]
        |_, _| (),
    );
}

#[inline(always)]
pub fn update_heap_with_callback<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    less: impl Fn(&ValueT, &ValueT) -> bool,
    callback_swap: impl FnMut(&ValueT, &ValueT),
) {
    assert!(position < array.len());
    if position != 0
        && less(unsafe { array.get_unchecked(position) }, unsafe {
            array.get_unchecked(parent_of(position))
        })
    {
        up_heap_with_callback(array, position, less, callback_swap);
    } else {
        down_heap_with_callback(array, position, less, callback_swap);
    }
}

#[inline(always)]
pub fn up_heap_with_callback<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    less: impl Fn(&ValueT, &ValueT) -> bool,
    mut callback_swap: impl FnMut(&ValueT, &ValueT),
) {
    assert!(position < array.len());
    let mut current = position;
    loop {
        debug_assert!(current < array.len());
        if current == 0 {
            break;
        }
        let parent = parent_of(current);
        debug_assert!(parent < array.len());
        if less(unsafe { array.get_unchecked(current) }, unsafe {
            array.get_unchecked(parent)
        }) {
            unsafe { array.swap_unchecked(parent, current) };
            callback_swap(unsafe { array.get_unchecked(parent) }, unsafe {
                array.get_unchecked(current)
            });
            current = parent;
        } else {
            break;
        }
    }
}

#[inline(always)]
pub fn down_heap_with_callback<ValueT>(
    array: &mut Vec<ValueT>,
    position: usize,
    less: impl std::ops::Fn(&ValueT, &ValueT) -> bool,
    mut callback_swap: impl FnMut(&ValueT, &ValueT),
) {
    assert!(position < array.len());
    let mut current = position;
    loop {
        debug_assert!(current < array.len());
        let left = left_of(current);
        if left >= array.len() {
            break;
        }
        let right = right_of(current);
        let child = if right >= array.len()
            || less(unsafe { array.get_unchecked(left) }, unsafe {
                array.get_unchecked(right)
            }) {
            left
        } else {
            right
        };
        debug_assert!(child < array.len());
        if less(unsafe { array.get_unchecked(child) }, unsafe {
            array.get_unchecked(current)
        }) {
            unsafe { array.swap_unchecked(current, child) };
            callback_swap(unsafe { array.get_unchecked(child) }, unsafe {
                array.get_unchecked(current)
            });
            current = child;
        } else {
            break;
        }
    }
}
