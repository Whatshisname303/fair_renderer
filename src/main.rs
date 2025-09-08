use std::error::Error;
use std::path::PathBuf;
use std::{fmt, fs, io};

use yaml_rust2::Yaml;

#[derive(Debug)]
struct ArgumentError(String);

impl fmt::Display for ArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Argument error: {}", self.0)
    }
}

impl Error for ArgumentError {}

impl From<io::Error> for ArgumentError {
    fn from(value: io::Error) -> Self {
        ArgumentError(value.to_string())
    }
}
impl From<std::str::Utf8Error> for ArgumentError {
    fn from(_: std::str::Utf8Error) -> Self {
        ArgumentError("found invalid utf8".to_string())
    }
}
impl From<yaml_rust2::ScanError> for ArgumentError {
    fn from(_: yaml_rust2::ScanError) -> Self {
        ArgumentError("yaml scan error".to_string())
    }
}

struct CompanyEntry {
    name: String,
    description: String,
    location: String,
    website: String,
    logo_url: String,
    work_authorization: String,
    job_titles: String,
    job_types: Vec<String>,
    majors: Vec<String>,
    school_years: Vec<String>,
    attending_sessions: Vec<String>,
}

fn main() -> Result<(), ArgumentError> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    let is_verbose = match args.iter().position(|a| a == "-v" || a == "--verbose") {
        Some(idx) => {
            args.remove(idx);
            true
        },
        None => false,
    };
    let no_output = match args.iter().position(|a| a == "-no" || a == "--no-output") {
        Some(idx) => {
            args.remove(idx);
            true
        },
        None => false,
    };
    let is_help = match args.iter().position(|a| a == "-h" || a == "--help") {
        Some(idx) => {
            args.remove(idx);
            true
        },
        None => false,
    };

    if is_help {
        println!("Will show help later");
        return Ok(());
    }

    let input_file_path = match args.get(0) {
        Some(_) => args.remove(0),
        None => return Err(ArgumentError("no input path provided".to_string())),
    };

    let input_data = fs::read(input_file_path)?;

    let json_data: serde_json::Value = match serde_json::from_slice(&input_data) {
        Ok(data) => data,
        Err(e) => return Err(ArgumentError("failed parsing json".to_string())),
    };

    let json_entries = match &json_data["results"] {
        serde_json::Value::Array(entries) => entries,
        _ => return Err(ArgumentError("invalid json data".to_string())),
    };

    // parsing ////////////////////////////////////////////////////////////////

    // value to string
    let v2s = |v: &serde_json::Value, err: &str| {
        match v {
            serde_json::Value::String(inner) => Ok(inner.clone()),
            _ => Err(ArgumentError(err.to_string())),
        }
    };

    let mut companies = Vec::new();

    // should also include entry index in error
    for json_entry in json_entries {
        let name = v2s(&json_entry["employer"]["name"], "name")?;
        let description = v2s(&json_entry["company_description"], "description")?;
        let location = v2s(&json_entry["location_name"], "location")?;
        let website = v2s(&json_entry["employer"]["website"], "website")?;
        let logo_url = v2s(&json_entry["employer"]["logo_url"], "logo_url")?;
        let work_authorization = v2s(&json_entry["work_authorization_requirements"], "work_auth")?;
        let job_titles = v2s(&json_entry["job_titles"], "job_titles")?;

        let job_types: Result<Vec<String>, ArgumentError> = match &json_entry["job_types"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["name"], "job_type")).collect(),
            _ => return Err(ArgumentError("job_types".to_string())),
        };
        let majors: Result<Vec<String>, ArgumentError> = match &json_entry["majors"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["name"], "major")).collect(),
            _ => return Err(ArgumentError("majors".to_string())),
        };
        let school_years: Result<Vec<String>, ArgumentError> = match &json_entry["school_years"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["name"], "school_year")).collect(),
            _ => return Err(ArgumentError("school_years".to_string())),
        };
        let attending_sessions: Result<Vec<String>, ArgumentError> = match &json_entry["attending_career_fair_sessions"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["display_name"], "session")).collect(),
            _ => return Err(ArgumentError("sessions".to_string())),
        };

        companies.push(CompanyEntry {
            name,
            description,
            location,
            website,
            logo_url,
            work_authorization,
            job_titles,
            job_types: job_types?,
            majors: majors?,
            school_years: school_years?,
            attending_sessions: attending_sessions?,
        });
    }

    println!("rendering data for {} companies", companies.len());

    let template_path = "./vault_templates/career_fair_2025";

    let user_fields = match read_fileclass_yaml(PathBuf::from(template_path).join("classes/company.md")) {
        Some(fields) => fields,
        None => return Err(ArgumentError("failed reading fileClass yaml".to_string())),
    };

    if no_output {
        println!("Exiting with no output");
        return Ok(());
    }

    let output_path = match args.get(0) {
        Some(_) => args.remove(0),
        None => return Err(ArgumentError("no output path provided, pass --no-ouptput if intentional".to_string())),
    };

    copy_dir_recurse(template_path.into(), output_path.clone().into())?;
    // todo: append company data to yaml and write new file

    let companies_dir = PathBuf::from(output_path.clone()).join("companies");

    for (i, company) in companies.iter().enumerate() {
        let file_path = companies_dir.join(company.name.clone() + ".md");

        let mut file_text = "---\nfileClass: company\n".to_string();

        for field in &user_fields {
            file_text.push_str(field);
            file_text.push_str(": \n");
        }

        file_text.push_str(&format!("location: {}\n", company.location));
        file_text.push_str(&format!("majors: {}\n", company.majors.join(", ")));
        file_text.push_str(&format!("job_titles: {}\n", company.job_titles));
        file_text.push_str(&format!("job_types: {}\n", company.job_types.join(", ")));
        file_text.push_str(&format!("school_years: {}\n", company.school_years.join(", ")));
        file_text.push_str(&format!("international: {}\n", company.work_authorization));
        file_text.push_str(&format!("sessions: {}\n", company.attending_sessions.join(", ")));
        file_text.push_str(&format!("website: {}\n", company.website));

        // end frontmatter
        file_text.push_str("---\n\n");

        file_text.push_str(&format!("<img src=\"{}\" style=\"width: 80px;\">\n\n", company.logo_url));
        file_text.push_str(&format!("### Description\n\n{}\n", company.description));

        if fs::write(&file_path, &file_text).is_err() {
            let alt_path = companies_dir.join(format!("error{i}.md"));
            if is_verbose {
                println!("Failed to write: {}. Instead writing: {}", file_path.to_string_lossy(), alt_path.to_string_lossy());
            }
            file_text.push_str("==This file failed to write, likely because of an issue with the name. If everything else looks fine then you can set the name yourself==\n\n");
            file_text.push_str(&format!("**Company name:** {}\n", company.name));
            fs::write(alt_path, &file_text)?;
        }
    }

    Ok(())
}

