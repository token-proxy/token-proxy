use axum::body::Body;
use axum::http::{header, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "dist/"]
struct FrontendAssets;

/// SPA 前端静态文件服务 + fallback
///
/// 先尝试精确匹配请求路径对应的静态文件；未命中时回退到 index.html，
/// 由 react-router-dom 在前端处理路由。
pub async fn serve_frontend(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if let Some(content) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(content.data))
            .unwrap();
    }

    match FrontendAssets::get("index.html") {
        Some(content) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(content.data))
            .unwrap(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}
