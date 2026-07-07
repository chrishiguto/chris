use registry::post_component;

#[allow(non_snake_case)]
#[post_component]
pub fn Bad(children: Vec<String>) {
    drop(children);
}

fn main() {}
