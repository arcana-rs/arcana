use std::convert::Infallible;

use arcana::es::adapter::{self, strategy, Adapt, AnyContext};
use futures::stream;

use crate::event;

impl adapter::Returning for Adapter {
    type Error = Infallible;
    type Transformed = event::Message;
}

#[derive(Debug)]
pub struct Adapter;

impl Adapt<event::message::Posted> for Adapter {
    type Strategy = strategy::AsIs;
}

impl Adapt<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Custom;
}

impl Adapt<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::chat::v1::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::email::v2::AddedAndConfirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::email::Confirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::email::Added> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>>
    for Adapter
{
    type Strategy = strategy::Skip;
}

// Basically same as Skip
impl strategy::Customize<event::chat::public::Created> for Adapter {
    type Context = dyn AnyContext;
    type Error = Infallible;
    type Transformed = event::Message;
    type TransformedStream<'out> =
        stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        _event: event::chat::public::Created,
        _context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::empty()
    }
}
