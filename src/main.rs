use std::path::PathBuf;
use std::{fmt, fs, io};

use yaml_rust2::{Yaml, YamlEmitter};
use yaml_rust2::yaml::Hash;

#[derive(Debug)]
struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error(format!("io error: {}", value))
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

struct CliArgs {
    input_path: String,
    output_path: Option<String>,
    template_path: Option<String>,
    verbose: bool,
}

// will exit program early if --help is passed, I do not care
fn parse_cli() -> Result<CliArgs, Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let is_help = args.iter().any(|a| a == "-h" || a == "--help") || args.is_empty();
    if is_help {
        print_help_msg();
        std::process::exit(0);
    }

    let is_verbose = args.iter().any(|a| a == "-v" || a == "--verbose");

    let input_data_path = match args.iter().position(|a| a == "-i" || a == "--input") {
        Some(idx) => {
            match args.get(idx + 1) {
                Some(value) if !value.starts_with('-') => Ok(value.clone()),
                _ => Err(Error(format!("expected a value for {}", args[idx]))),
            }
        },
        None => Err(Error(format!("must supply input data: --input [path_to_input]"))),
    };
    let template_data_path = match args.iter().position(|a| a == "-t" || a == "--template") {
        Some(idx) => {
            match args.get(idx + 1) {
                Some(value) if !value.starts_with('-') => Ok(Some(value.clone())),
                _ => Err(Error(format!("expected a value for {}", args[idx]))),
            }
        },
        None => Ok(None),
    };
    let output_data_path = match args.iter().position(|a| a == "-o" || a == "--output") {
        Some(idx) => {
            match args.get(idx + 1) {
                Some(value) if !value.starts_with('-') => Ok(Some(value.clone())),
                _ => Err(Error(format!("expected a value for {}", args[idx]))),
            }
        },
        None => Ok(None),
    };

    Ok(CliArgs {
        input_path: input_data_path?,
        output_path: output_data_path?,
        template_path: template_data_path?,
        verbose: is_verbose,
    })
}

fn main() {
    match real_main() {
        Ok(()) => {},
        Err(e) => println!("{}", e)
    };
}

// wrapper so that main prints Error Display rather than Debug
fn real_main() -> Result<(), Error> {
        let cli_args = parse_cli()?;

    let input_data = fs::read(&cli_args.input_path)?;

    let json_data: serde_json::Value = match serde_json::from_slice(&input_data) {
        Ok(data) => data,
        Err(_) => return Err(Error("input data is invalid json".to_string())),
    };

    let json_entries = match &json_data["results"] {
        serde_json::Value::Array(entries) => entries,
        _ => return Err(Error("input data is an invalid format".to_string())),
    };

    // value to string
    let v2s = |v: &serde_json::Value, err: &str| {
        match v {
            serde_json::Value::String(inner) => Ok(inner.clone()),
            _ => Err(Error(format!("json missing field: {}", err))),
        }
    };

    let mut companies = Vec::new();

    // maybe should also include entry index in error
    for json_entry in json_entries {
        let name = v2s(&json_entry["employer"]["name"], "name")?;
        let description = v2s(&json_entry["company_description"], "description")?;
        let location = v2s(&json_entry["location_name"], "location")?;
        let website = v2s(&json_entry["employer"]["website"], "website")?;
        let logo_url = v2s(&json_entry["employer"]["logo_url"], "logo_url")?;
        let work_authorization = v2s(&json_entry["work_authorization_requirements"], "work_auth")?;
        let job_titles = v2s(&json_entry["job_titles"], "job_titles")?;

        let job_types: Result<Vec<String>, Error> = match &json_entry["job_types"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["name"], "job_type")).collect(),
            _ => return Err(Error("json missing field: job_types".to_string())),
        };
        let majors: Result<Vec<String>, Error> = match &json_entry["majors"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["name"], "major")).collect(),
            _ => return Err(Error("json missing field: majors".to_string())),
        };
        let school_years: Result<Vec<String>, Error> = match &json_entry["school_years"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["name"], "school_year")).collect(),
            _ => return Err(Error("json missing field: school_years".to_string())),
        };
        let attending_sessions: Result<Vec<String>, Error> = match &json_entry["attending_career_fair_sessions"] {
            serde_json::Value::Array(arr) => arr.iter().map(|entry| v2s(&entry["display_name"], "session")).collect(),
            _ => return Err(Error("json missing field: sessions".to_string())),
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

    if cli_args.verbose {
        println!("rendering data for {} companies", companies.len());
    }

    let template_path = match &cli_args.template_path {
        Some(path) => path,
        None => "./vault_templates/career_fair_2025_template",
    };

    let file_class_bytes = match fs::read(PathBuf::from(template_path).join("classes/company.md")) {
        Ok(bytes) => bytes,
        Err(e) => return Err(Error(format!("could not read template path: {}", e))),
    };

    let (user_fields, new_fileclass) = match read_fileclass_yaml(&file_class_bytes) {
        Some((fields, fileclass)) => (fields, fileclass),
        None => return Err(Error("failed reading fileClass".to_string())),
    };

    let output_path = match cli_args.output_path {
        Some(path) => path,
        None => {
            println!("Exiting with no output");
            return Ok(())
        },
    };

    if let Err(e) = copy_dir_recurse(template_path.into(), output_path.clone().into()) {
        return Err(Error(format!("failed copying template to output path: {}", e)));
    };
    fs::write(PathBuf::from(output_path.clone()).join("classes/company.md"), new_fileclass)?;

    let companies_dir = PathBuf::from(output_path.clone()).join("companies");
    fs::create_dir_all(&companies_dir)?;

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
            if cli_args.verbose {
                println!("Failed to write: {}. Instead writing: {}", file_path.to_string_lossy(), alt_path.to_string_lossy());
            }
            file_text.push_str("==This file failed to write, likely because of an issue with the name. If everything else looks fine then you can set the name yourself==\n\n");
            file_text.push_str(&format!("**Company name:** {}\n", company.name));
            if fs::write(alt_path, &file_text).is_err() {
                return Err(Error("unable to write company file".to_string()));
            }
        }
    }

    Ok(())
}

