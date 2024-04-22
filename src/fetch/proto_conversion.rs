//! Conversion definitions from received proto object to model.
use std::borrow::Cow;

use grovedbg_grpc::{
    grove_dbg_client::GroveDbgClient, AbsolutePathReference, CousinReference, FetchRequest, Item,
    RemovedCousinReference, SiblingReference, Subtree, SumItem, Sumtree,
    UpstreamFromElementHeightReference, UpstreamRootHeightReference,
};

use crate::model::{Element, Node, Tree};

#[derive(Debug, thiserror::Error)]
#[error("Computed reference has no key")]
pub(crate) struct ReferenceWithoutKey;

impl TryFrom<AbsolutePathReference> for Element {
    type Error = ReferenceWithoutKey;

    fn try_from(
        AbsolutePathReference { mut path }: AbsolutePathReference,
    ) -> Result<Self, Self::Error> {
        if let Some(key) = path.pop() {
            Ok(Element::Reference {
                path: path.into(),
                key,
            })
        } else {
            Err(ReferenceWithoutKey)
        }
    }
}

struct PathCtx<'a, T> {
    path: Cow<'a, [Vec<u8>]>,
    value: T,
}

trait PathCtxExt: Sized {
    fn with_current_path<'a>(self, path: impl Into<Cow<'a, [Vec<u8>]>>) -> PathCtx<'a, Self>;

    fn with_current_path_key<'a>(
        self,
        path: impl Into<Cow<'a, [Vec<u8>]>>,
        key: impl Into<Cow<'a, [u8]>>,
    ) -> PathKeyCtx<'a, Self>;
}

impl<T> PathCtxExt for T {
    fn with_current_path<'a>(self, path: impl Into<Cow<'a, [Vec<u8>]>>) -> PathCtx<'a, Self> {
        PathCtx {
            path: path.into(),
            value: self,
        }
    }

    fn with_current_path_key<'a>(
        self,
        path: impl Into<Cow<'a, [Vec<u8>]>>,
        key: impl Into<Cow<'a, [u8]>>,
    ) -> PathKeyCtx<'a, Self> {
        PathKeyCtx {
            path: path.into(),
            key: key.into(),
            value: self,
        }
    }
}

impl TryFrom<PathCtx<'_, UpstreamRootHeightReference>> for Element {
    type Error = ReferenceWithoutKey;

    fn try_from(
        PathCtx {
            path,
            value:
                UpstreamRootHeightReference {
                    n_keep,
                    path_append,
                },
        }: PathCtx<UpstreamRootHeightReference>,
    ) -> Result<Self, Self::Error> {
        let mut path: Vec<_> = path
            .iter()
            .cloned()
            .take(n_keep as usize)
            .chain(path_append.into_iter())
            .collect();
        if let Some(key) = path.pop() {
            Ok(Element::Reference {
                path: path.into(),
                key,
            })
        } else {
            Err(ReferenceWithoutKey)
        }
    }
}

impl TryFrom<PathCtx<'_, UpstreamFromElementHeightReference>> for Element {
    type Error = ReferenceWithoutKey;

    fn try_from(
        PathCtx {
            path,
            value:
                UpstreamFromElementHeightReference {
                    n_remove,
                    path_append,
                },
        }: PathCtx<UpstreamFromElementHeightReference>,
    ) -> Result<Self, Self::Error> {
        let mut path_iter = path.iter();
        path_iter.nth_back(n_remove as usize);
        let mut path: Vec<_> = path_iter.cloned().chain(path_append.into_iter()).collect();
        if let Some(key) = path.pop() {
            Ok(Element::Reference {
                path: path.into(),
                key,
            })
        } else {
            Err(ReferenceWithoutKey)
        }
    }
}

struct PathKeyCtx<'a, T> {
    path: Cow<'a, [Vec<u8>]>,
    key: Cow<'a, [u8]>,
    value: T,
}

