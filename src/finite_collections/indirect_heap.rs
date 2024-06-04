use super::size::Size;
use super::Array;

#[inline(always)]
fn parent_of<SizeT>(position: SizeT) -> SizeT
where
    SizeT: Size,
{
    debug_assert!(position != SizeT::zero());
    (position + unsafe { SizeT::from(1).unwrap_unchecked() }) / unsafe { SizeT::from(2).unwrap_unchecked() }
        - unsafe { SizeT::from(1).unwrap_unchecked() }
}

#[inline(always)]
fn left_of<SizeT>(position: SizeT) -> SizeT
where
    SizeT: Size,
{
    (position + unsafe { SizeT::from(1).unwrap_unchecked() }) * unsafe { SizeT::from(2).unwrap_unchecked() }
        - unsafe { SizeT::from(1).unwrap_unchecked() }
}

#[inline(always)]
fn right_of<SizeT>(position: SizeT) -> SizeT
where
    SizeT: Size,
{
    (position + unsafe { SizeT::from(1).unwrap_unchecked() }) * unsafe { SizeT::from(2).unwrap_unchecked() }
}

#[inline(never)]
pub fn update_heap<SizeT, ValueT, LessT>(
    heap_array: &mut Array<SizeT, (SizeT, ValueT)>,
    position_array: &mut Array<SizeT, SizeT>,
    position: SizeT,
    less: &LessT,
) where
    SizeT: Size,
    LessT: std::ops::Fn(&(SizeT, ValueT), &(SizeT, ValueT)) -> bool,
{
    if position != SizeT::zero() && less(&heap_array[position], &heap_array[parent_of(position)]) {
        up_heap(heap_array, position_array, position, less);
    } else {
        down_heap(heap_array, position_array, position, less);
    }
}

#[inline(never)]
pub fn up_heap<SizeT, ValueT, LessT>(
    heap_array: &mut Array<SizeT, (SizeT, ValueT)>,
    position_array: &mut Array<SizeT, SizeT>,
    mut position: SizeT,
    less: &LessT,
) where
    SizeT: Size,
    LessT: std::ops::Fn(&(SizeT, ValueT), &(SizeT, ValueT)) -> bool,
{
    loop {
        debug_assert!(position_array[heap_array[position].0] == position);
        if position == SizeT::zero() {
            break;
        }
        let parent = parent_of(position);
        debug_assert!(position_array[heap_array[parent].0] == parent);
        if less(&heap_array[position], &heap_array[parent]) {
            heap_array.swap(parent, position);
            position_array.swap(heap_array[parent].0, heap_array[position].0);
            position = parent;
        } else {
            break;
        }
    }
}

#[inline(never)]
pub fn down_heap<SizeT, ValueT, LessT>(
    heap_array: &mut Array<SizeT, (SizeT, ValueT)>,
    position_array: &mut Array<SizeT, SizeT>,
    mut position: SizeT,
    less: &LessT,
) where
    SizeT: Size,
    LessT: std::ops::Fn(&(SizeT, ValueT), &(SizeT, ValueT)) -> bool,
{
    loop {
        debug_assert!(position_array[heap_array[position].0] == position);
        let left = left_of(position);
        debug_assert!(left >= heap_array.len() || position_array[heap_array[left].0] == left);
        let right = right_of(position);
        debug_assert!(right >= heap_array.len() || position_array[heap_array[right].0] == right);
        if left >= heap_array.len() {
            break;
        }
        let smaller_child =
            if right >= heap_array.len() || less(&heap_array[left], &heap_array[right]) { left } else { right };
        if less(&heap_array[smaller_child], &heap_array[position]) {
            heap_array.swap(position, smaller_child);
            position_array.swap(heap_array[position].0, heap_array[smaller_child].0);
            position = smaller_child;
        } else {
            break;
        }
    }
}
