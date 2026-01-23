use cucumber::World;
use dots_family_bdd::TimeWindowWorld;

#[tokio::main]
async fn main() {
    TimeWindowWorld::cucumber()
        .fail_on_skipped()
        .max_concurrent_scenarios(1)
        .run_and_exit("tests/features")
        .await;
}
