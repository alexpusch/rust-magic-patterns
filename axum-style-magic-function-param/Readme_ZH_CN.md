# Rust 中 Axum 风格的函数参数示例

原文链接(英文): https://github.com/alexpusch/rust-magic-patterns/tree/master/axum-style-magic-function-param

学习 `Rust` 时，我遇到了一门严格的静态类型语言。具体来说就是没有函数重载或可选参数。

发现 [Axum](https://github.com/tokio-rs/axum) 时，我惊奇地看到了如下特性：

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

`get` 方法可以接收不同类型的函数指针！这是什么魔法？🤯

为了弄清楚原理，我不得不创建一个简化版本来研究：

```rust
fn print_id(id: Id) {
    println!("id is {}", id.0);
}

// Param(param) 只是模式匹配
fn print_all(Param(param): Param, Id(id): Id) {
    println!("param is {param}, id is {id}");
}

pub fn main() {
    let context = Context::new("magic".into(), 33);

    trigger(context.clone(), print_id);
    trigger(context.clone(), print_all);
}
```

在示例中我们有一个接收 `Context` 对象和函数指针的 `trigger` 方法。函数指针可以接收一个或两个 `Id` 或 `Param` 类型的参数。魔法？

## 可移动组件

让我们来看看实现这个功能的可移动组件：

### Context 结构体
```rust
struct Context {
    param: String,
    id: u32,
}
```

`Context` 是接收的状态，在 `Axum` 的情况下就是 `Request`。这就是我们函数想要接收的 “组件” 的来源。在这个简化的例子中，它包含两个数据字段

### FromContext 特征

```rust
trait FromContext {
    fn from_context(context: &Context) -> Self;
}
```

第一个技巧是 `FromContext` 特征。它允许我们创建可以从 `Context` 对象中提取必要数据的 “提取器”。例如：

```rust
pub struct Param(pub String);

impl FromContext for Param {
    fn from_context(context: &Context) -> Self {
        Param(context.param.clone())
    }
}
```

这个特征将允许我们持有一个 `Context`，且调用期望 `Param` 参数的函数。后面会详细介绍。

### Handler 特征
```rust
trait Handler<T> {
    fn call(self, context: Context);
}
```

第二个技巧是 `Handler` 特征。我们将要为 [闭包类型](https://doc.rust-lang.org/reference/types/closure.html) `Fn(T)` 实现 `Handler` 特征。是的，我们可以为闭包类型实现这个特征。这个实现允许我们在函数调用和其参数之间有一个 “中间件”。这里我们会调用 `FromContext::from_context` 方法，将 `Context` 转换为函数期望的参数，`Param` 或 `Id`。

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

为支持多个函数参数，我们会继续为带2个、3个、4个等参数的闭包类型实现 `Handler` 特征。一个有趣的点是，这个实现与参数的顺序无关 - 它同时支持 `fn foo(p: Param, id: Id)` 和 `fn foo(id: Id, p: Param)` 这样的函数签名！

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

### 总结

`trigger` 函数的实现现在就变得很简单了

```rust
pub fn trigger<T, H>(context: Context, handler: H)
where
    H: Handler<T>,
{
    handler.call(context);
}
```

让我们来分析这个调用过程

```rust
  let context = Context::new("magic".into(), 33);

  trigger(context.clone(), print_id);
```

- `print_id` 的类型是 `Fn(Id)`，它有 `Handler<Id>` 的实现。
- 从 `Handler::call` 方法中，我们调用 `Id::from_context(context)`，它返回一个 `Id` 结构体实例。
- `print_id` 使用它期望的参数被调用。

魔法揭秘。