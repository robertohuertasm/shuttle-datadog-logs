# shuttle-datadog-logs

[![Rust](static/shuttle-datadog.png)](https://robertohuertas.com/2023/09/30/shuttle-datadog-logs)

This project shows how to use a [Shuttle](https://shuttle.rs)-powered [Axum](https://github.com/tokio-rs/axum) REST API that makes use of [Datadog](https://docs.datadoghq.com/logs/log_collection/) to manage logs.

We will be using the [dd-tracing-layer](https://crates.io/crates/dd-tracing-layer) crate to send the logs to [Datadog](https://docs.datadoghq.com).

## Blog Post

This project is a companion to the blog post: [Send the logs of your Shuttle-powered backend to Datadog](https://robertohuertas.com/2023/09/30/shuttle-datadog-logs) blog post.

Also available in [dev.to](https://dev.to/robertohuertasm/send-the-logs-of-your-shuttle-powered-backend-to-datadog-3imo) and [Medium](https://robertohuertasm.medium.com/send-the-logs-of-your-shuttle-powered-backend-to-datadog-9508dab9dc71).

## Endpoints

It exposes one endpoint:

- `GET /` - Returns a `200` status code with a `Hello, World!` message.


## Live demo

The project has been deployed with [Shuttle](https://shuttle.rs). You can try it out by visiting the following URL: [https://shuttle-datadog-logs.shuttleapp.rs](https://shuttle-datadog-logs.shuttleapp.rs/).
