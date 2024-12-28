use std::io::Read;
use wasmcloud_component::{
    http::{self, ErrorCode},
    wasi::http::outgoing_handler,
    wasi::http::types::*,
};

#[derive(serde::Deserialize)]
struct DogResponse {
    message: String,
}

struct DogFetcher;

impl http::Server for DogFetcher {
    fn handle(
        _request: http::Request<http::IncomingBody>,
    ) -> http::Result<http::Response<impl http::OutgoingBody>> {
        // Build request to dog.ceo
        let req = OutgoingRequest::new(Fields::new());
        req.set_scheme(Some(&Scheme::Https))
            .map_err(|_| ErrorCode::InternalError(Some("Failed to set scheme".into())))?;
        req.set_authority(Some("dog.ceo"))
            .map_err(|_| ErrorCode::InternalError(Some("Failed to set authority".into())))?;
        req.set_path_with_query(Some("/api/breeds/image/random"))
            .map_err(|_| ErrorCode::InternalError(Some("Failed to set path".into())))?;

        // Make the API call
        match outgoing_handler::handle(req, None) {
            Ok(resp) => {
                resp.subscribe().block();
                let response = resp
                    .get()
                    .ok_or_else(|| ErrorCode::InternalError(Some("Response missing".into())))?
                    .map_err(|_| {
                        ErrorCode::InternalError(Some("Response already consumed".into()))
                    })?
                    .map_err(|e| {
                        ErrorCode::InternalError(Some(format!("Request failed: {}", e)))
                    })?;

                if response.status() == 200 {
                    let response_body = response.consume().map_err(|_| {
                        ErrorCode::InternalError(Some("Failed to get response body".into()))
                    })?;

                    let body = {
                        let mut buf = vec![];
                        let mut stream = response_body.stream().map_err(|_| {
                            ErrorCode::InternalError(Some("Failed to get response stream".into()))
                        })?;
                        stream.read_to_end(&mut buf).map_err(|_| {
                            ErrorCode::InternalError(Some("Failed to read response".into()))
                        })?;
                        buf
                    };

                    let _trailers = IncomingBody::finish(response_body);
                    let dog_response: DogResponse =
                        serde_json::from_slice(&body).map_err(|_| {
                            ErrorCode::InternalError(Some("Failed to parse JSON".into()))
                        })?;

                    http::Response::builder()
                        .status(200)
                        .body(dog_response.message)
                        .map_err(|_| {
                            ErrorCode::InternalError(Some("Failed to build response".into()))
                        })
                } else {
                    http::Response::builder()
                        .status(response.status())
                        .body(format!(
                            "HTTP request failed with status code {}",
                            response.status()
                        ))
                        .map_err(|_| {
                            ErrorCode::InternalError(Some("Failed to build error response".into()))
                        })
                }
            }
            Err(e) => http::Response::builder()
                .status(500)
                .body(format!("Error fetching dog: {}", e))
                .map_err(|_| {
                    ErrorCode::InternalError(Some("Failed to build error response".into()))
                }),
        }
    }
}

http::export!(DogFetcher);
