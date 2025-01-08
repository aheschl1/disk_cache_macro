use tokio;
use disk_cache::cache_async;

#[cache_async(cache_root = "./cache/{arg}", invalidate_rate = 3600)]
async fn expensive_function_result(arg: i32) -> Result<String, tokio::io::Error> {
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    Ok("Hello".to_string())
}

#[cache_async(cache_root = "./cache/not_result/{arg}", invalidate_rate = 3600)]
async fn expensive_function_not_result(arg: i32) -> String {
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    "Hello".to_string()
}

#[tokio::test]
async fn check_correct_output(){
    let result1 = expensive_function_result(10).await.unwrap().unwrap();
    let result2 = expensive_function_result(20).await.unwrap().unwrap();
    let result3 = expensive_function_result(30).await.unwrap().unwrap();

    assert_eq!(result1, "Hello");
    assert_eq!(result2, "Hello");
    assert_eq!(result3, "Hello");
}

#[tokio::test]
async fn check_cache_created(){
    // clear the cache
    let cache_path = "./cache/10";
    std::fs::remove_file(cache_path).unwrap_or_default();
    let result1 = expensive_function_result(10).await.unwrap().unwrap();
    // make sure the cache is created
    assert!(std::fs::metadata(cache_path).is_ok());
}

#[tokio::test]
async fn check_cache_hit(){
    // clear the cache
    let cache_path = "./cache/40";
    std::fs::remove_file(cache_path).unwrap_or_default();
    let result1 = expensive_function_result(40).await.unwrap().unwrap();
    // make sure the cache is created
    assert!(std::fs::metadata(cache_path).is_ok());
    // sleep to let the cache be written
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    // modify the file and make sure the cache is hit
    std::fs::write(format!("{cache_path}/data.json"), "\"Hello world\"").unwrap();
    let result2 = expensive_function_result(40).await.unwrap().unwrap();
    assert_eq!(result2, "Hello world");
}

#[tokio::test]
async fn check_cache_not_result(){
    // clear the cache
    let cache_path = "./cache/not_result/50";
    std::fs::remove_file(cache_path).unwrap_or_default();
    let result1 = expensive_function_not_result(50).await.unwrap();
    // make sure the cache is created
    assert!(std::fs::metadata(cache_path).is_ok());
}

#[tokio::test]
async fn check_cache_hit_not_result(){
    // clear the cache
    let cache_path = "./cache/not_result/60";
    std::fs::remove_file(cache_path).unwrap_or_default();
    let result1 = expensive_function_not_result(60).await.unwrap();
    // make sure the cache is created
    assert!(std::fs::metadata(cache_path).is_ok());
    // sleep to let the cache be written
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    // modify the file and make sure the cache is hit
    std::fs::write(format!("{cache_path}/data.json"), "\"Hello world\"").unwrap();
    let result2 = expensive_function_not_result(60).await.unwrap();
    assert_eq!(result2, "Hello world");
}

#[tokio::test]
async fn check_correct_output_not_result(){
    let result1 = expensive_function_not_result(70).await.unwrap();
    let result2 = expensive_function_not_result(80).await.unwrap();
    let result3 = expensive_function_not_result(90).await.unwrap();

    assert_eq!(result1, "Hello");
    assert_eq!(result2, "Hello");
    assert_eq!(result3, "Hello");
}