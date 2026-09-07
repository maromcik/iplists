use axum::http;
use axum::http::Request;

fn unquote(val: &str) -> &str {
    val.trim().trim_start_matches('"').trim_end_matches('"')
}

fn bare_address(val: &str) -> &str {
    if val.starts_with('[') {
        val.split("]:")
            .next()
            .map(|s| s.trim_start_matches('[').trim_end_matches(']'))
            .unwrap_or(val)
    } else {
        val.split(':').next().unwrap_or(val)
    }
}

pub fn real_ip_remote_addr<'a>(req: &'a Request<impl Sized>) -> Option<&'a str> {
    req.headers()
        .get_all(http::header::FORWARDED)
        .into_iter()
        .filter_map(|hdr| hdr.to_str().ok())
        .flat_map(|val| val.split(';'))
        .flat_map(|vals| vals.split(','))
        .filter_map(|pair| pair.trim().split_once('='))
        .find_map(|(name, val)| {
            name.trim()
                .eq_ignore_ascii_case("for")
                .then(|| bare_address(unquote(val)))
        })
        .or_else(|| {
            req.headers()
                .get("x-forwarded-for")?
                .to_str()
                .ok()?
                .split(',')
                .next()
                .map(str::trim)
                .filter(|val| !val.is_empty())
        })
}
