//Gleb Zvonkov
//Nov 8, 2024
//ECE1724

use clap::Parser;
use reqwest::Error;
use serde_json::Value;
use std::collections::HashMap;
use tokio;
use url::Url;

#[derive(Parser)] //command line arguments
struct Args {
    url: String,
    #[arg(short = 'X', default_value_t = String::from("GET"))] //the method by default is GET
    method: String,
    #[arg(short)]
    data: Option<String>,
    #[arg(long)]
    json: Option<String>,
}

#[tokio::main] //use tokio for asynchronous main
async fn main() -> Result<(), Error> {
    let args = Args::parse(); //parse the arguments

    // Check for valid protocol base protocol
    if !(args.url.starts_with("http://") || args.url.starts_with("https://")) {
        eprintln!(
            "Requesting URL: {}\nMethod: {}\nError: The URL does not have a valid base protocol.",
            args.url, args.method
        );
        return Ok(());
    }

    //parse the URL and handle certain errors
    match Url::parse(&args.url) {
        Ok(parsed_url) => parsed_url,
        Err(e) => {
            handle_url_error(&args, e);
            return Ok(());
        }
    };

    //send request and handle error
    let response = match send_request(&args).await {
        Ok(response) => response,
        Err(_) => return Ok(()), // Error occurred, silently return and end execution
    };

    //handle the response from website
    handle_response(response, &args).await?;

    Ok(()) //no error happend
}

// Function to handle URL parsing errors
fn handle_url_error(args: &Args, error: url::ParseError) {
    let error_message = match error {
        url::ParseError::RelativeUrlWithoutBase => "The URL does not have a valid base protocol.",
        url::ParseError::InvalidPort => "The URL contains an invalid port number.",
        url::ParseError::InvalidIpv4Address => "The URL contains an invalid IPv4 address.",
        url::ParseError::InvalidIpv6Address => "The URL contains an invalid IPv6 address.",
        _ => "Some error occurred while parsing the URL.",
    };
    eprintln!(
        "Requesting URL: {}\nMethod: {}\nError: {}",
        args.url, args.method, error_message
    ); //print out the error message along with other infromation
}

//Send the request to web
//return a result
async fn send_request(args: &Args) -> Result<reqwest::Response, Error> {
    let client = reqwest::Client::new(); //create a new client to submit requests
    let method = if args.json.is_some() {
        String::from("POST") //if there is a json field then its post by defualt
    } else {
        args.method.clone() //otheriwse its whatever method was passed in
    };
    let response = match method.as_str() {
        "POST" => {
            if let Some(json_data) = &args.json {
                match serde_json::from_str::<serde_json::Value>(json_data) {
                    //check if its a valid json
                    Ok(_) => {
                        client
                            .post(&args.url)
                            .header("Content-Type", "application/json")
                            .body(json_data.clone())
                            .send()
                            .await //send the request with the json
                    }
                    Err(_) => {
                        //panic if its not valid json
                        eprintln!(
                            "Requesting URL: {}\nMethod: POST\nJSON: {}",
                            &args.url, json_data
                        );
                        panic!("Invalid JSON format: {}", json_data); //panic if its not valid json
                    }
                }
            } else {
                //if its not a json, its a key value pair
                let mut data = HashMap::new(); //create a new hashmap
                if let Some(ref post_data) = args.data {
                    for pair in post_data.split('&') {
                        //split the data on &
                        let mut parts = pair.splitn(2, '='); //split each key value pair on =
                        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
                            data.insert(key.to_string(), value.to_string()); //insert the key value pair into the hashmap
                        }
                    }
                }
                client.post(&args.url).form(&data).send().await //send the request with the key value pairs
            }
        }
        _ => client.get(&args.url).send().await, //get request, just send normally
    };
    match response {
        Ok(response) => Ok(response), //just return the response
        Err(e) => {
            if e.is_connect() || e.is_timeout() {
                //print sepcial error message for timeout or connection issue
                eprintln!("Requesting URL: {}\nMethod: {}\nError: Unable to connect to the server. Perhaps the network is offline or the server hostname cannot be resolved.", &args.url, args.method);
            } else {
                eprintln!(
                    "Requesting URL: {}\nMethod: {}\nError: An unexpected error occurred",
                    &args.url, args.method
                );
            }
            Err(e) //return the error
        }
    }
}

// Function to handle the response
// returns a result
async fn handle_response(response: reqwest::Response, args: &Args) -> Result<(), Error> {
    if response.status().is_success() {
        let body = response.text().await?; //get the body
        if args.method == "POST" {
            print_post_request_info(args).await; //print the data with the post request
        } else {
            println!("Requesting URL: {}\nMethod: {}", args.url, args.method); //print for get request
        }
        // Attempt to parse and sort JSON if possible
        match serde_json::from_str::<Value>(&body) {
            Ok(json_body) => {
                let sorted_json = serde_json::to_string_pretty(&json_body).unwrap();
                println!("Response body (JSON with sorted keys):\n{}", sorted_json);
            }
            Err(_) => {
                //if its not a json just print it normally
                println!("Response body:\n{}", body);
            }
        }
    } else {
        //repsone did not succed, so print the error code
        eprintln!(
            "Requesting URL: {}\nMethod: {}\nError: Request failed with status code: {}",
            args.url,
            args.method,
            response.status().as_u16()
        );
    }
    Ok(())
}

// Function to print POST request information
// its either has json or data
async fn print_post_request_info(args: &Args) {
    if let Some(json_data) = &args.json {
        //print the json
        println!(
            "Requesting URL: {}\nMethod: {}\nJSON: {}",
            args.url, args.method, json_data
        );
    } else {
        //print the data
        println!(
            "Requesting URL: {}\nMethod: {}\nData: {}",
            args.url,
            args.method,
            args.data.clone().unwrap_or_default()
        );
    }
}
