use super::geom::{Point, Rect};

pub trait FitsChecker<T> {
    fn fits_into(&mut self, rect: &Rect, item: &T) -> bool;
}

impl<F, T> FitsChecker<T> for F where F: FnMut(&Rect, &T) -> bool {
    fn fits_into(&mut self, rect: &Rect, item: &T) -> bool {
        (self)(rect, item)
    }
}

#[derive(Clone, Debug)]
pub struct QuadTree<T, F> {
    rect: Rect,
    root: Node<T>,
    fits_checker: F,
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

impl<T, F> QuadTree<T, F> {
    pub fn new(area: Rect, fits_checker: F) -> QuadTree<T, F> {
        QuadTree {
            root: Node::new(&area),
            rect: area,
            fits_checker,
        }
    }
}

impl<T, F> QuadTree<T, F> where F: FitsChecker<T> {
    pub fn insert(&mut self, item: T) {
        self.root.insert(item, &mut self.fits_checker)
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

    fn insert<F>(&mut self, item: T, fits_checker: &mut F) where F: FitsChecker<T> {
        for &mut QuarterRect { ref rect, ref mut node, } in self.children.iter_mut() {
            if fits_checker.fits_into(rect, &item) {
                let child = node.get_or_insert_with(|| Box::new(Node::new(rect)));
                return child.insert(item, fits_checker);
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
        let tree: QuadTree<(), _> = QuadTree::new(rt(0., 0., 100., 100.), ());
        assert_eq!(tree.rect, rt(0., 0., 100., 100.));
        assert_eq!(tree.root.children[0], QuarterRect { rect: rt(0., 0., 50., 50.), node: None, });
        assert_eq!(tree.root.children[1], QuarterRect { rect: rt(50., 0., 100., 50.), node: None, });
        assert_eq!(tree.root.children[2], QuarterRect { rect: rt(50., 50., 100., 100.), node: None, });
        assert_eq!(tree.root.children[3], QuarterRect { rect: rt(0., 50., 50., 100.), node: None, });
    }

    #[test]
    fn insert() {
        let mut tree = QuadTree::new(
            rt(0., 0., 100., 100.),
            |rect: &Rect, &Rect { ref lt, ref rb }: &Rect| rect.inside(lt) && rect.inside(rb),
        );
        tree.insert(rt(90., 90., 110., 110.));
        assert_eq!(tree.root.items, vec![rt(90., 90., 110., 110.)]);
        tree.insert(rt(70., 70., 80., 80.));
        assert_eq!(tree.root.children.get(2).and_then(|qr| qr.node.as_ref()).map(|n| &n.items[..]), Some([rt(70., 70., 80., 80.)].as_ref()));
        tree.insert(rt(85., 60., 95., 70.));
        assert_eq!(
            tree.root.children.get(2)
                .and_then(|qr| qr.node.as_ref())
                .and_then(|n| n.children.get(1))
                .and_then(|qr| qr.node.as_ref())
                .map(|n| &n.items[..]),
            Some([rt(85., 60., 95., 70.)].as_ref())
        );
    }
}
