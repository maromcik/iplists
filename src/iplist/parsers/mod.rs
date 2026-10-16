//! Provider-specific parsers for the geo-IP / ASN data pipeline.
//!
//! Each module in this directory adapts one external data source to the
//! crate's generic model. The currently supported provider is
//! [`maxmind`] (GeoLite2 CSV archives).
//!
//! ## Adding a new parser (data source)
//!
//! A parser plugs into three narrow seams. Using a hypothetical provider as
//! an example:
//!
//! 1. **Create `src/iplist/parsers/<provider>.rs`** and declare it here with
//!    `pub mod <provider>;`.
//!
//! 2. **Implement [`GeoDataParser`](crate::iplist::fetch::GeoDataParser)**
//!    for a wrapper over the downloaded bytes (e.g. `struct MyGeoData { body: Vec<u8> }`).
//!    `parse::<T>(name)` must return the deserialized rows of the *source
//!    file* `name` inside the download. The wrapper is constructed from
//!    raw bytes, so both the download path and the disk-cache path
//!    (see [`Downloader`](crate::iplist::fetch::Downloader) /
//!    [`Loader`](crate::iplist::fetch::Loader)) work unchanged. MaxMind
//!    implements this over a ZIP of CSVs; another provider could parse a
//!    plain CSV/JSON download instead. `Loader` enforces the
//!    `max_age` freshness limit, and `Downloader` expands
//!    `{year}`/`{month}`/`{day}` placeholders in the configured URIs, so
//!    usually no extra fetching code is needed.
//!
//! 3. **Implement [`LocationParser`](crate::iplist::iprange::LocationParser)
//!    and [`AsnParser`](crate::iplist::iprange::AsnParser)** on a unit
//!    struct (e.g. `pub struct MyParser;`). Map the provider's row types
//!    (via `From` impls) into [`Location`](crate::iplist::iprange::Location),
//!    [`IpLocationRange`](crate::iplist::iprange::IpLocationRange) and
//!    [`IpAsnRange`](crate::iplist::iprange::IpAsnRange). A cached-archive
//!    helper like MaxMind's `load_or_download` keeps each trait method
//!    from re-downloading within one refresh run.
//!
//! 4. **Wire it up** in `src/main.rs` and `src/list.rs`: call
//!    `update_ranges::<MyParser>(...)` instead of
//!    `update_ranges::<MaxMindParser>(...)` (the scheduler job in
//!    `schedule_tasks` uses the same type parameter). Everything else —
//!    indexing, tries, HTTP handlers, file generation, status reporting —
//!    is generic over the parser.
//!
//! ## Adding a new formatter (output format)
//!
//! Output formats are independent of parsers. To add one, extend
//! [`OutputFormat`](crate::iplist::formatter::OutputFormat) in
//! `src/iplist/formatter.rs` (variant + rendering match arm + HTTP
//! `Content-Type` in `IntoResponse`). See that module's docs for the exact
//! steps.

pub mod maxmind;
