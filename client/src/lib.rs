use std::rc::Rc;

use common::api::{self, chat::Api as ChatApi, worker::Api as WorkerApi};
use futures::StreamExt;
use kodec::binary::Codec;
use wasm_bindgen::{prelude::*, JsCast};

use js_utils::{console_log, document, event::When, set_panic_hook, spawn, window};
use web_sys::{
    HtmlInputElement, HtmlTextAreaElement, KeyboardEvent, MouseEvent, WebSocket, Worker,
};

use zzrpc::consumer::{Configuration, Consume};

#[wasm_bindgen(start)]
pub async fn main_client() -> Result<(), JsValue> {
    set_panic_hook();

    console_log!("Hello World!");

    let document = document();

    let text_area = Rc::new(
        document
            .get_element_by_id("text")
            .unwrap()
            .dyn_into::<HtmlTextAreaElement>()
            .unwrap(),
    );

    let write_line = move |line: &str| {
        let mut value = text_area.value();
        value += line;
        value += "\n";
        text_area.set_value(&value);
        text_area.set_scroll_top(text_area.scroll_height());
    };

    write_line("Hello.");

    // setting up web socket
    write_line("Connecting to server...");
    let host = window()
        .location()
        .host()
        .expect("couldn't extract host from location");
    let url = format!("ws://{host}/ws");
    let web_socket = Rc::new(WebSocket::new(&url).unwrap());
    let transport = mezzenger_websocket::Transport::new(&web_socket, Codec::default())
        .await
        .unwrap();
    let chat_consumer = api::chat::Consumer::consume(transport, Configuration::default());
    let chat_consumer = Rc::new(chat_consumer);
    write_line("Connected.");

    // setting up worker
    let worker = Rc::new(Worker::new("./worker.js").unwrap());
    let transport = mezzenger_webworker::Transport::new(&worker, Codec::default())
        .await
        .unwrap();
    let worker_consumer = api::worker::Consumer::consume(transport, Configuration::default());
    let worker_consumer = Rc::new(worker_consumer);

    // setting up event handlers
    let input = Rc::new(
        document
            .get_element_by_id("input")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap(),
    );

    let number = Rc::new(
        document
            .get_element_by_id("number")
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap(),
    );

    let input_clone = input.clone();
    let chat_consumer_clone = chat_consumer.clone();
    let write_line_clone = write_line.clone();
    let send = move || {
        let text = input_clone.value().trim().to_string();
        let chat_consumer = chat_consumer_clone.clone();
        let write_line_clone = write_line_clone.clone();
        spawn(async move {
            let _ = chat_consumer.message(text).await.map_err(|error| {
                write_line_clone(&format!("Error occurred while sending message: {error}."));
            });
        });
        input_clone.set_value("");
    };

    let send_clone = send.clone();
    let _input_handler = input
        .when("keyup", move |event: KeyboardEvent| {
            if event.key() == "Enter" {
                send_clone();
            }
        })
        .unwrap();

    let _send_handler = Rc::new(document.get_element_by_id("send").unwrap())
        .when("click", move |_event: MouseEvent| {
            send();
        })
        .unwrap();

    let number_clone = number.clone();
    let write_line_clone = write_line.clone();
    let get_input = move || {
        let value = number_clone.value();
        let input = value.parse::<u64>();
        if input.is_err() {
            write_line_clone(&format!("Error: {value} is not a non-negative integer."));
        }
        input.ok()
    };

    let get_input_clone = get_input.clone();
    let write_line_clone = write_line.clone();
    let worker_consumer_clone = worker_consumer.clone();
    let _fibonacci_handler = Rc::new(document.get_element_by_id("fibonacci").unwrap())
        .when("click", move |_event: MouseEvent| {
            if let Some(input) = get_input_clone() {
                write_line_clone(&format!("Calculating fibonacci({input})..."));
                let worker_consumer_clone = worker_consumer_clone.clone();
                let write_line_clone = write_line_clone.clone();
                spawn(async move {
                    match worker_consumer_clone.fibonacci(input).await {
                        Ok(result) => {
                            write_line_clone(&format!("fibonacci({input}) = {result}"));
                        }
                        Err(error) => {
                            write_line_clone(&format!(
                                "Error occurred while sending message to worker: {error}."
                            ));
                        }
                    }
                });
            }
        })
        .unwrap();

    let write_line_clone = write_line.clone();
    let _factorial_handler = Rc::new(document.get_element_by_id("factorial").unwrap())
        .when("click", move |_event: MouseEvent| {
            if let Some(input) = get_input() {
                let worker_consumer_clone = worker_consumer.clone();
                let write_line_clone = write_line_clone.clone();
                write_line_clone(&format!("Calculating {input}!..."));
                spawn(async move {
                    match worker_consumer_clone.factorial(input).await {
                        Ok(result) => {
                            write_line_clone(&format!("{input}! = {result}"));
                        }
                        Err(error) => {
                            write_line_clone(&format!(
                                "Error occurred while sending message to worker: {error}."
                            ));
                        }
                    }
                });
            }
        })
        .unwrap();

    // handle server messages
    let user_name = chat_consumer.user_name().await.unwrap();
    let connected_user_names = chat_consumer.user_names().await.unwrap();
    write_line(&format!("Your name: <{user_name}>."));
    if !connected_user_names.is_empty() {
        write_line(&format!(
            "Other connected users: {}.",
            connected_user_names
                .iter()
                .map(|user| format!("<{user}>"))
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }

    let write_line_clone = write_line.clone();
    let mut connected = chat_consumer.connected().await.unwrap();
    spawn(async move {
        while let Some(user_name) = connected.next().await {
            write_line_clone(&format!("New user connected: <{user_name}>."));
        }
    });

    let write_line_clone = write_line.clone();
    let mut disconnected = chat_consumer.disconnected().await.unwrap();
    spawn(async move {
        while let Some(user_name) = disconnected.next().await {
            write_line_clone(&format!("User <{user_name}> left."));
        }
    });

    let mut messages = chat_consumer.messages().await.unwrap();
    while let Some((user_name, message)) = messages.next().await {
        write_line(&format!("<{user_name}> {message}"));
    }

    write_line("Server disconnected.");

    Ok(())
}

#[wasm_bindgen]
pub fn add_numbers(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    use wasm_bindgen_test::wasm_bindgen_test;

    use crate::add_numbers;

    #[wasm_bindgen_test]
    fn test_add_numbers() {
        assert_eq!(5, add_numbers(2, 3));
    }
}
