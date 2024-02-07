use prost_build::{Service, ServiceGenerator};

pub struct ZmqServerGenerator {}

impl ServiceGenerator for ZmqServerGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        let name = &service.name;
        let methods = &service.methods;
        let pubsub_methods = filter_methods_by_type(methods, MethodType::PubSub);
        let pubsub_methods_string = pubsub_methods
            .iter()
            .map(|method| get_pubsub_method_string(method))
            .collect::<Vec<_>>()
            .join("\n");

        let service_handlers_trait = get_service_handlers_trait(&service);

        let code = format!(
            r#"
            // ====== START DEFINITIONS FOR {name} ======
            pub struct {name}Server {{
                pub_socket: zmq::Socket,
                rep_socket: Arc<Mutex<zmq::Socket>>,
                reply_handlers: Arc<dyn {name}Handlers + Send + Sync>,
            }}
            impl {name}Server {{
                pub fn new(
                        // path where we'll bind the PUB socket
                        pubsub_path: String,
                        // path where we'll bind the ROUTER socket
                        reply_path: String,
                        // handlers for the server requests
                        reply_handlers: Arc<dyn {name}Handlers + Send + Sync>,
                ) -> Self {{
                    let pub_socket = create_socket(&pubsub_path, zmq::PUB);
                    let rep_socket = create_socket(&reply_path, zmq::ROUTER);
                    Self {{
                        pub_socket,
                        rep_socket: Arc::new(Mutex::new(rep_socket)),
                        reply_handlers,
                    }}
                }}
                /// Starts listening for requests
                pub fn start_listening(&self) {{
                    loop {{
                        let rep_socket = self.rep_socket.lock().unwrap();
                        let poll_result = rep_socket.poll(zmq::POLLIN, 0);
                        drop(rep_socket);

                        if poll_result.is_err() {{
                            continue;
                        }}
                        // pollin is of type i16, result is i32
                        if (poll_result.unwrap()) == 0 {{
                            sleep(std::time::Duration::from_millis(50));
                            continue;
                        }}
                        let message = match self.rep_socket.lock().unwrap().recv_multipart(0) {{
                            Ok(msg) => msg,
                            Err(_) => {{
                                continue;
                            }}
                        }};
                        // message envelop: [identity, blank, request_id, method_name, input]
                        if message.len() < 4 {{
                            continue;
                        }}
                        let identity = message[0].clone();
                        // blank
                        let request_id = message[2].clone();
                        let method_name_raw = message[3].clone();
                        let input = message[4].clone();

                        let method_name = String::from_utf8_lossy(&method_name_raw).to_string();

                        let handlers = self.reply_handlers.clone();
                        let rep_socket = self.rep_socket.clone();

                        task::spawn(async move {{
                            let mut response = Vec::new();
                            response.push(identity);
                            response.push(request_id);
                            if handlers.has_handler(&method_name) {{
                                let result = handlers.call_handler(&method_name, &input).await;
                                match result {{
                                    Ok(validation_result) => {{
                                        response.push(validation_result);
                                    }}
                                    Err(e) => {{
                                        response.push(e.encode_to_vec());
                                    }}
                                }}
                            }} else {{
                                // handle error
                                // TODO better manage errors
                                let not_found_error_msg = "Method not found";
                                response.push(not_found_error_msg.as_bytes().to_vec());
                            }}
                            rep_socket.lock().unwrap()
                                .send_multipart(response, 0).unwrap();
                        }});
                    }}
                }}
                fn publish_message<T: prost::Message>(&mut self, name: &str, data: T) -> zmq::Result<()> {{
                    let message = data.encode_to_vec();
                    let messages = vec![name.as_bytes(), &message];
                    self.pub_socket.send_multipart(messages, 0)
                }}
                {pubsub_methods_string}
            }}
            {service_handlers_trait}
            // ====== END DEFINITIONS FOR {name} ======
            "#
        );
        buf.push_str(&code);
    }

    fn finalize(&mut self, _buf: &mut String) {
        const IMPORTS_CODE: &str = r#"
/// --------------------------------------------------------------
/// This file was generated by `protobuf_zmq_rust_generator` crate
/// DO NOT MODIFY DIRECTLY
/// --------------------------------------------------------------
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use prost::Message;
use tokio::task;
use zmq::SocketType;

fn create_socket(path: &str, socket_type: SocketType) -> zmq::Socket {
    let context = zmq::Context::new();
    let socket = context.socket(socket_type).unwrap();
    let protocol = "ipc://";
    create_path_if_not_exists(path);
    let endpoint = format!("{}{}", protocol, path);
    socket.bind(&endpoint).unwrap();
    socket
}
fn create_path_if_not_exists(path_str: &str) {
    let path = std::path::Path::new(path_str);
    let path1 = path.parent().unwrap();
    if !path1.exists() {
        std::fs::create_dir_all(path1).unwrap();
    }
}
"#;

        _buf.insert_str(0, IMPORTS_CODE);
    }
}


