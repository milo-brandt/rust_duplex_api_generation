use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct Channel<T>(pub u64, pub PhantomData<T>);
