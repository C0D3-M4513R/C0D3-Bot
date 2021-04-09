use std::io::Write;

use serenity::Error;
use serenity::builder::ExecuteWebhook;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::webhook::Webhook;
use tokio::runtime::Runtime;

use crate::constants::Constants;
use tracing_subscriber::fmt::{MakeWriter, FormatEvent, FmtContext};
use std::borrow::BorrowMut;
use serenity::model::id::WebhookId;
use tracing::Event;


type ChannelType<'a, 'b> = &'a mut ExecuteWebhook<'b>;
// trait Modify<'a,'b> = FnOnce(ChannelType) -> ChannelType;
pub struct Logger<'a> {
	///The channel, that will be used for debugging
	token: &'a str,
	id: u64,
	rt: Runtime,
	context: Http,
	channel: Webhook,
}

impl<'a> Logger<'a>{
	
	pub fn say_sync_str(&self, wait: Option<bool>, message: String) -> Option<Message> {
		self.rt.block_on(self.say_str(wait, message))
	}
	
	pub fn say_sync<'b, F>(&self, wait: Option<bool>, message: F) -> Option<Message>
		where for<'c> F: FnOnce(ChannelType<'c, 'b>) -> ChannelType<'c, 'b> {
		self.rt.block_on(self.say(wait, message))
	}
	
	pub async fn say_str(&self, wait:Option<bool>, message: String) -> Option<Message> {
		self.say(wait, |m| m.content(message)).await
	}
	
	pub async fn say<'b, F>(&self, wait: Option<bool>, message: F) -> Option<Message>
		where for<'c> F: FnOnce(ChannelType<'c, 'b>) -> ChannelType<'c, 'b> {
		match self.channel.execute(&self.context, wait.unwrap_or(false), message).await {
			Err(Error::Model(_)) => {
				tracing::error!("Token is reportedly none?");
				None
			},
			Err(Error::Http(_)) => {
				tracing::error!("Content is malformed, or the webhook's token is invalid.");
				None
			},
			Err(Error::Json(_)) =>{
				tracing::error!("Received invalid json!");
				None
			},
			Err(_)=>{
				tracing::error!("Error: Other. Reportedly, this should never happen, according to serenity?");
				None
			},
			Ok(v)=>v
		}
	}

	fn new(webhook_id: u64, webhook_token: &'a str) -> Self {
		let context = Http::new_with_token(webhook_token);
		let rt = tokio::runtime::Builder::new_multi_thread().thread_name("Discord-Webhook-Logging").build().unwrap();
		let channel = rt.block_on(context.get_webhook_with_token(webhook_id,webhook_token)).map_err(|_| tracing::error!("Discord Fucked up")).unwrap();
		Logger {
			token: webhook_token,
			id: webhook_id,
			rt,
			context,
			channel,
		}
	}
	
	pub fn new_default() -> Self {
		Self::new(
			Constants::get_constants().default_webhook_id,
			&Constants::get_constants().default_webhook_token
		)
	}
	
	// pub async fn get_logger() -> &'static Logger
	// {
	// 	static mut LOGGER: Option<Logger> = None;
	// 	unsafe {
	// 		LOGGER = Some(Logger::new_default());
	// 		LOGGER.as_ref().unwrap()
	// 	}
	// }
}

impl<'a> Write for Logger<'a> {
	///Note: This will always return buf.len(), if successful, because of potential encoding issues,
	///which would result in part of the buffer being retried, and sending a partial message.
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		
		let str = String::from_utf8_lossy(buf).to_string();
		match self.say_sync_str(None,str) {
			Some(_) => Ok(buf.len()),
			None => Err(std::io::Error::new(
				std::io::ErrorKind::Interrupted,
				"Printing too many Messages. Can't keep up. Messages have probably been dropped!")
			),
		}
	}
	///The Writes are not buffered, so this doesn't apply!
	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}