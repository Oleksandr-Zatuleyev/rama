mod utils;

#[tokio::test]
#[ignore]
async fn test_http_conn_state() {
    let exit_status = utils::ExampleRunner::run("http_high_level_client").await;
    assert!(exit_status.success());
}
