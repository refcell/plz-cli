use bat::PrettyPrinter;
use clap::Parser;
use colored::*;
use filepath::FilePath;
use question::{Answer, Question};
use reqwest::blocking::Client;
use serde_json::json;
use spinners::{Spinner, Spinners};
use std::{
    env,
    io::{Read, Write},
    process::Command,
};
use tempfile::tempfile;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Description of the command to execute
    prompt: String,

    /// Run the generated program without asking for confirmation
    #[clap(short = 'y', long)]
    force: bool,
}

fn main() {
    let cli = Cli::parse();
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_else(|_| {
        println!("{}", "This program requires an OpenAI API key to run. Please set the OPENAI_API_KEY environment variable.".red());
        std::process::exit(1);
    });

    let mut spinner = Spinner::new(Spinners::BouncingBar, "Generating your command...".into());

    let client = Client::new();
    let response = client
        .post("https://api.openai.com/v1/completions")
        .json(&json!({
            "top_p": 1,
            "temperature": 0,
            "suffix": "\n```",
            "max_tokens": 1000,
            "presence_penalty": 0,
            "frequency_penalty": 0,
            "model": "text-davinci-003",
            "prompt": format!("{}:\n```bash\n#!/bin/bash\n", cli.prompt),
        }))
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap_or_else(|_| {
            spinner.stop_and_persist(
                "✖".red().to_string().as_str(),
                "Failed to get a response. Have you set the OPENAI_API_KEY variable?"
                    .red()
                    .to_string(),
            );
            std::process::exit(1);
        });

    let text = response.json::<serde_json::Value>().unwrap()["choices"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();

    spinner.stop_and_persist(
        "✔".green().to_string().as_str(),
        "Got some code!".green().to_string(),
    );

    PrettyPrinter::new()
        .input_from_bytes(text.trim().as_bytes())
        .language("bash")
        .grid(true)
        .print()
        .unwrap();

    let mut file = tempfile().unwrap_or_else(|_| {
        spinner.stop_and_persist(
            "✖".red().to_string().as_str(),
            "Failed to create a temporary file.".red().to_string(),
        );
        std::process::exit(1);
    });
    file.write_all(text.as_bytes()).unwrap();

    let mut should_run = true;
    if !cli.force {
        should_run = Question::new(
            ">> Run the generated program? [Y/n]"
                .bright_black()
                .to_string()
                .as_str(),
        )
        .yes_no()
        .until_acceptable()
        .default(Answer::YES)
        .ask()
        .expect("Couldn't ask question.")
            == Answer::YES;
    }

    if should_run {
        spinner = Spinner::new(Spinners::BouncingBar, "Executing...".into());

        let output = Command::new("bash")
            .arg(file.path().expect("Couldn't get path of temporary file."))
            .output()
            .unwrap_or_else(|_| {
                spinner.stop_and_persist(
                    "✖".red().to_string().as_str(),
                    "Failed to execute the generated program.".red().to_string(),
                );
                std::process::exit(1);
            });

        if !output.status.success() {
            spinner.stop_and_persist(
                "✖".red().to_string().as_str(),
                "The program threw an error.".red().to_string(),
            );
            println!("{}", String::from_utf8_lossy(&output.stderr));
            std::process::exit(1);
        }

        spinner.stop_and_persist(
            "✔".green().to_string().as_str(),
            "Command ran successfully".green().to_string(),
        );

        println!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
