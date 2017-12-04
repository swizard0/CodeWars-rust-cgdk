
pub trait Coord: Sized + PartialOrd {
    fn cut_point<I>(coords: I) -> Self where I: Iterator<Item = Self>;
}

pub trait Point {
    type Axis: Clone;
    type Coord: Coord;

    fn coord(&self, axis: &Self::Axis) -> Self::Coord;
}

pub trait BoundingBox {
    type Point: Point;

    fn min_corner(&self) -> Self::Point;
    fn max_corner(&self) -> Self::Point;
}

pub trait Shape {
    type BoundingBox: BoundingBox;

    fn bounding_box(&self) -> Self::BoundingBox;
    fn cut(
        &self,
        fragment: &Self::BoundingBox,
        cut_axis: &<<Self::BoundingBox as BoundingBox>::Point as Point>::Axis,
        cut_coord: &<<Self::BoundingBox as BoundingBox>::Point as Point>::Coord,
    ) -> Option<(Self::BoundingBox, Self::BoundingBox)>;
}

pub struct KdvTree<P, B, S> where P: Point {
    axis: Vec<P::Axis>,
    shapes: Vec<S>,
    root: KdvNode<P::Axis, P::Coord, B>,
}

impl<P, B, S> KdvTree<P, B, S>
    where P: Point,
          B: BoundingBox<Point = P>,
          S: Shape<BoundingBox = B>,
{
    pub fn build<IA, II>(axis_it: IA, shapes_it: II) -> Option<KdvTree<P, B, S>>
        where IA: IntoIterator<Item = P::Axis>,
              II: IntoIterator<Item = S>
    {
        let axis: Vec<_> = axis_it.into_iter().collect();
        let shapes: Vec<_> = shapes_it.into_iter().collect();
        if shapes.is_empty() {
            None
        } else {
            let root_shapes: Vec<_> = shapes
                .iter()
                .enumerate()
                .map(|(i, s)| ShapeFragment {
                    bounding_box: s.bounding_box(),
                    shape_id: i,
                })
                .collect();
            Some(KdvTree {
                root: KdvNode::build(0, &axis, &shapes, root_shapes),
                axis, shapes,
            })
        }
    }

    pub fn intersects<'t, 's, SQ>(&'t self, shape: &'s SQ) -> IntersectIter<'t, 's, P::Axis, P::Coord, B, S, SQ>
        where SQ: Shape<BoundingBox = B>
    {
        IntersectIter {
            needle: shape,
            axis: &self.axis,
            shapes: &self.shapes,
            queue: vec![(&self.root, shape.bounding_box())],
            iter: None,
        }
    }
}

struct ShapeFragment<B> {
    bounding_box: B,
    shape_id: usize,
}

struct KdvNode<A, C, B> {
    cut_axis: A,
    cut_coord: C,
    shapes: Vec<ShapeFragment<B>>,
    left: Option<Box<KdvNode<A, C, B>>>,
    right: Option<Box<KdvNode<A, C, B>>>,
}

