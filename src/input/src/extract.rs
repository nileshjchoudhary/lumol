// Lumol, an extensible molecular simulation engine
// Copyright (C) 2015-2016 G. Fraux — BSD license
use toml::{Table, Value};
use error::{Error, Result};


/// Extract the table at the given `key`, from the `config` TOML table
/// interpreted as a `context`.
pub fn table<'a>(key: &str, config: &'a Table, context: &str) -> Result<&'a Table> {
    let table = try!(config.get(key).ok_or(Error::from(
        format!("Missing '{}' key in {}", key, context)
    )));
    return table.as_table().ok_or(Error::from(
        format!("'{}' must be a table in {}", key, context)
    ));
}

/// Extract the string at the given `key`, from the `config` TOML table
/// interpreted as a `context`
pub fn str<'a>(key: &str, config: &'a Table, context: &str) -> Result<&'a str> {
    let string = try!(config.get(key).ok_or(Error::from(
        format!("Missing '{}' key in {}", key, context)
    )));
    return string.as_str().ok_or(Error::from(
        format!("'{}' must be a string in {}", key, context)
    ));
}

/// Extract a number (integer or float) at the given `key`, from the `config`
/// TOML table interpreted as a `context`
pub fn number(key: &str, config: &Table, context: &str) -> Result<f64> {
    let number = try!(config.get(key).ok_or(Error::from(
        format!("Missing '{}' key in {}", key, context)
    )));
    match *number {
        ::toml::Value::Integer(v) => Ok(v as f64),
        ::toml::Value::Float(v) => Ok(v),
        _ => Err(Error::from(
            format!("'{}' must be a number in {}", key, context)
        ))
    }
}

/// Extract an array at the given `key`, from the `config` TOML table
/// interpreted as a `context`
pub fn slice<'a>(key: &str, config: &'a Table, context: &str) -> Result<&'a [Value]> {
    let array = try!(config.get(key).ok_or(Error::from(
        format!("Missing '{}' key in {}", key, context)
    )));
    return array.as_slice().ok_or(Error::from(
        format!("'{}' must be an array in {}", key, context)
    ))
}

/// Extract the string 'type' key in a TOML table
pub fn typ<'a>(config: &'a Table, context: &str) -> Result<&'a str> {
    let typ = try!(config.get("type").ok_or(Error::from(
        format!("Missing 'type' key in {}", context)
    )));
    return typ.as_str().ok_or(Error::from(
        format!("'type' key must be a string in {}", context)
    ));
}
