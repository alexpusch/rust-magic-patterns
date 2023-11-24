#[derive(Clone)]
pub struct Context {
    param: String,
    id: u32,
}

impl Context {
    pub fn new(param: String, id: u32) -> Self {
        Context { param, id }
    }
}
pub struct Param(pub String);

pub struct Id(pub u32);

pub trait FromContext {
    fn from_context(context: &Context) -> Self;
}

impl FromContext for Param {
    fn from_context(context: &Context) -> Self {
        Param(context.param.clone())
    }
}

impl FromContext for Id {
    fn from_context(context: &Context) -> Self {
        Id(context.id)
    }
}

pub trait Handler<T> {
    fn call(self, context: Context);
}

impl<F, T> Handler<T> for F
where
    F: Fn(T),
    T: FromContext,
{
    fn call(self, context: Context) {
        (self)(T::from_context(&context));
    }
}

impl<T1, T2, F> Handler<(T1, T2)> for F
where
    F: Fn(T1, T2),
    T1: FromContext,
    T2: FromContext,
{
    fn call(self, context: Context) {
        (self)(T1::from_context(&context), T2::from_context(&context));
    }
}

pub fn trigger<T, H>(context: Context, handler: H)
where
    H: Handler<T>,
{
    handler.call(context);
}
