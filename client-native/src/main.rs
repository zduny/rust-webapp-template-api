use anyhow::Result;
use clap::Parser;
use futures::{select, FutureExt, StreamExt};
use kodec::binary::Codec;
use lazy_static::lazy_static;
use mezzenger_websocket::Transport;
use regex::{Captures, Regex};
use rustyline_async::{Readline, ReadlineEvent, SharedWriter};
use std::io::Write;
use tokio::spawn;
use tokio_tungstenite::connect_async;
use url::Url;

use common::api::chat::{Api, Consumer};
use zzrpc::consumer::{Configuration, Consume};

lazy_static! {
    static ref FIBONACCI_PATTERN: Regex = Regex::new(r"^fibonacci\((0|[1-9][0-9]*)\)$").unwrap();
    static ref FACTORIAL_PATTERN: Regex = Regex::new(r"^(0|[1-9][0-9]*)!$").unwrap();
}

/// Web app native client
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server URL.
    #[arg(short, long, default_value = "ws://localhost:8080/ws")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Hello.");

    println!("Connecting to server...");
    let url = Url::parse(&args.url)?;
    let (web_socket, _) = connect_async(url).await?;
    let codec = Codec::default();
    let transport = Transport::new(web_socket, codec);
    let consumer = Consumer::consume(transport, Configuration::default());
    println!("Connected.");

    let user_name = consumer.user_name().await.unwrap();
    let connected_user_names = consumer.user_names().await.unwrap();
    println!("Your name: <{user_name}>.");

    if !connected_user_names.is_empty() {
        println!(
            "Other connected users: {}.",
            connected_user_names
                .iter()
                .map(|user| format!("<{user}>"))
                .collect::<Vec<String>>()
                .join(", ")
        );
    }

    let (mut readline, mut stdout) = Readline::new("> ".to_string())?;

    writeln!(
        stdout,
        "Type 'fibonacci(n)' to calculate n-th element of Fibonacci sequence."
    )?;
    writeln!(stdout, "Type 'n!' to calculate factorial on n.")?;

    {
        let mut messages = consumer.messages().await.unwrap();
        let mut connected = consumer.connected().await.unwrap();
        let mut disconnected = consumer.disconnected().await.unwrap();

        loop {
            select! {
                message = messages.next() => {
                    if let Some((user_name, message)) = message {
                        writeln!(stdout, "<{user_name}> {message}")?;
                    } else {
                        writeln!(stdout, "Server disconnected.")?;
                        writeln!(stdout, "Exiting...")?;
                        break;
                    }
                },
                connected = connected.next() => {
                    if let Some(user_name) = connected {
                        writeln!(stdout, "New user connected: <{user_name}>.")?;
                    }
                },
                disconnected = disconnected.next() => {
                    if let Some(user_name) = disconnected {
                        writeln!(stdout, "User <{user_name}> left.")?;
                    }
                },
                command = readline.readline().fuse() => match command {
                    Ok(event) => {
                        match event {
                            ReadlineEvent::Line(line) => {
                                let line = line.trim();
                                if let Some(captures) = FIBONACCI_PATTERN.captures(line) {
                                    if let Some(number) = extract_number(&captures) {
                                        let stdout_clone = stdout.clone();
                                        spawn(async move { handle_fibonacci(stdout_clone, number).await.unwrap() });
                                        readline.add_history_entry(line.to_string());
                                    } else {
                                        let input = captures.get(1).unwrap().as_str();
                                        writeln!(stdout, "Error: {input} is not a non-negative integer.")?;
                                    }
                                } else if let Some(captures) = FACTORIAL_PATTERN.captures(line) {
                                    if let Some(number) = extract_number(&captures) {
                                        let stdout_clone = stdout.clone();
                                        spawn(async move { handle_factorial(stdout_clone, number).await.unwrap(); });
                                        readline.add_history_entry(line.to_string());
                                    } else {
                                        let input = captures.get(1).unwrap().as_str();
                                        writeln!(stdout, "Error: {input} is not a non-negative integer.")?;
                                    }
                                } else {
                                    consumer.message(line.to_string()).await.unwrap();
                                }
                            }
                            ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                                writeln!(stdout, "Exiting...")?;
                                break;
                            },
                        }
                    },
                    Err(error) => {
                        writeln!(stdout, "Error occurred while handling command: {error}")?;
                        writeln!(stdout, "Exiting...")?;
                        break;
                    },
                },
            }
        }
    }

    readline.flush()?;

    Ok(())
}

fn extract_number<'a>(captures: &'a Captures<'a>) -> Option<u64> {
    captures
        .get(1)
        .and_then(|capture| capture.as_str().parse::<u64>().ok())
}

async fn handle_fibonacci(mut stdout: SharedWriter, number: u64) -> Result<()> {
    writeln!(stdout, "Calculating fibonacci({number})...")?;
    let result = tokio_rayon::spawn(move || common::fibonacci(number)).await;
    writeln!(stdout, "fibonacci({number}) = {result}")?;
    Ok(())
}

async fn handle_factorial(mut stdout: SharedWriter, number: u64) -> Result<()> {
    writeln!(stdout, "Calculating {number}!...")?;
    let result = tokio_rayon::spawn(move || common::factorial(number)).await;
    writeln!(stdout, "{number}! = {result}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{extract_number, FACTORIAL_PATTERN, FIBONACCI_PATTERN};

    #[test]
    fn test_fibonacci_pattern() {
        let captures = FIBONACCI_PATTERN.captures("fibonacci(123)").unwrap();
        let number = extract_number(&captures).unwrap();
        assert_eq!(number, 123);
    }

    #[test]
    fn test_factorial_pattern() {
        let captures = FACTORIAL_PATTERN.captures("0!").unwrap();
        let number = extract_number(&captures).unwrap();
        assert_eq!(number, 0);
    }
}
