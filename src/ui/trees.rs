//! Grove structure representation with egui-snarl

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use eframe::egui;
use egui_snarl::{
    ui::{PinInfo, SnarlStyle, SnarlViewer},
    InPinId, OutPinId, Snarl,
};

use super::commons::{binary_label, bytes_by_display_variant, DisplayVariant};
use crate::{
    trees::{self, InnerTree, InnerTreeNodeValue},
    Key, Path,
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
    children: BTreeSet<Key>,
    references: Vec<(Path, Key, DisplayVariant)>,
    key_display_variant: DisplayVariant,
    context: egui::Context,
    showing_inner_tree: bool,
    inner_tree: InnerTree,
    refererred_keys: BTreeMap<Key, (usize, DisplayVariant)>,
}

/// Draw an acyclic graph of subtrees (meaning only upper level trees are nodes
/// to be drawn)
pub(crate) fn draw_subtrees(
    context: egui::Context,
    snarl: &mut Snarl<SnarlSubtreeNode>,
    tree: &trees::Tree,
) {
    let mut deque: VecDeque<(Option<egui_snarl::NodeId>, usize, &trees::SubtreeNode)> =
        VecDeque::new();
    deque.push_back((None, 0, &tree.subtrees[[].as_ref()]));

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

    // Referenced subtrees input pins index structure
    let mut referenced_pins: BTreeMap<(Path, Key), (egui_snarl::NodeId, usize)> = BTreeMap::new();
    let mut reference_wires: Vec<(Path, Key, OutPinId)> = Vec::new();

    while let Some((parent_snarl_id, level, subtree)) = deque.pop_front() {
        let level_margin = max_height / (tree.levels_count[level] + 1) as f32;

        let mut path = subtree.parent_path.clone().unwrap_or_default();
        if let Some(key) = &subtree.key {
            path.push(key.clone());
        }

        let node_id = snarl.insert_node(
            (
                X_MARGIN * level as f32,
                level_margin * (levels_counters[level] + 1) as f32,
            )
                .into(),
            SnarlSubtreeNode {
                inner_tree: subtree.inner_tree.clone(),
                context: context.clone(),
                key: subtree
                    .key
                    .clone()
                    .unwrap_or_else(|| "ROOT".to_owned().into()),
                children: subtree.children.clone(),
                key_display_variant: DisplayVariant::String,
                showing_inner_tree: false,
                refererred_keys: tree
                    .referred_keys
                    .get(&path)
                    .into_iter()
                    .map(|set| {
                        set.into_iter()
                            .enumerate()
                            .map(|(i, key)| (key.clone(), (i + 1, DisplayVariant::String)))
                    })
                    .flatten()
                    .collect::<BTreeMap<_, _>>(),
                references: subtree
                    .inner_tree
                    .nodes
                    .iter()
                    .filter_map(|(key, item)| match &item.value {
                        InnerTreeNodeValue::Reference(path, _) => {
                            Some((path.clone(), key.clone(), DisplayVariant::String))
                        }
                        _ => None,
                    })
                    .collect(),
            },
        );

        // Populate references index with input pins of added subtree node
        for (key, (pin_id, _)) in &snarl[node_id].refererred_keys {
            let mut path = subtree.parent_path.clone().unwrap_or_default();
            if let Some(k) = &subtree.key {
                path.push(k.clone());
            }
            referenced_pins.insert((path, key.clone()), (node_id, *pin_id));
        }

        levels_counters[level] += 1;
        if let Some(parent_id) = parent_snarl_id {
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

        subtree.children.iter().for_each(|child_key| {
            let mut path = subtree.parent_path.clone().unwrap_or_default();
            if let Some(key) = &subtree.key {
                path.push(key.clone());
            }
            path.push(child_key.clone());
            deque.push_back((Some(node_id), level + 1, &tree.subtrees[&path]));
        });

        let mut ref_out_counter = snarl[node_id].children.len();
        subtree.inner_tree.nodes.values().for_each(|node| {
            if let InnerTreeNodeValue::Reference(path, key) = &node.value {
                reference_wires.push((
                    path.clone(),
                    key.clone(),
                    OutPinId {
                        node: node_id,
                        output: ref_out_counter,
                    },
                ));
                ref_out_counter += 1;
            }
        });
    }

    // Connect reference pins
    for (path, key, out_pin) in reference_wires {
        if let Some((node, input)) = referenced_pins.remove(&(path, key)) {
            snarl.connect(out_pin, InPinId { node, input });
        }
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
        node.children.len() + node.references.len()
    }

    fn inputs(&mut self, node: &SnarlSubtreeNode) -> usize {
        1 + node.refererred_keys.len()
    }

    fn show_input(
        &mut self,
        pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> PinInfo {
        if pin.id.input > 0 {
            let referred_key = snarl[pin.id.node]
                .refererred_keys
                .iter_mut()
                .find(|(_, (idx, _))| *idx == pin.id.input);
            if let Some((key, (_, display_variant))) = referred_key {
                binary_label(ui, &key, display_variant);
            }
            PinInfo::default().with_fill(egui::Color32::GREEN)
        } else {
            PinInfo::default()
        }
    }

    fn show_output(
        &mut self,
        pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        scale: f32,
        snarl: &mut Snarl<SnarlSubtreeNode>,
    ) -> egui_snarl::ui::PinInfo {
        let node = &mut snarl[pin.id.node];
        if pin.id.output < node.children.len() {
            pin.remotes.get(0).into_iter().for_each(|remote| {
                let node = &snarl[remote.node];
                let text = bytes_by_display_variant(&node.key, &node.key_display_variant);
                ui.label(text);
            });
            PinInfo::default()
        } else {
            let reference_key = &mut node.references[pin.id.output - node.children.len()];
            ui.set_max_size(egui::vec2(150.0 * scale, 20.0));
            binary_label(ui, &reference_key.1, &mut reference_key.2);
            PinInfo::default().with_fill(egui::Color32::GREEN)
        }
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
