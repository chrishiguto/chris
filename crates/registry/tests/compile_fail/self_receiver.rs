use registry::post_component;

struct Widget;

impl Widget {
    #[post_component]
    pub fn bad(self, msg: String) {
        drop((self, msg));
    }
}

fn main() {}
