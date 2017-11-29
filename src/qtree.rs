
pub struct QuadTree {
    root: Node,
}

struct Node {
    children: Vec<Node>,
}

impl QuadTree {
    pub fn new() -> QuadTree {
        QuadTree {
            root: Node::new(),
        }
    }
}

impl Node {
    fn new() -> Node {
        Node {
            children: Vec::with_capacity(4),
        }
    }
}
