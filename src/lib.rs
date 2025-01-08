use proc_macro::TokenStream;
use quote::quote;
use syn::ReturnType;
use syn::{parse_macro_input, punctuated::Punctuated, AttributeArgs, DeriveInput, ItemFn, Lit, Meta, NestedMeta, Type};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use std::{env, path::PathBuf};
use std;
use tokio;

/// `cache_async` is a procedural macro that caches the results of asynchronous functions to a specified directory.
/// 
/// Return type of the function must implement both `Serialize` and `Deserialize` from the `serde` crate in order to 
/// be cached and retrieved correctly, or, if the return type is a `Result<T, E>`, then `T` must implement `Serialize`.
/// 
/// It checks if a cache file exists and whether the cache is still valid based on the provided `invalidate_rate`. 
/// If the cache is valid, the cached result is returned. Otherwise, the function is executed, and the result is 
/// saved to the cache for future use. This macro is especially useful for functions that perform expensive or 
/// time-consuming operations and can benefit from caching the results to improve performance.
///
/// # Arguments
/// The macro accepts the following attributes:
/// - `cache_root`: A string representing the root directory where cache files will be stored. The default is `"cache"`.
/// - `invalidate_rate`: The time (in seconds) after which the cache should be considered invalid. The default is `3600` seconds (1 hour).
///
/// # Return Type
/// The return type of the function must implement both `Serialize` and `Deserialize` from the `serde` crate in order to 
/// be cached and retrieved correctly.

#[proc_macro_attribute]
pub fn cache_async(args: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input function
    let input = parse_macro_input!(item as ItemFn);
    let args = parse_macro_input!(args as AttributeArgs);

    let func_name = &input.sig.ident;
    let func_body = &input.block;
    let func_args = &input.sig.inputs;
    let func_output = &input.sig.output;

    let func_type = match func_output {
        syn::ReturnType::Type(_, t) => t,
        _ => panic!("Function must have a return type"),
    };
    let is_result = is_result_type(func_output).is_some();
    // here, we want to check if the return type is a Result type. Only then we can use the ? operator
    let mut calling_code = quote! { 
        let result: #func_type = async move { #func_body }.await;
    };
    if is_result{
        calling_code = quote! { 
            let result: #func_type = async move { #func_body }.await;
            if let Err(e) = result {
                return Ok(Err(e));
            }
            let result = result.unwrap();
        };
    }
    // also, if result type, we only need to cache the Ok part of the result: thus, we check if Ok part is serializable
    let mut where_clause = quote! {
        where #func_type: serde::Serialize + serde::de::DeserializeOwned
    };
    if is_result{
        let (ok_type, _) = is_result_type(func_output).unwrap();
        where_clause = quote! {
            where #ok_type: serde::Serialize + serde::de::DeserializeOwned
        };
    }
    // One other thing is that if there is a Result type, we need to return Ok(result) instead of result on cache hit
    let mut return_call = quote! { result };
    if is_result{
        return_call = quote! { Ok(result) };
    }

    // attributes
    let mut cache_path = PathBuf::from(expand_tilde("~/.cache/cache_serde"));
    let mut invalidate_rate = 3600; 
    // Parse the attributes
    for arg in args.iter() {
        match arg {
            NestedMeta::Meta(Meta::NameValue(nv)) if nv.path.is_ident("cache_root") => {
                if let Lit::Str(lit_str) = &nv.lit {
                    cache_path = PathBuf::from(lit_str.value());
                }
            },
            NestedMeta::Meta(Meta::NameValue(nv)) if nv.path.is_ident("invalidate_rate") => {
                if let Lit::Int(lit_int) = &nv.lit {
                    let seconds = lit_int.base10_parse::<i64>().unwrap();
                    invalidate_rate = seconds;
                }
            },
            _ => (),
        }
    }
    let cache_path: String = cache_path.to_str().expect("Invalid cache path").to_string();
    // figure out the header - depends on pub
    let func_vis = &input.vis;

    let output = quote! {
        #func_vis async fn #func_name(#func_args) -> Result<#func_type, tokio::io::Error> #where_clause {
            // now we have the cache path. put the data.json at the end
            let mut cache_path: String = format!("{}/data.json", format!(#cache_path).to_string()).to_string();
            let path: std::path::PathBuf = std::path::PathBuf::from(&cache_path);
            // Ensure the parent directory exists
            if let Some(parent) = path.parent() {
                if tokio::fs::metadata(parent).await.is_err() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            // Check if the cache is still valid
            let expiry = chrono::Duration::seconds(#invalidate_rate);
            if tokio::fs::try_exists(&cache_path).await?{
                let last_written = tokio::fs::metadata(&cache_path).await?.modified()?;
                let last_written = chrono::DateTime::<chrono::Utc>::from(last_written);
                let duration_since_last_written = chrono::Utc::now().signed_duration_since(last_written);
                if duration_since_last_written < expiry{
                    let data = tokio::fs::read_to_string(&cache_path).await?;
                    let result = serde_json::from_str(&data)?; // Deserialize the cached data
                    return Ok(#return_call);
                }
            }
            // Get the data from the function
            #calling_code
            // Write the data to the cache: spawn a task to write the data to the cache
            let string_data = serde_json::to_string(&result).unwrap();
            let _ = tokio::spawn(async move {
                tokio::fs::write(&cache_path, string_data).await.unwrap();
            });
            Ok(#return_call)
        } 
    };

    output.into()

}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(home_dir) = env::var_os("HOME") {
        PathBuf::from(path.replacen("~", &home_dir.to_string_lossy(), 1))
    } else {
        PathBuf::from(path) // Fallback to original path if HOME isn't set
    }
}

fn is_result_type(output: &ReturnType) -> Option<(&Type, &Type)> {
    if let ReturnType::Type(_, ty) = output {
        // Match the return type as a Path
        if let Type::Path(type_path) = &**ty {
            // Check if the last segment is "Result"
            if type_path.path.segments.last().map(|seg| seg.ident == "Result") == Some(true) {
                // Extract the generic arguments of Result<T, E>
                if let syn::PathArguments::AngleBracketed(args) = &type_path.path.segments.last().unwrap().arguments {
                    let mut args_iter = args.args.iter();

                    // Get T and E
                    let ok_type = args_iter.next().and_then(|arg| match arg {
                        syn::GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    });

                    let err_type = args_iter.next().and_then(|arg| match arg {
                        syn::GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    });

                    if let (Some(ok), Some(err)) = (ok_type, err_type) {
                        return Some((ok, err));
                    }
                }
            }
        }
    }
    None
}