fn get_pubsub_method_string(method: &prost_build::Method) -> String {
    let name = &method.name;
    let proto_name = &method.proto_name;
    let output_type = &method.output_type;

    // normal name is subscribe[rest] and now we want that to be publish[rest]
    let new_name = format!("publish{}", &name[9..]);

    format!(
        r#"
        pub fn {new_name}(&mut self, data: {output_type}) -> zmq::Result<()> {{
            self.publish_message("{proto_name}", data)
        }}
        "#,
    )
}

fn get_service_has_handler_fn_string(service: &Service) -> String {
    let methods = &service.methods;
    let reply_methods = filter_methods_by_type(methods, MethodType::RequestResponse);
    let methods_string = reply_methods
        .iter()
        .map(|method| format!(r#""{proto_name}" => true,"#, proto_name = method.proto_name, ))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"
        fn has_handler(&self, method_name: &str) -> bool {{
            match method_name {{
                {methods_string}
                _ => false,
            }}
        }}
        "#,
        methods_string = methods_string,
    )
}

fn filter_methods_by_type(
    methods: &Vec<prost_build::Method>,
    desired_type: MethodType,
) -> Vec<&prost_build::Method> {
    methods
        .iter()
        .filter(|&method| {
            let method_type = get_method_type(method);
            method_type == desired_type
        })
        .collect()
}

fn get_call_handler_fn_string(service: &Service) -> String {
    let methods = &service.methods;
    let reply_methods = filter_methods_by_type(methods, MethodType::RequestResponse);

    let matchers_string = reply_methods
        .iter()
        .map(|method| {
            format!(
                r#""{proto_name}" => {{
                let input = {input_type}::decode(encoded_input).unwrap();
                self.{name}(input)
                    .map_ok(|result| {{
                        result.encode_to_vec()
                    }})
                    .boxed()
            }}"#,
                name = method.name,
                proto_name = method.proto_name,
                input_type = method.input_type,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"
        fn call_handler(
            &self,
            method_name: &str,
            encoded_input: &[u8],
        ) -> BoxFuture<Result<Vec<u8>, ()>> {{
            match method_name {{
                {matchers_string}
                _ => async {{ Err(()) }}.boxed(),
            }}
        }}
        "#
    )
}

fn get_unimplemneted_methods_string(service: &Service) -> String {
    let methods = &service.methods;
    let reply_methods = filter_methods_by_type(methods, MethodType::RequestResponse);

    reply_methods
        .iter()
        .map(|method| {
            format!(
                r#"
            fn {name}(&self, _input: {input_type}) -> BoxFuture<Result<{output_type}, ()>> {{
                unimplemented!("Validate")
            }}
            "#,
                name = method.name,
                input_type = method.input_type,
                output_type = method.output_type,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_service_handlers_trait(service: &Service) -> String {
    let name = &service.name;
    let has_handler_fn_string = get_service_has_handler_fn_string(service);
    let call_handler_fn_string = get_call_handler_fn_string(service);
    let unimplemented_methods_string = get_unimplemneted_methods_string(service);

    format!(
        r#"
        // This will be used to implement the handlers for the server
        pub trait {name}Handlers {{
            {has_handler_fn_string}
            {call_handler_fn_string}
            {unimplemented_methods_string}
        }}
        "#,
    )
}

#[derive(PartialEq)]
enum MethodType {
    PubSub,
    RequestResponse,
}

fn get_method_type(method: &prost_build::Method) -> MethodType {
    /*
     * This still doesn't work. See https://github.com/tokio-rs/prost/pull/591
     * when merged, we'll be able to define based on options how to generate the code
     */

    // let options = &method.options;
    // let pubsub = options
    //     .iter()
    //     .find(|&option| option.identifier_value.clone().unwrap() == "pubsub");
    // match pubsub {
    //     Some(_) => MethodType::PubSub,
    //     None => MethodType::RequestResponse,
    // }

    // for now, let's get based on the name of the method
    // if starts with subscribe, it's pubsub; else, it's request/response
    let name = &method.name;
    if name.starts_with("subscribe") {
        MethodType::PubSub
    } else {
        MethodType::RequestResponse
    }
}
