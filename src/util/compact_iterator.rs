pub trait CompactIterator: Iterator {
    /// Combines adjacent items into a single range.
    ///
    /// This iterator assumes that the items are already sorted in ascending order.
    fn compact(self, adjacent: fn(&Self::Item, &Self::Item) -> bool) -> Compact<Self>
    where
        Self: Sized,
    {
        Compact::new(self, adjacent)
    }
}

impl<T: Sized> CompactIterator for T where T: Iterator {}

pub struct Compact<T>
where
    T: Iterator,
{
    it: T,
    adjacent: fn(&T::Item, &T::Item) -> bool,
    next_item: Option<T::Item>,
}

impl<T> Compact<T>
where
    T: Iterator,
{
    pub fn new(mut it: T, adjacent: fn(&T::Item, &T::Item) -> bool) -> Self {
        let next_item = it.next();
        Self {
            it,
            adjacent,
            next_item,
        }
    }
}

impl<T> Iterator for Compact<T>
where
    T: Iterator,
    T::Item: Clone,
{
    type Item = (T::Item, T::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let adjacent = self.adjacent;

        let min_item = match self.next_item.take() {
            Some(item) => item,
            None => return None,
        };

        let mut max_item = min_item.clone();
        while let Some(item) = self.it.next() {
            if adjacent(&max_item, &item) {
                max_item = item;
            } else {
                self.next_item = Some(item);
                break;
            }
        }

        Some((min_item, max_item))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compact_should_combine_adjacent_items_into_ranges() {
        // GIVEN
        let items: Vec<usize> = vec![1, 3, 4, 6, 7, 8, 10];

        // WHEN
        let compacted_items: Vec<(usize, usize)> =
            items.into_iter().compact(|&x, &y| x + 1 == y).collect();

        // THEN
        assert_eq!(compacted_items, vec![(1, 1), (3, 4), (6, 8), (10, 10)]);
    }
}
