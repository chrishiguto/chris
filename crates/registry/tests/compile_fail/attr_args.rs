use registry::post_component;

#[allow(non_snake_case)]
#[post_component(nope)]
pub fn Bad(msg: String) {
    drop(msg);
}

fn main() {}
