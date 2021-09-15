//! [`Adapter`] definitions.

pub mod transformer;

use std::{
    fmt::{Debug, Formatter},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future, stream, Stream, StreamExt as _};
use pin_project::pin_project;
use ref_cast::RefCast;

#[doc(inline)]
pub use self::transformer::Transformer;

/// TODO
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Wrapper<A>(pub A);

impl<A: WithError> WithError for Wrapper<A> {
    type Context = A::Context;
    type Error = A::Error;
    type Transformed = A::Transformed;
}

/// TODO
pub trait WithError {
    /// TODO
    type Context: ?Sized;

    /// TODO
    type Error;

    /// TODO
    type Transformed;
}

/// Facility to convert [`Event`]s.
/// Typical use cases include (but are not limited to):
///
/// - [`Skip`]ping unused [`Event`]s;
/// - Transforming (ex: from one [`Version`] to another);
/// - [`Split`]ting existing [`Event`]s into more granular ones.
///
/// Provided with blanket impl for [`Transformer`] implementors, so usually you
/// shouldn't implement it manually.
///
/// [`Event`]: crate::es::Event
/// [`Skip`]: transformer::strategy::Skip
/// [`Split`]: transformer::strategy::Split
/// [`Version`]: crate::es::event::Version
pub trait Adapter<Events> {
    /// Context for converting [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    type Context: ?Sized;

    /// Error of this [`Adapter`].
    type Error;

    /// Converted [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    type Transformed;

    /// [`Stream`] of [`Transformed`] [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    #[rustfmt::skip]
    type TransformedStream<'out>:
        Stream<Item = Result<Self::Transformed, Self::Error>> + 'out;

    /// Converts all incoming [`Event`]s into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform_all<'me, 'ctx, 'out>(
        &'me self,
        events: Events,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}

impl<A, Events> Adapter<Events> for A
where
    Events: Stream + 'static,
    A: WithError,
    Wrapper<A>: Transformer<Events::Item> + 'static,
    <Wrapper<A> as Transformer<Events::Item>>::Context: 'static,
    <A as WithError>::Transformed:
        From<<Wrapper<A> as Transformer<Events::Item>>::Transformed>,
    <A as WithError>::Error:
        From<<Wrapper<A> as Transformer<Events::Item>>::Error>,
{
    type Context = <Wrapper<A> as Transformer<Events::Item>>::Context;
    type Error = <A as WithError>::Error;
    type Transformed = <A as WithError>::Transformed;
    type TransformedStream<'out> = TransformedStream<'out, Wrapper<A>, Events>;

    fn transform_all<'me, 'ctx, 'out>(
        &'me self,
        events: Events,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        TransformedStream::new(RefCast::ref_cast(self), events, context)
    }
}

#[pin_project]
/// [`Stream`] for [`Adapter`] blanket impl.
pub struct TransformedStream<'out, Adapter, Events>
where
    Events: Stream,
    Adapter: Transformer<Events::Item>,
{
    #[pin]
    events: Events,
    #[pin]
    transformed_stream: AdapterTransformedStream<'out, Events::Item, Adapter>,
    adapter: &'out Adapter,
    context: &'out Adapter::Context,
}

impl<'out, Adapter, Events> Debug for TransformedStream<'out, Adapter, Events>
where
    Events: Debug + Stream,
    Adapter: Debug + Transformer<Events::Item>,
    Adapter::Context: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformStream")
            .field("events", &self.events)
            .field("adapter", &self.adapter)
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

type AdapterTransformedStream<'out, Event, Adapter> = future::Either<
    <Adapter as Transformer<Event>>::TransformedStream<'out>,
    stream::Empty<
        Result<
            <Adapter as Transformer<Event>>::Transformed,
            <Adapter as Transformer<Event>>::Error,
        >,
    >,
>;

impl<'out, Adapter, Events> TransformedStream<'out, Adapter, Events>
where
    Events: Stream,
    Adapter: Transformer<Events::Item>,
{
    fn new(
        adapter: &'out Adapter,
        events: Events,
        context: &'out Adapter::Context,
    ) -> Self {
        Self {
            events,
            transformed_stream: stream::empty().right_stream(),
            adapter,
            context,
        }
    }
}

impl<'out, Adapter, Events> Stream for TransformedStream<'out, Adapter, Events>
where
    Events: Stream,
    Adapter: Transformer<Events::Item> + WithError,
    <Adapter as WithError>::Transformed:
        From<<Adapter as Transformer<Events::Item>>::Transformed>,
    <Adapter as WithError>::Error:
        From<<Adapter as Transformer<Events::Item>>::Error>,
{
    type Item = Result<
        <Adapter as WithError>::Transformed,
        <Adapter as WithError>::Error,
    >;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let res =
                futures::ready!(this.transformed_stream.as_mut().poll_next(cx));
            if let Some(ev) = res {
                return Poll::Ready(Some(
                    ev.map(Into::into).map_err(Into::into),
                ));
            }

            let res = futures::ready!(this.events.as_mut().poll_next(cx));
            if let Some(event) = res {
                let new_stream =
                    Adapter::transform(*this.adapter, event, *this.context);
                this.transformed_stream.set(new_stream.left_stream());
            } else {
                return Poll::Ready(None);
            }
        }
    }
}

#[cfg(feature = "codegen")]
pub mod codegen {
    //! Re-exports for [`Transformer`] derive macro.
    //!
    //! [`Transformer`]: crate::es::adapter::Transformer

    pub use futures;
}