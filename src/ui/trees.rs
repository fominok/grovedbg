//! Grove structure representation with egui-snarl

use std::collections::{BTreeMap, VecDeque};

use eframe::egui;
use egui_snarl::{
    ui::{PinInfo, SnarlStyle, SnarlViewer},
    InPinId, OutPinId, Snarl,
};

use super::commons::{binary_label, bytes_by_display_variant, DisplayVariant};
use crate::{
    trees::{self, InnerTree, InnerTreeNodeValue},
    Key,
};

const X_MARGIN: f32 = 500.0;
const Y_MARGIN_MIN: f32 = 200.0;
const Y_MARGIN_PER_PIN: f32 = 20.0;

struct SnarlInnerTreeNode {
    key: Vec<u8>,
    key_display_variant: DisplayVariant,
    value: InnerTreeNodeValue,
}

/// Draws a generic tree
pub(crate) fn draw_iner_tree(snarl: &mut Snarl<SnarlInnerTreeNode>, tree: &InnerTree) {
    let mut deque: VecDeque<(Option<egui_snarl::NodeId>, usize, &[u8])> = VecDeque::new();
    let Some(root_id) = &tree.root_node_key else {
        return;
    };

    deque.push_back((None, 0, root_id));

    let levels_count = tree.nodes.len().ilog2() + 1;
    let max_level_count = 2u32.pow(levels_count - 1);

    // Keeps track of how many subtree nodes were placed at the same level to
    // compute poisiton accordingly
    let mut levels_counters = vec![0; levels_count as usize];

    // Keeps track of how many child nodes for each node were connected so far to
    // use the next available pin
    let mut child_counters = BTreeMap::new();

    // Adjusted vertical space a node with margin occupies depending

    // The maximum heigh (or how wide the tree is at the most populated level) to
    // achieve some symmetry and use space evenly
    let max_height = max_level_count as f32 * Y_MARGIN_MIN;

    while let Some((parent_key, level, subtree_key)) = deque.pop_front() {
        let level_margin = max_height / (2f32.powi(level as i32) + 1f32);

        let node_id = snarl.insert_node(
            (
                X_MARGIN * level as f32,
                level_margin * (levels_counters[level] + 1) as f32,
            )
                .into(),
            SnarlInnerTreeNode {
                key: subtree_key.to_owned(),
                key_display_variant: DisplayVariant::String,
                value: tree.nodes[subtree_key].value.clone(),
            },
        );
        levels_counters[level] += 1;
        if let Some(parent_key) = parent_key {
            let parent_out_pin_idx = child_counters.get(&parent_key).copied().unwrap_or_default();
            child_counters.insert(parent_key, parent_out_pin_idx + 1);
            snarl.connect(
                OutPinId {
                    node: parent_key,
                    output: parent_out_pin_idx,
                },
                InPinId {
                    node: node_id,
                    input: 0,
                },
            );
        }

        if let Some(left) = &tree.nodes[subtree_key].left {
            deque.push_back((Some(node_id), level + 1, left));
        }
        if let Some(right) = &tree.nodes[subtree_key].right {
            deque.push_back((Some(node_id), level + 1, right));
        }
    }
}

pub(crate) struct InnerTreeNodeViewer;

impl SnarlViewer<SnarlInnerTreeNode> for InnerTreeNodeViewer {
    fn show_header(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlInnerTreeNode>,
    ) {
        let node = &mut snarl[node];

        ui.set_min_width(X_MARGIN / 2.0 * scale);
        binary_label(ui, &node.key, &mut node.key_display_variant);
    }

    fn outputs(&mut self, node: &SnarlInnerTreeNode) -> usize {
        2
    }

    fn inputs(&mut self, _node: &SnarlInnerTreeNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        _ui: &mut egui::Ui,
        _scale: f32,
        _snarl: &mut Snarl<SnarlInnerTreeNode>,
    ) -> PinInfo {
        PinInfo::default()
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlInnerTreeNode>,
    ) -> egui_snarl::ui::PinInfo {
        pin.remotes.get(0).into_iter().for_each(|remote| {
            let node = &snarl[remote.node];
            let text = bytes_by_display_variant(&node.key, &node.key_display_variant);
            ui.label(text);
        });
        PinInfo::default()
    }

