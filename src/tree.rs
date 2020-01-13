use nonempty::NonEmpty;
use std::cmp::Ordering;

pub trait HasKey<K> {
    fn key(&self) -> &K;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubTree<K, A> {
    Node(A),
    Branch { key: K, forest: Box<Tree<K, A>> },
}

impl<K, A> SubTree<K, A> {
    /// Create a new `Branch` from a key and sub-tree.
    ///
    /// This function is a convenience for now having to
    /// remember to use `Box::new`.
    fn branch(key: K, tree: Tree<K, A>) -> Self {
        SubTree::Branch {
            key,
            forest: Box::new(tree),
        }
    }

    fn compare_by<F>(&self, other: &Self, f: &F) -> Ordering
    where
        F: Fn(&A, &A) -> Ordering,
    {
        match (self, other) {
            (SubTree::Node(node), SubTree::Node(other_node)) => f(&node, &other_node),
            (SubTree::Branch { forest, .. }, SubTree::Node(other_node)) => {
                let max_forest = forest.maximum_by(f);
                f(&max_forest, &other_node)
            }
            (SubTree::Node(node), SubTree::Branch { forest, .. }) => {
                let max_forest = &forest.maximum_by(f);
                f(&node, &max_forest)
            }
            (
                SubTree::Branch { forest, .. },
                SubTree::Branch {
                    forest: other_forest,
                    ..
                },
            ) => {
                let max_forest = forest.maximum_by(f);
                let max_other_forest = other_forest.maximum_by(f);
                f(&max_forest, &max_other_forest)
            }
        }
    }

