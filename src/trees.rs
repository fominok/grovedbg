//! Module for trees representation

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

use crate::{Key, Path};

/// Struct that represents a highlevel tree with GroveDB's subtrees as nodes.
#[derive(Debug)]
pub(crate) struct Tree {
    /// Data structure to hold all subtrees
    pub subtrees: BTreeMap<Path, SubtreeNode>,
    /// `Level: count` mapping to store how many subtrees are on each level used
    /// for better visualization of tree structures
    pub levels_count: Vec<usize>,
    /// Flag tree as updated to initialize re-rendering
    pub updated: bool,
    /// Shows the max amount of children that ever happened for a subtree,
    /// required to see how much space a node occupy regardless of content
    pub max_children_count: usize,
}

#[derive(Debug, Default)]
pub(crate) struct SubtreeNode {
    pub key: Option<Key>,
    pub parent_path: Option<Path>,
    pub children: BTreeSet<Key>,
    pub inner_tree: InnerTree,
    pub referred_keys: BTreeSet<Key>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct InnerTree {
    pub(crate) root_node_key: Option<Key>,
    pub(crate) nodes: BTreeMap<Key, InnerTreeNode>,
}

#[derive(Debug, Clone)]
pub(crate) struct InnerTreeNode {
    pub(crate) value: InnerTreeNodeValue,
    pub(crate) left: Option<Key>,
    pub(crate) right: Option<Key>,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum InnerTreeNodeValue {
    Scalar(Vec<u8>),
    Subtree(Option<Key>),
    Reference(Path, Key),
}

impl Tree {
    /// Create a tree with one empty subtree that is called Root Tree in the
    /// GroveDB domain.
    pub fn new(root_subtree_inner_tree_key: Key) -> Self {
        let root_node = SubtreeNode {
            inner_tree: InnerTree {
                root_node_key: Some(root_subtree_inner_tree_key),
                ..Default::default()
            },
            ..Default::default()
        };

        let mut subtrees = BTreeMap::new();
        subtrees.insert(vec![], root_node);

        Tree {
            levels_count: vec![1],
            subtrees,
            updated: true,
            max_children_count: 1,
        }
    }

    pub fn max_level_count(&self) -> usize {
        self.levels_count.iter().copied().max().unwrap_or_default()
    }

    pub fn insert(&mut self, path: Path, key: Key, node: InnerTreeNode) {
        let child_level = path.len() + 1;
        self.get_or_create_subtree_recursive(path.clone());

        // In case of adding a child subtree root we need to create a `SubtreeNode` for
        // it.
        if let InnerTreeNodeValue::Subtree(root_key) = &node.value {
            let mut child_path = path.clone();
            child_path.push(key.clone());
            let (created, child_subtree) = self.get_or_create_child_subtree(child_path);
            child_subtree.inner_tree.root_node_key = root_key.clone();
            self.subtrees
                .get_mut(&path)
                .expect("inserted above")
                .children
                .insert(key.clone());
            if created {
                if child_level >= self.levels_count.len() {
                    self.levels_count.resize(child_level + 1, 0);
                }
                self.levels_count[child_level] += 1;
            }
        }

        // In case of adding a reference the referenced subtree should be aware of it to
        // draw a pin after
        if let InnerTreeNodeValue::Reference(ref_path, key) = &node.value {
            self.subtrees
                .get_mut(ref_path)
                .expect("inserted above")
                .referred_keys
                .insert(key.clone());
        }

        self.subtrees
            .get_mut(&path)
            .expect("inserted above")
            .inner_tree
            .nodes
            .insert(key, node);
    }

    fn get_or_create_child_subtree(&mut self, subtree_path: Path) -> (bool, &mut SubtreeNode) {
        match self.subtrees.entry(subtree_path.clone()) {
            Entry::Vacant(entry) => {
                let mut parent_path = subtree_path;
                let key = parent_path.pop();
                let subtree = SubtreeNode {
                    key,
                    parent_path: Some(parent_path),
                    ..Default::default()
                };
                (true, entry.insert(subtree))
            }
            Entry::Occupied(subtree) => (false, subtree.into_mut()),
        }
    }

    fn get_or_create_subtree_recursive(&mut self, path: Path) {
        let mut current_level = 0;
        let mut current_path = vec![];
        let mut path_iter = path.into_iter();

        let mut working = true;
        while working {
            let (created, subtree) = self.get_or_create_child_subtree(current_path.clone());
            if let Some(path_segment) = path_iter.next() {
                subtree.children.insert(path_segment.clone());
                current_path.push(path_segment);
            } else {
                working = false;
            }
            if created {
                if current_level >= self.levels_count.len() {
                    self.levels_count.resize(current_level + 1, 0);
                }
                self.levels_count[current_level] += 1;
            }
            current_level += 1;
        }
    }
}

impl InnerTreeNode {
    pub fn left_child<'t>(&self, inner_tree: &'t InnerTree) -> Option<&'t InnerTreeNode> {
        self.left
            .as_ref()
            .map(|left| inner_tree.nodes.get(left))
            .flatten()
    }

