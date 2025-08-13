use component_macros::component;
use infrastructure_common::Component; // trait brought into scope to call generated methods

#[derive(Debug)]
#[component(singleton)]
struct OkService;

fn main() {
    // Ensure the macro generated impl provides the name method without manual impl conflicts
    let s = OkService;
    assert_eq!(s.name(), "OkService");
}
