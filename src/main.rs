use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use ron::de::from_reader;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Serialize, Deserialize, Debug)]
struct Service {
    binary_path: String,
    pid: Option<u32>,
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(subcommand)]
    action: Action,
}

#[derive(StructOpt)]
enum Action {
    Start {
        /// The path to the binary to run as a service
        binary_path: Service,
    },
    Stop {
        /// The name of the service to stop
        service_name: Service,
    },
}

impl FromStr for Service {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Service {
            binary_path: s.to_string(),
            pid: None,
        })
    }
}

impl Action {
    fn run(self) {
        match self {
            Action::Start { binary_path } => start_service(binary_path),
            Action::Stop { service_name } => stop_service(service_name),
        }
    }
}

fn main() {
    let args = Cli::from_args();
    args.action.run();
}

fn start_service(binary_path: Service) {
    let mut services = load_services();

    if services.iter().any(|s| s.binary_path == binary_path.binary_path) {
        eprintln!("Service with binary path {} already exists", binary_path.binary_path);
    } else {
        let child = Command::new(&binary_path.binary_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start service");

        let service_name = binary_path.binary_path.clone();
        let mut service = binary_path;
        service.pid = Some(child.id());
        services.push(service);
        save_services(&services);
        println!("Service with binary path {} started", service_name);
    }
}

fn stop_service(service_name: Service) {
    let mut services = load_services();

    if let Some(index) = services.iter().position(|s| s.binary_path == service_name.binary_path) {
        let service = &services[index];
        let pid = service.pid.expect("Service PID not found");

        Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .expect("Failed to stop service");

        services.remove(index);
        save_services(&services);
        println!("Service with binary path {} stopped", service_name.binary_path);
    } else {
        panic!("Service with binary path {} not found", service_name.binary_path);
    }
}

fn load_services() -> Vec<Service> {
    let path = get_config_path();
    if path.exists() {
        let file = File::open(&path).expect("Failed to open file");
        from_reader(file).expect("Failed to read file")
    } else {
        Vec::new()
    }
}

fn save_services(services: &[Service]) {
    let path = get_config_path();
    let pretty = PrettyConfig::new();
    let data = to_string_pretty(services, pretty).expect("Failed to serialize data");
    let mut file = File::create(&path).expect("Failed to create file");
    file.write_all(data.as_bytes()).expect("Failed to write file");
}

#[allow(deprecated)]
fn get_config_path() -> PathBuf {
    let mut path = env::home_dir().expect("Failed to get home directory");
    path.push(".config");
    path.push("cargo-service");
    fs::create_dir_all(&path).expect("Failed to create directory");
    path.push("cache.ron");
    path
}