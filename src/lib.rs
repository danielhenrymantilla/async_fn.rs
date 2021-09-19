#![cfg_attr(feature = "better-docs",
    cfg_attr(all(), doc = include_str!("../README.md")),
)]
#![no_std]
#![forbid(unsafe_code)]

/// One of the main macros of the crate.
///
/// `#[async_fn::bare_future]` allows to express the actual `Future` return type
/// of an `async fn`.
///
/// Implementation-wise, and thus, semantic-wise, what this macro does is very
/// simple: it removes the `async` from the `async fn`, and wraps the function
/// body within an `async move { … }` block (except for the statements inside
/// the [`before_async!`] eager prelude).
///
/// The point of doing this is to reduce the rightwards drift that an explicit
/// `async move` block requires.
///
/// ## Example / unsugaring
///
/// ```rust
/// use ::async_fn::prelude::*;
/// # use ::std::sync::Arc;
///
/// async fn foo(/* … */) { /* … */ }
/// async fn bar(/* … */) -> i32 {
///     // …
/// #   42
/// }
///
/// struct Foo {
///     // …
/// }
///
/// impl Foo {
///     #[bare_future]
///     async fn thread_safe_async_method(&self) -> impl Fut<'_, i32> + Send {
///         foo(/* … */).await;
///         bar(/* … */).await
///     }
/// }
/// # #[cfg(any())]
/// /// The above is sugar for:
/// impl Foo {
///     fn thread_safe_async_method(&self) -> impl Fut<'_, i32> + Send {
///         async move {
///             foo(/* … */).await;
///             bar(/* … */).await
///         }
///     }
/// }
/// ```
///
/// The advantages of using `#[bare_future]` over an `async move` body are thus
/// slim, but non-negligible:
///
///   - The function is still marked as / appears as `async fn`, which makes it
///     easier to grep for;
///   - The extra rightward drift in the `async move { … }` body is avoided.
///
/// # Extra features and benefits
///
/// ## 1 — The `'args` lifetime
///
/// > ⚠️ Not implemented yet. ⚠️
///
/// A `[bare_future] async fn` function will **feature a magical `'args`
/// lifetime** (parameter) which will represent the "intersection" of the
/// lifetimes / spans of usability of *each* function parameter. This makes it
/// so:
///
/// ```rust
/// # const _: &str = stringify! {
/// #[bare_future]
/// async fn fname(/* args… */) -> impl Fut<'args, RetTy>
/// /// Has the same semantics as:
/// async fn fname(/* args… */) -> RetTy
/// # };
/// ```
///
/// ## 2 — The `before_async!` eager prelude
///
///   - This is for an opposite goal to that of using `'args`, mainly, when
///     wanting to yield a `'static + Future`.
///
/// See [`before_async!`] for more info.
#[doc(inline)]
pub use ::async_fn_proc_macros::bare_future;

/// Convenience shorthand alias.
///
/// ```rust
/// # const _: &str = stringify! {
/// trait /* alias */ Fut<'fut, Ret> = ::core::future::Future<Output = Ret> + 'fut;
/// # };
/// ```
pub
trait Fut<'fut, Ret>
where
    Self : 'fut + ::core::future::Future<Output = Ret>,
{}

impl<'fut, Ret, F : ?Sized> Fut<'fut, Ret>
    for F
where
    Self : 'fut + ::core::future::Future<Output = Ret>,
{}

