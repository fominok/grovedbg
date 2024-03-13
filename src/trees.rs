//! Module for trees representation

use std::{cmp, collections::BTreeMap};

use slab::Slab;

use crate::{Key, Path};

pub(crate) type SubtreeNodeId = usize;
pub(crate) type InnerTreeNodeId = usize;

/// Struct that represents a highlevel tree with GroveDB's subtrees as nodes.
#[derive(Debug)]
pub(crate) struct Tree {
    /// Slab id of the root subtree node
    root_node_id: SubtreeNodeId,
    /// Data structure to hold all subtrees
    nodes: Slab<SubtreeNode>,
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
    key: Key,
    children: BTreeMap<Key, SubtreeNodeId>,
    pub(crate) inner_tree: InnerTree,
    snarl_id: Option<egui_snarl::NodeId>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct InnerTree {
    pub(crate) root_node_key: Option<Key>,
    pub(crate) nodes: BTreeMap<Key, InnerTreeNode>,
}

#[derive(Debug)]
enum Side {
    Left,
    Right,
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
            key: b"ROOT".to_vec(),
            ..Default::default()
        };
        let mut nodes = Slab::default();
        let root_node_id = nodes.insert(root_node);

        Tree {
            levels_count: vec![1],
            root_node_id,
            nodes,
            updated: true,
            max_children_count: 1,
        }
    }

    pub fn max_level_count(&self) -> usize {
        self.levels_count.iter().copied().max().unwrap_or_default()
    }

    pub fn insert(&mut self, path: Path, key: Key, node: InnerTreeNode) {
        let child_level = path.len() + 1;
        let subtree_id = self.get_or_create_subtree_recursive(path);

        // In case of adding a child subtree root we need to create a `SubtreeNode` for
        // it.
        if let InnerTreeNodeValue::Subtree(root_key) = &node.value {
            let (created, child_subtree_id) =
                self.get_or_create_child_subtree(subtree_id, key.clone());
            self.nodes[child_subtree_id].inner_tree.root_node_key = root_key.clone();
            if created {
                if child_level >= self.levels_count.len() {
                    self.levels_count.resize(child_level + 1, 0);
                }
                self.levels_count[child_level] += 1;
            }
        }

        self.nodes[subtree_id].inner_tree.nodes.insert(key, node);
    }

    pub fn root_subtree_id(&self) -> SubtreeNodeId {
        self.root_node_id
    }

    pub fn root_subtree(&self) -> &SubtreeNode {
        &self.nodes[self.root_node_id]
    }

    pub fn subtree_by_id(&self, id: SubtreeNodeId) -> Option<&SubtreeNode> {
        self.nodes.get(id)
    }

    fn get_or_create_child_subtree(
        &mut self,
        node_id: SubtreeNodeId,
        key: Key,
    ) -> (bool, SubtreeNodeId) {
        if self.nodes[node_id].children.contains_key(&key) {
            (false, self.nodes[node_id].children[&key])
        } else {
            let subtree_id = self.nodes.insert(SubtreeNode {
                key: key.clone(),
                ..Default::default()
            });
            self.nodes[node_id].children.insert(key, subtree_id);
            self.max_children_count =
                cmp::max(self.max_children_count, self.nodes[node_id].children.len());
            (true, subtree_id)
        }
    }

    fn get_or_create_subtree_recursive(&mut self, path: Path) -> SubtreeNodeId {
        let mut current_level = 0;
        let mut path_iter = path.into_iter();

        let mut current_node_id = self.root_node_id;

        while let Some(path_segment) = path_iter.next() {
            current_level += 1;
            let created;
            (created, current_node_id) =
                self.get_or_create_child_subtree(current_node_id, path_segment);
            if created {
                if current_level > self.levels_count.len() {
                    self.levels_count.resize(current_level + 1, 0);
                }
                self.levels_count[current_level] += 1;
            }
        }

        current_node_id
    }

    pub fn iter_subtree_children(
        &self,
        id: SubtreeNodeId,
    ) -> impl Iterator<Item = (SubtreeNodeId, &SubtreeNode)> {
        self.nodes
            .get(id)
            .into_iter()
            .map(|node| node.children.values().map(|id| (*id, &self.nodes[*id])))
            .flatten()
    }

    pub fn iter_subtree_children_ids<'a>(
        &'a self,
        id: SubtreeNodeId,
    ) -> impl Iterator<Item = SubtreeNodeId> + 'a {
        self.nodes
            .get(id)
            .into_iter()
            .map(|node| node.children.values().copied())
            .flatten()
    }

    pub fn get_inner_tree_root(&self, id: SubtreeNodeId) -> Option<&InnerTreeNode> {
        self.nodes
            .get(id)
            .map(|node| {
                let root_node_key = &node.inner_tree.root_node_key;
                root_node_key
                    .as_ref()
                    .map(|key| node.inner_tree.nodes.get(key))
                    .flatten()
            })
            .flatten()
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

impl SubtreeNode {
    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn inner_tree(&self) -> &InnerTree {
        &self.inner_tree
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

        let deep_subtree_id = tree.get_or_create_subtree_recursive(vec![
            b"0".to_vec(),
            b"1".to_vec(),
            b"2".to_vec(),
            b"3".to_vec(),
            b"4".to_vec(),
        ]);
        let deep_subtree = &tree.nodes[deep_subtree_id];

        assert_eq!(deep_subtree.key, b"4".to_vec());
        assert_eq!(tree.root_subtree().key, b"".to_vec());
        assert_eq!(
            tree.iter_subtree_children(tree.root_node_id)
                .map(|(_, node)| node.key().clone())
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

        let root_subtree = tree.root_subtree();
        let inner_tree = root_subtree.inner_tree();
        let value: Option<&InnerTreeNodeValue> = inner_tree
            .root_node()
            .and_then(|node| node.left_child(inner_tree))
            .and_then(|node| node.right_child(inner_tree))
            .map(|node| &node.value);
        assert_eq!(
            value,
            Some(InnerTreeNodeValue::Scalar(b"value112".to_vec())).as_ref()
        );
    }
}
