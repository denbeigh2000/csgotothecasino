use casino::aggregator;

#[tokio::main]
async fn main() {
    aggregator::serve().await.unwrap();
}