    pub fn maximum_by<F>(&self, f: &F) -> &A
    where
        F: Fn(&A, &A) -> Ordering,
    {
        match self {
            SubTree::Node(node) => node,
            SubTree::Branch { forest, .. } => forest.maximum_by(f),
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
    /// Create a new `Tree` containing a single `Branch` given
    /// the key and sub-tree.
    fn branch(key: K, forest: Self) -> Self {
        Tree(NonEmpty::new(SubTree::branch(key, forest)))
    }

    /// Create a new `Tree` containing a single `Node`.
    fn node(node: A) -> Self {
        Tree(NonEmpty::new(SubTree::Node(node)))
    }

    /// Create a new `Tree` that creates a series of
    /// `Branch`es built using the `keys`. The final `Branch`
    /// will contain the `node`.
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

    /// Perform a binary search in the sub-trees, based on comparing
    /// each of the sub-trees' key to the provided `key`.
    fn search(&self, key: &K) -> Result<usize, usize>
    where
        A: HasKey<K>,
        K: Ord,
    {
        self.0.binary_search_by(|tree| tree.key().cmp(key))
    }

    /// Insert a `node` into the list of sub-trees.
    ///
    /// The node's position will be based on the `Ord` instance
    /// of `K`.
    fn insert_node(&mut self, node: A)
    where
        A: HasKey<K>,
        K: Ord,
    {
        let result = self.search(&node.key());

        let node = SubTree::Node(node);

        match result {
            Ok(index) => {
                let old_node = self.0.get_mut(index).unwrap();
                *old_node = node
            }
            Err(index) => self.0.insert(index, node),
        }
    }

    /// Insert the `node` in the position given by `keys`.
    ///
    /// If the same path to a node is provided the `node` will replace the old one,
    /// i.e. if `a/b/c` exists in the tree and `a/b/c` is the full path to
    /// the node, then `c` will be replaced.
    ///
    /// If the path points to a branch, then the `node` will be inserted in this
    /// branch.
    ///
    /// If a portion of the path points to a node then a branch will be created in
    /// its place, i.e. if `a/b/c` exists in the tree and the provided path is `a/b/c/d`,
    /// then the node `c` will be replaced by a branch `c/d`.
    ///
    /// If the path does not exist it will be inserted into the set of sub-trees.
    fn insert(&mut self, keys: NonEmpty<K>, node: A)
    where
        A: HasKey<K>,
        K: Ord + Clone,
    {
        let (head, tail) = keys.split_first();
        let maybe_keys = NonEmpty::from_slice(tail);
        match self.search(head) {
            // Found the label in our set of sub-trees
            Ok(index) => match maybe_keys {
                // The keys have been exhausted and so its time to insert the node
                None => {
                    let sub_tree = self.0.get_mut(index).unwrap();
                    match sub_tree {
                        // Our sub-tree was a node. We technically have a branch because
                        // we have the `head` label, and the node's label.
                        SubTree::Node(_) => {
                            *sub_tree = SubTree::branch(head.clone(), Tree::node(node))
                        }
                        // The branche's label is head in this case so we can safely
                        // insert a node.
                        SubTree::Branch { forest, .. } => forest.insert_node(node),
                    }
                }
                Some(keys) => {
                    let sub_tree = self.0.get_mut(index).unwrap();
                    match sub_tree {
                        // We have reached a node, but still have keys left to get through.
                        SubTree::Node(_) => {
                            let new_tree = SubTree::branch(head.clone(), Tree::new(keys, node));
                            *sub_tree = new_tree
                        }
                        // We keep moving down the set of keys to find where to insert this node.
                        SubTree::Branch { forest, .. } => forest.insert(keys, node),
                    }
                }
            },
            // The label was not found and we have an index for insertion.
            Err(index) => match maybe_keys {
                // We create the branch with the head label and node, since there are
                // no more labels left.
                None => self
                    .0
                    .insert(index, SubTree::branch(head.clone(), Tree::node(node))),
                // We insert an entirely new branch with the full list of keys.
                Some(tail) => self
                    .0
                    .insert(index, SubTree::branch(head.clone(), Tree::new(tail, node))),
            },
        }
    }

    /// Find a `SubTree` given a search path. If the path does not match
    /// it will return `None`.
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

    pub fn maximum_by<F>(&self, f: &F) -> &A
    where
        F: Fn(&A, &A) -> Ordering,
    {
        self.0.maximum_by(|s, t| s.compare_by(t, f)).maximum_by(f)
    }

    pub fn maximum(&self) -> &A
    where
        A: Ord,
    {
        self.maximum_by(&|a, b| a.cmp(b))
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

    /// Insert the `node` in the position given by `keys`.
    ///
    /// If the same path to a node is provided the `node` will replace the old one,
    /// i.e. if `a/b/c` exists in the tree and `a/b/c` is the full path to
    /// the node, then `c` will be replaced.
    ///
    /// If the path points to a branch, then the `node` will be inserted in this
    /// branch.
    ///
    /// If a portion of the path points to a node then a branch will be created in
    /// its place, i.e. if `a/b/c` exists in the tree and the provided path is `a/b/c/d`,
    /// then the node `c` will be replaced by a branch `c/d`.
    ///
    /// If the path does not exist it will be inserted into the set of sub-trees.
    pub fn insert(&mut self, keys: Vec<K>, node: A)
    where
        A: HasKey<K>,
        K: Ord + Clone,
    {
        match self.0.as_mut() {
            Some(forest) => match NonEmpty::from_slice(&keys) {
                None => {
                    // Insert the node at the root
                    forest.insert_node(node)
                }
                Some(keys) => forest.insert(keys, node),
            },
            None => match NonEmpty::from_slice(&keys) {
                None => self.insert_forest(Tree::node(node)),
                Some(keys) => self.insert_forest(Tree::new(keys, node)),
            },
        }
    }

    /// Find a `SubTree` given a search path. If the path does not match
    /// it will return `None`.
    pub fn find(&self, keys: NonEmpty<K>) -> Option<&SubTree<K, A>>
    where
        A: HasKey<K>,
        K: Ord + Clone,
    {
        self.0.as_ref().and_then(|trees| trees.find(keys))
    }

    pub fn maximum_by<F>(&self, f: F) -> Option<&A>
    where
        F: Fn(&A, &A) -> Ordering,
    {
        self.0.as_ref().map(|trees| trees.maximum_by(&f))
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
    fn test_is_empty() {
        let mut tree = Forest::root();
        assert!(tree.is_empty());

        let a_node = TestNode {
            key: String::from("a"),
            id: 1,
        };

        tree.insert(vec![], a_node);
        assert!(!tree.is_empty());
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
    fn test_insert_replaces_node() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let b_path = vec![a_label, b_label];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label.clone(),
            id: 1,
        };

        tree.insert(b_path.clone(), c_node.clone());

        let new_c_node = TestNode {
            key: c_label,
            id: 3,
        };

        tree.insert(b_path, new_c_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::branch(
                    String::from("b"),
                    Tree(NonEmpty::new(SubTree::Node(new_c_node),))
                )
            )))
        );
    }

    #[test]
    fn test_insert_replaces_root_node() {
        let c_label = String::from("c");

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label.clone(),
            id: 1,
        };

        tree.insert(vec![], c_node.clone());

        let new_c_node = TestNode {
            key: c_label,
            id: 3,
        };

        tree.insert(vec![], new_c_node.clone());

        assert_eq!(tree, Forest(Some(Tree::node(new_c_node))));
    }

    #[test]
    fn test_insert_replaces_branch_node() {
        let a_label = String::from("a");
        let c_label = String::from("c");

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label.clone(),
            id: 1,
        };

        tree.insert(vec![a_label.clone()], c_node.clone());

        let new_c_node = TestNode {
            key: c_label,
            id: 3,
        };

