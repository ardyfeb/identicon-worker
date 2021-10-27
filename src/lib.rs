use serde::{Serialize, Deserialize};
use serde_qs::{Error};
use identicon_rs::Identicon;
use console_error_panic_hook::set_once as set_panic_hook;

use worker::*;


#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct OptionsQuery {
    hash: String,
    border: u32,
    size: u32,
    scale: u32,
    format: OptionsFormat
}

impl Default for OptionsQuery {
    fn default() -> Self {
        Self {
            hash: String::new(),
            border: 50,
            size: 5,
            scale: 500,
            format: Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum OptionsFormat {
    #[serde(rename = "png")]
    PNG,
    #[serde(rename = "jpeg")]
    JPG
}

impl Default for OptionsFormat {
    fn default() -> Self {
        Self::PNG
    }
}


fn log_request(req: &Request) {
    console_log!(
        "{} {}, located at: {:?}, within: {}",
        req.method().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

fn parse_query(raw: &str) -> std::result::Result<OptionsQuery, Error> {
    serde_qs::from_str(raw)
}

const CORS_HEADERS: [(&str, &str); 2] = [
    ("Access-Control-Allow-Origin",  "*"), 
    ("Access-Control-Allow-Methods", "GET,HEAD,OPTIONS")
];

const GUIDE_TEXT: &str = r#"
Identicon generator
---

Options:
    - hash: random string for generating identicon (required)
    - scale: pattern scale <number> (default: 500)
    - size: pattern seed size, should not larger than scale <number> (default: 5)
    - border: edge border <number> (default: 50)
    - format: image format <png/jpeg> (default: png)

Example: https://ic.ardyfeb.me/?hash=magic&format=png

Repository: https://github.com/ardyfeb/identicon-worker
"#;

#[event(fetch)]
pub async fn main(req: Request, _env: Env) -> Result<Response> {
    log_request(&req);
    set_panic_hook();

    if !matches!(req.method(), Method::Get) {
        return Response::error("Method not allowed", 405)
    }

    let url = req.url().unwrap();

    if !url.path().eq("/") {
        return Response::error("Not Found", 404);
    }

    let query = url.query().unwrap_or("");

    match parse_query(query) {
        Ok(params) => {
            if params.hash.is_empty() {
                return Response::ok(GUIDE_TEXT);
            }

            if params.size > params.scale {
                return Response::error(
                    "Un-processable Entity: size should not larger than scale", 422
                );
            }

            let mut icon = Identicon::new(&params.hash);

            icon = icon.scale(params.scale).unwrap();
            icon = icon.border(params.border);
            icon = icon.size(params.size).unwrap();

            // TODO: allow set from query params
            icon = icon.background_color((240, 240, 240));

            let (buff, mime) = match params.format {
                OptionsFormat::PNG => (icon.export_jpeg_data(), "image/png"),
                OptionsFormat::JPG => (icon.export_jpeg_data(), "image/jpeg")
            };
            let mut headers = Headers::new();

            headers.set("Content-type", mime).unwrap();
            headers.set("Cache-Control", "public,max-age=36000").unwrap();

            // apply cors headers
            for (key, value) in CORS_HEADERS {
                headers.set(key, value).unwrap();
            }

            Ok(Response::from_bytes(buff.unwrap())?.with_headers(headers))
        },
        Err(_) => Response::error("Bad request", 400)
    }
}
