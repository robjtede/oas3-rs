//! Structures and tools to parse, navigate and validate [OpenAPI v3] specifications.
//!
//! # Example
//!
//! ```no_run
//! match oas3::from_path("path/to/openapi.yaml") {
//!   Ok(spec) => println!("spec: {:?}", spec),
//!   Err(err) => println!("error: {}", err)
//! }
//! ```
//!
//! # Errors
//!
//! Operations typically result in a `openapi::Result` Type which is an alias for Rust's built-in
//! Result with the Err Type fixed to the [`openapi::errors::Error`] enum type.
//!
//! [OpenAPI v3]: https://github.com/OAI/OpenAPI-Specification

#![allow(unused_imports, dead_code, unused_variables)]
#![warn(missing_debug_implementations)]
#![deny(rust_2018_idioms, nonstandard_style)]

use std::{fs::File, io::Read, path::Path};

use lazy_static::lazy_static;
use regex::Regex;

mod error;
mod path;
mod spec;

pub use error::Error;
pub use spec::{Schema, Spec};

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "conformance")]
pub mod conformance;

/// Version 3.0.1 of the OpenApi specification.
///
/// Refer to the official
/// [specification](https://github.com/OAI/OpenAPI-Specification/blob/0dd79f6/versions/3.0.1.md)
/// for more information.
pub type OpenApiV3Spec = spec::Spec;

/// deserialize an open api spec from a path
pub fn from_path<P>(path: P) -> Result<OpenApiV3Spec, Error>
where
    P: AsRef<Path>,
{
    from_reader(File::open(path)?)
}

/// deserialize an open api spec from type which implements Read
pub fn from_reader<R>(read: R) -> Result<OpenApiV3Spec, Error>
where
    R: Read,
{
    Ok(serde_yaml::from_reader::<R, OpenApiV3Spec>(read)?)
}

/// serialize to a yaml string
pub fn to_yaml(spec: &OpenApiV3Spec) -> Result<String, Error> {
    Ok(serde_yaml::to_string(spec)?)
}

/// serialize to a json string
pub fn to_json(spec: &OpenApiV3Spec) -> Result<String, Error> {
    Ok(serde_json::to_string_pretty(spec)?)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, read_to_string, File},
        io::Write,
        path,
    };

    use pretty_assertions::assert_eq;

    use super::*;

    /// Helper function to write string to file.
    fn write_to_file<P>(path: P, filename: &str, data: &str)
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        println!("    Saving string to {:?}...", path);
        std::fs::create_dir_all(&path).unwrap();
        let full_filename = path.as_ref().to_path_buf().join(filename);
        let mut f = File::create(&full_filename).unwrap();
        f.write_all(data.as_bytes()).unwrap();
    }

    /// Convert a YAML `&str` to a JSON `String`.
    fn convert_yaml_str_to_json(yaml_str: &str) -> String {
        let yaml: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        let json: serde_json::Value = serde_yaml::from_value(yaml).unwrap();
        serde_json::to_string_pretty(&json).unwrap()
    }

    /// Deserialize and re-serialize the input file to a JSON string through two different
    /// paths, comparing the result.
    /// 1. File -> `String` -> `serde_yaml::Value` -> `serde_json::Value` -> `String`
    /// 2. File -> `Spec` -> `serde_json::Value` -> `String`
    ///
    /// Both conversion of `serde_json::Value` -> `String` are done
    /// using `serde_json::to_string_pretty`.
    /// Since the first conversion is independent of the current crate (and only
    /// uses serde json and yaml support), no information should be lost in the final
    /// JSON string. The second conversion goes through our `OpenApi`, so the final JSON
    /// string is a representation of _our_ implementation.
    /// By comparing those two JSON conversions, we can validate our implementation.
    fn compare_spec_through_json(
        input_file: &Path,
        save_path_base: &Path,
    ) -> (String, String, String) {
        // First conversion:
        //     File -> `String` -> `serde_yaml::Value` -> `serde_json::Value` -> `String`

        // Read the original file to string
        let spec_yaml_str = read_to_string(&input_file)
            .unwrap_or_else(|e| panic!("failed to read contents of {:?}: {}", input_file, e));
        // Convert YAML string to JSON string
        let spec_json_str = convert_yaml_str_to_json(&spec_yaml_str);

        // Second conversion:
        //     File -> `Spec` -> `serde_json::Value` -> `String`

        // Parse the input file
        let parsed_spec = from_path(&input_file).unwrap();
        // Convert to serde_json::Value
        let parsed_spec_json = serde_json::to_value(parsed_spec).unwrap();
        // Convert to a JSON string
        let parsed_spec_json_str: String = serde_json::to_string_pretty(&parsed_spec_json).unwrap();

        // Save JSON strings to file
        let api_filename = input_file
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(".yaml", ".json");

        let mut save_path = save_path_base.to_path_buf();
        save_path.push("yaml_to_json");
        write_to_file(&save_path, &api_filename, &spec_json_str);

        let mut save_path = save_path_base.to_path_buf();
        save_path.push("yaml_to_spec_to_json");
        write_to_file(&save_path, &api_filename, &parsed_spec_json_str);

        // Return the JSON filename and the two JSON strings
        (api_filename, parsed_spec_json_str, spec_json_str)
    }

    #[test]
    #[ignore = "lib does not support all schema attributes yet"]
    fn test_serialization_round_trip() {
        let save_path_base: path::PathBuf = ["target", "tests", "test_serialization_round_trip"]
            .iter()
            .collect();

        for entry in fs::read_dir("data/oas-samples").unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            println!("Testing if {:?} is deserializable", path);

            let (api_filename, parsed_spec_json_str, spec_json_str) =
                compare_spec_through_json(&path, &save_path_base);

            assert_eq!(
                parsed_spec_json_str.lines().collect::<Vec<_>>(),
                spec_json_str.lines().collect::<Vec<_>>(),
                "contents did not match for api {}",
                api_filename
            );
        }
    }
}
