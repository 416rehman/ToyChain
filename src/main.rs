mod common;
mod server;

use crate::common::Request;
use clap::{Arg, Command};

fn cli() -> Command {
    Command::new("Toychain")
        .about("ToyChain")
        .subcommand(Command::new("start-node").about("Start a ToyChain server node"))
        .subcommand(
            Command::new("create-account")
                .about("Create an account on Toychain")
                .arg(
                    Arg::new("id-of-account")
                        .help("The ID of the account to create")
                        .index(1)
                        .required(true)
                        .value_name("ID"),
                )
                .arg(
                    Arg::new("starting-balance")
                        .help("The starting balance of the account")
                        .index(2)
                        .required(true)
                        .value_name("BALANCE"),
                ),
        )
        .subcommand(
            Command::new("transfer")
                .about("Transfer funds between accounts on Toychain")
                .arg(
                    Arg::new("from-account")
                        .help("The account to transfer funds from")
                        .index(1)
                        .required(true)
                        .value_name("FROM"),
                )
                .arg(
                    Arg::new("to-account")
                        .help("The account to transfer funds to")
                        .index(2)
                        .required(true)
                        .value_name("TO"),
                )
                .arg(
                    Arg::new("amount")
                        .help("The amount of funds to transfer")
                        .index(3)
                        .required(true)
                        .value_name("AMOUNT"),
                ),
        )
        .subcommand(
            Command::new("balance")
                .about("Get the balance of an account on Toychain")
                .arg(
                    Arg::new("account")
                        .help("The account to get the balance of")
                        .index(1)
                        .required(true)
                        .value_name("ACCOUNT"),
                ),
        )
}

fn main() {
    // Parse the command line arguments
    let matches = cli().get_matches();

    // Use the hostname on Unix-like systems and the computer name on Windows as the NODE_ID
    let node_id = if cfg!(windows) {
        std::env::var("COMPUTERNAME").unwrap_or("localhost".to_string())
    } else {
        std::env::var("HOSTNAME").unwrap_or("localhost".to_string())
    };

    println!("Node ID: {}", node_id);

    // Handle the subcommands
    let request = match matches.subcommand() {
        // Server command - Starts the server
        Some(("start-node", _)) => {
            server::init_server(1337, 10);
            return; // Exit the program after starting the server
        }
        // Client commands
        Some(("create-account", args)) => {
            let id = args.get_one::<String>("id-of-account").unwrap();

            let balance = args.get_one::<String>("starting-balance").unwrap();
            let balance = balance.parse::<f64>().expect("Failed to parse balance.");

            Request::new_create_account_request(node_id, id.to_string(), balance)
        }
        Some(("transfer", args)) => {
            let from = args.get_one::<String>("from-account").unwrap();
            let to = args.get_one::<String>("to-account").unwrap();

            let amount = args.get_one::<String>("amount").unwrap();
            let amount = amount.parse::<f64>().expect("Failed to parse amount.");

            Request::new_transfer_funds_request(node_id, from.to_string(), to.to_string(), amount)
        }
        Some(("balance", args)) => {
            let account = args.get_one::<String>("account").unwrap();
            Request::new_get_funds_request(node_id, account.to_string())
        }
        _ => {
            eprintln!("Invalid command. Use `b --help` for usage information.");
            return;
        }
    };

    // UDP socket to send the request to the server. Port 0 = any available port
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to address.");
    let server_addr = "127.0.0.1:1337";

    // Serialize the request and send it to the server
    let request_bytes = bincode::serialize(&request).expect("Failed to serialize request.");

    // Send the request bytes to the server
    socket
        .send_to(&request_bytes, server_addr)
        .expect("Failed to send message.");
    println!("Request sent to server.");

    // Receive the response from the server. Responses from the server are just user-facing strings
    let mut buf = [0; 1024];
    let (amt, _) = match socket.recv_from(&mut buf) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to receive response. Make sure the server is running (use `start-node`).\nError: {}", e);
            return;
        }
    };

    // Convert the response bytes to a string
    let response = std::str::from_utf8(&buf[..amt]).expect("Failed to convert message to string.");

    println!("Response from server: {}", response);
}
