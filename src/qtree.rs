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

    pub fn lookup<'q, 'a>(&'a self, area: Rect, queue: &'q mut Vec<QuarterRectRef<'a, T>>) -> LookupIter<'q, 'a, T> {
        queue.clear();
        queue.push(QuarterRectRef { rect: &self.rect, node: &self.root, });
        LookupIter { area, queue, items_it: None, }
    }
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

pub struct QuarterRectRef<'a, T: 'a> {
    rect: &'a Rect,
    node: &'a Node<T>,
}

pub struct LookupIter<'q, 'a: 'q, T: 'a> {
    area: Rect,
    queue: &'q mut Vec<QuarterRectRef<'a, T>>,
    items_it: Option<::std::slice::Iter<'a, T>>,
}

impl<'q, 'a, T> Iterator for LookupIter<'q, 'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            if let Some(ref mut iter) = self.items_it {
                if let Some(item) = iter.next() {
                    return Some(item);
                }
            }
            self.items_it = None;
            while let Some(QuarterRectRef { rect, node, }) = self.queue.pop() {
                if !rect.intersects(&self.area) {
                    continue;
                }
                for qr in node.children.iter() {
                    if let Some(ref node) = qr.node {
                        self.queue.push(QuarterRectRef { rect: &qr.rect, node, });
                    }
                }
                self.items_it = Some(node.items.iter());
                continue 'outer;
            }
            return None;
        }
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
        tree.insert(&rt(40., 40., 60., 60.), "center");
        assert_eq!(tree.root.items, vec!["center"]);
        tree.insert(&rt(70., 70., 80., 80.), "south-east A");
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some(["south-east A"].as_ref())
        );
        tree.insert(&rt(85., 60., 95., 70.), "south-east, north-east");
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .and_then(|n| n.children.get(1))
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some(["south-east, north-east"].as_ref())
        );
        tree.insert(&rt(60., 74., 90., 76.), "south-east B");
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some(["south-east A", "south-east B"].as_ref())
        );
    }

    #[test]
    fn lookup() {
        let mut tree = QuadTree::new(rt(0., 0., 100., 100.));
        tree.insert(&rt(40., 40., 60., 60.), "center");
        tree.insert(&rt(70., 70., 80., 80.), "south-east A");
        tree.insert(&rt(85., 60., 95., 70.), "south-east, north-east");
        tree.insert(&rt(60., 74., 90., 76.), "south-east B");
        tree.insert(&rt(1., 1., 4., 4.), "north-west");

        let mut queue = Vec::new();
        assert_eq!(
            tree.lookup(rt(10., 10., 90., 90.), &mut queue).collect::<Vec<_>>(),
            vec![&"center", &"south-east A", &"south-east B", &"south-east, north-east"]
        );
        assert_eq!(
            tree.lookup(rt(10., 10., 20., 20.), &mut queue).collect::<Vec<_>>(),
            vec![&"center"]
        );
        assert_eq!(
            tree.lookup(rt(59., 69., 74., 81.), &mut queue).collect::<Vec<_>>(),
            vec![&"center", &"south-east A", &"south-east B"]
        );
        assert_eq!(
            tree.lookup(rt(2., 2., 3., 3.), &mut queue).collect::<Vec<_>>(),
            vec![&"center", &"north-west"]
        );
    }
}
