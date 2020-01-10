use nonempty::NonEmpty;

pub trait HasKey<K> {
    fn key(&self) -> &K;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubTree<K, A> {
    Node(A),
    Branch { key: K, forest: Box<Tree<K, A>> },
}

impl<K, A> SubTree<K, A> {
    fn branch(key: K, tree: Tree<K, A>) -> Self {
        SubTree::Branch {
            key,
            forest: Box::new(tree),
        }
    }
}

impl<K, A: HasKey<K>> HasKey<K> for SubTree<K, A> {
    fn key(&self) -> &K {
        match self {
            SubTree::Node(node) => &node.key(),
            SubTree::Branch { key, .. } => &key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tree<K, A>(NonEmpty<SubTree<K, A>>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Forest<K, A>(Option<Tree<K, A>>);

impl<K, A> Tree<K, A> {
    fn branch(key: K, forest: Self) -> Self {
        Tree(NonEmpty::new(SubTree::branch(key, forest)))
    }

    fn node(node: A) -> Self {
        Tree(NonEmpty::new(SubTree::Node(node)))
    }

    fn new(keys: NonEmpty<K>, node: A) -> Self
    where
        K: Ord + Clone,
    {
        let (start, middle, last) = keys.split();

        if start == last && middle.is_empty() {
            Tree::branch(start.clone(), Tree::node(node))
        } else {
            let mut branch = SubTree::branch(last.clone(), Tree::node(node));

            for key in middle.iter().rev() {
                branch = SubTree::branch(key.clone(), Tree(NonEmpty::new(branch)))
            }

            Tree::branch(start.clone(), Tree(NonEmpty::new(branch)))
        }
    }

    fn search(&self, key: &K) -> Result<usize, usize>
    where
        A: HasKey<K>,
        K: Ord,
    {
        self.0.binary_search_by(|tree| tree.key().cmp(key))
    }

    fn insert_node(&mut self, node: A)
    where
        A: HasKey<K>,
        K: Ord,
    {
        let result = self.search(&node.key());

        let node = SubTree::Node(node);

        match result {
            Ok(index) => self.0.insert(index, node),
            Err(index) => self.0.insert(index, node),
        }
    }

    fn insert(&mut self, keys: NonEmpty<K>, node: A)
    where
        A: HasKey<K>,
        K: Ord + Clone,
    {
        let (head, tail) = keys.split_first();
        let tail = NonEmpty::from_slice(tail);
        match self.search(head) {
            Ok(index) => match tail {
                None => {
                    let sub_tree = self.0.get_mut(index).unwrap();
                    match sub_tree {
                        SubTree::Node(_) => *sub_tree = SubTree::Node(node),
                        SubTree::Branch { forest, .. } => forest.insert_node(node),
                    }
                }
                Some(keys) => {
                    let sub_tree = self.0.get_mut(index).unwrap();
                    match sub_tree {
                        SubTree::Node(_) => *sub_tree = SubTree::Node(node),
                        SubTree::Branch { forest, .. } => forest.insert(keys, node),
                    }
                }
            },
            Err(index) => match tail {
                None => self
                    .0
                    .insert(index, SubTree::branch(head.clone(), Tree::node(node))),
                Some(tail) => {
                    let branch = Tree::new(tail, node);
                    self.0.insert(index, SubTree::branch(head.clone(), branch))
                }
            },
        }
    }

    pub fn find(&self, keys: NonEmpty<K>) -> Option<&SubTree<K, A>>
    where
        A: HasKey<K>,
        K: Ord + Clone,
    {
        let (head, tail) = keys.split_first();
        let tail = NonEmpty::from_slice(tail);
        match self.search(head) {
            Err(_) => None,
            Ok(index) => {
                let sub_tree = self.0.get(index).unwrap();
                match tail {
                    None => match sub_tree {
                        SubTree::Node(_) => Some(sub_tree),
                        SubTree::Branch { .. } => Some(sub_tree),
                    },
                    Some(keys) => match sub_tree {
                        SubTree::Node(_) => None,
                        SubTree::Branch { forest, .. } => forest.find(keys),
                    },
                }
            }
        }
    }
}

impl<K, A> Forest<K, A> {
    pub fn root() -> Self {
        Forest(None)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    fn insert_forest(&mut self, forest: Tree<K, A>) {
        self.0 = Some(forest)
    }

    pub fn insert(&mut self, keys: Vec<K>, node: A)
    where
        A: HasKey<K>,
        K: Ord + Clone,
    {
        match self.0.as_mut() {
            Some(forest) => match NonEmpty::from_slice(&keys) {
                None => forest.insert_node(node),
                Some(keys) => forest.insert(keys, node),
            },
            None => match NonEmpty::from_slice(&keys) {
                None => self.insert_forest(Tree::node(node)),
                Some(keys) => self.insert_forest(Tree::new(keys, node)),
            },
        }
    }

    pub fn find(&self, keys: NonEmpty<K>) -> Option<&SubTree<K, A>>
    where
        A: HasKey<K> + Clone,
        K: Ord + Clone,
    {
        self.0.as_ref().and_then(|trees| trees.find(keys))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestNode {
        key: String,
        id: u32,
    }

    impl HasKey<String> for TestNode {
        fn key(&self) -> &String {
            &self.key
        }
    }

    #[test]
    fn test_insert_root_node() {
        let a_label = String::from("a");

        let mut tree = Forest::root();

        let a_node = TestNode {
            key: a_label,
            id: 1,
        };

        tree.insert(vec![], a_node.clone());

        assert_eq!(tree, Forest(Some(Tree::node(a_node))));
    }

    #[test]
    fn test_insert_single_node() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let path = vec![a_label, b_label];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        tree.insert(path, c_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::branch(String::from("b"), Tree::node(c_node))
            )))
        );
    }

    #[test]
    fn test_insert_two_nodes() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let d_label = String::from("d");
        let b_path = vec![a_label, b_label];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        tree.insert(b_path.clone(), c_node.clone());

        let d_node = TestNode {
            key: d_label,
            id: 3,
        };

        tree.insert(b_path, d_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::branch(
                    String::from("b"),
                    Tree(NonEmpty::from((
                        SubTree::Node(c_node),
                        vec![SubTree::Node(d_node)]
                    )))
                )
            )))
        );
    }

    #[test]
    fn test_insert_two_nodes_out_of_order() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let d_label = String::from("d");
        let b_path = vec![a_label, b_label];

        let mut tree = Forest::root();

        let d_node = TestNode {
            key: d_label,
            id: 3,
        };

        tree.insert(b_path.clone(), d_node.clone());

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        tree.insert(b_path, c_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::branch(
                    String::from("b"),
                    Tree(NonEmpty::from((
                        SubTree::Node(c_node),
                        vec![SubTree::Node(d_node)]
                    )))
                )
            )))
        );
    }

    #[test]
    fn test_insert_branch() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let d_label = String::from("d");
        let e_label = String::from("e");
        let f_label = String::from("f");

        let b_path = vec![a_label.clone(), b_label];
        let e_path = vec![a_label, e_label];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        let d_node = TestNode {
            key: d_label,
            id: 3,
        };

        let f_node = TestNode {
            key: f_label,
            id: 2,
        };

        tree.insert(b_path.clone(), d_node.clone());
        tree.insert(b_path, c_node.clone());
        tree.insert(e_path, f_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree(NonEmpty::from((
                    SubTree::Branch {
                        key: String::from("b"),
                        forest: Box::new(Tree(NonEmpty::from((
                            SubTree::Node(c_node),
                            vec![SubTree::Node(d_node)]
                        ))))
                    },
                    vec![SubTree::Branch {
                        key: String::from("e"),
                        forest: Box::new(Tree::node(f_node))
                    },]
                )))
            )))
        );
    }

    #[test]
    fn test_insert_branches_and_node() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let d_label = String::from("d");
        let e_label = String::from("e");
        let f_label = String::from("f");
        let g_label = String::from("g");

        let b_path = vec![a_label.clone(), b_label];
        let f_path = vec![a_label.clone(), f_label.clone()];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        let d_node = TestNode {
            key: d_label,
            id: 3,
        };

        let e_node = TestNode {
            key: e_label,
            id: 2,
        };

        let g_node = TestNode {
            key: g_label,
            id: 2,
        };

        tree.insert(b_path.clone(), d_node.clone());
        tree.insert(b_path, c_node.clone());
        tree.insert(vec![a_label], e_node.clone());
        tree.insert(f_path, g_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree(NonEmpty::from((
                    SubTree::Branch {
                        key: String::from("b"),
                        forest: Box::new(Tree(NonEmpty::from((
                            SubTree::Node(c_node),
                            vec![SubTree::Node(d_node)]
                        ))))
                    },
                    vec![
                        SubTree::Node(e_node),
                        SubTree::Branch {
                            key: String::from("f"),
                            forest: Box::new(Tree::node(g_node))
                        },
                    ]
                )))
            )))
        );
    }

    #[test]
    fn test_find_root_node() {
        let a_label = String::from("a");

        let mut tree = Forest::root();

        let a_node = TestNode {
            key: a_label,
            id: 1,
        };

        tree.insert(vec![], a_node.clone());

        assert_eq!(
            tree.find(NonEmpty::new(String::from("a"))),
            Some(&SubTree::Node(a_node))
        );

        assert_eq!(tree.find(NonEmpty::new(String::from("b"))), None);
    }

    #[test]
    fn test_find_branch_and_node() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let path = vec![a_label, b_label];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        tree.insert(path, c_node.clone());

        assert_eq!(
            tree.find(NonEmpty::new(String::from("a"))),
            Some(&SubTree::Branch {
                key: String::from("a"),
                forest: Box::new(Tree::branch(String::from("b"), Tree::node(c_node.clone())))
            })
        );

        assert_eq!(
            tree.find(NonEmpty::from((String::from("a"), vec![String::from("b")]))),
            Some(&SubTree::Branch {
                key: String::from("b"),
                forest: Box::new(Tree::node(c_node.clone()))
            })
        );

        assert_eq!(
            tree.find(NonEmpty::from((
                String::from("a"),
                vec![String::from("b"), String::from("c")]
            ))),
            Some(&SubTree::Node(c_node))
        );

        assert_eq!(tree.find(NonEmpty::new(String::from("b"))), None);

        assert_eq!(
            tree.find(NonEmpty::from((String::from("a"), vec![String::from("c")]))),
            None
        );
    }
}
