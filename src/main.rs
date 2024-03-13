mod fetch;
mod trees;
mod ui;

use std::sync::{Arc, Mutex};

use eframe::egui;
use egui_snarl::{ui::SnarlStyle, Snarl};
use grovedbg_grpc::grove_dbg_client::GroveDbgClient;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{unbounded_channel, UnboundedReceiver},
};

use crate::ui::trees::{draw_subtrees, SnarlSubtreeNode, Viewer};

type Key = Vec<u8>;
type Path = Vec<Vec<u8>>;

enum Command {
    FetchAll,
}

fn start_messaging(mut channel: UnboundedReceiver<Command>, tree: Arc<Mutex<trees::Tree>>) {
    let spawn = tokio::spawn(async move {
        let mut client = GroveDbgClient::connect("http://[::1]:10000").await.unwrap();
        while let Some(cmd) = channel.recv().await {
            match cmd {
                Command::FetchAll => {
                    // TODO: error handling
                    let new_tree = fetch::full_fetch(&mut client).await.unwrap();
                    *tree.lock().unwrap() = new_tree;
                }
            }
        }
    });
}

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let rt = Runtime::new().unwrap();
    let _guard = rt.enter();

    let (sender, receiver) = unbounded_channel();
    sender.send(Command::FetchAll).unwrap();

    let tree = Arc::new(Mutex::new(trees::Tree::new(b"".to_vec())));

    start_messaging(receiver, Arc::clone(&tree));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "GroveDBG",
        options,
        Box::new(|_| Box::new(Application::new(tree))),
    )
}

struct Application {
    tree: Arc<Mutex<trees::Tree>>,
    snarl: Snarl<SnarlSubtreeNode>,
}

impl Application {
    fn new(tree: Arc<Mutex<trees::Tree>>) -> Self {
        Application {
            tree,
            snarl: Snarl::new(),
        }
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut lock = self.tree.lock().unwrap();
            if lock.updated {
                draw_subtrees(&mut self.snarl, &lock);
                lock.updated = false;
            }

            self.snarl.show(
                &mut Viewer,
                &SnarlStyle::default(),
                egui::Id::new("snarl"),
                ui,
            );
        });
    }
}
