use serenity::async_trait;
use serenity::client::bridge::gateway::ShardManager;
use serenity::client::{validate_token, Context};
use serenity::framework::standard::macros::{command, group};
use serenity::framework::StandardFramework;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::TypeMapKey;
use serenity::Client;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    async fn ready(&self, _: Context, ready: Ready) {
        match ready.shard {
            Some(shard) => tracing::info!(
                "{} is connected with shard {} of {}!",
                ready.user.name,
                shard[0],
                shard[1]
            ),
            None => tracing::info!("{} is connected!", ready.user.name),
        }
    }
}

pub async fn init_client() -> Client {
    tracing::debug!("Getting Client Token");
    let token = std::env::var("DISCORD_TOKEN").expect("No Token. Unable to Start Bot!");
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
        .configure(|c| c.owners(owners).prefix("~"))
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
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
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
