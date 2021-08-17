//! Module for utils used in proc macro expansion.

pub use static_assertions as sa;

/// Utils for ensuring that every [`Event`] variant has a unique combination of
/// [`Event::name()`] and [`Event::ver()`].
///
/// # Explanation
///
/// Main idea is that every [`Event`] or [`VersionedEvent`] deriver generates
/// `const fn __arcana_events() -> [Option<(&'static str, u16)>; size]`
/// const function. Size of outputted array determines max count of unique
/// [`VersionedEvent`]s inside [`Event`] and is tweakable inside
/// `arcana_codegen_impl` crate (default is `100_000` which should be plenty).
/// As these arrays are used only at compile-time, there should be no
/// performance impact at runtime.
///
/// - Structs
///
///   [`unique_event_name_and_ver_for_struct`] macro generates function, which
///   returns array with only first occupied entry. The rest of them are
///   [`None`].
///
/// - Enums
///
///   [`unique_event_name_and_ver_for_enum`] macro generates function, which
///   glues subtypes arrays into single continues array. First `n` entries are
///   occupied, while the rest of them are [`None`], where `n` is the number of
///   [`VersionedEvent`]s. As structs deriving [`VersionedEvent`] and enums
///   deriving [`Event`] have the same output by `__arcana_events()` const
///   function, top-level enum variants can have different levels of nesting.
///
///   [`unique_event_name_and_ver_check`] macro generates [`const_assert`]
///   check, which fails in case of duplicated [`Event::name()`] and
///   [`Event::ver()`].
///
///
/// [`const_assert`]: static_assertions::const_assert
/// [`Event`]: trait@crate::Event
/// [`Event::name()`]: trait@crate::Event::name()
/// [`Event::ver()`]: trait@crate::Event::ver()
/// [`VersionedEvent`]: trait@crate::VersionedEvent
pub mod unique_event_name_and_ver {
    #[doc(hidden)]
    #[macro_export]
    macro_rules! unique_event_name_and_ver_for_struct {
        ($max_events:literal, $event_name:literal, $event_ver:literal) => {
            #[allow(clippy::large_stack_arrays)]
            pub const fn __arcana_events(
            ) -> [Option<(&'static str, u16)>; $max_events] {
                let mut res = [None; $max_events];
                res[0] = Some(($event_name, $event_ver));
                res
            }
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! unique_event_name_and_ver_for_enum {
        ($max_events: literal, $($event_name: ty),* $(,)?) => {
            #[allow(clippy::large_stack_arrays)]
            pub const fn __arcana_events() ->
                [Option<(&'static str, u16)>; $max_events]
            {
                let mut res = [None; $max_events];

                let mut global = 0;

                $({
                    let ev = <$event_name>::__arcana_events();
                    let mut local = 0;
                    while let Some(s) = ev[local] {
                        res[global] = Some(s);
                        local += 1;
                        global += 1;
                    }
                })*

                res
            }
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! unique_event_name_and_ver_check {
        ($event:ty) => {
            $crate::private::sa::const_assert!(
                $crate::private::unique_event_name_and_ver::all_unique(
                    <$event>::__arcana_events()
                )
            );
        };
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn all_unique<const N: usize>(
        events: [Option<(&str, u16)>; N],
    ) -> bool {
        let mut outer = 0;
        while let Some((outer_name, outer_ver)) = events[outer] {
            let mut inner = outer + 1;
            while let Some((inner_name, inner_ver)) = events[inner] {
                if str_eq(inner_name, outer_name) && inner_ver == outer_ver {
                    return false;
                }
                inner += 1;
            }
            outer += 1;
        }

        true
    }

    #[doc(hidden)]
    #[must_use]
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