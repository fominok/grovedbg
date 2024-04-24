//! Tree structure UI module

use eframe::{
    egui::{self, Id},
    emath::TSTransform,
    epaint::{Color32, Pos2, Rect, Stroke},
};
use tokio::sync::mpsc::Sender;

use super::{
    common::{binary_label_colored, path_label},
    node::{draw_element, draw_node, element_to_color},
};
use crate::{
    fetch::Message,
    model::{Element, Key, KeySlice, LevelInfo, LevelsInfo, NodeCtx, Path, SubtreeCtx, Tree},
};

const NODE_WIDTH: f32 = 200.0;
const NODE_HEIGHT: f32 = 30.0;
const X_MARGIN: f32 = 100.0;
const Y_MARGIN: f32 = 200.0;
const KV_PER_PAGE: usize = 10;

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
    sender: &'t Sender<Message>,
}

impl<'u, 't> TreeDrawer<'u, 't> {
    pub(crate) fn new(
        ui: &'u mut egui::Ui,
        transform: TSTransform,
        rect: Rect,
        tree: &'t Tree,
        sender: &'t Sender<Message>,
    ) -> Self {
        Self {
            ui,
            transform,
            rect,
            references: vec![],
            tree,
            levels: tree.levels(),
            sender,
        }
    }

