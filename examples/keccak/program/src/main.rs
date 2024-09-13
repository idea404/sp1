#![no_main]
sp1_zkvm::entrypoint!(main);

use sha3::{Digest, Keccak256};

pub fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a system call which handles reading inputs
    let message = sp1_zkvm::io::read::<Vec<u8>>();

    let mut hasher = Keccak256::new();
    hasher.update(message);
    let hashed_msg = hasher.finalize();

    // For demonstration, we print the hashed message
    println!("Keccak hash: {:?}", hashed_msg);

    // Convert hashed_msg to a vec
    let hashed_msg_vec = hashed_msg.to_vec();

    // Write the output of the program.
    //
    // Behind the scenes, this also compiles down to a system call which handles writing
    sp1_zkvm::io::commit(&hashed_msg_vec);
}