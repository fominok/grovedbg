mod fetch;
mod model;
#[cfg(test)]
mod test_utils;
mod ui;

use std::sync::{Arc, Mutex};

use eframe::egui::{self, emath::TSTransform};
use fetch::Message;
use tokio::sync::mpsc::{channel, Receiver};

use crate::{
    model::Tree,
    ui::{draw_legend, TreeDrawer},
};

fn start_message_processing(receiver: Receiver<Message>, tree: Arc<Mutex<Tree>>) {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(fetch::process_messages(receiver, tree))
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let mut transform = TSTransform::default();

    let options = eframe::NativeOptions::default();

    let tree = Arc::new(Mutex::new(Tree::new()));

    let (sender, receiver) = channel(10);
    sender.blocking_send(Message::FetchRoot).unwrap();

    let tree_arc = Arc::clone(&tree);
    std::thread::spawn(|| start_message_processing(receiver, tree_arc));

    eframe::run_simple_native("GroveDB Visualizer", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("GroveDB Visualizer");
            ui.separator();

            let (id, rect) = ui.allocate_space(ui.available_size());

            let response = ui.interact(rect, id, egui::Sense::click_and_drag());
            // Allow dragging the background as well.
            if response.dragged() {
                transform.translation += response.drag_delta();
            }

            // Plot-like reset
            if response.double_clicked() {
                transform = TSTransform::default();
            }

            let local_transform =
                TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) * transform;

            if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                // Note: doesn't catch zooming / panning if a button in this PanZoom container
                // is hovered.
                if response.hovered() {
                    let pointer_in_layer = local_transform.inverse() * pointer;
                    let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                    let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                    // Zoom in on pointer:
                    transform = transform
                        * TSTransform::from_translation(pointer_in_layer.to_vec2())
                        * TSTransform::from_scaling(zoom_delta)
                        * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                    // Pan:
                    transform = TSTransform::from_translation(pan_delta) * transform;
                }
            }

            {
                let lock = tree.lock().unwrap();
                let drawer = TreeDrawer::new(ui, transform, rect, &lock, &sender);
                drawer.draw_tree();
            }

            draw_legend(ui);
        });
    })
}
