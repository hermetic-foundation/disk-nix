#![recursion_limit = "512"]

include!("sections/entry.rs");
include!("sections/run.rs");
include!("sections/schema.rs");
include!("sections/migrations.rs");
include!("sections/install.rs");
include!("sections/apply_io.rs");
include!("sections/graph_json.rs");
include!("sections/probe_status.rs");
include!("sections/list_renderers.rs");
include!("sections/inspect_plan_renderers.rs");
include!("sections/predicates.rs");
include!("sections/usage_details.rs");
include!("sections/display_helpers.rs");

#[cfg(test)]
mod tests;
