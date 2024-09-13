use sp1_sdk::{utils, ProverClient, SP1ProofWithPublicValues, SP1Stdin};

/// The ELF we want to execute inside the zkVM.
const KECCAK_ELF: &[u8] = include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");

fn main() {
    // Setup logging.
    utils::setup_logger();

    // Create an input stream and write the message to it.
    let message = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".to_vec();

    // The input stream that the program will read from using `sp1_zkvm::io::read`. Note that the
    // types of the elements in the input stream must match the types being read in the program.
    let mut stdin = SP1Stdin::new();
    stdin.write(&message);

    // Create a `ProverClient` method.
    let client = ProverClient::new();

    // Execute the program using the `ProverClient.execute` method, without generating a proof.
    let (_, report) = client.execute(KECCAK_ELF, stdin.clone()).run().unwrap();
    println!(
        "executed program with {} cycles",
        report.total_instruction_count()
    );

    // Generate the proof for the given program and input.
    let (pk, vk) = client.setup(KECCAK_ELF);
    let mut proof = client.prove(&pk, stdin).run().unwrap();

    println!("generated proof");

    // Read and verify the output.
    //
    // Note that this output is read from values commited to in the program using
    // `sp1_zkvm::io::commit`.
    let hashed_msg = proof.public_values.read::<[u8; 32]>();

    println!("Keccak hash: {:?}", hashed_msg);

    // Verify proof and public values
    client.verify(&proof, &vk).expect("verification failed");

    // Test a round trip of proof serialization and deserialization.
    proof
        .save("proof-with-pis")
        .expect("saving proof failed");
    let deserialized_proof =
        SP1ProofWithPublicValues::load("proof-with-pis").expect("loading proof failed");

    // Verify the deserialized proof.
    client
        .verify(&deserialized_proof, &vk)
        .expect("verification failed");

    println!("successfully generated and verified proof for the program!")
}
