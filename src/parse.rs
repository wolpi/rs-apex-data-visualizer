use std::fs::File;
use std::io::{prelude::*, BufReader};
use chrono::NaiveDateTime;
use serde::Serialize;

const SEPARATOR :&str = ",";
const TIMESTAMP_FORMAT_1 :&str = "%Y-%m-%d %H:%M:%S";
const TIMESTAMP_FORMAT_2 :&str = "%d/%m/%Y %H:%M";
const TIMESTAMP_FORMAT_3 :&str = "%Y-%m-%dT%H:%M:%S%Z";
const TIMESTAMP_FORMATS :[&str;3] = [TIMESTAMP_FORMAT_1, TIMESTAMP_FORMAT_2, TIMESTAMP_FORMAT_3];

#[derive(Clone, Serialize)]
pub struct Entry {
    pub value :f32,
    pub timestamp :u64,
}

impl Entry {
    pub fn new() -> Entry {
        Entry {
            value: 0.0,
            timestamp: 0,
        }
    }
}


pub fn parse_file(file :&File) -> Vec<Entry> {
    let reader = BufReader::new(file);

    let mut result = Vec::new();
    let mut line_counter = 0;
    for line_result in reader.lines() {
        if line_result.is_err() {
            print!("could not read line: ");
            println!("{}", line_result.unwrap_err());
            continue;
        }
        let line = line_result.unwrap();
        let parse_result = parse_line(&line, &line_counter);
        let line_error = parse_result.1;
        if !line_error {
            result.push(parse_result.0);
        }
        line_counter += 1;
    }
    return result;
}

fn parse_line(line :&String, line_counter:&u32) -> (Entry, bool) {
    //println!("{}", line);
    let mut entry = Entry::new();
    let mut line_error = false;
    let mut index :usize = 0;
    let mut err_msg :&str = "";

    let mut parse_result = parse_timestamp(line, & mut entry, &index);
    if parse_result.is_ok() {
        index = parse_result.unwrap();
    } else {
        line_error = true;
        err_msg = parse_result.err().unwrap();
    }

    if !line_error {
        parse_result = parse_float(line, &mut entry.value, &index, "could not parse: value float");
        //if parse_result.is_ok() {
        //    index = parse_result.unwrap();
        //} else {
        //    index += 1;
        //}
        if parse_result.is_err() {
            // never mind
        }
    }

    if line_error && *line_counter > 2 {
        println!("error parsing line: {}", err_msg);
        print!("    ");
        println!("{}", line);
    }
    return (entry, line_error);
}

fn parse_timestamp(line :&String, entry :&mut Entry, start_index :&usize) -> Result<usize, &'static str> {
    let first_char_result = (*line).chars().nth(0);
    if first_char_result.is_none() {
        return Result::Err("empty value");
    }
    let quotes_present = first_char_result.unwrap() == '"';
    let local_start_index = match quotes_present {
        true => *start_index + 1,
        false => *start_index
    };
    let end_index_offset =  match quotes_present {
        true => 1,
        false => 0
    };

    let line_remainer = &line[local_start_index .. line.len()];
    let find_result = line_remainer.find(SEPARATOR);
    let err_msg = "could not parse: timestamp";
    if find_result.is_some() {
        let index = find_result.unwrap();
        if index > 0 {
            let timestamp_str: &str = &line_remainer[0..index - end_index_offset];
            for timestamp_format in TIMESTAMP_FORMATS {
                let parse_result = NaiveDateTime::parse_from_str(timestamp_str, timestamp_format);
                if parse_result.is_ok() {
                    let timestamp = parse_result.unwrap();
                    entry.timestamp = timestamp.timestamp_millis() as u64;
                    return Result::Ok(local_start_index + index + end_index_offset + 1); // +1 for csv separator
                }
            }
        }
    }
    return Result::Err(err_msg);
}

fn parse_float(line :&String, target :&mut f32, start_index :&usize, err_msg :&'static str) -> Result<usize, &'static str> {
    let first_char_result = (*line).chars().nth(0);
    if first_char_result.is_none() {
        return Result::Err("empty value");
    }
    let quotes_present = first_char_result.unwrap() == '"';
    let local_start_index = match quotes_present {
        true => *start_index + 1,
        false => *start_index
    };
    let end_index_offset =  match quotes_present {
        true => 1,
        false => 0
    };

    let line_remainer = &line[local_start_index .. line.len()];
    let find_result = line_remainer.find(SEPARATOR);
    let end_index = if find_result.is_some() {
        find_result.unwrap()
    } else {
        line_remainer.len()
    };
    return if end_index > 0 {
        let mut int_str = &line_remainer[0..end_index - end_index_offset];
        let decimal_result = int_str.find(",");
        if decimal_result.is_some() {
            let decimal_index = decimal_result.unwrap();
            if decimal_index > 0 {
                int_str = &line_remainer[0..decimal_index];
            }
        }
        let parse_result = int_str.parse::<f32>();
        if parse_result.is_ok() {
            *target = parse_result.unwrap();
            Result::Ok(local_start_index + end_index + end_index_offset)
        } else {
            Result::Err(err_msg)
        }
    } else {
        Result::Err(err_msg)
    }
}
