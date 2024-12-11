use super::jwt::{Claims, JwtMiddleware};
use actix_web::test::{self, TestRequest};
use actix_web::{web, App, HttpResponse};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[actix_web::test]
async fn test_valid_token() {
    std::env::set_var("JWT_SECRET", "test_secret");

    let claims = Claims {
        exp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("test_secret".as_bytes()),
    )
    .unwrap();

    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware)
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_invalid_token() {
    std::env::set_var("JWT_SECRET", "test_secret");

    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware)
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();

    let err = test::try_call_service(&app, req).await.unwrap_err();
    let resp = err.error_response();
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
async fn test_missing_auth_header() {
    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware)
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get().uri("/").to_request();
    let err = test::try_call_service(&app, req).await.unwrap_err();
    let resp = err.error_response();
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
async fn test_missing_jwt_secret() {
    std::env::remove_var("JWT_SECRET");

    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware)
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/")
        .insert_header(("Authorization", "Bearer some_token"))
        .to_request();

    let err = test::try_call_service(&app, req).await.unwrap_err();
    let resp = err.error_response();
    assert_eq!(resp.status().as_u16(), 500);
}
