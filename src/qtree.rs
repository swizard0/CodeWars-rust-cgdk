use super::geom::{Point, Rect};

pub trait Item {
    fn fits_into(&self, rect: &Rect) -> bool;
}

#[derive(Clone, PartialEq, Debug)]
pub struct QuadTree<T> {
    rect: Rect,
    root: Node<T>,
}

#[derive(Clone, PartialEq, Debug)]
struct QuarterRect<T> {
    rect: Rect,
    node: Option<Box<Node<T>>>,
}

#[derive(Clone, PartialEq, Debug)]
struct Node<T> {
    children: [QuarterRect<T>; 4],
    items: Vec<T>,
}

impl<T> QuadTree<T> {
    pub fn new(area: Rect) -> QuadTree<T> {
        QuadTree {
            root: Node::new(&area),
            rect: area,
        }
    }
}

// impl<T> QuadTree<T> where T: Item {
//     pub fn insert(&mut self, item: T) -> bool {
//         self.root.insert(item)
//     }
// }

impl<T> Node<T> {
    fn new(parent: &Rect) -> Node<T> {
        let mid_x = parent.mid_x();
        let mid_y = parent.mid_y();
        Node {
            children: [
                QuarterRect {
                    rect: Rect { lt: parent.lt, rb: Point { x: mid_x, y: mid_y, }, },
                    node: None,
                },
                QuarterRect {
                    rect: Rect { lt: Point { x: mid_x, y: parent.top(), }, rb: Point { x: parent.right(), y: mid_y, }, },
                    node: None,
                },
                QuarterRect {
                    rect: Rect { lt: Point { x: mid_x, y: mid_y, }, rb: parent.rb, },
                    node: None,
                },
                QuarterRect {
                    rect: Rect { lt: Point { x: parent.left(), y: mid_y, }, rb: Point { x: mid_y, y: parent.bottom(), } },
                    node: None,
                },
            ],
            items: Vec::new(),
        }
    }
}

// impl<T> Node<T> where T: Item {
//     pub fn insert(&mut self, item: T) -> bool {
//         unimplemented!()
//     }
// }

#[cfg(test)]
mod test {
    use super::super::geom::{Point, Rect};
    use super::{QuadTree, QuarterRect, Node};

    #[test]
    fn make_new() {
        let tree: QuadTree<()> =
            QuadTree::new(Rect { lt: Point { x: 0., y: 0., }, rb: Point { x: 100., y: 100., }, });
        assert_eq!(tree, QuadTree {
            rect: Rect { lt: Point { x: 0., y: 0., }, rb: Point { x: 100., y: 100., }, },
            root: Node {
                children: [
                    QuarterRect { rect: Rect { lt: Point { x: 0., y: 0., }, rb: Point { x: 50., y: 50., } }, node: None, },
                    QuarterRect { rect: Rect { lt: Point { x: 50., y: 0., }, rb: Point { x: 100., y: 50., } }, node: None, },
                    QuarterRect { rect: Rect { lt: Point { x: 50., y: 50., }, rb: Point { x: 100., y: 100., } }, node: None, },
                    QuarterRect { rect: Rect { lt: Point { x: 0., y: 50., }, rb: Point { x: 50., y: 100., } }, node: None, },
                ],
                items: vec![],
            }
        });

    }
}
