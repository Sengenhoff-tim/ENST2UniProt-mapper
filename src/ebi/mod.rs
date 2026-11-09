//! EBI Proteins API: variation model, fetch, and conversion to `Variant`.

mod convert;
mod fetch;
mod log;
mod source_type;
mod types;

pub(crate) use fetch::{fetch_canon_variations, fetch_iso_variations, EBI_MAX_REQUESTS_PER_SECOND};
