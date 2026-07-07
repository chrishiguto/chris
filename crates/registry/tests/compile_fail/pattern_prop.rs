use registry::post_component;

#[allow(non_snake_case)]
#[post_component]
pub fn Bad((a, b): (String, String)) {
    drop((a, b));
}

fn main() {}
