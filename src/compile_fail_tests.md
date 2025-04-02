# The following snippets fail to compile

### Attribute must be applied to an _async_ `fn`

```rust ,compile_fail
use ::async_fn::prelude::*;

#[barefoot]
fn fun ()
  -> impl Fut<'static, ()>
{}
```


### Non-`Send` `-> impl Fut<…> + Send` `async fn`

```rust ,compile_fail
use ::async_fn::prelude::*;

#[barefoot] async
fn fun ()
  -> impl Fut<'static, ()> + Send
{
    let not_send = 0 as *const ();
    let _await_point = async {}.await
    drop(not_send);
}
```

### Non-`'static` `-> impl Fut<'static, …>` `async fn`

```rust ,compile_fail
use ::async_fn::prelude::*;

#[barefoot] async
fn fun (it: &())
  -> impl Fut<'static, ()>
{
    drop(it);
}
```

## `before_async!`

### Has to be called as the first statement of a `#[bare_future]`-annotated function

```rust ,compile_fail
::async_fn::before_async!();
```

```rust ,compile_fail
use ::async_fn::prelude::*;

#[barefoot] async
fn fun ()
  -> impl Fut<'static, ()>
{
    let () = ();
    before_async!();
}
```

### May not "implicitly" shadow another eponymous macro in scope

```rust ,compile_fail
use ::async_fn::prelude::*;

use ::core::stringify as before_async;

#[barefoot] async
fn fun ()
  -> impl Fut<'static, ()>
{
    before_async!();
}
```

```rust ,compile_fail
use ::async_fn::prelude::*;

macro_rules! before_async {() => ()}

#[barefoot] async
fn fun ()
  -> impl Fut<'static, ()>
{
    before_async!();
}
```

```rust ,compile_fail
use ::async_fn::prelude::{barefoot, Fut};

use blob_import::*;
mod blob_import {
    macro_rules! before_async {() => ()}
    pub(in super) use before_async;
}

#[barefoot] async
fn fun ()
  -> impl Fut<'static, ()>
{
    before_async!();
}
```