        tree.insert(vec![a_label], new_c_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::node(new_c_node),
            )))
        );
    }

    #[test]
    fn test_insert_replaces_branch_with_node() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let b_path = vec![a_label.clone(), b_label.clone()];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label.clone(),
            id: 1,
        };

        tree.insert(b_path, c_node.clone());

        let new_c_node = TestNode {
            key: b_label,
            id: 3,
        };

        tree.insert(vec![a_label], new_c_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::node(new_c_node),
            )))
        );
    }

    #[test]
    fn test_insert_replaces_node_with_branch() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let b_path = vec![a_label.clone(), b_label.clone()];

        let mut tree = Forest::root();

        let b_node = TestNode {
            key: b_label.clone(),
            id: 1,
        };

        tree.insert(vec![a_label], b_node);

        let new_c_node = TestNode {
            key: c_label,
            id: 3,
        };

        tree.insert(b_path, new_c_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::branch(
                    String::from("b"),
                    Tree(NonEmpty::new(SubTree::Node(new_c_node),))
                )
            )))
        );
    }

    #[test]
    fn test_insert_replaces_node_with_branch_foo() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let d_label = String::from("d");
        let c_path = vec![a_label.clone(), b_label.clone(), c_label.clone()];

        let mut tree = Forest::root();

        let b_node = TestNode {
            key: b_label.clone(),
            id: 1,
        };

        tree.insert(vec![a_label], b_node);
        println!("{:#?}", tree);

        let d_node = TestNode {
            key: d_label,
            id: 3,
        };

        tree.insert(c_path, d_node.clone());
        println!("{:#?}", tree);

        assert_eq!(
            tree,
            Forest(Some(Tree::branch(
                String::from("a"),
                Tree::branch(
                    String::from("b"),
                    Tree::branch(String::from("c"), Tree::node(d_node))
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
    fn test_insert_two_branches() {
        let a_label = String::from("a");
        let b_label = String::from("b");
        let c_label = String::from("c");
        let d_label = String::from("d");
        let e_label = String::from("e");
        let f_label = String::from("f");

        let b_path = vec![a_label.clone(), b_label];
        let e_path = vec![d_label, e_label];

        let mut tree = Forest::root();

        let c_node = TestNode {
            key: c_label,
            id: 1,
        };

        let f_node = TestNode {
            key: f_label,
            id: 2,
        };

        tree.insert(b_path.clone(), c_node.clone());
        tree.insert(e_path, f_node.clone());

        assert_eq!(
            tree,
            Forest(Some(Tree(NonEmpty::from((
                SubTree::Branch {
                    key: String::from("a"),
                    forest: Box::new(Tree::branch(String::from("b"), Tree::node(c_node))),
                },
                vec![SubTree::Branch {
                    key: String::from("d"),
                    forest: Box::new(Tree::branch(String::from("e"), Tree::node(f_node)))
                }]
            )))))
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

    #[test]
    fn test_maximum_by_root_nodes() {
        let mut tree = Forest::root();

        let a_node = TestNode {
            key: String::from("a"),
            id: 1,
        };

        let b_node = TestNode {
            key: String::from("b"),
            id: 3,
        };

        tree.insert(vec![], a_node.clone());
        tree.insert(vec![], b_node.clone());

        assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
        assert_eq!(
            tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
            Some(&a_node)
        );
    }

    #[test]
    fn test_maximum_by_branch_and_node() {
        let mut tree = Forest::root();

        let a_node = TestNode {
            key: String::from("a"),
            id: 1,
        };

        let b_node = TestNode {
            key: String::from("b"),
            id: 3,
        };

        tree.insert(vec![String::from("c")], a_node.clone());
        tree.insert(vec![], b_node.clone());

        assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
        assert_eq!(
            tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
            Some(&a_node)
        );
    }

    #[test]
    fn test_maximum_by_branch_and_branch() {
        let mut tree = Forest::root();

        let a_node = TestNode {
            key: String::from("a"),
            id: 1,
        };

        let b_node = TestNode {
            key: String::from("b"),
            id: 3,
        };

        tree.insert(vec![String::from("c")], a_node.clone());
        tree.insert(vec![String::from("d")], b_node.clone());

        assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
        assert_eq!(
            tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
            Some(&a_node)
        );
    }

    #[test]
    fn test_maximum_by_branch_nodes() {
        let mut tree = Forest::root();

        let a_node = TestNode {
            key: String::from("a"),
            id: 1,
        };

        let b_node = TestNode {
            key: String::from("b"),
            id: 3,
        };

        tree.insert(vec![String::from("c")], a_node.clone());
        tree.insert(vec![String::from("c")], b_node.clone());

        assert_eq!(tree.maximum_by(|a, b| a.id.cmp(&b.id)), Some(&b_node));
        assert_eq!(
            tree.maximum_by(|a, b| a.id.cmp(&b.id).reverse()),
            Some(&a_node)
        );
    }
}
