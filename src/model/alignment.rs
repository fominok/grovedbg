//! Nodes/Subtrees alignment implementation.

use std::{collections::BTreeMap, mem};

use super::Subtree;
use crate::model::{Path, Tree};

const NODE_WIDTH: f32 = 100.;
const NODE_HEIGHT: f32 = 100.;
pub(super) const COLLAPSED_SUBTREE_WIDTH: f32 = 400.;
pub(super) const COLLAPSED_SUBTREE_HEIGHT: f32 = 600.;
// pub(crate) const HORIZONTAL_MARGIN: f32 = 100.;
// const VERTICAL_MARGIN: f32 = 50.;

fn levels_count(n_nodes: usize) -> u32 {
    if n_nodes > 0 {
        n_nodes.ilog2() + 1
    } else {
        0
    }
}

fn leaves_level_count(n_levels: u32) -> u32 {
    if n_levels > 0 {
        2u32.pow(n_levels - 1)
    } else {
        0
    }
}

pub(super) fn expanded_subtree_dimentions(subtree: &Subtree) -> (f32, f32) {
    if let Some(root_node) = subtree.root_node() {
        let mut visible_nodes_n = 0;
        let mut queue = vec![root_node];
        if let Some(node) = queue.pop() {
            visible_nodes_n += 1;
            let state = node.ui_state.borrow();
            state
                .show_left
                .then_some(node.left_child.as_ref())
                .flatten()
                .and_then(|key| subtree.nodes.get(key))
                .into_iter()
                .for_each(|node| queue.push(node));
            state
                .show_right
                .then_some(node.right_child.as_ref())
                .flatten()
                .and_then(|key| subtree.nodes.get(key))
                .into_iter()
                .for_each(|node| queue.push(node));
        }

        let levels = levels_count(visible_nodes_n);
        let leaves = leaves_level_count(levels);

        (
            leaves as f32 * NODE_WIDTH,  // + (leaves - 1) as f32 * HORIZONTAL_MARGIN,
            levels as f32 * NODE_HEIGHT, // + (levels - 1) as f32 * VERTICAL_MARGIN,
        )
    } else {
        (0., 0.)
    }
}

pub(crate) struct AlignmentStats<'a> {
    pub max_width: f32,
    pub by_level: Vec<LevelAlignmentStats<'a>>,
}

#[derive(Default)]
pub(crate) struct LevelAlignmentStats<'a> {
    pub max_height: f32,
    pub expanded_subtree_width: BTreeMap<&'a Path, f32>,
}

impl<'a> AlignmentStats<'a> {
    pub(crate) fn new(tree: &'a Tree) -> Self {
        let mut max_width: f32 = 0.;
        let mut current_level_idx = 0;
        let mut current_level_width: f32 = 0.;
        let mut level_alignment_stats = Vec::new();
        let mut current_level_alignment_stats = LevelAlignmentStats::default();
        let mut current_level_subtrees_count = 0;

        for subtree in tree.iter_subtrees() {
            let level = subtree.path().len();
            if level != current_level_idx {
                max_width = max_width.max(
                    current_level_width, /* + (current_level_subtrees_count - 1) as f32 *
                                          *   HORIZONTAL_MARGIN, */
                );
                level_alignment_stats.push(mem::take(&mut current_level_alignment_stats));
                current_level_idx += 1;
                current_level_subtrees_count = 0;
                current_level_width = 0.;
            }

            if !subtree.subtree().visible() {
                continue;
            }

            if subtree.subtree().is_expanded() {
                if let Some(root_node) = subtree.subtree().root_node() {
                    let mut visible_nodes_n = 0;
                    let mut queue = vec![root_node];
                    if let Some(node) = queue.pop() {
                        visible_nodes_n += 1;
                        let state = node.ui_state.borrow();
                        state
                            .show_left
                            .then_some(node.left_child.as_ref())
                            .flatten()
                            .and_then(|key| subtree.subtree().nodes.get(key))
                            .into_iter()
                            .for_each(|node| queue.push(node));
                        state
                            .show_right
                            .then_some(node.right_child.as_ref())
                            .flatten()
                            .and_then(|key| subtree.subtree().nodes.get(key))
                            .into_iter()
                            .for_each(|node| queue.push(node));
                    }

                    let levels = levels_count(visible_nodes_n);
                    current_level_alignment_stats.max_height = current_level_alignment_stats
                        .max_height
                        .max(levels as f32 * NODE_HEIGHT); //  + (levels - 1) as f32 * VERTICAL_MARGIN);
                    let leaves = leaves_level_count(levels);
                    current_level_alignment_stats.expanded_subtree_width.insert(
                        subtree.path(),
                        leaves as f32 * NODE_WIDTH, // + (leaves - 1) as f32 * HORIZONTAL_MARGIN,
                    );
                } else {
                    continue;
                }
            } else {
                current_level_width += COLLAPSED_SUBTREE_WIDTH;
            }

            current_level_subtrees_count += 1;
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levels_count() {
        assert_eq!(levels_count(0), 0);
        assert_eq!(levels_count(1), 1);
        assert_eq!(levels_count(2), 2);
        assert_eq!(levels_count(3), 2);
        assert_eq!(levels_count(4), 3);
        assert_eq!(levels_count(7), 3);
        assert_eq!(levels_count(8), 4);
    }

    #[test]
    fn test_leaves_level_count() {
        assert_eq!(leaves_level_count(0), 0);
        assert_eq!(leaves_level_count(1), 1);
        assert_eq!(leaves_level_count(2), 2);
        assert_eq!(leaves_level_count(3), 4);
        assert_eq!(leaves_level_count(levels_count(8)), 8);
        assert_eq!(leaves_level_count(levels_count(15)), 8);
    }
}
