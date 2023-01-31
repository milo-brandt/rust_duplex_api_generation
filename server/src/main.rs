use axum::{Router, routing::post, Json, Server};
use serde::{Serialize, Deserialize};
use tower_http::{cors::{CorsLayer, Any}, catch_panic::CatchPanicLayer};
use std::net::{Ipv4Addr, SocketAddr, IpAddr};

#[tokio::main]
async fn main() {
   let cors = CorsLayer::new()
      .allow_methods(Any)
      .allow_origin(Any)
      .allow_headers(Any);

   let app = Router::new()
      .route("/echo", post(echo))
      .layer(CatchPanicLayer::new())
      .layer(cors);
   
   Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000))
      .serve(app.into_make_service())
      .await
      .unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
struct X {
   value: u64
}

async fn echo(
   message: Json<X>,
) -> Json<X> {
   println!("CALLED WITH {:?}", message);
   Json(X { value: message.value * 5 })
} 