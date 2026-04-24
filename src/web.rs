use alloc::string::String;
use embassy_net::Stack;
use embassy_time::Duration;
use esp_println::println;
use picoserve::{AppRouter, AppWithStateBuilder, Router, response::File, routing::{self, get, get_service}};

pub const WEB_TASK_POOL_SIZE: usize = 2;

#[derive(serde::Deserialize)]
struct PrintParams {
    text: String
}

pub struct Application;

impl AppWithStateBuilder for Application {
    type State = ();

    type PathRouter = impl routing::PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        println!("build");
        picoserve::Router::new()
            .route("/", routing::get_service(File::html(include_str!("website/index.html"))))
            .route("/css/style.css", routing::get_service(File::css(include_str!("website/css/style.css"))))
            .route("/js/main.js", routing::get_service(File::css(include_str!("website/js/main.js"))))
            .route("/print", get(async |picoserve::extract::Query(PrintParams { text} )| {
                println!("{}", text);
                
                picoserve::response::DebugValue(("text", text))
            }))
            .route("/uids", get(async || {
                json::array!["a", "bc"]
            }))
    }
}

pub struct WebApp {
    pub router: &'static Router<<Application as AppWithStateBuilder>::PathRouter>,
    pub config: &'static picoserve::Config<Duration>,
}

impl Default for WebApp {
    fn default() -> Self {
        let router = picoserve::make_static!(AppRouter<Application>, Application.build_app());

        let config = picoserve::make_static!(picoserve::Config<Duration>, picoserve::Config::new(picoserve::Timeouts {
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
    config: &'static picoserve::Config<Duration>
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    println!("trace?");
    log::trace!("server task");

    picoserve::Server::new(router, config, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never() 
}