impl TryFrom<PathKeyCtx<'_, CousinReference>> for Element {
    type Error = ReferenceWithoutKey;

    fn try_from(
        PathKeyCtx {
            path,
            key,
            value: CousinReference { swap_parent },
        }: PathKeyCtx<CousinReference>,
    ) -> Result<Self, Self::Error> {
        let mut path = path.into_owned();
        if let Some(parent) = path.last_mut() {
            *parent = swap_parent;
            Ok(Element::Reference {
                path: path.into(),
                key: key.into_owned(),
            })
        } else {
            Err(ReferenceWithoutKey)
        }
    }
}

impl TryFrom<PathKeyCtx<'_, RemovedCousinReference>> for Element {
    type Error = ReferenceWithoutKey;

    fn try_from(
        PathKeyCtx {
            path,
            key,
            value: RemovedCousinReference { swap_parent },
        }: PathKeyCtx<RemovedCousinReference>,
    ) -> Result<Self, Self::Error> {
        let mut path = path.into_owned();
        if let Some(_) = path.pop() {
            path.extend(swap_parent);
            Ok(Element::Reference {
                path: path.into(),
                key: key.into_owned(),
            })
        } else {
            Err(ReferenceWithoutKey)
        }
    }
}

impl From<PathCtx<'_, SiblingReference>> for Element {
    fn from(
        PathCtx {
            path,
            value: SiblingReference { sibling_key },
        }: PathCtx<SiblingReference>,
    ) -> Self {
        Element::Reference {
            path: path.into_owned().into(),
            key: sibling_key,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BadProtoElement {
    #[error(transparent)]
    EmptyPathReference(#[from] ReferenceWithoutKey),
    #[error("Proto Element is None")]
    NoneElement,
}

impl TryFrom<PathKeyCtx<'_, grovedbg_grpc::Element>> for Element {
    type Error = BadProtoElement;

    fn try_from(
        PathKeyCtx {
            path,
            key,
            value: grovedbg_grpc::Element { element },
        }: PathKeyCtx<grovedbg_grpc::Element>,
    ) -> Result<Self, Self::Error> {
        Ok(match element.ok_or(BadProtoElement::NoneElement)? {
            grovedbg_grpc::element::Element::Item(Item { value }) => Element::Item { value },
            grovedbg_grpc::element::Element::Subtree(Subtree { root_key }) => {
                Element::Subtree { root_key }
            }
            grovedbg_grpc::element::Element::SumItem(SumItem { value }) => {
                Element::SumItem { value }
            }
            grovedbg_grpc::element::Element::Sumtree(Sumtree { root_key, sum }) => {
                Element::Sumtree { root_key, sum }
            }
            grovedbg_grpc::element::Element::AbsolutePathReference(reference) => {
                reference.try_into()?
            }
            grovedbg_grpc::element::Element::UpstreamRootHeightReference(reference) => {
                reference.with_current_path(path).try_into()?
            }
            grovedbg_grpc::element::Element::UpstreamFromElementHeightReference(reference) => {
                reference.with_current_path(path).try_into()?
            }
            grovedbg_grpc::element::Element::CousinReference(reference) => {
                reference.with_current_path_key(path, key).try_into()?
            }
            grovedbg_grpc::element::Element::RemovedCousinReference(reference) => {
                reference.with_current_path_key(path, key).try_into()?
            }
            grovedbg_grpc::element::Element::SiblingReference(reference) => {
                reference.with_current_path(path).into()
            }
        })
    }
}

impl TryFrom<grovedbg_grpc::NodeUpdate> for Node {
    type Error = BadProtoElement;

    fn try_from(value: grovedbg_grpc::NodeUpdate) -> Result<Self, Self::Error> {
        Ok(Node {
            element: value
                .element
                .ok_or(BadProtoElement::NoneElement)?
                .with_current_path_key(value.path, value.key)
                .try_into()?,
            left_child: value.left_child,
            right_child: value.right_child,
            ..Default::default()
        })
    }
}
