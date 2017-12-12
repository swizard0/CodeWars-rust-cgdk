
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
    pub fn build<IA, II>(axis_it: IA, shapes_it: II) -> KdvTree<P, B, S>
        where IA: IntoIterator<Item = P::Axis>,
              II: IntoIterator<Item = S>
    {
        let axis: Vec<_> = axis_it.into_iter().collect();
        let shapes: Vec<_> = shapes_it.into_iter().collect();
        let root_shapes: Vec<_> = shapes
            .iter()
            .enumerate()
            .map(|(i, s)| ShapeFragment {
                bounding_box: s.bounding_box(),
                shape_id: i,
            })
            .collect();
        KdvTree {
            root: KdvNode::build(0, &axis, &shapes, root_shapes),
            axis, shapes,
        }
    }

    pub fn intersects<'t, 's, SN>(&'t self, shape: &'s SN) -> IntersectIter<'t, 's, P::Axis, P::Coord, S, B, SN, SN::BoundingBox>
        where SN: Shape<BoundingBox = S::BoundingBox>
    {
        IntersectIter {
            needle: shape,
            axis: &self.axis,
            shapes: &self.shapes,
            queue: vec![TraverseTask::Explore {
                node: &self.root,
                needle_fragment: shape.bounding_box(),
            }],
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

enum TraverseTask<'t, A: 't, C: 't, BS: 't, BN> {
    Explore { node: &'t KdvNode<A, C, BS>, needle_fragment: BN, },
    Intersect { needle_fragment: BN, shape_fragment: &'t ShapeFragment<BS>, axis_counter: usize, },
}

pub struct IntersectIter<'t, 's, A: 't, C: 't, SS: 't, BS: 't, SN: 's, BN> {
    needle: &'s SN,
    axis: &'t [A],
    shapes: &'t [SS],
    queue: Vec<TraverseTask<'t, A, C, BS, BN>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Intersection<'t, SS: 't, BS: 't, BN> {
    pub shape: &'t SS,
    pub shape_fragment: &'t BS,
    pub needle_fragment: BN,
}

impl<'t, 's, A, C, SS, BS, SN, BN> Iterator for IntersectIter<'t, 's, A, C, SS, BS, SN, BN>
    where A: Clone,
          C: Coord,
          SS: Shape<BoundingBox = BS>,
          BS: BoundingBox,
          BS::Point: Point<Axis = A, Coord = C>,
          SN: Shape<BoundingBox = BN>,
          BN: BoundingBox<Point = BS::Point> + Clone,
{
    type Item = Intersection<'t, SS, BS, BN>;

    fn next(&mut self) -> Option<Self::Item> {
        'outer: while let Some(task) = self.queue.pop() {
            match task {
                TraverseTask::Explore { node, needle_fragment, } => {
                    let needle_owner =
                        shape_owner(self.needle, needle_fragment.clone(), &node.cut_axis, &node.cut_coord);
                    fn schedule_explore<'t, A, C, BS, BN>(
                        queue: &mut Vec<TraverseTask<'t, A, C, BS, BN>>,
                        maybe_node: &'t Option<Box<KdvNode<A, C, BS>>>,
                        fragment: BN,
                    ) {
                        if let &Some(ref node) = maybe_node {
                            queue.push(TraverseTask::Explore {
                                node: node.as_ref(),
                                needle_fragment: fragment,
                            });
                        }
                    }
                    match needle_owner {
                        ShapeOwner::Me(fragment) => {
                            schedule_explore(&mut self.queue, &node.left, fragment.clone());
                            schedule_explore(&mut self.queue, &node.right, fragment);
                        },
                        ShapeOwner::Left(fragment) =>
                            schedule_explore(&mut self.queue, &node.left, fragment),
                        ShapeOwner::Right(fragment) =>
                            schedule_explore(&mut self.queue, &node.right, fragment),
                        ShapeOwner::Both { left_bbox, right_bbox, } => {
                            schedule_explore(&mut self.queue, &node.left, left_bbox);
                            schedule_explore(&mut self.queue, &node.right, right_bbox);
                        },
                    }
                    for shape_fragment in node.shapes.iter() {
                        self.queue.push(TraverseTask::Intersect {
                            shape_fragment,
                            needle_fragment: needle_fragment.clone(),
                            axis_counter: 0,
                        });
                    }
                },
                TraverseTask::Intersect { shape_fragment, needle_fragment, mut axis_counter, } => {
                    let no_intersection = self.axis.iter().any(|axis| {
                        let needle_min = needle_fragment.min_corner().coord(axis);
                        let needle_max = needle_fragment.max_corner().coord(axis);
                        let shape_min = shape_fragment.bounding_box.min_corner().coord(axis);
                        let shape_max = shape_fragment.bounding_box.max_corner().coord(axis);
                        needle_min > shape_max || needle_max < shape_min
                    });
                    if no_intersection {
                        continue;
                    }
                    let axis_total = self.axis.len();
                    for _ in 0 .. axis_total {
                        let cut_axis = &self.axis[axis_counter % axis_total];
                        let cut_coord = Coord::cut_point({
                            let min_coord = needle_fragment.min_corner().coord(cut_axis);
                            let max_coord = needle_fragment.max_corner().coord(cut_axis);
                            Some(min_coord).into_iter().chain(Some(max_coord).into_iter())
                        });
                        if let Some((left_fragment, right_fragment)) = self.needle.cut(&needle_fragment, cut_axis, &cut_coord) {
                            self.queue.push(TraverseTask::Intersect {
                                shape_fragment: shape_fragment.clone(),
                                needle_fragment: left_fragment,
                                axis_counter,
                            });
                            self.queue.push(TraverseTask::Intersect {
                                shape_fragment: shape_fragment,
                                needle_fragment: right_fragment,
                                axis_counter,
                            });
                            continue 'outer;
                        }
                        axis_counter += 1;
                    }
                    return Some(Intersection {
                        shape: &self.shapes[shape_fragment.shape_id],
                        shape_fragment: &shape_fragment.bounding_box,
                        needle_fragment,
                    });
                },
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::{min, max};
    use super::{KdvTree, Intersection};

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
        assert_eq!(intersects, vec![
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 72, y: 72 } },
                needle_fragment: Rect2d { lt: Point2d { x: 66, y: 64 }, rb: Point2d { x: 72, y: 64 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 72, y: 72 } },
                needle_fragment: Rect2d { lt: Point2d { x: 60, y: 64 }, rb: Point2d { x: 66, y: 64 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 56, y: 56 }, rb: Point2d { x: 64, y: 64 } },
                needle_fragment: Rect2d { lt: Point2d { x: 62, y: 64 }, rb: Point2d { x: 68, y: 64 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 56, y: 56 }, rb: Point2d { x: 64, y: 64 } },
                needle_fragment: Rect2d { lt: Point2d { x: 56, y: 64 }, rb: Point2d { x: 62, y: 64 } },
            },
        ]);
        let intersects: Vec<_> = tree
            .intersects(&Line2d { src: Point2d { x: 64, y: 16, }, dst: Point2d { x: 64, y: 80, }, })
            .collect();
        assert_eq!(intersects, [
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 72, y: 72 } },
                needle_fragment: Rect2d { lt: Point2d { x: 64, y: 72 }, rb: Point2d { x: 64, y: 80 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 72, y: 72 } },
                needle_fragment: Rect2d { lt: Point2d { x: 64, y: 64 }, rb: Point2d { x: 64, y: 72 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 56, y: 56 }, rb: Point2d { x: 64, y: 64 } },
                needle_fragment: Rect2d { lt: Point2d { x: 64, y: 58 }, rb: Point2d { x: 64, y: 64 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 56, y: 56 }, rb: Point2d { x: 64, y: 64 } },
                needle_fragment: Rect2d { lt: Point2d { x: 64, y: 52 }, rb: Point2d { x: 64, y: 58 } },
            },
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
            Intersection {
                shape: &Line2d { src: Point2d { x: 80, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 80, y: 44 }, rb: Point2d { x: 80, y: 69 } },
                needle_fragment: Rect2d { lt: Point2d { x: 74, y: 48 }, rb: Point2d { x: 81, y: 48 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 42, y: 42 }, rb: Point2d { x: 50, y: 50 } },
                needle_fragment: Rect2d { lt: Point2d { x: 50, y: 48 }, rb: Point2d { x: 58, y: 48 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 42, y: 42 }, rb: Point2d { x: 50, y: 50 } },
                needle_fragment: Rect2d { lt: Point2d { x: 42, y: 48 }, rb: Point2d { x: 50, y: 48 } },
            },
        ]);
        let intersects: Vec<_> = tree
            .intersects(&Line2d { src: Point2d { x: 40, y: 10, }, dst: Point2d { x: 90, y: 60, }, })
            .collect();
        assert_eq!(intersects, vec![
            Intersection {
                shape: &Line2d { src: Point2d { x: 80, y: 16 }, dst: Point2d { x: 80, y: 80 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 80, y: 44 }, rb: Point2d { x: 80, y: 69 } },
                needle_fragment: Rect2d { lt: Point2d { x: 74, y: 44 }, rb: Point2d { x: 82, y: 52 } },
            },
            Intersection {
                shape: &Line2d { src: Point2d { x: 16, y: 16 }, dst: Point2d { x: 80, y: 16 } },
                shape_fragment: &Rect2d { lt: Point2d { x: 29, y: 16 }, rb: Point2d { x: 58, y: 16 } },
                needle_fragment: Rect2d { lt: Point2d { x: 40, y: 10 }, rb: Point2d { x: 48, y: 18 } },
            },
        ]);
    }
}
