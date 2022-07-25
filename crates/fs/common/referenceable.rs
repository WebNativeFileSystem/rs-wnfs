use anyhow::Result;
use async_once_cell::OnceCell;
use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::AsyncSerialize;

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

/// This is an abstract data structure that can be used to cache reference to some data and vice versa.
/// Basically it is allows some "reference" of type `R` to some addressable value of `T`.
///
/// It supports representing the data as its reference or the value itself.
///
/// This data structure is backed by a [ReferenceableStore](crate::ReferenceableStore) which is used to resolve the reference to the actual value.
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

/// This represents a store that can keep values serializable values and return some reference (of type `Ref`) to them.
///
/// References can be used to fetch the corresponding value from the store.
#[async_trait(?Send)]
pub trait ReferenceableStore {
    type Ref;

    async fn get_value<V: DeserializeOwned>(&self, reference: &Self::Ref) -> Result<V>;
    async fn put_value<V: AsyncSerialize<StoreRef = Self::Ref>>(
        &mut self,
        value: &V,
    ) -> Result<Self::Ref>;
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
    pub async fn get_owned_value<RS: ReferenceableStore<Ref = R>>(self, store: &RS) -> Result<V>
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

    /// Gets the reference data stored in type.
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
    pub async fn resolve_value<'a, RS: ReferenceableStore<Ref = R>>(
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

    /// Gets the reference data stored in type. It attempts to get it from the store if it is not present in type.
    pub async fn resolve_reference<'a, RS: ReferenceableStore<Ref = R> + ?Sized>(
        &'a self,
        store: &mut RS,
    ) -> Result<&'a R>
    where
        V: AsyncSerialize<StoreRef = R>,
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
