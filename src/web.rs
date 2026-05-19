use embassy_net::Stack;
use embassy_time::Duration;
use esp_println::println;
use picoserve::{
    AppBuilder, AppRouter, Router,
    extract::Form,
    response::{File, IntoResponse},
    routing::{self, PathRouter, get, post},
};

use crate::{ReaderOperation, STATE};

pub const WEB_TASK_POOL_SIZE: usize = 2;

pub struct WebApp {
    pub router: &'static Router<<Application as AppBuilder>::PathRouter>,
    pub config: &'static picoserve::Config<Duration>,
}

pub struct Application {}

impl AppBuilder for Application {
    type PathRouter = impl PathRouter;

    fn build_app(self) -> Router<Self::PathRouter> {
        let router = Router::new();

        // Website
        let router = router
            .route(
                "/",
                routing::get_service(File::html(include_str!("website/index.html"))),
            )
            .route(
                "/css/style.css",
                routing::get_service(File::css(include_str!("website/css/style.css"))),
            )
            .route(
                "/js/main.js",
                routing::get_service(File::css(include_str!("website/js/main.js"))),
            );

        // API
        let router = router
            .route("/api/reader", get(get_state))
            .route("/api/reader/activate", post(activate_reader))
            .route("/api/reader/deactivate", post(deactivate_reader))
            .route("/api/reader/operation", post(set_operation));

        router
    }
}

async fn set_operation(Form(operation): Form<ReaderOperation>) -> impl IntoResponse {
    STATE.lock(|state| state.reader_operation = operation).await;
}

async fn get_state() -> impl IntoResponse {
    STATE
        .lock(|state| picoserve::response::Json(state.clone()))
        .await
}

async fn activate_reader() -> impl IntoResponse {
    STATE.lock(|state| state.reader_active = true).await;
}

async fn deactivate_reader() -> impl IntoResponse {
    STATE.lock(|state| state.reader_active = false).await;
}

impl Default for WebApp {
    fn default() -> Self {
        let router = picoserve::make_static!(
            AppRouter<Application>,
            Application::build_app(Application {})
        );

        let config = picoserve::make_static!(
            picoserve::Config<Duration>,
            picoserve::Config::new(picoserve::Timeouts {
                start_read_request: Some(Duration::from_secs(5)),
                read_request: Some(Duration::from_secs(1)),
                write: Some(Duration::from_secs(1)),
                persistent_start_read_request: Some(Duration::from_secs(1)),
            })
            .keep_connection_alive()
        );

        Self { router, config }
    }
}

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
pub async fn web_task(
    task_id: usize,
    stack: Stack<'static>,
    router: &'static AppRouter<Application>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    println!("trace?");
    log::trace!("server task");

    picoserve::Server::new(&router, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}
