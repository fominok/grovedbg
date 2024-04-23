mod proto_conversion;

use grovedbg_grpc::{grove_dbg_client::GroveDbgClient, FetchRequest};

use self::proto_conversion::BadProtoElement;
use crate::model::Tree;

pub(crate) type Client = GroveDbgClient<grovedbg_grpc::tonic::transport::Channel>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchError {
    #[error(transparent)]
    DataError(#[from] BadProtoElement),
    #[error("tonic fetch error: {0}")]
    TransportError(#[from] grovedbg_grpc::tonic::Status),
}

pub(crate) async fn fetch_root(client: &mut Client) -> Result<Tree, FetchError> {
    let root_subtree_root_node = client
        .fetch_node(FetchRequest {
            path: vec![],
            key: vec![],
        })
        .await?
        .into_inner();

    let mut tree = Tree::new();
    tree.set_root(root_subtree_root_node.key.clone());
    tree.insert(
        vec![].into(),
        root_subtree_root_node.key.clone(),
        root_subtree_root_node.try_into()?,
    );

    Ok(tree)
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
