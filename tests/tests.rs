use tokio;
use cache_serde::cache_async;

#[cache_async(cache_root = "./cache/{arg}", invalidate_rate = 3600)]
async fn expensive_function(arg: i32) -> Result<String, tokio::io::Error> {
    println!("Executing expensive function...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Ok("Hello2".to_string())
}

#[tokio::test]
async fn check_cached(){
    let result1 = expensive_function(10).await.unwrap();
    let result2 = expensive_function(20).await.unwrap();
    let result3 = expensive_function(30).await.unwrap();

    // assert_eq!(result1, "Hello");
    // assert_eq!(result2, "Hello");
    // assert_eq!(result3, "Hello");

    // let result = async move {
    //     let result4 = expensive_function(20).await.unwrap();
    //     assert_eq!(result4, 40);
    //     2
    // }.await;
}