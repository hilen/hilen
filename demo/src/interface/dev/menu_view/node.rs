#[derive(Default, Clone, Debug)]
pub struct Node<T> {
    pub open:   bool,
    pub index:  usize,
    pub depth:  usize,
    pub value:  T,
    pub leaves: Vec<Self>,
}

impl<T> Node<T> {
    pub fn empty(value: T) -> Self {
        Self {
            open: false,
            index: 0,
            depth: 0,
            value,
            leaves: vec![],
        }
    }

    pub fn new(value: T, leaves: Vec<Self>) -> Self {
        Self {
            open: false,
            index: 0,
            depth: 0,
            value,
            leaves,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.leaves.is_empty()
    }

    pub fn length(&self) -> usize {
        let mut length = 1;

        if self.open {
            for leaf in &self.leaves {
                length += leaf.length();
            }
        }

        length
    }

    pub fn update_indices(&mut self, index: usize, depth: usize) -> usize {
        self.index = index;
        self.depth = depth;
        let mut last_index = index + 1;

        if !self.open {
            return last_index;
        }

        for node in &mut self.leaves {
            last_index = node.update_indices(last_index, depth + 1);
        }

        last_index
    }

    pub fn val_at_index(&mut self, index: usize) -> &mut Self {
        self.find(index)
            .unwrap_or_else(|| panic!("Failed to find node at index {index}"))
    }

    fn find(&mut self, index: usize) -> Option<&mut Self> {
        if self.index == index {
            return Some(self);
        }

        if !self.open {
            return None;
        }

        for leaf in &mut self.leaves {
            if let Some(val) = leaf.find(index) {
                return Some(val);
            }
        }

        None
    }
}

impl<T> Iterator for Node<T> {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use crate::interface::dev::Node;

    #[test]
    fn test_node() {
        let mut root = Node::new(
            "Root",
            vec![
                Node::empty("A"),
                Node::new("Ooo", vec![Node::empty("a"), Node::empty("b")]),
                Node::empty("C"),
            ],
        );

        assert_eq!(root.length(), 1);

        root.open = true;

        assert_eq!(root.length(), 4);

        root.update_indices(0, 0);

        assert_eq!(root.val_at_index(3).value, "C");

        root.leaves[1].open = true;
        root.update_indices(0, 0);

        assert_eq!(root.val_at_index(3).value, "a");
        assert_eq!(root.val_at_index(5).value, "C");

        dbg!(&root);
    }
}
