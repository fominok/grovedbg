//! Tree structure UI module

use eframe::{
    egui::{self, Id},
    emath::TSTransform,
    epaint::{Color32, Pos2, Rect, Stroke},
};

use super::{
    common::{binary_label_colored, path_label},
    node::{draw_element, draw_node, element_to_color},
};
use crate::model::{Element, Key, KeySlice, LevelInfo, LevelsInfo, Node, Path, Subtree, Tree};

const NODE_WIDTH: f32 = 200.0;
const NODE_HEIGHT: f32 = 30.0;
const X_MARGIN: f32 = 100.0;
const Y_MARGIN: f32 = 200.0;

fn subtree_block_size(level_info: &LevelInfo) -> (f32, f32) {
    if level_info.max_subtree_size == 0 {
        return (0.0, 0.0);
    }
    let levels = level_info.max_subtree_size.ilog2() + 1;
    let leaves_level_width = 2u32.pow(levels - 1) * level_info.max_clusters as u32;

    (
        (X_MARGIN + NODE_WIDTH) * leaves_level_width as f32,
        (Y_MARGIN + NODE_HEIGHT) * levels as f32,
    )
}

pub(crate) struct TreeDrawer<'u, 't> {
    ui: &'u mut egui::Ui,
    transform: TSTransform,
    rect: Rect,
    references: Vec<(Pos2, Path, Key)>,
    tree: &'t Tree,
    levels: LevelsInfo,
}

impl<'u, 't> TreeDrawer<'u, 't> {
    pub(crate) fn new(
        ui: &'u mut egui::Ui,
        transform: TSTransform,
        rect: Rect,
        tree: &'t Tree,
    ) -> Self {
        Self {
            ui,
            transform,
            rect,
            references: vec![],
            tree,
            levels: tree.levels(),
        }
    }

