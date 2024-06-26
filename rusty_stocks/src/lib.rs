use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
};

use clap::{Arg, Command};
use statrs::statistics::Statistics;

use crate::calculations::{calculate_price_paths, run_forest};
use crate::stock::Stock;
use crate::stock::Tomorrow;

pub mod calculations;
pub mod stock;

type CustomResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
}

/*
    Attempt to open passed files and then parse them into stock objects, passing it to the desired method of prediction

    @param (config: Config) config object constructed by the get_args function

    @return (CustomResult()) custom result object which indicates that the function has finished
*/
pub fn run(config: Config) -> CustomResult<()> {
    for filename in config.files {
        match open_file(&filename) {
            Err(e) => eprintln!("{}: {}", filename, e),
            Ok(file) => {
                println!("{} Successfully Opened! Parsing Data...", filename);

                let mut stock_vec: Vec<Stock> = Vec::new();

                for (line_number, line) in file.lines().enumerate() {
                    if line_number == 0 {
                        continue;
                    }
                    let line: String = line.unwrap_or_else(|_| String::from(""));

                    if line.is_empty() {
                        continue;
                    } else {
                        let line_vec: Vec<&str> = line.split(',').collect();
                        let stock: Stock = Stock::new(
                            String::from(line_vec[0]),
                            line_vec[1].parse().unwrap(),
                            line_vec[2].parse().unwrap(),
                            line_vec[3].parse().unwrap(),
                            line_vec[4].parse().unwrap(),
                            line_vec[5].parse().unwrap(),
                            line_vec[6].parse().unwrap(),
                            Tomorrow::Predict,
                        );
                        stock_vec.push(stock);
                    }
                }

                let length = stock_vec.len();
                for i in 0..(length - 1) {
                    if stock_vec[i].get_price() <= stock_vec[i + 1].get_price() {
                        stock_vec[i].set_tomorrow(Tomorrow::Increase);
                    } else {
                        stock_vec[i].set_tomorrow(Tomorrow::Decrease);
                    }
                }

                for i in 0..(length - 1) {
                    let curr_price = stock_vec[i].get_price();
                    stock_vec[i + 1].set_return(curr_price);
                }

                let price_paths = calculate_price_paths(&stock_vec);

                let predicted: f64 = price_paths[price_paths.len() - 1].clone().iter().mean();

                println!("Monte Carlo methods predict a price of {}!", predicted);

                let mut num_inc: i32 = 0;
                let mut num_dec: i32 = 0;
                let mut avg_acc = 0.0;

                for _ in 0..10 {
                    let (res, accuracy) = run_forest(&stock_vec);

                    if res == 1.0 {
                        num_inc += 1;
                    } else {
                        num_dec += 1;
                    }

                    avg_acc += accuracy;
                }

                if num_inc >= num_dec {
                    println!(
                        "The Random Forest predicts an increase with a test accuracy of {}%!",
                        avg_acc * 10.0
                    );
                } else {
                    println!(
                        "The Random Forest predicts a decrease with a test accuracy of {}!",
                        avg_acc * 10.0
                    );
                }
            }
        }
    }

    Ok(())
}

/*
    Opens a passed file which is in respect to the current working directory

    @param (filename: &str) relative file path which is used to open the stock data file

    @return (CustomResult<Box<dyn BufRead>>) CustomResult containing BufRead object used to read the passed file
*/
fn open_file(filename: &str) -> CustomResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

/*
    Parses the command line argument including the filepaths and the number of prediction days

    @return (CustomResult<Config>) CustomResult containing Config object holding passed arguments
*/
pub fn get_args() -> CustomResult<Config> {
    let mut matches = Command::new("rusty_stocks")
        .version("0.1.0")
        .author("Derek Warner <derekw3@illinois.edu>, Chengxun Ren <cren8@illinois.edu>, Haozhe Chen <haozhe6@illinois.edu>, Aaryan Singh Gusain <agusain2@illinois.edu>")
        .about("A CLI stock prediction application")
        .arg(
            Arg::new("files")
                .help("Input File(s)")
                .default_value("-")
                .num_args(1..),
        )
        .get_matches();

    let files_vec: Vec<String> = matches.remove_many("files").unwrap().collect();

    Ok(Config { files: files_vec })
}
