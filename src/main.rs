use iota_client::{
    bee_message::prelude::{Essence, Output, Payload, UtxoInput},
    Client, Result, Seed,
};
extern crate dotenv;
use dotenv::dotenv;
use std::env;

/// Basic value spammer
// You need more Mi than you can create messages before the first transaction got confirmed, otherwise it will fail
// because the output isn't available

const DUST_THRESHOLD: u64 = 1_000_000;
#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let output_amount = env::var("amount").unwrap().parse::<u64>().unwrap();

    let iota = Client::builder()
        .finish()
        .await?;
    let bech32_hrp = iota.get_bech32_hrp().await?;
    // This example uses dotenv, which is not safe for use in production Configure your own seed in ".env"
    let seed = Seed::from_bytes(&hex::decode(
        env::var("NONSECURE_USE_OF_DEVELOPMENT_SEED_1").unwrap(),
    )?);

    let address = iota.get_addresses(&seed).with_range(0..1).finish().await?;
    println!("Send {} Mi to {}", output_amount, address[0]);

    let total_balance = loop {
        let total_balance = iota.get_balance(&seed).finish().await?;
        if total_balance < output_amount * DUST_THRESHOLD {
            println!(
                "Not enough funds {}/{}",
                total_balance,
                output_amount * DUST_THRESHOLD
            );
            std::thread::sleep(std::time::Duration::from_secs(10));
        } else {
            break total_balance;
        }
    };
    let mut available = total_balance;
    println!("Total balance: {}i", total_balance);

    let addresses = iota
        .get_addresses(&seed)
        .with_range(0..output_amount as usize + 1)
        .finish()
        .await?;

    let mut message_builder = iota.message().with_seed(&seed);

    for i in 0..total_balance / DUST_THRESHOLD {
        let mut amount = DUST_THRESHOLD;
        // Don't add more than we have or is allowed; One less here for remaining iotas
        if available == 0 || i >= output_amount {
            if available == 0 {
                break;
            }
            message_builder = message_builder.with_output(&addresses[i as usize], available)?;
            break;
        }
        available -= amount;
        // Add last amount so we don't create dust
        if available < amount {
            amount += available;
            available = 0;
        }
        message_builder = message_builder.with_output(&addresses[i as usize], amount)?;
    }

    let message = message_builder.finish().await?;

    println!(
        "Splitting transaction sent: https://explorer.iota.org/testnet/message/{}",
        message.id().0
    );
    let _ = iota
        .retry_until_included(&message.id().0, None, None)
        .await?;
    let mut outputs = Vec::new();
    if let Some(Payload::Transaction(tx)) = message.payload() {
        let Essence::Regular(essence) = tx.essence();
        for address in addresses.clone() {
            for (index, output) in essence.outputs().iter().enumerate() {
                if let Output::SignatureLockedSingle(output) = output {
                    if output.address().to_bech32(&bech32_hrp) == *address
                        && output.amount() == DUST_THRESHOLD
                    {
                        outputs.push(UtxoInput::new(tx.id(), index as u16)?);
                    }
                }
            }
        }
    }
    let indexation_key = env::var("index").unwrap();
    for round in 0..10000000 {
        let start = std::time::Instant::now();
        for i in 0..output_amount as usize {
            // let outputs = iota.get_address().outputs(&addresses[i], Default::default()).await?;
            match iota
                .message()
                .with_seed(&seed)
                .with_input(outputs[i].clone())
                .with_input_range(i..i + 1)
                .with_index(&indexation_key)
                .with_output(&addresses[i], DUST_THRESHOLD)?
                .finish()
                .await
            {
                Ok(message) => {
                    let id = message.id().0;
                    println!(
                        "Transaction {} sent: https://explorer.iota.org/testnet/message/{}",
                        round * output_amount as usize + i,
                        id
                    );
                    // update output
                    if let Some(Payload::Transaction(tx)) = message.payload() {
                        let Essence::Regular(essence) = tx.essence();
                        for address in addresses.clone() {
                            for (index, output) in essence.outputs().iter().enumerate() {
                                if let Output::SignatureLockedSingle(output) = output {
                                    if output.address().to_bech32(&bech32_hrp) == *address
                                        && output.amount() == DUST_THRESHOLD
                                    {
                                        outputs[i] = UtxoInput::new(tx.id(), index as u16)?;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    println!("{}", error_msg);
                    if error_msg.contains("doesn't have enough balance") {
                        println!("Looks like your computer can do the PoW faster than the messages get confirmed,
                        consider restarting it with a higher amount if you see this message multiple times");
                    }
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }
        }
        println!(
            "Round {} with {} messages took: {}s",
            round,
            output_amount,
            start.elapsed().as_secs()
        );
    }
    Ok(())
}
