use component_macros::component;
use infrastructure_common::Component;

#[derive(Debug)]
#[component(singleton, priority = 5)]
struct OkService;

fn main() {
    let s = OkService;
    assert_eq!(s.name(), "OkService");
    assert_eq!(s.priority(), 5);
}
