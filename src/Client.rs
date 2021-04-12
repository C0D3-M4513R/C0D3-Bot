use serenity::{Error, Client};
use crate::constants::Constants;
use serenity::model::guild::Guild;
use serenity::prelude::TypeMapKey;
use std::sync::Arc;
use serenity::client::bridge::gateway::ShardManager;
use serenity::futures::TryFutureExt;
use std::collections::{HashMap, HashSet};
use serenity::model::id::{ChannelId,GuildId};
use serenity::model::channel::{GuildChannel, Message};
use serenity::client::{validate_token, Context};
use serenity::http::Http;
use serenity::framework::StandardFramework;
use tokio::sync::Mutex;
use serenity::async_trait;
use serenity::framework::standard::macros::{group,command};
use serenity::model::gateway::Ready;

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
		tracing::info!("{} is connected!", ready.user.name);
		let cty = ctx.clone();
		let http = cty.http.clone();
		let http_ref = http.as_ref();
		let guilds = cty
			.cache
			.guilds()
			.await;
		
		for guild_i in &guilds {
			tracing::info!("Got guild Id:{}", guild_i.as_u64())
		}
		
		let tmp = Guild::get(
			cty
				.http
				.clone()
				.as_ref(),
			GuildId::from(Constants::get_constants().default_bot_logging_guild_id),
		).and_then(|guild| async move {
			tracing::info!("Got Guild!");
			guild
				// .as_ref()
				// .expect(constants::Constants::get_constants().err_bot_guild_invalid.as_str())
				.channels(http_ref).await
		}
		).inspect_err(|e:&serenity::Error|{
			tracing::error!("Inner error is '{}'",e.to_string());
			panic!("{}",serenity::Error::Other(Constants::get_constants().err_bot_guild_invalid.as_str()));
		}).and_then(|channels:HashMap<ChannelId, GuildChannel>| async move {
			tracing::info!("Got channels!");
			let channel = channels.get(&ChannelId::from(Constants::get_constants().default_bot_logging_channel_id));
			if channel.is_some(){
				tracing::info!("Got a valid logging Channel!");
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

pub async fn init_client() -> Client {
	tracing::debug!("Getting Client Token");
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
	
	
	let mut client =
		serenity::Client::builder(&token)
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
		tracing::error!("Client error: {:?}", why);
	}
	return client;
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> serenity::framework::standard::CommandResult {
	msg.reply(ctx, "Pong!").await?;
	
	Ok(())
}
