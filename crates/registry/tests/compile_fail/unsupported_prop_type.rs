use registry::post_component;

#[allow(non_snake_case)]
#[post_component]
pub fn Bad(data: Vec<String>) {
    drop(data);
}

fn main() {}
