use anyhow::Error;
use std::ops::{Deref, DerefMut};
use std::convert::From;
use serde::{Serialize, Serializer};
use core::fmt::{Debug, Display};

#[derive(Debug)]
pub struct SerializedError(Error);

pub type Result<T, E = SerializedError> = core::result::Result<T, E>;

impl Serialize for SerializedError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

impl Deref for SerializedError {
    type Target = Error;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SerializedError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <M> From<M> for SerializedError where
M: Display + Debug + Send + Sync + 'static {
    fn from(item: M) -> Self {
        SerializedError(Error::msg(item))
    }
}