    fn draw_node_area<'b>(
        &mut self,
        parent_coords: Option<Pos2>,
        coords: Pos2,
        subtree: &'b Subtree,
        key: KeySlice,
    ) -> Option<&'b Node> {
        let mut node = None;
        let layer_response = egui::Area::new(Id::new((
            parent_coords.map(|c| c.x).unwrap_or_default() as u32,
            parent_coords.map(|c| c.y).unwrap_or_default() as u32,
            key,
        )))
        .default_pos(coords)
        .order(egui::Order::Foreground)
        .show(self.ui.ctx(), |ui| {
            ui.set_clip_rect(self.transform.inverse() * self.rect);
            node = draw_node(ui, subtree, key);
            if let (Some(node), Some(out_coords)) = (&node, parent_coords) {
                let painter = ui.painter();
                painter.line_segment(
                    [out_coords, node.ui_state.borrow().input_point],
                    Stroke {
                        width: 1.0,
                        color: Color32::GRAY,
                    },
                );
            }
        })
        .response;

        if node.is_none() {
            return None;
        }

        layer_response.context_menu(|menu| {
            if menu.button("Collapse").clicked() {
                subtree.ui_state.borrow_mut().expanded = false;
            }
        });

        node.iter_mut().for_each(|n| {
            let mut state = n.ui_state.borrow_mut();
            state.input_point = layer_response.rect.center_top();
            state.output_point = layer_response.rect.center_bottom();
            state.left_sibling_point = layer_response.rect.left_center();
            state.right_sibling_point = layer_response.rect.right_center();
        });
        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
        node
    }

    fn draw_subtree_part(&mut self, mut coords: Pos2, subtree: &Subtree, key: KeySlice) {
        let mut current_level_nodes: Vec<(Option<Key>, Option<Key>)> = Vec::new();
        let mut next_level_nodes: Vec<(Option<Key>, Option<Key>)> = Vec::new();
        let mut level = 0;

        current_level_nodes.push((None, Some(key.to_vec())));

        let x_base = coords.x;

        loop {
            if level > 0 {
                coords.x = x_base - 2f32.powi(level - 2) * (X_MARGIN + NODE_WIDTH);
            }

            for (parent_key, node_key) in current_level_nodes.drain(..) {
                if let Some(node_key) = node_key {
                    let parent_out_coords = parent_key.map(|k| {
                        subtree
                            .nodes
                            .get(&k)
                            .unwrap()
                            .ui_state
                            .borrow()
                            .output_point
                    });
                    let node = self.draw_node_area(parent_out_coords, coords, subtree, &node_key);

                    if let Some((Element::Reference { path, key }, out)) =
                        node.map(|n| (&n.element, n.ui_state.borrow().output_point))
                    {
                        self.references
                            .push((out.clone(), path.clone(), key.clone()));
                    }

                    next_level_nodes.push((
                        Some(node_key.clone()),
                        node.as_ref().map(|n| n.left_child.clone()).flatten(),
                    ));
                    next_level_nodes.push((
                        Some(node_key),
                        node.as_ref().map(|n| n.right_child.clone()).flatten(),
                    ));
                }
                coords.x += X_MARGIN + NODE_WIDTH;
            }

            if next_level_nodes.is_empty() {
                break;
            }

            coords.y += NODE_HEIGHT + Y_MARGIN;
            std::mem::swap(&mut current_level_nodes, &mut next_level_nodes);
            level += 1;
        }
    }

    fn draw_subtree(&mut self, path: &Path, coords: Pos2, subtree_width: f32, subtree: &Subtree) {
        if subtree.ui_state.borrow().expanded {
            self.draw_subtree_expanded(coords, subtree_width, subtree);
        } else {
            self.draw_subtree_collapsed(path, coords, subtree_width, subtree);
        }
    }

    fn draw_subtree_collapsed(
        &mut self,
        path: &Path,
        coords: Pos2,
        _subtree_width: f32,
        subtree: &Subtree,
    ) {
        let layer_response = egui::Area::new(Id::new((coords.x as u32, coords.y as u32)))
            .default_pos(coords)
            .order(egui::Order::Foreground)
            .show(self.ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * self.rect);

                let mut stroke = Stroke::default();
                stroke.width = 1.0;

                egui::Frame::default()
                    .rounding(egui::Rounding::same(4.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(stroke)
                    .fill(Color32::BLACK)
                    .show(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.collapsing("🖧", |menu| {
                            if menu.button("Expand").clicked() {
                                subtree.ui_state.borrow_mut().expanded = true;
                            }
                        });

                        ui.allocate_ui(
                            egui::Vec2 {
                                x: NODE_WIDTH,
                                y: 10.0,
                            },
                            |ui| ui.separator(),
                        );

                        path_label(
                            ui,
                            path,
                            &mut subtree.ui_state.borrow_mut().path_display_variant,
                        );

                        ui.allocate_ui(
                            egui::Vec2 {
                                x: NODE_WIDTH,
                                y: 10.0,
                            },
                            |ui| ui.separator(),
                        );

                        for (key, node) in subtree.nodes.iter() {
                            if let Element::Reference {
                                path: ref_path,
                                key: ref_key,
                            } = &node.element
                            {
                                if path != ref_path {
                                    self.references.push((
                                        subtree.ui_state.borrow().output_point,
                                        ref_path.clone(),
                                        ref_key.clone(),
                                    ));
                                }
                            }

                            let color = element_to_color(&node.element);

                            binary_label_colored(
                                ui,
                                key,
                                &mut node.ui_state.borrow_mut().key_display_variant,
                                color,
                            );

                            if matches!(
                                node.element,
                                Element::Item { .. }
                                    | Element::SumItem { .. }
                                    | Element::Sumtree { .. }
                                    | Element::Reference { .. }
                            ) {
                                draw_element(ui, node);
                            }

                            ui.allocate_ui(
                                egui::Vec2 {
                                    x: NODE_WIDTH,
                                    y: 10.0,
                                },
                                |ui| ui.separator(),
                            );
                        }
                    });
            })
            .response;

        let mut state = subtree.ui_state.borrow_mut();
        state.input_point = layer_response.rect.center_top();
        state.output_point = layer_response.rect.center_bottom();

        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }

    fn draw_subtree_expanded(&mut self, mut coords: Pos2, subtree_width: f32, subtree: &Subtree) {
        let width_step = subtree_width
            / (subtree.cluster_roots.len() + subtree.root_node.as_ref().map(|_| 1).unwrap_or(0))
                as f32;
        let mut prev_point = None;

        // There is no sense in drawing empty subtree as merk
        if subtree.root_node.is_none() && subtree.cluster_roots.is_empty() {
            subtree.ui_state.borrow_mut().expanded = false;
        }

        subtree
            .root_node
            .iter()
            .chain(subtree.cluster_roots.iter())
            .for_each(|rn| {
                self.draw_subtree_part(coords, subtree, &rn);
                coords.x += width_step;

                if let Some(node) = subtree.nodes.get(rn.as_slice()) {
                    let state = node.ui_state.borrow();

                    if let Some(right_point) = prev_point {
                        // TODO need a better id
                        let layer_response =
                            egui::Area::new(Id::new(("sib", coords.x as u32, coords.y as u32)))
                                .default_pos(Pos2::new(0.0, 0.0))
                                .order(egui::Order::Background)
                                .show(self.ui.ctx(), |ui| {
                                    ui.set_clip_rect(self.transform.inverse() * self.rect);

                                    let painter = ui.painter();
                                    painter.line_segment(
                                        [state.left_sibling_point, right_point],
                                        Stroke {
                                            width: 1.0,
                                            color: Color32::KHAKI,
                                        },
                                    );
                                })
                                .response;
                        self.ui
                            .ctx()
                            .set_transform_layer(layer_response.layer_id, self.transform);
                    }

                    prev_point = Some(state.right_sibling_point);
                }
            });
    }

    pub(crate) fn draw_tree(mut self) {
        let max_width = subtree_block_size(&self.levels.levels_info[self.levels.widest_level_idx])
            .0
            * self.levels.levels_info[self.levels.widest_level_idx].n_subtrees as f32
            / 2.0;

        let mut current_level = 0;
        let mut idx_on_level = 0;

        let mut level_subtree_block_size = subtree_block_size(&self.levels.levels_info[0]);
        let mut level_subtree_width =
            max_width / (self.levels.levels_info[0].n_subtrees + 1) as f32;

        let mut current_pos = Pos2::new(level_subtree_width, 0.0);

        for (path, subtree) in self.tree.subtrees.iter() {
            if current_level != path.len() {
                current_level = path.len();
                idx_on_level = 0;

                level_subtree_block_size =
                    subtree_block_size(&self.levels.levels_info[current_level]);
                current_pos.y += level_subtree_block_size.1 + Y_MARGIN;

                level_subtree_width =
                    max_width / (self.levels.levels_info[current_level].n_subtrees + 1) as f32;
                current_pos.x = level_subtree_width;
            }

            self.draw_subtree(path, current_pos, level_subtree_width, subtree);

            current_pos.x += level_subtree_width;

            idx_on_level += 1;

            let root_in = subtree.get_subtree_input_point();
            let mut parent_path = path.clone();
            let key = parent_path.pop();
            let subtree_parent_out: Option<Pos2> = self
                .tree
                .get_subtree(&parent_path)
                .map(|s| key.map(|k| s.subtree().get_node_output(&k)))
                .flatten()
                .flatten();
            if let (Some(in_point), Some(out_point)) = (root_in, subtree_parent_out) {
                let layer_response = egui::Area::new(Id::new(("subtree_lines", path)))
                    .default_pos(Pos2::new(0.0, 0.0))
                    .order(egui::Order::Background)
                    .show(self.ui.ctx(), |ui| {
                        ui.set_clip_rect(self.transform.inverse() * self.rect);

                        let painter = ui.painter();
                        painter.line_segment(
                            [out_point, in_point],
                            Stroke {
                                width: 1.0,
                                color: Color32::GOLD,
                            },
                        );
                    })
                    .response;
                self.ui
                    .ctx()
                    .set_transform_layer(layer_response.layer_id, self.transform);
            }
        }

        let layer_response = egui::Area::new(Id::new("references"))
            .default_pos(Pos2::new(0.0, 0.0))
            .order(egui::Order::Background)
            .show(self.ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * self.rect);
                let painter = ui.painter();

                for (out_point, in_path, in_key) in self.references.into_iter() {
                    let Some(in_point) = self
                        .tree
                        .subtrees
                        .get(&in_path)
                        .map(|subtree| subtree.get_node_input(&in_key))
                        .flatten()
                    else {
                        continue;
                    };
                    painter.line_segment(
                        [out_point, in_point],
                        Stroke {
                            width: 1.0,
                            color: Color32::LIGHT_BLUE,
                        },
                    );
                }
            })
            .response;
        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }
}
