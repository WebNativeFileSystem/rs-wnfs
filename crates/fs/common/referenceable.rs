use anyhow::Result;
use async_once_cell::OnceCell;
use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::AsyncSerialize;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub enum Referenceable<R, V> {
    /// A variant of `Resolvable` that starts out as a value of R.
    /// It supports converting a reference to a value of `V` by caching it only once in `value_cache`.
    Encoded {
        reference: R,
        value_cache: OnceCell<V>,
    },
    /// A variant of `Resolvable` that starts out as a value of `V.`.
    /// It supports converting the value of `V` to a reference by caching it only once in `reference_cache`.
    Decoded {
        value: V,
        reference_cache: OnceCell<R>,
    },
}

#[async_trait(?Send)]
pub trait ReferenceableStore<V: ?Sized> {
    type Reference;

    async fn get_value(&self, reference: &Self::Reference) -> Result<V>
    where
        V: DeserializeOwned;

    async fn put_value(&mut self, value: &V) -> Result<Self::Reference>
    where
        V: AsyncSerialize;
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl<R, V> Referenceable<R, V> {
    /// Creates a new `Referenceable` that starts out as a value of `R`.
    pub fn from_reference(reference: R) -> Self {
        Self::Encoded {
            reference,
            value_cache: OnceCell::new(),
        }
    }

    /// Gets an owned value from type. It attempts to it get from the store if it is not present in type.
    pub async fn get_owned_value<RS: ReferenceableStore<V, Reference = R>>(
        self,
        store: &RS,
    ) -> Result<V>
    where
        V: DeserializeOwned,
    {
        match self {
            Self::Encoded {
                ref reference,
                value_cache,
            } => match value_cache.into_inner() {
                Some(cached) => Ok(cached),
                None => store.get_value(reference).await,
            },
            Self::Decoded { value, .. } => Ok(value),
        }
    }

    /// Gets the value stored in type.
    ///
    /// NOTE: This does not attempt to get it from the store if it does not exist.
    pub fn get_value(&self) -> Option<&V> {
        match self {
            Self::Encoded { value_cache, .. } => value_cache.get(),
            Self::Decoded { value, .. } => Some(value),
        }
    }

    /// Gets the reference stored in type.
    ///
    /// NOTE: This does not attempt to get it from the store if it does not exist.
    pub fn get_reference(&self) -> Option<&R> {
        match self {
            Self::Encoded { reference, .. } => Some(reference),
            Self::Decoded {
                reference_cache, ..
            } => reference_cache.get(),
        }
    }

    /// Gets the value stored in link. It attempts to get it from the store if it is not present in link.
    pub async fn resolve_value<'a, RS: ReferenceableStore<V, Reference = R>>(
        &'a self,
        store: &RS,
    ) -> Result<&'a V>
    where
        V: DeserializeOwned,
    {
        match self {
            Self::Encoded {
                reference,
                value_cache,
            } => {
                value_cache
                    .get_or_try_init(async { store.get_value(reference).await })
                    .await
            }
            Self::Decoded { value, .. } => Ok(value),
        }
    }

    /// Gets the reference stored in type. It attempts to get it from the store if it is not present in type.
    pub async fn resolve_reference<'a, RS: ReferenceableStore<V, Reference = R> + ?Sized>(
        &'a self,
        store: &mut RS,
    ) -> Result<&'a R>
    where
        V: AsyncSerialize,
    {
        match self {
            Self::Encoded { reference, .. } => Ok(reference),
            Self::Decoded {
                value,
                reference_cache,
            } => {
                reference_cache
                    .get_or_try_init(async { store.put_value(value).await })
                    .await
            }
        }
    }

    /// Checks if there is a value stored in link.
    pub fn has_value(&self) -> bool {
        match self {
            Self::Encoded { value_cache, .. } => value_cache.get().is_some(),
            _ => true,
        }
    }

    /// Checks if there is a Cid stored in link.
    pub fn has_reference(&self) -> bool {
        match self {
            Self::Decoded {
                reference_cache, ..
            } => reference_cache.get().is_some(),
            _ => true,
        }
    }
}

impl<R, V> From<V> for Referenceable<R, V> {
    fn from(value: V) -> Self {
        Self::Decoded {
            value,
            reference_cache: OnceCell::new(),
        }
    }
}

impl<R, V> Clone for Referenceable<R, V>
where
    V: Clone,
    R: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Encoded {
                reference,
                value_cache,
            } => Self::Encoded {
                reference: reference.clone(),
                value_cache: OnceCell::new_with(value_cache.get().cloned()),
            },
            Self::Decoded {
                value,
                reference_cache,
            } => Self::Decoded {
                value: value.clone(),
                reference_cache: OnceCell::new_with(reference_cache.get().cloned()),
            },
        }
    }
}

impl<R, V> PartialEq for Referenceable<R, V>
where
    R: PartialEq,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Encoded { reference, .. },
                Self::Encoded {
                    reference: reference2,
                    ..
                },
            ) => reference == reference2,
            (Self::Decoded { value, .. }, Self::Decoded { value: value2, .. }) => value == value2,
            (Self::Encoded { reference, .. }, Self::Decoded { value: value2, .. }) => {
                if let Some(reference2) = other.get_reference() {
                    reference == reference2
                } else if let Some(value) = self.get_value() {
                    value == value2
                } else {
                    false
                }
            }
            (
                Self::Decoded { value, .. },
                Self::Encoded {
                    reference: reference2,
                    ..
                },
            ) => {
                if let Some(reference) = self.get_reference() {
                    reference == reference2
                } else if let Some(value2) = other.get_value() {
                    value == value2
                } else {
                    false
                }
            }
        }
    }
}
