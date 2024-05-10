# Rusts Axum style magic function params example

<details>
  <summary>Additional languages</summary>
  <ul>
    <li>
      <a href='https://github.com/yushengguo557/rust-magic-patterns/blob/translation-zh-cn/axum-style-magic-function-param/Readme_ZH_CN.md'>Simplified Chinese</a> - <a href="https://github.com/yushengguo557">@yushengguo557</a>
    </li>
  </ul>
</details>

Learning Rust I met a rigid, statically typed language. Specifically it has no function overloading or optional function parameters.

Coming across [Axum](https://github.com/tokio-rs/axum) I was amazed to see stuff like:

```rust
let app = Router::new()
  .route("/users", get(get_users))
  .route("/products", get(get_product));

async fn get_users(Query(params): Query<Params>) -> impl IntoResponse {
    let users = /* ... */

    Json(users)
}

async fn get_product(State(db): State<Db>, Json(payload): Json<Payload>) -> String {
  let product = /* ... */

  product.to_string()
}
```

The `get` method can receive a function pointer to various types of functions! What kind of black magic is this? 🤯

I had to create a simplified version of this to figure this out.

```rust
fn print_id(id: Id) {
    println!("id is {}", id.0);
}

// Param(param) is just pattern matching
fn print_all(Param(param): Param, Id(id): Id) {
    println!("param is {param}, id is {id}");
}

pub fn main() {
    let context = Context::new("magic".into(), 33);

    trigger(context.clone(), print_id);
    trigger(context.clone(), print_all);
}
```

In the example we have a `trigger` method that receives a `Context` object and a function pointer. The function pointer might receive 1 or 2 parameters of the `Id` or `Param` types. Magic?

## Moving parts
Lets look at the moving parts to achieve this

### The context
```rust
struct Context {
    param: String,
    id: u32,
}
```

The `Context` is the received state, `Request` in Axums case. This is the source of the "parts" our functions want to receive. In this simplified example it contains two data fields

### The FromContext trait
```rust
trait FromContext {
    fn from_context(context: &Context) -> Self;
}
```

The first trick is the `FromContext` trait. It will allow us to create "Extractors" that extract the necessary data from the context object. For example
```rust
pub struct Param(pub String);

impl FromContext for Param {
    fn from_context(context: &Context) -> Self {
        Param(context.param.clone())
    }
}
```
This trait will allow us to hold a `Context` but call a function that expects `Param`. More on this later

### The Handler trait
```rust
trait Handler<T> {
    fn call(self, context: Context);
}
```

The second trick is the Handler trait. We will implement the trait for the [closure type](https://doc.rust-lang.org/reference/types/closure.html) `Fn(T)`. Yeah we can implement traits for closure types. This implementation will allow us to have a "middleware" between the function call and its arguments. Here we will call the `FromContext::from_context` method, converting the context to the expected function argument i.e `Param` or `Id`. 

```rust
impl<F, T> Handler<T> for F
where
    F: Fn(T),
    T: FromContext,
{
    fn call(self, context: Context) {
        (self)(T::from_context(&context));
    }
}
```

To Support multiple function parameters we'll go ahead and implement `Handler` for closure types with 2, 3, 4 and so on parameters. An interesting point here is that this implementation is agnostic to the order of the parameters - it will support both `fn foo(p: Param, id: Id)` and `fn foo(id: Id, p: Param)`!
```rust
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
```

### Putting it all together
The implementation of the `trigger` function is now straight forward
```rust
pub fn trigger<T, H>(context: Context, handler: H)
where
    H: Handler<T>,
{
    handler.call(context);
}
```

Lets examine what happens for this call
```rust
  let context = Context::new("magic".into(), 33);

  trigger(context.clone(), print_id);
```

- `print_id` is of type `Fn(Id)` which has an implementation for `Handler<Id>`.
- The `Handler::call` method is called from which we `Id::from_context(context)` which returns an instance of `Id` struct.
- `print_id` is called with the parameter it expects.

Magic demystified.