fn read_fileclass_yaml(file_path: PathBuf) -> Option<Vec<String>> {
    let file_class_bytes = fs::read(file_path).ok()?;
    let file_class_str = std::str::from_utf8(clean_yaml_md_file(&file_class_bytes)).ok()?;
    let file_class_yaml = yaml_rust2::YamlLoader::load_from_str(file_class_str).ok()?;
    let file_class = file_class_yaml.first()?;

    let fields = file_class.as_hash()?.get(&Yaml::from_str("fields"))?.as_vec()?;

    let mut field_names = Vec::with_capacity(fields.len());

    for field in fields {
        field_names.push(field.as_hash()?.get(&Yaml::from_str("name"))?.as_str()?.to_owned());
    }

    Some(field_names)
}

fn append_company_data_to_fileclass() {
}

fn write_frontmatter() {
}

// ugly code to strip the --- off the start and end from inline yaml
fn clean_yaml_md_file(mut bytes: &[u8]) -> &[u8] {
    while bytes.len() > 1 && bytes[0] != b'\r' && bytes[0] != b'\n' {
        bytes = &bytes[1..];
    }
    bytes = &bytes[1..];

    if bytes[0] == b'\n' {
        bytes = &bytes[1..];
    }

    while bytes.len() > 1 && bytes[bytes.len() - 1] == b'-' {
        let n = bytes.len() - 1;
        bytes = &bytes[..n];
    }

    return bytes;
}

fn copy_dir_recurse(src: std::path::PathBuf, dst: std::path::PathBuf) -> io::Result<()> {
    fs::create_dir(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_recurse(entry.path(), dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}