/// Helper macro to express **eagerly** executed code (before the `async`
/// suspension) when inside [`#[async_fn::bare_future]`][`bare_future`]-annotated function.
///
/// ### Eager _vs._ lazy / suspended code?
///
/// <details>
///
/// Consider:
///
/// ```rust
/// use ::async_fn::prelude::*;
///
/// /// Consider:
/// async fn lazy_print() -> i32 {
///     println!("Hi");
///     42
/// }
/// /// i.e.,
/// fn lazy_print_unsugared() -> impl Fut<'static, i32> {
///     /* return */ async move { // <- lazy future / suspension point
///         println!("Hi"); // <- this runs _after_ returning
///         42
///     }
/// }
///
/// /// vs.
/// fn eager_print() -> impl Fut<'static, i32> {
///     println!("Hi"); // <- this runs _before_ returning
///     /* return */ async move { // <- lazy future / suspension point
///         42
///     }
/// }
/// ```
///
/// Both `lazy_print().await` and `eager_print().await` shall behave the same
/// insofar that they'll both print `"Hi"` and then resolve to `42`.
///
/// That being said, when doing:
///
/// ```rust,ignore
/// let future = mk_future();
/// println!("Bye");
/// assert_eq!(future.await, 42);
/// ```
///
/// then, depending on whether `mk_future` refers to `eager_print` or
/// `lazy_print`, the `println!("Hi")` statement will, respectively, be
/// executed before returning the future **or be part of the not-yet-polled
/// future**. That is, `"Hi"` will be printed before `"Bye"` or _after_.
///
/// While this may look like a contrived example, when `future` is spawned /
/// to be polled within a parallel executor (_e.g._,
/// `mt_executor::spawn(mk_future()); println!("Bye");`), then in the eager case
/// we'll have the `"Hi"` statement definitely occur _before_ the `"Bye"`
/// statement, whereas in the lazy case there will be no clear ordering between
/// the two: `"Hi"` and `"Bye"` could appear in any order, or even intermingled!
///
/// Now, imagine if `"Hi"` were a dereference of a short-lived borrow, (such as
/// `let x = *borrowed_integer;` or `Arc::clone(borrowed_arc)`), and if
/// `"Bye"` were a statement dropping the borrowee. While in the eager case we'd
/// have a clear happens-before relationship that'd guarantee soundness, in the
/// lazy case we wouldn't have it, and it would thus be very well possible to
/// suffer from race conditions or a use-after-free! In Rust this means we'll
/// hit borrow-checker errors.
///
/// So this whole thing has to do with lifetimes:
///
///   - either the returned future is short-lived; this is the case of:
///
///       - `async fn …(borrowed_arc: &Arc<…>) -> i32`
///
///       - or `fn …_unsugared(borrowed_arc: &'_ Arc<…>) -> impl Fut<'_, i32>`,
///     )
///
///     Such futures are thus incompatible with a long-lived `spawn()`.
///
///   - or the future is to be compatible with long-lived `spawn()`s (it must be
///     `'static`, and often, also `Send`); this is the case of:
///
///       - `fn …(borrowed_arc: &'_ Arc<…>) -> impl Fut<'static, i32>`
///
///     and for that to actually pass borrowck **any dereference in the `fn`
///     body** (such as `Arc::clone(borrowed_arc)`) **has to be done
///     _eagerly_**:
///
///     ```rust,ignore
///     fn some_future(borrowed_arc: &'_ Arc<Stuff>) -> impl Fut<'static, Ret> {
///         let owned_arc = Arc::clone(borrowed_arc);
///         async move /* owned_arc */ {
///             stuff(&owned_arc).await
///         }
///     }
///     ```
///
/// But in the case of a `#[async_fn::bare_future] async fn`, the whole function body
/// is automagically wrapped within an `async` suspension!
///
/// That's thus the purpose of this macro:
///
/// </details>
///
/// ## Usage
///
/// When inside the body of a `#[async_fn::bare_future]` `async fn`, this macro can be
/// called **as the very first statement of the function's body**, with
/// statements inside it (any other usage is an error, or might error in future
/// semver-compatible releases ⚠️).
///
/// That will make the statements be executed _eagerly_ / before the `async`
/// suspension (hence the name of the macro).
///
/// ```rust
/// use ::async_fn::prelude::*;
/// use ::std::sync::Arc;
/// #
/// # pub enum Error {}
///
/// struct FooInner { /* … */ }
/// impl FooInner {
///     async fn stuff(&self) -> Result<i32, Error> {
///         /* … */
/// #       Ok(42)
///     }
/// }
///
/// pub struct Foo {
///     inner: Arc<FooInner>,
/// }
///
///
/// impl Foo {
///     #[async_fn::bare_future]
///     pub async fn stuff(&self) -> impl Fut<'static, Result<(), Error>> {
///         before_async! {
///             let inner = Arc::clone(&self.inner);
///         }
///         let x = inner.stuff().await?; // <- await point.
///         println!("{}", x);
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! before_async {(@)=>();(
        $($_:tt)*
) => (
    $crate::__::core::compile_error! {
        "\
            this can only be called as the first statement of a \
            `#[async_fn::bare_future]`-annotated function.\
        "
    }
)}

pub
mod prelude {
    #[doc(no_inline)]
    pub use crate::{
        bare_future,
        before_async,
        Fut,
    };

    /// Alternative name for [`bare_future`] 🙃
    pub use bare_future as barefoot;
}

#[doc(hidden)] /** Not part of the public API */ pub
mod __ {
    pub use ::core;

    pub mod before_async {
        pub use crate::before_async;
    }
}
