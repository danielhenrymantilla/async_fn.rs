#![forbid(unsafe_code)]

use ::async_fn::prelude::*;

const _: () = {
    #[barefoot] async
    fn fun ()
      -> impl Fut<'static, ()>
    {}
};

const _: () = {
    #[barefoot] async
    fn fun (it: &'_ ())
      -> impl Fut<'_, ()>
    {
        drop(it);
    }
};

const _: () = {
    #[barefoot] async
    fn fun (it: &())
      -> impl Fut<'_, ()>
    {
        drop(it);
    }
};

#[test]
fn static_future ()
{
    #[barefoot] async
    fn fun1 (&it: &i32)
      -> impl Fut<'static, i32>
    {
        it
    }

    #[barefoot] async
    fn fun2 (it: &i32)
      -> impl Fut<'static, i32>
    {
        before_async! {
            let &it = it;
        }
        it
    }

    let () = ::extreme::run(async {
        #[allow(unused)]
        use ::core::stringify as before_async;

        #[barefoot] async
        fn fun3 (it: &i32)
          -> impl Fut<'static, i32>
        {
            ::async_fn::before_async! {
                let &it = it;
            }
            it
        }

        let fut1 = fun1(&(|| 42)());
        let fut2 = fun2(&(|| 42)());
        let fut3 = fun3(&(|| 42)());
        assert_eq!(fut1.await, 42);
        assert_eq!(fut2.await, 42);
        assert_eq!(fut3.await, 42);
    });
}
