use component_macros::{lifecycle};

// Intentionally incorrect: applying lifecycle without required struct definition context
#[lifecycle(on_start = "start")] // expect failure because no struct follows
fn dummy() {}

fn main() {}
