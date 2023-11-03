use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::collections::HashMap;
use chrono::{NaiveDateTime, NaiveTime, NaiveDate};
use serde::Serialize;

const SEPARATOR :&str = ",";

const TIMESTAMP_FORMAT_1 :&str = "%Y-%m-%d %H:%M:%S";
const TIMESTAMP_FORMAT_2 :&str = "%d/%m/%Y %H:%M";
const TIMESTAMP_FORMAT_3 :&str = "%Y-%m-%dT%H:%M:%S%Z";
const TIMESTAMP_FORMATS :[&str;3] = [
    TIMESTAMP_FORMAT_1,
    TIMESTAMP_FORMAT_2,
    TIMESTAMP_FORMAT_3,
];

const TIME_FORMAT_1 :&str = "%H:%M";
const TIME_FORMATS :[&str;1] = [
    TIME_FORMAT_1,
];

const DATE_FORMAT_1 :&str = "%a %d %b %Y";
const DATE_FORMATS :[&str;1] = [
    DATE_FORMAT_1,
];


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


pub fn parse_file(file :&File, fallback_name :&str, file_entries :&mut HashMap<String, Vec<Entry>>) {
    let reader = BufReader::new(file);

    let mut index_to_name :HashMap<usize, String> = HashMap::new();
    let mut line_counter = 0;
    for line_result in reader.lines() {
        if line_result.is_err() {
            print!("could not read line: ");
            println!("{}", line_result.unwrap_err());
            continue;
        }
        let line = line_result.unwrap();
        if line_counter == 0 {
            let mut names :Vec<String> = Vec::new();
            let header_result = parse_header_line(&line, &mut names);
            if header_result.is_err() {
                println!("{}", header_result.unwrap_err());
                break;
            }
            let mut i = 0;
            for name in names {
                if i == 0 {
                    i = i + 1;
                    continue; // ignore first name, as that is timestamp
                }
                let name_to_use :String =
                    if name.len() == 0 { fallback_name.to_string() }
                    else { name };
                file_entries.insert(name_to_use.clone(), Vec::new());
                index_to_name.insert(i - 1, name_to_use);
                i = i + 1;
            }
        } else {
            let parse_result = parse_line(&line, &line_counter);
            let line_error = parse_result.1;
            if !line_error {
                let entries = parse_result.0;
                for i in 0..entries.len() {
                    let name_opt = index_to_name.get(&i);
                    if name_opt.is_some() {
                        let name = name_opt.unwrap();
                        let column_entries = file_entries.get_mut(name).unwrap();
                        column_entries.push(entries.get(i).unwrap().clone());
                    } else {
                        println!("bad csv file, invalid columns");
                    }
                }
            }
        }
        line_counter += 1;
    }
}

fn parse_header_line(line :&String, names: & mut Vec<String>) -> Result<bool, &'static str> {
    let first_char_result = (*line).chars().nth(0);
    if first_char_result.is_none() {
        return Result::Err("empty value");
    }
    let quotes_present = first_char_result.unwrap() == '"';
    let start_index_offset = match quotes_present {
        true => 1,
        false => 0
    };
    let end_index_offset =  match quotes_present {
        true => 1,
        false => 0
    };
    let mut line_index = 0;

    let line_len = line.len();
    while line_index <= line_len {
        let line_remainer = &line[line_index .. line.len()];
        let find_result = line_remainer.find(SEPARATOR);
        let index =
            if find_result.is_some() { find_result.unwrap() }
            else { line_remainer.len() };
        
        let name: &str = &line_remainer[start_index_offset..index - end_index_offset];
        names.push(String::from(name));
        line_index = start_index_offset + line_index + name.len() + 1 + end_index_offset;
    }
    return Result::Ok(true);
}

fn parse_line(line :&String, line_counter:&u32) -> (Vec<Entry>, bool) {
    //println!("parsing line (num: {}, len: {}): {}", line_counter, line.len(), line);
    let mut entries = Vec::new();
    let mut line_error = false;
    let mut index :usize = 0;
    let mut err_msg :&str = "";
    let mut _err_msg_full :String = String::new();
    let mut timestamp = 0;

    let parse_result = parse_timestamp(line, & mut timestamp, &index);
    if parse_result.is_ok() {
        index = parse_result.unwrap();
    } else {
        line_error = true;
        let err_tupel = parse_result.err().unwrap();
        err_msg = err_tupel.0;
        if err_tupel.1.is_some() {
            _err_msg_full = err_msg.to_owned() + ": '" + &err_tupel.1.unwrap() + "'";
            err_msg = &_err_msg_full;
        }
    }

    while index < line.len() && !line_error {
        let mut entry = Entry::new();
        entry.timestamp = timestamp;
        let parse_result = parse_float(line, &mut entry.value, &mut index, "could not parse: value float");
        if parse_result.is_ok() {
            entries.push(entry);
        } else {
            line_error = true;
            err_msg = parse_result.err().unwrap();
        }
    }

    if line_error && *line_counter > 2 {
        println!("error parsing line (number: {}, idx: {}, len: {}): {}", line_counter, index, line.len(), err_msg);
        println!("\t{}", line);
    }
    return (entries, line_error);
}

