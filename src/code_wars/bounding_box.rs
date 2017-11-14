use std::collections::BinaryHeap;
use super::rect::Rect;

pub struct BoundingBox<T> {
    left: BinaryHeap<Min<T>>,
    top: BinaryHeap<Min<T>>,
    right: BinaryHeap<Max<T>>,
    bottom: BinaryHeap<Max<T>>,
}

impl<T> BoundingBox<T> {
    pub fn new() -> BoundingBox<T> {
        BoundingBox {
            left: BinaryHeap::new(),
            top: BinaryHeap::new(),
            right: BinaryHeap::new(),
            bottom: BinaryHeap::new(),
        }
    }

    pub fn update(&mut self, x: f64, y: f64, radius: f64, v: &T) where T: Clone + PartialEq {
        update_bound(&mut self.left, x - radius, v);
        update_bound(&mut self.top, y - radius, v);
        update_bound(&mut self.right, x + radius, v);
        update_bound(&mut self.bottom, y + radius, v);
    }

    pub fn rect(&self) -> Option<Rect> {
        if let (Some(&Min { coord: left, .. }),
                Some(&Min { coord: top, .. }),
                Some(&Max { coord: right, ..}),
                Some(&Max { coord: bottom, ..})) =
            (self.left.peek(), self.top.peek(), self.right.peek(), self.bottom.peek()) {
                Some(Rect { left, top, right, bottom, })
            } else {
                None
            }
    }
}

fn update_bound<M, T>(heap: &mut BinaryHeap<M>, coord: f64, item: &T)
    where M: Bound<T>, T: Clone + PartialEq
{
    let delete = match heap.peek() {
        Some(v) if v.item() == item => true,
        _ => false,
    };
    if delete {
        heap.pop();
    }
    heap.push(M::new(coord, item.clone()));
}

trait Bound<T>: Ord {
    fn new(coord: f64, item: T) -> Self;
    fn item(&self) -> &T;
}

struct Min<T> {
    coord: f64,
    item: T,
}

impl<T> Bound<T> for Min<T> {
    fn new(coord: f64, item: T) -> Self {
        Min { coord, item, }
    }

    fn item(&self) -> &T {
        &self.item
    }
}

use std::cmp::{PartialEq, PartialOrd, Ord, Ordering};

impl<T> PartialEq for Min<T> {
    fn eq(&self, other: &Min<T>) -> bool {
        self.coord == other.coord
    }
}

impl<T> Eq for Min<T> {}

impl<T> Ord for Min<T> {
    fn cmp(&self, other: &Min<T>) -> Ordering {
        if other.coord < self.coord {
            Ordering::Less
        } else if other.coord > self.coord {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl<T> PartialOrd for Min<T> {
    fn partial_cmp(&self, other: &Min<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Max<T> {
    coord: f64,
    item: T,
}

impl<T> Bound<T> for Max<T> {
    fn new(coord: f64, item: T) -> Self {
        Max { coord, item, }
    }

    fn item(&self) -> &T {
        &self.item
    }
}

impl<T> PartialEq for Max<T> {
    fn eq(&self, other: &Max<T>) -> bool {
        self.coord == other.coord
    }
}

impl<T> Eq for Max<T> {}

impl<T> Ord for Max<T> {
    fn cmp(&self, other: &Max<T>) -> Ordering {
        if self.coord < other.coord {
            Ordering::Less
        } else if self.coord > other.coord {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl<T> PartialOrd for Max<T> {
    fn partial_cmp(&self, other: &Max<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::BoundingBox;
    use super::super::rect::Rect;

    #[test]
    fn empty() {
        let bbox: BoundingBox<()> = BoundingBox::new();
        assert_eq!(bbox.rect(), None);
    }

    #[test]
    fn one() {
        let mut bbox = BoundingBox::new();
        bbox.update(10., 11., 3., &1);
        assert_eq!(bbox.rect(), Some(Rect { left: 7., top: 8., right: 13., bottom: 14., }));
    }

    #[test]
    fn two() {
        let mut bbox = BoundingBox::new();
        bbox.update(10., 11., 3., &1);
        bbox.update(8., 13., 4., &2);
        assert_eq!(bbox.rect(), Some(Rect { left: 4., top: 8., right: 13., bottom: 17., }));
    }

    #[test]
    fn three_update() {
        let mut bbox = BoundingBox::new();
        bbox.update(10., 11., 3., &1);
        bbox.update(8., 13., 4., &2);
        bbox.update(8., 11., 3., &1);
        assert_eq!(bbox.rect(), Some(Rect { left: 4., top: 8., right: 12., bottom: 17., }));
    }
}
