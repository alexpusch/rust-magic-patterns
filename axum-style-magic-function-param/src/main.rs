use magic::{trigger, Context, Id, Param};

mod magic;

fn print_id(id: Id) {
    println!("id is {}", id.0);
}

fn print_param(Param(param): Param) {
    println!("param is {param}");
}

fn print_all(Param(param): Param, Id(id): Id) {
    println!("param is {param}, id is {id}");
}

fn print_all_switched(Id(id): Id, Param(param): Param) {
    println!("param is {param}, id is {id}");
}

pub fn main() {
    let context = Context::new("magic".into(), 33);

    trigger(context.clone(), print_id);
    trigger(context.clone(), print_param);
    trigger(context.clone(), print_all);
    trigger(context, print_all_switched);
}
