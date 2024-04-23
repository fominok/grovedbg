mod proto_conversion;

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use grovedbg_grpc::{grove_dbg_client::GroveDbgClient, FetchRequest};
use tokio::sync::mpsc::Receiver;

use self::proto_conversion::BadProtoElement;
use crate::model::{Key, Node, Path, Tree};

pub(crate) enum Message {
    FetchRoot,
    FetchNode { path: Path, key: Key },
    FetchBranch { path: Path, key: Key },
}

pub(crate) type Client = GroveDbgClient<grovedbg_grpc::tonic::transport::Channel>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchError {
    #[error(transparent)]
    DataError(#[from] BadProtoElement),
    #[error("tonic fetch error: {0}")]
    TransportError(#[from] grovedbg_grpc::tonic::Status),
}

pub(crate) async fn process_messages(mut receiver: Receiver<Message>, tree: Arc<Mutex<Tree>>) {
    // TODO error handling
    let mut client = GroveDbgClient::connect("http://[::1]:10000").await.unwrap();

    while let Some(message) = receiver.recv().await {
        match message {
            Message::FetchRoot => {
                let root_node = client
                    .fetch_node(FetchRequest {
                        path: vec![],
                        key: vec![],
                    })
                    .await
                    .unwrap()
                    .into_inner();

                let mut lock = tree.lock().unwrap();
                lock.set_root(root_node.key.clone());
                lock.insert(
                    vec![].into(),
                    root_node.key.clone(),
                    root_node.try_into().unwrap(),
                );
            }
            Message::FetchNode { path, key } => {
                let node = client
                    .fetch_node(FetchRequest {
                        path: path.to_vec(),
                        key: key.clone(),
                    })
                    .await
                    .unwrap()
                    .into_inner()
                    .try_into()
                    .unwrap();
                let mut lock = tree.lock().unwrap();
                lock.insert(path, key, node);
            }
            Message::FetchBranch { path, key } => {
                let mut queue = VecDeque::new();
                queue.push_back(key.clone());

                let mut to_insert = Vec::new();

                while let Some(node_key) = queue.pop_front() {
                    let node: Node = client
                        .fetch_node(FetchRequest {
                            path: path.to_vec(),
                            key: node_key.clone(),
                        })
                        .await
                        .unwrap()
                        .into_inner()
                        .try_into()
                        .unwrap();

                    if let Some(left) = &node.left_child {
                        queue.push_back(left.clone());
                    }

                    if let Some(right) = &node.right_child {
                        queue.push_back(right.clone());
                    }

                    to_insert.push((node_key, node));
                }

                let mut lock = tree.lock().unwrap();
                to_insert
                    .into_iter()
                    .for_each(|(key, node)| lock.insert(path.clone(), key, node));
            }
        }
    }
}

pub(crate) async fn fetch_root(tree: &mut Tree, client: &mut Client) -> Result<(), FetchError> {
    let root_subtree_root_node = client
        .fetch_node(FetchRequest {
            path: vec![],
            key: vec![],
        })
        .await?
        .into_inner();

    tree.set_root(root_subtree_root_node.key.clone());
    tree.insert(
        vec![].into(),
        root_subtree_root_node.key.clone(),
        root_subtree_root_node.try_into()?,
    );

    Ok(())
}

// pub(crate) async fn full_fetch(client: &mut Client) -> Result<Tree,
// FetchError> {     let root_subtree_root_node = client
//         .fetch_node(FetchRequest {
//             path: vec![],
//             key: vec![],
//         })
//         .await?
//         .into_inner();

//     let mut tree = Tree::new();
//     tree.set_root(root_subtree_root_node.key.clone());
//     let mut queue = VecDeque::new();
//     queue.push_back(root_subtree_root_node);

//     while let Some(node) = queue.pop_front() {
// let element: Element = match node.element {
//     Some(grovedbg_grpc::Element {
//         element: Some(grovedbg_grpc::element::Element::Item(Item { value })),
//     }) => Element::Item { value },
//     Some(grovedbg_grpc::Element {
//         element: Some(grovedbg_grpc::element::Element::Subtree(Subtree {
// root_key })),     }) => {
//         if let Some(key) = &root_key {
//             let mut path = node.path.clone();
//             if !node.key.is_empty() {
//                 path.push(node.key.clone());
//             }
//             queue.push_back(
//                 client
//                     .fetch_node(FetchRequest {
//                         path,
//                         key: key.clone(),
//                     })
//                     .await?
//                     .into_inner(),
//             );
//         }
//         Element::Subtree { root_key }
//     }
//     Some(grovedbg_grpc::Element {
//         element:
//             Some(grovedbg_grpc::element::Element::AbsolutePathReference(
//                 AbsolutePathReference { mut path },
//             )),
//     }) => {
//         if let Some(key) = path.pop() {
//             Element::Reference {
//                 path: path.into(),
//                 key,
//             }
//         } else {
//             continue;
//         }
//     }
//     Some(grovedbg_grpc::Element {
//         element:
//
// Some(grovedbg_grpc::element::Element::UpstreamRootHeightReference(
//                 UpstreamRootHeightReference {
//                     n_keep,
//                     path_append,
//                 },
//             )),
//     }) => {
//         let mut path: Vec<_> = node
//             .path
//             .iter()
//             .cloned()
//             .take(n_keep as usize)
//             .chain(path_append.into_iter())
//             .collect();
//         if let Some(key) = path.pop() {
//             Element::Reference {
//                 path: path.into(),
//                 key,
//             }
//         } else {
//             continue;
//         }
//     }
//     Some(grovedbg_grpc::Element {
//         element:
//
// Some(grovedbg_grpc::element::Element::UpstreamFromElementHeightReference(
//                 UpstreamFromElementHeightReference {
//                     n_remove,
//                     path_append,
//                 },
//             )),
//     }) => {
//         let mut path_iter = node.path.iter();
//         path_iter.nth_back(n_remove as usize);
//         let mut path: Vec<_> =
// path_iter.cloned().chain(path_append.into_iter()).collect();         if let
// Some(key) = path.pop() {             Element::Reference {
//                 path: path.into(),
//                 key,
//             }
//         } else {
//             continue;
//         }
//     }
//     Some(grovedbg_grpc::Element {
//         element:
//
// Some(grovedbg_grpc::element::Element::CousinReference(CousinReference {
//                 swap_parent,
//             })),
//     }) => {
//         let mut path = node.path.clone();
//         if let Some(parent) = path.last_mut() {
//             *parent = swap_parent;
//             Element::Reference {
//                 path: path.into(),
//                 key: node.key.clone(),
//             }
//         } else {
//             continue;
//         }
//     }
//     Some(grovedbg_grpc::Element {
//         element:
//             Some(grovedbg_grpc::element::Element::RemovedCousinReference(
//                 RemovedCousinReference { swap_parent },
//             )),
//     }) => {
//         let mut path = node.path.clone();
//         if let Some(_) = path.pop() {
//             path.extend(swap_parent);
//             Element::Reference {
//                 path: path.into(),
//                 key: node.key.clone(),
//             }
//         } else {
//             continue;
//         }
//     }
//     Some(grovedbg_grpc::Element {
//         element:
//
// Some(grovedbg_grpc::element::Element::SiblingReference(SiblingReference {
//                 sibling_key,
//             })),
//     }) => Element::Reference {
//         path: node.path.clone().into(),
//         key: sibling_key,
//     },
//     _ => {
//         continue;
//     }
// };

//         if let Some(left) = &node.left_child {
//             queue.push_back(
//                 client
//                     .fetch_node(FetchRequest {
//                         path: node.path.clone(),
//                         key: left.clone(),
//                     })
//                     .await?
//                     .into_inner(),
//             );
//         }

//         if let Some(right) = &node.right_child {
//             queue.push_back(
//                 client
//                     .fetch_node(FetchRequest {
//                         path: node.path.clone(),
//                         key: right.clone(),
//                     })
//                     .await?
//                     .into_inner(),
//             );
//         }

//         tree.insert(
//             node.path.clone().into(),
//             node.key.clone(),
//             node.try_into()?,
//             // Node {
//             //     element,
//             //     left_child: node.left_child,
//             //     right_child: node.right_child,
//             //     ui_state: Default::default(),
//             // },
//         );
//     }

//     Ok(tree)
// }
