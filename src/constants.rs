use core::hint;

static mut CONSTANTS: Option<Constants> = None;

pub struct Constants{
	pub discord_token:String,
	pub default_bot_logging_guild_id:u64,
	pub default_bot_logging_channel_id:u64,
	pub err_bot_guild_invalid:String,
	pub err_bot_channel_invalid:String,
	
	pub default_webhook_id:u64,
	pub default_webhook_token:String,
	pub webhook_err_str:String,
	pub webhook_uninit:String,
	pub webhook_rx:String,
	pub webhook_tx:String,
	pub webhook_closed: String
}
impl Constants{
	fn new() -> Self {
		tracing::info!("Reading .env!");
		let guild_id = get_dotenv_id::<u64>("LOGGING_GUILD_ID");
		let channel_id = get_dotenv_id::<u64>("LOGGING_CHANNEL_ID");
		Constants {
			discord_token:get_dotenv("DISCORD_TOKEN"),
			default_bot_logging_guild_id: guild_id,
			default_bot_logging_channel_id: channel_id,
			err_bot_guild_invalid: format!("The Chanel referenced by 'default_bot_logging_guild_id' (which is the guild, with the id `{}`) does not exist!", guild_id),
			err_bot_channel_invalid: format!("The Chanel referenced by 'default_bot_logging_channel_id' (which is the channel, with the id {}) does not exist!", channel_id),
			default_webhook_id: get_dotenv_id::<u64>("WEBHOOK_ID"),
			default_webhook_token: get_dotenv("WEBHOOK_TOKEN"),
			webhook_err_str: "Webhook Error detected. Not Proceeding any further, as debugging messages could be dropped!".to_string(),
			webhook_uninit: "Did you init the webhook?".to_string(),
			webhook_rx: "Is the webhook init? Seems like it is, but rx channel not available.".to_string(),
			webhook_tx: "Is the webhook init? Seems like it is, but tx channel not available.".to_string(),
			webhook_closed:"Channels got closed unexpectedly!".to_string(),
		}
	}
	pub fn get_constants() -> &'static Self{
		unsafe {
			if let None = CONSTANTS {
				CONSTANTS=Some(Constants::new())
			}
		}
		match unsafe { &CONSTANTS }.as_ref(){
			Some(v) => v,
			None => unsafe { hint::unreachable_unchecked() }
		}
	}
}
fn get_dotenv(name:&str) -> String{
	dotenv::var(name).expect(format!("There is some error with the setting {} in dotenv!",name).as_str())
}
fn get_dotenv_id<T: std::str::FromStr>(name:&str) -> T{
	let type_name = std::any::type_name::<T>();
	get_dotenv(name).parse::<T>().unwrap_or_else(|_|panic!("The String was correctly read, but could not be parsed into {}",type_name,))
}