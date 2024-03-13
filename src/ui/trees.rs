//! Grove structure representation with egui-snarl

use std::collections::{BTreeMap, VecDeque};

use eframe::egui;
use egui_snarl::{
    ui::{PinInfo, SnarlViewer},
    InPinId, OutPinId, Snarl,
};

use super::commons::{binary_label, bytes_by_display_variant, DisplayVariant};
use crate::{trees, Key};

const X_MARGIN: f32 = 500.0;
const Y_MARGIN_MIN: f32 = 200.0;
const Y_MARGIN_PER_PIN: f32 = 20.0;

#[derive(Debug)]
pub(crate) struct SnarlSubtreeNode {
    key: Key,
    children: Vec<Key>,
    key_display_variant: DisplayVariant,
}

/// Draw an acyclic graph of subtrees (meaning only upper level trees are nodes
/// to be drawn)
pub(crate) fn draw_subtrees(snarl: &mut Snarl<SnarlSubtreeNode>, tree: &trees::Tree) {
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
                key: subtree.key().clone(),
                children: children_keys,
                key_display_variant: DisplayVariant::String,
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

pub(crate) struct Viewer;

impl SnarlViewer<SnarlSubtreeNode> for Viewer {
    fn title(&mut self, node: &SnarlSubtreeNode) -> String {
        todo!()
        // String::from_utf8_lossy(&node.key).to_string()
    }

    fn show_header(
        &mut self,
        node: egui_snarl::NodeId,
        inputs: &[egui_snarl::InPin],
        outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) {
        let node = &mut snarl[node];
        ui.set_min_width(X_MARGIN / 2.0 * scale);
        binary_label(ui, &node.key, &mut node.key_display_variant);
    }

    fn outputs(&mut self, node: &SnarlSubtreeNode) -> usize {
        node.children.len()
    }

    fn inputs(&mut self, node: &SnarlSubtreeNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> PinInfo {
        PinInfo::default()
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui_snarl::ui::PinInfo {
        pin.remotes.get(0).into_iter().for_each(|remote| {
            let node = &snarl[remote.node];
            let text = bytes_by_display_variant(&node.key, &node.key_display_variant);
            ui.label(text);
        });
        PinInfo::default()
    }

    fn input_color(
        &mut self,
        pin: &egui_snarl::InPin,
        style: &egui::Style,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui::Color32 {
        todo!()
    }

    fn output_color(
        &mut self,
        pin: &egui_snarl::OutPin,
        style: &egui::Style,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui::Color32 {
        todo!()
    }
}
