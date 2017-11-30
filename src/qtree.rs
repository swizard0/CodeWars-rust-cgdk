use super::geom::{Point, Rect};

#[derive(Clone, Debug)]
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

    pub fn insert(&mut self, bbox: &Rect, item: T) {
        self.root.insert(bbox, item)
    }

    pub fn lookup<'a>(&'a self, area: Rect) -> LookupIter<'a, T> {
        LookupIter {
            area,
            node: &self.root,
            iter: None,
        }
    }
}

pub struct LookupIter<'a, T: 'a> {
    area: Rect,
    node: &'a Node<T>,
    iter: Option<::std::slice::Iter<'a, T>>,
}

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
                    rect: Rect { lt: Point { x: parent.left(), y: mid_y, }, rb: Point { x: mid_x, y: parent.bottom(), } },
                    node: None,
                },
            ],
            items: Vec::new(),
        }
    }

    fn insert(&mut self, bbox: &Rect, item: T) {
        for &mut QuarterRect { ref rect, ref mut node, } in self.children.iter_mut() {
            if rect.contains(bbox) {
                let child = node.get_or_insert_with(|| Box::new(Node::new(rect)));
                return child.insert(bbox, item);
            }
        }
        self.items.push(item);
    }
}

#[cfg(test)]
mod test {
    use super::super::geom::{axis_x, axis_y, Point, Rect};
    use super::{QuadTree, QuarterRect};

    fn rt(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect { lt: Point { x: axis_x(left), y: axis_y(top), }, rb: Point { x: axis_x(right), y: axis_y(bottom), }, }
    }

    #[test]
    fn make_new() {
        let tree: QuadTree<()> = QuadTree::new(rt(0., 0., 100., 100.));
        assert_eq!(tree.rect, rt(0., 0., 100., 100.));
        assert_eq!(tree.root.children[0], QuarterRect { rect: rt(0., 0., 50., 50.), node: None, });
        assert_eq!(tree.root.children[1], QuarterRect { rect: rt(50., 0., 100., 50.), node: None, });
        assert_eq!(tree.root.children[2], QuarterRect { rect: rt(50., 50., 100., 100.), node: None, });
        assert_eq!(tree.root.children[3], QuarterRect { rect: rt(0., 50., 50., 100.), node: None, });
    }

    #[test]
    fn insert() {
        let mut tree = QuadTree::new(rt(0., 0., 100., 100.));
        let rect = rt(90., 90., 110., 110.);
        tree.insert(&rect, rect.clone());
        assert_eq!(tree.root.items, vec![rt(90., 90., 110., 110.)]);
        let rect = rt(70., 70., 80., 80.);
        tree.insert(&rect, rect.clone());
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some([rt(70., 70., 80., 80.)].as_ref())
        );
        let rect = rt(85., 60., 95., 70.);
        tree.insert(&rect, rect.clone());
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .and_then(|n| n.children.get(1))
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some([rt(85., 60., 95., 70.)].as_ref())
        );
        let rect = rt(60., 74., 90., 76.);
        tree.insert(&rect, rect.clone());
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some([rt(70., 70., 80., 80.), rt(60., 74., 90., 76.)].as_ref())
        );
    }
}
