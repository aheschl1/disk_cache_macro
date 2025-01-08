# Disk Cache Macro

This library provides a macro that is useful for **reducing unnecessary network calls** for any API. By caching responses on disk, it minimizes redundant requests and improves the efficiency of your application. The macro automatically handles the storage and retrieval of cached data, ensuring that your application only makes network calls when absolutely necessary.

## `cache_async` Macro

`cache_async` is a procedural macro that caches the results of asynchronous functions to a specified directory.

### Requirements

The return type of the function must implement both `Serialize` and `Deserialize` from the `serde` crate in order to be cached and retrieved correctly.

### Functionality

It checks if a cache file exists and whether the cache is still valid based on the provided `invalidate_rate`. If the cache is valid, the cached result is returned. Otherwise, the function is executed, and the result is saved to the cache for future use. This macro is especially useful for functions that perform expensive or time-consuming operations and can benefit from caching the results to improve performance.

### Arguments

The macro accepts the following attributes:

- `cache_root`: A string representing the root directory where cache files will be stored. The default is `"cache"`.
- `invalidate_rate`: The time (in seconds) after which the cache should be considered invalid. The default is `3600` seconds (1 hour).

### Return Type

The return type of the function must implement both `Serialize` and `Deserialize` from the `serde` crate in order to be cached and retrieved correctly, or return Result<T, E> where T implements both Serialize and Deserialize.

The decorated functions return type will be wrapped in a Result<T, tokio::io::Error>.