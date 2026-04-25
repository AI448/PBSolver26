#![feature(uint_bit_width)]

mod parse_opb;
mod plbd_average;
mod plbd_normalizer;
// mod read_lines;
mod solve;
use crate::solve::solve;

use std::process::ExitCode;

fn main() -> ExitCode {
    match parse_opb::parse_opb(std::io::BufReader::new(std::io::stdin())) {
        Ok((objective, constraints)) => {
            if let Some(objective) = &objective {
                let sum_of_abs = objective
                    .sum
                    .iter()
                    .map(|wt| wt.weight.abs() as u128)
                    .sum::<u128>();
                if sum_of_abs >= 1 << 52 {
                    eprintln!("Too big coefficients sum_of_abs={}", sum_of_abs);
                    println!("s UNSUPPORTED");
                    return ExitCode::SUCCESS;
                }
            }

            for constraint in constraints.iter() {
                let sum_of_abs = constraint
                    .sum
                    .iter()
                    .map(|wt| wt.weight.abs() as u128)
                    .sum::<u128>();
                if sum_of_abs > u64::MAX as u128 {
                    eprintln!("Too big coefficients sum_of_abs={}", sum_of_abs);
                    println!("s UNSUPPORTED");
                    return ExitCode::SUCCESS;
                }
            }

            solve(&objective, &constraints);
            ExitCode::SUCCESS
        }
        Err(_error) => {
            println!("s UNSUPPORTED");
            return ExitCode::SUCCESS;
        }
    }
}
