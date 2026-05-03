use alloc::string::String;
use embassy_net::Stack;
use embassy_sync::blocking_mutex::{Mutex, raw::CriticalSectionRawMutex};
use embassy_time::Duration;
use esp_println::println;
use picoserve::{
    AppRouter, AppWithStateBuilder, Router,
    response::File,
    routing::{self, PathRouter, get, get_service},
};

use crate::{STATE, State};

pub const WEB_TASK_POOL_SIZE: usize = 2;

pub struct WebState {
    pub state: &'static Mutex<CriticalSectionRawMutex, State>,
}

#[derive(serde::Deserialize)]
struct PrintParams {
    text: String,
}

pub struct WebApp {
    pub router: &'static Router<
        <Application as AppWithStateBuilder>::PathRouter,
        <Application as AppWithStateBuilder>::State,
    >,
    pub config: &'static picoserve::Config<Duration>,
}

struct Application {}

impl AppWithStateBuilder for Application {
    type State = WebState;

    type PathRouter = impl PathRouter<Self::State>;

    fn build_app(self) -> Router<Self::PathRouter, Self::State> {
        Router::new()
            .with_state(WebState { state: &STATE })
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
            )
    }
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

    panic!()

    //picoserve::Server::new(router, config, &mut http_buffer)
    //    .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
    //    .await
    //   .into_never()
}
