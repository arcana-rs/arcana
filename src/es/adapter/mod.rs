//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use self::transformer::Transformer;

#[doc(inline)]
pub use arcana_core::es::adapter::{
    strategy, Adapt, Adapter, Returning, Strategy, TransformedStream, Wrapper,
};
