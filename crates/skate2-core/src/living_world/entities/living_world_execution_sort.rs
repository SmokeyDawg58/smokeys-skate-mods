//! EASTL execution-target quick_sort82e8df98. Comparator is fallible only
//! for host owner/numeric errors; comparisons retain original call order.

pub(crate) fn sort<Target: Copy, E>(items: &mut [Target], less: &mut impl FnMut(Target, Target) -> Result<bool, E>) -> Result<(), E> {
    if items.is_empty() { return Ok(()); }
    let depth = 2 * (usize::BITS - items.len().leading_zeros()) as i32 - 2;
    quick(items, depth, less)?;
    // Native guarded insertion of first28 followed by unguarded insertion.
    // A bounds guard only guards the native sentinel; ordered comparisons
    // and all swaps/shifts are identical, including NaN/equal-distance cases.
    for i in 1..items.len() {
        let value = items[i];
        let mut j = i;
        while j > 0 && less(value, items[j - 1])? {
            items[j] = items[j - 1];
            j -= 1;
        }
        items[j] = value;
    }
    Ok(())
}
fn median<Target: Copy, E>(a: Target, b: Target, c: Target,
    less: &mut impl FnMut(Target, Target) -> Result<bool, E>,
) -> Result<Target, E> {
    Ok(if less(a, b)? {
        if less(b, c)? { b } else if less(a, c)? { c } else { a }
    } else if less(a, c)? { a } else if less(b, c)? { c } else { b })
}
fn quick<Target: Copy, E>(mut items: &mut [Target], mut depth: i32,
    less: &mut impl FnMut(Target, Target) -> Result<bool, E>,
) -> Result<(), E> {
    // quick_sort_impl82d347c8 recurses into the RIGHT partition first.
    while items.len() > 28 && depth > 0 {
        let pivot = median(items[0], items[items.len() / 2], items[items.len() - 1], less)?;
        depth -= 1;
        // get_partition82e66780 copies the pivot before performing swaps.
        let mut low = 0;
        let mut high = items.len();
        let split = loop {
            while less(items[low], pivot)? { low += 1; }
            high -= 1;
            while less(pivot, items[high])? { high -= 1; }
            if low >= high { break low; }
            items.swap(low, high);
            low += 1;
        };
        let (left, right) = items.split_at_mut(split);
        quick(right, depth, less)?;
        items = left;
    }
    if depth == 0 { heap_sort(items, less)?; }
    Ok(())
}
fn heap_sort<Target: Copy, E>(items: &mut [Target], less: &mut impl FnMut(Target, Target) -> Result<bool, E>) -> Result<(), E> {
    if items.len() < 2 { return Ok(()); }
    for parent in (0..=(items.len() - 2) / 2).rev() {
        let value = items[parent];
        adjust(items, parent, parent, value, less)?;
    }
    for end in (1..items.len()).rev() {
        let value = items[end];
        items[end] = items[0];
        adjust(&mut items[..end], 0, 0, value, less)?;
    }
    Ok(())
}
fn adjust<Target: Copy, E>(items: &mut [Target], top: usize, mut hole: usize, value: Target,
    less: &mut impl FnMut(Target, Target) -> Result<bool, E>,
) -> Result<(), E> {
    let mut right = 2 * (hole + 1);
    // adjust_heap82d1eab0 selects children by the target distance comparator,
    // unlike the bucket-proxy specialization's pointer comparison.
    while right < items.len() {
        if less(items[right], items[right - 1])? { right -= 1; }
        items[hole] = items[right];
        hole = right;
        right = 2 * (hole + 1);
    }
    if right == items.len() {
        items[hole] = items[right - 1];
        hole = right - 1;
    }
    // promote_heap82d1a5c8 uses ordered less; NaN does not promote.
    while hole > top {
        let parent = (hole - 1) / 2;
        if !less(items[parent], value)? { break; }
        items[hole] = items[parent];
        hole = parent;
    }
    items[hole] = value;
    Ok(())
}
