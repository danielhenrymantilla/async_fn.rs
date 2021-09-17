#![allow(nonstandard_style, unused_imports)]

use ::core::{
    ops::Not as _,
};
use ::proc_macro::{
    TokenStream,
};
use ::proc_macro2::{
    Span,
    TokenStream as TokenStream2,
    TokenTree as TT,
};
use ::quote::{
    format_ident,
    quote,
    quote_spanned,
    ToTokens,
};
use ::syn::{*,
    parse::{Parse, Parser, ParseStream},
    punctuated::Punctuated,
    Result, // Explicitly shadow it
};

// # Example
//
// ```rust,ignore
// use ::async_fn::prelude::*;
//
// #[async_fn::bare_future]
// pub async fn start_spinning(&self, id: Id) -> impl Fut<Result<(), Error>> + 'static {
//     before_async! {
//         let inner = Arc::clone(&self.inner);
//     }
//     inner.start_spinning(id).await
// }
// ```
#[proc_macro_attribute] pub
fn bare_future (
    attrs: TokenStream,
    input: TokenStream,
) -> TokenStream
{
    bare_future_impl(attrs.into(), input.into())
        // .map(|ret| { println!("{}", ret); ret })
        .unwrap_or_else(|err| {
            let mut errors =
                err .into_iter()
                    .map(|err| Error::new(
                        err.span(),
                        format_args!("`#[async_fn::bare_future]`: {}", err),
                    ))
            ;
            let mut err = errors.next().unwrap();
            errors.for_each(|cur| err.combine(cur));
            err.to_compile_error()
        })
        .into()
}

fn bare_future_impl (
    attrs: TokenStream2,
    input: TokenStream2,
) -> Result<TokenStream2>
{
    let _: parse::Nothing = parse2(attrs)?;
    let mut fun: ImplItemMethod = parse2(input)?;
    let sig = &mut fun.sig;

    // Ensure the fun is an `async fn`.
    let async_ = if let Some(it) = sig.asyncness.take() { it } else {
        let span =
            Option::or(
                sig.unsafety.as_ref().map(|it| it.span),
                sig.abi.as_ref().map(|it| it.extern_token.span),
            )
            .unwrap_or(sig.fn_token.span)
        ;
        return Err(Error::new(span, "expected `async`"));
    };

    // The `'args` lifetime.
    // if sig.generics.lifetimes().all(|lt| lt.lifetime.ident != "args") {
    //     sig.generics.params.insert(0, parse_quote!('args));
    //     TODO: Copy-pasta the impl in:
    //     https://github.com/danielhenrymantilla/fix_hidden_lifetime_bug.rs
    //               /blob/v0.2.4/src/proc_macros/collect_lifetime_params.rs
    // }

    // Try to see if the block starts with a `before_async!` block:
    let mut stmts = match *fun.block.stmts {
        | [
            Stmt::Item(
                Item::Macro(ItemMacro {
                    ref attrs,
                    ident: None,
                    ref mut mac,
                    semi_token: _,
                }),
            ),
            ..
        ]
        | [
            Stmt::Semi(
                Expr::Macro(ExprMacro {
                    ref attrs,
                    ref mut mac,
                }),
                _,
            ),
            ..
        ]
        if mac.path.is_ident("before_async")
        || (0 .. mac.path.segments.len()).all({
            let expected_segments = ["async_fn", "before_async"];
            let mac = &mac;
            move |i| {
                mac.path.segments[i].ident == expected_segments[i]
            }
        })
        => {
            if let Some(attr) = attrs.first() {
                return Err(Error::new_spanned(
                    attr,
                    "unexpected attribute(s)",
                ));
            }
            let mut stmts = vec![];
            let () = mac.parse_body_with(|tts: ParseStream<'_>| Ok({
                while tts.is_empty().not() {
                    stmts.push(tts.parse::<Stmt>()?);
                }
            }))?;
            mac.tokens = quote!(@);
            // Rather than fully shadowing `before_async!` in a magical and thus
            // potentially uninituitive way (which would be achieved by stripping
            // the original `before_async!` once we've parsed it), we are gonna,
            // instead, shadow it with a macro that is brought into scope in a
            // fashion which is known to be fragile to name conflicts. This way,
            // if there was an actual `before_async!` macro in scope, a rather
            // nice error message will be printed.
            if mac.path.segments.len() == 1 {
                stmts.push(parse_quote!(
                    use ::async_fn::__::before_async::*;
                ));
            }
            stmts
        },

        | _ => vec![],
    };

    // Wrap the body in an `async move`
    stmts.push(Stmt::Expr(Expr::Async(ExprAsync {
        attrs: vec![],
        async_token: async_,
        capture: Some(token::Move { span: fun.sig.fn_token.span }),
        block: Block {
            brace_token: token::Brace { span: async_.span },
            stmts: ::core::mem::take(&mut fun.block.stmts),
        },
    })));
    fun.block.stmts = stmts;
    Ok(fun.into_token_stream())
}
