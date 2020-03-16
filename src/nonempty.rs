use nonempty::NonEmpty;

/// Split the last element out of a `NonEmpty` list.
pub fn split_last<T>(non_empty: &NonEmpty<T>) -> (Vec<T>, T)
where
    T: Clone + Eq,
{
    let (first, middle, last) = non_empty.split();

    // first == last, so drop first
    if first == last && middle.is_empty() {
        (vec![], last.clone())
    } else {
        // Create the prefix vector
        let mut vec = vec![first.clone()];
        let mut middle = middle.to_vec();
        vec.append(&mut middle);
        (vec, last.clone())
    }
}
