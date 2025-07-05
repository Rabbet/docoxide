use clap::Parser;

mod errors;
mod http_server;
mod metrics;
mod office;
mod validation;

#[derive(Parser, Debug)]
#[command(author, version, about = "Document conversion service")]
struct Args {
    /// Path to LibreOffice program directory
    #[arg(short, long, default_value = "/usr/lib/libreoffice/program")]
    office_path: String,

    /// HTTP server port
    #[arg(short, long, default_value_t = 5555)]
    port: u16,
}

fn main() {
    let args = Args::parse();
    let _stdout_subscriber = tracing_subscriber::fmt::init();

    http_server::serve(args.office_path, args.port);
}
