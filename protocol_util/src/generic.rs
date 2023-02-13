use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct Channel<T>(pub u64, pub PhantomData<T>);

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}