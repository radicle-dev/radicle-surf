use crate::file_system::Label;
use nonempty::NonEmpty;

pub trait HasLabel {
    fn label(&self) -> &Label;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SubTree<A> {
    Node(A),
    Branch(Box<Forest<A>>),
}

impl<A> SubTree<A> {
    fn branch(tree: Forest<A>) -> Self {
        SubTree::Branch(Box::new(tree))
    }

    fn label(&self) -> &Label
    where
        A: HasLabel,
    {
        match self {
            SubTree::Node(node) => &node.label(),
            SubTree::Branch(ref forest) => &forest.label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Forest<A> {
    label: Label,
    forest: NonEmpty<SubTree<A>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tree<A>(Vec<Forest<A>>);

impl<A> Forest<A> {
    fn new(labels: NonEmpty<Label>, node: A) -> Self {
        let (start, middle, last) = labels.split();
        let node = SubTree::Node(node);

        if start == last && middle.is_empty() {
            Forest {
                label: start.clone(),
                forest: NonEmpty::new(node),
            }
        } else {
            let mut branch = Forest {
                label: last.clone(),
                forest: NonEmpty::new(node),
            };

            for label in middle.iter().rev() {
                branch = Forest {
                    label: label.clone(),
                    forest: NonEmpty::new(SubTree::branch(branch)),
                }
            }

            Forest {
                label: start.clone(),
                forest: NonEmpty::new(SubTree::branch(branch)),
            }
        }
    }

    fn insert_node(&mut self, node: A)
    where
        A: HasLabel,
    {
        let result = self
            .forest
            .binary_search_by(|sub_tree| sub_tree.label().cmp(&node.label()));

        let node = SubTree::Node(node);

        match result {
            Ok(index) => self.forest.insert(index, node),
            Err(index) => self.forest.insert(index, node),
        }
    }

    fn insert(&mut self, labels: NonEmpty<Label>, node: A)
    where
        A: HasLabel,
    {
        let (head, tail) = labels.split_first();
        let tail = NonEmpty::from_slice(tail);
        match self
            .forest
            .binary_search_by(|sub_tree| sub_tree.label().cmp(head))
        {
            Ok(index) => match tail {
                None => {
                    let sub_tree = self.forest.get_mut(index).unwrap();
                    match sub_tree {
                        SubTree::Node(_) => *sub_tree = SubTree::Node(node),
                        SubTree::Branch(tree) => tree.insert_node(node),
                    }
                }
                Some(labels) => {
                    let sub_tree = self.forest.get_mut(index).unwrap();
                    match sub_tree {
                        SubTree::Node(_) => *sub_tree = SubTree::Node(node),
                        SubTree::Branch(tree) => tree.insert(labels, node),
                    }
                }
            },
            Err(index) => {
                let branch = SubTree::branch(Forest::new(labels, node));
                self.forest.insert(index, branch)
            }
        }
    }
}

impl<A> Tree<A> {
    pub fn root() -> Self {
        Tree(vec![])
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn insert_forest(&mut self, index: usize, forest: Forest<A>) {
        self.0.insert(index, forest)
    }

    fn search(&self, label: &Label) -> Result<usize, usize> {
        self.0.binary_search_by(|forest| forest.label.cmp(label))
    }

    pub fn insert(&mut self, labels: NonEmpty<Label>, node: A)
    where
        A: HasLabel,
    {
        let (l, ls) = labels.split_first();
        match self.search(l) {
            Ok(index) => {
                let forest = &mut self.0[index];
                match NonEmpty::from_slice(ls) {
                    None => forest.insert_node(node),
                    Some(labels) => forest.insert(labels, node),
                }
            }
            Err(index) => self.insert_forest(index, Forest::new(labels, node)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system::unsound;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestNode {
        label: Label,
        id: u32,
    }

    impl HasLabel for TestNode {
        fn label(&self) -> &Label {
            &self.label
        }
    }

    #[test]
    fn test_insert_single_node() {
        let a_label = unsound::label::new("a");
        let b_label = unsound::label::new("b");
        let c_label = unsound::label::new("c");

        let mut tree = Tree::root();

        let c_node = TestNode {
            label: c_label,
            id: 1,
        };

        tree.insert((a_label, vec![b_label]).into(), c_node.clone());

        assert_eq!(
            tree,
            Tree(vec![Forest {
                label: unsound::label::new("a"),
                forest: NonEmpty::new(SubTree::Branch(Box::new(Forest {
                    label: unsound::label::new("b"),
                    forest: NonEmpty::new(SubTree::Node(c_node))
                })))
            }])
        );
    }

    #[test]
    fn test_insert_two_nodes() {
        let a_label = unsound::label::new("a");
        let b_label = unsound::label::new("b");
        let c_label = unsound::label::new("c");
        let d_label = unsound::label::new("d");

        let mut tree = Tree::root();

        let c_node = TestNode {
            label: c_label,
            id: 1,
        };

        tree.insert(
            (a_label.clone(), vec![b_label.clone()]).into(),
            c_node.clone(),
        );

        let d_node = TestNode {
            label: d_label,
            id: 3,
        };

        tree.insert((a_label, vec![b_label]).into(), d_node.clone());

        assert_eq!(
            tree,
            Tree(vec![Forest {
                label: unsound::label::new("a"),
                forest: NonEmpty::new(SubTree::Branch(Box::new(Forest {
                    label: unsound::label::new("b"),
                    forest: (SubTree::Node(c_node), vec![SubTree::Node(d_node)]).into()
                })))
            }])
        );
    }

    #[test]
    fn test_insert_two_nodes_out_of_order() {
        let a_label = unsound::label::new("a");
        let b_label = unsound::label::new("b");
        let c_label = unsound::label::new("c");
        let d_label = unsound::label::new("d");

        let mut tree = Tree::root();

        let d_node = TestNode {
            label: d_label,
            id: 3,
        };

        tree.insert(
            (a_label.clone(), vec![b_label.clone()]).into(),
            d_node.clone(),
        );

        let c_node = TestNode {
            label: c_label,
            id: 1,
        };

        tree.insert((a_label, vec![b_label]).into(), c_node.clone());

        assert_eq!(
            tree,
            Tree(vec![Forest {
                label: unsound::label::new("a"),
                forest: NonEmpty::new(SubTree::Branch(Box::new(Forest {
                    label: unsound::label::new("b"),
                    forest: (SubTree::Node(c_node), vec![SubTree::Node(d_node)]).into()
                })))
            }])
        );
    }

    #[test]
    fn test_insert_branch() {
        let a_label = unsound::label::new("a");
        let b_label = unsound::label::new("b");
        let c_label = unsound::label::new("c");
        let d_label = unsound::label::new("d");
        let e_label = unsound::label::new("e");
        let f_label = unsound::label::new("f");

        let b_path = NonEmpty::from((a_label.clone(), vec![b_label]));
        let e_path = NonEmpty::from((a_label, vec![e_label]));

        let mut tree = Tree::root();

        let c_node = TestNode {
            label: c_label,
            id: 1,
        };

        let d_node = TestNode {
            label: d_label,
            id: 3,
        };

        let f_node = TestNode {
            label: f_label,
            id: 2,
        };

        tree.insert(b_path.clone(), d_node.clone());
        tree.insert(b_path, c_node.clone());
        tree.insert(e_path, f_node.clone());

        assert_eq!(
            tree,
            Tree(vec![Forest {
                label: unsound::label::new("a"),
                forest: NonEmpty::from((
                    SubTree::Branch(Box::new(Forest {
                        label: unsound::label::new("b"),
                        forest: NonEmpty::from((
                            SubTree::Node(c_node),
                            vec![SubTree::Node(d_node)]
                        ))
                    })),
                    vec![SubTree::branch(Forest {
                        label: unsound::label::new("e"),
                        forest: NonEmpty::new(SubTree::Node(f_node)),
                    })]
                ))
            }])
        );
    }

    #[test]
    fn test_insert_branches_and_node() {
        let a_label = unsound::label::new("a");
        let b_label = unsound::label::new("b");
        let c_label = unsound::label::new("c");
        let d_label = unsound::label::new("d");
        let e_label = unsound::label::new("e");
        let f_label = unsound::label::new("f");
        let g_label = unsound::label::new("g");

        let b_path = NonEmpty::from((a_label.clone(), vec![b_label]));
        let f_path = NonEmpty::from((a_label.clone(), vec![f_label.clone()]));

        let mut tree = Tree::root();

        let c_node = TestNode {
            label: c_label,
            id: 1,
        };

        let d_node = TestNode {
            label: d_label,
            id: 3,
        };

        let e_node = TestNode {
            label: e_label,
            id: 2,
        };

        let g_node = TestNode {
            label: g_label,
            id: 2,
        };

        tree.insert(b_path.clone(), d_node.clone());
        tree.insert(b_path, c_node.clone());
        tree.insert(NonEmpty::new(a_label), e_node.clone());
        tree.insert(f_path, g_node.clone());

        assert_eq!(
            tree,
            Tree(vec![Forest {
                label: unsound::label::new("a"),
                forest: NonEmpty::from((
                    SubTree::Branch(Box::new(Forest {
                        label: unsound::label::new("b"),
                        forest: NonEmpty::from((
                            SubTree::Node(c_node),
                            vec![SubTree::Node(d_node)]
                        ))
                    })),
                    vec![
                        SubTree::Node(e_node),
                        SubTree::branch(Forest {
                            label: unsound::label::new("f"),
                            forest: NonEmpty::new(SubTree::Node(g_node)),
                        })
                    ]
                ))
            }])
        );
    }
}