fn read_fileclass_yaml(file_class_bytes: &[u8]) -> Option<(Vec<String>, String)> {
    let file_class_str = std::str::from_utf8(clean_yaml_md_file(file_class_bytes)).ok()?;
    let mut file_class_yaml = yaml_rust2::YamlLoader::load_from_str(file_class_str).ok()?;
    let file_class = file_class_yaml.first_mut()?.as_mut_hash()?;

    let fields = file_class.get_mut(&Yaml::String("fields".to_string()))?.as_mut_vec()?;

    let mut field_names = Vec::with_capacity(fields.len());

    for field in fields.iter() {
        field_names.push(field.as_hash()?.get(&Yaml::from_str("name"))?.as_str()?.to_owned());
    }

    let field_strings = [
        "location", "majors", "job_titles", "job_types", "school_years",
        "international", "sessions", "website",
    ];
    let mut id = [b'a', b'b', b'c', b'd', b'e', b'f'];

    for st in field_strings {
        let mut hash = Hash::new();
        hash.insert(Yaml::String("name".to_string()), Yaml::String(st.to_string()));
        hash.insert(Yaml::String("type".to_string()), Yaml::String("Input".to_string()));
        hash.insert(Yaml::String("options".to_string()), Yaml::Hash(Hash::new()));
        hash.insert(Yaml::String("path".to_string()), Yaml::String("".to_string()));
        hash.insert(Yaml::String("id".to_string()), Yaml::String(std::str::from_utf8(&id).unwrap().to_string()));

        id[0] += 1;
        fields.push(Yaml::Hash(hash));
    }

    let mut id = [b'a', b'b', b'c', b'd', b'e', b'f'];

    // second loop needed to drop mutable reference (fields)
    for _ in field_strings {
        file_class.get_mut(&Yaml::String("fieldsOrder".to_string()))?
            .as_mut_vec()?
            .push(Yaml::String(std::str::from_utf8(&id).unwrap().to_string()));
        id[0] += 1;
    }

    let mut processed_fileclass = String::new();
    let mut emitter = YamlEmitter::new(&mut processed_fileclass);
    emitter.dump(file_class_yaml.first().unwrap()).ok()?;
    processed_fileclass.push_str("\n---"); // misses this for some reason

    Some((field_names, processed_fileclass))
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

fn print_help_msg() {
    let msg = ["This tool generates an obsidian vault based on career fair data\n",
    "\n",
    "Basic usage: cargo run -- --input [path_to_input_data] --out [path_to_put_vault]\n",
    "\n",
    "Arguments:\n",
    "   -i/--input [path_to_input_data] : required path to the json that contains the data to render\n",
    "   -o/--out [output_path]          : required path to put the generated vault\n",
    "   -t/--template [template_path]   : optional path to the template vault or will use a default\n",
    "   -v/--verbose                    : optional prints more debug info\n",
    "   -h/--help                       : prints this message\n"];
    println!("{}", msg.concat());
}