impl<A, C, B> KdvNode<A, C, B> {
    fn build<S>(depth: usize, axis: &[A], shapes: &[S], mut node_shapes: Vec<ShapeFragment<B>>) -> KdvNode<A, C, B>
        where S: Shape<BoundingBox = B>,
              B: BoundingBox,
              B::Point: Point<Axis = A, Coord = C>,
              A: Clone,
              C: Coord,
    {
        // locate cut point for coords
        let cut_axis = &axis[depth % axis.len()];
        let cut_coord = Coord::cut_point(
            node_shapes
                .iter()
                .flat_map(|sf| {
                    let bbox = &sf.bounding_box;
                    let min_coord = bbox.min_corner().coord(cut_axis);
                    let max_coord = bbox.max_corner().coord(cut_axis);
                    Some(min_coord).into_iter().chain(Some(max_coord).into_iter())
                })
        );

        // distribute shapes among children
        let mut left_shapes = Vec::new();
        let mut right_shapes = Vec::new();
        let mut head = 0;
        while head < node_shapes.len() {
            let ShapeFragment { shape_id, bounding_box, } = node_shapes.swap_remove(head);
            let owner = shape_owner(&shapes[shape_id], bounding_box, cut_axis, &cut_coord);
            match owner {
                ShapeOwner::Me(bounding_box) => {
                    let tail = node_shapes.len();
                    node_shapes.push(ShapeFragment { shape_id, bounding_box, });
                    node_shapes.swap(head, tail);
                    head += 1;
                },
                ShapeOwner::Left(bounding_box) =>
                    left_shapes.push(ShapeFragment { shape_id, bounding_box, }),
                ShapeOwner::Right(bounding_box) =>
                    right_shapes.push(ShapeFragment { shape_id, bounding_box, }),
                ShapeOwner::Both { left_bbox, right_bbox, } => {
                    left_shapes.push(ShapeFragment { shape_id, bounding_box: left_bbox, });
                    right_shapes.push(ShapeFragment { shape_id, bounding_box: right_bbox, });
                },
            }
        }

        // construct the node
        KdvNode {
            left: if left_shapes.is_empty() {
                None
            } else {
                Some(Box::new(KdvNode::build(depth + 1, axis, shapes, left_shapes)))
            },
            right: if right_shapes.is_empty() {
                None
            } else {
                Some(Box::new(KdvNode::build(depth + 1, axis, shapes, right_shapes)))
            },
            cut_axis: cut_axis.clone(),
            cut_coord: cut_coord,
            shapes: node_shapes,
        }
    }
}

enum ShapeOwner<B> {
    Me(B),
    Left(B),
    Right(B),
    Both { left_bbox: B, right_bbox: B, },
}

fn shape_owner<A, C, B, S>(shape: &S, fragment: B, cut_axis: &A, cut_coord: &C) -> ShapeOwner<B>
    where A: Clone,
          C: Coord,
          B: BoundingBox,
          B::Point: Point<Axis = A, Coord = C>,
          S: Shape<BoundingBox = B>,
{
    let min_coord = fragment.min_corner().coord(cut_axis);
    let max_coord = fragment.max_corner().coord(cut_axis);
    if min_coord.lt(cut_coord) && max_coord.le(cut_coord) {
        ShapeOwner::Left(fragment)
    } else if min_coord.ge(cut_coord) && max_coord.gt(cut_coord) {
        ShapeOwner::Right(fragment)
    } else if let Some((left_bbox, right_bbox)) = shape.cut(&fragment, cut_axis, cut_coord) {
        ShapeOwner::Both { left_bbox, right_bbox, }
    } else {
        ShapeOwner::Me(fragment)
    }
}

