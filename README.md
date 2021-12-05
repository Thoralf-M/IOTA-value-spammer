# Basic IOTA value spammer

Rename `.env.example` to `.env` and change/replace the seed, then run it with `cargo run --release`

You can also change the index and amount, but the amount should be higher than you can create messages, before the first sent transaction gets confirmed. Otherwise the spammer needs to wait for the confirmation before it can send new transactions.

You can get funds from https://faucet.chrysalis-devnet.iota.cafe/ or ask in the #development channel on Discord https://discord.iota.org.

The spammer will split the funds to 1 Mi per address and send always sends them to the same address again.
