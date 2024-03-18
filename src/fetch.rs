use std::collections::VecDeque;

use grovedbg_grpc::{
    element::Element, grove_dbg_client::GroveDbgClient, AbsolutePathReference, CousinReference,
    FetchRequest, Item, RemovedCousinReference, SiblingReference, Subtree,
    UpstreamFromElementHeightReference, UpstreamRootHeightReference,
};

use crate::{trees, Key, Path};

pub(crate) async fn full_fetch(
    client: &mut GroveDbgClient<grovedbg_grpc::tonic::transport::Channel>,
) -> Result<trees::Tree, grovedbg_grpc::tonic::Status> {
    let root_subtree_root_node = client
        .fetch_node(FetchRequest {
            path: vec![],
            key: vec![],
        })
        .await?
        .into_inner();

    let mut tree = trees::Tree::new(root_subtree_root_node.key.clone());
    let mut queue = VecDeque::new();
    queue.push_back(root_subtree_root_node);

    while let Some(node) = queue.pop_front() {
        let value = match node.element {
            Some(grovedbg_grpc::Element {
                element: Some(Element::Item(Item { value })),
            }) => trees::InnerTreeNodeValue::Scalar(value),
            Some(grovedbg_grpc::Element {
                element: Some(Element::Subtree(Subtree { root_key })),
            }) => {
                if let Some(key) = &root_key {
                    let mut path = node.path.clone();
                    if !node.key.is_empty() {
                        path.push(node.key.clone());
                    }
                    queue.push_back(
                        client
                            .fetch_node(FetchRequest {
                                path,
                                key: key.clone(),
                            })
                            .await?
                            .into_inner(),
                    );
                }

                trees::InnerTreeNodeValue::Subtree(root_key)
            }
            Some(grovedbg_grpc::Element {
                element: Some(Element::AbsolutePathReference(AbsolutePathReference { mut path })),
            }) => {
                if let Some(key) = path.pop() {
                    trees::InnerTreeNodeValue::Reference(path, key)
                } else {
                    continue;
                }
            }
            Some(grovedbg_grpc::Element {
                element:
                    Some(Element::UpstreamRootHeightReference(UpstreamRootHeightReference {
                        n_keep,
                        path_append,
                    })),
            }) => {
                let mut path: Vec<_> = node
                    .path
                    .iter()
                    .cloned()
                    .take(n_keep as usize)
                    .chain(path_append.into_iter())
                    .collect();
                if let Some(key) = path.pop() {
                    trees::InnerTreeNodeValue::Reference(path, key)
                } else {
                    continue;
                }
            }
            Some(grovedbg_grpc::Element {
                element:
                    Some(Element::UpstreamFromElementHeightReference(
                        UpstreamFromElementHeightReference {
                            n_remove,
                            path_append,
                        },
                    )),
            }) => {
                let mut path_iter = node.path.iter();
                path_iter.nth_back(n_remove as usize);
                let mut path: Vec<_> = path_iter.cloned().chain(path_append.into_iter()).collect();
                if let Some(key) = path.pop() {
                    trees::InnerTreeNodeValue::Reference(path, key)
                } else {
                    continue;
                }
            }
            Some(grovedbg_grpc::Element {
                element: Some(Element::CousinReference(CousinReference { swap_parent })),
            }) => {
                let mut path = node.path.clone();
                if let Some(parent) = path.last_mut() {
                    *parent = swap_parent;
                    trees::InnerTreeNodeValue::Reference(path, node.key.clone())
                } else {
                    continue;
                }
            }
            Some(grovedbg_grpc::Element {
                element:
                    Some(Element::RemovedCousinReference(RemovedCousinReference { swap_parent })),
            }) => {
                let mut path = node.path.clone();
                if let Some(_) = path.pop() {
                    path.extend(swap_parent);
                    trees::InnerTreeNodeValue::Reference(path, node.key.clone())
                } else {
                    continue;
                }
            }
            Some(grovedbg_grpc::Element {
                element: Some(Element::SiblingReference(SiblingReference { sibling_key })),
            }) => trees::InnerTreeNodeValue::Reference(node.path.clone(), sibling_key),
            _ => {
                continue;
            }
        };

        if let Some(left) = &node.left_child {
            queue.push_back(
                client
                    .fetch_node(FetchRequest {
                        path: node.path.clone(),
                        key: left.clone(),
                    })
                    .await?
                    .into_inner(),
            );
        }

        if let Some(right) = &node.right_child {
            queue.push_back(
                client
                    .fetch_node(FetchRequest {
                        path: node.path.clone(),
                        key: right.clone(),
                    })
                    .await?
                    .into_inner(),
            );
        }

        tree.insert(
            node.path,
            node.key,
            trees::InnerTreeNode {
                value,
                left: node.left_child,
                right: node.right_child,
            },
        );
    }

    Ok(tree)
}