    fn draw_node_area<'b>(
        &mut self,
        parent_coords: Option<Pos2>,
        coords: Pos2,
        node_ctx: NodeCtx<'b>,
    ) {
        let layer_response = egui::Area::new(Id::new(("area", node_ctx.egui_id())))
            .default_pos(coords)
            .order(egui::Order::Foreground)
            .show(self.ui.ctx(), |ui| {
                ui.set_clip_rect(self.transform.inverse() * self.rect);
                if let Some(out_coords) = parent_coords {
                    let painter = ui.painter();
                    painter.line_segment(
                        [out_coords, node_ctx.node().ui_state.borrow().input_point],
                        Stroke {
                            width: 1.0,
                            color: Color32::GRAY,
                        },
                    );
                }

                draw_node(ui, node_ctx);
            })
            .response;

        {
            let mut state = node_ctx.node().ui_state.borrow_mut();
            state.input_point = layer_response.rect.center_top();
            state.output_point = layer_response.rect.center_bottom();
            state.left_sibling_point = layer_response.rect.left_center();
            state.right_sibling_point = layer_response.rect.right_center();
        };
        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }

    fn draw_subtree_part<'a>(&mut self, mut coords: Pos2, node_ctx: NodeCtx<'a>) {
        let subtree_ctx = node_ctx.subtree_ctx();
        let mut current_level_nodes: Vec<(Option<KeySlice>, Option<KeySlice>)> = Vec::new();
        let mut next_level_nodes: Vec<(Option<KeySlice>, Option<KeySlice>)> = Vec::new();
        let mut level = 0;

        current_level_nodes.push((None, Some(node_ctx.key())));

        let x_base = coords.x;

        loop {
            if level > 0 {
                coords.x = x_base - 2f32.powi(level - 2) * (X_MARGIN + NODE_WIDTH);
            }

            for (parent_key, node_key) in current_level_nodes.drain(..) {
                if let Some(cur_node_ctx) = node_key.map(|k| subtree_ctx.get_node(&k)).flatten() {
                    let parent_out_coords = parent_key
                        .map(|k| subtree_ctx.subtree().get_node_output(&k))
                        .flatten();
                    self.draw_node_area(parent_out_coords, coords, cur_node_ctx);

                    let (node, _, key) = cur_node_ctx.split();

                    if let Element::Reference { path, key } = &node.element {
                        self.references.push((
                            cur_node_ctx.node().ui_state.borrow().output_point,
                            path.clone(),
                            key.clone(),
                        ));
                    }

                    next_level_nodes.push((Some(key), node.left_child.as_deref()));
                    next_level_nodes.push((Some(key), node.right_child.as_deref()));
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

    fn draw_subtree(&mut self, coords: Pos2, subtree_width: f32, subtree_ctx: SubtreeCtx) {
        if subtree_ctx.subtree().is_expanded() {
            self.draw_subtree_expanded(coords, subtree_width, subtree_ctx);
        } else {
            self.draw_subtree_collapsed(coords, subtree_ctx);
        }
    }

    fn draw_subtree_collapsed(&mut self, coords: Pos2, subtree_ctx: SubtreeCtx) {
        let subtree = subtree_ctx.subtree();

        let layer_response = egui::Area::new(subtree_ctx.egui_id())
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
                            if !subtree.is_empty() && menu.button("Expand").clicked() {
                                subtree.set_expanded();
                            }

                            if menu.button("Fetch all").clicked() {
                                if let Some(key) = &subtree.root_node {
                                    // TODO error handling
                                    self.sender.blocking_send(Message::FetchBranch {
                                        path: subtree_ctx.path().clone(),
                                        key: key.clone(),
                                    });
                                }
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
                            subtree_ctx.path(),
                            &mut subtree.path_display_variant_mut(),
                        );

                        ui.allocate_ui(
                            egui::Vec2 {
                                x: NODE_WIDTH,
                                y: 10.0,
                            },
                            |ui| ui.separator(),
                        );

                        for (key, node) in subtree
                            .nodes
                            .iter()
                            .skip(subtree.page_idx() * KV_PER_PAGE)
                            .take(KV_PER_PAGE)
                        {
                            if let Element::Reference {
                                path: ref_path,
                                key: ref_key,
                            } = &node.element
                            {
                                if subtree_ctx.path() != ref_path {
                                    self.references.push((
                                        subtree.get_subtree_output_point(),
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

                        if subtree.nodes.len() > KV_PER_PAGE {
                            ui.horizontal(|pagination| {
                                if pagination
                                    .add_enabled(subtree.page_idx() > 0, egui::Button::new("⬅"))
                                    .clicked()
                                {
                                    subtree.prev_page();
                                }
                                if pagination
                                    .add_enabled(
                                        (subtree.page_idx() + 1) * KV_PER_PAGE < subtree.n_nodes(),
                                        egui::Button::new("➡"),
                                    )
                                    .clicked()
                                {
                                    subtree.next_page();
                                }
                            });
                        }
                    });
            })
            .response;

        subtree.set_input_point(layer_response.rect.center_top());
        subtree.set_output_point(layer_response.rect.center_bottom());

        self.ui
            .ctx()
            .set_transform_layer(layer_response.layer_id, self.transform);
    }

    fn draw_subtree_expanded(
        &mut self,
        mut coords: Pos2,
        subtree_width: f32,
        subtree_ctx: SubtreeCtx,
    ) {
        let subtree = subtree_ctx.subtree();

        let cluster_roots_iter = subtree_ctx.iter_cluster_roots();

        let width_step = subtree_width
            / (cluster_roots_iter.len() + subtree.root_node.as_ref().map(|_| 1).unwrap_or(0))
                as f32;
        let mut prev_point = None;

        subtree_ctx
            .get_root()
            .into_iter()
            .chain(cluster_roots_iter)
            .for_each(|node_ctx| {
                self.draw_subtree_part(coords, node_ctx);
                coords.x += width_step;

                let node = node_ctx.node();

                let state = node.ui_state.borrow();

                if let Some(right_point) = prev_point {
                    let layer_response = egui::Area::new(Id::new(("clusters", node_ctx.egui_id())))
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
            });
    }

    pub(crate) fn draw_tree(mut self) {
        if self.levels.levels_info.is_empty() {
            return;
        }
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

        for subtree_ctx in self.tree.iter_subtrees() {
            if current_level != subtree_ctx.path().len() {
                current_level = subtree_ctx.path().len();
                idx_on_level = 0;

                level_subtree_block_size =
                    subtree_block_size(&self.levels.levels_info[current_level]);
                current_pos.y += level_subtree_block_size.1 + Y_MARGIN;

                level_subtree_width =
                    max_width / (self.levels.levels_info[current_level].n_subtrees + 1) as f32;
                current_pos.x = level_subtree_width;
            }

            self.draw_subtree(current_pos, level_subtree_width, subtree_ctx);

            current_pos.x += level_subtree_width;

            idx_on_level += 1;

            let root_in = subtree_ctx.subtree().get_subtree_input_point();
            let mut parent_path = subtree_ctx.path().clone();
            let key = parent_path.pop();
            let subtree_parent_out: Option<Pos2> = self
                .tree
                .get_subtree(&parent_path)
                .map(|s| key.map(|k| s.subtree().get_node_output(&k)))
                .flatten()
                .flatten();
            if let (Some(in_point), Some(out_point)) = (root_in, subtree_parent_out) {
                let layer_response =
                    egui::Area::new(Id::new(("subtree_lines", subtree_ctx.path())))
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
