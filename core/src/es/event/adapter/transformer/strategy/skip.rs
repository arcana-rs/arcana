//! [`Skip`] [`Strategy`] definition.

use futures::stream;

use crate::es::{event, event::adapter};

use super::Strategy;

/// [`Strategy`] for skipping [`Event`]s.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct Skip;

impl<Adapter, Event> Strategy<Adapter, Event> for Skip
where
    Event: event::VersionedOrRaw,
    Adapter: adapter::Returning,
    Adapter::Transformed: 'static,
    Adapter::Error: 'static,
{
    type Context = ();
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'o>
    where
        Adapter: 'o,
    = stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        _: &'me Adapter,
        _: Event,
        _: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out> {
        stream::empty()
    }
}
