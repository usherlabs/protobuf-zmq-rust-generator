use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::FutureExt;
use crate::generated::test_service::{MyRequestInput, MyRequestResult, MyServerServiceHandlers, MyServerServiceServer, SubscriptionItem};

mod generated;

fn create_socket_dir_if_not_exists() {
    let path = SOCKETS_PATH;
    if !std::path::Path::new(path).exists() {
        std::fs::create_dir(path).expect("Failed to create socket dir");
    }
}

const SOCKETS_PATH: &str = "/tmp/test_sockets/";


// multi thread test
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn request_server_test() {
    create_socket_dir_if_not_exists();
    println!("Starting server");
    struct MyServerServiceHandlersImpl {}
    impl MyServerServiceHandlers for MyServerServiceHandlersImpl {
        fn my_request_method(&self, input: MyRequestInput) -> BoxFuture<Result<MyRequestResult, ()>> {
            // i32 to u64
            let time_to_sleep = input.time_to_sleep as u64;
            // return a ValidationResult of ok if string says ok, not good otherwise
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(time_to_sleep)).await;
                let id = format!("ok after {time_to_sleep}ms");
                println!("id: {}", id);
                Ok(MyRequestResult { message: id, all_ok: true })
            }
                .boxed()
        }
    }
    let reply_handlers = Arc::new(MyServerServiceHandlersImpl {});

    let prove_server = MyServerServiceServer::new(
        "/tmp/test_sockets/test_pub".to_string(),
        "/tmp/test_sockets/test_req".to_string(),
        reply_handlers,
    );
    prove_server.start_listening();

    println!("stopped?");
}

// single thread test
#[tokio::test(flavor = "current_thread")]
async fn publish_test() {
    create_socket_dir_if_not_exists();
    println!("Starting publisher");
    struct EmptyProverHandlersImpl {}
    impl MyServerServiceHandlers for EmptyProverHandlersImpl {}
    let reply_handlers = Arc::new(EmptyProverHandlersImpl {});
    let mut prove_server = MyServerServiceServer::new(
        "/tmp/test_sockets/test_pub".to_string(),
        "/tmp/test_sockets/test_req".to_string(),
        reply_handlers,
    );

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        println!("Publishing message...");

        prove_server
            .publish_to_items(SubscriptionItem {
                data: "any_data".to_string(),
            })
            .expect("Failed to publish");
    }
}
