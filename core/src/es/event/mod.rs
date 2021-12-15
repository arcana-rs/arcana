//! [`Event`] machinery.

use std::{convert::TryFrom, marker::PhantomData, num::NonZeroU16};

use derive_more::{Deref, DerefMut, Display, Into};
use ref_cast::RefCast;

pub mod adapter;

#[doc(inline)]
pub use self::adapter::Adapter;

/// Fully qualified name of an [`Event`].
pub type Name = &'static str;

/// Revision number of an [`Event`].
#[derive(
    Clone, Copy, Debug, Display, Eq, Hash, Into, Ord, PartialEq, PartialOrd,
)]
pub struct Version(NonZeroU16);

impl Version {
    /// Creates a new [`Version`] out of the given `value`.
    ///
    /// The given `value` should not be `0` (zero) and fit into [`u16`] size.
    #[must_use]
    pub fn try_new<N>(value: N) -> Option<Self>
    where
        u16: TryFrom<N>,
    {
        Some(Self(NonZeroU16::new(u16::try_from(value).ok()?)?))
    }

    /// Creates a new [`Version`] out of the given `value` without checking its
    /// invariants.
    ///
    /// # Safety
    ///
    /// The given `value` must not be `0` (zero).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: u16) -> Self {
        Self(unsafe { NonZeroU16::new_unchecked(value) })
    }

    /// Returns the value of this [`Version`] as a primitive type.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

/// [`Event`] of a concrete [`Version`].
pub trait Versioned {
    /// [`Name`] of this [`Event`].
    const NAME: Name;

    /// [`Version`] of this [`Event`].
    const VERSION: Version;
}

/// [Event Sourcing] event describing something that has occurred (happened
/// fact).
///
/// [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html
pub trait Event {
    /// Returns [`Name`] of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    #[must_use]
    fn name(&self) -> Name;

    /// Returns [`Version`] of this [`Event`].
    #[must_use]
    fn version(&self) -> Version;
}

impl<Ev: Versioned + ?Sized> Event for Ev {
    fn name(&self) -> Name {
        <Self as Versioned>::NAME
    }

    fn version(&self) -> Version {
        <Self as Versioned>::VERSION
    }
}

/// State that can be calculated by applying the specified [`Event`].
pub trait Sourced<Ev: ?Sized> {
    /// Applies the specified [`Event`] to the current state.
    fn apply(&mut self, event: &Ev);
}

impl<Ev: Versioned + ?Sized, S: Sourced<Ev>> Sourced<Ev> for Option<S> {
    fn apply(&mut self, event: &Ev) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S: Sourced<dyn Event + 'e>> Sourced<dyn Event + 'e> for Option<S> {
    fn apply(&mut self, event: &(dyn Event + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S: Sourced<dyn Event + Send + 'e>> Sourced<dyn Event + Send + 'e>
    for Option<S>
{
    fn apply(&mut self, event: &(dyn Event + Send + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S: Sourced<dyn Event + Send + Sync + 'e>>
    Sourced<dyn Event + Send + Sync + 'e> for Option<S>
{
    fn apply(&mut self, event: &(dyn Event + Send + Sync + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

/// [`Event`] sourcing the specified state.
///
/// This is a reversed version of [`Sourced`] trait intended to simplify usage
/// of trait objects describing sets of [`Event`]s. Shouldn't be implemented
/// manually, but rather used as blanket impl.
///
/// # Example
///
/// ```rust
/// # use arcana::es::event::{self, Sourced as _};
/// #
/// #[derive(Debug, Eq, PartialEq)]
/// struct Chat;
///
/// #[derive(event::Versioned)]
/// #[event(name = "chat", version = 1)]
/// struct ChatEvent;
///
/// impl event::Initialized<ChatEvent> for Chat {
///     fn init(_: &ChatEvent) -> Self {
///         Self
///     }
/// }
///
/// let mut chat = Option::<Chat>::None;
/// let ev = event::Initial(ChatEvent);
/// let ev: &dyn event::Sourcing<Option<Chat>> = &ev;
/// chat.apply(ev);
/// assert_eq!(chat, Some(Chat));
/// ```
pub trait Sourcing<S: ?Sized> {
    /// Applies this [`Event`] to the specified `state`.
    fn apply_to(&self, state: &mut S);
}

impl<Ev: ?Sized, S: Sourced<Ev> + ?Sized> Sourcing<S> for Ev {
    fn apply_to(&self, state: &mut S) {
        state.apply(self);
    }
}

impl<'e, S: ?Sized> Sourced<dyn Sourcing<S> + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + 'e)) {
        event.apply_to(self);
    }
}

impl<'e, S: ?Sized> Sourced<dyn Sourcing<S> + Send + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + Send + 'e)) {
        event.apply_to(self);
    }
}

impl<'e, S: ?Sized> Sourced<dyn Sourcing<S> + Send + Sync + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + Send + Sync + 'e)) {
        event.apply_to(self);
    }
}

/// Before a state can be [`Sourced`] it needs to be [`Initialized`].
pub trait Initialized<Ev: ?Sized> {
    /// Creates an initial state from the given [`Event`].
    #[must_use]
    fn init(event: &Ev) -> Self;
}

/// Wrapper type to mark an [`Event`] that makes some [`Sourced`] state being
/// [`Initialized`].
///
/// Exists solely to solve specialization problems.
#[derive(Clone, Copy, Debug, Deref, DerefMut, Display, RefCast)]
#[repr(transparent)]
pub struct Initial<Ev: ?Sized>(pub Ev);

// Manual implementation due to `derive_more::From` not being able to strip
// `?Sized` trait bound.
impl<Ev> From<Ev> for Initial<Ev> {
    fn from(ev: Ev) -> Self {
        Self(ev)
    }
}

impl<Ev: Event + ?Sized, S: Initialized<Ev>> Sourced<Initial<Ev>>
    for Option<S>
{
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(S::init(&event.0));
    }
}

