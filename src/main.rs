use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use smvblock::{
    blockchain::User,
    node::{Node, NodeType},
};
use std::collections::HashMap;

fn decode_address(hex_str: &str) -> [u8; 32] {
    let bytes = hex::decode(hex_str).expect("Invalid hex string");
    bytes.try_into().expect("Expected 32-byte address")
}

#[tokio::main]
async fn main() {
    let mut node = Node::new(NodeType::FullNode, true).unwrap();
    let mut users: HashMap<String, (User, SigningKey)> = HashMap::new();
    let mut rl = Editor::<(), rustyline::history::FileHistory>::new().unwrap();

    println!("Welcome to smvblock REPL.");
    println!("Type `help` for commands, `exit` to quit.");

    loop {
        let line = rl.readline("smvblock> ");
        match line {
            Ok(input) => {
                let input = input.trim();
                rl.add_history_entry(input);

                if input == "exit" {
                    println!("Exiting smvblock REPL.");
                    break;
                } else if input == "help" {
                    println!("Commands:");
                    println!("  add-user <balance>");
                    println!("  stake <address> <amount>");
                    println!("  transact <from> <to> <amount>");
                    println!("  produce-block");
                    println!("  show-users");
                    println!("  exit");
                } else if input.starts_with("add-user ") {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    if parts.len() != 2 {
                        println!("Usage: add-user <balance>");
                        continue;
                    }
                    let balance: u64 = parts[1].parse().expect("Invalid balance");
                    let (user, pk) = User::generate(balance);
                    let addr_hex = hex::encode(user.address);

                    node.add_user(user.clone()).await.unwrap();
                    users.insert(addr_hex.clone(), (user, pk));

                    println!("Added user with address: {}", addr_hex);
                } else if input.starts_with("stake ") {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    if parts.len() != 3 {
                        println!("Usage: stake <address> <amount>");
                        continue;
                    }
                    let address = decode_address(parts[1]);
                    let amount: u64 = parts[2].parse().expect("Invalid amount");

                    node.stake(address, amount).await.unwrap();
                    println!("Staked {} tokens for {}", amount, parts[1]);
                } else if input.starts_with("transact ") {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    if parts.len() != 4 {
                        println!("Usage: transact <from> <to> <amount>");
                        continue;
                    }

                    let from = parts[1];
                    let to = decode_address(parts[2]);
                    let amount: u64 = parts[3].parse().expect("Invalid amount");

                    let (_, key) = users.get(from).expect("Sender not found");
                    let pk = key.clone();

                    node.send_transaction(pk, to, amount).await.unwrap();
                    println!("Sent {} tokens from {} to {}", amount, from, parts[2]);
                } else if input == "produce-block" {
                    let hash = node.produce_block().await.unwrap();
                    println!("Produced block: {}", hex::encode(hash));
                } else if input == "show-users" {
                    let all_users = node.get_users().await.unwrap();
                    for user in all_users {
                        println!(
                            "User: {}, Balance: {}, Stake: {}",
                            hex::encode(user.address),
                            user.balance,
                            user.stake
                        );
                    }
                } else {
                    println!("Unknown command. Type `help`.");
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("\nExiting smvblock REPL.");
                break;
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                break;
            }
        }
    }
}