pub struct IntersectIter<'t, 's, A: 't, C: 't, B: 't, ST: 't, SQ: 's> {
    needle: &'s SQ,
    axis: &'t [A],
    shapes: &'t [ST],
    queue: Vec<(&'t KdvNode<A, C, B>, B)>,
    iter: Option<(ShapeOwner<B>, (Option<&'t KdvNode<A, C, B>>, Option<&'t KdvNode<A, C, B>>), ::std::slice::Iter<'t, ShapeFragment<B>>)>,
}

impl<'t, 's, A, C, B, ST, SQ> Iterator for IntersectIter<'t, 's, A, C, B, ST, SQ>
    where A: Clone,
          C: Coord,
          B: BoundingBox + Clone,
          B::Point: Point<Axis = A, Coord = C>,
          ST: Shape<BoundingBox = B>,
          SQ: Shape<BoundingBox = B>,
{
    type Item = (&'t ST, &'t B);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((needle_owner, children, mut it)) = self.iter.take() {
                while let Some(sf) = it.next() {
                    let no_intersection = {
                        let mut needles_it = match needle_owner {
                            ShapeOwner::Me(ref bbox) | ShapeOwner::Left(ref bbox) | ShapeOwner::Right(ref bbox) =>
                                Some(bbox).into_iter().chain(None.into_iter()),
                            ShapeOwner::Both { ref left_bbox, ref right_bbox, } =>
                                Some(left_bbox).into_iter().chain(Some(right_bbox).into_iter()),
                        };
                        needles_it
                            .all(|needle_fragment| {
                                self.axis
                                    .iter()
                                    .any(|axis| {
                                        let needle_min = needle_fragment.min_corner().coord(axis);
                                        let needle_max = needle_fragment.max_corner().coord(axis);
                                        let shape_min = sf.bounding_box.min_corner().coord(axis);
                                        let shape_max = sf.bounding_box.max_corner().coord(axis);
                                        needle_min > shape_max || needle_max < shape_min
                                    })
                            })
                    };
                    if no_intersection {
                        continue;
                    }
                    let shape = &self.shapes[sf.shape_id];
                    self.iter = Some((needle_owner, children, it));
                    return Some((shape, &sf.bounding_box));
                }
                match (needle_owner, children) {
                    (ShapeOwner::Me(fragment), (maybe_left, maybe_right)) => {
                        if let Some(node) = maybe_left {
                            self.queue.push((node, fragment.clone()));
                        }
                        if let Some(node) = maybe_right {
                            self.queue.push((node, fragment));
                        }
                    },
                    (ShapeOwner::Left(fragment), (maybe_left, _)) => {
                        if let Some(node) = maybe_left {
                            self.queue.push((node, fragment));
                        }
                    },
                    (ShapeOwner::Right(fragment), (_, maybe_right)) => {
                        if let Some(node) = maybe_right {
                            self.queue.push((node, fragment));
                        }
                    },
                    (ShapeOwner::Both { left_bbox, right_bbox, }, (maybe_left, maybe_right)) => {
                        if let Some(node) = maybe_left {
                            self.queue.push((node, left_bbox));
                        }
                        if let Some(node) = maybe_right {
                            self.queue.push((node, right_bbox));
                        }
                    },
                }
            }

            if let Some((node, needle_fragment)) = self.queue.pop() {
                let needle_owner =
                    shape_owner(self.needle, needle_fragment, &node.cut_axis, &node.cut_coord);
                self.iter = Some((
                    needle_owner,
                    (node.left.as_ref().map(|b| b.as_ref()), node.right.as_ref().map(|b| b.as_ref())),
                    node.shapes.iter(),
                ));
                continue;
            }

            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::{min, max};
    use super::KdvTree;

    #[derive(PartialEq, PartialOrd, Debug)]
    struct Coord(i32);
    impl super::Coord for Coord {
        fn cut_point<I>(coords: I) -> Self where I: Iterator<Item = Self> {
            let mut total = 0;
            let mut sum = 0;
            for Coord(c) in coords {
                sum += c;
                total += 1;
            }
            Coord(sum / total)
        }
    }

    #[derive(Clone, Debug)]
    enum Axis { X, Y, }

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct Point2d { x: i32, y: i32, }
    impl super::Point for Point2d {
        type Axis = Axis;
        type Coord = Coord;

        fn coord(&self, axis: &Self::Axis) -> Self::Coord {
            Coord(match axis { &Axis::X => self.x, &Axis::Y => self.y, })
        }
    }

    #[derive(PartialEq, Clone, Debug)]
    struct Rect2d { lt: Point2d, rb: Point2d, }
    impl super::BoundingBox for Rect2d {
        type Point = Point2d;

        fn min_corner(&self) -> Self::Point { self.lt }
        fn max_corner(&self) -> Self::Point { self.rb }
    }

    #[derive(PartialEq, Debug)]
    struct Line2d { src: Point2d, dst: Point2d, }
    impl super::Shape for Line2d {
        type BoundingBox = Rect2d;

        fn bounding_box(&self) -> Self::BoundingBox {
            Rect2d {
                lt: Point2d { x: min(self.src.x, self.dst.x), y: min(self.src.y, self.dst.y), },
                rb: Point2d { x: max(self.src.x, self.dst.x), y: max(self.src.y, self.dst.y), },
            }
        }

        fn cut(&self, fragment: &Rect2d, cut_axis: &Axis, &Coord(cut_coord): &Coord) -> Option<(Rect2d, Rect2d)> {
            let bbox = self.bounding_box();
            let (side, x, y) = match cut_axis {
                &Axis::X => if cut_coord >= fragment.lt.x && cut_coord <= fragment.rb.x {
                    let factor = (cut_coord - bbox.lt.x) as f64 / (bbox.rb.x - bbox.lt.x) as f64;
                    (fragment.rb.x - fragment.lt.x, cut_coord, bbox.lt.y + (factor * (bbox.rb.y - bbox.lt.y) as f64) as i32)
                } else {
                    return None;
                },
                &Axis::Y => if cut_coord >= fragment.lt.y && cut_coord <= fragment.rb.y {
                    let factor = (cut_coord - bbox.lt.y) as f64 / (bbox.rb.y - bbox.lt.y) as f64;
                    (fragment.rb.y - fragment.lt.y, bbox.lt.x + (factor * (bbox.rb.x - bbox.lt.x) as f64) as i32, cut_coord)
                } else {
                    return None;
                },
            };
            if side < 10 {
                None
            } else {
                Some((Rect2d { lt: fragment.lt, rb: Point2d { x, y, } }, Rect2d { lt: Point2d { x, y, }, rb: fragment.rb, }))
            }
        }
    }

    #[test]
    fn kdv_tree_basic() {
        let shapes = vec![Line2d { src: Point2d { x: 16, y: 16, }, dst: Point2d { x: 80, y: 80, }, }];
        let tree = KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()), shapes).unwrap();

        assert_eq!(tree.intersects(&Line2d { src: Point2d { x: 116, y: 116, }, dst: Point2d { x: 180, y: 180, }, }).collect::<Vec<_>>(), vec![]);
        assert_eq!(tree.intersects(&Line2d { src: Point2d { x: 32, y: 48, }, dst: Point2d { x: 48, y: 64, }, }).collect::<Vec<_>>(), vec![]);
        assert_eq!(tree.intersects(&Line2d { src: Point2d { x: 48, y: 32, }, dst: Point2d { x: 64, y: 48, }, }).collect::<Vec<_>>(), vec![]);

        let intersects: Vec<_> = tree
            .intersects(&Line2d { src: Point2d { x: 16, y: 64, }, dst: Point2d { x: 80, y: 64, }, })
            .collect();
        assert_eq!(intersects, [
            (
                &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 72, y: 72 } },
            ),
            (
                &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 56, y: 56 }, rb: Point2d { x: 64, y: 64 } },
            ),
        ]);
        let intersects: Vec<_> = tree
            .intersects(&Line2d { src: Point2d { x: 64, y: 16, }, dst: Point2d { x: 64, y: 80, }, })
            .collect();
        assert_eq!(intersects, [
            (
                &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 72, y: 72 } },
            ),
            (
                &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 56, y: 56 }, rb: Point2d { x: 64, y: 64 } },
            ),
        ]);
    }

    #[test]
    fn kdv_tree_triangle() {
        let shapes = vec![
            Line2d { src: Point2d { x: 16, y: 16, }, dst: Point2d { x: 80, y: 16, }, },
            Line2d { src: Point2d { x: 16, y: 16, }, dst: Point2d { x: 80, y: 80, }, },
            Line2d { src: Point2d { x: 80, y: 16, }, dst: Point2d { x: 80, y: 80, }, },
        ];
        let tree = KdvTree::build(Some(Axis::X).into_iter().chain(Some(Axis::Y).into_iter()), shapes).unwrap();

        assert_eq!(tree.intersects(&Line2d { src: Point2d { x: 70, y: 45, }, dst: Point2d { x: 75, y: 50, }, }).collect::<Vec<_>>(), vec![]);

        let intersects: Vec<_> = tree
            .intersects(&Line2d { src: Point2d { x: 8, y: 48, }, dst: Point2d { x: 88, y: 48, }, })
            .collect();
        assert_eq!(intersects, vec![
            (
                &Line2d { src: Point2d { x: 80, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 80, y: 44 }, rb: Point2d { x: 80, y: 69 } },
            ),
            (
                &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 42, y: 42 }, rb: Point2d { x: 50, y: 50 } },
            ),
        ]);
        let intersects: Vec<_> = tree
            .intersects(&Line2d { src: Point2d { x: 40, y: 10, }, dst: Point2d { x: 90, y: 60, }, })
            .collect();
        assert_eq!(intersects, vec![
            (
                &Line2d { src: Point2d { x: 80, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                &Rect2d { lt: Point2d { x: 80, y: 44 }, rb: Point2d { x: 80, y: 69 } },
            ),
            (
                &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 16 } },
                &Rect2d { lt: Point2d { x: 29, y: 16 }, rb: Point2d { x: 58, y: 16 } },
            )
        ]);
    }
}
