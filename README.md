# protobuf-zqm-rust-generator

This crate works with [`prost`](https://github.com/tokio-rs/prost) to develop a service generator for a ZMQ + Protobuf implementation.

## How to Use

1. Create a `build.rs` file in your project's root. Look
   to [prost-build documentation](https://docs.rs/prost-build/latest/prost_build/) for detailed instructions.
2. Utilize our service generator during the build process:
     ```rust
        prost_build::Config::new()
        // Optional: defaults to $OUT, can be changed for autocomplete support
        .out_dir(out_dir)
        .service_generator(Box::new(ZmqServerGenerator {})) // here
        .compile_protos(& ["your_proto_file.proto"], & ["your/proto/location/"])
     ```
3. The generator will create a `your_proto_file.rs` in the `out_dir`, containing the generated code.

Now to operate a service server:

- Import the `[Method]Handler` trait and implement it for responses.
- For data publication, use the resulting server's `publish` method for each `pubsub` method in your `.proto` file.

Check our test files for comprehensive examples.

## Requirements

- Aim: Enable inter-process communication with minimal modifications when extending the API
- Provide type safety
- Support creation of a stream for data across different processes subscribed to a specific topic.
- Enable easy creation of asynchronous request-reply tasks across processes.

Given these, we have 2 patterns in operation:

### 1. Pub/Sub Pattern

- A PUBLISHER application binds to a socket. Any number of SUBSCRIBER applications can connect.
- For communication, the ZMQ frame protocol should be: `[methodName, Output]`, in bytes
    ```proto
    message EmptyInput {}

    message SubscriptionItem {
      string data = 1;
    }

    service MyServerService {
      rpc SubscribeToItems(EmptyInput) returns (stream SubscriptionItem) {}
    }
    ```

The data transferred should be `["SubscribeToItems", SubscriptionItem]`.

- Pub-sub methods should start with "SubscribeTo...".
- Clients can subscribe and filter events using the `methodName` message.
- The `.proto` file defined return type should be a data stream.

### 2. Request/Reply Pattern

- ROUTER/DEALER sockets are used to allow asynchronous requests.
- A server should handle multiple requests concurrently.
- The ZMQ frame protocol should be: `[requestId, BLANK, methodName, Input]`, in bytes. The server should reply
  with `[clientId, requestId, Output]`
    ```proto
    message MyRequestInput {
      int32 time_to_sleep = 1;
    }

    message MyRequestResult {
      bool all_ok = 1;
      string message = 2;
    }

    service MyServerService {
        rpc MyRequestMethod(MyRequestInput) returns (MyRequestResult) {}
    }
    ```
  The transferred data for this example should be `[requestId, BLANK, "MyRequestMethod", MyRequestInput]`.
    - `requestId` is a randomly generated string by the client
    - `BLANK` is an empty frame, used to mimic the original protocol for REQUEST/REPLY patterns.
    - `clientId` is included by default by clients. ROUTER should also include this in the reply to ensure the correct
      dispatching of the reply to a client.

It's possible to use both patterns on the same service:
Note: Currently, we only support building Server implementations with this package. Future updates may include client
implementations.

## Contributing

We welcome contributions to this project!