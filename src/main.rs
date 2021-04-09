mod constants;
mod logger;

use constants::Constants;
use logger::Logger;

use std::{
	collections::HashSet,
	sync::Arc,
};

use tracing::{error,warn, info, debug};
use tracing_subscriber::{
	FmtSubscriber,
	EnvFilter,
};
use serenity::{model::guild::Guild, model::id::ChannelId, model::channel::Message, async_trait, client::bridge::gateway::ShardManager, framework::{
	StandardFramework,
	standard::macros::{group, command},
}, http::Http, model::gateway::Ready, prelude::*, client::validate_token};
use serenity::model::id::GuildId;
use serenity::model::channel::GuildChannel;
use serenity::futures::TryFutureExt;
use std::collections::HashMap;
use std::borrow::Borrow;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
	type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[group]
#[commands(ping)]
struct General;

#[async_trait]
impl serenity::client::EventHandler for Handler {
	async fn ready(&self, ctx: Context, ready: Ready) {
		info!("{} is connected!", ready.user.name);
		let cty = ctx.clone();
		let http = cty.http.clone();
		let http_ref = http.as_ref();
		let guilds = cty
			.cache
			.guilds()
			.await;
		
		for guild_i in &guilds {
			info!("Got guild Id:{}", guild_i.as_u64())
		}
		
		let tmp = Guild::get(
			cty
				.http
				.clone()
				.as_ref(),
			GuildId::from(Constants::get_constants().default_bot_logging_guild_id),
		).and_then(|guild| async move {
           info!("Got Guild!");
			guild
				// .as_ref()
				// .expect(constants::Constants::get_constants().err_bot_guild_invalid.as_str())
				.channels(http_ref).await
			}
		).inspect_err(|_|
			panic!("{}",serenity::Error::Other(Constants::get_constants().err_bot_guild_invalid.as_str()))
		).and_then(|channels:HashMap<ChannelId, GuildChannel>| async move {
			info!("Got channels!");
			let channel = channels.get(&ChannelId::from(Constants::get_constants().default_bot_logging_channel_id));
			if channel.is_some(){
				info!("Got a valid logging Channel!");
			}else{
				panic!("{}",Constants::get_constants().err_bot_channel_invalid.as_str());
			}
			channel
				.unwrap()
				.say(ctx, format!("Bot {}#{} Shard {} startup", ready.user.name, ready.user.discriminator, cty.shard_id))
				.await
		}).inspect_err(|_|
			panic!("Unable to send a message!")
		).await;
		
		match tmp {
			Ok(v)=>drop(v),
			Err(_)=>unreachable!()
			}
	}
}

fn main() {
	// let rt_logger = tokio::runtime::Runtime::new().unwrap();
	let rt = tokio::runtime::Runtime::new().unwrap();
	// This will load the environment variables located at `./.env`, relative to
	// the CWD. See `./.env.example` for an example on how to structure this.
	dotenv::dotenv().expect("Failed to load .env file");
	
	// Initialize the logger to use environment variables.
	//
	// In this case, a good default is setting the environment variable
	// `RUST_LOG` to debug`.
	let subscriber = FmtSubscriber::builder()
		.with_env_filter(EnvFilter::from_default_env())
		.finish();
	
	tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");
	
	let webhook_sub = FmtSubscriber::builder()
		.with_writer(||{
			Logger::new_default()
		})
		.with_env_filter(EnvFilter::from_env("DISCORD_LOG"))
		.compact()
		.with_ansi(false)
		.finish();
	
	let _ = tracing::subscriber::set_default(webhook_sub);
	
	// warn!("test");
	
	rt.block_on(init_client());
	
	
}

async fn init_client() -> Client {
	let token = &Constants::get_constants().discord_token;
	assert!(validate_token(&token).is_ok(), "Invalid discord token!");
	
	let http = Http::new_with_token(&token);
	
	// We will fetch your bot's owners and id
	let (owners, _bot_id) = match http.get_current_application_info().await {
		Ok(info) => {
			let mut owners = HashSet::new();
			owners.insert(info.owner.id);
			
			(owners, info.id)
		}
		Err(why) => panic!("Could not access application info: {:?}", why),
	};
	
	// Create the framework
	let framework = StandardFramework::new()
		.configure(|c| c
			.owners(owners)
			.prefix("~"))
		.group(&GENERAL_GROUP);
	
	
	let mut client = serenity::Client::builder(&token)
		.framework(framework)
		.event_handler(Handler)
		.await
		.expect("serenity failed sonehow!");
	
	{
		let mut data = client.data.write().await;
		data.insert::<ShardManagerContainer>(client.shard_manager.clone());
	}
	
	let shard_manager = client.shard_manager.clone();
	
	tokio::spawn(async move {
		tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");
		shard_manager.lock().await.shutdown_all().await;
	});
	
	if let Err(why) = client.start_autosharded().await {
		error!("Client error: {:?}", why);
	}
	return client;
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> serenity::framework::standard::CommandResult {
	msg.reply(ctx, "Pong!").await?;
	
	Ok(())
}
