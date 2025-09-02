use core::panic;
use std::{env::{self}, io::Read};
use clap::Parser;
use std::fs::File;

#[derive(Parser, Debug)]
#[command(name = "Clgrader", version="??", about, long_about = "Grade TARGET_PROGRAM using the specification of CONFIGURATION_FILE.")]
struct Cli {


    name: Option<String>, 

    configuration_file: Option<String>,

    target_program: Option<String>,
    
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
//    println!("Hello, world!, 10 - 4 = {}", cli_grader::subtract(10, 4));

    let _args = Cli::parse();

    let arg_table: Vec<String> = env::args().collect();

    // if arg_table.len() > 4 {
    //     panic!("too much arg");
    // }else {
    //
    //
    //     let config = &arg_table[2];
    //     let target = &arg_table[3];
    //
    //     println!("config: {}, target: {}", config, target);

    if arg_table.len() < 4 {
        panic!("Not enough arguments 2 arguments must be given to clgrader");
    } else if arg_table.len() > 4 {
        panic!("Too much arguments are given only 4 are needed");

    } else {
        let config = &arg_table[2];
        let target = &arg_table[3];


        let mut config_opening = File::open(config)?;
        let mut target_opening = File::open(target)?;


        // let (conf_opening_result, target_opening_result) = match (config_opening, target_opening) {
        //     (Ok(_), Err(e)) => panic!("error opening target"),
        //     (Err(e), Ok(_)) => panic!("error op file"),
        //     (Err(a), Err(b)) => panic!("error on both"),
        //     (Ok(conf), Ok(targ)) => (conf, targ) 
        // };


        let mut config_content = String::new();
        let mut target_content = String::new();


        
        config_opening.read_to_string(&mut config_content)?;
        target_opening.read_to_string(&mut target_content)?;



        println!("config content: {}\n target_content: {}", config_content, target_content);
        

        // Should I check for file extension and start to implement Serde ??
    }

    Ok(())

}
