mod constants;
mod logger;
mod client;

use constants::Constants;
use logger::Logger;
use logger::LoggerWriter;

use tracing::{error,warn, info, debug};
use tracing_subscriber::{FmtSubscriber, EnvFilter, registry};
use tokio::runtime::Runtime;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::fmt::Layer;


#[must_use="What is the point of getting a Runtime, and not doing anything?"]
pub fn get_rt() -> &'static Runtime{
	static mut rt:Option<Runtime> = None;
	unsafe { &mut rt }.get_or_insert_with(||tokio::runtime::Runtime::new().unwrap())
}

fn main() {
	//Tie the Runtime to this main.
	//Must use for tokio::spawn to not panic
	let _rtg = get_rt().enter();
	// let rt_logger = tokio::runtime::Runtime::new().unwrap();
	
	// This will load the environment variables located at `./.env`, relative to
	// the CWD. See `./.env.example` for an example on how to structure this.
	dotenv::dotenv().expect("Failed to load .env file");
	
	// Initialize the logger to use environment variables.
	//
	// In this case, a good default is setting the environment variable
	// `RUST_LOG` to debug`.
	
	
	// let subscriber = FmtSubscriber::builder()
	// 	.with_env_filter(EnvFilter::from_default_env())
	// 	.finish();
	
	// let webhook_sub = FmtSubscriber::builder()
	// 	.with_writer(LoggerWriter::logger_writer)
	// 	.with_env_filter(EnvFilter::from_env("DISCORD_LOG"))
	// 	.compact()
	// 	.with_ansi(false)
	// 	.finish();
	
	
	let subscriber = Layer::new();
	tokio::spawn(Logger::logger().say_str(None,"Prefire Async Webhook Message, to create Async Webhook Writer".to_string()));
	LoggerWriter::logger_writer().say_str_sync("Prefire Sync Webhook Message, to create Sync Webhook Writer".to_string());
	
	let webhook_sub = Layer::new()
		.with_writer(LoggerWriter::logger_writer)
		.compact()
		.with_ansi(false);
	
	
	// tracing::info!("about to add the webhook logging. stdio logging should be suspended?");
	// tracing::subscriber::set_global_default(webhook_sub).expect("dsfnsdafsedanogvfanosgsog");
	// tracing::info!("added the webhook logging. stdio logging should now be suspended?");
	//
	
	let hc = libhoney::Config{
		options: libhoney::client::Options{
			api_key: "09ce78c7c38de75712bc9f9de35d9913".to_string(),
			dataset: "dataset-name".to_string(),
			..libhoney::client::Options::default()
		},
		transmission_options: Default::default()
	};
	
	let ht = tracing_honeycomb::new_honeycomb_telemetry_layer("C0D3-Bot-service",hc);
	
	// NOTE: the underlying subscriber MUST be the Registry subscriber
	let subscriber = registry::Registry::default() // provide underlying span data store
		.with(LevelFilter::INFO) // filter out low-level debug tracing (eg tokio executor)
		.with(subscriber) // log to stdout
		.with(webhook_sub)
		.with(ht); // publish to honeycomb backend
	
	tracing::subscriber::set_global_default(subscriber).expect("setting global default failed");
	
	tracing::info!("Client Startup");
	get_rt().block_on(client::init_client());
	tracing::debug!("Bye!");
	
}