// TODO: Replace `Ev` with `const Name` (same as `const &'static str`), once
//       `adt_const_params` feature stabilizes.
//       https://github.com/rust-lang/rust/issues/44580
/// Raw [`Versioned`] event.
///
/// This type is useful for repository-level [`Event`]s to avoid necessity of
/// describing every [`Version`] as a separate type.
///
/// As [`Event::version()`] is infallible method, every instance of this type
/// has to store [`Version`]. Deserialization errors should be handled before
/// creation or as a part of an [`Adapter`] implementation.
///
/// > __WARNING:__ This type isn't considered in const assertion generated by
/// >              [`Event`] derive macro, which ensures that every combination
/// >              of [`Name`] and [`Version`] corresponds to a single Rust
/// >              type. That means if your [`Event`] enum has some concrete
/// >              types with same [`Name`] as `Ev`, this struct should be
/// >              considered as a fallback for [`Version`]s not covered with
/// >              those concrete types.
///
/// [`Adapter`]: crate::es::event::Adapter
#[derive(
    Clone, Copy, Debug, Deref, DerefMut, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
pub struct Raw<Ev: ?Sized, Data> {
    /// Inner [`Event`] data.
    #[deref]
    #[deref_mut]
    pub data: Data,

    /// This [`Event`] [`Version`].
    pub version: Version,

    /// Used for [`Versioned::NAME`] in [`Event`] implementation.
    _event: PhantomData<Ev>,
}

impl<Ev: ?Sized, Data> Raw<Ev, Data> {
    /// Creates a new [`Raw`].
    #[must_use]
    pub const fn new(data: Data, version: Version) -> Self {
        Self {
            data,
            version,
            _event: PhantomData,
        }
    }
}

impl<Ev, Data> Event for Raw<Ev, Data>
where
    Ev: Versioned + ?Sized,
{
    fn name(&self) -> Name {
        Ev::NAME
    }

    fn version(&self) -> Version {
        self.version
    }
}

/// Marker trait for [`Versioned`] or [`Raw`] [`Event`]s. Used for [`Adapter`]
/// specialization.
///
/// [`Adapter`]: crate::es::event::Adapter
pub trait VersionedOrRaw {}

impl<Ev, Data> VersionedOrRaw for Raw<Ev, Data>
where
    Ev: ?Sized,
    Self: Event,
{
}

impl<Ev: Versioned> VersionedOrRaw for Ev {}

#[cfg(feature = "codegen")]
pub mod codegen {
    //! [`Event`] machinery aiding codegen.
    //!
    //! [`Event`]: super::Event

    use super::Raw;

    pub use futures;
    pub use ref_cast;

    impl<Ev: ?Sized, Data> Raw<Ev, Data> {
        #[doc(hidden)]
        #[must_use]
        pub const fn __arcana_events() -> [(&'static str, &'static str, u16); 0]
        {
            []
        }
    }

    /// Tracking of [`VersionedEvent`]s number.
    ///
    /// [`VersionedEvent`]: super::Versioned
    pub trait Versioned {
        /// Number of [`VersionedEvent`]s in this [`Event`].
        ///
        /// [`Event`]: super::Event
        /// [`VersionedEvent`]: super::Versioned
        const COUNT: usize;
    }

    impl<Ev: ?Sized, Data> Versioned for Raw<Ev, Data> {
        const COUNT: usize = 0;
    }

    /// Checks in compile time whether all the given combinations of
    /// [`Event::name`] and [`Event::version`] correspond to different Rust
    /// types.
    ///
    /// # Explanation
    ///
    /// Main idea is that every [`Event`] or [`event::Versioned`] deriving
    /// generates a hidden method:
    /// ```rust,ignore
    /// const fn __arcana_events() -> [(&'static str, &'static str, u16); size]
    /// ```
    /// It returns an array consisting of unique Rust type identifiers,
    /// [`event::Name`]s and [`event::Version`]s of all the [`Event`] variants.
    /// Correctness is checked then with asserting this function at compile time
    /// in `const` context.
    ///
    /// [`Event`]: super::Event
    /// [`Event::name`]: super::Event::name
    /// [`Event::version`]: super::Event::version
    /// [`event::Name`]: super::Name
    /// [`event::Version`]: super::Version
    /// [`event::Versioned`]: super::Versioned
    #[must_use]
    pub const fn has_different_types_with_same_name_and_ver<const N: usize>(
        events: [(&str, &str, u16); N],
    ) -> bool {
        let mut outer = 0;
        while outer < events.len() {
            let mut inner = outer + 1;
            while inner < events.len() {
                let (inner_ty, inner_name, inner_ver) = events[inner];
                let (outer_ty, outer_name, outer_ver) = events[outer];
                if !str_eq(inner_ty, outer_ty)
                    && str_eq(inner_name, outer_name)
                    && inner_ver == outer_ver
                {
                    return true;
                }
                inner += 1;
            }
            outer += 1;
        }

        false
    }

    /// Compares strings in `const` context.
    ///
    /// As there is no `const impl Trait` and `l == r` calls [`Eq`], we have to
    /// write custom comparison function.
    ///
    /// [`Eq`]: std::cmp::Eq
    // TODO: Remove once `Eq` trait is allowed in `const` context.
    const fn str_eq(l: &str, r: &str) -> bool {
        if l.len() != r.len() {
            return false;
        }

        let (l, r) = (l.as_bytes(), r.as_bytes());
        let mut i = 0;
        while i < l.len() {
            if l[i] != r[i] {
                return false;
            }
            i += 1;
        }

        true
    }
}
