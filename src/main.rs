mod fetch;
mod model;
mod ui;

use eframe::{
    egui::{self, emath::TSTransform, Id},
    emath::Align2,
};
use grovedbg_grpc::grove_dbg_client::GroveDbgClient;
use strum::IntoEnumIterator;

use crate::{
    model::{Element, Node, Tree},
    ui::{node::element_to_color, Drawer},
};

fn get_tree() -> Tree {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let mut client = GroveDbgClient::connect("http://[::1]:10000").await.unwrap();
            fetch::full_fetch(&mut client).await.unwrap()
        })
}

fn main() -> eframe::Result<()> {
    let mut transform = TSTransform::default();

    let options = eframe::NativeOptions::default();

    let tree = get_tree();

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

            let drawer = Drawer::new(ui, transform, rect, &tree);
            drawer.draw_tree();

            egui::Area::new(Id::new("legend"))
                .anchor(Align2::RIGHT_TOP, [-20.0, 50.0])
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::default()
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .stroke(ui.ctx().style().visuals.window_stroke)
                        .fill(ui.style().visuals.panel_fill)
                        .show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            Element::iter().for_each(|element| {
                                ui.label(
                                    egui::RichText::new(element.as_ref())
                                        .color(element_to_color(&element)),
                                );
                            });
                        });
                });
        });
    })
}

