use component_macros::component;

// Missing struct body to trigger a syntax or implementation failure expectation
#[component(singleton, priority = 5)]
struct ;

fn main() {}
