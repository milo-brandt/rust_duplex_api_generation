use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub trait Switchable {
    type Pair: Switchable<Pair = Self>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct OutChannel<T>(pub u64, pub PhantomData<T>);
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct InChannel<T>(pub u64, pub PhantomData<T>);

impl<T> Switchable for OutChannel<T> {
    type Pair = InChannel<T>;
}
impl<T> Switchable for InChannel<T> {
    type Pair = OutChannel<T>;
}
