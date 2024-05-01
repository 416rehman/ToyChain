use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use crate::common;
use crate::common::Operation;

/// A blockchain transaction that will be part of a block
#[derive(Serialize, Deserialize, Clone)]
struct Transaction {
    // The node that created the transaction
    node_id: String,
    // The account that the funds are being transferred from. If this is None,
    // then the transaction is a reward for mining a block or creating an account
    from_account_id: Option<String>,
    // The account that the funds are being transferred to
    to_account_id: String,
    // The amount of funds being transferred
    amount: f64,
    // Timestamp of the transaction
    datetime: std::time::SystemTime,
}

impl Transaction {
    /// Returns a new transaction with the given parameters
    fn new(node_id: String, from_account_id: Option<String>, to_account_id: String, amount: f64) -> Transaction {
        Transaction {
            node_id,
            from_account_id,
            to_account_id,
            amount,
            datetime: std::time::SystemTime::now(),
        }
    }
}

/// Blockchain block that contains transactions
#[derive(Serialize, Deserialize, Clone)]
struct Block {
    // All transactions in the block
    transactions: Vec<Transaction>,
    // Hash of the previous block
    previous_hash: String,
    // Hash of the block
    hash: String,
}

impl Block {
    /// Calculate and set the hash of the block if not already set
    fn calc_and_set_hash(&mut self) {
        if self.hash.is_empty() {
            let mut hasher = sha2::Sha256::new();
            hasher.update(bincode::serialize(&self).unwrap());
            let calculated_hash = hasher.finalize();
            self.hash = calculated_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        }
    }
}

/// State of the blockchain server
struct State {
    ledger: Mutex<Vec<Block>>,
    next_block_to_mint: Mutex<Block>,
}

impl State {
    /// Checks if an account exists in the ledger by checking all previous transactions ever made
    fn account_exists(&self, account_id: &String) -> bool {
        let ledger = self.ledger.lock().unwrap();
        ledger.iter().any(|block| block.transactions.iter().any(|transaction| {
            transaction.to_account_id == *account_id || transaction.from_account_id.as_ref().map_or(false, |from_id| from_id == account_id)
        }))
    }

    /// Gets the balance of an account by checking all previous transactions ever made
    fn get_balance(&self, account_id: &String) -> f64 {
        let ledger = self.ledger.lock().unwrap();
        let mut balance = 0.0;

        for block in ledger.iter() {
            for transaction in &block.transactions {
                // If account is the sender, subtract the amount
                if let Some(from_account_id) = &transaction.from_account_id {
                    if from_account_id == account_id {
                        balance -= transaction.amount;
                    }
                }

                // If account is the receiver, add the amount
                if &transaction.to_account_id == account_id {
                    balance += transaction.amount;
                }
            }
        }
        balance
    }
}

/// Mint blocks every specified interval
fn mint_blocks(state: Arc<State>, mint_interval_in_seconds: u64) {
    println!("Minting blocks every {} seconds.", mint_interval_in_seconds);
    loop {
        println!("Waiting {} seconds to mint the next block.", mint_interval_in_seconds);
        std::thread::sleep(std::time::Duration::from_secs(mint_interval_in_seconds));

        let mut next_block_to_mint = state.next_block_to_mint.lock().unwrap();
        if next_block_to_mint.transactions.is_empty() {
            println!("Skipping block minting as there are no transactions.");
            continue;
        }

        // Set the hash of the block
        next_block_to_mint.calc_and_set_hash();

        // Add the block to the ledger
        state.ledger.lock().unwrap().push(next_block_to_mint.clone());

        println!("Block {} minted with {} transactions.", &next_block_to_mint.hash, next_block_to_mint.transactions.len());

        // Reset the next block to mint to a new block
        next_block_to_mint.transactions.clear();
        next_block_to_mint.previous_hash = next_block_to_mint.hash.clone();
        next_block_to_mint.hash = "".to_string();
    }
}

/// Initializes the blockchain server on a given port. The server listens for requests from clients and processes them.
/// The server also mints blocks every specified interval and adds them to the ledger.
///
/// # Arguments
///
/// * `port`: the port on which the server will listen for requests
/// * `mint_interval_in_seconds`: the interval in seconds at which the server will mint blocks
///
/// Returns: This function should be called only once and will run indefinitely (until manually stopped).
pub fn init_server(port: u16, mint_interval_in_seconds: u64) {
    let addr = format!("0.0.0.0:{}", port);
    let socket = UdpSocket::bind(addr).expect("Failed to bind to address. Make sure PORT {} is not in use.");
    println!("Server started on port {}.", port);

    let state = Arc::new(State {
        ledger: Mutex::new(Vec::new()),
        next_block_to_mint: Mutex::new(Block {
            transactions: Vec::new(),
            previous_hash: "".to_string(),
            hash: "".to_string(),
        }),
    });

    let shared_state = state.clone();
    std::thread::spawn(move || mint_blocks(shared_state, mint_interval_in_seconds));

    let mut buf = [0u8; 1024];
    loop {
        let (amt, src) = match socket.recv_from(&mut buf) {
            Ok((amt, src)) => (amt, src),
            Err(e) => {
                eprintln!("Failed to receive request: {}", e);
                continue;
            }
        };

        let response = match bincode::deserialize(&buf[..amt]) {
            Ok(request) => process_request(state.clone(), request),
            Err(e) => {
                eprintln!("Failed to deserialize request: {} - from: {}", e, src);
                continue;
            }
        };

        if let Err(e) = socket.send_to(response.as_bytes(), src) {
            eprintln!("Failed to send response: {}", e);
        }

        println!("Response sent to client: {}", response);
    }
}

/// Processes a request received from a client. The client can request to create an account, transfer funds, or get funds.
///
/// # Arguments
///
/// * `state`: the current state of blockchain server
/// * `request`: the request from the client
///
/// Returns: String which can be sent back to the client as a response
fn process_request(state: Arc<State>, request: common::Request) -> String {
    println!("Received request from {}: {:?}", request.from_node, request.operation);

    match request.operation {
        Operation::CreateAccount(account_info) => {
            if state.account_exists(&account_info.account_id) {
                return format!("⚠️ Account {} already exists.", &account_info.account_id);
            };

            let transaction = Transaction::new(request.from_node, None, account_info.account_id.clone(), account_info.starting_balance);

            state.next_block_to_mint.lock().unwrap().transactions.push(transaction);

            return format!("✅ Transaction to create account {} with balance {} committed.", &account_info.account_id, &account_info.starting_balance);
        }

        Operation::TransferFunds(transfer_info) => {
            // Validate that the from and to accounts are different
            if transfer_info.from_account_id == transfer_info.to_account_id {
                return "❌ Cannot transfer funds to the same account.".to_string();
            }

            // Validate that the from account has sufficient funds
            let balance = state.get_balance(&transfer_info.from_account_id);
            if balance < transfer_info.amount {
                return format!("❌ Insufficient funds in account {} to transfer {}.", transfer_info.from_account_id, transfer_info.amount);
            }

            let transaction = Transaction::new(request.from_node, Some(transfer_info.from_account_id.clone()), transfer_info.to_account_id.clone(), transfer_info.amount);

            state.next_block_to_mint.lock().unwrap().transactions.push(transaction);
            return format!("✅ Transaction to transfer {} from {} to {} committed.", transfer_info.amount, &transfer_info.from_account_id, &transfer_info.to_account_id);
        }

        Operation::GetFunds(get_info) => {
            let balance = state.get_balance(&get_info.account_id);
            return format!("Account {} has a balance of {}.", get_info.account_id, balance);
        }
    }
}
