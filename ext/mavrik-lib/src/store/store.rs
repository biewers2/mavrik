use crate::store::store_state::StoreState;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;

/// A store that can have new entries pushed to it.
pub trait PushStore {
    type Id;
    type Error;

    /// Push a new entry to the store.
    ///
    /// # Returns
    ///
    /// The ID of the entry that was pushed.
    ///
    fn push<S, V>(
        &self,
        queue: S,
        value: V,
    ) -> impl Future<Output = Result<Self::Id, Self::Error>> + Send
    where
        S: AsRef<str> + Send,
        V: Serialize + Send;
}

/// A store that can have entries pulled from it.
pub trait PullStore {
    type Id: Serialize;
    type Error;

    /// Pull an entry from the store.
    ///
    /// # Returns
    ///
    /// The entry that was pulled.
    ///
    fn pull<D>(&self, id: Self::Id) -> impl Future<Output = Result<D, Self::Error>> + Send
    where
        D: DeserializeOwned;
}

/// A store that can have entries pulled for processing and then the results published.
pub trait ProcessStore {
    type Id;
    type Error;

    /// Pull the next entry from the store.
    ///
    /// # Returns
    ///
    /// A tuple containing the ID of the entry and the entry itself.
    ///
    fn dequeue<D>(&self) -> impl Future<Output = Result<(Self::Id, D), Self::Error>> + Send
    where
        D: DeserializeOwned;

    /// Publish the result of processing an entry.
    ///
    /// # Arguments
    ///
    /// `id` - The ID of the entry that was processed.
    /// `output` - The result of processing the entry.
    ///
    fn publish_result<S>(
        &self,
        id: Self::Id,
        output: S,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send
    where
        S: Serialize + Send;
}

/// A store that can be inspected and managed from an external actor.
///
/// This is useful for inspecting the state of the store, removing entries that should no longer be processed,
/// and so on.
pub trait QueryStore {
    type Error;

    /// Get the state of the store.
    ///
    /// # Returns
    ///
    /// The state of the store.
    ///
    fn state(&self) -> impl Future<Output = Result<StoreState, Self::Error>> + Send;
}
