use std::collections::VecDeque;

use grovedbg_grpc::{
    element::Element, grove_dbg_client::GroveDbgClient, FetchRequest, Item, Subtree,
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
