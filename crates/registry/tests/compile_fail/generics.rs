use registry::post_component;

#[allow(non_snake_case)]
#[post_component]
pub fn Bad<T: Clone>(msg: String) {
    drop(msg);
}

fn main() {}