    fn has_body(&mut self, _node: &SnarlInnerTreeNode) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlInnerTreeNode>,
    ) {
    }

    fn title(&mut self, node: &SnarlInnerTreeNode) -> String {
        // Not needed since `show_header` is implemented
        todo!()
    }

    fn input_color(
        &mut self,
        _pin: &egui_snarl::InPin,
        _style: &egui::Style,
        _snarl: &mut Snarl<SnarlInnerTreeNode>,
    ) -> egui::Color32 {
        todo!()
    }

    fn output_color(
        &mut self,
        _pin: &egui_snarl::OutPin,
        _style: &egui::Style,
        _snarl: &mut Snarl<SnarlInnerTreeNode>,
    ) -> egui::Color32 {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct SnarlSubtreeNode {
    key: Key,
    children: Vec<Key>,
    key_display_variant: DisplayVariant,
    context: egui::Context,
    showing_inner_tree: bool,
    inner_tree: InnerTree,
}

/// Draw an acyclic graph of subtrees (meaning only upper level trees are nodes
/// to be drawn)
pub(crate) fn draw_subtrees(
    context: egui::Context,
    snarl: &mut Snarl<SnarlSubtreeNode>,
    tree: &trees::Tree,
) {
    let mut deque: VecDeque<(Option<egui_snarl::NodeId>, usize, trees::SubtreeNodeId)> =
        VecDeque::new();
    deque.push_back((None, 0, tree.root_subtree_id()));

    // Keeps track of how many subtree nodes were placed at the same level to
    // compute poisiton accordingly
    let mut levels_counters = vec![0; tree.levels_count.len()];

    // Keeps track of how many child nodes for each node were connected so far to
    // use the next available pin
    let mut child_counters = BTreeMap::new();

    // Adjusted vertical space a node with margin occupies depending
    let y_margin = Y_MARGIN_MIN + tree.max_children_count as f32 * Y_MARGIN_PER_PIN;

    // The maximum heigh (or how wide the tree is at the most populated level) to
    // achieve some symmetry and use space evenly
    let max_height = tree.max_level_count() as f32 * y_margin;

    while let Some((parent_ui_node_id, level, subtree_id)) = deque.pop_front() {
        let Some(subtree) = tree.subtree_by_id(subtree_id) else {
            continue;
        };

        let (children_ids, children_keys): (Vec<trees::SubtreeNodeId>, Vec<Key>) = tree
            .iter_subtree_children(subtree_id)
            .map(|(child_id, child_node)| (child_id, child_node.key().clone()))
            .unzip();

        let level_margin = max_height / (tree.levels_count[level] + 1) as f32;

        let node_id = snarl.insert_node(
            (
                X_MARGIN * level as f32,
                level_margin * (levels_counters[level] + 1) as f32,
            )
                .into(),
            SnarlSubtreeNode {
                inner_tree: subtree.inner_tree.clone(),
                context: context.clone(),
                key: subtree.key().clone(),
                children: children_keys,
                key_display_variant: DisplayVariant::String,
                showing_inner_tree: false,
            },
        );
        levels_counters[level] += 1;
        if let Some(parent_id) = parent_ui_node_id {
            let parent_out_pin_idx = child_counters.get(&parent_id).copied().unwrap_or_default();
            child_counters.insert(parent_id, parent_out_pin_idx + 1);
            snarl.connect(
                OutPinId {
                    node: parent_id,
                    output: parent_out_pin_idx,
                },
                InPinId {
                    node: node_id,
                    input: 0,
                },
            );
        }

        children_ids.into_iter().for_each(|child_id| {
            deque.push_back((Some(node_id), level + 1, child_id));
        });
    }
}

pub(crate) struct SubtreeNodeViewer;

impl SnarlViewer<SnarlSubtreeNode> for SubtreeNodeViewer {
    fn show_header(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) {
        let node = &mut snarl[node];

        if ui.button("🖧").clicked() {
            node.showing_inner_tree = true;
        }

        ui.set_min_width(X_MARGIN / 2.0 * scale);
        binary_label(ui, &node.key, &mut node.key_display_variant);

        if node.showing_inner_tree {
            egui::Window::new("Merk tree")
                .open(&mut node.showing_inner_tree)
                .show(&node.context, |ui| {
                    let mut snarl: Snarl<SnarlInnerTreeNode> = Snarl::new();
                    draw_iner_tree(&mut snarl, &node.inner_tree);
                    snarl.show(
                        &mut InnerTreeNodeViewer,
                        &SnarlStyle::default(),
                        egui::Id::new("snarl_merk"),
                        ui,
                    );
                });
        }
    }

    fn outputs(&mut self, node: &SnarlSubtreeNode) -> usize {
        node.children.len()
    }

    fn inputs(&mut self, _node: &SnarlSubtreeNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        _ui: &mut egui::Ui,
        _scale: f32,
        _snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> PinInfo {
        PinInfo::default()
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui_snarl::ui::PinInfo {
        pin.remotes.get(0).into_iter().for_each(|remote| {
            let node = &snarl[remote.node];
            let text = bytes_by_display_variant(&node.key, &node.key_display_variant);
            ui.label(text);
        });
        PinInfo::default()
    }

    fn has_body(&mut self, _node: &SnarlSubtreeNode) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) {
    }

    fn title(&mut self, node: &SnarlSubtreeNode) -> String {
        // Not needed since `show_header` is implemented
        todo!()
    }

    fn input_color(
        &mut self,
        _pin: &egui_snarl::InPin,
        _style: &egui::Style,
        _snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui::Color32 {
        todo!()
    }

    fn output_color(
        &mut self,
        _pin: &egui_snarl::OutPin,
        _style: &egui::Style,
        _snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui::Color32 {
        todo!()
    }
}
