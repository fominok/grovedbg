use eframe::{
    egui,
    epaint::{Color32, Stroke},
};

use super::common::{binary_label, bytes_by_display_variant, path_label};
use crate::model::{Element, Node, NodeCtx};

pub(crate) fn draw_node<'a>(ui: &mut egui::Ui, node_ctx: NodeCtx<'a>) {
    let (node, _, key) = node_ctx.split();

    let mut stroke = Stroke::default();
    stroke.color = element_to_color(&node.element);
    stroke.width = 1.0;

    let response = egui::Frame::default()
        .rounding(egui::Rounding::same(4.0))
        .inner_margin(egui::Margin::same(8.0))
        .stroke(stroke)
        .fill(Color32::BLACK)
        .show(ui, |ui| {
            ui.style_mut().wrap = Some(false);
            binary_label(ui, key, &mut node.ui_state.borrow_mut().key_display_variant);
            draw_element(ui, &node);
        })
        .response;

    response.context_menu(|menu| {
        if menu.button("Collapse").clicked() {
            node_ctx.subtree().set_collapsed();
        }
    });
}

pub(crate) fn draw_element(ui: &mut egui::Ui, node: &Node) {
    match &node.element {
        Element::Item { value } => {
            binary_label(
                ui,
                value,
                &mut node.ui_state.borrow_mut().item_display_variant,
            );
        }
        Element::SumItem { value } => {
            ui.label(format!("Value: {value}"));
        }
        Element::Reference { path, key } => {
            path_label(
                ui,
                path,
                &mut node.ui_state.borrow_mut().item_display_variant,
            );
            ui.horizontal(|line| {
                line.add_space(20.0);
                line.label(bytes_by_display_variant(
                    key,
                    &mut node.ui_state.borrow_mut().item_display_variant,
                ));
            });
        }
        Element::Sumtree { sum, .. } => {
            ui.label(format!("Sum: {sum}"));
        }
        Element::Subtree { .. } => {
            ui.label("Subtree");
        }
        Element::SubtreePlaceholder => {
            ui.label("Subtree");
        }
    }
}

pub(crate) fn element_to_color(element: &Element) -> Color32 {
    match element {
        Element::Item { .. } => Color32::WHITE,
        Element::SumItem { .. } => Color32::DARK_GREEN,
        Element::Reference { .. } => Color32::LIGHT_BLUE,
        Element::Subtree { .. } => Color32::GOLD,
        Element::SubtreePlaceholder => Color32::RED,
        Element::Sumtree { .. } => Color32::GREEN,
    }
}