fn example_tree() -> Tree {
    // Subtrees schema (no internal nodes shown):
    // root
    // ├── subtree1 (2 subtrees)
    // │   ├── subtree11 (empty)
    // │   └── subtree12 (1 subtree)
    // │       └── subtree121 (2 scalars)
    // ├── subtree2 (1 subtree)
    // │   └── subtree21 (1 reference)
    // └── subtree3 (2 subtrees and 1 scalar)
    //     ├── subtree31 (1 sumtree and 1 scalar)
    //     │   └── sumtree311 (3 sum items)
    //     └── subtree32 (empty)
    //
    // subtree12 and subtree2 contain only one subtree node each
    // subtree21 has 1 reference under key key211 pointed to [subtree1]:subtree12
    // (node) subtree32 is empty

    let mut tree = Tree::new();

    // Building the root subtree:
    // subtree2
    // ├── subtree3
    // └── subtree1
    tree.insert(
        vec![].into(),
        b"subtree2".to_vec(),
        Node::new_subtree(b"subtree21".to_vec().into())
            .with_left_child(b"subtree1".to_vec())
            .with_right_child(b"subtree3".to_vec()),
    );
    tree.insert(
        vec![].into(),
        b"subtree1".to_vec(),
        Node::new_subtree(b"subtree11".to_vec().into()),
    );
    tree.insert(
        vec![].into(),
        b"subtree3".to_vec(),
        Node::new_subtree(b"subtree31".to_vec().into()),
    );

    tree.set_root(b"subtree2".to_vec());

    // Building subtree1
    // subtree11
    // └── subtree12
    tree.insert(
        vec![b"subtree1".to_vec()].into(),
        b"subtree11".to_vec(),
        Node::new_subtree(None) // subtree11 key points to empty subtree
            .with_left_child(b"subtree12".to_vec()),
    );
    tree.insert(
        vec![b"subtree1".to_vec()].into(),
        b"subtree12".to_vec(),
        Node::new_subtree(b"subtree121".to_vec().into()),
    );

    // Building subtree12
    tree.insert(
        vec![b"subtree1".to_vec(), b"subtree12".to_vec()].into(),
        b"subtree121".to_vec(),
        Node::new_subtree(b"key1211".to_vec().into()),
    );

    // Building subtree121
    // key1211: value1211
    // └── key1212: value1212
    tree.insert(
        vec![
            b"subtree1".to_vec(),
            b"subtree12".to_vec(),
            b"subtree121".to_vec(),
        ]
        .into(),
        b"key1211".to_vec(),
        Node::new_item(b"value1211".to_vec()).with_left_child(b"key1212".to_vec()),
    );
    tree.insert(
        vec![
            b"subtree1".to_vec(),
            b"subtree12".to_vec(),
            b"subtree121".to_vec(),
        ]
        .into(),
        b"key1212".to_vec(),
        Node::new_item(b"value1212".to_vec()),
    );

    // Building subtree2
    tree.insert(
        vec![b"subtree2".to_vec()].into(),
        b"subtree21".to_vec(),
        Node::new_subtree(b"key211".to_vec().into()),
    );

    // Building subtree21
    tree.insert(
        vec![b"subtree2".to_vec(), b"subtree21".to_vec()].into(),
        b"key211".to_vec(),
        Node::new_reference(vec![b"subtree1".to_vec()].into(), b"subtree12".to_vec()),
    );

    // Building subtree3
    // subtree31
    // ├── subtree32
    // └── key31: value31
    tree.insert(
        vec![b"subtree3".to_vec()].into(),
        b"subtree31".to_vec(),
        Node::new_subtree(b"sumtree311".to_vec().into())
            .with_left_child(b"key31".to_vec())
            .with_right_child(b"subtree32".to_vec()),
    );
    tree.insert(
        vec![b"subtree3".to_vec()].into(),
        b"subtree32".to_vec(),
        Node::new_subtree(None),
    );
    tree.insert(
        vec![b"subtree3".to_vec()].into(),
        b"key31".to_vec(),
        Node::new_item(b"value31".to_vec()),
    );

    // Building subtree31
    // sumtree311
    // └── key312: value312
    tree.insert(
        vec![b"subtree3".to_vec(), b"subtree31".to_vec()].into(),
        b"sumtree311".to_vec(),
        Node::new_sumtree(b"key3111".to_vec().into(), 10).with_left_child(b"key312".to_vec()),
    );
    tree.insert(
        vec![b"subtree3".to_vec(), b"subtree31".to_vec()].into(),
        b"key312".to_vec(),
        Node::new_item(b"value312".to_vec()),
    );

    // Building sumtree311
    // key3111: 2
    // ├── key3113: 3
    // │   └── key31131: 0
    // └── key3112: 5
    tree.insert(
        vec![
            b"subtree3".to_vec(),
            b"subtree31".to_vec(),
            b"sumtree311".to_vec(),
        ]
        .into(),
        b"key3111".to_vec(),
        Node::new_sum_item(2)
            .with_left_child(b"key3112".to_vec())
            .with_right_child(b"key3113".to_vec()),
    );
    tree.insert(
        vec![
            b"subtree3".to_vec(),
            b"subtree31".to_vec(),
            b"sumtree311".to_vec(),
        ]
        .into(),
        b"key3112".to_vec(),
        Node::new_sum_item(5),
    );
    tree.insert(
        vec![
            b"subtree3".to_vec(),
            b"subtree31".to_vec(),
            b"sumtree311".to_vec(),
        ]
        .into(),
        b"key3113".to_vec(),
        Node::new_sum_item(3).with_right_child(b"key31131".to_vec()),
    );
    tree.insert(
        vec![
            b"subtree3".to_vec(),
            b"subtree31".to_vec(),
            b"sumtree311".to_vec(),
        ]
        .into(),
        b"key31131".to_vec(),
        Node::new_sum_item(0),
    );

    tree
}

fn example_tree_with_clusters() -> Tree {
    let mut example_tree = example_tree();
    example_tree.insert(
        vec![
            b"subtree1".to_vec(),
            b"subtree11".to_vec(),
            b"subtree111".to_vec(),
            b"subtree1111".to_vec(),
        ]
        .into(),
        b"out_of_nowhere3".to_vec(),
        Node::new_item(b"something".to_vec()),
    );
    example_tree.insert(
        vec![
            b"subtree1".to_vec(),
            b"subtree11".to_vec(),
            b"subtree111".to_vec(),
            b"subtree1111".to_vec(),
        ]
        .into(),
        b"out_of_nowhere".to_vec(),
        Node::new_item(b"something".to_vec()),
    );

    example_tree.insert(
        vec![
            b"subtree1".to_vec(),
            b"subtree11".to_vec(),
            b"subtree111".to_vec(),
        ]
        .into(),
        b"unconnected".to_vec(),
        Node::new_item(b"yes".to_vec()),
    );

    example_tree.insert(
        vec![
            b"subtree1".to_vec(),
            b"subtree11".to_vec(),
            b"subtree111".to_vec(),
        ]
        .into(),
        b"subtree1111".to_vec(),
        Node::new_subtree(b"out_of_nowhere".to_vec().into()),
    );

    example_tree
}
