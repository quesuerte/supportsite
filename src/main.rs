use std::{
    fs,
    env,
    error::Error,
    fmt,
};
use urlencoding::encode;
use regex::Regex;

fn main() {
    let args: Vec<String> = env::args().collect();

    let (search_term, password_file) = parse_config(&args)
        .unwrap();

    let mut properties = Vec::new();
    let params = parse_password(&password_file,&mut properties).unwrap();

    match execute_requests(&search_term, &params) {
        Ok(output) => println!("search output: {}", output),
        Err(e) => eprintln!("Error executing requests to Denodo endpoints: {}",e)
    }
}

fn parse_config(args: &[String]) -> Result<(&str, &str),ParseError> {
    if args.len() != 3{
        return Err(ParseError::new("Two arguments should be provided: 'search term' 'credentials file''"));
    }

    Ok((&args[1], &args[2]))
}

fn execute_requests(search: &str, params: &Vec<(String,String)>) -> Result<String,RequestError> {
    let client = reqwest::blocking::Client::builder().cookie_store(true).build().unwrap();

    match client.post("https://auth.denodo.com/login")
    .form(&params)
    .send() {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => {
                println!("Authenticated successfully");
                println!("num cookies: {}", response.cookies().count());
                for cookie in response.cookies() {
                    println!("Cookie name: {}, cookie value: {}", cookie.name(), cookie.value());
                }
                for header in response.headers().iter() {
                    let (headername, headervalue) = header;
                    println!("Authentication header {}, value: {:?}", headername, headervalue)
                }
            },
            code => return Err(RequestError::new(&format!("Error sending request; HTTP response: {}, Body: {:?}", code.as_u16(), response.text())))
        },
        Err(e) => {
            return Err(RequestError::new(&format!("Erorr sending request: {e}")));
        }
    };

    match client.get(format!("https://search.denodo.com/results/ajax/casesCaseComments?filter={}&version=8",encode(search))).send() {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => {
                println!("Searched successfully");
                return match response.text(){
                    Ok(output) => Ok(output),
                    Err(e) => return Err(RequestError::new(&format!("Could not parse text from response: {}", e.to_string())))
                }
            },
            code => return Err(RequestError::new(&format!("failed to search; HTTP status code: {}",&code.as_u16().to_string())))
        },
        Err(e) => return Err(RequestError::new(&format!("Request to search endpoint failed: {}", e.to_string())))
    }
}

fn parse_password<'a>(file_name: &str, properties: &'a mut Vec<(String,String)>) -> Result<&'a Vec<(String,String)>,ParseError> {
    let client = reqwest::blocking::Client::builder().cookie_store(true).build().unwrap();

    let execution_code;
    let re = Regex::new("<input type=\"hidden\" name=\"execution\" value=\"([^\"]+)\"/>").unwrap();
    match client.get("https://auth.denodo.com/login")
    .send() {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => {
                println!("Retrieved auth page");
                match response.text() {
                    Ok(msg) => {
                        match re.captures(&msg) {
                            Some(cap) => match cap.get(1) {
                                Some(exec_value) => execution_code = String::from(exec_value.as_str()),
                                None => return Err(ParseError::new("no exec match"))
                            }
                            None => return Err(ParseError::new("Error parsing response for execution"))
                        }
                    }
                    Err(e) => return Err(ParseError::new(&format!("Error parsing text for response: {}",e)))
                }
            },
            code => return Err(ParseError::new(&format!("Error sending request; HTTP response: {}", code.as_u16())))
        },
        Err(e) => {
            return Err(ParseError::new(&format!("Erorr sending request: {e}")));
        }
    };
    properties.push((String::from("execution"),String::from(execution_code)));

    if let Ok(contents) = fs::read_to_string(file_name) {   
        for line in contents.lines() {
            if let Some(property) = line.split_once("=") {
                properties.push((String::from(property.0),String::from(property.1)));
                println!("property name: {} property value: {}", property.0, property.1);
            }
        }
        if properties.len() == 4 {
            return Ok(properties);
        }
        return Err(ParseError::new("Username and password not present in password file"));
    }
    Err(ParseError::new("Could not open password file"))
}

#[derive(Debug)]
struct ParseError {
    details: String
}

impl ParseError {
    fn new(msg: &str) -> ParseError {
        ParseError{details: msg.to_string()}
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Debug)]
struct RequestError {
    details: String
}

impl RequestError {
    fn new(msg: &str) -> RequestError {
        RequestError{details: msg.to_string()}
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for RequestError {
    fn description(&self) -> &str {
        &self.details
    }
}