fn parse_timestamp(line :&String, result :&mut u64, start_index :&usize) -> Result<usize, (&'static str, Option<String>)> {
    let first_char_result = (*line).chars().nth(0);
    if first_char_result.is_none() {
        return Result::Err(("empty value", None));
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
    let mut time_str_opt = None;
    if find_result.is_some() {
        let index = find_result.unwrap();
        if index > 0 {
            let timestamp_str: &str = &line_remainer[0..index - end_index_offset];
            let timestamp_str_trimmed = timestamp_str.trim();
            time_str_opt = Some(timestamp_str_trimmed);
            let success_index = local_start_index + index + end_index_offset + 1; // +1 for csv separator
            // try date-time formats
            for timestamp_format in TIMESTAMP_FORMATS {
                let parse_result = NaiveDateTime::parse_from_str(timestamp_str_trimmed, timestamp_format);
                if parse_result.is_ok() {
                    let timestamp = parse_result.unwrap();
                    *result = timestamp.timestamp_millis() as u64;
                    return Result::Ok(success_index);
                }
            }
            // try time formats
            for time_format in TIME_FORMATS {
                let parse_result = NaiveTime::parse_from_str(timestamp_str_trimmed, time_format);
                if parse_result.is_ok() {
                    let time = parse_result.unwrap();
                    let date_opt = NaiveDate::from_ymd_opt(1970, 1, 1);
                    if date_opt.is_some() {
                        *result = date_opt.unwrap().and_time(time).timestamp_millis() as u64;
                        return Result::Ok(success_index);
                    }
                }
            }
            // try date formats
            for date_format in DATE_FORMATS {
                let millis_opt = parse_date(timestamp_str_trimmed, date_format);
                if millis_opt.is_some() {
                    *result = millis_opt.unwrap();
                    return Result::Ok(success_index);
                }
            }
            // try date formats with trailing year
            let now = chrono::offset::Local::now();
            let time_with_year = timestamp_str_trimmed.to_owned() + " " + &now.format("%Y").to_string();
            for date_format in DATE_FORMATS {
                let millis_opt = parse_date(&time_with_year, date_format);
                if millis_opt.is_some() {
                    *result = millis_opt.unwrap();
                    return Result::Ok(success_index);
                }
            }
        }
    }
    if time_str_opt.is_some() {
        let return_time = Some(time_str_opt.unwrap().to_owned());
        return Result::Err((err_msg, return_time));
    }
    return Result::Err((err_msg, None));
}

fn parse_date(date: &str, date_format: &str) -> Option<u64> {
    let parse_result = NaiveDate::parse_from_str(date, date_format);
    if parse_result.is_ok() {
        let date = parse_result.unwrap();
        let millis_opt = date.and_hms_milli_opt(0, 0, 0, 0);
        if millis_opt.is_some() {
            return Some(millis_opt.unwrap().timestamp_millis() as u64);
        }
    }
    return None;
}

fn parse_float(line :&String, target :&mut f32, start_index :&mut usize, err_msg :&'static str) -> Result<usize, &'static str> {
    let line_remainer = &line[*start_index .. line.len()];
    let find_result = line_remainer.find(SEPARATOR);
    let end_index = if find_result.is_some() {
        find_result.unwrap()
    } else {
        line_remainer.len()
    };
    return if end_index > 0 {
        let first_char_result = (line_remainer).chars().nth(0);
        if first_char_result.is_none() {
            return Result::Err("empty value");
        }
        let quotes_present = first_char_result.unwrap() == '"';
        let local_start_index = match quotes_present {
            true => 1,
            false => 0
        };
        let end_index_offset =  match quotes_present {
            true => 1,
            false => 0
        };

        //println!("\tline_remainer: {}", line_remainer);

        if end_index - end_index_offset < 1 {
            return Result::Ok(line.len());
        }

        let mut int_str = &line_remainer[local_start_index .. end_index - end_index_offset];
        let decimal_result = int_str.find(",");
        if decimal_result.is_some() {
            let decimal_index = decimal_result.unwrap();
            if decimal_index > 0 {
                int_str = &line_remainer[0..decimal_index];
            }
        }
        //println!("\ttrying to parse float: {}", int_str);
        let number_str_trimmed = int_str.trim();
        let parse_result = number_str_trimmed.parse::<f32>();
        if parse_result.is_ok() {
            *target = parse_result.unwrap();
            *start_index = *start_index + int_str.len() + end_index_offset + 1;
            Result::Ok(*start_index)
        } else {
            Result::Err(err_msg)
        }
    } else {
        Result::Err(err_msg)
    }
}
