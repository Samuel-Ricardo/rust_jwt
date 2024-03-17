use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::Response,
};
use jwt_lib::model::user::User;
use serde_json::json;

pub struct Auth(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = Response<String>;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let access_token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|str| str.split(" ").nth(1));

        match access_token {
            Some(token) => {
                let user = jwt_lib::decode_jwt(token);

                match user {
                    Ok(user) => Ok(Auth(user)),
                    Err(err) => Err(Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(
                            json!({
                              "success": false,
                              "data": {
                                "message": err
                              }
                            })
                            .to_string(),
                        )
                        .unwrap_or_default()),
                }
            }
            None => Err(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "application/json")
                .body(
                    json!({
                      "success": false,
                      "data": {
                        "message": "No token provided"
                      }
                    })
                    .to_string(),
                )
                .unwrap_or_default()),
        }
    }
}
