//! Wrap with could be used to map the value of a method into another method.
//!
//! ```txt
//! spawn(f):
//!     f
//!         .wrap_with(|fut| move |e: &mut Executor| e.spawn(fut))
//!         ;; the orignal method `f` is called first and its output is passed
//!         ;; to the provided closure. Then the provided closure can return back
//!         ;; another closure which will be called using the provider.
//! ```

use std::marker::PhantomData;

use crate::ty::Param;
use crate::Method;

struct WrapWith<F, W, ArgsF, ArgsOutW> {
    method: F,
    wrapper: W,
    _p: PhantomData<(ArgsF, ArgsOutW)>,
}

impl<F, ArgsF, W, OutW, ArgsOutW> Method<()> for WrapWith<F, W, ArgsF, ArgsOutW>
where
    F: Method<ArgsF>,
    W: FnOnce(F::Output) -> OutW,
    OutW: Method<ArgsOutW>,
{
    type Output = OutW::Output;

    #[inline(always)]
    fn events(&self) -> Option<crate::Eventstore> {
        self.method.events()
    }

    #[inline(always)]
    fn dependencies() -> Vec<Param> {
        let mut dependencies = F::dependencies();
        dependencies.extend(OutW::dependencies());
        dependencies
    }

    #[inline(always)]
    fn call(self, provider: &crate::Provider) -> Self::Output {
        let out_f = self.method.call(provider);
        let out_w = (self.wrapper)(out_f);
        out_w.call(provider)
    }
}

#[inline(always)]
pub fn wrap_with<F, ArgsF, W, OutW, ArgsOutW>(
    method: F,
    wrapper: W,
) -> impl Method<(), Output = OutW::Output>
where
    F: Method<ArgsF>,
    W: FnOnce(F::Output) -> OutW,
    OutW: Method<ArgsOutW>,
{
    WrapWith {
        method,
        wrapper,
        _p: PhantomData,
    }
}
