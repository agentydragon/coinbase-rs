use coinbase_rs::{Public, MAIN_URL};

#[tokio::main]
async fn main() {
    let client: Public = Public::new(MAIN_URL);

    println!(
        "To buy 1 Bitcoin, you need {} USD.",
        client.buy_price("BTC-USD").await.unwrap().amount
    );
}
