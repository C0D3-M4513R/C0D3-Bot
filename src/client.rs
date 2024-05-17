use poise::{CreateReply, serenity_prelude as serenity};
use serenity::gateway::ShardManager;
use serenity::utils::{validate_token};
use serenity::prelude::TypeMapKey;
use serenity::Client;
use std::default::Default;
use std::sync::Arc;

use serenity::all::GatewayIntents;
use crate::message::MessageFlags;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

///Gets the latency of the current shard to discord's gateway.
#[poise::command(
    slash_command,
    install_context = "Guild|User",
)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    const CONVERSION_STEP:u32 = 1000;
    let time = ctx.ping().await;
    let ns = time.subsec_nanos();
    //get submicro_nanos
    let us = ns/CONVERSION_STEP;
    let ns = ns - us *CONVERSION_STEP;
    //get submilli_micros
    let ms = us /CONVERSION_STEP;
    let us = us - ms*CONVERSION_STEP;
    //get subsec_milli
    ctx.send(CreateReply::default().content(format!("Pong! Shard {}'s Latency to Gateway: {}s{ms}ms{us}µs{ns}ns", ctx.serenity_context().shard_id, time.as_secs(), )).ephemeral(true).reply(true)).await?;
    Ok(())
}

///Sends a message with some Component Link Buttons.
#[poise::command(
    slash_command,
    install_context = "Guild|User",
)]
async fn message(ctx: Context<'_>,
                 #[description = "Message"] message: String,
) -> Result<(), Error> {
    let message:super::message::Message = serde_json::from_str(message.as_str())?;

    match message.flags.as_ref().copied() {
        None | Some(MessageFlags::Reply) | Some(MessageFlags::Ephemral) => {
            ctx.send(message.into()).await?;
        },
        Some(MessageFlags::NoReply) => {
            ctx.channel_id().send_message(ctx, message.into()).await?;
            ctx.send(CreateReply::default().ephemeral(true).content("Send new Message!")).await?;
        },
        Some(MessageFlags::Edit{id: message_id}) => {
            ctx.channel_id().edit_message(ctx, message_id, message.into()).await?;
            ctx.send(CreateReply::default().ephemeral(true).content("Send new Message!")).await?;
        }
    }
    Ok(())
}


pub async fn init_client() -> Client {
    tracing::debug!("Getting Client Token");
    let token = std::env::var("DISCORD_TOKEN").expect("No Token. Unable to Start Bot!");
    assert!(validate_token(&token).is_ok(), "Invalid discord token!");

    let framework = poise::framework::FrameworkBuilder::default()
        .options(poise::FrameworkOptions {
            commands: vec![ping(), message()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let mut client = serenity::Client::builder(&token, GatewayIntents::default())
        .framework(framework)
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
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start_autosharded().await {
        tracing::error!("Client error: {:?}", why);
    }
    return client;
}