use rand::distributions::Distribution;
use rand::seq::SliceRandom;
use randomforest::{RandomForestClassifier, RandomForestClassifierOptions};
use randomforest::criterion::Gini;
use randomforest::table::{Table, TableBuilder};
use statrs::distribution::Normal;

use crate::stock::Stock;

/*
    Constructs a random forest crate TableBuilder which holds the stock data from
    the passed stock struct vector

    @param (stocks: &Vec<Stock>) vector of stock structs containing training dataset

    @return (TableBuilder) TableBuilder object with stock data inserted
*/
pub fn construct_table(stocks: &Vec<Stock>) -> TableBuilder {
    let mut table_builder: TableBuilder = TableBuilder::new();

    for stock in stocks {
        let _ = table_builder.add_row(&stock.get_array(), stock.get_label());
    }

    table_builder
}

/*
    Splits stocks into two sets, training and testing for cross-reference testing

    @param (stocks: &Vec<stock>) vector of stock structs parsed from file
    @param (training: f32) fraction of dataset to be in the training set

    @return (Vec<Stock>, Vec<Stock) partitioned training and testing datasets respectively
*/
pub fn split_data(stocks: &[Stock], training: f32) -> (Vec<Stock>, Vec<Stock>) {
    let mut indices: Vec<usize> = (0..stocks.len()).collect();
    indices.shuffle(&mut rand::thread_rng());
    let training_index: usize = (training * (stocks.len() as f32)) as usize;
    let mut training_set: Vec<Stock> = Vec::new();
    for idx in indices[0..training_index].iter() {
        training_set.push(stocks[*idx].clone());
    }

    let mut test_set: Vec<Stock> = Vec::new();
    for idx in indices[training_index..].iter() {
        test_set.push(stocks[*idx].clone());
    }

    (training_set, test_set)
}

/*
    Builds the random forest and predicts if it will increase or decrease between today and tomorrow

    @param (stocks: Vec<Stock>) vector of Stock objects parsed from the input file

    @return (f64, f32) the predicted result and accuracy respectively
*/
pub fn run_forest(stocks: &[Stock]) -> (f64, f32) {
    let ultimo: Stock = stocks[stocks.len() - 1].clone();
    let dataset: Vec<Stock> = stocks[0..stocks.len() - 1].to_vec();

    let (training_set, test_set) = split_data(&dataset, 0.9);

    let table_builder: TableBuilder = construct_table(&training_set);

    let table: Table = table_builder.build().unwrap();

    let classifier: RandomForestClassifier = RandomForestClassifierOptions::new().fit(Gini, table);

    let num_tests: f32 = test_set.len() as f32;
    let mut num_correct: f32 = 0.0;

    for stock in test_set {
        let result = classifier.predict(&stock.get_array());

        if result == stock.get_label() {
            num_correct += 1.0;
        }
    }

    let mut accuracy = num_correct / num_tests;
    let mut switch_flag: bool = false;

    // if the accuracy is less than 50% it is actually useful to do the opposite of what the model says
    if accuracy < 0.5 {
        accuracy = 1.0 - accuracy;
        switch_flag = true;
    }

    let mut result = classifier.predict(&ultimo.get_array());

    if switch_flag {
        result = if result == 1.0 { 0.0 } else { 1.0 };
    }

    (result, accuracy)
}

/*
    Calculates the drift for Brownian motion

    @param (stocks: &Vec<Stock>) vector of stock objects

    @return (f64, f64, f64) the calculated drift and variance respectively
*/
pub fn calculate_drift(stocks: &Vec<Stock>) -> (f64, f64) {
    let mut mean = 0.0;

    for stock in stocks {
        mean += stock.get_return();
    }

    mean /= stocks.len() as f64;

    let mut var = 0.0;

    for stock in stocks {
        var += (stock.get_return() - mean).powi(2);
    }

    var /= stocks.len() as f64;

    (mean - (0.5 * var), var)
}

/*
    Calculate the daily returns matrix which uses logarithmic daily returns to find the change in a specific stock

    @param (stocks: &Vec<Stock>) vector of stock objects

    @return (Vec<Vec<f64>>) daily return matrix with the coefficients to be used in Black-Scholes
*/
pub fn calculate_daily_returns(stocks: &Vec<Stock>) -> Vec<Vec<f64>> {
    let (drift, var) = calculate_drift(stocks);

    let std: f64 = var.sqrt();

    let days = 30;
    let trials = 50000;

    let mut rng = rand::thread_rng();

    let normal = Normal::new(0.0, 1.0).unwrap();

    let mut daily_returns: Vec<Vec<f64>> = Vec::new();

    for _ in 0..days {
        let mut z: Vec<f64> = Vec::new();
        for _ in 0..trials {
            z.push((drift + std * normal.sample(&mut rng)).exp());
        }
        daily_returns.push(z);
    }

    daily_returns
}

/*
    Calculate the price paths (random walks predicting prices) for the Monte Carlo trials

    @param (stocks: &Vec<Stock>) vector of stock object

    @return (Vec<Vec<f64>>) vector where the columns are individual random walks
*/
pub fn calculate_price_paths(stocks: &Vec<Stock>) -> Vec<Vec<f64>> {
    let daily_returns = calculate_daily_returns(stocks);

    let mut price_paths: Vec<Vec<f64>> = Vec::new();

    let mut first_day: Vec<f64> = Vec::new();
    for _ in 0..daily_returns[0].len() {
        first_day.push(stocks[stocks.len() - 1].get_price());
    }
    price_paths.push(first_day);

    for i in 1..daily_returns.len() {
        let mut price_path: Vec<f64> = Vec::new();
        for j in 0..daily_returns[0].len() {
            price_path.push(price_paths[i - 1][j] * daily_returns[i][j]);
        }
        price_paths.push(price_path);
    }

    price_paths
}