    pub fn right_child<'t>(&self, inner_tree: &'t InnerTree) -> Option<&'t InnerTreeNode> {
        self.right
            .as_ref()
            .map(|right| inner_tree.nodes.get(right))
            .flatten()
    }
}

impl InnerTree {
    pub fn root_node(&self) -> Option<&InnerTreeNode> {
        self.root_node_key
            .as_ref()
            .map(|key| self.nodes.get(key))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_or_create_subtree() {
        let mut tree = Tree::new(b"0".to_vec());

        tree.get_or_create_subtree_recursive(vec![b"00".to_vec()]);
        tree.get_or_create_subtree_recursive(vec![
            b"0".to_vec(),
            b"1".to_vec(),
            b"2".to_vec(),
            b"3".to_vec(),
            b"4".to_vec(),
        ]);
        let deep_subtree = &tree
            .subtrees
            .get(
                [
                    b"0".to_vec(),
                    b"1".to_vec(),
                    b"2".to_vec(),
                    b"3".to_vec(),
                    b"4".to_vec(),
                ]
                .as_ref(),
            )
            .unwrap();

        assert_eq!(deep_subtree.key, Some(b"4".to_vec()));
        assert_eq!(tree.subtrees[[].as_ref()].key, None);
        assert_eq!(
            tree.subtrees[[].as_ref()]
                .children
                .iter()
                .cloned()
                .collect::<Vec<Key>>(),
            vec![b"0".to_vec(), b"00".to_vec()]
        );
    }

    #[test]
    fn root_tree_inner_scalars() {
        // GroveDB root
        // └── root subtree (empty `path`)
        //     └── key1: value1
        //         ├── key11: value11
        //         │   ├── key111: value 111
        //         │   └── key112: value 112
        //         └── key12: value12
        //             └── key121: value121

        let mut tree = Tree::new(b"key1".to_vec());

        tree.insert(
            vec![],
            b"key1".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Scalar(b"value1".to_vec()),
                left: Some(b"key11".to_vec()),
                right: Some(b"key12".to_vec()),
            },
        );
        tree.insert(
            vec![],
            b"key11".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Scalar(b"value11".to_vec()),
                left: Some(b"key111".to_vec()),
                right: Some(b"key112".to_vec()),
            },
        );
        tree.insert(
            vec![],
            b"key111".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Scalar(b"value111".to_vec()),
                left: None,
                right: None,
            },
        );
        tree.insert(
            vec![],
            b"key112".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Scalar(b"value112".to_vec()),
                left: None,
                right: None,
            },
        );
        tree.insert(
            vec![],
            b"key12".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Scalar(b"value12".to_vec()),
                left: Some(b"key121".to_vec()),
                right: None,
            },
        );
        tree.insert(
            vec![],
            b"key121".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Scalar(b"value121".to_vec()),
                left: None,
                right: None,
            },
        );

        let root_subtree = tree.subtrees.get([].as_ref()).unwrap();
        let value: Option<&InnerTreeNodeValue> = root_subtree
            .inner_tree
            .root_node()
            .and_then(|node| node.left_child(&root_subtree.inner_tree))
            .and_then(|node| node.right_child(&root_subtree.inner_tree))
            .map(|node| &node.value);
        assert_eq!(
            value,
            Some(InnerTreeNodeValue::Scalar(b"value112".to_vec())).as_ref()
        );
    }

    #[test]
    fn test_nested_subtrees() {
        let mut tree = Tree::new(b"root_tree_merk_root".to_vec());
        tree.insert(
            vec![],
            b"root_tree_merk_root".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Subtree(Some(b"key01".to_vec())),
                left: Some(b"key1".to_vec()),
                right: Some(b"key2".to_vec()),
            },
        );
        tree.insert(
            vec![],
            b"key1".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Subtree(Some(b"key11".to_vec())),
                left: None,
                right: None,
            },
        );
        tree.insert(
            vec![],
            b"key2".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Subtree(Some(b"key21".to_vec())),
                left: None,
                right: None,
            },
        );
        tree.insert(
            vec![b"key2".to_vec()],
            b"key21".to_vec(),
            InnerTreeNode {
                value: InnerTreeNodeValue::Subtree(Some(b"key211".to_vec())),
                left: None,
                right: None,
            },
        );
    }
}
