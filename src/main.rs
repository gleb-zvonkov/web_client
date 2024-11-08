use clap::Parser;
use reqwest::Error;
use serde_json::Value;
use std::collections::HashMap;
use tokio;
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL to request
    url: String,

    /// HTTP method (GET or POST)
    #[arg(short = 'X', long, default_value_t = String::from("GET"))]
    method: String,

    /// Data for the POST request (key=value&key2=value2)
    #[arg(short, long)]
    data: Option<String>,

    /// JSON data for the POST request
    #[arg(long)]
    json: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse(); //parse the arguments

    let url_str = &args.url; //get the url

    let method = if args.json.is_some() {
        String::from("POST")
    } else {
        args.method.to_uppercase()
    };

    // Check for valid protocol (http or https)
    let protocol_valid = url_str.starts_with("http://") || url_str.starts_with("https://");
    if !protocol_valid {
        eprintln!(
            "Requesting URL: {}\nMethod: {}\nError: The URL does not have a valid base protocol.",
            url_str, method
        );
        return Ok(());
    }

    // Parse the URL with detailed error messages
    let url = match Url::parse(&url_str) {
        Ok(url) => url,
        Err(e) => {
            let error_message = match e {
                url::ParseError::RelativeUrlWithoutBase => {
                    "The URL does not have a valid base protocol."
                }
                url::ParseError::InvalidPort => "The URL contains an invalid port number.",
                url::ParseError::InvalidIpv4Address => "The URL contains an invalid IPv4 address.",
                url::ParseError::InvalidIpv6Address => "The URL contains an invalid IPv6 address.",
                _ => "An unknown error occurred while parsing the URL.",
            };
            eprintln!(
                "Requesting URL: {}\nMethod: {}\nError: {}",
                url_str, method, error_message
            ); //print the error message
            return Ok(()); //return from the program
        }
    };

    let client = reqwest::Client::new(); // create a new request

    let response = match if method == "POST" {
        if let Some(json_data) = &args.json {
            // If JSON data is provided, send it with the correct header
            match serde_json::from_str::<serde_json::Value>(json_data) {
                Ok(_) => {
                    // If parsing succeeds, send the data with the correct header
                    client
                        .post(url.as_str())
                        .header("Content-Type", "application/json")
                        .body(json_data.clone())
                        .send()
                        .await
                }
                Err(_) => panic!("Invalid JSON format: {}", json_data), // Panic if JSON is invalid
            }
        } else {
            // Regular POST with form data
            let mut data = HashMap::new(); //create a new hashmap
            if let Some(ref post_data) = args.data {
                //set the data to post
                for pair in post_data.split('&') {
                    //split it on the &
                    let mut parts = pair.splitn(2, '='); //split it on the equals
                    if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                        // set the key and value
                        data.insert(key.to_string(), value.to_string()); // insert as key-value pair
                    }
                }
            }
            // Send the POST request with form data
            client.post(url.as_str()).form(&data).send().await
        }
    } else {
        // Send the GET request
        client.get(url.as_str()).send().await
    } {
        Ok(response) => response,
        Err(e) => {
            // Handle connection errors (host resolution or network issues)
            if e.is_connect() || e.is_timeout() {
                eprintln!(
                    "Requesting URL: {}\nMethod: {}\nError: Unable to connect to the server. Perhaps the network is offline or the server hostname cannot be resolved.",
                    url_str, method
                );
            } else {
                eprintln!(
                    "Requesting URL: {}\nMethod: {}\nError: An unexpected error occurred: {}",
                    url_str, method, e
                );
            }
            return Ok(()); // Exit after handling the error
        }
    };

    // Handle the response status and body
    if response.status().is_success() {
        let body = response.text().await?;

        if method == "POST" {
            if args.json.is_some() {
                println!(
                    "Requesting URL: {}\nMethod: {}\nJSON: {}",
                    url_str,
                    method,
                    args.json.clone().unwrap_or_default()
                );
            } else {
                println!(
                    "Requesting URL: {}\nMethod: {}\nData: {}",
                    url_str,
                    method,
                    args.data.unwrap_or_default()
                );
            }
        } else {
            println!("Requesting URL: {}\nMethod: {}", url_str, method);
        }

        // Attempt to parse and pretty-print JSON, or print plain text if not JSON
        match serde_json::from_str::<Value>(&body) {
            //parse the string
            Ok(json_body) => {
                let sorted_json = serde_json::to_string_pretty(&json_body).unwrap(); //sort the keys
                println!("Response body (JSON with sorted keys):\n{}", sorted_json);
            }
            Err(_) => {
                println!("Response body:\n{}", body); //otherwise just print it normally
            }
        }
    } else {
        eprintln!(
            "Requesting URL: {}\nMethod: {}\nError: Request failed with status code: {}",
            url_str,
            method,
            response.status().as_u16() //print the response status number
        );
    }

    Ok(())
}
